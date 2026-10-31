//! The focused waits on every revision and transport (decision 0094 §7).
//! Expectations here are written from the decision's stated contract, not
//! read back from the producer: the two appended descriptors, their closed
//! output variants, and real SMTP-delivered payloads.
use super::structured::{Wire, march_view, schema_ok, structural, valid};
use super::*;

pub(super) const FOCUSED: [&str; 2] = ["swarmail_wait_for_code", "swarmail_wait_for_link"];

const ADDRESS: &str = r#"{"type":"object","properties":{"address":{"type":"string"},"name":{"type":"string"}},"required":["address"],"additionalProperties":false}"#;

/// Decision 0111 §4's closed object, written from the contract: five members,
/// seven per entry, the three `missed` names, and `nearest_subject` an entry
/// or null. No `maxLength`: the finite evaluator does not state it.
pub(super) fn near_misses_schema() -> Value {
    let count = json!({"type":"integer","minimum":0});
    let stamp = json!({"type":"integer","minimum":i64::MIN,"maximum":i64::MAX});
    let text = json!({"type":["string","null"]});
    let entry = json!({"type":"object","properties":{
        "email_id":{"type":"string"},"received_ms":stamp,"from":text,
        "to":{"type":"array","items":{"type":"string"},"maxItems":5},"to_total":count,
        "subject":text,
        "missed":{"type":"array","items":{"type":"string","enum":["to","from","subject"]}}},
        "required":["email_id","received_ms","from","to","to_total","subject","missed"],
        "additionalProperties":false});
    json!({"type":"object","properties":{
        "arrivals":{"type":"array","items":entry,"maxItems":10},
        "arrivals_total":count,"arrivals_truncated":{"type":"boolean"},
        "nearest_subject":{"anyOf":[entry,{"type":"null"}]},"last_arrival_ms":stamp},
        "required":["arrivals","arrivals_total","arrivals_truncated","nearest_subject","last_arrival_ms"],
        "additionalProperties":false})
}

fn filters(link: bool) -> Value {
    let mut properties = json!({
        "inbox":{"type":"string"},"to":{"type":"string"},"from":{"type":"string"},
        "subject":{"type":"string"},"since_ms":{"type":"integer"},
        "timeout_ms":{"type":"integer","minimum":0,"maximum":60000}
    });
    if link {
        for key in ["anchor_text", "host", "path"] {
            properties[key] = json!({"type":"string"});
        }
    }
    json!({"type":"object","properties":properties,"additionalProperties":false})
}

/// The input contract without its prose, and the clauses the prose owes.
pub(super) fn assert_march_descriptor(descriptor: &Value) {
    let name = descriptor["name"].as_str().unwrap();
    let link = name == FOCUSED[1];
    assert!(FOCUSED.contains(&name), "{descriptor}");
    let o = descriptor.as_object().unwrap();
    assert_eq!(
        o.keys().collect::<Vec<_>>(),
        ["description", "inputSchema", "name"]
    );
    assert_eq!(
        structural(&descriptor["inputSchema"]),
        filters(link),
        "{name}"
    );
    assert_eq!(
        descriptor["inputSchema"]["properties"]["timeout_ms"]["default"],
        5000
    );
    let description = descriptor["description"].as_str().unwrap();
    let clauses: &[&str] = if link {
        &[
            "same occurrence",
            "1024 UTF-8 bytes",
            "1 MiB",
            "selected_url",
            "ambiguous",
            "250",
            "distinct_urls",
            "timed_out",
        ]
    } else {
        &[
            "newest",
            "acceptance order",
            "ambiguous",
            "older_candidates",
            "250",
            "timed_out",
        ]
    };
    for clause in clauses {
        assert!(description.contains(clause), "{name}: {clause}");
    }
}

/// The served March catalogue is the captured thirteen-tool line with the
/// two focused descriptors appended, and nothing else changed.
pub(super) fn assert_appended(raw: &str, captured: &str) {
    let prefix = captured.strip_suffix("]}}").unwrap();
    let tail = raw
        .strip_prefix(prefix)
        .expect("captured catalogue is a prefix");
    let appended: Value = serde_json::from_str(&format!(
        "[{}",
        tail.strip_prefix(',').unwrap().strip_suffix("}}").unwrap()
    ))
    .unwrap();
    let appended = appended.as_array().unwrap();
    assert_eq!(appended.len(), 2);
    for descriptor in appended {
        assert_march_descriptor(descriptor);
    }
}

/// Decision 0094 §7's closed variants, stated independently.
pub(super) fn expected_output(name: &str) -> Value {
    let link = name == FOCUSED[1];
    let address: Value = serde_json::from_str(ADDRESS).unwrap();
    let u64_max = json!(u64::MAX);
    let count = json!({"type":"integer","minimum":0,"maximum":u64_max});
    let mut candidate = json!({
        "type":"object","additionalProperties":false,
        "properties":{
            "email_id":{"type":"string"},
            "received_ms":{"type":"integer","minimum":i64::MIN,"maximum":i64::MAX},
            "age_ms":count,
            "sender":{"anyOf":[address,{"type":"null"}]}
        }
    });
    let required: Vec<&str> = if link {
        candidate["properties"]["url"] = json!({"type":"string"});
        candidate["properties"]["link_index"] = count.clone();
        candidate["properties"]["anchor_text"] = json!({"type":["null","string"]});
        candidate["properties"]["rel"] = json!({"type":["null","string"]});
        candidate["properties"]["source"] =
            json!({"type":["null","string"],"enum":["text","html",null]});
        vec![
            "age_ms",
            "anchor_text",
            "email_id",
            "link_index",
            "received_ms",
            "rel",
            "sender",
            "source",
            "url",
        ]
    } else {
        candidate["properties"]["code"] = json!({"type":"string"});
        vec!["age_ms", "code", "email_id", "received_ms", "sender"]
    };
    candidate["required"] = json!(required);
    let list = |min: u64, max: u64| json!({"type":"array","items":candidate,"minItems":min,"maxItems":max});
    let least = |min: u64| json!({"type":"integer","minimum":min,"maximum":u64_max});
    let exact = |n: u64| json!({"type":"integer","const":n});
    let close = |status: &str, mut properties: Value| {
        properties["status"] = json!({"type":"string","const":status});
        let mut required: Vec<String> = properties.as_object().unwrap().keys().cloned().collect();
        required.sort();
        json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
    };
    let boolean = json!({"type":"boolean"});
    let never = json!({"type":"boolean","const":false});
    let arms = if link {
        vec![
            close(
                "selected",
                json!({"matched":least(1),"candidates":list(1,250),"candidates_total":least(1),"candidates_truncated":boolean,"distinct_urls":exact(1),"selected_url":{"type":"string"}}),
            ),
            close(
                "ambiguous",
                json!({"matched":least(1),"candidates":list(2,250),"candidates_total":least(2),"candidates_truncated":boolean,"distinct_urls":least(2)}),
            ),
            close(
                "timed_out",
                json!({"matched":exact(0),"candidates":list(0,0),"candidates_total":exact(0),"candidates_truncated":never,"distinct_urls":exact(0)}),
            ),
            close(
                "timed_out",
                json!({"matched":exact(0),"candidates":list(0,0),"candidates_total":exact(0),"candidates_truncated":never,"distinct_urls":exact(0),"near_misses":near_misses_schema()}),
            ),
        ]
    } else {
        vec![
            close(
                "selected",
                json!({"matched":least(1),"candidates":list(1,1),"candidates_total":exact(1),"candidates_truncated":never,"older_candidates":list(0,250),"older_candidates_total":count,"older_candidates_truncated":boolean,"selected":candidate}),
            ),
            close(
                "ambiguous",
                json!({"matched":least(1),"candidates":list(2,250),"candidates_total":least(2),"candidates_truncated":boolean,"older_candidates":list(0,250),"older_candidates_total":count,"older_candidates_truncated":boolean}),
            ),
            close(
                "timed_out",
                json!({"matched":exact(0),"candidates":list(0,0),"candidates_total":exact(0),"candidates_truncated":never,"older_candidates":list(0,0),"older_candidates_total":exact(0),"older_candidates_truncated":never}),
            ),
            close(
                "timed_out",
                json!({"matched":exact(0),"candidates":list(0,0),"candidates_total":exact(0),"candidates_truncated":never,"older_candidates":list(0,0),"older_candidates_total":exact(0),"older_candidates_truncated":never,"near_misses":near_misses_schema()}),
            ),
        ]
    };
    structural(&json!({"type":"object","oneOf":arms}))
}

fn html_mail(subject: &str, html: &str) -> Vec<u8> {
    format!(
        "From: App <app@x.io>\r\nTo: rcpt@x.io\r\nSubject: {subject}\r\nMIME-Version: 1.0\r\nContent-Type: text/html; charset=utf-8\r\n\r\n{html}\r\n"
    )
    .into_bytes()
}

fn text_mail(subject: &str, body: &str) -> Vec<u8> {
    format!("From: App <app@x.io>\r\nTo: rcpt@x.io\r\nSubject: {subject}\r\n\r\n{body}\r\n")
        .into_bytes()
}

/// A pre-0094 row: legacy links, no occurrence metadata.
fn historical(inbox: &str) -> swarmail::model::Email {
    swarmail::model::Email {
        id: format!("historical-{inbox}"),
        inbox: inbox.into(),
        from: None,
        envelope_from: None,
        to: vec![],
        cc: vec![],
        recipients: vec![],
        subject: None,
        received_at: "1970-01-01T00:00:00Z".into(),
        received_ms: 0,
        size: 1,
        text: None,
        html: None,
        link_metadata: None,
        links: vec!["https://old.test/verify".into()],
        codes: vec!["777777".into()],
        message_id: None,
        in_reply_to: None,
        references: vec![],
        raw: vec![],
    }
}

/// Each case's payload with the ages removed (they depend on wall time;
/// their domain is checked separately).
fn ageless(mut v: Value) -> Value {
    for key in ["candidates", "older_candidates"] {
        if let Some(list) = v.get_mut(key).and_then(Value::as_array_mut) {
            for entry in list {
                let age = entry.as_object_mut().unwrap().remove("age_ms").unwrap();
                assert!(age.as_u64().is_some(), "{age}");
            }
        }
    }
    if let Some(selected) = v.get_mut("selected") {
        selected.as_object_mut().unwrap().remove("age_ms").unwrap();
    }
    v
}

#[tokio::test]
async fn focused_waits_on_every_revision_and_transport() {
    let srv = start().await;
    let stale = smtp_common_raw(&srv, "codes", &text_mail("Old code", "Your code is 111111")).await;
    let fresh = smtp_common_raw(&srv, "codes", &text_mail("New code", "Your code is 222222")).await;
    let multi = smtp_common_raw(&srv, "multi", &text_mail("Codes", "Use 333333 or 444444")).await;
    let links = smtp_common_raw(
        &srv,
        "links",
        &html_mail(
            "Confirm",
            "<p><a href='https://app.test/confirm/1' rel='noopener'>Confirm</a> \
             <a href=https://app.test/unsub>Unsubscribe</a></p>",
        ),
    )
    .await;
    srv.store.insert(historical("history")).unwrap();
    let sender = json!({"name":"App","address":"app@x.io"});
    let code = |email: &str, code: &str, received_ms: &Value| json!({"email_id":email,"code":code,"received_ms":received_ms,"sender":sender});
    let stored = |id: &str| srv.store.get(id).unwrap().received_ms;
    let (stale_ms, fresh_ms, multi_ms, links_ms) = (
        json!(stored(&stale)),
        json!(stored(&fresh)),
        json!(stored(&multi)),
        json!(stored(&links)),
    );
    let confirm = json!({"url":"https://app.test/confirm/1","email_id":links,"link_index":0,
        "anchor_text":"Confirm","rel":"noopener","source":"html","received_ms":links_ms,"sender":sender});
    let unsub = json!({"url":"https://app.test/unsub","email_id":links,"link_index":1,
        "anchor_text":"Unsubscribe","rel":null,"source":"html","received_ms":links_ms,"sender":sender});
    // The link wait's selectors are not mail filters: the mail they refuse
    // still shows, with an empty `missed` (decision 0111).
    let near_links = json!({"email_id":links,"received_ms":links_ms,"from":"app@x.io","to":["rcpt@x.io"],
        "to_total":1,"subject":"Confirm","missed":[]});
    let cases = [
        (
            "wait_for_code",
            json!({"inbox":"codes","timeout_ms":0}),
            json!({
            "status":"selected","matched":2,"selected":code(&fresh,"222222",&fresh_ms),
            "candidates":[code(&fresh,"222222",&fresh_ms)],"candidates_total":1,"candidates_truncated":false,
            "older_candidates":[code(&stale,"111111",&stale_ms)],"older_candidates_total":1,"older_candidates_truncated":false}),
        ),
        (
            "wait_for_code",
            json!({"inbox":"multi","timeout_ms":0}),
            json!({
            "status":"ambiguous","matched":1,
            "candidates":[code(&multi,"333333",&multi_ms),code(&multi,"444444",&multi_ms)],"candidates_total":2,"candidates_truncated":false,
            "older_candidates":[],"older_candidates_total":0,"older_candidates_truncated":false}),
        ),
        (
            "wait_for_code",
            json!({"inbox":"empty","timeout_ms":0}),
            json!({
            "status":"timed_out","matched":0,"candidates":[],"candidates_total":0,"candidates_truncated":false,
            "older_candidates":[],"older_candidates_total":0,"older_candidates_truncated":false}),
        ),
        (
            "wait_for_link",
            json!({"inbox":"links","anchor_text":"^Confirm$","timeout_ms":0}),
            json!({
            "status":"selected","matched":1,"selected_url":"https://app.test/confirm/1","distinct_urls":1,
            "candidates":[confirm],"candidates_total":1,"candidates_truncated":false}),
        ),
        (
            "wait_for_link",
            json!({"inbox":"links","host":"^app[.]test$","timeout_ms":0}),
            json!({
            "status":"ambiguous","matched":1,"distinct_urls":2,
            "candidates":[confirm,unsub],"candidates_total":2,"candidates_truncated":false}),
        ),
        (
            "wait_for_link",
            json!({"inbox":"links","path":"^/nowhere$","timeout_ms":0}),
            json!({
            "status":"timed_out","matched":0,"distinct_urls":0,"candidates":[],"candidates_total":0,"candidates_truncated":false,
            "near_misses":{"arrivals":[near_links.clone()],"arrivals_total":1,"arrivals_truncated":false,
                "nearest_subject":null,"last_arrival_ms":links_ms}}),
        ),
        (
            "wait_for_link",
            json!({"inbox":"history","timeout_ms":0}),
            json!({
            "status":"selected","matched":1,"selected_url":"https://old.test/verify","distinct_urls":1,
            "candidates":[{"url":"https://old.test/verify","email_id":"historical-history","link_index":0,
                "anchor_text":null,"rel":null,"source":null,"received_ms":0,"sender":null}],
            "candidates_total":1,"candidates_truncated":false}),
        ),
        (
            "wait_for_link",
            json!({"inbox":"history","anchor_text":"","timeout_ms":0}),
            json!({
            "status":"timed_out","matched":0,"distinct_urls":0,"candidates":[],"candidates_total":0,"candidates_truncated":false,
            "near_misses":{"arrivals":[{"email_id":"historical-history","received_ms":0,"from":null,"to":[],
                "to_total":0,"subject":null,"missed":[]}],"arrivals_total":1,"arrivals_truncated":false,
                "nearest_subject":null,"last_arrival_ms":0}}),
        ),
        (
            "wait_for_code",
            json!({"inbox":"history","timeout_ms":0}),
            json!({
            "status":"selected","matched":1,"selected":{"email_id":"historical-history","code":"777777","received_ms":0,"sender":null},
            "candidates":[{"email_id":"historical-history","code":"777777","received_ms":0,"sender":null}],
            "candidates_total":1,"candidates_truncated":false,
            "older_candidates":[],"older_candidates_total":0,"older_candidates_truncated":false}),
        ),
    ];
    let errors = [
        (
            "wait_for_link",
            json!({"path":"["}),
            json!({"code":-32602,"message":"path must be a valid regex within the 1 MiB compiled-size limit"}),
        ),
        (
            "wait_for_code",
            json!({"host":"x"}),
            json!({"code":-32602,"message":"unknown argument: host"}),
        ),
        (
            "wait_for_code",
            json!({"since_ms":"1"}),
            json!({"code":-32602,"message":"since_ms must be an integer"}),
        ),
    ];
    for version in VERSIONS {
        for stdio in [false, true] {
            let mut wire = Wire::start(srv.http_addr, version, stdio).await;
            let list = wire.request(&json!({"id":1,"method":"tools/list"})).await;
            let tools = list["result"]["tools"].as_array().unwrap();
            assert_eq!(tools.len(), 15);
            let modern = version != VERSIONS[0];
            for (i, name) in FOCUSED.iter().enumerate() {
                let tool = &tools[13 + i];
                assert_eq!(tool["name"], *name);
                if modern {
                    assert_eq!(structural(&tool["outputSchema"]), expected_output(name));
                    assert_march_descriptor(&march_view(tool, true));
                } else {
                    assert_march_descriptor(tool);
                }
            }
            let schema = |tool: &str| {
                tools
                    .iter()
                    .find(|t| t["name"] == format!("swarmail_{tool}"))
                    .unwrap()["outputSchema"]
                    .clone()
            };
            for (tool, arguments, expected) in &cases {
                let raw = wire.raw(&call(json!(7), tool, arguments.clone())).await;
                let reply: Value = serde_json::from_str(&raw).unwrap();
                assert_eq!(reply["result"]["isError"], false, "{raw}");
                let text = reply["result"]["content"][0]["text"].as_str().unwrap();
                let payload: Value = serde_json::from_str(text).unwrap();
                assert_eq!(&ageless(payload.clone()), expected, "{tool} {arguments}");
                if modern {
                    assert!(
                        raw.ends_with(&format!(",\"structuredContent\":{text}}}}}")),
                        "{raw}"
                    );
                    assert_eq!(reply["result"]["structuredContent"], payload);
                    assert!(valid(&schema(tool), &payload), "{tool}: {payload}");
                } else {
                    assert!(reply["result"].get("structuredContent").is_none());
                }
            }
            for (tool, arguments, error) in &errors {
                let reply = wire.request(&call(json!(8), tool, arguments.clone())).await;
                assert_eq!(reply, json!({"jsonrpc":"2.0","id":8,"error":error}));
            }
            let reply = wire
                .request(&call(
                    json!(9),
                    "wait_for_link",
                    json!({"timeout_ms":60001}),
                ))
                .await;
            assert_eq!(reply["result"]["isError"], true);
            assert!(reply["result"].get("structuredContent").is_none());
            wire.finish().await;
        }
    }
    srv.stop().await;
}

async fn smtp_common_raw(srv: &swarmail::RunningServer, inbox: &str, raw: &[u8]) -> String {
    common::smtp_raw(srv.smtp_addr, inbox, raw).await
}

/// Each stated constraint refuses on its own: a wrong status, a missing or
/// extra member, a bad null, a range, a winner beside ambiguity, a count
/// contradicting the variant, a 251st entry and an unknown source.
#[tokio::test]
async fn focused_schema_negatives() {
    let srv = start().await;
    let mut wire = Wire::start(srv.http_addr, VERSIONS[2], false).await;
    let list = wire.request(&json!({"id":1,"method":"tools/list"})).await;
    let schema = |name: &str| {
        let tools = list["result"]["tools"].as_array().unwrap();
        let schema = tools.iter().find(|t| t["name"] == name).unwrap()["outputSchema"].clone();
        assert!(schema_ok(&schema));
        schema
    };
    let (code, link) = (schema(FOCUSED[0]), schema(FOCUSED[1]));
    let candidate = json!({"email_id":"e","code":"1234","received_ms":-5,"age_ms":0,"sender":null});
    let selected = json!({"status":"selected","matched":1,"selected":candidate,"candidates":[candidate],
        "candidates_total":1,"candidates_truncated":false,"older_candidates":[],"older_candidates_total":0,
        "older_candidates_truncated":false});
    assert!(valid(&code, &selected));
    let mut ambiguous = selected.clone();
    ambiguous["status"] = json!("ambiguous");
    ambiguous.as_object_mut().unwrap().remove("selected");
    ambiguous["candidates"] = json!([candidate, candidate]);
    ambiguous["candidates_total"] = json!(2);
    assert!(valid(&code, &ambiguous));
    for (base, pointer, value) in [
        (&selected, "/status", json!("chosen")),
        (&selected, "/matched", json!(0)),
        (&selected, "/candidates_total", json!(2)),
        (&selected, "/candidates_truncated", json!(true)),
        (&selected, "/selected/email_id", Value::Null),
        (&selected, "/selected/age_ms", json!(-1)),
        (&selected, "/selected/received_ms", json!(1.5)),
        (&selected, "/selected/sender", json!({"name":"x"})),
        (&selected, "/selected/extra", json!(1)),
        (&selected, "/extra", json!(1)),
        (&selected, "/older_candidates_total", json!(-1)),
        (&ambiguous, "/selected", candidate.clone()),
        (&ambiguous, "/candidates", json!([candidate])),
        (&ambiguous, "/candidates_total", json!(1)),
        (
            &ambiguous,
            "/older_candidates",
            json!(vec![candidate.clone(); 251]),
        ),
    ] {
        let mut bad = base.clone();
        match bad.pointer_mut(pointer) {
            Some(slot) => *slot = value,
            None => {
                let (parent, key) = pointer.rsplit_once('/').unwrap();
                bad.pointer_mut(parent).unwrap()[key] = value;
            }
        }
        assert!(!valid(&code, &bad), "{pointer}");
    }
    for key in selected.as_object().unwrap().keys() {
        let mut bad = selected.clone();
        bad.as_object_mut().unwrap().remove(key);
        assert!(!valid(&code, &bad), "missing {key}");
    }
    let mut timed_out = ambiguous.clone();
    timed_out["status"] = json!("timed_out");
    timed_out["matched"] = json!(0);
    assert!(!valid(&code, &timed_out), "timed_out carries no candidates");
    let entry = json!({"url":"https://x.io/","email_id":"e","link_index":0,"anchor_text":null,"rel":"",
        "source":null,"received_ms":0,"age_ms":0,"sender":{"address":"a@x.io"}});
    let selected = json!({"status":"selected","matched":1,"selected_url":"https://x.io/","distinct_urls":1,
        "candidates":[entry, entry],"candidates_total":2,"candidates_truncated":false});
    assert!(valid(&link, &selected));
    for (pointer, value) in [
        ("/distinct_urls", json!(2)),
        ("/selected_url", Value::Null),
        ("/candidates/0/source", json!("pdf")),
        ("/candidates/0/link_index", json!(-1)),
        ("/candidates/0/anchor_text", json!(1)),
        ("/candidates", json!(vec![entry.clone(); 251])),
        ("/candidates", json!([])),
    ] {
        let mut bad = selected.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        assert!(!valid(&link, &bad), "{pointer}");
    }
    let mut ambiguous = selected.clone();
    ambiguous["status"] = json!("ambiguous");
    assert!(!valid(&link, &ambiguous), "a winner beside ambiguity");
    ambiguous.as_object_mut().unwrap().remove("selected_url");
    assert!(
        !valid(&link, &ambiguous),
        "one destination is not ambiguous"
    );
    ambiguous["distinct_urls"] = json!(2);
    assert!(valid(&link, &ambiguous));
    wire.finish().await;
    srv.stop().await;
}

/// Decision 0097 §4: every current surface states the compatibility the
/// served catalogue actually has — thirteen existing March entries kept,
/// two focused waits appended, fifteen in all — and keeps the OpenAPI pin
/// for REST-backed schemas apart from the MCP-only focused schemas. The
/// blanket "catalogue bytes unchanged" claims the review found are gone.
#[tokio::test]
async fn served_prose_states_additive_discovery() {
    let srv = start().await;
    let list = json!({"jsonrpc":"2.0","id":1,"method":"tools/list"});
    let tools = post(srv.http_addr, &[], &list).await.1["result"]["tools"].clone();
    let names: Vec<&str> = tools
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(names.len(), 15);
    assert_eq!(names[13..], FOCUSED);
    assert!(names[..13].iter().all(|name| !FOCUSED.contains(name)));
    let flat = |text: &str| {
        text.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    };
    let (status, llms) = common::http_get_text(srv.http_addr, "/llms.txt").await;
    assert_eq!(status, 200);
    let (status, api) = common::http_get_text(srv.http_addr, "/openapi.json").await;
    assert_eq!(status, 200);
    let api: Value = serde_json::from_str(&api).unwrap();
    let readme =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md")).unwrap();
    let surfaces = [
        ("README", flat(&readme)),
        ("llms.txt", flat(&llms)),
        (
            "openapi",
            flat(
                api["paths"]["/mcp"]["post"]["description"]
                    .as_str()
                    .unwrap(),
            ),
        ),
    ];
    for (surface, text) in &surfaces {
        for clause in [
            "thirteen existing",
            "existing result bytes",
            "appends the two focused waits after them",
            "fifteen",
            "explicit output projections",
            "mcp-only",
            "independently stated expectations",
        ] {
            assert!(text.contains(clause), "{surface} omits {clause:?}");
        }
        for stale in [
            "march catalogue/result bytes are unchanged",
            "keeps its catalogue and result bytes",
            "preserves catalogue/result bytes",
            "thirteen mcp tools",
            "13 tools",
        ] {
            assert!(!text.contains(stale), "{surface} still claims {stale:?}");
        }
    }
    assert!(surfaces[0].1.contains("fifteen mcp tools"));
    srv.stop().await;
}
