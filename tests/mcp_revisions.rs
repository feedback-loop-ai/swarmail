//! Revision-qualified contracts against real HTTP and binary stdio servers.
mod common;
use common::{http_send_text_with_headers, smtp_send, start};
use serde_json::{Value, json};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
const VERSIONS: [&str; 3] = ["2025-03-26", "2025-06-18", "2025-11-25"];
async fn post(addr: std::net::SocketAddr, headers: &[(&str, &str)], body: &Value) -> (u16, Value) {
    let (status, text) =
        http_send_text_with_headers(addr, "POST", "/mcp", headers, Some(&body.to_string())).await;
    (
        status,
        if text.is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&text).unwrap()
        },
    )
}
fn initialize(version: Value) -> Value {
    json!({"jsonrpc":"2.0", "id":"init", "method":"initialize", "params":{"protocolVersion":version,"capabilities":{},"clientInfo":{"name":"revision-test","version":"1"}}})
}
fn call(id: Value, name: &str, arguments: Value) -> Value {
    json!({"jsonrpc":"2.0","id": id, "method":"tools/call", "params":{"name":format!("swarmail_{name}"), "arguments":arguments}})
}
fn text(result: &Value) -> Value {
    serde_json::from_str(result["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}
struct Binary {
    child: tokio::process::Child,
    input: tokio::process::ChildStdin,
    output: BufReader<tokio::process::ChildStdout>,
}
impl Binary {
    fn start(addr: std::net::SocketAddr) -> Self {
        let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_swarmail"))
            .args(["mcp", "--url", &format!("http://{addr}")])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            input,
            output,
        }
    }
    async fn send(&mut self, request: &Value) {
        self.input
            .write_all(format!("{request}\n").as_bytes())
            .await
            .unwrap();
    }
    async fn reply(&mut self) -> Value {
        let mut line = String::new();
        let n = tokio::time::timeout(Duration::from_secs(30), self.output.read_line(&mut line))
            .await
            .unwrap()
            .unwrap();
        assert!(n > 0, "binary closed stdout");
        serde_json::from_str(&line).unwrap()
    }
    async fn ready(&mut self, version: &str) {
        self.send(&initialize(json!(version))).await;
        assert_eq!(self.reply().await["result"]["protocolVersion"], version);
        self.send(&json!({"method":"notifications/initialized"}))
            .await;
        self.send(&json!({"id":"ready", "method":"ping"})).await;
        assert_eq!(
            self.reply().await["id"],
            "ready",
            "initialized has no stdout reply"
        );
    }
    async fn finish(mut self) {
        drop(self.input);
        let mut rest = String::new();
        self.output.read_to_string(&mut rest).await.unwrap();
        assert!(rest.is_empty(), "unexpected protocol lines: {rest}");
        assert!(
            tokio::time::timeout(Duration::from_secs(30), self.child.wait())
                .await
                .unwrap()
                .unwrap()
                .success()
        );
        let mut stderr = String::new();
        self.child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .await
            .unwrap();
        assert!(stderr.is_empty(), "{stderr}");
    }
}
#[tokio::test]
async fn revision_negotiation_matrix() {
    let srv = start().await;
    for (proposal, expected) in VERSIONS.into_iter().map(|v| (v, v)).chain([
        ("", VERSIONS[2]),
        ("2099-01-01", VERSIONS[2]),
        ("2026-07-28", VERSIONS[2]),
    ]) {
        let request = initialize(json!(proposal));
        let (status, reply) = post(srv.http_addr, &[], &request).await;
        assert_eq!(status, 200);
        assert_eq!(reply["id"], "init");
        assert_eq!(reply["result"]["protocolVersion"], expected);
        assert_eq!(reply["jsonrpc"], "2.0");
        assert!(reply.get("error").is_none());
        assert_eq!(reply["result"]["capabilities"], json!({"tools":{}}));
        assert_eq!(
            reply["result"]["serverInfo"],
            json!({"name":"swarmail","version":env!("CARGO_PKG_VERSION")})
        );
        let mut binary = Binary::start(srv.http_addr);
        binary.send(&request).await;
        assert_eq!(binary.reply().await, reply);
        binary.finish().await;
    }
    let absent = json!({"id":null, "method":"initialize"});
    assert_eq!(
        post(srv.http_addr, &[], &absent).await.1["result"]["protocolVersion"],
        VERSIONS[0]
    );
    let mut binary = Binary::start(srv.http_addr);
    binary.send(&absent).await;
    assert_eq!(
        binary.reply().await,
        post(srv.http_addr, &[], &absent).await.1
    );
    binary.finish().await;
    for invalid in [Value::Null, json!(false), json!(42), json!([]), json!({})] {
        let request = initialize(invalid);
        let (status, reply) = post(srv.http_addr, &[], &request).await;
        assert_eq!(status, 200);
        assert_eq!(reply["id"], "init");
        assert_eq!(reply["error"]["code"], -32602);
        assert_eq!(
            reply["error"]["message"],
            "protocolVersion must be a string"
        );
        let mut binary = Binary::start(srv.http_addr);
        binary.send(&request).await;
        assert_eq!(binary.reply().await, reply);
        binary.finish().await;
    }
    let batch = json!([initialize(json!(VERSIONS[2])), {"id":7,"method":"ping"}]);
    let (_, reply) = post(srv.http_addr, &[], &batch).await;
    assert_eq!(reply[0]["result"]["protocolVersion"], VERSIONS[0]);
    let mut binary = Binary::start(srv.http_addr);
    binary.send(&batch).await;
    assert_eq!(binary.reply().await, reply);
    binary.send(&json!([{"id":8,"method":"ping"}])).await;
    assert!(
        binary.reply().await.is_array(),
        "batched initialize does not transition"
    );
    binary.finish().await;
}
#[tokio::test]
async fn revision_headers_and_isolation() {
    let srv = start().await;
    for version in VERSIONS.into_iter().cycle().take(9) {
        let headers = [("mCp-PrOtOcOl-VeRsIoN", version)];
        assert_eq!(
            post(srv.http_addr, &headers, &json!({"id":2,"method":"ping"})).await,
            (200, json!({"jsonrpc":"2.0","id":2,"result":{}}))
        );
        assert_eq!(
            post(srv.http_addr, &headers, &initialize(json!(VERSIONS[1])))
                .await
                .1["result"]["protocolVersion"],
            VERSIONS[1]
        );
        assert_eq!(
            post(srv.http_addr, &headers, &initialize(json!("future")))
                .await
                .1["result"]["protocolVersion"],
            VERSIONS[2]
        );
        assert_eq!(
            post(
                srv.http_addr,
                &headers,
                &json!({"method":"notifications/initialized"})
            )
            .await,
            (202, Value::Null)
        );
    }
    for headers in [
        vec![("MCP-Protocol-Version", "")],
        vec![("MCP-Protocol-Version", "ÿ")],
        vec![("MCP-Protocol-Version", "2024-11-05")],
        vec![("MCP-Protocol-Version", "2026-07-28")],
        vec![("MCP-Protocol-Version", "2025-06-18, 2025-11-25")],
        vec![
            ("MCP-Protocol-Version", VERSIONS[1]),
            ("MCP-Protocol-Version", VERSIONS[1]),
        ],
        vec![
            ("MCP-Protocol-Version", VERSIONS[1]),
            ("MCP-Protocol-Version", VERSIONS[2]),
        ],
    ] {
        for request in [
            json!({"method":"notifications/initialized"}),
            json!([]),
            initialize(json!(VERSIONS[0])),
        ] {
            let (status, reply) = post(srv.http_addr, &headers, &request).await;
            assert_eq!(status, 400);
            assert_eq!(reply["error"]["code"], -32600);
            assert_eq!(reply["id"], Value::Null);
            assert!(
                reply["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("MCP-Protocol-Version")
            );
        }
        let (status, reply) =
            http_send_text_with_headers(srv.http_addr, "POST", "/mcp", &headers, Some("{")).await;
        assert_eq!(status, 400);
        assert_eq!(
            serde_json::from_str::<Value>(&reply).unwrap()["error"]["code"],
            -32700
        );
        let oversized = " ".repeat(2_097_152 + 1);
        let (status, reply) =
            http_send_text_with_headers(srv.http_addr, "POST", "/mcp", &headers, Some(&oversized))
                .await;
        assert_eq!(status, 413);
        assert_eq!(
            serde_json::from_str::<Value>(&reply).unwrap()["error"]["code"],
            -32600
        );
    }
    assert_eq!(
        post(
            srv.http_addr,
            &[("MCP-Protocol-Version", " \t2025-06-18 \t")],
            &json!({"id":1,"method":"ping"})
        )
        .await
        .0,
        200
    );
    let mut legacy = Binary::start(srv.http_addr);
    let mut modern = Binary::start(srv.http_addr);
    modern.ready(VERSIONS[2]).await;
    legacy.send(&json!([{"id":1,"method":"ping"}])).await;
    assert!(legacy.reply().await.is_array());
    modern.send(&json!([{"method":"ping"}])).await;
    assert_eq!(
        modern.reply().await["error"]["message"],
        "MCP 2025-11-25 does not support batches"
    );
    legacy.finish().await;
    modern.finish().await;
}
/// The current catalogue in served order, written out here rather than
/// copied from the production allowlist, so a new tool or a misclassified
/// hint fails instead of being blessed: nine reads (the deliverability
/// report's cache fill never changes stored mail or extraction, 0085; the
/// two focused waits appended by 0094) and six mutations.
const CATALOGUE: [(&str, bool); 15] = [
    ("swarmail_search_emails", true),
    ("swarmail_get_deliverability_report", true),
    ("swarmail_get_email", true),
    ("swarmail_get_latest_email", true),
    ("swarmail_delete_email", false),
    ("swarmail_clear_inbox", false),
    ("swarmail_clear_all", false),
    ("swarmail_wait_for_email", true),
    ("swarmail_extract_links", true),
    ("swarmail_extract_codes", true),
    ("swarmail_seed_email", false),
    ("swarmail_set_chaos", false),
    ("swarmail_clear_chaos", false),
    ("swarmail_wait_for_code", true),
    ("swarmail_wait_for_link", true),
];
#[tokio::test]
async fn revision_tool_annotations() {
    let srv = start().await;
    let request = json!({"id":1,"method":"tools/list"});
    let (_, legacy) = post(srv.http_addr, &[], &request).await;
    // The exact catalogue and its modern readOnlyHint, in served order.
    let table = CATALOGUE;
    assert_eq!(table.iter().filter(|(_, read)| *read).count(), 9);
    for version in VERSIONS {
        let (_, reply) = post(
            srv.http_addr,
            &[("MCP-Protocol-Version", version)],
            &request,
        )
        .await;
        let tools = reply["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(names, table.map(|(name, _)| name));
        for ((actual, original), (_, read_only)) in tools
            .iter()
            .zip(legacy["result"]["tools"].as_array().unwrap())
            .zip(table)
        {
            let mut descriptor = actual.clone();
            for absent in ["execution", "icons", "title"] {
                assert!(descriptor.get(absent).is_none(), "unexpected {absent}");
            }
            if version == VERSIONS[0] {
                assert!(descriptor.get("outputSchema").is_none());
                assert!(descriptor.get("annotations").is_none());
            } else {
                let schema = descriptor
                    .as_object_mut()
                    .unwrap()
                    .remove("outputSchema")
                    .unwrap();
                assert_eq!(schema["type"], "object", "{}", actual["name"]);
                assert!(schema["properties"].is_object() || schema["oneOf"].is_array());
                let hint = descriptor
                    .as_object_mut()
                    .unwrap()
                    .remove("annotations")
                    .unwrap();
                assert_eq!(
                    hint,
                    if read_only {
                        json!({"readOnlyHint":true})
                    } else {
                        json!({"readOnlyHint":false,"destructiveHint":true})
                    },
                    "{version} {}",
                    actual["name"]
                );
            }
            if version != VERSIONS[0] && actual["name"] == "swarmail_wait_for_email" {
                let description = descriptor["description"].as_str().unwrap();
                assert!(
                    !description.contains("batch"),
                    "modern wait promises batching: {description}"
                );
                descriptor["description"] = original["description"].clone();
            }
            if version != VERSIONS[0] && actual["name"] == "swarmail_get_deliverability_report" {
                let description = descriptor["description"].as_str().unwrap();
                assert!(
                    !description.contains("Only the synthetic demo rule ships"),
                    "modern report repeats the stale rule scope: {description}"
                );
                assert!(description.contains("bulk-sender checklist"));
                descriptor["description"] = original["description"].clone();
            }
            assert_eq!(&descriptor, original);
        }
        // The spawned bridge lists exactly what its negotiated revision serves.
        let mut binary = Binary::start(srv.http_addr);
        binary.ready(version).await;
        binary.send(&request).await;
        assert_eq!(binary.reply().await, reply, "{version} binary tools/list");
        binary.finish().await;
    }
}
fn batches() -> Vec<Value> {
    let mut note = call(
        json!(1),
        "seed_email",
        json!({"inbox":"revision", "to":"a@x.io"}),
    );
    note.as_object_mut().unwrap().remove("id");
    let members = vec![
        note.clone(),
        call(json!(2), "delete_email", json!({"id":"missing"})),
        call(json!(3), "clear_inbox", json!({"inbox":"revision"})),
        call(json!(4), "clear_all", json!({})),
        call(
            json!(5),
            "set_chaos",
            json!({"event":"data","probability":100}),
        ),
        call(json!(6), "clear_chaos", json!({})),
        call(
            json!(7),
            "wait_for_email",
            json!({"inbox":"unpopulated","timeout_ms":0}),
        ),
    ];
    vec![
        json!([]),
        json!([note.clone()]),
        json!(members),
        json!([[note.clone()]]),
        Value::Array(vec![note.clone(); 16]),
        Value::Array(vec![note; 17]),
    ]
}
#[tokio::test]
async fn modern_batches_refuse_before_effects() {
    let srv = start().await;
    smtp_send(
        srv.smtp_addr,
        Some("revision"),
        "a@x.io",
        "b@x.io",
        "preserved",
        "424242 https://x.io",
    )
    .await
    .unwrap();
    let read = call(json!(9), "search_emails", json!({"inbox":"revision"}));
    let before = post(srv.http_addr, &[], &read).await.1;
    let chaos_before = common::http_json(srv.http_addr, "GET", "/api/v1/chaos", None)
        .await
        .1;
    for version in &VERSIONS[1..] {
        let mut binary = Binary::start(srv.http_addr);
        binary.ready(version).await;
        for array in batches() {
            let expected = json!({"jsonrpc":"2.0","id":null,"error":{"code":-32600,"message":format!("MCP {version} does not support batches")}});
            assert_eq!(
                post(srv.http_addr, &[("MCP-Protocol-Version", version)], &array).await,
                (200, expected.clone())
            );
            binary.send(&array).await;
            assert_eq!(binary.reply().await, expected);
            assert_eq!(post(srv.http_addr, &[], &read).await.1, before);
            assert_eq!(
                common::http_json(srv.http_addr, "GET", "/api/v1/chaos", None)
                    .await
                    .1,
                chaos_before
            );
        }
        let oversized = json!([{"method":"tools/call","pad":"x".repeat(2_097_152)}]);
        assert_eq!(
            post(
                srv.http_addr,
                &[("MCP-Protocol-Version", version)],
                &oversized
            )
            .await
            .0,
            413
        );
        binary.send(&oversized).await;
        let reply = binary.reply().await;
        assert_eq!(reply["id"], Value::Null);
        assert_eq!(reply["error"]["code"], -32600);
        assert!(
            reply["error"]["message"]
                .as_str()
                .unwrap()
                .contains("request body refused")
        );
        binary.finish().await;
    }
    assert_eq!(post(srv.http_addr, &[], &batches()[1]).await.0, 202);
    assert_eq!(text(&post(srv.http_addr, &[], &read).await.1)["total"], 2);
}
#[tokio::test]
async fn revision_rpc_edges() {
    let srv = start().await;
    for version in VERSIONS {
        let headers = [("MCP-Protocol-Version", version)];
        for id in [json!(true), json!({}), json!([])] {
            let (status, reply) =
                post(srv.http_addr, &headers, &call(id, "clear_all", json!({}))).await;
            assert_eq!(status, 200);
            assert_eq!(reply["id"], Value::Null);
            assert_eq!(reply["error"]["code"], -32600);
        }
        for id in [json!("s"), json!(-1.5), Value::Null] {
            for (request, code) in [
                (json!({"id":id,"method":"unknown"}), -32601),
                (call(id.clone(), "unknown", json!({})), -32601),
                (
                    call(id.clone(), "search_emails", json!({"limit":null})),
                    -32602,
                ),
            ] {
                let (_, reply) = post(srv.http_addr, &headers, &request).await;
                assert_eq!(reply["id"], id);
                assert_eq!(reply["error"]["code"], code);
            }
        }
        let (_, reply) = post(
            srv.http_addr,
            &headers,
            &call(json!(1), "wait_for_email", json!({"timeout_ms":-1})),
        )
        .await;
        assert_eq!(reply["result"]["isError"], true);
        assert!(reply["result"].get("structuredContent").is_none());
        let (_, reply) = post(
            srv.http_addr,
            &headers,
            &call(json!(1), "wait_for_email", json!({"timeout_ms":0})),
        )
        .await;
        assert_eq!(reply["result"]["isError"], false);
    }
}
#[tokio::test]
async fn revision_smtp_visibility() {
    let srv = start().await;
    for version in VERSIONS {
        let reply = smtp_send(
            srv.smtp_addr,
            Some(version),
            "a@x.io",
            "b@x.io",
            version,
            "424242 https://x.io",
        )
        .await
        .unwrap();
        assert!(reply.starts_with("250"));
        let headers = [("MCP-Protocol-Version", version)];
        let (_, result) = post(
            srv.http_addr,
            &headers,
            &call(json!(1), "search_emails", json!({"inbox":version})),
        )
        .await;
        let data = text(&result);
        assert_eq!(data["total"], 1);
        let (_, got) = post(
            srv.http_addr,
            &headers,
            &call(json!(2), "get_email", json!({"id":data["emails"][0]["id"]})),
        )
        .await;
        assert_eq!(text(&got), data["emails"][0]);
    }
}
#[tokio::test]
async fn bridge_revision_lifecycle() {
    let srv = start().await;
    for version in &VERSIONS[1..] {
        let mut binary = Binary::start(srv.http_addr);
        binary.send(&initialize(json!(version))).await;
        binary
            .send(&call(
                json!(2),
                "seed_email",
                json!({"inbox":version,"to":"a@x.io"}),
            ))
            .await;
        binary
            .send(&call(Value::Null, "clear_all", json!({})))
            .await;
        binary.send(&json!({"id":3,"method":"ping"})).await;
        assert_eq!(binary.reply().await["result"]["protocolVersion"], *version);
        let replies = [
            binary.reply().await,
            binary.reply().await,
            binary.reply().await,
        ];
        assert_eq!(
            replies
                .iter()
                .filter(|r| r["error"]["code"] == -32600)
                .count(),
            2
        );
        assert!(
            replies
                .iter()
                .any(|r| r["id"] == 3 && r["result"] == json!({}))
        );
        let read = call(json!(7), "search_emails", json!({"inbox":version}));
        assert_eq!(text(&post(srv.http_addr, &[], &read).await.1)["total"], 0);
        binary
            .send(&json!({"id":4,"method":"notifications/initialized"}))
            .await;
        assert_eq!(binary.reply().await["error"]["code"], -32601);
        binary
            .send(&json!({"method":"notifications/initialized"}))
            .await;
        binary
            .send(&json!({"method":"notifications/initialized"}))
            .await;
        binary
            .send(&call(
                json!(5),
                "seed_email",
                json!({"inbox":version,"to":"a@x.io"}),
            ))
            .await;
        assert_eq!(binary.reply().await["id"], 5);
        binary.send(&initialize(json!(VERSIONS[0]))).await;
        assert_eq!(binary.reply().await["error"]["code"], -32600);
        binary
            .send(&json!({"method":"initialize", "params":{"protocolVersion":VERSIONS[0]}}))
            .await;
        binary.send(&json!({"id":6,"method":"ping"})).await;
        assert_eq!(binary.reply().await["id"], 6);
        binary.finish().await;
    }
}
#[tokio::test]
async fn bridge_modern_wait_allows_seed() {
    let srv = start().await;
    for version in &VERSIONS[1..] {
        let mut binary = Binary::start(srv.http_addr);
        binary.ready(version).await;
        binary
            .send(&call(
                json!(1),
                "wait_for_email",
                json!({"inbox":version, "timeout_ms":60000}),
            ))
            .await;
        common::wait_for_metric_row(
            srv.http_addr,
            &format!("swarmail_awaits_active{{inbox=\"{version}\"}}"),
            1,
        )
        .await;
        binary
            .send(&call(
                json!(2),
                "seed_email",
                json!({"inbox":version,"to":"a@x.io"}),
            ))
            .await;
        let replies = [binary.reply().await, binary.reply().await];
        let wait = replies.iter().find(|r| r["id"] == 1).unwrap();
        assert_eq!(text(wait)["matched"], 1);
        binary.finish().await;
    }
}

/// The method plane's header admission. Accepted 0043 §4 keeps a
/// guard-admitted OPTIONS at 405/Allow: POST with no Access-Control-*,
/// whatever MCP-Protocol-Version carries; that leaves the June/November
/// header-400 MUST unmet on OPTIONS alone (recorded beside C1–C6, not in it).
/// Every other method refuses an invalid header 400 and a hostile Origin is
/// refused 403 before either.
#[tokio::test]
async fn revision_transport_methods() {
    fn assert_method_not_allowed(response: &reqwest::Response, context: &str) {
        assert_eq!(response.status(), 405, "{context}");
        assert_eq!(response.headers()["allow"], "POST", "{context}");
        assert!(
            response
                .headers()
                .keys()
                .all(|name| !name.as_str().starts_with("access-control-")),
            "no CORS grant: {context}"
        );
    }
    let srv = start().await;
    let client = reqwest::Client::new();
    for method in [
        reqwest::Method::GET,
        reqwest::Method::PUT,
        reqwest::Method::DELETE,
        reqwest::Method::PATCH,
        reqwest::Method::OPTIONS,
        reqwest::Method::HEAD,
    ] {
        let options = method == reqwest::Method::OPTIONS;
        for version in [
            None,
            Some(VERSIONS[0]),
            Some(VERSIONS[1]),
            Some(VERSIONS[2]),
            Some("2026-07-28"),
            Some("bad"),
        ] {
            let mut request =
                client.request(method.clone(), format!("http://{}/mcp", srv.http_addr));
            if let Some(version) = version {
                request = request.header("MCP-Protocol-Version", version);
            }
            let response = request.send().await.unwrap();
            let context = format!("{method} {version:?}");
            if options || !matches!(version, Some("bad" | "2026-07-28")) {
                assert_method_not_allowed(&response, &context);
                continue;
            }
            assert_eq!(response.status(), 400, "{context}");
            assert_eq!(response.headers()["content-type"], "application/json");
            if method != reqwest::Method::HEAD {
                let reply: Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
                assert_eq!(reply["id"], Value::Null);
                assert_eq!(reply["error"]["code"], -32600);
            }
        }
        for values in [
            vec![reqwest::header::HeaderValue::from_static("")],
            vec![reqwest::header::HeaderValue::from_static(
                "2025-06-18, 2025-11-25",
            )],
            vec![reqwest::header::HeaderValue::from_static("2025-06-18"); 2],
            vec![reqwest::header::HeaderValue::from_bytes(b"\xff").unwrap()],
        ] {
            let context = format!("{method} invalid header shape {values:?}");
            let mut request =
                client.request(method.clone(), format!("http://{}/mcp", srv.http_addr));
            for value in values {
                request = request.header("MCP-Protocol-Version", value);
            }
            let response = request.send().await.unwrap();
            if options {
                assert_method_not_allowed(&response, &context);
            } else {
                assert_eq!(response.status(), 400, "{context}");
            }
        }
        for version in [None, Some(VERSIONS[2]), Some("bad")] {
            let mut request = client
                .request(method.clone(), format!("http://{}/mcp", srv.http_addr))
                .header("Origin", "https://foreign.example");
            if let Some(version) = version {
                request = request.header("MCP-Protocol-Version", version);
            }
            let response = request.send().await.unwrap();
            assert_eq!(
                response.status(),
                403,
                "{method} {version:?} Origin precedence"
            );
            assert!(
                response
                    .headers()
                    .keys()
                    .all(|name| !name.as_str().starts_with("access-control-")),
                "{method} {version:?}: no CORS grant on refusal"
            );
        }
    }
}

#[tokio::test]
async fn revision_response_disposition() {
    let srv = start().await;
    let client = reqwest::Client::new();
    for version in VERSIONS {
        for body in [
            json!({"jsonrpc":"2.0","id":7,"result":{}}),
            json!({"jsonrpc":"2.0","id":7,"error":{"code":-32603,"message":"peer error"}}),
        ] {
            let response = client
                .post(format!("http://{}/mcp", srv.http_addr))
                .header("MCP-Protocol-Version", version)
                .body(body.to_string())
                .send()
                .await
                .unwrap();
            // C1: the operator retains 200/-32600 as a documented deviation.
            // 0029/0044/0047 pin envelopes/classification, not this HTTP 200;
            // the 2026-10-26 ruling commissions no transport repair.
            assert_eq!(response.status(), 200);
            assert_eq!(response.headers()["content-type"], "application/json");
            assert_eq!(
                serde_json::from_str::<Value>(&response.text().await.unwrap()).unwrap(),
                json!({"jsonrpc":"2.0","id":7,"error":{"code":-32600,"message":"missing method"}})
            );
        }
        let response = client
            .post(format!("http://{}/mcp", srv.http_addr))
            .header("MCP-Protocol-Version", version)
            .body(json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 202);
        assert!(response.bytes().await.unwrap().is_empty());
    }
    // March response-only and mixed batches retain per-member missing-method errors.
    let (_, response_only) = post(
        srv.http_addr,
        &[],
        &json!([
            {"jsonrpc":"2.0","id":7,"result":{}},
            {"jsonrpc":"2.0","id":8,"error":{"code":-32603,"message":"peer error"}}
        ]),
    )
    .await;
    assert_eq!(response_only.as_array().unwrap().len(), 2);
    for (reply, id) in response_only.as_array().unwrap().iter().zip([7, 8]) {
        assert_eq!(
            reply,
            &json!({"jsonrpc":"2.0","id":id,"error":{"code":-32600,"message":"missing method"}})
        );
    }
    let (_, reply) = post(
        srv.http_addr,
        &[],
        &json!([{"jsonrpc":"2.0","id":7,"result":{}}, {"jsonrpc":"2.0","id":8,"method":"ping"}]),
    )
    .await;
    assert_eq!(reply[0]["error"]["code"], -32600);
    assert_eq!(reply[1]["result"], json!({}));
}

#[tokio::test]
async fn revision_served_contract() {
    let srv = start().await;
    let (_, api) = common::http_json(srv.http_addr, "GET", "/openapi.json", None).await;
    let endpoint = &api["paths"]["/mcp"];
    let description = endpoint["post"]["description"].as_str().unwrap();
    let (status, llms) = common::http_get_text(srv.http_addr, "/llms.txt").await;
    assert_eq!(status, 200);
    let llms = llms.split_once("\n## MCP\n").unwrap().1;
    let llms = llms
        .split_once("\n## ")
        .map_or(llms, |(section, _)| section);
    for clause in [
        "2025-03-26 only",
        "2025-06-18",
        "2025-11-25",
        "MCP-Protocol-Version",
        "outputSchema",
        "structuredContent",
        "10 MiB",
        "tokens-per-success",
        "version negotiation",
        "full revision conformance is not claimed",
        "operator-chosen C1-C6 deviations",
        "specs/mcp-version-negotiation/reconciliation.md#exact-conflict-register",
        "docs/decisions/0104-mcp-candidate-proof.md",
        "except OPTIONS",
    ] {
        assert!(
            description.contains(clause),
            "missing {clause}: {description}"
        );
    }
    for clause in [
        "Revisions: 2025-03-26, 2025-06-18, 2025-11-25.",
        "Only 2025-03-26 supports batches",
        "except\nOPTIONS",
        "the nine reads",
        "the six mutations",
        "outputSchema",
        "structuredContent",
        "10 MiB",
        "Tokens-per-success",
        "full revision conformance is not claimed",
        "specs/mcp-version-negotiation/reconciliation.md#exact-conflict-register",
        "docs/decisions/0104-mcp-candidate-proof.md",
    ] {
        assert!(llms.contains(clause), "llms.txt missing {clause}");
    }
    // The served reference names exactly the current catalogue.
    let named: std::collections::BTreeSet<&str> = llms
        .split('`')
        .filter(|token| token.starts_with("swarmail_"))
        .collect();
    assert_eq!(
        named,
        CATALOGUE.iter().map(|(name, _)| *name).collect(),
        "llms.txt catalogue"
    );
    assert!(
        endpoint["post"]["responses"]["400"]["description"]
            .as_str()
            .unwrap()
            .contains("-32600")
    );
    assert!(endpoint["get"]["responses"].get("405").is_some());
    let options = &endpoint["options"];
    assert!(options["responses"].get("400").is_none());
    assert!(options["responses"].get("405").is_some());
    assert!(options["description"].as_str().unwrap().contains("0043 §4"));
}

/// These fixed descriptors use only this explicitly checked 2020-12 vocabulary.
/// This checks our published schemas, not arbitrary remote JSON Schemas.
#[tokio::test]
async fn revision_tool_schemas_and_text_safety() {
    let srv = start().await;
    let payload = "<script>alert('mail')</script>\n\u{1b}[31m \"}]}\n{\"jsonrpc\":\"2.0\"}";
    for version in VERSIONS {
        let headers = [("MCP-Protocol-Version", version)];
        let (_, list) = post(
            srv.http_addr,
            &headers,
            &json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
        )
        .await;
        let tools = list["result"]["tools"].as_array().unwrap();
        let mut names = std::collections::HashSet::new();
        for tool in tools {
            let name = tool["name"].as_str().unwrap();
            assert!(names.insert(name));
            assert!((1..=128).contains(&name.len()));
            assert!(
                name.bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
            );
            let schema = &tool["inputSchema"];
            assert_eq!(schema["type"], "object");
            assert_eq!(schema["additionalProperties"], false);
            assert!(schema.get("$schema").is_none(), "default 2020-12 dialect");
            for key in schema.as_object().unwrap().keys() {
                assert!(
                    ["type", "properties", "required", "additionalProperties"]
                        .contains(&key.as_str())
                );
            }
            if let Some(required) = schema["required"].as_array() {
                for key in required {
                    assert!(schema["properties"].get(key.as_str().unwrap()).is_some());
                }
            }
            if let Some(properties) = schema["properties"].as_object() {
                for property in properties.values() {
                    let kind = property["type"].as_str().unwrap();
                    assert!(["string", "integer"].contains(&kind));
                    for key in property.as_object().unwrap().keys() {
                        assert!(
                            [
                                "type",
                                "description",
                                "enum",
                                "minimum",
                                "maximum",
                                "default"
                            ]
                            .contains(&key.as_str())
                        );
                    }
                    if let Some(variants) = property["enum"].as_array() {
                        assert_eq!(kind, "string");
                        assert!(variants.iter().all(Value::is_string));
                    }
                    for bound in ["minimum", "maximum", "default"] {
                        if let Some(value) = property.get(bound) {
                            assert_eq!(kind, "integer");
                            assert!(value.is_i64());
                        }
                    }
                }
            }
            let (_, refusal) = post(
                srv.http_addr,
                &headers,
                &call(
                    json!(2),
                    name.strip_prefix("swarmail_").unwrap(),
                    json!({"$schema":"https://unsupported.example/dialect"}),
                ),
            )
            .await;
            assert_eq!(
                refusal["error"]["code"], -32602,
                "schemas are not tool inputs"
            );
        }
        let (_, seed) = post(
            srv.http_addr,
            &headers,
            &call(
                json!(3),
                "seed_email",
                json!({"inbox":version,"to":"a@x.io","html":payload}),
            ),
        )
        .await;
        let id = text(&seed)["id"].clone();
        let (_, result) = post(
            srv.http_addr,
            &headers,
            &call(json!(4), "get_email", json!({"id":id})),
        )
        .await;
        assert_eq!(result["result"]["isError"], false);
        assert!(result.get("error").is_none());
        let content = result["result"]["content"].as_array().unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0].as_object().unwrap().len(), 2);
        let rendered = content[0]["text"].as_str().unwrap();
        assert!(!rendered.contains('\n'));
        assert!(!rendered.contains('\u{1b}'));
        let data = text(&result);
        assert_eq!(
            data["html"],
            format!("{payload}\r\n"),
            "seed MIME body remains string data"
        );
        assert!(data.get("raw").is_none());
        if version == VERSIONS[0] {
            assert!(result["result"].get("structuredContent").is_none());
        } else {
            assert_eq!(result["result"]["structuredContent"], text(&result));
        }
    }
}

/// Decision 0114 (proposed): issue #158's resources, subscriptions and
/// 2025-11-25 tasks are evaluated and not shipped. The refusal is pinned by
/// method name, on every revision, over real HTTP and the binary bridge, so
/// the record's claim is something a reader can run: capabilities stay exactly
/// `{"tools":{}}` and none of these methods is ever answered as a feature.
#[tokio::test]
async fn resources_and_tasks_methods_are_refused_by_name() {
    const METHODS: [&str; 9] = [
        "resources/list",
        "resources/read",
        "resources/templates/list",
        "resources/subscribe",
        "resources/unsubscribe",
        "tasks/get",
        "tasks/list",
        "tasks/result",
        "tasks/cancel",
    ];
    let srv = start().await;
    for version in VERSIONS {
        let headers = [("MCP-Protocol-Version", version)];
        let (_, init) = post(srv.http_addr, &[], &initialize(json!(version))).await;
        assert_eq!(init["result"]["protocolVersion"], version);
        assert_eq!(init["result"]["capabilities"], json!({"tools":{}}));
        let mut binary = Binary::start(srv.http_addr);
        binary.send(&initialize(json!(version))).await;
        assert_eq!(binary.reply().await, init);
        binary
            .send(&json!({"method":"notifications/initialized"}))
            .await;
        for (n, method) in METHODS.into_iter().enumerate() {
            let request = json!({"jsonrpc":"2.0","id":n,"method":method,"params":{"uri":"swarmail://inbox/x"}});
            let (status, reply) = post(srv.http_addr, &headers, &request).await;
            assert_eq!(status, 200, "{version} {method}");
            assert_eq!(reply["id"], n, "{version} {method}");
            assert_eq!(reply["error"]["code"], -32601, "{version} {method}");
            assert!(reply.get("result").is_none(), "{version} {method}");
            binary.send(&request).await;
            assert_eq!(binary.reply().await, reply, "bridge {version} {method}");
        }
        binary.finish().await;
    }
}

#[path = "mcp_revisions/focused.rs"]
mod focused;
#[path = "mcp_revisions/structured.rs"]
mod structured;
