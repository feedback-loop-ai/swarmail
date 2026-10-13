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
use tokio::sync::watch;

/// One wire frame of the feed: the event name and its JSON payload. Kept
/// plain (rather than an axum `Event`) so unit tests can assert on it
/// without an HTTP layer.
pub type Frame = (String, String);

/// The event name every payload is sent under; the browser listens for it.
pub const EVENT: &str = "threads";

/// The feed's state: the store, the inbox, the watcher taken *before* the
/// first scan, whether the snapshot has gone out yet, and the shutdown
/// watch that ends the stream.
struct Feed {
    store: Arc<Store>,
    inbox: String,
    rx: broadcast::Receiver<Arc<crate::model::Email>>,
    shutdown: watch::Receiver<bool>,
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
/// `shutdown` is the server's stop watch: once signaled (or once its sender
/// is dropped), the stream ends — one frame may still race out, then the
/// response completes and axum's graceful shutdown finishes. Without it an
/// open feed would hold `stop()` forever.
pub fn feed_events(
    store: Arc<Store>,
    inbox: String,
    rx: broadcast::Receiver<Arc<crate::model::Email>>,
    shutdown: watch::Receiver<bool>,
) -> impl Stream<Item = Frame> {
    unfold(
        Feed {
            store,
            inbox,
            rx,
            shutdown,
            warm: false,
        },
        |mut feed| async move {
            if !feed.warm {
                feed.warm = true;
                return Some((view_frame(&feed.store, &feed.inbox), feed));
            }
            // Post-snapshot: the watcher and the stop watch race in one
            // select. A closed sender rides the same arm as a signal — the
            // error path's drop-cascade is an end-of-stream, not a hang.
            let wake = tokio::select! {
                res = feed.rx.recv() => res,
                _ = feed.shutdown.changed() => return None,
            };
            match wake {
                Ok(_) | Err(RecvError::Lagged(_)) => {
                    Some((view_frame(&feed.store, &feed.inbox), feed))
                }
                // The sender is released when the inbox's mail is removed
                // (decision 0010): a drain is a destructive teardown, the
                // end-of-stream is its honest signal, and the browser's
                // `EventSource` reconnect re-establishes decision 0002 at
                // the wire boundary (`feed_stream` subscribes before the
                // first view scan) with the post-drain snapshot.
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
    shutdown: watch::Receiver<bool>,
) -> impl Stream<Item = Result<Event, Infallible>> {
    let rx = store.subscribe(&inbox);
    feed_events(store, inbox, rx, shutdown)
        .map(|(event, data)| Ok(Event::default().event(event).data(data)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::email;

    fn json_of(frame: &Frame) -> serde_json::Value {
        serde_json::from_str(&frame.1).unwrap()
    }

    /// A stop watch that never fires, for the tests that drive the
    /// watcher's arms; the shutdown tests inject their own. The sender
    /// travels with the receiver — a dropped sender is an end-of-stream,
    /// and the tests that want one drop it explicitly.
    fn quiet_shutdown() -> (watch::Sender<bool>, watch::Receiver<bool>) {
        watch::channel(false)
    }

    #[tokio::test]
    async fn the_first_frame_is_the_snapshot_and_pushes_follow_inserts() {
        let store = Arc::new(Store::new(0));
        let (_shutdown_tx, shutdown_rx) = quiet_shutdown();
        let stream = feed_events(
            store.clone(),
            "box".into(),
            store.subscribe("box"),
            shutdown_rx,
        );
        tokio::pin!(stream);
        // Snapshot first: the inbox is empty, the frame is an empty array.
        let snapshot = stream.as_mut().next().await.unwrap();
        assert_eq!(snapshot.0, EVENT);
        assert_eq!(json_of(&snapshot), serde_json::json!([]));
        // An accepted email wakes the watcher; the frame is the whole view.
        let mut mail = email("m1", "box", "to@x.io");
        mail.subject = Some("Hello".into());
        let _ = store.insert(mail);
        let push = stream.as_mut().next().await.unwrap();
        let view = json_of(&push);
        assert_eq!(view[0]["subject"], "Hello");
        assert_eq!(view[0]["count"], 1);
    }

    #[tokio::test]
    async fn a_lagged_watcher_gets_a_recomputed_view() {
        let store = Arc::new(Store::new(0));
        let (tx, rx) = broadcast::channel(1);
        let (_shutdown_tx, shutdown_rx) = quiet_shutdown();
        let stream = feed_events(store.clone(), "box".into(), rx, shutdown_rx);
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
        let _ = store.insert(first);
        let _ = store.insert(second);
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
        let (_shutdown_tx, shutdown_rx) = quiet_shutdown();
        let stream = feed_events(Arc::new(Store::new(0)), "box".into(), rx, shutdown_rx);
        tokio::pin!(stream);
        // The snapshot goes out regardless; the next poll ends the stream.
        assert_eq!(stream.as_mut().next().await.unwrap().0, EVENT);
        assert!(stream.as_mut().next().await.is_none());
    }

    /// The feed ends when its inbox's mail is removed (decision 0010, AC4
    /// feed half): the clear releases the channel the feed holds, and the
    /// `Closed` arm — the honest end-of-stream signal — becomes reachable.
    /// Deterministic by injection: `feed_events` takes the receiver the
    /// caller subscribed with, so no scheduler trick is needed to place the
    /// release after the snapshot.
    #[tokio::test]
    async fn a_cleared_inbox_ends_the_feed_after_the_snapshot() {
        let store = Arc::new(Store::new(0));
        let stream = feed_events(
            store.clone(),
            "box".into(),
            store.subscribe("box"),
            quiet_shutdown().1,
        );
        tokio::pin!(stream);
        // The snapshot goes out first: the post-subscribe view of the box.
        assert_eq!(
            json_of(&stream.as_mut().next().await.unwrap()),
            serde_json::json!([])
        );
        let _ = store.clear("box"); // the drain releases the feed's channel
        assert!(
            stream.as_mut().next().await.is_none(),
            "the released channel must end the stream"
        );
    }

    /// The stop watch ends the feed: the frame the mail raced out goes
    /// first, then the stream ends — one trailing frame, then the response
    /// completes and axum's graceful shutdown finishes (AC1). Deterministic
    /// by injection: the signal fires only after the push frame is
    /// consumed, so the select's shutdown arm is the only ready one.
    #[tokio::test]
    async fn a_stop_signal_ends_the_stream_after_one_trailing_frame() {
        let store = Arc::new(Store::new(0));
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let stream = feed_events(
            store.clone(),
            "box".into(),
            store.subscribe("box"),
            shutdown_rx,
        );
        tokio::pin!(stream);
        assert_eq!(stream.as_mut().next().await.unwrap().0, EVENT);
        store.insert(email("m1", "box", "to@x.io")).unwrap();
        let push = stream.as_mut().next().await.unwrap();
        assert_eq!(json_of(&push)[0]["count"], 1);
        let _ = shutdown_tx.send(true);
        assert!(
            stream.as_mut().next().await.is_none(),
            "the signaled watch must end the stream"
        );
    }

    /// A dropped stop sender rides the same arm as a signal: the error
    /// path's drop-cascade is an end-of-stream, not a hang.
    #[tokio::test]
    async fn a_dropped_stop_sender_ends_the_stream() {
        let store = Arc::new(Store::new(0));
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let stream = feed_events(
            store.clone(),
            "box".into(),
            store.subscribe("box"),
            shutdown_rx,
        );
        tokio::pin!(stream);
        assert_eq!(stream.as_mut().next().await.unwrap().0, EVENT);
        drop(shutdown_tx);
        assert!(
            stream.as_mut().next().await.is_none(),
            "the released sender must end the stream"
        );
    }

    #[tokio::test]
    async fn the_wire_stream_yields_a_frame_per_feed_frame() {
        let store = Arc::new(Store::new(0));
        let (_shutdown_tx, shutdown_rx) = quiet_shutdown();
        let stream = feed_stream(store, "box".into(), shutdown_rx);
        tokio::pin!(stream);
        // Infallible by construction; the exact wire bytes are an e2e
        // concern (asserted over real HTTP in tests/ui.rs).
        assert!(stream.as_mut().next().await.unwrap().is_ok());
    }
}
