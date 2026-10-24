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

#[derive(Clone, Default)]
struct HoldInstaller {
    slots: Arc<Mutex<std::collections::HashMap<String, RequestHold>>>,
}

impl HoldInstaller {
    async fn insert(&self, target: &str, hold: RequestHold) {
        self.slots.lock().await.insert(target.to_string(), hold);
    }
}

/// The outer test layer attaches a request-owned one-shot only to an exact
/// raw request URI. Metrics and unrelated requests continue through the real
/// production router. Taking the hold releases the map lock before dispatch.
async fn install_hold(
    State(installer): State<HoldInstaller>,
    mut request: Request,
    next: Next,
) -> Response {
    let target = request
        .uri()
        .path_and_query()
        .map(|path| path.as_str().to_string());
    let hold = if let Some(target) = target {
        installer.slots.lock().await.remove(&target)
    } else {
        None
    };
    if let Some(hold) = hold {
        request.extensions_mut().insert(hold);
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
        let mut lines = head.split("\r\n");
        let status = lines
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .expect("HTTP response has a status")
            .parse::<u16>()
            .expect("HTTP status is numeric");
        let mut content_lengths = Vec::new();
        let mut transfer_encoding = false;
        for line in lines {
            let (name, value) = line
                .split_once(':')
                .expect("HTTP response header has a colon");
            if name.eq_ignore_ascii_case("content-length") {
                content_lengths.push(
                    value
                        .trim()
                        .parse::<usize>()
                        .expect("HTTP Content-Length is numeric"),
                );
            }
            transfer_encoding |= name.eq_ignore_ascii_case("transfer-encoding");
        }
        assert!(
            !transfer_encoding,
            "the test reader requires unchunked HTTP"
        );
        assert_eq!(
            content_lengths.len(),
            1,
            "one Content-Length frames the body"
        );
        let body = response[boundary + 4..].to_vec();
        assert_eq!(
            body.len(),
            content_lengths[0],
            "the complete HTTP body matches Content-Length"
        );
        (status, body)
    })
    .await
    .expect("test API HTTP operation exceeded its deadline")
}

async fn scrape_bytes(addr: std::net::SocketAddr) -> Vec<u8> {
    let (status, body) = get(addr, "/metrics").await;
    assert_eq!(status, 200, "metrics response status");
    body
}

fn parse_scrape(body: &[u8]) -> Vec<metrics::Sample> {
    let text = std::str::from_utf8(body).expect("metrics body is UTF-8");
    metrics::parse_prometheus(text).unwrap_or_else(|error| panic!("{error}:\n{text}"))
}

async fn scrape(addr: std::net::SocketAddr) -> Vec<metrics::Sample> {
    parse_scrape(&scrape_bytes(addr).await)
}

fn lines_with_prefix<'a>(body: &'a [u8], prefix: &[u8]) -> Vec<&'a [u8]> {
    body.split_inclusive(|byte| *byte == b'\n')
        .filter(|line| line.windows(prefix.len()).any(|window| window == prefix))
        .collect()
}

fn samples_with_label_prefix<'a>(
    samples: &'a [metrics::Sample],
    prefix: &[u8],
) -> Vec<&'a metrics::Sample> {
    samples
        .iter()
        .filter(|sample| {
            sample
                .1
                .iter()
                .any(|(_, value)| value.as_bytes().starts_with(prefix))
        })
        .collect()
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
    let installer = HoldInstaller::default();
    installer.insert(&path, hold).await;
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
    let during_body = scrape_bytes(addr).await;
    let during = parse_scrape(&during_body);
    let expected_line = format!("swarmail_http_requests_in_flight{{inbox=\"{decoded}\"}} 1\n");
    let request_lines = lines_with_prefix(&during_body, prefix.as_bytes());
    assert_eq!(request_lines.len(), 1, "unexpected request wire rows");
    assert_eq!(request_lines[0], expected_line.as_bytes());
    let request_rows = samples_with_label_prefix(&during, prefix.as_bytes());
    assert_eq!(
        request_rows.len(),
        1,
        "unexpected request labels: {during:?}"
    );
    assert_eq!(request_rows[0].0, "swarmail_http_requests_in_flight");
    assert_eq!(
        request_rows[0].1,
        vec![(String::from("inbox"), decoded.clone())]
    );
    assert_eq!(request_rows[0].2, 1.0);
    assert!(
        !during
            .iter()
            .any(|sample| inbox_label(sample) == Some(raw.as_str())),
        "the percent-encoded alias appeared: {during:?}"
    );
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

    let after_body = scrape_bytes(addr).await;
    let after = parse_scrape(&after_body);
    assert_no_await_registration(&after, &aliases, "after the refusal");
    assert!(
        lines_with_prefix(&after_body, prefix.as_bytes()).is_empty(),
        "the request wire row survived guard release"
    );
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
async fn decoded_metric_labels_match_the_local_exposition_contract() {
    struct Fixture {
        id: String,
        raw_suffix: String,
        decoded_suffix: String,
        wire_suffix: String,
        admitted: bool,
    }

    let prefix = "issue202-label-conformance";
    let mut fixtures = vec![
        Fixture {
            id: "backslash".into(),
            raw_suffix: "%5Cend".into(),
            decoded_suffix: r"\end".into(),
            wire_suffix: r"\\end".into(),
            admitted: true,
        },
        Fixture {
            id: "quote".into(),
            raw_suffix: "a%22b".into(),
            decoded_suffix: "a\"b".into(),
            wire_suffix: r#"a\"b"#.into(),
            admitted: true,
        },
        Fixture {
            id: "literal-backslash-n".into(),
            raw_suffix: "%5Cn".into(),
            decoded_suffix: r"\n".into(),
            wire_suffix: r"\\n".into(),
            admitted: true,
        },
        Fixture {
            id: "line-feed".into(),
            raw_suffix: "%0A".into(),
            decoded_suffix: "\n".into(),
            wire_suffix: r"\n".into(),
            admitted: false,
        },
        Fixture {
            id: "carriage-return".into(),
            raw_suffix: "%0D".into(),
            decoded_suffix: "\r".into(),
            wire_suffix: "\r".into(),
            admitted: false,
        },
        Fixture {
            id: "nul".into(),
            raw_suffix: "%00".into(),
            decoded_suffix: "\0".into(),
            wire_suffix: "\0".into(),
            admitted: false,
        },
        Fixture {
            id: "cr-nul-pair".into(),
            raw_suffix: "%0D%00".into(),
            decoded_suffix: "\r\0".into(),
            wire_suffix: "\r\0".into(),
            admitted: false,
        },
        Fixture {
            id: "mixed".into(),
            raw_suffix: "mix%5Cslash%22quote%0A%E2%80%AEend".into(),
            decoded_suffix: "mix\\slash\"quote\n\u{202e}end".into(),
            wire_suffix: "mix\\\\slash\\\"quote\\n\u{202e}end".into(),
            admitted: false,
        },
    ];

    let c1 = [
        ("%C2%80", "\u{80}"),
        ("%C2%81", "\u{81}"),
        ("%C2%82", "\u{82}"),
        ("%C2%83", "\u{83}"),
        ("%C2%84", "\u{84}"),
        ("%C2%85", "\u{85}"),
        ("%C2%86", "\u{86}"),
        ("%C2%87", "\u{87}"),
        ("%C2%88", "\u{88}"),
        ("%C2%89", "\u{89}"),
        ("%C2%8A", "\u{8a}"),
        ("%C2%8B", "\u{8b}"),
        ("%C2%8C", "\u{8c}"),
        ("%C2%8D", "\u{8d}"),
        ("%C2%8E", "\u{8e}"),
        ("%C2%8F", "\u{8f}"),
        ("%C2%90", "\u{90}"),
        ("%C2%91", "\u{91}"),
        ("%C2%92", "\u{92}"),
        ("%C2%93", "\u{93}"),
        ("%C2%94", "\u{94}"),
        ("%C2%95", "\u{95}"),
        ("%C2%96", "\u{96}"),
        ("%C2%97", "\u{97}"),
        ("%C2%98", "\u{98}"),
        ("%C2%99", "\u{99}"),
        ("%C2%9A", "\u{9a}"),
        ("%C2%9B", "\u{9b}"),
        ("%C2%9C", "\u{9c}"),
        ("%C2%9D", "\u{9d}"),
        ("%C2%9E", "\u{9e}"),
        ("%C2%9F", "\u{9f}"),
    ];
    for (index, (raw, scalar)) in c1.into_iter().enumerate() {
        fixtures.push(Fixture {
            id: format!("c1-{:02x}", 0x80 + index),
            raw_suffix: raw.into(),
            decoded_suffix: scalar.into(),
            wire_suffix: scalar.into(),
            admitted: false,
        });
    }

    let bidi = [
        ("061c", "%D8%9C", "\u{061c}"),
        ("200e", "%E2%80%8E", "\u{200e}"),
        ("200f", "%E2%80%8F", "\u{200f}"),
        ("202a", "%E2%80%AA", "\u{202a}"),
        ("202b", "%E2%80%AB", "\u{202b}"),
        ("202c", "%E2%80%AC", "\u{202c}"),
        ("202d", "%E2%80%AD", "\u{202d}"),
        ("202e", "%E2%80%AE", "\u{202e}"),
        ("2066", "%E2%81%A6", "\u{2066}"),
        ("2067", "%E2%81%A7", "\u{2067}"),
        ("2068", "%E2%81%A8", "\u{2068}"),
        ("2069", "%E2%81%A9", "\u{2069}"),
    ];
    for (codepoint, raw, scalar) in bidi {
        fixtures.push(Fixture {
            id: format!("bidi-{codepoint}"),
            raw_suffix: raw.into(),
            decoded_suffix: scalar.into(),
            wire_suffix: scalar.into(),
            admitted: true,
        });
    }

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test API listener");
    let addr = listener.local_addr().expect("read test API address");
    let (state, shutdown) = app_state(addr);
    let installer = HoldInstaller::default();
    let test_router = router(state).layer(middleware::from_fn_with_state(
        installer.clone(),
        install_hold,
    ));
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

    let initial = scrape(addr).await;
    assert!(
        samples_with_label_prefix(&initial, prefix.as_bytes()).is_empty(),
        "exclusive conformance labels exist before the test: {initial:?}"
    );

    for fixture in fixtures {
        let attribution = format!("{prefix}-{}", fixture.id);
        let raw = format!("{attribution}{}", fixture.raw_suffix);
        let decoded = format!("{attribution}{}", fixture.decoded_suffix);
        let expected_wire_label = format!("{attribution}{}", fixture.wire_suffix);
        let path = format!("/api/v1/inboxes/{raw}/count");

        let before_body = scrape_bytes(addr).await;
        let before = parse_scrape(&before_body);
        assert!(
            lines_with_prefix(&before_body, attribution.as_bytes()).is_empty(),
            "fixture wire row exists before its request: {attribution:?}"
        );
        assert!(
            samples_with_label_prefix(&before, attribution.as_bytes()).is_empty(),
            "fixture sample exists before its request: {before:?}"
        );
        assert_no_await_registration(
            &before,
            &[raw.as_str(), decoded.as_str()],
            "before the count request",
        );

        let (hold, reached, release) = request_hold();
        installer.insert(&path, hold).await;
        let mut clients = JoinSet::new();
        let request_path = path.clone();
        clients.spawn(async move { get(addr, &request_path).await });
        let response_status = tokio::time::timeout(HTTP_DEADLINE, reached)
            .await
            .expect("count response never reached the accounting hold")
            .expect("request-owned hold was dropped before the count response");
        let expected_status = if fixture.admitted { 200 } else { 400 };
        assert_eq!(response_status.as_u16(), expected_status, "{attribution}");

        let during_body = scrape_bytes(addr).await;
        let during = parse_scrape(&during_body);
        let expected_line =
            format!("swarmail_http_requests_in_flight{{inbox=\"{expected_wire_label}\"}} 1\n");
        let wire_rows = lines_with_prefix(&during_body, attribution.as_bytes());
        assert_eq!(
            wire_rows.len(),
            1,
            "extra or missing wire rows: {attribution:?}"
        );
        assert_eq!(
            wire_rows[0],
            expected_line.as_bytes(),
            "unexpected exposition bytes for {attribution:?}"
        );
        let rows = samples_with_label_prefix(&during, attribution.as_bytes());
        assert_eq!(rows.len(), 1, "extra or missing parsed rows: {during:?}");
        assert_eq!(rows[0].0, "swarmail_http_requests_in_flight");
        assert_eq!(
            rows[0].1,
            vec![(String::from("inbox"), decoded.clone())],
            "strict reader must recover the exact decoded bytes"
        );
        assert_eq!(rows[0].1[0].1.as_bytes(), decoded.as_bytes());
        assert_eq!(rows[0].2, 1.0);
        assert!(
            !during
                .iter()
                .any(|sample| inbox_label(sample) == Some(raw.as_str())),
            "encoded label alias appeared: {during:?}"
        );
        assert_no_await_registration(
            &during,
            &[raw.as_str(), decoded.as_str()],
            "during the count request",
        );

        release.send(()).expect("middleware is waiting for release");
        let (status, body) = tokio::time::timeout(HTTP_DEADLINE, clients.join_next())
            .await
            .expect("released count request did not finish")
            .expect("request client task remains owned")
            .expect("request client task did not panic");
        assert_eq!(status, expected_status, "{attribution}");
        if fixture.admitted {
            let response: serde_json::Value =
                serde_json::from_slice(&body).expect("admitted count returns JSON");
            assert_eq!(
                response,
                serde_json::json!({"count": 0, "inbox": decoded}),
                "count response retains decoded inbox identity"
            );
        } else {
            assert_eq!(
                body, br#"{"error":"inbox must not contain control characters"}"#,
                "refusal body is unchanged for {attribution:?}"
            );
        }

        let after_body = scrape_bytes(addr).await;
        let after = parse_scrape(&after_body);
        assert!(
            lines_with_prefix(&after_body, attribution.as_bytes()).is_empty(),
            "released request wire row survived: {attribution:?}"
        );
        assert!(
            samples_with_label_prefix(&after, attribution.as_bytes()).is_empty(),
            "released request sample survived: {after:?}"
        );
        assert_no_await_registration(
            &after,
            &[raw.as_str(), decoded.as_str()],
            "after the count request",
        );
    }

    shutdown.send(true).expect("stop the test router state");
    graceful_stop.send(()).expect("stop the test listener");
    let joined = tokio::time::timeout(SERVER_DEADLINE, server.join_next())
        .await
        .expect("test API server did not stop")
        .expect("test API server task is owned");
    joined.expect("test API server task did not panic");
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
