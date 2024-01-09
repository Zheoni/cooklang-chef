use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive},
        Sse,
    },
};
use futures::{Stream, TryStreamExt};
use tokio_stream::wrappers::{errors::BroadcastStreamRecvError, BroadcastStream};

use crate::serve::{async_index::Update, S};

pub async fn sse_updates(
    State(state): State<S>,
) -> Sse<impl Stream<Item = Result<Event, BroadcastStreamRecvError>>> {
    let stream = BroadcastStream::new(state.updates_stream.resubscribe()).map_ok(|updt| {
        let e = Event::default();
        match updt {
            Update::Modified { path } => e.event("modified").data(path),
            Update::Added { path } => e.event("added").data(path),
            Update::Deleted { path } => e.event("deleted").data(path),
            Update::Renamed { from, to } => e
                .event("renamed")
                .json_data(serde_json::json!({
                    "from": from,
                    "to": to
                }))
                .unwrap(),
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
