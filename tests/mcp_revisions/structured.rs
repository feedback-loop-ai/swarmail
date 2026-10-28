use super::*;

// The independently captured thirteen-tool March baseline (pre-#153 #152
// candidate 3a2eef9; see march-current-main.provenance.json), its request log
// and the case list both it and these workflows drive.
const CASES: &str = include_str!("../fixtures/mcp-structured-output/march-current-main.cases.json");
const BASELINE: &str = include_str!("../fixtures/mcp-structured-output/march-current-main.jsonl");
const REQUESTS: &str =
    include_str!("../fixtures/mcp-structured-output/march-current-main.requests.jsonl");
// The original twelve-tool capture (dc67f4a), kept as historical evidence.
const HISTORICAL_MARCH: &str = include_str!("../fixtures/mcp-structured-output/march.jsonl");

/// The exact catalogue, in served order.
const NAMES: [&str; 13] = [
    "swarmail_search_emails",
    "swarmail_get_deliverability_report",
    "swarmail_get_email",
    "swarmail_get_latest_email",
    "swarmail_delete_email",
    "swarmail_clear_inbox",
    "swarmail_clear_all",
    "swarmail_wait_for_email",
    "swarmail_extract_links",
    "swarmail_extract_codes",
    "swarmail_seed_email",
    "swarmail_set_chaos",
    "swarmail_clear_chaos",
];

// Fixed restored-row shapes include fields that ingest no longer creates.
fn historical() -> swarmail::model::Email {
    use swarmail::model::{Email, EmailAddress};
    Email {
        id: "historical".into(),
        inbox: "contract".into(),
        from: Some(EmailAddress {
            name: Some("Sender \"雪\"".into()),
            address: "from@x.io".into(),
        }),
        envelope_from: None,
        to: vec![EmailAddress {
            name: None,
            address: "to@x.io".into(),
        }],
        cc: vec![EmailAddress {
            name: Some("Cc".into()),
            address: "cc@x.io".into(),
        }],
        recipients: vec!["bcc@x.io".into()],
        subject: Some("quoted \"雪\" \\ subject".into()),
        received_at: "1969-12-31T23:59:59Z".into(),
        received_ms: -1000,
        size: 42,
        text: Some("line\n\t\u{0}\u{1b} 雪 \\".into()),
        html: Some("<b>untrusted</b>".into()),
        link_metadata: None,
        links: vec!["https://x.io/?q=雪".into()],
        codes: (0..65).map(|i| format!("{i:04}")).collect(),
        message_id: Some("message@x".into()),
        in_reply_to: Some("parent@x".into()),
        references: vec!["root@x".into()],
        raw: b"never exposed".to_vec(),
    }
}

fn minimal() -> swarmail::model::Email {
    let mut minimal = historical();
    minimal.id = "minimal".into();
    minimal.inbox = "other".into();
    minimal.envelope_from = Some(String::new());
    minimal.from = None;
    minimal.to.clear();
    minimal.cc.clear();
    minimal.recipients.clear();
    minimal.subject = None;
    minimal.text = None;
    minimal.html = None;
    minimal.links.clear();
    minimal.codes.clear();
    minimal.message_id = None;
    minimal.in_reply_to = None;
    minimal.references.clear();
    minimal
}

// The swarmail-bulk/1 checklist restated from its published rule table, not
// from the formatter: every hint cites its RFC source and both provider pages.
const GMAIL: &str = "https://support.google.com/mail/answer/81126";
const YAHOO: &str = "https://senders.yahooinc.com/best-practices/";
fn finding(rule_id: &str, severity: &str, location: Value, hint: &str, source: &str) -> Value {
    json!({"rule_id":rule_id,"severity":severity,"location":location,
        "fix_hint":format!("{hint} Sources: {source} {GMAIL} {YAHOO}")})
}
fn header(name: &str, occurrence: Value) -> Value {
    json!({"kind":"header","name":name,"occurrence":occurrence})
}
/// Findings of a message with a From field and a text/plain body but no
/// Date, Message-ID or one-click unsubscribe, and no trusted authentication.
fn checklist(from: Value) -> Vec<Value> {
    let rfc7489 = "https://www.rfc-editor.org/rfc/rfc7489";
    vec![
        finding(
            "bulk.authenticated_sender.not_evaluated.v1",
            "error",
            header("from", from.clone()),
            "Not evaluated: trusted From identity or passing-and-aligned DKIM/SPF evidence is incomplete.",
            &format!("{rfc7489}#section-3.1"),
        ),
        finding(
            "bulk.date.v1",
            "error",
            header("date", Value::Null),
            "Provide exactly one calendar-valid Date using current RFC 5322 syntax and a numeric zone; obsolete-only forms are unsupported by this checklist.",
            "https://www.rfc-editor.org/rfc/rfc5322#section-3.6.1",
        ),
        finding(
            "bulk.dmarc_presence.not_evaluated.v1",
            "error",
            header("from", from.clone()),
            "Not evaluated: trusted applicable DMARC policy evidence is unavailable.",
            &format!("{rfc7489}#section-6.1"),
        ),
        finding(
            "bulk.from_alignment.not_evaluated.v1",
            "error",
            header("from", from),
            "Not evaluated: trusted From identity or DKIM/SPF alignment evidence is incomplete.",
            &format!("{rfc7489}#section-3.1"),
        ),
        finding(
            "bulk.message_id.v1",
            "error",
            header("message-id", Value::Null),
            "Provide exactly one Message-ID using the supported current RFC 5322 syntax; obsolete-only forms are unsupported by this checklist.",
            "https://www.rfc-editor.org/rfc/rfc5322#section-3.6.4",
        ),
        finding(
            "bulk.one_click.v1",
            "error",
            header("list-unsubscribe", Value::Null),
            "Provide one HTTPS List-Unsubscribe list and one exact List-Unsubscribe-Post: List-Unsubscribe=One-Click value. This structural check does not verify DKIM header coverage or endpoint operation.",
            "https://www.rfc-editor.org/rfc/rfc8058#section-3",
        ),
    ]
}
fn report_of(findings: Vec<Value>) -> Value {
    json!({"rule_set_id":"swarmail-bulk/1","findings":findings})
}

pub(super) enum Wire {
    Http(std::net::SocketAddr, Option<&'static str>),
    Stdio(Box<Binary>),
}
impl Wire {
    pub(super) async fn start(
        addr: std::net::SocketAddr,
        version: &'static str,
        stdio: bool,
    ) -> Self {
        if stdio {
            let mut binary = Binary::start(addr);
            binary.ready(version).await;
            Self::Stdio(Box::new(binary))
        } else {
            Self::Http(addr, Some(version))
        }
    }
    pub(super) async fn raw(&mut self, request: &Value) -> String {
        match self {
            Self::Http(addr, version) => {
                let headers = version
                    .map(|v| vec![("MCP-Protocol-Version", v)])
                    .unwrap_or_default();
                let (status, raw) = http_send_text_with_headers(
                    *addr,
                    "POST",
                    "/mcp",
                    &headers,
                    Some(&request.to_string()),
                )
                .await;
                assert_eq!(status, 200);
                raw
            }
            Self::Stdio(binary) => {
                binary.send(request).await;
                let mut raw = String::new();
                assert!(
                    tokio::time::timeout(
                        Duration::from_secs(30),
                        binary.output.read_line(&mut raw)
                    )
                    .await
                    .unwrap()
                    .unwrap()
                        > 0
                );
                raw.strip_suffix('\n').unwrap().to_owned()
            }
        }
    }
    pub(super) async fn request(&mut self, request: &Value) -> Value {
        serde_json::from_str(&self.raw(request).await).unwrap()
    }
    pub(super) async fn finish(self) {
        if let Self::Stdio(binary) = self {
            (*binary).finish().await;
        }
    }
}

/// One transport's transcript: request lines (the case identities) and
/// response lines, the seed UUID replaced by $SEED_ID after counting it.
struct Run {
    requests: Vec<String>,
    lines: Vec<String>,
}

async fn workflow(version: Option<&'static str>, stdio: bool, modern: bool) -> Run {
    let srv = start().await;
    let mut wire = if stdio {
        let mut binary = Binary::start(srv.http_addr);
        if let Some(version) = version {
            binary.ready(version).await;
        } else {
            let mut init = initialize(json!(VERSIONS[0]));
            init["params"]
                .as_object_mut()
                .unwrap()
                .remove("protocolVersion");
            binary.send(&init).await;
            assert_eq!(
                binary.reply().await["result"]["protocolVersion"],
                VERSIONS[0]
            );
        }
        Wire::Stdio(Box::new(binary))
    } else {
        Wire::Http(srv.http_addr, version)
    };
    let list_request = json!({"id":1,"method":"tools/list"});
    let catalogue = wire.raw(&list_request).await;
    let list: Value = serde_json::from_str(&catalogue).unwrap();
    let mut run = Run {
        requests: vec![list_request.to_string()],
        lines: vec![catalogue],
    };
    let mut seen = std::collections::BTreeSet::new();
    // Text alone supplies the seed ID used below, including in modern sessions.
    let mut seeded = Value::Null;
    let cases: Value = serde_json::from_str(CASES).unwrap();
    for case in cases.as_array().unwrap() {
        let name = case["tool"].as_str().unwrap();
        if case["restore"] == true {
            srv.store.insert(historical()).unwrap();
            srv.store.insert(minimal()).unwrap();
        }
        let mut arguments = case["arguments"].clone();
        if arguments["id"] == "$seed" {
            arguments["id"] = seeded.clone();
        }
        let request = call(json!(2), name, arguments);
        let raw = wire.raw(&request).await;
        let reply: Value = serde_json::from_str(&raw).unwrap();
        if reply["result"]["isError"] == false {
            let payload = text(&reply);
            if name == "seed_email" {
                seeded = payload["id"].clone();
            }
            if modern {
                assert_eq!(payload, reply["result"]["structuredContent"]);
                let tool = list["result"]["tools"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|t| t["name"] == format!("swarmail_{name}"))
                    .unwrap();
                assert!(valid(&tool["outputSchema"], &payload), "{name}: {payload}");
                if name == "get_deliverability_report" {
                    // Stated expectation and the served REST report, read
                    // while the seeded message is still stored.
                    assert_eq!(payload, report_of(checklist(json!(0))));
                    let path = format!(
                        "/api/v1/messages/{}/deliverability",
                        seeded.as_str().unwrap()
                    );
                    let rest = common::http_json(srv.http_addr, "GET", &path, None).await;
                    assert_eq!(rest, (200, payload.clone()));
                }
                seen.insert(format!("swarmail_{name}"));
            } else {
                assert!(reply["result"].get("structuredContent").is_none());
            }
        } else {
            assert!(reply["result"].get("structuredContent").is_none());
        }
        let (request, raw) = match seeded.as_str() {
            Some(id) if name == "seed_email" => {
                assert_eq!(raw.matches(id).count(), if modern { 2 } else { 1 });
                (request.to_string(), raw.replace(id, "$SEED_ID"))
            }
            Some(id) => {
                assert!(!raw.contains(id), "{name} echoes the seed id");
                (request.to_string().replace(id, "$SEED_ID"), raw)
            }
            None => (request.to_string(), raw),
        };
        run.requests.push(request);
        run.lines.push(raw);
    }
    if modern {
        assert_eq!(
            seen,
            NAMES.iter().map(|n| n.to_string()).collect(),
            "every tool succeeds"
        );
    }
    wire.finish().await;
    srv.stop().await;
    run
}

#[tokio::test]
async fn legacy_bytes() {
    let requests: Vec<&str> = REQUESTS.lines().collect();
    for version in [None, Some(VERSIONS[0])] {
        for stdio in [false, true] {
            let run = workflow(version, stdio, false).await;
            assert_eq!(run.requests, requests, "case identities");
            assert_eq!(run.lines.len(), BASELINE.lines().count());
            // Discovery is additive (decision 0094 §7): the captured
            // thirteen-entry catalogue is a byte prefix, then the two
            // appended descriptors; every result line is byte-identical.
            let baseline: Vec<&str> = BASELINE.lines().collect();
            super::focused::assert_appended(&run.lines[0], baseline[0]);
            assert_eq!(
                run.lines[1..].join("\n") + "\n",
                baseline[1..].join("\n") + "\n"
            );
        }
    }
}

/// The dc67f4a twelve-tool record stays as captured: every shared case row of
/// the current baseline is byte-identical to it, and the catalogues differ
/// only by the report tool and current main's report threshold on the wait.
#[test]
fn historical_twelve_tool_fixture_is_preserved() {
    let old: Vec<&str> = HISTORICAL_MARCH.lines().collect();
    let new: Vec<&str> = BASELINE.lines().collect();
    let cases: Value = serde_json::from_str(CASES).unwrap();
    let shared: Vec<usize> = (0..cases.as_array().unwrap().len())
        .filter(|&i| cases[i]["tool"] != "get_deliverability_report")
        .collect();
    assert_eq!((old.len(), new.len(), shared.len()), (26, 32, 25));
    for (k, i) in shared.into_iter().enumerate() {
        assert_eq!(new[1 + i], old[1 + k], "case {i}");
    }
    let old: Value = serde_json::from_str(old[0]).unwrap();
    let mut new: Value = serde_json::from_str(new[0]).unwrap();
    let tools = new["result"]["tools"].as_array_mut().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names, NAMES);
    tools.remove(1);
    tools[6]["inputSchema"]["properties"]
        .as_object_mut()
        .unwrap()
        .remove("no_findings_at_or_above")
        .unwrap();
    assert_eq!(new, old);
}

// Deliberately finite evaluator for our emitted vocabulary. No reference is
// emitted: even a local $ref is refused, including unresolved/cyclic references.
// Only the two inspected OpenAPI patterns are admitted, each by a finite
// character-class matcher; any other pattern refuses the whole schema.
const RULE_ID_PATTERN: &str = "^[!-~]+$";
const HEADER_NAME_PATTERN: &str = "^[\\x21-\\x39\\x3b-\\x40\\x5b-\\x7e]+$";
fn pattern_matches(pattern: &str, s: &str) -> bool {
    let allowed = |c: char| match pattern {
        RULE_ID_PATTERN => ('!'..='~').contains(&c),
        HEADER_NAME_PATTERN => matches!(c, '\x21'..='\x39' | '\x3b'..='\x40' | '\x5b'..='\x7e'),
        _ => false,
    };
    !s.is_empty() && s.chars().all(allowed)
}
/// An integer bound exactly as published: a JSON integer, never a float.
fn bound(v: &Value) -> Option<i128> {
    v.as_i64()
        .map(i128::from)
        .or_else(|| v.as_u64().map(i128::from))
}
/// Exact comparison of a JSON number with an integer bound. Integers compare
/// as i128; a float compares by its exact value, never by a rounded bound.
fn compare(v: &Value, bound: i128) -> Option<std::cmp::Ordering> {
    use std::cmp::Ordering::{Equal, Greater, Less};
    if let Some(n) = v.as_i64() {
        return Some(i128::from(n).cmp(&bound));
    }
    if let Some(n) = v.as_u64() {
        return Some(i128::from(n).cmp(&bound));
    }
    let x = v.as_f64()?;
    // 2^127 is exact in f64; beyond it every published bound is surpassed.
    let limit = 2f64.powi(127);
    if x >= limit {
        return Some(Greater);
    }
    if x < -limit {
        return Some(Less);
    }
    // |trunc(x)| < 2^127 and integral, so the conversion is exact.
    let whole = x.trunc();
    Some(match (whole as i128).cmp(&bound) {
        Equal if x > whole => Greater,
        Equal if x < whole => Less,
        other => other,
    })
}
pub(super) fn schema_ok(s: &Value) -> bool {
    let Some(o) = s.as_object() else {
        return false;
    };
    if o.is_empty() {
        return false;
    }
    for (key, value) in o {
        let ok = match key.as_str() {
            "type" => {
                let kinds = ["object", "array", "string", "integer", "boolean", "null"];
                if let Some(kind) = value.as_str() {
                    kinds.contains(&kind)
                } else if let Some(types) = value.as_array() {
                    !types.is_empty()
                        && types
                            .iter()
                            .all(|v| v.as_str().is_some_and(|k| kinds.contains(&k)))
                        && types.iter().collect::<std::collections::HashSet<_>>().len()
                            == types.len()
                } else {
                    false
                }
            }
            "properties" => value.as_object().is_some_and(|p| p.values().all(schema_ok)),
            "required" => value.as_array().is_some_and(|keys| {
                keys.iter()
                    .all(|k| k.as_str().is_some_and(|k| s["properties"].get(k).is_some()))
                    && keys.iter().collect::<std::collections::HashSet<_>>().len() == keys.len()
            }),
            "items" => schema_ok(value),
            "additionalProperties" => value == &json!(false),
            "minimum" | "maximum" => bound(value).is_some(),
            "minItems" | "maxItems" | "minLength" => value.as_u64().is_some(),
            "pattern" => value
                .as_str()
                .is_some_and(|p| [RULE_ID_PATTERN, HEADER_NAME_PATTERN].contains(&p)),
            "enum" => value.as_array().is_some_and(|variants| {
                !variants.is_empty()
                    && variants
                        .iter()
                        .enumerate()
                        .all(|(i, v)| !variants[..i].contains(v))
            }),
            "const" => true,
            "oneOf" | "anyOf" => value
                .as_array()
                .is_some_and(|arms| !arms.is_empty() && arms.iter().all(schema_ok)),
            "title" | "description" => value.is_string(),
            "default" => true,
            _ => false,
        };
        if !ok {
            return false;
        }
    }
    if let (Some(min), Some(max)) = (s.get("minimum"), s.get("maximum"))
        && bound(min) > bound(max)
    {
        return false;
    }
    o.contains_key("type") || o.contains_key("anyOf") || o.contains_key("oneOf")
}
fn evaluate(s: &Value, v: &Value) -> bool {
    if let Some(kind) = s.get("type") {
        let matches = |kind: &str| match kind {
            "object" => v.is_object(),
            "array" => v.is_array(),
            "string" => v.is_string(),
            "integer" => {
                v.as_i64().is_some()
                    || v.as_u64().is_some()
                    || v.as_f64().is_some_and(|n| n.fract() == 0.0)
            }
            "boolean" => v.is_boolean(),
            "null" => v.is_null(),
            _ => false,
        };
        if !if let Some(kind) = kind.as_str() {
            matches(kind)
        } else {
            kind.as_array()
                .unwrap()
                .iter()
                .any(|k| matches(k.as_str().unwrap()))
        } {
            return false;
        }
    }
    if s.get("const").is_some_and(|c| c != v) {
        return false;
    }
    if s["enum"]
        .as_array()
        .is_some_and(|variants| !variants.contains(v))
    {
        return false;
    }
    for (key, refused) in [
        ("minimum", std::cmp::Ordering::Less),
        ("maximum", std::cmp::Ordering::Greater),
    ] {
        if let Some(limit) = s.get(key)
            && compare(v, bound(limit).unwrap()).is_none_or(|o| o == refused)
        {
            return false;
        }
    }
    if let Some(string) = v.as_str() {
        if s["minLength"]
            .as_u64()
            .is_some_and(|n| (string.chars().count() as u64) < n)
        {
            return false;
        }
        if s["pattern"]
            .as_str()
            .is_some_and(|p| !pattern_matches(p, string))
        {
            return false;
        }
    }
    if let Some(object) = v.as_object() {
        if let Some(required) = s["required"].as_array()
            && required
                .iter()
                .any(|k| !object.contains_key(k.as_str().unwrap()))
        {
            return false;
        }
        for (key, value) in object {
            match s["properties"].get(key) {
                Some(property) if !evaluate(property, value) => return false,
                None if s["additionalProperties"] == false => return false,
                _ => {}
            }
        }
    }
    if let Some(array) = v.as_array() {
        if s["minItems"]
            .as_u64()
            .is_some_and(|n| (array.len() as u64) < n)
        {
            return false;
        }
        if s["maxItems"]
            .as_u64()
            .is_some_and(|n| (array.len() as u64) > n)
        {
            return false;
        }
        if let Some(items) = s.get("items")
            && !array.iter().all(|v| evaluate(items, v))
        {
            return false;
        }
    }
    for (key, exactly_one) in [("oneOf", true), ("anyOf", false)] {
        if let Some(arms) = s[key].as_array() {
            let count = arms.iter().filter(|s| evaluate(s, v)).count();
            if count == 0 || (exactly_one && count != 1) {
                return false;
            }
        }
    }
    true
}
pub(super) fn valid(s: &Value, v: &Value) -> bool {
    schema_ok(s) && evaluate(s, v)
}
fn number(literal: &str) -> Value {
    serde_json::from_str(literal).unwrap()
}

#[test]
fn evaluator_selftest() {
    let schema = json!({"type":"object","properties":{
        "x":{"type":"array","minItems":1,"items":{"type":"integer","minimum":0,"maximum":100}},
        "yes":{"type":"boolean","const":true}, "maybe":{"type":["string","null"]}
    }, "required":["x","yes"],"additionalProperties":false});
    for value in [
        json!({"x":[0,100,1.0],"yes":true}),
        json!({"x":[42],"yes":true,"maybe":null}),
        json!({"x":[42],"yes":true,"maybe":"雪"}),
    ] {
        assert!(valid(&schema, &value));
    }
    for value in [
        json!({}),
        json!({"x":[],"yes":true}),
        json!({"x":[-1],"yes":true}),
        json!({"x":[101],"yes":true}),
        json!({"x":[1.5],"yes":true}),
        json!({"x":["1"],"yes":true}),
        json!({"x":[1],"yes":false}),
        json!({"x":[1],"yes":true,"raw":"leak"}),
        json!({"x":[1],"yes":true,"maybe":1}),
        json!([]),
    ] {
        assert!(!valid(&schema, &value), "{value}");
    }
    assert!(valid(
        &json!({"type":"integer","minimum":0}),
        &json!(u64::MAX)
    ));
    assert!(!valid(
        &json!({"type":"integer","maximum":100}),
        &json!(u64::MAX)
    ));
    assert!(valid(
        &json!({"anyOf":[{"type":"string"},{"type":"null"}]}),
        &Value::Null
    ));
    assert!(!valid(
        &json!({"oneOf":[{"type":"integer"},{"type":"integer"}]}),
        &json!(1)
    ));

    // enum: membership by JSON equality, case-sensitive, nothing coerced.
    let severity = json!({"type":"string","enum":["info","warning","error"]});
    for good in ["info", "warning", "error"] {
        assert!(valid(&severity, &json!(good)));
    }
    for bad in [
        json!("Error"),
        json!("fatal"),
        json!(""),
        json!(0),
        Value::Null,
    ] {
        assert!(!valid(&severity, &bad), "{bad}");
    }
    // maxItems / minItems: inclusive counts.
    let pair = json!({"type":"array","minItems":1,"maxItems":2,"items":{"type":"integer"}});
    assert!(valid(&pair, &json!([1, 2])));
    assert!(!valid(&pair, &json!([1, 2, 3])));
    assert!(!valid(&pair, &json!([])));
    // minLength counts characters, not UTF-8 bytes: "雪" is three bytes.
    let two = json!({"type":"string","minLength":2});
    assert!(!valid(&two, &json!("雪")));
    assert!(valid(&two, &json!("雪雪")));
    assert!(valid(&two, &json!("ab")));
    assert!(!valid(&json!({"type":"string","minLength":1}), &json!("")));

    // The two admitted patterns, against the regex crate's reading of the
    // same source text, so the finite matchers cannot drift from it.
    let samples = [
        "a",
        "x-swarmail-demo",
        "message-id",
        "From",
        "a:b",
        "a b",
        "",
        "~",
        "!",
        "[",
        "@",
        "\u{7f}",
        "é",
        "a\n",
        "bulk.one_click.v1",
        "\t",
        "9;",
    ];
    for pattern in [RULE_ID_PATTERN, HEADER_NAME_PATTERN] {
        let re = regex::Regex::new(pattern).unwrap();
        let schema = json!({"type":"string","pattern":pattern});
        for sample in samples {
            assert_eq!(
                valid(&schema, &json!(sample)),
                re.is_match(sample),
                "{pattern} {sample:?}"
            );
        }
    }
    let name = json!({"type":"string","pattern":HEADER_NAME_PATTERN});
    for good in ["x-swarmail-demo", "list-unsubscribe", "a;b", "@[`{~"] {
        assert!(valid(&name, &json!(good)), "{good}");
    }
    for bad in ["From", "a:b", "a b", "", "é", "Z"] {
        assert!(!valid(&name, &json!(bad)), "{bad}");
    }
    let rule = json!({"type":"string","pattern":RULE_ID_PATTERN});
    assert!(valid(&rule, &json!("A:Z~")));
    for bad in ["a b", "", "é", "a\u{7f}"] {
        assert!(!valid(&rule, &json!(bad)), "{bad:?}");
    }

    // Exact integer bounds: u64 coordinates, signed i64 ends and 2^53.
    let coordinate = json!({"type":"integer","minimum":0,"maximum":u64::MAX});
    for good in [
        json!(0),
        json!(i64::MAX),
        json!(u64::MAX),
        json!(u64::MAX - 1),
        number("0.0"),
        number("9007199254740992.0"),
        number("18446744073709549568.0"),
    ] {
        assert!(valid(&coordinate, &good), "{good}");
    }
    for bad in [
        json!(-1),
        json!(i64::MIN),
        number("18446744073709551616"),
        number("18446744073709551616.0"),
        // The nearest double to u64::MAX is 2^64: a float never rounds in.
        number("18446744073709551615.0"),
        number("1e300"),
        number("-1e300"),
        number("-0.5"),
        number("1.5"),
        json!("1"),
        Value::Null,
    ] {
        assert!(!valid(&coordinate, &bad), "{bad}");
    }
    let edge =
        json!({"type":"integer","minimum":9007199254740991u64,"maximum":9007199254740992u64});
    assert!(valid(&edge, &json!(9007199254740992u64)));
    assert!(valid(&edge, &number("9007199254740992.0")));
    assert!(!valid(&edge, &json!(9007199254740993u64)));
    assert!(!valid(&edge, &json!(9007199254740990u64)));
    let signed = json!({"type":"integer","minimum":i64::MIN,"maximum":i64::MAX});
    assert!(valid(&signed, &json!(i64::MIN)));
    assert!(valid(&signed, &json!(i64::MAX)));
    assert!(!valid(&signed, &json!(i64::MAX as u64 + 1)));
    assert!(!valid(&signed, &number("-9223372036854777856.0")));
    assert!(!valid(&signed, &number("9223372036854775808.0")));
    // Fractions compare exactly against neighbouring integer bounds (the
    // emitted vocabulary has no number type, so the comparison is direct).
    use std::cmp::Ordering::{Equal, Greater, Less};
    for (value, bound, ordering) in [
        ("4.5", 5, Less),
        ("4.5", 4, Greater),
        ("-4.5", -5, Greater),
        ("-4.5", -4, Less),
        ("4.0", 4, Equal),
        ("-0.0", 0, Equal),
        ("18446744073709551615.0", i128::from(u64::MAX), Greater),
        ("1e300", i128::MAX, Greater),
        ("-1e300", i128::MIN, Less),
    ] {
        assert_eq!(compare(&number(value), bound), Some(ordering), "{value}");
    }
    assert_eq!(compare(&json!("4"), 4), None);
    assert_eq!(compare(&json!(u64::MAX), i128::from(u64::MAX)), Some(Equal));
    assert_eq!(compare(&json!(i64::MIN), i128::from(i64::MIN)), Some(Equal));
    assert!(!valid(
        &json!({"type":["integer","string"],"maximum":4}),
        &json!("4")
    ));

    for bad in [
        json!({}),
        json!(true),
        json!({"type":"bogus"}),
        json!({"type":[]}),
        json!({"type":["string","string"]}),
        json!({"type":42}),
        json!({"type":"string","pattern":"x"}),
        json!({"type":"string","pattern":"^[!-~]*$"}),
        json!({"type":"string","pattern":1}),
        json!({"type":"array","items":false}),
        json!({"type":"array","minItems":-1}),
        json!({"type":"array","maxItems":-1}),
        json!({"type":"array","maxItems":2.0}),
        json!({"type":"string","minLength":"1"}),
        json!({"type":"string","minLength":1.0}),
        json!({"type":"string","maxLength":1}),
        json!({"type":"string","format":"uuid"}),
        json!({"type":"string","enum":[]}),
        json!({"type":"string","enum":"info"}),
        json!({"type":"string","enum":["info","info"]}),
        json!({"type":"integer","minimum":"0"}),
        json!({"type":"integer","minimum":0.0}),
        json!({"type":"integer","maximum":number("18446744073709551616")}),
        json!({"type":"integer","minimum":2,"maximum":1}),
        json!({"type":"integer","minimum":u64::MAX,"maximum":i64::MAX}),
        json!({"type":"object","required":["absent"]}),
        json!({"type":"object","properties":[]}),
        json!({"type":"object","additionalProperties":true}),
        json!({"type":"string","description":1}),
        json!({"type":"string","examples":["x"]}),
        json!({"oneOf":[{"type":"string"}],"discriminator":{"propertyName":"kind"}}),
        json!({"anyOf":[]}),
        json!({"anyOf":[{"type":"string"},{"type":"null","unknown":true}]}),
        json!({"oneOf":[{"type":"string"},{"type":"null","maxLength":1}]}),
        json!({"oneOf":[{"type":"string"},{"type":"string","pattern":"^x$"}]}),
        json!({"$ref":"https://external"}),
        json!({"$ref":"#/$defs/missing"}),
        json!({"$ref":"#","$defs":{"cycle":{"$ref":"#"}}}),
        json!({"type":"object","properties":{"x":{"$ref":"#/components/schemas/DeliverabilitySeverity"}}}),
    ] {
        assert!(!valid(&bad, &json!("data")), "accepted {bad}");
    }
}

/// Strip the modern-only descriptor members, checking each, leaving the
/// March descriptor the independent baseline served.
pub(super) fn march_view(tool: &Value, read_only: bool) -> Value {
    let mut descriptor = tool.clone();
    let o = descriptor.as_object_mut().unwrap();
    assert_eq!(o.remove("outputSchema").unwrap()["type"], "object");
    assert_eq!(
        o.remove("annotations").unwrap(),
        if read_only {
            json!({"readOnlyHint":true})
        } else {
            json!({"readOnlyHint":false,"destructiveHint":true})
        }
    );
    descriptor
}

#[tokio::test]
async fn modern_all_tools_http_and_stdio() {
    let baseline: Vec<&str> = BASELINE.lines().collect();
    let requests: Vec<&str> = REQUESTS.lines().collect();
    let march: Value = serde_json::from_str(baseline[0]).unwrap();
    let reads = [0, 1, 2, 3, 7, 8, 9];
    for version in &VERSIONS[1..] {
        for stdio in [false, true] {
            let run = workflow(Some(version), stdio, true).await;
            assert_eq!(run.requests, requests, "case identities");
            assert_eq!(run.lines.len(), baseline.len());
            let list: Value = serde_json::from_str(&run.lines[0]).unwrap();
            let tools = list["result"]["tools"].as_array().unwrap();
            let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
            assert_eq!(names[..13], NAMES);
            assert_eq!(names[13..], super::focused::FOCUSED);
            for tool in &tools[13..] {
                super::focused::assert_march_descriptor(&march_view(tool, true));
            }
            for (i, (tool, original)) in tools
                .iter()
                .zip(march["result"]["tools"].as_array().unwrap())
                .enumerate()
            {
                let mut descriptor = march_view(tool, reads.contains(&i));
                let description = descriptor["description"].as_str().unwrap();
                if i == 1 {
                    for clause in [
                        "swarmail-bulk/1",
                        "bulk-sender checklist",
                        "demo marker",
                        "not_evaluated",
                        "256 findings",
                        "65536 compact JSON bytes",
                        "structuredContent",
                    ] {
                        assert!(description.contains(clause), "{clause}: {description}");
                    }
                    assert!(!description.contains("Only the synthetic demo rule ships"));
                    assert!(!description.contains("2025-03-26"));
                    descriptor["description"] = original["description"].clone();
                } else if i == 7 {
                    assert!(!description.contains("batch"));
                    descriptor["description"] = original["description"].clone();
                }
                assert_eq!(&descriptor, original);
            }
            // Errors keep March's bytes; a success is March's line plus the
            // same JSON as structuredContent, byte for byte.
            for (raw, base) in run.lines.iter().zip(&baseline).skip(1) {
                let expected: Value = serde_json::from_str(base).unwrap();
                if expected.get("error").is_some() || expected["result"]["isError"] == true {
                    assert_eq!(raw, base);
                    continue;
                }
                let text = expected["result"]["content"][0]["text"].as_str().unwrap();
                let tail = base.strip_suffix("}}").unwrap();
                assert_eq!(raw, &format!("{tail},\"structuredContent\":{text}}}}}"));
                let actual: Value = serde_json::from_str(raw).unwrap();
                assert_eq!(
                    actual["result"]["structuredContent"],
                    serde_json::from_str::<Value>(text).unwrap()
                );
            }
        }
    }
}

// Schema-aware removal of annotation-only keywords: property names remain data.
pub(super) fn structural(s: &Value) -> Value {
    let mut s = s.clone();
    let o = s.as_object_mut().unwrap();
    for k in ["description", "title", "default"] {
        o.remove(k);
    }
    if let Some(properties) = o.get_mut("properties") {
        for property in properties.as_object_mut().unwrap().values_mut() {
            *property = structural(property);
        }
    }
    if let Some(items) = o.get_mut("items") {
        *items = structural(items);
    }
    for key in ["anyOf", "oneOf"] {
        if let Some(arms) = o.get_mut(key) {
            for arm in arms.as_array_mut().unwrap() {
                *arm = structural(arm);
            }
        }
    }
    for key in ["required", "type"] {
        if let Some(Value::Array(values)) = o.get_mut(key) {
            values.sort_by_key(Value::to_string);
        }
    }
    s
}
fn source_matches(actual: &Value, expected: &Value) -> bool {
    structural(actual) == structural(expected)
}
fn resolve(s: &Value, components: &Value) -> Value {
    if let Some(reference) = s["$ref"].as_str() {
        assert_eq!(s.as_object().unwrap().len(), 1);
        let name = reference.strip_prefix("#/components/schemas/").unwrap();
        return resolve(&components[name], components);
    }
    let mut s = structural(s);
    if let Some(p) = s.get_mut("properties") {
        for v in p.as_object_mut().unwrap().values_mut() {
            *v = resolve(v, components);
        }
    }
    if let Some(items) = s.get_mut("items") {
        *items = resolve(items, components);
    }
    for key in ["anyOf", "oneOf"] {
        if let Some(a) = s.get_mut(key) {
            for v in a.as_array_mut().unwrap() {
                *v = resolve(v, components);
            }
        }
    }
    s
}
fn close(mut s: Value, required: Value) -> Value {
    s["additionalProperties"] = json!(false);
    s["required"] = required;
    s
}
fn wrapper(properties: Value, required: Value) -> Value {
    close(json!({"type":"object","properties":properties}), required)
}
/// The report needs no closing or requiredness: the source already carries
/// both. Only three classified annotations move, each checked as removed.
fn report_projection(c: &Value) -> Value {
    let mut report = resolve(&c["DeliverabilityReport"], c);
    assert_eq!(
        report["properties"]["rule_set_id"]
            .as_object_mut()
            .unwrap()
            .remove("examples"),
        Some(json!(["swarmail-bulk/1"]))
    );
    let location = &mut report["properties"]["findings"]["items"]["properties"]["location"];
    // oneOf over exclusive const tags carries the whole dispatch.
    assert_eq!(
        location.as_object_mut().unwrap().remove("discriminator"),
        Some(json!({"propertyName":"kind"}))
    );
    for arm in location["oneOf"].as_array_mut().unwrap() {
        let kind = &mut arm["properties"]["kind"];
        assert!(kind.get("type").is_none() && kind["const"].is_string());
        kind["type"] = json!("string");
    }
    report
}
fn projections(c: &Value) -> Value {
    let mut address = close(resolve(&c["EmailAddress"], c), json!(["address"]));
    address["properties"]["name"]["type"] = json!("string");
    let mut email = close(
        resolve(&c["Email"], c),
        json!([
            "id",
            "inbox",
            "envelope_from",
            "to",
            "received_at",
            "received_ms",
            "size"
        ]),
    );
    email["properties"]["from"] = address.clone();
    email["properties"]["to"]["items"] = address.clone();
    email["properties"]["cc"]["items"] = address;
    for key in ["subject", "text", "html"] {
        email["properties"][key]["type"] = json!("string");
    }
    email["properties"]["size"]["minimum"] = json!(0);
    let mut list = close(resolve(&c["EmailList"], c), json!(["total", "emails"]));
    list["properties"]["total"]["minimum"] = json!(0);
    list["properties"]["emails"]["items"] = email.clone();
    let mut search = list.clone();
    search["properties"]["inbox"] = json!({"type":"string"});
    search["required"] = json!(["inbox", "total", "emails"]);
    let mut latest_emails = list["properties"]["emails"].clone();
    latest_emails["minItems"] = json!(1);
    let mut found = close(
        resolve(&c["AwaitResponse"], c),
        json!(["matched", "total", "emails"]),
    );
    for key in ["matched", "total"] {
        found["properties"][key]["minimum"] = json!(0);
    }
    found["properties"]["emails"] = latest_emails.clone();
    let mut timeout = found.clone();
    timeout["properties"]
        .as_object_mut()
        .unwrap()
        .remove("emails");
    timeout["properties"]["timed_out"] = json!({"type":"boolean","const":true});
    timeout["required"] = json!(["matched", "total", "timed_out"]);
    let mut chaos = close(
        resolve(&c["ChaosConfig"], c),
        json!(["connect", "mail_from", "rcpt", "data"]),
    );
    for event in ["connect", "mail_from", "rcpt", "data"] {
        chaos["properties"][event]["anyOf"][0]["required"] = json!(["probability", "delay_ms"]);
    }
    json!({
        "swarmail_search_emails": search,
        "swarmail_get_deliverability_report": report_projection(c),
        "swarmail_get_email": email,
        "swarmail_get_latest_email":wrapper(json!({"emails":latest_emails}),json!(["emails"])),
        "swarmail_wait_for_email":{"type":"object","oneOf":[found,timeout]},
        "swarmail_extract_links":wrapper(json!({"links":email["properties"]["links"]}),json!(["links"])),
        "swarmail_extract_codes":wrapper(json!({"codes":email["properties"]["codes"]}),json!(["codes"])),
        "swarmail_seed_email":wrapper(json!({"id":email["properties"]["id"],"links":email["properties"]["links"],"codes":email["properties"]["codes"]}),json!(["id","links","codes"])),
        "swarmail_set_chaos":chaos,
        "swarmail_delete_email":wrapper(json!({"deleted":{"type":"boolean","const":true}}),json!(["deleted"])),
        "swarmail_clear_inbox":wrapper(json!({"removed":{"type":"integer","minimum":0}}),json!(["removed"])),
        "swarmail_clear_all":wrapper(json!({"removed":{"type":"integer","minimum":0}}),json!(["removed"])),
        "swarmail_clear_chaos":wrapper(json!({"chaos":{"type":"string","const":"disabled"}}),json!(["chaos"]))
    })
}
async fn served_report_schema(addr: std::net::SocketAddr, version: &'static str) -> Value {
    let mut wire = Wire::Http(addr, Some(version));
    let list = wire.request(&json!({"id":1,"method":"tools/list"})).await;
    assert_eq!(list["result"]["tools"][1]["name"], NAMES[1]);
    list["result"]["tools"][1]["outputSchema"].clone()
}
#[tokio::test]
async fn served_openapi_pins_and_negative_controls() {
    let srv = start().await;
    let (_, api) = common::http_json(srv.http_addr, "GET", "/openapi.json", None).await;
    let c = &api["components"]["schemas"];
    let pinned: Value = serde_json::from_str(include_str!(
        "../fixtures/mcp-structured-output/openapi-source.json"
    ))
    .unwrap();
    let report_pins: Value = serde_json::from_str(include_str!(
        "../fixtures/mcp-structured-output/openapi-report-source.json"
    ))
    .unwrap();
    assert_eq!(pinned.as_object().unwrap().len(), 6);
    assert_eq!(report_pins.as_object().unwrap().len(), 4);
    for (name, expected) in pinned
        .as_object()
        .unwrap()
        .iter()
        .chain(report_pins.as_object().unwrap())
    {
        assert!(source_matches(&c[name], expected), "source changed: {name}");
    }
    let location = "DeliverabilityLocation";
    for (name, pointer, value) in [
        ("EmailAddress", "/properties/address/type", json!("integer")),
        ("Email", "/required", json!(["subject"])),
        ("Email", "/properties/envelope_from/type", json!("string")),
        ("ChaosRule", "/properties/delay_ms/maximum", json!(300001)),
        (
            "DeliverabilitySeverity",
            "/enum",
            json!(["info", "warning"]),
        ),
        (location, "/oneOf/0/properties/kind/const", json!("Header")),
        (location, "/oneOf/1/additionalProperties", json!(true)),
        (location, "/oneOf/0/required", json!(["kind", "name"])),
        (
            location,
            "/oneOf/0/properties/name/pattern",
            json!(RULE_ID_PATTERN),
        ),
        (
            location,
            "/oneOf/2/properties/byte_offset/maximum",
            json!(i64::MAX),
        ),
        (location, "/oneOf/1/properties/path/items/minimum", json!(1)),
        (location, "/discriminator/propertyName", json!("type")),
        (
            "DeliverabilityFinding",
            "/required",
            json!(["rule_id", "severity", "location"]),
        ),
        (
            "DeliverabilityFinding",
            "/properties/rule_id/minLength",
            json!(0),
        ),
        (
            "DeliverabilityFinding",
            "/properties/fix_hint/minLength",
            json!(0),
        ),
        (
            "DeliverabilityFinding",
            "/additionalProperties",
            json!(true),
        ),
        (
            "DeliverabilityReport",
            "/properties/findings/maxItems",
            json!(257),
        ),
        (
            "DeliverabilityReport",
            "/properties/findings/items",
            json!({"type":"object"}),
        ),
        (
            "DeliverabilityReport",
            "/properties/rule_set_id/examples",
            json!(["demo/1"]),
        ),
    ] {
        let mut drift = c[name].clone();
        // Email publishes no required list; the drift adds one.
        match drift.pointer_mut(pointer) {
            Some(slot) => *slot = value,
            None => drift[pointer.strip_prefix('/').unwrap()] = value,
        }
        let pins = if name.starts_with("Deliverability") {
            &report_pins
        } else {
            &pinned
        };
        assert!(!source_matches(&drift, &pins[name]), "{name} {pointer}");
    }
    let expected = projections(c);
    for version in &VERSIONS[1..] {
        let mut wire = Wire::Http(srv.http_addr, Some(version));
        let catalogue = wire.request(&json!({"id":1,"method":"tools/list"})).await;
        let tools = catalogue["result"]["tools"].as_array().unwrap();
        let served: std::collections::BTreeSet<&str> =
            tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        let projected: std::collections::BTreeSet<&str> = expected
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        let focused: std::collections::BTreeSet<&str> =
            super::focused::FOCUSED.into_iter().collect();
        assert_eq!(served, projected.union(&focused).copied().collect());
        assert_eq!(
            served,
            NAMES.into_iter().chain(focused.iter().copied()).collect()
        );
        for tool in tools {
            let name = tool["name"].as_str().unwrap();
            let schema = &tool["outputSchema"];
            assert!(schema_ok(schema), "{name}");
            assert_eq!(schema["type"], "object");
            // The focused waits have no REST counterpart to pin; their
            // independent expectation is decision 0094's stated contract.
            let expected = if focused.contains(name) {
                super::focused::expected_output(name)
            } else {
                expected[name].clone()
            };
            assert_eq!(structural(schema), structural(&expected), "{name}");
            assert!(!valid(schema, &json!({})), "vacuous {name}");
        }
    }
    // Each report constraint is load-bearing in the equality above.
    let report = &expected["swarmail_get_deliverability_report"];
    let served = served_report_schema(srv.http_addr, VERSIONS[2]).await;
    let finding = "/properties/findings/items";
    let location = "/properties/findings/items/properties/location";
    for (pointer, value) in [
        (
            format!("{finding}/properties/severity/enum"),
            json!(["info", "warning"]),
        ),
        (
            format!("{location}/oneOf/2/properties/kind/const"),
            json!("Link"),
        ),
        (
            format!("{location}/oneOf/0/required"),
            json!(["kind", "name"]),
        ),
        (
            format!("{location}/oneOf/1/additionalProperties"),
            Value::Null,
        ),
        (
            format!("{location}/oneOf/0/properties/name/pattern"),
            json!(RULE_ID_PATTERN),
        ),
        (
            format!("{location}/oneOf/0/properties/occurrence/anyOf/0/maximum"),
            json!(i64::MAX),
        ),
        (
            format!("{finding}/properties/rule_id/pattern"),
            json!(HEADER_NAME_PATTERN),
        ),
        (
            format!("{finding}/required"),
            json!(["rule_id", "severity", "location"]),
        ),
        ("/properties/findings/maxItems".into(), json!(257)),
        ("/required".into(), json!(["rule_set_id"])),
        (format!("{location}/oneOf"), json!([])),
    ] {
        let mut drift = report.clone();
        *drift.pointer_mut(&pointer).unwrap() = value;
        assert_ne!(structural(&served), structural(&drift), "{pointer}");
    }
    let mut wrapped = report.clone();
    let arms = wrapped.pointer(location).unwrap()["oneOf"].clone();
    *wrapped.pointer_mut(location).unwrap() = json!({"type":"object","anyOf":arms});
    assert_ne!(structural(&served), structural(&wrapped));

    let schema = &expected["swarmail_get_email"];
    let email = serde_json::to_value(historical()).unwrap();
    assert!(valid(schema, &email));
    for key in schema["required"].as_array().unwrap() {
        let mut bad = email.clone();
        bad.as_object_mut().unwrap().remove(key.as_str().unwrap());
        assert!(!valid(schema, &bad));
    }
    for (pointer, value) in [
        ("/raw", json!("raw")),
        ("/extra", json!(1)),
        ("/from/name", Value::Null),
        ("/to/0/address", json!(4)),
        ("/to", json!("array?")),
        ("/subject", Value::Null),
        ("/size", json!(-1)),
        ("/received_ms", json!(1.5)),
    ] {
        let mut bad = email.clone();
        if let Some(slot) = bad.pointer_mut(pointer) {
            *slot = value;
        } else {
            bad[pointer.strip_prefix('/').unwrap()] = value;
        }
        assert!(!valid(schema, &bad), "{pointer}");
    }
    let wait = &expected["swarmail_wait_for_email"];
    assert!(
        valid(wait, &json!({"matched":1,"total":0,"emails":[email]})),
        "no invented count equality"
    );
    assert!(valid(
        wait,
        &json!({"matched":1,"total":0,"timed_out":true})
    ));
    for bad in [
        json!({"matched":0,"total":0}),
        json!({"matched":0,"total":0,"emails":[]}),
        json!({"matched":0,"total":0,"timed_out":false}),
        json!({"matched":0,"total":0,"timed_out":true,"emails":[email]}),
        json!({"matched":-1,"total":0,"timed_out":true}),
    ] {
        assert!(!valid(wait, &bad));
    }
    for (name, bad) in [
        ("swarmail_delete_email", json!({"deleted":false})),
        ("swarmail_clear_all", json!({"removed":-1})),
        ("swarmail_clear_chaos", json!({"chaos":"enabled"})),
        ("swarmail_seed_email", json!({"id":"id","links":[]})),
        ("swarmail_extract_codes", json!({"codes":[1]})),
        (
            "swarmail_set_chaos",
            json!({"connect":{"probability":101,"delay_ms":0},"mail_from":null,"rcpt":null,"data":null}),
        ),
        (
            "swarmail_set_chaos",
            json!({"connect":{"probability":0,"delay_ms":300001},"mail_from":null,"rcpt":null,"data":null}),
        ),
        (
            "swarmail_get_deliverability_report",
            json!({"rule_set_id":"r","findings":[{}]}),
        ),
    ] {
        assert!(!valid(&expected[name], &bad), "{name}");
    }
    srv.stop().await;
}

/// Typed-only evidence: the production collectors never emit a Link location,
/// an empty report or extreme coordinates, so these are serialization witnesses
/// of the public report types against the served schema, not wire receipts.
#[tokio::test]
async fn report_schema_typed_forms_and_negatives() {
    use swarmail::deliverability::{Finding, Location, Report, Severity};
    let srv = start().await;
    let schema = served_report_schema(srv.http_addr, VERSIONS[1]).await;
    assert_eq!(
        schema,
        served_report_schema(srv.http_addr, VERSIONS[2]).await
    );
    assert!(schema_ok(&schema));
    let forms = [
        (
            Location::Header {
                name: "x-swarmail-demo".into(),
                occurrence: Some(0),
            },
            json!({"kind":"header","name":"x-swarmail-demo","occurrence":0}),
        ),
        (
            Location::Header {
                name: "from".into(),
                occurrence: None,
            },
            json!({"kind":"header","name":"from","occurrence":null}),
        ),
        (
            Location::Header {
                name: "~".into(),
                occurrence: Some(u64::MAX),
            },
            json!({"kind":"header","name":"~","occurrence":u64::MAX}),
        ),
        (
            Location::MimePart { path: vec![] },
            json!({"kind":"mime_part","path":[]}),
        ),
        (
            Location::MimePart {
                path: vec![0, u64::MAX],
            },
            json!({"kind":"mime_part","path":[0,u64::MAX]}),
        ),
        (
            Location::Link {
                part_path: vec![],
                byte_offset: 0,
            },
            json!({"kind":"link","part_path":[],"byte_offset":0}),
        ),
        (
            Location::Link {
                part_path: vec![u64::MAX, 0],
                byte_offset: u64::MAX,
            },
            json!({"kind":"link","part_path":[u64::MAX,0],"byte_offset":u64::MAX}),
        ),
    ];
    let severities = [Severity::Info, Severity::Warning, Severity::Error];
    let mut findings = Vec::new();
    for (i, (location, wire)) in forms.into_iter().enumerate() {
        let finding = Finding {
            rule_id: "typed.v1".into(),
            location,
            severity: severities[i % 3],
            fix_hint: "雪".into(),
        };
        let value = serde_json::to_value(&finding).unwrap();
        let severity = ["info", "warning", "error"][i % 3];
        assert_eq!(
            value,
            json!({"rule_id":"typed.v1","location":wire,"severity":severity,"fix_hint":"雪"})
        );
        findings.push(finding);
    }
    let empty = serde_json::to_value(Report {
        rule_set_id: "swarmail-bulk/1".into(),
        findings: vec![],
    })
    .unwrap();
    assert_eq!(
        empty,
        json!({"rule_set_id":"swarmail-bulk/1","findings":[]})
    );
    assert!(valid(&schema, &empty));
    // No invented minimum length or constant for the rule-set ID.
    let full = serde_json::to_value(Report {
        rule_set_id: String::new(),
        findings,
    })
    .unwrap();
    assert!(valid(&schema, &full), "{full}");
    let base = full["findings"][0].clone();
    let at = |n: usize| json!({"rule_set_id":"r","findings":vec![base.clone(); n]});
    assert!(valid(&schema, &at(256)));
    assert!(!valid(&schema, &at(257)));

    let refuse = |value: Value, label: &str| assert!(!valid(&schema, &value), "accepted {label}");
    let with_location = |location: Value| {
        let mut finding = base.clone();
        finding["location"] = location;
        json!({"rule_set_id":"r","findings":[finding]})
    };
    let with_finding = |key: &str, value: Option<Value>| {
        let mut finding = base.clone();
        match value {
            Some(value) => finding[key] = value,
            None => {
                finding.as_object_mut().unwrap().remove(key).unwrap();
            }
        }
        json!({"rule_set_id":"r","findings":[finding]})
    };
    refuse(json!({"findings":[]}), "no rule_set_id");
    refuse(json!({"rule_set_id":"r"}), "no findings");
    refuse(
        json!({"rule_set_id":"r","findings":[],"extra":1}),
        "report extra",
    );
    refuse(json!({"rule_set_id":1,"findings":[]}), "numeric rule set");
    refuse(json!({"rule_set_id":"r","findings":null}), "null findings");
    for key in ["rule_id", "severity", "location", "fix_hint"] {
        refuse(with_finding(key, None), key);
    }
    for (key, value) in [
        ("extra", json!(1)),
        ("rule_id", json!("")),
        ("rule_id", json!("a b")),
        ("rule_id", json!("雪")),
        ("fix_hint", json!("")),
        ("fix_hint", json!(1)),
        ("severity", json!("fatal")),
        ("severity", json!("Error")),
        ("severity", Value::Null),
    ] {
        refuse(
            with_finding(key, Some(value.clone())),
            &format!("{key}={value}"),
        );
    }
    let overflow = number("18446744073709551616");
    for location in [
        json!({"kind":"header","name":"from"}),
        json!({"kind":"header","occurrence":0}),
        json!({"name":"from","occurrence":0}),
        json!({"kind":"header","name":"from","occurrence":0,"path":[]}),
        json!({"kind":"header","name":"From","occurrence":0}),
        json!({"kind":"header","name":"a:b","occurrence":0}),
        json!({"kind":"header","name":"","occurrence":0}),
        json!({"kind":"header","name":"from","occurrence":-1}),
        json!({"kind":"header","name":"from","occurrence":1.5}),
        json!({"kind":"header","name":"from","occurrence":overflow}),
        json!({"kind":"header","name":"from","occurrence":"0"}),
        json!({"kind":"Header","name":"from","occurrence":0}),
        json!({"kind":"mime_part"}),
        json!({"kind":"mime_part","path":[],"name":"from"}),
        json!({"kind":"mime_part","path":[-1]}),
        json!({"kind":"mime_part","path":[0.5]}),
        json!({"kind":"mime_part","path":[overflow]}),
        json!({"kind":"mime_part","path":0}),
        json!({"kind":"mime_part","part_path":[],"byte_offset":0}),
        json!({"kind":"link","part_path":[]}),
        json!({"kind":"link","byte_offset":0}),
        json!({"kind":"link","part_path":[],"byte_offset":-1}),
        json!({"kind":"link","part_path":[],"byte_offset":overflow}),
        json!({"kind":"link","part_path":[],"byte_offset":0,"url":"https://x.io"}),
        json!({"kind":"link","path":[],"byte_offset":0}),
        json!({"kind":"other"}),
        json!([{"kind":"header","name":"from","occurrence":0}]),
        Value::Null,
    ] {
        refuse(with_location(location.clone()), &location.to_string());
    }
    srv.stop().await;
}

/// Wire receipts for the production-reachable forms: real SMTP bytes, the
/// demo marker at every severity, header occurrence 0 and null and the root
/// MIME path, over HTTP and the real bridge on both modern revisions.
#[tokio::test]
async fn report_stored_bytes_over_modern_wire() {
    let srv = start().await;
    let plain_text = finding(
        "bulk.plain_text.v1",
        "warning",
        json!({"kind":"mime_part","path":[]}),
        "Include a sender-supplied text/plain body. This is checklist advice, not proof of equivalent content or an unconditional provider mandate.",
        "https://www.rfc-editor.org/rfc/rfc2046#section-5.1.4",
    );
    let demo = |severity: &str| {
        json!({"rule_id":"demo.marker.v1","severity":severity,
            "location":{"kind":"header","name":"x-swarmail-demo","occurrence":0},
            "fix_hint":"Remove the X-Swarmail-Demo header."})
    };
    let mut cases = Vec::new();
    for (raw, findings) in [
        (
            &b"From: s@x.io\r\nX-Swarmail-Demo: info\r\nContent-Type: text/html\r\n\r\n<p>x</p>\r\n"[..],
            [checklist(json!(0)), vec![plain_text, demo("info")]].concat(),
        ),
        (
            b"From: s@x.io\r\nX-Swarmail-Demo: warning\r\n\r\nbody\r\n",
            [checklist(json!(0)), vec![demo("warning")]].concat(),
        ),
        (
            b"X-Swarmail-Demo: error\r\n\r\nbody\r\n",
            [checklist(Value::Null), vec![demo("error")]].concat(),
        ),
    ] {
        let id = common::smtp_raw(srv.smtp_addr, "report-wire", raw).await;
        let path = format!("/api/v1/messages/{id}/deliverability");
        let rest = common::http_json(srv.http_addr, "GET", &path, None).await;
        assert_eq!(rest, (200, report_of(findings)));
        cases.push((id, rest.1));
    }
    for version in &VERSIONS[1..] {
        let schema = served_report_schema(srv.http_addr, version).await;
        for stdio in [false, true] {
            let mut wire = Wire::start(srv.http_addr, version, stdio).await;
            for (id, expected) in &cases {
                let reply = wire
                    .request(&call(
                        json!(5),
                        "get_deliverability_report",
                        json!({"id":id}),
                    ))
                    .await;
                assert_eq!(reply["result"]["isError"], false);
                assert_eq!(&reply["result"]["structuredContent"], expected);
                assert_eq!(&text(&reply), expected);
                assert!(valid(&schema, expected));
            }
            let missing = wire
                .request(&call(
                    json!(6),
                    "get_deliverability_report",
                    json!({"id":"gone"}),
                ))
                .await;
            assert_eq!(
                missing,
                json!({"jsonrpc":"2.0","id":6,"result":{"content":[{"type":"text","text":"message gone not found"}],"isError":true}})
            );
            wire.finish().await;
        }
    }
    let mut march = Wire::Http(srv.http_addr, None);
    let reply = march
        .request(&call(
            json!(7),
            "get_deliverability_report",
            json!({"id":cases[0].0}),
        ))
        .await;
    assert!(reply["result"].get("structuredContent").is_none());
    assert_eq!(text(&reply), cases[0].1);
    srv.stop().await;
}

#[tokio::test]
async fn real_bridge_dual_output_size_cost() {
    const CAP: usize = 10 * 1024 * 1024;
    let srv = start().await;
    let body = "x".repeat(600_000);
    for _ in 0..6 {
        assert!(
            smtp_send(
                srv.smtp_addr,
                Some("large"),
                "from@x.io",
                "to@x.io",
                "large",
                &body
            )
            .await
            .unwrap()
            .starts_with("250")
        );
    }
    let query = call(json!("large"), "search_emails", json!({"inbox":"large"}));
    for version in &VERSIONS[1..] {
        let mut binary = Binary::start(srv.http_addr);
        binary.ready(version).await;
        let mut wire = Wire::Stdio(Box::new(binary));
        let raw = wire.raw(&query).await;
        assert!(raw.len() < CAP && raw.len() > 7_000_000);
        let reply: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(text(&reply), reply["result"]["structuredContent"]);
        assert_eq!(text(&reply)["emails"].as_array().unwrap().len(), 6);
        wire.finish().await;
    }
    for _ in 0..4 {
        smtp_send(
            srv.smtp_addr,
            Some("large"),
            "from@x.io",
            "to@x.io",
            "large",
            &body,
        )
        .await
        .unwrap();
    }
    let mut legacy = Wire::Http(srv.http_addr, None);
    let raw = legacy.raw(&query).await;
    assert!(raw.len() < CAP && raw.len() > 6_000_000);
    let mut legacy_binary = Binary::start(srv.http_addr);
    legacy_binary.ready(VERSIONS[0]).await;
    let mut legacy_binary = Wire::Stdio(Box::new(legacy_binary));
    assert_eq!(legacy_binary.raw(&query).await, raw);
    legacy_binary.finish().await;
    for version in &VERSIONS[1..] {
        let mut http = Wire::Http(srv.http_addr, Some(version));
        assert!(http.raw(&query).await.len() > CAP);
        let mut binary = Binary::start(srv.http_addr);
        binary.ready(version).await;
        let mut wire = Wire::Stdio(Box::new(binary));
        let refusal = wire.request(&query).await;
        assert_eq!(
            refusal,
            json!({"jsonrpc":"2.0","id":"large","error":{"code":-32000,"message":"response exceeds the 10 MiB cap"}})
        );
        assert_eq!(
            wire.request(&json!({"id":"alive","method":"ping"})).await,
            json!({"jsonrpc":"2.0","id":"alive","result":{}})
        );
        wire.finish().await;
    }
    assert_eq!(
        srv.store
            .count("large", &swarmail::store::Filter::default()),
        10
    );
    srv.stop().await;
}

#[tokio::test]
async fn modern_storage_errors_and_revision_isolation() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("mail.db");
    let srv = common::start_with(|mut c| {
        c.data_file = Some(db.clone());
        c
    })
    .await;
    srv.store.insert(historical()).unwrap();
    let insert = common::refusing_trigger(&db, "insert");
    let delete = common::refusing_trigger(&db, "delete");
    for version in &VERSIONS[1..] {
        for stdio in [false, true] {
            let mut wire = if stdio {
                let mut binary = Binary::start(srv.http_addr);
                binary.ready(version).await;
                Wire::Stdio(Box::new(binary))
            } else {
                Wire::Http(srv.http_addr, Some(version))
            };
            for (name, args) in [
                ("seed_email", json!({"to":"a@x.io","inbox":"refused"})),
                ("delete_email", json!({"id":"historical"})),
                ("clear_inbox", json!({"inbox":"contract"})),
                ("clear_all", json!({})),
            ] {
                let result = wire.request(&call(json!(1), name, args)).await;
                assert_eq!(
                    result,
                    json!({"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"storage commit failed"}],"isError":true}})
                );
            }
            for (args, message) in [
                (
                    json!({"timeout_ms":null}),
                    "timeout_ms must be an integer between 0 and 60000",
                ),
                (
                    json!({"timeout_ms":60001}),
                    "timeout_ms must be at most 60000",
                ),
            ] {
                let result = wire.request(&call(json!(2), "wait_for_email", args)).await;
                assert_eq!(result["result"]["isError"], true);
                assert_eq!(result["result"]["content"][0]["text"], message);
                assert!(result["result"].get("structuredContent").is_none());
            }
            wire.finish().await;
        }
    }
    drop(insert);
    drop(delete);
    // A notification's admitted effect is visible in the next read; HTTP is
    // stateless and its last negotiated version cannot leak into missing headers.
    let mut notification = call(
        Value::Null,
        "seed_email",
        json!({"inbox":"notify","to":"a@x.io","html":"https://x.io/ 123456"}),
    );
    notification.as_object_mut().unwrap().remove("id");
    assert_eq!(
        post(
            srv.http_addr,
            &[("MCP-Protocol-Version", VERSIONS[1])],
            &notification
        )
        .await,
        (202, Value::Null)
    );
    let mut legacy = Binary::start(srv.http_addr);
    let mut modern = Binary::start(srv.http_addr);
    modern.ready(VERSIONS[2]).await;
    for version in [
        Some(VERSIONS[1]),
        None,
        Some(VERSIONS[2]),
        Some(VERSIONS[0]),
        None,
    ] {
        let mut wire = Wire::Http(srv.http_addr, version);
        let reply = wire
            .request(&call(json!(3), "search_emails", json!({"inbox":"notify"})))
            .await;
        assert_eq!(text(&reply)["total"], 1);
        assert_eq!(
            reply["result"].get("structuredContent").is_some(),
            matches!(version, Some("2025-06-18" | "2025-11-25"))
        );
    }
    for (binary, is_modern) in [(&mut legacy, false), (&mut modern, true)] {
        binary
            .send(&call(json!(4), "get_email", json!({"id":"historical"})))
            .await;
        assert_eq!(
            binary.reply().await["result"]
                .get("structuredContent")
                .is_some(),
            is_modern
        );
    }
    legacy.finish().await;
    modern.finish().await;
    assert_eq!(
        srv.store
            .count("refused", &swarmail::store::Filter::default()),
        0
    );
    srv.stop().await;
}

#[tokio::test]
async fn modern_parked_wait_shutdown_keeps_error_shape() {
    for version in &VERSIONS[1..] {
        for stdio in [false, true] {
            let srv = start().await;
            let mut wire = if stdio {
                let mut b = Binary::start(srv.http_addr);
                b.ready(version).await;
                Wire::Stdio(Box::new(b))
            } else {
                Wire::Http(srv.http_addr, Some(version))
            };
            let request = tokio::spawn(async move {
                let result = wire
                    .request(&call(
                        json!(9),
                        "wait_for_email",
                        json!({"inbox":"shutdown","timeout_ms":60000}),
                    ))
                    .await;
                wire.finish().await;
                result
            });
            common::wait_for_metric_row(
                srv.http_addr,
                "swarmail_awaits_active{inbox=\"shutdown\"}",
                1,
            )
            .await;
            tokio::time::timeout(Duration::from_secs(30), srv.stop())
                .await
                .unwrap();
            assert_eq!(
                request.await.unwrap(),
                json!({"jsonrpc":"2.0","id":9,"result":{"content":[{"type":"text","text":"shutting down"}],"isError":true}})
            );
        }
    }
}

#[tokio::test]
async fn modern_seed_nonempty_arrays_validate() {
    let srv = start().await;
    for version in &VERSIONS[1..] {
        let mut wire = Wire::Http(srv.http_addr, Some(version));
        let tools = wire.request(&json!({"id":0,"method":"tools/list"})).await;
        let schema = &tools["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == "swarmail_seed_email")
            .unwrap()["outputSchema"];
        let reply=wire.request(&call(json!(1),"seed_email",json!({"inbox":version,"to":"a@x.io","html":"<a href=\"https://x.io/\">雪</a> 123456"}))).await;
        let payload = text(&reply);
        assert_eq!(payload["links"], json!(["https://x.io/"]));
        assert_eq!(payload["codes"], json!(["123456"]));
        assert!(valid(schema, &payload));
        assert_eq!(payload, reply["result"]["structuredContent"]);
    }
    srv.stop().await;
}
