use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use cooklang::{CooklangParser, MetadataResult};
use cooklang_fs::{FsIndex, RecipeEntry};
use notify::{RecommendedWatcher, Watcher};
use serde::Serialize;
use tokio::sync::{broadcast, mpsc, RwLock};

pub struct AsyncFsIndex {
    indexes: Arc<RwLock<Indexes>>,
}

pub struct RecipeData {
    pub metadata: MetadataResult,
    pub ingredients: Vec<String>,
    pub cookware: Vec<String>,
}

struct Indexes {
    parser: CooklangParser,
    fs: FsIndex,
    srch: BTreeMap<Utf8PathBuf, RecipeData>,
}

impl Indexes {
    fn new(fs: FsIndex) -> Self {
        // Empty (owned) parser just for metadata
        let parser = cooklang::CooklangParser::new(
            cooklang::Extensions::SPECIAL_METADATA,
            cooklang::Converter::empty(),
        );
        let mut srch = BTreeMap::new();
        let insert_search_entry = |index: &mut BTreeMap<_, _>, entry: RecipeEntry| {
            let content = entry.read().expect("can't read recipe");
            index.insert(
                entry.path().to_owned(),
                RecipeData {
                    metadata: content.metadata(&parser),
                    ingredients: content.ingredients(&parser),
                    cookware: content.cookware(&parser),
                },
            );
        };
        for entry in fs.get_all() {
            insert_search_entry(&mut srch, entry);
        }

        Self { fs, srch, parser }
    }

    fn revalidate(&mut self, path: &Utf8Path) -> Result<(), cooklang_fs::Error> {
        self.srch.remove(path);
        self.insert_srch(path)
    }

    fn remove(&mut self, path: &Utf8Path) {
        self.srch.remove(path);
        let _ = self.fs.remove(path);
    }

    fn insert_srch(&mut self, path: &Utf8Path) -> Result<(), cooklang_fs::Error> {
        let content = RecipeEntry::new(path).read()?;
        self.srch.insert(
            path.to_owned(),
            RecipeData {
                metadata: content.metadata(&self.parser),
                ingredients: content.ingredients(&self.parser),
                cookware: content.cookware(&self.parser),
            },
        );
        Ok(())
    }

    fn insert(&mut self, path: &Utf8Path) -> Result<(), cooklang_fs::Error> {
        let _ = self.fs.insert(path);
        self.insert_srch(path)
    }
}

// the paths are relative to the base path, but without the base path itself
// so not './recipe.cook', just 'recipe.cook'.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Update {
    Modified { path: Utf8PathBuf },
    Added { path: Utf8PathBuf },
    Deleted { path: Utf8PathBuf },
    Renamed { from: Utf8PathBuf, to: Utf8PathBuf },
}

impl AsyncFsIndex {
    pub fn new(index: FsIndex) -> (Self, broadcast::Receiver<Update>) {
        let (in_updt_tx, mut in_updt_rx) = mpsc::channel::<Update>(1);
        let (out_updates_tx, out_updates_rx) = broadcast::channel::<Update>(1);
        watch_changes_task(in_updt_tx, index.base_path());

        let indexes = Arc::new(RwLock::new(Indexes::new(index)));

        let indexes2 = Arc::clone(&indexes);
        tokio::spawn(async move {
            let indexes = indexes2;
            while let Some(update) = in_updt_rx.recv().await {
                match &update {
                    Update::Modified { path } => {
                        tracing::info!("Updated '{path}'");
                        let _ = indexes.write().await.revalidate(path);
                    }
                    Update::Added { path } => {
                        tracing::info!("Added '{path}'");
                        let _ = indexes.write().await.insert(path);
                    }
                    Update::Deleted { path } => {
                        tracing::info!("Deleted '{path}'");
                        indexes.write().await.remove(path);
                    }
                    Update::Renamed { from, to } => {
                        tracing::info!("Renamed '{from}' to '{to}'");
                        let mut indexes = indexes.write().await;
                        indexes.remove(from);
                        let _ = indexes.insert(to);
                    }
                }
                // resend update after index is updated
                let _ = out_updates_tx.send(update);
            }
        });

        (Self { indexes }, out_updates_rx)
    }

    pub fn resolve_blocking(
        &self,
        recipe: &str,
        relative_to: Option<&Utf8Path>,
    ) -> Result<RecipeEntry, cooklang_fs::Error> {
        let indexes = self.indexes.blocking_read();
        // make sure the path is inside the base path, the cli is allowed to be outside, not here
        indexes.fs.resolve(recipe, relative_to)
    }

    pub async fn get(&self, recipe: &str) -> Result<RecipeEntry, cooklang_fs::Error> {
        let indexes = self.indexes.read().await;
        indexes.fs.get(recipe)
    }

    pub async fn search<T>(
        &self,
        pred: impl Fn(&RecipeEntry, Option<&RecipeData>) -> bool,
        map: impl Fn(RecipeEntry, Option<&RecipeData>) -> T,
        skip: usize,
        take: usize,
    ) -> Vec<T> {
        let indexes = self.indexes.read().await;
        indexes
            .fs
            .get_all()
            .filter_map(|entry| {
                let tokens = indexes.srch.get(entry.path());
                match pred(&entry, tokens) {
                    true => Some((entry, tokens)),
                    false => None,
                }
            })
            .skip(skip)
            .take(take)
            .map(|(entry, meta)| map(entry, meta))
            .collect()
    }
}

fn watch_changes_task(tx: mpsc::Sender<Update>, base_path: &Utf8Path) {
    let watched_path = base_path.canonicalize().expect("Bad base path");
    let base_path = base_path.to_owned();

    tokio::spawn(async move {
        let (mut watcher, mut w_rx) = async_watcher().unwrap();
        watcher
            .watch(&watched_path, notify::RecursiveMode::Recursive)
            .unwrap();

        // debounce updates
        const MIN_DELAY: Duration = Duration::from_millis(500);
        let mut pending: Option<tokio::task::JoinHandle<()>> = None;
        let mut send = |updt| {
            if let Some(handle) = pending.take() {
                handle.abort();
            }
            let tx2 = tx.clone();
            let handle = tokio::spawn(async move {
                tokio::time::sleep(MIN_DELAY).await;
                let _ = tx2.send(updt).await;
            });
            pending = Some(handle);
        };

        // watcher returns canonicalized paths, iter_paths strips the
        // canonicalized based path then this restores the path prefixed with
        // the base path not canonicalized
        let restore_path = |p| base_path.join(p);

        while let Some(res) = w_rx.recv().await {
            let ev = match res {
                Ok(ev) => ev,
                Err(e) => {
                    tracing::error!("Error in file watcher: {}", e);
                    continue;
                }
            };
            let paths = iter_paths(&watched_path, &ev.paths);
            match ev.kind {
                notify::EventKind::Create(_) => {
                    for path in paths {
                        send(Update::Added {
                            path: restore_path(path),
                        });
                    }
                }
                notify::EventKind::Modify(notify::event::ModifyKind::Name(rename)) => {
                    if let Some((from, to)) =
                        handle_rename(&ev.paths, rename, &mut w_rx, &watched_path).await
                    {
                        send(Update::Renamed {
                            from: restore_path(from),
                            to: restore_path(to),
                        })
                    } else {
                        // fallback
                        for path in paths {
                            send(Update::Modified {
                                path: restore_path(path),
                            });
                        }
                    }
                }
                notify::EventKind::Modify(_) => {
                    for path in paths {
                        send(Update::Modified {
                            path: restore_path(path),
                        });
                    }
                }
                notify::EventKind::Remove(_) => {
                    for path in paths {
                        send(Update::Deleted {
                            path: restore_path(path),
                        });
                    }
                }
                _ => {}
            }
        }
    });
}

async fn handle_rename(
    paths: &[PathBuf],
    rename: notify::event::RenameMode,
    w_rx: &mut mpsc::Receiver<Result<notify::Event, notify::Error>>,
    watched_path: &Path,
) -> Option<(Utf8PathBuf, Utf8PathBuf)> {
    let mut paths = iter_paths(watched_path, paths);

    match rename {
        notify::event::RenameMode::From => {
            let mut paths = paths.collect::<Vec<_>>();
            if paths.len() != 1 {
                return None;
            }

            let next_res = tokio::select! {
                ev = w_rx.recv() => ev,
                _  = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => None,
            };

            if let Some(Ok(next_ev)) = next_res {
                let mut next_paths = iter_paths(watched_path, &next_ev.paths).collect::<Vec<_>>();
                if next_paths.len() != 1 {
                    return None;
                }
                if let notify::EventKind::Modify(notify::event::ModifyKind::Name(
                    notify::event::RenameMode::To,
                )) = next_ev.kind
                {
                    let from = paths.pop().unwrap();
                    let to = next_paths.pop().unwrap();
                    return Some((from, to));
                }
            }
            None
        }
        notify::event::RenameMode::Both => {
            let from = paths.next()?;
            let to = paths.next()?;
            if paths.next().is_some() {
                return None;
            }
            Some((from, to))
        }
        _ => None,
    }
}

fn iter_paths<'a>(
    base_path: &'a Path,
    paths: &'a [PathBuf],
) -> impl Iterator<Item = Utf8PathBuf> + 'a {
    paths
        .iter()
        .filter_map(move |path| {
            path.strip_prefix(base_path)
                .ok()
                .and_then(|p| Utf8Path::from_path(p).map(Utf8Path::to_path_buf))
        })
        .filter(|p| p.extension() == Some("cook"))
}

fn async_watcher() -> notify::Result<(
    RecommendedWatcher,
    mpsc::Receiver<notify::Result<notify::Event>>,
)> {
    let (tx, rx) = mpsc::channel(1);
    let watcher = RecommendedWatcher::new(
        move |res| {
            tx.blocking_send(res).unwrap();
        },
        notify::Config::default(),
    )?;
    Ok((watcher, rx))
}
