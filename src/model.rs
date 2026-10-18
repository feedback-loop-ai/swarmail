//! Core data model.

use serde::{Deserialize, Deserializer, Serialize};

/// The blank-inbox refusal (decision 0044 §3): one sentence, every plane.
pub const BLANK_INBOX_REFUSAL: &str = "inbox must not be blank";

/// The one inbox-name admission rule (decision 0044 §3): a client-supplied
/// inbox that is empty after Unicode whitespace trimming names nothing and
/// is refused — an empty name was stored and then linked to a 404 (issue
/// #91). A nonblank name is kept verbatim, padding included; nothing is
/// renamed and no stored row is rewritten. Every admitting plane calls this
/// (REST, MCP, UI, compat, the shared seed builder); SMTP AUTH and POP3
/// `USER` already refuse an empty-after-trim name (decision 0028 §2).
pub fn validate_inbox(name: &str) -> Result<&str, &'static str> {
    if name.trim().is_empty() {
        Err(BLANK_INBOX_REFUSAL)
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
