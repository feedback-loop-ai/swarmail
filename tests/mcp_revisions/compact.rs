//! Decision 0092 over the real wire: the compact default of the two list
//! reads, the `fields` projection of the four reads, and March's frozen
//! vocabulary. Every case runs on June and November, over HTTP and over the
//! spawned stdio bridge, against a mail delivered through real SMTP.
use super::structured::{Wire, valid};
use super::*;
use std::collections::BTreeSet;

/// One multipart mail: text and html alternatives with two anchors, a code,
/// and a named PDF (`%PDF-`, five decoded bytes).
const MAIL: &[u8] = b"From: Ann <ann@x.io>\r\nTo: bob@x.io\r\nSubject: Quarterly\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=b\r\n\r\n\
--b\r\nContent-Type: multipart/alternative; boundary=a\r\n\r\n\
--a\r\nContent-Type: text/plain\r\n\r\nSee the links, code 482913\r\n\
--a\r\nContent-Type: text/html\r\n\r\n<a href=\"https://x.io/one\">One</a> <a href=\"https://x.io/two\">Two</a>\r\n--a--\r\n\
--b\r\nContent-Type: application/pdf; name=\"q3.pdf\"\r\nContent-Disposition: attachment; filename=\"q3.pdf\"\r\nContent-Transfer-Encoding: base64\r\n\r\nJVBERi0=\r\n--b--\r\n";

/// The compact item of [`MAIL`], stated member by member.
fn compact(id: &Value, received_ms: &Value) -> Value {
    json!({
        "anchors": [
            {"anchor_text": "One", "url": "https://x.io/one"},
            {"anchor_text": "Two", "url": "https://x.io/two"}
        ],
        "attachments": [{"content_type": "application/pdf", "filename": "q3.pdf", "size": 5}],
        "codes": ["482913"],
        "from": {"address": "ann@x.io", "name": "Ann"},
        "id": id,
        "received_ms": received_ms,
        "snippet": "See the links, code 482913",
        "subject": "Quarterly",
        "to": [{"address": "bob@x.io"}],
    })
}

/// The wire under test plus the served output schemas of its session.
struct Session {
    wire: Wire,
    schemas: Value,
}

impl Session {
    async fn open(addr: std::net::SocketAddr, version: &'static str, stdio: bool) -> Self {
        let mut wire = Wire::start(addr, version, stdio).await;
        let list = wire.request(&json!({"id":1,"method":"tools/list"})).await;
        let schemas = list["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| {
                (
                    t["name"].as_str().unwrap().to_string(),
                    t["outputSchema"].clone(),
                )
            })
            .collect::<serde_json::Map<_, _>>()
            .into();
        Self { wire, schemas }
    }

    /// A successful call: both representations agree and validate against
    /// the tool's served output schema.
    async fn ok(&mut self, tool: &str, arguments: Value) -> Value {
        let reply = self.wire.request(&call(json!(2), tool, arguments)).await;
        assert_eq!(reply["result"]["isError"], false, "{tool}: {reply}");
        let payload = text(&reply);
        assert_eq!(payload, reply["result"]["structuredContent"], "{tool}");
        assert!(
            valid(&self.schemas[format!("swarmail_{tool}")], &payload),
            "{tool}: {payload}"
        );
        payload
    }

    /// A staging refusal: the JSON-RPC error, as (code, message).
    async fn refused(&mut self, tool: &str, arguments: Value) -> (i64, String) {
        let reply = self.wire.request(&call(json!(3), tool, arguments)).await;
        assert!(reply.get("result").is_none(), "{tool}: {reply}");
        (
            reply["error"]["code"].as_i64().unwrap(),
            reply["error"]["message"].as_str().unwrap().to_string(),
        )
    }
}

fn keys(item: &Value) -> BTreeSet<&str> {
    item.as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
}

#[tokio::test]
async fn modern_compact_default_and_fields_over_http_and_stdio() {
    let srv = start().await;
    let id: Value = common::smtp_raw(srv.smtp_addr, "box", MAIL).await.into();
    let (status, full) = common::http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{}", id.as_str().unwrap()),
        None,
    )
    .await;
    assert_eq!(status, 200);
    let expected = compact(&id, &full["received_ms"]);
    // The mail's own members the full view carries, plus the three derived.
    let derived = ["snippet", "anchors", "attachments"];
    for version in &VERSIONS[1..] {
        for stdio in [false, true] {
            let mut s = Session::open(srv.http_addr, version, stdio).await;

            // The default of both list reads is the compact item, in the text
            // block and in structuredContent alike, with no body.
            let search = s.ok("search_emails", json!({"inbox":"box"})).await;
            assert_eq!(
                search,
                json!({"inbox":"box","total":1,"emails":[expected.clone()]}),
                "{version} stdio={stdio}"
            );
            let latest = s.ok("get_latest_email", json!({"inbox":"box"})).await;
            assert_eq!(latest, json!({"emails":[expected.clone()]}));
            for item in search["emails"].as_array().unwrap() {
                assert!(item.get("text").is_none() && item.get("html").is_none());
            }

            // get_email without fields is the whole email, REST's bytes.
            let whole = s.ok("get_email", json!({"id":id})).await;
            assert_eq!(whole, full);
            // A wait without fields is the whole email too.
            let wait = s
                .ok("wait_for_email", json!({"inbox":"box","timeout_ms":0}))
                .await;
            assert_eq!(wait, json!({"matched":1,"total":1,"emails":[full.clone()]}));

            // Each single name on each of the four reads yields exactly that
            // member (when the mail carries it) plus the id, with the value
            // the full view or the compact rules give it.
            for name in crate::FIELD_NAMES {
                let value = if derived.contains(&name) {
                    expected.get(name)
                } else {
                    full.get(name)
                };
                let mut want = json!({"id": id});
                if let Some(value) = value {
                    want[name] = value.clone();
                }
                let args = |extra: Value| {
                    let mut a = extra;
                    a["fields"] = json!([name]);
                    a
                };
                let search = s.ok("search_emails", args(json!({"inbox":"box"}))).await;
                assert_eq!(search["emails"], json!([want.clone()]), "search {name}");
                let latest = s.ok("get_latest_email", args(json!({"inbox":"box"}))).await;
                assert_eq!(latest["emails"], json!([want.clone()]), "latest {name}");
                let one = s.ok("get_email", args(json!({"id":id}))).await;
                assert_eq!(one, want, "get_email {name}");
                let wait = s
                    .ok(
                        "wait_for_email",
                        args(json!({"inbox":"box","timeout_ms":0})),
                    )
                    .await;
                assert_eq!(wait["emails"], json!([want]), "wait {name}");
                assert_eq!(
                    (wait["matched"].clone(), wait["total"].clone()),
                    (json!(1), json!(1))
                );
            }

            // Several names at once, repeated and in any order: the id plus
            // the named members, the keys in the server's one order.
            let two = s
                .ok(
                    "get_email",
                    json!({"id":id,"fields":["text","subject","text","anchors"]}),
                )
                .await;
            assert_eq!(
                keys(&two),
                BTreeSet::from(["anchors", "id", "subject", "text"])
            );
            assert_eq!(two["text"], full["text"]);
            // The control: bodies never appear unless named.
            let lean = s
                .ok(
                    "search_emails",
                    json!({"inbox":"box","fields":["subject","snippet"]}),
                )
                .await;
            assert_eq!(
                keys(&lean["emails"][0]),
                BTreeSet::from(["id", "snippet", "subject"])
            );

            // An empty list is the ids alone, on every read.
            for (tool, args) in [
                ("search_emails", json!({"inbox":"box","fields":[]})),
                ("get_latest_email", json!({"inbox":"box","fields":[]})),
                (
                    "wait_for_email",
                    json!({"inbox":"box","timeout_ms":0,"fields":[]}),
                ),
            ] {
                let payload = s.ok(tool, args).await;
                assert_eq!(payload["emails"], json!([{"id": id}]), "{tool}");
            }
            assert_eq!(
                s.ok("get_email", json!({"id":id,"fields":[]})).await,
                json!({"id": id})
            );

            // The timeout arm of a wait is unchanged by `fields`.
            let idle = s
                .ok(
                    "wait_for_email",
                    json!({"inbox":"idle","timeout_ms":0,"fields":["subject"]}),
                )
                .await;
            assert_eq!(idle, json!({"matched":0,"total":0,"timed_out":true}));

            // Refusals: the sentence by name or by type, -32602, and before
            // any lookup — a missing id with a bad name answers the refusal.
            let unknown = "fields must be an array of known field names (unknown: nope)";
            for (tool, args) in [
                (
                    "search_emails",
                    json!({"inbox":"box","fields":["subject","nope"]}),
                ),
                ("get_latest_email", json!({"inbox":"box","fields":["nope"]})),
                ("get_email", json!({"id":"missing","fields":["nope"]})),
                (
                    "wait_for_email",
                    json!({"inbox":"idle","timeout_ms":0,"fields":["nope"]}),
                ),
            ] {
                assert_eq!(
                    s.refused(tool, args).await,
                    (-32602, unknown.to_string()),
                    "{tool}"
                );
            }
            for bad in [
                json!("subject"),
                json!(null),
                json!(7),
                json!({}),
                json!([1]),
                json!(["id", null]),
            ] {
                for (tool, args) in [
                    ("search_emails", json!({"inbox":"box"})),
                    ("get_latest_email", json!({"inbox":"box"})),
                    ("get_email", json!({"id":"missing"})),
                    ("wait_for_email", json!({"inbox":"idle","timeout_ms":0})),
                ] {
                    let mut args = args;
                    args["fields"] = bad.clone();
                    assert_eq!(
                        s.refused(tool, args).await,
                        (-32602, "fields must be an array of strings".to_string()),
                        "{tool} {bad}"
                    );
                }
            }
            // A missing id without fields keeps its own answer.
            let reply = s
                .wire
                .request(&call(json!(4), "get_email", json!({"id":"missing"})))
                .await;
            assert_eq!(reply["result"]["isError"], true);

            // `fields` belongs to the four reads: every other tool keeps its
            // closed vocabulary.
            for (tool, args) in [
                ("get_deliverability_report", json!({"id":id})),
                ("extract_links", json!({"id":id})),
                ("extract_codes", json!({"id":id})),
                ("delete_email", json!({"id":id})),
                ("seed_email", json!({"to":"a@x.io"})),
                ("wait_for_code", json!({"inbox":"box","timeout_ms":0})),
                ("wait_for_link", json!({"inbox":"box","timeout_ms":0})),
                ("clear_inbox", json!({"inbox":"idle"})),
            ] {
                let mut args = args;
                args["fields"] = json!(["id"]);
                assert_eq!(
                    s.refused(tool, args).await,
                    (-32602, "unknown argument: fields".to_string()),
                    "{tool}"
                );
            }
            // Nothing above stored, removed or changed a mail.
            assert_eq!(s.ok("get_email", json!({"id":id})).await, full);
            s.wire.finish().await;
        }
    }
    srv.stop().await;
}

/// A refused `fields` on a wait registers no waiter: the cumulative
/// registration row (the one `park_await` counts at its single site) does
/// not move, and the answer is immediate though the timeout is a minute.
#[tokio::test]
async fn a_refused_fields_never_parks_the_wait() {
    let srv = start().await;
    let row = r#"swarmail_awaits_registered_total{inbox="never"}"#;
    let read = || async {
        let (_, metrics) = common::http_get_text(srv.http_addr, "/metrics").await;
        common::metric_row_value(&metrics, row)
    };
    for version in &VERSIONS[1..] {
        for stdio in [false, true] {
            let before = read().await;
            let mut s = Session::open(srv.http_addr, version, stdio).await;
            let refused = tokio::time::timeout(
                Duration::from_secs(10),
                s.refused(
                    "wait_for_email",
                    json!({"inbox":"never","timeout_ms":60000,"fields":["nope"]}),
                ),
            )
            .await
            .expect("the refusal is staged before the wait parks");
            assert_eq!(refused.0, -32602);
            assert_eq!(read().await, before, "{version} stdio={stdio}");
            s.wire.finish().await;
        }
    }
    // The control: a valid wait does register, so the row can move.
    let mut s = Session::open(srv.http_addr, VERSIONS[2], false).await;
    let idle = s
        .ok(
            "wait_for_email",
            json!({"inbox":"never","timeout_ms":0,"fields":["subject"]}),
        )
        .await;
    assert_eq!(idle["timed_out"], true);
    assert_eq!(read().await, Some(1));
    srv.stop().await;
}

/// March keeps its frozen vocabulary (decision 0082 §1): no `fields`, whole
/// emails from search and latest, on the missing-version default and on the
/// explicit March header alike.
#[tokio::test]
async fn march_has_no_fields_and_keeps_whole_emails() {
    let srv = start().await;
    let id: Value = common::smtp_raw(srv.smtp_addr, "box", MAIL).await.into();
    let (_, full) = common::http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{}", id.as_str().unwrap()),
        None,
    )
    .await;
    for version in [None, Some(VERSIONS[0])] {
        let mut wire = Wire::Http(srv.http_addr, version);
        for (tool, args) in [
            ("search_emails", json!({"inbox":"box","fields":["id"]})),
            ("get_latest_email", json!({"inbox":"box","fields":["id"]})),
            ("get_email", json!({"id":id,"fields":["id"]})),
            (
                "wait_for_email",
                json!({"inbox":"box","timeout_ms":0,"fields":["id"]}),
            ),
        ] {
            let reply = wire.request(&call(json!(2), tool, args)).await;
            assert_eq!(reply["error"]["code"], -32602, "{tool}");
            assert_eq!(
                reply["error"]["message"], "unknown argument: fields",
                "{tool}"
            );
        }
        let search = wire
            .request(&call(json!(3), "search_emails", json!({"inbox":"box"})))
            .await;
        assert_eq!(text(&search)["emails"], json!([full.clone()]));
        assert!(search["result"].get("structuredContent").is_none());
        let latest = wire
            .request(&call(json!(4), "get_latest_email", json!({"inbox":"box"})))
            .await;
        assert_eq!(text(&latest)["emails"], json!([full.clone()]));
    }
    srv.stop().await;
}

/// A newsletter-shaped mail built by ingest and then pinned to fixed
/// volatile members: a text and an HTML alternative of a few kilobytes, ten
/// labelled links and a PDF.
fn newsletter() -> swarmail::model::Email {
    let paragraph = "Our quarterly roundup covers product updates, customer stories, upcoming events and a few notes from the team about what we are building next. ";
    let text: String = (0..10)
        .map(|i| format!("{paragraph}Read more: https://news.example.test/story/{i}\r\n"))
        .collect();
    let html: String = (0..10)
        .map(|i| {
            format!(
                "<p style=\"margin:0 0 12px\">{paragraph}<a href=\"https://news.example.test/story/{i}\" style=\"color:#06c\">Read story {i}</a></p>\r\n"
            )
        })
        .collect();
    let raw = format!(
        "From: News <news@example.test>\r\nTo: reader@x.io\r\nSubject: Quarterly roundup\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=b\r\n\r\n\
         --b\r\nContent-Type: multipart/alternative; boundary=a\r\n\r\n\
         --a\r\nContent-Type: text/plain\r\n\r\n{text}--a\r\nContent-Type: text/html\r\n\r\n{html}--a--\r\n\
         --b\r\nContent-Type: application/pdf; name=\"roundup.pdf\"\r\nContent-Disposition: attachment; filename=\"roundup.pdf\"\r\nContent-Transfer-Encoding: base64\r\n\r\nJVBERi0=\r\n--b--\r\n"
    );
    let mut mail = swarmail::smtp::build_email(
        raw.as_bytes(),
        "news",
        &["reader@x.io".to_string()],
        "news@example.test".into(),
    );
    mail.id = "newsletter".into();
    mail.received_at = "2026-10-30T12:00:00Z".into();
    mail.received_ms = 1_793_361_600_000;
    mail
}

/// The byte table of the story (`specs/compact-email-view/implementation.md`),
/// asserted here so its figures come from the served bytes and not from prose.
/// They are bytes, never tokens: the token metric is not measured by this
/// change (no paid trial). Both fixtures are constants: the restored row of
/// the structured-output baseline (65 stored codes make it code-heavy) and a
/// newsletter-shaped mail.
#[tokio::test]
async fn the_story_byte_table_is_what_the_wire_serves() {
    let srv = start().await;
    srv.store.insert(super::structured::historical()).unwrap();
    srv.store.insert(newsletter()).unwrap();
    let mut march = Wire::Http(srv.http_addr, None);
    let mut modern = Wire::Http(srv.http_addr, Some(VERSIONS[2]));
    let mut table = Vec::new();
    let mut anchors = Vec::new();
    for (inbox, id) in [("contract", "historical"), ("news", "newsletter")] {
        let search = call(json!(2), "search_emails", json!({"inbox":inbox}));
        let get = call(json!(2), "get_email", json!({"id":id}));
        let march_line = march.raw(&search).await;
        let march_reply: Value = serde_json::from_str(&march_line).unwrap();
        let march_payload = text(&march_reply);
        let march_item = march_payload["emails"][0].to_string();

        let compact_line = modern.raw(&search).await;
        let compact_reply: Value = serde_json::from_str(&compact_line).unwrap();
        assert_eq!(
            text(&compact_reply),
            compact_reply["result"]["structuredContent"]
        );
        let compact_item = text(&compact_reply)["emails"][0].to_string();
        anchors.push(
            text(&compact_reply)["emails"][0]
                .get("anchors")
                .map(|list| list.as_array().unwrap().len()),
        );

        // Before this change a modern search line carried the March text
        // block and then the same payload again as structuredContent.
        let doubled =
            march_line.len() + ",\"structuredContent\":".len() + march_payload.to_string().len();
        // get_email stays the whole email, in both representations.
        let get_line = modern.raw(&get).await;
        table.push([
            march_item.len(),
            compact_item.len(),
            doubled,
            compact_line.len(),
            get_line.len(),
        ]);
    }
    // [March full item, compact item, the modern search line as it was
    // (both representations), the compact search line now, the whole
    // get_email line]: bytes.
    assert_eq!(
        table,
        [
            [949, 714, 2324, 1812, 2232],
            [5214, 1836, 10852, 4050, 10768]
        ]
    );
    srv.stop().await;
}
