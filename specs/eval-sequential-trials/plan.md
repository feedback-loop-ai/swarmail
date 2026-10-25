# Implementation plan — sequential eval trial port

Date: 2026-10-25. Implements [spec.md](spec.md) and proposed
[0076](../../docs/decisions/0076-sequential-eval-trials.md) on c13a507/428a8ec.

## Technical context

Rust 2024 standalone unpublished evals crate, own workspace/lock, Tokio Unix
process groups, real loopback SMTP/MCP/fixture HTTP and existing standard file
locking/libc. No new dependencies, manifest/lock edits or root runtime changes.
No plan-template.md exists in this checkout; this plan supplies the requested
technical context, constitution check, structure, research, data model,
contracts, quickstart and risks inside the four authorized design artifacts.

## Constitution check

Preserve accepted 0001–0005, 0020 deterministic proof placement, 0045 §8,
0050's pinned gates and 0056 unsigned commits. The verified scoped #199 ruling
permits sequential behavior and dated 0055 amendment, not acceptance of 0076.
Keep accepted 0071 and shipped proposed 0063 behavior. The four-file chief
commit excludes docs/decisions/README.md; T1 adds its single proposed row before
whole-gate success. Do not rewrite accepted bodies or their status.

No gate edits, coverage exclusions, floor reductions, dependency widening,
routing changes or source-server fixes. Use fresh inboxes, chaos 0/100, observed
readiness and causal release; no wall-clock validity assertions or sleep-based
synchronization. Tests involving only ordinary protocol operations retain
concurrency; coordinate only parent scopes that produce eval trials.

## Research and resolved integration

All seven packet checksums and candidate engine digest match. port.md lists ten
textual conflicts; semantic composition also reaches result readers and current
provenance/trap fixtures. Use patch 03 as a behavior/test reference, not wholesale
replacement. Patch 02 supplies historical design; patch 01 is earlier framing,
not new candidate evidence. Intake and both positions resolve every design
choice needed locally. No GitHub, journal or sibling checkout is required.

Current runner::run_trial wraps execute_trial with target recheck and finalize.
Admission must enclose that wrapper. Current RuntimeDirs::create/apply already
provides closed HOME/TMPDIR/XDG maps. Groups/OwnedGroup::ensure_cleanup own
cleanup; extend their ownership instead of adding a supervisor. main's
matching_trial_row/run_child_assigned require bounded matching row/status and
artifacts::matches_cell plus agent/target identities. Preserve those checks
when adding finite diagnostic capture. Current result/catalogue/snapshot are
already 2; artifacts have separate existing envelope versions. #225's actual
cleanup-entry/Pending test is newer authority than old probe timing changes.

## Project structure and files

| Files | Implementation purpose |
|---|---|
| evals/src/admission.rs; evals/src/lib.rs | Restore narrow private admission and module registration. |
| evals/src/runner.rs | Request/row concurrency, admitted task lifetime, current cleanup ownership, target recheck/finalize integration and causal test seams. |
| evals/src/main.rs | CLI validation, sequential worker admission categories, current context checks and suffix accounting; update inline fixtures. |
| evals/src/results.rs, report.rs, artifacts.rs | Use shared TrialResult validation; adjust only concurrency-dependent construction/validation if needed, without new readers or scoring. |
| evals/tests/common/{mod,coordination}.rs | Shared parent-test-scope coordination and existing helper adaptation. |
| evals/tests/{proof,runner,report,boundary,provenance,identity,scoring,fixture}.rs and existing data fixtures | Audit every trial-producing scope and row fixture; restore sequential controls and preserve newer provenance/trap/cleanup assertions. Only touched owners need edits. |
| evals/README.md; evals/run-plan.md | Current CLI/limit contract and linked local #151 sequential baseline procedure. |
| docs/decisions/README.md; docs/decisions/0055-agent-eval-harness.md | Exactly one proposed 0076 row and dated scoped amendment preserving accepted history/main addenda. |
| specs/agent-eval-harness/{spec,plan,tasks}.md | Explicit local issue-#148 AC4 replacement; supersede live overlap proof without changing positive-control AC4. |
| specs/eval-sequential-trials/{spec,plan,tasks}.md; docs/decisions/0076-sequential-eval-trials.md | This design and subsequent truthful implementation progress. |

Root src/, root/eval manifests and locks, deny.toml and bundles stay outside
implementation scope. Existing scorer/fixture behavior needs no production edit.

## Admission and ownership approach

1. Restore Admission with stable `/tmp/swarmail-evals-<effective-uid>/trial.lock`.
   Create/check owner-only 0700 directory and 0600 regular single-link file;
   retain no-follow, close-on-exec, nonblocking open/lock and directory/file
   descriptor-to-path device/inode checks. Wrong owner/type/mode or inaccessible
   state fails closed. Standard try_lock distinguishes contention from failure.
   Closing releases; never unlink or truncate the shared rendezvous. No public
   test-path override, waiting queue, fairness or stale-pid recovery.
2. Add request concurrency and validate default/1 before trial work. Allocate
   the existing assigned-or-fresh trial ID once, retaining request provenance.
   Admission failures use current refused_row/finalize as applicable, with
   complete typed availability and concurrency 1. Refusal finalization may
   retain evidence but must not launch trial preparation/agent children.
3. The admitted task owns the current complete run flow; awaiting its handle
   does not own its lifetime. Dropping the library waiter on a surviving Tokio
   runtime leaves the task and reservation alive. Share admission through
   Groups, OwnedGroup, existing reapers/drains and private-state owners. Keep
   outer ownership through target recheck and finalization; ensure private
   state is disposed before the final reservation reference is released.
4. Preserve signal subscription before source/build/trial helpers, body abort
   and join, latched stop, real direct waits and concurrent fixed TERM/KILL
   budgets. Extend the existing owner on unproved cleanup: reporting an error
   does not prove cessation. Retain reservation/private state while ownership
   remains live or unknown, emit `trial_cleanup_unproved`, and stop matrix
   admission. No second cleanup task hierarchy or renewed cleanup budget.
5. Matrix parent keeps product planning/context ownership without acquiring a
   trial reservation. Its workers acquire through the same trial path. Restore
   only bounded finite category capture needed for refusal/unproved cleanup;
   discard unknown stderr, drain output, preserve bounds and current context/
   retained equality checks. Propagate cleanup stop even with a valid stdout
   row; retain committed prefix and zero-time refused suffix, no retries.

## Data model and contracts

TrialRequest adds supported concurrency with default 1; direct library
construction is updated everywhere. CLI trial/matrix expose --concurrency.
Other parsed integers refuse with concurrency_unsupported; malformed syntax
uses arguments_invalid. Busy admission is concurrent_trials_refused; unsafe/
unavailable admission is trial_admission_unavailable. These diagnostic
categories remain separate from the unchanged eight-class failure taxonomy.

TrialResult adds required integer concurrency, valid only at 1, beside current
schema_version 2 and required provenance. Set it in every row constructor and
fixture, including fallback, retention error and never-started matrix rows.
Shared validate plus strict deserialization serve report, worker and retained
readers. No serde default or decoder fallback. Preserve native argv privacy,
seed/time/coordinate binding, target/harness/agent identity and finite reasons.
No change to catalogue/snapshot/result business versions or artifact envelopes.
Existing pre-concurrency v2 evidence needs its historical reader.

Fresh HOME uses existing per-role RuntimeDirs. Multiple agents in one trial
remain allowed; concurrency counts supported trials, not agent processes.
Use actual retained check/replay, with current tasks::score as sole scorer;
no matching-row shortcut may bypass context validation or retained equality.

## Proof design

Restore named shipped controls in their existing suites:
sequential_admission_rejects_unsafe_rendezvous,
sequential_admission_refuses_independent_cli_trials,
sequential_admission_library_and_cleanup_lifetime,
sequential_trial_concurrency_requests, sequential_rows_record_concurrency,
sequential_rows_reject_missing_or_invalid_concurrency,
sequential_matrix_refuses_overlap_and_unsupported_concurrency,
sequential_poacher_control_refuses_competing_trial and
sequential_contracts_and_decision_links_are_honest. Expand current owners'
controls for provenance and finalization rather than importing old fixtures.
Filters that match zero tests are not proof; inspect the executed inventory.

Use the inert acknowledged victim/contender/release test described in AC6.
Temporarily bypass admission only in the test build; require failure at the
refusal assertion, then restore. Separately release admission early under a
held-cleanup test; require its reservation assertion to fail, then restore.
Schema, compile or tooling errors do not count as red controls. No exploitation
client is required. Keep #225's actual Pending handshake and current both-signal,
source-helper, envelope-expiry, privacy and no-clobber assertions intact.

A distinct test-only coordination file serializes trial-producing parent tests
across test binaries, including inline probes and provenance/trap tests. Acquire
once per outer test; contenders and child probe helpers must not acquire it.
Never replace production try_lock with blocking admission or set global Cargo
test threads to one. Sequentialize per_task's trial fan-out and replace
`two_concurrent_isolation_trials_both_pass`; preserve all sixteen oracle/null
controls and within-trial inbox/foreign-token tests.

Two real sequential helper agents write a HOME sentinel and observe its absence
next time. Check distinct writable paths and exclusion of caller HOME. Row tests
cover all emission classes and invalid concurrency through report, worker and
actual retained check, alongside current missing-provenance/context mismatch
refusals. Document tests check links, active versus historical wording, unique
0076 status/index, exact limitation and CI/operator ownership.

## Quickstart and verification

Use preprovisioned tools offline; install nothing. Verify `.forge/bin/brokkr`
SHA-256 equals ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f.
Follow the single [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change).
Run on the committed candidate:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
bash scripts/seal-branch-test.sh
cargo deny check licenses
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --workspace --locked
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
```

Also run the existing fail-closed native-tls absence proof: cargo tree -i
native-tls must fail specifically because no matching package exists; success
or an unrelated failure refuses. Hermetic verify checks shims.sha256 and runs
pinned copies, never candidate wrappers. Root coverage must report zero missed
lines, branches and functions and passing selftest. Evals remains standalone:
root green does not execute it. Full eval suite with default threading includes
focused sequential controls, current runner_cleanup_await_pending_and_bypass_
controls_cover_signals_and_faults and cleanup_handshake_ tests. Preserve current
recipe routing; the engine alone dispatches verify/protected review.

After implementation, `swarmail-evals trial --task signup --agent oracle
--concurrency 1` and sequential matrix use the normal CLI; parsed 2 refuses.
Use current README build/executable paths, source/provenance and retention
options. No baseline measurements or new published outcomes are requested.

CI owns macOS eval through #201 and house-listed platform/network checks;
operator owns live settings, sealing and decision acceptance. These are recorded
owners, not extra local acceptance conditions. No push/merge/tag/publication.

## Design-stage verification observations

All 18 local checks were attempted on design commit d9be175 before this
notes-only amendment; candidate-bound logs and command/exit receipts are
run-local under `.forge/results/chief-199-gates/`. This is fresh design-stage
evidence, not a successful implementation or engine-owned verify receipt.

Root fmt/clippy, engine digest/compile, shim integrity, coverage selftest,
publish/seal proofs, license check, fail-closed native-tls absence, eval fmt,
design selftest/check and delivery proof passed. Root cargo tests and exact
coverage failed at the decision inventory: 0076 has no index row. T1 owns that
known four-file-design gap; exact coverage produced no successful verdict.

Standalone eval clippy/tests failed compilation in unchanged
`evals/src/runner.rs` test_probes: UnixStream writes at lines 3168, 3182 and
3726 lack `std::io::Write` in scope (E0599). T3 must restore that narrow test
trait import while porting current cleanup proofs, then execute those proofs;
this is not permission to replace or weaken #225. No eval execution/pass claim
follows from the failed compilation. Final implementation requires fresh full
gates; these observations waive none. The final amended design also receives
its own structural check and link/clean-tree checks.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Old patches erase provenance/traps or change versions | Integrate current constructors/readers; preserve every newer assertion and actual replay equality. Accept fixture edits and strict historical-row rejection. |
| Cancellation or finalization releases admission early | Retained admitted task plus existing group/drain/private-state owners; causal held-cleanup, drop-waiter and finalization/disposal proofs with early-release red control. |
| Valid row masks unproved cleanup | Finite diagnostic capture feeds existing stop_admission even on valid rows; test full zero-time suffix and retained equality. |
| Unsafe/shared rendezvous splits admission | Fixed uid path, exact ownership/type/mode/link checks and descriptor/path identity; never unlink. Advisory limit remains explicit. |
| Hard death leaves descendants after lock release | Invalidate baseline and require operator-confirmed cleanup; no unlocked-file recovery claim. |
| Ordinary tests spuriously contend or deadlock | One shared outer test-scope coordinator; audit all launchers, never acquire in child helpers; default-thread full suite. Accept slower proofs. |
| Unproved cleanup retains admission indefinitely | Fail closed on surviving runtime, keep existing owner and finite diagnostics; accept manual intervention/resource retention. |
| Hostile same-uid process ignores admission | Exact limitation in all contracts; #221 owns confinement. No isolation or authenticated-evidence claim. |
| Four-file design leaves decision index incomplete | T1 adds one proposed 0076 row and reruns inventory; record design-stage red gates honestly, never waive final gates. |
