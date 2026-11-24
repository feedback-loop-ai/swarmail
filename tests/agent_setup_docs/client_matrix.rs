//! The MCP client matrix's documents and checked-in receipts (issue #392,
//! epic #164).
//!
//! The receipt checker is the evals harness's own
//! (`evals/src/client_matrix/receipt.rs`), compiled here by path so the
//! checked-in directory, the handbook's example and the runner's drafts are
//! read by one checker. Its mutation cases live beside it in `evals/`.
//! A well-formed receipt is never a compatibility verdict (decisions 0079
//! and 0080).
//!
//! Every test here is file-only: no subprocess, no socket, no wait.

#[allow(dead_code)] // the runner's draft helpers are unused here
#[path = "../../evals/src/client_matrix/receipt.rs"]
mod receipt;

use receipt::{CLIENTS, EMPTY, FIELDS, MATRIX, README, RECEIPTS, STEPS, TOOLS, scan};
use std::path::Path;

#[test]
fn client_matrix_checked_in_receipts_are_well_formed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join(RECEIPTS);
    assert!(
        dir.join(README).is_file(),
        "{RECEIPTS}/README.md is tracked so a path typo cannot pass as empty"
    );
    let status = scan(&dir).unwrap_or_else(|e| panic!("{e}"));
    println!("client matrix: {status}");
    assert!(status == EMPTY || status.ends_with("no compatibility verdict"));
}

#[test]
fn client_matrix_an_empty_directory_passes_with_no_receipts_yet() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(scan(dir.path()).unwrap(), EMPTY);
    std::fs::write(dir.path().join(README), "# receipts\n").unwrap();
    assert_eq!(scan(dir.path()).unwrap(), EMPTY);
}

// ---------------------------------------------------------------- the docs

fn doc(path: &str) -> String {
    super::squash(&super::raw(path))
}

/// Relative link destinations of `text` resolve to files beside `page`.
fn dangling(page: &str, text: &str) -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let base = root.join(page).parent().unwrap().to_owned();
    text.split("](")
        .skip(1)
        .filter_map(|rest| rest.split(')').next())
        .filter(|target| !target.starts_with("http"))
        .map(|target| target.split('#').next().unwrap())
        .filter(|path| !path.is_empty() && !base.join(path).exists())
        .map(str::to_owned)
        .collect()
}

#[test]
fn client_matrix_the_guide_names_every_route_step_tool_and_field() {
    let guide = doc(MATRIX);
    for heading in [
        "### Claude Code",
        "### Codex",
        "### MCP Inspector",
        "### Claude Desktop",
    ] {
        assert!(guide.contains(heading), "{MATRIX} lacks {heading}");
    }
    for needle in [
        "(setup.md#1-start-the-server)",
        "(setup.md#2-claude-code)",
        "(setup.md#3-codex)",
        "(setup.md#where-mail-lands)",
        "headless (`claude -p`)",
        "headless (`codex exec`)",
        "This route needs a person.",
        "`482913`",
        "`matrix.txt`",
        "decoded size `13`",
        "stored as <id>",
        "\"timeout_ms\": 0}`",
        "structuredContent",
        "**Status, 2026-11-22: no receipts yet.**",
        "never says that a client is compatible",
    ] {
        assert!(guide.contains(needle), "{MATRIX} lacks {needle:?}");
    }
    for step in STEPS {
        assert!(
            guide.contains(&format!("| `{step}` |")),
            "{MATRIX} has no row for {step}"
        );
    }
    for tool in TOOLS {
        assert!(guide.contains(tool), "{MATRIX} never names {tool}");
    }
    for client in CLIENTS {
        assert!(
            guide.contains(&format!("`{client}`")),
            "{MATRIX} never names {client}"
        );
    }
    for field in FIELDS {
        assert!(
            guide.contains(&format!("| `{field}` |")),
            "{MATRIX} has no row for {field}"
        );
    }
    let readme = format!("{RECEIPTS}/{README}");
    for page in [MATRIX, readme.as_str()] {
        assert_eq!(
            dangling(page, &super::raw(page)),
            Vec::<String>::new(),
            "{page}"
        );
    }
}

#[test]
fn client_matrix_the_guides_example_receipt_is_well_formed() {
    let raw = super::raw(MATRIX);
    let examples: Vec<&str> = raw
        .split("\n```json\n")
        .skip(1)
        .map(|rest| rest.split("\n```\n").next().unwrap())
        .collect();
    assert_eq!(examples.len(), 1, "{MATRIX} has exactly one json fence");
    let name = "2026-11-25-claude-code.json";
    assert!(raw.contains(&format!("file name `{name}`")));
    receipt::validate(examples[0].as_bytes(), name).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn client_matrix_every_status_page_links_the_matrix_and_dates_the_empty_state() {
    for (page, link) in [
        ("docs/agents/setup.md", "](client-matrix.md)"),
        ("docs/guide/mcp.md", "](../agents/client-matrix.md)"),
        ("docs/evals/README.md", "](../agents/client-matrix.md)"),
        (
            "specs/mcp-version-negotiation/implementation.md",
            "](../../docs/agents/client-matrix.md)",
        ),
        (
            "docs/evals/clients/README.md",
            "](../../agents/client-matrix.md)",
        ),
    ] {
        let text = doc(page);
        assert!(
            text.contains(link),
            "{page} does not link the matrix with {link}"
        );
        assert!(
            text.contains("2026-11-22"),
            "{page} carries no dated status"
        );
        assert!(
            text.contains("no receipts yet") || text.contains("no receipt exists"),
            "{page}"
        );
        assert!(
            !text.contains("Closes #392") && !text.contains("Fixes #392"),
            "{page}"
        );
    }
}

#[test]
fn client_matrix_the_inspector_cli_route_is_documented_with_its_limits() {
    let guide = doc(MATRIX);
    for needle in [
        "#### Headless: the Inspector CLI through the runner",
        "(../decisions/0138-inspector-cli-evidence.md)",
        "`swarmail-evals client-matrix run`",
        "`--cli <observer URL> --transport http`",
        "| `list_tools` | `--method tools/list` |",
        "--tool-name swarmail_get_latest_email --tool-arg count=1 --tool-arg inbox=default",
        "--tool-name swarmail_wait_for_code --tool-arg inbox=default --tool-arg timeout_ms=0",
        "--tool-name swarmail_search_emails --tool-arg inbox=default",
        "--tool-name swarmail_list_attachments --tool-arg id=<stored ID>",
        "npx --offline --no-install",
        "The runner never installs it",
        "npm ls -g @modelcontextprotocol/inspector",
        "A clean exit without the request on the wire fails.",
        "A red draft is published as red",
        "The UI route needs a person",
        "on the operator's approval",
        "**Status, 2026-11-24:** this route has run only against the offline stand-in",
        "`claude --version`",
        "`codex --version`",
        "**About Claude**",
    ] {
        assert!(guide.contains(needle), "{MATRIX} lacks {needle:?}");
    }
    for (page, needle) in [
        ("docs/guide/mcp.md", "remain operator-owned and owed"),
        ("docs/guide/mcp.md", "all four receipts remain owed"),
        (
            "specs/mcp-version-negotiation/implementation.md",
            "## Client matrix runner — 2026-11-24",
        ),
        (
            "specs/mcp-version-negotiation/implementation.md",
            "operator-owned and owed",
        ),
        (
            "evals/README.md",
            "**Status, 2026-11-24: the runner is proven only with an offline stand-in; no client receipt exists.**",
        ),
        ("docs/evals/README.md", "Update 2026-11-24"),
        ("docs/evals/clients/README.md", "client-matrix validate"),
    ] {
        assert!(doc(page).contains(needle), "{page} lacks {needle:?}");
    }
    for page in [
        "docs/guide/mcp.md",
        "evals/README.md",
        "specs/mcp-version-negotiation/implementation.md",
    ] {
        assert_eq!(
            dangling(page, &super::raw(page)),
            Vec::<String>::new(),
            "{page}"
        );
    }
}

#[test]
fn client_matrix_the_runner_delivers_the_handbooks_fixture() {
    let fenced = receipt::fenced_fixture(&super::raw(MATRIX)).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        fenced,
        receipt::FIXTURE,
        "{MATRIX}'s fixture block and evals' receipt::FIXTURE must be the same message"
    );
}
