//! Cold failure diagnostics for the refused decoded-control request window.
//! The harness is separate because its timeout and unreachable-request paths
//! run only when the deterministic schedule fails (decision 0020).

#[path = "../../tests/common/metrics.rs"]
mod metrics;

use super::{AppState, router};
use crate::chaos::Chaos;
use crate::store::Store;
use crate::webhook::Webhooks;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::Response;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, oneshot};
use tokio::task::JoinSet;

const HTTP_DEADLINE: Duration = Duration::from_secs(5);
const SERVER_DEADLINE: Duration = Duration::from_secs(10);

type HoldChannels = (oneshot::Sender<StatusCode>, oneshot::Receiver<()>);
type HoldSlot = Arc<Mutex<Option<HoldChannels>>>;

/// A cloneable, request-owned test handoff. The accounting middleware takes
/// it from that request's extensions and waits only after its real downstream
/// response exists; dropping either endpoint releases the hold.
#[derive(Clone)]
pub(super) struct RequestHold(HoldSlot);

impl RequestHold {
    pub(super) async fn after_response(self, status: StatusCode) {
        let channels = self.0.lock().await.take();
        if let Some((reached, release)) = channels
            && reached.send(status).is_ok()
        {
            let _ = release.await;
        }
    }
}

#[derive(Clone)]
struct HoldInstaller {
    target: Arc<str>,
    slot: Arc<Mutex<Option<RequestHold>>>,
}

impl HoldInstaller {
    fn new(target: &str, hold: RequestHold) -> Self {
        Self {
            target: Arc::from(target),
            slot: Arc::new(Mutex::new(Some(hold))),
        }
    }
}

/// The outer test layer attaches the one-shot only to the exact raw request
/// URI. Metrics and unrelated requests continue directly through the real
/// production router.
async fn install_hold(
    State(installer): State<HoldInstaller>,
    mut request: Request,
    next: Next,
) -> Response {
    let matches_target = request
        .uri()
        .path_and_query()
        .is_some_and(|path| path.as_str() == installer.target.as_ref());
    if matches_target {
        let hold = installer.slot.lock().await.take();
        if let Some(hold) = hold {
            request.extensions_mut().insert(hold);
        }
    }
    next.run(request).await
}

fn request_hold() -> (
    RequestHold,
    oneshot::Receiver<StatusCode>,
    oneshot::Sender<()>,
) {
    let (reached, reached_rx) = oneshot::channel();
    let (release, release_rx) = oneshot::channel();
    (
        RequestHold(Arc::new(Mutex::new(Some((reached, release_rx))))),
        reached_rx,
        release,
    )
}

async fn get(addr: std::net::SocketAddr, path: &str) -> (u16, Vec<u8>) {
    tokio::time::timeout(HTTP_DEADLINE, async {
        let mut stream = TcpStream::connect(addr).await.expect("connect to test API");
        stream
            .write_all(
                format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n")
                    .as_bytes(),
            )
            .await
            .expect("write test API request");
        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .await
            .expect("read complete test API response");
        let boundary = response
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .expect("HTTP response has a header/body boundary");
        let head = std::str::from_utf8(&response[..boundary]).expect("HTTP headers are ASCII");
        let status = head
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .expect("HTTP response has a status")
            .parse::<u16>()
            .expect("HTTP status is numeric");
        (status, response[boundary + 4..].to_vec())
    })
    .await
    .expect("test API HTTP operation exceeded its deadline")
}

async fn scrape(addr: std::net::SocketAddr) -> Vec<metrics::Sample> {
    let (status, body) = get(addr, "/metrics").await;
    assert_eq!(status, 200, "metrics response status");
    let text = std::str::from_utf8(&body).expect("metrics body is UTF-8");
    metrics::parse_prometheus(text).unwrap_or_else(|error| panic!("{error}:\n{text}"))
}

fn inbox_label(sample: &metrics::Sample) -> Option<&str> {
    sample
        .1
        .iter()
        .find(|(name, _)| name == "inbox")
        .map(|(_, value)| value.as_str())
}

fn assert_no_await_registration(samples: &[metrics::Sample], names: &[&str], stage: &str) {
    for family in ["swarmail_awaits_active", "swarmail_awaits_registered_total"] {
        for name in names {
            assert!(
                !samples
                    .iter()
                    .any(|sample| { sample.0 == family && inbox_label(sample) == Some(*name) }),
                "{family} registered {name:?} {stage}: {samples:?}"
            );
        }
    }
}

fn app_state(addr: std::net::SocketAddr) -> (AppState, tokio::sync::watch::Sender<bool>) {
    let (shutdown, receiver) = tokio::sync::watch::channel(false);
    (
        AppState {
            store: Arc::new(Store::new(0)),
            chaos: Arc::new(Chaos::default()),
            webhooks: Arc::new(Webhooks::default()),
            started: Instant::now(),
            shutdown: receiver,
            smtp_degraded: Arc::new(AtomicBool::new(false)),
            pop3_degraded: Arc::new(AtomicBool::new(false)),
            http_origin_guard: Some(addr.port()),
        },
        shutdown,
    )
}

#[tokio::test]
async fn refused_control_request_keeps_its_decoded_metric_until_the_refusal_returns() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test API listener");
    let addr = listener.local_addr().expect("read test API address");
    let (state, shutdown) = app_state(addr);
    let prefix = "issue202-refusal-hold";
    let raw = format!("{prefix}-%0D%00");
    let decoded = format!("{prefix}-\r\0");
    let path = format!("/api/v1/inboxes/{raw}/await?timeout_ms=60000");
    let (hold, reached, release) = request_hold();
    let installer = HoldInstaller::new(&path, hold);
    let test_router = router(state).layer(middleware::from_fn_with_state(installer, install_hold));
    let (graceful_stop, graceful_rx) = oneshot::channel::<()>();
    let mut server = JoinSet::new();
    server.spawn(async move {
        axum::serve(listener, test_router)
            .with_graceful_shutdown(async {
                let _ = graceful_rx.await;
            })
            .await
            .expect("the real API router serves the test listener");
    });

    let aliases = [decoded.as_str(), raw.as_str()];
    let before = scrape(addr).await;
    assert_no_await_registration(&before, &aliases, "before the request");
    assert!(
        !before.iter().any(|sample| {
            sample.0 == "swarmail_http_requests_in_flight"
                && inbox_label(sample).is_some_and(|label| label.starts_with(prefix))
        }),
        "the exclusive request label exists before the request: {before:?}"
    );

    let mut clients = JoinSet::new();
    clients.spawn(async move { get(addr, &path).await });
    let response_status = tokio::time::timeout(HTTP_DEADLINE, reached)
        .await
        .expect("the refusal response never reached the accounting hold")
        .expect("the request-owned hold was dropped before the refusal");
    assert_eq!(response_status, StatusCode::BAD_REQUEST);

    // The response object now exists and its accounting guards are still
    // alive, but no registry mutex is held across this real scrape.
    let during = scrape(addr).await;
    let request_rows: Vec<_> = during
        .iter()
        .filter(|sample| {
            sample.0 == "swarmail_http_requests_in_flight"
                && inbox_label(sample).is_some_and(|label| label.starts_with(prefix))
        })
        .collect();
    assert_eq!(
        request_rows.len(),
        1,
        "unexpected request labels: {during:?}"
    );
    assert_eq!(inbox_label(request_rows[0]), Some(decoded.as_str()));
    assert_eq!(request_rows[0].2, 1.0);
    let control_samples: Vec<_> = during
        .iter()
        .filter(|sample| {
            sample.0 == "swarmail_http_requests_in_flight"
                && inbox_label(sample)
                    .is_some_and(|label| label.contains('\r') || label.contains('\0'))
        })
        .collect();
    assert_eq!(
        control_samples.len(),
        1,
        "unexpected control or injected request samples: {during:?}"
    );
    assert_eq!(inbox_label(control_samples[0]), Some(decoded.as_str()));
    assert_no_await_registration(&during, &aliases, "during the refused request");

    release
        .send(())
        .expect("the middleware is still waiting for release");
    let (status, body) = tokio::time::timeout(HTTP_DEADLINE, clients.join_next())
        .await
        .expect("the released refusal did not finish")
        .expect("the request client task remains owned")
        .expect("the request client task did not panic");
    assert_eq!(status, 400);
    assert_eq!(
        body,
        br#"{"error":"inbox must not contain control characters"}"#
    );

    let after = scrape(addr).await;
    assert_no_await_registration(&after, &aliases, "after the refusal");
    assert!(
        !after.iter().any(|sample| {
            sample.0 == "swarmail_http_requests_in_flight"
                && (inbox_label(sample) == Some(decoded.as_str())
                    || inbox_label(sample) == Some(raw.as_str()))
        }),
        "the request row survived guard release: {after:?}"
    );

    shutdown.send(true).expect("stop the test router state");
    graceful_stop.send(()).expect("stop the test listener");
    let joined = tokio::time::timeout(SERVER_DEADLINE, server.join_next())
        .await
        .expect("the test API server did not stop")
        .expect("the test API server task is owned");
    joined.expect("the test API server task did not panic");
}

#[tokio::test]
async fn cancelled_observers_and_dropped_release_senders_do_not_strand_the_hold() {
    let (hold, reached, release) = request_hold();
    drop(reached);
    tokio::time::timeout(HTTP_DEADLINE, hold.after_response(StatusCode::BAD_REQUEST))
        .await
        .expect("a cancelled observer must not strand the request");
    drop(release);

    let (hold, reached, release) = request_hold();
    let mut owner = JoinSet::new();
    owner.spawn(async move { hold.after_response(StatusCode::BAD_REQUEST).await });
    let status = tokio::time::timeout(HTTP_DEADLINE, reached)
        .await
        .expect("the hold did not report its response")
        .expect("the response observer was dropped");
    assert_eq!(status, StatusCode::BAD_REQUEST);
    drop(release);
    let joined = tokio::time::timeout(HTTP_DEADLINE, owner.join_next())
        .await
        .expect("a dropped release sender must free the hold")
        .expect("the hold owner task remains owned");
    joined.expect("the hold owner task did not panic");

    let (hold, reached, release) = request_hold();
    let mut owner = JoinSet::new();
    owner.spawn(async move { hold.after_response(StatusCode::BAD_REQUEST).await });
    let status = tokio::time::timeout(HTTP_DEADLINE, reached)
        .await
        .expect("the cancellable hold did not report its response")
        .expect("the response observer was dropped");
    assert_eq!(status, StatusCode::BAD_REQUEST);
    owner.abort_all();
    let joined = tokio::time::timeout(HTTP_DEADLINE, owner.join_next())
        .await
        .expect("the cancelled hold owner did not finish")
        .expect("the cancelled hold owner task remains owned");
    assert!(joined.is_err(), "the owner was explicitly cancelled");
    drop(release);
}
