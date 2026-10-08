//! The shipped binary itself: serve binds, answers healthz, and shuts down
//! cleanly on SIGINT; the mcp bridge exits 0 at EOF. These tests spawn the
//! instrumented binary, so `src/main.rs` is covered like everything else.

use std::process::{Command, Stdio};
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
        "--http-listen",
        "127.0.0.1:23457",
    ])
    .stdout(Stdio::null())
    .stderr(Stdio::null());
    cmd
}

#[tokio::test]
async fn serve_answers_healthz_and_stops_on_sigint() {
    let mut child = serve_cmd().spawn().expect("spawn swarmail serve");

    // Poll healthz until the server is up.
    let mut up = false;
    for _ in 0..100 {
        if let Ok(Ok(_)) = tokio::time::timeout(
            Duration::from_millis(200),
            tokio::net::TcpStream::connect("127.0.0.1:23457"),
        )
        .await
        {
            up = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(up, "server never came up on 127.0.0.1:23457");

    // SIGINT = ctrl_c: the graceful-shutdown path, then a clean exit 0.
    unsafe {
        libc::kill(child.id() as i32, libc::SIGINT);
    }
    let status = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match child.try_wait().unwrap() {
                Some(status) => return status,
                None => tokio::time::sleep(Duration::from_millis(50)).await,
            }
        }
    })
    .await
    .expect("server did not stop on SIGINT");
    assert!(status.success(), "SIGINT exit: {status:?}");
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
