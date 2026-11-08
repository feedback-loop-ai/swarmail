//! The bridge against the real `swarmail mcp` stdio transport
//! (`SWARMAIL_BIN`, no download): the agent-side HTTP endpoint the runner
//! gives competitors' adapters, proved on a real stdio MCP server
//! (proposed decision 0129 §4, spec R3).

mod common;

use serde_json::{Value, json};
use std::process::Stdio;
use swarmail_evals::bridge::Bridge;
use swarmail_evals::mcp::McpClient;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

struct Fixture {
    server: Child,
    stdio: Child,
    bridge: Option<Bridge>,
    _dir: tempfile::TempDir,
}

impl Fixture {
    async fn start() -> Fixture {
        let bin = common::swarmail_bin();
        let dir = common::tempdir();
        let mut server = Command::new(bin)
            .args([
                "serve",
                "--smtp-listen",
                "127.0.0.1:0",
                "--pop3-listen",
                "127.0.0.1:0",
                "--http-listen",
                "127.0.0.1:0",
            ])
            .current_dir(dir.path())
            .env_clear()
            .env("HOME", dir.path())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .expect("swarmail serve starts");
        let mut lines = BufReader::new(server.stdout.take().unwrap());
        let http = loop {
            let mut line = String::new();
            let n = tokio::time::timeout(
                std::time::Duration::from_secs(30),
                lines.read_line(&mut line),
            )
            .await
            .expect("the startup line in time")
            .unwrap();
            assert!(n > 0, "swarmail exited before announcing");
            if let Some((_, http)) = swarmail_evals::runner::parse_swarmail_addrs(&line) {
                break http;
            }
        };
        // Keep draining so the server never blocks on a full pipe.
        tokio::spawn(async move {
            let mut sink = String::new();
            while lines.read_line(&mut sink).await.is_ok_and(|n| n > 0) {
                sink.clear();
            }
        });
        let mut stdio = Command::new(bin)
            .args(["mcp", "--url", &format!("http://{http}")])
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .expect("swarmail mcp starts");
        let to: ChildStdin = stdio.stdin.take().unwrap();
        let from: ChildStdout = stdio.stdout.take().unwrap();
        let bridge = Bridge::start(to, BufReader::new(from)).await.unwrap();
        Fixture {
            server,
            stdio,
            bridge: Some(bridge),
            _dir: dir,
        }
    }

    fn url(&self) -> String {
        self.bridge.as_ref().unwrap().url()
    }
}

fn http() -> reqwest::Client {
    reqwest::Client::builder().no_proxy().build().unwrap()
}

async fn post(url: &str, body: &Value) -> (u16, Value) {
    let r = http()
        .post(url)
        .header("content-type", "application/json")
        .body(body.to_string())
        .send()
        .await
        .unwrap();
    let status = r.status().as_u16();
    let text = r.text().await.unwrap();
    (status, serde_json::from_str(&text).unwrap_or(Value::Null))
}

fn init(id: i64) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": "initialize", "params": {
        "protocolVersion": "2025-03-26", "capabilities": {},
        "clientInfo": {"name": "bridge-test", "version": "9.9"}}})
}

#[tokio::test]
async fn the_bridge_carries_initialize_tools_calls_batches_and_notifications() {
    let f = Fixture::start().await;
    let url = f.url();
    // The eval client itself negotiates and calls through the bridge.
    let mut client = McpClient::connect(&url).await.expect("initialize");
    let tools = client.rpc("tools/list", json!({})).await.unwrap();
    assert!(
        tools["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["name"] == "swarmail_wait_for_email")
    );
    let waited = client
        .wait("bridge-inbox", None, 1, 1)
        .await
        .expect("a tool call");
    assert_eq!(waited["timed_out"], true, "{waited}");
    assert_eq!(client.tool_calls(), 1);

    // A batch is forwarded as one line and answered as one array.
    let (status, reply) = post(
        &url,
        &json!([
            {"jsonrpc": "2.0", "id": 11, "method": "ping"},
            {"jsonrpc": "2.0", "id": "b", "method": "tools/list"},
            {"jsonrpc": "2.0", "method": "notifications/initialized"}
        ]),
    )
    .await;
    assert_eq!(status, 200, "{reply}");
    let replies = reply
        .as_array()
        .unwrap_or_else(|| panic!("an array: {reply}"));
    assert_eq!(replies.len(), 2, "{reply}");
    let ids: Vec<&Value> = replies.iter().map(|r| &r["id"]).collect();
    assert!(
        ids.contains(&&json!(11)) && ids.contains(&&json!("b")),
        "{reply}"
    );

    // Notifications only: forwarded, answered 202 with no body.
    for body in [
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        json!([{"jsonrpc": "2.0", "method": "notifications/initialized"}]),
    ] {
        let r = http()
            .post(&url)
            .body(body.to_string())
            .send()
            .await
            .unwrap();
        assert_eq!(r.status().as_u16(), 202);
        assert!(r.text().await.unwrap().is_empty());
    }
    // GET is not allowed.
    assert_eq!(
        http().get(&url).send().await.unwrap().status().as_u16(),
        405
    );
    f.bridge.unwrap().stop().await;
}

#[tokio::test]
async fn the_bridge_refuses_a_foreign_host_and_origin_and_bodies_it_cannot_correlate() {
    let f = Fixture::start().await;
    let url = f.url();
    let port = f.bridge.as_ref().unwrap().addr.port();
    for (header, value) in [
        ("host", "evil.example".to_string()),
        ("host", format!("evil.example:{port}")),
        ("origin", "http://evil.example".to_string()),
        ("origin", format!("http://127.0.0.1:{}", port + 1)),
    ] {
        let r = http()
            .post(&url)
            .header(header, &value)
            .body(init(1).to_string())
            .send()
            .await
            .unwrap();
        assert_eq!(r.status().as_u16(), 403, "{header}: {value}");
    }
    // A loopback Origin of the bound port passes the guard.
    let r = http()
        .post(&url)
        .header("origin", format!("http://localhost:{port}"))
        .body(init(1).to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(r.status().as_u16(), 200);
    // Locally refused: nothing is forwarded for any of these.
    let seventeen: Vec<Value> = (0..17)
        .map(|i| json!({"jsonrpc": "2.0", "id": 100 + i, "method": "ping"}))
        .collect();
    for (body, want) in [
        ("{not json".to_string(), 400),
        ("[]".to_string(), 400),
        ("7".to_string(), 400),
        (Value::Array(seventeen).to_string(), 400),
        (
            r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#.to_string(),
            400,
        ),
        (r#"[{"id":1}]"#.to_string(), 400),
        (format!("[\"{}\"]", "x".repeat(70_000)), 413),
    ] {
        let r = http().post(&url).body(body.clone()).send().await.unwrap();
        assert_eq!(r.status().as_u16(), want, "{}", &body[..body.len().min(40)]);
    }
    // The adapter is still healthy after every refusal.
    let (status, r) = post(&url, &json!({"jsonrpc": "2.0", "id": 3, "method": "ping"})).await;
    assert_eq!((status, r["id"].clone()), (200, json!(3)));
    f.bridge.unwrap().stop().await;
}

#[tokio::test]
async fn a_request_id_already_in_flight_is_refused_with_a_correlated_error() {
    let f = Fixture::start().await;
    let url = f.url();
    assert_eq!(post(&url, &init(1)).await.0, 200);
    // A held request: a wait that takes about five seconds.
    let held = json!({"jsonrpc": "2.0", "id": 7, "method": "tools/call", "params": {
        "name": "swarmail_wait_for_email",
        "arguments": {"inbox": "held", "count": 1, "timeout_ms": 5000}}});
    let first = {
        let (url, held) = (url.clone(), held.clone());
        tokio::spawn(async move { post(&url, &held).await })
    };
    // Retry until the duplicate meets the held one in flight. A fixed try
    // count raced the hold on slow runners; try for as long as the held
    // request is unanswered, so only a bridge that never refuses fails.
    let mut refused = None;
    while !first.is_finished() {
        let (status, r) = post(&url, &json!({"jsonrpc": "2.0", "id": 7, "method": "ping"})).await;
        if r["error"]["code"] == -32600 {
            refused = Some((status, r));
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let (status, r) = refused.expect("the duplicate was refused while the first was in flight");
    assert_eq!((status, r["id"].clone()), (200, json!(7)), "{r}");
    // The held request still gets its own answer, and the id is free again.
    let (status, first) = first.await.unwrap();
    assert_eq!((status, first["id"].clone()), (200, json!(7)), "{first}");
    assert!(first["result"].is_object(), "{first}");
    let (_, again) = post(&url, &json!({"jsonrpc": "2.0", "id": 7, "method": "ping"})).await;
    assert!(again["result"].is_object(), "{again}");
    f.bridge.unwrap().stop().await;
}

#[tokio::test]
async fn an_adapter_that_has_exited_yields_503() {
    let mut f = Fixture::start().await;
    let url = f.url();
    assert_eq!(post(&url, &init(1)).await.0, 200);
    f.stdio.kill().await.unwrap();
    let mut status = 0;
    for _ in 0..200 {
        status = post(&url, &json!({"jsonrpc": "2.0", "id": 2, "method": "ping"}))
            .await
            .0;
        if status == 503 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    assert_eq!(status, 503);
    assert!(f.bridge.as_ref().unwrap().adapter_exited());
    // Notifications are refused the same way: nothing can be forwarded.
    let r = http()
        .post(&url)
        .body(json!({"jsonrpc": "2.0", "method": "notifications/initialized"}).to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(r.status().as_u16(), 503);
    f.bridge.take().unwrap().stop().await;
    let _ = f.server.kill().await;
}

#[tokio::test]
async fn only_negotiation_metadata_is_kept_never_a_body() {
    let f = Fixture::start().await;
    let url = f.url();
    assert_eq!(post(&url, &init(1)).await.0, 200);
    let canary = "CANARY-secret-body-zzz";
    let (_, r) = post(
        &url,
        &json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
            "name": "swarmail_wait_for_email",
            "arguments": {"inbox": canary, "count": 1, "timeout_ms": 1}}}),
    )
    .await;
    assert!(r["result"].is_object(), "{r}");
    // A hostile client name is bounded and stripped of control bytes.
    let (_, _) = post(
        &url,
        &json!({"jsonrpc": "2.0", "id": 3, "method": "initialize", "params": {
            "protocolVersion": "2025-03-26",
            "clientInfo": {"name": format!("x\u{1b}[31m{}", "n".repeat(500)), "version": 5}}}),
    )
    .await;
    let sessions = f.bridge.as_ref().unwrap().sessions();
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].client_name.as_deref(), Some("bridge-test"));
    assert_eq!(sessions[0].client_version.as_deref(), Some("9.9"));
    assert_eq!(sessions[0].protocol_version.as_deref(), Some("2025-03-26"));
    assert!(sessions[1].client_name.as_ref().unwrap().len() <= 128);
    assert!(!sessions[1].client_name.as_ref().unwrap().contains('\u{1b}'));
    assert_eq!(sessions[1].client_version, None, "a non-string is dropped");
    let kept = serde_json::to_string(&sessions).unwrap();
    assert!(!kept.contains(canary), "{kept}");
    assert_eq!(
        serde_json::from_str::<Value>(&kept).unwrap()[0]
            .as_object()
            .unwrap()
            .len(),
        3,
        "three fields and nothing else"
    );
    f.bridge.unwrap().stop().await;
}

/// A scripted adapter on in-memory pipes: it records every line the bridge
/// forwards and answers each request only once released, so a test holds
/// exactly as many requests in flight as it chooses and counts what reached
/// the adapter.
struct FakeAdapter {
    seen: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    release: std::sync::Arc<tokio::sync::Notify>,
    bridge: Bridge,
}

impl FakeAdapter {
    async fn start() -> FakeAdapter {
        use tokio::io::AsyncWriteExt;
        let (to_adapter, adapter_in) = tokio::io::duplex(1 << 20);
        let (adapter_out, from_adapter) = tokio::io::duplex(1 << 20);
        let bridge = Bridge::start(to_adapter, BufReader::new(from_adapter))
            .await
            .unwrap();
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let release = std::sync::Arc::new(tokio::sync::Notify::new());
        let (log, gate) = (seen.clone(), release.clone());
        tokio::spawn(async move {
            let mut lines = BufReader::new(adapter_in).lines();
            let mut out = adapter_out;
            let mut held: Vec<Value> = Vec::new();
            let mut open = false;
            loop {
                tokio::select! {
                    line = lines.next_line() => {
                        let Ok(Some(line)) = line else { break };
                        log.lock().unwrap().push(line.clone());
                        if let Ok(v) = serde_json::from_str::<Value>(&line) && let Some(id) = v.get("id") {
                            let reply = json!({"jsonrpc": "2.0", "id": id, "result": {}});
                            if open {
                                let _ = out.write_all(format!("{reply}\n").as_bytes()).await;
                            } else {
                                held.push(reply);
                            }
                        }
                    }
                    _ = gate.notified() => {
                        open = true;
                        for r in held.drain(..) {
                            let _ = out.write_all(format!("{r}\n").as_bytes()).await;
                        }
                    }
                }
            }
        });
        FakeAdapter {
            seen,
            release,
            bridge,
        }
    }

    fn forwarded(&self) -> usize {
        self.seen.lock().unwrap().len()
    }

    async fn until_forwarded(&self, n: usize) {
        let until = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while self.forwarded() < n {
            assert!(
                std::time::Instant::now() < until,
                "only {} of {n} lines reached the adapter",
                self.forwarded()
            );
            tokio::task::yield_now().await;
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_seventeenth_request_in_flight_is_refused_busy_and_never_forwarded() {
    let a = FakeAdapter::start().await;
    let url = a.bridge.url();
    assert_eq!(swarmail_evals::bridge::MAX_IN_FLIGHT, 16);
    // Sixteen requests the adapter holds: all forwarded, none answered.
    let held: Vec<_> = (100..116)
        .map(|id| {
            let url = url.clone();
            tokio::spawn(async move {
                post(&url, &json!({"jsonrpc": "2.0", "id": id, "method": "ping"})).await
            })
        })
        .collect();
    a.until_forwarded(16).await;
    // The seventeenth meets sixteen in flight: refused here, correlated and
    // at once, and nothing more reaches the adapter.
    let (status, r) = post(&url, &json!({"jsonrpc": "2.0", "id": 17, "method": "ping"})).await;
    assert_eq!((status, r["id"].clone()), (200, json!(17)), "{r}");
    assert_eq!(r["error"]["code"], -32000, "{r}");
    assert!(
        r["error"]["message"].as_str().unwrap().starts_with("busy"),
        "{r}"
    );
    // A batch over the bound is refused member by member, each correlated.
    let batch = json!([{"jsonrpc": "2.0", "id": 18, "method": "ping"},
        {"jsonrpc": "2.0", "id": 19, "method": "ping"}]);
    let (_, r) = post(&url, &batch).await;
    let ids: Vec<_> = r
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["id"].clone())
        .collect();
    assert_eq!(ids, vec![json!(18), json!(19)], "{r}");
    assert!(
        r.as_array()
            .unwrap()
            .iter()
            .all(|m| m["error"]["code"] == -32000),
        "{r}"
    );
    assert_eq!(a.forwarded(), 16, "the adapter saw exactly sixteen");
    // Released, the sixteen are all answered and the bound frees up.
    a.release.notify_one();
    for h in held {
        let (status, r) = h.await.unwrap();
        assert!(status == 200 && r["result"].is_object(), "{status} {r}");
    }
    let (_, again) = post(&url, &json!({"jsonrpc": "2.0", "id": 17, "method": "ping"})).await;
    assert!(again["result"].is_object(), "{again}");
    assert_eq!(a.forwarded(), 17);
    a.bridge.stop().await;
}

#[tokio::test]
async fn a_pretty_printed_body_reaches_the_adapter_as_one_line() {
    let a = FakeAdapter::start().await;
    a.release.notify_one();
    let url = a.bridge.url();
    // Embedded newlines and indentation: forwarded raw, a line-oriented
    // adapter would read half an object. The bridge re-serialises it.
    let pretty = "{\n  \"jsonrpc\": \"2.0\",\n  \"id\": 3,\n  \"method\": \"initialize\",\n  \"params\": {\n    \"protocolVersion\": \"2025-03-26\",\n    \"capabilities\": {},\n    \"clientInfo\": {\"name\": \"pretty\", \"version\": \"1\"}\n  }\n}\n";
    let r = http()
        .post(&url)
        .header("content-type", "application/json")
        .body(pretty.to_owned())
        .send()
        .await
        .unwrap();
    assert_eq!(r.status().as_u16(), 200);
    a.until_forwarded(1).await;
    let seen = a.seen.lock().unwrap().clone();
    assert_eq!(seen.len(), 1, "one line, not one per source line: {seen:?}");
    assert!(!seen[0].contains('\n'), "{:?}", seen[0]);
    let parsed: Value = serde_json::from_str(&seen[0]).unwrap();
    assert_eq!(parsed, serde_json::from_str::<Value>(pretty).unwrap());
    a.bridge.stop().await;
}

/// A stdio server that, like every adapter, refuses a second `initialize`
/// with "already initialized", and counts how many reached it.
const ONE_SESSION_SERVER: &str = r#"
import sys, json
seen = 0
for line in sys.stdin:
    msg = json.loads(line)
    if "id" not in msg: continue
    if msg["method"] == "initialize":
        seen += 1
        if seen > 1:
            print(json.dumps({"jsonrpc": "2.0", "id": msg["id"], "error": {"code": -32600, "message": "already initialized"}}), flush=True)
            continue
        r = {"protocolVersion": "2025-03-26", "capabilities": {"tools": {}}, "serverInfo": {"name": "one-session", "version": "1"}}
    elif msg["method"] == "ping":
        r = {"initializes_seen": seen}
    else:
        r = {}
    print(json.dumps({"jsonrpc": "2.0", "id": msg["id"], "result": r}), flush=True)
"#;

#[tokio::test]
async fn several_clients_share_one_stdio_session_because_a_later_initialize_is_answered_from_the_first()
 {
    let python = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|d| d.join("python3"))
        .find(|p| p.is_file())
        .expect("python3 is on PATH");
    let mut child = Command::new(python)
        .args(["-I", "-c", ONE_SESSION_SERVER])
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let to: ChildStdin = child.stdin.take().unwrap();
    let from: ChildStdout = child.stdout.take().unwrap();
    let bridge = Bridge::start(to, BufReader::new(from)).await.unwrap();
    let url = bridge.url();
    // The runner's probe, the oracle and an agent CLI that reconnects each
    // say initialize; the adapter would refuse all but the first.
    for name in ["probe", "oracle", "agent", "agent again"] {
        let mut body = init(1);
        body["params"]["clientInfo"]["name"] = name.into();
        let (status, reply) = post(&url, &body).await;
        assert_eq!(status, 200);
        assert_eq!(reply["id"], 1);
        assert_eq!(
            reply["result"]["serverInfo"]["name"], "one-session",
            "{name}: {reply}"
        );
        assert_eq!(reply["result"]["protocolVersion"], "2025-03-26");
    }
    let (_, ping) = post(&url, &json!({"jsonrpc": "2.0", "id": 9, "method": "ping"})).await;
    assert_eq!(
        ping["result"]["initializes_seen"], 1,
        "only the first reached the adapter: {ping}"
    );
    // Every client's negotiation is logged, never a body.
    let names: Vec<_> = bridge
        .sessions()
        .iter()
        .filter_map(|s| s.client_name.clone())
        .collect();
    assert_eq!(names, ["probe", "oracle", "agent", "agent again"]);
    bridge.stop().await;
}
