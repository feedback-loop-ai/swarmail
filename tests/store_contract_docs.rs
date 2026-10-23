//! The store and maildrop contracts as documented (decisions 0037 and
//! 0038, issues #84 and #87), and the decision record's own integrity
//! (issue #144): index membership, statuses and audited amendment links. #87 takes the documentation branch of
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

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
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

/// Run 21's contracts as published (decision 0044, AC11): the
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

/// A decision file name's number: `NNNN-slug.md` — four ASCII digits, a
/// hyphen and a nonempty slug of lowercase ASCII letters, digits and
/// hyphens. Anything else is not a decision file name.
fn decision_number(name: &str) -> Option<&str> {
    let (number, rest) = name.split_at_checked(4)?;
    let slug = rest.strip_prefix('-')?.strip_suffix(".md")?;
    let slug_ok = !slug.is_empty()
        && slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    (number.bytes().all(|b| b.is_ascii_digit()) && slug_ok).then_some(number)
}

/// The entries of `docs/decisions/`, sorted.
fn decision_dir() -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/decisions");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{} is unreadable: {e}", dir.display()))
        .map(|entry| {
            entry
                .expect("a readable directory entry")
                .file_name()
                .into_string()
                .expect("a UTF-8 file name")
        })
        .collect();
    names.sort();
    names
}

/// The decision file a number names in a directory listing.
fn file_of(names: &[String], number: &str) -> String {
    names
        .iter()
        .find(|name| decision_number(name) == Some(number))
        .unwrap_or_else(|| panic!("no decision file is numbered {number}"))
        .clone()
}

/// Every membership disagreement between the index and a directory
/// listing, in both directions: a malformed file name, two files with one
/// number, a row that is not one link, a row labeled with another number,
/// a file listed twice, two rows with one number, a row without a file and
/// a file without a row. `README.md` is the index, not a decision.
fn inventory_problems(index: &str, names: &[String]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut files: BTreeMap<&str, &str> = BTreeMap::new();
    for name in names.iter().filter(|name| *name != "README.md") {
        match decision_number(name) {
            None => problems.push(format!("{name}: not a decision file name (NNNN-slug.md)")),
            Some(number) => match files.entry(number) {
                Entry::Occupied(first) => problems.push(format!(
                    "{number}: two decision files, {} and {name}",
                    first.get()
                )),
                Entry::Vacant(slot) => {
                    slot.insert(name);
                }
            },
        }
    }
    let mut rows: BTreeMap<&str, usize> = BTreeMap::new();
    let mut labels: BTreeMap<&str, &str> = BTreeMap::new();
    for row in index.lines().filter(|l| l.starts_with("| [")) {
        let cells: Vec<&str> = row.split('|').map(str::trim).collect();
        let link = cells
            .get(1)
            .and_then(|cell| cell.strip_prefix('['))
            .and_then(|cell| cell.strip_suffix(')'))
            .and_then(|cell| cell.split_once("]("));
        let Some((label, file)) = link else {
            problems.push(format!("an index row without one record link: {row}"));
            continue;
        };
        if decision_number(file) != Some(label) {
            problems.push(format!("{file}: its index row is labeled {label}"));
        }
        *rows.entry(file).or_default() += 1;
        if let Some(other) = labels.insert(label, file).filter(|other| *other != file) {
            problems.push(format!("{label}: two index rows, for {other} and {file}"));
        }
    }
    for (file, count) in &rows {
        if *count > 1 {
            problems.push(format!("{file}: {count} index rows"));
        }
        if !names.iter().any(|name| name == file) {
            problems.push(format!("{file}: an index row with no file"));
        }
    }
    for file in files.values() {
        if !rows.contains_key(file) {
            problems.push(format!("{file}: no index row"));
        }
    }
    problems
}

/// Every numbered decision file has exactly one index row, every row has a
/// file, numbers and rows are unique, and (with
/// `every_index_row_matches_its_records_status`) every status agrees.
#[test]
fn every_decision_file_has_exactly_one_resolving_index_row() {
    let names = decision_dir();
    let index = raw("docs/decisions/README.md");
    let problems = inventory_problems(&index, &names);
    assert!(problems.is_empty(), "{problems:#?}");
    let decisions = names.iter().filter(|n| *n != "README.md").count();
    assert_eq!(
        index_rows(&index).len(),
        decisions,
        "one row per decision file"
    );
    let disagreements = status_disagreements(&index, record);
    assert!(disagreements.is_empty(), "{disagreements:#?}");
}

/// The inventory check on disposable copies of the real index and listing:
/// the old missing 0035/0055 rows, a dangling row, a duplicate row, a
/// mislabeled row, a second file with a taken number and every malformed
/// file name are each refused by name.
#[test]
fn the_inventory_refuses_missing_dangling_duplicate_and_malformed_entries() {
    let names = decision_dir();
    let index = raw("docs/decisions/README.md");
    let row_of = |number: &str| {
        let file = file_of(&names, number);
        index
            .lines()
            .find(|l| l.starts_with(&format!("| [{number}]({file})")))
            .unwrap_or_else(|| panic!("the index lists {file}"))
            .to_string()
    };
    let without = |rows: &[&str]| {
        index
            .lines()
            .filter(|l| !rows.contains(l))
            .collect::<Vec<_>>()
            .join("\n")
    };

    // The state issue #144 found: no 0035 row, and later no 0055 row.
    let (r35, r55) = (row_of("0035"), row_of("0055"));
    assert_eq!(
        inventory_problems(&without(&[r35.as_str(), r55.as_str()]), &names),
        [
            format!("{}: no index row", file_of(&names, "0035")),
            format!("{}: no index row", file_of(&names, "0055")),
        ]
    );

    let dangling = format!("{index}| [0099](0099-ghost.md) | ghost | proposed | |\n");
    assert_eq!(
        inventory_problems(&dangling, &names),
        ["0099-ghost.md: an index row with no file"]
    );

    let r34 = row_of("0034");
    let duplicate = format!("{index}{r34}\n");
    assert_eq!(
        inventory_problems(&duplicate, &names),
        [format!("{}: 2 index rows", file_of(&names, "0034"))]
    );

    let file36 = file_of(&names, "0036");
    let mislabeled = index.replacen(
        &format!("| [0036]({file36})"),
        &format!("| [0037]({file36})"),
        1,
    );
    let problems = inventory_problems(&mislabeled, &names);
    assert!(
        problems.contains(&format!("{file36}: its index row is labeled 0037")),
        "{problems:#?}"
    );
    assert!(
        problems
            .iter()
            .any(|p| p.starts_with("0037: two index rows")),
        "{problems:#?}"
    );

    let mut taken = names.clone();
    taken.push("0035-a-second-record.md".into());
    let problems = inventory_problems(&index, &taken);
    assert_eq!(
        problems,
        [format!(
            "0035: two decision files, {} and 0035-a-second-record.md",
            file_of(&names, "0035")
        )]
    );

    for malformed in [
        "35-short.md",
        "00351-long.md",
        "0035_underscore.md",
        "abcd-letters.md",
        "0035-.md",
        "0035-Upper.md",
        "0035-text.txt",
        "0035",
    ] {
        let mut listing = names.clone();
        listing.push(malformed.into());
        assert_eq!(
            inventory_problems(&index, &listing),
            [format!(
                "{malformed}: not a decision file name (NNNN-slug.md)"
            )],
            "{malformed} is refused"
        );
    }
}

/// A disposable record with the given status header.
fn fixture_record(status: &str) -> String {
    format!("# fixture\n\n- **Status:** {status}\n- **Date:** 2026-10-19\n")
}

/// A disposable index with one row per (file, status cell).
fn fixture_index(rows: &[(&str, &str)]) -> String {
    let mut index = String::from(
        "# Decision records\n\n| # | Decision | Status | Citation |\n|---|---|---|---|\n",
    );
    for (file, status) in rows {
        index.push_str(&format!(
            "| [{}]({file}) | fixture | {status} | |\n",
            &file[..4]
        ));
    }
    index
}

/// The status comparison on disposable fixtures, independent of today's
/// mix of accepted and proposed records: all-accepted, all-proposed and
/// mixed trees pass; a status change on either side alone refuses, naming
/// both sides; the same change on both sides passes; and the founding
/// records' `**Status**:` spelling reads the same.
#[test]
fn a_status_change_passes_only_when_record_and_index_move_together() {
    const ALPHA: &str = "0001-alpha.md";
    const BETA: &str = "0002-beta.md";
    const ACCEPTED: &str = "accepted (operator, fixture)";
    const PROPOSED: &str = "proposed — acceptance is the operator's.";
    const ACCEPTED_ROW: &str = "**accepted** (operator, fixture)";
    const PROPOSED_ROW: &str = "proposed — awaiting the operator";
    let names: Vec<String> = ["README.md", ALPHA, BETA].map(String::from).into();
    let check = |rows: &[(&str, &str)], alpha: &str, beta: &str| {
        let index = fixture_index(rows);
        let problems = inventory_problems(&index, &names);
        assert!(problems.is_empty(), "{problems:#?}");
        let (alpha, beta) = (alpha.to_string(), beta.to_string());
        status_disagreements(&index, move |file| match file {
            ALPHA => alpha.clone(),
            BETA => beta.clone(),
            other => panic!("the fixture has no {other}"),
        })
    };
    let accepted = fixture_record(ACCEPTED);
    let proposed = fixture_record(PROPOSED);

    let all_accepted = [(ALPHA, ACCEPTED_ROW), (BETA, ACCEPTED_ROW)];
    assert!(check(&all_accepted, &accepted, &accepted).is_empty());
    let all_proposed = [(ALPHA, PROPOSED_ROW), (BETA, PROPOSED_ROW)];
    assert!(check(&all_proposed, &proposed, &proposed).is_empty());
    let mixed = [(ALPHA, ACCEPTED_ROW), (BETA, PROPOSED_ROW)];
    assert!(check(&mixed, &accepted, &proposed).is_empty());

    assert_eq!(
        check(&mixed, &accepted, &accepted),
        ["0002-beta.md: the record says accepted, the index says proposed"],
        "accepting only the record refuses"
    );
    assert_eq!(
        check(&all_accepted, &accepted, &proposed),
        ["0002-beta.md: the record says proposed, the index says accepted"],
        "accepting only the row refuses"
    );
    assert_eq!(
        check(&mixed, &proposed, &proposed),
        ["0001-alpha.md: the record says proposed, the index says accepted"],
        "a one-sided return to proposed refuses too"
    );
    assert!(
        check(&all_accepted, &accepted, &accepted).is_empty(),
        "the coherent pair passes"
    );

    let founding = "# fixture\n\n**Status**: accepted (original design, v0.1)\n".to_string();
    let founding_rows = [(ALPHA, "accepted (v0.1 design)"), (BETA, PROPOSED_ROW)];
    assert!(check(&founding_rows, &founding, &proposed).is_empty());

    // Membership refusals on the fixture itself: missing, dangling, duplicate.
    let index = fixture_index(&[(ALPHA, ACCEPTED_ROW)]);
    assert_eq!(
        inventory_problems(&index, &names),
        ["0002-beta.md: no index row"]
    );
    let index = fixture_index(&[
        (ALPHA, ACCEPTED_ROW),
        (BETA, PROPOSED_ROW),
        ("0003-gamma.md", PROPOSED_ROW),
    ]);
    assert_eq!(
        inventory_problems(&index, &names),
        ["0003-gamma.md: an index row with no file"]
    );
    let index = fixture_index(&[
        (ALPHA, ACCEPTED_ROW),
        (BETA, PROPOSED_ROW),
        (BETA, PROPOSED_ROW),
    ]);
    assert_eq!(
        inventory_problems(&index, &names),
        ["0002-beta.md: 2 index rows"]
    );
}

/// The audited amendment relationships (issue #144, AC7): (newer, older,
/// the newer record's header field that names the older, the verb that
/// field uses, and the state the older record's dated marker records).
/// The older record carries `*(<verb> by [newer](newer-file) (<state>`.
/// A finite, test-owned list — no prose is inferred.
const AMENDMENTS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "0018",
        "0008",
        "**Amends:**",
        "Amended",
        "accepted 2026-10-13",
    ),
    (
        "0026",
        "0008",
        "**Amends:**",
        "Amended",
        "accepted 2026-10-14",
    ),
    (
        "0028",
        "0004",
        "**Amends:**",
        "Amended",
        "accepted 2026-10-16",
    ),
    (
        "0031",
        "0005",
        "**Amends:**",
        "Amended",
        "accepted 2026-10-16",
    ),
    (
        "0030",
        "0018",
        "**Extends:**",
        "Extended",
        "accepted 2026-10-16",
    ),
    (
        "0030",
        "0023",
        "**Extends:**",
        "Extended",
        "accepted 2026-10-16",
    ),
    ("0034", "0022", "**Amends:**", "Amended", ACCEPTED_1019),
    ("0034", "0028", "**Amends:**", "Amended", ACCEPTED_1019),
    ("0035", "0020", "**Amends:**", "Amended", ACCEPTED_1019),
    ("0036", "0027", "**Amends:**", "Amended", ACCEPTED_1019),
    ("0037", "0028", "**Amends:**", "Amended", ACCEPTED_1019),
    ("0038", "0021", "**Amends:**", "Amended", ACCEPTED_1019),
    ("0038", "0026", "**Amends:**", "Amended", ACCEPTED_1019),
    ("0039", "0029", "**Amends:**", "Amended", ACCEPTED_1019),
    ("0040", "0031", "**Amends:**", "Amended", ACCEPTED_1019),
    ("0041", "0029", "**Amends:**", "Amended", ACCEPTED_1019),
    ("0042", "0033", "**Amends:**", "Amended", ACCEPTED_1019),
    ("0043", "0030", "**Amends:**", "Amended", ACCEPTED_1019),
    (
        "0044",
        "0029",
        "**Proposes amendments to:**",
        "Amended",
        ACCEPTED_1019,
    ),
    (
        "0044",
        "0030",
        "**Proposes amendments to:**",
        "Amended",
        ACCEPTED_1019,
    ),
    (
        "0044",
        "0031",
        "**Proposes amendments to:**",
        "Amended",
        ACCEPTED_1019,
    ),
    (
        "0044",
        "0039",
        "**Proposes amendments to:**",
        "Updated",
        ACCEPTED_1019,
    ),
    (
        "0045",
        "0007",
        "**Amends/clarifies:**",
        "Amended/clarified",
        ACCEPTED_1019,
    ),
    (
        "0045",
        "0009",
        "**Amends/clarifies:**",
        "Amended/clarified",
        ACCEPTED_1019,
    ),
    (
        "0045",
        "0015",
        "**Amends/clarifies:**",
        "Amended/clarified",
        ACCEPTED_1019,
    ),
    (
        "0045",
        "0018",
        "**Amends/clarifies:**",
        "Amended/clarified",
        ACCEPTED_1019,
    ),
    (
        "0045",
        "0023",
        "**Records existing amendments:**",
        "Recorded",
        ACCEPTED_1019,
    ),
    (
        "0046",
        "0027",
        "**Proposes amendments to:**",
        "Amended",
        ACCEPTED_1019,
    ),
    (
        "0046",
        "0028",
        "**Proposes amendments to:**",
        "Amended",
        ACCEPTED_1019,
    ),
    (
        "0046",
        "0034",
        "**Updates proposals:**",
        "Updated",
        ACCEPTED_1019,
    ),
    (
        "0046",
        "0036",
        "**Updates proposals:**",
        "Updated",
        ACCEPTED_1019,
    ),
    (
        "0046",
        "0044",
        "**Updates proposals:**",
        "Updated",
        ACCEPTED_1019,
    ),
    (
        "0046",
        "0029",
        "**Updates proposals:**",
        "Strengthened",
        ACCEPTED_1019,
    ),
    (
        "0047",
        "0033",
        "**Supplements:**",
        "Supplemented",
        ACCEPTED_1019,
    ),
    (
        "0047",
        "0042",
        "**Supplements:**",
        "Supplemented",
        ACCEPTED_1019,
    ),
    (
        "0047",
        "0029",
        "**Proposes amendment to:**",
        "Amended",
        ACCEPTED_1019,
    ),
    (
        "0047",
        "0039",
        "**Updates proposals:**",
        "Updated",
        ACCEPTED_1019,
    ),
    (
        "0047",
        "0043",
        "**Updates proposals:**",
        "Updated",
        ACCEPTED_1019,
    ),
    (
        "0047",
        "0044",
        "**Updates proposals:**",
        "Updated",
        ACCEPTED_1019,
    ),
    (
        "0048",
        "0025",
        "**Proposes amendment to:**",
        "Amended",
        ACCEPTED_1019,
    ),
    (
        "0048",
        "0029",
        "**Proposes amendment to:**",
        "Amended",
        ACCEPTED_1019,
    ),
    (
        "0048",
        "0041",
        "**Clarifies/extends:**",
        "Clarified/extended",
        ACCEPTED_1019,
    ),
    (
        "0048",
        "0044",
        "**Clarifies/extends:**",
        "Clarified/extended",
        ACCEPTED_1019,
    ),
    (
        "0049",
        "0040",
        "**Updates proposed:**",
        "Updated",
        ACCEPTED_1019,
    ),
    (
        "0049",
        "0044",
        "**Updates proposed:**",
        "Updated",
        ACCEPTED_1019,
    ),
    (
        "0049",
        "0038",
        "**Updates proposed:**",
        "Updated",
        ACCEPTED_1019,
    ),
    (
        "0049",
        "0030",
        "**Extends/clarifies accepted:**",
        "Extended/clarified",
        ACCEPTED_1019,
    ),
    (
        "0049",
        "0031",
        "**Extends/clarifies accepted:**",
        "Extended/clarified",
        ACCEPTED_1019,
    ),
    ("0050", "0035", "**Refines:**", "Refined", ACCEPTED_1019),
    ("0050", "0045", "**Refines:**", "Refined", ACCEPTED_1019),
    (
        "0051",
        "0019",
        "**Amends/clarifies:**",
        "Amended/clarified",
        ACCEPTED_1024,
    ),
    (
        "0051",
        "0045",
        "**Amends/clarifies:**",
        "Amended/clarified",
        ACCEPTED_1024,
    ),
];

/// The marker state of a relationship the operator accepted on 2026-10-19.
const ACCEPTED_1019: &str = "accepted 2026-10-19";
/// The marker state of the accepted 0051 amendment recorded for #184.
const ACCEPTED_1024: &str = "accepted 2026-10-24";

/// A header field of a whitespace-normalized record: the text after
/// `label` up to the next field or section.
fn header_field<'a>(text: &'a str, label: &str) -> Option<&'a str> {
    let rest = &text[text.find(label)? + label.len()..];
    let end = [rest.find(" - **"), rest.find(" ## ")]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(rest.len());
    Some(&rest[..end])
}

/// Both sides of every listed relationship. `read` maps a decision number
/// to its record's text, so fixtures can stand in for the tree. The newer
/// record's field must name the older; the older must carry the dated
/// marker linking the newer; and the marker's state must be the newer
/// record's own status, so no marker claims an acceptance the record lacks.
fn amendment_problems(
    pairs: &[(&str, &str, &str, &str, &str)],
    names: &[String],
    read: impl Fn(&str) -> String,
) -> Vec<String> {
    let normal = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut problems = Vec::new();
    for &(newer, older, field, verb, state) in pairs {
        let newer_raw = read(newer);
        match header_field(&normal(&newer_raw), field) {
            None => problems.push(format!("{newer} declares no {field}")),
            Some(named) if !named.contains(older) => {
                problems.push(format!("{newer}'s {field} does not name {older}"))
            }
            Some(_) => {}
        }
        let status = file_status(&newer_raw, newer);
        if !state.starts_with(status) {
            problems.push(format!(
                "{newer} is {status}, but its marker in {older} says {state}"
            ));
        }
        let marker = format!("*({verb} by [{newer}]({}) ({state}", file_of(names, newer));
        if !normal(&read(older)).contains(&marker) {
            problems.push(format!("{older} carries no dated marker {marker:?}"));
        }
    }
    problems
}

/// Every audited relationship holds on both sides in the tree, and each
/// side removed in turn is refused — the successor's declaration and the
/// predecessor's dated marker alike.
#[test]
fn every_audited_amendment_is_linked_on_both_sides() {
    let names = decision_dir();
    let tree = |number: &str| record(&file_of(&names, number));
    let problems = amendment_problems(AMENDMENTS, &names, tree);
    assert!(problems.is_empty(), "{problems:#?}");

    for (i, pair) in AMENDMENTS.iter().enumerate() {
        let &(newer, older, field, verb, state) = pair;
        let one = &AMENDMENTS[i..=i];

        let no_declaration = |number: &str| {
            let text = tree(number);
            if number == newer {
                text.replace(field, "")
            } else {
                text
            }
        };
        let problems = amendment_problems(one, &names, no_declaration);
        assert_eq!(
            problems,
            [format!("{newer} declares no {field}")],
            "{newer} -> {older}: removing the declaration must fail"
        );

        let marker = format!("*({verb} by [{newer}]({}) ({state}", file_of(&names, newer));
        let no_marker = |number: &str| {
            let text = tree(number);
            if number == older {
                text.split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .replace(&marker, "")
            } else {
                text
            }
        };
        let problems = amendment_problems(one, &names, no_marker);
        assert_eq!(
            problems,
            [format!("{older} carries no dated marker {marker:?}")],
            "{newer} -> {older}: removing the marker must fail"
        );
    }
}

/// Issue #144's original case: 0045 §§6–7 claimed pointers in 0015, 0018
/// and 0023 that did not exist. With the 2026-10-19 marker sections cut
/// from those three records, all three 0045 relationships are refused.
#[test]
fn the_pointers_0045_promised_are_required() {
    let names = decision_dir();
    let promised: Vec<_> = AMENDMENTS
        .iter()
        .copied()
        .filter(|&(newer, older, ..)| newer == "0045" && ["0015", "0018", "0023"].contains(&older))
        .collect();
    assert_eq!(promised.len(), 3, "0045 -> 0015, 0018 and 0023 are audited");
    let before = |number: &str| {
        let text = record(&file_of(&names, number));
        match text.split_once("## Later-decision markers — 2026-10-19") {
            Some((head, _)) if ["0015", "0018", "0023"].contains(&number) => head.to_string(),
            _ => text,
        }
    };
    let problems = amendment_problems(&promised, &names, before);
    assert_eq!(problems.len(), 3, "{problems:#?}");
    for older in ["0015", "0018", "0023"] {
        assert!(
            problems
                .iter()
                .any(|p| p.starts_with(&format!("{older} carries no dated marker"))),
            "{older}: {problems:#?}"
        );
    }
}

/// The operator's 0051 adoption and the four #184 errata remain explicit,
/// dated and bounded to the cited contract facts.
#[test]
fn issue_184_adoption_and_factual_errata_are_recorded() {
    let adopted = record("0051-release-records.md");
    assert_eq!(file_status(&adopted, "0051"), "accepted");
    states(
        &doc("docs/decisions/0051-release-records.md"),
        "0051",
        &[
            "accepted (operator, 2026-10-24)",
            "Original 2026-10-19 proposal status",
            "Corrected 2026-10-23 proposal status",
            "SHA-256 89fff17e5a160de79e837bb0e61b2f00bfeef85241d515d1a18e7679f9f257dd",
            "statements that the choice remains open",
            "successful joint image-and-chart tag release",
            "tag version = Cargo package version = Chart appVersion = Chart version",
            "preparation versions may differ",
        ],
    );

    let index = raw("docs/decisions/README.md");
    let row = index
        .lines()
        .find(|line| line.starts_with("| [0051]("))
        .expect("0051 index row");
    assert_eq!(
        index_rows(&index)
            .into_iter()
            .find(|(file, _)| file == "0051-release-records.md")
            .map(|(_, status)| status),
        Some("accepted")
    );
    states(
        row,
        "0051 index row",
        &[
            "accepted (operator, 2026-10-24)",
            "89fff17e5a160de79e837bb0e61b2f00bfeef85241d515d1a18e7679f9f257dd",
        ],
    );

    let release_integrity = doc("docs/decisions/0019-release-tag-integrity.md");
    states(
        &release_integrity,
        "0019",
        &[
            "proposed 2026-10-19 — not accepted",
            "Amended/clarified by [0051](0051-release-records.md) (accepted 2026-10-24",
            "successful joint image-and-chart tag release only",
            "preparation versions may differ",
            "exact-ref binds and absent-tag refusals remain",
        ],
    );
    let delivery = doc("docs/decisions/0045-delivery-repairs.md");
    states(
        &delivery,
        "0045",
        &[
            "proposed 2026-10-19 — not accepted",
            "Amended/clarified by [0051](0051-release-records.md) (accepted 2026-10-24",
            "main's continuous image publication keeps its separate §4 role",
        ],
    );

    states(
        &doc("docs/decisions/0028-wire-octets-auth-refusals-and-the-maildrop-snapshot.md"),
        "0028",
        &[
            "Erratum — 2026-10-24",
            "disabled AUTH returns 502",
            "repeated AUTH returns 503 before credential decoding",
            "unsupported mechanism returns 504",
            "eligible AUTH syntax or credential failures return 501",
            "failed USER returns -ERR",
            "subsequent PASS return -ERR USER required first",
            "One successful AUTH per TLS epoch remains",
        ],
    );
    states(
        &doc("docs/decisions/0030-loopback-plane-bounded-results.md"),
        "0030",
        &[
            "Erratum — 2026-10-24",
            "Applicable page and wait domains retain their 250 maximum",
            "its count is an unsigned exact expected count, zero is allowed",
            "Its limit is an unsigned, type-checked inert value with no published default or maximum",
            "src/static/openapi.json",
        ],
    );
    states(
        &doc("docs/decisions/0007-hermetic-gates-single-model.md"),
        "0007",
        &[
            "accepted on 2026-10-13",
            "recorded on 2026-10-24, not backdated",
            "judging axis only",
            "supplies the later route authority",
        ],
    );
    states(
        &doc("docs/decisions/0023-off-worker-commits.md"),
        "0023",
        &[
            "watcher-lifetime record referred to in the provenance above is [0021](0021-watcher-channel-lifetime.md)",
            "0021 owns watcher channel lifetime and atomic registration",
            "0023 remains the record for off-worker commits",
        ],
    );
}

/// A fabricated proposed record must not claim an accepted marker.
#[test]
fn a_marker_cannot_claim_an_acceptance_the_record_lacks() {
    let names = decision_dir();
    let proposed_0051 = format!(
        "{}- **Amends/clarifies:** [0019](0019-release-tag-integrity.md)\n",
        fixture_record("proposed — acceptance is the operator's.")
    );
    let fixture = |number: &str| {
        if number == "0051" {
            proposed_0051.clone()
        } else {
            record(&file_of(&names, number))
        }
    };
    let claimed = [(
        "0051",
        "0019",
        "**Amends/clarifies:**",
        "Amended/clarified",
        ACCEPTED_1024,
    )];
    let problems = amendment_problems(&claimed, &names, fixture);
    assert_eq!(
        problems,
        ["0051 is proposed, but its marker in 0019 says accepted 2026-10-24"],
        "{problems:#?}"
    );
}

/// 0035's known limits keep their historical numbers 1, 2, 3, 5; a dated
/// clarification maps old item 5 to the fourth item without renumbering.
#[test]
fn the_0035_numbering_is_clarified_not_rewritten() {
    let text = doc("docs/decisions/0035-coverage-gate-refuses-by-effect.md");
    states(
        &text,
        "0035",
        &[
            "5. A source file syn cannot parse refuses loudly",
            "## Clarification — 2026-10-19 (known-limits numbering)",
            "Read old item **5**",
            "as the fourth sequential item",
            "keeps its identity",
        ],
    );
    refuses(&text, "0035", &["4. A source file syn cannot parse"]);
}

/// House rules state the rule for code under a proposed decision (accepted
/// 0045 §8, issue #144 AC8), and the decision directory is described as
/// holding both rulings and proposals.
#[test]
fn house_rules_state_the_rule_for_code_under_a_proposal() {
    let rules = doc("docs/house-rules.md");
    states(
        &rules,
        "docs/house-rules.md",
        &[
            "**Code under a proposed decision** (accepted decision 0045 §8)",
            "A proposal is a status, not operator acceptance.",
            "Compatible implementation — work that preserves every accepted ruling — may land while its decision is still proposed.",
            "A change that contradicts an accepted ruling waits for operator acceptance of the decision that amends it",
            "that holds for amendments to every one of decisions 0001–0005",
            "Weakening a frozen guarantee is refused regardless of green tests",
            "is not standing permission for future ones",
        ],
    );
    let index = doc("docs/decisions/README.md");
    states(
        &index,
        "docs/decisions/README.md",
        &["Accepted rulings, and proposals awaiting the operator,"],
    );
    let agents = doc("AGENTS.md");
    states(
        &agents,
        "AGENTS.md",
        &["accepted rulings, and the proposals awaiting the operator,"],
    );
    refuses(
        &index,
        "docs/decisions/README.md",
        &["Accepted rulings on why"],
    );
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

// ---------- #142: one gate list, and the corrected records ----------

/// A GitHub heading anchor: lowercased, punctuation other than `-`/`_`
/// dropped, each space a hyphen — `Gates — part of every change` is
/// `gates--part-of-every-change`.
fn anchor(heading: &str) -> String {
    heading
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(*c, ' ' | '-' | '_'))
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

/// The jobs `.github/workflows/ci.yml` declares: the keys indented exactly
/// two spaces under `jobs:`.
fn ci_jobs() -> Vec<String> {
    let ci = raw(".github/workflows/ci.yml");
    let (_, declared) = ci.split_once("\njobs:\n").expect("ci.yml has a jobs map");
    declared
        .lines()
        .filter_map(|l| l.strip_prefix("  "))
        .filter(|l| !l.starts_with([' ', '#']))
        .filter_map(|l| l.strip_suffix(':'))
        .map(str::to_string)
        .collect()
}

/// The text of `text` from the heading line `from` up to the next `## `
/// heading.
fn section<'a>(text: &'a str, from: &str) -> &'a str {
    let start = text
        .find(from)
        .unwrap_or_else(|| panic!("no section {from:?}"));
    let rest = &text[start + from.len()..];
    &rest[..rest.find("\n## ").unwrap_or(rest.len())]
}

/// Issue #142, D01/D03: house rules' Gates section is the one inventory,
/// and every other document points at it by its real anchor instead of
/// carrying a shorter list of its own. Red when a pointer goes stale (the
/// heading renamed under it), when a pointer is replaced by a divergent
/// unlocked list again, or when a charter compiles the repo root.
#[test]
fn the_gate_inventory_has_one_home_and_everyone_points_at_it() {
    let rules = raw("docs/house-rules.md");
    let heading = rules
        .lines()
        .find_map(|l| l.strip_prefix("## Gates"))
        .map(|rest| format!("Gates{rest}"))
        .expect("house rules carry a Gates section");
    let link = format!("docs/house-rules.md#{}", anchor(&heading));
    let inventory = doc_text(section(&rules, "## Gates"));
    states(
        &inventory,
        "house rules' Gates",
        &[
            "cargo fmt --all -- --check",
            "cargo clippy --workspace --all-targets --locked -- -D warnings",
            "cargo test --workspace --locked",
            "brokkr compile --bundle bundles/self",
            "bash scripts/coverage-gate.sh selftest",
            "bash scripts/publish-guard-test.sh",
            "cargo deny check licenses",
            "`native-tls` is absent",
            "The verify seat runs",
        ],
    );
    for (path, prefix) in [
        ("README.md", ""),
        ("CONTRIBUTING.md", ""),
        ("AGENTS.md", ""),
        ("agents/charters/implementer.md", "../../"),
        ("agents/charters/review-correctness.md", "../../"),
        ("agents/charters/reviewer.md", "../../"),
    ] {
        let text = raw(path);
        let wanted = format!("({prefix}{link})");
        assert!(text.contains(&wanted), "{path} must link {wanted}");
        // The divergent short lists #142 found: unlocked clippy and test
        // lines, `fmt --check` without `--all`, and the root bundle.
        refuses(
            &doc(path),
            path,
            &[
                "cargo clippy --all-targets -- -D warnings",
                "`cargo fmt --check`",
                "--bundle .`",
                "--bundle . ",
            ],
        );
    }
    refuses(
        &doc("agents/README.md"),
        "agents/README.md",
        &["--bundle .`"],
    );
}

/// D09: the README's CI table names every job ci.yml declares and its
/// suite table every root suite under `tests/`, so a new job or suite
/// that is not documented — or a stale name such as the old `chart` job —
/// goes red.
#[test]
fn the_readme_names_every_ci_job_and_every_root_suite() {
    let readme = raw("README.md");
    let testing = section(&readme, "## Testing & coverage");
    let jobs = ci_jobs();
    assert!(jobs.len() >= 10, "ci.yml jobs were found: {jobs:?}");
    for job in &jobs {
        assert!(
            testing.contains(&format!("`{job}`")),
            "README's CI table must name the `{job}` job"
        );
    }
    let mut suites = 0;
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    for entry in std::fs::read_dir(dir).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        if name.ends_with(".rs") {
            suites += 1;
            assert!(
                testing.contains(&format!("| `tests/{name}` |")),
                "README's suite table must list tests/{name}"
            );
        }
    }
    assert!(suites >= 20, "the root suites were found");
    refuses(
        &doc("README.md"),
        "README",
        &["the CI `chart` job", "functions** (today"],
    );
}

/// D05: `deny.toml` allows exactly what house rules' Dependencies names,
/// with the two unused allowances #142 removed.
#[test]
fn the_license_allowlist_is_the_one_house_rules_names() {
    let deny = raw("deny.toml");
    let (_, list) = deny.split_once("allow = [").expect("deny.toml allow list");
    let (entries, _) = list.split_once(']').unwrap();
    let allowed: Vec<&str> = entries
        .lines()
        .map(|l| l.trim())
        .filter(|l| l.starts_with('"'))
        .map(|l| l.trim_end_matches(',').trim_matches('"'))
        .collect();
    assert!(allowed.contains(&"CDLA-Permissive-2.0"), "{allowed:?}");
    for unused in ["Zlib", "CC-BY-4.0"] {
        assert!(
            !allowed.contains(&unused),
            "{unused} is an unused allowance (#142)"
        );
    }
    let rules = raw("docs/house-rules.md");
    let deps = doc_text(section(&rules, "## Dependencies"));
    for license in &allowed {
        assert!(
            deps.contains(license),
            "house rules' Dependencies must name {license}"
        );
    }
}

/// Whitespace-normalized text of an excerpt.
fn doc_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// D06–D20: the corrected sentences, each with the false one it replaced
/// refused. The entity-table size is read from the table itself.
#[test]
fn the_corrected_release_and_runtime_claims_stay_corrected() {
    let readme = doc("README.md");
    let k8s = file_status(&record("0008-kubernetes-deployment-contract.md"), "0008");
    states(
        &readme,
        "README",
        &[
            &format!("0008-kubernetes-deployment-contract.md), *{k8s}*"),
            "not published on crates.io",
            "append-only, with dated errata",
            "`serve` and `gen-cert` log to stdout",
            "Only `mcp` logs to stderr",
            "exactly 100/100/100",
            "under `target/coverage/`",
            "2,097,152 bytes",
            "plain-text `413`",
            "a second AUTH on the same connection answers `503`",
            "files *both* tests' mail under `a`",
            "Open a fresh connection per inbox",
            "ends the session with **no reply**",
            "is accepted at startup, and then every STARTTLS handshake fails",
            "**Stopping:**",
            "knife's edge",
            "`docker stop -t 30`",
        ],
    );
    refuses(
        &readme,
        "README",
        &[
            "The crate ships to",
            "ships to [crates.io]",
            "release notes are frozen",
            "every other subcommand to stderr",
        ],
    );

    let notes = doc("docs/releases/v0.1.0.md");
    let entities = raw("src/extract/entities.rs");
    let (_, table) = entities
        .split_once("static NAMED: &[(&str, &str)] = &[")
        .expect("the entity table");
    let (table, _) = table.split_once("\n];").unwrap();
    let names = table.lines().filter(|l| l.starts_with("    (\"")).count();
    states(
        &notes,
        "docs/releases/v0.1.0.md",
        &[
            "## Errata and provenance",
            "no CI job runs `cargo bench`",
            "71-entry subset",
            &format!("all {} semicolon-ended WHATWG names", thousands(names)),
            "a nonblank `?query=` is a 400",
            "`Re[2]:` is not a marker",
            "fixed, value-free text",
            "`messages_unread_count` is now `messages_unread`",
            "docker run --rm --stop-timeout 30",
        ],
    );
    refuses(
        &doc("docs/index.html"),
        "docs/index.html",
        &["Both run in CI."],
    );
}

// Issue #182: these claims describe existing handlers; they must not turn
// HTTP refusals, JSON-RPC errors and normal tool results into one envelope.
#[test]
fn issue_182_unknown_mcp_arguments_keep_their_json_rpc_envelope() {
    for path in ["README.md", "src/static/llms.txt"] {
        let text = doc(path);
        states(
            &text,
            path,
            &["`-32602` in HTTP 200 for a call with an `id`"],
        );
        refuses(
            &text,
            path,
            &["an MCP argument key (`{\"inbx\": \"zz\"}`), or an unknown key"],
        );
    }
}

#[test]
fn issue_182_seed_and_webhook_data_errors_do_not_cover_extractor_refusals() {
    for path in ["README.md", "src/static/llms.txt"] {
        let text = doc(path);
        states(
            &text,
            path,
            &[
                "Seed and webhook JSON data errors are `400` with a JSON `error`",
                "Malformed JSON syntax stays the extractor's plain-text `400`",
                "missing/wrong media type its plain-text `415`",
                "oversize its plain-text `413`",
                "Chaos JSON data errors stay the extractor's `422`",
            ],
        );
        refuses(
            &text,
            path,
            &[
                "A seed or webhook body error is a `400` with a JSON `error`",
                "Seed and webhook body errors are 400 with a JSON `error`",
            ],
        );
    }
}

#[test]
fn issue_182_mcp_wait_results_keep_distinct_envelopes() {
    for path in ["README.md", "src/static/llms.txt"] {
        let text = doc(path);
        states(
            &text,
            path,
            &[
                "Except for the supplied `timeout_ms` refusal described below",
                "returns JSON-RPC `-32602` in HTTP 200 for a call with an `id`",
                "before any effect",
                "An elapsed `swarmail_wait_for_email` timeout is a normal tool result",
                "`isError: false`",
                "with `matched`, `total` and `timed_out: true`",
                "On `swarmail_wait_for_email`, a supplied `timeout_ms`",
                "must be an integer from 0 through 60000",
                "malformed values and values above 60000 return an HTTP-200 tool result",
                "with `isError: true` and refusal text",
                "not a JSON-RPC `error` object",
                "Other protocol and argument refusals use a JSON-RPC `error`",
                "`shutting down`",
            ],
        );
        refuses(
            &text,
            path,
            &[
                "Errors are always JSON-RPC",
                "Protocol and argument refusals use a JSON-RPC `error`",
                "Every tool validates its supplied arguments at the envelope: a wrong-typed or malformed value",
            ],
        );
    }
}

#[test]
fn issue_182_thread_keys_name_the_full_case_folding_limit() {
    states(
        &doc("README.md"),
        "README",
        &["not full case folding: `ß` and `ss` remain distinct in subject fallback keys"],
    );
}

#[test]
fn issue_182_pop3_documents_the_snapshot_and_live_store_skew() {
    states(
        &doc("README.md"),
        "README",
        &[
            "a snapshot taken at successful `PASS`, not a lock",
            "numbering stays stable",
            "new arrivals appear in the next session",
            "Listings read the snapshot",
            "`RETR`, `TOP` and `DELE` consult the live store",
            "mail removed elsewhere can remain listed",
            "`-ERR no such message`",
        ],
    );
}
