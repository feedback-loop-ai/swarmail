# Delivery proof residuals

Created: 2026-10-24. Status: design draft. Story: NEXT-171, epic #192,
findings 3.1–3.4. Baseline: `45d68df37044de3360439b34c39687975dd59b84`.
Run: `part-of-epic-192-close-every-ope-0db51b58`.

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

## Synthesis and rulings

Read both run-local positions at `.brokkr/state/design/positions/` and the
intake framing under `.brokkr/state/tasks/` for this run. The following rules
resolve their disagreement rather than combining incompatible alternatives.

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
   safe original matrix. Record the complete case count and outputs. A
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
   shim's bytes with `shims.sha256` and pins the exact expected member set.
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
   coverage failure in the same failing seat receipt. A covered clean twin
   passes. Actual dispatched validator refuses missing risks even after its
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
   labelled separately. No real-build refusal is claimed by this option.
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

This chief observed a clean tracked baseline and equal local HEAD/origin/main
at the commit above; this is an attached branch, so detached fast-forward
instructions do not apply. No network freshness is claimed. The local engine
hash is `e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`,
matching tooling; its help has no evaluator command. Matching core source
and historical live receipts are not supplied in this box.

The intake records a live canonical journal inspection and candidate overlap
check on 2026-10-24. This chief inspected its framing, fleet snapshot and
worktree registry; the canonical journal and sibling worktrees are not mounted
here. Prunable registry entries do not establish inactivity. No conflict was
recorded for this new spec/decision or the bounded regression/doc paths.
Before implementation, root must refresh live ownership, exact-core access
and qualifying self receipts/provider availability. Tooling changes and
verify-seat preflight rewrites stay outside scope (#172/#206 reservations
require reconciliation). Park the dependent execution with the missing path,
identity or provider; never turn these missing inputs into a proof pass.

## Non-goals

No Store, SMTP, await, pruning, extraction, webhook, release pipeline, recipe
or engine behavior change. No generic policy framework, shell parser,
compiler-provenance mechanism, parallel delivery launch by this design seat,
or invented journal events. Accept == stored, subscribe before checking,
oldest-only pruning and ingest-time extraction retain their accepted meaning.
