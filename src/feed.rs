//! The live inbox feed: server-sent events over the store's own watcher.
//!
//! Why SSE rather than short polling: the per-inbox broadcast watcher this
//! builds on is the very channel `await` long-polls with, so a push costs
//! nothing new, has no interval to tune and cannot miss a mail — a poller
//! would add an empty round trip per client per tick and a window between
//! "the poll read" and "the insert landed". The payload is the full thread
//! view (the `GET /threads` shape), so the browser renders it and nothing
//! else; the view is capped at `VIEW_CAP` mails, like the HTML views.
//!
//! Ordering is the load-bearing part: `feed_stream` **subscribes before the
//! first view scan** (decision 0002). Mail that lands between the scan and
//! the client's arrival is already buffered in the watcher and pushed on the
//! next frame, so the snapshot can never be stale in a way the push does not
//! repair. A watcher that falls behind (`Lagged`) is repaired the same way —
//! the frame is the recomputed view, not the missed inserts.

use crate::store::Store;
use crate::threads;
use axum::response::sse::Event;
use futures_util::StreamExt;
use futures_util::stream::{Stream, unfold};
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;

/// One wire frame of the feed: the event name and its JSON payload. Kept
/// plain (rather than an axum `Event`) so unit tests can assert on it
/// without an HTTP layer.
pub type Frame = (String, String);

/// The event name every payload is sent under; the browser listens for it.
pub const EVENT: &str = "threads";

/// The feed's state: the store, the inbox, the watcher taken *before* the
/// first scan, and whether the snapshot has gone out yet.
struct Feed {
    store: Arc<Store>,
    inbox: String,
    rx: broadcast::Receiver<Arc<crate::model::Email>>,
    warm: bool,
}

/// A frame carrying the current thread view. `serde_json` cannot fail on a
/// vec of serializable emails, so the failure mode is a panic, not an error
/// the router would have to name.
pub fn view_frame(store: &Store, inbox: &str) -> Frame {
    (
        EVENT.to_string(),
        serde_json::to_string(&threads::view(store, inbox)).expect("thread view serializes"),
    )
}

/// The feed's frames: one snapshot, then one full view per accepted email
/// (and per watcher lag). `rx` is the watcher the caller subscribed with
/// before scanning — passing it in is what makes that ordering visible.
pub fn feed_events(
    store: Arc<Store>,
    inbox: String,
    rx: broadcast::Receiver<Arc<crate::model::Email>>,
) -> impl Stream<Item = Frame> {
    unfold(
        Feed {
            store,
            inbox,
            rx,
            warm: false,
        },
        |mut feed| async move {
            if !feed.warm {
                feed.warm = true;
                return Some((view_frame(&feed.store, &feed.inbox), feed));
            }
            match feed.rx.recv().await {
                Ok(_) | Err(RecvError::Lagged(_)) => {
                    Some((view_frame(&feed.store, &feed.inbox), feed))
                }
                // The watcher's sender is only gone if the store is: end the
                // stream, which ends the response.
                Err(RecvError::Closed) => None,
            }
        },
    )
}

/// The wire stream the router serves: the frames above, framed as
/// server-sent events. The subscribe-before-scan ordering lives here
/// (decision 0002) — the watcher is taken before `feed_events` can scan.
pub fn feed_stream(
    store: Arc<Store>,
    inbox: String,
) -> impl Stream<Item = Result<Event, Infallible>> {
    let rx = store.subscribe(&inbox);
    feed_events(store, inbox, rx).map(|(event, data)| Ok(Event::default().event(event).data(data)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::email;

    fn json_of(frame: &Frame) -> serde_json::Value {
        serde_json::from_str(&frame.1).unwrap()
    }

    #[tokio::test]
    async fn the_first_frame_is_the_snapshot_and_pushes_follow_inserts() {
        let store = Arc::new(Store::new(0));
        let stream = feed_events(store.clone(), "box".into(), store.subscribe("box"));
        tokio::pin!(stream);
        // Snapshot first: the inbox is empty, the frame is an empty array.
        let snapshot = stream.as_mut().next().await.unwrap();
        assert_eq!(snapshot.0, EVENT);
        assert_eq!(json_of(&snapshot), serde_json::json!([]));
        // An accepted email wakes the watcher; the frame is the whole view.
        let mut mail = email("m1", "box", "to@x.io");
        mail.subject = Some("Hello".into());
        store.insert(mail);
        let push = stream.as_mut().next().await.unwrap();
        let view = json_of(&push);
        assert_eq!(view[0]["subject"], "Hello");
        assert_eq!(view[0]["count"], 1);
    }

    #[tokio::test]
    async fn a_lagged_watcher_gets_a_recomputed_view() {
        let store = Arc::new(Store::new(0));
        let (tx, rx) = broadcast::channel(1);
        let stream = feed_events(store.clone(), "box".into(), rx);
        tokio::pin!(stream);
        assert_eq!(
            json_of(&stream.as_mut().next().await.unwrap()),
            serde_json::json!([])
        );
        // Two mails land while the watcher can only hold one frame: the
        // second send lags it, and the lag frame must carry both.
        let mut first = email("lag-1", "box", "to@x.io");
        first.subject = Some("First".into());
        let mut second = email("lag-2", "box", "to@x.io");
        second.subject = Some("Second".into());
        store.insert(first);
        store.insert(second);
        tx.send(Arc::new(email("wake-1", "box", "to@x.io")))
            .unwrap();
        tx.send(Arc::new(email("wake-2", "box", "to@x.io")))
            .unwrap();
        let frame = stream.as_mut().next().await.unwrap();
        assert_eq!(frame.0, EVENT);
        let view = json_of(&frame);
        assert_eq!(
            view.as_array().unwrap().len(),
            2,
            "the lagged view is repaired"
        );
    }

    #[tokio::test]
    async fn a_closed_watcher_ends_the_stream_after_the_snapshot() {
        let (tx, rx) = broadcast::channel(4);
        drop(tx);
        let stream = feed_events(Arc::new(Store::new(0)), "box".into(), rx);
        tokio::pin!(stream);
        // The snapshot goes out regardless; the next poll ends the stream.
        assert_eq!(stream.as_mut().next().await.unwrap().0, EVENT);
        assert!(stream.as_mut().next().await.is_none());
    }

    #[tokio::test]
    async fn the_wire_stream_yields_a_frame_per_feed_frame() {
        let store = Arc::new(Store::new(0));
        let stream = feed_stream(store, "box".into());
        tokio::pin!(stream);
        // Infallible by construction; the exact wire bytes are an e2e
        // concern (asserted over real HTTP in tests/ui.rs).
        assert!(stream.as_mut().next().await.unwrap().is_ok());
    }
}
