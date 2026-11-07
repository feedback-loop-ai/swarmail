//! The targets of the agent-suite comparison (proposed decision 0129 §1, §4)
//! and the one line a launched adapter announces.
//!
//! The set is closed: Swarmail, and three competitors reached through a
//! thin adapter the runner owns. Which backend a trial uses is the variable
//! being measured; everything else about a trial — the fixture, the goal and
//! the scorer — is the same on every target, apart from the facts the
//! frozen product record states per target.

use std::net::SocketAddr;

/// One target of the comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Target {
    Swarmail,
    Mailpit,
    Mailhog,
    Mailslurper,
}

/// Every target, in the product record's order.
pub const ALL: [Target; 4] = [
    Target::Swarmail,
    Target::Mailpit,
    Target::Mailhog,
    Target::Mailslurper,
];

impl Target {
    /// The name a `--target` takes and a lock entry carries.
    pub fn name(self) -> &'static str {
        match self {
            Target::Swarmail => "swarmail",
            Target::Mailpit => "mailpit",
            Target::Mailhog => "mailhog",
            Target::Mailslurper => "mailslurper",
        }
    }

    /// The name a comparison row and the product record carry.
    pub fn comparison_name(self) -> &'static str {
        match self {
            Target::Swarmail => "swarmail-mcp",
            Target::Mailpit => "mailpit-mcp",
            Target::Mailhog => "mailhog-mcp",
            Target::Mailslurper => "mailslurper-mcp",
        }
    }

    /// How the target is named to an agent.
    pub fn display(self) -> &'static str {
        match self {
            Target::Swarmail => "Swarmail",
            Target::Mailpit => "Mailpit",
            Target::Mailhog => "MailHog",
            Target::Mailslurper => "MailSlurper",
        }
    }

    pub fn is_competitor(self) -> bool {
        self != Target::Swarmail
    }

    /// The `mail_compare` mode that serves this target's adapter, if any.
    pub fn adapter_mode(self) -> Option<&'static str> {
        match self {
            Target::Swarmail => None,
            Target::Mailpit => Some("mailpit-mcp"),
            Target::Mailhog => Some("mailhog-mcp"),
            Target::Mailslurper => Some("mailslurper-mcp"),
        }
    }

    /// Whether the runner can launch this competitor in this build.
    pub fn adapter_built(self) -> bool {
        self.is_competitor() && self.adapter_mode().is_some()
    }

    /// Parse a `--target` value, refusing by name.
    pub fn parse(value: &str) -> Result<Target, String> {
        ALL.into_iter().find(|t| t.name() == value).ok_or_else(|| {
            format!(
                "unknown --target {value:?}: expected one of {}",
                ALL.map(Target::name).join(", ")
            )
        })
    }

    /// The tool-name prefix an adapter's tools carry; a competitor's tools
    /// are named for the backend, so an agent is never offered a Swarmail
    /// name for another product.
    pub fn tool_prefix(self) -> &'static str {
        match self {
            Target::Swarmail => "swarmail_",
            Target::Mailpit => "mailpit_",
            Target::Mailhog => "mailhog_",
            Target::Mailslurper => "mailslurper_",
        }
    }
}

/// What an adapter announced as its first output line: the SMTP address of
/// the backend it owns and the digest of the tool list it serves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Announcement {
    pub smtp: SocketAddr,
    pub tools_sha256: String,
}

/// Parse `smtp=ADDR tools_sha256=HEX`: loopback only, nothing else on the
/// line, the digest a lowercase SHA-256.
pub fn parse_announcement(line: &str) -> Option<Announcement> {
    let mut parts = line.trim_end().split(' ');
    let smtp = parts
        .next()?
        .strip_prefix("smtp=")?
        .parse::<SocketAddr>()
        .ok()?;
    let digest = parts.next()?.strip_prefix("tools_sha256=")?;
    if parts.next().is_some()
        || !smtp.ip().is_loopback()
        || smtp.port() == 0
        || digest.len() != 64
        || !digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    Some(Announcement {
        smtp,
        tools_sha256: digest.to_owned(),
    })
}

/// Whether a tool name is one an agent wrapper may allow: the shape every
/// adapter's names have, so a name never needs quoting in a CLI argument.
pub fn valid_tool_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && name.len() <= 64
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Whether a server key is one a wrapper may use (it becomes part of an
/// allowlist entry, `mcp__<server>__<tool>`).
pub fn valid_server_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 32
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
