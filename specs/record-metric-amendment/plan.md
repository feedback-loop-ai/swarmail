# Plan: record the metric amendment

Implements [spec.md](spec.md). Baseline: `59a37f2fadaf2e0faafbdf581ae91db4d72e8bbc`.
The checkout has no `.specify/templates/plan-template.md`; its required
technical context, constitution, structure, research, model, contracts and
quickstart sections are supplied directly below.

## Technical context

This is a documentation-only acceptance record in one Rust crate. Markdown
ADR files and the decision index expose authority; existing Rust integration
tests inventory files and statuses. `src/api.rs::inbox_of_path` still returns
the raw selected segment. `count_in_flight` uses it before dispatch; await
accounting uses decoded handler identity. These observations explain the
pending runtime work and authorize no source changes in this task.

## Constitution check

The supplied operator acceptance satisfies 0045 §8 for this amendment only.
No new decision is proposed by this design and none is self-accepted. Preserve
0001–0003, 0005, 0046's admitted identity and 0047's cap/overflow. Amend 0043
through a dated addendum, preserving its original ruling and later markers.
Preserve reservations even where no corresponding file exists. Use unsigned
commits under 0056, never push, and retain the 0057 engine route and protected
four-seat review. No dependency, coverage, proof-chain or policy change is
needed. Every delivery gate remains required; the documentation scope grants
no waiver and the old spec's gate list is not the current inventory.

## Project structure and files touched

The chief commits exactly `specs/record-metric-amendment/{spec,plan,tasks}.md`.
Later implementation touches these eight logical existing documents only:

| Path | Planned change |
|---|---|
| `docs/decisions/0062-decoded-inbox-metrics.md` → `docs/decisions/0065-decoded-inbox-metrics.md` | Rename this amendment only; correct heading/status/numbering; add dated acceptance and original-draft provenance; retain five ruling clauses verbatim. |
| `docs/decisions/README.md` | Add one resolving accepted 0065 row with date and narrow scope. |
| `docs/decisions/0043-mcp-origin-boundary.md` | Insert a dated accepted-0065 marker immediately after clause 6; retain all original bytes around it, including 0047 markers. |
| `specs/http-plane-edges/spec.md` | Insert a dated scoped supersession link at Scope boundaries beside raw-key preservation. |
| `specs/http-plane-edges/plan.md` | Insert a dated scoped link beside Contracts and proof design AC3's raw-key paragraph. |
| `specs/decoded-inbox-metrics/spec.md` | Correct active decision number/link; add dated acceptance clarification of prior prerequisites without claiming runtime completion. |
| `specs/decoded-inbox-metrics/plan.md` | Correct current number/links and annotate historical design scope/status; preserve the full old validation receipt exactly. |
| `specs/decoded-inbox-metrics/tasks.md` | Reconcile acceptance/index prerequisite and old active identity, retaining unchecked tasks until their entire paired proof is complete. |

This design's own artifacts may receive dated implementation evidence. No
other tracked file is in scope. The rename exception removes the incorrectly
named amendment; it never permits modifying the actual #180/#181/#174 lanes.

## Research and resolved choices

Read both current panel positions, the current intake framing, decision index,
frozen decisions and relevant 0043/0045/0047/0057 authority. The baseline index
ends with proposed 0061 and lacks the misnumbered amendment. The original
plan records missing-index failures; those receipts are history, not new test
results. Existing `store_contract_docs` tests dynamically inspect inventory
and statuses, including missing/dangling/duplicate/malformed entries and
status-change controls. Its finite audited-pair list does not prove the new
semantic pointer. Review that pointer directly; do not extend tests merely
to claim automated coverage of this record.

The panels agree on eight-document scope. Choose exact clause preservation,
small dated references and explicit manual comparison over rewriting history
or building a validator. Keep the task-supplied chat provenance distinct from
an independently verified transcript. No external research or further
operator approval is required to record the acceptance already supplied.

## Data model and implementation approach

No runtime or persistent data model changes. Documentary identity moves from
one mistaken filename/heading to 0065. Status becomes accepted while original
proposed status remains historical. References point to that one authority;
no alias record or reserved-number placeholder is created.

First capture baseline bodies using Git, then rename the amendment. Correct
only its identity/status/false numbering paragraph and append dated provenance.
The note must scope the original Proposed ruling heading, clause 5 prerequisite,
four-file handoff and no-acceptance assertion to the prior draft. Add the index
row. Insert narrow dated markers in 0043 and the HTTP-plane documents; explain
that unchanged refusal means not all request and await rows can agree. Repair
the amendment's owning references and add a dated clarification beside obsolete
acceptance prerequisites, including plan risks/quickstart where relevant.
Current links and identity may change; historical receipts and paragraphs may
not be silently modernized. Keep the earlier tasks unchecked when they mix a
satisfied prerequisite with unfinished indexing, testing or runtime work.

## Contracts and proof design

Use `git show` at the baseline to compare the five-clause block between
`## Proposed ruling` and `## Alternatives, panel ruling and costs` exactly.
Compare the entire old plan section beginning `## Design validation receipt`
exactly. For 0043 and both HTTP-plane files, subtract only the newly inserted
dated marker blocks and require equality with their baseline bodies. Review
all other amendment edits individually as metadata, active-reference correction
or dated clarification. Preserve alternatives/costs/proof blocks, with any new
annotation outside their original paragraphs.

Check that the old amendment filename is absent, one 0065 file and one index
row exist, and row status/date and target match. Inspect active markdown links
in affected documents and resolve their relative targets. Search for old
0062 references only in this amendment's owning documentation: classify each
as a corrected active reference or preserved historical evidence. Do not use
a global zero-occurrences rule. Compare changed paths against the table;
reserved-lane and unrelated decision bodies must remain unchanged.

Run `cargo test --locked --test store_contract_docs`. The inventory and status
mutation controls are negative proofs already present; no new runtime mutant
or test file is needed. Passing these tests does not prove semantic equivalence:
review the exact clauses, dated scope and provenance separately. Keep run-local
command output and an honest dated receipt in this design's plan, never in the
old receipt. Full gates and independent review follow the authoritative house
rules; exact coverage remains zero missed lines, branches and functions.

## Quickstart and validation

Implement tasks in order after the design check. Inspect `git diff --check`,
`git diff --name-status` and targeted historical comparisons; run the documentation
suite, then the complete [house-rule gates](../../docs/house-rules.md#gates--part-of-every-change)
with the pinned engine and proof scripts, offline without installs. Validate
the candidate-local `.forge/bin/brokkr` digest against `bundles/self/tooling.json`.
Do not substitute historical gate lists. Preserve CI-only ownership and record
any failed or unavailable gate rather than reporting success.

For this design phase, stage exactly the three new artifacts, commit unsigned
with an imperative subject and why body, and run the bundle's design checker
against HEAD. HEAD must identify only `record-metric-amendment`; do not edit
another spec directory or an ADR in this phase. There is no new proposal to
declare. The sequence owns its deterministic final verdict. Write the required
`drafted` result separately through workspace hands; it is not a delivery claim.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Acceptance is invented or broadened. | Attribute the task-supplied 2026-10-24 ruling for this amendment only; preserve prior draft status as history. |
| Another lane's reservation is consumed. | Explicit #180=0062, #181=0063, #174=0064 provenance; rename only the known amendment, no global replacement. |
| Old and current documentation contradict. | Dated scoped pointers plus owning-reference/prerequisite repairs; accept several small edits. |
| Exact semantics drift while status changes. | Verbatim five-clause comparison and independent review; annotate instead of rewriting. |
| Historical failures disappear or become false successes. | Preserve old receipt byte-for-byte; record current evidence separately and leave mixed/runtime tasks unchecked. |
| Acceptance is mistaken for runtime repair. | Explicitly state unchanged source/deployment and deferred selector migration; do not advertise new runtime behavior. |
| Tests pass without proving the new amendment scope. | Manual marker/link/provenance review complements dynamic inventory tests; finite audited pairs are not represented as new coverage. |
| Strict fallback is misread as admission or universal label equality. | Retain refusal/no-registration and literal-percent collision caveats; all other metric/await semantics stand. |
| Generic design rules induce a second proposal or false proposed status. | This records an existing accepted ruling, creates no new semantic rule and declares no new ADR; no checker or policy change. |
| Pre-existing missing-index failures prevent a clean design-phase gate run. | Report fresh results accurately; implementation owns index repair and full rerun before delivery. No out-of-scope repair or green claim. |

## Design-phase validation receipt — 2026-10-24

Fresh checks for this acceptance-recording design ran with
`CARGO_NET_OFFLINE=true`. Formatting, Clippy, pinned bundle compilation,
coverage selftest, publish-guard proof, seal-branch keyless proof, license
check and fail-closed native-tls absence passed. Public shim checksums passed;
canonical bundle scripts were executed directly. The candidate-local engine
matches `bundles/self/tooling.json`'s digest
`e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`.

`cargo test --workspace --locked` failed the two existing documentation tests
`every_decision_file_has_exactly_one_resolving_index_row` and
`the_inventory_refuses_missing_dangling_duplicate_and_malformed_entries`:
`0062-decoded-inbox-metrics.md` has no index row. The coverage run reached the
same two failures and refused the instrumented run. No exact-coverage pass,
full-suite pass or completed delivery is claimed. These failures exist in the
unchanged prior draft at this design's baseline; this phase adds only its own
three design artifacts. T2/T3 repair the identity/index, and T7 must rerun all
required local gates before delivery. CI-only proofs were not run here.

Fresh logs and exit results are under
`.brokkr/state/design/acceptance-chief-checks/`; the instrumented log is
`target/coverage/run.FYQZol/run.log`. The earlier decoded-inbox plan's receipt
was not edited. The committed design's structural check runs after this
commit and is reported in the run-local phase result; the sequence's check
remains the authority on advancement.


## Recovery implementation receipt — 2026-10-24

Recovery baseline: `faef809`, following the concluded run
`epic-192-issue-202-record-the-op-6b55bb9e`; the current design artifacts
passed their deterministic check at `7fdeba6`. This receipt distinguishes
that prior candidate's retained claims from fresh evidence on the recovery
worktree. No runtime behavior, accepted decision or test source changed.

The prior four review results remain historical evidence. The recovery framing
and design confirm three LOW residuals: the HTTP-plane paragraph boundary,
missing owning evidence/task reconciliation, and the future refused-control
request-label proof. The tests INFO limitation remains explicit: the finite
`AMENDMENTS` test list does not prove the semantic 0065-to-0043 marker. It is
covered by direct comparison; no validator or test inventory was expanded.

Task evidence reconciliation for the original recording run:

- **T1 checked:** the `59a37f2` comparison fixtures, supplied acceptance and
  reservations, prior framing/reviews, and bounded prior path inventory were
  inspected. The `faef809` candidate is the historical baseline; these facts
  are not reused as fresh gate results.
- **T2 checked:** only the decoded-inbox amendment moved to 0065; the old
  0062 path is absent, status/date/reservations agree, and clauses 1–5 compare
  byte-for-byte with the original proposal at `59a37f2`. The explanatory
  paragraph outside those clauses is not miscounted as a ruling clause.
- **T3 checked:** fresh `CARGO_NET_OFFLINE=true cargo test --locked --test
  store_contract_docs` passed 19/19, including inventory, status and existing
  refusal controls. Its finite amendment-pair limitation remains disclosed.
- **T4 checked:** removing the dated 0065 blocks reconstructs the original
  0043 and HTTP-plane plan bodies exactly. The HTTP-plane spec also reconstructs
  exactly after removing its dated block and the single AC1 paragraph separator;
  a focused `faef809` negative/corrected-candidate positive comparison passed.
- **T5 checked:** owning links/status/provenance were reviewed, the prior
  decoded-plan content including both historical receipts remains a byte-for-byte
  prefix of the recovery version, and decoded runtime T2–T8 plus new T9 remain
  unchecked. No runtime or test success is asserted.
- **T6 checked:** the six recovery edits are confined to the allowed spec/plan/task
  documents. Decision files/index, source, tests, dependencies, gates, policy,
  recipes, reserved lanes and the unrelated 0017 reference are unchanged.
  Relative links resolve.
- **T7 checked:** the targeted suite and full local gate inventory passed;
  exact fresh outcomes are recorded below. CI-only proofs remain CI-owned.

The original task's review T8 remains open for independent review. For this
recovery, its dated route clarification allows only the task-authorized Haiku
dispatch fallback for GLM 5.3 tests; official DeepSeek Flash remains the
guarantees reviewer, never Pro, and no dispatch fallback substitutes for a
substantive verdict. The security residual is not cleared by this receipt.

## Final recovery gate receipt — 2026-10-24

Fresh gate run with `CARGO_NET_OFFLINE=true` on the finalized recovery diff
passed: `cargo fmt --all -- --check`;
`cargo clippy --workspace --all-targets --locked -- -D warnings`;
`cargo test --workspace --locked`; pinned `.forge/bin/brokkr compile --bundle
bundles/self`; `bash scripts/coverage-gate.sh`;
`bash scripts/coverage-gate.sh selftest`; `bash scripts/publish-guard-test.sh`;
`bash scripts/seal-branch-test.sh`; and `cargo deny check licenses`. The
worktree brokkr SHA-256 matched tooling.json
`e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`; bundle
compilation reported zero unruled valuations. Coverage was exact: 12,953/12,953
lines, 762/762 branches, and 1,437/1,437 functions. Coverage selftest passed;
publish guard reported 51 mutation-red/restored-green receipts; seal proof
reported 7 cases and 10 mutants. `cargo tree --locked -i native-tls` refused
with Cargo's explicit “did not match any packages” result (exit 101), proving
absence; an unrecognized error would have failed this check. The targeted
`CARGO_NET_OFFLINE=true cargo test --locked --test store_contract_docs` passed
19/19.

After this receipt and task-state reconciliation were finalized, the complete
local gate inventory was rerun; all commands returned zero with the same exact
coverage totals. No tracked file changed after that final rerun. The CI-owned
chart/history, Docker, network advisory, tag release-rate and real port-80
bridge proofs were not run here; house rules assign them to CI. Independent
review and security disposition remain downstream, and this receipt does not
claim runtime decoding or transient refused-control behavior.
