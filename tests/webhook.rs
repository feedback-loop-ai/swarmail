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

use common::{http_get_text, http_json, smtp_send, start, start_with};
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
const SINK_LIFETIME: Duration = Duration::from_secs(25);

/// Pause before re-arming a failed accept. EMFILE is transient — on Linux
/// the connection is still sitting in the accept queue — so a short retry
/// serves it instead of losing it.
const ACCEPT_RETRY: Duration = Duration::from_millis(10);

/// The first *completed* exchange claims the capture channel; every later
/// exchange only gets a reply. A connection that dies mid-request captures
/// nothing, so the capture a test asserts on is always a real one. A plain
/// mutex, never an async one: the critical section does no awaiting.
type Capture<T> = std::sync::Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<T>>>>;

/// An HTTP sink that replies `status` to every connection for its whole
/// [`SINK_LIFETIME`], and reports the first completed request head plus the
/// hit count (resolved at the `expect`-th connection, or at the window's end
/// with whatever the sink actually saw). Each connection is served in its own
/// task — one stalled exchange can never block the next accept — and an
/// accept error is retried, never silently fatal.
async fn http_sink(
    status: u16,
    expect: usize,
) -> (
    std::net::SocketAddr,
    tokio::sync::oneshot::Receiver<String>,
    tokio::sync::oneshot::Receiver<usize>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx_head, rx_head) = tokio::sync::oneshot::channel();
    let (tx_count, rx_count) = tokio::sync::oneshot::channel();
    let reply = format!("HTTP/1.1 {status} TEST\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    let capture: Capture<String> = std::sync::Arc::new(std::sync::Mutex::new(Some(tx_head)));
    tokio::spawn(async move {
        let mut tx_count = Some(tx_count);
        let mut hits = 0usize;
        let deadline = std::time::Instant::now() + SINK_LIFETIME;
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            if left.is_zero() {
                break;
            }
            match tokio::time::timeout(left, listener.accept()).await {
                Ok(Ok((stream, _))) => {
                    hits += 1;
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
                    if hits == expect
                        && let Some(tx) = tx_count.take()
                    {
                        let _ = tx.send(hits);
                    }
                }
                Ok(Err(e)) => {
                    eprintln!("http sink accept error, retrying: {e}");
                    tokio::time::sleep(ACCEPT_RETRY).await;
                }
                Err(_) => break, // the budget window is over
            }
        }
        // The window closed short of `expect` connections: report what the
        // sink actually saw, not the count the test hoped for.
        if let Some(tx) = tx_count {
            let _ = tx.send(hits);
        }
    });
    (addr, rx_head, rx_count)
}

/// What the TLS sink saw: the request head and the negotiated TLS version.
#[derive(Debug)]
struct Captured {
    head: String,
    tls: String,
}

/// A TLS sink: mints nothing, just serves the given certificate. It replies
/// `reply` to every connection for its whole [`SINK_LIFETIME`] — through the
/// dispatcher's entire retry budget, so a failed attempt's retry is still
/// served — and reports the first completed exchange's request head with its
/// negotiated TLS version. A failed handshake (an untrusted client rejecting
/// our cert) still counts as an attempt. The count resolves at the
/// `expect`-th connection, or at the window's end with what the sink actually
/// saw. Each connection is served in its own task — one stalled or partial
/// exchange can never block the next accept — and an accept error is retried,
/// never silently fatal.
async fn tls_sink(
    reply: &'static [u8],
    cert_der: Vec<u8>,
    key_der: Vec<u8>,
    expect: usize,
) -> (SocketAddr, Receiver<Captured>, Receiver<usize>) {
    tls_sink_cfg(reply, cert_der, key_der, expect, 0).await
}

/// [`tls_sink`] with the first `drop_first` accepted connections dropped
/// unanswered — a deterministic attempt failure below the HTTP layer, for the
/// retry proof.
async fn tls_sink_cfg(
    reply: &'static [u8],
    cert_der: Vec<u8>,
    key_der: Vec<u8>,
    expect: usize,
    drop_first: usize,
) -> (SocketAddr, Receiver<Captured>, Receiver<usize>) {
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
    let capture: Capture<Captured> = std::sync::Arc::new(std::sync::Mutex::new(Some(tx_req)));
    tokio::spawn(async move {
        let mut tx_count = Some(tx_count);
        let mut hits = 0usize;
        let deadline = std::time::Instant::now() + SINK_LIFETIME;
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            if left.is_zero() {
                break;
            }
            match tokio::time::timeout(left, listener.accept()).await {
                Ok(Ok((stream, _))) => {
                    hits += 1;
                    let ordinal = hits;
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
                    if hits == expect
                        && let Some(tx) = tx_count.take()
                    {
                        let _ = tx.send(hits);
                    }
                }
                Ok(Err(e)) => {
                    eprintln!("tls sink accept error, retrying: {e}");
                    tokio::time::sleep(ACCEPT_RETRY).await;
                }
                Err(_) => break, // the budget window is over
            }
        }
        if let Some(tx) = tx_count {
            let _ = tx.send(hits);
        }
    });
    (addr, rx_req, rx_count)
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
/// likes, for the sink's whole [`SINK_LIFETIME`], and reports the first
/// completed request (head and body) the moment it arrives plus the hit count
/// (resolved at the `expect`-th connection). Each connection is served in its
/// own task; an accept error is retried, never silently fatal.
async fn redirect_sink(
    status: u16,
    location: String,
    expect: usize,
) -> (SocketAddr, Receiver<String>, Receiver<usize>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx_first, rx_first) = tokio::sync::oneshot::channel();
    let (tx_count, rx_count) = tokio::sync::oneshot::channel();
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
    tokio::spawn(async move {
        let mut tx_count = Some(tx_count);
        let mut hits = 0usize;
        let deadline = std::time::Instant::now() + SINK_LIFETIME;
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            if left.is_zero() {
                break;
            }
            match tokio::time::timeout(left, listener.accept()).await {
                Ok(Ok((stream, _))) => {
                    hits += 1;
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
                    if hits == expect
                        && let Some(tx) = tx_count.take()
                    {
                        let _ = tx.send(hits);
                    }
                }
                Ok(Err(e)) => {
                    eprintln!("redirect sink accept error, retrying: {e}");
                    tokio::time::sleep(ACCEPT_RETRY).await;
                }
                Err(_) => break, // the budget window is over
            }
        }
        if let Some(tx) = tx_count {
            let _ = tx.send(hits);
        }
    });
    (addr, rx_first, rx_count)
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
    let (addr, rx, _) = http_sink(204, 1).await;
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

#[tokio::test]
async fn failing_target_is_retried_four_times_not_more() {
    let srv = start().await;
    // The sink refuses everything: the dispatcher must try exactly the
    // four-attempt budget (deliver's 0..4) and then drop with a warning.
    let (addr, _, rx_count) = http_sink(500, 4).await;
    let targets = format!(r#"[{{"url": "http://{addr}/hooks", "inbox": "retrybox"}}]"#);
    let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200);

    // A mail for another inbox must never reach the failing target: the hit
    // count below would exceed 4 if the inbox filter leaked.
    smtp_send(srv.smtp_addr, None, "f@x.io", "other@x.io", "filtered", "x")
        .await
        .unwrap();
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

    // Backoff is 100/400/1600 ms after the first attempt: 2.1 s of retries.
    tokio::time::sleep(Duration::from_millis(2800)).await;
    // The sink's accept loop ends only when the dispatcher stops connecting.
    // Give any (incorrect) extra attempt a moment, then check via the count
    // channel — it resolves when the sink saw exactly `expect` connections
    // OR when its loop already finished; either way the count is the proof.
    let seen = rx_count.await.unwrap();
    assert_eq!(seen, 4, "expected the 4-attempt retry budget, no more");
}

#[tokio::test]
async fn https_target_is_delivered_over_tls() {
    let srv = start().await;
    let (ca_pem, leaf, leaf_key) = mint_cert();
    let (addr, rx, _) = tls_sink(
        b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
        leaf,
        leaf_key,
        1,
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
    let (addr, _, rx_count) = tls_sink(
        b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
        untrusted.cert.der().to_vec(),
        untrusted.key_pair.serialize_der(),
        4,
    )
    .await;
    let targets = format!(r#"[{{"url": "https://127.0.0.1:{}/hook"}}]"#, addr.port());
    let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200);

    smtp_send(srv.smtp_addr, None, "f@x.io", "r@x.io", "distrusted", "x")
        .await
        .unwrap();

    // Backoff is 100/400/1600 ms: 2.1 s of retries, then the warning.
    tokio::time::sleep(Duration::from_millis(2800)).await;
    let seen = rx_count.await.unwrap();
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
    let (addr, _, rx_count) = tls_sink(
        b"HTTP/1.1 500 Internal Server Error\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
        leaf,
        leaf_key,
        4,
    )
    .await;
    let targets = serde_json::json!([{
        "url": format!("https://127.0.0.1:{}/hook", addr.port()),
        "ca_pem": ca_pem,
    }])
    .to_string();
    let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200);

    smtp_send(srv.smtp_addr, None, "f@x.io", "r@x.io", "servererror", "x")
        .await
        .unwrap();

    // Backoff is 100/400/1600 ms after the first attempt: 2.1 s of retries.
    tokio::time::sleep(Duration::from_millis(2800)).await;
    let seen = rx_count.await.unwrap();
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
    // the store: the retry budget elapses, the mail is still queryable.
    tokio::time::sleep(Duration::from_millis(2800)).await;
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
    let (addr, rx_first, rx_count) = redirect_sink(302, format!("http://{dest}/stolen"), 4).await;
    let targets = format!(r#"[{{"url": "http://{addr}/hook", "secret": "topsecret"}}]"#);
    let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200);

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
    // microseconds of the 302, and again on every retry: let the whole
    // retry budget elapse with the destination listening, then read its
    // capture — it must be empty.
    tokio::time::sleep(Duration::from_millis(2800)).await;
    // Every attempt has been served by now; give a would-be forwarded
    // request a final beat to arrive before the listener closes.
    let seen = tokio::time::timeout(Duration::from_secs(10), rx_count)
        .await
        .expect("the redirect sink never finished serving the budget")
        .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
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
    let (addr, rx_first, rx_count) = redirect_sink(307, format!("http://{dest}/stolen"), 4).await;
    let targets = format!(r#"[{{"url": "http://{addr}/hook", "secret": "topsecret"}}]"#);
    let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200);

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

    tokio::time::sleep(Duration::from_millis(2800)).await;
    let seen = tokio::time::timeout(Duration::from_secs(10), rx_count)
        .await
        .expect("the redirect sink never finished serving the budget")
        .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
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
        let (addr, rx, _) = tls_sink(
            b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
            leaf,
            leaf_key,
            1,
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
    let (addr, rx, _) = tls_sink_cfg(
        b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
        leaf,
        leaf_key,
        1,
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
    let (addr, rx, count) = tls_sink(
        b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
        leaf,
        leaf_key,
        2,
    )
    .await;

    let targets = serde_json::json!([{
        "url": format!("https://127.0.0.1:{}/hook", addr.port()),
        "ca_pem": ca_pem,
    }])
    .to_string();
    let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200);

    smtp_send(srv.smtp_addr, None, "f@x.io", "r@x.io", "first pass", "x")
        .await
        .unwrap();
    smtp_send(srv.smtp_addr, None, "f@x.io", "r@x.io", "second pass", "x")
        .await
        .unwrap();

    // The first delivery is captured over TLS; the sink's connection count
    // proves the second one arrived the same way (through the cached client).
    let captured = tokio::time::timeout(Duration::from_secs(5), rx)
        .await
        .expect("the first delivery timed out")
        .unwrap();
    assert!(
        captured.tls.contains("TLSv1"),
        "not negotiated over TLS: {:?}",
        captured
    );
    let hits = tokio::time::timeout(Duration::from_secs(5), count)
        .await
        .expect("the second delivery timed out")
        .unwrap();
    assert_eq!(hits, 2, "both passes must reach the sink over TLS");
}

/// A `ca_pem` whose base64 does not decode: `from_pem` refuses, the client
/// build fails before any dial, and the target sees no connection at all.
#[tokio::test]
async fn garbage_ca_pem_builds_no_client_and_dials_nothing() {
    let srv = start().await;
    let pair = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
    let (addr, rx_req, rx_count) = tls_sink(
        b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
        pair.cert.der().to_vec(),
        pair.key_pair.serialize_der(),
        1,
    )
    .await;
    // A ca_pem whose base64 does not decode: the rustls client refuses it,
    // so no client is ever built and the target is never dialed.
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
    assert_eq!(st, 200, "{put}");

    smtp_send(srv.smtp_addr, None, "f@x.io", "r@x.io", "garbage ca", "x")
        .await
        .unwrap();

    // The backoff window passes with the sink silent: no dial was attempted.
    let budget = tokio::time::timeout(Duration::from_millis(2800), rx_count).await;
    assert!(
        budget.is_err(),
        "the sink never saw a connection: {budget:?}"
    );

    // The server still serves (the failure was contained to the delivery).
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["count"], 1);
    drop(rx_req);
}

// ---------- the delivery bound (AC4/AC5/AC6/AC7) ----------

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

/// Poll `/metrics` until `name` reaches `want`, under a deadline. The metric
/// is the production observable the dispatcher itself writes; the deadline
/// turns a broken implementation into a failure instead of a hang.
async fn poll_metric(addr: std::net::SocketAddr, name: &str, want: u64) -> u64 {
    let deadline = std::time::Instant::now() + Duration::from_secs(25);
    loop {
        let (st, text) = http_get_text(addr, "/metrics").await;
        assert_eq!(st, 200, "metrics must answer while polling {name}");
        let value = metric(&text, name);
        if value >= want {
            return value;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "{name} never reached {want} (stuck at {value})"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
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

    for i in 0..50 {
        let reply = smtp_send(srv.smtp_addr, None, "f@x.io", "storm@x.io", "storm", "x")
            .await
            .unwrap_or_else(|e| panic!("mail {i} refused: {e}"));
        assert!(reply.starts_with("250"), "mail {i} refused: {reply}");
    }

    poll_metric(srv.http_addr, "swarmail_webhook_dropped_total", 18).await;
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

    poll_metric(srv.http_addr, "swarmail_webhook_dropped_total", 18).await;
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
    let _ = release.send(true);
    poll_metric(srv.http_addr, "swarmail_webhook_delivered_total", 3).await;

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
