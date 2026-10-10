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

use common::{http_get_text, http_json, smtp_send, start};
use std::net::{IpAddr, SocketAddr};
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
/// client is rebuilt on its next delivery, never a failed delivery. One mail
/// fans out to 33 real TLS servers, each with its own freshly minted CA, so
/// the dispatcher must cycle the whole cache and rebuild on the way. On the
/// old unbounded cache this is red: `/metrics` carried no ca_cache counters,
/// and nothing kept `entries` at the cap.
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

    let targets: Vec<serde_json::Value> = sinks
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

    // One accepted mail fans out to every target: 33 distinct ca_pems go
    // through the cache in one dispatcher pass.
    smtp_send(srv.smtp_addr, None, "f@x.io", "r@x.io", "cachebound", "x")
        .await
        .unwrap();

    // Every one of the `count` deliveries must have happened over real TLS:
    // an empty `tls` capture would mean its handshake was rejected. The bound
    // is generous — the dispatcher's whole 4-attempt budget plus slack — but
    // a generous bound alone is not the fix: the sinks above now stay bound
    // through that same budget, so a transient attempt failure (an EMFILE
    // accept under a starved sandbox, a handshake that dies mid-flight) is
    // retried against a live listener instead of being lost with its sink.
    for (i, (_, _, rx)) in sinks.into_iter().enumerate() {
        let captured = tokio::time::timeout(Duration::from_secs(30), rx)
            .await
            .unwrap_or_else(|_| panic!("delivery {i} timed out"))
            .unwrap();
        assert!(
            captured.tls.contains("TLSv1"),
            "delivery {i} was not over TLS: {captured:?}"
        );
        assert!(
            captured.head.contains("\"event\":\"received\""),
            "delivery {i} carried no payload: {captured:?}"
        );
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
