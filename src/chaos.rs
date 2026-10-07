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
    /// Fail after DATA (mail is NOT stored).
    Data,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosRule {
    /// Percentage chance (0–100) the event fires.
    pub probability: u8,
    /// SMTP error line to reply with, e.g. "451 4.7.1 try again later".
    #[serde(default)]
    pub error: Option<String>,
    /// Delay before responding/processing, milliseconds.
    #[serde(default)]
    pub delay_ms: u64,
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
}
