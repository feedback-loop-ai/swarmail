//! The shipped binary itself: serve binds, answers healthz, and shuts down
//! cleanly on SIGINT; the mcp bridge exits 0 at EOF. These tests spawn the
//! instrumented binary, so `src/main.rs` is covered like everything else.

mod common;

use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_swarmail")
}

fn serve_cmd() -> Command {
    let mut cmd = Command::new(binary());
    cmd.args([
        "serve",
        "--smtp-listen",
        "127.0.0.1:23456",
        "--pop3-listen",
        "127.0.0.1:0",
        "--http-listen",
        "127.0.0.1:23457",
    ])
    .stdout(Stdio::null())
    .stderr(Stdio::null());
    cmd
}

/// Poll the HTTP port until the server answers (or fail loudly).
async fn wait_http_up(addr: std::net::SocketAddr) {
    for _ in 0..100 {
        if tokio::time::timeout(
            Duration::from_millis(200),
            tokio::net::TcpStream::connect(addr),
        )
        .await
        .is_ok_and(|s| s.is_ok())
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("server never came up on {addr}");
}

/// SIGINT = ctrl_c: the graceful-shutdown path, then a clean exit 0. The
/// deadline is liveness against a hang, not the shutdown budget: process
/// exit adds spawn and teardown overhead on top of the server's own path,
/// which a loaded runner stretches past 10s — the budget itself is proven
/// by the API-level held-lock tests on the server's own clock.
async fn sigint_and_wait(child: &mut Child, deadline: Duration) -> std::process::ExitStatus {
    unsafe {
        libc::kill(child.id() as i32, libc::SIGINT);
    }
    tokio::time::timeout(deadline, async {
        loop {
            match child.try_wait().unwrap() {
                Some(status) => return status,
                None => tokio::time::sleep(Duration::from_millis(50)).await,
            }
        }
    })
    .await
    .expect("server did not stop on SIGINT")
}

#[tokio::test]
async fn serve_answers_healthz_and_stops_on_sigint() {
    let mut child = serve_cmd().spawn().expect("spawn swarmail serve");
    wait_http_up("127.0.0.1:23457".parse().unwrap()).await;
    let status = sigint_and_wait(&mut child, Duration::from_secs(30)).await;
    assert!(status.success(), "SIGINT exit: {status:?}");
}

#[tokio::test]
async fn serve_restores_a_data_file_across_a_restart() {
    let db = std::env::temp_dir().join(format!(
        "swarmail-binary-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let smtp_addr: std::net::SocketAddr = "127.0.0.1:23458".parse().unwrap();
    let http_addr: std::net::SocketAddr = "127.0.0.1:23459".parse().unwrap();

    // First run: --data-file as the flag; one real SMTP mail in.
    let mut child = Command::new(binary())
        .args([
            "serve",
            "--smtp-listen",
            "127.0.0.1:23458",
            "--pop3-listen",
            "127.0.0.1:0",
            "--http-listen",
            "127.0.0.1:23459",
            "--data-file",
        ])
        .arg(&db)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn swarmail serve");
    wait_http_up(http_addr).await;
    let reply = common::smtp_send(
        smtp_addr,
        Some("binpersist"),
        "a@x.io",
        "b@x.io",
        "Binary persist",
        "the body",
    )
    .await
    .expect("smtp send");
    assert!(reply.starts_with("250"), "{reply}");
    let status = sigint_and_wait(&mut child, Duration::from_secs(30)).await;
    assert!(status.success(), "SIGINT exit: {status:?}");

    // Restart with the SWARMAIL_DATA_FILE env knob instead: same file, full
    // state back, and the port answers again.
    let mut child = Command::new(binary())
        .args([
            "serve",
            "--smtp-listen",
            "127.0.0.1:23458",
            "--pop3-listen",
            "127.0.0.1:0",
            "--http-listen",
            "127.0.0.1:23459",
        ])
        .env("SWARMAIL_DATA_FILE", &db)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("respawn swarmail serve");
    wait_http_up(http_addr).await;
    let (st, list) = common::http_json(
        http_addr,
        "GET",
        "/api/v1/inboxes/binpersist/messages",
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(list["total"], 1, "mail did not survive the restart: {list}");
    assert_eq!(list["emails"][0]["subject"], "Binary persist");
    let status = sigint_and_wait(&mut child, Duration::from_secs(30)).await;
    assert!(status.success(), "SIGINT exit: {status:?}");
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", db.display()));
    }
}

#[tokio::test]
async fn mcp_bridge_exits_zero_at_eof_without_touching_the_network() {
    let mut child = Command::new(binary())
        .args(["mcp", "--url", "http://127.0.0.1:1"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn swarmail mcp");
    let status = child.wait().unwrap();
    assert!(status.success(), "mcp EOF exit: {status:?}");
}

/// A pipelined burst sender for the kill leg: each mail's command block is
/// one write (PIPELINING), the four replies read back in order, and every
/// observed acceptance contributes its `stored as` id. When the socket dies
/// — the SIGKILL resets it — the session ends with the ids observed so far.
struct KillBurst {
    reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: tokio::net::tcp::OwnedWriteHalf,
}

impl KillBurst {
    async fn connect(addr: std::net::SocketAddr, inbox: &str) -> Self {
        let stream = TcpStream::connect(addr).await.unwrap();
        stream.set_nodelay(true).unwrap();
        let (r, w) = stream.into_split();
        let mut reader = BufReader::new(r);
        let mut writer = w;
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("220"), "{line}");
        writer.write_all(b"EHLO kill-leg\r\n").await.unwrap();
        loop {
            line.clear();
            reader.read_line(&mut line).await.unwrap();
            if !line.starts_with("250-") {
                break;
            }
        }
        let plain = format!("\u{0}{inbox}\u{0}whatever");
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, plain);
        writer
            .write_all(format!("AUTH PLAIN {b64}\r\n").as_bytes())
            .await
            .unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("235"), "{line}");
        Self { reader, writer }
    }

    /// One pipelined batch; `None` once the transport is gone (the kill).
    async fn send_batch(&mut self, subjects: &[String]) -> Option<Vec<String>> {
        let mut mail = String::new();
        for subject in subjects {
            mail.push_str(&format!(
                "MAIL FROM:<sender@x.io>\r\nRCPT TO:<rcpt@x.io>\r\nDATA\r\n\
                 From: sender@x.io\r\nSubject: {subject}\r\n\r\nbody\r\n.\r\n"
            ));
        }
        self.writer.write_all(mail.as_bytes()).await.ok()?;
        let mut observed = Vec::new();
        let mut line = String::new();
        for i in 0..subjects.len() * 4 {
            line.clear();
            // A killed server closes its sockets: the pending replies still
            // arrive, then `Ok(0)` (or a reset). Either way the transport is
            // gone — keep the ids observed so far and stop; only a non-empty
            // non-250 line is a rejection.
            match self.reader.read_line(&mut line).await {
                Ok(0) | Err(_) => return Some(observed),
                Ok(_) => {}
            }
            if i % 4 == 3 {
                assert!(line.starts_with("250"), "mail rejected: {line}");
                observed.push(
                    line.trim()
                        .rsplit_once("stored as ")
                        .expect("the 250 names the stored id")
                        .1
                        .to_string(),
                );
            }
        }
        Some(observed)
    }
}

/// The only net that can see an ack-before-fsync, a commit-after-`250`, a
/// write-behind queue, or a softened pragma: burst real SMTP mail with
/// `--data-file`, SIGKILL the server mid-burst, respawn on the same file,
/// and demand every id whose `250` line was fully observed. No wall-clock
/// durability margin is involved — the kill lands wherever it lands, and
/// only observed acceptances are asserted; the commit precedes the reply
/// byte (decision 0001), so each of them is already durable.
#[tokio::test]
async fn a_hard_kill_loses_no_mail_whose_250_was_observed() {
    let db = std::env::temp_dir().join(format!(
        "swarmail-binary-kill-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let smtp_addr: std::net::SocketAddr = "127.0.0.1:23470".parse().unwrap();
    let http_addr: std::net::SocketAddr = "127.0.0.1:23471".parse().unwrap();
    let spawn_serve = || {
        Command::new(binary())
            .args([
                "serve",
                "--smtp-listen",
                "127.0.0.1:23470",
                "--pop3-listen",
                "127.0.0.1:0",
                "--http-listen",
                "127.0.0.1:23471",
                "--data-file",
            ])
            .arg(&db)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn swarmail serve")
    };

    let mut child = spawn_serve();
    wait_http_up(http_addr).await;

    // Three pipelined writers hammering the same data file, each recording
    // the ids of every observed acceptance.
    let observed_total = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut senders = Vec::new();
    for w in 0..3u32 {
        let addr = smtp_addr;
        let observed_total = observed_total.clone();
        senders.push(tokio::spawn(async move {
            let mut sender = KillBurst::connect(addr, "killbox").await;
            let mut observed: Vec<String> = Vec::new();
            let mut sent = 0u32;
            loop {
                let subjects: Vec<String> = (0..50)
                    .map(|k| format!("kill-{w}-{}-{k}", sent + k))
                    .collect();
                match sender.send_batch(&subjects).await {
                    Some(ids) => {
                        observed_total.fetch_add(ids.len(), std::sync::atomic::Ordering::Relaxed);
                        observed.extend(ids);
                    }
                    None => return observed, // the socket died with the kill
                }
                sent += 50;
            }
        }));
    }

    // Kill mid-burst: as soon as 150 acceptances have been observed, or
    // after a deadline that only exists so a wedged ingest fails the leg
    // loudly instead of hanging it.
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while observed_total.load(std::sync::atomic::Ordering::Relaxed) < 150 {
        assert!(
            std::time::Instant::now() < deadline,
            "the writers never observed 150 acceptances — ingest is wedged"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    unsafe {
        libc::kill(child.id() as i32, libc::SIGKILL);
    }
    // Reap the corpse so the port is free for the respawn.
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if child.try_wait().unwrap().is_some() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("SIGKILLed server did not die");

    // Every sender's socket dies with the process; each returns the ids it
    // observed up to the kill.
    let mut observed: Vec<String> = Vec::new();
    for sender in senders {
        observed.extend(sender.await.unwrap());
    }
    assert!(
        !observed.is_empty(),
        "no 250 was ever observed — the leg would assert nothing"
    );

    // Respawn on the same file and demand every observed id back.
    let mut child = spawn_serve();
    wait_http_up(http_addr).await;
    let (st, list) = common::http_json(
        http_addr,
        "GET",
        "/api/v1/inboxes/killbox/messages?limit=1000000",
        None,
    )
    .await;
    assert_eq!(st, 200);
    let restart: std::collections::HashSet<String> = list["emails"]
        .as_array()
        .expect("email list")
        .iter()
        .map(|e| e["id"].as_str().expect("id").to_string())
        .collect();
    for id in &observed {
        assert!(
            restart.contains(id),
            "a mail whose 250 was observed ({id}) was lost across a hard kill"
        );
    }
    let status = sigint_and_wait(&mut child, Duration::from_secs(30)).await;
    assert!(status.success(), "SIGINT exit: {status:?}");
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", db.display()));
    }
}

/// A unique scratch dir for cert pairs.
fn cert_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "swarmail-bincert-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

/// The operator loop, end to end: `gen-cert` mints the pair, `serve`
/// upgrades real clients with it, and the mail is queryable.
#[tokio::test]
async fn gen_cert_writes_a_pair_the_server_serves() {
    let dir = cert_dir("e2e");
    let out = Command::new(binary())
        .args(["gen-cert", "--domain", "localhost", "--out"])
        .arg(&dir)
        .output()
        .expect("spawn swarmail gen-cert");
    assert!(out.status.success(), "gen-cert failed: {out:?}");
    assert!(String::from_utf8_lossy(&out.stdout).contains("cert.pem"));

    let cert = dir.join("cert.pem");
    let key = dir.join("key.pem");
    assert!(cert.is_file() && key.is_file(), "gen-cert wrote no pair");

    let mut child = Command::new(binary())
        .args([
            "serve",
            "--smtp-listen",
            "127.0.0.1:23460",
            "--pop3-listen",
            "127.0.0.1:0",
            "--http-listen",
            "127.0.0.1:23461",
            "--tls-cert",
        ])
        .arg(&cert)
        .args(["--tls-key"])
        .arg(&key)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn swarmail serve");
    let smtp_addr: std::net::SocketAddr = "127.0.0.1:23460".parse().unwrap();
    wait_http_up("127.0.0.1:23461".parse().unwrap()).await;

    let pair_pem = std::fs::read_to_string(&cert).unwrap();
    let root = rustls_pemfile::certs(&mut pair_pem.as_bytes())
        .next()
        .unwrap()
        .unwrap();
    let mut tls = common::starttls(smtp_addr, "localhost", root).await;
    tls.send("MAIL FROM:<cli@x.io>").await;
    assert!(tls.reply_raw().await.starts_with("250"));
    tls.send("RCPT TO:<cli@y.io>").await;
    assert!(tls.reply_raw().await.starts_with("250"));
    let stored = tls.data("Subject: from the cli\r\n\r\nbody").await;
    assert!(stored.starts_with("250"), "{stored}");
    let status = sigint_and_wait(&mut child, Duration::from_secs(30)).await;
    assert!(status.success(), "SIGINT exit: {status:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The same TLS config through SWARMAIL_TLS_CERT / SWARMAIL_TLS_KEY.
#[tokio::test]
async fn serve_takes_tls_from_the_environment() {
    let dir = cert_dir("env");
    let out = Command::new(binary())
        .args(["gen-cert", "--domain", "localhost", "--out"])
        .arg(&dir)
        .output()
        .expect("spawn swarmail gen-cert");
    assert!(out.status.success());

    let mut child = Command::new(binary())
        .args([
            "serve",
            "--smtp-listen",
            "127.0.0.1:23462",
            "--pop3-listen",
            "127.0.0.1:0",
            "--http-listen",
            "127.0.0.1:23463",
        ])
        .env("SWARMAIL_TLS_CERT", dir.join("cert.pem"))
        .env("SWARMAIL_TLS_KEY", dir.join("key.pem"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn swarmail serve");
    let smtp_addr: std::net::SocketAddr = "127.0.0.1:23462".parse().unwrap();
    wait_http_up("127.0.0.1:23463".parse().unwrap()).await;

    let pair_pem = std::fs::read_to_string(dir.join("cert.pem")).unwrap();
    let root = rustls_pemfile::certs(&mut pair_pem.as_bytes())
        .next()
        .unwrap()
        .unwrap();
    let mut tls = common::starttls(smtp_addr, "localhost", root).await;
    tls.send("NOOP").await;
    assert!(tls.reply_raw().await.starts_with("250"));
    let status = sigint_and_wait(&mut child, Duration::from_secs(30)).await;
    assert!(status.success(), "SIGINT exit: {status:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A malformed cert pair must not sit half-alive: the server exits non-zero
/// and says which side of the pair was wrong.
#[tokio::test]
async fn serve_with_a_malformed_cert_exits_non_zero_and_says_why() {
    let dir = cert_dir("bad");
    std::fs::create_dir_all(&dir).unwrap();
    let cert = dir.join("cert.pem");
    let key = dir.join("key.pem");
    std::fs::write(&cert, "not a certificate").unwrap();
    std::fs::write(&key, "not a key").unwrap();

    let mut child = Command::new(binary())
        .args([
            "serve",
            "--smtp-listen",
            "127.0.0.1:23464",
            "--pop3-listen",
            "127.0.0.1:0",
            "--http-listen",
            "127.0.0.1:23465",
            "--tls-cert",
        ])
        .arg(&cert)
        .args(["--tls-key"])
        .arg(&key)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn swarmail serve");
    let status = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match child.try_wait().unwrap() {
                Some(status) => return status,
                None => tokio::time::sleep(Duration::from_millis(50)).await,
            }
        }
    })
    .await
    .expect("server with a broken cert must exit, not hang");
    assert!(!status.success(), "broken TLS must be a non-zero exit");
    let mut stderr = String::new();
    use std::io::Read;
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert!(stderr.contains("tls-cert"), "{stderr}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// gen-cert failing (the --out path is occupied by a file) exits non-zero.
#[tokio::test]
async fn gen_cert_to_an_occupied_target_exits_non_zero() {
    let occupied = cert_dir("occupied");
    std::fs::write(&occupied, "a file, not a dir").unwrap();
    let out = Command::new(binary())
        .args(["gen-cert", "--domain", "localhost", "--out"])
        .arg(&occupied)
        .output()
        .expect("spawn swarmail gen-cert");
    assert!(!out.status.success(), "occupied --out must fail");
    assert!(String::from_utf8_lossy(&out.stderr).contains("swarmail"));
    let _ = std::fs::remove_file(&occupied);
}

// ---------- decision 0024: the same refusal under the real SIGINT ----------

/// The process leg of the one shutdown policy (0024): the real binary, a
/// disposable data file, the test's own connection holding `BEGIN
/// IMMEDIATE` through exit, three concurrent seeds queued on the mutation
/// boundary — a real SIGINT, and the process exits 0 within the budget
/// (upper bound only, sequence-independent: the seed that wins the boundary
/// first burns its own 5s bound, the queued ones refuse at the check in
/// microseconds). The 300ms pause is sequencing, not an outcome wait. The
/// restarted copy is exactly the committed set — here empty, every commit
/// having refused against the held lock — never a partial row.
#[tokio::test]
async fn sigint_under_queued_seeds_exits_clean_and_stores_nothing() {
    let db = std::env::temp_dir().join(format!(
        "swarmail-sigint-refuse-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let http_addr: std::net::SocketAddr = "127.0.0.1:23473".parse().unwrap();
    let mut child = Command::new(binary())
        .args([
            "serve",
            "--smtp-listen",
            "127.0.0.1:23472",
            "--pop3-listen",
            "127.0.0.1:0",
            "--http-listen",
            "127.0.0.1:23473",
            "--data-file",
        ])
        .arg(&db)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn swarmail serve");
    wait_http_up(http_addr).await;

    // The test's own connection: the writer lock the commits must refuse
    // against, held through the whole exit.
    let _lock = common::hold_write_lock(&db);

    let mut seeds = Vec::new();
    for inbox in ["sigint-one", "sigint-two", "sigint-three"] {
        let body = r#"{"to": "s@x.io", "text": "a mail that must not land"}"#;
        let route = format!("/api/v1/inboxes/{inbox}/seed");
        seeds.push(tokio::spawn(async move {
            common::http_json(http_addr, "POST", &route, Some(body)).await
        }));
    }
    tokio::time::sleep(Duration::from_millis(300)).await; // the seeds queue on the boundary

    // Real SIGINT: the same graceful path `stop()` takes, at the process
    // level. The exit wait is liveness (30s); the budget — at most one
    // busy bound plus ε — is what the API-level held-lock tests prove on
    // the server's own clock.
    let status = sigint_and_wait(&mut child, Duration::from_secs(30)).await;
    assert!(status.success(), "SIGINT exit: {status:?}");

    for seed in seeds {
        let (st, body) = tokio::time::timeout(Duration::from_secs(30), seed)
            .await
            .expect("every queued seed must answer before the process exits")
            .unwrap();
        assert_eq!(st, 503, "{body}");
        assert_eq!(
            body,
            serde_json::json!({ "error": "storage commit failed" }),
            "{body}"
        );
    }

    // The restarted copy is exactly the committed set: empty. The lock is
    // released first — the restart must be able to restore.
    drop(_lock);
    let mut restarted = Command::new(binary())
        .args([
            "serve",
            "--smtp-listen",
            "127.0.0.1:23472",
            "--pop3-listen",
            "127.0.0.1:0",
            "--http-listen",
            "127.0.0.1:23473",
            "--data-file",
        ])
        .arg(&db)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("restart swarmail serve");
    wait_http_up(http_addr).await;
    for inbox in ["sigint-one", "sigint-two", "sigint-three"] {
        let (st, list) = common::http_json(
            http_addr,
            "GET",
            &format!("/api/v1/inboxes/{inbox}/messages"),
            None,
        )
        .await;
        assert_eq!(st, 200, "{list}");
        assert_eq!(list["total"], 0, "the refused set never landed: {list}");
    }
    let status = sigint_and_wait(&mut restarted, Duration::from_secs(30)).await;
    assert!(status.success(), "restart SIGINT exit: {status:?}");
    let _ = std::fs::remove_file(&db);
    let _ = std::fs::remove_file(db.with_extension("db-wal"));
    let _ = std::fs::remove_file(db.with_extension("db-shm"));
}
