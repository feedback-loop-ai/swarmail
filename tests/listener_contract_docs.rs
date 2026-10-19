//! The listener contract as documented (decision 0036, issues #77–#80): the
//! README, the dated controlling clarification of 0027 and proposed 0036
//! must state what the listeners actually do. #80 deliberately leaves the
//! runtime unchanged, so its red/green proof is this documentary one: the
//! reviewed text — "a trickler cannot hold a session forever", the
//! unconditional `stopping` answer, the success-only recovery — fails here,
//! and the corrected text passes. Historical 0027 prose above its
//! clarification is not a failure by itself; only the controlling text is
//! read. Whitespace is normalized so line wrapping never decides a verdict.
//!
//! The documented numbers — the session cap, the DATA bound, their
//! aggregate and both idle bounds — are built from the defaults the
//! listeners actually run with (`SmtpConfig::default()` and
//! `pop3::DEFAULT_IDLE_TIMEOUT`), so moving a constant turns these red
//! (issue #143). Statuses are read from the records, never pinned: the
//! index/record agreement lives in `store_contract_docs.rs`.

use std::path::Path;
use std::time::Duration;
use swarmail::smtp::SmtpConfig;

/// A repository document, whitespace-normalized to single spaces.
fn doc(path: &str) -> String {
    let full = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    let text = std::fs::read_to_string(&full)
        .unwrap_or_else(|e| panic!("{} is unreadable: {e}", full.display()));
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Every phrase is present in `text`, each failure naming the document.
fn states<S: AsRef<str>>(text: &str, what: &str, phrases: &[S]) {
    for phrase in phrases {
        let phrase = phrase.as_ref();
        assert!(text.contains(phrase), "{what} must state: {phrase:?}");
    }
}

/// A record's status word from its `- **Status:**` header (the spellings
/// `store_contract_docs.rs` checks against the index).
fn status_of(record: &str) -> &'static str {
    let header = doc(&format!("docs/decisions/{record}"));
    let status = header
        .split("- **Status:** ")
        .nth(1)
        .unwrap_or_else(|| panic!("{record} carries no Status header"));
    if status.starts_with("accepted") {
        "accepted"
    } else if status.starts_with("proposed") {
        "proposed"
    } else {
        panic!("{record}: unsupported status {status:?}")
    }
}

/// A duration in whole minutes, or in seconds when it is not one — so a
/// bound moved off the documented minute never rounds back onto it.
fn minutes(d: Duration) -> String {
    if d.as_secs().is_multiple_of(60) && d.subsec_nanos() == 0 {
        format!("{} min", d.as_secs() / 60)
    } else {
        format!("{d:?}")
    }
}

/// The residual and drain statements both current documents must carry,
/// their numbers taken from the running defaults: the SMTP session cap,
/// the DATA bound in MiB, and their product in GiB.
fn residual() -> Vec<String> {
    let smtp = SmtpConfig::default();
    let mib = smtp.max_message_size as f64 / f64::from(1 << 20);
    let gib = smtp.max_sessions as f64 * smtp.max_message_size as f64 / f64::from(1 << 30);
    vec![
        "no session lifetime, no DATA lifetime and no minimum transfer rate".into(),
        "inside each window".into(),
        format!("all {} SMTP slots", smtp.max_sessions),
        format!("approaching {mib} MiB"),
        format!(
            "about {gib} GiB across {} connections before allocation",
            smtp.max_sessions
        ),
        "not an RSS ceiling".into(),
        "connection refusal".into(),
        "only if its request reaches the handler".into(),
    ]
}

#[test]
fn the_readme_states_the_current_listener_posture() {
    let readme = doc("README.md");
    states(&readme, "README.md", &residual());
    states(
        &readme,
        "README.md",
        &[
            format!(
                "SMTP {}, POP3 {}",
                minutes(SmtpConfig::default().idle_timeout),
                minutes(swarmail::pop3::DEFAULT_IDLE_TIMEOUT)
            ),
            "per read and per write, not a session lifetime".into(),
            "observed waiting or succeeds".into(),
            "reaped right after each accept, before the cap is checked".into(),
            format!(
                "decision 0036, {}",
                status_of("0036-smtp-trickle-lifetime.md")
            ),
        ],
    );
    for stale in [
        "has no idle timeout",
        "clears on the next success",
        "5 s drain deadline",
    ] {
        assert!(!readme.contains(stale), "README.md still claims {stale:?}");
    }
}

#[test]
fn the_0027_clarification_controls_and_links_proposed_0036() {
    let record = doc("docs/decisions/0027-living-listeners-bounded-sessions.md");
    let heading = "## Clarification — 2026-10-18 (controlling)";
    let (history, clarification) = record
        .split_once(heading)
        .expect("0027 must carry the dated controlling clarification");
    // The accepted history stays: status and the original ruling survive.
    assert!(history.contains("- **Status:** accepted"), "{history}");
    assert!(history.contains("## Decision"), "the ruling is kept");
    states(clarification, "the 0027 clarification", &residual());
    states(
        clarification,
        "the 0027 clarification",
        &[
            // Dated, controlling text: it recorded 0036 as proposed on its
            // date, and acceptance adds a pointer rather than rewriting it.
            "[0036](0036-smtp-trickle-lifetime.md), which is **proposed**".to_string(),
            "*Superseded:* \"The flag clears on the next successful accept.\"".into(),
            "*Superseded:* the unconditional".into(),
            "*Superseded:* the cap \"checked after the top-of-loop reap\"".into(),
            "*Superseded:* §9's \"`config.rs` ends at zero diff\"".into(),
            "*Superseded:* \"it bounds silence *and* the pathological trickle\"".into(),
            "\"Plaintext sessions can no longer be held forever by silence, by a zero window, or by a trickler\"".into(),
            format!(
                "SMTP keeps {} s; POP3 uses its own {} s",
                SmtpConfig::default().idle_timeout.as_secs(),
                swarmail::pop3::DEFAULT_IDLE_TIMEOUT.as_secs()
            ),
        ],
    );
    assert!(
        !clarification.contains("accepted (operator"),
        "the clarification must not claim 0036's acceptance"
    );
}

/// Batch 1 of epic #120 as documented (decision 0046, issues #121, #126,
/// #127, #128, #131): the README states the per-successful-write POP3 bound
/// with its retention residual, the failed-session drop, `TCP_NODELAY`, the
/// two named path limitations and the one verbatim inbox rule; the dated
/// pointers on 0027 and 0036 point at 0046 as it stood when they were
/// written (proposed). Red before: the README promised no
/// whole-reply semantics either way and named neither path limitation.
#[test]
fn the_readme_and_0046_state_the_batch_one_listener_posture() {
    let readme = doc("README.md");
    states(
        &readme,
        "README.md",
        &[
            "A POP3 reply has no whole-reply deadline either",
            "every write that moves bytes starts a fresh window",
            "keeps its slot and that reply's buffer for as long as it takes",
            "drops its connection at once",
            "gets one more idle window for its close",
            "sets `TCP_NODELAY` before its first byte",
            "a quoted local part containing `>`",
            "a source route (`<@hop1,@hop2:bob@y.io>`) is not stripped",
            "inbox must not contain control characters",
            "padding included, so `\" pad \"` names the same inbox everywhere",
        ],
    );
    assert!(
        !readme.contains("no one reply write may wait longer"),
        "README.md still claims a whole-write POP3 bound"
    );
    // 0046's index row is checked against its record, whatever the status,
    // by `every_index_row_matches_its_records_status`; the dated pointers
    // below are history and keep the status they were written under.
    for record in [
        "docs/decisions/0027-living-listeners-bounded-sessions.md",
        "docs/decisions/0036-smtp-trickle-lifetime.md",
    ] {
        let text = doc(record);
        let (_, pointer) = text
            .split_once("## Pointer — 2026-10-18 (proposed 0046)")
            .unwrap_or_else(|| panic!("{record} must carry the dated 0046 pointer"));
        assert!(
            pointer.contains("[0046](0046-smtp-pop3-edges.md)"),
            "{record}"
        );
        assert!(pointer.contains("**proposed**"), "{record}");
        assert!(pointer.contains("moves bytes"), "{record}: {pointer}");
    }
}

/// 0036 states its ruling and the trickle residual's aggregate, computed
/// from the running SMTP defaults; its status is whatever its own header
/// says (index agreement is `store_contract_docs.rs`'s check), so the
/// operator accepting it leaves this green.
#[test]
fn decision_0036_states_the_ruling_and_the_aggregate() {
    let record = doc("docs/decisions/0036-smtp-trickle-lifetime.md");
    let smtp = SmtpConfig::default();
    let gib = smtp.max_sessions as f64 * smtp.max_message_size as f64 / f64::from(1 << 30);
    status_of("0036-smtp-trickle-lifetime.md");
    states(
        &record,
        "0036",
        &[
            "Add no session lifetime, DATA lifetime or minimum DATA rate.".to_string(),
            format!("about {gib} GiB aggregate"),
        ],
    );
}
