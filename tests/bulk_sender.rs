//! Bulk checklist fixtures cross SMTP 250 and the public report endpoint.
mod common;
use common::*;
use serde_json::{Value, json};

const BASE: &str = "From: sender@example.test\r\nMessage-ID: <bulk@example.test>\r\nDate: Mon, 26 Oct 2026 12:00:00 +0000\r\nList-Unsubscribe: <https://example.test/unsubscribe>\r\nList-Unsubscribe-Post: List-Unsubscribe=One-Click\r\n";
const SOURCES: &str =
    "https://support.google.com/mail/answer/81126 https://senders.yahooinc.com/best-practices/";
const HINTS: [(&str, &str); 4] = [
    (
        "Provide one HTTPS List-Unsubscribe list and one exact List-Unsubscribe-Post: List-Unsubscribe=One-Click value. This structural check does not verify DKIM header coverage or endpoint operation.",
        "https://www.rfc-editor.org/rfc/rfc8058#section-3",
    ),
    (
        "Provide exactly one Message-ID using the supported current RFC 5322 syntax; obsolete-only forms are unsupported by this checklist.",
        "https://www.rfc-editor.org/rfc/rfc5322#section-3.6.4",
    ),
    (
        "Provide exactly one calendar-valid Date using current RFC 5322 syntax and a numeric zone; obsolete-only forms are unsupported by this checklist.",
        "https://www.rfc-editor.org/rfc/rfc5322#section-3.6.1",
    ),
    (
        "Include a sender-supplied text/plain body. This is checklist advice, not proof of equivalent content or an unconditional provider mandate.",
        "https://www.rfc-editor.org/rfc/rfc2046#section-5.1.4",
    ),
];
async fn report(srv: &swarmail::RunningServer, raw: &[u8]) -> (String, u16, Value) {
    let id = smtp_raw(srv.smtp_addr, "bulk", raw).await;
    let (status, value) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{id}/deliverability"),
        None,
    )
    .await;
    (id, status, value)
}
fn finding(value: &Value, id: &str) -> Option<Value> {
    value["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["rule_id"] == id)
        .cloned()
}
fn expected(index: usize, occurrence: Value, header: &str) -> Value {
    let rule = ["one_click", "message_id", "date", "plain_text"][index];
    let (hint, rfc) = HINTS[index];
    json!({"rule_id":format!("bulk.{rule}.v1"), "severity":if index == 3 { "warning" } else { "error" },
        "location":if index == 3 { json!({"kind":"mime_part","path":[]}) } else { json!({"kind":"header","name":header,"occurrence":occurrence}) },
        "fix_hint":format!("{hint} Sources: {rfc} {SOURCES}")})
}

#[tokio::test]
async fn bulk_one_click_identity_date_pairs() {
    let srv = start().await;
    for (index, name, good, bad) in [
        (
            0,
            "List-Unsubscribe",
            "<https://example.test/unsubscribe>",
            "<mailto:leave@example.test>",
        ),
        (
            1,
            "Message-ID",
            "<bulk@example.test>",
            "<bad..id@example.test>",
        ),
        (
            2,
            "Date",
            "Mon, 26 Oct 2026 12:00:00 +0000",
            "Tue, 26 Oct 2026 12:00:00 +0000",
        ),
    ] {
        for (replacement, pass, occurrence) in [
            (Some(good), true, json!(0)),
            (Some(bad), false, json!(0)),
            (None, false, Value::Null),
        ] {
            let headers = BASE.replace(
                &format!("{name}: {good}\r\n"),
                &replacement
                    .map(|v| format!("{name}: {v}\r\n"))
                    .unwrap_or_default(),
            );
            let (_, status, value) =
                report(&srv, format!("{headers}\r\nbody\r\n").as_bytes()).await;
            assert_eq!(status, 200, "{value}");
            assert_eq!(
                finding(
                    &value,
                    &format!("bulk.{}.v1", ["one_click", "message_id", "date"][index])
                ),
                if pass {
                    None
                } else {
                    Some(expected(index, occurrence, &name.to_ascii_lowercase()))
                }
            );
        }
        let (_, status, value) = report(
            &srv,
            format!("{BASE}{name}: {good}\r\n\r\nbody\r\n").as_bytes(),
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(
            finding(
                &value,
                &format!("bulk.{}.v1", ["one_click", "message_id", "date"][index])
            ),
            Some(expected(index, json!(1), &name.to_ascii_lowercase()))
        );
    }
    for list in [
        "<HTTPS://example.test:443/u>",
        "<mailto:leave@example.test>,\r\n\t<https://example.test/u>",
    ] {
        let headers = BASE.replace(
            "List-Unsubscribe: <https://example.test/unsubscribe>",
            &format!("lIsT-uNsUbScRiBe: {list}"),
        );
        assert!(
            finding(
                &report(&srv, format!("{headers}\r\nb\r\n").as_bytes())
                    .await
                    .2,
                "bulk.one_click.v1"
            )
            .is_none()
        );
    }
    for list in [
        "<https://x/%zz>",
        "<https://x>junk",
        "<https://u@x/u>",
        "<https://x/#f>",
        "<https://x:99999>",
        "<https://x>, broken",
    ] {
        let headers = BASE.replace("<https://example.test/unsubscribe>", list);
        let (_, status, value) = report(&srv, format!("{headers}\r\nb\r\n").as_bytes()).await;
        assert_eq!(status, 200);
        assert_eq!(
            finding(&value, "bulk.one_click.v1"),
            Some(expected(0, json!(0), "list-unsubscribe"))
        );
    }
    for (post, occurrence) in [
        ("", Value::Null),
        (
            "List-Unsubscribe-Post: list-unsubscribe=One-Click\r\n",
            json!(0),
        ),
        (
            "List-Unsubscribe-Post: List-Unsubscribe=One-Click; x=1\r\n",
            json!(0),
        ),
        (
            "List-Unsubscribe-Post: List-Unsubscribe=One-Click\r\nList-Unsubscribe-Post: List-Unsubscribe=One-Click\r\n",
            json!(1),
        ),
    ] {
        let headers = BASE.replace(
            "List-Unsubscribe-Post: List-Unsubscribe=One-Click\r\n",
            post,
        );
        let value = report(&srv, format!("{headers}\r\nb\r\n").as_bytes())
            .await
            .2;
        assert_eq!(
            finding(&value, "bulk.one_click.v1"),
            Some(expected(0, occurrence, "list-unsubscribe-post"))
        );
    }
    for (name, value, rule) in [
        (
            "Message-ID",
            "(pre)\r\n\t<left@[x(y)]> (post)",
            "message_id",
        ),
        (
            "Date",
            "(pre) 29 Feb 2000\r\n\t23:59:60 -0000 (post)",
            "date",
        ),
        ("Date", "Mon, 26 Oct 000000002026 12:00 +0000", "date"),
        ("Date", "29 Feb 1000000000000000 12:00 +0000", "date"),
    ] {
        let line = BASE.lines().find(|line| line.starts_with(name)).unwrap();
        let raw = format!(
            "{}\r\nbody\r\n",
            BASE.replace(line, &format!("{name}: {value}"))
        );
        assert!(
            finding(
                &report(&srv, raw.as_bytes()).await.2,
                &format!("bulk.{rule}.v1")
            )
            .is_none()
        );
    }
    for (index, name, value) in [
        (1, "Message-ID", &b"<a@x>\xff"[..]),
        (1, "Message-ID", b"<a@x>\0"),
        (1, "Message-ID", b"<a@x> <b@y>"),
        (2, "Date", b"31 Apr 2026 12:00 +0000"),
        (2, "Date", b"26 Oct 2026 12:00 GMT"),
    ] {
        let line = BASE.lines().find(|line| line.starts_with(name)).unwrap();
        let mut raw = BASE.replace(&format!("{line}\r\n"), "").into_bytes();
        raw.extend_from_slice(format!("{name}: ").as_bytes());
        raw.extend_from_slice(value);
        raw.extend_from_slice(b"\r\n\r\nb\r\n");
        let value = report(&srv, &raw).await.2;
        assert_eq!(
            finding(
                &value,
                &format!("bulk.{}.v1", ["one_click", "message_id", "date"][index])
            ),
            Some(expected(index, json!(0), &name.to_ascii_lowercase()))
        );
    }
    srv.stop().await;
}

#[tokio::test]
async fn bulk_plain_text_pairs_preserve_stored_fields() {
    let srv = start().await;
    for (mime, pass) in [
        ("\r\n", true),
        ("Content-Type: text/plain\r\n\r\n", true),
        (
            "Content-Type: text/plain\r\nContent-Transfer-Encoding: base64\r\n\r\nNDI0MjQy\r\n",
            true,
        ),
        (
            "Content-Type: text/html\r\n\r\n<p>424242 https://example.test/</p>\r\n",
            false,
        ),
        (
            "Content-Type: text/plain; name=note.txt\r\n\r\nbody\r\n",
            false,
        ),
        ("Content-Disposition: attachment\r\n\r\nbody\r\n", false),
        (
            "Content-Disposition: inline; filename*=utf-8''note.txt\r\n\r\nbody\r\n",
            false,
        ),
        (
            "Content-Disposition: inline; filename=note.txt\r\n\r\nbody\r\n",
            false,
        ),
        (
            "Content-Type: text/plain; name*=utf-8''note.txt\r\n\r\nbody\r\n",
            false,
        ),
        (
            "Content-Type: message/rfc822\r\n\r\nContent-Type: text/plain\r\n\r\nembedded\r\n",
            false,
        ),
        (
            "Content-Type: multipart/digest; boundary=x\r\n\r\n--x\r\n\r\nContent-Type: text/plain\r\n\r\nembedded\r\n--x--\r\n",
            false,
        ),
        (
            "Content-Type: multipart/mixed; boundary=x\r\n\r\n--x\r\nContent-Type: text/plain\r\n\r\nbody\r\n--x--\r\n",
            true,
        ),
        (
            "Content-Type: multipart/mixed; boundary=x\r\nContent-Disposition: attachment\r\n\r\n--x\r\n\r\nbody\r\n--x--\r\n",
            false,
        ),
        (
            "Content-Type: multipart/mixed; boundary=x\r\n\r\n--x\r\nContent-Type: multipart/alternative; boundary=y\r\n\r\n--y\r\n\r\nbody 424242 https://example.test/\r\n--y--\r\n--x--\r\n",
            true,
        ),
    ] {
        let raw = format!("{BASE}{mime}");
        let id = smtp_raw(srv.smtp_addr, "plain", raw.as_bytes()).await;
        let path = format!("/api/v1/messages/{id}");
        let before = http_json(srv.http_addr, "GET", &path, None).await;
        let (status, first) = http_json(
            srv.http_addr,
            "GET",
            &format!("{path}/deliverability"),
            None,
        )
        .await;
        assert_eq!(status, 200, "{mime}: {first}");
        assert_eq!(
            finding(&first, "bulk.plain_text.v1"),
            if pass {
                None
            } else {
                Some(expected(3, Value::Null, ""))
            },
            "{mime}"
        );
        assert_eq!(
            http_json(
                srv.http_addr,
                "GET",
                &format!("{path}/deliverability"),
                None
            )
            .await
            .1,
            first
        );
        assert_eq!(http_json(srv.http_addr, "GET", &path, None).await, before);
        assert_eq!(
            http_bytes(srv.http_addr, &format!("{path}/raw")).await,
            (200, raw.into_bytes())
        );
    }
    srv.stop().await;
}

#[tokio::test]
async fn bulk_untrusted_auth_headers_stay_unknown() {
    let srv = start().await;
    for from in [
        "",
        "From: sender@example.test\r\n",
        "From: a@x\r\nFrom: b@y\r\n",
    ] {
        let raw = format!(
            "{}Authentication-Results: mx; dmarc=pass; dkim=pass; spf=pass\r\nDKIM-Signature: v=1; d=example.test; b=fake\r\n{from}\r\nbody\r\n",
            BASE.replace("From: sender@example.test\r\n", "")
        );
        let (_, status, value) = report(&srv, raw.as_bytes()).await;
        assert_eq!(status, 200);
        assert_eq!(value["rule_set_id"], "swarmail-html/3");
        let occurrence = match from {
            "" => Value::Null,
            "From: sender@example.test\r\n" => json!(0),
            _ => json!(1),
        };
        let mut expected = Vec::new();
        for (rule, hint, rfc) in [
            (
                "authenticated_sender",
                "Not evaluated: trusted From identity or passing-and-aligned DKIM/SPF evidence is incomplete.",
                "https://www.rfc-editor.org/rfc/rfc7489#section-3.1",
            ),
            (
                "dmarc_presence",
                "Not evaluated: trusted applicable DMARC policy evidence is unavailable.",
                "https://www.rfc-editor.org/rfc/rfc7489#section-6.1",
            ),
            (
                "from_alignment",
                "Not evaluated: trusted From identity or DKIM/SPF alignment evidence is incomplete.",
                "https://www.rfc-editor.org/rfc/rfc7489#section-3.1",
            ),
        ] {
            expected.push(json!({"rule_id":format!("bulk.{rule}.not_evaluated.v1"),"severity":"error","location":{"kind":"header","name":"from","occurrence":occurrence},"fix_hint":format!("{hint} Sources: {rfc} {SOURCES}")}));
        }
        assert_eq!(
            value,
            json!({"rule_set_id":"swarmail-html/3","findings":expected})
        );
    }
    srv.stop().await;
}

fn padded_headers(prefix: &str, size: usize) -> String {
    let mut header = format!("{prefix}X-Pad: ");
    header.extend(std::iter::repeat_n('x', size - header.len() - 4));
    header.push_str("\r\n\r\n");
    assert_eq!(header.len(), size);
    header
}
fn multipart(parts: usize) -> String {
    format!(
        "Content-Type: multipart/mixed; boundary=x\r\n\r\n{}--x--\r\n",
        "--x\r\n\r\nbody\r\n".repeat(parts)
    )
}
fn nested(depth: usize) -> String {
    let mut raw = "\r\nbody\r\n".to_string();
    for n in 0..depth {
        raw = format!(
            "Content-Type: multipart/mixed; boundary=b{n}\r\n\r\n--b{n}\r\n{raw}--b{n}--\r\n"
        );
    }
    raw
}
#[tokio::test]
async fn bulk_analysis_budget_boundaries_and_incomplete_mime() {
    let srv = start().await;
    let mut fixtures = Vec::new();
    for (over, expected) in [(0, 200), (1, 503)] {
        let mut raw = format!("{BASE}\r\n");
        raw.extend(std::iter::repeat_n('x', 8_388_608 + over - raw.len() - 2));
        raw.push_str("\r\n");
        fixtures.push(("raw", raw, expected));
        fixtures.push((
            "header",
            format!("{}body\r\n", padded_headers("", 65_536 + over)),
            expected,
        ));
        let root = padded_headers(
            "Content-Type: multipart/mixed; boundary=x\r\n",
            65_534 + over,
        );
        let child = padded_headers("", 65_536);
        fixtures.push((
            "aggregate",
            format!("{root}--x\r\n{child}--x\r\n{child}--x\r\n{child}--x\r\n\r\n--x--\r\n"),
            expected,
        ));
        fixtures.push(("parts", multipart(255 + over), expected));
        fixtures.push(("depth", nested(32 + over), expected));
    }
    for raw in [
        "Subject: no separator\r\n",
        "Content-Type: multipart/mixed; boundary=x\r\n\r\n--x\r\n\r\nplain\r\n",
        "Content-Type: multipart/mixed; boundary=x\r\n\r\n--x\r\n\r\nplain\r\n--x\r\nContent-Type: text/html\r\n",
        "Content-Type: text/plain\r\nContent-Type: text/html\r\n\r\nbody\r\n",
        "Content-Type: multipart/mixed; boundary=x\r\nContent-Transfer-Encoding: base64\r\n\r\nLS14\r\n",
        "Content-Type: multipart/mixed; boundary=\"bad@boundary\"\r\n\r\nbody\r\n",
    ] {
        fixtures.push(("incomplete", raw.into(), 503));
    }
    for (name, raw, expected) in fixtures {
        let (id, status, value) = report(&srv, raw.as_bytes()).await;
        assert_eq!(status, expected, "{name}: {value}");
        if expected == 503 {
            assert_eq!(value, json!({"error":"deliverability analysis failed"}));
        }
        assert_eq!(
            http_bytes(srv.http_addr, &format!("/api/v1/messages/{id}/raw")).await,
            (200, raw.into_bytes()),
            "{name}"
        );
    }
    srv.stop().await;
}
