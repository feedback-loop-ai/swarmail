//! Cold-path concurrency proofs for webhook CA construction (decision 0020).
use super::*;

/// A fresh self-signed CA for this test module; every call is a distinct key.
fn ca_pem() -> String {
    let key = rcgen::KeyPair::generate().unwrap();
    let mut params = rcgen::CertificateParams::new(Vec::<String>::new()).unwrap();
    params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    params.key_usages = vec![rcgen::KeyUsagePurpose::KeyCertSign];
    params.self_signed(&key).unwrap().pem()
}

/// A one-request TLS server with a per-run root and a leaf for loopback.
async fn tls_sink_requests(
    requests: usize,
) -> (
    String,
    std::net::SocketAddr,
    tokio::task::JoinHandle<Vec<Vec<u8>>>,
) {
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

    let ca_key = rcgen::KeyPair::generate().unwrap();
    let mut ca_params = rcgen::CertificateParams::new(Vec::<String>::new()).unwrap();
    ca_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    ca_params.key_usages = vec![rcgen::KeyUsagePurpose::KeyCertSign];
    let ca = ca_params.self_signed(&ca_key).unwrap();
    let leaf_key = rcgen::KeyPair::generate().unwrap();
    let mut leaf_params = rcgen::CertificateParams::new(Vec::<String>::new()).unwrap();
    leaf_params
        .subject_alt_names
        .push(rcgen::SanType::IpAddress(std::net::IpAddr::from([
            127, 0, 0, 1,
        ])));
    leaf_params.key_usages = vec![rcgen::KeyUsagePurpose::DigitalSignature];
    let leaf = leaf_params.signed_by(&leaf_key, &ca, &ca_key).unwrap();
    let server_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from(leaf.der().to_vec())],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(leaf_key.serialize_der())),
        )
        .unwrap();
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let mut bodies = Vec::new();
        while bodies.len() < requests {
            let (stream, _) = listener.accept().await.unwrap();
            let tls = acceptor.accept(stream).await.unwrap();
            let mut reader = BufReader::new(tls);
            // In the duplicate-miss proof the loser must reuse the winner's
            // client, including its idle TLS connection. A private losing
            // client opens another connection and cannot complete request 2.
            let on_connection = if bodies.is_empty() && requests > 1 {
                2
            } else {
                1
            };
            for index in 0..on_connection {
                let mut length = 0usize;
                loop {
                    let mut line = String::new();
                    assert_ne!(
                        reader.read_line(&mut line).await.unwrap(),
                        0,
                        "request headers end"
                    );
                    if line == "\r\n" {
                        break;
                    }
                    if let Some(value) = line.strip_prefix("content-length: ") {
                        length = value.trim().parse().unwrap();
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).await.unwrap();
                let response: &[u8] = if index + 1 == on_connection {
                    b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
                } else {
                    b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\n\r\n"
                };
                reader.get_mut().write_all(response).await.unwrap();
                bodies.push(body);
            }
        }
        bodies
    });
    (ca.pem(), addr, task)
}

async fn tls_sink_once() -> (
    String,
    std::net::SocketAddr,
    tokio::task::JoinHandle<Vec<u8>>,
) {
    let (pem, addr, task) = tls_sink_requests(1).await;
    (
        pem,
        addr,
        tokio::spawn(async move { task.await.unwrap().remove(0) }),
    )
}

async fn test_server() -> crate::RunningServer {
    crate::run_on(&crate::config::Config {
        smtp_listen: "127.0.0.1:0".into(),
        http_listen: "127.0.0.1:0".into(),
        pop3_listen: "127.0.0.1:0".into(),
        max_per_inbox: 0,
        data_file: None,
        tls_cert: None,
        tls_key: None,
        smtp: Default::default(),
        pop3_idle_timeout: crate::pop3::DEFAULT_IDLE_TIMEOUT,
    })
    .await
    .unwrap()
}

async fn api_json(
    server: &crate::RunningServer,
    method: reqwest::Method,
    targets: Option<&[WebhookTarget]>,
) -> (reqwest::StatusCode, serde_json::Value) {
    let mut request = reqwest::Client::new().request(
        method,
        format!("http://{}/api/v1/webhooks", server.http_addr),
    );
    if let Some(targets) = targets {
        let mut value = serde_json::to_value(targets).unwrap();
        for target in value.as_array_mut().unwrap() {
            target
                .as_object_mut()
                .unwrap()
                .retain(|_, value| !value.is_null());
        }
        request = request
            .header("content-type", "application/json")
            .body(value.to_string());
    }
    let response = request
        .send()
        .await
        .unwrap_or_else(|_| panic!("webhook API transport"));
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .unwrap_or_else(|_| panic!("webhook API response read"));
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or_else(|_| panic!("webhook API JSON")),
    )
}

/// Cleanup runs even after an assertion: no failed proof can strand a
/// blocking job in runtime teardown. A watchdog release always fails proof.
struct BuildWatchdog {
    releases: Vec<std::sync::mpsc::Sender<()>>,
    done: std::sync::mpsc::Sender<()>,
    thread: Option<std::thread::JoinHandle<()>>,
    fired: Arc<std::sync::atomic::AtomicBool>,
}
impl BuildWatchdog {
    fn new(releases: Vec<std::sync::mpsc::Sender<()>>) -> Self {
        let (done, observed) = std::sync::mpsc::channel();
        let fired = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = fired.clone();
        let cleanup = releases.clone();
        let thread = std::thread::spawn(move || {
            if observed.recv_timeout(Duration::from_secs(20)).is_err() {
                flag.store(true, Ordering::SeqCst);
                for release in cleanup {
                    let _ = release.send(());
                }
            }
        });
        Self {
            releases,
            done,
            thread: Some(thread),
            fired,
        }
    }
    fn assert_unused(&self) {
        assert!(
            !self.fired.load(Ordering::SeqCst),
            "watchdog was not the success path"
        );
    }
}
impl Drop for BuildWatchdog {
    fn drop(&mut self) {
        for release in &self.releases {
            let _ = release.send(());
        }
        let _ = self.done.send(());
        self.thread.take().unwrap().join().unwrap();
    }
}

/// Real SMTP exchange with AUTH selecting an isolated inbox. The final
/// DATA 250 follows synchronous Store::insert.
async fn smtp_send_for_inbox(addr: std::net::SocketAddr, inbox: &str) -> String {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let (read, mut write) = stream.into_split();
    let mut reader = BufReader::new(read);
    async fn reply(reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>) -> Vec<String> {
        let mut lines = Vec::new();
        loop {
            let mut line = String::new();
            assert_ne!(
                reader.read_line(&mut line).await.unwrap(),
                0,
                "SMTP reply line"
            );
            let more = line.as_bytes().get(3) == Some(&b'-');
            lines.push(line);
            if !more {
                return lines;
            }
        }
    }
    assert!(reply(&mut reader).await[0].starts_with("220"));
    write
        .write_all(b"EHLO ca-construction-test\r\n")
        .await
        .unwrap();
    assert!(reply(&mut reader).await.last().unwrap().starts_with("250"));
    write.write_all(b"NOOP\r\n").await.unwrap();
    assert!(reply(&mut reader).await[0].starts_with("250"));
    let plain = format!("\u{0}{inbox}\u{0}test");
    use base64::Engine as _;
    let encoded = base64::engine::general_purpose::STANDARD.encode(plain);
    write
        .write_all(format!("AUTH PLAIN {encoded}\r\n").as_bytes())
        .await
        .unwrap();
    assert!(reply(&mut reader).await[0].starts_with("235"));
    for command in ["MAIL FROM:<from@example.test>", "RCPT TO:<to@example.test>"] {
        write
            .write_all(format!("{command}\r\n").as_bytes())
            .await
            .unwrap();
        assert!(reply(&mut reader).await[0].starts_with("250"));
    }
    write.write_all(b"DATA\r\n").await.unwrap();
    assert!(reply(&mut reader).await[0].starts_with("354"));
    write
        .write_all(
            b"From: from@example.test\r\nTo: to@example.test\r\nSubject: held\r\n\r\nbody\r\n.\r\n",
        )
        .await
        .unwrap();
    let accepted = reply(&mut reader).await.remove(0);
    write.write_all(b"QUIT\r\n").await.unwrap();
    accepted
}

async fn smtp_read_reply(
    reader: &mut tokio::io::BufReader<tokio::net::tcp::OwnedReadHalf>,
) -> Vec<String> {
    use tokio::io::AsyncBufReadExt;
    let mut lines = Vec::new();
    loop {
        let mut line = String::new();
        assert_ne!(
            reader.read_line(&mut line).await.unwrap(),
            0,
            "SMTP reply line"
        );
        let more = line.as_bytes().get(3) == Some(&b'-');
        lines.push(line);
        if !more {
            return lines;
        }
    }
}

/// Drive a complete SMTP DATA transaction but leave its final reply unread.
/// The caller can coordinate with an owner-local Store queue observation.
async fn smtp_send_data_waiting(
    addr: std::net::SocketAddr,
    inbox: &str,
) -> (
    tokio::io::BufReader<tokio::net::tcp::OwnedReadHalf>,
    tokio::net::tcp::OwnedWriteHalf,
) {
    use tokio::io::{AsyncWriteExt, BufReader};
    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let (read, mut write) = stream.into_split();
    let mut reader = BufReader::new(read);
    assert!(smtp_read_reply(&mut reader).await[0].starts_with("220"));
    write.write_all(b"EHLO ca-capacity-test\r\n").await.unwrap();
    assert!(
        smtp_read_reply(&mut reader)
            .await
            .last()
            .unwrap()
            .starts_with("250")
    );
    let plain = format!("\u{0}{inbox}\u{0}test");
    use base64::Engine as _;
    let encoded = base64::engine::general_purpose::STANDARD.encode(plain);
    write
        .write_all(format!("AUTH PLAIN {encoded}\r\n").as_bytes())
        .await
        .unwrap();
    assert!(smtp_read_reply(&mut reader).await[0].starts_with("235"));
    for command in ["MAIL FROM:<from@example.test>", "RCPT TO:<to@example.test>"] {
        write
            .write_all(format!("{command}\r\n").as_bytes())
            .await
            .unwrap();
        assert!(smtp_read_reply(&mut reader).await[0].starts_with("250"));
    }
    write.write_all(b"DATA\r\n").await.unwrap();
    assert!(smtp_read_reply(&mut reader).await[0].starts_with("354"));
    write
        .write_all(
            b"From: from@example.test\r\nTo: to@example.test\r\nSubject: saturated\r\n\r\nbody\r\n.\r\n",
        )
        .await
        .unwrap();
    (reader, write)
}

async fn inbox_count(client: &reqwest::Client, addr: std::net::SocketAddr, inbox: &str) -> u64 {
    let response = client
        .get(format!("http://{addr}/api/v1/inboxes/{inbox}/count"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let text = response.text().await.unwrap();
    let body: serde_json::Value = serde_json::from_str(&text).unwrap();
    body["count"].as_u64().unwrap()
}

/// Panics in GET/PUT preparation join as value-free 500 responses; a PUT
/// worker failure occurs before the single configuration replacement.
#[test]
fn api_preparation_panic_is_keyed_and_one_shot() {
    tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(4)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let _serial = CACHE_TESTS.lock().await;
            let config = crate::config::Config {
                smtp_listen: "127.0.0.1:0".into(),
                http_listen: "127.0.0.1:0".into(),
                pop3_listen: "127.0.0.1:0".into(),
                max_per_inbox: 0,
                data_file: None,
                tls_cert: None,
                tls_key: None,
                smtp: Default::default(),
                pop3_idle_timeout: crate::pop3::DEFAULT_IDLE_TIMEOUT,
            };
            let server = crate::run_on(&config).await.unwrap();
            let prior = WebhookTarget {
                url: "http://prior-user:prior-password@127.0.0.1:9/prior".into(),
                inbox: Some("prior-inbox".into()),
                secret: Some("prior-secret".into()),
                ca_pem: Some(format!(
                    "{}{}",
                    ca_pem(),
                    rcgen::KeyPair::generate().unwrap().serialize_pem()
                )),
            };
            server.webhooks.set(vec![prior.clone()]);
            let calls = server.webhooks.installations.load(Ordering::SeqCst);
            let cache_before = ca_cache_stats();
            let lru_before = cache_lru_keys();
            let client = reqwest::Client::new();

            let unrelated = crate::run_on(&config).await.unwrap();
            let other = WebhookTarget {
                url: "http://127.0.0.1:9/unrelated".into(),
                inbox: None,
                secret: None,
                ca_pem: Some(ca_pem()),
            };
            unrelated.webhooks.set(vec![other.clone()]);
            let unrelated_url = format!("http://{}/api/v1/webhooks", unrelated.http_addr);
            let get_guard = panic_next_api_preparation_for_test(prior.ca_pem.as_deref().unwrap());
            // Complete both unrelated handlers while A is armed: neither
            // may trigger or consume A's one-shot failure.
            for method in [reqwest::Method::GET, reqwest::Method::PUT] {
                let mut request = client.request(method.clone(), &unrelated_url);
                if method == reqwest::Method::PUT {
                    request = request.header("content-type", "application/json").body(
                        serde_json::json!([{ "url": other.url, "ca_pem": other.ca_pem }])
                            .to_string(),
                    );
                }
                assert_eq!(request.send().await.unwrap().status(), 200);
            }
            let get = client
                .get(format!("http://{}/api/v1/webhooks", server.http_addr))
                .send()
                .await
                .unwrap();
            assert_eq!(get.status(), reqwest::StatusCode::INTERNAL_SERVER_ERROR);
            let get_body = get.text().await.unwrap();
            assert!(
                serde_json::from_str::<serde_json::Value>(&get_body).unwrap()
                    == serde_json::json!({"error": "webhook response preparation failed"}),
                "fixed native GET failure"
            );
            assert!(!get_body.contains("injected"));

            assert_eq!(
                client
                    .get(format!("http://{}/api/v1/webhooks", server.http_addr))
                    .send()
                    .await
                    .unwrap()
                    .status(),
                200,
                "matching GET consumes once"
            );
            let replacement_pem = ca_pem();
            let put_guard = panic_next_api_preparation_for_test(&replacement_pem);
            // Dropping A's consumed guard cannot clear the newly armed key.
            drop(get_guard);
            for method in [reqwest::Method::GET, reqwest::Method::PUT] {
                let mut request = client.request(method.clone(), &unrelated_url);
                if method == reqwest::Method::PUT {
                    request = request.header("content-type", "application/json").body(
                        serde_json::json!([{ "url": other.url, "ca_pem": other.ca_pem }])
                            .to_string(),
                    );
                }
                assert_eq!(request.send().await.unwrap().status(), 200);
            }
            let put = client
                .put(format!("http://{}/api/v1/webhooks", server.http_addr))
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(
                    serde_json::json!([{
                        "url": "http://127.0.0.1:9/replacement",
                        "secret": "secret-value-sentinel",
                        "ca_pem": replacement_pem,
                    }])
                    .to_string(),
                )
                .send()
                .await
                .unwrap();
            assert_eq!(put.status(), reqwest::StatusCode::INTERNAL_SERVER_ERROR);
            let put_body = put.text().await.unwrap();
            assert!(
                serde_json::from_str::<serde_json::Value>(&put_body).unwrap()
                    == serde_json::json!({"error": "webhook configuration preparation failed"}),
                "fixed native PUT failure"
            );
            assert!(!put_body.contains("secret-value-sentinel"));
            assert!(!put_body.contains("injected"));
            let current = server.webhooks.current();
            assert_eq!(current.len(), 1);
            assert!(current[0].url == prior.url, "prior URL unchanged");
            assert!(current[0].inbox == prior.inbox, "prior inbox unchanged");
            assert!(current[0].secret == prior.secret, "prior secret unchanged");
            assert!(current[0].ca_pem == prior.ca_pem, "prior raw CA unchanged");
            assert_eq!(
                server.webhooks.installations.load(Ordering::SeqCst),
                calls,
                "failed preparation never swaps"
            );
            assert_eq!(
                ca_cache_stats(),
                cache_before,
                "failed API preparation is cache neutral"
            );
            assert!(
                cache_lru_keys() == lru_before,
                "failed API preparation preserves recency"
            );
            // The failed PUT retained every raw field above; a subsequent
            // matching PUT succeeds, with no poisoned hook mutex.
            assert_eq!(
                client
                    .put(format!("http://{}/api/v1/webhooks", server.http_addr))
                    .header("content-type", "application/json")
                    .body(
                        serde_json::json!([{ "url": other.url, "ca_pem": replacement_pem }])
                            .to_string()
                    )
                    .send()
                    .await
                    .unwrap()
                    .status(),
                200,
                "matching PUT consumes once"
            );
            drop(put_guard);
            // An unconsumed owner's cleanup removes its own key.
            let cleanup = panic_next_api_preparation_for_test(other.ca_pem.as_deref().unwrap());
            drop(cleanup);
            assert_eq!(
                client.get(&unrelated_url).send().await.unwrap().status(),
                200
            );
            unrelated.stop().await;
            server.stop().await;
        });
}

/// Same-key and full-cache cold races both publish through one map/deque
/// owner. Duplicate successful construction is counted even when its client
/// loses the publication race; errors and cancelled waiters never publish.
#[test]
fn concurrent_cache_misses_recheck_publication_and_preserve_lru_bijection() {
    tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(8)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let _serial = CACHE_TESTS.lock().await;
            let same = ca_pem();
            let baseline = ca_cache_stats();
            let (entered_a, release_a, finished_a) = install_ca_build_pause(&same);
            let (entered_b, release_b, finished_b) = install_ca_build_pause(&same);
            let (watchdog_done_tx, watchdog_done_rx) = std::sync::mpsc::channel();
            let watchdog_releases = [release_a.clone(), release_b.clone()];
            let watchdog_fired = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let watchdog_flag = watchdog_fired.clone();
            let watchdog = std::thread::spawn(move || {
                if watchdog_done_rx
                    .recv_timeout(Duration::from_secs(10))
                    .is_err()
                {
                    watchdog_flag.store(true, Ordering::SeqCst);
                    for release in watchdog_releases {
                        let _ = release.send(());
                    }
                }
            });
            let first_key = same.clone();
            let second_key = same.clone();
            let first = tokio::spawn(async move { client_for(Some(&first_key)).await });
            let second = tokio::spawn(async move { client_for(Some(&second_key)).await });
            let first_thread = tokio::time::timeout(Duration::from_secs(5), entered_a)
                .await
                .expect("first same-key build enters")
                .unwrap();
            let second_thread = tokio::time::timeout(Duration::from_secs(5), entered_b)
                .await
                .expect("second same-key build enters before publication")
                .unwrap();
            assert_ne!(
                first_thread, second_thread,
                "both confirmed misses build independently"
            );
            release_a.send(()).unwrap();
            release_b.send(()).unwrap();
            for finished in [finished_a, finished_b] {
                tokio::time::timeout(Duration::from_secs(5), finished)
                    .await
                    .expect("both duplicate builds complete")
                    .unwrap();
            }
            drop((
                first.await.unwrap().unwrap(),
                second.await.unwrap().unwrap(),
            ));
            assert_eq!(ca_cache_stats().builds, baseline.builds + 2);
            assert_cache_bijection();
            let after_same = ca_cache_stats();
            client_for(Some(&same)).await.unwrap();
            assert_eq!(
                ca_cache_stats(),
                after_same,
                "the published winner is a hit"
            );

            // Fill to the exact cap, then hold two distinct misses before
            // publication so both evict against the live full cache.
            for _ in 0..CA_CLIENT_CAP {
                client_for(Some(&ca_pem())).await.unwrap();
            }
            assert_eq!(ca_cache_stats().entries, CA_CLIENT_CAP);
            let full = ca_cache_stats();
            let root_a = ca_pem();
            let root_b = ca_pem();
            let (entry_a, release_a, done_a) = install_ca_build_pause(&root_a);
            let (entry_b, release_b, done_b) = install_ca_build_pause(&root_b);
            let job_a_key = root_a.clone();
            let job_b_key = root_b.clone();
            let job_a = tokio::spawn(async move { client_for(Some(&job_a_key)).await });
            let job_b = tokio::spawn(async move { client_for(Some(&job_b_key)).await });
            tokio::time::timeout(Duration::from_secs(5), entry_a)
                .await
                .expect("first distinct root misses")
                .unwrap();
            tokio::time::timeout(Duration::from_secs(5), entry_b)
                .await
                .expect("second distinct root misses before either publishes")
                .unwrap();
            release_a.send(()).unwrap();
            release_b.send(()).unwrap();
            for finished in [done_a, done_b] {
                tokio::time::timeout(Duration::from_secs(5), finished)
                    .await
                    .expect("distinct builds complete")
                    .unwrap();
            }
            drop((job_a.await.unwrap().unwrap(), job_b.await.unwrap().unwrap()));
            let after = ca_cache_stats();
            assert_eq!(after.entries, CA_CLIENT_CAP);
            assert_eq!(after.builds, full.builds + 2);
            assert_eq!(after.evictions, full.evictions + 2);
            assert_cache_bijection();
            assert!(!watchdog_fired.load(Ordering::SeqCst));
            watchdog_done_tx.send(()).unwrap();
            watchdog.join().unwrap();
        });
}

/// A panic in the off-worker builder becomes a fixed attempt error, leaves
/// the cache usable and retrying the same PEM succeeds without a poisoned
/// mutex or disclosure of panic text.
#[tokio::test]
async fn builder_panic_is_value_free_and_does_not_poison_cache() {
    let _serial = CACHE_TESTS.lock().await;
    let pem = ca_pem();
    let before = ca_cache_stats();
    install_ca_build_panic(&pem);
    let error = client_for(Some(&pem)).await.unwrap_err();
    assert_eq!(error, "webhook CA client construction task failed");
    assert!(!error.contains("injected"));
    assert_eq!(ca_cache_stats(), before);
    client_for(Some(&pem)).await.unwrap();
    assert_eq!(ca_cache_stats().builds, before.builds + 1);
    assert_cache_bijection();
}

/// A blocking-builder panic becomes an ordinary value-free attempt failure.
/// The existing retry budget reaches a real verified TLS sink and reports
/// one delivered terminal outcome, proving the cache remains unpoisoned.
#[tokio::test]
async fn delivery_builder_join_failure_retries_to_one_successful_outcome() {
    let _serial = CACHE_TESTS.lock().await;
    let (pem, addr, sink) = tls_sink_once().await;
    let server = test_server().await;
    let before = protocol_stats(&server).await.0;
    install_ca_build_panic(&pem);
    server.webhooks.set(vec![protocol_target(&pem, addr)]);
    accounting_mail(&server, 1).await;
    let after = await_protocol_deliveries(&server, 1).await;
    assert_eq!(
        after.builds,
        before.builds + 1,
        "failed join counts no construction"
    );
    let terminal = server.webhooks.snapshot();
    assert_eq!(
        (terminal.delivered, terminal.exhausted, terminal.aborted),
        (1, 0, 0)
    );
    let body = tokio::time::timeout(Duration::from_secs(5), sink)
        .await
        .expect("retry reaches TLS sink")
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(value["event"] == "received", "retry delivered stored mail");
    server.stop().await;
}

/// Invalid unchecked embedding input exhausts the existing four attempts;
/// it cannot count a successful build, publish a partial client or poison
/// the cache. SMTP acceptance and queryability still precede the outcome.
#[tokio::test]
async fn failed_delivery_build_exhausts_once_without_cache_work() {
    let _serial = CACHE_TESTS.lock().await;
    let server = test_server().await;
    let before = protocol_stats(&server).await.0;
    server.webhooks.set(vec![protocol_target(
        "-----BEGIN CERTIFICATE-----\n!!!",
        "127.0.0.1:9".parse().unwrap(),
    )]);
    accounting_mail(&server, 1).await;
    tokio::time::timeout(Duration::from_secs(5), async {
        while server.webhooks.snapshot().exhausted == 0 {
            protocol_stats(&server).await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("existing retry budget exhausts");
    assert_eq!(
        protocol_stats(&server).await.0,
        before,
        "failed constructions are cache neutral"
    );
    let terminal = server.webhooks.snapshot();
    assert_eq!(
        (
            terminal.delivered,
            terminal.exhausted,
            terminal.aborted,
            terminal.dropped
        ),
        (0, 1, 0, 0)
    );
    assert_cache_bijection();
    server.stop().await;
}

/// A cancelled delivery waiter cannot publish its successful blocking build.
/// The work may finish and increment builds, as the metric describes actual
/// construction rather than live cache insertions.
#[test]
fn cancelled_delivery_build_counts_work_without_late_publication() {
    tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(4)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let _serial = CACHE_TESTS.lock().await;
            let pem = ca_pem();
            let before = ca_cache_stats();
            let (entered, release, finished) = install_ca_build_pause(&pem);
            let waiter_pem = pem.clone();
            let waiter = tokio::spawn(async move { client_for(Some(&waiter_pem)).await });
            tokio::time::timeout(Duration::from_secs(5), entered)
                .await
                .expect("delivery build entered")
                .unwrap();
            waiter.abort();
            assert!(matches!(waiter.await, Err(error) if error.is_cancelled()));
            release.send(()).unwrap();
            tokio::time::timeout(Duration::from_secs(5), finished)
                .await
                .expect("detached builder finishes")
                .unwrap();
            tokio::time::timeout(Duration::from_secs(5), async {
                while ca_cache_stats().builds == before.builds {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .expect("completed successful CPU work is counted");
            assert_eq!(ca_cache_stats().builds, before.builds + 1);
            let cache = ca_clients().lock().unwrap();
            assert!(!cache.clients.contains_key(&pem));
            drop(cache);
            assert_cache_bijection();
        });
}

/// Shut down the real server while a delivery's uncached build is held.
/// `stop` must join the dispatcher and count exactly one aborted delivery;
/// releasing successful CPU work afterward may increment `builds` but cannot
/// publish a cache entry, send HTTP or alter the terminal ledger.
#[test]
fn shutdown_during_held_build_aborts_once_without_late_publish_or_send() {
    tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(4)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let _serial = CACHE_TESTS.lock().await;
            let config = crate::config::Config {
                smtp_listen: "127.0.0.1:0".into(),
                http_listen: "127.0.0.1:0".into(),
                pop3_listen: "127.0.0.1:0".into(),
                max_per_inbox: 0,
                data_file: None,
                tls_cert: None,
                tls_key: None,
                smtp: Default::default(),
                pop3_idle_timeout: crate::pop3::DEFAULT_IDLE_TIMEOUT,
            };
            let server = crate::run_on(&config).await.unwrap();
            let webhooks = Arc::clone(&server.webhooks);
            let pem = ca_pem();
            let sink = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            sink.set_nonblocking(true).unwrap();
            let sink_addr = sink.local_addr().unwrap();
            server.webhooks.set(vec![WebhookTarget {
                url: format!("https://{sink_addr}/hook"),
                inbox: Some("shutdown-held".into()),
                secret: None,
                ca_pem: Some(pem.clone()),
            }]);
            let before = ca_cache_stats();
            let (entered, release, finished) = install_ca_build_pause(&pem);
            let (watchdog_done_tx, watchdog_done_rx) = std::sync::mpsc::channel();
            let watchdog_release = release.clone();
            let watchdog_fired = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let watchdog_flag = watchdog_fired.clone();
            let watchdog = std::thread::spawn(move || {
                if watchdog_done_rx
                    .recv_timeout(Duration::from_secs(8))
                    .is_err()
                {
                    watchdog_flag.store(true, Ordering::SeqCst);
                    let _ = watchdog_release.send(());
                }
            });

            let accepted = tokio::time::timeout(
                Duration::from_secs(5),
                smtp_send_for_inbox(server.smtp_addr, "shutdown-held"),
            )
            .await
            .expect("SMTP commits before webhook delivery builds");
            assert!(accepted.starts_with("250"), "{accepted}");
            tokio::time::timeout(Duration::from_secs(5), entered)
                .await
                .expect("delivery construction is held before HTTP")
                .unwrap();

            tokio::time::timeout(Duration::from_secs(5), server.stop())
                .await
                .expect("stop joins the dispatcher while its delivery is held");
            let stopped = webhooks.snapshot();
            assert_eq!(stopped.delivered, 0);
            assert_eq!(
                stopped.aborted, 1,
                "the held delivery has one terminal owner"
            );
            assert_eq!(stopped.dropped, 0);
            assert_eq!(stopped.exhausted, 0);
            assert_eq!(stopped.lagged, 0);

            release.send(()).unwrap();
            tokio::time::timeout(Duration::from_secs(5), finished)
                .await
                .expect("the detached blocking build resumes")
                .unwrap();
            tokio::time::timeout(Duration::from_secs(5), async {
                while ca_cache_stats().builds != before.builds + 1 {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .expect("successful detached CA construction is counted");
            assert_eq!(ca_cache_stats().builds, before.builds + 1);
            assert_eq!(ca_cache_stats().entries, before.entries);
            let cache = ca_clients().lock().unwrap();
            assert!(!cache.clients.contains_key(&pem));
            drop(cache);
            let after = webhooks.snapshot();
            assert_eq!(after.delivered, stopped.delivered);
            assert_eq!(after.aborted, stopped.aborted);
            assert_eq!(after.dropped, stopped.dropped);
            assert_eq!(after.exhausted, stopped.exhausted);
            assert_eq!(after.lagged, stopped.lagged);
            assert!(
                matches!(
                    sink.accept(),
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
                ),
                "a cancelled waiter cannot make a late HTTP connection"
            );
            assert!(!watchdog_fired.load(Ordering::SeqCst));
            watchdog_done_tx.send(()).unwrap();
            watchdog.join().unwrap();
        });
}

fn cache_lru_keys() -> Vec<String> {
    ca_clients().lock().unwrap().lru.iter().cloned().collect()
}

fn assert_cache_bijection() {
    let cache = ca_clients().lock().unwrap();
    assert_eq!(cache.clients.len(), cache.lru.len());
    let unique: std::collections::HashSet<&String> = cache.lru.iter().collect();
    assert_eq!(unique.len(), cache.lru.len(), "LRU keys are unique");
    assert!(cache.clients.keys().all(|key| unique.contains(key)));
}

/// Hold actual delivery construction on a single-worker runtime while
/// real SMTP and HTTP continue. The finite blocking pool has four slots;
/// one held client leaves capacity for the commit and an independent
/// distinct-PEM build. DATA 250 is immediately queryable.
#[test]
fn smtp_acceptance_and_queryability_continue_during_held_distinct_ca_build() {
    tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(4)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let _serial = CACHE_TESTS.lock().await;
            let config = crate::config::Config {
                smtp_listen: "127.0.0.1:0".into(),
                http_listen: "127.0.0.1:0".into(),
                pop3_listen: "127.0.0.1:0".into(),
                max_per_inbox: 0,
                data_file: None,
                tls_cert: None,
                tls_key: None,
                smtp: Default::default(),
                pop3_idle_timeout: crate::pop3::DEFAULT_IDLE_TIMEOUT,
            };
            let server = crate::run_on(&config).await.unwrap();
            let (pem, tls_addr, sink) = tls_sink_once().await;
            server.webhooks.set(vec![WebhookTarget {
                url: format!("https://127.0.0.1:{}/hook", tls_addr.port()),
                inbox: Some("held-ca".into()),
                secret: None,
                ca_pem: Some(pem.clone()),
            }]);

            let warm = ca_pem();
            let independent = ca_pem();
            client_for(Some(&warm)).await.unwrap();
            let baseline = ca_cache_stats();
            let (entered, release, finished) = install_ca_build_pause(&pem);
            let (watchdog_done_tx, watchdog_done_rx) = std::sync::mpsc::channel();
            let watchdog_release = release.clone();
            let watchdog_fired = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let watchdog_flag = watchdog_fired.clone();
            let watchdog = std::thread::spawn(move || {
                if watchdog_done_rx
                    .recv_timeout(Duration::from_secs(10))
                    .is_err()
                {
                    watchdog_flag.store(true, Ordering::SeqCst);
                    let _ = watchdog_release.send(());
                }
            });

            let accepted = tokio::time::timeout(
                Duration::from_secs(5),
                smtp_send_for_inbox(server.smtp_addr, "held-ca"),
            )
            .await
            .expect("first real SMTP transaction completes");
            assert!(accepted.starts_with("250"), "{accepted}");
            let build_thread = tokio::time::timeout(Duration::from_secs(5), entered)
                .await
                .expect("delivery entered the actual CA builder")
                .expect("the build observer stayed connected");
            assert_ne!(build_thread, std::thread::current().id());
            let (tick_tx, tick_rx) = tokio::sync::oneshot::channel();
            let ticker = tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_millis(1));
                interval.tick().await;
                interval.tick().await;
                let _ = tick_tx.send(());
            });
            tokio::time::timeout(Duration::from_secs(5), tick_rx)
                .await
                .expect("the async-worker ticker fires while construction is held")
                .expect("the ticker remains connected");
            ticker.await.unwrap();

            // While the first delivery's build stays held, a new SMTP
            // session must complete banner, EHLO, NOOP, DATA and commit.
            let accepted = tokio::time::timeout(
                Duration::from_secs(5),
                smtp_send_for_inbox(server.smtp_addr, "smtp-progress"),
            )
            .await
            .expect("SMTP progresses while CA construction is held");
            assert!(accepted.starts_with("250"), "{accepted}");
            let client = reqwest::Client::new();
            let count_text = tokio::time::timeout(
                Duration::from_secs(5),
                client
                    .get(format!(
                        "http://{}/api/v1/inboxes/smtp-progress/count",
                        server.http_addr
                    ))
                    .send(),
            )
            .await
            .expect("HTTP query progresses while CA construction is held")
            .unwrap()
            .text()
            .await
            .unwrap();
            let count: serde_json::Value = serde_json::from_str(&count_text).unwrap();
            assert_eq!(count["count"], 1, "DATA 250 means queryable: {count}");
            let metrics = tokio::time::timeout(
                Duration::from_secs(5),
                client
                    .get(format!("http://{}/metrics", server.http_addr))
                    .send(),
            )
            .await
            .expect("metrics progresses while CA construction is held")
            .unwrap()
            .text()
            .await
            .unwrap();
            assert!(metrics.contains("swarmail_webhook_ca_cache_entries"));
            client_for(Some(&warm)).await.unwrap(); // a cache hit
            assert_eq!(ca_cache_stats().builds, baseline.builds);
            client_for(Some(&independent)).await.unwrap(); // distinct cold build B
            assert_eq!(ca_cache_stats().builds, baseline.builds + 1);
            assert!(!watchdog_fired.load(Ordering::SeqCst));

            release.send(()).unwrap();
            tokio::time::timeout(Duration::from_secs(5), finished)
                .await
                .expect("builder completion is observed")
                .unwrap();
            let delivered = tokio::time::timeout(Duration::from_secs(5), sink)
                .await
                .expect("held CA build resumes and verified TLS delivery completes")
                .unwrap();
            assert!(
                String::from_utf8_lossy(&delivered).contains("\"event\":\"received\""),
                "real TLS sink received the webhook payload"
            );
            watchdog_done_tx.send(()).unwrap();
            watchdog.join().unwrap();
            server.stop().await;
        });
}

/// Hold actual HTTP GET, then actual PUT validation and projection. Every
/// hold leaves runtime/config/cache/SMTP progress possible before release.
#[test]
fn webhook_api_preparation_is_off_worker_and_swaps_once() {
    tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(4)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let _serial = CACHE_TESTS.lock().await;
            let server = Arc::new(test_server().await);
            let pem = ca_pem();
            let original = WebhookTarget {
                url: "http://127.0.0.1:9/original".into(),
                inbox: Some("not-progress".into()),
                secret: None,
                ca_pem: Some(pem.clone()),
            };
            server.webhooks.set(vec![original.clone()]);
            let calls = server.webhooks.installations.load(Ordering::SeqCst);
            let before = ca_cache_stats();
            let lru = cache_lru_keys();
            let (get_entered, get_release, get_finished) = install_ca_build_pause(&pem);
            let (validation_entered, validation_release, validation_finished) =
                install_ca_build_pause(&pem);
            let (projection_entered, projection_release, projection_finished) =
                install_ca_build_pause(&pem);
            let watchdog = BuildWatchdog::new(vec![
                get_release.clone(),
                validation_release.clone(),
                projection_release.clone(),
            ]);
            let request_server = server.clone();
            let get =
                tokio::spawn(
                    async move { api_json(&request_server, reqwest::Method::GET, None).await },
                );
            let thread = tokio::time::timeout(Duration::from_secs(5), get_entered)
                .await
                .expect("GET builder entered")
                .unwrap();
            assert_ne!(thread, std::thread::current().id());
            assert!(
                smtp_send_for_inbox(server.smtp_addr, "get-progress")
                    .await
                    .starts_with("250")
            );
            assert_eq!(
                inbox_count(&reqwest::Client::new(), server.http_addr, "get-progress").await,
                1
            );
            assert!(
                server.webhooks.current()[0].url == original.url,
                "GET released configuration guard"
            );
            assert_eq!(ca_cache_stats(), before);
            get_release.send(()).unwrap();
            get_finished.await.unwrap();
            let (status, echo) = get.await.unwrap();
            assert_eq!(status, reqwest::StatusCode::OK);
            assert!(echo[0]["ca_pem"] == pem, "GET public roots");
            let mut replacement = original.clone();
            replacement.url = "http://127.0.0.1:9/replacement".into();
            let request_server = server.clone();
            let put = tokio::spawn(async move {
                api_json(&request_server, reqwest::Method::PUT, Some(&[replacement])).await
            });
            for (entered, release, finished) in [
                (validation_entered, validation_release, validation_finished),
                (projection_entered, projection_release, projection_finished),
            ] {
                let thread = tokio::time::timeout(Duration::from_secs(5), entered)
                    .await
                    .expect("actual PUT builder entered")
                    .unwrap();
                assert_ne!(thread, std::thread::current().id());
                assert_eq!(
                    server.webhooks.installations.load(Ordering::SeqCst),
                    calls,
                    "no early swap"
                );
                assert!(
                    server.webhooks.current()[0].url == original.url,
                    "old configuration remains whole"
                );
                assert_eq!(ca_cache_stats(), before);
                assert_eq!(cache_lru_keys(), lru);
                assert!(
                    smtp_send_for_inbox(server.smtp_addr, "put-progress")
                        .await
                        .starts_with("250")
                );
                release.send(()).unwrap();
                tokio::time::timeout(Duration::from_secs(5), finished)
                    .await
                    .expect("PUT build finished")
                    .unwrap();
            }
            let (status, echo) = put.await.unwrap();
            assert_eq!(status, reqwest::StatusCode::OK);
            assert!(
                echo[0]["url"] == "http://127.0.0.1:9/replacement",
                "PUT echoes installed list"
            );
            assert!(echo[0]["ca_pem"] == pem, "PUT public roots");
            assert!(
                server.webhooks.current()[0].url == "http://127.0.0.1:9/replacement",
                "owner installed replacement"
            );
            assert_eq!(
                server.webhooks.installations.load(Ordering::SeqCst),
                calls + 1,
                "one swap"
            );
            assert_eq!(ca_cache_stats(), before);
            assert_eq!(cache_lru_keys(), lru);
            watchdog.assert_unused();
            Arc::try_unwrap(server).ok().unwrap().stop().await;
        });
}

/// Abort the real request-originated publication owner, observe its drop,
/// then let both detached preparation builds finish. A disconnected client
/// or an aborted helper alone would not demonstrate this boundary.
#[tokio::test]
async fn cancelled_put_handler_cannot_install_after_preparation() {
    let _serial = CACHE_TESTS.lock().await;
    let server = Arc::new(test_server().await);
    server.webhooks.set(vec![WebhookTarget {
        url: "http://127.0.0.1:9/prior".into(),
        inbox: None,
        secret: None,
        ca_pem: None,
    }]);
    let calls = server.webhooks.installations.load(Ordering::SeqCst);
    let before = ca_cache_stats();
    let lru = cache_lru_keys();
    let pem = ca_pem();
    let (entered, release, finished) = install_ca_build_pause(&pem);
    let (projected, project_release, project_finished) = install_ca_build_pause(&pem);
    let watchdog = BuildWatchdog::new(vec![release.clone(), project_release.clone()]);
    let (abort, dropped) = crate::api::cancel_put_for(server.webhooks.clone());
    let request_server = server.clone();
    let put = tokio::spawn(async move {
        api_json(
            &request_server,
            reqwest::Method::PUT,
            Some(&[WebhookTarget {
                url: "http://127.0.0.1:9/late".into(),
                inbox: None,
                secret: None,
                ca_pem: Some(pem),
            }]),
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(5), entered)
        .await
        .expect("real PUT validation entered")
        .unwrap();
    abort.abort();
    tokio::time::timeout(Duration::from_secs(5), dropped)
        .await
        .expect("actual PUT handler future dropped")
        .unwrap();
    assert_eq!(
        put.await.unwrap().0,
        reqwest::StatusCode::INTERNAL_SERVER_ERROR
    );
    release.send(()).unwrap();
    finished.await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), projected)
        .await
        .expect("cancelled worker still prepares echo")
        .unwrap();
    project_release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), project_finished)
        .await
        .expect("detached projection completed")
        .unwrap();
    assert_eq!(
        server.webhooks.installations.load(Ordering::SeqCst),
        calls,
        "no late installation"
    );
    assert!(
        server.webhooks.current()[0].url == "http://127.0.0.1:9/prior",
        "prior list retained"
    );
    assert_eq!(ca_cache_stats(), before);
    assert_eq!(cache_lru_keys(), lru);
    watchdog.assert_unused();
    Arc::try_unwrap(server).ok().unwrap().stop().await;
}

/// GET projection and PUT validation/projection are cache-neutral even at
/// a full cache: they preserve the exact LRU order, and the next cold delivery
/// still evicts the same oldest client.
#[tokio::test]
async fn api_preparation_preserves_lru_order_and_next_eviction() {
    let _serial = CACHE_TESTS.lock().await;
    // Seed every slot with canonical roots, independent of earlier tests'
    // raw-text cache identities. Counters are never reset.
    for _ in 0..CA_CLIENT_CAP {
        client_for(Some(&ca_pem())).await.unwrap();
    }
    let before = ca_cache_stats();
    let lru_before = cache_lru_keys();
    assert_eq!(before.entries, CA_CLIENT_CAP);
    let oldest = lru_before.first().unwrap().clone();
    let target = WebhookTarget {
        url: "http://127.0.0.1:9/lru-probe".into(),
        inbox: None,
        secret: None,
        ca_pem: Some(oldest.clone()),
    };
    let server = test_server().await;
    server.webhooks.set(vec![target.clone()]);
    let (status, projected) = api_json(&server, reqwest::Method::GET, None).await;
    assert_eq!(status, reqwest::StatusCode::OK);
    assert!(projected[0]["ca_pem"] == oldest, "GET projection succeeds");
    let (status, echo) = api_json(&server, reqwest::Method::PUT, Some(&[target])).await;
    assert_eq!(status, reqwest::StatusCode::OK);
    assert!(echo[0]["ca_pem"] == oldest, "PUT preparation succeeds");
    assert_eq!(ca_cache_stats(), before);
    assert_eq!(cache_lru_keys(), lru_before);

    let (cold, addr, sink) = tls_sink_once().await;
    server.webhooks.set(vec![protocol_target(&cold, addr)]);
    accounting_mail(&server, 1).await;
    let after = await_protocol_deliveries(&server, 1).await;
    tokio::time::timeout(Duration::from_secs(5), sink)
        .await
        .expect("next eviction delivers real TLS")
        .unwrap();
    assert_eq!(after.entries, CA_CLIENT_CAP);
    assert_eq!(after.evictions, before.evictions + 1);
    {
        let cache = ca_clients().lock().unwrap();
        assert!(!cache.clients.contains_key(&oldest));
    }
    assert_cache_bijection();
    server.stop().await;
}

/// Deliberately exhaust Tokio's blocking pool with one held, real delivery
/// CA build. With capacity 1, five-second operation deadlines and an
/// independent eight-second release watchdog, the second SMTP DATA parse
/// queues in Store::offload, stays unqueryable, then commits and
/// returns 250 after controlled release. This records the shared-pool
/// saturation residual separately from the spare-capacity progress proof.
#[test]
fn exhausted_blocking_pool_queues_smtp_parse_until_delivery_build_releases() {
    use tokio::io::AsyncWriteExt;
    tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(1)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let _serial = CACHE_TESTS.lock().await;
            let config = crate::config::Config {
                smtp_listen: "127.0.0.1:0".into(),
                http_listen: "127.0.0.1:0".into(),
                pop3_listen: "127.0.0.1:0".into(),
                max_per_inbox: 0,
                data_file: None,
                tls_cert: None,
                tls_key: None,
                smtp: Default::default(),
                pop3_idle_timeout: crate::pop3::DEFAULT_IDLE_TIMEOUT,
            };
            let server = crate::run_on(&config).await.unwrap();
            let (pem, tls_addr, sink) = tls_sink_once().await;
            server.webhooks.set(vec![WebhookTarget {
                url: format!("https://127.0.0.1:{}/hook", tls_addr.port()),
                inbox: Some("pool-holder".into()),
                secret: None,
                ca_pem: Some(pem.clone()),
            }]);

            let runtime_thread = std::thread::current().id();
            let (entered, release, finished) = install_ca_build_pause(&pem);
            let (watchdog_done_tx, watchdog_done_rx) = std::sync::mpsc::channel();
            let watchdog_release = release.clone();
            let watchdog_fired = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let watchdog_flag = watchdog_fired.clone();
            let watchdog = std::thread::spawn(move || {
                if watchdog_done_rx
                    .recv_timeout(Duration::from_secs(8))
                    .is_err()
                {
                    watchdog_flag.store(true, Ordering::SeqCst);
                    let _ = watchdog_release.send(());
                }
            });
            let accepted = tokio::time::timeout(
                Duration::from_secs(5),
                smtp_send_for_inbox(server.smtp_addr, "pool-holder"),
            )
            .await
            .expect("the holder transaction completes before delivery construction");
            assert!(accepted.starts_with("250"), "{accepted}");
            let builder_thread = tokio::time::timeout(Duration::from_secs(5), entered)
                .await
                .expect("the actual delivery builder enters its held state")
                .unwrap();
            assert_ne!(builder_thread, runtime_thread);

            // Install after the holder's store work is complete. The SMTP
            // owner sends this immediately before Store::offload; on this
            // current-thread runtime, that session continues through job
            // submission before the observer can run.
            let queued = crate::smtp::observe_next_data_parse_submission_for_test();
            let (mut reader, mut writer) =
                smtp_send_data_waiting(server.smtp_addr, "pool-waiter").await;
            tokio::time::timeout(Duration::from_secs(5), queued)
                .await
                .expect("SMTP DATA parse is submitted while the only blocking slot is held")
                .expect("the queued-work observation remains connected");
            let client = reqwest::Client::new();
            assert_eq!(
                inbox_count(&client, server.http_addr, "pool-waiter").await,
                0,
                "the queued parse cannot be queryable before pool capacity returns"
            );

            release.send(()).unwrap();
            tokio::time::timeout(Duration::from_secs(5), finished)
                .await
                .expect("the held builder completes")
                .unwrap();
            let accepted =
                tokio::time::timeout(Duration::from_secs(5), smtp_read_reply(&mut reader))
                    .await
                    .expect("the queued parse and insert recover after release")
                    .remove(0);
            assert!(accepted.starts_with("250"), "{accepted}");
            assert_eq!(
                inbox_count(&client, server.http_addr, "pool-waiter").await,
                1,
                "the recovered DATA 250 is immediately queryable"
            );
            writer.write_all(b"QUIT\r\n").await.unwrap();
            let delivered = tokio::time::timeout(Duration::from_secs(5), sink)
                .await
                .expect("released delivery completes over verified TLS")
                .unwrap();
            assert!(String::from_utf8_lossy(&delivered).contains("\"event\":\"received\""));
            assert!(!watchdog_fired.load(Ordering::SeqCst));
            watchdog_done_tx.send(()).unwrap();
            watchdog.join().unwrap();
            server.stop().await;
        });
}

/// A real cache miss is held at builder entry on a blocking thread.
/// While it waits, the sole async worker can observe the cache, use a
/// warm client and build an independent distinct PEM.
#[test]
fn cold_client_construction_leaves_the_async_worker_and_cache_mutex_free() {
    tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(4)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let _serial = CACHE_TESTS.lock().await;
            let warm = ca_pem();
            let cold_a = ca_pem();
            let cold_b = ca_pem();
            client_for(Some(&warm)).await.unwrap();

            let runtime_thread = std::thread::current().id();
            let (entered, release, finished) = install_ca_build_pause(&cold_a);
            let (watchdog_done_tx, watchdog_done_rx) = std::sync::mpsc::channel();
            let watchdog_release = release.clone();
            let watchdog_fired = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let watchdog_flag = watchdog_fired.clone();
            let watchdog = std::thread::spawn(move || {
                if watchdog_done_rx
                    .recv_timeout(Duration::from_secs(8))
                    .is_err()
                {
                    watchdog_flag.store(true, Ordering::SeqCst);
                    let _ = watchdog_release.send(());
                }
            });
            let building = tokio::spawn(async move { client_for(Some(&cold_a)).await });
            let build_thread = tokio::time::timeout(Duration::from_secs(5), entered)
                .await
                .expect("the real builder must enter")
                .expect("the builder observer must remain connected");
            assert_ne!(
                build_thread, runtime_thread,
                "construction must leave Tokio"
            );

            let baseline = ca_cache_stats();
            let progress = async {
                tokio::task::yield_now().await;
                let warmed = client_for(Some(&warm)).await.unwrap();
                let observed = ca_cache_stats();
                let independent = client_for(Some(&cold_b)).await.unwrap();
                (warmed, observed, independent)
            };
            let (warmed, observed, independent) =
                tokio::time::timeout(Duration::from_secs(5), progress)
                    .await
                    .expect("cache access and a distinct build progress while A is held");
            assert_eq!(observed.builds, baseline.builds);
            assert_eq!(ca_cache_stats().builds, baseline.builds + 1);
            drop((warmed, independent));
            assert!(
                !watchdog_fired.load(Ordering::SeqCst),
                "watchdog released the builder"
            );
            release.send(()).unwrap();
            tokio::time::timeout(Duration::from_secs(5), finished)
                .await
                .expect("builder completion is observed")
                .unwrap();
            building.await.unwrap().unwrap();
            watchdog_done_tx.send(()).unwrap();
            watchdog.join().unwrap();
        });
}

/// Failure cleanup must release even if the async test panics: Tokio cannot
/// cancel a running blocking closure when its runtime is dropped.
struct ProjectionWatchdog {
    releases: Vec<std::sync::mpsc::Sender<()>>,
    done: Option<std::sync::mpsc::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    fired: Arc<std::sync::atomic::AtomicBool>,
}

impl ProjectionWatchdog {
    fn new(releases: Vec<std::sync::mpsc::Sender<()>>) -> Self {
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let fired = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = fired.clone();
        let fallback = releases.clone();
        let thread = std::thread::spawn(move || {
            if done_rx.recv_timeout(Duration::from_secs(15)).is_err() {
                flag.store(true, Ordering::SeqCst);
                for release in fallback {
                    let _ = release.send(());
                }
            }
        });
        Self {
            releases,
            done: Some(done_tx),
            thread: Some(thread),
            fired,
        }
    }

    fn finish(mut self) {
        self.join();
        assert!(
            !self.fired.load(Ordering::SeqCst),
            "watchdog invalidates proof"
        );
    }

    fn join(&mut self) {
        for release in &self.releases {
            let _ = release.send(());
        }
        let _ = self.done.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}

impl Drop for ProjectionWatchdog {
    fn drop(&mut self) {
        if self.thread.is_some() {
            self.join();
        }
    }
}

/// One poll of the real buffered reader, at the controlled pre-release
/// observation. Any bytes (including a partial reply), EOF or error fail.
async fn assert_data_reply_pending(
    reader: &mut tokio::io::BufReader<tokio::net::tcp::OwnedReadHalf>,
) {
    use std::{pin::Pin, task::Poll};
    use tokio::io::AsyncBufRead;
    std::future::poll_fn(|cx| {
        assert!(
            Pin::new(&mut *reader).poll_fill_buf(cx).is_pending(),
            "no DATA reply bytes may be observable while the parse is queued"
        );
        Poll::Ready(())
    })
    .await;
}

/// Finite, controlled resource experiment: real unauthenticated HTTP GETs
/// occupy actual builders, or an explicitly cancelled projection keeps its
/// running builder. It measures coupling, not an attack/latency threshold.
fn projection_capacity_case(blocking_slots: usize, get_count: usize, cancel_waiter: bool) {
    use tokio::io::AsyncWriteExt;
    tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(blocking_slots)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let _serial = CACHE_TESTS.lock().await;
            let config = crate::config::Config {
                smtp_listen: "127.0.0.1:0".into(),
                http_listen: "127.0.0.1:0".into(),
                pop3_listen: "127.0.0.1:0".into(),
                max_per_inbox: 0,
                data_file: None,
                tls_cert: None,
                tls_key: None,
                smtp: Default::default(),
                pop3_idle_timeout: crate::pop3::DEFAULT_IDLE_TIMEOUT,
            };
            let server = crate::run_on(&config).await.unwrap();
            let pem = ca_pem();
            assert_eq!(public_ca_pem(&pem), pem, "canonical one-root fixture");
            let original = WebhookTarget {
                url: "http://user:password@127.0.0.1:9/unused".into(),
                inbox: Some("not-the-measured-inbox".into()),
                secret: Some("withheld".into()),
                ca_pem: Some(pem.clone()),
            };
            server.webhooks.set(vec![original.clone()]);
            let installed = serde_json::to_value(server.webhooks.current()).unwrap();
            let cache_before = ca_cache_stats();
            let lru_before = cache_lru_keys();
            let runtime_thread = std::thread::current().id();
            let client = reqwest::Client::new();
            let mut entries = Vec::new();
            let mut releases = Vec::new();
            let mut finishes = Vec::new();
            for _ in 0..get_count {
                let (entered, release, finished) = install_ca_build_pause(&pem);
                entries.push(entered);
                releases.push(release);
                finishes.push(finished);
            }
            let watchdog = ProjectionWatchdog::new(releases.clone());
            eprintln!(
                "projection case: async_threads=1 blocking_slots={blocking_slots} jobs={get_count} targets_per_job=1 roots_per_target=1 pem_bytes={} cancelled_waiter={cancel_waiter}",
                pem.len()
            );
            let mut gets = Vec::new();
            let cancelled = if cancel_waiter {
                assert_eq!(get_count, 1);
                Some(tokio::spawn(redacted_targets(vec![original])))
            } else {
                for _ in 0..get_count {
                    let http = client.clone();
                    let addr = server.http_addr;
                    gets.push(tokio::spawn(async move {
                        let response = http
                            .get(format!("http://{addr}/api/v1/webhooks"))
                            .send()
                            .await
                            .unwrap();
                        assert_eq!(response.status(), reqwest::StatusCode::OK);
                        serde_json::from_str::<serde_json::Value>(
                            &response.text().await.unwrap(),
                        )
                        .unwrap()
                    }));
                }
                None
            };
            for (index, entered) in entries.into_iter().enumerate() {
                let thread = tokio::time::timeout(Duration::from_secs(5), entered)
                    .await
                    .expect("actual projection builder entered")
                    .unwrap();
                assert_ne!(thread, runtime_thread);
                eprintln!("builder {index}: submitted and entered off-worker, held");
            }
            if let Some(waiter) = cancelled {
                waiter.abort();
                assert!(matches!(waiter.await, Err(error) if error.is_cancelled()));
                eprintln!("specific projection waiter: aborted and joined as cancelled");
            }
            for finished in &mut finishes {
                assert!(matches!(
                    finished.try_recv(),
                    Err(tokio::sync::oneshot::error::TryRecvError::Empty)
                ));
            }
            let queued = crate::smtp::observe_next_data_parse_submission_for_test();
            let (mut reader, mut writer) = tokio::time::timeout(
                Duration::from_secs(5),
                smtp_send_data_waiting(server.smtp_addr, "projection-measured"),
            )
            .await
            .expect("real SMTP reaches DATA terminator while GET builders held");
            tokio::time::timeout(Duration::from_secs(5), queued)
                .await
                .expect("SMTP parser submitted, not merely a created task")
                .unwrap();
            eprintln!("SMTP: parse submitted after all builders entered");
            // This real request also proves the sole async thread progresses.
            let count = tokio::time::timeout(
                Duration::from_secs(5),
                inbox_count(&client, server.http_addr, "projection-measured"),
            )
            .await
            .expect("cheap HTTP progresses while builders held");
            let spare_slot = blocking_slots > get_count;
            if spare_slot {
                let accepted = tokio::time::timeout(
                    Duration::from_secs(5),
                    smtp_read_reply(&mut reader),
                )
                .await
                .expect("spare blocking slot permits parse/insert before GET release")
                .remove(0);
                assert!(accepted.starts_with("250"), "{accepted}");
                assert_eq!(
                    inbox_count(&client, server.http_addr, "projection-measured").await,
                    1
                );
                eprintln!("SMTP before release: DATA 250, immediate count=1");
            } else {
                assert_eq!(count, 0, "saturated parse remains unqueryable");
                assert_data_reply_pending(&mut reader).await;
                eprintln!("SMTP before release: DATA reader Pending, count=0; cheap HTTP completed");
            }
            for finished in &mut finishes {
                assert!(matches!(
                    finished.try_recv(),
                    Err(tokio::sync::oneshot::error::TryRecvError::Empty)
                ), "running jobs have not finished or reclaimed their slots");
            }
            assert_eq!(serde_json::to_value(server.webhooks.current()).unwrap(), installed);
            assert_eq!(ca_cache_stats(), cache_before);
            assert_eq!(cache_lru_keys(), lru_before);
            for (index, (release, finished)) in releases.into_iter().zip(finishes).enumerate() {
                release.send(()).unwrap();
                eprintln!("builder {index}: explicit release");
                tokio::time::timeout(Duration::from_secs(5), finished)
                    .await
                    .expect("actual builder finished after release")
                    .unwrap();
                eprintln!("builder {index}: finished");
            }
            for get in gets {
                let body = tokio::time::timeout(Duration::from_secs(5), get)
                    .await
                    .expect("safe GET response completes")
                    .unwrap();
                assert_eq!(body, serde_json::json!([{
                    "url": "http://127.0.0.1:9/unused",
                    "inbox": "not-the-measured-inbox",
                    "secret": null,
                    "ca_pem": pem,
                }]));
            }
            if !spare_slot {
                let accepted = tokio::time::timeout(
                    Duration::from_secs(5),
                    smtp_read_reply(&mut reader),
                )
                .await
                .expect("SMTP parse/insert recovers after builder release")
                .remove(0);
                assert!(accepted.starts_with("250"), "{accepted}");
                assert_eq!(
                    inbox_count(&client, server.http_addr, "projection-measured").await,
                    1,
                    "DATA 250 is immediately queryable"
                );
                eprintln!("SMTP after release: DATA 250, immediate count=1");
            }
            writer.write_all(b"QUIT\r\n").await.unwrap();
            assert!(smtp_read_reply(&mut reader).await[0].starts_with("221"));
            assert_eq!(serde_json::to_value(server.webhooks.current()).unwrap(), installed);
            assert_eq!(ca_cache_stats(), cache_before);
            assert_eq!(cache_lru_keys(), lru_before);
            tokio::time::timeout(Duration::from_secs(5), server.stop())
                .await
                .expect("server joins after all observed blocking work completes");
            watchdog.finish();
            eprintln!("cleanup: configuration/cache/LRU unchanged, server stopped, watchdog=false");
        });
}

#[test]
fn get_projection_exhausts_blocking_pool_until_release() {
    projection_capacity_case(1, 1, false);
}

#[test]
fn concurrent_get_projection_capacity_controls_smtp_progress() {
    projection_capacity_case(2, 2, false);
    projection_capacity_case(3, 2, false);
}

#[test]
fn cancelled_get_preparation_keeps_its_blocking_slot_until_finish() {
    projection_capacity_case(1, 1, true);
}

/// Read the actual public metric names, after deliveries quiesce. This is
/// intentionally independent of ca_cache_stats(), used only to inspect the
/// private map/deque invariants behind the corresponding protocol values.
async fn protocol_stats(server: &crate::RunningServer) -> (CaCacheStats, u64) {
    let response = reqwest::Client::new()
        .get(format!("http://{}/metrics", server.http_addr))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let text = response.text().await.unwrap();
    let metric = |name: &str| -> u64 {
        text.lines()
            .find_map(|line| {
                let mut parts = line.split_whitespace();
                (parts.next() == Some(name)).then(|| parts.next().unwrap().parse().unwrap())
            })
            .unwrap_or_else(|| panic!("required protocol metric missing"))
    };
    (
        CaCacheStats {
            entries: metric("swarmail_webhook_ca_cache_entries") as usize,
            builds: metric("swarmail_webhook_ca_cache_builds_total"),
            evictions: metric("swarmail_webhook_ca_cache_evictions_total"),
        },
        metric("swarmail_webhook_delivered_total"),
    )
}

async fn await_protocol_deliveries(server: &crate::RunningServer, delivered: u64) -> CaCacheStats {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let (stats, actual) = protocol_stats(server).await;
            assert!(actual <= delivered, "no extra terminal delivery");
            if actual == delivered {
                return stats;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("real TLS deliveries quiesce before metric comparison")
}

fn protocol_target(pem: &str, addr: std::net::SocketAddr) -> WebhookTarget {
    WebhookTarget {
        url: format!("https://{addr}/hook"),
        inbox: Some("accounting".into()),
        secret: None,
        ca_pem: Some(pem.into()),
    }
}

async fn accounting_mail(server: &crate::RunningServer, expected_count: u64) {
    assert!(
        smtp_send_for_inbox(server.smtp_addr, "accounting")
            .await
            .starts_with("250")
    );
    assert_eq!(
        inbox_count(&reqwest::Client::new(), server.http_addr, "accounting").await,
        expected_count,
        "queryable immediately after DATA 250"
    );
}

/// Same-key races and full-cache distinct races originate in real SMTP,
/// complete verified TLS, and expose exact build/eviction deltas on HTTP.
/// One async worker, eight blocking slots, five-second progress deadlines;
/// a separate twenty-second watchdog is cleanup only, never success.
#[test]
fn protocol_ca_cache_publication_and_accounting() {
    tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(8)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let _serial = CACHE_TESTS.lock().await;
            let old = { std::mem::take(&mut *ca_clients().lock().unwrap()) };
            drop(old); // Fixture isolation, outside the cache guard; counters never reset.
            let server = test_server().await;
            let (pem, addr, sink) = tls_sink_requests(6).await;
            let (pem_b, addr_b, sink_b) = tls_sink_requests(1).await;
            let raw_a = format!("{pem}# distinct raw cache identity A\n");
            let (entry_one, release_one, done_one) = install_ca_build_pause(&pem);
            let (entry_two, release_two, done_two) = install_ca_build_pause(&pem);
            let (entry_a, release_a, done_a) = install_ca_build_pause(&raw_a);
            let (entry_b, release_b, done_b) = install_ca_build_pause(&pem_b);
            let watchdog = BuildWatchdog::new(vec![
                release_one.clone(),
                release_two.clone(),
                release_a.clone(),
                release_b.clone(),
            ]);
            let before = protocol_stats(&server).await.0;
            assert_eq!(before.entries, 0);
            server.webhooks.set(vec![protocol_target(&pem, addr)]);
            accounting_mail(&server, 1).await;
            tokio::time::timeout(Duration::from_secs(5), entry_one)
                .await
                .expect("first SMTP miss enters")
                .unwrap();
            accounting_mail(&server, 2).await;
            tokio::time::timeout(Duration::from_secs(5), entry_two)
                .await
                .expect("duplicate SMTP miss enters before publication")
                .unwrap();
            release_one.send(()).unwrap();
            done_one.await.unwrap();
            await_protocol_deliveries(&server, 1).await;
            release_two.send(()).unwrap();
            done_two.await.unwrap();
            let duplicates = await_protocol_deliveries(&server, 2).await;
            assert_eq!(
                duplicates.builds,
                before.builds + 2,
                "losing duplicate construction counts"
            );
            assert_eq!(
                duplicates.evictions, before.evictions,
                "one winner means no eviction"
            );
            assert_eq!(duplicates.entries, 1, "one same-key winner");
            assert_cache_bijection();

            // Equivalent certificates with different raw trailing text keep their
            // distinct identities. Helper builds establish a full known cache;
            // every claimed delta below is driven and observed over protocols.
            for i in 1..CA_CLIENT_CAP {
                client_for(Some(&format!("{pem}# setup raw identity {i}\n")))
                    .await
                    .unwrap();
            }
            let full = protocol_stats(&server).await.0;
            assert_eq!(full.entries, CA_CLIENT_CAP);
            accounting_mail(&server, 3).await; // Touch the original oldest key over SMTP.
            let touched = await_protocol_deliveries(&server, 3).await;
            assert_eq!(touched, full, "a real hit constructs/evicts nothing");
            let lru = cache_lru_keys();
            assert!(lru.last() == Some(&pem), "hit updates recency");
            let victim_one = lru[0].clone();
            let victim_two = lru[1].clone();
            server.webhooks.set(vec![
                protocol_target(&raw_a, addr),
                protocol_target(&pem_b, addr_b),
            ]);
            accounting_mail(&server, 4).await;
            for entry in [entry_a, entry_b] {
                tokio::time::timeout(Duration::from_secs(5), entry)
                    .await
                    .expect("distinct misses both enter while full")
                    .unwrap();
            }
            release_a.send(()).unwrap();
            done_a.await.unwrap();
            await_protocol_deliveries(&server, 4).await;
            assert!(
                !ca_clients()
                    .lock()
                    .unwrap()
                    .clients
                    .contains_key(&victim_one),
                "first actual LRU removal"
            );
            release_b.send(()).unwrap();
            done_b.await.unwrap();
            let distinct = await_protocol_deliveries(&server, 5).await;
            assert_eq!(distinct.builds, full.builds + 2);
            assert_eq!(distinct.evictions, full.evictions + 2);
            assert_eq!(distinct.entries, CA_CLIENT_CAP);
            {
                let cache = ca_clients().lock().unwrap();
                assert!(
                    !cache.clients.contains_key(&victim_two),
                    "second actual LRU removal"
                );
                assert!(
                    cache.clients.contains_key(&pem) && cache.clients.contains_key(&raw_a),
                    "raw text identity is preserved"
                );
            }
            assert_cache_bijection();
            server.webhooks.set(vec![protocol_target(&pem, addr)]);
            accounting_mail(&server, 5).await;
            assert_eq!(
                await_protocol_deliveries(&server, 6).await,
                distinct,
                "cached delivery adds no work"
            );
            server
                .webhooks
                .set(vec![protocol_target(&victim_one, addr)]);
            accounting_mail(&server, 6).await;
            let rebuilt = await_protocol_deliveries(&server, 7).await;
            assert_eq!(rebuilt.builds, distinct.builds + 1);
            assert_eq!(rebuilt.evictions, distinct.evictions + 1);
            assert_eq!(rebuilt.entries, CA_CLIENT_CAP);
            assert_cache_bijection();
            for task in [sink, sink_b] {
                let bodies = tokio::time::timeout(Duration::from_secs(5), task)
                    .await
                    .expect("all TLS requests completed")
                    .unwrap();
                for body in bodies {
                    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
                    assert!(value["event"] == "received", "real stored mail payload");
                }
            }
            let terminal = server.webhooks.snapshot();
            assert_eq!(terminal.delivered, 7);
            assert_eq!(
                (
                    terminal.exhausted,
                    terminal.aborted,
                    terminal.dropped,
                    terminal.lagged
                ),
                (0, 0, 0, 0)
            );
            watchdog.assert_unused();
            server.stop().await;
        });
}
