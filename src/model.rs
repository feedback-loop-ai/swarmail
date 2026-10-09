//! Core data model.

use serde::{Deserialize, Serialize};

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
