//! Core data model.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmailAddress {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub address: String,
}

/// A captured email.
#[derive(Debug, Clone, Serialize)]
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
    #[serde(skip)]
    pub raw: Vec<u8>,
}
