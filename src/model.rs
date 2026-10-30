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
fn required_nullable<'de, D: Deserializer<'de>>(de: D) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(de)
}

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

/// One attachment part as ingest listed it (decision 0092 §5): the outer
/// parser list in parser order, with the labels the sender gave and the
/// decoded part length. `None` is a label the mail did not carry.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Attachment {
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub size: u64,
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
    /// The ingest-time attachment list (decision 0092 §5): `Some([])` is a
    /// known empty list, `None` a row from before the column or a mail the
    /// parser could not read — unknown, never backfilled. Internal like
    /// `link_metadata`: no Email serialization carries it, so every existing
    /// read, feed and webhook payload keeps its bytes.
    #[serde(skip)]
    pub attachments: Option<Vec<Attachment>>,
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
    #[serde(skip)]
    pub raw: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
