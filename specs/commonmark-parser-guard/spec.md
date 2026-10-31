# Specification — parse the tracked-link guard with pulldown-cmark

- Created: 2026-10-30; status: committed design for implementation.
- Feature: issue #248, follow-up to #245 (run
  `epic-192-follow-up-issue-245-clo-52600d1d`, `REVIEW-RESIDUAL-OK`).
- Scope: `evals/` only — `evals/tests/boundary.rs`, `evals/Cargo.toml`,
  `evals/Cargo.lock`, `evals/README.md`, and a dated amendment appended to
  accepted [0055](../../docs/decisions/0055-agent-eval-harness.md) plus its
  index row.

## What and why

The tracked-document guard in `evals/tests/boundary.rs` refuses any tracked
Markdown link, image or reference definition whose destination normalizes
into repository-root `.forge/` (the operator ruling recorded in
[`docs/rulings/epic-192.md`](../../docs/rulings/epic-192.md)). #245 fixed four
CommonMark gaps in the guard's hand-written scanner; its review then found
five more in the same code: ordered-list siblings treated as paragraph
interruptions (R1, G1, F1 at `boundary.rs:296-305`, `:322`), the
thematic/setext branch (R2 at `:342-344`), and a definition locator that
returns the physical line end for an invalid bare-line definition the `legacy`
flag accepts, so `markdown_destinations` skips the rest of that line
(security seat, non-security, `:502`, `:525-531`). Every review of the hand
scanner finds a new edge case, and its misses are silent.

This change deletes the re-implemented CommonMark — block scanning, inline
scanning, the definition locator and the escape/entity decoder — and walks
the real link, image and reference-definition events of `pulldown-cmark`
0.13.6, approved by the operator on 2026-10-30 as a dev-dependency of the
evals crate only. The guard keeps every link form it catches today, gains the
forms the review found, and stops refusing lines CommonMark renders as text.

## Authority and scope boundaries

The framing is the run's task block: issue #248 and the operator's four
decisions of 2026-10-30. `.brokkr/state/tasks/` does not exist in this
worktree; both panel positions under `.brokkr/state/design/positions/` were
read and are run-local evidence, not deliverables.

Accepted [0055](../../docs/decisions/0055-agent-eval-harness.md) rules that
every evals dependency at every depth reuses a root-locked version. The
operator ruled on 2026-10-30 to **amend 0055 with named exceptions**: a dated
amendment appended to 0055 (its body is not rewritten) records that
`evals/Cargo.lock` may hold operator-approved dev-dependencies by name, and
the invariant test carries an explicit minimal list of those names so any
other unlocked package still fails. That amendment is the vehicle the
operator named; this spec declares no new decision document. Accepted
[0049 §2](../../docs/decisions/0049-extraction-compat.md) still owns
`scripts/data/html-entities.json`; evals keeps reading it for the parser
conformance proof and the file does not move. Accepted
[0050](../../docs/decisions/0050-delivery-gates.md) proof ownership and
[0056](../../docs/decisions/0056-signing-policy.md) unsigned seat commits are
preserved. Frozen guarantees 0001–0005 are untouched: nothing under `src/`
changes, the root `Cargo.toml`, `Cargo.lock` and `deny.toml` stay byte-identical,
and the exact root coverage denominator does not move.

Out of scope, stated so nobody mistakes silence for coverage: raw HTML
anchors (`<a href>`), footnote bodies, GFM extensions, the `](` splitter in
`sequential_contracts_and_decision_links_are_honest` (a filesystem-existence
check, not the delivery-state guard; it stays as is), and wiring evals into
CI or the verify seat (deferred by 0055 to #201). The guard remains proven by
the implement seat running the evals suite and by reviewers reproducing it;
no artifact of this change may claim CI runs it.

## Panel synthesis and rulings

Both positions adopt the parser and delete the hand scanner. Where they
agree the spec adopts them without comment. The rulings below are where they
diverge or where one position raised a point the other did not.

1. **Unused definitions are checked (robustness B1; simplicity agrees).**
   `Event::Start(Tag::Link | Tag::Image)` walks cover rendered links only.
   The guard also reads `Parser::reference_definitions()` and checks every
   `LinkDef.dest`, used or not, reporting `LinkDef.span.start`. A pure event
   walk would silently drop the container-definition pins of #245.

2. **One decoder: delete `markdown_unescape` (both).** The parser's
   `dest_url` and `LinkDef.dest` are already CommonMark-decoded.
   `packet_destination` receives that string and keeps only its URL logic:
   scheme detection, `//` authority, `#`/`?` cut, percent decoding and `..`
   clamping. A second decode would flip `&amp;period;` and `&bsol;` from
   allowed to refused.

3. **Entity proof moves to the parser; the snapshot stays (robustness over
   simplicity).** Simplicity's six parser rows are adopted as tracked pins,
   and in addition the 2125-name loop over `scripts/data/html-entities.json`
   is kept, now driving the parser: for every semicolon-terminated name,
   `[x](<&name;>)` must decode to the snapshot's `characters`. Cost to
   simplicity: one loop survives. Reason: the guard's verdict now depends on
   the parser's entity table, and this is the only test that makes a
   parser-bump decoding regression loud. If the pinned parser disagrees with
   the snapshot for any name, the implementer records the divergence by name
   in the implementation record and pins it explicitly; the loop must not be
   weakened to hide it.

4. **Exception list is exact triples, not bare names (robustness over
   simplicity).** The operator asked for "an explicit, minimal list of those
   names". The invariant test's key is already `(name, version, source)`, so
   the list is `("pulldown-cmark", "0.13.6", crates.io)` and
   `("unicase", "2.10.0", crates.io)` — the names are there, and the version
   and source cannot drift under a `[patch]` or a path override. The test
   asserts that the foreign set equals the exception set exactly: a stale
   allowance, a third crate such as `pulldown-cmark-escape`, or a different
   version all fail. It also asserts `pulldown-cmark` appears only under
   `[dev-dependencies]` in `evals/Cargo.toml`. Cost: a parser bump edits the
   test. That is the point — a parser bump is a guard-behaviour change and
   must be a reviewed edit.

5. **Diagnostics report the construct's start line and the decoded
   destination (both).** Today's message echoes the raw spelling and the
   destination's own line; the byte-to-source map that provided it is the
   code the `:502` residual lives in. The new message is
   `<path>:<line of the link, image or definition start>: <decoded
   destination>: tracked Markdown links into .forge/`. No index arithmetic
   re-derives a destination offset inside a definition. Line counting uses
   `text.as_bytes()[..offset]` so an off-boundary offset cannot panic the
   guard. The refused pins state their expected line explicitly instead of
   computing it with `rfind`.

6. **Four non-link forms the old scanner refused are knowingly released
   (simplicity's list, ruled here).** "Keep every link form" means every form
   that renders as a link, image or definition. Each of these is pinned as
   allowed with a comment so the choice is on record:
   - an invalid bare-line definition (`para\n[r]: ../.forge/p.md`,
     `[r]: ../.forge/p.md trailing prose`): CommonMark text; this is exactly
     the `legacy` leniency the security residual lives in;
   - a shadowed duplicate definition (`[r]: ok\n[r]: ../.forge/p.md`): the
     closest call — a packet path in definition syntax that no rendered link
     can reach. pulldown-cmark keeps the first definition only; recovering
     the second means a hand-written definition parser, which is the code
     the operator told us to delete. Cost: such a line sits unreported until
     an edit makes it live, at which point the guard refuses it. The repo's
     557 tracked documents contain none today;
   - HTML blocks and comments (`<div>\n[bad](../.forge/p.md)\n</div>`,
     `<!-- [bad](../.forge/p.md) -->`): literal HTML, not a Markdown link;
   - the outer bracket pair around a nested link (`[a [b](in)](out)`): links
     cannot contain links, so `out` is text; the inner link is still checked.
   If the operator wants any of these refused, that is a separate plain-text
   lint, not parser work.

7. **Existing pins whose verdict changes are listed, justified and
   bounded.** The parser is the authority; the existing corpus is the
   acceptance net. From the simplicity differential, three pins flip:
   `[ok]: ../rulings/p.md\n2. [r]: ../.forge/p.md` becomes allowed (a `2.`
   item cannot interrupt the paragraph the definition is extracted from, so
   `[r]:` is text — its `>`-quoted sibling is expected to follow by the same
   lazy-continuation rule), and `> [r]: ok\n>     [example](../.forge/p.md)`
   and `- [r]: ok\n      [example](../.forge/p.md)` become refused (the
   indented line after a definition is paragraph text, so the link is real).
   Three `markdown_scanner_covers_the_documented_grammar` positives
   (`[full][r]`, `[collapsed][]`, `[shortcut]`) place the definition on the
   line after a paragraph, where it cannot start; they gain a blank line so
   the link form stays pinned on a valid fixture. Any further flip the
   implementer finds is recorded in the implementation record with its
   CommonMark justification, and is acceptable only as a rendered link gained
   or a non-link text losing a false refusal — never a rendered link,
   image or definition lost.

8. **R2 is pinned by construct, not by recovered text.** The issue gives
   R2 only as the thematic/setext branch at `:342-344`; the code is deleted.
   Pins cover a definition followed by `---` and by `===`, a multi-line
   definition closed by `---`, and a paragraph link under a setext underline,
   with the parser's verdicts.

9. **Hostile-input pins are bounded (robustness, trimmed).** NUL inside a
   label, CRLF and lone-CR documents, a single very long line, ten thousand
   `[` before a link, and the existing 1..255 backtick-run stress. A
   stack-overflow abort on absurd nesting is an accepted fail-closed test
   failure; no pre-scan is added to prettify it.

10. **The stale CI claim in `evals/Cargo.toml` is corrected.** Its comment
    says CI and pinned verify run the evals checks; `.github/` names evals
    nowhere and 0055 defers that wiring. The comment is reworded to match
    `evals/README.md` ("until #201"). No workflow changes.

11. **Decisions index.** The 0055 row does not summarise the root-lock rule,
    so the operator's condition for rewriting it is not met; the row's
    summary gains a short pointer to the dated amendment so a reader of the
    index knows 0055 has one.

## Acceptance Criteria

- **AC1 — Dependency boundary.** `evals/Cargo.toml` gains exactly
  `pulldown-cmark = { version = "=0.13.6", default-features = false }` under
  a `[dev-dependencies]` section. `evals/Cargo.lock` gains exactly the
  packages `pulldown-cmark 0.13.6` and `unicase 2.10.0` from crates.io, with
  `bitflags` and `memchr` resolving to the root lock's versions (`2.13.2` and
  `2.8.3`). Root `Cargo.toml`, `Cargo.lock` and `deny.toml` are byte-identical
  to main `c8226ff`. Proof: `git diff --stat c8226ff -- Cargo.toml Cargo.lock
  deny.toml` is empty; `git diff c8226ff -- evals/Cargo.lock` shows only the
  two `[[package]]` blocks and the one dependency-list line.

- **AC2 — Invariant test with named exceptions.**
  `every_evals_package_is_already_in_the_root_lock` carries a constant list
  of exactly the two `(name, version, source)` triples above, and asserts:
  (a) every package of `evals/Cargo.lock` is in the root lock or in the list;
  (b) the set of packages not in the root lock equals the list exactly, so a
  listed package missing from the lock, present in the root lock, or any
  unlisted foreign package fails; (c) each listed source is the crates.io
  registry; (d) `pulldown-cmark-escape`, `native-tls` and `openssl` are
  absent; (e) `pulldown-cmark` is named in `evals/Cargo.toml` only under
  `[dev-dependencies]`. Proof: a unit assertion on the pure filter with a
  made-up foreign package reports it, and one with a listed package absent
  from the lock fails.

- **AC3 — Amendment to 0055.** A dated section
  `## Dated amendment — 2026-10-30, named evals dev-dependency exceptions
  (#248)` is appended to `docs/decisions/0055-agent-eval-harness.md`; no
  earlier text of the file changes. It records the operator's 2026-10-30
  ruling, the two named crates with exact versions, dev-dependency-only
  placement, that every unlisted package still refuses, the local-cache
  precondition for `--offline --locked`, and that evals enforcement stays
  seat-run until #201. The 0055 row in `docs/decisions/README.md` keeps its
  status cell and gains a pointer to the amendment. Proof: `git diff
  c8226ff -- docs/decisions/0055-agent-eval-harness.md` is additions only;
  `sequential_contracts_and_decision_links_are_honest` stays green.

- **AC4 — The hand scanner is gone.** `evals/tests/boundary.rs` no longer
  defines `MarkdownBlock`, `Container`, `spaces`, `quote_prefix`,
  `list_prefix`, `fence_marker`, `thematic`, `heading`, `setext`,
  `markdown_blocks`, `skip_space`, `destination`, `inline_end`,
  `reference_destination` or `markdown_unescape`. `markdown_destinations`
  walks `Parser::new_ext(text, Options::empty()).into_offset_iter()` for
  `Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. })`,
  then every `(label, LinkDef)` of `reference_definitions()`, and returns
  `(start offset, decoded destination)` sorted by offset then destination.
  Proof: `grep -c` for each deleted name is zero; the grammar test and every
  `tracked_*` test pass.

- **AC5 — Every rendered link form the guard catches today is still
  refused.** Inline links and images with bare, angle-bracketed and titled
  destinations; full, collapsed and shortcut references with their
  definitions; definitions with no use at all, at top level and inside `>`,
  `-`, `+`, `*`, `1.`, `12)`, nested `> - 1. >`, tab-prefixed and multi-line
  forms; destinations containing backticks; entity and numeric references
  decoded once. Code spans, fenced and indented code stay allowed. Proof:
  every refused entry of today's `tracked_*` tests is retained (with the
  three flips of ruling 7 moved and justified in a comment) and
  `tracked_markdown_never_links_into_delivery_state` passes on the tracked
  tree.

- **AC6 — The review's edge cases are caught by the parser.**
  `1. a\n2. [r]: ../.forge/p.md` (R1/G1/F1) and its inline twin
  `1. a\n2. [bad](../.forge/p.md)` are refused; the R2 constructs of ruling 8
  are pinned; `[r]: ../.forge/p.md garbage [bad](../.forge/q.md)` is refused
  for `../.forge/q.md` at top level, inside `>` and inside `-` (the `:502`
  skip); `[r]: ../.forge/p.md trailing` with no link is allowed. Proof: each
  is a `tracked_cases` entry; the implementation record shows the R1/G1/F1
  and `:502` entries failing against the old scanner before the swap.

- **AC7 — Released non-link forms are pinned as allowed.** The four forms of
  ruling 6 appear in an allowed list with a comment naming the ruling.
  Proof: they are `tracked_cases` allowed entries.

- **AC8 — Diagnostics.** A refusal reads
  `<path>:<line>: <decoded destination>: tracked Markdown links into .forge/`
  where `<line>` is the construct's start line. Proof: `tracked_cases` takes
  an explicit expected line per refused pin; one dedicated test pins a
  multi-line definition reporting the `[r]:` line, a CRLF document, and a
  multibyte prefix, plus the decoded spelling for an entity fixture. Output
  is deterministic: violations are sorted before joining.

- **AC9 — Entity conformance.** For every semicolon-terminated name in
  `scripts/data/html-entities.json`, the parser's destination for
  `[x](<&name;>)` equals the snapshot's `characters`, 2125 names counted.
  The six parser pins `&period;`, `&sol;`, `&percnt;`, `&#46;`,
  `&#0000046;`, `&#x00002e;` are refused and `&#00000046;` (eight digits)
  is allowed. `packet_destination` performs no entity or backslash decoding:
  `../&percnt;ff` is no longer an encoding error at that layer (the parser
  pin `[x](../&percnt;ff)` is), and the raw `\` row leaves the
  `packet_paths_are_classified_without_filesystem_access` list.

- **AC10 — Hostile input.** The pins of ruling 9 terminate and classify
  correctly, with no wall-clock assertion.

- **AC11 — Gates.** From the repo root, every house gate of
  `docs/house-rules.md § Gates` passes `--locked`. From `evals/`:
  `cargo fmt --all -- --check`, `cargo clippy --all-targets --offline
  --locked -- -D warnings` and `cargo test --offline --locked` pass. Root
  `cargo deny check licenses` does not read `evals/Cargo.lock`; the
  implementation record says so and shows `cargo deny --manifest-path
  evals/Cargo.toml --offline check licenses` passing at the real `evals/`
  path with the root `deny.toml`. Commits are unsigned; nothing is pushed.

## External proof ownership

CI and the verify seat do not run the evals suite (0055, #201). The
evals fmt, clippy, test and license runs are the implement seat's recorded
evidence and the reviewers' reproduction. Acceptance of the 0055 amendment's
wording as a standing rule, like every acceptance, is the operator's; the
amendment records a ruling the operator already gave in chat on 2026-10-30.
