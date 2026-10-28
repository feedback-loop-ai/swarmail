use super::*;

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
        links: vec!["https://x.io/?q=雪".into()],
        codes: (0..65).map(|i| format!("{i:04}")).collect(),
        message_id: Some("message@x".into()),
        in_reply_to: Some("parent@x".into()),
        references: vec!["root@x".into()],
        raw: b"never exposed".to_vec(),
    }
}

enum Wire {
    Http(std::net::SocketAddr, Option<&'static str>),
    Stdio(Box<Binary>),
}
impl Wire {
    async fn raw(&mut self, request: &Value) -> String {
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
    async fn request(&mut self, request: &Value) -> Value {
        serde_json::from_str(&self.raw(request).await).unwrap()
    }
    async fn finish(self) {
        if let Self::Stdio(binary) = self {
            (*binary).finish().await;
        }
    }
}

async fn workflow(version: Option<&'static str>, stdio: bool, modern: bool) -> Vec<String> {
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
    let catalogue = wire.raw(&json!({"id":1,"method":"tools/list"})).await;
    let list: Value = serde_json::from_str(&catalogue).unwrap();
    let mut transcript = vec![catalogue];
    let mut seen = std::collections::BTreeSet::new();
    // Text alone supplies the seed ID used below, including in modern sessions.
    let mut seeded = Value::Null;
    for (name, mut arguments) in [
        ("search_emails", json!({"inbox":"contract"})),
        ("wait_for_email", json!({"inbox":"contract","timeout_ms":0})),
        ("get_latest_email", json!({"inbox":"contract"})),
        ("clear_inbox", json!({"inbox":"contract"})),
        ("clear_all", json!({})),
        (
            "seed_email",
            json!({"inbox":"seed","to":"a@x.io","text":"plain"}),
        ),
        ("delete_email", json!({"id":"$seed"})),
        ("get_email", json!({"id":"missing"})),
        ("get_email", json!({"id":"historical"})),
        ("get_latest_email", json!({"inbox":"contract"})),
        ("search_emails", json!({"inbox":"contract"})),
        ("search_emails", json!({"inbox":"contract","offset":10})),
        ("wait_for_email", json!({"inbox":"contract","timeout_ms":0})),
        ("extract_links", json!({"id":"historical"})),
        ("extract_codes", json!({"id":"historical"})),
        ("extract_links", json!({"id":"minimal"})),
        ("extract_codes", json!({"id":"minimal"})),
        ("get_email", json!({"id":"minimal"})),
        ("set_chaos", json!({"event":"connect","probability":0})),
        (
            "set_chaos",
            json!({"event":"data","probability":100,"delay_ms":300000,"error":"451 雪"}),
        ),
        ("clear_chaos", json!({})),
        ("clear_inbox", json!({"inbox":"contract"})),
        ("clear_all", json!({})),
        ("search_emails", json!({"unexpected":1})),
        ("no_such_tool", json!({})),
    ] {
        if name == "get_email" && arguments["id"] == "historical" {
            srv.store.insert(historical()).unwrap();
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
            srv.store.insert(minimal).unwrap();
        }
        if arguments["id"] == "$seed" {
            arguments["id"] = seeded.clone();
        }
        let raw = wire.raw(&call(json!(2), name, arguments)).await;
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
                seen.insert(name);
            } else {
                assert!(reply["result"].get("structuredContent").is_none());
            }
        } else {
            assert!(reply["result"].get("structuredContent").is_none());
        }
        let normalized = if name == "seed_email" {
            let id = seeded.as_str().unwrap();
            assert_eq!(raw.matches(id).count(), if modern { 2 } else { 1 });
            raw.replace(id, "$SEED_ID")
        } else {
            raw
        };
        transcript.push(normalized);
    }
    if modern {
        assert_eq!(seen.len(), 12);
    }
    wire.finish().await;
    srv.stop().await;
    transcript
}

#[tokio::test]
async fn legacy_bytes() {
    for version in [None, Some(VERSIONS[0])] {
        for stdio in [false, true] {
            let transcript = workflow(version, stdio, false).await.join("\n") + "\n";
            assert_eq!(
                transcript,
                include_str!("../fixtures/mcp-structured-output/march.jsonl")
            );
        }
    }
}

// Deliberately finite evaluator for our emitted vocabulary. No reference is
// emitted: even a local $ref is refused, including unresolved/cyclic references.
fn schema_ok(s: &Value) -> bool {
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
            "minimum" | "maximum" => value
                .as_i64()
                .is_some_and(|n| (-300000..=300000).contains(&n)),
            "minItems" => value.as_u64().is_some(),
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
        && min.as_i64().unwrap() > max.as_i64().unwrap()
    {
        return false;
    }
    o.contains_key("type") || o.contains_key("anyOf") || o.contains_key("oneOf")
}
fn numeric_bound(v: &Value, bound: i64, lower: bool) -> bool {
    if let Some(n) = v
        .as_i64()
        .map(i128::from)
        .or_else(|| v.as_u64().map(i128::from))
    {
        if lower {
            n >= i128::from(bound)
        } else {
            n <= i128::from(bound)
        }
    } else if let Some(n) = v.as_f64() {
        // Bounds are checked small integers, exactly representable as f64.
        if lower {
            n >= bound as f64
        } else {
            n <= bound as f64
        }
    } else {
        false
    }
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
    for (key, lower) in [("minimum", true), ("maximum", false)] {
        if let Some(bound) = s.get(key)
            && !numeric_bound(v, bound.as_i64().unwrap(), lower)
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
fn valid(s: &Value, v: &Value) -> bool {
    schema_ok(s) && evaluate(s, v)
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
    for bad in [
        json!({}),
        json!(true),
        json!({"type":"bogus"}),
        json!({"type":[]}),
        json!({"type":["string","string"]}),
        json!({"type":42}),
        json!({"type":"string","pattern":"x"}),
        json!({"type":"array","items":false}),
        json!({"type":"array","minItems":-1}),
        json!({"type":"integer","minimum":"0"}),
        json!({"type":"integer","minimum":2,"maximum":1}),
        json!({"type":"object","required":["absent"]}),
        json!({"type":"object","properties":[]}),
        json!({"type":"object","additionalProperties":true}),
        json!({"type":"string","description":1}),
        json!({"anyOf":[]}),
        json!({"anyOf":[{"type":"string"},{"type":"null","unknown":true}]}),
        json!({"$ref":"https://external"}),
        json!({"$ref":"#/$defs/missing"}),
        json!({"$ref":"#","$defs":{"cycle":{"$ref":"#"}}}),
    ] {
        assert!(!valid(&bad, &json!("data")), "accepted {bad}");
    }
}

#[tokio::test]
async fn modern_all_tools_http_and_stdio() {
    for version in &VERSIONS[1..] {
        for stdio in [false, true] {
            let transcript = workflow(Some(version), stdio, true).await;
            // Compare the text block's literal JSON string to pre-change bytes,
            // without reserializing the expected JSON.
            for (raw, baseline) in transcript.iter().skip(1).zip(
                include_str!("../fixtures/mcp-structured-output/march.jsonl")
                    .lines()
                    .skip(1),
            ) {
                let actual: Value = serde_json::from_str(raw).unwrap();
                let expected: Value = serde_json::from_str(baseline).unwrap();
                assert_eq!(actual["result"]["content"], expected["result"]["content"]);
                assert_eq!(actual["result"]["isError"], expected["result"]["isError"]);
                if actual.get("error").is_some() {
                    assert_eq!(raw, baseline);
                }
            }
        }
    }
}

// Schema-aware removal of annotation-only keywords: property names remain data.
fn structural(s: &Value) -> Value {
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
#[tokio::test]
async fn served_openapi_pins_and_negative_controls() {
    let srv = start().await;
    let (_, api) = common::http_json(srv.http_addr, "GET", "/openapi.json", None).await;
    let c = &api["components"]["schemas"];
    let pinned: Value = serde_json::from_str(include_str!(
        "../fixtures/mcp-structured-output/openapi-source.json"
    ))
    .unwrap();
    for (name, expected) in pinned.as_object().unwrap() {
        assert!(source_matches(&c[name], expected), "source changed: {name}");
    }
    for (name, pointer, value) in [
        ("EmailAddress", "/properties/address/type", json!("integer")),
        ("Email", "/required", json!(["subject"])),
        ("Email", "/properties/envelope_from/type", json!("string")),
        ("ChaosRule", "/properties/delay_ms/maximum", json!(300001)),
    ] {
        let mut drift = c[name].clone();
        if pointer == "/required" {
            drift["required"] = value;
        } else {
            *drift.pointer_mut(pointer).unwrap() = value;
        }
        assert!(!source_matches(&drift, &pinned[name]));
    }
    let expected = projections(c);
    let mut wire = Wire::Http(srv.http_addr, Some(VERSIONS[1]));
    let catalogue = wire.request(&json!({"id":1,"method":"tools/list"})).await;
    let tools = catalogue["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), expected.as_object().unwrap().len());
    for tool in tools {
        let name = tool["name"].as_str().unwrap();
        let schema = &tool["outputSchema"];
        assert!(schema_ok(schema));
        assert_eq!(schema["type"], "object");
        assert_eq!(structural(schema), structural(&expected[name]), "{name}");
        assert!(!valid(schema, &json!({})), "vacuous {name}");
    }
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
    ] {
        assert!(!valid(&expected[name], &bad), "{name}");
    }
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
