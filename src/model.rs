//! Core data model.

use serde::de::{self, MapAccess, Visitor, value::MapAccessDeserializer};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

/// The blank-inbox refusal (decision 0044 §3): one sentence, every plane.
pub const BLANK_INBOX_REFUSAL: &str = "inbox must not be blank";

/// The control-character refusal (decision 0046 §4): one sentence, every
/// plane.
pub const CONTROL_INBOX_REFUSAL: &str = "inbox must not contain control characters";

/// The one inbox-name admission rule (decisions 0044 §3, 0046 §4): a
/// client-supplied inbox that is empty after Unicode whitespace trimming
/// names nothing and is refused — an empty name was stored and then linked
/// to a 404 (issue #91) — and so is one carrying any Unicode control
/// character (C0, DEL, C1): a NUL-separated credential must never become an
/// inbox name holding its password (issue #128). Any other name is kept
/// verbatim, padding included: no trimming, folding or normalization, and
/// no stored row is renamed. Every admitting plane calls this — REST, MCP,
/// UI, compat, the shared seed builder, webhook inbox filters, SMTP AUTH
/// and POP3 `USER` — so one name means one inbox on all of them.
pub fn validate_inbox(name: &str) -> Result<&str, &'static str> {
    if name.trim().is_empty() {
        Err(BLANK_INBOX_REFUSAL)
    } else if name.contains(char::is_control) {
        Err(CONTROL_INBOX_REFUSAL)
    } else {
        Ok(name)
    }
}

/// A supplied optional string (decision 0044 §3, 0025 §2): an omitted key
/// is `None` through `#[serde(default)]`, while a *present* value must be a
/// string — an explicit `null` refuses like any other wrong type instead of
/// riding the default. Pair with `#[serde(default)]`.
pub fn present_string<'de, D: Deserializer<'de>>(de: D) -> Result<Option<String>, D::Error> {
    String::deserialize(de).map(Some)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmailAddress {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub address: String,
}

/// One link occurrence as ingest found it (decision 0094 §1): every URL
/// match in source order, text part first, before the exact-string
/// deduplication `links` applies. Labels are lexical, never rendered: a
/// `None` is unknown, `Some("")` a known empty label. Every member is
/// required on restore — a missing nullable member is corruption, not an
/// implicit null — and an unknown member refuses (decision 0094 §2).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LinkMetadata {
    pub url: String,
    pub anchor_text: Option<String>,
    pub rel: Option<String>,
    pub source: LinkSource,
}

/// The closed member set an occurrence object restores from. A derived
/// struct reader also admits a positional array, so it is reached only
/// through [`LinkMetadata`]'s map-only reader (decision 0097 §2), which
/// decodes the raw entries: a duplicate key refuses rather than collapsing.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Members {
    url: String,
    #[serde(deserialize_with = "required_nullable")]
    anchor_text: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    rel: Option<String>,
    source: LinkSource,
}

impl<'de> Deserialize<'de> for LinkMetadata {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Object;
        impl<'de> Visitor<'de> for Object {
            type Value = LinkMetadata;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a link occurrence object")
            }
            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<LinkMetadata, A::Error> {
                let Members {
                    url,
                    anchor_text,
                    rel,
                    source,
                } = Members::deserialize(MapAccessDeserializer::new(map))?;
                Ok(LinkMetadata {
                    url,
                    anchor_text,
                    rel,
                    source,
                })
            }
        }
        de.deserialize_map(Object)
    }
}

/// A nullable member that must be present. A plain `Option` field takes a
/// missing key as `None`; naming a deserializer turns that default off.
fn required_nullable<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    de: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(de)
}

/// The calendar fields ingest read from a part's first `VEVENT` (decision
/// 0110 §4): the values exactly as the calendar spelled them — no timezone
/// resolution, no instant — except `summary`, whose RFC 5545 text escapes
/// are resolved. Every member is required on restore (a missing nullable
/// member is corruption, not an implicit null) and an unknown member
/// refuses, through the same map-only reader the link occurrences use.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct IcsEvent {
    pub dtstart: Option<String>,
    pub dtstart_tzid: Option<String>,
    pub dtend: Option<String>,
    pub dtend_tzid: Option<String>,
    pub summary: Option<String>,
    pub uid: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IcsMembers {
    #[serde(deserialize_with = "required_nullable")]
    dtstart: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    dtstart_tzid: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    dtend: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    dtend_tzid: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    summary: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    uid: Option<String>,
}

/// One of a mail's listed attachments as ingest recorded it (decision 0110
/// §1): label, decoded size, SHA-256 of exactly those bytes, whether the
/// bytes were kept under the per-mail budget, and for a calendar part its
/// fields. `content` is `Some` exactly when `stored`; it is never part of
/// the persisted object (the bytes ride their own column) and never of any
/// Email serialization.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Attachment {
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub size: u64,
    pub sha256: String,
    pub stored: bool,
    pub ics: Option<IcsEvent>,
    #[serde(skip)]
    pub content: Option<Vec<u8>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AttachmentMembers {
    #[serde(deserialize_with = "required_nullable")]
    filename: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    content_type: Option<String>,
    size: u64,
    sha256: String,
    stored: bool,
    #[serde(deserialize_with = "required_nullable")]
    ics: Option<IcsEvent>,
}

/// A mail's attachment list: the exact candidate `total` and the first
/// entries (at most `MAX_ATTACHMENTS`) in part order. Held as `Some` on
/// every parsed mail (`Some` of an empty list is known empty) and `None`
/// where the list is unknown — a row from before the columns, or a mail
/// the parser could not read. Internal: no Email serialization carries it.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct AttachmentList {
    pub total: u64,
    pub parts: Vec<Attachment>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListMembers {
    total: u64,
    parts: Vec<Attachment>,
}

/// Generates the strict map-only reader for a persisted object: the derived
/// reader behind `$members` also admits a positional array, so the public
/// type reaches it only through `deserialize_map` (decision 0097 §2).
macro_rules! map_only {
    ($ty:ident, $members:ident, $what:literal, |$m:ident| $build:expr) => {
        impl<'de> Deserialize<'de> for $ty {
            fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
                struct Object;
                impl<'de> Visitor<'de> for Object {
                    type Value = $ty;
                    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        f.write_str($what)
                    }
                    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<$ty, A::Error> {
                        let $m = $members::deserialize(MapAccessDeserializer::new(map))?;
                        Ok($build)
                    }
                }
                de.deserialize_map(Object)
            }
        }
    };
}

map_only!(IcsEvent, IcsMembers, "a calendar event object", |m| {
    IcsEvent {
        dtstart: m.dtstart,
        dtstart_tzid: m.dtstart_tzid,
        dtend: m.dtend,
        dtend_tzid: m.dtend_tzid,
        summary: m.summary,
        uid: m.uid,
    }
});

map_only!(Attachment, AttachmentMembers, "an attachment object", |m| {
    Attachment {
        filename: m.filename,
        content_type: m.content_type,
        size: m.size,
        sha256: m.sha256,
        stored: m.stored,
        ics: m.ics,
        content: None,
    }
});

map_only!(
    AttachmentList,
    ListMembers,
    "an attachment list object",
    |m| AttachmentList {
        total: m.total,
        parts: m.parts,
    }
);

/// Which selected body an occurrence came from.
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LinkSource {
    Text,
    Html,
}

/// Exactly the string ingest writes (decision 0097 §1). A derived enum
/// reader also admits a tagged object such as `{"html":null}`.
impl<'de> Deserialize<'de> for LinkSource {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        match String::deserialize(de)?.as_str() {
            "text" => Ok(Self::Text),
            "html" => Ok(Self::Html),
            other => Err(de::Error::unknown_variant(other, &["text", "html"])),
        }
    }
}

/// A captured email.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Email {
    /// UUIDv7 — sortable, unique.
    pub id: String,
    /// Inbox (mailbox) this email was captured into.
    pub inbox: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<EmailAddress>,
    /// The SMTP envelope sender (MAIL FROM's reverse path), captured at
    /// ingest (decision 0044 §8): `Some("")` is a known null reverse path
    /// (`MAIL FROM:<>`, a bounce), `None` is unknown — a row stored before
    /// the field existed. Never backfilled from the header `From`.
    pub envelope_from: Option<String>,
    pub to: Vec<EmailAddress>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cc: Vec<EmailAddress>,
    /// Envelope recipients (RCPT TO), which may include BCCs not present in headers.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recipients: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// RFC3339 receive timestamp.
    pub received_at: String,
    /// Unix millis receive timestamp (easy filtering).
    pub received_ms: i64,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<String>,
    /// The ingest-time occurrences behind `links` (decision 0094): `Some`
    /// for every fresh SMTP or seeded mail — `Some([])` is known empty —
    /// and `None` for a row restored from before the column existed, never
    /// backfilled. Internal: no Email serialization carries it, so every
    /// existing read, feed and webhook payload keeps its bytes; only the
    /// focused link wait exposes it.
    #[serde(skip)]
    pub link_metadata: Option<Vec<LinkMetadata>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub codes: Vec<String>,
    /// RFC 5322 Message-ID, normalized (angle brackets stripped) — the
    /// anchor a thread is built from. Extracted at ingest (decision 0005).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    /// RFC 5322 In-Reply-To: the first id the header parses to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<String>,
    /// RFC 5322 References, in header order (root → parent).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<String>,
    /// The ingest-time attachment list (decision 0110): `Some` for every
    /// parsed mail, `None` where unknown, never backfilled. Internal like
    /// `link_metadata`: no Email serialization carries it, so every
    /// existing read, feed and webhook payload keeps its bytes.
    #[serde(skip)]
    pub attachments: Option<AttachmentList>,
    #[serde(skip)]
    pub raw: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    fn part_json(extra: &str) -> String {
        format!(
            r#"{{"filename":"a.ics","content_type":"text/calendar","size":3,"sha256":"{SHA}","stored":true,"ics":null{extra}}}"#
        )
    }

    /// Decision 0110 §6: the persisted object restores only from the exact
    /// closed shape ingest writes — every member present (nullable ones as
    /// null), no unknown member, no duplicate, no positional array — and a
    /// calendar object is held to the same rule.
    #[test]
    fn the_persisted_attachment_object_is_closed_and_map_only() {
        let ics = r#"{"dtstart":"20261102T150000","dtstart_tzid":null,"dtend":null,"dtend_tzid":null,"summary":"s","uid":"u"}"#;
        let list: AttachmentList = serde_json::from_str(&format!(
            r#"{{"total":2,"parts":[{},{}]}}"#,
            part_json(""),
            part_json("").replace(r#""ics":null"#, &format!(r#""ics":{ics}"#))
        ))
        .unwrap();
        assert_eq!(list.total, 2);
        assert_eq!(list.parts[0].ics, None);
        assert_eq!(list.parts[0].content, None);
        let event = list.parts[1].ics.as_ref().unwrap();
        assert_eq!(event.summary.as_deref(), Some("s"));
        assert_eq!(event.dtend, None);
        // The object writes back as it came, without the bytes.
        assert_eq!(
            serde_json::to_string(&list.parts[0]).unwrap(),
            part_json("")
        );

        let part_with = |from: &str, to: &str| part_json("").replace(from, to);
        for (corrupt, cause) in [
            ("[]".to_string(), "an attachment list object"),
            ("null".to_string(), "an attachment list object"),
            (r#"{"total":1}"#.to_string(), "missing field `parts`"),
            (r#"{"parts":[]}"#.to_string(), "missing field `total`"),
            (
                r#"{"total":0,"parts":[],"extra":1}"#.to_string(),
                "unknown field `extra`",
            ),
            (
                r#"{"total":0,"total":1,"parts":[]}"#.to_string(),
                "duplicate field `total`",
            ),
            (r#"{"total":-1,"parts":[]}"#.to_string(), "invalid value"),
            (
                format!(r#"{{"total":1,"parts":[["a",null,1,"{SHA}",true,null]]}}"#),
                "an attachment object",
            ),
            (
                format!(
                    r#"{{"total":1,"parts":[{}]}}"#,
                    part_with(r#""filename":"a.ics","#, "")
                ),
                "missing field `filename`",
            ),
            (
                format!(
                    r#"{{"total":1,"parts":[{}]}}"#,
                    part_with(r#","ics":null"#, "")
                ),
                "missing field `ics`",
            ),
            (
                format!(
                    r#"{{"total":1,"parts":[{}]}}"#,
                    part_json(r#","content":"x""#)
                ),
                "unknown field `content`",
            ),
            (
                format!(
                    r#"{{"total":1,"parts":[{}]}}"#,
                    part_with(r#""stored":true"#, r#""stored":"yes""#)
                ),
                "invalid type",
            ),
            (
                format!(
                    r#"{{"total":1,"parts":[{}]}}"#,
                    part_with(r#""size":3"#, r#""size":"3""#)
                ),
                "invalid type",
            ),
            (
                format!(
                    r#"{{"total":1,"parts":[{}]}}"#,
                    part_with(r#""ics":null"#, r#""ics":["a",null,null,null,null,null]"#)
                ),
                "a calendar event object",
            ),
            (
                format!(
                    r#"{{"total":1,"parts":[{}]}}"#,
                    part_with(r#""ics":null"#, r#""ics":{"dtstart":null}"#)
                ),
                "missing field",
            ),
            (
                format!(
                    r#"{{"total":1,"parts":[{}]}}"#,
                    part_with(
                        r#""ics":null"#,
                        &format!(
                            r#""ics":{}"#,
                            ics.replace(r#""uid":"u""#, r#""uid":"u","tz":null"#)
                        )
                    )
                ),
                "unknown field `tz`",
            ),
        ] {
            let error = serde_json::from_str::<AttachmentList>(&corrupt).unwrap_err();
            assert!(error.to_string().contains(cause), "{corrupt}: {error}");
        }
    }

    /// The list and its bytes are internal: no Email serialization carries
    /// them, so every read, feed and webhook keeps its bytes (decision 0110
    /// §7).
    #[test]
    fn the_attachment_list_never_reaches_an_email_serialization() {
        let mut mail = crate::store::email("id", "box", "a@x.io");
        let without = serde_json::to_string(&mail).unwrap();
        mail.attachments = Some(AttachmentList {
            total: 1,
            parts: vec![Attachment {
                filename: Some("a.bin".into()),
                content_type: None,
                size: 1,
                sha256: SHA.into(),
                stored: true,
                ics: Some(IcsEvent::default()),
                content: Some(vec![1]),
            }],
        });
        assert_eq!(serde_json::to_string(&mail).unwrap(), without);
        assert!(!without.contains("attachments"));
    }

    /// The one name rule (decision 0046 §4): blank refuses, any Unicode
    /// control refuses, and everything else — padding, UTF-8 — comes back
    /// verbatim (issue #128).
    #[test]
    fn the_inbox_rule_refuses_blank_and_controls_and_keeps_the_rest() {
        for blank in ["", " ", "\t", "  \u{3000} "] {
            assert_eq!(validate_inbox(blank), Err(BLANK_INBOX_REFUSAL), "{blank:?}");
        }
        for control in [
            "\0user\0s3cretpw",
            "a\u{1}b",
            "a\tb",
            "pad\r",
            "del\u{7f}",
            "c1\u{85}",
            "c1\u{9f}x",
        ] {
            assert_eq!(
                validate_inbox(control),
                Err(CONTROL_INBOX_REFUSAL),
                "{control:?}"
            );
        }
        for kept in ["default", " pad", "pad ", " pad ", "j\u{f6}rg", "a b"] {
            assert_eq!(validate_inbox(kept), Ok(kept));
        }
    }
}
