//! Cold diagnostics and subprocess fixtures stay in the integration harness
//! (0020); no production seam or denominator change is involved.
use super::*;
use std::io::{Read, Write};

fn fixture(mode: &str) -> Command {
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args([
        "--exact",
        "helper_reliability::fixture_child",
        "--ignored",
        "--nocapture",
    ])
    .env("SWARMAIL_HELPER_FIXTURE", mode)
    .kill_on_drop(true);
    cmd
}

/// This is a child process, never a simulated pending future in the parent.
/// No shell or descendant is involved. The pipe/socket events establish
/// readiness; parking is deliberate nontermination, not synchronization.
#[test]
#[ignore = "subprocess fixture, invoked only by the reliability tests"]
fn fixture_child() {
    match std::env::var("SWARMAIL_HELPER_FIXTURE").unwrap().as_str() {
        "stderr" => {
            println!("stdout-source-marker");
            // More than a pipe's capacity: startup must drain concurrently.
            std::io::stderr()
                .write_all(&vec![b'x'; 128 * 1024])
                .unwrap();
            eprintln!("fixture source: startup refused: diagnostic-marker");
            std::process::exit(23);
        }
        "bridge-open" | "bridge-closed" => {
            println!("{{\"fixture_ready\":true}}");
            std::io::stdout().flush().unwrap();
            if std::env::var("SWARMAIL_HELPER_FIXTURE").unwrap() == "bridge-closed" {
                // Only this isolated fixture owns fd 1.
                assert_eq!(unsafe { libc::close(1) }, 0);
            }
            loop {
                std::thread::park();
            }
        }
        mode @ ("silent" | "partial" | "chatter") => {
            let addr = std::env::var("SWARMAIL_FIXTURE_READY").unwrap();
            let mut ready = std::net::TcpStream::connect(addr).unwrap();
            ready.write_all(b"ready").unwrap();
            if mode == "partial" {
                print!("partial-startup-marker");
                std::io::stdout().flush().unwrap();
            }
            if mode == "chatter" {
                let mut token = [0];
                while ready.read_exact(&mut token).is_ok() {
                    println!("unrelated startup chatter");
                    std::io::stdout().flush().unwrap();
                    ready.write_all(b"x").unwrap();
                }
            }
            loop {
                std::thread::park();
            }
        }
        other => panic!("unknown fixture: {other}"),
    }
}

#[tokio::test]
async fn startup_failure_keeps_stderr_source_and_exit_status() {
    let error = match try_spawn_serve(fixture("stderr"), common::HELPER_DEADLINE).await {
        Ok(mut serve) => {
            kill_serve(&mut serve).await;
            panic!("fixture unexpectedly ready");
        }
        Err(error) => error,
    };
    assert!(
        error.contains("stdout EOF before startup addresses"),
        "{error}"
    );
    assert!(error.contains("diagnostic-marker"), "{error}");
    assert!(error.contains("stdout-source-marker"), "{error}");
    assert!(error.contains("exit status: 23"), "{error}");
    assert!(error.contains("fixture source: startup refused"), "{error}");
    assert!(error.contains("cleanup: Ok(ExitStatus"), "{error}");
}

#[tokio::test]
async fn startup_deadline_covers_silence_partial_lines_and_chatter() {
    for mode in ["silent", "partial", "chatter"] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut cmd = fixture(mode);
        cmd.env(
            "SWARMAIL_FIXTURE_READY",
            listener.local_addr().unwrap().to_string(),
        );
        let startup = try_spawn_serve(cmd, Duration::from_secs(2));
        let drive = async {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut ready = [0; 5];
            stream.read_exact(&mut ready).await.unwrap();
            assert_eq!(&ready, b"ready");
            if mode == "chatter" {
                loop {
                    if stream.write_all(b"x").await.is_err() {
                        break;
                    }
                    let mut ack = [0];
                    if stream.read_exact(&mut ack).await.is_err() {
                        break;
                    }
                }
            }
        };
        let (result, ()) = tokio::time::timeout(common::HELPER_DEADLINE, async {
            tokio::join!(startup, drive)
        })
        .await
        .expect("startup deadline and fixture handshake must settle");
        let error = match result {
            Ok(mut serve) => {
                kill_serve(&mut serve).await;
                panic!("fixture unexpectedly ready");
            }
            Err(error) => error,
        };
        assert!(error.contains("Err(Elapsed"), "{mode}: {error}");
        assert!(error.contains("cleanup: Ok(ExitStatus"), "{mode}: {error}");
        if mode == "partial" {
            assert!(error.contains("partial-startup-marker"), "{error}");
        }
        if mode == "chatter" {
            assert!(error.contains("unrelated startup chatter"), "{error}");
        }
    }
}

#[tokio::test]
async fn never_exiting_bridge_child_is_timed_out_killed_and_reaped() {
    for mode in ["bridge-open", "bridge-closed"] {
        let mut bridge = McpChild::spawn_command(fixture(mode));
        // libtest prints preamble lines; the fixture's JSON line is its
        // readiness handshake, not a sleep or a wall-clock validity check.
        let mut line = String::new();
        tokio::time::timeout(common::HELPER_DEADLINE, async {
            loop {
                line.clear();
                assert_ne!(bridge.stdout.read_line(&mut line).await.unwrap(), 0);
                if line.contains("fixture_ready") {
                    break;
                }
            }
        })
        .await
        .expect("fixture readiness");
        if mode == "bridge-closed" {
            let mut tail = Vec::new();
            tokio::time::timeout(
                common::HELPER_DEADLINE,
                bridge.stdout.read_to_end(&mut tail),
            )
            .await
            .expect("fixture closed stdout")
            .unwrap();
        }
        assert!(
            bridge.child.try_wait().unwrap().is_none(),
            "fixture is still alive"
        );
        // A poll proves the operation is pending before its deadline.
        let mut finish = Box::pin(bridge.finish_with_deadline(common::HELPER_DEADLINE));
        std::future::poll_fn(|cx| {
            assert!(finish.as_mut().poll(cx).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        drop(finish);
        // Expire immediately after readiness. This tests the real helper's
        // timeout branch without spending 30 seconds or asserting a ceiling.
        let result = bridge.finish_with_deadline(Duration::ZERO).await;
        // Tokio's id() observes completion without reaping. Check it before
        // try_wait(), which could otherwise repair a missing helper reap;
        // the completed handle then supplies cached status without PID reuse.
        let completed = bridge.child.id().is_none();
        if !completed {
            let cleanup = kill_and_reap(&mut bridge.child).await;
            panic!(
                "bridge helper returned before completing child reap: {mode}; cleanup: {cleanup:?}"
            );
        }
        let error = result.unwrap_err();
        assert!(error.contains("Err(Elapsed"), "{error}");
        assert!(error.contains("cleanup: Ok(ExitStatus"), "{error}");
        let status = bridge
            .child
            .try_wait()
            .unwrap()
            .expect("already reaped status");
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(status.signal(), Some(libc::SIGKILL));
    }
}

/// Deliberately return bytes followed by the original read error. The
/// fully framed prefix would parse successfully if the error were ignored.
#[tokio::test]
async fn http_read_error_preserves_its_source_instead_of_parsing_prefix_bytes() {
    struct FailsAfterBytes(usize);
    impl tokio::io::AsyncRead for FailsAfterBytes {
        fn poll_read(
            mut self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            let bytes = b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\nx";
            if self.0 < bytes.len() {
                let end = (self.0 + buf.remaining()).min(bytes.len());
                buf.put_slice(&bytes[self.0..end]);
                self.0 = end;
                std::task::Poll::Ready(Ok(()))
            } else {
                std::task::Poll::Ready(Err(std::io::Error::other("read-failure-source-marker")))
            }
        }
    }
    let error = common::read_http_response(&mut FailsAfterBytes(0), "regression HTTP read")
        .await
        .unwrap_err();
    assert_eq!(
        error,
        "regression HTTP read: the read failed: read-failure-source-marker"
    );
}

/// A real TCP reset after the reader has consumed response bytes. The
/// reader's acknowledgement makes reset-after-prefix deterministic; a
/// reset immediately after write would race delivery of those bytes.
#[tokio::test]
async fn http_read_error_after_real_response_bytes_cannot_become_success() {
    struct ObservedReader {
        stream: TcpStream,
        consumed: Option<tokio::sync::oneshot::Sender<()>>,
    }
    impl tokio::io::AsyncRead for ObservedReader {
        fn poll_read(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            let before = buf.filled().len();
            let result = std::pin::Pin::new(&mut self.stream).poll_read(cx, buf);
            if matches!(result, std::task::Poll::Ready(Ok(())))
                && buf.filled().len() > before
                && let Some(tx) = self.consumed.take()
            {
                tx.send(()).expect("prefix consumed before reset");
            }
            result
        }
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let stream = TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    let (mut peer, _) = listener.accept().await.unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let mut reader = ObservedReader {
        stream,
        consumed: Some(tx),
    };
    let reset = async {
        peer.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\nx")
            .await
            .unwrap();
        tokio::time::timeout(common::HELPER_DEADLINE, rx)
            .await
            .unwrap()
            .unwrap();
        use std::os::fd::AsRawFd;
        let linger = libc::linger {
            l_onoff: 1,
            l_linger: 0,
        };
        assert_eq!(
            unsafe {
                libc::setsockopt(
                    peer.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_LINGER,
                    (&linger as *const libc::linger).cast(),
                    std::mem::size_of_val(&linger) as libc::socklen_t,
                )
            },
            0
        );
        drop(peer);
    };
    let (result, ()) = tokio::join!(
        common::read_http_response(&mut reader, "real HTTP read"),
        reset
    );
    let error = result.unwrap_err();
    let source = std::io::Error::from_raw_os_error(libc::ECONNRESET);
    assert_eq!(error, format!("real HTTP read: the read failed: {source}"));
}
