# Plan — parse the tracked-link guard with pulldown-cmark

Companion to [spec.md](spec.md); tasks in [tasks.md](tasks.md).

## Technical context

- Language and crate: Rust 2024, `evals/` standalone workspace (accepted
  0055). Root crate untouched.
- New dev-dependency: `pulldown-cmark = "=0.13.6"`, `default-features =
  false` (drops `getopts` and `html`, so `pulldown-cmark-escape` never
  enters the tree). Its closure is `bitflags 2` (root lock: 2.13.2),
  `memchr 2.5+` (root lock: 2.8.3) and `unicase 2.6+` (new: 2.10.0). All four
  crate files are already in the local registry cache
  (`~/.cargo/registry/cache/*/pulldown-cmark-0.13.6.crate`,
  `unicase-2.10.0.crate`), so `--offline --locked` resolves without network.
- Parser API used (pulldown-cmark 0.13.6 `src/parse.rs`):
  `Parser::new_ext(&str, Options)`, `into_offset_iter()` yielding
  `(Event, Range<usize>)`, `OffsetIter::reference_definitions() -> &RefDefs`,
  `RefDefs::iter() -> (label, &LinkDef { dest: CowStr, title, span: Range })`.
  `RefDefs` wraps a `HashMap`: iteration order is unspecified, so the guard
  sorts.
- Licenses: pulldown-cmark MIT, unicase MIT OR Apache-2.0; both already in
  `deny.toml`. No allowlist change.

## Constitution check

- Frozen guarantees 0001–0005: not touched. No `src/` change, no root
  dependency, no root lock change. The exact root coverage gate's
  denominator is unchanged; evals is outside it by 0055.
- Dependency policy: the evals-only exception is the operator's 2026-10-30
  ruling, recorded as a dated amendment to accepted 0055. The test that
  enforces the root-lock rule stays, with an exact two-entry exception.
- Decision discipline: no new decision document; the spec cites 0055 as
  authority. Only the operator accepts; the amendment records a ruling
  already given.
- Signing: commits unsigned (`commit.gpgsign=false`), never pushed (0056).
- Gates: all root gates `--locked`; evals fmt/clippy/test `--offline
  --locked`; evals license check by explicit manifest path (root `cargo deny`
  reads only the root lock).

## Project structure and files touched

| File | Change |
|---|---|
| `evals/Cargo.toml` | add `[dev-dependencies]` with the exact pin; rewrite the header comment: packages reuse root versions except the named 0055 exceptions, and the evals checks are seat-run until #201 (not CI/verify) |
| `evals/Cargo.lock` | the two new `[[package]]` blocks and the `pulldown-cmark` line in `swarmail-evals`'s dependency list; nothing else |
| `evals/tests/boundary.rs` | invariant test with the exception list; delete lines 125–662 (hand scanner and decoder); new `markdown_destinations`; `packet_destination` without `markdown_unescape`; sorted violations; `tracked_cases` with explicit lines and spelling-agnostic repair; retained, moved and new pins; entity conformance through the parser; hostile-input pins; module doc |
| `evals/README.md` | the "every package … is already in the root lock" sentence gains the named exception clause and points at the 0055 amendment |
| `docs/decisions/0055-agent-eval-harness.md` | appended dated amendment section only |
| `docs/decisions/README.md` | 0055 row summary gains "amended 2026-10-30: named evals dev-dependency exceptions" |
| `specs/commonmark-parser-guard/implementation.md` | implement seat's record: red/green evidence, pin flips with justification, gate receipts, entity divergences if any |

Untouched: root `Cargo.toml`, `Cargo.lock`, `deny.toml`, `src/`, `tests/`,
`scripts/`, `.brokkr/`, `.github/`, `scripts/data/html-entities.json`.

## Research and implementation approach

### 1. Dependency and lock (AC1)

Add the pin, then resolve offline from `evals/`:

```bash
cd evals && cargo metadata --offline --format-version 1 >/dev/null
git diff -- Cargo.lock
```

The diff must be exactly: `[[package]] name = "pulldown-cmark" version =
"0.13.6"` with dependencies `bitflags`, `memchr`, `unicase`; `[[package]]
name = "unicase" version = "2.10.0"`; and `"pulldown-cmark"` added to the
`swarmail-evals` dependency list. If `bitflags` or `memchr` resolve to any
other version than the root lock's, run `cargo update --offline -p bitflags
--precise 2.13.2` / `-p memchr --precise 2.8.3`. The positions' trial found
no such correction needed; the diff is the proof either way.

### 2. Invariant test (AC2)

```rust
/// Operator-approved evals dev-dependencies outside the root lock
/// (0055, dated amendment 2026-10-30, #248). Exact triples: a different
/// version, a path or git override, or a third crate all fail.
const APPROVED_FOREIGN: [(&str, &str, &str); 2] = [
    ("pulldown-cmark", "0.13.6", CRATES_IO),
    ("unicase", "2.10.0", CRATES_IO),
];
```

Factor the filter into a pure `foreign_packages(ours, theirs) -> BTreeSet`
and assert `foreign == APPROVED_FOREIGN` as sets (both directions). Keep the
`native-tls`/`openssl` absence line and add `pulldown-cmark-escape`. Read
`evals/Cargo.toml`, split at `[dev-dependencies]`, and assert
`pulldown-cmark` occurs only after that header. Unit-assert the pure filter
with a fabricated foreign triple (reported) and with a listed triple missing
from `ours` (set inequality). `CRATES_IO` is the literal
`registry+https://github.com/rust-lang/crates.io-index` already in the lock.

### 3. The guard (AC4, AC5, AC8)

```rust
// The parser owns CommonMark: blocks, containers, code, inlines, escapes and
// character references. Definitions need their own walk because a definition
// nothing uses emits no event, and the guard refuses those too.
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
```

`check_tracked_markdown` keeps its shape; the line is
`text.as_bytes()[..offset].iter().filter(|&&b| b == b'\n').count() + 1`,
and `violations.sort()` precedes the join. `packet_destination` starts from
`let path = target;` — the entity and backslash decoding is gone with
`markdown_unescape`. No GFM options: a table-row link is still an event
without them, and extensions would make block parsing depend on a feature
flag.

`tracked_cases(refused: &[(&str, &str, usize)], allowed: &[&str])`: the
third field is the expected line. Repair stays spelling-agnostic:
`source.replace("forge", "rulings")` turns `.forge`, `&period;forge`,
`&#46;forge` and `&percnt;2eforge` into allowed `rulings` paths without
knowing the raw spelling, so every refused pin still proves that the
repaired document passes.

### 4. Pin corpus (AC5–AC10)

Work the existing `tracked_*` tests in place; do not rename them, so the
#245 traceability survives.

- **Retained refused entries** keep their source and gain an explicit line.
  Decoded targets replace raw ones where entities appear (for example
  `("[bad](../&#46;forge/p.md)", "../.forge/p.md", 1)`).
- **Three flips (spec ruling 7)** move between lists with a comment naming
  the ruling and the CommonMark rule. The `>`-quoted `2.` sibling moves with
  the first if the parser agrees; if the parser refuses it, keep it refused
  and record the parser's reading in the implementation record.
- **Grammar test** (`markdown_scanner_covers_the_documented_grammar`): the
  three reference positives gain a blank line between use and definition;
  the entity rows that used to prove the decoder become parser rows; the
  `r"../.forge/packet\(1\).md"` expectation becomes the decoded
  `../.forge/packet(1).md`.
- **New refused pins (AC6):** `1. a\n2. [r]: ../.forge/p.md` (line 2),
  `1. a\n2. [bad](../.forge/p.md)`, `[r]: ../.forge/p.md garbage
  [bad](../.forge/q.md)` targeting `../.forge/q.md` at top level, under `>`
  and under `-`; R2 constructs: `[r]: ../.forge/p.md\n---`,
  `[r]:\n  ../.forge/p.md\n---`, `[bad](../.forge/p.md)\n===`, and
  `[r]: ../.forge/p.md\n===` with whichever verdict the parser gives,
  commented.
- **New allowed pins (AC6, AC7):** `[r]: ../.forge/p.md trailing`,
  `para\n[r]: ../.forge/p.md`, `[r]: ../rulings/p.md\n[r]: ../.forge/p.md`
  (shadowed duplicate), `<div>\n[bad](../.forge/p.md)\n</div>`,
  `<!-- [bad](../.forge/p.md) -->`, `[a [b](../rulings/in)](../.forge/out)`
  — and its refused twin `[a [b](../.forge/in)](../rulings/out)` for the
  inner link.
- **Entity rows (AC9):** refused `&period;`, `&sol;`, `&percnt;` (as
  `&percnt;2eforge`), `&#46;`, `&#0000046;`, `&#x00002e;`; allowed
  `&#00000046;`, `&amp;period;`, `&bsol;`, `&PERIOD;`, the existing
  no-semicolon and over-long forms. `markdown_reference_decoder_obeys_
  commonmark_boundaries` becomes `markdown_parser_decodes_the_pinned_entity_
  table`: for each semicolon-terminated snapshot name, `markdown_destinations
  (&format!("[x](<{name}>)"))` yields one destination equal to
  `characters`; count 2125. The percent-encoding error assertion moves to
  `[x](../&percnt;ff)` through `markdown_destinations` then
  `packet_destination`.
- **Diagnostics test (AC8):** one new test asserting exact
  `name:line: decoded` for a multi-line definition (line of `[r]:`), a CRLF
  document, a multibyte-prefixed line and an entity spelling.
- **Hostile input (AC10):** `[a\0b](../.forge/p.md)` refused,
  `"[bad](../.forge/p.md)\r"` and `"a\rb\r[bad](../.forge/p.md)"` refused
  with the parser's line, a 1 MiB prose line followed by a refused link,
  `"[".repeat(10_000) + "[bad](../.forge/p.md)"` refused, and the existing
  backtick-run stress retained.

### 5. Red-before-green (AC6)

Before deleting the scanner, add the R1/G1/F1 pin and the `:502` pin and
run `cargo test --offline --locked --test boundary tracked_` from `evals/`
against the old code; record the failing assertions in
`implementation.md`. Then swap the scanner and rerun green.

### 6. Documents (AC3)

Append to `docs/decisions/0055-agent-eval-harness.md`:

```markdown
## Dated amendment — 2026-10-30, named evals dev-dependency exceptions (#248)

The operator ruled on 2026-10-30 (chat direction to the dispatcher) that
`evals/Cargo.lock` may hold operator-approved **dev-dependencies by name**,
each pinned exactly and resolved from the local cargo cache so
`--offline --locked` still needs no network. The named exceptions are
`pulldown-cmark 0.13.6` (`default-features = false`, no
`pulldown-cmark-escape`) and its transitive `unicase 2.10.0`; `bitflags`
and `memchr` keep reusing the root lock. The invariant test
`every_evals_package_is_already_in_the_root_lock` carries that list as
exact (name, version, source) triples and fails for any other package not
in the root lock, for a listed package at another version or source, and
for `pulldown-cmark` anywhere but `[dev-dependencies]`. Root Cargo.toml,
Cargo.lock and deny.toml are unchanged; the root `cargo deny check
licenses` does not read the evals lock, so the evals license check runs by
manifest path. CI/verify wiring for evals remains deferred to #201: the
guard is seat-run. The body above is unchanged; this amendment adds one
bounded exception and accepts nothing.
```

The index row for 0055 keeps its status cell; its summary cell ends with
"; amended 2026-10-30 — named evals dev-dependency exceptions (#248)".
`evals/README.md` lines 51–52 read: every package is in the root lock at
the same version and source except the named 0055 exceptions, checked by
`tests/boundary.rs`.

## Data model and contracts

- `markdown_destinations(&str) -> Vec<(usize, String)>`: byte offset of the
  construct start, decoded destination; sorted; one entry per link, image
  or definition; a used reference yields both its link event and its
  definition (two real places to fix).
- `packet_destination(&Path, &str) -> Result<bool, String>`: unchanged
  signature; input is already CommonMark-decoded.
- Refusal line: `<relative path>:<line>: <decoded destination>: tracked
  Markdown links into .forge/`, lines sorted.
- `APPROVED_FOREIGN`: `[(&str, &str, &str); 2]`; the lock's foreign set
  must equal it.

## Quickstart and validation

From `evals/` (implement seat and reviewers; CI does not run these):

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --offline --locked -- -D warnings
cargo test --offline --locked --test boundary
cargo test --offline --locked
```

From the repo root:

```bash
cargo deny --manifest-path evals/Cargo.toml --offline check licenses   # pass --config deny.toml if it is not picked up
git diff --stat c8226ff -- Cargo.toml Cargo.lock deny.toml src tests scripts .brokkr .github   # must be empty
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
brokkr compile --bundle .brokkr/recipes/self
bash scripts/coverage-gate.sh && bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
bash scripts/seal-branch-test.sh
cargo deny check licenses
```

Record each command and its status in `implementation.md`. State plainly
that the root license gate does not cover `evals/Cargo.lock` and that the
manifest-path run is the evidence.

## Risks

| Risk | Mitigation |
|---|---|
| The parser disagrees with the pinned WHATWG snapshot for some entity name, so the 2125-row conformance loop fails | Do not weaken the loop. Record each divergent name in `implementation.md`, pin it explicitly with the parser's output, and report it as a finding; the guard's verdict only depends on names decoding to `.`, `/` or `%`, so the exposure to the delivery-state check is bounded and named. |
| A pin flip beyond the three listed surfaces during the swap | Allowed only as a rendered link gained or a non-link losing a false refusal; every flip is justified by CommonMark rule in `implementation.md`. A lost rendered link, image or definition is a stop, not a flip. |
| `bitflags`/`memchr` resolve away from the root lock's versions | `cargo update --offline -p <crate> --precise <root version>`; AC1's lock diff is the proof. |
| Cold cargo cache on another machine makes `--offline --locked` fail | A loud failure, not a flaky one. The 0055 amendment names the precondition; the crates are standard crates.io artifacts. |
| `cargo deny` by manifest path does not find the root `deny.toml` from `evals/` | Pass `--config deny.toml` explicitly; a run that reports 150+ rejections has read no config and is not evidence. |
| The guard still runs nowhere automated (0055 defers CI to #201) | Not widened here. The spec, the Cargo.toml comment, the README and the amendment all say the guard is seat-run; the implement seat records the evals commands; reviewers reproduce them. |
| Unordered `RefDefs` iteration makes failure text non-deterministic | `found.sort()` and `violations.sort()`; the diagnostics test asserts exact text. |
| Deep nesting overflows the parser's stack and aborts the test binary | Accepted fail-closed failure on review-gated tracked docs; no pre-scan. |
| Shadowed duplicate definitions into `.forge/` go unreported until they become live | Ruled and pinned as allowed (spec ruling 6); the live form is refused. A separate text lint is the operator's call. |
| A future parser bump silently changes guard behaviour | Exact `=0.13.6` pin, `--locked`, and the exact-triple exception test turn a bump into a reviewed edit that must re-run the corpus and the entity conformance loop. |
