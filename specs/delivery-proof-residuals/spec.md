# Delivery proof residuals

Created: 2026-10-24. Status: design draft. Story: NEXT-171, epic #192,
findings 3.1–3.4. Baseline: `45d68df37044de3360439b34c39687975dd59b84`.
Original run: `part-of-epic-192-close-every-ope-0db51b58`.
Recovery: `epic-192-issue-171-recovery-on-t-181c6f9c`, fresh intake on
`a76c5e5bdf82663a35ac0553e173a1bc7dac8e84`; not a linked continuation.

Decision: 0062

Proposed clarification: [0062](../../docs/decisions/0062-delivery-proof-residuals.md).
Accepted authorities: [0050](../../docs/decisions/0050-delivery-gates.md),
[0045 §8](../../docs/decisions/0045-delivery-repairs.md),
[0056](../../docs/decisions/0056-signing-policy.md) and
[0057](../../docs/decisions/0057-codex-delivery-recipes.md).
Only the operator accepts decisions. This draft grants no semantic exception.

## What and why

Close the four delivery-constitution findings by supplying missing execution
proofs, making existing integrity checks resistant to trivial test bypasses,
and documenting the release comparator's existing limitation. A structural
assertion cannot establish what brokkr's evaluator or a dispatched seat did.
A script's existence cannot establish its identity. Operators need receipts
that distinguish those claims before relying on delivery verdicts.

This is a bounded continuation of [delivery-gates](../delivery-gates/spec.md),
not a replacement for its AC1–AC13. This design owns NEXT-171 and its evidence
index; the earlier spec keeps its original proof obligations. T08/R04/D04
remain open until their entire obligations are satisfied. No story or epic
completion follows from a committed design.

## Recovery synthesis — 2026-10-24 (controlling)

Read both recovery positions, `simplicity.md` and `robustness.md`, under
`.brokkr/state/design/positions/`, both intake framings, the existing design,
delivery-gates artifacts, accepted authorities and copied root evidence.
The recovery preserves this slug and proposed 0062. It does not reserve a
second decision number or infer NEXT-187 coordination from a dated snapshot.
Only these three artifacts and the existing proposal change in chief's commit.

| Question | Ruling and losing alternative's cost |
|---|---|
| Preserve or recreate | Adopt both recovery positions: revise `delivery-proof-residuals` and 0062 in place, preserving the prior design history below. Reject another slug, ADR or framework: it would duplicate ownership without discharging any proof. Root must reconcile 0062 with NEXT-187 before integration. |
| Exact evaluator | Adopt both positions' exact rule/destination/severity oracle and robustness's complete-count/unique-case assertions. The copied real-core probe is useful but its broad stop/refused/park predicate cannot close AC8. Reuse its source/API approach in a new run-local child. Cost: provenance, aggregation ingress and a failing permissive-policy control remain mandatory. |
| Diagnostic reachability | Rule for robustness against simplicity's unchanged-runtime preference. Strong shim tests fail the ordinary Rust suite before verify reaches coverage; coverage then fails again at its own instrumented tests before producing counts. Coordinate a bounded change to both canonical owners: continue after test failure only to collect canonical coverage diagnostics, never to permit success. Reject hiding the test, restoring the shim during the red run, skipping a suite, or attaching an unrelated coverage run. Cost: two narrowly changed pinned scripts, renewed hash review, additional diagnostic execution and ownership coordination. |
| Pins and scope | Adopt simplicity's existing integration-test owner, fixed approved hashes and no new dependency; adopt robustness's named disabled/commented/deleted-call controls and actual dispatch evidence. Reject a general shell parser or hashes computed from arbitrary candidate bytes. Cost: legitimate canonical edits require explicit reviewed hash updates. |
| Live self | Adopt both positions: the exact tracked self route must genuinely dispatch design, refuse, retry and pass, with the retained lifecycle/ledger and stop controls. Neither this codex-fast run nor `epic-192-issue-171-live-self-bun-c69c9fa3` qualifies; the latter reportedly parked before design. Cost: live provider/journal access can keep the criterion open; no substitute recipe, fallback or fabricated receipt. |
| Comparator | Both recovery positions support the prior dated-marker choice. Retain it; reject immediate comparator redesign because no compatible macro-provenance mechanism has been demonstrated. Cost: ordinary if-shaped release flips remain falsely accepted by the check. Documentation satisfies the issue's offered marker alternative only; older refusal obligations remain open and 0050's normative requirement is unchanged. |
| Completion | Adopt robustness's separate proof claims and simplicity's compact receipt index. Reconcile every T08/R04/D04 sub-obligation; D04 spans AC1–AC13. Reject checkbox closure from the four narrower issue bullets. Cost: broader missing evidence can block closure without authorizing unrelated code repair. Keep the full engine lane and four independent reviewers. |

### Diagnostic contract

After pinned resources and launched-engine identity are established, verify
records any ordinary `cargo test` failure, continues through successful pinned
compile to one canonical coverage invocation, and then returns `fail` even if
that invocation succeeds. Other preflight/identity failures still refuse
before execution; earlier fmt/clippy or compile failures need not continue.
No candidate-controlled flag selects this behavior. A failed coverage command
also returns `fail` and preserves the earlier test/integrity diagnostics.
Successful candidates still execute every required gate.

The coverage gate runs the full instrumented suite with failure status retained,
then exports its report from the same fresh build/profile directory. It may
collect diagnostics after failing tests; it must never emit a passing verdict
from that run, even at exact 100/100/100. Missing or invalid reports refuse;
all production-inventory, expansion, exclusion and exact-count checks remain.
The combined red fixture must produce an actual uncovered-production diagnostic
and retained LCOV/counts, not just an instrumented-test-failed message. The
comparator function and its allowance remain byte-identical.

## Original synthesis — 2026-10-24, prior run (historical)

The table below records the original run
`part-of-epic-192-close-every-ope-0db51b58`. Its implementation instructions
are superseded by the recovery rulings above; its proof obligations remain.

| Question | Ruling and cost |
|---|---|
| Proof ownership | Adopt simplicity's existing policy, integration test and canonical proof owners; adopt robustness's engine/candidate/manifest identity binding and actual refusal controls. Reject a second evaluator, new application crate and duplicated script tree. Cost: the exact recorded engine core may block execution. |
| Evaluator ingress | Adopt robustness's representative real aggregation regressions alongside the complete evaluator matrix. Reuse the three pinned engine patches and their tests. Reject patching or upgrading the engine merely to gain a test interface. Cost: source/build provenance and upstream regression receipts are required. |
| Live self run | Adopt both positions' historical-receipt reconciliation first, then a real isolated run if no qualifying evidence exists. Reject compile success, inherited self resources and this codex-fast run as substitutes. Cost: providers and a complete self lifecycle, including a real retry, may be necessary. |
| Pins | Adopt simplicity's fixed reviewed SHA-256 expectations and robustness's deleted/commented/disabled-call mutants. Pin complete canonical validator and verify-seat bytes in the existing Rust test; compare public shim bytes with the protected manifest. Reject a universal shell parser or self-derived expected hash. Cost: legitimate canonical edits require explicit digest review and ownership coordination. |
| Comparator | Choose simplicity's dated factual marker, the alternative explicitly offered by the issue. Reject robustness's immediate strict-comparison repair: no demonstrated small provenance mechanism separates honest debug assertions from ordinary if-shaped debug flags in the present expansion. Do not remove Store assertions or change production behavior. Cost: the existing false acceptance remains; finding 3.4 closes as documented, never repaired or an accepted exemption. Adopt robustness's insistence that any future semantic exception waits for operator acceptance. This explicitly supersedes intake's preference for a red comparator fixture; it does not supersede 0050's normative equality requirement. |
| Scope and process | Choose a new slug rather than simplicity's reuse of the old one, to isolate NEXT-171 from the wider AC1–AC13 repair history in the chief's identifiable commit. Keep this continuation small and link the old owner. Draft 0062 because the clarification touches a ruling, while preserving accepted semantics. Cost: one proposed record and later index/dated-pointer maintenance. Retain the engine lane and full four-seat review. |

## Requirements

The policy, phase/result vocabulary and safety-fact vocabulary stay unchanged.
For the original residual matrix, set `spec_defect=false` and
`fixes_applied=false` explicitly. A valid above-medium severity selects
`REVIEW-RESIDUAL-ABOVE-MEDIUM` → stop/hard even with invalid security.
Explicit false security plus supported severity at most medium selects
`REVIEW-RESIDUAL-OK` → ship/flagged. All remaining residual rows select
`REVIEW-RESIDUAL-UNSAFE` → stop/hard. Additional scope/fixes cases exercise
the complete current policy, including earlier routing in Codex overlays;
never filter away rules to make expected outcomes true.

Every receipt identifies the candidate commit/tree, engine binary digest,
recorded base and ordered patches, bundle manifest/digest, policy digest,
fixture or harness digest, command, exit, expected and actual outcome, and
retained diagnostics. Engine executions additionally identify the journal,
run, attempt and event sequences. Missing or partial rows are incomplete,
not successful. No secrets enter receipts. Raw evidence remains ignored;
`specs/delivery-proof-residuals/receipts.md` is the implementation's committed
index of retained, independently readable evidence, hashes and obligation IDs.
Links to unavailable ephemeral files cannot close an obligation.

Implementation owns evaluator and mutation receipts; the authorized root
execution context owns provider dispatch and read-only journal exports;
hermetic verify owns final gate verdicts; root owns integration and CI.
Use `.forge/epic192/next-171/<attempt>/` for `preflight.json`,
`evaluator.json`, `aggregation.json`, `pins.json`, `verify-dispatch.json`,
`self-live.json`, `design-stops.json`, `comparator-observation.json` and
`gates.json`, with adjacent logs and immutable retained exports. These are
planned paths, not evidence asserted to exist.

## Acceptance Criteria

1. **AC1 — Real evaluator (3.1; old AC8/T08).** Run literal JSON cases through
   the exact recorded brokkr evaluator or a harness linked to its recorded
   patched core. Cover every supported severity crossed with both security
   booleans; each safety field absent, null, and wrong-typed; both absent;
   unknown and wrong-case severity; every invalid-security form crossed with
   every above-medium severity; and security-hold/clean controls. Wrong types
   include boolean-like strings, numbers, arrays and objects. Assert rule ID,
   destination and severity per row, with no fixture coercion or defaults.
   Add absent/null/malformed/true scope and fixes controls to the explicitly
   safe original matrix. Assert independent expected table cardinality, unique
   case IDs and complete execution count; zero rows or a partial prefix fails. Record all outputs. A
   permissive-policy negative control must demonstrably fail assertions;
   restored policy passes. Compilation must be warning-free and unsuppressed.
   Representative missing/malformed member facts also traverse the real
   recorded aggregation path and remain unknown rather than safe.
2. **AC2 — Live self (3.2; old AC7).** One retained, qualifying live run is
   governed by the identified `bundles/self` manifest from intake through
   local ship/done, using actual providers and hermetic exec seats. The
   dispatched final design check refuses an intentionally incomplete chief
   commit, the policy retries once, a corrected committed HEAD passes, and
   implement advances only after `designed`. Link run/attempt/sequence IDs,
   governing manifest, engine identity and local ledger. A bundle name or a
   historical count is insufficient. Separate actual-dispatch controls prove
   second failure stops and leaked phase-level `drafted` cannot advance.
   No push, tag, publication or issue closure is part of this proof.
3. **AC3 — Pins (3.3).** `tests/delivery_constitution.rs` compares every public
   shim's bytes with `shims.sha256`, protects the manifest with a fixed approved
   expectation, and pins the exact seven-member set.
   Independently reviewed fixed hashes cover canonical design-check and
   verify-seat, supplemented by named assertions for launched-engine
   resolution, digest comparison, missing/mismatched refusal and the pinned
   delivery-proof invocation. Both `bash scripts/` and `bash ./scripts/`
   candidate invocation forms fail the verify guard. Bounded disposable
   mutants must fail for appended shim bytes retaining target text; deleted,
   commented-out or disabled identity enforcement or delivery-proof calls;
   each candidate path form; and an existing exit-zero validator file.
   Restored controls pass. Expected hashes are never regenerated by tests.
4. **AC4 — Actual boundary (old AC4/AC5; R04).** Actual pinned verify dispatch
   on a candidate containing both an uncovered production line and an
   exit-zero public coverage shim records both integrity and canonical
   coverage failure in the same failing seat receipt, despite the strengthened
   Rust suite failing on the shim. The canonical coverage report identifies
   the uncovered production location and exact missed counts. Test failure is
   sticky in both scripts: a separate covered-but-failing-test control cannot
   pass even if its coverage counts are complete. Missing/invalid reports and
   missing pinned resources refuse; no diagnostics execute with unproved
   engine identity. A covered clean twin passes. Actual dispatched validator
   refuses missing risks even after its
   candidate copy is replaced by exit zero; the selected pinned algorithm
   remains unchanged. Engine identity absent/mismatch cases refuse. Component
   selftests and text checks remain distinct, insufficient witnesses here.
5. **AC5 — Comparator marker (3.4).** Append a dated addendum explicitly
   referring to 0050 §2, without rewriting its accepted body: the release
   comparator admits equal-indentation `if true {` → `if false {` without
   establishing macro provenance, including ordinary `cfg!(debug_assertions)`
   branches. Link proposed 0062 and a retained observation. State that this
   documents a known limit, grants no exception and does not prove release
   equivalence or refusal of arbitrary flips. Preserve the existing comparator,
   exact coverage floor and honest debug-assert controls. A synthetic textual
   reproduction must be labelled textual; any real-build observation must be
   labelled separately. Invoke the actual comparator in a bounded textual
   regression: the documented shape is admitted only for release, while
   no-coverage and changed-body controls refuse. No real-build refusal is
   claimed by this option.
6. **AC6 — Reconciliation and completion.** Update current instructions in
   `specs/delivery-gates/{spec,plan,tasks}.md` to accepted 0050/0056/0057 while
   preserving dated history and original obligations. Index 0062 as proposed.
   Link every T08/R04/D04 sub-obligation to matching receipts, including the
   broader old AC1–AC13 obligations of D04. Check those boxes only when all
   their proofs exist; unresolved historical work remains explicitly open and
   blocks a claim that the requested closure is complete. No rebuilding
   unrelated repairs or borrowing another candidate's gate verdicts.
7. **AC7 — Final verification.** All gates in the single charter inventory at
   [house rules](../../docs/house-rules.md#gates--part-of-every-change), the
   fail-closed native-tls proof, pinned design selftest and delivery proof
   pass on the restored final candidate. Required CI-only evidence remains
   CI-owned. The issue uses engine/full design plus four independent review
   lenses under 0057; official DeepSeek Flash only, DashScope GLM-5.3 for
   tests, no provider substitution. No frozen or public runtime contract,
   application dependency, license allowlist or coverage denominator changes.

## Preconditions and present evidence

Recovery starts from clean tracked HEAD `a76c5e5` and retains its design.
The candidate-local engine digest is
`e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`,
matching tooling and the copied probe receipt. The copied core at
`.forge/epic192/brokkr-build-source` and the probe's real
`compose::resolve` / `Machine::from_table/evaluate` calls are available.
The receipt records 102110 cases and zero failures under a broad oracle;
rule/severity were logged, not independently asserted. This is real but
insufficient AC8 evidence. Preserve it and its host-specific runner unchanged.
New evidence belongs under
`.forge/epic192/next-171/epic-192-issue-171-recovery-on-t-181c6f9c/`.
Verify source composite, recorded lock applicability and current policy hashes
before reuse; the historical build receipt's `staged_not_activated` does not
establish launch provenance or live-provider success.

The copied fleet snapshot is dated 2026-10-23T20:05:53Z. The canonical shared
journal and sibling workspaces are unavailable in this box. Root refreshes
reservations before overlapping writes, specifically NEXT-172's verifier
preflight, NEXT-206's tooling/digest hunk and NEXT-187's 0062 reservation.
The only additional runtime surface authorized by this recovery is the bounded
post-test diagnostic path in verify and coverage; tooling, providers, policy,
comparator and engine patches remain unchanged. Unavailable coordination
leaves that dependent proof open, not a global bar to independent local work.

The reported live-self attempt
`epic-192-issue-171-live-self-bun-c69c9fa3` parked before design after repeated
DSH transport failures; no raw current transport diagnostic is supplied.
It cannot close AC2. The authorized root context retains the next real
attempt's provider/model, timestamp, run/attempt/event IDs and exact diagnostic
if still blocked. Do not assert today's provider availability from history.

Historical chief-check logs under `.forge/epic192/next-171/chief-checks/`
record workspace test exit 101 and coverage exit 1 for unindexed 0062.
Other historical component passes are not this recovery's final gate proof.
Implementation must add the proposed index entry and satisfy all remaining
criteria; chief's strict four-file artifact boundary cannot repair that index.
All implementation boxes remain open at design.

## Non-goals

No Store, SMTP, await, pruning, extraction, webhook, release pipeline, recipe,
policy or engine behavior change. The bounded gate diagnostic change above
is required; it never relaxes admission. No generic policy framework, shell parser,
compiler-provenance mechanism, parallel delivery launch by this design seat,
or invented journal events. Accept == stored, subscribe before checking,
oldest-only pruning and ingest-time extraction retain their accepted meaning.
