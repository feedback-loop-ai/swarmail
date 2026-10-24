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
    CA_CONTRACTS, http_get_text, http_json, metric_now, smtp_send, start, start_with,
    wait_for_metric_from,
};
use std::collections::BTreeSet;
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

fn stored_fixture(recipient: &str, subject: &str, text: &str) -> (String, String, String) {
    // smtp_send writes the DATA body followed by its CRLF terminator.
    (
        recipient.to_owned(),
        subject.to_owned(),
        format!("{text}\r\n"),
    )
}

/// One immediate REST witness for every accepted SMTP fixture. The identity
/// set and distinct ids prove the accepted mails are queryable independently
/// of webhook delivery and budget counters.
async fn assert_stored_fixtures(
    http: SocketAddr,
    inbox: &str,
    expected: impl IntoIterator<Item = (String, String, String)>,
) {
    let expected: BTreeSet<_> = expected.into_iter().collect();
    let path = format!("/api/v1/inboxes/{inbox}/messages?limit={}", expected.len());
    let (status, body) = http_json(http, "GET", &path, None).await;
    assert_eq!(status, 200, "{inbox}: {body}");
    assert_eq!(body["total"], expected.len(), "{inbox}: {body}");
    let emails = body["emails"].as_array().expect("the REST message page");
    assert_eq!(emails.len(), expected.len(), "{inbox}: {body}");

    let mut actual = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for email in emails {
        let id = email["id"].as_str().expect("stored mail id");
        assert!(!id.is_empty(), "stored mail id is nonempty: {email}");
        assert!(
            ids.insert(id.to_owned()),
            "stored mail ids are distinct: {body}"
        );
        let recipients = email["recipients"]
            .as_array()
            .expect("stored envelope recipients");
        assert_eq!(recipients.len(), 1, "one fixture recipient: {email}");
        actual.insert((
            recipients[0]
                .as_str()
                .expect("stored recipient address")
                .to_owned(),
            email["subject"]
                .as_str()
                .expect("stored subject")
                .to_owned(),
            email["text"].as_str().expect("stored text").to_owned(),
        ));
    }
    assert_eq!(
        actual, expected,
        "{inbox}: stored fixture identities differ"
    );
}

/// The first *completed* exchange claims the capture channel; every later
/// exchange only gets a reply. A connection that dies mid-request captures
/// nothing, so the capture a test asserts on is always a real one. A plain
/// mutex, never an async one: the critical section does no awaiting.
type Capture<T> = std::sync::Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<T>>>>;

/// An HTTP sink that replies `status` to every connection for its whole
/// `window`, and reports the first completed request head. Each connection
/// is served in its own task — one stalled exchange can never block the next
/// accept — and an accept error is retried, never silently fatal.
///
/// Its [`Hits`] counter is live: incremented at accept and readable before
/// the window closes (issue #102). Budget tests read it only after the
/// dispatcher's exhaustion signal proves no later attempt can arrive.
async fn http_sink(
    status: u16,
    window: Duration,
) -> (
    std::net::SocketAddr,
    tokio::sync::oneshot::Receiver<String>,
    Hits,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx_head, rx_head) = tokio::sync::oneshot::channel();
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
    });
    (addr, rx_head, hits)
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
/// attempt. Each connection is served in its own task — one stalled or
/// partial exchange can never block the next accept — and an accept error
/// is retried, never silently fatal.
async fn tls_sink(
    reply: &'static [u8],
    cert_der: Vec<u8>,
    key_der: Vec<u8>,
    window: Duration,
) -> (SocketAddr, Receiver<Captured>, Hits) {
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
) -> (SocketAddr, Receiver<Captured>, Hits) {
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
    });
    (addr, rx_req, hits)
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
/// request (head and body) the moment it arrives. Its [`Hits`] counter is
/// live like every other sink. Each connection is served in its own task;
/// an accept error is retried, never silently fatal.
async fn redirect_sink(
    status: u16,
    location: String,
    window: Duration,
) -> (SocketAddr, Receiver<String>, Hits) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx_first, rx_first) = tokio::sync::oneshot::channel();
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
    });
    (addr, rx_first, hits)
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
    let _ca_contract = CA_CONTRACTS.lock().await;
    let srv = start().await;
    let (addr, rx, _hits) = http_sink(204, SINK_LIFETIME).await;
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
    let _ca_contract = CA_CONTRACTS.lock().await;
    let srv = start().await;
    let (addr, rx, _hits) = http_sink(204, SINK_LIFETIME).await;
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
    let _ca_contract = CA_CONTRACTS.lock().await;
    let srv = start().await;
    // The sink refuses everything: the dispatcher must try exactly the
    // four-attempt budget (deliver's 0..4) and then drop with a warning.
    // The window outlives any budget the loop could ever spend — a mutant
    // fifth attempt fires at ~+8.5 s — and the test never waits on it:
    // the budget is observed by signal below.
    let (addr, _, sink_hits) = http_sink(500, SINK_LIFETIME).await;
    let targets = format!(r#"[{{"url": "http://{addr}/hooks", "inbox": "retrybox"}}]"#);
    let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200);

    // A mail for another inbox must never reach the failing target: the hit
    // count below would exceed 4 if the inbox filter leaked.
    smtp_send(srv.smtp_addr, None, "f@x.io", "other@x.io", "filtered", "x")
        .await
        .unwrap();
    assert_stored_fixtures(
        srv.http_addr,
        "default",
        [stored_fixture("other@x.io", "filtered", "x")],
    )
    .await;

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
    assert_stored_fixtures(
        srv.http_addr,
        "retrybox",
        [stored_fixture("r@x.io", "retried", "x")],
    )
    .await;

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
    let seen = sink_hits.load(Ordering::Relaxed);
    assert_eq!(seen, 4, "expected the 4-attempt retry budget, no more");
}

#[tokio::test]
async fn https_target_is_delivered_over_tls() {
    let _ca_contract = CA_CONTRACTS.lock().await;
    let srv = start().await;
    let (ca_pem, leaf, leaf_key) = mint_cert();
    let (addr, rx, _hits) = tls_sink(
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
        "ca_pem": ca_pem.clone(),
    }])
    .to_string();
    let (st, echoed) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200, "{echoed}");
    let put_ca = echoed[0]["ca_pem"]
        .as_str()
        .expect("PUT echo retains public CA material");
    assert!(ca_pem.contains('\n'), "generated PEM includes line breaks");
    assert_eq!(
        put_ca.as_bytes(),
        ca_pem.as_bytes(),
        "ca_pem PUT echo is byte-for-byte"
    );
    assert_eq!(
        echoed[0]["secret"],
        serde_json::Value::Null,
        "PUT echo withholds secret"
    );
    assert!(
        !echoed.to_string().contains("topsecret"),
        "PUT echo never returns secret"
    );
    let (get_status, listed) = http_json(srv.http_addr, "GET", "/api/v1/webhooks", None).await;
    assert_eq!(get_status, 200, "{listed}");
    let returned_ca = listed[0]["ca_pem"]
        .as_str()
        .expect("GET retains public CA material");
    assert!(ca_pem.contains('\n'), "generated PEM includes line breaks");
    assert_eq!(
        returned_ca.as_bytes(),
        ca_pem.as_bytes(),
        "ca_pem GET is byte-for-byte"
    );
    assert_eq!(
        listed[0]["secret"],
        serde_json::Value::Null,
        "GET withholds the secret"
    );

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
    let _ca_contract = CA_CONTRACTS.lock().await;
    let srv = start().await;
    // The sink serves a self-signed cert the target does not trust: every
    // attempt dies in the handshake, and the dispatcher must make exactly
    // the four-attempt budget of connections.
    let untrusted = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
    // The window outlives any budget the loop could spend; the count is
    // observed by signal below, never by the sink's clock (issue #102).
    let (addr, _, sink_hits) = tls_sink(
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
    assert_stored_fixtures(
        srv.http_addr,
        "default",
        [stored_fixture("r@x.io", "distrusted", "x")],
    )
    .await;

    // The budget is spent when the server's own row says so (issue #102);
    // the read after it sees every handshake attempt, a fifth included.
    wait_for_metric_from(
        srv.http_addr,
        "swarmail_webhook_exhausted_total",
        exhausted_base,
        1,
    )
    .await;
    let seen = sink_hits.load(Ordering::Relaxed);
    assert_eq!(seen, 4, "expected 4 handshake attempts, no more");
}

#[tokio::test]
async fn non_2xx_over_tls_is_retried_four_times_not_more() {
    let _ca_contract = CA_CONTRACTS.lock().await;
    let srv = start().await;
    let (ca_pem, leaf, leaf_key) = mint_cert();
    // Every handshake succeeds and the sink answers 500 to each request: a
    // non-2xx received over an established TLS session must cost exactly the
    // same four-attempt budget as a non-2xx over plain http — TLS may not
    // change the failure semantics, only the transport.
    // The window outlives any budget the loop could spend (issue #102).
    let (addr, _, sink_hits) = tls_sink(
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
    assert_stored_fixtures(
        srv.http_addr,
        "default",
        [stored_fixture("r@x.io", "servererror", "x")],
    )
    .await;

    // The budget is spent when the server's own row says so (issue #102).
    wait_for_metric_from(
        srv.http_addr,
        "swarmail_webhook_exhausted_total",
        exhausted_base,
        1,
    )
    .await;
    let seen = sink_hits.load(Ordering::Relaxed);
    assert_eq!(seen, 4, "expected 4 attempts over TLS, no more");
}

#[tokio::test]
async fn refused_connection_never_blocks_the_smtp_path() {
    let _ca_contract = CA_CONTRACTS.lock().await;
    let srv = start().await;
    // Own a bound, non-listening port through the final observation. Reuse
    // is disabled so another local test cannot take the refused target.
    let reservation = tokio::net::TcpSocket::new_v4().unwrap();
    reservation.set_reuseaddr(false).unwrap();
    reservation
        .bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
        .unwrap();
    let addr = reservation.local_addr().unwrap();
    let refused = tokio::time::timeout(
        common::HELPER_DEADLINE,
        tokio::net::TcpStream::connect(addr),
    )
    .await
    .expect("the reserved non-listening target must refuse promptly")
    .expect_err("a bound non-listening target must not accept connections");
    assert_eq!(
        refused.kind(),
        std::io::ErrorKind::ConnectionRefused,
        "the webhook target must fail at connect rather than during TLS or reading"
    );

    let targets = format!(r#"[{{"url": "https://{addr}/hook"}}]"#);
    let (st, _) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200);

    // The exhaustion baseline is taken before the mail that will exhaust
    // its budget: only a NEW exhaustion satisfies the poll below (issue #51).
    let exhausted_base = metric_now(srv.http_addr, "swarmail_webhook_exhausted_total").await;
    // Acceptance must not wait on the (failing) delivery: SMTP returned and
    // the mail is queryable before exhaustion is observed below.
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
    drop(reservation);
}

/// The redirect e2e: a target that answers 302 Found with an absolute
/// Location at a second real local sink. The delivery client follows no
/// redirect (Policy::none()), so the destination records nothing at all,
/// while the explicit target records the full-fidelity POST — secret and
/// body included — and the 302 costs exactly the ordinary retry budget.
#[tokio::test]
async fn redirect_302_never_leaks_the_secret_to_the_destination() {
    let _ca_contract = CA_CONTRACTS.lock().await;
    let srv = start().await;
    // The destination is bound FIRST so its port is known and no leak can
    // ever race past the listener that would catch it.
    let (tx_stop, rx_stop) = watch::channel(false);
    let (dest, rx_leaked) = destination_sink(rx_stop).await;
    // The window outlives any budget the loop could spend (issue #102).
    let (addr, rx_first, sink_hits) =
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
    let seen = sink_hits.load(Ordering::Relaxed);
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
    let _ca_contract = CA_CONTRACTS.lock().await;
    let srv = start().await;
    let (tx_stop, rx_stop) = watch::channel(false);
    let (dest, rx_leaked) = destination_sink(rx_stop).await;
    // The window outlives any budget the loop could spend (issue #102).
    let (addr, rx_first, sink_hits) =
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
    let seen = sink_hits.load(Ordering::Relaxed);
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

/// The per-PEM cache stays bounded while real concurrent SMTP sessions fan
/// out through more distinct TLS roots than the cache can retain. Every
/// DATA 250 is followed by an immediate real API count query for that fresh
/// inbox; every configured distinct sink completes verified TLS delivery.
#[test]
fn deliveries_through_more_pems_than_the_cache_cap_all_succeed() {
    // Explicit finite capacity: one async worker, 64 blocking slots, 33
    // distinct roots and eight SMTP sessions. No arbitrary-load bound.
    tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(64)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let _ca_contract = CA_CONTRACTS.lock().await;
            let srv = start().await;
            let count = swarmail::webhook::CA_CLIENT_CAP + 1;
            let concurrent_smtp = 8usize;
            let (status, before) = http_get_text(srv.http_addr, "/metrics").await;
            assert_eq!(status, 200);
            let builds_before = metric(&before, "swarmail_webhook_ca_cache_builds_total");
            let evictions_before = metric(&before, "swarmail_webhook_ca_cache_evictions_total");
            let delivered_before = metric(&before, "swarmail_webhook_delivered_total");

            // A real TLS server per root: its own CA, a leaf for 127.0.0.1, one
            // 204 per connection. Bound them all before registering any target.
            let mut sinks = Vec::new();
            for _ in 0..count {
                let (ca_pem, leaf, leaf_key) = mint_cert();
                let (addr, rx, hits) = tls_sink(
                    b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
                    leaf,
                    leaf_key,
                    SINK_LIFETIME,
                )
                .await;
                sinks.push((ca_pem, addr, rx, hits));
            }

            // Each target has its own normalized-URL delivery budget; the two
            // CA workers admit builds across all 33 roots. Eight SMTP sessions provide a real distinct-PEM
            // fan-out while remaining below every per-target delivery cap.
            let targets: Vec<serde_json::Value> = sinks
                .iter()
                .map(|(ca_pem, addr, _, _)| {
                    serde_json::json!({
                        "url": format!("https://127.0.0.1:{}/hook", addr.port()),
                        "ca_pem": ca_pem,
                    })
                })
                .collect();
            // Establish every cold root through real SMTP/TLS without global
            // admission pressure. The subsequent >32-root fan-out still churns
            // the 32-entry cache. Cold simultaneous overload has separate
            // refusal/accounting proofs; no fairness is promised under it.
            for (index, target) in targets.iter().enumerate() {
                let body = serde_json::to_string(&vec![target]).unwrap();
                let (status, _) =
                    http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&body)).await;
                assert_eq!(status, 200);
                assert!(
                    smtp_send(
                        srv.smtp_addr,
                        Some("warm-ca-fanout"),
                        "from@example.test",
                        "to@example.test",
                        "warm root",
                        "body"
                    )
                    .await
                    .unwrap()
                    .starts_with("250")
                );
                wait_for_metric_from(
                    srv.http_addr,
                    "swarmail_webhook_delivered_total",
                    delivered_before,
                    (index + 1) as u64,
                )
                .await;
            }
            let body = serde_json::to_string(&targets).unwrap();
            let (st, echo) = http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&body)).await;
            assert_eq!(st, 200, "{echo}");

            let mut sessions = tokio::task::JoinSet::new();
            for i in 0..concurrent_smtp {
                let inbox = format!("ca-fanout-{i}");
                let smtp_addr = srv.smtp_addr;
                let http_addr = srv.http_addr;
                sessions.spawn(async move {
                    let accepted = smtp_send(
                        smtp_addr,
                        Some(&inbox),
                        "from@example.test",
                        "to@example.test",
                        "ca fanout",
                        "body",
                    )
                    .await
                    .unwrap();
                    assert!(accepted.starts_with("250"), "{accepted}");
                    let (status, count) = http_json(
                        http_addr,
                        "GET",
                        &format!("/api/v1/inboxes/{inbox}/count"),
                        None,
                    )
                    .await;
                    assert_eq!(status, 200, "immediate query after DATA 250: {count}");
                    assert_eq!(count["count"], 1, "DATA 250 means queryable: {count}");
                });
            }
            while let Some(done) = sessions.join_next().await {
                done.unwrap();
            }

            let expected_deliveries = (count * (concurrent_smtp + 1)) as u64;
            wait_for_metric_from(
                srv.http_addr,
                "swarmail_webhook_delivered_total",
                delivered_before,
                expected_deliveries,
            )
            .await;
            for (i, (_, _, rx, sink_hits)) in sinks.iter_mut().enumerate() {
                let captured = tokio::time::timeout(Duration::from_secs(30), rx)
                    .await
                    .unwrap_or_else(|_| panic!("delivery {i} of {count} roots timed out"))
                    .unwrap();
                assert!(
                    captured.tls.contains("TLSv1"),
                    "delivery {i} was not over verified TLS: {captured:?}"
                );
                assert!(
                    captured.head.contains("\"event\":\"received\""),
                    "delivery {i} carried no payload: {captured:?}"
                );
                assert_eq!(
                    sink_hits.load(Ordering::Relaxed),
                    concurrent_smtp + 1,
                    "sink {i} receives its warm-up and each fan-out mail"
                );
            }

            let (st, metrics) = http_get_text(srv.http_addr, "/metrics").await;
            assert_eq!(st, 200);
            assert_eq!(
                metric(&metrics, "swarmail_webhook_ca_cache_entries"),
                swarmail::webhook::CA_CLIENT_CAP as u64,
                "the cache remains exactly at its {count}-root cap"
            );
            assert!(
                metric(&metrics, "swarmail_webhook_ca_cache_evictions_total") > evictions_before,
                "distinct roots beyond the cap evict entries:\n{metrics}"
            );
            assert!(
                metric(&metrics, "swarmail_webhook_ca_cache_builds_total")
                    >= builds_before + count as u64,
                "every distinct root has a successful delivery-side build:\n{metrics}"
            );
        });
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
    let _ca_contract = CA_CONTRACTS.lock().await;
    let srv = start().await;
    let (ca_pem, leaf, leaf_key) = mint_cert();
    let (addr, rx, _hits) = tls_sink_cfg(
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
    let _ca_contract = CA_CONTRACTS.lock().await;
    let srv = start().await;
    let (ca_pem, leaf, leaf_key) = mint_cert();
    // The window outlives everything the two passes could spend; the count
    // is read by signal below (issue #102).
    let (addr, rx, sink_hits) = tls_sink(
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
    let hits_seen = sink_hits.load(Ordering::Relaxed);
    assert_eq!(hits_seen, 2, "both passes must reach the sink over TLS");
}

/// A `ca_pem` whose base64 does not decode: the PUT refuses with 400,
/// naming the defect through the delivery TLS stack itself (decision 0029
/// §5) — a misconfig is discovered at the door, never at delivery time,
/// and nothing is applied.
#[tokio::test]
async fn garbage_ca_pem_is_refused_at_put_and_nothing_is_applied() {
    let _ca_contract = CA_CONTRACTS.lock().await;
    let srv = start().await;
    // The sink exists only to prove the refused target never dialed: the
    // GET below is the structural proof that nothing was applied, so no
    // delivery can ever be armed for the mail and no connection can ever
    // be made — the zero is read after the mail is stored, with no window
    // to race (issue #102).
    let pair = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
    let (addr, _rx_req, sink_hits) = tls_sink(
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
    assert_eq!(
        sink_hits.load(Ordering::Relaxed),
        0,
        "the sink never saw a connection"
    );

    // The server still serves (the failure was contained to the PUT).
    let (st, body) = http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(st, 200);
    assert_eq!(body["count"], 1);
}

// ---------- the delivery bound (AC4/AC5/AC6/AC7) ----------

/// A zero window accepts no connection. The listener closure owns the
/// capture sender, and it creates per-connection tasks only after accept;
/// awaiting capture cancellation therefore observes listener closure before
/// the live hit count is read.
#[tokio::test]
async fn a_window_closing_before_any_attempt_observes_zero() {
    let (_, capture, hits) = http_sink(204, Duration::ZERO).await;
    assert!(
        tokio::time::timeout(Duration::from_secs(15), capture)
            .await
            .expect("the HTTP listener closes its capture channel")
            .is_err(),
        "no HTTP request was captured"
    );
    assert_eq!(
        hits.load(Ordering::Relaxed),
        0,
        "no HTTP connection arrived"
    );

    let (_, capture, hits) =
        redirect_sink(302, "http://127.0.0.1:1/nowhere".into(), Duration::ZERO).await;
    assert!(
        tokio::time::timeout(Duration::from_secs(15), capture)
            .await
            .expect("the redirect listener closes its capture channel")
            .is_err(),
        "no redirect request was captured"
    );
    assert_eq!(
        hits.load(Ordering::Relaxed),
        0,
        "no redirect connection arrived"
    );

    let (ca_pem, leaf, leaf_key) = mint_cert();
    let (_, capture, hits) = tls_sink_cfg(
        b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
        leaf,
        leaf_key,
        Duration::ZERO,
        0,
    )
    .await;
    drop(ca_pem);
    assert!(
        tokio::time::timeout(Duration::from_secs(15), capture)
            .await
            .expect("the TLS listener closes its capture channel")
            .is_err(),
        "no TLS request was captured"
    );
    assert_eq!(hits.load(Ordering::Relaxed), 0, "no TLS connection arrived");
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
    let _ca_contract = CA_CONTRACTS.lock().await;
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
    let mut expected = Vec::with_capacity(50);
    for i in 0..50 {
        let subject = format!("storm {i:02}");
        let reply = smtp_send(
            srv.smtp_addr,
            Some("stormbox"),
            "f@x.io",
            "storm@x.io",
            &subject,
            "x",
        )
        .await
        .unwrap_or_else(|e| panic!("mail {i} refused: {e}"));
        assert!(reply.starts_with("250"), "mail {i} refused: {reply}");
        expected.push(stored_fixture("storm@x.io", &subject, "x"));
    }
    assert_stored_fixtures(srv.http_addr, "stormbox", expected).await;

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
    let _ca_contract = CA_CONTRACTS.lock().await;
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
    let _ca_contract = CA_CONTRACTS.lock().await;
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
    let _ca_contract = CA_CONTRACTS.lock().await;
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
    let mut expected = Vec::with_capacity(total);
    for i in 0..total {
        let subject = format!("two sinks {i:02}");
        smtp_send(
            srv.smtp_addr,
            Some("twosink"),
            "f@x.io",
            "two@x.io",
            &subject,
            "x",
        )
        .await
        .unwrap_or_else(|e| panic!("mail {i} refused: {e}"));
        expected.push(stored_fixture("two@x.io", &subject, "x"));
    }
    assert_stored_fixtures(srv.http_addr, "twosink", expected).await;

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
    let _ca_contract = CA_CONTRACTS.lock().await;
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
    let _ca_contract = CA_CONTRACTS.lock().await;
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
    let _ca_contract = CA_CONTRACTS.lock().await;
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
    let _ca_contract = CA_CONTRACTS.lock().await;
    let srv = start().await;
    let (addr, rx, _hits) = http_sink(204, SINK_LIFETIME).await;
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

/// Privacy proofs must never render a response, parser error or equality
/// operand: negative controls deliberately put real generated key bytes on
/// this wire. Transport/JSON failures use fixed diagnostics too.
async fn private_webhook_json(
    addr: std::net::SocketAddr,
    method: reqwest::Method,
    body: Option<&str>,
) -> (u16, serde_json::Value) {
    let client = reqwest::Client::new();
    let mut request = client.request(method, format!("http://{addr}/api/v1/webhooks"));
    if let Some(body) = body {
        request = request
            .header("content-type", "application/json")
            .body(body.to_owned());
    }
    let response = request
        .send()
        .await
        .unwrap_or_else(|_| panic!("privacy transport failed"));
    let status = response.status().as_u16();
    let bytes = response
        .bytes()
        .await
        .unwrap_or_else(|_| panic!("privacy response read failed"));
    let value = private_response_json(&bytes);
    (status, value)
}

fn private_body_sentinel(body: &str) -> &str {
    body.get(64..96).expect("generated private-body sentinel")
}

fn private_response_json(bytes: &[u8]) -> serde_json::Value {
    serde_json::from_slice(bytes).unwrap_or_else(|_| panic!("privacy response is not JSON"))
}

// One assertion owner for the refusal wire proof and its diagnostic controls.
// No response, parser text or installed-state operand reaches a panic message.
fn assert_private_ca_refusal(
    status: u16,
    response: &serde_json::Value,
    index: usize,
    sentinel: &str,
    preserved: bool,
) {
    assert!(status == 400, "private CA refusal status");
    assert!(
        response["error"].as_str().is_some_and(|error| {
            error.starts_with(&format!(
        "webhook target {index}: webhook ca_pem could not be loaded into the rustls client: "
    ))
        }),
        "private CA refusal prefix"
    );
    assert!(
        !response.to_string().contains(sentinel),
        "private DER escaped in validation error"
    );
    assert!(preserved, "private CA refusal changed installed state");
}

/// Noncanonical CRLF and 37-column wrapping must not survive projection.
fn rewrap_certificate(pem: &str) -> String {
    let body = pem
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect::<String>();
    let lines = body
        .as_bytes()
        .chunks(37)
        .map(|line| std::str::from_utf8(line).unwrap())
        .collect::<Vec<_>>()
        .join("\r\n");
    format!("-----BEGIN CERTIFICATE-----\r\n{lines}\r\n-----END CERTIFICATE-----\r\n")
}

/// Unauthenticated GET/PUT project canonical DER in order, including
/// duplicates, without disclosing any other input. Boolean comparisons keep
/// the failure path private even with a deliberately broken projection.
#[tokio::test]
async fn private_keys_in_ca_pem_never_appear_in_get_or_put_responses() {
    let _ca_contract = CA_CONTRACTS.lock().await;
    let srv = start().await;
    let first = unrelated_ca_pem();
    let second = unrelated_ca_pem();
    let key = rcgen::KeyPair::generate().unwrap().serialize_pem();
    let key_body = key
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect::<String>();
    // Pin generated private bytes rather than the common PKCS#8 header.
    let sentinel = private_body_sentinel(&key_body);
    let rsa =
        format!("-----BEGIN RSA PRIVATE KEY-----\n{key_body}\n-----END RSA PRIVATE KEY-----\n");
    let ec = format!("-----BEGIN EC PRIVATE KEY-----\n{key_body}\n-----END EC PRIVATE KEY-----\n");
    let encrypted = format!(
        "-----BEGIN ENCRYPTED PRIVATE KEY-----\n{key_body}\n-----END ENCRYPTED PRIVATE KEY-----\n"
    );
    let unrelated = "-----BEGIN OPERATOR DATA-----\ncHJpdmF0ZS11bnJlbGF0ZWQtc2VudGluZWw=\n-----END OPERATOR DATA-----\n";
    let mixed = format!(
        "operator-before\n{key}{}{rsa}operator-between\n{second}{unrelated}{}{encrypted}{ec}operator-after\n",
        rewrap_certificate(&first),
        rewrap_certificate(&first)
    );
    let canonical = format!("{first}{second}{first}");
    let targets = serde_json::json!([
        {"url": "http://user:password-sentinel@127.0.0.1:9/hook", "inbox": "private-key-echo", "secret": "secret-sentinel", "ca_pem": mixed},
        {"url": "http://127.0.0.1:9/absent"}
    ]).to_string();
    let (status, put) =
        private_webhook_json(srv.http_addr, reqwest::Method::PUT, Some(&targets)).await;
    assert_eq!(status, 200, "private mixed PEM admission");
    let (status, get) = private_webhook_json(srv.http_addr, reqwest::Method::GET, None).await;
    assert_eq!(status, 200, "private mixed PEM GET");
    for response in [&put, &get] {
        assert!(
            response.as_array().is_some_and(|list| list.len() == 2),
            "response list shape"
        );
        let text = response.to_string();
        for forbidden in [
            "PRIVATE KEY",
            sentinel,
            "OPERATOR DATA",
            "cHJpdmF0ZS11bnJlbGF0ZWQtc2VudGluZWw=",
            "operator-before",
            "operator-between",
            "operator-after",
            "password-sentinel",
            "secret-sentinel",
        ] {
            assert!(
                !text.contains(forbidden),
                "private or unrelated input disclosed"
            );
        }
        for target in response.as_array().unwrap() {
            assert!(
                target.as_object().is_some_and(|object| object.len() == 4
                    && ["url", "inbox", "secret", "ca_pem"]
                        .iter()
                        .all(|key| object.contains_key(*key))),
                "four response fields"
            );
            assert!(target["secret"].is_null(), "secret withheld");
        }
        assert!(
            response[0]["url"] == "http://127.0.0.1:9/hook",
            "userinfo removed"
        );
        assert!(
            response[0]["inbox"] == "private-key-echo",
            "inbox preserved"
        );
        assert!(
            response[0]["ca_pem"] == canonical,
            "canonical order and duplicates"
        );
        let shown = response[0]["ca_pem"].as_str().expect("public string");
        let decoded = rustls_pemfile::certs(&mut shown.as_bytes())
            .collect::<Result<Vec<_>, _>>()
            .expect("public certificates decode");
        let expected = rustls_pemfile::certs(&mut canonical.as_bytes())
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(decoded == expected, "DER order and duplicates preserved");
        assert!(
            response[1]["ca_pem"].is_null() && response[1]["inbox"].is_null(),
            "absent options stay null"
        );
    }
    let raw = srv.webhooks.current();
    assert!(
        raw[0].ca_pem.as_deref() == Some(mixed.as_str()),
        "installed PEM unchanged"
    );
    assert!(
        raw[0].secret.as_deref() == Some("secret-sentinel"),
        "installed secret unchanged"
    );
    assert!(
        raw[0].url == "http://user:password-sentinel@127.0.0.1:9/hook",
        "installed URL unchanged"
    );

    srv.webhooks.set(vec![swarmail::webhook::WebhookTarget {
        url: "http://127.0.0.1:9/direct".into(),
        inbox: None,
        secret: None,
        ca_pem: Some(format!("{key}{first}")),
    }]);
    let (status, direct) = private_webhook_json(srv.http_addr, reqwest::Method::GET, None).await;
    assert_eq!(status, 200);
    assert!(
        !direct.to_string().contains("PRIVATE KEY"),
        "direct key label withheld"
    );
    assert!(
        !direct.to_string().contains(sentinel),
        "direct key body withheld"
    );
    assert!(direct[0]["ca_pem"] == first, "direct public certificate");
}

/// Direct installation bypasses admission; any parse/load failure empties
/// the whole projection, including a valid prefix and relabeled private DER.
#[tokio::test]
async fn direct_webhook_install_projects_unloadable_ca_as_empty_on_get() {
    let _ca_contract = CA_CONTRACTS.lock().await;
    let srv = start().await;
    let good = unrelated_ca_pem();
    let key = rcgen::KeyPair::generate().unwrap();
    let invalid = [
        "".to_string(),
        "operator note without a certificate".to_string(),
        "-----BEGIN CERTIFICATE-----\n!!!\n-----END CERTIFICATE-----\n".to_string(),
        "-----BEGIN CERTIFICATE-----\n!!!".to_string(),
        format!("{good}{}", framed_pem(b"invalid DER")),
        format!("{good}-----BEGIN CERTIFICATE-----\n!!!\n-----END CERTIFICATE-----\n"),
        framed_pem(&key.serialize_der()),
        format!("{good}{}", framed_pem(&key.serialize_der())),
    ];
    for (index, ca_pem) in invalid.into_iter().enumerate() {
        let target = swarmail::webhook::WebhookTarget {
            url: "http://127.0.0.1:9/direct-invalid".into(),
            inbox: Some(format!("direct-invalid-{index}")),
            secret: None,
            ca_pem: Some(ca_pem.clone()),
        };
        assert!(
            target.redacted().ca_pem.as_deref() == Some(""),
            "synchronous fail-closed case {index}"
        );
        srv.webhooks.set(vec![target]);
        let (status, response) =
            private_webhook_json(srv.http_addr, reqwest::Method::GET, None).await;
        assert_eq!(status, 200, "unprojectable GET case {index}");
        assert!(response[0]["ca_pem"] == "", "empty projection case {index}");
        assert!(
            response[0]["inbox"] == format!("direct-invalid-{index}"),
            "inbox case {index}"
        );
        assert!(response[0]["secret"].is_null(), "secret null case {index}");
        assert!(
            srv.webhooks.current()[0].ca_pem.as_deref() == Some(ca_pem.as_str()),
            "raw input preserved case {index}"
        );
    }
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

fn without_json_location(text: &str) -> &str {
    text.split_once(" at line ").map_or(text, |(head, _)| head)
}

/// A prior target carrying URL credentials and a secret, filtered to
/// `inbox` and pointed at a fresh sink: the configuration every refusal
/// below must leave whole.
async fn install_prior(srv: &swarmail::RunningServer, inbox: &str) -> Receiver<String> {
    let (addr, rx, _hits) = http_sink(204, SINK_LIFETIME).await;
    let prior = serde_json::json!([{
        "url": format!("http://user:hunter2@{addr}/prior"),
        "inbox": inbox,
        "secret": "kept-secret",
    }])
    .to_string();
    let (st, _) = private_webhook_json(srv.http_addr, reqwest::Method::PUT, Some(&prior)).await;
    assert!(st == 200, "prior webhook installation");
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
    let _ca_contract = CA_CONTRACTS.lock().await;
    let srv = start().await;
    let rx = install_prior(&srv, "ca122").await;
    let before = installed(&srv);
    let good = unrelated_ca_pem();
    let private_der = rcgen::KeyPair::generate().unwrap().serialize_der();
    use base64::Engine as _;
    let private_body = base64::engine::general_purpose::STANDARD.encode(&private_der);
    let private_sentinel = private_body_sentinel(&private_body);
    let mislabeled_private_key = framed_pem(&private_der);
    let malformed = "-----BEGIN CERTIFICATE-----\n!!!not base64!!!\n-----END CERTIFICATE-----\n";
    for (_case, pem) in [
        ("no certificate section", "not a pem".to_string()),
        ("empty certificate", framed_pem(b"")),
        ("malformed base64", malformed.to_string()),
        ("invalid DER", framed_pem(b"not a DER certificate")),
        (
            "private key DER relabeled as certificate",
            mislabeled_private_key,
        ),
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
                    private_webhook_json(srv.http_addr, reqwest::Method::PUT, Some(&body)).await;
                assert_private_ca_refusal(
                    st,
                    &resp,
                    index,
                    private_sentinel,
                    installed(&srv) == before,
                );
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
    let _ca_contract = CA_CONTRACTS.lock().await;
    let srv = start().await;
    let rx = install_prior(&srv, "keys135").await;
    let before = installed(&srv);
    let (_, echo) = http_json(srv.http_addr, "GET", "/api/v1/webhooks", None).await;
    assert_eq!(echo[0]["secret"], serde_json::Value::Null, "{echo}");
    assert_eq!(echo[0]["ca_pem"], serde_json::Value::Null, "{echo}");

    let fine = r#"{"url": "http://127.0.0.1:9/fine"}"#;
    for (body, expected) in [
        (
            r#"{"url":"http://127.0.0.1:9/root"}"#,
            "invalid webhooks body: Failed to deserialize the JSON body into the target type: expected a JSON array of webhook targets, found an object",
        ),
        (
            "null",
            "invalid webhooks body: Failed to deserialize the JSON body into the target type: expected a JSON array of webhook targets, found null",
        ),
        (
            r#""root string""#,
            r#"invalid webhooks body: Failed to deserialize the JSON body into the target type: expected a JSON array of webhook targets, found a string"#,
        ),
    ] {
        let (status, response) =
            http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(body)).await;
        assert_eq!(status, 400, "{body}: {response}");
        assert_eq!(
            without_json_location(response["error"].as_str().unwrap()),
            expected,
            "{body}: {response}"
        );
        assert!(
            !response.to_string().contains("hunter2")
                && !response.to_string().contains("kept-secret"),
            "root shape refusal does not reveal installed credentials: {response}"
        );
        assert_eq!(installed(&srv), before, "{body}");
    }

    let bad_rows = [
        (
            r#"{"url": "http://127.0.0.1:9/x", "secrt": "DO_NOT_ECHO_181"}"#,
            "[1].secrt: unknown field `secrt`, expected one of `url`, `inbox`, `secret`, `ca_pem`",
        ),
        (
            r#"{"url": [], "secrt": "DO_NOT_ECHO_181"}"#,
            "[1].url: expected a string, found an array",
        ),
        (
            r#"{"secrt": "DO_NOT_ECHO_181", "url": []}"#,
            "[1].secrt: unknown field `secrt`, expected one of `url`, `inbox`, `secret`, `ca_pem`",
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
        (
            r#"{"url": []}"#,
            "[1].url: expected a string, found an array",
        ),
        (
            r#"{"url": {}}"#,
            "[1].url: expected a string, found an object",
        ),
        (r#"{"url": null}"#, "[1].url: expected a string, found null"),
        (
            r#"{"url": "http://127.0.0.1:9/x", "inbox": []}"#,
            "[1].inbox: expected a string, found an array",
        ),
        (
            r#"{"url": "http://127.0.0.1:9/x", "inbox": {}}"#,
            "[1].inbox: expected a string, found an object",
        ),
        (
            r#"{"url": "http://127.0.0.1:9/x", "inbox": ["DO_NOT_ECHO_181"]}"#,
            "[1].inbox: expected a string, found an array",
        ),
        (
            r#"{"url": "http://127.0.0.1:9/x", "secret": []}"#,
            "[1].secret: expected a string, found an array",
        ),
        (
            r#"{"url": "http://127.0.0.1:9/x", "secret": {}}"#,
            "[1].secret: expected a string, found an object",
        ),
        (
            r#"{"url": "http://127.0.0.1:9/x", "secret": ["DO_NOT_ECHO_181"]}"#,
            "[1].secret: expected a string, found an array",
        ),
        (
            r#"{"url": "http://127.0.0.1:9/x", "ca_pem": []}"#,
            "[1].ca_pem: expected a string, found an array",
        ),
        (
            r#"{"url": "http://127.0.0.1:9/x", "ca_pem": {}}"#,
            "[1].ca_pem: expected a string, found an object",
        ),
        (
            r#"{"url": "http://127.0.0.1:9/x", "ca_pem": ["DO_NOT_ECHO_181"]}"#,
            "[1].ca_pem: expected a string, found an array",
        ),
        (
            "[]",
            "[1]: expected a webhook target object, found an array",
        ),
        (
            "42",
            "[1]: expected a webhook target object, found a number",
        ),
        ("null", "[1]: expected a webhook target object, found null"),
        (
            r#""http://user:hunter2@127.0.0.1:9/x""#,
            "[1]: expected a webhook target object, found a string",
        ),
        (r#"{"inbox": "a"}"#, "[1]: missing field `url`"),
    ];
    for (bad, path_and_diagnostic) in bad_rows {
        let body = format!("[{fine}, {bad}]");
        let (status, response) =
            http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&body)).await;
        assert_eq!(status, 400, "{bad}: {response}");
        let error = response["error"].as_str().unwrap();
        let expected = format!(
            "invalid webhooks body: Failed to deserialize the JSON body into the target type: {path_and_diagnostic}"
        );
        assert_eq!(without_json_location(error), expected, "{bad}: {response}");
        assert!(
            !response.to_string().contains("hunter2")
                && !response.to_string().contains("kept-secret")
                && !response.to_string().contains("DO_NOT_ECHO_181"),
            "data refusal does not disclose credentials or values: {response}"
        );
        assert_eq!(installed(&srv), before, "{bad}");
    }

    // Duplicate keys must be raw request text: a JSON value builder would
    // erase the order-sensitive failure this row preserves.
    let duplicate = format!(
        "[{fine}, {{\"url\":\"http://127.0.0.1:9/first\",\"url\":\"http://127.0.0.1:9/second\"}}]"
    );
    let (status, response) =
        http_json(srv.http_addr, "PUT", "/api/v1/webhooks", Some(&duplicate)).await;
    assert_eq!(status, 400, "{duplicate}: {response}");
    assert_eq!(
        without_json_location(response["error"].as_str().unwrap()),
        "invalid webhooks body: Failed to deserialize the JSON body into the target type: [1]: duplicate field `url`",
        "{response}"
    );
    assert_eq!(
        installed(&srv),
        before,
        "duplicate target field replaced nothing"
    );

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
    let _ca_contract = CA_CONTRACTS.lock().await;
    let srv = start().await;
    let (second, leaf, leaf_key) = mint_cert();
    let (addr, rx, _hits) = tls_sink(
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
    let _ca_contract = CA_CONTRACTS.lock().await;
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

// Capture every deliberately failing diagnostic in a child process. Only
// boolean verdicts leave the parent; captured private bytes are never logged
// or written to receipts, even when the diagnostic regression is detected.
async fn private_diagnostic_control(mode: &str, expected: &str) {
    use base64::Engine as _;
    let key = base64::engine::general_purpose::STANDARD
        .encode(rcgen::KeyPair::generate().unwrap().serialize_der());
    let output = tokio::time::timeout(
        common::HELPER_DEADLINE,
        tokio::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "private_failure_diagnostic_probe",
                "--ignored",
                "--nocapture",
            ])
            .env("SWARMAIL_PRIVATE_DIAGNOSTIC_MODE", mode)
            .env("SWARMAIL_PRIVATE_DIAGNOSTIC_BODY", &key)
            .kill_on_drop(true)
            .output(),
    )
    .await
    .expect("private diagnostic child deadline")
    .expect("private diagnostic child launch");
    let text = [output.stdout, output.stderr].concat();
    let text = String::from_utf8_lossy(&text);
    assert!(
        output.status.code() == Some(101),
        "diagnostic control must fail"
    );
    assert!(
        text.contains(expected),
        "diagnostic control failed for another reason"
    );
    for forbidden in [
        &key,
        &key[64..96],
        "PRIVATE KEY",
        "invalid type",
        "expected value at line",
    ] {
        assert!(
            !text.contains(forbidden),
            "private diagnostic output was not value-free"
        );
    }
}

#[tokio::test]
async fn webhook_private_failure_diagnostics_are_value_free() {
    for (mode, expected) in [
        ("status", "private CA refusal status"),
        ("prefix", "private CA refusal prefix"),
        ("private", "private DER escaped in validation error"),
        ("state", "private CA refusal changed installed state"),
        ("private-json", "privacy response is not JSON"),
    ] {
        private_diagnostic_control(mode, expected).await;
    }
}

async fn private_diagnostic_peer(status: u16, body: String) -> std::net::SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut peer, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            request.push(peer.read_u8().await.unwrap());
        }
        let response = format!(
            "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        peer.write_all(response.as_bytes()).await.unwrap();
        peer.shutdown().await.unwrap();
    });
    addr
}

#[tokio::test]
async fn http_json_rejects_non_json_without_body_diagnostics() {
    private_diagnostic_control("common-json", "status 200 body is not JSON").await;
    for status in [204, 304] {
        let addr = private_diagnostic_peer(status, String::new()).await;
        let (observed, body) = http_json(addr, "GET", "/api/v1/webhooks", None).await;
        assert_eq!(observed, status);
        assert!(body.is_null(), "empty no-content is absence");
    }
    private_diagnostic_control("empty-json", "status 200 body is not JSON").await;
}

#[tokio::test]
#[ignore = "private diagnostic child, invoked only with in-memory output capture"]
async fn private_failure_diagnostic_probe() {
    let mode = std::env::var("SWARMAIL_PRIVATE_DIAGNOSTIC_MODE").unwrap();
    let key = std::env::var("SWARMAIL_PRIVATE_DIAGNOSTIC_BODY").unwrap();
    let sentinel = private_body_sentinel(&key);
    let private_body = format!("-----BEGIN PRIVATE KEY-----\n{key}\n-----END PRIVATE KEY-----");
    match mode.as_str() {
        "common-json" | "empty-json" => {
            let body = if mode == "empty-json" {
                String::new()
            } else {
                private_body
            };
            let addr = private_diagnostic_peer(200, body).await;
            http_json(addr, "GET", "/api/v1/webhooks", None).await;
        }
        "private-json" => {
            private_response_json(private_body.as_bytes());
        }
        _ => {
            let prefix =
                "webhook target 0: webhook ca_pem could not be loaded into the rustls client: ";
            let error = if mode == "prefix" {
                private_body
            } else if mode == "private" {
                // A body-only leak discriminates a private-body sentinel
                // from the shared PKCS#8 header, even without a PEM label.
                format!("{prefix}{}", &key[64..96])
            } else if mode == "status" {
                format!("{prefix}{private_body}")
            } else {
                prefix.to_owned()
            };
            let response = serde_json::json!({"error": error});
            assert_private_ca_refusal(
                if mode == "status" { 200 } else { 400 },
                &response,
                0,
                sentinel,
                mode != "state",
            );
        }
    }
    panic!("diagnostic probe did not fail");
}
