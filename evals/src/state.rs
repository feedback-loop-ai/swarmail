//! The fixture's scoring view: a versioned, secret-free snapshot of what
//! the app observed. The scorer reads this and nothing else — never a
//! mailbox, a transcript or the agent's own claim (decision 0055 §3).
//!
//! These are the *public* shapes. The fixture's private state (tokens,
//! codes, passwords, ICS bytes) lives in `fixture.rs` and is converted into
//! these types field by field, so adding a secret to the private state can
//! never leak it into a snapshot by accident.

use serde::{Deserialize, Serialize};

/// The snapshot schema this build writes and the scorer accepts.
pub const SNAPSHOT_SCHEMA: u32 = 2;

/// The closed failure taxonomy (decision 0055 §2). An unknown spelling
/// fails to deserialize, so a misspelled class in a catalogue or a result
/// row goes red at the boundary instead of becoming a new category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureClass {
    WrongMessage,
    StaleCode,
    WrongLink,
    WaitTimeout,
    GaveUp,
    ToolMisuse,
    Budget,
    HarnessError,
}

impl FailureClass {
    pub const ALL: [FailureClass; 8] = [
        FailureClass::WrongMessage,
        FailureClass::StaleCode,
        FailureClass::WrongLink,
        FailureClass::WaitTimeout,
        FailureClass::GaveUp,
        FailureClass::ToolMisuse,
        FailureClass::Budget,
        FailureClass::HarnessError,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            FailureClass::WrongMessage => "wrong_message",
            FailureClass::StaleCode => "stale_code",
            FailureClass::WrongLink => "wrong_link",
            FailureClass::WaitTimeout => "wait_timeout",
            FailureClass::GaveUp => "gave_up",
            FailureClass::ToolMisuse => "tool_misuse",
            FailureClass::Budget => "budget",
            FailureClass::HarnessError => "harness_error",
        }
    }
}

/// The eight fixture flows (#149).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Flow {
    Signup,
    Otp,
    Reset,
    MagicLogin,
    Invite,
    Support,
    Digest,
    Marketing,
}

impl Flow {
    pub const ALL: [Flow; 8] = [
        Flow::Signup,
        Flow::Otp,
        Flow::Reset,
        Flow::MagicLogin,
        Flow::Invite,
        Flow::Support,
        Flow::Digest,
        Flow::Marketing,
    ];

    pub fn parse(s: &str) -> Option<Flow> {
        serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
    }
}

/// Whether an identity is driven by an agent or exists only to be a
/// look-alike decoy (the sibling-address trap).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Agent,
    Decoy,
}

/// Outcome facts per identity. Each one is set only by a real transition
/// on the public plane, and each has a backing accepted event — the scorer
/// refuses a snapshot whose fact has none (a fabricated success flag).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Facts {
    pub confirmed: bool,
    pub otp_verified: bool,
    pub logged_in: bool,
    pub magic_login: bool,
    pub reset_count: u32,
    pub post_reset_login: bool,
    pub reset_replay_rejected: bool,
    pub invite_accepted: bool,
    pub invite_time_matched: bool,
    pub reply_received: bool,
    pub digest_selected: bool,
    pub marketing_completed: bool,
    pub slow_released: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityState {
    pub identity: String,
    pub inbox: String,
    pub role: Role,
    pub facts: Facts,
}

/// Where a delivery stands. Only a final DATA `250` is `delivered`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    /// Accepted by Swarmail: the final DATA reply was 250.
    Delivered,
    /// Refused with a transient 4xx; the fixture holds it for one retry.
    Parked,
    /// Deliberately held until the agent releases it (slow-delivery).
    Held,
    /// A permanent refusal, an exhausted retry or an ambiguous transport
    /// failure. Never retried; the runner treats it as a harness error.
    Failed,
}

/// One SMTP attempt: the stage that answered last and its reply code
/// (0 when the transport failed before any reply).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attempt {
    pub stage: String,
    pub code: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Delivery {
    pub seq: u64,
    pub identity: String,
    pub flow: Flow,
    /// The message's kind (e.g. `signup`, `otp-2`, `sibling-decoy`).
    pub message: String,
    pub status: DeliveryStatus,
    pub attempts: Vec<Attempt>,
}

/// One public-plane action, accepted or refused. `claimed` is the
/// `X-Eval-Identity` the request named (absent when it named none);
/// `category` is the failure class a refusal is evidence of.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub seq: u64,
    pub claimed: Option<String>,
    pub flow: Option<Flow>,
    pub action: String,
    pub accepted: bool,
    pub category: Option<FailureClass>,
}

/// The frozen, versioned scoring view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub schema_version: u32,
    pub trial_id: String,
    pub task: String,
    pub frozen: bool,
    pub identities: Vec<IdentityState>,
    pub deliveries: Vec<Delivery>,
    pub events: Vec<Event>,
}

/// Action names shared by the fixture (which records them) and the scorer
/// (which reads them). One owner, so the two cannot drift.
pub mod action {
    pub const CONFIRM: &str = "confirm";
    pub const OTP_VERIFY: &str = "otp_verify";
    pub const MAGIC_LOGIN: &str = "magic_login";
    pub const RESET: &str = "reset";
    pub const RESET_REPLAY: &str = "reset_replay";
    pub const LOGIN: &str = "login";
    pub const INVITE_ACCEPT: &str = "invite_accept";
    pub const SUPPORT_REPLY: &str = "support_reply";
    pub const DIGEST_SELECT: &str = "digest_select";
    pub const MARKETING: &str = "marketing";
    pub const RELEASE: &str = "release";
    pub const DECOY_LINK: &str = "decoy_link";
    pub const DECOY_MESSAGE: &str = "decoy_message";
    pub const SUPERSEDED_CODE: &str = "superseded_code";
    pub const MARKETING_MANGLED: &str = "marketing_mangled";
    pub const DIGEST_DECOY: &str = "digest_decoy";
    pub const FOREIGN_TOKEN: &str = "foreign_token";
}
