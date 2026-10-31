//! The attachment tools (decision 0110 §7): `swarmail_list_attachments`
//! answers the list ingest recorded for one mail — label, type, decoded
//! size, SHA-256, whether the bytes are kept, and a calendar part's fields —
//! and `swarmail_get_attachment` answers one stored part's bytes.
//!
//! Both read only what ingest stored (decision 0005): the handler holds no
//! parser and no access to the message's raw bytes, so the bytes it serves
//! are the bytes the listed hash describes, whatever the parser later
//! learns. A part over the per-mail budget is metadata only; the REST raw
//! route still serves the whole mail (decision 0022).

use super::{McpContext, Revision, arg_usize, error_result, refusal, required_id, text_result};
use crate::attachments::{
    MAX_ATTACHMENT_STORED_BYTES, MAX_ATTACHMENTS, MAX_LIST_JSON_BYTES, encode_content,
};
use serde_json::{Value, json};

pub(super) const LIST: &str = "swarmail_list_attachments";
pub(super) const GET: &str = "swarmail_get_attachment";

/// Stage and answer one call. Every argument refusal — the id, then the
/// index — answers `-32602` by name before the store is read; store-state
/// refusals are `isError` results, in the order the contract states.
pub(super) fn call(
    ctx: &McpContext,
    name: &str,
    a: &Value,
    revision: Revision,
) -> Result<Value, (i64, String)> {
    let id = required_id(a)?;
    let index = if name == GET {
        Some(arg_usize(a, "index")?.ok_or_else(|| refusal("index", "a non-negative integer"))?)
    } else {
        None
    };
    let Some(email) = ctx.store.get(&id) else {
        return Ok(error_result(format!("message {id} not found")));
    };
    let Some(list) = email.attachments.as_ref().filter(|list| !list.labels_only) else {
        return Ok(error_result(format!(
            "the attachments of message {id} are unknown: it was stored before \
             attachments were listed, or its MIME could not be read"
        )));
    };
    let Some(index) = index else {
        // The answer's bound (decision 0110 §3), charged on what is sent: a
        // prefix of the list, and `truncated` names any cut. The first entry
        // always fits: the labels of one are far under the bound.
        let mut attachments = Vec::new();
        let mut charged = 0;
        for (index, part) in list.parts.iter().enumerate() {
            let mut entry = serde_json::to_value(part).unwrap();
            entry["index"] = json!(index);
            charged += entry.to_string().len();
            if charged > MAX_LIST_JSON_BYTES {
                break;
            }
            attachments.push(entry);
        }
        return Ok(text_result(
            json!({
                "id": id,
                "total": list.total,
                "truncated": list.total > attachments.len() as u64,
                "attachments": attachments,
            }),
            revision,
        ));
    };
    let Some(part) = list.parts.get(index) else {
        return Ok(error_result(format!(
            "attachment index {index} is past the list of {} attachments of message {id}",
            list.parts.len()
        )));
    };
    let Some(bytes) = &part.content else {
        return Ok(error_result(format!(
            "attachment {index} of message {id} is {} bytes and its bytes were not kept: \
             the per-mail storage budget is {MAX_ATTACHMENT_STORED_BYTES} bytes (1 MiB), \
             first-fit in list order; GET /api/v1/messages/{id}/raw serves the whole mail",
            part.size
        )));
    };
    let (encoding, content) = encode_content(bytes);
    Ok(text_result(
        json!({
            "index": index,
            "filename": part.filename,
            "content_type": part.content_type,
            "size": part.size,
            "sha256": part.sha256,
            "encoding": encoding,
            "content": content,
        }),
        revision,
    ))
}

/// The two descriptors, appended after the focused waits so every earlier
/// entry keeps its position and bytes (decision 0094 §7). Inputs are closed.
pub(super) fn descriptors() -> [Value; 2] {
    [
        json!({ "name": LIST, "description": "List the attachments of one captured email, recorded once at ingest: for each part its index, filename, lowercased content type, decoded size in bytes, SHA-256 (lowercase hex of exactly the bytes get_attachment returns), whether its bytes are stored, and for a calendar part (text/calendar, application/ics or a .ics filename) the first VEVENT's dtstart, dtend, their TZID, summary and uid, verbatim. Lists at most 250 parts in message order and at most 3 MiB of JSON, so labels that escape heavily can list fewer; total is the exact count and truncated says whether more exist. A stored part cut from the list stays fetchable by its index. An attached message is one entry. Unknown ids are an error.",
            "inputSchema": { "type": "object", "properties": {
                "id": { "type": "string", "description": "Email id" } },
                "required": ["id"], "additionalProperties": false } }),
        json!({ "name": GET, "description": "Fetch one attachment of a captured email by its index in swarmail_list_attachments. content is the part's decoded bytes whole, never truncated: encoding utf-8 when they are valid UTF-8 with no control bytes beyond tab, newline and carriage return, otherwise base64; sha256 matches the list. Only 1 MiB of decoded bytes is stored per email, first-fit in list order, so a larger part (or one that no longer fits) is listed with stored false and is refused here, naming GET /api/v1/messages/{id}/raw, which serves the whole mail.",
            "inputSchema": { "type": "object", "properties": {
                "id": { "type": "string", "description": "Email id" },
                "index": { "type": "integer", "minimum": 0, "description": "Position in the attachment list" } },
                "required": ["id", "index"], "additionalProperties": false } }),
    ]
}

/// The closed output schemas (decision 0110 §7): the list answer, or the
/// one fetched part. Every member a variant emits is required; nullable
/// members are present as null.
pub(super) fn output_schema(list: bool) -> Value {
    let object = super::output_object;
    let count = json!({"type":"integer","minimum":0,"maximum":u64::MAX});
    let text = json!({"type":["string","null"]});
    let sha256 = json!({"type":"string","pattern":"^[0-9a-f]{64}$"});
    if !list {
        return object(
            json!({
                "index":count, "filename":text, "content_type":text, "size":count,
                "sha256":sha256, "encoding":{"type":"string","enum":["utf-8","base64"]},
                "content":{"type":"string"}
            }),
            json!([
                "index",
                "filename",
                "content_type",
                "size",
                "sha256",
                "encoding",
                "content"
            ]),
        );
    }
    let keys = [
        "dtstart",
        "dtstart_tzid",
        "dtend",
        "dtend_tzid",
        "summary",
        "uid",
    ];
    let ics = object(
        Value::Object(
            keys.iter()
                .map(|key| (key.to_string(), text.clone()))
                .collect(),
        ),
        json!(keys),
    );
    let entry = object(
        json!({
            "index":count, "filename":text, "content_type":text, "size":count,
            "sha256":sha256, "stored":{"type":"boolean"},
            "ics":{"anyOf":[ics,{"type":"null"}]}
        }),
        json!([
            "index",
            "filename",
            "content_type",
            "size",
            "sha256",
            "stored",
            "ics"
        ]),
    );
    object(
        json!({
            "id":{"type":"string"}, "total":count, "truncated":{"type":"boolean"},
            "attachments":{"type":"array","items":entry,"maxItems":MAX_ATTACHMENTS}
        }),
        json!(["id", "total", "truncated", "attachments"]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Email;
    use base64::Engine as _;
    use std::sync::Arc;

    fn context() -> (tokio::sync::watch::Sender<bool>, McpContext) {
        let (stop, shutdown) = tokio::sync::watch::channel(false);
        let ctx = McpContext {
            tool_rate: Arc::new(super::super::ToolRate::default()),
            store: Arc::new(crate::store::Store::new(0)),
            chaos: Arc::new(crate::chaos::Chaos::default()),
            shutdown,
        };
        (stop, ctx)
    }

    async fn answer(ctx: &McpContext, tool: &str, arguments: Value) -> Value {
        let req = json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
                         "params":{"name": tool, "arguments": arguments}});
        super::super::handle(ctx, &req).await.unwrap()
    }

    fn payload(reply: &Value) -> Value {
        serde_json::from_str(reply["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
    }

    fn error_text(reply: &Value) -> &str {
        assert_eq!(reply["result"]["isError"], true, "{reply}");
        reply["result"]["content"][0]["text"].as_str().unwrap()
    }

    /// A real parse of a mixed mail: a text attachment, a binary one with a
    /// NUL, an invite, and a part over the budget.
    fn mail(ctx: &McpContext) -> Arc<Email> {
        let big = base64::engine::general_purpose::STANDARD.encode(vec![
            9u8;
            crate::attachments::MAX_ATTACHMENT_STORED_BYTES
                + 1
        ]);
        let nul = base64::engine::general_purpose::STANDARD.encode(b"a\0b");
        let raw = format!(
            "From: a@x.io\r\nTo: b@x.io\r\nSubject: s\r\nMIME-Version: 1.0\r\n\
             Content-Type: multipart/mixed; boundary=\"bd\"\r\n\r\n\
             --bd\r\nContent-Type: text/plain\r\n\r\nhello\r\n\
             --bd\r\nContent-Type: text/plain; name=\"note.txt\"\r\n\
             Content-Disposition: attachment; filename=\"note.txt\"\r\n\r\nline one\r\n\
             --bd\r\nContent-Type: application/octet-stream\r\n\
             Content-Transfer-Encoding: base64\r\n\
             Content-Disposition: attachment; filename=\"nul.bin\"\r\n\r\n{nul}\r\n\
             --bd\r\nContent-Type: text/calendar\r\n\r\n\
             BEGIN:VEVENT\r\nUID:u1\r\nDTSTART;TZID=Europe/Berlin:20261102T150000\r\nEND:VEVENT\r\n\
             --bd\r\nContent-Type: application/octet-stream\r\n\
             Content-Transfer-Encoding: base64\r\n\
             Content-Disposition: attachment; filename=\"big.bin\"\r\n\r\n{big}\r\n\
             --bd--\r\n"
        );
        let email = crate::smtp::build_email(raw.as_bytes(), "box", &[], "a@x.io".into());
        let id = email.id.clone();
        ctx.store.insert(email).unwrap();
        ctx.store.get(&id).unwrap()
    }

    #[tokio::test]
    async fn list_answers_the_recorded_entries() {
        let (_stop, ctx) = context();
        let email = mail(&ctx);
        let reply = answer(&ctx, LIST, json!({"id": email.id})).await;
        let list = payload(&reply);
        assert_eq!(reply["result"]["isError"], false);
        assert_eq!(list["id"], email.id.as_str());
        assert_eq!(
            (list["total"].clone(), list["truncated"].clone()),
            (json!(4), json!(false))
        );
        let entries = list["attachments"].as_array().unwrap();
        let names: Vec<_> = entries.iter().map(|e| e["filename"].clone()).collect();
        assert_eq!(
            names,
            [
                json!("note.txt"),
                json!("nul.bin"),
                json!(null),
                json!("big.bin")
            ]
        );
        assert_eq!(entries[0]["index"], 0);
        assert_eq!(entries[0]["size"], 8);
        assert_eq!(
            entries[0]["sha256"],
            crate::attachments::sha256_hex(b"line one")
        );
        assert_eq!(entries[0]["stored"], true);
        assert_eq!(entries[0]["ics"], Value::Null);
        assert_eq!(entries[2]["content_type"], "text/calendar");
        assert_eq!(entries[2]["ics"]["uid"], "u1");
        assert_eq!(entries[2]["ics"]["dtstart_tzid"], "Europe/Berlin");
        assert_eq!(entries[3]["stored"], false);
        assert_eq!(entries[3]["size"], 1_048_577);
        assert!(entries.iter().all(|e| e.get("content").is_none()));
        // Modern revisions repeat the same JSON as structuredContent.
        let req = json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
                         "params":{"name": LIST, "arguments": {"id": email.id}}});
        let modern = super::super::tool_call(&ctx, &req, Revision::June)
            .await
            .unwrap();
        assert_eq!(modern["structuredContent"], list);
        assert!(reply["result"].get("structuredContent").is_none());
    }

    #[tokio::test]
    async fn a_truncated_list_says_so() {
        let (_stop, ctx) = context();
        let mut email = crate::store::email("long", "box", "a@x.io");
        email.attachments = Some(crate::model::AttachmentList {
            total: 300,
            ..Default::default()
        });
        ctx.store.insert(email).unwrap();
        let list = payload(&answer(&ctx, LIST, json!({"id": "long"})).await);
        assert_eq!(list["total"], 300);
        assert_eq!(list["truncated"], true);
        assert_eq!(list["attachments"], json!([]));
    }

    /// The list answer's bound at its edge (decision 0110 §3): an entry that
    /// brings the charge to exactly `MAX_LIST_JSON_BYTES` is listed, one byte
    /// more is cut, and the cut is a prefix that `truncated` names beside the
    /// exact `total`. A stored part cut from the list stays fetchable.
    #[tokio::test]
    async fn the_list_answer_cuts_a_prefix_at_its_json_bound() {
        let (_stop, ctx) = context();
        let part = |filename: String, content: &[u8]| crate::model::Attachment {
            filename: Some(filename),
            content_type: None,
            size: content.len() as u64,
            sha256: crate::attachments::sha256_hex(content),
            stored: true,
            ics: None,
            content: Some(content.to_vec()),
        };
        // The JSON of an entry with a one-byte name, measured as the handler
        // sends it, so the name's length lands the charge on the edge.
        let mut measured = serde_json::to_value(part("x".into(), b"a")).unwrap();
        measured["index"] = json!(0);
        let overhead = measured.to_string().len() - 1;
        let exact = MAX_LIST_JSON_BYTES - overhead;
        for (name_len, cut) in [(exact, false), (exact + 1, true)] {
            let mut email = crate::store::email(&format!("edge-{name_len}"), "box", "b@x.io");
            let parts = vec![part("n".repeat(name_len), b"a")];
            email.attachments = Some(crate::model::AttachmentList {
                total: 1,
                parts,
                ..Default::default()
            });
            let id = email.id.clone();
            ctx.store.insert(email).unwrap();
            let list = payload(&answer(&ctx, LIST, json!({"id": id})).await);
            assert_eq!(list["total"], 1);
            assert_eq!(list["truncated"], cut);
            assert_eq!(
                list["attachments"].as_array().unwrap().len(),
                usize::from(!cut)
            );
            if cut {
                let fetched = answer(&ctx, GET, json!({"id": id, "index": 0})).await;
                assert_eq!(payload(&fetched)["content"], "a");
            }
        }
    }

    /// A restored row of decision 0108's first listing keeps its labels for
    /// the compact view, but neither tool can answer it: it holds no hash and
    /// no bytes, and its empty list may hide a bare calendar part.
    #[tokio::test]
    async fn a_labels_only_list_answers_unknown() {
        let (_stop, ctx) = context();
        let mut email = crate::store::email("labels", "box", "a@x.io");
        email.attachments = Some(crate::model::AttachmentList {
            labels_only: true,
            ..Default::default()
        });
        ctx.store.insert(email).unwrap();
        for (tool, arguments) in [
            (LIST, json!({"id": "labels"})),
            (GET, json!({"id": "labels", "index": 0})),
        ] {
            let reply = answer(&ctx, tool, arguments).await;
            assert!(error_text(&reply).contains("are unknown"), "{reply}");
        }
    }

    #[tokio::test]
    async fn get_answers_text_as_utf8_and_binary_as_base64() {
        let (_stop, ctx) = context();
        let email = mail(&ctx);
        let text = payload(&answer(&ctx, GET, json!({"id": email.id, "index": 0})).await);
        assert_eq!(text["encoding"], "utf-8");
        assert_eq!(text["content"], "line one");
        assert_eq!(text["filename"], "note.txt");
        assert_eq!(text["content_type"], "text/plain");
        assert_eq!(text["size"], 8);
        assert_eq!(text["index"], 0);
        assert_eq!(text["sha256"], crate::attachments::sha256_hex(b"line one"));
        let binary = payload(&answer(&ctx, GET, json!({"id": email.id, "index": 1})).await);
        assert_eq!(binary["encoding"], "base64");
        assert_eq!(binary["content"], "YQBi");
        assert_eq!(binary["sha256"], crate::attachments::sha256_hex(b"a\0b"));
        let calendar = payload(&answer(&ctx, GET, json!({"id": email.id, "index": 2})).await);
        assert_eq!(calendar["filename"], Value::Null);
        assert_eq!(calendar["encoding"], "utf-8");
    }

    #[tokio::test]
    async fn store_state_refusals_are_error_results_in_order() {
        let (_stop, ctx) = context();
        let email = mail(&ctx);
        let id = &email.id;
        for (tool, arguments) in [
            (LIST, json!({"id": "nope"})),
            (GET, json!({"id": "nope", "index": 0})),
        ] {
            let reply = answer(&ctx, tool, arguments).await;
            assert_eq!(error_text(&reply), "message nope not found");
        }
        let mut unknown = crate::store::email("old", "box", "a@x.io");
        unknown.attachments = None;
        ctx.store.insert(unknown).unwrap();
        for (tool, arguments) in [
            (LIST, json!({"id": "old"})),
            (GET, json!({"id": "old", "index": 99})),
        ] {
            let reply = answer(&ctx, tool, arguments).await;
            assert!(error_text(&reply).starts_with("the attachments of message old are unknown"));
        }
        let reply = answer(&ctx, GET, json!({"id": id, "index": 4})).await;
        assert_eq!(
            error_text(&reply),
            format!("attachment index 4 is past the list of 4 attachments of message {id}")
        );
        let reply = answer(&ctx, GET, json!({"id": id, "index": 3})).await;
        let text = error_text(&reply);
        for needle in [
            "1048577 bytes",
            "1048576 bytes (1 MiB)",
            &format!("GET /api/v1/messages/{id}/raw"),
        ] {
            assert!(text.contains(needle), "{text}");
        }
    }

    /// Malformed arguments refuse `-32602` by name before the store is read.
    #[tokio::test]
    async fn malformed_arguments_refuse_before_the_store_is_read() {
        let (_stop, ctx) = context();
        for (tool, arguments, message) in [
            (LIST, json!({}), "missing id"),
            (LIST, json!({"id": 1}), "id must be a string"),
            (LIST, json!({"id": null}), "id must be a string"),
            (
                LIST,
                json!({"id": "x", "index": 0}),
                "unknown argument: index",
            ),
            (
                GET,
                json!({"id": "x"}),
                "index must be a non-negative integer",
            ),
            (
                GET,
                json!({"id": "x", "index": -1}),
                "index must be a non-negative integer",
            ),
            (
                GET,
                json!({"id": "x", "index": 1.5}),
                "index must be a non-negative integer",
            ),
            (
                GET,
                json!({"id": "x", "index": "0"}),
                "index must be a non-negative integer",
            ),
            (
                GET,
                json!({"id": "x", "index": null}),
                "index must be a non-negative integer",
            ),
            (GET, json!({"index": 0}), "missing id"),
            (
                GET,
                json!({"id": "x", "index": 0, "extra": 1}),
                "unknown argument: extra",
            ),
        ] {
            let reply = answer(&ctx, tool, arguments.clone()).await;
            assert_eq!(
                reply["error"]["code"], -32602,
                "{tool} {arguments}: {reply}"
            );
            assert_eq!(reply["error"]["message"], message, "{tool} {arguments}");
        }
    }

    /// Decision 0005: the handler serves stored fields only — it holds no
    /// parser and never reads the message's raw bytes. The needles are
    /// assembled so this test does not name them itself.
    #[test]
    fn the_handler_holds_no_parser_and_no_raw_read() {
        let source = include_str!("attachments.rs");
        let (handler, _tests) = source.split_once("#[cfg(test)]").unwrap();
        for needle in [
            concat!("mail", "_parser"),
            concat!("Message", "Parser"),
            concat!("build", "_email"),
            concat!(".", "raw"),
            concat!("scan", "_ics"),
        ] {
            assert!(!handler.contains(needle), "the handler names {needle}");
        }
    }

    #[test]
    fn descriptors_state_the_figures_and_both_schemas_are_closed() {
        let [list, get] = descriptors();
        assert_eq!(list["name"], LIST);
        assert_eq!(get["name"], GET);
        for descriptor in [&list, &get] {
            assert_eq!(descriptor["inputSchema"]["additionalProperties"], false);
        }
        assert_eq!(get["inputSchema"]["required"], json!(["id", "index"]));
        let list_text = list["description"].as_str().unwrap();
        assert!(list_text.contains("250"));
        assert!(list_text.contains("SHA-256"));
        let get_text = get["description"].as_str().unwrap();
        assert!(get_text.contains("1 MiB"));
        assert!(get_text.contains("/api/v1/messages/{id}/raw"));
        for schema in [output_schema(true), output_schema(false)] {
            assert_eq!(schema["type"], "object");
            assert_eq!(schema["additionalProperties"], false);
        }
        assert_eq!(
            output_schema(true)["properties"]["attachments"]["maxItems"],
            250
        );
    }
}
