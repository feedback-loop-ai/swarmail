//! Chaos injection: deliberate SMTP failure/delay simulation.

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChaosEvent {
    /// Drop the connection on arrival.
    Connect,
    /// Reject MAIL FROM.
    MailFrom,
    /// Reject RCPT TO.
    Rcpt,
    /// Applied after DATA completes (issue #17): a fired rule waits, then
    /// fails — an explicit `error` line, else `451 4.3.0 chaos`; the mail
    /// is never stored, delay-only or not.
    Data,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosRule {
    /// Percentage chance (0–100) the event fires.
    pub probability: u8,
    /// SMTP error line to reply with, e.g. "451 4.7.1 try again later".
    /// `None` is not a pass: a fired rule always fails its stage, with the
    /// stage's default when no line is set — `451 4.3.0 chaos` at
    /// `mail_from`/`rcpt`/`data` (and at `data` the mail is never stored),
    /// a close with no banner at `connect` (issue #17).
    #[serde(default)]
    pub error: Option<String>,
    /// Delay before responding/processing, milliseconds.
    #[serde(default)]
    pub delay_ms: u64,
}

/// The probability domain, ruled once (issue #32 slice 2, decision 0025):
/// an integer 0..=100. Both transports validate against it — REST's PUT for
/// every rule it would install, MCP's `set_chaos` for the rule it would
/// set — so the wires cannot drift again.
pub(crate) const PROBABILITY_MAX: u8 = 100;

/// The one refusal sentence for an out-of-domain probability, shared by
/// both transports' envelopes: REST's `400` body and MCP's JSON-RPC error
/// message carry the same words because they enforce the same rule.
pub(crate) const PROBABILITY_REFUSAL: &str = "probability must be an integer between 0 and 100";

impl ChaosRule {
    /// The rule's own domain check (decision 0025): the probability must be
    /// an integer 0..=`PROBABILITY_MAX`. Called by both transports before
    /// any effect, so a refused rule is never stored.
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.probability > PROBABILITY_MAX {
            return Err(PROBABILITY_REFUSAL.to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChaosConfig {
    #[serde(default)]
    pub connect: Option<ChaosRule>,
    #[serde(default)]
    pub mail_from: Option<ChaosRule>,
    #[serde(default)]
    pub rcpt: Option<ChaosRule>,
    #[serde(default)]
    pub data: Option<ChaosRule>,
}

impl ChaosConfig {
    /// A PUT replaces the whole config: every present rule must be in the
    /// domain, in all four events, before any caller sets it (decision
    /// 0025) — a partial check would accept a bad rule in a slot it forgot
    /// to look at.
    pub(crate) fn validate(&self) -> Result<(), String> {
        for rule in [
            self.connect.as_ref(),
            self.mail_from.as_ref(),
            self.rcpt.as_ref(),
            self.data.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            rule.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct Chaos {
    config: RwLock<ChaosConfig>,
}

impl Chaos {
    pub fn set(&self, config: ChaosConfig) {
        *self.config.write().unwrap() = config;
    }

    pub fn clear(&self) {
        *self.config.write().unwrap() = ChaosConfig::default();
    }

    pub fn current(&self) -> ChaosConfig {
        self.config.read().unwrap().clone()
    }

    /// Returns the rule to apply for `event`, if it fires.
    pub fn check(&self, event: ChaosEvent) -> Option<ChaosRule> {
        let rule = match event {
            ChaosEvent::Connect => self.config.read().unwrap().connect.clone(),
            ChaosEvent::MailFrom => self.config.read().unwrap().mail_from.clone(),
            ChaosEvent::Rcpt => self.config.read().unwrap().rcpt.clone(),
            ChaosEvent::Data => self.config.read().unwrap().data.clone(),
        }?;
        if rand::rng().random_range(0..100) < rule.probability as u32 {
            Some(rule)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_probability_never_fires() {
        let chaos = Chaos::default();
        chaos.set(ChaosConfig {
            rcpt: Some(ChaosRule {
                probability: 0,
                error: Some("451 x".into()),
                delay_ms: 0,
            }),
            ..Default::default()
        });
        for _ in 0..1000 {
            assert!(chaos.check(ChaosEvent::Rcpt).is_none());
        }
    }

    #[test]
    fn full_probability_always_fires() {
        let chaos = Chaos::default();
        chaos.set(ChaosConfig {
            data: Some(ChaosRule {
                probability: 100,
                error: Some("451 x".into()),
                delay_ms: 0,
            }),
            ..Default::default()
        });
        for _ in 0..100 {
            assert!(chaos.check(ChaosEvent::Data).is_some());
        }
    }

    /// The domain's poles and ceiling (issue #32 slice 2, decision 0025):
    /// 0 and 100 validate, everything above refuses with the one shared
    /// sentence both transports carry.
    #[test]
    fn probability_validates_at_the_poles_and_refuses_above_the_ceiling() {
        let rule = |p: u8| ChaosRule {
            probability: p,
            error: None,
            delay_ms: 0,
        };
        assert_eq!(rule(0).validate(), Ok(()));
        assert_eq!(rule(100).validate(), Ok(()));
        for p in [101u8, 150, 255] {
            assert_eq!(
                rule(p).validate(),
                Err(PROBABILITY_REFUSAL.to_string()),
                "{p}"
            );
        }
    }

    /// The config walk sees all four events (decision 0025): an in-domain
    /// all-four config validates, an all-absent one validates, and a bad
    /// rule planted in *each* slot refuses with the shared sentence —
    /// every `Option` arm of the walk taken on every run.
    #[test]
    fn config_validation_walks_every_event_slot() {
        let rule = |p: u8| ChaosRule {
            probability: p,
            error: None,
            delay_ms: 0,
        };
        let full = ChaosConfig {
            connect: Some(rule(10)),
            mail_from: Some(rule(20)),
            rcpt: Some(rule(30)),
            data: Some(rule(40)),
        };
        assert_eq!(full.validate(), Ok(()));
        assert_eq!(ChaosConfig::default().validate(), Ok(()));

        let bad_rule = || ChaosRule {
            probability: 150,
            error: None,
            delay_ms: 0,
        };
        for bad in [
            ChaosConfig {
                connect: Some(bad_rule()),
                ..Default::default()
            },
            ChaosConfig {
                mail_from: Some(bad_rule()),
                ..Default::default()
            },
            ChaosConfig {
                rcpt: Some(bad_rule()),
                ..Default::default()
            },
            ChaosConfig {
                data: Some(bad_rule()),
                ..Default::default()
            },
        ] {
            assert_eq!(bad.validate(), Err(PROBABILITY_REFUSAL.to_string()));
        }
    }
}
