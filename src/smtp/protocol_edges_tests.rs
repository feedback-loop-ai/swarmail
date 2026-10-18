//! A stalled STARTTLS reader releases its session slot (issue #121,
//! decision 0046 §1, decision 0020 rule 2).
//!
//! The proof drives the real listener with a generated certificate and a
//! real TLS client. Its cold path is the deadline diagnostic: with the
//! failed session's graceful TLS close back, the close flushes into the
//! same shut window forever, the session never finishes, and only that
//! deadline lets the test end red instead of hanging. The line runs only
//! while the bug is present, so the test cannot live inline.

use super::tests::{finished_while_parked, greeted, reply, tls_connect, tls_connector};
use super::*;

/// A real STARTTLS peer that stops reading cannot keep its slot past the
/// reply deadline (issue #121, AC1). One slot, a 200 ms idle bound, a 1 KiB
/// client receive buffer and 100,000 pipelined `EHLO`s over the negotiated
/// TLS session, whose replies (~13 MB) outrun every buffer: the client
/// never reads a reply byte, so the server's reply write backs up behind
/// the shut window and expires. The session's own completion is Tokio's
/// flag, observed while accept is parked (EOF is no task barrier), against
/// a real deadline whose first check precedes any yield. Then a fresh peer
/// is greeted `220` while the stalled client still holds its connection
/// open. Red on 2a325c9: the failed session went on to a TLS shutdown that
/// flushed into the same shut window forever, so the session never
/// finished and every new peer got `421 4.7.0`.
#[tokio::test]
async fn starttls_stalled_reply_releases_slot() {
    let pair = crate::tls::TlsConfig::generate_self_signed("localhost").unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    SPAWNED.with(|spawned| spawned.borrow_mut().clear());
    let server = tokio::spawn(serve(
        listener,
        Arc::new(Store::new(0)),
        Arc::new(Chaos::default()),
        SmtpConfig {
            idle_timeout: Duration::from_millis(200),
            max_sessions: 1,
            ..SmtpConfig::default()
        },
        Some(Arc::new(pair.server_config().unwrap())),
        Arc::new(AtomicBool::new(false)),
        std::future::pending(),
    ));
    let socket = tokio::net::TcpSocket::new_v4().unwrap();
    socket.set_recv_buffer_size(1024).unwrap();
    let mut tcp = socket.connect(addr).await.unwrap();
    assert!(reply(&mut tcp).await.starts_with("220 "));
    tcp.write_all(b"STARTTLS\r\n").await.unwrap();
    assert_eq!(reply(&mut tcp).await, "220 2.0.0 Ready to start TLS\r\n");
    let tls = tls_connect(&tls_connector(&pair), tcp).await;
    // The read half is held, never read: the connection stays open.
    let (unread, mut write) = tokio::io::split(tls);
    let flood = tokio::spawn(async move {
        let _ = write.write_all(&b"EHLO x\r\n".repeat(100_000)).await;
        write
    });

    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !finished_while_parked(0) {
        assert!(
            std::time::Instant::now() < deadline,
            "the stalled STARTTLS session never released its slot"
        );
        tokio::task::yield_now().await;
    }
    let _fresh = greeted(addr).await;
    // Only now may the stalled peer go: it was open throughout.
    drop(unread);
    flood.abort();
    let _ = flood.await;
    server.abort();
    let _ = server.await;
}
