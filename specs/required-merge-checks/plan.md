# Plan — Require the missing merge checks

## Technical context

Implement the scoped [spec](spec.md) for issue #205. This is a GitHub
protection repair plus Markdown in a single Rust crate; there is no runtime
code, dependency, database schema or new service. The executor needs an
explicitly available authorized GitHub capability. Chief has workspace
capability only; this plan grants no external access. No plan template exists
under `.specify/`; these sections follow `dialects/speckit/design.md` and the
repository's established plan structure.

## Constitution check

Keep accept==stored, subscribe-before-check, oldest-only pruning and ingest
extraction unchanged (0001–0005). Preserve dependency policy, exact coverage,
all gates, pinned proofs, unknown-fact refusal and protected review. Commit
unsigned, never push (0056). Gate-policy work remains engine-class under
0057 with the accepted five-result matrix in 0058. No accepted record changes
and no ADR is declared. Proposed release/runtime decisions are outside scope.

Chief commits exactly this directory's spec.md, plan.md and tasks.md. Full
implementation gates and review remain downstream obligations; a structural
design check does not prove live configuration or issue completion.

## Project structure and files touched

| Path/system | Purpose and permitted change |
|---|---|
| `specs/required-merge-checks/{spec,plan,tasks}.md` | Replace obsolete watcher/probe execution instructions; retain dated historical obligation map |
| `docs/house-rules.md` Gates | Downstream: update the one complete verified merge-blocker inventory; retain commands and CI/tag distinctions |
| `feedback-loop-ai/swarmail` main required-status-check subresource | Downstream authorized executor: exact identity-preserving two-context union only |
| Full protection, applicable rulesets, permissions and real check data | Read-only baseline, producer and postcondition evidence; rulesets are not rewritten |
| `.github/workflows/ci.yml`, `scripts/docker-smoke.sh`, `bundles/self/tooling.json` | Read existing producer/proof definitions; no edits |
| `tests/delivery_constitution.rs`, `tests/delivery_recipes.rs` | Run existing regressions; no source-string tests to certify remote behavior |
| `.forge/epic192/issue-205/` | Ignored run-specific raw receipts, comparison, controls and operation record; preserve prior evidence |

No watcher, Rust, Cargo, allowlist, recipe, adapter, gate, shim, tooling,
release or CI edits. No second committed inventory or executable framework.
No probe branches/PRs or external owner changes. Positions stay uncommitted
under `.brokkr/state/`.

## Research and evidence

Both positions and current framing
`.brokkr/state/tasks/triage-and-continue-epic-192-iss-9881cf64.md` were read,
as were the prior three-file proposal and relevant accepted rulings. Earlier
implementation results `5fd46e63-7d69-4f57-8b29-335785c5a4cd` and
`fce52e40-7b58-4944-9fd4-db93b31b33ba` in `.forge/results/` report missing
GitHub capability/full receipts and unknown watcher ownership at `bb8b040`.
They remain historical failures, not evidence of today's remote state.

CI runs on push and pull_request. `brokkr-compile` displays exactly
`brokkr bundle (pinned)` and validates pinned tooling; `docker-smoke` displays
exactly `docker smoke`, has read-only contents permission and a 30-minute
limit, and runs the real smoke script. Existing
`ci_runs_the_chart_history_check_and_the_docker_smoke` checks wiring and
permissions, not protection membership or server refusal. The task's renewed
admin/omission findings lack a fresh complete raw receipt in this workspace.
The earlier watcher search found no owner; current scope removes that discovery
dependency without claiming the watcher is repaired.

## Implementation approach

1. **Establish capability and capture truth.** The authorized executor records
   timestamped permission, full main protection, all applicable effective rules
   and genuine producer receipts. Enumerate complete responses/pages; unknown
   or malformed state stops before writing. Confirm target, admin, strict and
   admin enforcement. Distinguish writable branch requirements from ruleset
   contributions. Preserve old receipts and use a run-specific child directory.
   If access is unavailable, record that prerequisite; do not guess a baseline.
2. **Coordinate and prepare.** Reserve one settings writer through the existing
   operator/driver arrangement, recording owner and interval. Derive `B0`, `Q0`
   and `A` as specified. Inspect the supported narrow API payload, including how
   its context/check representations preserve legacy unbound and app-bound
   requirements. Resolve conflicting bindings. Prepare the nonsecret request
   and comparison; do not serialize the effective union into the branch
   resource. Exercise comparison controls before mutation.
3. **Re-read, then apply.** Immediately re-read under coordination. Baseline
   change invalidates the prepared request; reconcile/recompute and recheck
   scope. Skip an equivalent already-applied mutation. Use only the required-
   check operation, preserving strict if supplied in its payload. Do not change
   admin enforcement or send a full-protection template. Save intent before
   action and response/request ID when available afterward.
4. **Verify and reconcile.** Read full protection and effective rules back.
   Compare `B1 = B0 union A`, `Q1 = Q0`, effective identities, strict/admin and
   all unrelated semantic fields. Sort equivalent entries and remove transport
   metadata only; do not flatten identities. An error, timeout or missing
   read-back leaves UNKNOWN until fresh state resolves it. After interruption,
   inspect remote truth first: the write may have landed without a response.
   Never blindly retry or restore an old snapshot. Unexpected drift requires
   reconciliation without expanding repair scope.
5. **Document verified state.** After comparison passes, update only the Gates
   inventory with every effective exact context and relevant producer constraint
   and source. Compare its set with read-back. Preserve existing commands and
   CI/tag descriptions. Do not create a second maintained list.
6. **Finish proof and handoff.** Commit documentation unsigned and run all
   house gates, hermetic verify and the five reviews. Re-read protection at
   handoff and compare with the recorded result and inventory. Drift invalidates
   a current-state claim. Retain nonsecret raw evidence/checksums and an
   operator-accessible durable handoff; ignored files alone are not guaranteed
   durable after workspace deletion. Record both live refusal proofs PENDING,
   historical watcher/integration work outstanding and no issue closure.
   No probe cleanup is needed or claimed.

## Data model and contracts

These are run-local evidence records, not a production schema:

| Record | Minimum content |
|---|---|
| Observation | Target, UTC time, actor permission, raw full protection/effective-rule receipts, complete pages, exact names/apps and policy source |
| Intended operation | Run identity, coordination owner/interval, baseline, two additions, endpoint/method and nonsecret payload |
| Outcome | Response/status/request ID if available; verified, no-op or UNKNOWN; read-back paths and semantic comparison |
| Control proof | Comparator identity, inputs and expected/actual results for passing union/no-op and refusal cases |
| Handoff | Candidate SHA, final observation, inventory equality, receipt checksums/location, gate/review receipts and pending obligations |

The comparator is small run-local code or an equally reproducible explicit
comparison retained with evidence. Preserve context/app identities,
branch-versus-ruleset ownership and all semantic protection fields. Do not
store tokens/credential headers or add a mock GitHub service. This comparison
proves captured configuration, not live refusal behavior.

## Validation and quickstart

While chief's design commit is HEAD, preflight its structure with:

```bash
bash bundles/self/scripts/design-check.sh check .
```

The sequence's dispatched pinned check owns the design verdict; this local
preflight neither advances the phase nor replaces it.

Downstream, obtain fresh authorized receipts and writer coordination first.
Before writing, prove the comparison accepts the exact union and already-
applied state and rejects: missing Brokkr, missing Docker, removed original
tuple, changed producer binding, strict=false, admin=false, unrelated protection
change, ruleset change and stale pre-write baseline. Incomplete/malformed
observations produce unknown/refusal. Simulate an interrupted response followed
by a fresh already-applied observation to prove reconciliation makes no duplicate
write; unresolved read-back stays UNKNOWN. These controls are not live refusal
experiments.

After documentation changes, retain existing targeted regressions:

```bash
cargo test --locked --test delivery_constitution
cargo test --locked --test delivery_recipes
```

They may be evidenced within the full workspace test run rather than rerun.
Run every command in the authoritative
[house inventory](../../docs/house-rules.md#gates--part-of-every-change), with
locked Cargo operations, the candidate-local digest-matched engine and pinned
proof ownership. Include exact coverage/selftest, publish/seal proofs,
licenses, fail-closed native-tls absence and shim identity checks. Do not
maintain a second gate list here or weaken it for a documentation-only diff.
Keep CI-only obligations separate; offline verify installs nothing.

The engine dispatches five separate 0058 reviewers: Astra correctness,
official DeepSeek Flash security, Sonnet 5.5 tests, Astra guarantees and
independent Fable 5.1. No engine-lane fallback and never DeepSeek Pro. Missing
gate/reviewer results stay outstanding. Design authoring and the comparator
do not count as these results. Root integration, live negative proofs and
old watcher obligations remain outside this run.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Missing capability repeats prior failures | Explicit prerequisite before action; preserve unknown status rather than bypass the seat boundary |
| Required checks/app bindings lost | Complete tuple-preserving union, narrow operation, negative controls and full read-back |
| Ruleset policy copied or altered | Separate writable/effective sets and sources; read-only rulesets and semantic equality |
| Concurrent administrator edits | Coordinated interval and immediate/final reads; recompute on drift; accept residual race without claiming API CAS |
| Disconnect after successful write | Intent/response record, UNKNOWN until read-back, fresh reconciliation and no stale rollback |
| Wrong producer or truncated response | Genuine receipts, full pages and identity-aware comparison; conservative refusal costs time |
| Inventory drift or tag checks added to PRs | Document verified full effective set in one Gates section and preserve scope distinctions |
| Read-back mistaken for refusal proof | PENDING red-Brokkr/red-Docker entries and unclaimed historical obligations; #205 stays open |
| Two more jobs delay/block merges | Existing jobs are intended proof costs; do not soften requirements or add bypasses |
| Evidence lost with workspace | Raw retention, checksums and durable operator handoff; no secrets or journal fabrication |
| Small diff bypasses review | All house gates and five distinct 0058 results; protected review/security hold unchanged |
