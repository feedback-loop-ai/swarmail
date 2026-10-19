//! The store and maildrop contracts as documented (proposed decisions 0037
//! and 0038, issues #84 and #87). #87 takes the documentation branch of
//! each either/or — the watcher residue, the absent global cap and the
//! parse-start timestamps are stated, not changed — so its red/green proof
//! is this documentary one: the reviewed text ("bounded memory", the
//! per-inbox knob "that bounds it", the dot-stuffed POP3 count) fails here,
//! and the corrected text passes. Accepted decisions are historical and
//! are not read as current claims. Whitespace is normalized so line
//! wrapping never decides a verdict.
//!
//! Every documented number is built from the code it documents — a public
//! constant, or a default observed on a running server — so changing the
//! constant turns these tests red (issue #143). A decision's status is
//! never hard-coded: it is read from the record's own `Status` header and
//! compared with the index row, in both directions, so the operator
//! accepting a decision (file and row together) keeps CI green.

mod common;

use std::path::Path;

/// A repository document, verbatim.
fn raw(path: &str) -> String {
    let full = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    std::fs::read_to_string(&full)
        .unwrap_or_else(|e| panic!("{} is unreadable: {e}", full.display()))
}

/// A repository document, whitespace-normalized to single spaces.
fn doc(path: &str) -> String {
    raw(path).split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The two status spellings the decision process has: the first word of a
/// status, with the emphasis and code marks the records use stripped
/// (`proposed`, `` `proposed` ``, `**accepted**`). Anything else is not a
/// status this check knows, and says so.
fn status_word(text: &str, what: &str) -> &'static str {
    let word = text
        .trim_start_matches(|c: char| c == '*' || c == '`' || c.is_whitespace())
        .split(|c: char| !c.is_ascii_alphabetic())
        .next()
        .unwrap_or("");
    match word {
        "accepted" => "accepted",
        "proposed" => "proposed",
        other => panic!("{what}: unsupported status spelling {other:?} in {text:?}"),
    }
}

/// A decision record's status, from its header line — `- **Status:** …`
/// or the founding records' `**Status**: …`.
fn file_status(record: &str, what: &str) -> &'static str {
    let line = record
        .lines()
        .find_map(|l| {
            l.strip_prefix("- **Status:**")
                .or_else(|| l.strip_prefix("**Status**:"))
        })
        .unwrap_or_else(|| panic!("{what} carries no Status header"));
    status_word(line, what)
}

/// The index's rows as (record file, status cell).
fn index_rows(index: &str) -> Vec<(String, &'static str)> {
    index
        .lines()
        .filter(|l| l.starts_with("| [0"))
        .map(|row| {
            let cells: Vec<&str> = row.split('|').map(str::trim).collect();
            let file = cells[1]
                .split_once("](")
                .and_then(|(_, rest)| rest.strip_suffix(')'))
                .unwrap_or_else(|| panic!("an index row without a record link: {row}"));
            (file.to_string(), status_word(cells[3], row))
        })
        .collect()
}

/// Every index row's status against its record's own header. `read` maps a
/// record file name to its text, so a fixture can stand in for the tree.
/// Returns each disagreement, naming both sides.
fn status_disagreements(index: &str, read: impl Fn(&str) -> String) -> Vec<String> {
    index_rows(index)
        .into_iter()
        .filter_map(|(file, indexed)| {
            let recorded = file_status(&read(&file), &file);
            (recorded != indexed)
                .then(|| format!("{file}: the record says {recorded}, the index says {indexed}"))
        })
        .collect()
}

/// A decision record under `docs/decisions/`, verbatim.
fn record(file: &str) -> String {
    raw(&format!("docs/decisions/{file}"))
}

/// A byte count as the documents spell it: `4,096`.
fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// A size in whole MiB, or its byte count when it is not one — so a
/// constant moved off the documented MiB never rounds back onto it.
fn mib(bytes: usize) -> String {
    if bytes.is_multiple_of(1 << 20) {
        format!("{} MiB", bytes >> 20)
    } else {
        format!("{bytes} B")
    }
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

/// The await-metrics registry cap the README documents, observed on a
/// running server rather than restated: this test binary is the only
/// user of the process-global registry, so after the documented cap plus
/// one distinct awaited names, the scrape holds exactly the cap's worth of
/// named rows and one overflow sample. A cap moved either way changes the
/// count, and the documented `4,096` with it, so the README phrases go red.
#[tokio::test]
async fn the_documented_registry_cap_is_the_running_one() {
    let readme = doc("README.md");
    let documented: usize = readme
        .split(" names. A name first seen after that has no row of its own")
        .next()
        .and_then(|head| head.rsplit("the first ").next())
        .map(|n| n.replace(',', ""))
        .and_then(|n| n.parse().ok())
        .expect("the README documents the registry cap as `the first N names`");
    let srv = common::start().await;
    let mut waits = tokio::task::JoinSet::new();
    let names: Vec<String> = (0..=documented).map(|i| format!("cap-{i}")).collect();
    for chunk in names.chunks(names.len().div_ceil(16)) {
        let chunk = chunk.to_vec();
        let addr = srv.http_addr;
        waits.spawn(async move {
            for name in chunk {
                let (st, body) = common::http_json(
                    addr,
                    "GET",
                    &format!("/api/v1/inboxes/{name}/await?timeout_ms=0"),
                    None,
                )
                .await;
                assert!(st < 500, "the await on {name} answered {st}: {body}");
            }
        });
    }
    while let Some(joined) = waits.join_next().await {
        joined.expect("an await batch completes");
    }
    let (st, scrape) = common::http_get_text(srv.http_addr, "/metrics").await;
    assert_eq!(st, 200, "{scrape}");
    // The overflow sample's label is one line feed, rendered `"\n"`.
    let overflow = "swarmail_awaits_registered_total{inbox=\"\\n\"}";
    let named = scrape
        .lines()
        .filter(|l| l.starts_with("swarmail_awaits_registered_total{inbox="))
        .filter(|l| !l.starts_with(overflow))
        .count();
    let cap = thousands(named);
    states(
        &readme,
        "README",
        &[
            &format!("for the first {cap} names"),
            &format!("{cap}-name cap on the await-metrics registry"),
        ],
    );
    assert!(
        scrape.lines().any(|l| l == format!("{overflow} 1")),
        "the one name past the cap is counted in the overflow sample: {scrape}"
    );
    srv.stop().await;
}

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
            "parse-start stamps",
            "before parsing and before acceptance",
            "need not sort in acceptance order across sessions or clock steps",
            "0038-store-memory-and-timestamps.md",
        ],
    );
    refuses(&readme, "README", OLD_MEMORY_CLAIMS);
}

/// The store module and the chart name 0038 with its current status, read
/// from the record: accepting it means updating these with it, never a
/// test that pins `proposed`.
#[test]
fn the_store_module_and_chart_state_the_memory_contract() {
    let status = file_status(&record("0038-store-memory-and-timestamps.md"), "0038");
    let store = doc("src/store.rs");
    states(
        &store,
        "src/store.rs",
        // Each phrase sits on one `//!` line of the module doc.
        &[
            &format!("Memory is not globally bounded (decision 0038, {status})"),
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
            &format!("amended by {status} 0038"),
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

/// Run 21's contracts as published (proposed decision 0044, AC11): the
/// widened recipient predicate, the nullable envelope sender, the renamed
/// unread key, the wipe route's honest name, latest's limits, the bounded
/// snippet and page budget and the unordered batch effects — stated in
/// README and llms.txt and in the served schema, with the drifted claims
/// issue #99 named gone.
#[test]
fn the_compat_validation_contracts_are_published() {
    let readme = doc("README.md");
    states(
        &readme,
        "README",
        &[
            "the SMTP envelope recipients — so a Bcc'd address matches",
            "`envelope_from`, the SMTP `MAIL FROM` reverse path",
            "the envelope's `messages_unread` is the match count",
            "swarmail's own explicit wipe of every inbox — not an upstream route",
            "a nonblank `?query=` is a `400` — search, then fetch the id",
            "the members' effects have **no** order",
            &format!(
                "its JSON would pass {} (`compat page exceeds byte budget {}; reduce limit or fetch raw by id",
                mib(swarmail::compat::MAX_COMPAT_PAGE_BYTES),
                swarmail::compat::MAX_COMPAT_PAGE_BYTES
            ),
            &format!(
                "at most an {}-byte prefix of its body",
                swarmail::compat::SNIPPET_SOURCE_PREFIX_BYTES
            ),
            "0044-compat-validation.md",
        ],
    );
    refuses(
        &readme,
        "README",
        &["messages_unread_count", "MailHog's wipe alias"],
    );
    let llms = doc("src/static/llms.txt");
    states(
        &llms,
        "llms.txt",
        &[
            "the SMTP envelope (RCPT TO), so a Bcc'd address matches",
            "`messages_unread`, `start`",
            "the effects have no order",
        ],
    );
    refuses(
        &llms,
        "llms.txt",
        &["messages_unread_count", "= the header From;"],
    );

    let openapi: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("src/static/openapi.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let to = openapi["paths"]["/api/v1/inboxes/{inbox}/messages"]["get"]["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "to")
        .unwrap()["description"]
        .clone();
    assert!(
        to.as_str()
            .unwrap()
            .contains("SMTP envelope recipients (RCPT TO)"),
        "{to}"
    );
    let envelope = &openapi["components"]["schemas"]["Email"]["properties"]["envelope_from"];
    assert_eq!(
        envelope["type"],
        serde_json::json!(["string", "null"]),
        "{envelope}"
    );
    for path in [
        "/api/v1/message/latest/raw",
        "/api/v1/message/latest/headers",
    ] {
        assert!(
            openapi["paths"][path]["get"].is_object(),
            "{path} is published"
        );
    }
    let wipe = openapi["paths"]["/api/v1/delete-all"]["delete"]["summary"]
        .as_str()
        .unwrap();
    assert!(wipe.contains("not an upstream route"), "{wipe}");
}

/// The records these contract tests (and the listener's) cite.
const CITED: &[&str] = &[
    "0036-smtp-trickle-lifetime.md",
    "0037-pop3-size-accounting.md",
    "0038-store-memory-and-timestamps.md",
    "0044-compat-validation.md",
    "0046-smtp-pop3-edges.md",
];

/// Every index row agrees with its record's own Status header — whatever
/// that status is. Red when either side moves alone; green when the
/// operator accepts a decision by updating both.
#[test]
fn every_index_row_matches_its_records_status() {
    let index = raw("docs/decisions/README.md");
    let rows = index_rows(&index);
    for file in CITED {
        assert!(
            rows.iter().any(|(indexed, _)| indexed == file),
            "the index must list {file}"
        );
    }
    let disagreements = status_disagreements(&index, record);
    assert!(disagreements.is_empty(), "{disagreements:#?}");
}

/// The comparison on disposable copies, never the real records: for each
/// cited proposal, accepting only the record or only the index row is a
/// disagreement naming both sides, and accepting both together passes.
#[test]
fn a_status_change_passes_only_when_record_and_index_move_together() {
    let index = raw("docs/decisions/README.md");
    let accepted_row = "| **accepted** (operator, fixture) |";
    for file in &CITED[..4] {
        let original = record(file);
        let header = original
            .lines()
            .find(|l| l.starts_with("- **Status:** proposed"))
            .unwrap_or_else(|| panic!("{file} is proposed in the tree"));
        let accepted_record =
            original.replacen(header, "- **Status:** accepted (operator, fixture)", 1);
        let row = index
            .lines()
            .find(|l| l.contains(&format!("]({file})")))
            .unwrap_or_else(|| panic!("the index must list {file}"));
        let accepted_index = index.replacen(
            row,
            &row.replacen("| proposed — awaiting the operator |", accepted_row, 1),
            1,
        );
        assert_ne!(accepted_index, index, "the fixture accepts {file}'s row");
        let read = |text: &str| {
            let text = text.to_string();
            move |name: &str| {
                if name == *file {
                    text.clone()
                } else {
                    record(name)
                }
            }
        };

        let record_only = status_disagreements(&index, read(&accepted_record));
        assert_eq!(
            record_only,
            [format!(
                "{file}: the record says accepted, the index says proposed"
            )],
            "an accepted record under a proposed row must disagree"
        );
        let index_only = status_disagreements(&accepted_index, read(&original));
        assert_eq!(
            index_only,
            [format!(
                "{file}: the record says proposed, the index says accepted"
            )],
            "an accepted row over a proposed record must disagree"
        );
        let both = status_disagreements(&accepted_index, read(&accepted_record));
        assert!(
            both.is_empty(),
            "accepting {file} in both places passes: {both:#?}"
        );
    }
}

/// The status parser reads every spelling the records and the index use,
/// and refuses one it does not know rather than guessing.
#[test]
fn status_spellings_parse_and_an_unknown_one_refuses() {
    assert_eq!(status_word(" `proposed` — acceptance", "t"), "proposed");
    assert_eq!(status_word(" **accepted** (operator)", "t"), "accepted");
    assert_eq!(
        file_status("x\n**Status**: accepted (v0.1)\n", "t"),
        "accepted"
    );
    assert_eq!(file_status("x\n- **Status:** proposed\n", "t"), "proposed");
    let unknown = std::panic::catch_unwind(|| status_word(" superseded by 0099", "t"));
    assert!(unknown.is_err(), "an unknown status spelling refuses");
}
