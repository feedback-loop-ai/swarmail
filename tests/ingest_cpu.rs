//! Concurrent ingest CPU: memory mode against data-file mode (issue #333,
//! decision 0124).
//!
//! The issue measured the in-memory mode — which does strictly less work, no
//! SQLite and no fsync — at 3-5x the CPU of `--data-file` under many
//! concurrent one-connection-per-mail SMTP sessions. This file is the
//! relation's proof: real sessions, a real server, one process whose own CPU
//! clock is the measurement, so it holds exactly ONE test (`getrusage` sees
//! every thread of the binary, and a sibling test would leak into it).
//!
//! What it can and cannot show. On glibc the gap the issue measured does not
//! exist even on the unfixed tree: the cost lives in the allocator of the
//! static musl image under a CPU quota, which no CI runner builds. So this
//! test is the *relation guard* — memory mode may not cost more per mail than
//! data-file mode, within a stated margin. The deterministic detector for the
//! cause itself (more parses in flight than usable cores) is the parse-gate
//! tests in `src/store.rs`, `src/smtp.rs` and `src/api.rs`.
//!
//! Load, bounded by construction: one client thread, `SESSIONS` concurrent
//! connections, `MAILS` mails of ~30 KB per run, a two-worker server runtime,
//! six runs, and a watchdog on each run's client side (`RUN_BOUND`: a wedged
//! peer fails the test with a named message instead of hanging the CI slot; it
//! bounds the procedure and asserts nothing about speed). Measured, not timed:
//! nothing here asserts on wall-clock speed. `getrusage` counts the client
//! thread too (formatting ~30 KB and the socket writes, the same in both
//! modes), which pulls the ratio toward 1: the margin is a looser bound on
//! server cost than it reads, and on hosts with more than `SESSIONS` usable
//! cores the gate itself is never reached here.

mod common;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

/// Mails per run (one connection each, as a notification service sends).
const MAILS: usize = 240;
/// Concurrent sessions: eight connections driven from one client thread,
/// against a two-worker server runtime — a fixed, small load that does not
/// grow with the host (the 30-job musl probe in 0124 is the nproc/4-bounded one).
const SESSIONS: usize = 8;
/// Runs per mode; alternating, so drift (thermal, neighbours) hits both.
const RUNS: usize = 3;
/// The stated margin: memory CPU per mail <= MARGIN x data-file CPU per mail.
/// The two are not close — data-file mode adds a SQLite transaction and an
/// fsync per mail — so 1.25 is a floor for a regression, not a tolerance for
/// noise; a tree that makes memory mode the dearer one fails it.
const MARGIN: f64 = 1.25;
/// Watchdog on one run's client side. Generous (a run takes seconds); it exists
/// so a stalled server fails with a message rather than waiting on the CI job's
/// own timeout.
const RUN_BOUND: std::time::Duration = std::time::Duration::from_secs(120);

/// A multipart/alternative body of roughly 30 KB: a text part and an HTML
/// part with links and a code, the shape a notification service sends. No
/// line starts with a dot, so the wire needs no dot-stuffing.
fn body(seed: usize) -> String {
    let text = format!("Hello user {seed}. Verify https://x.io/v?t={seed} code {seed}1234.\r\n")
        .repeat(260);
    let html = format!(
        "<html><body><p>Hello <b>user {seed}</b></p>\
         <a href=\"https://x.io/v?t={seed}\">verify</a><p>code {seed}1234</p></body></html>\r\n"
    )
    .repeat(150);
    format!(
        "From: noreply@x.io\r\nTo: u{seed}@example.com\r\nSubject: cpu probe {seed}\r\n\
         MIME-Version: 1.0\r\nContent-Type: multipart/alternative; boundary=\"BND\"\r\n\r\n\
         --BND\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{text}\
         --BND\r\nContent-Type: text/html; charset=utf-8\r\n\r\n{html}--BND--\r\n"
    )
}

/// One delivery on its own connection; returns the `250` line.
async fn deliver(addr: std::net::SocketAddr, seed: usize) -> String {
    let stream = TcpStream::connect(addr).await.unwrap();
    let (read, mut write) = stream.into_split();
    let mut read = BufReader::new(read);
    let mut line = String::new();
    read.read_line(&mut line).await.unwrap();
    assert!(line.starts_with("220"), "{line}");
    for command in [
        "MAIL FROM:<noreply@x.io>\r\n".to_string(),
        format!("RCPT TO:<u{seed}@example.com>\r\n"),
        "DATA\r\n".to_string(),
    ] {
        write.write_all(command.as_bytes()).await.unwrap();
        line.clear();
        read.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("250") || line.starts_with("354"), "{line}");
    }
    write.write_all(body(seed).as_bytes()).await.unwrap();
    write.write_all(b".\r\n").await.unwrap();
    line.clear();
    read.read_line(&mut line).await.unwrap();
    let accepted = line.clone();
    write.write_all(b"QUIT\r\n").await.unwrap();
    // Drain to EOF so the server's session ends before the next is counted.
    let mut rest = Vec::new();
    let _ = read.read_to_end(&mut rest).await;
    accepted
}

/// User + system CPU of this whole process, in seconds.
fn process_cpu_seconds() -> f64 {
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
    assert_eq!(rc, 0, "getrusage failed");
    let seconds = |t: libc::timeval| t.tv_sec as f64 + t.tv_usec as f64 / 1e6;
    seconds(usage.ru_utime) + seconds(usage.ru_stime)
}

/// One run: a fresh server on its own runtime, `MAILS` mails over `SESSIONS`
/// concurrent connections; returns CPU milliseconds per accepted mail.
fn run(server_rt: &tokio::runtime::Runtime, data_file: Option<std::path::PathBuf>) -> f64 {
    let server = server_rt.block_on(common::start_with(move |mut c| {
        c.data_file = data_file;
        c
    }));
    let addr = server.smtp_addr;
    let client = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let before = process_cpu_seconds();
    let finished = client.block_on(async {
        tokio::time::timeout(RUN_BOUND, async {
            let mut sessions = tokio::task::JoinSet::new();
            for worker in 0..SESSIONS {
                sessions.spawn(async move {
                    let mut accepted = 0;
                    let mut seed = worker;
                    while seed < MAILS {
                        let reply = deliver(addr, seed).await;
                        assert!(reply.starts_with("250"), "{reply}");
                        accepted += 1;
                        seed += SESSIONS;
                    }
                    accepted
                });
            }
            let mut total = 0;
            while let Some(done) = sessions.join_next().await {
                total += done.unwrap();
            }
            assert_eq!(total, MAILS);
        })
        .await
    });
    finished.unwrap_or_else(|_| panic!("{MAILS} mails did not finish within {RUN_BOUND:?}"));
    let spent = process_cpu_seconds() - before;
    // Every 250 is a queryable mail (0001); with a data file also committed.
    assert_eq!(server.store.emails_inserted(), MAILS as u64);
    server_rt.block_on(server.stop());
    spent * 1000.0 / MAILS as f64
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    values[values.len() / 2]
}

#[test]
fn concurrent_memory_ingest_costs_no_more_cpu_per_mail_than_data_file_ingest() {
    let server_rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let (mut memory, mut file) = (Vec::new(), Vec::new());
    println!(
        "nproc={} sessions={SESSIONS} mails/run={MAILS} bytes/mail={}",
        std::thread::available_parallelism().map_or(1, usize::from),
        body(1).len()
    );
    for round in 0..RUNS {
        memory.push(run(&server_rt, None));
        file.push(run(
            &server_rt,
            Some(dir.path().join(format!("run{round}.db"))),
        ));
        println!(
            "round {round}: memory {:.3} ms/mail, data-file {:.3} ms/mail",
            memory[round], file[round]
        );
    }
    let (memory, file) = (median(memory), median(file));
    println!(
        "median: memory {memory:.3} ms/mail, data-file {file:.3} ms/mail, ratio {:.2} (limit {MARGIN})",
        memory / file
    );
    assert!(
        memory <= MARGIN * file,
        "in-memory ingest cost {memory:.3} ms/mail against {file:.3} for the data file: \
         more than {MARGIN}x (issue #333)"
    );
}
