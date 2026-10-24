# Tasks — Align the required merge checks

Ordered implementation tasks for [spec.md](spec.md), using [plan.md](plan.md).
These checkboxes track this candidate's receipts; earlier completion claims
do not carry forward. The dated clarifications and prior failures remain in the
spec as superseded history. The selector correction is already in the base.
Chief's design commit and local preflights do not mark implementation complete.

- [x] **T1 — Confirm bounded inputs and record provenance (AC2, AC3).** Verify
  the README and operator rulings in their directory and the candidate-local
  engine digest. Carry the dated operator observation into implementation
  receipts without inventing raw app/ruleset/helper evidence.
  **Test:** `sha256sum -c SHA256SUMS` passes for both packet files; the engine
  digest equals the supplied pin and tooling record. Receipt review identifies
  date/observer/settings/green-main SHA and distinguishes local proof.
- [x] **T2 — Align Gates and contributor ownership (AC1, AC4).** Add the exact
  nine-context main-required list and dated attribution to house rules. Keep
  existing commands and CI/publication scopes. Correct CONTRIBUTING's
  native-tls wording and link its required-main guidance to the sole inventory.
  **Test:** run-local comparison passes nine unique exact names against both
  the independent spec receipt and workflow job producers/matrix. Diff review
  confirms preserved commands, CI scopes, one registry and only scoped Markdown.
- [x] **T3 — Prove inventory comparison sensitivity (AC1).** Retain the small
  comparator and its outputs in ignored implementation evidence, using scratch
  fixtures and untouched packet bytes.
  **Test:** the actual comparator accepts baseline and separately rejects
  missing brokkr, missing Docker, missing an existing check, duplicate,
  misspelling, added tag-rate context and added publisher context. Each control
  reaches semantic comparison instead of stopping at an input hash mismatch.
- [x] **T4 — Reconcile actual progress and evidence (AC2, AC3, AC4).** Update
  these three artifacts from implementation receipts; preserve dated history,
  bounded scope and explicit CI/operator ownership. Remove no historical
  failure and promote no historical pass to candidate proof.
  **Test:** read spec/plan/tasks together and search for obsolete current
  selector/watcher/live-proof/archive prerequisites. Confirm any such mention
  is history or an explicit out-of-scope disposition, not acceptance work.
  Verify no unsupported app/ruleset claim and no duplicate maintained inventory.
- [x] **T5 — Prove and commit the implementation candidate (AC5).** Run every
  authoritative house gate with the digest-matched candidate-local engine and
  preserve candidate-bound logs/exits, including exact coverage and selftest.
  Commit scoped files unsigned; leave no uncommitted implementation.
  **Test:** all local gate results pass, fail-closed native-tls and shim checks
  pass, and the pinned proof chain retains design selftest/tamper-boundary
  obligations. `git diff --check` is clean and commit/worktree inspection
  matches the proved candidate. Failed or unavailable proof is recorded,
  never converted to a pass or bypassed by a documentation-only argument.
- [ ] **T6 — Hand the committed candidate to protected verification/review (AC5).**
  Preserve the existing engine route and candidate identity through verify,
  five distinct 0058 reviewer results and ship. Record scoped receipts through
  the existing run process; no new archival service or remote prerequisite.
  **Test:** engine-produced pinned verify and separate correctness/security/
  tests/guarantees/independent results bind to the committed candidate, with
  worst-verdict and security-hold behavior intact. Chief positions/preflights
  are not counted as those results. No push, merge, tag, publication, decision
  acceptance or issue closure is performed by this task.

## Implementation receipt — 2026-10-24

T1–T4 are complete: packet hashes and pinned engine identity matched, the
house rules and contributor wording are aligned, the baseline comparison and
all seven independent negative controls passed, and current scope/evidence
was reconciled across these three artifacts. The comparator source and output
are retained under .forge/epic192/issue205/implement-47e37eec/.

T5 records the committed-candidate gate and integrity proofs at the run-local
receipt path. T6 is the handoff to the engine's protected verification and
review phases; the implement seat
does not claim those results. Live refusal experiments remain operator-owned
and out of scope, not pending acceptance work for this implementation.

## Ownership outside this delivery

Live branch settings and throwaway-PR red-check merge-refusal experiments are
operator-owned and out of scope. Watcher discovery/repair/activation and absent
historical raw-packet archival are not prerequisites for this bounded repair.
There are no pending external acceptance checkboxes for this run. This design
claims neither that experiments happened nor that any external watcher changed.

CI owns Docker/Helm/network/macOS/port-80/release evidence under the single
house inventory. Namespace proofs are unrelated to this feature. Those owners
retain their responsibilities; local tests do not claim their results. The
operator controls issue closure separately from this documentation delivery.
