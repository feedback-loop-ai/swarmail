//! Batch members poll concurrently (issue #91, decision 0044 §2; the
//! placement is 0020 rule 2).
//!
//! The proof holds sixteen maximum-length waits in one batch and observes,
//! through the store's own watcher count, that every one of them is
//! registered before any mail is delivered — so their waiting intervals
//! overlap instead of summing. Its cold path is the readiness deadline: with
//! serial dispatch only the first wait is ever registered, and only that
//! deadline lets the test end red instead of parking for sixteen minutes.
//! The line runs only while the bug is present, so the test cannot live
//! inline.

use super::*;
use std::time::{Duration, Instant};

/// Sixteen waits at the 60 s ceiling, each on its own inbox, are all
/// registered at once; then one delivery per inbox releases them all and
/// every reply is a match, in member order. No sleep orders anything: the
/// delivery side spins against a real deadline on the same task, yielding
/// to the batch between looks.
#[tokio::test]
async fn sixteen_maximum_waits_register_together_before_any_delivery() {
    let (_stop, shutdown) = tokio::sync::watch::channel(false);
    let ctx = McpContext {
        tool_rate: Arc::new(ToolRate::default()),
        store: Arc::new(Store::new(0)),
        chaos: Arc::new(Chaos::default()),
        shutdown,
    };
    let members: Vec<String> = (0..MCP_BATCH_CAP)
        .map(|i| {
            json!({"jsonrpc": "2.0", "id": i, "method": "tools/call", "params": {
                "name": "swarmail_wait_for_email",
                "arguments": {"inbox": format!("batch-wait-{i}"), "timeout_ms": 60_000}}})
            .to_string()
        })
        .collect();
    let body = format!("[{}]", members.join(","));
    let store = ctx.store.clone();
    let deliver = async {
        let deadline = Instant::now() + Duration::from_secs(30);
        while store.watcher_count() < MCP_BATCH_CAP {
            assert!(
                Instant::now() < deadline,
                "only {} of {MCP_BATCH_CAP} batch waits were ever registered together",
                store.watcher_count()
            );
            tokio::task::yield_now().await;
        }
        for i in 0..MCP_BATCH_CAP {
            let email = crate::store::email(&format!("m{i}"), &format!("batch-wait-{i}"), "a@x.io");
            store.insert(email).unwrap();
        }
    };
    let (response, ()) = tokio::join!(dispatch(&ctx, body.as_bytes()), deliver);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let replies: Value = serde_json::from_slice(&bytes).unwrap();
    let replies = replies.as_array().unwrap();
    assert_eq!(replies.len(), MCP_BATCH_CAP);
    for (i, reply) in replies.iter().enumerate() {
        assert_eq!(reply["id"], i, "replies keep member order");
        let result: Value =
            serde_json::from_str(reply["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(result["matched"], 1, "{reply}");
        assert!(result.get("timed_out").is_none(), "{reply}");
    }
}
