//! The crate boundary (operator ruling 2026-10-18; decision 0055): evals
//! is its own workspace, outside the published crate, and every package in
//! its lock is already in the root lock at the same version and source.

use std::collections::HashSet;
use std::path::Path;

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

#[test]
fn every_evals_package_is_already_in_the_root_lock() {
    let ours = packages(&std::fs::read_to_string(root().join("evals/Cargo.lock")).unwrap());
    let theirs = packages(&std::fs::read_to_string(root().join("Cargo.lock")).unwrap());
    let foreign: Vec<_> = ours
        .iter()
        .filter(|p| p.0 != "swarmail-evals" && !theirs.contains(*p))
        .collect();
    assert!(
        foreign.is_empty(),
        "packages outside the root lock: {foreign:?}"
    );
    assert!(ours.iter().all(|p| p.0 != "native-tls" && p.0 != "openssl"));
    assert!(ours.iter().any(|p| p.0 == "swarmail-evals"));
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
    assert!(rows[0].contains("proposed"));
    let decision =
        std::fs::read_to_string(root().join("docs/decisions/0076-sequential-eval-trials.md"))
            .unwrap();
    assert!(decision.contains("**Status:** proposed"));
    assert!(
        index
            .lines()
            .any(|line| line.starts_with("| [0063]") && line.contains("proposed"))
    );
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

// Bounded Markdown convention check: inline links/images and reference
// definitions, angle destinations, titles and escaped/balanced delimiters.
// Code spans and backtick/tilde fences are masked. HTML and arbitrary renderer
// extensions are outside this grammar. Keep it test-only, with no packet I/O.
fn without_code(text: &str) -> Vec<u8> {
    let mut bytes = text.as_bytes().to_vec();
    let mut fence = None;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let raw = line.as_bytes();
        let indent = raw.iter().take_while(|&&b| b == b' ').count();
        let rest = &raw[indent..];
        let marker = rest.first().copied().unwrap_or_default();
        let width = rest.iter().take_while(|&&b| b == marker).count();
        let delimiter = indent <= 3 && matches!(marker, b'`' | b'~') && width >= 3;
        let was_fenced = fence.is_some();
        if let Some((opening, size)) = fence {
            if delimiter
                && marker == opening
                && width >= size
                && rest[width..].iter().all(u8::is_ascii_whitespace)
            {
                fence = None;
            }
        } else if delimiter {
            fence = Some((marker, width));
        }
        if was_fenced || delimiter {
            for byte in &mut bytes[offset..offset + raw.len()] {
                if *byte != b'\n' {
                    *byte = b' ';
                }
            }
        }
        offset += raw.len();
    }
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
        } else if bytes[i] == b'`' {
            let size = bytes[i..].iter().take_while(|&&b| b == b'`').count();
            let mut end = i + size;
            let mut closing = None;
            while end < bytes.len() {
                if bytes[end] == b'`' {
                    let count = bytes[end..].iter().take_while(|&&b| b == b'`').count();
                    if count == size {
                        closing = Some(end + size);
                        break;
                    }
                    end += count;
                } else {
                    end += 1;
                }
            }
            if let Some(end) = closing {
                for byte in &mut bytes[i..end] {
                    if *byte != b'\n' {
                        *byte = b' ';
                    }
                }
                i = end;
            } else {
                i += size;
            }
        } else {
            i += 1;
        }
    }
    bytes
}

fn skip_space(bytes: &[u8], mut i: usize) -> usize {
    while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
        i += 1;
    }
    i
}

// Returns the original destination range and first byte after it. Markdown
// escapes are retained for diagnostics and removed only during classification.
fn destination(bytes: &[u8], start: usize) -> Option<(std::ops::Range<usize>, usize)> {
    let angle = bytes.get(start) == Some(&b'<');
    let begin = start + usize::from(angle);
    let mut i = begin;
    let mut depth = 0;
    while let Some(&byte) = bytes.get(i) {
        match byte {
            b'\\' if bytes.get(i + 1).is_some_and(u8::is_ascii_punctuation) => i += 2,
            b'>' if angle => return Some((begin..i, i + 1)),
            b'\n' | b'\r' if angle => return None,
            b'(' if !angle => {
                depth += 1;
                i += 1;
            }
            b')' if !angle && depth > 0 => {
                depth -= 1;
                i += 1;
            }
            b')' if !angle => break,
            _ if !angle && byte.is_ascii_whitespace() => break,
            _ => i += 1,
        }
    }
    (!angle && depth == 0).then_some((begin..i, i))
}

fn inline_end(bytes: &[u8], end: usize) -> Option<usize> {
    let mut i = skip_space(bytes, end);
    if bytes.get(i) == Some(&b')') {
        return Some(i + 1);
    }
    // A title must be separated from its destination by whitespace.
    if i == end {
        return None;
    }
    let close = match bytes.get(i)? {
        b'"' => b'"',
        b'\'' => b'\'',
        b'(' => b')',
        _ => return None,
    };
    i += 1;
    while let Some(&byte) = bytes.get(i) {
        if byte == b'\\' && bytes.get(i + 1).is_some_and(u8::is_ascii_punctuation) {
            i += 2;
        } else if byte == close {
            let end = skip_space(bytes, i + 1);
            return (bytes.get(end) == Some(&b')')).then_some(end + 1);
        } else {
            i += 1;
        }
    }
    None
}

fn markdown_destinations(text: &str) -> Vec<(usize, String)> {
    let bytes = without_code(text);
    let mut found = Vec::new();
    let mut offset = 0;
    for line in bytes.split_inclusive(|&b| b == b'\n') {
        let indent = line.iter().take_while(|&&b| b == b' ').count();
        if indent <= 3 && line.get(indent) == Some(&b'[') {
            let mut end = indent + 1;
            while end < line.len() && line[end] != b']' {
                end += if line[end] == b'\\' { 2 } else { 1 };
            }
            if end > indent + 1 && line.get(end..end + 2) == Some(b"]:") {
                let start = skip_space(&bytes, offset + end + 2);
                if let Some((range, _)) = destination(&bytes, start) {
                    found.push((range.start, text[range].to_string()));
                }
            }
        }
        offset += line.len();
    }
    let mut brackets = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'[' => {
                brackets.push(i);
                i += 1;
            }
            b']' => {
                let opening = brackets.pop();
                if opening.is_some() && bytes.get(i + 1) == Some(&b'(') {
                    let start = skip_space(&bytes, i + 2);
                    if let Some((range, end)) = destination(&bytes, start)
                        && let Some(end) = inline_end(&bytes, end)
                    {
                        found.push((range.start, text[range].to_string()));
                        i = end;
                        continue;
                    }
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    found
}

fn packet_destination(document: &Path, target: &str) -> Result<bool, String> {
    let mut unescaped = Vec::new();
    let bytes = target.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && bytes.get(i + 1).is_some_and(u8::is_ascii_punctuation) {
            i += 1;
        }
        unescaped.push(bytes[i]);
        i += 1;
    }
    let path = std::str::from_utf8(&unescaped).map_err(|e| e.to_string())?;
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
                text[..offset].bytes().filter(|&b| b == b'\n').count() + 1
            );
            if packet_destination(relative, &target).map_err(|e| format!("{context}: {e}"))? {
                violations.push(format!("{context}: tracked Markdown links into .forge/"));
            }
        }
    }
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
    for (source, target) in [
        (r#"[ruling](../.forge/packet.md)"#, "../.forge/packet.md"),
        (
            r#"![ruling](<../.forge/packet.md> 'title')"#,
            "../.forge/packet.md",
        ),
        (
            r#"[nested [label]](../.forge/(packet).md "title")"#,
            "../.forge/(packet).md",
        ),
        (
            r#"[escaped\] label](../.forge/packet\(1\).md "title")"#,
            r"../.forge/packet\(1\).md",
        ),
        (
            "[label](../.forge/(packet).md\n (a title))",
            "../.forge/(packet).md",
        ),
        (
            "[full][r]\n[r]: <../.forge/packet.md> \"title\"",
            "../.forge/packet.md",
        ),
        (
            "[collapsed][]\n[collapsed]: ../.forge/packet.md",
            "../.forge/packet.md",
        ),
        (
            "[shortcut]\n   [shortcut]:\n  ../.forge/packet.md",
            "../.forge/packet.md",
        ),
        (
            "[missing](../.forge/nonexistent.md)",
            "../.forge/nonexistent.md",
        ),
    ] {
        let found = markdown_destinations(source);
        assert_eq!(found.len(), 1, "{source}: {found:?}");
        assert_eq!(found[0].1, target, "{source}");
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
    assert!(source[found[0].0..].starts_with(&found[0].1));
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
        r"../../\.forge/packet\(1\).md",
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
            "[bad][packet]\n[packet]: ../.forge/packets/ruling.md",
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

#[cfg(unix)]
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
