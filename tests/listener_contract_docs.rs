//! The listener contract as documented (decision 0036, issues #77–#80): the
//! README, the dated controlling clarification of 0027 and proposed 0036
//! must state what the listeners actually do. #80 deliberately leaves the
//! runtime unchanged, so its red/green proof is this documentary one: the
//! reviewed text — "a trickler cannot hold a session forever", the
//! unconditional `stopping` answer, the success-only recovery — fails here,
//! and the corrected text passes. Historical 0027 prose above its
//! clarification is not a failure by itself; only the controlling text is
//! read. Whitespace is normalized so line wrapping never decides a verdict.

use std::path::Path;

/// A repository document, whitespace-normalized to single spaces.
fn doc(path: &str) -> String {
    let full = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    let text = std::fs::read_to_string(&full)
        .unwrap_or_else(|e| panic!("{} is unreadable: {e}", full.display()));
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Every phrase is present in `text`, each failure naming the document.
fn states(text: &str, what: &str, phrases: &[&str]) {
    for phrase in phrases {
        assert!(text.contains(phrase), "{what} must state: {phrase:?}");
    }
}

/// The residual and drain statements both current documents must carry.
const RESIDUAL: &[&str] = &[
    "no session lifetime, no DATA lifetime and no minimum transfer rate",
    "inside each window",
    "all 256 SMTP slots",
    "approaching 50 MiB",
    "about 12.5 GiB across 256 connections before allocation",
    "not an RSS ceiling",
    "connection refusal",
    "only if its request reaches the handler",
];

#[test]
fn the_readme_states_the_current_listener_posture() {
    let readme = doc("README.md");
    states(&readme, "README.md", RESIDUAL);
    states(
        &readme,
        "README.md",
        &[
            "SMTP 5 min, POP3 10 min",
            "per read and per write, not a session lifetime",
            "observed waiting or succeeds",
            "reaped right after each accept, before the cap is checked",
            "decision 0036, proposed",
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
    states(clarification, "the 0027 clarification", RESIDUAL);
    states(
        clarification,
        "the 0027 clarification",
        &[
            "[0036](0036-smtp-trickle-lifetime.md), which is **proposed**",
            "*Superseded:* \"The flag clears on the next successful accept.\"",
            "*Superseded:* the unconditional",
            "*Superseded:* the cap \"checked after the top-of-loop reap\"",
            "*Superseded:* §9's \"`config.rs` ends at zero diff\"",
            "*Superseded:* \"it bounds silence *and* the pathological trickle\"",
            "\"Plaintext sessions can no longer be held forever by silence, by a zero window, or by a trickler\"",
            "SMTP keeps 300 s; POP3 uses its own 600 s",
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
/// two named path limitations and the one verbatim inbox rule; 0046 is
/// indexed as proposed and the dated pointers on 0027 and 0036 point at it
/// without claiming its acceptance. Red before: the README promised no
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
    let index = doc("docs/decisions/README.md");
    let row = index
        .split("| [0046](0046-smtp-pop3-edges.md) |")
        .nth(1)
        .expect("the decision index must list 0046");
    let row = row.split(" | [").next().unwrap_or(row);
    assert!(row.contains("proposed — awaiting the operator"), "{row}");
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

#[test]
fn decision_0036_is_proposed_and_indexed() {
    let record = doc("docs/decisions/0036-smtp-trickle-lifetime.md");
    states(
        &record,
        "0036",
        &[
            "- **Status:** proposed — acceptance is the operator's.",
            "Add no session lifetime, DATA lifetime or minimum DATA rate.",
            "about 12.5 GiB aggregate",
        ],
    );
    let index = doc("docs/decisions/README.md");
    let row = index
        .split("| [0036](0036-smtp-trickle-lifetime.md) |")
        .nth(1)
        .expect("the decision index must list 0036");
    let row = row.split(" | [").next().unwrap_or(row);
    assert!(row.contains("proposed — awaiting the operator"), "{row}");
}
