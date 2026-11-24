//! The evidence judge's portable proofs (issue #392, proposed 0140): each
//! launch is built from synthetic exchanges in the shape Swarmail answers,
//! so every refusal is reached without a socket, a process or a wait.

use super::evidence::*;
use super::receipt::{Outcome, Status, TOOLS};
use crate::capture::Exchange;
use serde_json::{Value, json};

const ID: &str = "01a20f10-0ef2-70ba-9c1e-79fb1f2dfdce";
const REV: &str = "2025-11-25";

fn exchange(request: Value, reply: Value) -> Exchange {
    Exchange {
        post: true,
        request: serde_json::to_vec(&request).unwrap(),
        reply: Some((200, serde_json::to_vec(&reply).unwrap())),
    }
}

fn init_request(id: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": "initialize", "params": {}})
}

fn init_pair(id: Value, revision: &str) -> Exchange {
    exchange(
        init_request(id.clone()),
        json!({"jsonrpc": "2.0", "id": id, "result": {
            "protocolVersion": revision,
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "swarmail", "version": "0.5.0"},
        }}),
    )
}

fn notification() -> Exchange {
    Exchange {
        post: true,
        request: br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#.to_vec(),
        reply: Some((202, Vec::new())),
    }
}

fn email() -> Value {
    json!({"id": ID, "subject": SUBJECT, "codes": [CODE]})
}

/// Swarmail's modern result for `step`: a text block and structured output.
fn result_for(step: &str) -> Value {
    let body = match step {
        "list_tools" => {
            return json!({"tools": TOOLS.iter().map(|n| json!({"name": n})).collect::<Vec<_>>()});
        }
        "read" => json!({"emails": [email()]}),
        "wait_for_code" => {
            json!({"status": "selected", "selected": {"code": CODE, "email_id": ID}})
        }
        "structured_content" => json!({"emails": [email()], "inbox": "default", "total": 1}),
        "list_attachments" => json!({"id": ID, "attachments": [
            {"filename": ATTACHMENT, "content_type": ATTACHMENT_TYPE, "size": ATTACHMENT_SIZE}
        ]}),
        _ => unreachable!(),
    };
    json!({"content": [{"type": "text", "text": body.to_string()}], "isError": false,
           "structuredContent": body})
}

fn call_of(step: &str) -> Call {
    plan(ID)
        .into_iter()
        .find(|(s, _)| *s == step)
        .map(|(_, c)| c)
        .unwrap()
}

fn request_for(step: &str, id: Value) -> Value {
    match call_of(step) {
        Call::ListTools => json!({"jsonrpc": "2.0", "id": id, "method": "tools/list"}),
        Call::Tool { name, arguments } => json!({"jsonrpc": "2.0", "id": id,
            "method": "tools/call", "params": {"name": name, "arguments": arguments}}),
    }
}

fn reply_for(step: &str, result: Value) -> Exchange {
    exchange(
        request_for(step, json!(1)),
        json!({"jsonrpc": "2.0", "id": 1, "result": result}),
    )
}

/// A clean launch of `step`: handshake, notification, the call and its
/// reply, and the client's printed result.
fn launch(step: &str) -> Launch {
    let result = result_for(step);
    Launch {
        exchanges: vec![
            init_pair(json!(0), REV),
            notification(),
            reply_for(step, result.clone()),
        ],
        overflowed: false,
        client: Ok((true, serde_json::to_vec_pretty(&result).unwrap())),
    }
}

const STEPS5: [&str; 5] = [
    "list_tools",
    "read",
    "wait_for_code",
    "structured_content",
    "list_attachments",
];

#[track_caller]
fn failed(outcome: Outcome, needle: &str) {
    assert_eq!(
        outcome.status,
        Status::Fail,
        "expected a failure naming {needle:?}"
    );
    let reason = outcome.reason.unwrap();
    assert!(
        reason.contains(needle),
        "{reason:?} does not name {needle:?}"
    );
}

fn judged(step: &str, l: &Launch) -> Outcome {
    judge(step, &call_of(step), l, REV, ID)
}

#[test]
fn every_step_passes_on_a_clean_launch_and_ids_repeat_across_launches() {
    for step in STEPS5 {
        // Every launch reuses ids 0 and 1: ordinary across launches.
        assert_eq!(judged(step, &launch(step)), Outcome::pass(), "{step}");
    }
    let (init, reply, revision) = first_handshake(&launch("list_tools"));
    assert_eq!(init, Outcome::pass());
    assert_eq!(revision.as_deref(), Some(REV));
    assert_eq!(reply.unwrap()["result"]["serverInfo"]["name"], "swarmail");
}

#[test]
fn a_batched_exchange_is_paired_by_id() {
    let step = "read";
    let result = result_for(step);
    let mut l = launch(step);
    l.exchanges = vec![exchange(
        json!([init_request(json!(0)), request_for(step, json!(1))]),
        json!([
            {"jsonrpc": "2.0", "id": 1, "result": result},
            {"jsonrpc": "2.0", "id": 0, "result": {"protocolVersion": REV}},
        ]),
    )];
    assert_eq!(judged(step, &l), Outcome::pass());
}

#[test]
fn a_successful_exit_without_the_wire_exchange_fails() {
    for step in STEPS5 {
        let mut l = launch(step);
        l.exchanges.pop();
        failed(judged(step, &l), "the client sent no");
    }
    // Two of the expected call in one launch is not one observation.
    let mut l = launch("read");
    let mut extra = request_for("read", json!(2));
    extra["id"] = json!(2);
    l.exchanges.push(exchange(
        extra,
        json!({"jsonrpc": "2.0", "id": 2, "result": {}}),
    ));
    failed(judged("read", &l), "the client sent 2 tools/call");
    // A call of another tool is not the checklist's.
    let mut l = launch("read");
    l.exchanges.push(exchange(
        json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call",
               "params": {"name": "swarmail_clear_all"}}),
        json!({"jsonrpc": "2.0", "id": 5, "result": {}}),
    ));
    failed(
        judged("read", &l),
        "unexpected tools/call of swarmail_clear_all",
    );
    l.exchanges[3] = exchange(
        json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {}}),
        json!({"jsonrpc": "2.0", "id": 5, "result": {}}),
    );
    failed(judged("read", &l), "an unnamed tool");
    // Other arguments than the checklist's.
    let mut l = launch("read");
    let mut request = request_for("read", json!(1));
    request["params"]["arguments"]["count"] = json!(2);
    l.exchanges[2] = exchange(
        request,
        json!({"jsonrpc": "2.0", "id": 1, "result": result_for("read")}),
    );
    failed(judged("read", &l), "the client sent arguments");
}

#[test]
fn missing_mismatched_and_reused_ids_fail() {
    // The reply carries another id: the call has no paired reply.
    let mut l = launch("read");
    l.exchanges[2] = exchange(
        request_for("read", json!(1)),
        json!({"jsonrpc": "2.0", "id": 9, "result": result_for("read")}),
    );
    failed(judged("read", &l), "no JSON-RPC reply");
    // The reply has no id at all.
    l.exchanges[2] = exchange(
        request_for("read", json!(1)),
        json!({"jsonrpc": "2.0", "result": result_for("read")}),
    );
    failed(judged("read", &l), "no JSON-RPC reply");
    // An id reused within one launch is ambiguous.
    let mut l = launch("read");
    l.exchanges[2] = exchange(
        request_for("read", json!(0)),
        json!({"jsonrpc": "2.0", "id": 0, "result": result_for("read")}),
    );
    failed(judged("read", &l), "reused within one launch");
    let (init, reply, revision) = first_handshake(&l);
    failed(init, "reused within one launch");
    assert_eq!((reply, revision), (None, None));
    // No reply at all: Swarmail answered nothing, the client saw 502.
    let mut l = launch("read");
    l.exchanges[2].reply = None;
    failed(judged("read", &l), "no JSON-RPC reply");
    // An HTTP failure carrying a paired body.
    let mut l = launch("read");
    l.exchanges[2].reply.as_mut().unwrap().0 = 500;
    failed(judged("read", &l), "HTTP 500");
    // An unparseable request or a GET carries no JSON-RPC and is skipped.
    let mut l = launch("read");
    let junk = Exchange {
        post: true,
        request: b"nope".to_vec(),
        reply: None,
    };
    let get = Exchange {
        post: false,
        request: Vec::new(),
        reply: Some((405, Vec::new())),
    };
    l.exchanges.insert(1, junk);
    l.exchanges.insert(1, get);
    assert_eq!(judged("read", &l), Outcome::pass());
}

#[test]
fn an_ambiguous_changed_or_absent_handshake_fails_the_launch() {
    let mut l = launch("read");
    l.exchanges.insert(1, init_pair(json!(7), REV));
    failed(judged("read", &l), "2 initialize requests");
    let mut l = launch("read");
    l.exchanges[0] = init_pair(json!(0), "2025-06-18");
    failed(
        judged("read", &l),
        "this launch negotiated 2025-06-18, the first launch 2025-11-25",
    );
    let mut l = launch("read");
    l.exchanges.remove(0);
    failed(judged("read", &l), "no initialize request was observed");
    let (init, reply, revision) = first_handshake(&l);
    failed(init, "no initialize request");
    assert_eq!((reply, revision), (None, None));
}

#[test]
fn a_first_handshake_error_is_kept_as_captured() {
    let error = json!({"jsonrpc": "2.0", "id": 0, "error": {"code": -32602, "message": "bad"}});
    let mut l = launch("list_tools");
    l.exchanges[0] = exchange(init_request(json!(0)), error.clone());
    let (init, reply, revision) = first_handshake(&l);
    failed(init, "JSON-RPC error -32602");
    assert_eq!(reply, Some(error));
    assert_eq!(revision, None);
    // An error without a numeric code.
    l.exchanges[0] = exchange(
        init_request(json!(0)),
        json!({"jsonrpc": "2.0", "id": 0, "error": {}}),
    );
    failed(first_handshake(&l).0, "JSON-RPC error ?");
    // No reply to initialize, and a reply without a usable revision.
    l.exchanges[0].reply = None;
    failed(first_handshake(&l).0, "initialize got no JSON-RPC reply");
    let blank = json!({"jsonrpc": "2.0", "id": 0, "result": {"protocolVersion": " "}});
    l.exchanges[0] = exchange(init_request(json!(0)), blank.clone());
    let (init, reply, revision) = first_handshake(&l);
    failed(init, "no protocolVersion");
    assert_eq!((reply, revision.as_deref()), (Some(blank), Some(" ")));
    // A success body behind an HTTP failure.
    let mut l = launch("list_tools");
    l.exchanges[0].reply.as_mut().unwrap().0 = 400;
    let (init, reply, _) = first_handshake(&l);
    failed(init, "initialize answered HTTP 400");
    assert!(reply.is_some());
}

#[test]
fn a_server_or_tool_error_fails_the_step() {
    let mut l = launch("read");
    l.exchanges[2] = exchange(
        request_for("read", json!(1)),
        json!({"jsonrpc": "2.0", "id": 1, "error": {"code": -32601}}),
    );
    failed(judged("read", &l), "JSON-RPC error");
    l.exchanges[2] = reply_for("read", json!({"isError": true, "content": []}));
    failed(judged("read", &l), "the tool returned isError");
    l.exchanges[2] = exchange(
        request_for("read", json!(1)),
        json!({"jsonrpc": "2.0", "id": 1}),
    );
    failed(judged("read", &l), "the reply has no result");
}

#[test]
fn march_text_only_search_fails_structured_content_only() {
    let mut text_only = result_for("structured_content");
    text_only
        .as_object_mut()
        .unwrap()
        .remove("structuredContent");
    let mut l = launch("structured_content");
    l.exchanges[2] = reply_for("structured_content", text_only);
    failed(
        judged("structured_content", &l),
        "text block only, no structuredContent",
    );
    // The text payload alone still proves read, wait and attachments, and
    // structured output alone proves them too.
    for step in ["read", "wait_for_code", "list_attachments"] {
        for strip in ["structuredContent", "content"] {
            let mut r = result_for(step);
            r.as_object_mut().unwrap().remove(strip);
            let mut l = launch(step);
            l.exchanges[2] = reply_for(step, r.clone());
            l.client = Ok((true, serde_json::to_vec(&r).unwrap()));
            assert_eq!(judged(step, &l), Outcome::pass(), "{step} without {strip}");
        }
    }
}

#[test]
fn the_client_must_finish_and_print_what_it_received() {
    let mut l = launch("read");
    l.client = Ok((false, Vec::new()));
    failed(
        judged("read", &l),
        "the client exited with a failure status",
    );
    l.client = Err("helper_timeout".into());
    failed(
        judged("read", &l),
        "the client did not finish: helper_timeout",
    );
    l.client = Ok((true, b"not json".to_vec()));
    failed(judged("read", &l), "the client printed no JSON result");
    l.client = Ok((true, b"{}".to_vec()));
    failed(judged("read", &l), "the client's result: no JSON payload");
    let mut l = launch("read");
    l.overflowed = true;
    failed(judged("read", &l), "exceeded the observer's bound");
}

#[test]
fn each_step_refuses_a_result_without_its_values() {
    let text = |v: Value| json!({"content": [{"type": "text", "text": v.to_string()}]});
    let wrong_size = json!({"id": ID, "attachments": [
        {"filename": ATTACHMENT, "content_type": ATTACHMENT_TYPE, "size": 12}
    ]});
    for (step, result, needle) in [
        ("list_tools", json!({}), "no tools array"),
        (
            "list_tools",
            json!({"tools": [{"name": TOOLS[0]}]}),
            "swarmail_wait_for_code is not listed",
        ),
        ("read", text(json!({"emails": []})), "not exactly one email"),
        (
            "read",
            text(json!({"emails": [{"id": "other", "subject": SUBJECT}]})),
            "not the stored fixture",
        ),
        (
            "read",
            json!({"content": [{"type": "text", "text": "[]"}]}),
            "no JSON payload",
        ),
        (
            "wait_for_code",
            text(json!({"status": "timeout"})),
            "status is \"timeout\"",
        ),
        (
            "wait_for_code",
            text(json!({"status": "selected", "selected": {"code": "1", "email_id": ID}})),
            "not the fixture's",
        ),
        (
            "wait_for_code",
            text(json!({"status": "selected", "selected": {"code": CODE, "email_id": "x"}})),
            "not the fixture's",
        ),
        (
            "structured_content",
            json!({"structuredContent": []}),
            "no structuredContent object",
        ),
        (
            "structured_content",
            json!({"structuredContent": {"emails": [{"id": "x"}]}}),
            "lacks the stored ID",
        ),
        (
            "list_attachments",
            text(json!({"id": "x"})),
            "not of the stored ID",
        ),
        ("list_attachments", text(wrong_size), "no matrix.txt entry"),
        ("initialize", json!({}), "not a judged step"),
    ] {
        let error = expect(step, &result, ID).expect_err(step);
        assert!(error.contains(needle), "{step}: {error:?} lacks {needle:?}");
    }
}

#[test]
fn the_inspector_arguments_follow_its_cli_grammar() {
    let [list, read, wait, search, attachments] = plan(ID).map(|(_, c)| c.inspector_args());
    assert_eq!(list, ["--transport", "http", "--method", "tools/list"]);
    assert_eq!(
        read,
        [
            "--transport",
            "http",
            "--method",
            "tools/call",
            "--tool-name",
            TOOLS[0],
            "--tool-arg",
            "count=1",
            "--tool-arg",
            "inbox=default"
        ]
    );
    assert!(wait.ends_with(&["--tool-arg".into(), "timeout_ms=0".into()]));
    assert_eq!(search[search.len() - 1], "inbox=default");
    assert_eq!(attachments[attachments.len() - 1], format!("id={ID}"));
}

#[test]
fn reasons_are_short_single_lines_and_redaction_keeps_the_envelope() {
    assert_eq!(sanitize("a\nb\tc"), "a b c");
    assert_eq!(sanitize(&"x".repeat(500)).chars().count(), 200);
    assert_eq!(sanitize("\n\n"), "unspecified failure");
    let mut reply = json!({"jsonrpc": "2.0", "id": "host-7", "result": {
        "protocolVersion": "host-7",
        "serverInfo": {"name": "swarmail on host-7", "version": "0.5.0"},
        "instructions": ["see /home/me/x", 3, null],
        "_meta": {"x-extension": true}}});
    let secrets = ["host-7", "/home/me", "/", " "].map(String::from);
    redact(&mut reply, &secrets);
    assert_eq!(reply["id"], "host-7");
    assert_eq!(reply["result"]["protocolVersion"], "host-7");
    assert_eq!(reply["result"]["serverInfo"]["name"], "<redacted>");
    assert_eq!(reply["result"]["serverInfo"]["version"], "0.5.0");
    assert_eq!(
        reply["result"]["instructions"],
        json!(["<redacted>", 3, null])
    );
    assert_eq!(reply["result"]["_meta"]["x-extension"], true);
}

#[test]
fn the_runner_delivers_the_checklist_fixture_with_crlf_lines() {
    let message = String::from_utf8(super::fixture_message()).unwrap();
    assert!(message.starts_with("From: Matrix Sender <sender@matrix.test>\r\n"));
    assert!(message.contains(&format!("Subject: {SUBJECT}\r\n")));
    assert!(message.contains(&format!("Your verification code is {CODE}.")));
    assert!(message.contains(&format!("filename=\"{ATTACHMENT}\"")));
    assert!(message.ends_with("--matrix-boundary--\r\n"));
    assert!(!message.replace("\r\n", "").contains('\n'));
    for broken in [
        "no block",
        "`matrix-fixture.eml`:\n no fence",
        "`matrix-fixture.eml`:\n```text\nunterminated\n",
    ] {
        assert_eq!(
            super::receipt::fenced_fixture(broken).unwrap_err(),
            "the checklist's fixture block is missing"
        );
    }
    let page = "`matrix-fixture.eml`:\n\n   ```text\n   A: b\n\n   c\n   ```\n";
    assert_eq!(super::receipt::fenced_fixture(page).unwrap(), "A: b\n\nc\n");
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

#[test]
fn the_run_command_refuses_bad_arguments_before_starting_anything() {
    let absent = "/nonexistent-client-matrix-output";
    let base = |flag: &str, value: &str| {
        let mut args: Vec<String> = [
            "run",
            "--binary",
            "/bin/sh",
            "--commit",
            &"a".repeat(40),
            "--client-version",
            "1",
            "--os",
            "x",
            "--recorded-by",
            "operator",
            "--output-dir",
            absent,
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let at = args.iter().position(|a| a == flag).unwrap();
        args[at + 1] = value.into();
        args.extend(["--".into(), "/bin/true".into()]);
        args
    };
    let rt = runtime();
    for (flag, value) in [
        ("--binary", "relative/swarmail"),
        ("--commit", "abc"),
        ("--client-version", " "),
        ("--os", ""),
        ("--recorded-by", "ci"),
    ] {
        assert_eq!(
            rt.block_on(super::command(&base(flag, value))),
            super::EXIT_ERROR,
            "{flag}"
        );
    }
    assert!(!std::path::Path::new(absent).exists());
    // Parse refusals: no subcommand, and no client argv.
    assert_eq!(rt.block_on(super::command(&[])), super::EXIT_ERROR);
    let mut no_client = base("--os", "x");
    no_client.truncate(no_client.len() - 2);
    assert_eq!(rt.block_on(super::command(&no_client)), super::EXIT_ERROR);
}

#[test]
fn validate_reads_files_and_directories() {
    let dir = tempfile::tempdir().unwrap();
    let rt = runtime();
    let run = |paths: &[&std::path::Path]| {
        let mut args = vec!["validate".to_string()];
        args.extend(paths.iter().map(|p| p.display().to_string()));
        rt.block_on(super::command(&args))
    };
    assert_eq!(run(&[dir.path()]), super::EXIT_GREEN);
    let bad = dir.path().join("2026-11-25-codex.json");
    std::fs::write(&bad, b"{}").unwrap();
    assert_eq!(run(&[&bad]), super::EXIT_ERROR);
    assert_eq!(run(&[dir.path()]), super::EXIT_ERROR);
}
