//! Offline CLI assembly, private admission and real helper exchanges.
//! Stand-ins establish assembly, never live provider behavior.
mod common;
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::Request,
    http::Response,
    routing::any,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::Path,
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use swarmail_evals::{
    fixture_http::{Helper, RESPONSE_LIMIT},
    runner,
    wrappers::{self, INPUTS, Inputs},
};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

fn environment(dir: &Path, app: &str) -> BTreeMap<String, String> {
    let goal = "Literal goal \n \"quoted\" 'single' ;$(touch NEVER) `touch NEVER` $HOME 🦀\t\u{007f}\u{0085}\n";
    let prompt = dir.join("prompt with spaces.md");
    std::fs::write(&prompt, goal).unwrap();
    [
        (INPUTS[0], "signup".into()),
        (INPUTS[1], app.into()),
        (INPUTS[2], "http://127.0.0.1:12345/mcp".into()),
        (INPUTS[3], "fixture-user@example.test".into()),
        (INPUTS[4], "inbox ;$(literal)".into()),
        (INPUTS[5], goal.into()),
        (INPUTS[6], prompt.to_str().unwrap().into()),
        (INPUTS[7], "600000".into()),
    ]
    .into_iter()
    .map(|(k, v)| (k.into(), v))
    .collect()
}
#[test]
fn exact_eight_input_admission_and_literal_values() {
    let dir = tempfile::tempdir().unwrap();
    let values = environment(dir.path(), "http://127.0.0.1:12346/");
    let inputs = Inputs::read(&values).unwrap();
    assert_eq!(
        inputs.goal.as_bytes(),
        std::fs::read(&inputs.prompt_file).unwrap()
    );
    assert_eq!(inputs.inbox, "inbox ;$(literal)");
    assert_eq!(inputs.deadline_ms, 600_000);
    assert_eq!(inputs.mcp.as_str(), "http://127.0.0.1:12345/mcp");
    for name in INPUTS {
        let mut invalid = values.clone();
        invalid.remove(name);
        assert!(Inputs::read(&invalid).is_err(), "missing {name}");
        invalid.insert(name.into(), String::new());
        assert!(Inputs::read(&invalid).is_err(), "empty {name}");
    }
    for (key, invalids) in [
        (INPUTS[0], vec!["unknown-task"]),
        (
            INPUTS[1],
            vec![
                "https://127.0.0.1/",
                "http://example.test/",
                "http://user:pass@127.0.0.1/",
                "http://127.0.0.1/?x=y",
                "http://127.0.0.1/#f",
                "http://127.0.0.1:0/",
                "http://127.0.0.1/else",
            ],
        ),
        (
            INPUTS[2],
            vec![
                "http://127.0.0.1/",
                "http://localhost:3/mcp",
                "http://127.0.0.1/mcp?x=y",
            ],
        ),
        (INPUTS[3], vec!["bad\r\nX-Other: override", "non-ascii-🦀"]),
        (INPUTS[4], vec!["bad\ninbox"]),
        (INPUTS[6], vec!["relative", "/absent/fixture-prompt.md"]),
        (
            INPUTS[7],
            vec![
                "0",
                "3600001",
                "-1",
                "+1",
                "1.5",
                " 1",
                "18446744073709551616",
            ],
        ),
    ] {
        for value in invalids {
            let mut v = values.clone();
            v.insert(key.into(), value.into());
            assert!(Inputs::read(&v).is_err(), "{key}={value}");
        }
    }
    assert!(wrappers::endpoint("http://[::1]:12345/", "/").is_ok());
    let mut v = values.clone();
    v.insert(INPUTS[5].into(), "mismatch".into());
    assert!(Inputs::read(&v).is_err());
    let link = dir.path().join("prompt-link");
    std::os::unix::fs::symlink(&inputs.prompt_file, &link).unwrap();
    v = values.clone();
    v.insert(INPUTS[6].into(), link.to_str().unwrap().into());
    assert!(Inputs::read(&v).is_err());
    assert!(!dir.path().join("NEVER").exists());
}

#[test]
fn invalid_entrypoints_never_echo_private_configuration_or_launch_a_cli() {
    let dir = tempfile::tempdir().unwrap();
    let values = environment(dir.path(), "http://127.0.0.1:12346/");
    let canary = "SYNTHETIC-CREDENTIAL-CANARY-DO-NOT-RETAIN";
    let config = dir.path().join("private.json");
    std::fs::write(
        &config,
        json!({"credential":canary,"executable":"/bin/false"}).to_string(),
    )
    .unwrap();
    let config_before = std::fs::read(&config).unwrap();
    for (command, model) in [
        ("agent-claude", "claude-opus-5-5"),
        ("agent-codex", "gpt-6.1-sol"),
        ("agent-dsh", "dashscope/glm-5.3"),
    ] {
        for model in [model, "unsupported ;$(touch NEVER) `echo secret`"] {
            let out = std::process::Command::new(common::exe())
                .env_clear()
                .envs(&values)
                .env("HOME", dir.path())
                .env("ANTHROPIC_API_KEY", canary)
                .env("OPENAI_API_KEY", canary)
                .current_dir(dir.path())
                .args([command, "--model", model, "--config"])
                .arg(&config)
                .output()
                .unwrap();
            assert_eq!(out.status.code(), Some(2));
            assert!(out.stdout.is_empty());
            let error = String::from_utf8(out.stderr).unwrap();
            assert!(!error.contains(canary));
            assert!(!error.contains(model));
            assert!(!error.contains("private.json"));
        }
    }
    assert_eq!(std::fs::read(&config).unwrap(), config_before);
    assert!(!dir.path().join("NEVER").exists());
    for (h, m) in [
        ("claude", "claude-sonnet-5-5"),
        ("codex", "gpt-6-luna"),
        ("dsh", "spark-glm/GLM-5.3-Flash-EXL3"),
    ] {
        assert!(wrappers::validate_model(h, m).is_ok());
    }
}

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    identity: String,
    headers: axum::http::HeaderMap,
    body: Vec<u8>,
}

async fn http_server() -> (
    String,
    tokio::sync::mpsc::UnboundedReceiver<HttpRequest>,
    tokio::task::JoinHandle<()>,
    Arc<AtomicUsize>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let count = Arc::new(AtomicUsize::new(0));
    let requests = count.clone();
    let router = Router::new().fallback(any(move |request: Request| {
        let tx = tx.clone();
        let requests = requests.clone();
        async move {
            requests.fetch_add(1, Ordering::SeqCst);
            let method = request.method().to_string();
            let path = request.uri().to_string();
            let identity = request
                .headers()
                .get("X-Eval-Identity")
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            let headers = request.headers().clone();
            let body = to_bytes(request.into_body(), 100_000).await.unwrap();
            tx.send(HttpRequest {
                method,
                path: path.clone(),
                identity,
                headers,
                body: body.to_vec(),
            })
            .unwrap();
            if path == "/oversized" {
                Response::new(Body::from(vec![b'x'; RESPONSE_LIMIT + 1]))
            } else if path == "/hang" {
                std::future::pending().await
            } else {
                Response::new(Body::from("{\"decoys\":[\"keep\"],\"ok\":false}"))
            }
        }
    }));
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (url, rx, task, count)
}

#[tokio::test]
async fn helper_makes_real_identity_bound_requests_and_refuses_overrides_escapes_and_overflow() {
    let (url, mut rx, server, count) = http_server().await;
    let dir = tempfile::tempdir().unwrap();
    let helper = Helper::new(Inputs::read(&environment(dir.path(), &url)).unwrap()).unwrap();
    for (method, path, body) in [
        ("GET", "/", None),
        ("GET", "/flow?token=abc", None),
        ("POST", "/confirm", Some(json!({"token":"literal"}))),
    ] {
        let mut args = json!({"method":method,"path":path});
        if let Some(ref body) = body {
            args["body"] = body.clone();
        }
        let result = helper.call(args).await.unwrap();
        assert_eq!(result["status"], 200);
        assert_eq!(result["body"], "{\"decoys\":[\"keep\"],\"ok\":false}");
        let request = rx.recv().await.unwrap();
        assert_eq!(
            (
                request.method.as_str(),
                request.path.as_str(),
                request.identity.as_str()
            ),
            (method, path, "fixture-user@example.test")
        );
        assert_eq!(
            request.body,
            body.map(|v| serde_json::to_vec(&v).unwrap())
                .unwrap_or_default()
        );
    }
    for args in [
        json!({"method":"DELETE","path":"/"}),
        json!({"method":"GET","path":"/","body":{}}),
        json!({"method":"GET","path":"/","headers":{"X-Eval-Identity":"foreign"}}),
        json!({"method":"GET","path":"/","url":url}),
        json!({"method":"POST","path":"/","body":"x".repeat(64*1024)}),
    ] {
        assert!(helper.call(args).await.is_err());
    }
    for path in [
        "https://evil.test/",
        "//127.0.0.1/",
        "/\\evil",
        "/%5cevil",
        "/%2f%2fevil/",
        "/#fragment",
        "/%23fragment",
        "/%252f%252fevil",
        "/%00",
        "relative",
        "/%zz",
        "/%",
        "/\n",
    ] {
        assert!(
            helper
                .call(json!({"method":"GET","path":path}))
                .await
                .is_err(),
            "{path}"
        );
    }
    assert_eq!(count.load(Ordering::SeqCst), 3);
    assert!(
        helper
            .call(json!({"method":"GET","path":"/oversized"}))
            .await
            .is_err()
    );
    rx.recv().await.unwrap();
    let mut inputs = Inputs::read(&environment(dir.path(), &url)).unwrap();
    inputs.deadline_ms = 2000;
    let limited = Helper::new(inputs).unwrap();
    assert_eq!(
        limited
            .call(json!({"method":"GET","path":"/hang"}))
            .await
            .unwrap_err(),
        "http_request_timeout"
    );
    rx.recv().await.unwrap();
    assert_eq!(
        limited
            .call(json!({"method":"GET","path":"/"}))
            .await
            .unwrap_err(),
        "http_deadline_expired"
    );
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn redirects_never_reach_a_second_listener() {
    let (other, _rx, other_server, requests) = http_server().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    let router = Router::new().fallback(any(move || {
        let other = other.clone();
        async move {
            Response::builder()
                .status(302)
                .header("location", other)
                .body(Body::empty())
                .unwrap()
        }
    }));
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let dir = tempfile::tempdir().unwrap();
    let helper = Helper::new(Inputs::read(&environment(dir.path(), &url)).unwrap()).unwrap();
    assert_eq!(
        helper
            .call(json!({"method":"GET","path":"/"}))
            .await
            .unwrap()["status"],
        302
    );
    assert_eq!(requests.load(Ordering::SeqCst), 0);
    server.abort();
    other_server.abort();
    let _ = server.await;
    let _ = other_server.await;
}

#[tokio::test]
async fn actual_stdio_helper_lists_exactly_one_tool_and_does_not_echo_unrelated_secrets() {
    let (url, mut requests, server, _) = http_server().await;
    let dir = tempfile::tempdir().unwrap();
    let values = environment(dir.path(), &url);
    let mut cmd = tokio::process::Command::new(common::exe());
    let runtime = runner::RuntimeDirs::create(dir.path(), "helper").unwrap();
    runtime.apply(&mut cmd);
    cmd.arg("fixture-http")
        .envs(values)
        .env("PWD", dir.path())
        .current_dir(dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = cmd.spawn().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = tokio::io::BufReader::new(child.stdout.take().unwrap()).lines();
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26"}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"eval_http","arguments":{"method":"GET","path":"/"}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"arbitrary_http","arguments":{"method":"GET","path":"/"}}}),
    ];
    for m in messages {
        stdin.write_all(format!("{m}\n").as_bytes()).await.unwrap();
    }
    drop(stdin);
    let mut replies = Vec::new();
    for _ in 0..4 {
        replies.push(
            serde_json::from_str::<Value>(
                &tokio::time::timeout(std::time::Duration::from_secs(5), stdout.next_line())
                    .await
                    .unwrap()
                    .unwrap()
                    .unwrap(),
            )
            .unwrap(),
        );
    }
    assert_eq!(replies[0]["result"]["protocolVersion"], "2025-03-26");
    let tools = replies[1]["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "eval_http");
    assert_eq!(replies[2]["result"]["isError"], false);
    assert!(replies[3].get("error").is_some());
    assert_eq!(
        requests.recv().await.unwrap().identity,
        "fixture-user@example.test"
    );
    assert!(child.wait().await.unwrap().success());
    server.abort();
    let _ = server.await;
}

// These exchanges exercise the executable's bounded stdin reader, not Helper::rpc.
// Drain both output pipes while writing so an early reader refusal cannot hang.
async fn stdio_bytes(url: &str, bytes: Vec<u8>) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    let runtime = runner::RuntimeDirs::create(dir.path(), "helper").unwrap();
    let mut command = tokio::process::Command::new(common::exe());
    runtime.apply(&mut command);
    command
        .arg("fixture-http")
        .envs(environment(dir.path(), url))
        .env("PWD", dir.path())
        .current_dir(dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command.spawn().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let write = tokio::spawn(async move { stdin.write_all(&bytes).await });
    let out = tokio::spawn(async move {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).await.unwrap();
        bytes
    });
    let err = tokio::spawn(async move {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).await.unwrap();
        bytes
    });
    let waited = tokio::time::timeout(std::time::Duration::from_secs(10), child.wait()).await;
    if waited.is_err() {
        child.kill().await.unwrap();
        child.wait().await.unwrap();
    }
    let (written, stdout, stderr) =
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            (write.await.unwrap(), out.await.unwrap(), err.await.unwrap())
        })
        .await
        .unwrap();
    let status = waited
        .expect("fixture helper did not exit after stdin EOF")
        .unwrap();
    if let Err(error) = written {
        assert!(!status.success());
        assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
    }
    std::process::Output {
        status,
        stdout,
        stderr,
    }
}

async fn stdio_exchange(url: &str, messages: &[Value]) -> std::process::Output {
    let mut bytes = Vec::new();
    for message in messages {
        serde_json::to_writer(&mut bytes, message).unwrap();
        bytes.push(b'\n');
    }
    stdio_bytes(url, bytes).await
}

async fn finish_http(
    mut requests: tokio::sync::mpsc::UnboundedReceiver<HttpRequest>,
    server: tokio::task::JoinHandle<()>,
    count: Arc<AtomicUsize>,
) -> Vec<HttpRequest> {
    // Helpers have exited and every HTTP exchange has completed. Join the
    // listener before inspecting the final count; no absence-by-sleep test.
    server.abort();
    assert!(server.await.unwrap_err().is_cancelled());
    let captured = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        let mut captured = Vec::new();
        while let Some(request) = requests.recv().await {
            captured.push(request);
        }
        captured
    })
    .await
    .unwrap();
    assert_eq!(captured.len(), count.load(Ordering::SeqCst));
    captured
}

const REVISIONS: [&str; 3] = ["2025-03-26", "2025-06-18", "2025-11-25"];
const META_SENTINEL: &str = "synthetic-request-metadata-do-not-forward-151";

fn initialize(version: &str) -> Value {
    json!({"jsonrpc":"2.0","id":"init","method":"initialize","params":{
        "protocolVersion":version,"capabilities":{},"clientInfo":{"name":"wire-test","version":"1"}
    }})
}

fn initialized(version: &str) -> Value {
    json!({"jsonrpc":"2.0","id":"init","result":{
        "protocolVersion":version,"capabilities":{"tools":{}},
        "serverInfo":{"name":"eval-fixture-http","version":"1"}
    }})
}

fn tool_call(id: usize, arguments: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{
        "name":"eval_http","arguments":arguments
    }})
}

fn tool_result(id: usize, text: &str, is_error: bool) -> Value {
    json!({"jsonrpc":"2.0","id":id,"result":{
        "content":[{"type":"text","text":text}],"isError":is_error
    }})
}

fn http_result(id: usize) -> Value {
    tool_result(
        id,
        &json!({
            "status":200,"body":"{\"decoys\":[\"keep\"],\"ok\":false}","encoding":"utf-8"
        })
        .to_string(),
        false,
    )
}

fn protocol_error(id: usize, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}

fn replies(output: &std::process::Output) -> Vec<Value> {
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = std::str::from_utf8(&output.stdout).unwrap();
    assert!(stdout.ends_with('\n'));
    stdout
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn assert_metadata_discarded(outputs: &[std::process::Output], requests: &[HttpRequest]) {
    for output in outputs {
        assert!(!String::from_utf8_lossy(&output.stdout).contains(META_SENTINEL));
        assert!(!String::from_utf8_lossy(&output.stderr).contains(META_SENTINEL));
    }
    for request in requests {
        assert!(!request.path.contains(META_SENTINEL));
        for (name, value) in &request.headers {
            assert!(!name.as_str().contains(META_SENTINEL));
            assert!(!String::from_utf8_lossy(value.as_bytes()).contains(META_SENTINEL));
        }
        assert!(!String::from_utf8_lossy(&request.body).contains(META_SENTINEL));
        assert_eq!(request.identity, "fixture-user@example.test");
    }
}

#[tokio::test]
async fn actual_stdio_helper_accepts_request_metadata() {
    let (url, requests, server, count) = http_server().await;
    let mut outputs = Vec::new();
    let mut expected = Vec::new();
    for version in REVISIONS {
        for meta in [
            None,
            Some(json!({})),
            Some(json!({"progressToken":7,"example.test/marker":META_SENTINEL})),
            Some(json!({"progressToken":META_SENTINEL})),
            Some(json!({"example.test/extension":{"nested":[META_SENTINEL,{"enabled":true}]}})),
            // Discard is opaque, without introducing a stricter metadata validator.
            Some(Value::Null),
            Some(json!([META_SENTINEL])),
            Some(json!(META_SENTINEL)),
            Some(json!(false)),
            Some(json!(3)),
        ] {
            let mut messages = vec![
                initialize(version),
                json!({"jsonrpc":"2.0","method":"notifications/initialized","params":{}}),
                json!({"jsonrpc":"2.0","id":2,"method":"ping","params":{}}),
                json!({"jsonrpc":"2.0","id":3,"method":"tools/list","params":{}}),
                tool_call(4, json!({"method":"GET","path":"/"})),
                tool_call(
                    5,
                    json!({"method":"POST","path":"/confirm","body":{
                        "_meta":{"literal":"legitimate-tool-data"},"token":"unchanged"
                    }}),
                ),
            ];
            if let Some(meta) = meta {
                for message in &mut messages {
                    message["params"]["_meta"] = meta.clone();
                }
            }
            outputs.push(stdio_exchange(&url, &messages).await);
            expected.push(vec![
                initialized(version),
                json!({"jsonrpc":"2.0","id":2,"result":{}}),
                json!({"jsonrpc":"2.0","id":3,"result":{"tools":[{
                    "name":"eval_http",
                    "description":"Request the public fixture app. Identity is supplied automatically, including GET /. Read GET / to discover the app's forms. Paths are app-relative; mail is read with Swarmail MCP.",
                    "inputSchema":{"type":"object","properties":{
                        "method":{"type":"string","enum":["GET","POST"]},"path":{"type":"string"},"body":{}
                    },"required":["method","path"],"additionalProperties":false}
                }]}}),
                http_result(4),
                http_result(5),
            ]);
        }
    }
    let requests = finish_http(requests, server, count).await;
    assert_metadata_discarded(&outputs, &requests);
    for (output, expected) in outputs.iter().zip(expected) {
        // Equality includes the entire stream: no initialized/progress replies.
        assert_eq!(replies(output), expected);
    }
    assert_eq!(requests.len(), outputs.len() * 2);
    for pair in requests.as_chunks::<2>().0 {
        assert_eq!(
            (&*pair[0].method, &*pair[0].path, &*pair[0].body),
            ("GET", "/", &b""[..])
        );
        assert_eq!((&*pair[1].method, &*pair[1].path), ("POST", "/confirm"));
        assert_eq!(pair[1].headers["content-type"], "application/json");
        assert_eq!(
            pair[1].body,
            serde_json::to_vec(&json!({
                "_meta":{"literal":"legitimate-tool-data"},"token":"unchanged"
            }))
            .unwrap()
        );
    }
}

#[tokio::test]
async fn actual_stdio_helper_preserves_closed_calls() {
    let (url, requests, server, count) = http_server().await;
    let mut outputs = Vec::new();
    for version in REVISIONS {
        let args = json!({"method":"GET","path":"/"});
        let mut messages = vec![initialize(version)];
        let mut expected = vec![initialized(version)];
        for params in [
            json!({"name":"eval_http","arguments":args,"unknown":META_SENTINEL}),
            json!({"name":"eval_http","arguments":{"method":"GET","path":"/","_meta":{}}}),
            json!({"name":"eval_http","arguments":{"method":"GET","path":"/","headers":{}}}),
            json!({"arguments":args}),
            json!({"name":"eval_http"}),
            json!({"name":"eval_http","arguments":{"path":"/"}}),
            json!({"name":"eval_http","arguments":{"method":"GET"}}),
            json!({"name":"eval_http","arguments":null}),
            json!({"name":"eval_http","arguments":[]}),
            json!({"name":"eval_http","arguments":"wrong-shape"}),
            json!({"name":"eval_http","arguments":{"method":7,"path":"/"}}),
            json!({"name":"eval_http","arguments":{"method":"GET","path":null}}),
            json!({"name":null,"arguments":args}),
            json!({"name":"unknown_tool","arguments":args}),
        ] {
            for meta in [None, Some(json!({"progressToken":META_SENTINEL}))] {
                let id = messages.len();
                let mut params = params.clone();
                if let Some(meta) = meta {
                    params["_meta"] = meta;
                }
                messages
                    .push(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":params}));
                expected.push(protocol_error(id, -32602, "tool_arguments_invalid"));
            }
        }
        for params in [Value::Null, json!([]), json!("wrong-shape")] {
            let id = messages.len();
            messages.push(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":params}));
            expected.push(protocol_error(id, -32602, "tool_arguments_invalid"));
        }
        let id = messages.len();
        messages.push(json!({"jsonrpc":"2.0","id":id,"method":"resources/list","params":{"_meta":{"progressToken":META_SENTINEL}}}));
        expected.push(protocol_error(id, -32601, "method_not_found"));
        let id = messages.len();
        messages.push(tool_call(id, json!({"method":"DELETE","path":"/"})));
        expected.push(tool_result(id, "fixture_request_refused", true));
        outputs.push((stdio_exchange(&url, &messages).await, expected));
    }
    let requests = finish_http(requests, server, count).await;
    assert!(requests.is_empty());
    for (output, expected) in outputs {
        assert_metadata_discarded(std::slice::from_ref(&output), &requests);
        assert_eq!(replies(&output), expected);
    }
}

#[tokio::test]
async fn actual_stdio_helper_names_get_body_refusal() {
    let (url, requests, server, count) = http_server().await;
    let mut outputs = Vec::new();
    let bodies = [
        json!(""),
        json!("nonempty"),
        Value::Null,
        json!({}),
        json!([]),
        json!(false),
        json!(true),
        json!(0),
        json!(1.5),
    ];
    for version in REVISIONS {
        let mut messages = vec![
            initialize(version),
            tool_call(1, json!({"method":"GET","path":"/"})),
        ];
        let mut expected = vec![initialized(version), http_result(1)];
        for body in &bodies {
            let id = messages.len();
            messages.push(tool_call(
                id,
                json!({"method":"GET","path":"/","body":body}),
            ));
            expected.push(tool_result(id, "http_get_body_not_supported", true));
        }
        for body in std::iter::once(None).chain(bodies.iter().cloned().map(Some)) {
            let id = messages.len();
            let mut arguments = json!({"method":"POST","path":"/confirm"});
            if let Some(body) = body {
                arguments["body"] = body;
            }
            messages.push(tool_call(id, arguments));
            expected.push(http_result(id));
        }
        let id = messages.len();
        messages.push(tool_call(
            id,
            json!({"method":"GET","path":"//invalid.example/"}),
        ));
        expected.push(tool_result(id, "fixture_request_refused", true));
        outputs.push((stdio_exchange(&url, &messages).await, expected));
    }
    let requests = finish_http(requests, server, count).await;
    for (output, expected) in &outputs {
        assert_eq!(replies(output), *expected);
    }
    assert_eq!(requests.len(), REVISIONS.len() * (bodies.len() + 2));
    for group in requests.chunks_exact(bodies.len() + 2) {
        assert_eq!(group[0].method, "GET");
        assert!(group[0].body.is_empty());
        assert!(!group[0].headers.contains_key("content-type"));
        for (request, body) in group[1..]
            .iter()
            .zip(std::iter::once(None).chain(bodies.iter().map(Some)))
        {
            assert_eq!(request.method, "POST");
            assert_eq!(request.path, "/confirm");
            assert_eq!(request.identity, "fixture-user@example.test");
            assert_eq!(
                request.body,
                body.map(|v| serde_json::to_vec(v).unwrap())
                    .unwrap_or_default()
            );
            assert_eq!(request.headers.contains_key("content-type"), body.is_some());
        }
    }
}

#[tokio::test]
async fn actual_stdio_helper_bounds_metadata() {
    let (url, requests, server, count) = http_server().await;
    let mut outputs = Vec::new();
    for version in REVISIONS {
        let initial = initialize(version);
        let mut bytes = serde_json::to_vec(&initial).unwrap();
        bytes.push(b'\n');
        let mut oversized = tool_call(2, json!({"method":"GET","path":"/"}));
        oversized["params"]["_meta"] = json!({
            "progressToken":META_SENTINEL,"example.test/padding":"x".repeat(swarmail_evals::fixture_http::INPUT_LIMIT)
        });
        serde_json::to_writer(&mut bytes, &oversized).unwrap();
        bytes.push(b'\n');
        outputs.push((version, stdio_bytes(&url, bytes).await));
    }
    let requests = finish_http(requests, server, count).await;
    assert!(requests.is_empty());
    for (version, output) in outputs {
        assert_metadata_discarded(std::slice::from_ref(&output), &requests);
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(
            output.stdout,
            format!("{}\n", initialized(version)).as_bytes()
        );
        assert_eq!(
            output.stderr,
            b"swarmail-evals: harness error: operation_failed\n"
        );
    }
}

#[path = "wrappers_launch/mod.rs"]
mod launch;
