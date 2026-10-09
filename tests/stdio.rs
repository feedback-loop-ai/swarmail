//! The stdio JSON-RPC bridge, driven in-process over duplex pipes.

use std::time::Duration;
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, duplex};

/// Drive the bridge with two duplex pipes and collect its stdout.
/// Input is fed and closed (EOF), the bridge's writes are read back.
async fn bridge_over(base_url: &str, input: &[u8]) -> (i32, String) {
    // Pipe 1: test writes input; the bridge reads it (until EOF).
    let (mut input_end, bridge_input) = duplex(64 * 1024);
    // Pipe 2: the bridge writes; the test reads.
    let (mut bridge_output, mut output_end) = duplex(64 * 1024);

    input_end.write_all(input).await.unwrap();
    drop(input_end); // EOF for the bridge's reader

    let (r, w) = tokio::io::split(bridge_input);
    drop(w); // the bridge's end is read-only
    let mut lines = tokio::io::BufReader::new(r).lines();
    let code = swarmail::stdio::bridge(base_url, &mut lines, &mut bridge_output).await;
    drop(bridge_output); // EOF for the test's reader

    let mut collected = Vec::new();
    let _ = tokio::time::timeout(
        Duration::from_secs(5),
        output_end.read_to_end(&mut collected),
    )
    .await
    .expect("bridge output never ended");
    (code, String::from_utf8_lossy(&collected).to_string())
}

mod common {
    use swarmail::RunningServer;
    pub async fn start() -> RunningServer {
        let cfg = swarmail::config::Config {
            smtp_listen: "127.0.0.1:0".into(),
            http_listen: "127.0.0.1:0".into(),
            pop3_listen: "127.0.0.1:0".into(),
            max_per_inbox: 0,
            data_file: None,
            tls_cert: None,
            tls_key: None,
            smtp: Default::default(),
        };
        swarmail::run_on(&cfg).await.unwrap()
    }
}

#[tokio::test]
async fn bridge_forwards_jsonrpc_and_exits_zero_at_eof() {
    let srv = common::start().await;
    let base = format!("http://{}", srv.http_addr);
    let input = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n\n";
    let (code, out) = bridge_over(&base, input).await;
    assert_eq!(code, 0);
    let responses: Vec<serde_json::Value> = out
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(
        responses.len(),
        1,
        "blank line skipped, one response: {out}"
    );
    assert_eq!(responses[0]["result"]["protocolVersion"], "2025-03-26");
}

#[tokio::test]
async fn bridge_replies_jsonrpc_error_when_the_server_is_unreachable() {
    // Port 1 on loopback refuses instantly; the error becomes a JSON-RPC
    // error with the request's id preserved.
    let input = b"{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"tools/list\"}\n";
    let (code, out) = bridge_over("http://127.0.0.1:1", input).await;
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(v["id"], 7);
    assert_eq!(v["error"]["code"], -32000);
}

#[tokio::test]
async fn bridge_posts_under_a_base_url_path_prefix() {
    // A base URL with a path keeps the prefix: POST {path}/mcp. Port 1
    // refuses, so the parse (not the reply) is what this proves.
    let input = b"{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"tools/list\"}\n";
    let (code, out) = bridge_over("http://127.0.0.1:1/api/v2", input).await;
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(v["error"]["code"], -32000);
}

#[tokio::test]
async fn bridge_reports_read_errors_with_exit_one() {
    struct Boom;
    impl tokio::io::AsyncRead for Boom {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            _buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::task::Poll::Ready(Err(std::io::Error::other("boom")))
        }
    }
    let mut lines = tokio::io::BufReader::new(Boom).lines();
    let mut sink = duplex(1024).0;
    let code = swarmail::stdio::bridge("http://127.0.0.1:1", &mut lines, &mut sink).await;
    assert_eq!(code, 1);
}
