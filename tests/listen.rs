//! Every listener binds through `swarmail::listen`: the backlog that keeps a
//! burst of fresh connections from overflowing, and `TcpListener::bind`'s
//! address handling kept around it.

use std::net::SocketAddr;

#[tokio::test]
async fn listen_binds_both_families_and_reports_refusals() {
    let v4 = swarmail::listen("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = v4.local_addr().unwrap();
    assert!(addr.is_ipv4());
    let v6 = swarmail::listen("[::1]:0").await.unwrap();
    assert!(v6.local_addr().unwrap().is_ipv6());

    // A port another listener holds is refused with the bind error itself.
    let taken = swarmail::listen(&addr.to_string()).await.unwrap_err();
    assert_eq!(taken.kind(), std::io::ErrorKind::AddrInUse);
    // So is an address that does not parse or resolve.
    assert!(swarmail::listen("not an address").await.is_err());
}

/// A burst far past the platform default backlog of 128, all opened before
/// the accept loop can catch up: every one must reach the greeting. With the
/// default backlog, the kernel dropped the overflow and those clients waited
/// for a greeting that never came. Linux only: the burst needs more open
/// files than a default macOS runner allows.
#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread")]
async fn a_burst_of_fresh_smtp_connections_is_greeted() {
    let cfg = swarmail::config::Config {
        smtp_listen: "127.0.0.1:0".into(),
        http_listen: "127.0.0.1:0".into(),
        pop3_listen: "127.0.0.1:0".into(),
        max_per_inbox: 0,
        data_file: None,
        tls_cert: None,
        tls_key: None,
        smtp: swarmail::smtp::SmtpConfig {
            max_sessions: 1000,
            ..Default::default()
        },
        pop3_idle_timeout: swarmail::pop3::DEFAULT_IDLE_TIMEOUT,
    };
    let srv = swarmail::run_on(&cfg).await.unwrap();
    let addr = srv.smtp_addr;
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..600 {
        tasks.spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let mut banner = String::new();
            let mut reader = tokio::io::BufReader::new(stream);
            let read = reader.read_line(&mut banner);
            tokio::time::timeout(std::time::Duration::from_secs(5), read)
                .await
                .is_ok_and(|n| n.is_ok() && banner.starts_with("220"))
        });
    }
    let mut greeted = 0;
    while let Some(ok) = tasks.join_next().await {
        greeted += usize::from(ok.unwrap());
    }
    assert_eq!(greeted, 600, "every connection of the burst is greeted");
    srv.stop().await;
}
