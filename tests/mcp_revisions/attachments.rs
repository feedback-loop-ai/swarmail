//! The attachment tools on every revision (decision 0110 §7). Expectations
//! are written from the decision's stated contract, not read back from the
//! producer: the two appended descriptors and their closed output schemas.
use super::structured::{SHA256_PATTERN, structural, valid};
use super::*;

pub(super) const ATTACHMENTS: [&str; 2] = ["swarmail_list_attachments", "swarmail_get_attachment"];

/// The input contract without its prose, and the clauses the prose owes.
pub(super) fn assert_march_descriptor(descriptor: &Value) {
    let name = descriptor["name"].as_str().unwrap();
    assert!(ATTACHMENTS.contains(&name), "{descriptor}");
    let o = descriptor.as_object().unwrap();
    assert_eq!(
        o.keys().collect::<Vec<_>>(),
        ["description", "inputSchema", "name"]
    );
    let get = name == ATTACHMENTS[1];
    let mut properties = json!({"id":{"type":"string"}});
    if get {
        properties["index"] = json!({"type":"integer","minimum":0});
    }
    let required = if get {
        json!(["id", "index"])
    } else {
        json!(["id"])
    };
    assert_eq!(
        structural(&descriptor["inputSchema"]),
        structural(&json!({
            "type":"object","properties":properties,
            "required":required,"additionalProperties":false
        })),
        "{name}"
    );
    let description = descriptor["description"].as_str().unwrap();
    let clauses: &[&str] = if get {
        &[
            "never truncated",
            "utf-8",
            "base64",
            "1 MiB",
            "stored false",
            "GET /api/v1/messages/{id}/raw",
        ]
    } else {
        &[
            "250",
            "SHA-256",
            "stored",
            "first VEVENT",
            "total",
            "truncated",
        ]
    };
    for clause in clauses {
        assert!(description.contains(clause), "{name}: {clause}");
    }
}

/// Decision 0110 §7's closed output schemas, stated independently.
pub(super) fn expected_output(name: &str) -> Value {
    let u64_max = json!(u64::MAX);
    let count = json!({"type":"integer","minimum":0,"maximum":u64_max});
    let text = json!({"type":["string","null"]});
    let digest = json!({"type":"string","pattern":SHA256_PATTERN});
    let closed = |properties: Value, required: Value| json!({"type":"object","properties":properties,"required":required,"additionalProperties":false});
    if name == ATTACHMENTS[1] {
        return structural(&closed(
            json!({
                "index":count,"filename":text,"content_type":text,"size":count,
                "sha256":digest,"encoding":{"type":"string","enum":["utf-8","base64"]},
                "content":{"type":"string"}
            }),
            json!([
                "content",
                "content_type",
                "encoding",
                "filename",
                "index",
                "sha256",
                "size"
            ]),
        ));
    }
    let ics = closed(
        json!({
            "dtstart":text,"dtstart_tzid":text,"dtend":text,"dtend_tzid":text,
            "summary":text,"uid":text
        }),
        json!([
            "dtend",
            "dtend_tzid",
            "dtstart",
            "dtstart_tzid",
            "summary",
            "uid"
        ]),
    );
    let entry = closed(
        json!({
            "index":count,"filename":text,"content_type":text,"size":count,
            "sha256":digest,"stored":{"type":"boolean"},
            "ics":{"anyOf":[ics,{"type":"null"}]}
        }),
        json!([
            "content_type",
            "filename",
            "ics",
            "index",
            "sha256",
            "size",
            "stored"
        ]),
    );
    structural(&closed(
        json!({
            "id":{"type":"string"},"total":count,"truncated":{"type":"boolean"},
            "attachments":{"type":"array","items":entry,"maxItems":250}
        }),
        json!(["attachments", "id", "total", "truncated"]),
    ))
}

/// The closed schemas accept exactly the shapes the contract states and
/// refuse each drift, so the equality above is not vacuous.
#[test]
fn the_stated_output_schemas_are_not_vacuous() {
    let list = expected_output(ATTACHMENTS[0]);
    let sha = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    let entry = json!({"index":0,"filename":null,"content_type":"text/calendar","size":3,
        "sha256":sha,"stored":true,
        "ics":{"dtstart":"20261102T150000","dtstart_tzid":null,"dtend":null,
               "dtend_tzid":null,"summary":"s","uid":"u"}});
    let ok = json!({"id":"i","total":1,"truncated":false,"attachments":[entry]});
    assert!(valid(&list, &ok));
    let mut drift = ok.clone();
    drift["attachments"][0]["sha256"] = json!("XYZ");
    assert!(!valid(&list, &drift));
    let mut drift = ok.clone();
    drift["attachments"][0]["ics"] = json!({"dtstart":"x"});
    assert!(!valid(&list, &drift));
    let mut drift = ok.clone();
    drift["attachments"][0]["extra"] = json!(1);
    assert!(!valid(&list, &drift));
    let mut drift = ok;
    drift["attachments"] = json!(vec![entry.clone(); 251]);
    assert!(!valid(&list, &drift));
    assert!(!valid(&list, &json!({})));

    let get = expected_output(ATTACHMENTS[1]);
    let ok = json!({"index":0,"filename":"a.txt","content_type":null,"size":1,
        "sha256":sha,"encoding":"utf-8","content":"x"});
    assert!(valid(&get, &ok));
    let mut drift = ok.clone();
    drift["encoding"] = json!("hex");
    assert!(!valid(&get, &drift));
    let mut drift = ok;
    drift["size"] = json!(-1);
    assert!(!valid(&get, &drift));
}

fn quoted_mail(parts: &[(&str, Vec<u8>)]) -> Vec<u8> {
    use base64::Engine;
    let mut raw = String::from(
        "From: app@x.io\r\nTo: rcpt@x.io\r\nSubject: attachments\r\nMIME-Version: 1.0\r\n\
         Content-Type: multipart/mixed; boundary=\"bd\"\r\n\r\n--bd\r\n\
         Content-Type: text/plain\r\n\r\nsee attached\r\n",
    );
    for (kind, bytes) in parts {
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        let wrapped: Vec<&str> = encoded
            .as_bytes()
            .chunks(76)
            .map(|c| std::str::from_utf8(c).unwrap())
            .collect();
        raw.push_str(&format!(
            "--bd\r\nContent-Type: {kind}\r\nContent-Transfer-Encoding: base64\r\n\
             Content-Disposition: attachment; filename=\"{}\"\r\n\r\n{}\r\n",
            kind.replace('/', "-"),
            wrapped.join("\r\n"),
        ));
    }
    raw.push_str("--bd--\r\n");
    raw.into_bytes()
}

fn tool_call(id: i64, name: &str, arguments: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":arguments}})
}

/// Both tools on every revision and both transports, with the largest
/// answers the feature can give: a 1 MiB part of characters that double
/// when escaped, and a 1 MiB binary part. Each passes the stdio bridge's
/// unchanged 10 MiB response cap (decision 0110 §3), and a modern success
/// carries the same JSON as `structuredContent`, valid against the stated
/// schema; March stays text-only.
#[tokio::test]
async fn attachment_tools_on_every_revision_and_transport() {
    let srv = start().await;
    let worst_text = vec![b'"'; 1_048_576];
    let binary = vec![0xA5u8; 1_048_576];
    // The budget is per mail and first-fit, so each 1 MiB part rides its own.
    let text_mail = quoted_mail(&[
        ("text/plain", worst_text.clone()),
        (
            "text/calendar",
            b"BEGIN:VEVENT\r\nUID:wire\r\nEND:VEVENT\r\n".to_vec(),
        ),
    ]);
    let binary_mail = quoted_mail(&[("application/octet-stream", binary.clone())]);
    let id = common::smtp_raw(srv.smtp_addr, "revisions", &text_mail).await;
    let binary_id = common::smtp_raw(srv.smtp_addr, "revisions", &binary_mail).await;
    for version in VERSIONS {
        let modern = version != VERSIONS[0];
        for stdio in [false, true] {
            let mut wire = super::structured::Wire::start(srv.http_addr, version, stdio).await;
            let reply = wire
                .request(&tool_call(1, ATTACHMENTS[0], json!({"id": id})))
                .await;
            let result = &reply["result"];
            assert_eq!(result["isError"], false, "{version} {stdio}");
            let listed: Value =
                serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
            assert_eq!(listed["total"], 2);
            assert_eq!(listed["attachments"][1]["ics"]["uid"], "wire");
            assert_eq!(listed["attachments"][1]["stored"], false);
            if modern {
                assert_eq!(result["structuredContent"], listed);
                assert!(valid(&expected_output(ATTACHMENTS[0]), &listed));
            } else {
                assert!(result.get("structuredContent").is_none());
            }
            for (mail, bytes) in [(&id, &worst_text), (&binary_id, &binary)] {
                let reply = wire
                    .request(&tool_call(
                        2,
                        ATTACHMENTS[1],
                        json!({"id": mail, "index": 0}),
                    ))
                    .await;
                let result = &reply["result"];
                assert_eq!(result["isError"], false, "{version} {stdio} {mail}");
                let fetched: Value =
                    serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
                assert_eq!(fetched["size"], bytes.len());
                assert!(fetched["content_type"].is_string());
                if mail == &id {
                    assert_eq!(fetched["sha256"], listed["attachments"][0]["sha256"]);
                    assert_eq!(fetched["encoding"], "utf-8");
                    assert_eq!(fetched["content"].as_str().unwrap().as_bytes(), &bytes[..]);
                } else {
                    use base64::Engine;
                    assert_eq!(fetched["encoding"], "base64");
                    assert_eq!(
                        base64::engine::general_purpose::STANDARD
                            .decode(fetched["content"].as_str().unwrap())
                            .unwrap(),
                        *bytes
                    );
                }
                if modern {
                    assert_eq!(result["structuredContent"], fetched);
                    assert!(valid(&expected_output(ATTACHMENTS[1]), &fetched));
                } else {
                    assert!(result.get("structuredContent").is_none());
                }
            }
            // Refusals keep their shapes on every revision.
            let reply = wire
                .request(&tool_call(
                    3,
                    ATTACHMENTS[1],
                    json!({"id": id, "index": -1}),
                ))
                .await;
            assert_eq!(reply["error"]["code"], -32602);
            let reply = wire
                .request(&tool_call(4, ATTACHMENTS[0], json!({"id": "nope"})))
                .await;
            assert_eq!(reply["result"]["isError"], true);
            assert!(reply["result"].get("structuredContent").is_none());
            wire.finish().await;
        }
    }
    srv.stop().await;
}
