# Implementation record — parse the tracked-link guard with pulldown-cmark

Companion to [spec.md](spec.md), [plan.md](plan.md), [tasks.md](tasks.md).
Issue #248; start: main `c8226ff` plus the design commit `c9894d9`.

## Red before green (AC6, task T2)

The R1/G1/F1 and `:502` pins were added to the old scanner's
`tracked_container_definitions_are_checked` first. Against the hand scanner:

- `1. a\n2. [r]: ../.forge/p.md` — **fails**: `tracked destination escaped the
  guard` (`markdown_destinations` returned `[]`). This is R1/G1/F1.
- `[r]: ../.forge/p.md garbage [bad](../.forge/q.md)` (top level) — **fails**:
  the scanner returned the invalid definition's `../.forge/p.md` and skipped the
  rest of the line, so the real `q.md` link was never reported. This is the
  security residual (`:502`, `:525-531`). The `>` and `-` twins already found
  `q.md` on the old code.
- `1. a\n2. [bad](../.forge/p.md)` already passed on the old scanner (the inline
  twin was never missed).

After the swap all of them pass under the parser (`tracked_container_definitions_are_checked`,
`tracked_review_residuals_follow_the_parser`).

## Dependency (AC1)

`evals/Cargo.toml` gains `[dev-dependencies] pulldown-cmark = { version =
"=0.13.6", default-features = false }`. `git diff evals/Cargo.lock` is exactly
the two `[[package]]` blocks (`pulldown-cmark 0.13.6`, `unicase 2.10.0`) and
the `pulldown-cmark` line in `swarmail-evals`'s dependency list; `bitflags`
(2.13.2) and `memchr` (2.8.3) stayed on the root lock's versions without a
`cargo update`. `git diff --stat c8226ff -- Cargo.toml Cargo.lock deny.toml src
tests scripts .brokkr .github` is empty. The manifest header comment no
longer claims CI or pinned verify run the evals checks.

## What changed in the guard (AC2–AC10)

- `every_evals_package_is_already_in_the_root_lock` asserts the foreign set
  equals the two exact `(name, version, source)` triples, the crates.io source,
  the absence of `native-tls`, `openssl` and `pulldown-cmark-escape`, and that
  `pulldown-cmark` is named only under `[dev-dependencies]`. A second test
  (`the_foreign_package_filter_reports_everything_but_the_named_exceptions`)
  proves the pure filter reports a made-up package, another version, another
  source, a stale (absent) allowance and a listed package already in the root
  lock.
- The hand scanner and decoder are deleted (`MarkdownBlock`, `Container`,
  `markdown_blocks`, `destination`, `inline_end`, `reference_destination`,
  `markdown_unescape` and the line helpers): `grep -cE` over those names is 0.
  `markdown_destinations` walks the parser's link/image start events, then
  every `reference_definitions()` entry, sorted. `packet_destination` takes the
  parser's decoded destination and decodes nothing else.
- Diagnostics are `<path>:<start line>: <decoded destination>: tracked Markdown
  links into .forge/`, counted over bytes, violations sorted.
- `tracked_cases` takes an explicit line per refused pin and repairs by
  `forge` → `rulings`.

## Pins whose verdict moved (spec ruling 7)

- `[ok]: ../rulings/p.md\n2. [r]: ../.forge/p.md` — refused → **allowed**. A
  `2.` item cannot interrupt the paragraph the definition is taken from, so
  `[r]:` is text.
- `> [ok]: ../rulings/p.md\n2. [r]: ../.forge/p.md` — **stays refused** (line
  2). The plan expected the quoted twin to follow the unquoted one; the parser
  does not: it closes a quoted definition-only paragraph at the unquoted line,
  so `2.` opens a list and `[r]:` is a definition. Fail-closed; kept refused
  and commented in the test.
- `> [ok]: ../rulings/p.md\n>     [example](../.forge/p.md)` and
  `- [ok]: ../rulings/p.md\n      [example](../.forge/p.md)` — allowed →
  **refused**: the indented line after a definition is paragraph text, so the
  link is real. (Fixture spelled `[ok]` rather than `[r]` so the repair step
  keeps the definition valid; the construct is the same.)
- `markdown_scanner_covers_the_documented_grammar`: the three reference
  positives gained a blank line (a definition cannot start under a paragraph
  line); a used reference now counts two places (link and definition), so the
  table carries a place count. The tracked-inventory fixture
  `[bad][packet]\n[packet]: …` gained the same blank line.
- No other pin moved, and no rendered link, image or definition was lost.

## Entity conformance (AC9)

All 2125 semicolon-terminated names of `scripts/data/html-entities.json`
decode through the parser (`[x](<&name;>)`) to the snapshot's `characters`.
There are no divergences to pin. The six numeric/named refusals and the
eight-digit allowance are in `tracked_character_references_are_decoded_once`;
the `&percnt;ff` encoding error is asserted behind the parser; the raw `\` row
left `packet_paths_are_classified_without_filesystem_access` and a `\.forge`
row now sits in its allowed list.

## Released forms and hostile input (AC7, AC10)

Spec ruling 6's four forms are `tracked_review_residuals_follow_the_parser`
allowed entries with a comment. R2 constructs are refused under the parser:
`[r]: …\n---`, `[r]: …\n===`, `[r]:\n  …\n---`, and a link under `---`/`===`.
Hostile-input pins (NUL in a label, trailing and lone CR, a 1 MiB line, 10 000
`[`, and the retained 1..255 backtick stress) terminate with no wall-clock
assertion. Lone-CR documents report line 1: the guard counts `\n` only.

## Gate receipts

From `evals/` (offline, locked): `cargo fmt --all -- --check` pass; `cargo clippy
--all-targets --offline --locked -- -D warnings` pass; `cargo test --offline
--locked` pass (boundary: 19 tests).

From the repo root, locked: `cargo fmt --all -- --check` pass; `cargo clippy
--workspace --all-targets --locked -- -D warnings` pass; `cargo test
--workspace --locked` pass; `bash scripts/coverage-gate.sh` pass (exact
100/100/100); `bash scripts/coverage-gate.sh selftest` pass; `bash
scripts/publish-guard-test.sh` pass; `bash scripts/seal-branch-test.sh` pass;
`cargo deny check licenses` pass.

**Not run: `brokkr compile --bundle .brokkr/recipes/self`.** The pinned
brokkr (0.12.0, `tooling.json`) is not installed in this seat's box and there is
no network to fetch it; `brokkr` is not on PATH. Nothing in `.brokkr/` changed
(protected diff above is empty), so the verify seat's run is the evidence.

## Licenses (AC11)

The root `cargo deny check licenses` reads the root lock only; it does not read
`evals/Cargo.lock`. Evidence for the evals tree: `cargo deny --manifest-path
evals/Cargo.toml --offline check licenses` and the same with `--config
deny.toml` both report `licenses ok` (one warning: the unmatched
`CDLA-Permissive-2.0` allowance, which the evals tree does not use).
`cargo deny … list` omits dev-dependencies, so it does not show the new
crates; a probe run with a restrictive config (Apache-2.0 only) named
`pulldown-cmark` in its report and `--exclude-dev` removed it, which shows the
check covers the dev-dependency. pulldown-cmark is MIT, unicase MIT OR
Apache-2.0.

## Not covered

CI and the verify seat do not run the evals suite (decision 0055, #201); the
evals commands above are this seat's evidence for reviewers to reproduce.
Nothing was pushed or merged; no decision was accepted.
