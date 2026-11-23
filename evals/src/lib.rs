//! The Swarmail agent eval harness (epic #164 Phase 0; decision 0055).
//!
//! One unpublished crate, one executable: the fixture app (#149), the v2
//! task catalogue with oracle solutions (#150), and the agent-agnostic
//! trial runner that #148's brokkr recipe will wrap. Outside the shipped
//! crate and its coverage denominator by the operator's 2026-10-18 ruling.

pub mod admission;
// Protocol 4's API section (accepted decision 0132 §9.2): the wire driver's
// one client per tool and its scheduler and exact checks, compiled here by
// path so each tool has exactly one client. They use only packages this
// crate already names as direct dependencies.
#[path = "../../benches/mail_compare/api.rs"]
pub mod api;
pub mod apibench;
pub mod bridge;
pub mod capture;
#[path = "../../benches/mail_compare/clients/mod.rs"]
pub mod clients;
pub mod comparison;
pub mod driver;
pub mod fixture;
pub mod isolation;
pub mod mail;
pub mod mailsource;
pub mod mcp;
pub mod native_source;
// Story C's wait-best observer and its stream transport, and story D's
// suite plan and record: the driver's modules, compiled here by path so the
// reference suite waits and accounts exactly as the driver checks.
#[path = "../../benches/mail_compare/clients/observe.rs"]
pub mod observe;
pub mod oracle;
pub mod report;
pub mod runner;
pub mod state;
#[path = "../../benches/mail_compare/clients/stream.rs"]
pub mod stream;
#[path = "../../benches/mail_compare/suite.rs"]
pub mod suite;
pub mod target;
pub mod tasks;
pub mod testexec;

pub mod artifacts;
pub mod identity;
pub mod interpreted;
pub mod provenance;
pub mod results;

// Only the Linux-only runner probes (`runner::test_probes`) take the lock.
#[cfg(all(test, target_os = "linux"))]
#[path = "../tests/common/coordination.rs"]
pub(crate) mod test_support;

pub mod baseline;
pub mod fixture_http;
pub mod journal;
pub mod toolvalues;
pub mod wrappers;
