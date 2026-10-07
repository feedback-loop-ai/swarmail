//! stdio MCP transport: bridge newline-delimited JSON-RPC on stdin/stdout to a
//! running Swarmail's `POST /mcp`. Lets Claude Desktop / Claude Code / Codex
//! launch `swarmail mcp` as a subprocess.

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

/// POST `body` to `{base}/mcp`, return the response body string.
async fn rpc_post(base: &str, body: &str) -> Result<String, String> {
    let rest = base
        .strip_prefix("http://")
        .ok_or_else(|| "SWARMAIL_URL must be http:// (the local server has no TLS)".to_string())?;
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let mut stream = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        tokio::net::TcpStream::connect(authority.to_string()),
    )
    .await
    .map_err(|_| "connect timeout".to_string())?
    .map_err(|e| e.to_string())?;

    let req = format!(
        "POST {path}/mcp HTTP/1.1\r\nHost: {authority}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(req.as_bytes())
        .await
        .map_err(|e| e.to_string())?;

    let mut buf = Vec::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        stream.read_to_end(&mut buf),
    )
    .await
    .map_err(|_| "read timeout".to_string())?
    .map_err(|e| e.to_string())?;
    let raw = String::from_utf8_lossy(&buf);
    let body = raw.split("\r\n\r\n").nth(1).unwrap_or("{}");
    // Chunked encoding would need a decoder; axum replies with content-length
    // for Json responses, but be forgiving: prefer the raw tail.
    Ok(body.to_string())
}

pub async fn run_stdio_bridge(base_url: &str) -> i32 {
    let stdin = tokio::io::stdin();
    let mut lines = BufReader::new(stdin).lines();
    let mut stdout = tokio::io::stdout();

    loop {
        match lines.next_line().await {
            Ok(None) => return 0, // EOF
            Ok(Some(line)) => {
                if line.trim().is_empty() {
                    continue;
                }
                match rpc_post(base_url, &line).await {
                    Ok(resp) => {
                        let _ = stdout.write_all(resp.trim().as_bytes()).await;
                        let _ = stdout.write_all(b"\n").await;
                        let _ = stdout.flush().await;
                    }
                    Err(e) => {
                        // Reply as a JSON-RPC error so the client stays usable.
                        let id = serde_json::from_str::<serde_json::Value>(&line)
                            .ok()
                            .and_then(|r| r.get("id").cloned())
                            .unwrap_or(serde_json::Value::Null);
                        let err = serde_json::json!({
                            "jsonrpc": "2.0", "id": id,
                            "error": { "code": -32000, "message": e }
                        });
                        let _ = stdout.write_all(err.to_string().as_bytes()).await;
                        let _ = stdout.write_all(b"\n").await;
                        let _ = stdout.flush().await;
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "stdio read error");
                return 1;
            }
        }
    }
}
