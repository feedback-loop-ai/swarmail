//! The crate boundary (operator ruling 2026-10-18; decision 0055): evals
//! is its own workspace, outside the published crate, and every package in
//! its lock is already in the root lock at the same version and source, but
//! for the operator-approved dev-dependencies the 2026-10-30 amendment to
//! the decision names.

use std::collections::{BTreeSet, HashSet};
use std::path::Path;

use pulldown_cmark::{Event, Options, Parser, Tag};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

/// (name, version, source) for every `[[package]]` in a Cargo.lock.
fn packages(lock: &str) -> HashSet<(String, String, String)> {
    lock.split("[[package]]")
        .skip(1)
        .map(|block| {
            let field = |k: &str| {
                block
                    .lines()
                    .find_map(|l| l.strip_prefix(&format!("{k} = ")))
                    .map(|v| v.trim_matches('"').to_string())
                    .unwrap_or_default()
            };
            (field("name"), field("version"), field("source"))
        })
        .collect()
}

const CRATES_IO: &str = "registry+https://github.com/rust-lang/crates.io-index";

/// Operator-approved evals dev-dependencies outside the root lock (decision
/// 0055, dated amendment 2026-10-30, #248). Exact triples: a different
/// version, a path or git override, or a third crate all fail the test.
const APPROVED_FOREIGN: [(&str, &str, &str); 2] = [
    ("pulldown-cmark", "0.13.6", CRATES_IO),
    ("unicase", "2.10.0", CRATES_IO),
];

type Package = (String, String, String);

fn approved_foreign() -> BTreeSet<Package> {
    APPROVED_FOREIGN
        .iter()
        .map(|&(name, version, source)| (name.into(), version.into(), source.into()))
        .collect()
}

/// Packages of the evals lock that the root lock does not hold.
fn foreign_packages(ours: &HashSet<Package>, theirs: &HashSet<Package>) -> BTreeSet<Package> {
    ours.iter()
        .filter(|p| p.0 != "swarmail-evals" && !theirs.contains(*p))
        .cloned()
        .collect()
}

#[test]
fn every_evals_package_is_already_in_the_root_lock() {
    let ours = packages(&std::fs::read_to_string(root().join("evals/Cargo.lock")).unwrap());
    let theirs = packages(&std::fs::read_to_string(root().join("Cargo.lock")).unwrap());
    assert_eq!(
        foreign_packages(&ours, &theirs),
        approved_foreign(),
        "packages outside the root lock must be exactly the named 0055 exceptions"
    );
    assert!(APPROVED_FOREIGN.iter().all(|p| p.2 == CRATES_IO));
    for absent in ["native-tls", "openssl", "pulldown-cmark-escape"] {
        assert!(ours.iter().all(|p| p.0 != absent), "{absent} in the lock");
    }
    assert!(ours.iter().any(|p| p.0 == "swarmail-evals"));
    // The parser is a dev-dependency only: nothing outside that section of the
    // manifest, comments aside, may name it.
    let manifest = std::fs::read_to_string(root().join("evals/Cargo.toml")).unwrap();
    assert_eq!(manifest.matches("[dev-dependencies]").count(), 1);
    let (head, dev) = manifest.split_once("[dev-dependencies]").unwrap();
    let names_parser = |text: &str| {
        text.lines()
            .filter(|l| !l.trim_start().starts_with('#'))
            .any(|l| l.contains("pulldown-cmark"))
    };
    assert!(!names_parser(head));
    assert!(dev.lines().any(|l| l.starts_with("pulldown-cmark = ")));
}

#[test]
fn the_foreign_package_filter_reports_everything_but_the_named_exceptions() {
    let package = |name: &str, version: &str, source: &str| -> Package {
        (name.into(), version.into(), source.into())
    };
    let theirs: HashSet<Package> = [package("serde", "1.0.0", CRATES_IO)].into();
    let mut ours: HashSet<Package> = theirs.clone();
    ours.insert(package("swarmail-evals", "0.1.0", ""));
    ours.extend(approved_foreign());
    assert_eq!(foreign_packages(&ours, &theirs), approved_foreign());
    // A made-up package outside the root lock is reported.
    let mut extra = ours.clone();
    extra.insert(package("made-up", "1.0.0", CRATES_IO));
    assert_ne!(foreign_packages(&extra, &theirs), approved_foreign());
    // A listed package at another version or source is reported.
    let mut moved = ours.clone();
    moved.remove(&package("unicase", "2.10.0", CRATES_IO));
    moved.insert(package("unicase", "2.10.1", CRATES_IO));
    assert_ne!(foreign_packages(&moved, &theirs), approved_foreign());
    let mut path = ours.clone();
    path.remove(&package("unicase", "2.10.0", CRATES_IO));
    path.insert(package("unicase", "2.10.0", ""));
    assert_ne!(foreign_packages(&path, &theirs), approved_foreign());
    // A listed package missing from the lock, or already in the root lock,
    // leaves the set unequal to the list: a stale allowance fails too.
    let mut missing = ours.clone();
    missing.remove(&package("unicase", "2.10.0", CRATES_IO));
    assert_ne!(foreign_packages(&missing, &theirs), approved_foreign());
    let mut rooted = theirs.clone();
    rooted.insert(package("unicase", "2.10.0", CRATES_IO));
    assert_ne!(foreign_packages(&ours, &rooted), approved_foreign());
}

#[test]
fn evals_is_its_own_unpublished_workspace_and_the_root_does_not_package_it() {
    let manifest = std::fs::read_to_string(root().join("evals/Cargo.toml")).unwrap();
    assert!(manifest.lines().any(|l| l.trim() == "[workspace]"));
    assert!(manifest.contains("publish = false"));
    let root_manifest = std::fs::read_to_string(root().join("Cargo.toml")).unwrap();
    assert!(
        !root_manifest.contains("evals"),
        "the root manifest names nothing of evals"
    );
}

#[test]
fn sequential_contracts_and_decision_links_are_honest() {
    for relative in [
        "evals/README.md",
        "evals/run-plan.md",
        "specs/agent-eval-harness/spec.md",
        "docs/decisions/0055-agent-eval-harness.md",
    ] {
        let path = root().join(relative);
        let text = std::fs::read_to_string(&path).unwrap();
        for phrase in [
            "baseline runs sequentially",
            "fresh HOME",
            "concurrency",
            "Concurrent trials are not isolated",
            "same-uid",
            "shared TMPDIR",
            "do-nothing agent passes",
            "MCP URL, app URL, identity and inbox context",
            "#201",
            "#221",
        ] {
            assert!(text.contains(phrase), "{relative}: missing {phrase}");
        }
        // Check bounded local Markdown links; anchors and external links need no filesystem lookup.
        for suffix in text.split("](").skip(1) {
            let target = suffix.split(')').next().unwrap().split('#').next().unwrap();
            if !target.is_empty() && !target.contains("://") {
                assert!(
                    path.parent().unwrap().join(target).exists(),
                    "{relative}: broken link {target}"
                );
            }
        }
    }
    let spec = std::fs::read_to_string(root().join("specs/agent-eval-harness/spec.md")).unwrap();
    assert!(spec.contains("AC4 — Positive control"));
    assert!(!spec.contains("Two simultaneously active isolation trials"));
    let index = std::fs::read_to_string(root().join("docs/decisions/README.md")).unwrap();
    let rows: Vec<_> = index
        .lines()
        .filter(|line| line.starts_with("| [0076]"))
        .collect();
    assert_eq!(rows.len(), 1);
    // The supplied 2026-10-26 operator rulings accepted 0063 and 0076.
    assert!(rows[0].contains("accepted (operator, 2026-10-26)"));
    let decision =
        std::fs::read_to_string(root().join("docs/decisions/0076-sequential-eval-trials.md"))
            .unwrap();
    assert!(decision.contains("**Status:** accepted (operator, 2026-10-26)"));
    assert!(index.lines().any(
        |line| line.starts_with("| [0063]") && line.contains("accepted (operator, 2026-10-26)")
    ));
    assert!(
        index
            .lines()
            .any(|line| line.starts_with("| [0071]") && line.contains("accepted"))
    );
    assert_eq!(
        index
            .lines()
            .filter(|line| line.starts_with("| [0077]"))
            .count(),
        1
    );
    let plan = std::fs::read_to_string(root().join("specs/agent-eval-harness/plan.md")).unwrap();
    assert!(!plan.contains("two concurrently active isolation trials"));
}

// The parser owns CommonMark: blocks, containers, code, inlines, escapes and
// character references. Definitions need their own walk because a definition
// nothing uses emits no event, and the guard refuses those too (spec
// commonmark-parser-guard, ruling 1).
fn markdown_destinations(text: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    let mut events = Parser::new_ext(text, Options::empty()).into_offset_iter();
    for (event, range) in events.by_ref() {
        if let Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) = event {
            found.push((range.start, dest_url.into_string()));
        }
    }
    for (_, definition) in events.reference_definitions().iter() {
        found.push((definition.span.start, definition.dest.to_string()));
    }
    found.sort();
    found
}

// `target` is the parser's already-decoded destination: no second decode here.
fn packet_destination(document: &Path, target: &str) -> Result<bool, String> {
    let path = target;
    let scheme = path.split_once(':').is_some_and(|(prefix, _)| {
        prefix
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphabetic)
            && prefix
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"+.-".contains(&b))
    });
    if scheme || path.starts_with("//") {
        return Ok(false);
    }
    let path = path.split(['#', '?']).next().unwrap();
    if path.is_empty() {
        return Ok(false);
    }
    let mut decoded = Vec::new();
    let mut bytes = path.as_bytes().iter().copied().peekable();
    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            let mut pair = bytes.clone();
            if let (Some(a), Some(b)) = (pair.next(), pair.next())
                && let (Some(a), Some(b)) = ((a as char).to_digit(16), (b as char).to_digit(16))
            {
                decoded.push((a * 16 + b) as u8);
                bytes = pair;
                continue;
            }
        }
        decoded.push(byte);
    }
    let path = std::str::from_utf8(&decoded).map_err(|e| format!("invalid path encoding: {e}"))?;
    let mut components = Vec::new();
    if !path.starts_with('/') {
        components.extend(
            document
                .parent()
                .unwrap()
                .iter()
                .map(|c| c.to_str().unwrap()),
        );
    }
    for component in path.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                // Clamp at the repository root: over-parented historical links
                // must not escape the convention check by spelling extra ../.
                components.pop();
            }
            component => components.push(component),
        }
    }
    Ok(components.first() == Some(&".forge"))
}

fn git(root: &Path) -> std::process::Command {
    let mut command = std::process::Command::new("git");
    command
        .arg("-C")
        .arg(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE");
    command
}

fn markdown_inventory(bytes: &[u8]) -> Result<Vec<&Path>, String> {
    if !bytes.is_empty() && bytes.last() != Some(&0) {
        return Err("Git inventory is not NUL-terminated".into());
    }
    let mut paths = Vec::new();
    for raw in bytes.split(|&b| b == 0).filter(|s| !s.is_empty()) {
        let name = std::str::from_utf8(raw).map_err(|e| format!("Git path encoding: {e}"))?;
        let path = Path::new(name);
        if path.is_absolute()
            || path
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err(format!("invalid Git inventory path: {name}"));
        }
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("md"))
        {
            paths.push(path);
        }
    }
    if paths.is_empty() {
        return Err("Git inventory contains no tracked Markdown".into());
    }
    Ok(paths)
}

fn check_tracked_markdown(root: &Path) -> Result<(), String> {
    let top = git(root)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("Git discovery failed: {e}"))?;
    if !top.status.success() {
        return Err("Git discovery failed".into());
    }
    let top = std::str::from_utf8(&top.stdout).map_err(|e| format!("Git root encoding: {e}"))?;
    if Path::new(top.trim_end())
        .canonicalize()
        .map_err(|e| e.to_string())?
        != root.canonicalize().map_err(|e| e.to_string())?
    {
        return Err("Git discovery resolved a different repository root".into());
    }
    let inventory = git(root)
        .args(["ls-files", "--cached", "-z"])
        .output()
        .map_err(|e| format!("Git inventory failed: {e}"))?;
    if !inventory.status.success() {
        return Err("Git inventory failed".into());
    }
    let mut violations = Vec::new();
    for relative in markdown_inventory(&inventory.stdout)? {
        let text = std::fs::read_to_string(root.join(relative))
            .map_err(|e| format!("{}: document read failed: {e}", relative.display()))?;
        for (offset, target) in markdown_destinations(&text) {
            let context = format!(
                "{}:{}: {target}",
                relative.display(),
                text.as_bytes()[..offset]
                    .iter()
                    .filter(|&&b| b == b'\n')
                    .count()
                    + 1
            );
            if packet_destination(relative, &target).map_err(|e| format!("{context}: {e}"))? {
                violations.push(format!("{context}: tracked Markdown links into .forge/"));
            }
        }
    }
    violations.sort();
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations.join("\n"))
    }
}

#[test]
fn tracked_markdown_never_links_into_delivery_state() {
    check_tracked_markdown(root()).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn markdown_scanner_covers_the_documented_grammar() {
    // (source, decoded destination, how many places name it): a used reference
    // is two places to fix, its link and its definition.
    for (source, target, places) in [
        (r#"[ruling](../.forge/packet.md)"#, "../.forge/packet.md", 1),
        (
            r#"![ruling](<../.forge/packet.md> 'title')"#,
            "../.forge/packet.md",
            1,
        ),
        (
            r#"[nested [label]](../.forge/(packet).md "title")"#,
            "../.forge/(packet).md",
            1,
        ),
        (
            r#"[escaped\] label](../.forge/packet\(1\).md "title")"#,
            "../.forge/packet(1).md",
            1,
        ),
        (
            "[label](../.forge/(packet).md\n (a title))",
            "../.forge/(packet).md",
            1,
        ),
        (
            "[full][r]\n\n[r]: <../.forge/packet.md> \"title\"",
            "../.forge/packet.md",
            2,
        ),
        (
            "[collapsed][]\n\n[collapsed]: ../.forge/packet.md",
            "../.forge/packet.md",
            2,
        ),
        (
            "[shortcut]\n\n   [shortcut]:\n  ../.forge/packet.md",
            "../.forge/packet.md",
            2,
        ),
        (
            "[missing](../.forge/nonexistent.md)",
            "../.forge/nonexistent.md",
            1,
        ),
    ] {
        let found = markdown_destinations(source);
        assert_eq!(found.len(), places, "{source}: {found:?}");
        assert!(
            found.iter().all(|(_, d)| d == target),
            "{source}: {found:?}"
        );
    }
    for source in [
        "ordinary ../.forge/packet.md prose",
        "`[example](../.forge/packet.md)`",
        "`` `[example](../.forge/packet.md)` ``",
        "`multiline\n[example](../.forge/packet.md)`",
        "```markdown\n[example](../.forge/packet.md)\n```",
        "~~~\n[r]: ../.forge/packet.md\n~~~",
        "   ````md\n```\n[example](../.forge/packet.md)\n````",
        "\\[escaped](../.forge/packet.md)",
        "[unclosed](<../.forge/packet.md)",
        "[unbalanced](../.forge/(packet.md)",
        "[invalid](../.forge/packet.md adjacent)",
        "[invalid](../.forge/packet.md \"unclosed)",
    ] {
        assert!(markdown_destinations(source).is_empty(), "{source}");
    }
    let source = "é `ignored` [ok](../docs/ruling.md)\n~~~\n[bad](../.forge/x)\n~~~\n[ok](#anchor)";
    let found = markdown_destinations(source);
    assert_eq!(
        found.iter().map(|(_, s)| s.as_str()).collect::<Vec<_>>(),
        ["../docs/ruling.md", "#anchor"]
    );
    // Offsets are the start of the construct, here after a multibyte prefix.
    assert!(source[found[0].0..].starts_with("[ok](../docs"));
    assert!(source[found[1].0..].starts_with("[ok](#anchor)"));
}

#[test]
fn packet_paths_are_classified_without_filesystem_access() {
    let document = Path::new("specs/deep/spec.md");
    for target in [
        "../../.forge",
        "../../.forge/packet.md",
        ".././../.forge/packet.md?raw=1#row",
        "/.forge/packet.md",
        "/docs/../.forge/x",
        "../../../.forge/x",
        "../../../../.forge/x",
        "../../%2eforge/packet.md",
        "../../%2Eforge%2Fpacket.md",
        "..%2f..%2f.forge/x",
        "../../.forge/packet%20with%20spaces.md",
    ] {
        assert!(packet_destination(document, target).unwrap(), "{target}");
    }
    for target in [
        "",
        "#anchor",
        "?query",
        "../../docs/rulings/epic-192.md",
        "../.forge/nested.md",
        "../../.forge-other/x",
        "../../other.forge/x",
        "../../.forge/../docs/ruling.md",
        "https://example.test/.forge/x",
        "mailto:operator@example.test",
        "urn:example:.forge/x",
        "//example.test/.forge/x",
        "../../%252eforge/x",
        "../../%not-an-escape/x",
        // The parser decodes backslashes; this layer does not decode twice.
        r"../../\.forge/x",
    ] {
        assert!(!packet_destination(document, target).unwrap(), "{target}");
    }
    assert!(
        packet_destination(document, "../../%ff/x")
            .unwrap_err()
            .contains("encoding")
    );
}

fn fixture_git(root: &Path, args: &[&str]) {
    let output = git(root).args(args).output().unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn tracked_inventory_refuses_existing_packets_and_recovers_after_repair() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture_git(root, &["init", "--quiet"]);
    std::fs::create_dir_all(root.join(".hidden")).unwrap();
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::create_dir_all(root.join(".forge/packets")).unwrap();
    std::fs::write(root.join(".gitignore"), ".forge/\n").unwrap();
    std::fs::write(root.join("docs/ruling.md"), "Tracked authority\n").unwrap();
    std::fs::write(root.join(".forge/packets/ruling.md"), "Ignored evidence\n").unwrap();
    let name = ".hidden/note with spaces.MD";
    let doc = root.join(name);
    std::fs::write(&doc, "[allowed](../docs/ruling.md)").unwrap();
    fixture_git(root, &["add", "--", ".gitignore", "docs/ruling.md", name]);
    // An untracked Markdown violation cannot affect the tracked inventory.
    std::fs::write(root.join("untracked.md"), "[bad](.forge/packets/ruling.md)").unwrap();
    check_tracked_markdown(root).unwrap();
    for (source, target) in [
        (
            "[bad](../.forge/packets/ruling.md)",
            "../.forge/packets/ruling.md",
        ),
        (
            "![bad](<../.forge/packets/ruling.md> 'title')",
            "../.forge/packets/ruling.md",
        ),
        (
            "[bad][packet]\n\n[packet]: ../.forge/packets/ruling.md",
            "../.forge/packets/ruling.md",
        ),
        (
            "[bad](../docs/../.forge/packets/ruling.md)",
            "../docs/../.forge/packets/ruling.md",
        ),
        (
            "[bad](/.forge/packets/ruling.md)",
            "/.forge/packets/ruling.md",
        ),
        (
            "[bad](../../../.forge/packets/ruling.md)",
            "../../../.forge/packets/ruling.md",
        ),
        (
            "[bad](../%2eforge%2fpackets/ruling.md)",
            "../%2eforge%2fpackets/ruling.md",
        ),
        (
            "[bad](../.forge/packets/missing.md)",
            "../.forge/packets/missing.md",
        ),
    ] {
        std::fs::write(&doc, source).unwrap();
        let error = check_tracked_markdown(root).unwrap_err();
        assert!(
            error.contains(name) && error.contains(target) && error.contains("links into .forge/"),
            "{error}"
        );
        std::fs::write(&doc, "[repaired](../docs/ruling.md)").unwrap();
        check_tracked_markdown(root).unwrap();
    }
    std::fs::remove_file(&doc).unwrap();
    assert!(
        check_tracked_markdown(root)
            .unwrap_err()
            .contains("document read failed")
    );
    std::fs::write(&doc, [0xff]).unwrap();
    assert!(
        check_tracked_markdown(root)
            .unwrap_err()
            .contains("document read failed")
    );
    std::fs::write(&doc, "[bad](../%ff)").unwrap();
    let error = check_tracked_markdown(root).unwrap_err();
    assert!(
        error.contains(name) && error.contains("../%ff") && error.contains("encoding"),
        "{error}"
    );
    std::fs::write(root.join(".git/index"), "corrupt").unwrap();
    assert!(
        check_tracked_markdown(root)
            .unwrap_err()
            .contains("Git inventory failed")
    );
}

#[test]
fn inventory_and_git_discovery_fail_closed() {
    for bytes in [
        b"".as_slice(),
        b"plain.txt\0",
        b"docs/ruling.md",
        b"../ruling.md\0",
        b"/ruling.md\0",
        b"bad\xff.md\0",
    ] {
        assert!(markdown_inventory(bytes).is_err(), "{bytes:?}");
    }
    assert_eq!(
        markdown_inventory(b".hidden/note with spaces.MD\0docs/ruling.md\0other.txt\0").unwrap(),
        [
            Path::new(".hidden/note with spaces.MD"),
            Path::new("docs/ruling.md")
        ]
    );
    let temp = tempfile::tempdir().unwrap();
    assert!(
        check_tracked_markdown(temp.path())
            .unwrap_err()
            .contains("Git discovery failed")
    );
    fixture_git(temp.path(), &["init", "--quiet"]);
    assert!(
        check_tracked_markdown(temp.path())
            .unwrap_err()
            .contains("no tracked Markdown")
    );
    std::fs::create_dir(temp.path().join("subdirectory")).unwrap();
    assert!(
        check_tracked_markdown(&temp.path().join("subdirectory"))
            .unwrap_err()
            .contains("different repository root")
    );
}

// Linux only: macOS file systems (APFS, HFS+) refuse a non-UTF-8 file name
// with EILSEQ, so such a tracked name cannot be created there.
#[cfg(target_os = "linux")]
#[test]
fn non_utf8_tracked_names_refuse_instead_of_disappearing() {
    use std::os::unix::ffi::OsStringExt;
    let temp = tempfile::tempdir().unwrap();
    fixture_git(temp.path(), &["init", "--quiet"]);
    let name = std::ffi::OsString::from_vec(b"bad\xff.md".to_vec());
    std::fs::write(temp.path().join(&name), "# document").unwrap();
    let status = git(temp.path())
        .args(["add", "--"])
        .arg(&name)
        .status()
        .unwrap();
    assert!(status.success());
    assert!(
        check_tracked_markdown(temp.path())
            .unwrap_err()
            .contains("Git path encoding")
    );
}

// Each case is a real indexed document; classification must not depend on
// whether the delivery-state destination exists in the fixture repository.
// A refused case is (source, the parser's decoded destination, the line the
// construct starts on). Repair is spelling-agnostic: every `forge` becomes
// `rulings`, whatever the source spells (`.forge`, `&period;forge`, ...).
fn tracked_cases(refused: &[(&str, &str, usize)], allowed: &[&str]) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture_git(root, &["init", "--quiet"]);
    std::fs::create_dir(root.join("docs")).unwrap();
    let name = "docs/é note.MD";
    let document = root.join(name);
    for &(source, target, line) in refused {
        std::fs::write(&document, source).unwrap();
        fixture_git(root, &["add", "--", name]);
        let error = check_tracked_markdown(root).expect_err(&format!(
            "tracked destination escaped the guard: {source:?}"
        ));
        assert!(
            error.contains(&format!(
                "{name}:{line}: {target}: tracked Markdown links into .forge/"
            )),
            "{source:?}: {error}"
        );
        let repaired = source.replace("forge", "rulings");
        std::fs::write(&document, &repaired).unwrap();
        fixture_git(root, &["add", "--", name]);
        check_tracked_markdown(root).unwrap_or_else(|e| panic!("repaired {repaired:?}: {e}"));
    }
    for source in allowed {
        std::fs::write(&document, source).unwrap();
        fixture_git(root, &["add", "--", name]);
        check_tracked_markdown(root).unwrap_or_else(|e| panic!("{source:?}: {e}"));
    }
}

#[test]
fn tracked_code_spans_respect_blocks() {
    tracked_cases(
        &[
            (
                "`unmatched\n\n[bad](../.forge/p.md)\n`",
                "../.forge/p.md",
                3,
            ),
            (
                "`unmatched\n \t \n[bad](../.forge/p.md)\n`",
                "../.forge/p.md",
                3,
            ),
            (
                "`unmatched\r\n  \r\n[bad](../.forge/p.md)\r\n`",
                "../.forge/p.md",
                3,
            ),
            (
                "`unmatched\n# [bad](../.forge/p.md)\n`",
                "../.forge/p.md",
                2,
            ),
            (
                "`unmatched\n---\n[bad](../.forge/p.md)\n`",
                "../.forge/p.md",
                3,
            ),
            (
                "> `unmatched\n\n[bad](../.forge/p.md)\n`",
                "../.forge/p.md",
                3,
            ),
            (
                "- `unmatched\n- [bad](../.forge/p.md)\n  `",
                "../.forge/p.md",
                2,
            ),
            (
                "`unmatched\n> [bad](../.forge/p.md)\n> `",
                "../.forge/p.md",
                2,
            ),
            (
                "[broken](<../rulings/`\n\n[bad](../.forge/p.md)\n`",
                "../.forge/p.md",
                3,
            ),
            ("``wrong size [bad](../.forge/p.md)`", "../.forge/p.md", 1),
        ],
        &[
            "`[example](../.forge/p.md)`",
            "`` `[example](../.forge/p.md)` ``",
            "`soft break\n[example](../.forge/p.md)`",
            "> `soft break\n> [example](../.forge/p.md)`",
            "- `soft break\n  [example](../.forge/p.md)`",
            "# `[example](../.forge/p.md)`",
            "`literal unmatched [ok](../rulings/p.md)",
            "\\`[ok](../rulings/p.md)`",
        ],
    );
}

#[test]
fn tracked_destinations_own_backticks() {
    tracked_cases(
        &[
            (
                "[bad](../.forge/`p`) [ok](../rulings/`q`)",
                "../.forge/`p`",
                1,
            ),
            ("[bad](<../.forge/`p`> \"`title`\")", "../.forge/`p`", 1),
            ("[r]: ../.forge/`p`", "../.forge/`p`", 1),
            ("[r]: <../.forge/`p`> '`title`'", "../.forge/`p`", 1),
            (
                "[ok](../rulings/`p) [bad](../.forge/q`)`",
                "../.forge/q`",
                1,
            ),
            (
                "[ok](../rulings/p \"`title\") [bad](../.forge/q)`",
                "../.forge/q",
                1,
            ),
        ],
        &[
            "`[example](../.forge/`p`)`", // equal runs close before p: no complete link
            "``[example](../.forge/`p`)``",
            "[ok](../rulings/p \"[example](../.forge/q)\")",
            "[ok](<../rulings/`p`>)",
        ],
    );
}

#[test]
fn tracked_container_definitions_are_checked() {
    tracked_cases(
        &[
            ("> [r]: ../.forge/p.md", "../.forge/p.md", 1),
            // R1/G1/F1 (#245 review): `2.` is a sibling item of an existing
            // ordered list, so it is a list item whatever the paragraph says.
            ("1. a\n2. [r]: ../.forge/p.md", "../.forge/p.md", 2),
            ("1. a\n2. [bad](../.forge/p.md)", "../.forge/p.md", 2),
            ("- [r]: ../.forge/p.md", "../.forge/p.md", 1),
            ("+ [r]: <../.forge/p.md>", "../.forge/p.md", 1),
            ("* [r]: ../.forge/p.md", "../.forge/p.md", 1),
            ("1. [r]: ../.forge/p.md", "../.forge/p.md", 1),
            ("12) [r]: ../.forge/p.md", "../.forge/p.md", 1),
            ("> - 1. > [r]: ../.forge/p.md", "../.forge/p.md", 1),
            ("> [r]:\n>   ../.forge/p.md", "../.forge/p.md", 1),
            ("- [r]:\n    ../.forge/p.md", "../.forge/p.md", 1),
            ("1. [r]:\n   ../.forge/p.md", "../.forge/p.md", 1),
            ("- intro\n\n  [r]: ../.forge/p.md", "../.forge/p.md", 3),
            (
                "> - é\n>\n>   [r]:\n>     <../.forge/p.md>",
                "../.forge/p.md",
                3,
            ),
            (">\t[r]: ../.forge/p.md", "../.forge/p.md", 1),
            ("-\t[r]: ../.forge/p.md", "../.forge/p.md", 1),
            // Ruling 7: the line after a definition that is not part of it is
            // paragraph text, so a link on it is a real link.
            (
                "> [ok]: ../rulings/p.md\n>     [example](../.forge/p.md)",
                "../.forge/p.md",
                2,
            ),
            (
                "- [ok]: ../rulings/p.md\n      [example](../.forge/p.md)",
                "../.forge/p.md",
                2,
            ),
            // The quoted twin of the `2.` pin stays refused: pulldown-cmark
            // closes a quoted definition-only paragraph at the unquoted line,
            // so `2.` opens a list there. Fail-closed, and the parser's reading.
            (
                "> [ok]: ../rulings/p.md\n2. [r]: ../.forge/p.md",
                "../.forge/p.md",
                2,
            ),
        ],
        &[
            "> ```md\n> [r]: ../.forge/p.md\n> ```",
            "- ~~~\n  [r]: ../.forge/p.md\n  ~~~",
            "> - ```\n>   [r]: ../.forge/p.md\n>   ```",
            ">     [r]: ../.forge/p.md",
            "-     [r]: ../.forge/p.md",
            "1.     [r]: ../.forge/p.md",
            "- intro\n\n      [r]: ../.forge/p.md",
            "    [r]: ../.forge/p.md",
            "\t[r]: ../.forge/p.md",
            "prose > [r]: ../.forge/p.md",
            "-prose [r]: ../.forge/p.md",
            "1.prose [r]: ../.forge/p.md",
            "1234567890. [r]: ../.forge/p.md",
            "> prose\n> [r]: ../.forge/p.md", // definition cannot interrupt a paragraph
            "- prose\n  [r]: ../.forge/p.md",
            "> prose\n[r]: ../.forge/p.md", // lazy continuation is still prose
            // Ruling 7: `2.` cannot interrupt the paragraph the definition
            // was taken from, so `[r]:` is text.
            "[ok]: ../rulings/p.md\n2. [r]: ../.forge/p.md",
        ],
    );
}

#[test]
fn tracked_character_references_are_decoded_once() {
    tracked_cases(
        &[
            ("[bad](../&#46;forge/p.md)", "../.forge/p.md", 1),
            ("[bad](../&#x2e;forge&#x2F;p.md)", "../.forge/p.md", 1),
            ("[bad](../&period;forge&sol;p.md)", "../.forge/p.md", 1),
            ("[r]: ../&#X2E;forge/p.md", "../.forge/p.md", 1),
            ("> [r]: ../&period;forge/p.md", "../.forge/p.md", 1),
            ("[bad](../&percnt;2eforge/p.md)", "../%2eforge/p.md", 1),
            (
                "[bad](../.forge/p.md&quest;raw=1&num;row)",
                "../.forge/p.md?raw=1#row",
                1,
            ),
            ("[bad](../&#0000046;forge/p.md)", "../.forge/p.md", 1),
            ("[bad](../&#x00002e;forge/p.md)", "../.forge/p.md", 1),
        ],
        &[
            "[ok](../&period;rulings/p.md)",
            "[ok](https&colon;//example.test/.forge/p.md)",
            "[ok](&sol;&sol;example.test/.forge/p.md)",
            "[ok](&num;../.forge/p.md)",
            "[ok](&quest;../.forge/p.md)",
            "[ok](../\\&period;forge/p.md)",
            "[ok](../&amp;period;forge/p.md)",
            "[ok](../&bsol;.forge/p.md)",
            "[ok](../&PERIOD;forge/p.md)",
            "[ok](../&periodforge/p.md)",
            "[ok](../&#00000046;forge/p.md)",
            "[ok](../&#x000002e;forge/p.md)",
            "[ok](../&#46forge/p.md)",
            "[ok](../&#x2eforge/p.md)",
            "[ok](../&percnt;252eforge/p.md)",
        ],
    );
}

#[test]
fn markdown_parser_decodes_the_pinned_entity_table() {
    // The parser owns decoding, so the guard's verdict depends on its entity
    // table. Hold it to the pinned WHATWG snapshot, including multi-scalar
    // mappings and the longest names, so a parser bump that changes decoding
    // fails here instead of silently moving the guard.
    let snapshot: serde_json::Value =
        serde_json::from_str(include_str!("../../scripts/data/html-entities.json")).unwrap();
    let mut count = 0;
    for (name, entry) in snapshot.as_object().unwrap() {
        if name.ends_with(';') {
            let found = markdown_destinations(&format!("[x](<{name}>)"));
            assert_eq!(found.len(), 1, "{name}: {found:?}");
            assert_eq!(found[0].1, entry["characters"].as_str().unwrap(), "{name}");
            count += 1;
        }
    }
    assert_eq!(count, 2125);
    // The percent-encoding error sits behind the parser's decode.
    let found = markdown_destinations("[x](../&percnt;ff)");
    assert_eq!(found[0].1, "../%ff");
    assert!(
        packet_destination(Path::new("docs/note.md"), &found[0].1)
            .unwrap_err()
            .contains("encoding")
    );
}

#[test]
fn tracked_combined_contexts_keep_original_source_locations() {
    tracked_cases(
        &[
            (
                "> - é\r\n>\r\n>   [r]:\r\n>     <../&period;forge/`p`>\r\n",
                "../.forge/`p`",
                3,
            ),
            (
                "> - é [bad](<../&#46;forge/`p`> 'λ `title`')",
                "../.forge/`p`",
                1,
            ),
            (
                "- intro\n  > [r]:\n  > ../&#46;forge/p.md",
                "../.forge/p.md",
                2,
            ),
            ("\\``unmatched [bad](../.forge/p.md)", "../.forge/p.md", 1),
            (
                "[unfinished\n\n[label](../.forge/p.md)",
                "../.forge/p.md",
                3,
            ),
            (
                "[bad](../.forge/p.md \"unclosed)\n\n[real](../.forge/q.md)",
                "../.forge/q.md",
                3,
            ),
        ],
        &[
            "> - ``[example](../&period;forge/`p`)``",
            "> - ~~~\r\n>   [r]: ../&period;forge/p.md\r\n>   ~~~",
            "> -     [r]: ../&period;forge/p.md",
            "> [r]: ../.forge/p.md trailing prose",
            "> [r]: ../.forge/p.md \"title\" trailing prose",
            "> [r]: ../rulings/p.md\n>   \"title [example](../.forge/p.md)\"",
            "\\``[example](../.forge/p.md)`",
            "[label\n\n](../.forge/p.md)",
            "before\n2. [r]: ../.forge/p.md", // ordered list cannot interrupt at 2
            "> - prose\n>   `soft\n>   [example](../.forge/p.md)`",
        ],
    );
    // No timeout assertion: a growing set of unmatched delimiter widths proves
    // that the parser advances without treating them as document-wide spans.
    let mut source = String::new();
    for width in 1..256 {
        source.push_str(&"`".repeat(width));
        source.push_str(" unmatched ");
    }
    source.push_str("\n\n[bad](../.forge/p.md)");
    tracked_cases(&[(&source, "../.forge/p.md", 3)], &[]);
}

#[test]
fn tracked_review_residuals_follow_the_parser() {
    tracked_cases(
        &[
            // #245 review, security residual (`:502`): the invalid bare-line
            // definition is text, so the real link after it is the violation
            // and the rest of the line is not skipped.
            (
                "[r]: ../.forge/p.md garbage [bad](../.forge/q.md)",
                "../.forge/q.md",
                1,
            ),
            (
                "> [r]: ../.forge/p.md garbage [bad](../.forge/q.md)",
                "../.forge/q.md",
                1,
            ),
            (
                "- [r]: ../.forge/p.md garbage [bad](../.forge/q.md)",
                "../.forge/q.md",
                1,
            ),
            // R2 (#245 review): a setext underline or rule after a definition
            // does not unmake it, nor the link it underlines.
            ("[r]: ../.forge/p.md\n---", "../.forge/p.md", 1),
            ("[r]: ../.forge/p.md\n===", "../.forge/p.md", 1),
            ("[r]:\n  ../.forge/p.md\n---", "../.forge/p.md", 1),
            ("[bad](../.forge/p.md)\n---", "../.forge/p.md", 1),
            ("[bad](../.forge/p.md)\n===", "../.forge/p.md", 1),
            // Links cannot contain links: the inner one is the link.
            ("[a [b](../.forge/in)](../rulings/out)", "../.forge/in", 1),
        ],
        &[
            // Spec ruling 6: four non-link forms the hand scanner refused are
            // text to CommonMark, released on record. A separate plain-text
            // lint is the operator's call, not parser work.
            "[r]: ../.forge/p.md trailing",
            "para\n[r]: ../.forge/p.md",
            // A shadowed duplicate: the parser keeps the first definition only.
            "[r]: ../rulings/p.md\n[r]: ../.forge/p.md",
            "<div>\n[bad](../.forge/p.md)\n</div>",
            "<!-- [bad](../.forge/p.md) -->",
            // The outer pair around a nested link is text.
            "[a [b](../rulings/in)](../.forge/out)",
        ],
    );
}

fn tracked_error(source: &str) -> String {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture_git(root, &["init", "--quiet"]);
    std::fs::write(root.join("é.md"), source).unwrap();
    fixture_git(root, &["add", "--", "é.md"]);
    check_tracked_markdown(root).unwrap_err()
}

#[test]
fn violations_report_the_construct_start_line_and_the_decoded_destination() {
    let refusal = |line: usize, target: &str| {
        format!("é.md:{line}: {target}: tracked Markdown links into .forge/")
    };
    // A multi-line definition reports the line of `[r]:`, not of its destination.
    assert_eq!(
        tracked_error("intro\n\n[r]:\n  ../.forge/p.md\n"),
        refusal(3, "../.forge/p.md")
    );
    assert_eq!(
        tracked_error("a\r\nb\r\n[bad](../.forge/p.md)"),
        refusal(3, "../.forge/p.md")
    );
    assert_eq!(
        tracked_error("ééé λ [bad](../.forge/p.md)"),
        refusal(1, "../.forge/p.md")
    );
    assert_eq!(
        tracked_error("λ\n\nééé λ [bad](../.forge/p.md)"),
        refusal(3, "../.forge/p.md")
    );
    // The decoded spelling, not the raw one.
    assert_eq!(
        tracked_error("[bad](../&period;forge/p.md)"),
        refusal(1, "../.forge/p.md")
    );
    // A used reference is two places to fix; the output is sorted and stable.
    assert_eq!(
        tracked_error("[bad][r]\n\n[r]: ../.forge/p.md"),
        format!(
            "{}\n{}",
            refusal(1, "../.forge/p.md"),
            refusal(3, "../.forge/p.md")
        )
    );
    assert_eq!(
        tracked_error("[a](../.forge/z.md)\n\n[b](../.forge/a.md)"),
        format!(
            "{}\n{}",
            refusal(1, "../.forge/z.md"),
            refusal(3, "../.forge/a.md")
        )
    );
}

#[test]
fn hostile_documents_terminate_and_classify() {
    // No wall-clock assertion: each document must simply be classified.
    let long_line = format!("{}\n\n[bad](../.forge/p.md)", "x".repeat(1 << 20));
    let brackets = format!("{}[bad](../.forge/p.md)", "[".repeat(10_000));
    tracked_cases(
        &[
            ("[a\0b](../.forge/p.md)", "../.forge/p.md", 1),
            // The guard counts `\n` only: lone-CR documents report line 1.
            ("[bad](../.forge/p.md)\r", "../.forge/p.md", 1),
            ("a\rb\r[bad](../.forge/p.md)", "../.forge/p.md", 1),
            (&long_line, "../.forge/p.md", 3),
            (&brackets, "../.forge/p.md", 1),
        ],
        &[],
    );
}
