use std::path::{Path, PathBuf};

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use cooklang_fs::{FsIndex, RecipeEntry};
use notify::{RecommendedWatcher, Watcher};
use serde::Serialize;
use tokio::sync::{mpsc, oneshot};

pub struct AsyncFsIndex {
    tx: mpsc::Sender<Message>,
}

pub type Responder<T> = oneshot::Sender<Result<T, cooklang_fs::Error>>;

#[derive(Debug)]
enum Message {
    Get {
        recipe: String,
        resp: Responder<RecipeEntry>,
    },
    Update(Update),
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Update {
    Modified { path: Utf8PathBuf },
    Added { path: Utf8PathBuf },
    Deleted { path: Utf8PathBuf },
    Renamed { path: Utf8PathBuf, to: Utf8PathBuf },
}

impl AsyncFsIndex {
    pub fn new(mut index: FsIndex) -> Result<(Self, mpsc::Receiver<Update>)> {
        index.index_all()?;

        let (tx, mut rx) = mpsc::channel::<Message>(1);
        let (updates_tx, updates_rx) = mpsc::channel::<Update>(1);
        watch_changes_task(tx.clone(), index.base_path());

        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    Message::Get { recipe, resp } => {
                        let r = index.get(&recipe);
                        let _ = resp.send(r);
                    }
                    Message::Update(updated) => {
                        let _ = updates_tx.send(updated.clone()).await;
                        match updated {
                            Update::Modified { .. } => {}
                            Update::Added { path } => {
                                index.add_recipe(path.as_str()).unwrap();
                            }
                            Update::Deleted { path } => {
                                index.remove_recipe(path.as_str()).unwrap();
                            }
                            Update::Renamed { path: from, to } => {
                                index.remove_recipe(from.as_str()).unwrap();
                                index.add_recipe(to.as_str()).unwrap();
                            }
                        }
                    }
                }
            }
        });

        Ok((Self { tx }, updates_rx))
    }

    pub async fn get(&self, recipe: String) -> Result<RecipeEntry, cooklang_fs::Error> {
        tracing::trace!("Looking up '{recipe}'");
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Message::Get { recipe, resp: tx })
            .await
            .unwrap();
        rx.await.unwrap()
    }
}

fn watch_changes_task(tx: mpsc::Sender<Message>, base_path: &Utf8Path) {
    let base_path = base_path.canonicalize().expect("Bad base path");

    tokio::spawn(async move {
        let (mut watcher, mut w_rx) = async_watcher().unwrap();
        watcher
            .watch(&base_path, notify::RecursiveMode::Recursive)
            .unwrap();

        while let Some(res) = w_rx.recv().await {
            let ev = match res {
                Ok(ev) => ev,
                Err(e) => {
                    tracing::error!("Error in file watcher: {}", e);
                    continue;
                }
            };
            match ev.kind {
                notify::EventKind::Create(_) => {
                    for path in iter_paths(&base_path, &ev.paths) {
                        tx.send(Message::Update(Update::Added { path }))
                            .await
                            .unwrap();
                    }
                }
                notify::EventKind::Modify(notify::event::ModifyKind::Name(rename)) => {
                    if let Some(msg) = handle_rename(&ev.paths, rename, &mut w_rx, &base_path).await
                    {
                        tx.send(msg).await.unwrap();
                    } else {
                        // fallback
                        for path in iter_paths(&base_path, &ev.paths) {
                            tx.send(Message::Update(Update::Modified { path }))
                                .await
                                .unwrap();
                        }
                    }
                }
                notify::EventKind::Modify(_) => {
                    for path in iter_paths(&base_path, &ev.paths) {
                        tx.send(Message::Update(Update::Modified { path }))
                            .await
                            .unwrap();
                    }
                }
                notify::EventKind::Remove(_) => {
                    for path in iter_paths(&base_path, &ev.paths) {
                        tx.send(Message::Update(Update::Deleted { path }))
                            .await
                            .unwrap();
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
) -> Option<Message> {
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
                    return Some(Message::Update(Update::Renamed {
                        path: paths.pop().unwrap(),
                        to: next_paths.pop().unwrap(),
                    }));
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
            Some(Message::Update(Update::Renamed { path: from, to }))
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
            path.strip_prefix(&base_path)
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
