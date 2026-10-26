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
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

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

async fn http_server() -> (
    String,
    tokio::sync::mpsc::UnboundedReceiver<(String, String, String, Vec<u8>)>,
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
            let body = to_bytes(request.into_body(), 100_000).await.unwrap();
            tx.send((method, path.clone(), identity, body.to_vec()))
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
        let (m, p, id, b) = rx.recv().await.unwrap();
        assert_eq!(
            (m.as_str(), p.as_str(), id.as_str()),
            (method, path, "fixture-user@example.test")
        );
        assert_eq!(
            b,
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
        requests.recv().await.unwrap().2,
        "fixture-user@example.test"
    );
    assert!(child.wait().await.unwrap().success());
    server.abort();
    let _ = server.await;
}

#[path = "wrappers_launch/mod.rs"]
mod launch;
