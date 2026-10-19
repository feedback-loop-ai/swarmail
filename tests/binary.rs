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
    pop3_addr: SocketAddr,
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
/// (`... smtp=127.0.0.1:45678 pop3=127.0.0.1:45679 http=127.0.0.1:45680`);
/// the prose in the message body would be a format contract too fragile to
/// parse.
fn parse_addrs(line: &str) -> Option<(SocketAddr, SocketAddr, SocketAddr)> {
    let smtp = line.split("smtp=").nth(1)?.split_whitespace().next()?;
    let pop3 = line.split("pop3=").nth(1)?.split_whitespace().next()?;
    let http = line.split("http=").nth(1)?.split_whitespace().next()?;
    Some((smtp.parse().ok()?, pop3.parse().ok()?, http.parse().ok()?))
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
    let (smtp_addr, pop3_addr, http_addr) =
        addrs.unwrap_or_else(|| panic!("the server exited before it was up:\n{transcript}"));
    Serve {
        child,
        smtp_addr,
        pop3_addr,
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

/// Issue #88's total over real HTTP (decision 0043 §5). The total is a
/// process-global static, so only a serve child of its own reads it
/// exactly; per-inbox uniqueness cannot isolate it inside the parallel
/// in-process suites. Every request counts, the scrape included: quiet
/// scrapes read 1, one parked await plus the scrape reads 2, and once the
/// wait is satisfied over real SMTP and joined, scrapes read 1 again —
/// 0/1/0 with the observer subtracted. The labeled awaits row is the
/// readiness signal, never a sleep. Red before the repair: the guard
/// only decremented, and the first scrape read `18446744073709551615`.
#[tokio::test]
async fn the_in_flight_total_counts_each_request_once_and_returns_to_rest() {
    let serve = spawn_serve(serve_command()).await;
    wait_http_up(serve.http_addr).await;
    let addr = serve.http_addr;
    let total = "swarmail_http_requests_in_flight";
    for _ in 0..3 {
        assert_eq!(common::metric_now(addr, total).await, 1, "a quiet scrape");
    }
    let wait = tokio::spawn(async move {
        common::http_json(
            addr,
            "GET",
            "/api/v1/inboxes/totalbox/await?timeout_ms=60000",
            None,
        )
        .await
    });
    common::wait_for_metric_row(addr, "swarmail_awaits_active{inbox=\"totalbox\"}", 1).await;
    let (st, text) = common::http_get_text(addr, "/metrics").await;
    assert_eq!(st, 200, "{text}");
    assert_eq!(
        common::metric_value(&text, total),
        Some(2),
        "the parked await and the scrape:\n{text}"
    );
    let reply = common::smtp_send(
        serve.smtp_addr,
        Some("totalbox"),
        "a@x.io",
        "b@x.io",
        "Total",
        "the body",
    )
    .await
    .expect("smtp send");
    assert!(reply.starts_with("250"), "{reply}");
    let (st, body) = wait.await.expect("the await joins");
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["matched"], 1, "{body}");
    for _ in 0..3 {
        assert_eq!(common::metric_now(addr, total).await, 1, "back at rest");
    }
}

/// The three bind knobs: env var, flag, and compiled default port.
const BIND_KNOBS: [(&str, &str, u16); 3] = [
    ("SWARMAIL_SMTP_LISTEN", "--smtp-listen", 1025),
    ("SWARMAIL_POP3_LISTEN", "--pop3-listen", 1110),
    ("SWARMAIL_HTTP_LISTEN", "--http-listen", 8025),
];

/// The bind knobs work in their env shape alone — no flags, the three
/// `SWARMAIL_*_LISTEN` vars, the spelling the image and the chart use
/// (decision 0030 §1), each `127.0.0.1:0`. The child owns its ports from
/// the bind on — no port is chosen, released and named by the test, so a
/// parallel test can never take one first (issue #143) — and every
/// connection below goes to the address the ready line printed. An
/// ignored env value binds the compiled default port instead (or fails
/// to start when that port is busy), so each printed port must be an
/// ephemeral one, not its default. The held-listener cases below are the
/// other half: an env value naming an occupied port must refuse.
#[tokio::test]
async fn serve_takes_its_binds_from_the_environment() {
    let mut cmd = Command::new(binary());
    cmd.args(["serve"]);
    for (var, _, _) in BIND_KNOBS {
        cmd.env(var, "127.0.0.1:0");
    }
    let mut serve = spawn_serve(cmd).await;
    let bound = [serve.smtp_addr, serve.pop3_addr, serve.http_addr];
    for ((var, _, default), addr) in BIND_KNOBS.into_iter().zip(bound) {
        assert_ne!(
            addr.port(),
            default,
            "{var}=127.0.0.1:0 must bind an ephemeral port, not the default: {addr}"
        );
    }
    let mut smtp = common::SmtpConn::connect(serve.smtp_addr).await;
    smtp.send("QUIT").await;
    assert!(smtp.reply().await.starts_with("221"), "SMTP answers");
    let mut pop3 = common::Pop3::connect(serve.pop3_addr).await;
    assert!(pop3.cmd("QUIT").await.starts_with("+OK"), "POP3 answers");
    let (st, body) = common::http_json(serve.http_addr, "GET", "/healthz", None).await;
    assert_eq!(st, 200, "the env-bound server answers healthz: {body}");
    let status = signal_and_wait(&mut serve.child, libc::SIGINT, Duration::from_secs(30)).await;
    serve.reaped = true;
    assert!(status.success(), "SIGINT exit: {status:?}");
}

/// One protocol's env bind names a port this test holds occupied for the
/// whole case; the other two binds are flags on `:0`, so the only bind
/// that can fail is the env one. The child must exit non-zero with the
/// address-in-use reason and never print its ready line. A child that
/// ignored the env value would bind its compiled default instead, come
/// up, and fail this case.
async fn assert_env_bind_refused(var: &str) {
    let held = std::net::TcpListener::bind("127.0.0.1:0").expect("a held port binds");
    let addr = held.local_addr().expect("the held addr is local");
    let mut cmd = Command::new(binary());
    cmd.arg("serve");
    for (other, flag, _) in BIND_KNOBS {
        if other == var {
            cmd.env(var, addr.to_string());
        } else {
            cmd.args([flag, "127.0.0.1:0"]);
        }
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd.kill_on_drop(true);
    let mut child = cmd.spawn().expect("spawn swarmail serve");
    let mut stdout = BufReader::new(child.stdout.take().expect("piped stdout"));
    let mut stderr = child.stderr.take().expect("piped stderr");
    // Stdout to EOF, line by line: a ready line means the env value was
    // not the bind, and fails at once rather than at the deadline.
    let mut transcript = String::new();
    loop {
        let mut line = String::new();
        let n = tokio::time::timeout(Duration::from_secs(30), stdout.read_line(&mut line))
            .await
            .unwrap_or_else(|_| panic!("{var}={addr}: the child neither exited nor came up"))
            .expect("read the child's stdout");
        if n == 0 {
            break;
        }
        assert!(
            parse_addrs(&line).is_none(),
            "{var}={addr}: the server came up although the env named an occupied port: {line}"
        );
        transcript.push_str(&line);
    }
    let mut reason = String::new();
    tokio::time::timeout(Duration::from_secs(30), stderr.read_to_string(&mut reason))
        .await
        .expect("the child's stderr closes")
        .expect("read the child's stderr");
    let status = tokio::time::timeout(Duration::from_secs(30), child.wait())
        .await
        .expect("the refused child exits")
        .expect("the child's status");
    assert!(
        !status.success(),
        "{var}={addr} must refuse: {status:?}\n{transcript}{reason}"
    );
    assert!(
        reason.contains("swarmail: ") && reason.contains("in use"),
        "{var}={addr}: the refusal says the address is in use: {reason}"
    );
    drop(held);
}

#[tokio::test]
async fn an_smtp_env_bind_on_a_held_port_refuses() {
    assert_env_bind_refused("SWARMAIL_SMTP_LISTEN").await;
}

#[tokio::test]
async fn a_pop3_env_bind_on_a_held_port_refuses() {
    assert_env_bind_refused("SWARMAIL_POP3_LISTEN").await;
}

#[tokio::test]
async fn an_http_env_bind_on_a_held_port_refuses() {
    assert_env_bind_refused("SWARMAIL_HTTP_LISTEN").await;
}

#[tokio::test]
async fn serve_restores_a_data_file_across_a_restart() {
    let dir = tempfile::TempDir::with_prefix("swarmail-binary-").unwrap();
    let db = dir.path().join("persist.db");
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
    drop(restarted);
    drop(dir);
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
    // A bridge that ignores its stdin must not hang the test forever: the
    // wait is bounded, naming the step (issue #102).
    let status = tokio::time::timeout(common::HELPER_DEADLINE, child.wait())
        .await
        .expect("the mcp bridge never exited")
        .unwrap();
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
    let out = tokio::time::timeout(common::HELPER_DEADLINE, child.wait_with_output())
        .await
        .expect("the mcp bridge never answered")
        .expect("mcp child");
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
    let dir = tempfile::TempDir::with_prefix("swarmail-binary-kill-").unwrap();
    let db = dir.path().join("kill.db");
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

    // Respawn on the same file and demand every observed id back. The read
    // pages like any client must (decision 0030 §3) — no harness asks for
    // an unbounded page.
    let mut restarted = spawn_serve(serve_with_data(&db)).await;
    wait_http_up(restarted.http_addr).await;
    let restart = common::list_all_ids(restarted.http_addr, "killbox").await;
    for id in &observed {
        assert!(
            restart.contains(id),
            "a mail whose 250 was observed ({id}) was lost across a hard kill"
        );
    }
    let status = signal_and_wait(&mut restarted.child, libc::SIGINT, Duration::from_secs(30)).await;
    restarted.reaped = true;
    assert!(status.success(), "SIGINT exit: {status:?}");
    drop(restarted);
    drop(dir);
}

/// A unique scratch dir for cert pairs, removed by its guard's drop — even
/// when the test panics (issue #143). Paths that must not exist yet (the
/// `--out` target gen-cert creates) are placed inside it.
fn cert_dir(name: &str) -> tempfile::TempDir {
    tempfile::TempDir::with_prefix(format!("swarmail-bincert-{name}-")).unwrap()
}

/// The operator loop, end to end: `gen-cert` mints the pair, `serve`
/// upgrades real clients with it, and the mail is queryable.
#[tokio::test]
async fn gen_cert_writes_a_pair_the_server_serves() {
    let scratch = cert_dir("e2e");
    let dir = scratch.path().join("pair");
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
    drop(scratch);
}

/// The same TLS config through SWARMAIL_TLS_CERT / SWARMAIL_TLS_KEY.
#[tokio::test]
async fn serve_takes_tls_from_the_environment() {
    let scratch = cert_dir("env");
    let dir = scratch.path().join("pair");
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
    drop(scratch);
}

/// A malformed cert pair must not sit half-alive: the server exits non-zero
/// and says which side of the pair was wrong.
#[tokio::test]
async fn serve_with_a_malformed_cert_exits_non_zero_and_says_why() {
    let dir = cert_dir("bad");
    let cert = dir.path().join("cert.pem");
    let key = dir.path().join("key.pem");
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
    drop(dir);
}

/// gen-cert failing (the --out path is occupied by a file) exits non-zero.
#[tokio::test]
async fn gen_cert_to_an_occupied_target_exits_non_zero() {
    let scratch = cert_dir("occupied");
    let occupied = scratch.path().join("occupied");
    std::fs::write(&occupied, "a file, not a dir").unwrap();
    let out = Command::new(binary())
        .args(["gen-cert", "--domain", "localhost", "--out"])
        .arg(&occupied)
        .output()
        .await
        .expect("spawn swarmail gen-cert");
    assert!(!out.status.success(), "occupied --out must fail");
    assert!(String::from_utf8_lossy(&out.stderr).contains("swarmail"));
    drop(scratch);
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
    let dir = tempfile::TempDir::with_prefix("swarmail-sigint-refuse-").unwrap();
    let db = dir.path().join("refuse.db");
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
    drop(restarted);
    drop(dir);
}

/// SIGTERM is the signal a container manager actually sends (`docker stop`
/// delivers the image's STOPSIGNAL — 0027 fixes it to SIGTERM). It must
/// take the same graceful path as SIGINT — exit 0, no half-drained
/// process — and everything committed before the signal must be queryable
/// after a restart on the same data file.
#[tokio::test]
async fn sigterm_takes_the_graceful_path_and_the_mail_survives_a_restart() {
    let dir = tempfile::TempDir::with_prefix("swarmail-sigterm-").unwrap();
    let db = dir.path().join("sigterm.db");
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
    drop(again);
    drop(dir);
}

/// One server per data file (decision 0033, issue #85): while server A
/// owns the file, server B on the same path exits non-zero with the named
/// refusal and never announces readiness, and A keeps serving. A SIGKILLed
/// and reaped A frees the file for C at once with its mail intact; a
/// gracefully stopped C frees it for D. The sidecar is never removed, and
/// nothing sleeps or retries: readiness and process exit order every step.
#[tokio::test]
async fn a_second_server_on_a_held_data_file_refuses_and_a_dead_owner_frees_it() {
    let dir = tempfile::TempDir::with_prefix("swarmail-binary-one-writer-").unwrap();
    let db = dir.path().join("mail.db");
    let mut a = spawn_serve(serve_with_data(&db)).await;
    wait_http_up(a.http_addr).await;

    // B: same file, its own ephemeral ports. `kill_on_drop` reaps it even
    // if the deadline fires, so a hung B can never leak into the run.
    let mut second = serve_with_data(&db);
    second
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let out = tokio::time::timeout(common::HELPER_DEADLINE, second.output())
        .await
        .expect("a refused second server exits instead of serving")
        .expect("spawn the second server");
    assert!(
        !out.status.success(),
        "the second server must exit non-zero: {out:?}"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    let refusal = format!(
        "the data file is already in use by another server: {}",
        db.display()
    );
    assert!(stderr.contains(&refusal), "{stderr}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.lines().all(|line| parse_addrs(line).is_none()),
        "the refused server announced readiness: {stdout}"
    );

    // A is untouched by the refusal: healthy, accepting, queryable.
    let (st, body) = common::http_json(a.http_addr, "GET", "/healthz", None).await;
    assert_eq!(st, 200, "the owner keeps serving: {body}");
    let reply = common::smtp_send(
        a.smtp_addr,
        Some("onewriter"),
        "a@x.io",
        "b@x.io",
        "Owned",
        "the body",
    )
    .await
    .expect("smtp send");
    assert!(reply.starts_with("250"), "{reply}");
    let path = "/api/v1/inboxes/onewriter/messages";
    let (st, list) = common::http_json(a.http_addr, "GET", path, None).await;
    assert_eq!(st, 200, "{list}");
    assert_eq!(list["total"], 1, "the owner stores and serves: {list}");

    // Kill A outright and reap it: the kernel drops the lock with the
    // process, and the inert sidecar stays where it was.
    tokio::time::timeout(common::HELPER_DEADLINE, a.child.kill())
        .await
        .expect("the killed owner is reaped")
        .expect("kill the owner");
    a.reaped = true;
    assert!(dir.path().join("mail.db.lock").is_file());

    let mut c = spawn_serve(serve_with_data(&db)).await;
    wait_http_up(c.http_addr).await;
    let (st, list) = common::http_json(c.http_addr, "GET", path, None).await;
    assert_eq!(st, 200, "{list}");
    assert_eq!(list["total"], 1, "the killed owner's mail survives: {list}");
    assert_eq!(list["emails"][0]["subject"], "Owned");
    let status = signal_and_wait(&mut c.child, libc::SIGTERM, Duration::from_secs(30)).await;
    c.reaped = true;
    assert!(status.success(), "SIGTERM exit: {status:?}");

    // A graceful stop frees the file just the same.
    let mut d = spawn_serve(serve_with_data(&db)).await;
    wait_http_up(d.http_addr).await;
    let (st, list) = common::http_json(d.http_addr, "GET", path, None).await;
    assert_eq!(st, 200, "{list}");
    assert_eq!(list["total"], 1, "a graceful stop frees the file: {list}");
    let status = signal_and_wait(&mut d.child, libc::SIGINT, Duration::from_secs(30)).await;
    d.reaped = true;
    assert!(status.success(), "SIGINT exit: {status:?}");
}

/// Run a `serve` that must be refused because `spelled` reaches a held
/// data file, and assert 0033's named refusal: non-zero, never ready.
async fn assert_refused_on(spelled: &std::path::Path) {
    // Each refused rival sees its own TMPDIR: the identity guard's root is
    // fixed, so a different temp directory never selects another guard.
    let tmp = tempfile::TempDir::with_prefix("swarmail-binary-rival-tmp-").unwrap();
    let mut second = serve_with_data(spelled);
    second
        .env("TMPDIR", tmp.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let out = tokio::time::timeout(common::HELPER_DEADLINE, second.output())
        .await
        .expect("a refused second server exits instead of serving")
        .expect("spawn the second server");
    assert!(
        !out.status.success(),
        "{spelled:?} must exit non-zero: {out:?}"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    let refusal = format!(
        "the data file is already in use by another server: {}",
        spelled.display()
    );
    assert!(stderr.contains(&refusal), "{stderr}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.lines().all(|line| parse_addrs(line).is_none()),
        "the refused server announced readiness: {stdout}"
    );
}

/// Accept one mail over real SMTP: the 250 is the claim the restart checks.
async fn deliver(serve: &Serve, inbox: &str, subject: &str) {
    let reply = common::smtp_send(
        serve.smtp_addr,
        Some(inbox),
        "a@x.io",
        "b@x.io",
        subject,
        "the body",
    )
    .await
    .expect("smtp send");
    assert!(reply.starts_with("250"), "{reply}");
}

/// The subjects an inbox serves, newest first.
async fn subjects(serve: &Serve, inbox: &str) -> Vec<String> {
    let path = format!("/api/v1/inboxes/{inbox}/messages");
    let (st, list) = common::http_json(serve.http_addr, "GET", &path, None).await;
    assert_eq!(st, 200, "{list}");
    list["emails"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["subject"].as_str().unwrap().to_string())
        .collect()
}

/// One server per data *file*, not per name (decision 0047 §1, issue
/// #129; red before: the alias server started, and the two servers' mail
/// broke the cap and lost acknowledged rows). For a symlink, a hard link
/// and a hard link in another directory (macOS fix-up: the identity guard
/// lives under a fixed root, never beside the file or under the rival's
/// own TMPDIR), in both orders, a second server reaching the held file exits
/// with the named refusal and never announces readiness; the owner keeps
/// serving. Under a cap of 2 every 250 the owners gave is accounted for
/// across a SIGKILL and graceful restarts: the newest two survive and the
/// oldest were evicted, exactly as one writer would leave them. A killed
/// owner is reopened by the name it used — 0047 §1's residual: owning the
/// inode does not move a crashed owner's `-wal` to another hard link's
/// basename — and names change hands only across graceful stops.
#[tokio::test]
async fn aliases_of_a_held_data_file_refuse_and_the_owners_mail_survives() {
    let dir = tempfile::TempDir::with_prefix("swarmail-binary-alias-").unwrap();
    let db = dir.path().join("mail.db");
    let soft = dir.path().join("soft.db");
    let hard = dir.path().join("hard.db");
    let elsewhere = tempfile::TempDir::with_prefix("swarmail-binary-alias-far-").unwrap();
    let far = elsewhere.path().join("far.db");
    let capped = |spelled: &std::path::Path| {
        let mut cmd = serve_with_data(spelled);
        cmd.args(["--max-per-inbox", "2"]);
        cmd
    };
    // The file must exist before a hard link can name it.
    let mut first = spawn_serve(capped(&db)).await;
    wait_http_up(first.http_addr).await;
    std::os::unix::fs::symlink(&db, &soft).unwrap();
    std::fs::hard_link(&db, &hard).unwrap();
    std::fs::hard_link(&db, &far).unwrap();

    let mut sent = 0;
    for alias in [&soft, &hard, &far] {
        // The real name owns; the alias is refused.
        assert_refused_on(alias).await;
        sent += 1;
        deliver(&first, "aliased", &format!("m{sent}")).await;
        let status = signal_and_wait(&mut first.child, libc::SIGINT, Duration::from_secs(30)).await;
        first.reaped = true;
        assert!(status.success(), "SIGINT exit: {status:?}");

        // The alias owns; the real name is refused.
        let mut owner = spawn_serve(capped(alias)).await;
        wait_http_up(owner.http_addr).await;
        assert_refused_on(&db).await;
        let (st, body) = common::http_json(owner.http_addr, "GET", "/healthz", None).await;
        assert_eq!(st, 200, "the owner keeps serving: {body}");
        sent += 1;
        deliver(&owner, "aliased", &format!("m{sent}")).await;

        // A SIGKILLed and reaped owner frees the file at once, by its name.
        tokio::time::timeout(common::HELPER_DEADLINE, owner.child.kill())
            .await
            .expect("the killed owner is reaped")
            .expect("kill the owner");
        owner.reaped = true;
        let mut owner = spawn_serve(capped(alias)).await;
        wait_http_up(owner.http_addr).await;
        assert_refused_on(&db).await;
        assert_eq!(
            subjects(&owner, "aliased").await[0],
            format!("m{sent}"),
            "the killed owner's 250 survives"
        );
        let status =
            signal_and_wait(&mut owner.child, libc::SIGTERM, Duration::from_secs(30)).await;
        owner.reaped = true;
        assert!(status.success(), "SIGTERM exit: {status:?}");

        first = spawn_serve(capped(&db)).await;
        wait_http_up(first.http_addr).await;
        assert_eq!(
            subjects(&first, "aliased").await,
            vec![format!("m{sent}"), format!("m{}", sent - 1)],
            "the newest two of {sent} acknowledged mails, oldest evicted"
        );
    }
    let status = signal_and_wait(&mut first.child, libc::SIGINT, Duration::from_secs(30)).await;
    first.reaped = true;
    assert!(status.success(), "SIGINT exit: {status:?}");
}

/// A data file `serve` creates is no wider than SQLite's own 0644 (decision
/// 0047 §1): run under umask 000, where the plain-open default would be
/// 0666, it is 0644 exactly — and SQLite's companions follow it.
#[tokio::test]
async fn a_created_data_file_is_no_wider_than_0644() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::TempDir::with_prefix("swarmail-binary-mode-").unwrap();
    let db = dir.path().join("mail.db");
    let mut cmd = Command::new("sh");
    cmd.args(["-c", "umask 000 && exec \"$0\" \"$@\"", binary()])
        .args(serve_with_data(&db).as_std().get_args());
    let mut serve = spawn_serve(cmd).await;
    wait_http_up(serve.http_addr).await;
    let mode = std::fs::metadata(&db).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o644, "{mode:o}");
    let status = signal_and_wait(&mut serve.child, libc::SIGINT, Duration::from_secs(30)).await;
    serve.reaped = true;
    assert!(status.success(), "SIGINT exit: {status:?}");
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
        format!(
            "POST /api/v1/inboxes/stall/seed HTTP/1.1\r\nHost: {}\r\n\
             Expect: 100-continue\r\nContent-Type: application/json\r\n\
             Content-Length: 10\r\n\r\n",
            serve.http_addr
        )
        .as_bytes(),
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
    // The URL carries credentials: the line names the target without them
    // (issue #98, decision 0041 §3).
    let (st, resp) = common::http_json(
        serve.http_addr,
        "PUT",
        "/api/v1/webhooks",
        Some(r#"[{"url": "http://user:hunter2@127.0.0.1:1/hook"}]"#),
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
    assert!(
        !line.contains("hunter2") && !line.contains("user@"),
        "the warn carries no URL credentials: {line:?}"
    );

    let status = signal_and_wait(&mut serve.child, libc::SIGINT, Duration::from_secs(30)).await;
    serve.reaped = true;
    assert!(status.success(), "SIGINT exit: {status:?}");
}

// ---------- the webhook boundary (decision 0041) ----------

/// A sink that accepts one delivery: it reads the request whole, answers
/// `reply` (or, with `None`, never answers and holds the socket open), and
/// hands the request text over the channel as soon as it is read.
async fn one_request_sink(
    reply: Option<&'static str>,
) -> (SocketAddr, tokio::sync::oneshot::Receiver<String>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        let request = loop {
            let n = stream.read(&mut chunk).await.unwrap_or(0);
            buf.extend_from_slice(&chunk[..n]);
            let text = String::from_utf8_lossy(&buf).into_owned();
            if n == 0 || request_complete(&text) {
                break text;
            }
        };
        let _ = tx.send(request);
        match reply {
            Some(reply) => {
                let _ = stream.write_all(reply.as_bytes()).await;
            }
            // Held open until the test process ends: the delivery stays in
            // flight for as long as the server lives.
            None => std::mem::forget(stream),
        }
    });
    (addr, rx)
}

/// A request is complete once its head is in and its Content-Length body
/// has followed it.
fn request_complete(text: &str) -> bool {
    text.split_once("\r\n\r\n").is_some_and(|(head, body)| {
        let len = head
            .lines()
            .find_map(|line| {
                let line = line.to_ascii_lowercase();
                line.strip_prefix("content-length:")?
                    .trim()
                    .parse::<usize>()
                    .ok()
            })
            .unwrap_or(0);
        body.len() >= len
    })
}

/// A recording proxy: every connection is counted and answered 502, so a
/// client that wrongly routes through it sees a real reply (and retries)
/// rather than a hang. The count is the evidence.
async fn recording_proxy() -> (SocketAddr, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let hits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = std::sync::Arc::clone(&hits);
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            tokio::spawn(async move {
                let mut chunk = [0u8; 4096];
                let _ = stream.read(&mut chunk).await;
                let _ = stream
                    .write_all(
                        b"HTTP/1.1 502 Proxy\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    )
                    .await;
            });
        }
    });
    (addr, hits)
}

/// A self-signed CA's PEM: a `ca_pem` puts the delivery on the per-root
/// client, the other client path.
fn a_ca_pem() -> String {
    let key = rcgen::KeyPair::generate().unwrap();
    let mut params = rcgen::CertificateParams::new(Vec::<String>::new()).unwrap();
    params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    params.key_usages = vec![rcgen::KeyUsagePurpose::KeyCertSign];
    params.self_signed(&key).unwrap().pem()
}

/// Issue #97: with every proxy variable pointed at a recording proxy — in
/// the child's environment only, both spellings, no bypass list — the
/// delivery still reaches its target directly, secret and body included,
/// and the proxy sees nothing, before or after the delivery. A fresh child
/// per client path (`ca_pem` absent or present), so neither cached client
/// was built under another test's environment.
async fn a_delivery_dials_its_target_and_never_the_proxy(ca_pem: Option<String>) {
    let (proxy, proxy_hits) = recording_proxy().await;
    let (target, request) =
        one_request_sink(Some("HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n")).await;
    let mut cmd = serve_command();
    let proxy_url = format!("http://{proxy}");
    for var in [
        "HTTP_PROXY",
        "http_proxy",
        "HTTPS_PROXY",
        "https_proxy",
        "ALL_PROXY",
        "all_proxy",
    ] {
        cmd.env(var, &proxy_url);
    }
    cmd.env_remove("NO_PROXY").env_remove("no_proxy");
    let mut serve = spawn_serve(cmd).await;
    wait_http_up(serve.http_addr).await;

    // An absent `ca_pem` is omitted, never `null` — a present null is
    // refused (decision 0048 §2).
    let mut target_json = serde_json::json!({
        "url": format!("http://{target}/direct"),
        "secret": "s3cret",
    });
    if let Some(pem) = ca_pem {
        target_json["ca_pem"] = pem.into();
    }
    let targets = serde_json::json!([target_json]).to_string();
    let (st, resp) =
        common::http_json(serve.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200, "{resp}");
    common::smtp_send(serve.smtp_addr, None, "f@x.io", "r@x.io", "direct", "x")
        .await
        .expect("the mail is accepted");

    let seen = tokio::time::timeout(Duration::from_secs(15), request)
        .await
        .unwrap_or_else(|_| {
            panic!(
                "the target never received the delivery; the proxy saw {} connection(s)",
                proxy_hits.load(std::sync::atomic::Ordering::SeqCst)
            )
        })
        .expect("the sink read a request");
    assert!(seen.starts_with("POST /direct HTTP/1.1"), "{seen}");
    assert!(
        seen.to_ascii_lowercase()
            .contains("x-swarmail-secret: s3cret"),
        "{seen}"
    );
    assert!(seen.contains("\"event\":\"received\""), "{seen}");

    let status = signal_and_wait(&mut serve.child, libc::SIGTERM, Duration::from_secs(30)).await;
    serve.reaped = true;
    assert!(status.success(), "SIGTERM exit: {status:?}");
    assert_eq!(
        proxy_hits.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "the proxy saw nothing"
    );
}

#[tokio::test]
async fn the_default_client_ignores_proxy_environment() {
    a_delivery_dials_its_target_and_never_the_proxy(None).await;
}

#[tokio::test]
async fn the_ca_pem_client_ignores_proxy_environment() {
    a_delivery_dials_its_target_and_never_the_proxy(Some(a_ca_pem())).await;
}

/// Issue #98's shutdown count, visible from the shipped binary: with one
/// delivery in flight against a sink that read it and never answers, a
/// SIGTERM ends the process cleanly and its stdout — at the default level,
/// no `RUST_LOG` — carries exactly one `webhook dispatcher stopped` line
/// with all five totals: the in-flight delivery aborted, a refused one
/// exhausted, nothing else. Ordered by the sink's read, the exhaustion
/// counter and the process's EOF, never a sleep.
///
/// Issue #135 (decision 0048 §6) rides the same run. The final event's
/// *message* is `webhook dispatcher stopped` — red before: the constant was
/// a field, `DISPATCHER_STOPPED="webhook dispatcher stopped"` — and the
/// exhausted delivery's warn names its transport cause past `error
/// sending request`, with neither the URL's credentials nor the secret
/// anywhere on stdout.
#[tokio::test]
async fn sigterm_with_a_delivery_in_flight_prints_the_final_counts() {
    let (stalled, request) = one_request_sink(None).await;
    // Bind and drop: a port nothing listens on, so every dial is refused.
    let refused = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    let mut cmd = serve_command();
    cmd.env_remove("RUST_LOG");
    let mut serve = spawn_serve(cmd).await;
    wait_http_up(serve.http_addr).await;

    let targets = format!(
        r#"[{{"url": "http://user:hunter2@{stalled}/hook"}},
            {{"url": "http://user:hunter3@{refused}/hook", "secret": "marker-secret"}}]"#
    );
    let (st, resp) =
        common::http_json(serve.http_addr, "PUT", "/api/v1/webhooks", Some(&targets)).await;
    assert_eq!(st, 200, "{resp}");
    common::smtp_send(serve.smtp_addr, None, "f@x.io", "r@x.io", "in flight", "x")
        .await
        .expect("the mail is accepted");
    tokio::time::timeout(Duration::from_secs(10), request)
        .await
        .expect("the delivery reached the stalled sink")
        .expect("the sink read a request");
    common::wait_for_metric_from(serve.http_addr, "swarmail_webhook_exhausted_total", 0, 1).await;

    unsafe {
        libc::kill(serve.child.id().expect("a live pid") as i32, libc::SIGTERM);
    }
    let mut rest = String::new();
    let mut stdout = serve.stdout.take().expect("piped stdout");
    tokio::time::timeout(Duration::from_secs(30), stdout.read_to_string(&mut rest))
        .await
        .expect("the server's stdout reaches EOF")
        .expect("stdout reads");
    let status = tokio::time::timeout(Duration::from_secs(30), serve.child.wait())
        .await
        .expect("the server exits")
        .expect("the server is reaped");
    serve.reaped = true;
    assert!(status.success(), "SIGTERM exit: {status:?}");

    let finals: Vec<&str> = rest
        .lines()
        .filter(|l| l.contains("webhook dispatcher stopped"))
        .collect();
    assert_eq!(finals.len(), 1, "exactly one final line: {rest}");
    // The message position: the fmt layer writes the `message` field bare
    // and every other field as `name=value`, in their order — the sentence
    // follows the target directly, the five totals after it.
    let (_, event) = finals[0]
        .split_once("swarmail::webhook: ")
        .expect("the event names its target");
    assert!(
        event.starts_with("webhook dispatcher stopped delivered="),
        "the sentence is the message: {:?}",
        finals[0]
    );
    assert!(!finals[0].contains("DISPATCHER_STOPPED"), "{:?}", finals[0]);
    assert!(
        !finals[0].contains('"'),
        "no quoted field value: {:?}",
        finals[0]
    );
    let fields: Vec<&str> = finals[0].split_whitespace().collect();
    for total in [
        "delivered=0",
        "dropped=0",
        "lagged=0",
        "exhausted=1",
        "aborted=1",
    ] {
        assert!(fields.contains(&total), "{total} in {:?}", finals[0]);
    }

    let dropped: Vec<&str> = rest
        .lines()
        .filter(|l| l.contains("webhook dropped after retries"))
        .collect();
    assert_eq!(dropped.len(), 1, "one exhaustion line: {rest}");
    let (_, cause) = dropped[0]
        .split_once("error=error sending request")
        .expect("the display prefix is kept");
    assert!(
        cause.starts_with(": ") && cause.len() > ": ".len() + 8,
        "the transport cause follows the prefix: {:?}",
        dropped[0]
    );
    for secret in ["hunter2", "hunter3", "marker-secret"] {
        assert!(!rest.contains(secret), "no {secret} on stdout: {rest}");
    }
}

// ---------- the fallible EOF helper (issue #102, 0018 §2) ----------

/// A peer that accepts and closes with nothing written back is "no answer":
/// the helper maps it to `(0, "")`, exactly the client-visible half of 0018
/// §2's open outcome. On a real socket, not a mock (0004).
#[tokio::test]
async fn the_eof_helper_reads_an_accept_and_close_as_no_answer() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        // One read of the request head, then close with nothing written.
        let mut buf = [0u8; 4096];
        let _ = stream.read(&mut buf).await;
        drop(stream);
    });
    let (status, body) =
        common::http_send_text_allow_eof(addr, "POST", "/api/v1/inboxes/eof/seed", Some("{}"))
            .await;
    assert_eq!(
        (status, body.as_str()),
        (0, ""),
        "an accept-and-close is no answer"
    );
}

/// A 503 with an empty body is a real answer: its status rides through
/// untouched. The old helper rewrote it to 0 because the body was empty —
/// the exact corruption 0018 §2's outcome table forbids.
#[tokio::test]
async fn the_eof_helper_never_rewrites_a_real_status() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 4096];
        let _ = stream.read(&mut buf).await;
        stream
            .write_all(
                b"HTTP/1.1 503 Storage Refused\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
            )
            .await
            .unwrap();
    });
    let (status, body) =
        common::http_send_text_allow_eof(addr, "POST", "/api/v1/inboxes/eof/seed", Some("{}"))
            .await;
    assert_eq!((status, body.as_str()), (503, ""), "the status stays 503");
}

// ---------- the stdio bridge against the real server (decision 0039) ----------

/// A `swarmail mcp` child bridged to a real `serve`, fed and read line by
/// line. stderr is discarded so its pipe can never fill; the child dies
/// with the test.
struct McpChild {
    child: Child,
    stdin: Option<tokio::process::ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl McpChild {
    fn spawn(serve: &Serve) -> McpChild {
        let mut cmd = Command::new(binary());
        cmd.args(["mcp", "--url", &format!("http://{}", serve.http_addr)]);
        McpChild::spawn_command(cmd)
    }

    /// Any command that ends in `swarmail mcp`, wired the same way.
    fn spawn_command(mut cmd: Command) -> McpChild {
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .expect("spawn swarmail mcp");
        let stdin = child.stdin.take();
        let stdout = BufReader::new(child.stdout.take().expect("piped stdout"));
        McpChild {
            child,
            stdin,
            stdout,
        }
    }

    async fn send(&mut self, bytes: &[u8]) {
        let stdin = self.stdin.as_mut().expect("stdin is open");
        stdin.write_all(bytes).await.expect("feed the bridge");
        stdin.flush().await.expect("feed the bridge");
    }

    /// The next reply line, bounded by the helper deadline.
    async fn reply(&mut self) -> serde_json::Value {
        let mut line = String::new();
        tokio::time::timeout(common::HELPER_DEADLINE, self.stdout.read_line(&mut line))
            .await
            .expect("the bridge never replied")
            .expect("read the bridge's stdout");
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("{line:?} is not JSON: {e}"))
    }

    /// Close stdin; every remaining stdout line (each must parse on its
    /// own) and the exit status.
    async fn finish(mut self) -> (std::process::ExitStatus, Vec<serde_json::Value>) {
        drop(self.stdin.take());
        let mut rest = String::new();
        tokio::time::timeout(
            common::HELPER_DEADLINE,
            self.stdout.read_to_string(&mut rest),
        )
        .await
        .expect("the bridge never closed stdout")
        .expect("read the bridge's stdout");
        let status = tokio::time::timeout(common::HELPER_DEADLINE, self.child.wait())
            .await
            .expect("the bridge never exited")
            .expect("mcp child");
        let replies = rest
            .lines()
            .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("{l:?} is not JSON: {e}")))
            .collect();
        (status, replies)
    }
}

/// The text payload of a tool result.
fn tool_text(reply: &serde_json::Value) -> serde_json::Value {
    let text = reply["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("no tool text in {reply}"));
    serde_json::from_str(text).expect("the tool text is JSON")
}

/// Batches through the shipped bridge answer exactly as `POST /mcp` does
/// (issue #72): two pings, a mixed batch, a batch with an unknown method,
/// the empty and over-cap refusals — and nothing for notification batches.
/// The old bridge wrote nothing for any array.
#[tokio::test]
async fn mcp_forwards_batches_as_the_http_endpoint_answers_them() {
    let serve = spawn_serve(serve_command()).await;
    wait_http_up(serve.http_addr).await;
    let note = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    let batches = [
        r#"[{"jsonrpc":"2.0","id":1,"method":"ping"},{"jsonrpc":"2.0","id":2,"method":"ping"}]"#
            .to_string(),
        format!(r#"[{{"jsonrpc":"2.0","id":3,"method":"ping"}},{note}]"#),
        r#"[{"jsonrpc":"2.0","id":4,"method":"nope"}]"#.to_string(),
        format!("[{note},{note}]"),
        "[]".to_string(),
        format!("[{}]", vec![note; 17].join(",")),
    ];
    let mut expected = Vec::new();
    for batch in &batches {
        let (_, body) = common::http_send_text(serve.http_addr, "POST", "/mcp", Some(batch)).await;
        if !body.trim().is_empty() {
            expected.push(serde_json::from_str::<serde_json::Value>(&body).unwrap());
        }
    }
    assert_eq!(expected.len(), 5, "HTTP answers all but the notifications");

    let mut mcp = McpChild::spawn(&serve);
    mcp.send(format!("{}\n", batches.join("\n")).as_bytes())
        .await;
    let (status, replies) = mcp.finish().await;
    assert!(status.success(), "mcp exit: {status:?}");
    let canonical = |v: &[serde_json::Value]| {
        let mut lines: Vec<String> = v.iter().map(|r| r.to_string()).collect();
        lines.sort();
        lines
    };
    assert_eq!(
        canonical(&replies[..]),
        canonical(&expected[..]),
        "stdio answers as HTTP does"
    );
}

/// A wait and the seed that satisfies it, sent back to back with a ping,
/// return the mail (issue #90). The old bridge read the seed only after the
/// wait had timed out. Correct in either arrival order: the wait subscribes
/// before it scans (decision 0002).
#[tokio::test]
async fn mcp_answers_a_wait_with_the_seed_sent_beside_it() {
    let serve = spawn_serve(serve_command()).await;
    wait_http_up(serve.http_addr).await;
    let mut mcp = McpChild::spawn(&serve);
    mcp.send(
        concat!(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"swarmail_wait_for_email","arguments":{"inbox":"beside","timeout_ms":4000}}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"swarmail_seed_email","arguments":{"inbox":"beside","to":"a@x.io","subject":"the seed"}}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":3,"method":"ping"}"#,
            "\n",
        )
        .as_bytes(),
    )
    .await;
    let (status, replies) = mcp.finish().await;
    assert!(status.success(), "mcp exit: {status:?}");
    assert_eq!(replies.len(), 3, "{replies:?}");
    let wait = replies
        .iter()
        .find(|r| r["id"] == 1)
        .expect("the wait's reply");
    let result = tool_text(wait);
    assert!(
        result.get("timed_out").is_none(),
        "a false timeout: {result}"
    );
    assert_eq!(result["emails"][0]["subject"], "the seed", "{result}");
}

/// A ping sent behind a registered five-second wait is answered first
/// (issue #90): the wait is observed parked on its own `/metrics` row, the
/// ping's reply must arrive before the wait's, and the seed then releases
/// the wait. The old bridge answered the wait (timed out) first.
#[tokio::test]
async fn mcp_answers_a_ping_behind_a_parked_wait_first() {
    let serve = spawn_serve(serve_command()).await;
    wait_http_up(serve.http_addr).await;
    let mut mcp = McpChild::spawn(&serve);
    mcp.send(
        concat!(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"swarmail_wait_for_email","arguments":{"inbox":"behind","timeout_ms":5000}}}"#,
            "\n",
        )
        .as_bytes(),
    )
    .await;
    common::wait_for_metric_row(
        serve.http_addr,
        r#"swarmail_awaits_active{inbox="behind"}"#,
        1,
    )
    .await;
    mcp.send(b"{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}\n")
        .await;
    let first = mcp.reply().await;
    assert_eq!(first["id"], 2, "the ping waited behind the wait: {first}");

    mcp.send(
        concat!(
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"swarmail_seed_email","arguments":{"inbox":"behind","to":"a@x.io"}}}"#,
            "\n",
        )
        .as_bytes(),
    )
    .await;
    let (status, replies) = mcp.finish().await;
    assert!(status.success(), "mcp exit: {status:?}");
    assert_eq!(replies.len(), 2, "{replies:?}");
    let wait = replies
        .iter()
        .find(|r| r["id"] == 1)
        .expect("the wait's reply");
    let result = tool_text(wait);
    assert!(result.get("timed_out").is_none(), "{result}");
    assert_eq!(
        result["emails"].as_array().map(Vec::len),
        Some(1),
        "{result}"
    );
}

/// A non-UTF-8 line and a malformed one each get a null-id -32700 and the
/// bridge keeps going (issue #90): the ping behind them is answered and the
/// process exits 0 at EOF. The old bridge died on the first byte 0xFF.
#[tokio::test]
async fn mcp_answers_bad_lines_and_keeps_serving() {
    let serve = spawn_serve(serve_command()).await;
    wait_http_up(serve.http_addr).await;
    let mut mcp = McpChild::spawn(&serve);
    mcp.send(b"\xff\xfe\n{nope\n{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"ping\"}\n")
        .await;
    let (status, replies) = mcp.finish().await;
    assert!(status.success(), "mcp exit: {status:?}");
    assert_eq!(replies.len(), 3, "{replies:?}");
    let parse_errors = replies
        .iter()
        .filter(|r| r["error"]["code"] == -32700 && r["id"].is_null())
        .count();
    assert_eq!(parse_errors, 2, "{replies:?}");
    let ping = replies
        .iter()
        .find(|r| r["id"] == 9)
        .expect("the ping's reply");
    assert_eq!(ping["result"], serde_json::json!({}), "{ping}");
}

/// An id-less object without a string method is no notification on the
/// bridge either (decision 0047 §3, issue #132; red before: the bridge
/// wrote nothing for it): `{"foo":"boo"}` gets exactly one `-32600` line
/// with a null id, and the ping behind it is answered.
#[tokio::test]
async fn mcp_answers_a_malformed_object_with_one_invalid_request_line() {
    let serve = spawn_serve(serve_command()).await;
    wait_http_up(serve.http_addr).await;
    let mut mcp = McpChild::spawn(&serve);
    mcp.send(b"{\"foo\":\"boo\"}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}\n")
        .await;
    let (status, replies) = mcp.finish().await;
    assert!(status.success(), "mcp exit: {status:?}");
    assert_eq!(replies.len(), 2, "{replies:?}");
    let invalid = replies
        .iter()
        .find(|r| r["id"].is_null())
        .expect("the malformed object's reply");
    assert_eq!(invalid["error"]["code"], -32600, "{invalid}");
    assert_eq!(invalid["error"]["message"], "missing method", "{invalid}");
    let ping = replies
        .iter()
        .find(|r| r["id"] == 2)
        .expect("the ping's reply");
    assert_eq!(ping["result"], serde_json::json!({}), "{ping}");
}

/// A raw `POST /mcp` of `body` and the JSON the server answered — a
/// refused body may be cut off mid-write, which is not a failure here.
async fn post_mcp_raw(addr: SocketAddr, body: &[u8]) -> serde_json::Value {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let mut request = format!(
        "POST /mcp HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    request.extend_from_slice(body);
    let _ = stream.write_all(&request).await;
    let mut reply = Vec::new();
    let _ = stream.read_to_end(&mut reply).await;
    let split = reply
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("a response head");
    serde_json::from_slice(&reply[split + 4..]).expect("a JSON body")
}

/// An oversized request is refused by the bridge itself, with its own ids
/// (decision 0047 §4, issue #133; red before: the 3 MiB line got the
/// server's null-id 413 and the 40 MiB one a `-32000` broken pipe). A 3 MiB
/// and a 40 MiB seed with id 7 each get exactly one `-32600` line with id 7
/// in the server's own 413 words — read from a real over-limit POST — an
/// oversized two-request batch gets one two-element array in member order,
/// an oversized notification gets nothing, none of them seeds a mail, and
/// the ping behind them is answered. Local refusals are written as their
/// lines are read, so the order is the input's.
#[tokio::test]
async fn oversized_bridge_requests_keep_ids_without_effects() {
    let serve = spawn_serve(serve_command()).await;
    wait_http_up(serve.http_addr).await;
    let limit = 2_097_152;
    let over_http = post_mcp_raw(serve.http_addr, &vec![b' '; limit + 1]).await;
    let words = over_http["error"]["message"].clone();
    assert!(
        words
            .as_str()
            .unwrap()
            .contains("(the limit is 2097152 bytes)"),
        "{over_http}"
    );

    let seed = |id: &str, size: usize| {
        format!(
            r#"{{"jsonrpc":"2.0",{id}"method":"tools/call","params":{{"name":"swarmail_seed_email","arguments":{{"inbox":"oversized","to":"a@x.io","text":"{}"}}}}}}"#,
            "x".repeat(size)
        )
    };
    let mib = 1024 * 1024;
    let lines = [
        seed(r#""id":7,"#, 3 * mib),
        seed(r#""id":7,"#, 40 * mib),
        format!(
            r#"[{},{{"jsonrpc":"2.0","id":8,"method":"ping"}}]"#,
            seed(r#""id":7,"#, 3 * mib)
        ),
        seed("", 3 * mib),
        r#"{"jsonrpc":"2.0","id":9,"method":"ping"}"#.to_string(),
    ];
    let mut mcp = McpChild::spawn(&serve);
    for line in &lines {
        mcp.send(format!("{line}\n").as_bytes()).await;
    }
    let (status, replies) = mcp.finish().await;
    assert!(status.success(), "mcp exit: {status:?}");
    let refused = |id: serde_json::Value| serde_json::json!({"jsonrpc":"2.0","id":id,"error":{"code":-32600,"message":words}});
    assert_eq!(
        replies,
        [
            refused(7.into()),
            refused(7.into()),
            serde_json::json!([refused(7.into()), refused(8.into())]),
            serde_json::json!({"jsonrpc":"2.0","id":9,"result":{}}),
        ]
    );
    let (st, count) = common::http_json(
        serve.http_addr,
        "GET",
        "/api/v1/inboxes/oversized/count",
        None,
    )
    .await;
    assert_eq!(st, 200, "{count}");
    assert_eq!(count["count"], 0, "no oversized line reached the server");
}

/// `serve` bound on 127.0.0.1:80.
const SERVE_ON_EIGHTY: [&str; 7] = [
    "serve",
    "--smtp-listen",
    "127.0.0.1:0",
    "--pop3-listen",
    "127.0.0.1:0",
    "--http-listen",
    "127.0.0.1:80",
];

/// A URL with no port dials HTTP's 80 (decision 0047 §4, issue #133; red
/// before: "invalid socket address"): a real bridge given
/// `--url http://127.0.0.1` answers a ping from a real server bound on
/// 127.0.0.1:80 — never an ephemeral-port stand-in. Ignored by default
/// because binding 127.0.0.1:80 is privileged (and unprivileged user
/// namespaces are refused on hosts that restrict them); CI runs it
/// explicitly, after lowering `net.ipv4.ip_unprivileged_port_start` on
/// Linux and as root on macOS (which frees 80 to users only on the wildcard
/// address). Where 80 cannot be bound it fails
/// loudly at the server's startup.
#[tokio::test]
#[ignore = "needs a host that lets this user bind 127.0.0.1:80; CI runs it explicitly"]
async fn portless_bridge_pings_port_eighty() {
    let mut serve = Command::new(binary());
    serve.args(SERVE_ON_EIGHTY);
    let serve = spawn_serve(serve).await;
    assert_eq!(serve.http_addr.port(), 80);
    wait_http_up(serve.http_addr).await;
    let mut bridge = Command::new(binary());
    bridge.args(["mcp", "--url", "http://127.0.0.1"]);
    let mut mcp = McpChild::spawn_command(bridge);
    mcp.send(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n")
        .await;
    let (status, replies) = mcp.finish().await;
    assert!(status.success(), "mcp exit: {status:?}");
    assert_eq!(
        replies,
        [serde_json::json!({"jsonrpc":"2.0","id":1,"result":{}})]
    );
}
