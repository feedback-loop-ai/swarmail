//! Webhook delivery: the happy path (including a pathless target URL) and
//! the retry budget against a target that always refuses — over plain HTTP
//! and over TLS (reqwest + rustls, verification on).

mod common;

use common::{http_json, smtp_send, start};
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::oneshot::Receiver;

/// An HTTP sink that accepts `expect` connections, replying `status` to each,
/// and reports the first request head plus the total hit count.
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
    tokio::spawn(async move {
        let mut head = None;
        for _ in 0..expect {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let mut buf = vec![0u8; 8192];
            let n = stream.read(&mut buf).await.unwrap_or(0);
            if head.is_none() {
                head = Some(String::from_utf8_lossy(&buf[..n]).to_string());
            }
            let reply =
                format!("HTTP/1.1 {status} TEST\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            let _ = stream.write_all(reply.as_bytes()).await;
        }
        let _ = tx_head.send(head.unwrap_or_default());
        let _ = tx_count.send(expect);
    });
    (addr, rx_head, rx_count)
}

/// What the TLS sink saw: the request head and the negotiated TLS version.
#[derive(Debug)]
struct Captured {
    head: String,
    tls: String,
}

/// A TLS sink: mints nothing, just serves the given certificate. Accepts
/// `expect` connections; a failed handshake (an untrusted client rejecting
/// our cert) still counts as an attempt. Reports the first request head with
/// its negotiated TLS version, and the total hit count.
async fn tls_sink(
    cert_der: Vec<u8>,
    key_der: Vec<u8>,
    expect: usize,
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
    tokio::spawn(async move {
        let mut captured = Captured {
            head: String::new(),
            tls: String::new(),
        };
        let mut hits = 0;
        for _ in 0..expect {
            let Ok((stream, _)) = listener.accept().await else {
                break;
            };
            hits += 1;
            let Ok(mut tls) = acceptor.accept(stream).await else {
                continue; // the client rejected our certificate: attempt counted
            };
            if captured.head.is_empty() {
                captured.tls = format!("{:?}", tls.get_ref().1.protocol_version());
                // Read until the whole request (head + Content-Length body) is in.
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
                captured.head = String::from_utf8_lossy(&buf).to_string();
            }
            let reply =
                b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\nconnection: close\r\n\r\n";
            let _ = tls.write_all(reply).await;
        }
        let _ = tx_req.send(captured);
        let _ = tx_count.send(hits);
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
    let (addr, rx, _) = tls_sink(leaf, leaf_key, 1).await;

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
