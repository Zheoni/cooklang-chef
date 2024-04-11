use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive},
        Sse,
    },
};
use futures::{Stream, TryStreamExt};
use tokio_stream::wrappers::{errors::BroadcastStreamRecvError, BroadcastStream};

use crate::cmd::serve::{async_index::Update, S};

use super::clean_path;

pub async fn sse_updates(
    State(state): State<S>,
) -> Sse<impl Stream<Item = Result<Event, BroadcastStreamRecvError>>> {
    let base_path = state.base_path.clone();
    let stream = BroadcastStream::new(state.updates_stream.resubscribe()).map_ok(move |updt| {
        let e = Event::default();
        let p = |path| clean_path(path, &base_path);
        match updt {
            Update::Modified { path } => e.event("modified").data(p(&path)),
            Update::Added { path } => e.event("added").data(p(&path)),
            Update::Deleted { path } => e.event("deleted").data(p(&path)),
            Update::Renamed { from, to } => e
                .event("renamed")
                .json_data(serde_json::json!({
                    "from": p(&from),
                    "to": p(&to)
                }))
                .unwrap(),
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
