> Historical #195 artifact restored on 2026-10-25. The current port and its
> proof obligations are governed by [webhook-ca-port-lows](../webhook-ca-port-lows/spec.md).
> Earlier stops and gate receipts below are dated history. 0067/0068 are
> accepted; 0069 was accepted for #220 on 2026-10-25 and is not implemented here.
> The low shared-blocking-pool availability residual is tracked by #220.

# Tasks — webhook pool review

Run: `continue-epic-192-issue-195-from-6086030b`; baseline 4f794ba.
Execute in order. Each task is paired with its proof; AC numbers refer to
[spec.md](spec.md#acceptance-criteria). A checked task is not a security verdict.
The historical records below describe earlier executions, not this run.

- [x] C1 (AC1/AC6): Reconcile both current positions, intake, actual decision numbers and recovered evidence. **Proof:** chief-6086030b/provenance.json records all 12 hash/size matches, HEAD/tree/source blobs, no source/test diff from d8dcf4b and its ancestry; the two unlisted files remain unlisted and no live refresh is claimed.
- [x] C2 (AC1/AC2/AC3/AC5): Revalidate current contract and preservation behavior without a duplicate production patch. **Tests:** served GET regression, existing API/webhook/construction suites and selector tests via the workspace suite; retain historical sanitized red/restored-green receipts with their original provenance.
- [x] C3 (AC7): Verify the existing decision index and every local house gate on this draft; record candidate, executable, command and exit. **Tests:** store_contract_docs inventory/status tests, complete house gate inventory, fail-closed native-tls, shim integrity and pinned design/delivery proof selftests. A failed command stays failed; local checks do not impersonate hermetic verify.
- [x] C4 (AC4/AC7): Commit exactly spec.md, plan.md, tasks.md and existing proposed 0069 with explicit no-queue ruling, costs and hard stop. **Proof:** exact staged paths, unsigned HEAD commit, clean four artifacts and passing pinned structural preflight. Leave this commit last; write only the ignored chief result afterward.

## Later obligations — blocked before semantic implementation

- [ ] L1 (AC4/AC8): Reconcile authenticated current issue framing; resolve numeric W/B with J = W, byte/copy/output formula, oversized-input posture, fairness, worker recovery and shutdown ownership. Obtain the operator's explicit authorization and necessary acceptance of the exact amendment. **Proof:** dated ruling tied to the completed proposal and accepted-clause map; absent authority means STOP.
- [ ] L2 (AC4/AC8, BLOCKED by L1): Implement only the authorized bounded CA facility across GET/PUT/delivery. **Tests:** exact resource limits before copies/submission, no pending queue, retained-result accounting, refusal/cancellation, duplicates/churn and value-free worker recovery; prove actual work owns capacity.
- [ ] L3 (AC2/AC3/AC5/AC8, BLOCKED by L1–L2): Prove isolation and lifecycle without changing privacy or frozen guarantees. **Tests:** real protocol progress including persistent commit/250/immediate queryability, exact resource ownership, shutdown/ledger/no-late-publication, complete-root generated-key wire/TLS/fan-out and workspace regression suites. Real servers and deterministic synchronization; follow 0020 placement.
- [ ] L4 (AC7/AC8, BLOCKED by L1–L3): Run all house gates and actual pinned verifier on the final authorized candidate. **Proof:** candidate/executable-bound exact coverage and selftest, publish/seal/license/native-tls and dispatch evidence; CI-only obligations remain CI-owned.
- [ ] L5 (AC6/AC8, BLOCKED by L4): Obtain five separate protected 0058 results: Astra correctness, official DeepSeek Flash security, Sonnet 5.5 tests, Astra guarantees, independent Fable 5.1. **Proof:** actual provider/route/candidate and independent result identities. True/absent/null/malformed safety facts stop; only authorized security resolves its finding. Never Pro, old-panel substitution or unauthorized fallback.

## Current chief execution — 2026-10-24 run 6086030b

C1–C3 evidence is under `.forge/results/chief-6086030b/`: provenance.json,
checks.json and hashed command logs. All 12 recovered entries match their
manifest; source/tests and d8dcf4b ancestry are preserved. The workspace suite
passes, including the real served GET test, all 14 construction tests, all 29
webhook wire tests, selector tests, decision-index inventory/status and frozen
behavior regressions. No fresh mutant was executed; sanitized historical
removal-red/restored-green and privacy receipts retain their original scope.

Every local house gate passes on the four-file draft over 4f794ba: fmt, clippy,
workspace tests, checksum-matched pinned brokkr compile, exact coverage and
selftest, publish/seal proof and licenses. Coverage is 13144/13144 lines,
778/778 branches and 1463/1463 functions. Shim integrity, fail-closed native-tls
absence, design-check selftest and delivery-proof selftest also pass. The
native-tls command's expected exit 101 names an unmatched package, not a generic
dependency failure. The workspace tool response timed out at 300 seconds; the
saved receipts show the runner completed every command successfully afterward.
No timeout is counted as a passing command.

Receipts bind the tested draft's hashes and binary identity; final editorial
formatting and this outcome note follow those checks. C4's exact-commit and
structural preflight receipts are written after committing, under the same
ignored directory. The sequence's actual pinned validator owns its verdict.
These are local design checks, not a final hermetic verifier or five-office
protected review. CI-only proofs, runtime remediation, completed proposal
choices, operator acceptance and security resolution remain outstanding.
Historical true/low remains unchanged. L1–L5 remain unchecked across the stop.

## Historical record boundary — 2026-10-24 run 6086030b

The following entries are preserved from the earlier continuation. C1–C4 and
L1–L6 within those entries refer to that earlier task list and candidate, not
the list above. In particular, both earlier positions ran on d8dcf4b; this run's
robustness position ran only the served contract test on 4f794ba, while simplicity
reports 14 construction tests plus that test. The previous missing-index and
coverage failures are historical failures; the current gates must establish
current outcomes. No earlier checked box or donor gate satisfies them.

## Clarification — 2026-10-24 current continuation

The following dated entry preserves f2fc20b's earlier T1–T9 evidence. Its words
“fresh” and checked T1–T7 refer to that earlier execution, not this chief or
current verification. Former T8 failed on the stale selector; d8dcf4b corrects
it. Former T9's current protected review remains absent. Use recovered receipts
with their documented limits; the original raw logs need not all survive.
Both current positions separately passed 14 construction tests and one served
contract test on d8dcf4b. Those supply mechanism evidence, not security clearance.
Current task completion is recorded only when actually performed; later tasks
remain unchecked across the stop.

## Implementation evidence — 2026-10-24

T1–T7 implementation proofs are recorded under `.forge/results/`:
`webhook-pool-review-baseline.json`, `webhook-get-500-mutation.json`,
`webhook-pool-api.log`, `webhook-pool-construction.log`,
`webhook-pool-wire.log` and `webhook-pool-resource-evaluation.json`.
Historical privacy mutant receipts remain historical; the generated private-key
response omission is preserved. The fresh GET-only deletion control fails with
exit 101 and restoration passes with exit 0. API tests: 65 passed; construction
harness: 14 passed; webhook wire tests: 29 passed.

The held one-slot GET and cancelled-waiter cases observe DATA reader Pending
and count zero; explicit release restores DATA 250 and immediate count one.
Two held GETs saturate two slots; three slots allow the same finite SMTP
exchange to complete before either GET releases. All cases join their work,
preserve configuration/cache/LRU, and assert watchdog=false. These results
establish coupling and recovery, not an attack threshold or a global bound.
No runtime mitigation or accepted-ruling amendment was selected. 0067 and
0068 remain proposed. The historical security residual remains unresolved;
only the authorized security office can change that disposition.

T8 and T9 remain unchecked until their actual current-candidate proofs exist.
Local command receipts cannot stand in for hermetic engine dispatch or the
five required protected review results. Any failed gate remains a failure,
including unrelated baseline failures; no implementation-complete or safe
security fact follows from the focused tests above.

## Chief execution record — 2026-10-24

C1: independently verified all 12 recovered manifest entries, both current
robustness log hashes, exact historical aggregate and unchanged runtime/test
blobs relative to f2fc20b. d8dcf4b changes only the selector test. Issue snapshot
is OPEN; no live refresh. Brokkr SHA-256 matches the current tooling record.

C2: fresh local workspace execution passes all 14 construction tests, the
served GET regression and delivery recipe tests. The separately executed
webhook wire suite passes all 29 tests. Existing privacy-safe historical
mutation receipts remain historical. No runtime or selector edits were made.

C3: the four artifacts are the chief design payload; post-commit structural
preflight is recorded in the chief's ignored command/result evidence. This
completion means draft artifacts only, not authorization or security clearance.

C4: local fmt, clippy, pinned brokkr compile, licenses, shim integrity,
fail-closed native-tls absence, coverage selftest, publish-guard proof,
seal-branch proof and design-check selftest pass. Workspace tests fail (101)
in the decision-index inventory and its fixture selftest because 0069 had no
index row at that candidate. The later numbering/index correction added the
row; those test failures remain historical and the index test has not been
rerun. Coverage refused (1) on the old instrumented failures; no current exact
coverage success is claimed. These are real failed gates, not waived
requirements. L1 verifies the corrected index on the current tree.

Command/exits/log hashes and provenance are under
`.forge/results/chief-design-checks/` (`base.json`, `proofs.json`, `wire.json`,
`provenance.json`). Commands ran on the four-file draft over d8dcf4b, before
this execution note/final commit; they are local design evidence, not an actual
pinned hermetic verifier result for the final commit. Protected five-office
review, full verification and security resolution remain outstanding. No CI-only
check or implementation completion is claimed. L1–L6 stay unchecked.
