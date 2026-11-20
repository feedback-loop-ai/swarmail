//! The README's agent section cites the published eval runs (#394).
//!
//! Every link in `## For AI agents` must resolve, every figure it quotes
//! must be a link to the page that states it, and that page must still
//! state it verbatim. The section is read with a small inline grammar —
//! text, code spans and `[label](destination)` links — and anything else
//! (reference links, images, HTML, bare URLs, escapes, fences) is refused,
//! so a citation cannot slip past the check in a form it does not read.
//! File-only: no server, no child process.

use std::path::Path;

const SECTION: &str = "## For AI agents";

const BASELINE: &str = "docs/evals/baseline-2026-11-01.md";
const GLM: &str = "docs/evals/baseline-2026-11-01-glm-dashscope.md";
const HARD: &str = "docs/evals/hard-2026-11-03.md";
const EXPERT: &str = "docs/evals/expert-2026-11-07.md";
const INDEX: &str = "docs/evals/README.md";

/// The pages a figure may cite. The throughput and agent-comparison pages
/// are absent on purpose: the README quotes nothing from them (#349).
const EVAL_PAGES: [&str; 4] = [BASELINE, GLM, HARD, EXPERT];

/// Navigation and proof links the section may carry besides the runs.
const PROOFS: [&str; 6] = [
    INDEX,
    "docs/agents/setup.md",
    "docs/guide/mcp.md",
    "tests/agent_setup_docs.rs",
    "tests/mcp.rs",
    "tests/stdio.rs",
];

/// Every figure the section quotes, with the page that states it. Stated
/// here, not derived from either document, so a figure moved to another
/// page that happens to contain the same digits is still caught.
const CITED: [(&str, &str); 13] = [
    ("a4f552d", BASELINE),
    ("16", BASELINE),
    ("395 of 400", BASELINE),
    ("80 of 80", GLM),
    ("475 of 480", GLM),
    ("8732445", HARD),
    ("74 of 75 scored trials", HARD),
    ("c95386e", EXPERT),
    ("151/160", EXPERT),
    ("0/5", EXPERT),
    ("95%", BASELINE),
    ("5/5", BASELINE),
    ("[0.57, 1.0]", BASELINE),
];

const SUPERLATIVES: [&str; 8] = [
    "best",
    "fastest",
    "most",
    "perfect",
    "flawless",
    "unbeatable",
    "unmatched",
    "world-class",
];

#[derive(Debug)]
struct Link {
    /// The label with code spans unwrapped and whitespace squashed.
    label: String,
    dest: String,
}

#[derive(Debug, Default)]
struct Parsed {
    links: Vec<Link>,
    /// The prose as read: labels inline, each code span as `code`.
    prose: String,
}

fn squash(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_fence(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("```") || t.starts_with("~~~")
}

/// The section's body lines, and the README with the section cut out.
fn section(readme: &str) -> Result<(String, String), String> {
    let lines: Vec<&str> = readme.lines().collect();
    let mut fenced = false;
    let mut starts = Vec::new();
    let mut ends = Vec::new();
    for (n, line) in lines.iter().enumerate() {
        if is_fence(line) {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        if *line == SECTION {
            starts.push(n);
        } else if line.starts_with("# ") || line.starts_with("## ") {
            ends.push(n);
        }
    }
    let start = match starts.as_slice() {
        [] => return Err(format!("README.md has no `{SECTION}` section")),
        [one] => *one,
        _ => return Err(format!("README.md has more than one `{SECTION}` section")),
    };
    let end = ends.into_iter().find(|&n| n > start).unwrap_or(lines.len());
    let body = lines[start + 1..end].join("\n");
    let rest = [&lines[..start], &lines[end..]].concat().join("\n");
    Ok((body, rest))
}

/// Read a code span opening at byte `i` (a run of backticks); its content
/// and the byte after its closing run.
fn code_span(text: &str, i: usize) -> Result<(String, usize), String> {
    let run = text[i..].bytes().take_while(|&b| b == b'`').count();
    let fence = "`".repeat(run);
    let mut from = i + run;
    loop {
        let Some(at) = text[from..].find(&fence) else {
            return Err(format!("unclosed code span: {:?}", &text[i..]));
        };
        let at = from + at;
        let after = text[at + run..].bytes().take_while(|&b| b == b'`').count();
        if after == 0 {
            return Ok((text[i + run..at].to_string(), at + run));
        }
        from = at + run + after;
    }
}

/// The forms the grammar refuses wherever they appear, named.
fn refused(text: &str, i: usize) -> Option<&'static str> {
    let rest = &text[i..];
    if rest.starts_with("![") {
        Some("an image")
    } else if rest.starts_with('<') {
        Some("HTML or an autolink")
    } else if rest.starts_with('&') {
        Some("an HTML entity")
    } else if rest.starts_with('\\') {
        Some("a backslash escape")
    } else if rest.starts_with("http://")
        || rest.starts_with("https://")
        || rest.starts_with("www.")
    {
        Some("a bare URL")
    } else if rest.starts_with("docs/evals") {
        Some("a bare eval path")
    } else {
        None
    }
}

/// Parse one paragraph of the section with the restricted grammar.
fn parse(text: &str) -> Result<Parsed, String> {
    let mut out = Parsed::default();
    let mut i = 0;
    while i < text.len() {
        if let Some(what) = refused(text, i) {
            return Err(format!("unsupported syntax, {what}: {:?}", &text[i..]));
        }
        let c = text[i..].chars().next().unwrap();
        match c {
            '`' => {
                let (code, next) = code_span(text, i)?;
                if code.contains("docs/evals") {
                    return Err(format!("unsupported syntax, a bare eval path: `{code}`"));
                }
                if code.bytes().any(|b| b.is_ascii_digit()) {
                    return Err(format!("uncited figure in a code span: `{code}`"));
                }
                out.prose.push_str("code");
                i = next;
            }
            ']' => return Err(format!("a stray `]`: {:?}", &text[i..])),
            '[' => {
                let (link, next) = link(text, i)?;
                out.prose.push_str(&link.label);
                out.links.push(link);
                i = next;
            }
            '#' if text[i + 1..].starts_with(|c: char| c.is_ascii_digit()) => {
                // An issue reference, `#391`: a pointer, not a measurement.
                let digits = text[i + 1..].bytes().take_while(u8::is_ascii_digit).count();
                out.prose.push_str(&text[i..i + 1 + digits]);
                i += 1 + digits;
            }
            d if d.is_ascii_digit() => {
                return Err(format!("uncited figure: {:?}", &text[i..]));
            }
            _ => {
                out.prose.push(c);
                i += c.len_utf8();
            }
        }
    }
    out.prose = squash(&out.prose);
    Ok(out)
}

/// Read `[label](destination)` opening at byte `i`.
fn link(text: &str, i: usize) -> Result<(Link, usize), String> {
    let mut label = String::new();
    let mut j = i + 1;
    loop {
        if j >= text.len() {
            return Err(format!("an unclosed link label: {:?}", &text[i..]));
        }
        if let Some(what) = refused(text, j) {
            return Err(format!("unsupported syntax, {what}: {:?}", &text[j..]));
        }
        let c = text[j..].chars().next().unwrap();
        match c {
            '`' => {
                let (code, next) = code_span(text, j)?;
                label.push_str(&code);
                j = next;
            }
            '[' => return Err(format!("a nested `[` in a link label: {:?}", &text[i..])),
            ']' => break,
            _ => {
                label.push(c);
                j += c.len_utf8();
            }
        }
    }
    let after = j + 1;
    if !text[after..].starts_with('(') {
        return Err(format!(
            "not an inline link (a reference, shortcut or definition form): {:?}",
            &text[i..]
        ));
    }
    let Some(close) = text[after + 1..].find(')') else {
        return Err(format!("a malformed link destination: {:?}", &text[i..]));
    };
    let dest = &text[after + 1..after + 1 + close];
    if dest.is_empty() || dest.contains(char::is_whitespace) || dest.contains('(') {
        return Err(format!("a malformed link destination: {:?}", &text[i..]));
    }
    let link = Link {
        label: squash(&label),
        dest: dest.to_string(),
    };
    Ok((link, after + 1 + close + 1))
}

/// GitHub's anchor for each simple heading of `page`, outside fences;
/// `None` for a heading the check does not claim to slug.
fn anchors(page: &str) -> Vec<Option<String>> {
    let mut fenced = false;
    let mut out = Vec::new();
    for line in page.lines() {
        if is_fence(line) {
            fenced = !fenced;
            continue;
        }
        let hashes = line.bytes().take_while(|&b| b == b'#').count();
        if fenced || hashes == 0 || hashes > 6 || !line[hashes..].starts_with(' ') {
            continue;
        }
        let heading = line[hashes..].trim();
        let simple = heading
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || " -_`,:'()".contains(c));
        out.push(simple.then(|| {
            heading
                .to_ascii_lowercase()
                .chars()
                .filter_map(|c| match c {
                    ' ' => Some('-'),
                    c if c.is_ascii_alphanumeric() || c == '-' || c == '_' => Some(c),
                    _ => None,
                })
                .collect()
        }));
    }
    out
}

/// `figure` occurs in `page` not as part of a longer number or word.
fn states(page: &str, figure: &str) -> bool {
    page.match_indices(figure).any(|(at, _)| {
        let before = page[..at].chars().next_back();
        let rest = &page[at + figure.len()..];
        let mut after = rest.chars();
        let next = after.next();
        let joined = |c: char| c.is_ascii_alphanumeric() || c == '/';
        !before.is_some_and(|c| joined(c) || c == '.')
            && !next.is_some_and(joined)
            && !(next == Some('.') && after.next().is_some_and(|c| c.is_ascii_digit()))
    })
}

fn read(root: &Path, path: &str) -> Result<String, String> {
    std::fs::read_to_string(root.join(path)).map_err(|e| format!("{path} does not resolve: {e}"))
}

/// Check one link: its destination, its fragment, and any figure it quotes.
fn check_link(root: &Path, link: &Link) -> Result<(), String> {
    let dest = &link.dest;
    if dest.contains(['%', '\\', '?', '<', '>', ':']) {
        return Err(format!("an unsupported destination form: {dest}"));
    }
    if dest.starts_with('/') || dest.contains("./") || dest.contains("//") {
        return Err(format!("an absolute or traversing destination: {dest}"));
    }
    let (path, fragment) = match dest.split_once('#') {
        Some((path, fragment)) => (path, Some(fragment)),
        None => (dest.as_str(), None),
    };
    if !EVAL_PAGES.contains(&path) && !PROOFS.contains(&path) {
        return Err(format!("an unsupported destination: {dest}"));
    }
    let raw = read(root, path)?;
    if let Some(fragment) = fragment {
        if !path.starts_with("docs/evals/") {
            return Err(format!("a fragment off the eval pages: {dest}"));
        }
        let found = anchors(&raw)
            .into_iter()
            .filter(|a| a.as_deref() == Some(fragment))
            .count();
        match found {
            1 => {}
            0 => return Err(format!("#{fragment} does not resolve on {path}")),
            _ => return Err(format!("#{fragment} is ambiguous on {path}")),
        }
    }
    if link.label.bytes().any(|b| b.is_ascii_digit()) {
        if !CITED.contains(&(link.label.as_str(), path)) {
            return Err(format!(
                "figure {:?} is not pinned to {path}: cite the page that states it",
                link.label
            ));
        }
        if !states(&squash(&raw), &link.label) {
            return Err(format!(
                "figure {:?} no longer appears on {path}",
                link.label
            ));
        }
    }
    Ok(())
}

/// Number of sentences in a paragraph's prose.
fn sentences(prose: &str) -> usize {
    let bytes = prose.as_bytes();
    (0..bytes.len())
        .filter(|&n| b".!?".contains(&bytes[n]))
        .filter(|&n| bytes.get(n + 1).is_none_or(|b| b.is_ascii_whitespace()))
        .count()
}

fn page_of(link: &Link) -> &str {
    link.dest.split('#').next().unwrap_or_default()
}

/// The whole check, over a repository root.
fn check(root: &Path) -> Result<(), String> {
    let readme = read(root, "README.md")?;
    let (body, rest) = section(&readme)?;
    if rest.contains("docs/evals") {
        return Err("README.md cites docs/evals outside the agent section".into());
    }
    let mut links = Vec::new();
    let mut eval_paragraphs = Vec::new();
    for paragraph in body.split("\n\n").filter(|p| !p.trim().is_empty()) {
        for line in paragraph.lines() {
            if is_fence(line) || line.starts_with("    ") || line.starts_with('\t') {
                return Err(format!("unsupported syntax, a code block: {line:?}"));
            }
        }
        let parsed = parse(paragraph)?;
        let lower = parsed.prose.to_ascii_lowercase();
        let words: Vec<&str> = lower
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
            .collect();
        if let Some(word) = SUPERLATIVES.iter().find(|s| words.contains(s)) {
            return Err(format!("a superlative in the agent section: {word:?}"));
        }
        if parsed
            .links
            .iter()
            .any(|l| EVAL_PAGES.contains(&page_of(l)))
        {
            eval_paragraphs.push(parsed.prose.clone());
        }
        links.extend(parsed.links);
    }
    for link in &links {
        check_link(root, link)?;
    }
    for (figure, page) in CITED {
        if !links
            .iter()
            .any(|l| l.label == figure && page_of(l) == page)
        {
            return Err(format!("required citation {figure:?} → {page} is missing"));
        }
    }
    if !links.iter().any(|l| l.dest == INDEX) {
        return Err(format!("the agent section does not link {INDEX}"));
    }
    let [paragraph] = eval_paragraphs.as_slice() else {
        return Err(format!(
            "the eval runs must be cited in one paragraph, found {}",
            eval_paragraphs.len()
        ));
    };
    let n = sentences(paragraph);
    if !(3..=5).contains(&n) {
        return Err(format!("the eval paragraph has {n} sentences, not 3 to 5"));
    }
    Ok(())
}

fn repo() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// A passing copy of everything the check reads, in a fresh directory.
fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for path in ["README.md"].into_iter().chain(EVAL_PAGES).chain(PROOFS) {
        let to = dir.path().join(path);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(repo().join(path), &to).unwrap();
    }
    assert_eq!(check(dir.path()), Ok(()), "the unmodified copy passes");
    dir
}

/// Rewrite `path` in a fixture, requiring the edit to change something.
fn edit(dir: &tempfile::TempDir, path: &str, f: impl FnOnce(&str) -> String) {
    let at = dir.path().join(path);
    let before = std::fs::read_to_string(&at).unwrap();
    let after = f(&before);
    assert_ne!(before, after, "the mutation of {path} changed nothing");
    std::fs::write(at, after).unwrap();
}

/// Mutate a fresh fixture's `path` and return the check's diagnostic.
fn broken(path: &str, f: impl FnOnce(&str) -> String) -> String {
    let dir = fixture();
    edit(&dir, path, f);
    check(dir.path()).expect_err("the damaged copy is refused")
}

/// Mutate the first occurrence of `from` in the README.
fn broken_readme(from: &str, to: &str) -> String {
    broken("README.md", |s| s.replacen(from, to, 1))
}

fn red(error: &str, expected: &str) {
    assert!(
        error.contains(expected),
        "expected {expected:?}, got {error:?}"
    );
}

#[test]
fn the_readme_cites_each_figure_from_its_own_page() {
    assert_eq!(check(repo()), Ok(()));
}

#[test]
fn every_pinned_figure_is_a_figure_its_page_states() {
    for (figure, page) in CITED {
        let text = squash(&std::fs::read_to_string(repo().join(page)).unwrap());
        assert!(states(&text, figure), "{page} no longer states {figure:?}");
    }
}

#[test]
fn a_supported_fragment_passes_and_an_absent_one_is_refused() {
    let dir = fixture();
    edit(&dir, "README.md", |s| {
        s.replacen(
            "[0/5](docs/evals/expert-2026-11-07.md)",
            "[0/5](docs/evals/expert-2026-11-07.md#stale_code-codex-luna-on-otp-late)",
            1,
        )
    });
    assert_eq!(check(dir.path()), Ok(()));
    red(
        &broken_readme("#success-by-task-and-by-model", "#no-such-heading"),
        "#no-such-heading does not resolve on docs/evals/baseline-2026-11-01.md",
    );
    red(
        &broken_readme("](docs/agents/setup.md)", "](docs/agents/setup.md#start)"),
        "a fragment off the eval pages",
    );
}

#[test]
fn an_ambiguous_fragment_is_refused() {
    red(
        &broken(BASELINE, |s| {
            format!("{s}\n## Success by task and by model\n")
        }),
        "#success-by-task-and-by-model is ambiguous",
    );
}

#[test]
fn a_missing_page_or_readme_is_refused() {
    for path in [HARD, "README.md"] {
        let dir = fixture();
        std::fs::remove_file(dir.path().join(path)).unwrap();
        red(
            &check(dir.path()).unwrap_err(),
            &format!("{path} does not resolve"),
        );
    }
}

#[test]
fn a_missing_or_duplicate_section_is_refused() {
    red(
        &broken_readme("## For AI agents\n", "## For agents\n"),
        "has no `## For AI agents` section",
    );
    red(
        &broken("README.md", |s| {
            format!("{s}\n## For AI agents\n\nAgain.\n")
        }),
        "more than one `## For AI agents` section",
    );
}

#[test]
fn a_deleted_citation_is_refused() {
    red(
        &broken_readme(
            "scored [0/5](docs/evals/expert-2026-11-07.md) on",
            "failed on",
        ),
        "required citation \"0/5\" → docs/evals/expert-2026-11-07.md is missing",
    );
    red(
        &broken_readme("[eval index](docs/evals/README.md)", "eval index"),
        "the agent section does not link docs/evals/README.md",
    );
}

#[test]
fn a_figure_its_page_no_longer_states_is_refused() {
    // Every occurrence goes, so the figure is truly absent from the page.
    red(
        &broken(GLM, |s| s.replace("475 of 480", "476 of 480")),
        "figure \"475 of 480\" no longer appears on docs/evals/baseline-2026-11-01-glm-dashscope.md",
    );
    // A longer number containing the figure is not the figure.
    red(
        &broken(HARD, |s| {
            s.replace("74 of 75 scored trials", "174 of 75 scored trials")
        }),
        "figure \"74 of 75 scored trials\" no longer appears",
    );
}

#[test]
fn a_figure_without_a_citation_is_refused() {
    red(
        &broken_readme("has the rest.", "has the rest. It took 12 hours."),
        "uncited figure: \"12 hours.",
    );
    red(
        &broken_readme("on `otp-late`", "on `otp-late` (`48 s`)"),
        "uncited figure in a code span: `48 s`",
    );
}

#[test]
fn a_figure_citing_another_real_page_is_refused() {
    // The DashScope page states "16" too; the pin still names the baseline.
    red(
        &broken_readme(
            "[16](docs/evals/baseline-2026-11-01.md)",
            "[16](docs/evals/baseline-2026-11-01-glm-dashscope.md)",
        ),
        "figure \"16\" is not pinned to docs/evals/baseline-2026-11-01-glm-dashscope.md",
    );
    // The hard page restates the baseline's total; it is not its source.
    red(
        &broken_readme(
            "[395 of 400](docs/evals/baseline-2026-11-01.md)",
            "[395/400](docs/evals/hard-2026-11-03.md)",
        ),
        "figure \"395/400\" is not pinned to docs/evals/hard-2026-11-03.md",
    );
}

#[test]
fn unsupported_link_forms_beside_good_links_are_refused() {
    let index = "](docs/evals/README.md)";
    let hard = "[74 of 75 scored trials](docs/evals/hard-2026-11-03.md)";
    let end = "has the rest.";
    let cases = [
        (
            "[eval index](docs/evals/README.md)",
            "[eval index][idx]",
            "not an inline link",
        ),
        (
            end,
            "has the rest.\n\n[idx]: docs/evals/README.md",
            "not an inline link",
        ),
        (end, "has the rest. [the index]", "not an inline link"),
        (end, "has the rest. ![chart](docs/x.svg)", "an image"),
        (
            end,
            "has the rest. <a href=\"x\">x</a>",
            "HTML or an autolink",
        ),
        (end, "has the rest. See https://example.com.", "a bare URL"),
        (
            end,
            "has the rest. See docs/evals/hard-2026-11-03.md.",
            "a bare eval path",
        ),
        ("on `otp-late`", "on `docs/evals/x.md`", "a bare eval path"),
        ("[eval index]", "[eval <b>index</b>]", "HTML or an autolink"),
        (end, "has the rest &amp; more.", "an HTML entity"),
        (end, "has the rest\\.", "a backslash escape"),
        (end, "has the rest.\n\n```\nx\n```", "a code block"),
        (end, "has the rest.\n\n    indented", "a code block"),
        (end, "has the rest. A ] stray.", "a stray `]`"),
        (end, "has the rest. [unclosed", "an unclosed link label"),
        (end, "has the rest. `unclosed", "unclosed code span"),
        ("[eval index]", "[eval [nested] index]", "a nested `[`"),
        (
            index,
            "](docs/evals/README.md \"t\")",
            "a malformed link destination",
        ),
        (
            index,
            "](docs/evals/README.md",
            "a malformed link destination",
        ),
        (index, "]()", "a malformed link destination"),
        (
            index,
            "](/docs/evals/README.md)",
            "an absolute or traversing",
        ),
        (
            hard,
            "[74 of 75 scored trials](docs/evals/../evals/hard-2026-11-03.md)",
            "an absolute or traversing",
        ),
        (
            hard,
            "[74 of 75 scored trials](docs/evals/hard%2D2026-11-03.md)",
            "an unsupported destination form",
        ),
        (
            index,
            "](https://github.com/x/y/blob/main/docs/evals/README.md)",
            "an unsupported destination form",
        ),
        (
            index,
            "](docs/evals/throughput-2026-11-07.md)",
            "an unsupported destination: docs/evals/throughput-2026-11-07.md",
        ),
        (
            index,
            "](docs/evals/agent-comparison-2026-11-09.md)",
            "an unsupported destination: docs/evals/agent-comparison-2026-11-09.md",
        ),
    ];
    for (from, to, expected) in cases {
        red(&broken_readme(from, to), expected);
    }
}

#[test]
fn an_eval_link_outside_the_agent_section_is_refused() {
    red(
        &broken_readme(
            "- [For AI agents](#for-ai-agents)",
            "- [For AI agents](#for-ai-agents) ([runs](docs/evals/README.md))",
        ),
        "cites docs/evals outside the agent section",
    );
}

#[test]
fn the_paragraph_stays_short_single_and_plain() {
    red(
        &broken_readme("has the rest.", "has the rest. One more. And one more."),
        "the eval paragraph has 7 sentences, not 3 to 5",
    );
    red(
        &broken_readme("measure models", "measure the best models"),
        "a superlative in the agent section: \"best\"",
    );
    red(
        &broken_readme("On the hard tier, on main", "\n\nOn the hard tier, on main"),
        "the eval runs must be cited in one paragraph, found 2",
    );
}

#[test]
fn the_section_ends_at_the_next_heading_and_skips_fenced_headings() {
    let readme = "# T\n```\n## For AI agents\n```\n## For AI agents\n\nbody\n\n## Next\n\nafter\n";
    let (body, rest) = section(readme).unwrap();
    assert_eq!(body, "\nbody\n");
    assert!(rest.contains("## Next") && !rest.contains("body"));
    let (body, _) = section("## For AI agents\nlast\n").unwrap();
    assert_eq!(body, "last");
}

#[test]
fn anchors_follow_github_for_simple_headings_only() {
    let page = "# Top\n```\n## Fenced\n```\n## Success by task\n\
                ### `stale_code`: luna on `otp-late`\n## A + B\n####### Seven\n#nospace\n";
    assert_eq!(
        anchors(page),
        [
            Some("top".to_string()),
            Some("success-by-task".to_string()),
            Some("stale_code-luna-on-otp-late".to_string()),
            None,
        ]
    );
}

#[test]
fn figures_match_on_token_boundaries() {
    assert!(states("passed 0/5, every", "0/5"));
    assert!(states("0/5", "0/5"));
    assert!(!states("passed 10/5 and 0/50", "0/5"));
    assert!(!states("passed 1.0/5 and v0/5", "0/5"));
    assert!(!states("rate 0/5.5", "0/5"));
    assert!(states("rate 0/5.", "0/5"));
}

#[test]
fn code_spans_close_on_a_run_of_their_own_length() {
    assert_eq!(code_span("``a`b`` c", 0), Ok(("a`b".to_string(), 7)));
    assert_eq!(code_span("`a``b` c", 0), Ok(("a``b".to_string(), 6)));
}

#[test]
fn damaged_copies_never_touch_the_real_readme() {
    let before = std::fs::read(repo().join("README.md")).unwrap();
    let dir = fixture();
    assert_ne!(dir.path(), repo());
    edit(&dir, "README.md", |s| s.replace("475 of 480", "999 of 999"));
    assert!(check(dir.path()).is_err());
    assert_eq!(std::fs::read(repo().join("README.md")).unwrap(), before);
    assert_eq!(check(repo()), Ok(()));
}
