# Tasks — per-finding weight and report total (#337)

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md). Every task names the test
that proves it. "Inline" means the `#[cfg(test)] mod tests` of the named file,
which is measured by the coverage gate (0020 rule 1).

## Record

- [ ] **T1 — Commit the 0122 integration note.** Commit the dated 2026-11-07
  note that intake left uncommitted in
  `docs/decisions/0122-deterministic-spam-rules.md`. Add one sentence to it: MCP
  serializes the same report with sorted keys, `weight` and `total` sort last
  there too, and no existing MCP byte moves. Make no body edit and no status
  change. In `specs/spam-signal-rules/spec.md`, append a dated line to AC16 that
  points to `specs/finding-weight-total/spec.md`.
  *Proof:* `git diff main -- docs/decisions/` shows only the appended note, and
  `bash scripts/design-check.sh` is not affected. Review reads it.

## Core (`src/deliverability.rs`)

- [ ] **T2 — Fields.** Add `pub weight: u64` as the last field of `Finding` and
  `pub total: u64` as the last field of `Report`, each with its doc comment. Add
  no serde default. Write `weight: 0` at every production construction site
  (plan § Technical context) and in every test literal, including the one in
  `src/ui.rs`.
  *Proof:* inline `report_shape_is_required_and_closed`, extending the existing
  `dns` required/closed test at ≈1337. Removing `weight` from a finding, or
  `total` from a report, fails `from_value`. An extra key fails. A round trip
  is equal.
- [ ] **T3 — `weight_of` and `push` (AC1, AC2).** Add the private pure function
  `weight_of` and the unconditional assignment in `push` after the cap check.
  *Proof:* inline `push_weighs_by_severity_across_families`:
  - `spam.subject.all_caps.v1` warning → 3;
  - `html.tracking-pixel.v1` info → 1;
  - `bulk.one_click.v1` error → 10;
  - each read on the struct and as
    `serde_json::to_value(&report)["findings"][i]["weight"]`;
  - a finding built with `weight: 99` comes out at 3.

  Inline `zero_weight_exceptions`:
  - `demo.marker.v1` at info, warning and error → 0;
  - a `bulk.<rule>.not_evaluated.v1` id at error, and a `.not_evaluated.` id at info
    and at warning → 0;
  - failing controls: `bulk.x.v1` at error is 10, and a report holding the demo
    error plus a real error totals 10.

  Extend one evaluator-level test that already yields a real
  `bulk.*.not_evaluated.v1` error, and assert its weight is 0.
- [ ] **T4 — Total in `finish_with` (AC3).** Add the `try_fold`/`checked_add`
  sum after sort and dedup, place it in `Report`, and keep the byte check
  after it.
  *Proof:* inline `total_is_the_sum_after_dedup`:
  - an empty report → 0;
  - info + warning + error → 14;
  - an exact duplicate pushed twice counts once (the pre-dedup sum 24 ≠ the
    actual 14 is asserted);
  - 256 distinct error findings through `push` → exactly 2560
    (`MAX_FINDINGS as u64 * 10`);
  - `total` equals the sum of the serialized `findings[*].weight`.
- [ ] **T5 — Overflow (AC4).** No production code beyond T4.
  *Proof:* inline `total_overflow_fails_the_report`:
  `Collector(vec![a(weight u64::MAX), b(weight 1)]).finish(&Inputs::default(), Lookups::default()) == Err(AnalysisError::Failed)`.
  The comment says why it bypasses `push`: the arm is unreachable through
  `push` under today's bounds. Confirm that the coverage gate reports
  0 missed lines, branches and functions in `finish_with`.
- [ ] **T6 — Byte bound counts the new keys (AC5).** Recompute both exact-edge
  tests (`report_limits_refuse_without_truncation` ≈1390 and the second one at
  ≈2490–2510) from findings pushed through `push` at error severity
  (weight 10). The overhead must come from the serialized report, never a
  literal.
  *Proof:* the exact-fit report serializes to 65,536 bytes and contains
  `"weight":10` and `"total":10`. Adding one byte to the hint gives
  `Err(Failed)`.
- [ ] **T7 — Cache identity (AC6).** No production change.
  *Proof:* inline `cache_hit_and_miss_bytes_are_identical`, beside
  `cache_identity_covers_bytes_config_rules`:
  - a miss on service A;
  - a hit on A (`Arc::ptr_eq`);
  - a miss on a fresh service B, for one mail with `X-Swarmail-Demo: warning`
    and a spam warning;
  - all three `serde_json::to_vec` outputs are equal and contain `"weight":`
    and `"total":`.
- [ ] **T8 — Predicates ignore weight (AC8, inline).** No production change.
  *Proof:* inline `qualifies_ignores_weight_and_total`:
  - 256 distinct info findings (total 256) qualify at warning;
  - one warning (total 3) refuses at warning;
  - a sole zero-weight error (`bulk.x.not_evaluated.v1`, total 0) refuses at
    error;
  - a `temp_fail` report with no findings (total 0) refuses at info.

  A source-scan assertion over the body of `Report::qualifies` (read through
  `include_str!("deliverability.rs")`, split at the function) finds neither
  `weight` nor `total`.

## Probe composition (`src/deliverability/links.rs`)

- [ ] **T9 — Composed weights and total (AC7).** No production change, because
  `compose` already re-pushes.
  *Proof:* extend inline `compose_finding_table` or
  `compose_is_pure_and_order_independent`:
  - an observed `link.reachability.v1` (warning) weighs 3;
  - the composed total equals the cached total plus the observed weights after
    dedup;
  - the cached report's own total is unchanged.

## Rule-set id and fixtures

- [ ] **T10 — `swarmail-html/8` (AC11).** Set `RULE_SET_ID` and move every
  **live** pin: test expectations in `src/` and `tests/`, the html catalogue
  snapshot and `html-rules.json` if a test ties them to `RULE_SET_ID`,
  `openapi.json`'s route description and example, `MODERN_REPORT_DESCRIPTION`,
  llms.txt and the guide's current-id sentence. Dated history keeps `/7`.
  *Proof:* `cargo test --workspace --locked` is green. After the change, every
  remaining `swarmail-html/7` hit from `grep -rn` sits in a dated history line,
  a decision record or an earlier spec, and the commit message lists them.
- [ ] **T11 — Expectation updates.** Every `json!` finding expectation in the
  inline suites (`auth.rs`, `dkim.rs`, `html.rs`, `links.rs`, `spam.rs`,
  `deliverability.rs`) and in `tests/bulk_sender.rs`, `tests/spam_rules.rs` and
  `tests/deliverability.rs` gains its `"weight":N`, and every report
  expectation gains `"total":T`. Each value comes from the table, never from
  running the code and copying its output.
  *Proof:* those suites are green. Each family's fixtures are AC1's
  per-family evidence.
- [ ] **T12 — Pinned March fixture (AC13).** Replace line 8 of
  `tests/fixtures/mcp-structured-output/march-current-main.jsonl` with the
  bytes this tree serves for the seeded mail (sorted keys, `/8`, weights,
  total). Recompute `fixture_sha256`. Append a 2026-11-07 (#337) entry to the
  provenance `updated` string and to the fixture README, in their established
  style. Grep `tests/fixtures/` for other recorded report bytes or digests and
  treat them the same way.
  *Proof:* the structured suite (`tests/mcp_revisions/structured.rs`), which
  checks the row against the served report and the digest against the file,
  is green.

## Schemas and wire

- [ ] **T13 — OpenAPI and MCP schemas (AC10).** Make the plan § Contracts edits
  to `openapi.json`, `src/mcp.rs` `OUTPUT_SCHEMAS` and
  `openapi-report-source.json`.
  *Proof:* `served_openapi_pins_and_negative_controls` gains negative-control
  rows that drop `weight` from the finding's `required`, and `total` from the
  report's `required`. Each must fail the pin. Extend the existing
  structured-output validation so a served report validates against the new
  schema, and a report with `weight: -1` or without `total` does not.
- [ ] **T14 — REST key order, cache identity and predicates on the wire (AC6,
  AC8, AC9).**
  *Proof:* in `tests/deliverability.rs`, extend `default_dns_policy_exact_json`:
  - the raw text starts with `{"rule_set_id":"swarmail-html/8","findings":[`
    and ends with `…{MAIL_LOOKUPS}}},"total":T}`, with `T` the sum of the
    parsed weights;
  - every finding's raw substring ends `,"weight":N}`;
  - the sorted key list is `["dns","findings","rule_set_id","total"]`;
  - a second GET of the same report is byte-identical.

  In `a_message_that_passes_everything_is_returned_by_an_await`, or a sibling
  test, add the same gate message carrying `X-Swarmail-Demo: error` in its own
  inbox. Assert its `total` equals the passing message's `total`, then assert
  the passing message is returned by REST `await` and MCP
  `swarmail_wait_for_email` at `error` while the demo copy is not, and REST
  `assert` agrees. If the extra header changes the authentication outcome, the
  test says so and picks another zero-weight difference. It never weakens the
  equal-total condition.
- [ ] **T15 — MCP key order and identity (AC6, AC9).**
  *Proof:* in `tests/mcp_revisions/structured.rs`, the report rows on
  2025-06-18 and 2025-11-25 check the raw text block:
  - it ends with `,"rule_set_id":"swarmail-html/8","total":T}`;
  - every finding object ends with `"severity":"…","weight":N}`;
  - `structuredContent` equals the parsed text;
  - two calls return identical text.

  The 2025-03-26 text row is covered by T12's pinned row.

## Served text

- [ ] **T16 — Status words (AC12).** Remove "proposed NNNN" from `openapi.json`,
  the MCP descriptions, llms.txt, the guide's current-state prose and the
  `src/deliverability*` comments wherever the decision's status line says
  accepted. Dated history stays.
  *Proof:* a new test in `tests/deliverability.rs`,
  `served_text_names_no_accepted_decision_as_proposed`. It finds every
  `proposed NNNN` and `proposed in NNNN` in `src/static/openapi.json` and in the
  MCP `tools/list` descriptions served over HTTP. For each one it reads
  `docs/decisions/NNNN-*.md`, and it fails when the status line starts with
  `accepted`. As a negative control, it runs the same check on a line that
  says "proposed 0122" and must flag it.
- [ ] **T17 — Guide and llms.txt (AC12).** Make the plan § Served text edits,
  including the *Weight and total* subsection (table, exceptions, predicates
  ignore weight, the 2,560 bound, headroom cost). Mirror it in llms.txt.
  *Proof:* the `spam.rs` guide test (≈692) asserts
  `**Weight:** N` with `N == weight_of(id, severity)` for each spam rule (3,
  1, 3, 3, 3, 1). It also asserts the old phrase "not in the report" is
  absent. A served-llms test asserts the subsection's table line.

## Gates

- [ ] **T18 — Every house gate, locked (AC14).** Run the house gates:
  - `cargo fmt --all -- --check`;
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`;
  - `cargo test --workspace --locked`;
  - `cargo build --locked --bin swarmail`;
  - in `evals/`: fmt, clippy and `SWARMAIL_BIN=<candidate> cargo test --locked`;
  - `bash scripts/coverage-gate.sh` (exact 100%) and
    `bash scripts/coverage-gate.sh selftest`;
  - `bash scripts/publish-guard-test.sh`;
  - `bash scripts/seal-branch-test.sh`;
  - `cargo deny check licenses`.

  The brokkr bundle compile runs in verify if no brokkr is on PATH; say so.
  *Proof:* the gate transcript in the implement result. `git diff main --
  Cargo.lock evals/Cargo.lock deny.toml` is empty.
