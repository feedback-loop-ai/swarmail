//! The Swarmail agent eval harness (epic #164 Phase 0; decision 0055).
//!
//! One unpublished crate, one executable: the fixture app (#149), the v2
//! task catalogue with oracle solutions (#150), and the agent-agnostic
//! trial runner that #148's brokkr recipe will wrap. Outside the shipped
//! crate and its coverage denominator by the operator's 2026-10-18 ruling.

pub mod fixture;
pub mod mail;
pub mod mcp;
pub mod oracle;
pub mod report;
pub mod runner;
pub mod state;
pub mod tasks;
