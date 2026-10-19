//! Webhook delivery: the happy path (including a pathless target URL), the
//! retry budget against a target that always refuses — over plain HTTP and
//! over TLS (reqwest + rustls, verification on) — and the redirect contract:
//! a 3xx target takes the secret nowhere. Plus the bounded per-PEM client
//! cache: more distinct roots than the cap still deliver, every one of them.
//!
//! The sinks here are real servers, and they are built to outlive the
//! dispatcher's whole retry budget: one transient attempt failure must never
//! cost a delivery. (The 2026-10-09 hermetic red of the cap test was exactly
//! that — a sink whose `accept()` hit EMFILE under the sandbox's fd pressure
//! ended after one connection and silently unbound, so the retried delivery
//! was refused for good and the test asserted on an empty capture.) A sink
//! serves every connection in its own task, re-arms a failed accept, keeps
//! its listener bound for [`SINK_LIFETIME`], and claims its capture channel
//! only for an exchange that actually completed.

mod common;

use common::{
    http_get_text, http_json, metric_now, smtp_send, start, start_with, wait_for_metric_from,
};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::oneshot::Receiver;
use tokio::sync::watch;

/// How long a test sink keeps its listener bound and serving: the whole
/// dispatcher retry budget (`deliver`'s four attempts under the 5s
/// POST_TIMEOUT plus the 100/400/1600ms backoff is ~22.1s) plus slack. A
/// sink that unbinds earlier converts one transient attempt failure into a
/// permanently lost delivery — the exact mechanism of the 2026-10-09
/// hermetic red of the cap test, where an EMFILE accept under the sandbox's
/// fd pressure ended a sink after a single connection and every retry was
/// refused.
///
/// The budget tests bind their sinks for this lifetime too — it outlives
/// any budget a loop could spend (a mutant fifth attempt fires at
/// ~+8.5 s) — and then never wait on it: the test ends long before the
/// lifetime does, and the budget itself is observed by signal (issue
/// #102), not by the sink's clock.
const SINK_LIFETIME: Duration = Duration::from_secs(25);

/// Pause before re-arming a failed accept. EMFILE is transient — on Linux
/// the connection is still sitting in the accept queue — so a short retry
/// serves it instead of losing it.
const ACCEPT_RETRY: Duration = Duration::from_millis(10);

/// The live hit counter every sink reports beside its channels: the number
/// of connections accepted so far, readable at any moment. The budget tests
/// read it only *after* the server's own `swarmail_webhook_exhausted_total`
/// row has risen over its pre-mail baseline (issue #102) — by then the
/// dispatcher has spent its whole budget and made its final await, so the
/// counter is final and a mutant fifth attempt is *seen* rather than
/// landing after some window has closed.
type Hits = Arc<AtomicUsize>;

/// A sink's hits, read once the exhaustion signal has fired.
fn hits(hits: &Hits) -> usize {
    hits.load(std::sync::atomic::Ordering::Relaxed)
}

/// The first *completed* exchange claims the capture channel; every later
/// exchange only gets a reply. A connection that dies mid-request captures
/// nothing, so the capture a test asserts on is always a real one. A plain
/// mutex, never an async one: the critical section does no awaiting.
type Capture<T> = std::sync::Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<T>>>>;

/// An HTTP sink that replies `status` to every connection for its whole
/// `window`, and reports the first completed request head plus the final
/// observed hit count. Each connection is served in its own task — one
/// stalled exchange can never block the next accept — and an accept error
/// is retried, never silently fatal.
///
/// The count channel resolves **only at window close**, with the final
/// observed count (issue #51): an early fire at the `expect`-th connection
/// would resolve every "4 attempts, no more" assertion *before* the window
/// it claims to cover, and a fifth attempt would be unobservable. Awaiting
/// `rx_count` is the readiness signal for "the window has closed on
/// everything that will ever arrive".
///
/// The returned [`Hits`] counter is the same count, live: incremented at
/// accept, readable before the window closes (issue #102). Reading it is
/// sound after a monotonic counter on `/metrics` has signalled that the
/// dispatcher is done — never instead of one.
async fn http_sink(
    status: u16,
    window: Duration,
) -> (
    std::net::SocketAddr,
    tokio::sync::oneshot::Receiver<String>,
    tokio::sync::oneshot::Receiver<usize>,
    Hits,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx_head, rx_head) = tokio::sync::oneshot::channel();
    let (tx_count, rx_count) = tokio::sync::oneshot::channel();
    let hits: Hits = Arc::new(AtomicUsize::new(0));
    let reply = format!("HTTP/1.1 {status} TEST\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    let capture: Capture<String> = std::sync::Arc::new(std::sync::Mutex::new(Some(tx_head)));
    let sink_hits = hits.clone();
    tokio::spawn(async move {
        let deadline = std::time::Instant::now() + window;
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            if left.is_zero() {
                break;
            }
            match tokio::time::timeout(left, listener.accept()).await {
                Ok(Ok((stream, _))) => {
                    sink_hits.fetch_add(1, Ordering::Relaxed);
                    let capture = capture.clone();
                    let reply = reply.clone();
                    tokio::spawn(async move {
                        let mut stream = stream;
                        // The whole request (head + Content-Length body), so a
                        // request split across TCP segments is still one
                        // exchange.
                        let mut buf = Vec::new();
                        let mut chunk = [0u8; 4096];
                        loop {
                            let n = stream.read(&mut chunk).await.unwrap_or(0);
                            if n == 0 {
                                break;
                            }
                            buf.extend_from_slice(&chunk[..n]);
                            if content_length(&buf).is_some_and(|len| buf.len() >= len) {
                                break;
                            }
                        }
                        if content_length(&buf).is_some() {
                            let mut slot = capture.lock().unwrap();
                            if let Some(tx) = slot.take() {
                                let _ = tx.send(String::from_utf8_lossy(&buf).to_string());
                            }
                        }
                        let _ = stream.write_all(reply.as_bytes()).await;
                    });
                }
                Ok(Err(e)) => {
                    eprintln!("http sink accept error, retrying: {e}");
                    tokio::time::sleep(ACCEPT_RETRY).await;
                }
                Err(_) => break, // the window is over
            }
        }
        // The window closed: report what the sink actually saw — the count
        // is observed, never echoed from a hope.
        let _ = tx_count.send(sink_hits.load(Ordering::Relaxed));
    });
    (addr, rx_head, rx_count, hits)
}

/// What the TLS sink saw: the request head and the negotiated TLS version.
#[derive(Debug)]
struct Captured {
    head: String,
    tls: String,
}

/// A TLS sink: mints nothing, just serves the given certificate. It replies
/// `reply` to every connection for its whole `window` — through the
/// dispatcher's entire retry budget when a budget test asks, so a failed
/// attempt's retry is still served — and reports the first completed
/// exchange's request head with its negotiated TLS version. A failed
/// handshake (an untrusted client rejecting our cert) still counts as an
/// attempt. The count channel resolves only at window close, with the final
/// observed count (issue #51). Each connection is served in its own task —
/// one stalled or partial exchange can never block the next accept — and an
/// accept error is retried, never silently fatal.
async fn tls_sink(
    reply: &'static [u8],
    cert_der: Vec<u8>,
    key_der: Vec<u8>,
    window: Duration,
) -> (SocketAddr, Receiver<Captured>, Receiver<usize>, Hits) {
    tls_sink_cfg(reply, cert_der, key_der, window, 0).await
}

/// [`tls_sink`] with the first `drop_first` accepted connections dropped
/// unanswered — a deterministic attempt failure below the HTTP layer, for the
/// retry proof. The [`Hits`] counter is live, as on every sink.
async fn tls_sink_cfg(
    reply: &'static [u8],
    cert_der: Vec<u8>,
    key_der: Vec<u8>,
    window: Duration,
    drop_first: usize,
) -> (SocketAddr, Receiver<Captured>, Receiver<usize>, Hits) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let key = tokio_rustls::rustls::pki_types::PrivateKeyDer::Pkcs8(
        tokio_rustls::rustls::pki_types::PrivatePkcs8KeyDer::from(key_der),
    );
    let config = tokio_rustls::rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            vec![tokio_rustls::rustls::pki_types::CertificateDer::from(
                cert_der,
            )],
            key,
        )
        .unwrap();
    let acceptor = tokio_rustls::TlsAcceptor::from(std::sync::Arc::new(config));
    let (tx_req, rx_req) = tokio::sync::oneshot::channel();
    let (tx_count, rx_count) = tokio::sync::oneshot::channel();
    let hits: Hits = Arc::new(AtomicUsize::new(0));
    let capture: Capture<Captured> = std::sync::Arc::new(std::sync::Mutex::new(Some(tx_req)));
    let sink_hits = hits.clone();
    tokio::spawn(async move {
        let deadline = std::time::Instant::now() + window;
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            if left.is_zero() {
                break;
            }
            match tokio::time::timeout(left, listener.accept()).await {
                Ok(Ok((stream, _))) => {
                    let ordinal = sink_hits.fetch_add(1, Ordering::Relaxed) + 1;
                    let acceptor = acceptor.clone();
                    let capture = capture.clone();
                    tokio::spawn(async move {
                        if ordinal <= drop_first {
                            return; // dropped unanswered: attempt counted, nothing captured
                        }
                        let Ok(mut tls) = acceptor.accept(stream).await else {
                            return; // the client rejected our certificate: attempt counted
                        };
                        // Read until the whole request (head + Content-Length
                        // body) is in — the capture stays with the first
                        // exchange that actually completed.
                        let mut buf = Vec::new();
                        let mut chunk = [0u8; 4096];
                        loop {
                            let n = tls.read(&mut chunk).await.unwrap_or(0);
                            if n == 0 {
                                break;
                            }
                            buf.extend_from_slice(&chunk[..n]);
                            if content_length(&buf).is_some_and(|len| buf.len() >= len) {
                                break;
                            }
                        }
                        if content_length(&buf).is_some() {
                            let mut slot = capture.lock().unwrap();
                            if let Some(tx) = slot.take() {
                                let _ = tx.send(Captured {
                                    head: String::from_utf8_lossy(&buf).to_string(),
                                    tls: format!("{:?}", tls.get_ref().1.protocol_version()),
                                });
                            }
                        }
                        let _ = tls.write_all(reply).await;
                    });
                }
                Ok(Err(e)) => {
                    eprintln!("tls sink accept error, retrying: {e}");
                    tokio::time::sleep(ACCEPT_RETRY).await;
                }
                Err(_) => break, // the window is over
            }
        }
        // The window closed: the count is observed, never echoed.
        let _ = tx_count.send(sink_hits.load(Ordering::Relaxed));
    });
    (addr, rx_req, rx_count, hits)
}

/// The Content-Length of a request head already fully buffered, if any.
fn content_length(buf: &[u8]) -> Option<usize> {
    let head_end = buf.windows(4).position(|w| w == b"\r\n\r\n")? + 4;
    let head = String::from_utf8_lossy(&buf[..head_end]);
    let len: usize = head
        .lines()
        .find_map(|l| {
            let (name, value) = l.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().ok())?
        })
        .unwrap_or(0);
    Some(head_end + len)
}

/// A redirecting "target": the attacker's endpoint. Answers every connection
/// with `status` and an absolute `Location` header pointing wherever it
/// likes, for the sink's whole `window`, and reports the first completed
/// request (head and body) the moment it arrives plus the final observed
/// hit count (resolved only at window close, issue #51), with the same live
/// [`Hits`] counter as every other sink. Each connection is
/// served in its own task; an accept error is retried, never silently fatal.
async fn redirect_sink(
    status: u16,
    location: String,
    window: Duration,
) -> (SocketAddr, Receiver<String>, Receiver<usize>, Hits) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx_first, rx_first) = tokio::sync::oneshot::channel();
    let (tx_count, rx_count) = tokio::sync::oneshot::channel();
    let hits: Hits = Arc::new(AtomicUsize::new(0));
    let reason = match status {
        301 => "Moved Permanently",
        302 => "Found",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        _ => "Redirect",
    };
    let reply = format!(
        "HTTP/1.1 {status} {reason}\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    let capture: Capture<String> = std::sync::Arc::new(std::sync::Mutex::new(Some(tx_first)));
    let sink_hits = hits.clone();
    tokio::spawn(async move {
        let deadline = std::time::Instant::now() + window;
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            if left.is_zero() {
                break;
            }
            match tokio::time::timeout(left, listener.accept()).await {
                Ok(Ok((stream, _))) => {
                    sink_hits.fetch_add(1, Ordering::Relaxed);
                    let capture = capture.clone();
                    let reply = reply.clone();
                    tokio::spawn(async move {
                        let mut stream = stream;
                        // Read the whole request (head + Content-Length body).
                        let mut buf = Vec::new();
                        let mut chunk = [0u8; 4096];
                        loop {
                            let n = stream.read(&mut chunk).await.unwrap_or(0);
                            if n == 0 {
                                break;
                            }
                            buf.extend_from_slice(&chunk[..n]);
                            if content_length(&buf).is_some_and(|len| buf.len() >= len) {
                                break;
                            }
                        }
                        if content_length(&buf).is_some() {
                            let mut slot = capture.lock().unwrap();
                            if let Some(tx) = slot.take() {
                                let _ = tx.send(String::from_utf8_lossy(&buf).to_string());
                            }
                        }
                        let _ = stream.write_all(reply.as_bytes()).await;
                    });
                }
                Ok(Err(e)) => {
                    eprintln!("redirect sink accept error, retrying: {e}");
                    tokio::time::sleep(ACCEPT_RETRY).await;
                }
                Err(_) => break, // the window is over
            }
        }
        // The window closed: the count is observed, never echoed.
        let _ = tx_count.send(sink_hits.load(Ordering::Relaxed));
    });
    (addr, rx_first, rx_count, hits)
}

/// The redirect destination: records the head of every request that arrives
/// until `stop` flips, then reports them. An empty report is the no-leak
/// proof; the listener is bound before the target is registered so a leak
/// can never race past it.
async fn destination_sink(stop: watch::Receiver<bool>) -> (SocketAddr, Receiver<Vec<String>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let mut stop = stop;
        let mut captured = Vec::new();
        loop {
            tokio::select! {
                _ = stop.changed() => break,
                accepted = listener.accept() => {
                    let Ok((mut stream, _)) = accepted else { break };
                    let mut buf = vec![0u8; 8192];
                    let n = stream.read(&mut buf).await.unwrap_or(0);
                    captured.push(String::from_utf8_lossy(&buf[..n]).to_string());
                }
            }
        }
        let _ = tx.send(captured);
    });
    (addr, rx)
}

/// A CA and a leaf cert for 127.0.0.1, minted per test: (ca_pem, leaf_der,
/// leaf_key_der). Real PKI, real verification — the client trusts the CA
/// only because the target hands it over as `ca_pem`.
fn mint_cert() -> (String, Vec<u8>, Vec<u8>) {
    let ca_key = rcgen::KeyPair::generate().unwrap();
    let mut ca_params = rcgen::CertificateParams::new(Vec::<String>::new()).unwrap();
    ca_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    ca_params.key_usages = vec![rcgen::KeyUsagePurpose::KeyCertSign];
    let ca = ca_params.self_signed(&ca_key).unwrap();

    let leaf_key = rcgen::KeyPair::generate().unwrap();
    let mut leaf_params = rcgen::CertificateParams::new(Vec::<String>::new()).unwrap();
    leaf_params
        .subject_alt_names
        .push(rcgen::SanType::IpAddress(IpAddr::from([127, 0, 0, 1])));
    leaf_params.key_usages = vec![rcgen::KeyUsagePurpose::DigitalSignature];
    let leaf = leaf_params.signed_by(&leaf_key, &ca, &ca_key).unwrap();

    (ca.pem(), leaf.der().to_vec(), leaf_key.serialize_der())
}

#[tokio::test]
async fn delivered_to_a_pathless_target_with_secret() {
    let srv = start().await;
    let (addr, rx, _, _hits) = http_sink(204, SINK_LIFETIME).await;
    let targets = format!(r#"[{{"url": "http://{addr}", "secret": "topsecret"}}]"#);
    let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200);

    smtp_send(srv.smtp_addr, None, "f@x.io", "r@x.io", "hooked", "x")
        .await
        .unwrap();

    let head = tokio::time::timeout(Duration::from_secs(5), rx)
        .await
        .expect("webhook delivery timed out")
        .unwrap();
    assert!(head.starts_with("POST / HTTP/1.1"), "{head}");
    // hyper lowercases wire header names; HTTP/1.1 names are case-insensitive.
    assert!(
        head.to_ascii_lowercase()
            .contains("x-swarmail-secret: topsecret"),
        "{head}"
    );
    assert!(head.contains("\"event\":\"received\""), "{head}");
}

/// The webhook payload carries the stored, capped codes (decision 0049 §4,
/// issue #139): a mail with 70 distinct codes is delivered with exactly the
/// first 64 — the same array REST serves — never the uncapped scan. Red at
/// 2a325c9: all 70 rode the payload.
#[tokio::test]
async fn the_webhook_payload_carries_the_capped_codes() {
    let srv = start().await;
    let (addr, rx, _, _hits) = http_sink(204, SINK_LIFETIME).await;
    let targets = format!(r#"[{{"url": "http://{addr}/hook"}}]"#);
    let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200);
    let numbers: Vec<String> = (0..70).map(|n| (3_000_000 + n).to_string()).collect();
    smtp_send(
        srv.smtp_addr,
        Some("capped"),
        "f@x.io",
        "r@x.io",
        "codes",
        &numbers.join(" "),
    )
    .await
    .unwrap();
    let request = tokio::time::timeout(Duration::from_secs(5), rx)
        .await
        .expect("webhook delivery timed out")
        .unwrap();
    let payload: serde_json::Value = serde_json::from_str(payload_of(&request)).unwrap();
    assert_eq!(payload["email"]["codes"], serde_json::json!(numbers[..64]));
    let (st, stored) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/capped/messages",
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(stored["emails"][0]["codes"], payload["email"]["codes"]);
}

#[tokio::test]
async fn failing_target_is_retried_four_times_not_more() {
    let srv = start().await;
    // The sink refuses everything: the dispatcher must try exactly the
    // four-attempt budget (deliver's 0..4) and then drop with a warning.
    // The window outlives any budget the loop could ever spend — a mutant
    // fifth attempt fires at ~+8.5 s — and the test never waits on it:
    // the budget is observed by signal below.
    let (addr, _, _count, sink_hits) = http_sink(500, SINK_LIFETIME).await;
    let targets = format!(r#"[{{"url": "http://{addr}/hooks", "inbox": "retrybox"}}]"#);
    let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200);

    // A mail for another inbox must never reach the failing target: the hit
    // count below would exceed 4 if the inbox filter leaked.
    smtp_send(srv.smtp_addr, None, "f@x.io", "other@x.io", "filtered", "x")
        .await
        .unwrap();

    // The exhaustion baseline is taken before the mail that will exhaust
    // its budget: only a NEW exhaustion satisfies the poll below (issue #51).
    let exhausted_base = metric_now(srv.http_addr, "swarmail_webhook_exhausted_total").await;
    smtp_send(
        srv.smtp_addr,
        Some("retrybox"),
        "f@x.io",
        "r@x.io",
        "retried",
        "x",
    )
    .await
    .unwrap();

    // Budget by signal, never by clock (issue #102): the hit count is read
    // only after the server's own rendered row says the budget is spent.
    // The dispatcher's counters increment after its last await, so this
    // poll resolves strictly after every connection the dispatcher will
    // ever make — a fifth attempt is *seen* by the sink, not lost past a
    // closing window.
    wait_for_metric_from(
        srv.http_addr,
        "swarmail_webhook_exhausted_total",
        exhausted_base,
        1,
    )
    .await;
    let seen = hits(&sink_hits);
    assert_eq!(seen, 4, "expected the 4-attempt retry budget, no more");
}

#[tokio::test]
async fn https_target_is_delivered_over_tls() {
    let srv = start().await;
    let (ca_pem, leaf, leaf_key) = mint_cert();
    let (addr, rx, _, _hits) = tls_sink(
        b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
        leaf,
        leaf_key,
        SINK_LIFETIME,
    )
    .await;

    // The target trusts the test CA through its own ca_pem; verification
    // stays on — an https URL without a trusted root must fail (see the
    // handshake test below), so reaching the sink here IS the TLS proof.
    let targets = serde_json::json!([{
        "url": format!("https://127.0.0.1:{}/hook", addr.port()),
        "secret": "topsecret",
        "ca_pem": ca_pem,
    }])
    .to_string();
    let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200);

    smtp_send(srv.smtp_addr, None, "f@x.io", "r@x.io", "hooked", "x")
        .await
        .unwrap();

    let captured = tokio::time::timeout(Duration::from_secs(5), rx)
        .await
        .expect("https webhook delivery timed out")
        .unwrap();
    assert!(
        captured.tls.contains("TLSv1"),
        "not negotiated over TLS: {:?}",
        captured
    );
    assert!(
        captured.head.starts_with("POST /hook HTTP/1.1"),
        "{captured:?}"
    );
    let head = captured.head.to_ascii_lowercase();
    assert!(head.contains("x-swarmail-secret: topsecret"), "{head}");
    assert!(head.contains("content-type: application/json"), "{head}");
    assert!(
        captured.head.contains("\"event\":\"received\""),
        "{captured:?}"
    );
}

#[tokio::test]
async fn untrusted_certificate_fails_the_handshake_and_is_retried() {
    let srv = start().await;
    // The sink serves a self-signed cert the target does not trust: every
    // attempt dies in the handshake, and the dispatcher must make exactly
    // the four-attempt budget of connections.
    let untrusted = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
    // The window outlives any budget the loop could spend; the count is
    // observed by signal below, never by the sink's clock (issue #102).
    let (addr, _, _count, sink_hits) = tls_sink(
        b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
        untrusted.cert.der().to_vec(),
        untrusted.key_pair.serialize_der(),
        SINK_LIFETIME,
    )
    .await;
    let targets = format!(r#"[{{"url": "https://127.0.0.1:{}/hook"}}]"#, addr.port());
    let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200);

    // Baseline before the mail; only a NEW exhaustion resolves the poll.
    let exhausted_base = metric_now(srv.http_addr, "swarmail_webhook_exhausted_total").await;
    smtp_send(srv.smtp_addr, None, "f@x.io", "r@x.io", "distrusted", "x")
        .await
        .unwrap();

    // The budget is spent when the server's own row says so (issue #102);
    // the read after it sees every handshake attempt, a fifth included.
    wait_for_metric_from(
        srv.http_addr,
        "swarmail_webhook_exhausted_total",
        exhausted_base,
        1,
    )
    .await;
    let seen = hits(&sink_hits);
    assert_eq!(seen, 4, "expected 4 handshake attempts, no more");
}

#[tokio::test]
async fn non_2xx_over_tls_is_retried_four_times_not_more() {
    let srv = start().await;
    let (ca_pem, leaf, leaf_key) = mint_cert();
    // Every handshake succeeds and the sink answers 500 to each request: a
    // non-2xx received over an established TLS session must cost exactly the
    // same four-attempt budget as a non-2xx over plain http — TLS may not
    // change the failure semantics, only the transport.
    // The window outlives any budget the loop could spend (issue #102).
    let (addr, _, _count, sink_hits) = tls_sink(
        b"HTTP/1.1 500 Internal Server Error\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
        leaf,
        leaf_key,
        SINK_LIFETIME,
    )
    .await;
    let targets = serde_json::json!([{
        "url": format!("https://127.0.0.1:{}/hook", addr.port()),
        "ca_pem": ca_pem,
    }])
    .to_string();
    let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200);

    // Baseline before the mail; only a NEW exhaustion resolves the poll.
    let exhausted_base = metric_now(srv.http_addr, "swarmail_webhook_exhausted_total").await;
    smtp_send(srv.smtp_addr, None, "f@x.io", "r@x.io", "servererror", "x")
        .await
        .unwrap();

    // The budget is spent when the server's own row says so (issue #102).
    wait_for_metric_from(
        srv.http_addr,
        "swarmail_webhook_exhausted_total",
        exhausted_base,
        1,
    )
    .await;
    let seen = hits(&sink_hits);
    assert_eq!(seen, 4, "expected 4 attempts over TLS, no more");
}

#[tokio::test]
async fn refused_connection_never_blocks_the_smtp_path() {
    let srv = start().await;
    // A port with nothing behind it: every attempt is refused on connect.
    let dead = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = dead.local_addr().unwrap();
    drop(dead);

    let targets = format!(r#"[{{"url": "https://{addr}/hook"}}]"#);
    let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200);

    // The exhaustion baseline is taken before the mail that will exhaust
    // its budget: only a NEW exhaustion satisfies the poll below (issue #51).
    let exhausted_base = metric_now(srv.http_addr, "swarmail_webhook_exhausted_total").await;
    // Acceptance must not wait on the (failing) delivery: 250 comes back
    // immediately and the mail is queryable while the retries still run.
    smtp_send(srv.smtp_addr, None, "f@x.io", "r@x.io", "refused", "x")
        .await
        .unwrap();
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(
        body["count"], 1,
        "the mail must be stored despite the dead target"
    );

    // The dispatcher keeps retrying in the background without disturbing
    // the store: the retry budget elapses — observed, not slept for, as one
    // new exhaustion over the pre-mail baseline — and the mail is still
    // queryable.
    wait_for_metric_from(
        srv.http_addr,
        "swarmail_webhook_exhausted_total",
        exhausted_base,
        1,
    )
    .await;
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["count"], 1, "retries must not touch the store");
}

/// The redirect e2e: a target that answers 302 Found with an absolute
/// Location at a second real local sink. The delivery client follows no
/// redirect (Policy::none()), so the destination records nothing at all,
/// while the explicit target records the full-fidelity POST — secret and
/// body included — and the 302 costs exactly the ordinary retry budget.
#[tokio::test]
async fn redirect_302_never_leaks_the_secret_to_the_destination() {
    let srv = start().await;
    // The destination is bound FIRST so its port is known and no leak can
    // ever race past the listener that would catch it.
    let (tx_stop, rx_stop) = watch::channel(false);
    let (dest, rx_leaked) = destination_sink(rx_stop).await;
    // The window outlives any budget the loop could spend (issue #102).
    let (addr, rx_first, _count, sink_hits) =
        redirect_sink(302, format!("http://{dest}/stolen"), SINK_LIFETIME).await;
    let targets = format!(r#"[{{"url": "http://{addr}/hook", "secret": "topsecret"}}]"#);
    let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200);

    // Baseline before the mail; only a NEW exhaustion resolves the poll.
    let exhausted_base = metric_now(srv.http_addr, "swarmail_webhook_exhausted_total").await;
    smtp_send(srv.smtp_addr, None, "f@x.io", "r@x.io", "redirected", "x")
        .await
        .unwrap();

    // Acceptance is never delayed by the redirecting target: the 250 has
    // returned and the mail is queryable while the retries still run.
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["count"], 1, "the mail must be stored despite the 302");

    // Non-vacuousness: the explicit target itself got the real delivery —
    // the secret rides along exactly as on a non-redirecting target.
    let head = tokio::time::timeout(Duration::from_secs(5), rx_first)
        .await
        .expect("the redirect target never saw the delivery")
        .unwrap();
    assert!(head.starts_with("POST /hook HTTP/1.1"), "{head}");
    assert!(
        head.to_ascii_lowercase()
            .contains("x-swarmail-secret: topsecret"),
        "{head}"
    );
    assert!(head.contains("\"event\":\"received\""), "{head}");

    // A redirect-following client would land on the destination within
    // microseconds of the 302, and again on every retry — the destination
    // is asked only after the budget is spent (below), so nothing it could
    // have received is still in flight. Budget by signal, never by clock
    // (issue #102): the exhaustion row is the proof point that every
    // attempt has been served, and the hit count read after it sees a
    // mutant fifth attempt too.
    wait_for_metric_from(
        srv.http_addr,
        "swarmail_webhook_exhausted_total",
        exhausted_base,
        1,
    )
    .await;
    let seen = hits(&sink_hits);
    tx_stop.send(true).unwrap();
    let leaked = tokio::time::timeout(Duration::from_secs(5), rx_leaked)
        .await
        .expect("the destination sink never stopped")
        .unwrap();
    assert!(
        leaked.is_empty(),
        "the redirect destination received: {leaked:?}"
    );

    // The redirect itself is an ordinary non-2xx: exactly the budget.
    assert_eq!(seen, 4, "a 302 must cost exactly the retry budget, no more");

    // And the store was untouched by all of it.
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["count"], 1, "retries must not touch the store");
}

/// The 307 shape: 307 and 308 share reqwest's body-preserving semantics, so
/// a body-forwarding redirect would hand the destination the whole POST —
/// secret header and `{"event":"received"}` body. The client follows
/// nothing: the destination records nothing, the explicit target records
/// the full POST, the budget is unchanged.
#[tokio::test]
async fn redirect_307_never_leaks_the_secret_or_body_to_the_destination() {
    let srv = start().await;
    let (tx_stop, rx_stop) = watch::channel(false);
    let (dest, rx_leaked) = destination_sink(rx_stop).await;
    // The window outlives any budget the loop could spend (issue #102).
    let (addr, rx_first, _count, sink_hits) =
        redirect_sink(307, format!("http://{dest}/stolen"), SINK_LIFETIME).await;
    let targets = format!(r#"[{{"url": "http://{addr}/hook", "secret": "topsecret"}}]"#);
    let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200);

    // Baseline before the mail; only a NEW exhaustion resolves the poll.
    let exhausted_base = metric_now(srv.http_addr, "swarmail_webhook_exhausted_total").await;
    smtp_send(
        srv.smtp_addr,
        None,
        "f@x.io",
        "r@x.io",
        "redirected307",
        "x",
    )
    .await
    .unwrap();

    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["count"], 1, "the mail must be stored despite the 307");

    let head = tokio::time::timeout(Duration::from_secs(5), rx_first)
        .await
        .expect("the redirect target never saw the delivery")
        .unwrap();
    assert!(head.starts_with("POST /hook HTTP/1.1"), "{head}");
    assert!(
        head.to_ascii_lowercase()
            .contains("x-swarmail-secret: topsecret"),
        "{head}"
    );
    assert!(head.contains("\"event\":\"received\""), "{head}");

    // Budget by signal, never by clock (issue #102): the count is read
    // after the exhaustion row has risen, so it is the final total — and
    // the final beat before the destination is asked what it saw.
    wait_for_metric_from(
        srv.http_addr,
        "swarmail_webhook_exhausted_total",
        exhausted_base,
        1,
    )
    .await;
    let seen = hits(&sink_hits);
    tx_stop.send(true).unwrap();
    let leaked = tokio::time::timeout(Duration::from_secs(5), rx_leaked)
        .await
        .expect("the destination sink never stopped")
        .unwrap();
    assert!(
        leaked.is_empty(),
        "the redirect destination received: {leaked:?}"
    );
    assert_eq!(seen, 4, "a 307 must cost exactly the retry budget, no more");

    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["count"], 1, "retries must not touch the store");
}

/// The value of a Prometheus metric line from a `/metrics` body.
fn metric(text: &str, name: &str) -> u64 {
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let mut parts = line.split_whitespace();
        if parts.next() == Some(name) {
            return parts
                .next()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(|| panic!("metric {name} carries no value:\n{text}"));
        }
    }
    panic!("metrics missing {name}:\n{text}");
}

/// The per-PEM client cache is bounded: one more distinct root than the cap
/// forces evictions, and every delivery still succeeds — an evicted root's
/// client is rebuilt on its next delivery, never a failed delivery. 33 real
/// TLS servers, each with its own freshly minted CA, are fanned out in two
/// waves (the delivery bound is 32 in flight, and a wave that fit under it
/// would not prove the point), so the dispatcher must cycle the whole cache
/// and rebuild on the way. On the old unbounded cache this is red:
/// `/metrics` carried no ca_cache counters, and nothing kept `entries` at
/// the cap.
#[tokio::test]
async fn deliveries_through_more_pems_than_the_cache_cap_all_succeed() {
    let srv = start().await;
    // Cap + 1 distinct roots: eviction is unavoidable, correctness is not
    // optional.
    let count = swarmail::webhook::CA_CLIENT_CAP + 1;

    // A real TLS server per root: its own CA, a leaf for 127.0.0.1, one
    // 204 per connection. Bound them all before registering any target.
    let mut sinks = Vec::new();
    for _ in 0..count {
        let (ca_pem, leaf, leaf_key) = mint_cert();
        let (addr, rx, _, _hits) = tls_sink(
            b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
            leaf,
            leaf_key,
            SINK_LIFETIME,
        )
        .await;
        sinks.push((ca_pem, addr, rx));
    }

    // The fan-out is bounded at WEBHOOK_DELIVERY_CAP deliveries in flight,
    // so the 33 targets go in two waves, each under the bound: one accepted
    // mail per wave, every target of the wave served by that pass.
    let wave_size = swarmail::webhook::WEBHOOK_DELIVERY_CAP / 2 + 1;
    for (w, wave) in sinks.chunks_mut(wave_size).enumerate() {
        let targets: Vec<serde_json::Value> = wave
            .iter()
            .map(|(ca_pem, addr, _)| {
                serde_json::json!({
                    "url": format!("https://127.0.0.1:{}/hook", addr.port()),
                    "ca_pem": ca_pem,
                })
            })
            .collect();
        let body = serde_json::to_string(&targets).unwrap();
        let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&body)).await;
        assert_eq!(st, 200);

        smtp_send(srv.smtp_addr, None, "f@x.io", "r@x.io", "cachebound", "x")
            .await
            .unwrap();

        // Every one of this wave's deliveries must have happened over real
        // TLS: an empty `tls` capture would mean its handshake was rejected.
        // The bound is generous — the dispatcher's whole 4-attempt budget
        // plus slack — but a generous bound alone is not the fix: the sinks
        // above now stay bound through that same budget, so a transient
        // attempt failure (an EMFILE accept under a starved sandbox, a
        // handshake that dies mid-flight) is retried against a live
        // listener instead of being lost with its sink.
        for (i, (_, _, rx)) in wave.iter_mut().enumerate() {
            let captured = tokio::time::timeout(Duration::from_secs(30), rx)
                .await
                .unwrap_or_else(|_| panic!("delivery {i} of wave {w} timed out"))
                .unwrap();
            assert!(
                captured.tls.contains("TLSv1"),
                "delivery {i} of wave {w} was not over TLS: {captured:?}"
            );
            assert!(
                captured.head.contains("\"event\":\"received\""),
                "delivery {i} of wave {w} carried no payload: {captured:?}"
            );
        }
    }

    // The production-honest introspection: the bound held and eviction
    // happened, observable from outside the process like any operator would.
    let (st, metrics) = http_get_text(srv.http_addr, "/metrics").await;
    assert_eq!(st, 200);
    let entries = metric(&metrics, "swarmail_webhook_ca_cache_entries");
    assert_eq!(
        entries,
        swarmail::webhook::CA_CLIENT_CAP as u64,
        "the cache must sit exactly at its cap after {count} distinct roots"
    );
    assert!(
        metric(&metrics, "swarmail_webhook_ca_cache_evictions_total") >= 1,
        "distinct roots beyond the cap must have evicted:\n{metrics}"
    );
    assert!(
        metric(&metrics, "swarmail_webhook_ca_cache_builds_total") >= count as u64,
        "every distinct root must have built its client:\n{metrics}"
    );
}

/// The sink must survive a failed first attempt. The 2026-10-09 hermetic red
/// of the cap test was exactly this shape: the sink's `accept()` failed once
/// (EMFILE under the sandbox's fd pressure), the helper swallowed the error
/// and unbound, and the delivery was lost for good. Here the first attempt
/// dies deterministically below the HTTP layer — the connection is dropped
/// unanswered before any TLS — and the delivery must still land over real
/// TLS on a later attempt, which is the dispatcher's contract (attempt 0
/// fails, the 100ms-backoff retry succeeds).
#[tokio::test]
async fn a_dropped_first_attempt_is_retried_and_still_delivered_over_tls() {
    let srv = start().await;
    let (ca_pem, leaf, leaf_key) = mint_cert();
    let (addr, rx, _, _hits) = tls_sink_cfg(
        b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
        leaf,
        leaf_key,
        SINK_LIFETIME,
        1, // the first connection is dropped unanswered: attempt 0 never handshakes
    )
    .await;
    let targets = serde_json::json!([{
        "url": format!("https://127.0.0.1:{}/hook", addr.port()),
        "secret": "topsecret",
        "ca_pem": ca_pem,
    }])
    .to_string();
    let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200);

    smtp_send(srv.smtp_addr, None, "f@x.io", "r@x.io", "droppedfirst", "x")
        .await
        .unwrap();

    // The retry budget is 4 attempts x 5s plus backoff; a successful retry
    // lands well inside that. The capture must be the real exchange: the
    // dropped attempt resolved nothing.
    let captured = tokio::time::timeout(Duration::from_secs(30), rx)
        .await
        .expect("the retried delivery never landed")
        .unwrap();
    assert!(
        captured.tls.contains("TLSv1"),
        "the retried delivery was not over TLS: {captured:?}"
    );
    assert!(
        captured.head.contains("\"event\":\"received\""),
        "the retried delivery carried no payload: {captured:?}"
    );
}

/// Delivering twice through the same ca_pem exercises the LRU's
/// already-present arm (`touch_lru` moving an existing key to the back) —
/// the second build must reuse the cached client and still land.
#[tokio::test]
async fn repeated_deliveries_through_one_pem_reuse_the_cache() {
    let srv = start().await;
    let (ca_pem, leaf, leaf_key) = mint_cert();
    // The window outlives everything the two passes could spend; the count
    // is read by signal below (issue #102).
    let (addr, rx, _count, sink_hits) = tls_sink(
        b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
        leaf,
        leaf_key,
        SINK_LIFETIME,
    )
    .await;

    let targets = serde_json::json!([{
        "url": format!("https://127.0.0.1:{}/hook", addr.port()),
        "ca_pem": ca_pem,
    }])
    .to_string();
    let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200);

    // The delivered baseline is taken before either mail exists: only NEW
    // deliveries resolve the poll below (issue #51).
    let delivered_base = metric_now(srv.http_addr, "swarmail_webhook_delivered_total").await;
    smtp_send(srv.smtp_addr, None, "f@x.io", "r@x.io", "first pass", "x")
        .await
        .unwrap();
    smtp_send(srv.smtp_addr, None, "f@x.io", "r@x.io", "second pass", "x")
        .await
        .unwrap();

    // The first delivery is captured over TLS; the second pass is *in* the
    // count read after the server's own row says both deliveries happened —
    // a third connection would red (issue #102).
    let captured = tokio::time::timeout(Duration::from_secs(5), rx)
        .await
        .expect("the first delivery timed out")
        .unwrap();
    assert!(
        captured.tls.contains("TLSv1"),
        "not negotiated over TLS: {:?}",
        captured
    );
    wait_for_metric_from(
        srv.http_addr,
        "swarmail_webhook_delivered_total",
        delivered_base,
        2,
    )
    .await;
    let hits_seen = hits(&sink_hits);
    assert_eq!(hits_seen, 2, "both passes must reach the sink over TLS");
}

/// A `ca_pem` whose base64 does not decode: the PUT refuses with 400,
/// naming the defect through the delivery TLS stack itself (decision 0029
/// §5) — a misconfig is discovered at the door, never at delivery time,
/// and nothing is applied.
#[tokio::test]
async fn garbage_ca_pem_is_refused_at_put_and_nothing_is_applied() {
    let srv = start().await;
    // The sink exists only to prove the refused target never dialed: the
    // GET below is the structural proof that nothing was applied, so no
    // delivery can ever be armed for the mail and no connection can ever
    // be made — the zero is read after the mail is stored, with no window
    // to race (issue #102).
    let pair = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
    let (addr, _rx_req, _count, sink_hits) = tls_sink(
        b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
        pair.cert.der().to_vec(),
        pair.key_pair.serialize_der(),
        SINK_LIFETIME,
    )
    .await;
    let broken = "-----BEGIN CERTIFICATE-----\n!!!not base64!!!\n-----END CERTIFICATE-----";
    let targets = serde_json::json!([{
        "url": format!("https://127.0.0.1:{}/hook", addr.port()),
        "ca_pem": broken,
    }]);
    let (st, put) = http_json(
        srv.http_addr,
        "PUT",
        "/api/v1/webhooks",
        Some(&targets.to_string()),
    )
    .await;
    assert_eq!(st, 400, "{put}");
    assert!(
        put["error"].as_str().unwrap().contains("ca_pem"),
        "the refusal names the defect: {put}"
    );

    // Nothing was applied — the GET is empty, so no delivery is ever armed
    // and the mail still lands in the store (the refusal precedes every
    // effect, decision 0025 §5).
    let (_, body) = http_json(srv.http_addr, "GET", "/api/v1/webhooks", None).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
    smtp_send(srv.smtp_addr, None, "f@x.io", "r@x.io", "garbage ca", "x")
        .await
        .unwrap();
    // Nothing applied (the GET above was empty) ⇒ no delivery exists to
    // arm ⇒ the sink can never be dialed: the zero is structural, not a
    // wait-for-nothing (issue #102).
    assert_eq!(hits(&sink_hits), 0, "the sink never saw a connection");

    // The server still serves (the failure was contained to the PUT).
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["count"], 1);
}

// ---------- the delivery bound (AC4/AC5/AC6/AC7) ----------

/// The can-fire leg for the sink protocol itself (issue #51): a window
/// that closes before any attempt can arrive resolves its count channel
/// with `0`. The channel is *observed* — it carries what the sink saw, not
/// an expectation echoed back at the `expect`-th hit (there is no expect to
/// fire at). Red if a sink ever fires early again: an early fire at a
/// count that was never reached would resolve `None`-never or echo a
/// hoped-for number, and both are visible here.
#[tokio::test]
async fn a_window_closing_before_any_attempt_observes_zero() {
    let (_, _, rx, _) = http_sink(204, Duration::ZERO).await;
    assert_eq!(rx.await.unwrap(), 0, "a closed window saw nothing");
    let (_, _, rx, _) =
        redirect_sink(302, "http://127.0.0.1:1/nowhere".into(), Duration::ZERO).await;
    assert_eq!(rx.await.unwrap(), 0, "a closed window saw nothing");
    let (ca_pem, leaf, leaf_key) = mint_cert();
    let (_, _, rx, _) = tls_sink_cfg(
        b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
        leaf,
        leaf_key,
        Duration::ZERO,
        0,
    )
    .await;
    drop(ca_pem);
    assert_eq!(rx.await.unwrap(), 0, "a closed window saw nothing");
}

/// A sink that answers every request with 200 after `hold`, and counts
/// concurrent in-flight requests. The hold is the deterministic way to fill
/// the delivery bound: it outlasts the whole accept phase, so the bound —
/// not a completed delivery — is what refuses the excess.
async fn delaying_sink(
    hold: Duration,
) -> (std::net::SocketAddr, Arc<AtomicUsize>, Arc<AtomicUsize>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let hits = Arc::new(AtomicUsize::new(0));
    let max = Arc::new(AtomicUsize::new(0));
    let cur = Arc::new(AtomicUsize::new(0));
    let reply = b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec();
    let max_counter = Arc::clone(&max);
    let hits_counter = Arc::clone(&hits);
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            hits_counter.fetch_add(1, Ordering::Relaxed);
            let now = cur.fetch_add(1, Ordering::Relaxed) + 1;
            max_counter.fetch_max(now, Ordering::Relaxed);
            let cur = Arc::clone(&cur);
            let reply = reply.clone();
            tokio::spawn(async move {
                let hold = tokio::time::sleep(hold);
                tokio::pin!(hold);
                // Reply after the hold — unless the peer hangs up first,
                // which ends the request's in-flight life just as surely
                // (a timed-out attempt leaves a ghost behind otherwise).
                let mut buf = [0u8; 4096];
                let peer_left = loop {
                    tokio::select! {
                        _ = &mut hold => break false,
                        read = stream.read(&mut buf) => {
                            if read.unwrap_or(0) == 0 {
                                break true;
                            }
                        }
                    }
                };
                if !peer_left {
                    let _ = stream.write_all(&reply).await;
                }
                cur.fetch_sub(1, Ordering::Relaxed);
            });
        }
    });
    (addr, hits, max)
}

/// A sink that accepts every connection and never answers: each delivery it
/// receives wedges until the server kills it. Reports the connection count
/// (a wedged delivery is a connection) and the count's resolution channel.
async fn stalled_sink() -> (std::net::SocketAddr, Arc<AtomicUsize>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&hits);
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            counter.fetch_add(1, Ordering::Relaxed);
            tokio::spawn(async move {
                // Read until the peer hangs up — the EOF is the abort.
                let mut buf = [0u8; 4096];
                loop {
                    match stream.read(&mut buf).await {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {}
                    }
                }
            });
        }
    });
    (addr, hits)
}

/// A sink that captures every request body, holds every request until
/// `release` fires, then answers the first with 500 and the rest with 200.
/// The hold is what makes the payload-replay proof deterministic: the first
/// delivery is still in flight when the store has already pruned its mail.
async fn gated_sink() -> (
    std::net::SocketAddr,
    Arc<AtomicUsize>,
    Arc<std::sync::Mutex<Vec<String>>>,
    watch::Sender<bool>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let hits = Arc::new(AtomicUsize::new(0));
    let bodies: Arc<std::sync::Mutex<Vec<String>>> = Arc::default();
    let (release, gate) = watch::channel(false);
    let counter = Arc::clone(&hits);
    let all = Arc::clone(&bodies);
    tokio::spawn(async move {
        let mut first = true;
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            counter.fetch_add(1, Ordering::Relaxed);
            let is_first = first;
            first = false;
            let bodies = Arc::clone(&all);
            let mut gate = gate.clone();
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                loop {
                    let n = stream.read(&mut chunk).await.unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                    if content_length(&buf).is_some_and(|len| buf.len() >= len) {
                        break;
                    }
                }
                bodies
                    .lock()
                    .unwrap()
                    .push(String::from_utf8_lossy(&buf).to_string());
                let _ = gate.changed().await;
                let reply = if is_first {
                    "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                } else {
                    "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                };
                let _ = stream.write_all(reply.as_bytes()).await;
            });
        }
    });
    (addr, hits, bodies, release)
}

/// The POSTed payload of a captured request: everything after the head.
fn payload_of(request: &str) -> &str {
    request.split("\r\n\r\n").nth(1).unwrap_or("")
}

/// AC4: the bound holds exactly. Fifty mails against one target that replies
/// slower than the accept path can feed it: 32 deliveries are in flight
/// (each a connection at the sink), the other 18 fan-outs are refused at the
/// bound, and nothing else — one refused row per refused mail.
#[tokio::test]
async fn the_delivery_bound_holds_exactly_and_the_excess_is_counted() {
    let srv = start().await;
    // The hold outlives the whole observation: no attempt ever gets a
    // reply, so a delivery leaves the bound only through its own budget
    // (22.1 s, past this test) — the arithmetic below cannot be blurred by
    // a completed delivery. The dispatcher's own POST_TIMEOUT turns each
    // attempt into a retry, which the high-water mark absorbs.
    let (addr, hits, max) = delaying_sink(Duration::from_secs(40)).await;
    let body = serde_json::json!([{ "url": format!("http://{addr}/hook") }]).to_string();
    let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&body)).await;
    assert_eq!(st, 200);

    // The refusal baseline precedes the storm: only refusals this storm
    // caused satisfy the poll (issue #51).
    let dropped_base = metric_now(srv.http_addr, "swarmail_webhook_dropped_total").await;
    for i in 0..50 {
        let reply = smtp_send(srv.smtp_addr, None, "f@x.io", "storm@x.io", "storm", "x")
            .await
            .unwrap_or_else(|e| panic!("mail {i} refused: {e}"));
        assert!(reply.starts_with("250"), "mail {i} refused: {reply}");
    }

    wait_for_metric_from(
        srv.http_addr,
        "swarmail_webhook_dropped_total",
        dropped_base,
        18,
    )
    .await;
    assert_eq!(
        max.load(Ordering::Relaxed),
        32,
        "the bound is exactly the high-water mark of requests in flight"
    );
    assert!(
        hits.load(Ordering::Relaxed) >= 32,
        "every one of the 32 deliveries reached the wire at least once"
    );
    let (_, text) = http_get_text(srv.http_addr, "/metrics").await;
    assert_eq!(metric(&text, "swarmail_webhook_delivered_total"), 0);
    assert_eq!(metric(&text, "swarmail_webhook_aborted_total"), 0);
}

/// AC5+AC6: a storm against one stalled target never pushes back on the
/// accept path. Every mail is accepted synchronously and stored — the wedge
/// downstream costs the accept path nothing — and the bound refuses exactly
/// the excess, one row per refused mail.
#[tokio::test]
async fn a_storm_against_a_stalled_target_never_pushes_back_on_the_accept_path() {
    let srv = start().await;
    let (addr, hits) = stalled_sink().await;
    let body = serde_json::json!([{ "url": format!("http://{addr}/hook") }]).to_string();
    let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&body)).await;
    assert_eq!(st, 200);

    // The refusal baseline precedes the storm (issue #51).
    let dropped_base = metric_now(srv.http_addr, "swarmail_webhook_dropped_total").await;
    for i in 0..50 {
        // smtp_send asserts the 250: a pushback would fail right here.
        smtp_send(
            srv.smtp_addr,
            Some("stormbox"),
            "f@x.io",
            "storm@x.io",
            "storm",
            "x",
        )
        .await
        .unwrap_or_else(|e| panic!("mail {i} refused: {e}"));
    }

    // Decision 0001 intact: every accepted mail is queryable, exactly.
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/stormbox/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(
        body["count"], 50,
        "the accept path never waited on delivery"
    );

    wait_for_metric_from(
        srv.http_addr,
        "swarmail_webhook_dropped_total",
        dropped_base,
        18,
    )
    .await;
    tokio::time::timeout(Duration::from_secs(15), async {
        while hits.load(Ordering::Relaxed) < 32 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("32 wedged deliveries must reach the sink");
}

/// AC7: a delivery that outlives its mail replays its own bytes. The first
/// delivery is held while two more mails prune its mail out of a
/// `max_per_inbox: 2` store — the payload is gone from the store (404) but
/// the delivery still completes with byte-identical content, because it
/// never re-reads the store.
#[tokio::test]
async fn a_delivery_outliving_its_mail_replays_its_own_bytes() {
    let srv = start_with(|mut c| {
        c.max_per_inbox = 2;
        c
    })
    .await;
    let (addr, hits, bodies, release) = gated_sink().await;
    let body = serde_json::json!([{ "url": format!("http://{addr}/hook") }]).to_string();
    let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&body)).await;
    assert_eq!(st, 200);

    let first = smtp_send(
        srv.smtp_addr,
        Some("prunebox"),
        "f@x.io",
        "prune@x.io",
        "one",
        "body one",
    )
    .await
    .unwrap();
    let (_, id) = first
        .rsplit_once("stored as ")
        .expect("the 250 names the id");
    let id = id.trim();

    // The first delivery is in flight (and held) before the flood.
    tokio::time::timeout(Duration::from_secs(15), async {
        while hits.load(Ordering::Relaxed) < 1 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the first delivery must reach the sink");

    // Two more mails prune the original, oldest-only (decision 0003).
    smtp_send(
        srv.smtp_addr,
        Some("prunebox"),
        "f@x.io",
        "prune@x.io",
        "two",
        "body two",
    )
    .await
    .unwrap();
    smtp_send(
        srv.smtp_addr,
        Some("prunebox"),
        "f@x.io",
        "prune@x.io",
        "three",
        "body three",
    )
    .await
    .unwrap();
    let (st, _) = http_get_text(srv.http_addr, &format!("/api/v1/messages/{id}")).await;
    assert_eq!(
        st, 404,
        "the mail must be pruned before the delivery is released"
    );

    // The held delivery is answered 500; every later request 200. The retry
    // replays the same bytes; the two later mails deliver on their own.
    let delivered_base = metric_now(srv.http_addr, "swarmail_webhook_delivered_total").await;
    let _ = release.send(true);
    wait_for_metric_from(
        srv.http_addr,
        "swarmail_webhook_delivered_total",
        delivered_base,
        3,
    )
    .await;

    let captured = bodies.lock().unwrap().clone();
    assert_eq!(
        captured.len(),
        4,
        "the held delivery, the two later mails, and the retry: {captured:?}"
    );
    let first_payload = payload_of(&captured[0]);
    let replays = captured
        .iter()
        .filter(|b| payload_of(b) == first_payload)
        .count();
    assert!(
        replays >= 2,
        "the payload must be replayed byte-identical, not re-read from the pruned store: {captured:?}"
    );
}

/// The per-target budget's own acceptance (issue #47): a stalled target
/// cannot cause drops for a healthy one. A cap's worth plus eight mails
/// fan out to both targets; the stalled one wedges a slot per delivery
/// (its hold outlives the whole test) and refuses its own excess — under
/// the old shared pool those wedges filled the one cap and the healthy
/// fan-outs were refused with them. Per target, the healthy sink receives
/// every mail, and the ledger's `dropped` row is exactly the stalled
/// target's own refusals — neither row is the other's.
#[tokio::test]
async fn a_stalled_target_cannot_starve_a_healthy_one() {
    let srv = start().await;
    // The stalled hold outlives the test: its wedges are still holding
    // their slots when the assertions run, which is what makes the
    // arithmetic exact (each excess mail's stalled spawn is refused, each
    // healthy spawn is not).
    let (stalled, stalled_hits, _stalled_max) = delaying_sink(Duration::from_secs(60)).await;
    let (healthy, healthy_hits, _healthy_max) = delaying_sink(Duration::ZERO).await;
    let body = serde_json::json!([
        { "url": format!("http://{stalled}/hook") },
        { "url": format!("http://{healthy}/hook") },
    ])
    .to_string();
    let (st, resp) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&body)).await;
    assert_eq!(st, 200, "{resp}");

    let total = swarmail::webhook::WEBHOOK_DELIVERY_CAP + 8;
    for i in 0..total {
        smtp_send(
            srv.smtp_addr,
            Some("twosink"),
            "f@x.io",
            "two@x.io",
            "two sinks",
            "x",
        )
        .await
        .unwrap_or_else(|e| panic!("mail {i} refused: {e}"));
    }

    // Every mail reached the healthy sink — observed on the ledger, so the
    // count and the wire agree, and on the sink's own counter, so the
    // deliveries were real POSTs and not ledger bookkeeping.
    tokio::time::timeout(Duration::from_secs(15), async {
        while srv.webhooks.snapshot().delivered < total as u64 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("every fan-out must reach the healthy sink");
    assert_eq!(
        healthy_hits.load(Ordering::Relaxed),
        total,
        "the healthy sink saw every mail"
    );

    // The stalled target's refusals are its own: exactly the mails beyond
    // its cap, while the wedged ones still hold their slots.
    let ledger = srv.webhooks.snapshot();
    assert_eq!(
        ledger.dropped, 8,
        "the stalled target's excess is its own refusals: {ledger:?}"
    );
    assert_eq!(ledger.delivered, total as u64, "{ledger:?}");
    assert_eq!(ledger.aborted, 0, "{ledger:?}");
    assert!(
        stalled_hits.load(Ordering::Relaxed) >= swarmail::webhook::WEBHOOK_DELIVERY_CAP,
        "the stall was real: wedged deliveries must have dialed the sink"
    );
}

/// A target that cannot be delivered is refused at PUT (issue #47, decision
/// 0029 §5): a scheme-forgotten `localhost:9000` (which `Url::parse` reads
/// as *scheme* `localhost` — today that misconfig surfaces only at
/// delivery), a non-HTTP scheme, an unparsable URL and a secret that could
/// not ride a header all answer 400 naming their defect, and nothing is
/// applied — the config the GET returns stays empty.
#[tokio::test]
async fn a_target_that_cannot_be_delivered_is_refused_at_put_and_applies_nothing() {
    let srv = start().await;
    for url in [
        "localhost:9000/hook", // scheme-forgotten: no host to dial
        "ftp://127.0.0.1:9/hook",
        "http://", // parses to an empty host
        "",
    ] {
        let body = serde_json::json!([{ "url": url }]).to_string();
        let (st, resp) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&body)).await;
        assert_eq!(st, 400, "{url:?}: {resp}");
        let message = resp["error"].as_str().expect("a JSON refusal body");
        assert!(
            message.contains("webhook URL"),
            "{url:?}: the refusal names the URL defect: {resp}"
        );
    }

    // The header parser's domain (issue #75, decision 0041 §1): U+0001 and
    // DEL were accepted here and then failed every delivery locally.
    for secret in [
        "a\r\nb",
        "a\0b",
        "bad\u{1}secret",
        "bad\u{7f}secret",
        " padded",
        "padded\t",
        "x".repeat(4097).as_str(),
    ] {
        let body = serde_json::json!([
            { "url": "http://127.0.0.1:9/hook", "secret": secret }
        ])
        .to_string();
        let (st, resp) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&body)).await;
        assert_eq!(st, 400, "secret {secret:?}: {resp}");
        let message = resp["error"].as_str().expect("a JSON refusal body");
        assert!(
            message.contains("secret"),
            "the refusal names the secret defect: {resp}"
        );
    }

    // A refused PUT applies nothing: the config is still empty, and a legal
    // target stores after the refusals.
    let (_, body) = http_json(srv.http_addr, "GET", "/api/v1/webhooks", None).await;
    assert_eq!(body.as_array().expect("the config list").len(), 0, "{body}");
    let (st, resp) = http_json(
        srv.http_addr,
        "PUT",
        "/api/v1/webhooks",
        Some(r#"[{"url": "http://127.0.0.1:9/hook", "inbox": "ok"}]"#),
    )
    .await;
    assert_eq!(st, 200, "{resp}");
}

/// A webhook inbox filter answers to the one inbox-name rule (issue #128,
/// decision 0046 §4): a blank or control-bearing filter is a 400 naming the
/// rule and replaces nothing — the installed list survives whole — while a
/// padded filter is installed verbatim and an omitted one stays "every
/// inbox". Red before: `"  "` and `"a\0b"` were installed as filters no
/// admitted inbox could ever match.
#[tokio::test]
async fn a_webhook_inbox_filter_answers_to_the_inbox_rule() {
    let srv = start().await;
    let installed = r#"[{"url": "http://127.0.0.1:9/hook", "inbox": " pad "},
                        {"url": "http://127.0.0.1:9/all"}]"#;
    let (st, resp) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(installed)).await;
    assert_eq!(st, 200, "{resp}");
    for (inbox, sentence) in [
        ("", "inbox must not be blank"),
        ("  \u{3000}", "inbox must not be blank"),
        ("a\0b", "inbox must not contain control characters"),
        ("pad\r", "inbox must not contain control characters"),
        ("c1\u{85}", "inbox must not contain control characters"),
    ] {
        let body = serde_json::json!([
            { "url": "http://127.0.0.1:9/ok" },
            { "url": "http://127.0.0.1:9/hook", "inbox": inbox }
        ])
        .to_string();
        let (st, resp) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&body)).await;
        assert_eq!(st, 400, "{inbox:?}: {resp}");
        assert_eq!(
            resp["error"],
            format!("webhook target 1: {sentence}"),
            "{inbox:?}"
        );
    }
    let (_, body) = http_json(srv.http_addr, "GET", "/api/v1/webhooks", None).await;
    assert_eq!(
        body,
        serde_json::json!([
            {"url": "http://127.0.0.1:9/hook", "inbox": " pad ", "secret": null, "ca_pem": null},
            {"url": "http://127.0.0.1:9/all", "inbox": null, "secret": null, "ca_pem": null}
        ]),
        "the refused lists replaced nothing"
    );
}

// ---------- decision 0041: the webhook boundary ----------

/// A refused list replaces nothing (issue #75): with a configuration in
/// place, a PUT whose second target carries a U+0001 secret is a 400 that
/// names the secret without echoing it, and GET still returns exactly the
/// prior list.
#[tokio::test]
async fn a_list_with_one_unheaderable_secret_leaves_the_prior_config_whole() {
    let srv = start().await;
    let prior = r#"[{"url": "http://127.0.0.1:9/prior", "inbox": "kept", "secret": "s"}]"#;
    let (st, before) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(prior)).await;
    assert_eq!(st, 200, "{before}");

    let body = serde_json::json!([
        { "url": "http://127.0.0.1:9/fine" },
        { "url": "http://127.0.0.1:9/bad", "secret": "bad\u{1}secret" },
    ])
    .to_string();
    let (st, resp) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&body)).await;
    assert_eq!(st, 400, "{resp}");
    let message = resp["error"].as_str().expect("a JSON refusal body");
    assert!(
        message.starts_with("webhook target 1: webhook secret"),
        "{resp}"
    );
    assert!(
        !message.contains("bad"),
        "the refusal never echoes the secret: {resp}"
    );

    let (_, after) = http_json(srv.http_addr, "GET", "/api/v1/webhooks", None).await;
    assert_eq!(after, before, "the prior list is untouched");
}

/// URL credentials are delivered, never echoed (issue #98, decision 0041
/// §3): a target spelled `user:hunter2@` comes back from PUT and GET with
/// its userinfo removed — query kept — while the sink receives the Basic
/// authorization reqwest derives from it, and the accepted secret's exact
/// bytes (internal tab, UTF-8) in its header (issue #75).
#[tokio::test]
async fn url_credentials_reach_the_sink_and_never_the_echo() {
    let srv = start().await;
    let (addr, rx, _, _hits) = http_sink(204, SINK_LIFETIME).await;
    let body = serde_json::json!([{
        "url": format!("http://user:hunter2@{addr}/hook?k=v"),
        "secret": "a\tb é",
    }])
    .to_string();
    let (st, echo) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&body)).await;
    assert_eq!(st, 200, "{echo}");
    let (_, listed) = http_json(srv.http_addr, "GET", "/api/v1/webhooks", None).await;
    for shown in [&echo, &listed] {
        assert_eq!(
            shown[0]["url"],
            format!("http://{addr}/hook?k=v"),
            "{shown}"
        );
        assert_eq!(shown[0]["secret"], serde_json::Value::Null, "{shown}");
        assert!(!shown.to_string().contains("hunter2"), "{shown}");
    }

    smtp_send(srv.smtp_addr, None, "f@x.io", "r@x.io", "basic", "x")
        .await
        .unwrap();
    let head = tokio::time::timeout(Duration::from_secs(5), rx)
        .await
        .expect("webhook delivery timed out")
        .unwrap();
    let lower = head.to_ascii_lowercase();
    // base64("user:hunter2")
    assert!(
        lower.contains("authorization: basic dxnlcjpodw50zxiy"),
        "{head}"
    );
    assert!(lower.contains("x-swarmail-secret: a\tb é\r\n"), "{head}");
    assert!(head.starts_with("POST /hook?k=v HTTP/1.1"), "{head}");
}

// ---------- decision 0048: complete CA validation and the closed target ----------

/// A well-framed PEM certificate block whose base64 carries `bytes`.
fn framed_pem(bytes: &[u8]) -> String {
    use base64::Engine as _;
    let body = base64::engine::general_purpose::STANDARD.encode(bytes);
    format!("-----BEGIN CERTIFICATE-----\n{body}\n-----END CERTIFICATE-----\n")
}

/// A fresh self-signed CA, PEM-encoded, that signs nothing the test serves.
fn unrelated_ca_pem() -> String {
    let key = rcgen::KeyPair::generate().unwrap();
    let mut params = rcgen::CertificateParams::new(Vec::<String>::new()).unwrap();
    params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    params.key_usages = vec![rcgen::KeyUsagePurpose::KeyCertSign];
    params.self_signed(&key).unwrap().pem()
}

/// The installed list, field by field — the server's own state, not the
/// redacted GET: a refusal that cleared a secret or a URL's credentials
/// would pass a GET comparison.
fn installed(srv: &swarmail::RunningServer) -> Vec<[Option<String>; 4]> {
    srv.webhooks
        .current()
        .into_iter()
        .map(|t| [Some(t.url), t.inbox, t.secret, t.ca_pem])
        .collect()
}

/// A prior target carrying URL credentials and a secret, filtered to
/// `inbox` and pointed at a fresh sink: the configuration every refusal
/// below must leave whole.
async fn install_prior(srv: &swarmail::RunningServer, inbox: &str) -> Receiver<String> {
    let (addr, rx, _, _hits) = http_sink(204, SINK_LIFETIME).await;
    let prior = serde_json::json!([{
        "url": format!("http://user:hunter2@{addr}/prior"),
        "inbox": inbox,
        "secret": "kept-secret",
    }])
    .to_string();
    let (st, resp) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&prior)).await;
    assert_eq!(st, 200, "{resp}");
    rx
}

/// The prior target still delivers with its own credentials: the refusals
/// replaced nothing the delivery path reads.
async fn assert_prior_delivers(srv: &swarmail::RunningServer, inbox: &str, rx: Receiver<String>) {
    smtp_send(srv.smtp_addr, Some(inbox), "f@x.io", "r@x.io", "prior", "x")
        .await
        .unwrap();
    let head = tokio::time::timeout(Duration::from_secs(10), rx)
        .await
        .expect("the prior target still delivers")
        .unwrap();
    let lower = head.to_ascii_lowercase();
    assert!(head.starts_with("POST /prior HTTP/1.1"), "{head}");
    assert!(lower.contains("x-swarmail-secret: kept-secret"), "{head}");
    // base64("user:hunter2")
    assert!(
        lower.contains("authorization: basic dxnlcjpodw50zxiy"),
        "{head}"
    );
}

/// Issue #122 (decision 0048 §1): every certificate a `ca_pem` carries must
/// load into the delivery client at PUT. Red before: a well-framed block
/// of junk DER — the issue's reproduction — and a junk block after a good
/// one answered 200, replaced the working list, and every delivery then
/// failed building its client. Each defect refuses on an http and an https
/// target, alone and after a valid target, with the named `ca_pem`
/// sentence at the bad target's index; the installed list — credentials
/// and all — is untouched, and the prior target still delivers with them.
#[tokio::test]
async fn webhook_ca_put_rejects_every_invalid_certificate() {
    let srv = start().await;
    let rx = install_prior(&srv, "ca122").await;
    let before = installed(&srv);
    let good = unrelated_ca_pem();
    let malformed = "-----BEGIN CERTIFICATE-----\n!!!not base64!!!\n-----END CERTIFICATE-----\n";
    for (case, pem) in [
        ("no certificate section", "not a pem".to_string()),
        ("empty certificate", framed_pem(b"")),
        ("malformed base64", malformed.to_string()),
        ("invalid DER", framed_pem(b"not a DER certificate")),
        (
            "valid then invalid DER",
            format!("{good}{}", framed_pem(b"junk")),
        ),
        ("valid then malformed block", format!("{good}{malformed}")),
    ] {
        for url in ["http://127.0.0.1:9/hook", "https://127.0.0.1:9/hook"] {
            let bad = serde_json::json!({ "url": url, "ca_pem": pem });
            let fine = serde_json::json!({ "url": "http://127.0.0.1:9/fine", "ca_pem": good });
            for (index, list) in [(0, vec![bad.clone()]), (1, vec![fine, bad])] {
                let body = serde_json::Value::Array(list).to_string();
                let (st, resp) =
                    http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&body)).await;
                assert_eq!(st, 400, "{case} {url} at {index}: {resp}");
                let error = resp["error"].as_str().unwrap();
                assert!(
                    error.starts_with(&format!(
                        "webhook target {index}: webhook ca_pem could not be loaded into the rustls client: "
                    )),
                    "{case} {url} at {index}: {resp}"
                );
                assert_eq!(installed(&srv), before, "{case} {url} at {index}");
            }
        }
    }
    assert_prior_delivers(&srv, "ca122", rx).await;
}

/// Issue #135 (decision 0048 §2): the target object is closed and its
/// optional strings are omitted or strings. Red before: `secrt` installed a
/// target with no secret, and `null` rode the default. Each defect — after
/// a valid target — is a 400 naming the key and leaves the installed list
/// whole; replaying the GET echo, whose absent values are `null`, refuses
/// the same way (the echo is a diagnostic, 0048 §3). Omission still
/// installs.
#[tokio::test]
async fn webhook_unknown_and_null_fields_preserve_configuration() {
    let srv = start().await;
    let rx = install_prior(&srv, "keys135").await;
    let before = installed(&srv);
    let (_, echo) = http_json(srv.http_addr, "GET", "/api/v1/webhooks", None).await;
    assert_eq!(echo[0]["secret"], serde_json::Value::Null, "{echo}");
    assert_eq!(echo[0]["ca_pem"], serde_json::Value::Null, "{echo}");

    let fine = r#"{"url": "http://127.0.0.1:9/fine"}"#;
    for (bad, named) in [
        (
            r#"{"url": "http://127.0.0.1:9/x", "secrt": "s3cret"}"#,
            "[1].secrt: unknown field `secrt`",
        ),
        (
            r#"{"url": "http://127.0.0.1:9/x", "inbox": null}"#,
            "[1].inbox: expected a string, found null",
        ),
        (
            r#"{"url": "http://127.0.0.1:9/x", "secret": null}"#,
            "[1].secret: expected a string, found null",
        ),
        (
            r#"{"url": "http://127.0.0.1:9/x", "ca_pem": null}"#,
            "[1].ca_pem: expected a string, found null",
        ),
    ] {
        let body = format!("[{fine}, {bad}]");
        let (st, resp) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&body)).await;
        assert_eq!(st, 400, "{bad}: {resp}");
        let error = resp["error"].as_str().unwrap();
        assert!(error.contains(named), "{bad}: {resp}");
        assert!(!error.contains("s3cret"), "{bad}: {resp}");
        assert_eq!(installed(&srv), before, "{bad}");
    }
    let (st, resp) = http_json(
        srv.http_addr,
        "PUT",
        "/api/v1/webhooks",
        Some(&echo.to_string()),
    )
    .await;
    assert_eq!(st, 400, "the echo is not a PUT template: {resp}");
    // The first null the parse meets: `ca_pem`, the re-serialized echo's
    // keys being sorted.
    assert!(
        resp["error"]
            .as_str()
            .unwrap()
            .contains("[0].ca_pem: expected a string, found null"),
        "{resp}"
    );
    assert_eq!(installed(&srv), before, "the replay replaced nothing");
    assert_prior_delivers(&srv, "keys135", rx).await;

    let (st, resp) = http_json(
        srv.http_addr,
        "PUT",
        "/api/v1/webhooks",
        Some(&format!("[{fine}]")),
    )
    .await;
    assert_eq!(st, 200, "omission installs: {resp}");
    assert_eq!(
        installed(&srv),
        vec![[
            Some("http://127.0.0.1:9/fine".to_string()),
            None,
            None,
            None
        ]]
    );
}

/// Issue #122 (decision 0048 §1): a bundle's later roots are trusted, not
/// just its first. The sink's leaf is signed by the *second* CA of the
/// target's `ca_pem`; verification stays on, so the delivery reaching the
/// sink is the proof. Red before: only the first certificate was decoded,
/// the handshake failed on every attempt and nothing arrived.
#[tokio::test]
async fn webhook_second_ca_root_delivers_over_verified_https() {
    let srv = start().await;
    let (second, leaf, leaf_key) = mint_cert();
    let (addr, rx, _, _hits) = tls_sink(
        b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
        leaf,
        leaf_key,
        SINK_LIFETIME,
    )
    .await;
    let targets = serde_json::json!([{
        "url": format!("https://127.0.0.1:{}/hook", addr.port()),
        "ca_pem": format!("{}{second}", unrelated_ca_pem()),
    }])
    .to_string();
    let (st, resp) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200, "{resp}");
    smtp_send(srv.smtp_addr, None, "f@x.io", "r@x.io", "second root", "x")
        .await
        .unwrap();
    let captured = tokio::time::timeout(Duration::from_secs(10), rx)
        .await
        .expect("the second root's leaf was trusted")
        .unwrap();
    assert!(captured.tls.contains("TLSv1"), "{captured:?}");
    assert!(
        captured.head.starts_with("POST /hook HTTP/1.1"),
        "{captured:?}"
    );
}

/// PUT validation is offline (decision 0048 §1): building a target's CA
/// client dials nothing. The target's listener has no queued connection
/// once the PUT has answered — a dial made by the handler would have
/// completed its handshake, and so been queued, before the 200 — and the
/// positive control, a matching mail, does reach the same listener.
#[tokio::test]
async fn webhook_validation_does_not_dial() {
    let srv = start().await;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    let targets = serde_json::json!([{
        "url": format!("http://{addr}/hook"),
        "inbox": "nodial",
        "ca_pem": unrelated_ca_pem(),
    }])
    .to_string();
    let (st, resp) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200, "{resp}");
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock,
        "the PUT dialed nothing"
    );

    let listener = tokio::net::TcpListener::from_std(listener).unwrap();
    smtp_send(
        srv.smtp_addr,
        Some("nodial"),
        "f@x.io",
        "r@x.io",
        "dial",
        "x",
    )
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(10), listener.accept())
        .await
        .expect("the delivery dials the target")
        .unwrap();
}

/// The final shutdown sweep survives a lagged ring and accounts the backlog
/// it drains (issue #76): a broadcast receiver's first read can be
/// `Lagged(n)` with retained mail still behind it, and the old
/// `while let Ok(...)` loop exited on that error — recording neither the
/// lag nor the retained notifications' aborts. Driven on the unmodified
/// public APIs, current-thread, the way the review reproduced it: no
/// yield to the dispatcher, more inserts than the ring holds, then a
/// shutdown. The task must finish, and the ledger must say what happened.
#[tokio::test(flavor = "current_thread")]
async fn the_final_sweep_survives_a_lagged_ring_and_accounts_the_backlog() {
    use swarmail::store::Store;
    use swarmail::webhook::{WebhookTarget, Webhooks, spawn_dispatcher};
    // A subscriber, so the dispatcher's final default-level ledger line
    // actually executes under coverage — a disabled tracing callsite never
    // reaches its format machinery.
    let _subscriber = {
        use tracing_subscriber::util::SubscriberInitExt;
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .finish()
            .set_default()
    };

    let store = Arc::new(Store::new(0));
    let webhooks = Arc::new(Webhooks::default());
    webhooks.set(vec![WebhookTarget {
        inbox: Some("backlog".to_string()),
        url: "http://127.0.0.1:1/hook".to_string(),
        secret: Some("s3cret".to_string()),
        ca_pem: None,
    }]);

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let (drained_tx, drained_rx) = tokio::sync::watch::channel(false);
    let mut tasks = tokio::task::JoinSet::new();
    spawn_dispatcher(
        &store,
        &webhooks,
        shutdown_rx.clone(),
        drained_rx.clone(),
        &mut tasks,
    );

    // More unique mails than the 8192-slot broadcast ring, none yielding
    // to the dispatcher in between: the receiver must wake lagged.
    for i in 0..8200u64 {
        let email = swarmail::model::Email {
            id: format!("lag-{i:05}"),
            inbox: "backlog".to_string(),
            from: None,
            envelope_from: None,
            to: vec![],
            cc: vec![],
            recipients: vec![],
            subject: Some(format!("lag {i}")),
            received_at: "2026-10-18T00:00:00Z".to_string(),
            received_ms: 0,
            size: 1,
            text: Some("x".to_string()),
            html: None,
            links: vec![],
            codes: vec![],
            message_id: None,
            in_reply_to: None,
            references: vec![],
            raw: b"x".to_vec(),
        };
        store.insert(email).expect("the insert lands");
    }

    shutdown_tx.send_replace(true);
    drained_tx.send_replace(true);
    let mut saw_lag = false;
    while let Some(done) = tasks.join_next().await {
        done.expect("the dispatcher task ends cleanly, never hung on the backlog");
        saw_lag = true;
    }
    assert!(saw_lag, "the dispatcher task was spawned and awaited");

    let ledger = webhooks.snapshot();
    // The ring is 8192 and the dispatcher never ran: everything it saw was
    // lag or an abort of the retained backlog — and the sweep walked BOTH
    // instead of exiting on the first Lagged(n).
    assert_eq!(ledger.delivered, 0, "a port-1 target delivers nothing");
    assert!(
        ledger.lagged >= 1,
        "the lag is recorded, not swallowed: {:?}",
        ledger
    );
    assert_eq!(
        ledger.aborted,
        8200u64 - ledger.lagged,
        "every retained notification is accounted: {:?}",
        ledger
    );
}
