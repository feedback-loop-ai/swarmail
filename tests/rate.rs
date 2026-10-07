//! The 5k-mail guarantee + end-to-end throughput smoke.
//!
//! Run (release mode):
//!   cargo test --release --test rate -- --ignored --nocapture
//!
//! 100 concurrent SMTP sessions × 50 mails = 5000, every session a fresh
//! connection — exactly the pattern that leaked MailSlurper sessions under
//! load. Every accepted DATA must be queryable, and the exact count must hold.

mod common;

use common::{http_json, smtp_send, start_with};

#[tokio::test]
#[ignore = "throughput smoke — run with: cargo test --release --test rate -- --ignored --nocapture"]
async fn five_k_burst_guarantee_and_throughput() {
    let srv = start_with(|mut c| {
        c.max_per_inbox = 0;
        c
    })
    .await;

    const CONNS: usize = 100;
    const PER_CONN: usize = 50;
    let started = std::time::Instant::now();

    let mut tasks = Vec::new();
    for c in 0..CONNS {
        let smtp_addr = srv.smtp_addr;
        tasks.push(tokio::spawn(async move {
            for i in 0..PER_CONN {
                smtp_send(
                    smtp_addr,
                    None,
                    "noreply@load.io",
                    &format!("user{c}-{i}@example.com"),
                    &format!("load {c}/{i}"),
                    "Throughput load body with a code 123456.",
                )
                .await
                .unwrap_or_else(|e| panic!("conn {c} mail {i} rejected: {e}"));
            }
        }));
    }
    for t in tasks {
        t.await.unwrap();
    }
    let smtp_elapsed = started.elapsed();

    let (status, body) =
        http_json(srv.http_addr, "GET", "/api/v1/inboxes/default/count", None).await;
    assert_eq!(status, 200);
    assert_eq!(
        body["count"], 5000,
        "the 5k guarantee: every accepted mail stored, exactly, losslessly"
    );

    let rate = CONNS as f64 * PER_CONN as f64 / smtp_elapsed.as_secs_f64();
    println!();
    println!("=== Swarmail end-to-end ingest (release) ===");
    println!("5000 mails over 100 concurrent fresh SMTP sessions");
    println!("elapsed: {smtp_elapsed:.2?}  →  {rate:.0} mails/sec");
    println!("(fresh session per mail — the Kratos courier pattern)");
}
