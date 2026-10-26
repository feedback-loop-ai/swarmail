# Tasks — conditional envelopes

Date: 2026-10-26. Implementation boxes below reflect the dated local proof in plan.md.
Each task owns the paired proof; design preflight is not implementation proof.
Run tasks in order, mutations sequentially. Preserve exact selector names or
revise this ledger with the delivered names before implementation handoff.

- [x] **T01 — Recheck evidence and freeze scope (AC1–AC6).** Verify all packet
  entries from their directory, the pinned engine digest and clean baseline;
  inspect the accepted addenda and three allowed implementation record paths.
  **Proof:** directory-local `sha256sum -c SHA256SUMS`, engine SHA-256 equality,
  `git status --short` and source/tree identities recorded in the proof summary.
- [x] **T02 — Add the requested regressions before production edits (AC1–AC2).**
  Add `conditional_missing_marker_uses_ordinary_source`,
  `conditional_markers_ignore_ascii_case` and `conditional_marker_must_be_final`
  inline; correct the existing no-marker table row. Assert exact fallback/full
  arrays plus a following anchor, and a later attribute after a non-final marker.
  **Proof:** each corresponding exact unit command below runs one test;
  missing/non-final are behavioral baseline reds. Record mixed-case baseline
  honestly, even if green. Retain source/log hashes, exits and assertions.
- [x] **T03 — Qualify before callbacks and bound payload scanning (AC1, AC3).**
  Implement the local range qualifier, preserving the first close/abrupt rules,
  raw ASCII recognition, final marker and header ordering. Reuse bounded `tag`
  visitation; retain prior closed values on unfinished tails, outer `Kind::Other`
  and existing `LinkRegions`. Extend the existing boundary table for all AC1/AC3
  edges, including a separately qualified unfinished-tail fixture.
  **Proof:** the three exact unit cases and exact existing
  `extract::tests::conditional_comment_attributes_are_source_bounded` each run
  one test and pass; `cargo test --locked --lib extract::` passes with a recorded
  nonzero total and exact code/link arrays.
- [x] **T04 — Prove measured work and decode ownership (AC4).** Add local counters
  at every specified loop/probe and decode call; add
  `conditional_scan_work_is_bounded` with small exact counts, ordered ranges,
  partition/decode assertions and sizes 1/16/256/4096 for the plan's input classes.
  **Proof:** its exact unit command selects one test and passes every separate
  category bound; existing unrelated exact work pins stay green under `extract::`.
- [x] **T05 — Demonstrate both negative controls (AC2, AC4).** Temporarily make
  final-marker comparison case-sensitive; run the mixed-case evaluator red.
  Restore exactly and run green. Separately repeat real measured suffix work
  while preserving outputs/partition; run the work evaluator red on its bound.
  Restore exactly and run green. **Proof:** each invocation selects one test;
  record mutation patches, original/mutant/restored hashes, command/log digests,
  intended assertion failures and exits. No compile failure or fabricated
  counter increment qualifies. Confirm original equals restored bytes.
- [x] **T06 — Extend the real-wire evaluator (AC5).** Extend
  `outlook_conditional_attributes_store_only_complete_urls` with qualified,
  missing, mixed-case and non-final fixtures on isolated authenticated inboxes,
  both existing shim projections, native exact stored links/codes after 250
  and raw/body checks. Keep conditional-only and duplicate-anchor complete-URL
  singleton controls. **Proof:** exact wire selector below runs one test and
  passes; full compat suite passes. No mocks, sleeps or new storage seam.
- [x] **T07 — Retain adjacent and historical contracts (AC3, AC5).** Run the
  existing nested-URL, independent-MIME, historical live/reopen and #136 tests
  listed below; compare exact arrays and historical raw/body bytes. Review the
  committed diff against this follow-up's allowed scope. **Proof:** each named
  exact wire selector runs one passing test; `store_contract_docs` passes with
  recorded nonzero total. No decision, refusal, store, SMTP or routing changes.
- [x] **T08 — Close local gates and evidence (AC6).** Restore mutants, execute
  every house gate and supplemental proof in the plan offline with the checked
  local engine. Record exact coverage totals, zero misses and selftest success.
  Update this ledger and the plan's dated proof summary with delivered selectors,
  counts, source/tree identities, commands, exits and log hashes. Commit only
  scoped implementation work unsigned. **Proof:** all gate receipts pass on
  final restored code; inspect `git diff --check`, final commit paths and clean
  tracked worktree. Protected verify/review remain owed to the engine; external
  CI/operator duties are not silently marked locally passed.

## Exact evaluator commands and expected selections

Set `CARGO_NET_OFFLINE=true`. Each line below must select **exactly one test**.
The first four selectors are introduced by T02/T04; do not report zero selected
against the baseline as red. Broader suites record actual nonzero counts.

```bash
cargo test --locked --lib extract::tests::conditional_missing_marker_uses_ordinary_source -- --exact
cargo test --locked --lib extract::tests::conditional_markers_ignore_ascii_case -- --exact
cargo test --locked --lib extract::tests::conditional_marker_must_be_final -- --exact
cargo test --locked --lib extract::tests::conditional_scan_work_is_bounded -- --exact
cargo test --locked --lib extract::tests::conditional_comment_attributes_are_source_bounded -- --exact
cargo test --locked --test compat outlook_conditional_attributes_store_only_complete_urls -- --exact
cargo test --locked --test compat historical_outlook_arrays_survive_reads_and_reopen -- --exact
cargo test --locked --test compat independent_mime_links_remain_text_first -- --exact
cargo test --locked --test compat nested_attribute_urls_and_curly_suffixes_survive_fresh_ingest -- --exact
cargo test --locked --test api smtp_issue_136_link_fidelity -- --exact
```

Owning suites: `cargo test --locked --lib extract::`,
`cargo test --locked --test compat`, and
`cargo test --locked --test store_contract_docs`.
The final full workspace gate covers frozen-guarantee regressions as well.


## Implementation disposition — 2026-10-26

T01–T08 are complete locally. Each of the ten exact evaluator commands above
selected one passing test on final code; owning extraction/compat suites ran
41/36 passing tests and documentation contracts ran 25. Missing/non-final and
corrected-old-fixture baseline behavioral reds, the actual mixed-case baseline
green, case/work mutant reds and exact-restoration greens are retained with
hashes in plan.md. All house and supplemental gates pass, including literal
14988/14988 lines, 838/838 branches and 1610/1610 functions plus coverage
selftest. The proof summary binds the source snapshot/tree and log identities.
Protected verify/review and CI/operator duties remain owed to their owners.
