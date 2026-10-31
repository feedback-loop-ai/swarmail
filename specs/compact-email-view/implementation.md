# Implementation story — compact email view and field projections

Issue #155, decision 0108 (proposed). Zero-Codex `claude-careful` lane. This
records what was built, what was proved and what was not.

## T0 — the stacked base, before any edit (AC6)

On the untouched base `f663c44`, in a scratch worktree sharing no edits with
this branch:

```
cargo test --locked --test mcp_revisions --test focused_waits   # exit 0
  focused_waits: 5 passed
  mcp_revisions: 28 passed — including structured::legacy_bytes,
  structured::modern_all_tools_http_and_stdio and
  structured::served_openapi_pins_and_negative_controls
```

The design's reading of the dispatcher's resolution of
`tests/mcp_revisions/structured.rs` stands: of the fourteen removed lines,
nine are `pub(super)` visibility widenings and five are replaced by an equal or
stronger form (`assert_appended` — `tests/mcp_revisions/focused.rs:74` — for
the whole-transcript equality, with byte equality on every later line; the
`names[..13]` / `names[13..]` split; the union with the focused set in the
served-versus-projected equalities). No #153 structured-output assertion was
lost; nothing was restored. Work that belongs only to #153 or #154 stayed
theirs.

The modern-revision sites that read `text`/`html` from search or latest, found
by running the suites after the change and fixed: `revision_smtp_visibility`,
`real_bridge_dual_output_size_cost` (now measures bodies through `fields`) and
the search/latest lines of `modern_all_tools_http_and_stdio`. The default-version
suites (`tests/mcp.rs`, `tests/p2.rs`, `tests/stdio.rs`) are March and passed
untouched.

## What changed

- **Attachments at ingest** (`src/model.rs`, `src/smtp.rs`, `src/persist.rs`).
  `Email.attachments: Option<Vec<Attachment>>` (`#[serde(skip)]`, so every
  existing serialization, feed and webhook payload keeps its bytes) is built in
  `build_email` from the parser's outer attachment list and written in
  `record_insert`'s transaction to the nullable column `attachments_json`
  (stored as `[[filename|null, type|null, size], ...]`). `NULL` is unknown
  history and is never backfilled; any other value restores through a strict
  generic reader (`strict_column`, shared with `link_metadata`) or the restore
  refuses at column 20. `Store::insert` gained no await and no second write.
- **The compact view** (`src/api.rs`). `CompactEmail` is built from stored
  fields only; the snippet is `compat::snippet` (now `pub(crate)`), so the
  existing 8192-byte visited-work proof transfers by identity. `anchors` and
  `attachments` are cut at 250 with exact `_total` members.
- **MCP** (`src/mcp.rs`). On June and November, search and latest answer
  compact items; `fields` is accepted by search, latest, get_email and
  wait_for_email, staged before any lookup and before a wait parks. The modern
  catalogue is built once (`MODERN_TOOLS`) and `unknown_argument` reads the
  revision's catalogue, so March answers `unknown argument: fields`. One closed
  `projected_email` output schema (`required: ["id"]`) serves the four reads.
- **REST** (`src/api.rs`, `src/static/openapi.json`). `?view=compact` on the
  inbox list only, `compact` the only value; the unchanged path is the
  unchanged code. `CompactEmail` / `CompactEmailList` are documented; `Email`
  and `EmailList` are untouched, so the OpenAPI pins still derive from them.
- **Documents**: README, `llms.txt`, the decision index row for 0108.

One thing the design did not state, found while building: `EmailAddress` declares `name` before `address`, so a struct write and a
`serde_json::Value` write of the same address differ. Every plane therefore
writes the compact item through a `Value` (the REST list included), so the
REST list, the text block and `structuredContent` agree byte for byte. The
REST envelope keeps the unchanged `total, emails` order.

## Token metric (AC7)

**not measured: no paid trial.** No claim about median tokens per success is
made. The harness's own MCP client (`evals/src/mcp.rs`) pins `2025-03-26` and
serves the oracle, not the agents under trial; it is unchanged.

The byte table below is **bytes, never tokens**. Its figures are asserted in
`tests/mcp_revisions/compact.rs::the_story_byte_table_is_what_the_wire_serves`
(the fixed restored row of the structured-output baseline, which carries 65
stored codes, and a newsletter-shaped mail built by ingest with fixed volatile
members):

| Fixture | March full item | Compact item | Modern search line before (both representations) | Modern search line now | `get_email` line (unchanged) |
|---|---:|---:|---:|---:|---:|
| restored row (code-heavy) | 949 | 714 (−24.8%) | 2324 | 1812 (−22.0%) | 2232 |
| newsletter, 10 links, PDF | 5214 | 1836 (−64.8%) | 10852 | 4050 (−62.7%) | 10768 |

The restored row shows the floor: stored `codes` are served uncapped, so a
mail that carries many of them saves little. The newsletter shows the typical
case; its 20 anchors are the text part's and the HTML part's occurrences of the
same ten links, as ingest records them. Two reasons the real metric may still
not move: the eval prompts steer agents to `swarmail_wait_for_email`, which
stays full by default, and each of the four modern descriptors grew by the
`fields` enum, a cost paid once per discovery.

## Gates (AC8)

All run locked in this worktree:

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| `cargo test --workspace --locked` | all binaries ok |
| `cargo test --manifest-path evals/Cargo.toml --locked` | ok on the second run; see below |
| `bash scripts/coverage-gate.sh` | pass, exact 100/100/100 (19604/19604 lines, 1334/1334 branches, 2009/2009 functions) |
| `bash scripts/coverage-gate.sh selftest` | pass |
| `bash scripts/publish-guard-test.sh` | OK |
| `bash scripts/seal-branch-test.sh` | 7 cases and 10 mutants passed |
| `cargo deny check licenses` | licenses ok |
| `brokkr compile --bundle .brokkr/recipes/self` | **not run**: the pinned brokkr binary is not installed in this sandbox and nothing may be installed. This change touches nothing under `.brokkr/`, `scripts/`, `Cargo.toml`, `Cargo.lock` or `deny.toml`, so it cannot move the bundle; the verify seat runs it. |

The first evals run failed one test, `runner::test_probes::cleanup_observation_error_and_forced_expiry_fail_with_live_owned_processes`
(`evals/src/runner.rs:4503`), a process-timing probe in a crate that does not
depend on this one; run alone it passed, and the whole suite then passed
(52 + 19 + 3 + 15 + 18 + 5 + 6 + 27 + 18 + 37). It is reported as a load-sensitive flake,
not fixed here.

No dependency was added, no `coverage(off)`, no seam, no new production file.
Commits are unsigned; nothing was pushed, merged, published or accepted.

## Residual choices for the operator

- `fields` on REST (a comma list on the list and await routes) is not
  delivered; it needs a separate decision because the single-message and
  thread query domains are accepted-empty (0074 §1, 0048 §4).
- A client that never negotiates a version stays on March and gets no compact
  view (0082 §1).
- Decision 0108 is `proposed`; it was drafted as 0092 and renumbered at the
  merge of main, where 0092 is official-mail-comparison.
