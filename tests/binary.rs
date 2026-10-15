//! The shipped binary itself: serve binds, answers healthz, and shuts down
//! cleanly on SIGINT; the mcp bridge exits 0 at EOF. These tests spawn the
//! instrumented binary, so `src/main.rs` is covered like everything else.

mod common;

use std::net::SocketAddr;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::{Child, ChildStdout, Command};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_swarmail")
}

/// A `serve` child whose bound addresses were read back from its own
/// startup line (the tracing subscriber's stdout). Fixed ports would make
/// these tests collide with themselves — cargo runs a binary's tests in
/// parallel, and one leaked server from a failed assertion would wedge
/// every later run — so the child binds `:0` and the test learns the real
/// ports from the process's own observability: the same startup line an
/// operator sees. On drop, a child that was not reaped is killed: a
/// failing assertion must never leak a server (still bound, still holding
/// its data file) into the rest of the run.
struct Serve {
    child: Child,
    smtp_addr: SocketAddr,
    http_addr: SocketAddr,
    /// The reader parked just past the startup line: the signal legs
    /// sequence on the server's own log lines, never a sleep.
    stdout: Option<BufReader<ChildStdout>>,
    reaped: bool,
}

impl Drop for Serve {
    fn drop(&mut self) {
        if !self.reaped {
            // A failed assertion must not leave the binary alive: SIGKILL
            // is fired synchronously (`start_kill`) — `Drop` cannot await
            // the reap, and the killed process cannot outlive this test
            // process for long.
            let _ = self.child.start_kill();
        }
    }
}

/// The startup line carries the bound addrs as tracing fields
/// (`... smtp=127.0.0.1:45678 http=127.0.0.1:45679`); the prose in the
/// message body would be a format contract too fragile to parse.
fn parse_addrs(line: &str) -> Option<(SocketAddr, SocketAddr)> {
    let smtp = line.split("smtp=").nth(1)?.split_whitespace().next()?;
    let http = line.split("http=").nth(1)?.split_whitespace().next()?;
    Some((smtp.parse().ok()?, http.parse().ok()?))
}

/// A `serve` command on OS-assigned ports.
fn serve_command() -> Command {
    let mut cmd = Command::new(binary());
    cmd.args([
        "serve",
        "--smtp-listen",
        "127.0.0.1:0",
        "--pop3-listen",
        "127.0.0.1:0",
        "--http-listen",
        "127.0.0.1:0",
    ]);
    cmd
}

/// `serve --data-file <db>` on OS-assigned ports — the persistence tests'
/// spawn shape.
fn serve_with_data(db: &std::path::Path) -> Command {
    let mut cmd = serve_command();
    cmd.arg("--data-file").arg(db);
    cmd
}

/// Spawn `serve` and read its bound addrs off the startup line; the rest of
/// stderr is drained in the background so the pipe can never fill and block
/// the server. A server that dies before it is up fails here loudly, with
/// whatever it said on the way out.
async fn spawn_serve(cmd: Command) -> Serve {
    let mut cmd = cmd;
    // The startup line is the tracing subscriber's output — stdout. Both
    // pipes are drained (stderr in the background) so neither can fill and
    // block the server.
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn swarmail serve");
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    tokio::spawn(async move {
        let mut reader = BufReader::new(stderr);
        let mut rest = String::new();
        loop {
            let n = reader.read_line(&mut rest).await.unwrap_or(0);
            if n == 0 {
                break;
            }
            rest.clear();
        }
    });
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    let mut transcript = String::new();
    let addrs = loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .await
            .expect("read the startup line");
        if n == 0 {
            break None;
        }
        if let Some(pair) = parse_addrs(&line) {
            break Some(pair);
        }
        transcript.push_str(&line);
    };
    let (smtp_addr, http_addr) =
        addrs.unwrap_or_else(|| panic!("the server exited before it was up:\n{transcript}"));
    Serve {
        child,
        smtp_addr,
        http_addr,
        stdout: Some(reader),
        reaped: false,
    }
}

/// Read the server's own stdout until a line mentioning `needle` appears
/// (or fail loudly): the sequencing anchor for the signal legs — the second
/// signal must land *during* the drain, which only the server's own
/// "shutting down" line can attest without a sleep.
async fn wait_for_line(serve: &mut Serve, needle: &str, deadline: Duration) -> String {
    let reader = serve.stdout.as_mut().expect("piped stdout");
    let end = tokio::time::Instant::now() + deadline;
    let mut transcript = String::new();
    let mut line = String::new();
    loop {
        assert!(
            tokio::time::Instant::now() < end,
            "the server never said {needle:?}: {transcript}"
        );
        match tokio::time::timeout(Duration::from_millis(200), reader.read_line(&mut line)).await {
            Ok(Ok(0)) | Err(_) => continue,
            Ok(Ok(_)) => {
                transcript.push_str(&line);
                if line.contains(needle) {
                    return std::mem::take(&mut line);
                }
                line.clear();
            }
            Ok(Err(e)) => panic!("reading the server's stdout failed: {e}"),
        }
    }
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
async fn signal_and_wait(
    child: &mut Child,
    sig: libc::c_int,
    deadline: Duration,
) -> std::process::ExitStatus {
    unsafe {
        libc::kill(child.id().expect("a live pid") as i32, sig);
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
    .expect("the server did not stop on the signal")
}

#[tokio::test]
async fn serve_answers_healthz_and_stops_on_sigint() {
    let mut serve = spawn_serve(serve_command()).await;
    wait_http_up(serve.http_addr).await;
    let status = signal_and_wait(&mut serve.child, libc::SIGINT, Duration::from_secs(30)).await;
    serve.reaped = true;
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
    // First run: --data-file as the flag; one real SMTP mail in.
    let mut first = spawn_serve(serve_with_data(&db)).await;
    wait_http_up(first.http_addr).await;
    let reply = common::smtp_send(
        first.smtp_addr,
        Some("binpersist"),
        "a@x.io",
        "b@x.io",
        "Binary persist",
        "the body",
    )
    .await
    .expect("smtp send");
    assert!(reply.starts_with("250"), "{reply}");
    let status = signal_and_wait(&mut first.child, libc::SIGINT, Duration::from_secs(30)).await;
    first.reaped = true;
    assert!(status.success(), "SIGINT exit: {status:?}");

    // Restart with the SWARMAIL_DATA_FILE env knob instead: same file, full
    // state back, and the port answers again.
    let mut restarted = {
        let mut cmd = serve_command();
        cmd.env("SWARMAIL_DATA_FILE", &db);
        spawn_serve(cmd).await
    };
    wait_http_up(restarted.http_addr).await;
    let (st, list) = common::http_json(
        restarted.http_addr,
        "GET",
        "/api/v1/inboxes/binpersist/messages",
        None,
    )
    .await;
    assert_eq!(st, 200);
    assert_eq!(list["total"], 1, "mail did not survive the restart: {list}");
    assert_eq!(list["emails"][0]["subject"], "Binary persist");
    let status = signal_and_wait(&mut restarted.child, libc::SIGINT, Duration::from_secs(30)).await;
    restarted.reaped = true;
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
    let status = child.wait().await.unwrap();
    assert!(status.success(), "mcp EOF exit: {status:?}");
}

/// The mcp subcommand's stdout is protocol and nothing else (decision 0029
/// §4): its tracing goes to stderr, so a line-parsing client reads exactly
/// the JSON-RPC replies it asked for — one request in, one reply out, and
/// stdout parses as that reply alone.
#[tokio::test]
async fn mcp_subcommand_keeps_stdout_protocol_only() {
    let mut child = Command::new(binary())
        .args(["mcp", "--url", "http://127.0.0.1:1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn swarmail mcp");
    let request = br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(request)
        .await
        .expect("feed the bridge");
    let out = child.wait_with_output().await.expect("mcp child");
    assert!(out.status.success(), "mcp exit: {out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let reply: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout is protocol only: {stdout:?}");
    assert_eq!(reply["id"], 1, "{reply}");
    assert_eq!(reply["error"]["code"], -32000, "{reply}");
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
    let mut serve = spawn_serve(serve_with_data(&db)).await;
    wait_http_up(serve.http_addr).await;
    let smtp_addr = serve.smtp_addr;

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
        libc::kill(serve.child.id().expect("a live pid") as i32, libc::SIGKILL);
    }
    // Reap the corpse — nothing is left to the reaper, and the pid can
    // never be reused under the guard's later kill.
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if serve.child.try_wait().unwrap().is_some() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("SIGKILLed server did not die");
    serve.reaped = true;

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
    let mut restarted = spawn_serve(serve_with_data(&db)).await;
    wait_http_up(restarted.http_addr).await;
    let (st, list) = common::http_json(
        restarted.http_addr,
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
    let status = signal_and_wait(&mut restarted.child, libc::SIGINT, Duration::from_secs(30)).await;
    restarted.reaped = true;
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
        .await
        .expect("spawn swarmail gen-cert");
    assert!(out.status.success(), "gen-cert failed: {out:?}");
    assert!(String::from_utf8_lossy(&out.stdout).contains("cert.pem"));

    let cert = dir.join("cert.pem");
    let key = dir.join("key.pem");
    assert!(cert.is_file() && key.is_file(), "gen-cert wrote no pair");

    let mut serve = {
        let mut cmd = serve_command();
        cmd.arg("--tls-cert").arg(&cert);
        cmd.args(["--tls-key"]).arg(&key);
        spawn_serve(cmd).await
    };
    wait_http_up(serve.http_addr).await;
    let smtp_addr = serve.smtp_addr;

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
    let status = signal_and_wait(&mut serve.child, libc::SIGINT, Duration::from_secs(30)).await;
    serve.reaped = true;
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
        .await
        .expect("spawn swarmail gen-cert");
    assert!(out.status.success());

    let mut serve = {
        let mut cmd = serve_command();
        cmd.env("SWARMAIL_TLS_CERT", dir.join("cert.pem"));
        cmd.env("SWARMAIL_TLS_KEY", dir.join("key.pem"));
        spawn_serve(cmd).await
    };
    wait_http_up(serve.http_addr).await;
    let smtp_addr = serve.smtp_addr;

    let pair_pem = std::fs::read_to_string(dir.join("cert.pem")).unwrap();
    let root = rustls_pemfile::certs(&mut pair_pem.as_bytes())
        .next()
        .unwrap()
        .unwrap();
    let mut tls = common::starttls(smtp_addr, "localhost", root).await;
    tls.send("NOOP").await;
    assert!(tls.reply_raw().await.starts_with("250"));
    let status = signal_and_wait(&mut serve.child, libc::SIGINT, Duration::from_secs(30)).await;
    serve.reaped = true;
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
            "127.0.0.1:0",
            "--pop3-listen",
            "127.0.0.1:0",
            "--http-listen",
            "127.0.0.1:0",
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
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .await
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
        .await
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
/// microseconds). The seeds' arrival on the boundary is observed over
/// `/metrics`, not timed for. The restarted copy is exactly the committed
/// set — here empty, every commit having refused against the held lock —
/// never a partial row.
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
    let mut serve = spawn_serve(serve_with_data(&db)).await;
    wait_http_up(serve.http_addr).await;
    let http_addr = serve.http_addr;

    // The test's own connection: the writer lock the commits must refuse
    // against, held through the whole exit.
    let _lock = common::hold_write_lock(&db);

    let mut seeds = Vec::new();
    for inbox in ["sigint-one", "sigint-two", "sigint-three"] {
        let body = r#"{"to": "s@x.io", "text": "a mail that must not land"}"#;
        let route = format!("/api/v1/inboxes/{inbox}/seed");
        seeds.push(tokio::spawn(async move {
            // The refusal is what the held lock produces on the server's
            // own clock, but 0018 §2 leaves the client-visible half open:
            // a commit in flight at stop either gets its 503 or sees the
            // socket die first (EOF) — macOS runners have shown both.
            // What the test forbids is a stored mail, proven below by the
            // restart, never a particular client-visible answer.
            let (st, text) =
                common::http_send_text_allow_eof(http_addr, "POST", &route, Some(body)).await;
            let parsed =
                serde_json::from_str::<serde_json::Value>(&text).unwrap_or(serde_json::json!({}));
            (st, parsed)
        }));
    }
    // The seeds are inside the router, observed: each seed's own labeled
    // row — the request is in flight and its commit is queued on the
    // mutation boundary. A per-inbox row can only be raised by this
    // process's seeds, never a parallel test's scrape (issue #51).
    for inbox in ["sigint-one", "sigint-two", "sigint-three"] {
        common::wait_for_metric_row(
            http_addr,
            &format!("swarmail_http_requests_in_flight{{inbox={inbox:?}}}"),
            1,
        )
        .await;
    }

    // Real SIGINT: the same graceful path `stop()` takes, at the process
    // level. The exit wait is liveness (30s); the budget — at most one
    // busy bound plus ε — is what the API-level held-lock tests prove on
    // the server's own clock.
    let status = signal_and_wait(&mut serve.child, libc::SIGINT, Duration::from_secs(30)).await;
    serve.reaped = true;
    assert!(status.success(), "SIGINT exit: {status:?}");

    for seed in seeds {
        let (st, body) = tokio::time::timeout(Duration::from_secs(30), seed)
            .await
            .expect("every queued seed must answer before the process exits")
            .unwrap();
        assert!(
            st == 503 || st == 0,
            "a seed answered something other than the refusal or EOF: {st} {body}"
        );
        if st == 503 {
            assert_eq!(
                body,
                serde_json::json!({ "error": "storage commit failed" }),
                "{body}"
            );
        }
    }

    // The restarted copy is exactly the committed set: empty. The lock is
    // released first — the restart must be able to restore.
    drop(_lock);
    let mut restarted = spawn_serve(serve_with_data(&db)).await;
    wait_http_up(restarted.http_addr).await;
    for inbox in ["sigint-one", "sigint-two", "sigint-three"] {
        let (st, list) = common::http_json(
            restarted.http_addr,
            "GET",
            &format!("/api/v1/inboxes/{inbox}/messages"),
            None,
        )
        .await;
        assert_eq!(st, 200, "{list}");
        assert_eq!(list["total"], 0, "the refused set never landed: {list}");
    }
    let status = signal_and_wait(&mut restarted.child, libc::SIGINT, Duration::from_secs(30)).await;
    restarted.reaped = true;
    assert!(status.success(), "restart SIGINT exit: {status:?}");
    let _ = std::fs::remove_file(&db);
    let _ = std::fs::remove_file(db.with_extension("db-wal"));
    let _ = std::fs::remove_file(db.with_extension("db-shm"));
}

/// SIGTERM is the signal a container manager actually sends (`docker stop`
/// delivers the image's STOPSIGNAL — 0027 fixes it to SIGTERM). It must
/// take the same graceful path as SIGINT — exit 0, no half-drained
/// process — and everything committed before the signal must be queryable
/// after a restart on the same data file.
#[tokio::test]
async fn sigterm_takes_the_graceful_path_and_the_mail_survives_a_restart() {
    let db = std::env::temp_dir().join(format!(
        "swarmail-sigterm-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut serve = spawn_serve(serve_with_data(&db)).await;
    wait_http_up(serve.http_addr).await;
    let reply = common::smtp_send(
        serve.smtp_addr,
        Some("sigterm"),
        "a@x.io",
        "b@x.io",
        "Term graceful",
        "the body",
    )
    .await
    .expect("smtp send");
    assert!(reply.starts_with("250"), "{reply}");

    let status = signal_and_wait(&mut serve.child, libc::SIGTERM, Duration::from_secs(30)).await;
    serve.reaped = true;
    assert!(status.success(), "SIGTERM exit: {status:?}");

    // The same data file, a fresh process: the committed mail is queryable.
    let mut again = spawn_serve(serve_with_data(&db)).await;
    wait_http_up(again.http_addr).await;
    let (st, list) = common::http_json(
        again.http_addr,
        "GET",
        "/api/v1/inboxes/sigterm/messages",
        None,
    )
    .await;
    assert_eq!(st, 200, "{list}");
    assert_eq!(list["total"], 1, "mail did not survive SIGTERM: {list}");
    assert_eq!(list["emails"][0]["subject"], "Term graceful");
    let status = signal_and_wait(&mut again.child, libc::SIGINT, Duration::from_secs(30)).await;
    again.reaped = true;
    assert!(status.success(), "restart SIGINT exit: {status:?}");
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", db.display()));
    }
}

/// The drain is held open by a body that never lands, so the second signal
/// provably arrives *during* it: the sequence is anchored on the server's
/// own `shutting down` line, never a sleep. The operator has said they are
/// done — the process exits 70 instead of outliving its drain.
#[tokio::test]
async fn a_second_signal_during_the_drain_exits_70() {
    let mut serve = spawn_serve(serve_command()).await;
    wait_http_up(serve.http_addr).await;

    // The park: a dispatched request whose body never completes. Its 100
    // Continue is the proof the handler is waiting on bytes (and so the
    // drain is held), not on a guess.
    let mut a = TcpStream::connect(serve.http_addr).await.unwrap();
    a.write_all(
        b"POST /api/v1/inboxes/stall/seed HTTP/1.1\r\nHost: t\r\n\
          Expect: 100-continue\r\nContent-Type: application/json\r\n\
          Content-Length: 10\r\n\r\n",
    )
    .await
    .unwrap();
    a.write_all(b"{\"to").await.unwrap();
    let mut first = [0u8; 64];
    let n = tokio::time::timeout(Duration::from_secs(10), a.read(&mut first))
        .await
        .expect("hyper never asked for the body")
        .unwrap();
    assert!(first[..n].starts_with(b"HTTP/1.1 100"), "{:?}", &first[..n]);

    // First SIGTERM: the graceful drain begins, and says so on stdout.
    unsafe {
        libc::kill(serve.child.id().expect("a live pid") as i32, libc::SIGTERM);
    }
    wait_for_line(&mut serve, "shutting down", Duration::from_secs(10)).await;

    // The second signal lands inside the drain: forced exit 70.
    let status = signal_and_wait(&mut serve.child, libc::SIGTERM, Duration::from_secs(30)).await;
    serve.reaped = true;
    assert_eq!(status.code(), Some(70), "second SIGTERM exit: {status:?}");
}

/// The exhausted delivery's cause is visible in the shipped binary's own
/// stdout at the default level (issue #47): a target that cannot be dialed
/// burns its four attempts and the final `warn!` carries the target URL and
/// the cause — no debug flag, no new metrics family. The line is read off
/// the process's own stdout: the same stream an operator tails.
#[tokio::test]
async fn an_exhausted_delivery_names_its_target_and_cause_in_stdout() {
    let mut serve = spawn_serve(serve_command()).await;
    wait_http_up(serve.http_addr).await;

    // A port nothing listens on: every attempt is refused at once, so the
    // four-attempt budget burns in ~2.1 s of backoff, not 22 s of timeouts.
    let (st, resp) = common::http_json(
        serve.http_addr,
        "PUT",
        "/api/v1/webhooks",
        Some(r#"[{"url": "http://127.0.0.1:1/hook"}]"#),
    )
    .await;
    assert_eq!(st, 200, "{resp}");

    common::smtp_send(
        serve.smtp_addr,
        None,
        "f@x.io",
        "r@x.io",
        "visible cause",
        "x",
    )
    .await
    .expect("the mail is accepted");

    let line = wait_for_line(
        &mut serve,
        "webhook dropped after retries",
        Duration::from_secs(30),
    )
    .await;
    assert!(
        line.contains("http://127.0.0.1:1/hook"),
        "the warn names the target: {line:?}"
    );
    assert!(
        line.contains("error="),
        "the warn carries the cause: {line:?}"
    );

    let status = signal_and_wait(&mut serve.child, libc::SIGINT, Duration::from_secs(30)).await;
    serve.reaped = true;
    assert!(status.success(), "SIGINT exit: {status:?}");
}
