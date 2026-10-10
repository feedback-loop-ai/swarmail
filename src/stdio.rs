//! stdio MCP transport: bridge newline-delimited JSON-RPC on stdin/stdout to a
//! running Swarmail's `POST /mcp`. Lets Claude Desktop / Claude Code / Codex
//! launch `swarmail mcp` as a subprocess.

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

/// The dial deadline production uses; a parameter of [`dial`] so the expiry
/// arm is verifiable in-process against a peer that never answers.
const DIAL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
/// The read deadline production uses.
const READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Dial the MCP server: the deadline expiry and the io failure surface as
/// distinct, client-usable error strings.
async fn dial(
    authority: &str,
    deadline: std::time::Duration,
) -> Result<tokio::net::TcpStream, String> {
    tokio::time::timeout(
        deadline,
        tokio::net::TcpStream::connect(authority.to_string()),
    )
    .await
    .map_err(|_| "connect timeout".to_string())?
    .map_err(|e| e.to_string())
}

/// POST `body` to `{base}/mcp`, return the response body string.
async fn rpc_post(base: &str, body: &str) -> Result<String, String> {
    let rest = base
        .strip_prefix("http://")
        .ok_or_else(|| "SWARMAIL_URL must be http:// (the local server has no TLS)".to_string())?;
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let mut stream = dial(authority, DIAL_TIMEOUT).await?;

    let req = format!(
        "POST {path}/mcp HTTP/1.1\r\nHost: {authority}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );

    // One owner for the transport failure: the request write and the response
    // read surface through the same io-error text, whatever the peer does.
    let mut buf = Vec::new();
    let exchanged = async {
        stream.write_all(req.as_bytes()).await?;
        stream.read_to_end(&mut buf).await
    };
    tokio::time::timeout(READ_TIMEOUT, exchanged)
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
    bridge(
        base_url,
        &mut BufReader::new(tokio::io::stdin()).lines(),
        &mut tokio::io::stdout(),
    )
    .await
}

/// The bridge core, generic over reader/writer so tests drive it in-process.
pub async fn bridge<R, W>(base_url: &str, lines: &mut tokio::io::Lines<R>, stdout: &mut W) -> i32
where
    R: tokio::io::AsyncBufRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The dial deadline expires when the peer never completes the SYN:
    /// a listener whose accept queue is full drops further SYNs silently,
    /// so the dial hangs until the injected deadline ends the wait. The
    /// deadline is a parameter, so the production arm is exercised with a
    /// 50 ms wait instead of the five-second production deadline.
    #[tokio::test]
    async fn dial_deadline_expires_with_the_named_error() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        // Flood the accept queue (std listens with a 128 backlog): nobody
        // accepts, so every dial past the queue capacity hangs. Each flood
        // dial carries its own short deadline; the ones that complete hold
        // the queue, and the first timeout tells us it is full.
        let mut flood = Vec::new();
        loop {
            let outcome = dial(&addr.to_string(), std::time::Duration::from_millis(20)).await;
            if let Ok(c) = outcome {
                flood.push(c);
                assert!(flood.len() < 100_000, "the queue never filled");
                continue;
            }
            // The queue is full: the dial deadline expired. The deadline is
            // the only failure a full queue can produce, checked through the
            // formatted outcome — no arm of its own to cover.
            let reported = format!("{outcome:?}");
            assert!(
                reported.contains("connect timeout"),
                "the queue-full dial ended on the deadline: {reported}"
            );
            break;
        }
        let started = std::time::Instant::now();
        let err = dial(&addr.to_string(), std::time::Duration::from_millis(50))
            .await
            .unwrap_err();
        assert_eq!(err, "connect timeout", "{err}");
        assert!(
            started.elapsed() >= std::time::Duration::from_millis(45),
            "the deadline, not an instant refusal, ended the dial"
        );
        drop(flood);
    }
}
