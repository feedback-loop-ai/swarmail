# Issue #259 clean follow-up — re-prove the combined report on its committed tree

Date: 2026-10-29. Status: design draft for run
`follow-up-issue-259-combined-htm-0c3e43d5` (issue #259 follow-up).

## Feature and rationale

Prove the committed combined HTML and bulk deliverability report candidate on
its exact clean tree, and repair only what that proof reproduces.

The candidate is HEAD `61320c6036ef8e8507111632abd0245ccf4ca781`, tree
`8750196325a442e5bd67a33b03d2b4b908e3bc49`, clean at intake and at design. Its
delta over the previous design commit `976861f` is two test files, and
`git diff 976861f 61320c6` hashes to
`498dd063f286b4b65a3cfe9dfcc5c219a5f80abd200d62b95938403731d24d1e`, the saved
Opus patch `.forge/evidence/implementer-original.patch`. The prior run
`verify-and-independently-protect-e769a24f` left that patch uncommitted
(`committed: false` in its implement result): its pinned-bundle gate step
passed, then the engine parked at `VERIFY-PASS` with "verification candidate is
dirty or unreadable". A green gate on a dirty tree binds nothing. Nobody has
verified or reviewed the committed candidate, and that is the gap this
follow-up closes.

Nothing in this design changes product semantics. The review findings of the
checksummed `.forge/issue259-review` packet (one MEDIUM, two LOW, five INFO)
were repaired in `c8dacc6`; `61320c6` strengthened the wire proofs; the
earlier-stopped audit's R1–R6 are either fixed on this tree or operator-owned.
The deliverable is therefore a candidate-bound proof plus honest classification
of what stays open, not a diff. A change that only exists to look productive
would widen the exact-coverage denominator and the review surface for no
criterion, and is refused here.

## Evidence the chief read, and its limits

Both run-local positions in full; the intake results `d3b32236` and `1ffcbe72`
(the framing — `.brokkr/state/tasks/` does not exist); the four packet files
(SHA256SUMS pass); `.forge/evidence/earlier-stopped-audit.json` (R1–R6, the
original-finding dispositions, AC1–AC12 of the stopped run); the dirty-run
journal's implement and verify events; house rules; the decision index; 0081,
0085, 0090, 0091; `specs/combined-report-review/*`; and the source anchors named
below, each confirmed on this tree. The chief ran no gate and no test. Every
"observed" cell below is a reading of the tree, not verification.

## Panel adjudication

| Question | Ruling | What the losing side costs |
|---|---|---|
| Scope of change | **Simplicity wins.** No production delta is planned. Repair only a defect reproduced with a concrete failing input, in the owning function, with the test that already owns that behaviour extended. | Robustness loses nothing it asked for: it also refused frameworks and rewrites. The cost is that a latent defect no proof reaches stays latent; the mitigation is robustness's standard of proof below. |
| Standard of proof | **Robustness wins.** Whole-report oracles independent of any response; the empty-message matrix cell by cell; progress and inspected work asserted separately; a real counted repeated-read control, not an inflated counter. | Simplicity's "re-run and record" alone is refused: a passing receipt that compares a filtered subset or copies a response proves agreement, not correctness. Cost: the implementer must read the two changed test files, not trust the patch hash. |
| Intake AC "fails on 61320c6" | **Restated** (simplicity). The tests already pass on the base; no new test can fail there without faking a regression. Sensitivity is shown by run-local mutation controls on a disposable copy: red, byte-exact restore, green, hashes recorded. | The run loses a permanent red/green artifact. A permanent fault seam would be production or seam code taken only by tests, under exact coverage forever, and is refused with robustness's agreement. |
| Whole-report comparison | **Adopted** (robustness). Already present in `61320c6`: `bulk_findings()` is a literal oracle and the CSS pairs compare the full `{"rule_set_id":"swarmail-html/3","findings":...}` object. The implementer confirms this by reading and extends only an adapter that still compares a subset. | None beyond reading time. |
| Empty-message matrix | **Adopted** (robustness) as cells; **mapped, not duplicated** (simplicity). Each cell names the exact existing assertion; only an unmapped cell gets a new durable assertion in the owning suite. | A second matrix is refused: it duplicates an oracle review must read. |
| html/2 in README and llms.txt | **Leave as recorded** (reject simplicity's optional deletion). The sentence is labelled historical and followed by the current combined behaviour. Deleting it changes served bytes pinned by `report_served_docs_match_contract` for no criterion. | A reader meets one historical identity in living docs; the audit table below flags it for the operator. |
| 0081, 0085/0090 pointers, appended note, old spec headers | **Unresolved, unedited** (both positions). Classified below; nothing is rewritten or accepted. | Stale-looking text persists until the operator rules. |
| New decision | **None declared.** No ruling, contract or route changes. Compatible work under proposed 0085/0089/0090/0091 is permitted by accepted [0045](../../docs/decisions/0045-delivery-repairs.md) §8. | If implementation finds a semantic defect, it returns to design for a proposed decision rather than fixing it under this spec. |
| Eval suite and #260 | **Both positions agree.** Evals run only in an evidenced same-user nonoverlap window with a captured, hashed executable; #260 and this tree certify nothing about each other. | Honest deferral when the window or the #260 identity is unavailable. |

## Restated intake criteria

Intake `d3b32236` AC2, AC3 and AC5 are restated; the rest of both intakes is
adopted as written.

1. **Declaration context.** The paired tests already on the base are the
   proof: `reviewed_custom_properties_preserve_declaration_context` and
   `component_values_keep_outer_and_following_declarations_visible` inline in
   `src/deliverability/html.rs`, and the inline/stylesheet pairs of
   `css_component_and_literal_url_pairs_survive_smtp_reports` on the wire.
   Sensitivity: on a disposable copy, collapse `Context::Component` into
   `Context::Declarations`; the negatives go red while the positive pairs stay
   green; restore byte-exactly; green. Record source, patch and log hashes.
2. **Counted work.** `inspected_work_has_a_fixed_linear_bound_separate_from_progress`
   with `C = 8192`, `K = 4096` unchanged is the proof. Sensitivity: on a
   disposable copy, add one counted repeated suffix read inside one helper
   (`terminated` or `tag`), observe the progress assertion pass and the bound
   assertion fail, restore byte-exactly, observe green. No committed hook, no
   enlarged bound, no fabricated increment.
3. **Identity.** Live owners already say `swarmail-html/3` and are pinned by
   `RULE_SET_ID`, the catalogue snapshot, the tracked fixture and
   `report_served_docs_match_contract`. The audit table below is the required
   artifact; no new test.

## swarmail-html/2 occurrence audit (tree 8750196)

`grep -rn "html/2"` over tracked files finds eleven occurrences in seven files.

| Occurrence | Class | Disposition |
|---|---|---|
| `src/deliverability.rs:12` `RULE_SET_ID`, `src/deliverability/html-rules.json`, `tests/fixtures/html-compatibility.json`, `src/static/openapi.json` example | Live, say `swarmail-html/3` | Correct; pinned by tests. |
| `README.md:1414`, `src/static/llms.txt:373` | Living docs, labelled "historical", followed by the current html/3 combined failure | Leave; flag to the operator. |
| `docs/decisions/0085-deliverability-report.md:138`, `0090-html-client-compatibility.md:76` | Dated 2026-10-28 pointers to proposed 0091, describing the HTML-only repair | Historical. 0091's appended integration note already reconciles html/2 with html/3. Unresolved for the operator; not edited. |
| `docs/decisions/0091-html-review-repair.md:26` and `:91` | Original ruling text plus the appended "Combined bulk and HTML identity — proposed integration note" | Historical plus a proposed note; neither is acceptance. Unedited. |
| `specs/html-review-repair/{spec,tasks}.md`, `specs/html-integration-proof/{spec,plan}.md`, `specs/combined-report-review/{spec,plan}.md` | Historical specs | Unedited (operator ruling 2026-10-26: specs are historical; corrections go through decisions). |

## Disposition of every earlier finding

Observed = read on tree 8750196 by the chief; the implementer re-executes.

| Finding | Observed on this tree | Required disposition |
|---|---|---|
| Review MEDIUM / independent 2: custom-property braces create `html.css.position.v1` | `Context::{Rules, Declarations, Component}` in `html.rs`; inline pairs and wire pairs present | Re-run; mutation control (restated criterion 1). |
| Review LOW / independent 5: progress-only work proof | `Work::{byte, prefix, charge, decode}`; bound test with four sizes, accepting and failing bodies | Re-run; counted repeated-read control (criterion 2); state exclusions. |
| Independent 1: zero-byte message | `collect_with` returns `Ok(())` on empty raw; combined admission fails first; `empty_smtp_message_has_an_exact_composable_report` and the stdio witness pin 250, zero raw, repeated 503, `isError`, UI | Map every matrix cell; add only unmapped cells. |
| Independent 3 / R1: 0090 index row, missing 0091 | Index rows 0081–0091 contiguous in one table; 0091 row and record present | Confirm by rendering the table; no edit expected. |
| Independent 4: slash-star in literal `url(...)` | `url_start`/`url_end`; inline and wire pairs | Re-run. |
| Independent 6: hard-coded provenance | Retained cost; negative tests exist | No change; record. |
| Independent 7: strict malformed input | Retained, documented cost | No change; record. |
| Independent 8: displaced header-overflow witness | `report_failure_is_never_clean_and_ui_keeps_message` sends malformed HTML and header overflow to separate inboxes | Re-run; keep the witnesses separate. |
| R2: served empty-message wording | README, llms.txt and OpenAPI now state that an empty message is accepted and stored, and that combined analysis fails (503, `isError`) | Verify served bytes through `report_served_docs_match_contract`; no edit unless a contradiction is demonstrated. |
| R3: html/2 versus html/3 | Live identity is html/3 | Audit table above. |
| R4: durable empty and CSS proofs | `61320c6` added the full-object comparisons and the count/threshold loops | Read and confirm; this is the preserved Opus work. |
| R5: evals CI comment | `evals/Cargo.toml` now says standalone gates "remain owned by #201"; README agrees | Confirm; no edit. |
| R6: 0081 provenance | Record says proposed; index row says "proposed — awaiting the operator"; 0058 pointer says proposed; the packet's `operator-rulings.md` says accepted 2026-10-26, "recorded on main" | Report both verbatim with attribution. Edit neither. |

## Operator-owned items left unresolved

- **0081 status.** Repository: proposed. Checksummed packet: accepted on
  2026-10-26. The run follows the directed Spark `glm-flash` tests routing and
  infers no acceptance.
- **Appended note in `specs/html-review-repair/implementation.md`.** Historical
  ledger with a dated combined-behaviour paragraph appended. Preserved; it is
  not fresh proof and is not byte-identical historical evidence.
- **html/2 pointers** in 0085, 0090, README and llms.txt, classified above.
- **Spec header drift.** `specs/combined-report-review/` names `c793401` as its
  start; this run starts at `61320c6`. Historical; this spec names the base.
- **#260.** The combined #260 integration is live elsewhere; its identity is not
  in this worktree. Neither tree certifies the other.

## Decision

No decision is declared. The governing records stay proposed:
[0085](../../docs/decisions/0085-deliverability-report.md),
[0089](../../docs/decisions/0089-bulk-sender-checklist.md),
[0090](../../docs/decisions/0090-html-client-compatibility.md),
[0091](../../docs/decisions/0091-html-review-repair.md). Accepted
[0045](../../docs/decisions/0045-delivery-repairs.md) §8 permits compatible work
beneath them. Frozen 0001–0003 and 0005 are untouched: no analysis enters
`Store::insert`, the SMTP `250` path, the watcher, pruning or ingest extraction.

## Acceptance Criteria

Each criterion names the proof that can go red. "Recorded" means written in the
implement result's notes or in run-local evidence, bound to the final HEAD and
tree; nothing below authorizes a tracked artifact beyond this spec directory
and a repair the criteria themselves demand.

1. **AC1 — Bound identity.** Start HEAD `61320c6`, tree `8750196`, clean. The
   patch hash equality `sha256(git diff 976861f 61320c6) == 498dd063…` and the
   four packet SHA256SUMS are reproduced. The final HEAD, tree and `git status
   --porcelain` (empty) are recorded. If any tracked file changes, an unsigned
   candidate commit exists before `complete`; a candidate commit is not
   shipping. **Proof:** `git rev-parse HEAD HEAD^{tree}`, `git status`,
   `sha256sum -c`, `git log -1 --format=%G?` showing `N`.
2. **AC2 — Opus tests preserved.** `git diff 61320c6 -- tests/deliverability.rs
   tests/stdio.rs` is empty, or contains only additions each justified in the
   notes; no assertion present at `61320c6` is removed or weakened.
   **Proof:** the diff and the justification list.
3. **AC3 — Declaration context.** The two inline pairs and the wire pairs named
   in restated criterion 1 pass on the candidate. The disposable-copy collapse
   of `Component` into `Declarations` turns the negatives red with the positives
   still green; byte-exact restoration is green. **Proof:** the test run; the
   control's mutated and restored source hashes, patch hash and red/green log
   hashes; `git status` clean on the candidate afterwards.
4. **AC4 — Literal URL.** `reviewed_unquoted_url_keeps_comment_characters_literal`,
   `literal_urls_protect_data_without_disabling_ordinary_comments` and
   `malformed_supported_syntax_fails_and_nesting_is_bounded` pass; the wire pairs
   show slash-star inside `url(...)` produces no failure and no finding while an
   ordinary unterminated comment still fails the whole report.
5. **AC5 — Counted work.** `inspected_work_has_a_fixed_linear_bound_separate_from_progress`
   passes with `C = 8192`, `K = 4096` unchanged. The disposable-copy counted
   repeated-suffix read passes the progress assertion and fails the bound
   assertion; restoration is green. The result notes state the model's
   exclusions (mail-parser internals, allocation, hash-map cost, RSS, latency).
   **Proof:** test run; control hashes and logs as in AC3; `grep -n 8192
   src/deliverability/html.rs` unchanged.
6. **AC6 — Empty-message matrix.** Every cell of the plan's matrix (stored zero
   raw × {exact assert, positive-deadline wait, zero-time cold wait} × {info,
   warning, error} × count {0, 1}; truly empty inbox × {exact assert, wait})
   maps to an exact existing assertion in
   `empty_smtp_message_has_an_exact_composable_report`,
   `report_threshold_exact_count_and_min_one` or `report_stdio_text_contract`,
   or gains a durable assertion in that suite. SMTP `250` precedes any analysis;
   lookup is `200` with zero raw bytes; REST repeats the exact sanitized `503`;
   MCP and stdio return `isError` with the exact text "deliverability analysis
   failed"; the UI retains the message. **Proof:** the cell-to-assertion map in
   the notes and the passing suites.
7. **AC7 — Whole-report oracles.** `html_findings_share_one_report_and_predicate_contract`
   and `css_component_and_literal_url_pairs_survive_smtp_reports` compare
   complete report objects whose bulk and HTML expectations are literal in the
   test (`bulk_findings()` plus the catalogue row), never copied from a response;
   repeat REST, MCP text block and UI agree. `reviewed_version_snapshot_and_every_rule_pair` pins the
   fixture independently. **Proof:** reading the two tests and the passing run.
8. **AC8 — Identity audit.** `RULE_SET_ID`, the embedded catalogue, the tracked
   fixture and the served docs say `swarmail-html/3`. The audit table above is
   re-checked against the final tree with `grep -rn "html/2"`; every occurrence
   is classified historical or living-labelled-historical; none is edited.
   **Proof:** `report_served_docs_match_contract`, `reviewed_version_snapshot_and_every_rule_pair`,
   the grep output.
9. **AC9 — Status honesty.** The status lines of 0081, 0085, 0089, 0090 and 0091,
   their index rows, the 0058 pointer, `operator-rulings.md` (SHA256SUMS still
   pass) and `specs/html-review-repair/implementation.md` are byte-unchanged.
   The notes quote both 0081 statements with attribution and call the item
   unresolved. **Proof:** `git diff 61320c6 -- docs/decisions specs/html-review-repair`
   empty; `sha256sum -c`.
10. **AC10 — Frozen guarantees.** `git diff 61320c6 --stat -- src/store.rs
    src/smtp.rs src/persist.rs src/model.rs src/extract.rs` is empty. Burst,
    p2, rate (default target), persistence and the declared deliverability
    concurrency harness pass unchanged. **Proof:** the diff and the suites.
11. **AC11 — Gates.** Every command of the one
    [gate inventory](../../docs/house-rules.md#gates--part-of-every-change) runs
    `--locked` on the final candidate with the bundle-pinned brokkr: fmt, clippy
    with warnings denied, workspace tests, bundle compile without policy-sweep
    warnings, exact coverage with integer `missed == 0` for lines, branches and
    functions, the coverage selftest, publish and seal proofs, licenses, and
    the fail-closed `native-tls` absence. Command, exit and log digest are
    recorded. A candidate-copy pass is an observation; only the engine's
    hermetic verify emits `verified_candidate`.
12. **AC12 — Evals.** The standalone eval suite (fmt, locked all-target clippy,
    locked tests) runs only after same-user suite-wide nonoverlap and cleanup
    evidence exists, against a root executable captured from this candidate's
    build with its SHA-256 recorded before and after. Without that evidence the
    criterion is recorded unresolved, not bypassed. No paid trial.
13. **AC13 — #260 and handoff.** The notes record this candidate's final
    identity, state that the #260 tree's identity is not visible here, and that
    neither certifies the other. Verify and the five-member protected review
    (Astra correctness, official DeepSeek Flash security, Spark GLM-5.3-Flash
    tests via dsh, Astra guarantees, independent Fable) are engine-owned; no
    seat fabricates a result or ledger entry. No push, merge, publish, tag,
    issue closure or acceptance.
