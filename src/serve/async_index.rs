use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use cooklang_fs::{FsIndex, RecipeEntry};
use notify::{RecommendedWatcher, Watcher};
use serde::Serialize;
use tokio::sync::{broadcast, mpsc, oneshot};

pub struct AsyncFsIndex {
    calls_tx: mpsc::Sender<Call>,
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

type Responder<T> = oneshot::Sender<Result<T, cooklang_fs::Error>>;

#[derive(Debug)]
enum Call {
    Get {
        recipe: String,
        resp: Responder<RecipeEntry>,
    },
}

impl AsyncFsIndex {
    pub fn new(mut index: FsIndex) -> Result<(Self, broadcast::Receiver<Update>)> {
        index.index_all()?;

        let (in_updt_tx, mut in_updt_rx) = mpsc::channel::<Update>(1);
        let (calls_tx, mut calls_rx) = mpsc::channel::<Call>(1);
        let (out_updates_tx, out_updates_rx) = broadcast::channel::<Update>(1);
        watch_changes_task(in_updt_tx, index.base_path());

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(call) = calls_rx.recv() => {
                        match call {
                            Call::Get { recipe, resp } => {
                                let res = index.get(&recipe);
                                let _ = resp.send(res);
                            }
                        }
                    }
                    Some(update) = in_updt_rx.recv() => {
                        match &update {
                            Update::Modified { .. } => {}
                            Update::Added { path } => {
                                index.add_recipe(&index.base_path().join(path)).unwrap();
                            }
                            Update::Deleted { path } => {
                                index.remove_recipe(&index.base_path().join(path)).unwrap();
                            }
                            Update::Renamed { from, to } => {
                                index.remove_recipe(&index.base_path().join(from)).unwrap();
                                index.add_recipe(&index.base_path().join(to)).unwrap();
                            },
                        }
                        // resend update after index is updated
                        let _ = out_updates_tx.send(update);
                    }
                    else => break,
                }
            }
        });

        Ok((Self { calls_tx }, out_updates_rx))
    }

    pub async fn get(&self, recipe: String) -> Result<RecipeEntry, cooklang_fs::Error> {
        tracing::trace!("Looking up '{recipe}'");
        let (tx, rx) = oneshot::channel();
        self.calls_tx
            .send(Call::Get { recipe, resp: tx })
            .await
            .unwrap();
        rx.await.unwrap()
    }
}

fn watch_changes_task(tx: mpsc::Sender<Update>, base_path: &Utf8Path) {
    let base_path = base_path.canonicalize().expect("Bad base path");

    tokio::spawn(async move {
        let (mut watcher, mut w_rx) = async_watcher().unwrap();
        watcher
            .watch(&base_path, notify::RecursiveMode::Recursive)
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

        while let Some(res) = w_rx.recv().await {
            let ev = match res {
                Ok(ev) => ev,
                Err(e) => {
                    tracing::error!("Error in file watcher: {}", e);
                    continue;
                }
            };
            let paths = iter_paths(&base_path, &ev.paths);
            match ev.kind {
                notify::EventKind::Create(_) => {
                    for path in paths {
                        send(Update::Added { path });
                    }
                }
                notify::EventKind::Modify(notify::event::ModifyKind::Name(rename)) => {
                    if let Some(msg) = handle_rename(&ev.paths, rename, &mut w_rx, &base_path).await
                    {
                        send(msg);
                    } else {
                        // fallback
                        for path in paths {
                            send(Update::Modified { path });
                        }
                    }
                }
                notify::EventKind::Modify(_) => {
                    for path in paths {
                        send(Update::Modified { path });
                    }
                }
                notify::EventKind::Remove(_) => {
                    for path in paths {
                        send(Update::Deleted { path });
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
    base_path: &Path,
) -> Option<Update> {
    let mut paths = iter_paths(base_path, paths);

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
                let mut next_paths = iter_paths(base_path, &next_ev.paths).collect::<Vec<_>>();
                if next_paths.len() != 1 {
                    return None;
                }
                if let notify::EventKind::Modify(notify::event::ModifyKind::Name(
                    notify::event::RenameMode::To,
                )) = next_ev.kind
                {
                    return Some(Update::Renamed {
                        from: paths.pop().unwrap(),
                        to: next_paths.pop().unwrap(),
                    });
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
            Some(Update::Renamed { from, to })
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
