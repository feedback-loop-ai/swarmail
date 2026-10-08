//! Process lifecycle: both servers answer while running and stop cleanly
//! when asked — the same graceful path SIGINT takes in production.

mod common;

use common::{SmtpConn, start};
use std::time::Duration;

#[tokio::test]
async fn servers_serve_then_stop_and_release_their_ports() {
    let srv = start().await;
    let smtp_addr = srv.smtp_addr;
    let http_addr = srv.http_addr;

    // Both listeners answer while running.
    let (st, _) = common::http_json(http_addr, "GET", "/healthz", None).await;
    assert_eq!(st, 200);
    let mut c = SmtpConn::connect(smtp_addr).await;
    c.send("NOOP").await;
    assert!(c.reply().await.starts_with("250"));

    // The graceful stop: both serve futures return, exit is logged, and the
    // ports actually close.
    srv.stop().await;

    // HTTP is gone.
    let refused_http = tokio::net::TcpStream::connect(http_addr).await.is_err();
    assert!(refused_http, "http listener should be closed after stop");
    // SMTP is gone (retry a beat: the OS may need a moment to release).
    let mut smtp_closed = false;
    for _ in 0..20 {
        if tokio::net::TcpStream::connect(smtp_addr).await.is_err() {
            smtp_closed = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(smtp_closed, "smtp listener should be closed after stop");
}
