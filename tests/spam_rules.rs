//! The built-in spam signals through real SMTP, REST and MCP (issue #260,
//! proposed decision 0122): one failing and one passing message per rule, the
//! recorded rule-set version, determinism, laziness and the fail-closed text bound.
mod common;
use common::*;
use serde_json::{Value, json};
use swarmail::RunningServer;

const FROM: &str = "\"Billing\" <billing@example.test>";
const SUBJECT: &str = "Your receipt";
const BODY: &str = "Your order has shipped.";

async fn report(srv: &RunningServer, id: &str) -> (u16, Value) {
    http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{id}/deliverability"),
        None,
    )
    .await
}

/// A clean baseline message; each fixture changes only its trigger.
fn message(from: &str, subject: &str, body: &str) -> String {
    format!("From: {from}\r\nTo: t@example.test\r\nSubject: {subject}\r\n\r\n{body}\r\n")
}

fn spam(report: &Value) -> Vec<Value> {
    let findings = report["findings"].as_array().unwrap();
    findings
        .iter()
        .filter(|f| f["rule_id"].as_str().unwrap().starts_with("spam."))
        .cloned()
        .collect()
}

fn others(report: &Value) -> Vec<Value> {
    let findings = report["findings"].as_array().unwrap();
    findings
        .iter()
        .filter(|f| !f["rule_id"].as_str().unwrap().starts_with("spam."))
        .cloned()
        .collect()
}

/// `(rule id, failing message, passing message, expected finding)`, the expected
/// objects written here independently of the code under test.
fn fixtures() -> Vec<(&'static str, String, String, Value)> {
    let subject = |id: &str, severity: &str, hint: &str| {
        json!({"rule_id":id,"severity":severity,
            "location":{"kind":"header","name":"subject","occurrence":0},"fix_hint":hint})
    };
    let zw = "\u{200B}";
    vec![
        (
            "spam.subject.all_caps.v1",
            message(FROM, "WINTER SALE", BODY),
            message(FROM, "WINTER Sale", BODY),
            subject(
                "spam.subject.all_caps.v1",
                "warning",
                "Write the Subject in sentence case; an all-capital subject reads as shouting and is a common spam signal.",
            ),
        ),
        (
            "spam.subject.punctuation.v1",
            message(FROM, "Your receipt!!", BODY),
            message(FROM, "Your receipt!", BODY),
            subject(
                "spam.subject.punctuation.v1",
                "info",
                "Remove repeated !, ? or $ characters from the Subject.",
            ),
        ),
        (
            "spam.subject.phrases.v1",
            message(FROM, "Act now to keep your account", BODY),
            message(FROM, "Act soon to keep your account", BODY),
            subject(
                "spam.subject.phrases.v1",
                "warning",
                "Remove high-pressure sales wording from the Subject; see the guide's phrase list.",
            ),
        ),
        (
            "spam.from.display_name_address.v1",
            message("\"ceo@bank.example\" <billing@example.test>", SUBJECT, BODY),
            message("\"Billing\" <billing@example.test>", SUBJECT, BODY),
            json!({"rule_id":"spam.from.display_name_address.v1","severity":"warning",
                "location":{"kind":"header","name":"from","occurrence":0},
                "fix_hint":"Do not put an email address in the From display name unless it is the sending address; a different address there looks like spoofing."}),
        ),
        (
            "spam.body.phrases.v1",
            message(FROM, SUBJECT, "Your order has shipped. Act now."),
            message(FROM, SUBJECT, "Your order has shipped. Act soon."),
            json!({"rule_id":"spam.body.phrases.v1","severity":"warning",
                "location":{"kind":"mime_part","path":[]},
                "fix_hint":"Remove high-pressure sales wording from the text/plain body; see the guide's phrase list."}),
        ),
        (
            "spam.body.zero_width.v1",
            message(
                FROM,
                SUBJECT,
                &format!("Your{zw} order{zw} has{zw} shipped."),
            ),
            message(FROM, SUBJECT, &format!("Your{zw} order{zw} has shipped.")),
            json!({"rule_id":"spam.body.zero_width.v1","severity":"info",
                "location":{"kind":"mime_part","path":[]},
                "fix_hint":"Remove zero-width characters from the text/plain body; they are used to hide words from filters."}),
        ),
    ]
}

#[tokio::test]
async fn every_rule_has_a_failing_and_passing_fixture() {
    let srv = start().await;
    let table = fixtures();
    assert_eq!(table.len(), 6);
    for (rule, failing, passing, expected) in table {
        let fail_id = smtp_raw(srv.smtp_addr, "spam-pairs", failing.as_bytes()).await;
        let pass_id = smtp_raw(srv.smtp_addr, "spam-pairs", passing.as_bytes()).await;
        let (status, fail) = report(&srv, &fail_id).await;
        assert_eq!(status, 200, "{rule}");
        let (status, pass) = report(&srv, &pass_id).await;
        assert_eq!(status, 200, "{rule}");
        assert_eq!(spam(&fail), vec![expected], "{rule}");
        assert_eq!(spam(&pass), Vec::<Value>::new(), "{rule}");
        assert_eq!(
            others(&fail),
            others(&pass),
            "{rule}: only the trigger differs"
        );
    }
    srv.stop().await;
}

#[tokio::test]
async fn a_clean_baseline_has_no_spam_finding() {
    let srv = start().await;
    let id = smtp_raw(
        srv.smtp_addr,
        "spam-clean",
        message(FROM, SUBJECT, BODY).as_bytes(),
    )
    .await;
    let (_, clean) = report(&srv, &id).await;
    assert!(spam(&clean).is_empty());
    srv.stop().await;
}

async fn mcp(srv: &RunningServer, version: Option<&str>, id: &str) -> Value {
    let body = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
        "name":"swarmail_get_deliverability_report","arguments":{"id":id}}});
    let headers: Vec<(&str, &str)> = version
        .map(|v| vec![("MCP-Protocol-Version", v)])
        .unwrap_or_default();
    let (status, text) = http_send_text_with_headers(
        srv.http_addr,
        "POST",
        "/mcp",
        &headers,
        Some(&body.to_string()),
    )
    .await;
    assert_eq!(status, 200);
    serde_json::from_str(&text).unwrap()
}

#[tokio::test]
async fn report_identity_and_shape() {
    let srv = start().await;
    let ordinary = message(FROM, "ACT NOW SALE!!", "Act now");
    let demo = format!("X-Swarmail-Demo: warning\r\n{ordinary}");
    for raw in [ordinary, demo] {
        let id = smtp_raw(srv.smtp_addr, "spam-identity", raw.as_bytes()).await;
        let (status, rest) = report(&srv, &id).await;
        assert_eq!(status, 200);
        assert_eq!(rest["rule_set_id"], "swarmail-html/6");
        // No weight or total: exactly the contract keys (decision 0085, with
        // the `dns` member of proposed 0120).
        let mut keys: Vec<_> = rest.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, ["dns", "findings", "rule_set_id"]);
        for finding in rest["findings"].as_array().unwrap() {
            let mut keys: Vec<_> = finding.as_object().unwrap().keys().cloned().collect();
            keys.sort();
            assert_eq!(keys, ["fix_hint", "location", "rule_id", "severity"]);
        }
        assert_eq!(spam(&rest).len(), 4, "{rest}");
        // The default (March) revision: JSON in one text block, no structuredContent.
        let march = mcp(&srv, None, &id).await;
        assert_eq!(march["result"]["isError"], false);
        assert!(march["result"].get("structuredContent").is_none());
        let text: Value =
            serde_json::from_str(march["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(text, rest);
        // A modern revision adds the same report as structuredContent.
        for version in ["2025-06-18", "2025-11-25"] {
            let modern = mcp(&srv, Some(version), &id).await;
            assert_eq!(modern["result"]["isError"], false, "{version}");
            assert_eq!(modern["result"]["structuredContent"], rest, "{version}");
            assert_eq!(
                modern["result"]["structuredContent"]["rule_set_id"],
                "swarmail-html/6"
            );
        }
    }
    srv.stop().await;
}

#[tokio::test]
async fn spam_report_is_deterministic_and_lazy() {
    let srv = start().await;
    let raw = message("\"x@y.co\" <a@b.co>", "ACT NOW!! FREE", "risk-free");
    let first = smtp_raw(srv.smtp_addr, "spam-lazy", raw.as_bytes()).await;
    let second = smtp_raw(srv.smtp_addr, "spam-lazy", raw.as_bytes()).await;
    let path = |id: &str| format!("/api/v1/messages/{id}");
    // Nothing is analysed on the SMTP path: reading the report changes no stored field.
    let before = http_send_text(srv.http_addr, "GET", &path(&first), None).await;
    assert_eq!(before.0, 200);
    let (_, miss) = http_send_text(
        srv.http_addr,
        "GET",
        &format!("{}/deliverability", path(&first)),
        None,
    )
    .await;
    let (_, hit) = http_send_text(
        srv.http_addr,
        "GET",
        &format!("{}/deliverability", path(&first)),
        None,
    )
    .await;
    assert_eq!(miss, hit, "a cache hit is byte-identical to the miss");
    let after = http_send_text(srv.http_addr, "GET", &path(&first), None).await;
    assert_eq!(after, before);
    let (_, twin) = http_send_text(
        srv.http_addr,
        "GET",
        &format!("{}/deliverability", path(&second)),
        None,
    )
    .await;
    assert_eq!(
        twin, miss,
        "the same bytes as another message give the same report"
    );
    let report: Value = serde_json::from_str(&miss).unwrap();
    assert_eq!(spam(&report).len(), 5, "{report}");
    let (_, raw_back) = http_bytes(srv.http_addr, &format!("{}/raw", path(&first))).await;
    assert_eq!(raw_back, raw.as_bytes(), "stored bytes untouched");
    srv.stop().await;
}

/// A text/plain body of at least `n` decoded bytes in 76-byte lines.
fn big_body(n: usize) -> String {
    let line = format!("{}\r\n", "x".repeat(76));
    line.repeat(n / line.len() + 1)
}

#[tokio::test]
async fn oversized_text_fails_closed() {
    let srv = start().await;
    let small = smtp_raw(
        srv.smtp_addr,
        "spam-size",
        message(FROM, SUBJECT, &big_body(1 << 20)).as_bytes(),
    )
    .await;
    assert_eq!(report(&srv, &small).await.0, 200);
    let over = message(FROM, SUBJECT, &big_body(4_194_305));
    let id = smtp_raw(srv.smtp_addr, "spam-size", over.as_bytes()).await;
    assert_eq!(
        report(&srv, &id).await,
        (503, json!({"error":"deliverability analysis failed"}))
    );
    for version in [None, Some("2025-06-18")] {
        let reply = mcp(&srv, version, &id).await;
        assert_eq!(reply["result"]["isError"], true, "{version:?}");
        assert!(reply["result"].get("structuredContent").is_none());
    }
    // The mail itself was accepted and is stored (0001): only the analysis failed.
    let (status, stored) = http_bytes(srv.http_addr, &format!("/api/v1/messages/{id}/raw")).await;
    assert_eq!(status, 200);
    assert_eq!(stored, over.as_bytes());
    srv.stop().await;
}
