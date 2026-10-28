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
    // The operator accepted 0063 and 0076 (#250); the record, not this
    // test, is the authority on status.
    let accepted = "accepted (operator, 2026-10-26)";
    assert!(rows[0].contains(accepted));
    let decision =
        std::fs::read_to_string(root().join("docs/decisions/0076-sequential-eval-trials.md"))
            .unwrap();
    assert!(decision.contains(&format!("**Status:** {accepted}")));
    assert!(
        index
            .lines()
            .any(|line| line.starts_with("| [0063]") && line.contains(accepted))
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

// Bounded CommonMark convention check, not an HTML/extension renderer. Block
// context owns code eligibility; inline parsing owns admitted destinations.
#[derive(Default)]
struct MarkdownBlock {
    bytes: Vec<u8>,
    source: Vec<usize>,
    container: bool,
}

impl MarkdownBlock {
    fn push(&mut self, line: &[u8], source: &[usize]) {
        self.bytes.extend_from_slice(line);
        self.source.extend_from_slice(source);
    }

    fn paragraph_open(&self) -> bool {
        let mut i = 0;
        while i < self.bytes.len() {
            if let Some((_, end)) = reference_destination(&self.bytes, i, false) {
                i = end;
            } else {
                return true;
            }
        }
        false
    }

    fn target(&self, text: &str, range: std::ops::Range<usize>) -> (usize, String) {
        let start = self.source[range.start];
        let end = range.clone().last().map_or(start, |i| self.source[i] + 1);
        (start, text[start..end].to_owned())
    }
}

#[derive(Clone, Copy)]
enum Container {
    Quote,
    List(usize), // indentation columns relative to the enclosing container
}

fn spaces(bytes: &[u8], start: usize) -> usize {
    bytes[start..].iter().take_while(|&&b| b == b' ').count()
}

fn quote_prefix(bytes: &[u8], start: usize) -> Option<usize> {
    let indent = spaces(bytes, start);
    let i = start + indent;
    (indent <= 3 && bytes.get(i) == Some(&b'>'))
        .then(|| i + 1 + usize::from(bytes.get(i + 1) == Some(&b' ')))
}

// Marker width and padding are columns, after tab expansion. More than four
// padding spaces use one, leaving the rest as content (possibly indented code).
fn list_prefix(bytes: &[u8], start: usize, interrupt: bool) -> Option<(usize, usize)> {
    let indent = spaces(bytes, start);
    if indent > 3 {
        return None;
    }
    let marker = start + indent;
    let mut end = marker;
    if matches!(bytes.get(end), Some(b'-' | b'+' | b'*')) {
        end += 1;
    } else {
        while bytes.get(end).is_some_and(u8::is_ascii_digit) && end - marker < 10 {
            end += 1;
        }
        if end == marker
            || end - marker > 9
            || !matches!(bytes.get(end), Some(b'.' | b')'))
            || (interrupt && &bytes[marker..end] != b"1")
        {
            return None;
        }
        end += 1;
    }
    if end < bytes.len() && bytes[end] != b' ' {
        return None;
    }
    let padding = spaces(bytes, end);
    if interrupt && end + padding == bytes.len() {
        return None;
    }
    let padding = if (1..=4).contains(&padding) {
        padding
    } else {
        1
    };
    let content = (end + padding).min(bytes.len());
    Some((content, end + padding - start))
}

fn fence_marker(bytes: &[u8]) -> Option<(u8, usize)> {
    let indent = spaces(bytes, 0);
    let rest = &bytes[indent..];
    let marker = *rest.first()?;
    let width = rest.iter().take_while(|&&b| b == marker).count();
    (indent <= 3 && matches!(marker, b'`' | b'~') && width >= 3).then_some((marker, width))
}

fn thematic(bytes: &[u8]) -> bool {
    let indent = spaces(bytes, 0);
    let rest = &bytes[indent..];
    let marker = rest.first().copied().unwrap_or_default();
    indent <= 3
        && matches!(marker, b'-' | b'_' | b'*')
        && rest.iter().filter(|&&b| b == marker).count() >= 3
        && rest.iter().all(|&b| b == marker || b == b' ')
}

fn heading(bytes: &[u8]) -> bool {
    let indent = spaces(bytes, 0);
    let rest = &bytes[indent..];
    let width = rest.iter().take_while(|&&b| b == b'#').count();
    indent <= 3 && (1..=6).contains(&width) && rest.get(width).is_none_or(|&b| b == b' ')
}

fn setext(bytes: &[u8]) -> bool {
    let indent = spaces(bytes, 0);
    let rest = &bytes[indent..];
    let marker = rest.first().copied().unwrap_or_default();
    indent <= 3
        && matches!(marker, b'=' | b'-')
        && rest.iter().take_while(|&&b| b == marker).count() > 0
        && rest
            .iter()
            .skip_while(|&&b| b == marker)
            .all(|&b| b == b' ')
}

fn markdown_blocks(text: &str) -> Vec<MarkdownBlock> {
    let mut blocks = Vec::new();
    let mut current = MarkdownBlock::default();
    let mut containers = Vec::new();
    let mut fence = None;
    let mut offset = 0;
    for raw in text.split_inclusive('\n') {
        // Expand tabs only in this source-mapped view. A tab can straddle a
        // container prefix; destinations and diagnostics still use raw bytes.
        let line = raw.trim_end_matches('\n').trim_end_matches('\r');
        let mut bytes = Vec::new();
        let mut source = Vec::new();
        for (i, b) in line.bytes().enumerate() {
            let count = if b == b'\t' { 4 - bytes.len() % 4 } else { 1 };
            bytes.extend(std::iter::repeat_n(
                if b == b'\t' { b' ' } else { b },
                count,
            ));
            source.extend(std::iter::repeat_n(offset + i, count));
        }
        let mut start = 0;
        let mut matched = 0;
        for container in &containers {
            let next = match container {
                Container::Quote => quote_prefix(&bytes, start),
                Container::List(indent) => {
                    let count = spaces(&bytes, start);
                    (count >= *indent || start + count == bytes.len())
                        .then_some((start + indent).min(bytes.len()))
                }
            };
            if let Some(next) = next {
                start = next;
                matched += 1;
            } else {
                break;
            }
        }
        let rest = &bytes[start..];
        let blank = rest.iter().all(|&b| b == b' ');
        let interrupt = heading(rest)
            || thematic(rest)
            || fence_marker(rest).is_some()
            || quote_prefix(&bytes, start).is_some()
            || list_prefix(&bytes, start, true).is_some();
        let lazy = matched < containers.len()
            && current.paragraph_open()
            && fence.is_none()
            && !blank
            && !interrupt;
        if !lazy && matched < containers.len() {
            containers.truncate(matched);
            fence = None;
            if !current.bytes.is_empty() {
                blocks.push(std::mem::take(&mut current));
            }
        }
        if !lazy && fence.is_none() {
            loop {
                // A thematic break takes precedence over a list marker.
                if thematic(&bytes[start..]) {
                    break;
                }
                let next = if let Some(next) = quote_prefix(&bytes, start) {
                    Some((next, Container::Quote))
                } else {
                    list_prefix(&bytes, start, current.paragraph_open())
                        .map(|(next, indent)| (next, Container::List(indent)))
                };
                let Some((next, container)) = next else { break };
                if !current.bytes.is_empty() {
                    blocks.push(std::mem::take(&mut current));
                }
                containers.push(container);
                start = next;
            }
        }
        let rest = &bytes[start..];
        if let Some((marker, width)) = fence {
            if let Some((closing, size)) = fence_marker(rest)
                && closing == marker
                && size >= width
                && rest[spaces(rest, 0) + size..].iter().all(|&b| b == b' ')
            {
                fence = None;
            }
        } else if rest.iter().all(|&b| b == b' ')
            || thematic(rest)
            || (!current.bytes.is_empty() && setext(rest))
        {
            if !current.bytes.is_empty() {
                blocks.push(std::mem::take(&mut current));
            }
        } else if let Some((marker, width)) = fence_marker(rest)
            && (marker != b'`' || !rest[spaces(rest, 0) + width..].contains(&b'`'))
        {
            if !current.bytes.is_empty() {
                blocks.push(std::mem::take(&mut current));
            }
            fence = Some((marker, width));
        } else if spaces(rest, 0) < 4 || !current.bytes.is_empty() {
            let heading = heading(rest);
            if heading && !current.bytes.is_empty() {
                blocks.push(std::mem::take(&mut current));
            }
            current.container = !containers.is_empty();
            current.push(rest, &source[start..]);
            current.push(b"\n", &[offset + line.len()]);
            if heading {
                blocks.push(std::mem::take(&mut current));
            }
        }
        offset += raw.len();
    }
    if !current.bytes.is_empty() {
        blocks.push(current);
    }
    blocks
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

// Definition destinations are checked directly, without resolving uses. Keep
// the historical bare-line policy; a container definition cannot interrupt
// prose. The caller consumes definition destinations/titles before code spans.
fn reference_destination(
    bytes: &[u8],
    start: usize,
    legacy: bool,
) -> Option<(std::ops::Range<usize>, usize)> {
    let indent = spaces(bytes, start);
    let opening = start + indent;
    if indent > 3 || bytes.get(opening) != Some(&b'[') {
        return None;
    }
    let mut i = opening + 1;
    while let Some(&byte) = bytes.get(i) {
        match byte {
            b'\\' if bytes.get(i + 1).is_some_and(u8::is_ascii_punctuation) => i += 2,
            b'[' => return None,
            b']' => break,
            _ => i += 1,
        }
    }
    if i == opening + 1 || bytes.get(i..i + 2) != Some(b"]:") {
        return None;
    }
    let begin = skip_space(bytes, i + 2);
    let (range, end) = destination(bytes, begin)?;
    if range.is_empty() {
        return None;
    }
    let line_end = bytes[end..]
        .iter()
        .position(|&b| b == b'\n')
        .map_or(bytes.len(), |n| end + n);
    let title = skip_space(bytes, end);
    let mut consumed = line_end;
    let mut valid = title >= line_end;
    if title > end && matches!(bytes.get(title), Some(b'"' | b'\'' | b'(')) {
        let close = if bytes[title] == b'(' {
            b')'
        } else {
            bytes[title]
        };
        let mut i = title + 1;
        while i < bytes.len() {
            if bytes[i] == b'\\' && bytes.get(i + 1).is_some_and(u8::is_ascii_punctuation) {
                i += 2;
            } else if bytes[i] == close {
                let mut tail = i + 1;
                while bytes.get(tail) == Some(&b' ') {
                    tail += 1;
                }
                if bytes.get(tail).is_none_or(|&b| b == b'\n') {
                    consumed = tail;
                    valid = true;
                }
                break;
            } else {
                i += 1;
            }
        }
    }
    (valid || legacy).then_some((range, (consumed + 1).min(bytes.len())))
}

fn markdown_destinations(text: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for block in markdown_blocks(text) {
        let bytes = &block.bytes;
        // Index delimiter runs once; unmatched runs never rescan a suffix.
        let mut runs: std::collections::HashMap<usize, Vec<usize>> = Default::default();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'`' {
                let size = bytes[i..].iter().take_while(|&&b| b == b'`').count();
                runs.entry(size).or_default().push(i);
                i += size;
            } else {
                i += 1;
            }
        }
        let mut brackets = Vec::new();
        let mut prose = false;
        let mut i = 0;
        while i < bytes.len() {
            if (i == 0 || bytes[i - 1] == b'\n')
                && (!block.container || !prose)
                && let Some((range, end)) = reference_destination(bytes, i, !block.container)
            {
                found.push(block.target(text, range));
                i = end;
                continue;
            }
            // After definitions, indented code may begin without a blank
            // line. Definition continuation bytes were already consumed above.
            if !prose && (i == 0 || bytes[i - 1] == b'\n') && spaces(bytes, i) >= 4 {
                i += bytes[i..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .unwrap_or(bytes.len() - i);
                i = (i + 1).min(bytes.len());
                continue;
            }
            match bytes[i] {
                b'\\' if bytes.get(i + 1).is_some_and(u8::is_ascii_punctuation) => {
                    prose = true;
                    i += 2;
                }
                b'`' => {
                    prose = true;
                    let size = bytes[i..].iter().take_while(|&&b| b == b'`').count();
                    let positions = runs.get(&size).map(Vec::as_slice).unwrap_or_default();
                    let next = positions.partition_point(|&p| p <= i);
                    i = positions.get(next).map_or(i + size, |p| p + size);
                }
                b'[' => {
                    prose = true;
                    brackets.push(i);
                    i += 1;
                }
                b']' => {
                    let opening = brackets.pop();
                    if opening.is_some() && bytes.get(i + 1) == Some(&b'(') {
                        let start = skip_space(bytes, i + 2);
                        if let Some((range, end)) = destination(bytes, start)
                            && let Some(end) = inline_end(bytes, end)
                        {
                            found.push(block.target(text, range));
                            i = end;
                            continue;
                        }
                    }
                    i += 1;
                }
                _ => {
                    prose |= !bytes[i].is_ascii_whitespace();
                    i += 1;
                }
            }
        }
    }
    found
}

// CommonMark references differ from HTML attribute decoding: digit limits,
// replacement scalars and no legacy numeric remapping. Escapes and references
// consume original input together once; emitted syntax is never interpreted.
fn markdown_unescape(target: &str) -> String {
    static NAMES: std::sync::OnceLock<std::collections::HashMap<String, String>> =
        std::sync::OnceLock::new();
    let names = NAMES.get_or_init(|| {
        let snapshot: serde_json::Value =
            serde_json::from_str(include_str!("../../scripts/data/html-entities.json")).unwrap();
        snapshot
            .as_object()
            .unwrap()
            .iter()
            .filter(|(name, _)| name.ends_with(';'))
            .map(|(name, entry)| {
                (
                    name.clone(),
                    entry["characters"].as_str().unwrap().to_owned(),
                )
            })
            .collect()
    });
    let bytes = target.as_bytes();
    let mut output = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && bytes.get(i + 1).is_some_and(u8::is_ascii_punctuation) {
            output.push(bytes[i + 1] as char);
            i += 2;
            continue;
        }
        if bytes[i] == b'&' {
            if bytes.get(i + 1) == Some(&b'#') {
                let hex = matches!(bytes.get(i + 2), Some(b'x' | b'X'));
                let start = i + if hex { 3 } else { 2 };
                let limit = if hex { 6 } else { 7 };
                let mut end = start;
                while end < bytes.len()
                    && end - start <= limit
                    && if hex {
                        bytes[end].is_ascii_hexdigit()
                    } else {
                        bytes[end].is_ascii_digit()
                    }
                {
                    end += 1;
                }
                if (1..=limit).contains(&(end - start)) && bytes.get(end) == Some(&b';') {
                    let number =
                        u32::from_str_radix(&target[start..end], if hex { 16 } else { 10 })
                            .unwrap();
                    output.push(
                        char::from_u32(number)
                            .filter(|&c| c != '\0')
                            .unwrap_or('\u{fffd}'),
                    );
                    i = end + 1;
                    continue;
                }
            } else {
                // The longest semicolon-terminated snapshot key is 33 bytes.
                let mut end = i + 1;
                while end < bytes.len() && end - i < 33 && bytes[end].is_ascii_alphanumeric() {
                    end += 1;
                }
                if bytes.get(end) == Some(&b';')
                    && let Some(value) = names.get(&target[i..=end])
                {
                    output.push_str(value);
                    i = end + 1;
                    continue;
                }
            }
        }
        let ch = target[i..].chars().next().unwrap();
        output.push(ch);
        i += ch.len_utf8();
    }
    output
}

fn packet_destination(document: &Path, target: &str) -> Result<bool, String> {
    let path = markdown_unescape(target);
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

// Each case is a real indexed document; classification must not depend on
// whether the delivery-state destination exists in the fixture repository.
fn tracked_cases(refused: &[(&str, &str)], allowed: &[&str]) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture_git(root, &["init", "--quiet"]);
    std::fs::create_dir(root.join("docs")).unwrap();
    let name = "docs/é note.MD";
    let document = root.join(name);
    for &(source, target) in refused {
        std::fs::write(&document, source).unwrap();
        fixture_git(root, &["add", "--", name]);
        let error = check_tracked_markdown(root).expect_err(&format!(
            "tracked destination escaped the guard: {source:?}"
        ));
        let offset = source.rfind(target).unwrap();
        let line = source[..offset].bytes().filter(|&b| b == b'\n').count() + 1;
        assert!(
            error.contains(&format!(
                "{name}:{line}: {target}: tracked Markdown links into .forge/"
            )),
            "{source:?}: {error}"
        );
        let repaired = source.replace(target, "../rulings/p.md");
        std::fs::write(&document, repaired).unwrap();
        fixture_git(root, &["add", "--", name]);
        check_tracked_markdown(root).unwrap();
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
            ("`unmatched\n\n[bad](../.forge/p.md)\n`", "../.forge/p.md"),
            (
                "`unmatched\n \t \n[bad](../.forge/p.md)\n`",
                "../.forge/p.md",
            ),
            (
                "`unmatched\r\n  \r\n[bad](../.forge/p.md)\r\n`",
                "../.forge/p.md",
            ),
            ("`unmatched\n# [bad](../.forge/p.md)\n`", "../.forge/p.md"),
            (
                "`unmatched\n---\n[bad](../.forge/p.md)\n`",
                "../.forge/p.md",
            ),
            ("> `unmatched\n\n[bad](../.forge/p.md)\n`", "../.forge/p.md"),
            (
                "- `unmatched\n- [bad](../.forge/p.md)\n  `",
                "../.forge/p.md",
            ),
            ("`unmatched\n> [bad](../.forge/p.md)\n> `", "../.forge/p.md"),
            (
                "[broken](<../rulings/`\n\n[bad](../.forge/p.md)\n`",
                "../.forge/p.md",
            ),
            ("``wrong size [bad](../.forge/p.md)`", "../.forge/p.md"),
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
            ("[bad](../.forge/`p`) [ok](../rulings/`q`)", "../.forge/`p`"),
            ("[bad](<../.forge/`p`> \"`title`\")", "../.forge/`p`"),
            ("[r]: ../.forge/`p`", "../.forge/`p`"),
            ("[r]: <../.forge/`p`> '`title`'", "../.forge/`p`"),
            ("[ok](../rulings/`p) [bad](../.forge/q`)`", "../.forge/q`"),
            (
                "[ok](../rulings/p \"`title\") [bad](../.forge/q)`",
                "../.forge/q",
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
            ("> [r]: ../.forge/p.md", "../.forge/p.md"),
            (
                "[ok]: ../rulings/p.md\n2. [r]: ../.forge/p.md",
                "../.forge/p.md",
            ),
            (
                "> [ok]: ../rulings/p.md\n2. [r]: ../.forge/p.md",
                "../.forge/p.md",
            ),
            ("- [r]: ../.forge/p.md", "../.forge/p.md"),
            ("+ [r]: <../.forge/p.md>", "../.forge/p.md"),
            ("* [r]: ../.forge/p.md", "../.forge/p.md"),
            ("1. [r]: ../.forge/p.md", "../.forge/p.md"),
            ("12) [r]: ../.forge/p.md", "../.forge/p.md"),
            ("> - 1. > [r]: ../.forge/p.md", "../.forge/p.md"),
            ("> [r]:\n>   ../.forge/p.md", "../.forge/p.md"),
            ("- [r]:\n    ../.forge/p.md", "../.forge/p.md"),
            ("1. [r]:\n   ../.forge/p.md", "../.forge/p.md"),
            ("- intro\n\n  [r]: ../.forge/p.md", "../.forge/p.md"),
            (
                "> - é\n>\n>   [r]:\n>     <../.forge/p.md>",
                "../.forge/p.md",
            ),
            (">\t[r]: ../.forge/p.md", "../.forge/p.md"),
            ("-\t[r]: ../.forge/p.md", "../.forge/p.md"),
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
        ],
    );
}

#[test]
fn tracked_character_references_are_decoded_once() {
    tracked_cases(
        &[
            ("[bad](../&#46;forge/p.md)", "../&#46;forge/p.md"),
            (
                "[bad](../&#x2e;forge&#x2F;p.md)",
                "../&#x2e;forge&#x2F;p.md",
            ),
            (
                "[bad](../&period;forge&sol;p.md)",
                "../&period;forge&sol;p.md",
            ),
            ("[r]: ../&#X2E;forge/p.md", "../&#X2E;forge/p.md"),
            ("> [r]: ../&period;forge/p.md", "../&period;forge/p.md"),
            ("[bad](../&percnt;2eforge/p.md)", "../&percnt;2eforge/p.md"),
            (
                "[bad](../.forge/p.md&quest;raw=1&num;row)",
                "../.forge/p.md&quest;raw=1&num;row",
            ),
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
fn markdown_reference_decoder_obeys_commonmark_boundaries() {
    for (source, decoded) in [
        ("&#46; &#0000046; &#x2e; &#X00002E;", ". . . ."),
        ("&#0; &#xD800; &#xDFFF; &#x110000; &#9999999;", "� � � � �"),
        (
            "&#128; &#x80; &#1; &#x10FFFF;",
            "\u{80} \u{80} \u{1} \u{10ffff}",
        ),
        ("&NotEqualTilde;", "\u{2242}\u{338}"),
        ("&CounterClockwiseContourIntegral;", "∳"),
        (
            "&period; &Period; &PERIOD; &period &unknown;",
            ". &Period; &PERIOD; &period &unknown;",
        ),
        (
            "&#; &#x; &#X; &#-46; &#+46; &#４６;",
            "&#; &#x; &#X; &#-46; &#+46; &#４６;",
        ),
        (
            "&#00000046; &#x000002e; &#46 &#x2e",
            "&#00000046; &#x000002e; &#46 &#x2e",
        ),
        (
            r"\&period; &amp;period; &bsol;&period;",
            "&period; &period; \\.",
        ),
        (r"\\&period; \a", "\\. \\a"),
        ("&#38;period; &#92;&period;", "&period; \\."),
        ("é&sol;λ", "é/λ"),
    ] {
        assert_eq!(markdown_unescape(source), decoded, "{source}");
    }
    // Reuse the entire pinned table, including multi-scalar mappings. This also
    // protects the bounded name lookahead from silently dropping a long name.
    let snapshot: serde_json::Value =
        serde_json::from_str(include_str!("../../scripts/data/html-entities.json")).unwrap();
    let mut count = 0;
    for (name, entry) in snapshot.as_object().unwrap() {
        if name.ends_with(';') {
            assert_eq!(
                markdown_unescape(name),
                entry["characters"].as_str().unwrap(),
                "{name}"
            );
            count += 1;
        }
    }
    assert_eq!(count, 2125);
    assert!(
        packet_destination(Path::new("docs/note.md"), "../&percnt;ff")
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
                "../&period;forge/`p`",
            ),
            (
                "> - é [bad](<../&#46;forge/`p`> 'λ `title`')",
                "../&#46;forge/`p`",
            ),
            (
                "- intro\n  > [r]:\n  > ../&#46;forge/p.md",
                "../&#46;forge/p.md",
            ),
            ("[bad](../&#0000046;forge/p.md)", "../&#0000046;forge/p.md"),
            ("[bad](../&#x00002e;forge/p.md)", "../&#x00002e;forge/p.md"),
            ("\\``unmatched [bad](../.forge/p.md)", "../.forge/p.md"),
            ("[unfinished\n\n[label](../.forge/p.md)", "../.forge/p.md"),
            (
                "[bad](../.forge/p.md \"unclosed)\n\n[real](../.forge/q.md)",
                "../.forge/q.md",
            ),
        ],
        &[
            "> - ``[example](../&period;forge/`p`)``",
            "> - ~~~\r\n>   [r]: ../&period;forge/p.md\r\n>   ~~~",
            "> -     [r]: ../&period;forge/p.md",
            "> [r]: ../.forge/p.md trailing prose",
            "> [r]: ../.forge/p.md \"title\" trailing prose",
            "> [r]: ../rulings/p.md\n>     [example](../.forge/p.md)",
            "> [r]: ../rulings/p.md\n>   \"title [example](../.forge/p.md)\"",
            "- [r]: ../rulings/p.md\n      [example](../.forge/p.md)",
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
    tracked_cases(&[(&source, "../.forge/p.md")], &[]);
}
