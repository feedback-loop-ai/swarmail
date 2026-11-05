# Implementation ledger — built-in deterministic spam signals (#260)

Spec: [spec.md](spec.md) · Tasks: [tasks.md](tasks.md) · Decision: proposed
[0122](../../docs/decisions/0122-deterministic-spam-rules.md). Base `main` 8732445.

## What shipped

- `src/deliverability/spam.rs`: the six rules as typed compile-time data
  (`Signal`, exhaustive `id`/`severity`/`hint`), the 14-phrase ASCII-mode
  `regex::bytes` alternation built once, one pass zero-width count, first-wins
  Subject/From from the root header list, and an eligible-text walk modelled on
  `html.rs` that sums decoded eligible text against `SPAM_TEXT_BYTES`
  (4,194,304) before any rule matches. At most one finding per rule.
- `src/deliverability.rs`: `mod spam;`, one call `spam::collect(raw, out)?;` after
  `html::collect`, `RULE_SET_ID = "swarmail-spam/1"`.
- Inline tests in `spam.rs` (15), wire tests in `tests/spam_rules.rs` (5), the
  guide section (`docs/guide/deliverability.md`, mirrored in `src/static/llms.txt`),
  OpenAPI and modern MCP descriptor text, the 0122 index row, and the testing
  guide's suite-table row for the new suite (a docs test requires it).
- No weight and no total anywhere in `src/` (AC10/AC16).

## T1 — audit of existing expectations (AC12)

Every existing expected report in `tests/`, the inline tests and
`tests/fixtures/mcp-structured-output/` was checked by running the whole suite
against the new collector. Those suites compare whole reports, so a spam finding
on any existing fixture would have failed them. None did: no existing
Subject/From/text body trips a spam rule. The only edits to existing expectations
are the identity token, plus two fixtures that tested the engine's 8 MiB raw bound
with an 8 MiB `text/plain` body. The new, normative 4 MiB text cap (spec "Bounds")
fails such a body first, so:

- `tests/bulk_sender.rs` `bulk_analysis_budget_boundaries_and_incomplete_mime`:
  the `raw` fixture's content type is now `application/octet-stream` (was the
  defaulted `text/plain`). It still pins 200 at 8,388,608 bytes and 503 at +1.
- `src/deliverability/html.rs` `raw_decoded_structural_and_contribution_limits_are_exact`:
  the 8 MiB raw-bound body is `Content-Type: application/octet-stream` (was
  `text/plain`), same boundaries.

Neither change loosens a rule; each moves a raw-bound test off the surface the
new, documented text cap owns. The text cap itself is proved in
`spam::tests::text_cap_is_exact` and `oversized_text_fails_closed`.

## AC6 — remaining `swarmail-html/3` matches on the final tree

`git grep -n 'swarmail-html/3'` — every match is historical text; `src/` code,
`html-rules.json`, `openapi.json`, MCP descriptors and test assertions name
`swarmail-spam/1`.

| match | class |
|---|---|
| `docs/decisions/0082`, `0105`, `0107` | decision records (history) |
| `docs/releases/v0.2.0.md` | release notes (history) |
| `docs/guide/deliverability.md`, `src/static/llms.txt` version-history line | the guide's rule-set version history |
| `specs/combined-report-review`, `issue259-clean-followup`, `mcp-report-output`, and this spec's own spec/plan/tasks | other (and this) specs describing the past edit |
| `tests/fixtures/mcp-structured-output/README.md`, `march-current-main.provenance.json` | fixture provenance prose describing the past `html/3` edit (each now also records the 2026-11-03 `spam/1` edit) |

One-token edits (not re-captures, per the recorded precedent) were made in
`tests/fixtures/mcp-structured-output/march-current-main.jsonl` (the seed report
row) and `openapi-report-source.json`; the provenance's `fixture_sha256` and a
dated `updated` note follow. `tests/fixtures/html-compatibility.json` carries the
token in its catalogue snapshot, which `html-rules.json` must equal.

## Parser facts the implementation relies on (mail-parser 0.11.9)

- A defaulted part (no Content-Type) is `PartType::Text`; a `multipart/digest`
  child without Content-Type is `PartType::Message`, so it is never eligible.
  `text/csv` and other text types are also `PartType::Text`, so eligibility checks
  the content type explicitly.
- Matching `PartType::Text` directly means no `text_contents()` `None` arm exists
  that no input could reach.
- A From address list is never empty; the display-name rule matches the first
  mailbox with a slice pattern, so an empty list needs no separate arm.
- A comment after a bare address (`a@b.co (c@d.co)`) is parsed as the display
  name; the guide lists this as a known false positive.

## Bounds

Raw 8,388,608 bytes, header block 65,536, 256 parts, depth 32 (existing
admission); aggregate decoded eligible text 4,194,304 (new, exact at 4,194,304 and
failing at 4,194,305 and across parts); at most 6 spam findings. Proven with the
oversized fixtures above. No load or timing test was run; none was applied (no
CPU workers, no minutes of load).

## Gate receipts (final tree)

Run locally on the final tree, all `--locked`, offline:

| gate | outcome |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | clean |
| `cargo test --workspace --locked` | all suites pass (lib 545 incl. 15 new, `spam_rules` 5) |
| `cargo build --locked --bin swarmail` | built |
| evals: fmt, clippy `-D warnings`, `SWARMAIL_BIN=<candidate> cargo test --locked` | clean; every suite passes |
| `bash scripts/coverage-gate.sh` | pass, exact 100/100/100 (lines/branches/functions) |
| `bash scripts/coverage-gate.sh selftest` | pass |
| `bash scripts/publish-guard-test.sh` | OK |
| `bash scripts/seal-branch-test.sh` | 7 cases and 10 mutants passed |
| `cargo deny check licenses` | licenses ok |
| every-recipe `brokkr compile` | **not run here**: the pinned brokkr binary (`.brokkr/recipes/self/tooling.json`) is not provisioned in the implement seat's box and the seat has no network. This change touches nothing under `.brokkr/`; the verify seat and CI run it. |

`git diff HEAD -- Cargo.toml Cargo.lock evals deny.toml .github scripts .brokkr`
is empty (AC13). No load was applied.

## Open item — AC16

Issue #260 asks for weights and a total. They amend accepted 0085 and wait for
the operator's acceptance of 0122; **#260 remains open for that part.** The
guide labels each proposed weight "proposed in 0122 — not in the report".
