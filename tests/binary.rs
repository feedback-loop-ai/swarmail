//! The shipped binary itself: serve binds, answers healthz, and shuts down
//! cleanly on SIGINT; the mcp bridge exits 0 at EOF. These tests spawn the
//! instrumented binary, so `src/main.rs` is covered like everything else.

mod common;

use std::process::{Child, Command, Stdio};
use std::time::Duration;

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

/// SIGINT = ctrl_c: the graceful-shutdown path, then a clean exit 0.
async fn sigint_and_wait(child: &mut Child) -> std::process::ExitStatus {
    unsafe {
        libc::kill(child.id() as i32, libc::SIGINT);
    }
    tokio::time::timeout(Duration::from_secs(10), async {
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
    let status = sigint_and_wait(&mut child).await;
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
    let status = sigint_and_wait(&mut child).await;
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
    let status = sigint_and_wait(&mut child).await;
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
    let status = sigint_and_wait(&mut child).await;
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
    let status = sigint_and_wait(&mut child).await;
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
