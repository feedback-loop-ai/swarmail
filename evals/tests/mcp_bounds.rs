//! The MCP client's transport bounds (accepted decision 0132 §7, spec AC7,
//! repair R11): a reply is streamed under 16 MiB and an operation's replies
//! under one shared 1 GiB budget, each chunk checked before it is appended;
//! a redirect is refused, never followed. Every reply path is covered —
//! initialization, a tool result and a JSON-RPC error.
//!
//! The responder is a bounded loopback HTTP/1.1 server speaking real framing
//! (declared lengths and chunked streams). It is not a target: it provokes
//! the refusals no real target can be made to send.

use serde_json::{Value, json};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use swarmail_evals::mcp::{self, McpClient};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// What the responder sends for one JSON-RPC request.
#[derive(Clone, Copy)]
enum Reply {
    /// The ordinary answer to the method, with a matching id.
    Ok,
    /// A `302` pointing at a second listener that counts its connections.
    Redirect,
    /// A declared `content-length` over the reply bound, and no body.
    DeclaredHuge,
    /// A chunked stream that never ends below the reply bound.
    StreamedHuge,
    /// A JSON-RPC error reply padded past the reply bound, streamed.
    HugeError,
}

/// Serve `plan(method)` for every request; returns the address and the
/// count of connections a redirect target received.
async fn responder(plan: fn(&str) -> Reply) -> (SocketAddr, Arc<AtomicU64>) {
    let followed = Arc::new(AtomicU64::new(0));
    let decoy = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let decoy_addr = decoy.local_addr().unwrap();
    let f = followed.clone();
    tokio::spawn(async move {
        while let Ok((_s, _)) = decoy.accept().await {
            f.fetch_add(1, Ordering::SeqCst);
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut s, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = Vec::new();
                loop {
                    let mut chunk = [0u8; 4096];
                    let Ok(n) = s.read(&mut chunk).await else {
                        return;
                    };
                    if n == 0 {
                        return;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                    while let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&buf[..end]).to_ascii_lowercase();
                        let len = head
                            .lines()
                            .find_map(|l| l.strip_prefix("content-length:"))
                            .map_or(0, |v| v.trim().parse::<usize>().unwrap());
                        if buf.len() < end + 4 + len {
                            break;
                        }
                        let body: Value =
                            serde_json::from_slice(&buf[end + 4..end + 4 + len]).unwrap();
                        buf.drain(..end + 4 + len);
                        if !answer(&mut s, &body, plan, decoy_addr).await {
                            return;
                        }
                    }
                }
            });
        }
    });
    (addr, followed)
}

async fn answer(
    s: &mut tokio::net::TcpStream,
    req: &Value,
    plan: fn(&str) -> Reply,
    decoy: SocketAddr,
) -> bool {
    let method = req["method"].as_str().unwrap_or_default();
    let Some(id) = req.get("id").cloned() else {
        // A notification: accepted, no body.
        return s
            .write_all(b"HTTP/1.1 202 Accepted\r\ncontent-length: 0\r\n\r\n")
            .await
            .is_ok();
    };
    let ok = |result: Value| json!({"jsonrpc": "2.0", "id": id, "result": result}).to_string();
    let fixed = |body: String| {
        format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        )
    };
    match plan(method) {
        Reply::Ok => {
            let body = if method == "initialize" {
                ok(json!({"protocolVersion": mcp::PROTOCOL_VERSION}))
            } else {
                ok(json!({"isError": false, "content": [{"type": "text", "text": "{\"n\":1}"}]}))
            };
            s.write_all(fixed(body).as_bytes()).await.is_ok()
        }
        Reply::Redirect => s
            .write_all(
                format!(
                    "HTTP/1.1 302 Found\r\nlocation: http://{decoy}/mcp\r\ncontent-length: 0\r\n\r\n"
                )
                .as_bytes(),
            )
            .await
            .is_ok(),
        Reply::DeclaredHuge => {
            let _ = s
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n",
                        mcp::MAX_REPLY_BYTES + 1
                    )
                    .as_bytes(),
                )
                .await;
            false
        }
        Reply::StreamedHuge | Reply::HugeError => {
            let lead = if matches!(plan(method), Reply::HugeError) {
                format!(
                    "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"error\":{{\"code\":-32603,\"message\":\""
                )
            } else {
                format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":\"")
            };
            if s
                .write_all(b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n")
                .await
                .is_err()
            {
                return false;
            }
            let _ = s
                .write_all(format!("{:x}\r\n{lead}\r\n", lead.len()).as_bytes())
                .await;
            // 64 KiB chunks, up to twice the bound: the client must stop
            // reading first. The writer gives up once the client hangs up.
            let pad = vec![b'x'; 64 << 10];
            for _ in 0..(2 * mcp::MAX_REPLY_BYTES / (64 << 10)) {
                let mut c = format!("{:x}\r\n", pad.len()).into_bytes();
                c.extend_from_slice(&pad);
                c.extend_from_slice(b"\r\n");
                if s.write_all(&c).await.is_err() {
                    return false;
                }
            }
            false
        }
    }
}

fn url(addr: SocketAddr) -> String {
    format!("http://{addr}/mcp")
}

fn only_tools(huge: Reply) -> fn(&str) -> Reply {
    match huge {
        Reply::Redirect => |m| {
            if m == "tools/call" {
                Reply::Redirect
            } else {
                Reply::Ok
            }
        },
        Reply::DeclaredHuge => |m| {
            if m == "tools/call" {
                Reply::DeclaredHuge
            } else {
                Reply::Ok
            }
        },
        Reply::StreamedHuge => |m| {
            if m == "tools/call" {
                Reply::StreamedHuge
            } else {
                Reply::Ok
            }
        },
        Reply::HugeError => |m| {
            if m == "tools/call" {
                Reply::HugeError
            } else {
                Reply::Ok
            }
        },
        Reply::Ok => |_| Reply::Ok,
    }
}

#[tokio::test]
async fn a_legitimate_reply_is_admitted_and_charged_to_the_operation() {
    let (addr, _) = responder(only_tools(Reply::Ok)).await;
    let mut c = McpClient::connect(&url(addr)).await.unwrap();
    let mut spent = 0;
    let v = c
        .call_spending("t", json!({}), None, &mut spent)
        .await
        .unwrap();
    assert_eq!(v, json!({"n": 1}));
    let one = spent;
    assert!(one > 0);
    // A second call of the same operation charges the same budget.
    c.call_spending("t", json!({}), None, &mut spent)
        .await
        .unwrap();
    assert_eq!(spent, 2 * one);
}

#[tokio::test]
async fn a_redirect_is_refused_and_never_followed() {
    let (addr, followed) = responder(only_tools(Reply::Redirect)).await;
    let mut c = McpClient::connect(&url(addr)).await.unwrap();
    let e = c.call("t", json!({})).await.unwrap_err();
    assert!(e.contains(mcp::REDIRECT), "{e}");
    assert_eq!(followed.load(Ordering::SeqCst), 0);
    // Initialization is refused the same way.
    let (addr, followed) = responder(|_| Reply::Redirect).await;
    let e = McpClient::connect(&url(addr)).await.err().unwrap();
    assert!(e.contains(mcp::REDIRECT), "{e}");
    assert_eq!(followed.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_declared_or_streamed_reply_over_its_bound_is_refused() {
    for huge in [Reply::DeclaredHuge, Reply::StreamedHuge, Reply::HugeError] {
        let (addr, _) = responder(only_tools(huge)).await;
        let mut c = McpClient::connect(&url(addr)).await.unwrap();
        let mut spent = 0;
        let e = c
            .call_spending("t", json!({}), None, &mut spent)
            .await
            .unwrap_err();
        assert!(e.contains(mcp::OVERSIZED), "{e}");
        // Nothing past the bound was accounted, so nothing past it was kept.
        assert!(spent <= mcp::MAX_REPLY_BYTES, "{spent}");
    }
}

#[tokio::test]
async fn an_initialization_reply_over_its_bound_is_refused() {
    for huge in [|_: &str| Reply::DeclaredHuge, |_: &str| Reply::StreamedHuge] {
        let (addr, _) = responder(huge).await;
        let e = McpClient::connect(&url(addr)).await.err().unwrap();
        assert!(e.contains(mcp::OVERSIZED), "{e}");
    }
}

#[tokio::test]
async fn an_operation_shares_one_budget_across_its_calls() {
    let (addr, _) = responder(only_tools(Reply::Ok)).await;
    let mut c = McpClient::connect(&url(addr)).await.unwrap();
    // The operation has all but ten bytes of its budget spent already: the
    // next reply, small as it is, is refused before it is appended.
    let mut spent = mcp::MAX_OPERATION_BYTES - 10;
    let e = c
        .call_spending("t", json!({}), None, &mut spent)
        .await
        .unwrap_err();
    assert!(e.contains(mcp::OVERSIZED), "{e}");
    assert_eq!(spent, mcp::MAX_OPERATION_BYTES - 10);
    // Checked arithmetic: a budget at the edge never wraps into admission.
    let mut spent = u64::MAX;
    let e = c
        .call_spending("t", json!({}), None, &mut spent)
        .await
        .unwrap_err();
    assert!(e.contains(mcp::OVERSIZED), "{e}");
}

#[test]
fn a_bound_refusal_keeps_its_class_through_the_worker() {
    use swarmail_evals::api::Outcome;
    use swarmail_evals::apibench::classify;
    let f = |e: &str| classify(e, false);
    assert_eq!(
        f(&format!("tools/call: {}: past 1 bytes", mcp::OVERSIZED)),
        Outcome::Failed {
            reason: "oversized".into()
        }
    );
    assert_eq!(
        f(&format!("POST x: {} (status 302)", mcp::REDIRECT)),
        Outcome::Failed {
            reason: "redirect".into()
        }
    );
}
