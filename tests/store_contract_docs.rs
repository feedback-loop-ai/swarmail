//! The store and maildrop contracts as documented (proposed decisions 0037
//! and 0038, issues #84 and #87). #87 takes the documentation branch of
//! each either/or — the watcher residue, the absent global cap and the
//! parse-start timestamps are stated, not changed — so its red/green proof
//! is this documentary one: the reviewed text ("bounded memory", the
//! per-inbox knob "that bounds it", the dot-stuffed POP3 count) fails here,
//! and the corrected text passes. Accepted decisions are historical and
//! are not read as current claims. Whitespace is normalized so line
//! wrapping never decides a verdict.

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

/// No phrase is present in `text`.
fn refuses(text: &str, what: &str, phrases: &[&str]) {
    for phrase in phrases {
        assert!(!text.contains(phrase), "{what} still claims: {phrase:?}");
    }
}

/// The reviewed claims #87 names, gone from every current document. Short
/// enough to sit on one source line, so a comment prefix (`//!`, `#`) at a
/// line break cannot hide one from the check.
const OLD_MEMORY_CLAIMS: &[&str] = &[
    "bounded memory",
    "is the knob that bounds",
    "restored size of the claim",
];

#[test]
fn the_readme_states_the_memory_and_timestamp_contract() {
    let readme = doc("README.md");
    states(
        &readme,
        "README",
        &[
            "Memory is not globally bounded",
            "not a global byte or RSS cap",
            "`0` disables it",
            "the inbox count is unlimited",
            "all persisted rows restore before bind without application of that cap",
            "the data file's size on disk is not its size in RAM",
            "about 780 B per name observed on one build, not a fixed ceiling",
            "4,096-name cap on the await-metrics registry",
            "parse-start stamps",
            "before parsing and before acceptance",
            "need not sort in acceptance order across sessions or clock steps",
            "0038-store-memory-and-timestamps.md",
        ],
    );
    refuses(&readme, "README", OLD_MEMORY_CLAIMS);
}

#[test]
fn the_store_module_and_chart_state_the_memory_contract() {
    let store = doc("src/store.rs");
    states(
        &store,
        "src/store.rs",
        // Each phrase sits on one `//!` line of the module doc.
        &[
            "Memory is not globally bounded (decision 0038, proposed)",
            "zero disables it) is not a global byte or",
            "RSS cap: inbox count is unlimited",
            "applying the cap; its size on disk is not a measure of RAM",
            "per name observed on one build, not a fixed ceiling",
            "parsing and before acceptance",
        ],
    );
    refuses(&store, "src/store.rs", OLD_MEMORY_CLAIMS);
    let chart = doc("charts/swarmail/values.yaml");
    states(
        &chart,
        "charts/swarmail/values.yaml",
        // Each phrase sits on one `#` line of the comment.
        &[
            "amended by proposed 0038",
            "it is not a global byte or RSS cap, and the inbox count is",
            "restores every persisted row before bind without applying the cap",
            "disk does not measure it",
        ],
    );
    refuses(&chart, "charts/swarmail/values.yaml", OLD_MEMORY_CLAIMS);
}

#[test]
fn the_pop3_size_rule_is_stated_alike_everywhere() {
    let readme = doc("README.md");
    states(
        &readme,
        "README",
        &[
            "the transparency dots RETR adds not counted (RFC 1939 §11)",
            "0037-pop3-size-accounting.md",
        ],
    );
    refuses(
        &readme,
        "README",
        &["CRLF-terminated and dot-stuffed, the terminating"],
    );
    let pop3 = doc("src/pop3.rs");
    states(
        &pop3,
        "src/pop3.rs",
        &["transparency dots RETR adds and the client removes"],
    );
}

#[test]
fn the_index_lists_both_proposals_as_proposed() {
    let index = doc("docs/decisions/README.md");
    for file in [
        "0037-pop3-size-accounting.md",
        "0038-store-memory-and-timestamps.md",
    ] {
        let row = index
            .split("| [")
            .find(|row| row.contains(file))
            .unwrap_or_else(|| panic!("the index must list {file}"));
        assert!(row.contains("proposed — awaiting the operator"), "{row}");
    }
    for file in [
        "docs/decisions/0037-pop3-size-accounting.md",
        "docs/decisions/0038-store-memory-and-timestamps.md",
    ] {
        assert!(
            doc(file).contains("**Status:** proposed"),
            "{file} stays proposed"
        );
    }
}
