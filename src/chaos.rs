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

/// The reply-line text cap. RFC 5321 §4.5.3.1.2 allows a 512-octet reply
/// line *including* the CRLF `write_line` appends, so the supplied text is
/// at most 510: the accounting is stated here, not hidden in the parser.
pub(crate) const MAX_REPLY_LINE_OCTETS: usize = 510;

/// The delay ceiling: RFC 5321 §4.5.3.2's stage-timer scale (five minutes).
/// Deliberately *not* the 60 s control-plane await ceiling — that governs a
/// different plane; a chaos stall is a wire condition, not a wait.
pub(crate) const MAX_DELAY_MS: u64 = 300_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChaosRule {
    /// Percentage chance (0–100) the event fires.
    pub probability: u8,
    /// SMTP error line to reply with, e.g. "451 4.7.1 try again later".
    /// `None` is not a pass: a fired rule always fails its stage, with the
    /// stage's default when no line is set — `451 4.3.0 chaos` at
    /// `mail_from`/`rcpt`/`data` (and at `data` the mail is never stored),
    /// a close with no banner at `connect` (issue #17). Omission is `None`;
    /// a present `null` refuses (decision 0025 §2, 0044 §3) — only the
    /// per-event rule itself may be `null`, which clears that event. An
    /// unset line is omitted on the way out too, so a GET'd config PUTs
    /// back unchanged.
    #[serde(
        default,
        deserialize_with = "crate::model::present_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub error: Option<String>,
    /// Delay before responding/processing, milliseconds, at most
    /// [`MAX_DELAY_MS`] — a rule is a stage condition, not an unbounded
    /// parking spot for a session task.
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

/// The one refusal sentence for an out-of-domain reply line, shared by both
/// transports (decision 0029 §3): REST's `400` body and MCP's JSON-RPC
/// error message carry the same words — the field's name and its domain.
pub(crate) const CHAOS_ERROR_REFUSAL: &str =
    "error must be a single-line 4xx/5xx SMTP reply of at most 510 octets";

/// The one refusal sentence for an out-of-domain delay, shared by both
/// transports (decision 0029 §3).
pub(crate) const DELAY_MS_REFUSAL: &str = "delay_ms must be an integer between 0 and 300000";

/// The reply-line grammar's own function: every refusal names one shared
/// sentence, wherever the line came from.
fn validate_error_line(error: &str) -> Result<(), String> {
    if error.len() > MAX_REPLY_LINE_OCTETS {
        return Err(CHAOS_ERROR_REFUSAL.to_string());
    }
    // A single line: CR or LF anywhere would desynchronize the session
    // stream the rule is written into.
    if error.bytes().any(|b| b == b'\r' || b == b'\n') {
        return Err(CHAOS_ERROR_REFUSAL.to_string());
    }
    let bytes = error.as_bytes();
    if bytes.len() < 3 {
        return Err(CHAOS_ERROR_REFUSAL.to_string());
    }
    if !(b'4'..=b'5').contains(&bytes[0]) {
        return Err(CHAOS_ERROR_REFUSAL.to_string());
    }
    if !bytes[1].is_ascii_digit() || !bytes[2].is_ascii_digit() {
        return Err(CHAOS_ERROR_REFUSAL.to_string());
    }
    // Three octets of code, then a space — `4514` is not a code.
    if bytes.len() > 3 && bytes[3] != b' ' {
        return Err(CHAOS_ERROR_REFUSAL.to_string());
    }
    Ok(())
}

impl ChaosRule {
    /// The rule's own domain check (decision 0025, extended by 0029 §3):
    /// the probability must be an integer 0..=`PROBABILITY_MAX`; a supplied
    /// `error` must be a reply line; a supplied `delay_ms` must fit the
    /// stage scale. Called by both transports before any effect, so a
    /// refused rule is never stored.
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.probability > PROBABILITY_MAX {
            return Err(PROBABILITY_REFUSAL.to_string());
        }
        if let Some(error) = &self.error {
            validate_error_line(error)?;
        }
        if self.delay_ms > MAX_DELAY_MS {
            return Err(DELAY_MS_REFUSAL.to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

    /// Set one event's rule inside a single write-lock scope (decision 0029
    /// §3): MCP's `set_chaos` arms a single event without the
    /// read-modify-write that let two concurrent arms of *different* events
    /// erase each other — the second arm's write landed on a snapshot taken
    /// before the first arm's landed.
    pub fn set_event(&self, event: ChaosEvent, rule: ChaosRule) {
        let mut config = self.config.write().unwrap();
        match event {
            ChaosEvent::Connect => config.connect = Some(rule),
            ChaosEvent::MailFrom => config.mail_from = Some(rule),
            ChaosEvent::Rcpt => config.rcpt = Some(rule),
            ChaosEvent::Data => config.data = Some(rule),
        }
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

    /// The reply-line corpus (decision 0029 §3): every refusal arm of the
    /// grammar is taken on every run, with the legal poles beside them —
    /// 510 octets is the line, 511 is refused; `451` alone is a reply;
    /// `4514` is not a code.
    #[test]
    fn the_error_line_grammar_refuses_by_name() {
        let rule = |error: &str| ChaosRule {
            probability: 50,
            error: Some(error.to_string()),
            delay_ms: 0,
        };
        for legal in ["451", "450 4.7.1 no", "551 5.1.1 nope", "500 ok"] {
            assert_eq!(rule(legal).validate(), Ok(()), "{legal}");
        }
        // Exactly at the cap: legal. One octet over: refused.
        let at_cap = format!("500 {}", "x".repeat(MAX_REPLY_LINE_OCTETS - 4));
        assert_eq!(at_cap.len(), MAX_REPLY_LINE_OCTETS, "{at_cap}");
        assert_eq!(rule(&at_cap).validate(), Ok(()));
        let over = format!("500 {}", "x".repeat(MAX_REPLY_LINE_OCTETS - 3));
        assert_eq!(over.len(), MAX_REPLY_LINE_OCTETS + 1);
        assert_eq!(rule(&over).validate(), Err(CHAOS_ERROR_REFUSAL.to_string()));
        // A single line: CR, LF and CRLF all refuse, wherever they sit.
        for broken in ["451\r\nx", "451\nx", "451\rx", "\r451", "\n451"] {
            assert_eq!(
                rule(broken).validate(),
                Err(CHAOS_ERROR_REFUSAL.to_string()),
                "{broken}"
            );
        }
        // A 4xx/5xx code: three octets, first in 4..=5, then digits, and a
        // fourth octet only as a space.
        for broken in [
            "",
            "45",
            "not-a-code",
            "250 2.0.0 OK",
            "999 x",
            "4x1 x",
            "45x x",
            "4514",
            "4514 no-space",
        ] {
            assert_eq!(
                rule(broken).validate(),
                Err(CHAOS_ERROR_REFUSAL.to_string()),
                "{broken}"
            );
        }
    }

    /// The delay ceiling (decision 0029 §3): the stage scale's edge is
    /// legal, one millisecond past it refuses, and `u64::MAX` — the
    /// unbounded park this decision deletes — refuses with the same words.
    #[test]
    fn the_delay_ceiling_refuses_over_the_stage_scale() {
        let rule = |delay_ms: u64| ChaosRule {
            probability: 50,
            error: None,
            delay_ms,
        };
        assert_eq!(rule(0).validate(), Ok(()));
        assert_eq!(rule(MAX_DELAY_MS).validate(), Ok(()));
        assert_eq!(
            rule(MAX_DELAY_MS + 1).validate(),
            Err(DELAY_MS_REFUSAL.to_string())
        );
        assert_eq!(rule(u64::MAX).validate(), Err(DELAY_MS_REFUSAL.to_string()));
    }

    /// The chaos shape is closed (decision 0029 §3): an unknown key
    /// refuses at the REST door — the extractor's 422 — and an unknown
    /// rule key likewise. The sentence is serde's; the rule is the same.
    #[test]
    fn the_chaos_shape_is_closed_to_unknown_keys() {
        let err =
            serde_json::from_str::<ChaosConfig>(r#"{"data": {"probability": 10}, "solo": 1}"#)
                .expect_err("an unknown event key refuses");
        assert!(err.to_string().contains("unknown field"), "{err}");
        let err = serde_json::from_str::<ChaosRule>(r#"{"probability": 10, "solo": 1}"#)
            .expect_err("an unknown rule key refuses");
        assert!(err.to_string().contains("unknown field"), "{err}");
    }

    /// Two concurrent arms of *different* events both survive (decision
    /// 0029 §3): the whole match happens inside one write-lock scope, so
    /// neither arm's write can erase the other's. Each round holds the
    /// write lock across the arms' spawn and rendezvous, then releases it
    /// and joins. The channel is a zero-capacity rendezvous: an arm's
    /// `send` cannot complete until the main thread's `recv` does, so the
    /// main paces the arms one handshake at a time — by the release, every
    /// arm has run past its send and parked inside `set_event`'s first
    /// lock acquisition. A buffered channel cannot do this: its sends
    /// complete before the arm is even scheduled, and the release then
    /// fires before any arm has reached the lock, leaving the arms to
    /// serialize naturally — a reintroduced read-clone + separate write
    /// (a lost update) went undetected for 28 of 50 runs that way. The
    /// single-round shape still leaves a last-handshake race (an arm
    /// between its `send` and its lock attempt escapes the parked epoch),
    /// so the round runs sixteen times on a fresh `Chaos` each: the
    /// mutant must win every round, and one overlapped pair of read-clones
    /// — the last writer's stale snapshot — fails the assert (166/200
    /// single-round reds became 200/200; issue #102). The probabilities
    /// sit at the poles, where the fire / no-fire branch (decision 0025)
    /// is stable by construction.
    #[test]
    fn two_concurrent_set_event_arms_both_survive() {
        let rule = |p: u8| ChaosRule {
            probability: p,
            error: None,
            delay_ms: 0,
        };
        for _ in 0..16 {
            let chaos = std::sync::Arc::new(Chaos::default());
            // Held across spawn, rendezvous and join — a rendezvous
            // channel, no sleeps, no polling: the arms cannot complete a
            // single lock scope until the release, so the mutant's
            // separate write always lands on a stale snapshot.
            let held = chaos.config.write().unwrap();
            let (tx, rx) = std::sync::mpsc::sync_channel(0);
            let handles: Vec<_> = [
                (ChaosEvent::Connect, 100u8),
                (ChaosEvent::MailFrom, 0),
                (ChaosEvent::Rcpt, 100),
                (ChaosEvent::Data, 0),
            ]
            .into_iter()
            .map(|(event, p)| {
                let chaos = std::sync::Arc::clone(&chaos);
                let tx = tx.clone();
                std::thread::spawn(move || {
                    tx.send(()).expect("the test still listens");
                    chaos.set_event(event, rule(p))
                })
            })
            .collect();
            drop(tx);
            for _ in &handles {
                rx.recv().expect("every arm announces itself");
            }
            drop(held);
            for handle in handles {
                handle.join().expect("the arm does not panic");
            }
            let config = chaos.current();
            assert_eq!(config.connect, Some(rule(100)), "{config:?}");
            assert_eq!(config.mail_from, Some(rule(0)), "{config:?}");
            assert_eq!(config.rcpt, Some(rule(100)), "{config:?}");
            assert_eq!(config.data, Some(rule(0)), "{config:?}");
        }
    }
}
