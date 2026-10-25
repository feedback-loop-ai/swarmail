# Plan — retain failed-cleanup drains

Implements [spec.md](spec.md); ordered work and proof are in [tasks.md](tasks.md).

## Technical context

Rust 2024, standalone unpublished `evals/` workspace with its own lockfile;
Tokio JoinHandle/watch ownership, existing Unix child/group observation and
advisory admission. No dependency, Cargo feature or schema change. No plan
template exists in this checkout; the required template sections are supplied
here, without extra research/data-model/contract files outside the chief's scope.

## Constitution check

Preserve 0001–0005 and accepted 0055's current lifetime contract. Keep 0076 and
all other decision files/statuses unchanged. Follow 0020 test placement: inline
is the default; a concrete cold failure path justifies a cfg(test) sibling harness,
not private seam access alone. Reuse the existing runner/probe family. Every seam
is test-only, exercised and scoped to its intended process/group/task. Do not
introduce global uncoordinated state or sleep-based synchronization. Root coverage
remains literal zero missed lines, branches and functions. Evals is outside that
denominator and owes its full separate suite. No route or gate changes.

## Research and resolved choices

Source confirms a destructive ownership transfer at `cleanup_once` and an empty
failure-owner vector in `ensure_cleanup`; specialized stdout consumers lack the
ordinary drain's lifetime capture. Process cessation is insufficient proof of
async task completion. The review's failed-observation test waits for probe exit,
so its current observation is too late to establish surviving-runtime retention.
Both positions support a local transfer fix and causal controls. No external
input, network access, new isolation mechanism or new decision is needed.

## Project structure and files

| Path | Authorized purpose |
| --- | --- |
| `evals/src/runner.rs` | Restore pending drains; scoped test seams, regression cases and existing cleanup-probe handshake changes. |
| Existing `evals/tests/common/` support, only if needed by those regressions | Reuse parent trial coordination and identity-aware teardown; avoid broad harness refactoring. |
| `specs/retain-cleanup-drains/{spec,plan,tasks}.md` | Three-file chief design; later record truthful progress/red-green evidence in these owning artifacts. |

Expected production diff is solely runner.rs. If a cold-path test needs a new
sibling harness, declare it under cfg(test) beside the runner per 0020 and record
why; no new production abstraction follows. `evals/src/main.rs` is a verification
target only. Existing decisions, broader documentation, admission, root source,
manifests, locks, deny policy and bundles need no edits.

## Implementation approach

1. In `cleanup_once`, take the drain collection under the existing short lock and
   consume it with an explicit iterator. Await each current handle by mutable
   reference with the original `final_deadline`. Successful joins are consumed
   once; preserve the existing treatment of a completed JoinError.
2. On timeout, restore the current handle followed by the remaining iterator to
   `self.drains` under a short lock, before returning the unchanged diagnostic.
   There are no new drains registered after spawning returns. Do not await under
   the lock or discard the suffix. Earlier failure exits already leave the vector
   in the group; signal-error exits after completed drains need no restoration.
3. Keep `ensure_cleanup` as the sole owner. It publishes/caches the result, observes
   group cessation and successful reaping on failure, then awaits retained handles
   without another cleanup deadline. Its Arc retains runtime/admission until the
   last task finishes. Do not create another supervisor or renew the verdict.

## Data model and contracts

No persisted or public data changes. The internal collection owns all outstanding
JoinHandles. During bounded joins the local iterator owns them; on failure ownership
returns to OwnedGroup; the existing failure task takes and joins them. Only
completed handles leave this chain. Existing watch receivers can observe cached
results without owning the group, which is useful for honest lifetime tests.

The public finite cleanup category, harness-error row, matrix stop/full suffix,
concurrency 1, output limits and private-state disposal ordering stay unchanged.
A completed retained task allows resource release, not revision of the cached
error. Unlimited retention of unresolved work is the existing fail-closed cost.

## Proof design

Add proposed exact test names `cleanup_drain_timeout_retains_helper_stdout`,
`cleanup_drain_timeout_retains_cargo_stdout` and
`cleanup_drain_timeout_retains_unjoined_suffix` in the existing runner test family.
If implementation must rename them, update this plan/tasks with actual filters.
Use a real short-lived child and pipe in the helper/Cargo spawn paths; a test-only
hold inside the actual stdout task acknowledges pending work near completion.
No hostile child, escaped descendant, namespace, external Cargo download or
unrelated synthetic future substitutes for those two paths.

Acknowledge child reaping, empty group and ordinary stderr completion. Keep the
stdout task pending until the actual ten-second absolute cleanup budget expires;
no production timeout knob or Tokio feature expansion. Acknowledge the failed
verdict and owner progress so a scheduling delay cannot impersonate retention.
Use paths, weak observations, watch receivers and release channels that do not
own admission/runtime/group. Drop all legitimate caller owners before assertions.
Releasing a hold is not task completion: acknowledge completion and observe owner
disposal before proving release. Keep a non-owning cached-result receiver to check
that later success of the drains leaves the original error unchanged.

For the suffix case hold two tasks without independent admission/runtime captures;
release the first and verify continued refusal/presence while the second is still
pending, then verify release. This targeted seam supplements the real stdout cases.
Use bounded observation deadlines; explicit acknowledgments establish ordering.

For observation failure and forced expiry, extend the existing out-of-process
probe's actual-return acknowledgment so its Tokio runtime survives the failed
verdict. Assert refusal/private-state retention before probe exit; safely identify,
terminate and observe/reap the surviving owned group, clear the injected observation
fault as required to observe cessation, and prove eventual release. Retain checked
teardown and Pending controls. Parent tests acquire existing trial_scope once;
contender/probe children must not recursively acquire that coordination lock.

Before the repair, run the helper/Cargo/suffix regressions against d973b0e production
logic with only test support added. Require the intended admission/lifetime failure,
not setup timeout or missing filter. After repair all pass. Separately remove the
failed-cleanup retention temporarily for the two fault modes and require refusal
assertions to fail. Restore every temporary mutation before full verification.
Record exact commands, matched cases, assertion failures and green observations in
the task evidence. Do not rewrite history or inspect another worktree for controls.

## Quickstart and verification

Work offline with provisioned tools and the current bundle routes. Verify the
engine SHA-256 is `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`
and `sha256sum --check --strict bundles/self/scripts/shims.sha256` passes.
The house inventory remains authoritative at
[house-rules.md](../../docs/house-rules.md#gates--part-of-every-change).
Run its entire local inventory, using `.forge/bin/brokkr compile --bundle bundles/self`.
Hermetic verify executes checked pinned copies of coverage-gate.sh (normal and
selftest), publish-guard-test.sh and seal-branch-test.sh in bundles/self/scripts.
Also run pinned design-check selftest, delivery-proof-test and the intake's
fail-closed native-tls absence proof: success or an unexpected cargo-tree error
refuses; only the explicit package-not-found diagnostic proves absence.

Standalone commands (default test threading):

```bash
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --workspace --locked
cargo test --manifest-path evals/Cargo.toml --locked cleanup_drain_timeout_retains_
cargo test --manifest-path evals/Cargo.toml --locked cleanup_observation_error_and_forced_expiry_fail_with_live_owned_processes
cargo test --manifest-path evals/Cargo.toml --locked runner_cleanup_await_pending_and_bypass_controls_cover_signals_and_faults
cargo test --manifest-path evals/Cargo.toml --locked cleanup_handshake_
cargo test --manifest-path evals/Cargo.toml --locked sequential_admission_library_and_cleanup_lifetime
cargo test --manifest-path evals/Cargo.toml --locked unproved_worker_cleanup_stops_matrix_with_full_refusal_suffix
```

The full eval command includes proof, runner, report, trap, privacy and provenance
suites; root green does not substitute for it. Commit restored implementation
before final gates and bind receipts to HEAD/tree. No unmatched test filter or
prior candidate receipt qualifies. Structural design-check runs against this
chief commit; later feature verification is a separate obligation.

## Risks

| Risk | Mitigation and accepted cost |
| --- | --- |
| Only the current handle is restored | Explicit iterator suffix transfer and a two-pending-task release-order control. |
| False green from a caller or ordinary drain owning lifetimes | Prove stderr completion, drop strong owners, keep only non-owning observations, require baseline red at admission/lifetime. |
| Failure is published before background owner progresses | Acknowledge the relevant owner transition/completion; do not infer retention from one immediate poll. |
| Test runtime exits too early | Hold the probe alive after failure until refusal and cleanup/release observations finish. |
| Indefinite drain blocks future cooperating trials | Accept existing fail-closed retention; report error promptly, add no forced-release timeout or retry. |
| Contention/flaky interleaving or unsafe teardown | Existing parent trial_scope, task-scoped holds, real deadlines and identity-aware cleanup; no sleep as synchronization. |
| Regression support becomes a general framework | Reuse existing family; justify only seams needed for the three ownership cases and two failure modes. |
| Unrelated known review issue fails a required gate | Record command and exact failure, surface scope conflict, keep gate and scope intact. |
| Root green hides eval failure | Run standalone full locked suite and matched focused tests separately. |
| External proof unavailable | CI/operator ownership stays explicit; no new macOS, network, namespace or live-setting acceptance condition. |
