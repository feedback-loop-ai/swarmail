# Implementation plan — matrix cancellation

Implements [spec.md](spec.md) and proposed
[0062](../../docs/decisions/0062-matrix-cancellation.md). No implementation
or gate success is inferred from this plan.

## Technical context

Rust 2024 standalone `evals/` workspace. Existing Tokio process/signal/I/O,
libc, serde and tempfile support the work; no dependency, feature or lock
change. Matrix has one active trial and a sequential product iterator.
The root crate is unchanged and evals stays outside its exact coverage
report under accepted 0055. There is no plan template in this checkout;
this document supplies the required sections directly.

## Constitution check

Preserve accepted 0001–0005 and 0055, separate schema/accounting from
execution, keep real process/protocol tests and 0020's deterministic seam
rules. This proposed decision does not authorize contradiction of any
accepted ruling; compatible implementation may proceed under house rules.
Only the operator accepts 0062. Preserve the one canonical gate inventory,
100% root coverage, allowlist, pinned tooling and unsigned/no-push delivery.
No recipe edits, new root member or coverage exclusions.

## Research and resolved design questions

`evals/src/main.rs::matrix` resolves the binary synchronously, then awaits
`run_child`, whose opaque `output()` prevents parent cancellation ownership.
`runner::resolve_swarmail_bin` may spawn Cargo. `run_trial` resolves before
signals, but matrix supplies an already-resolved SWARMAIL_BIN, so its trial
startup cannot build. Handler installation precedes every trial-owned
process spawn. This grounds direct PID signalling without a production
startup channel; preserve and test that invariant.

`run_trial` aborts/joins its workload and calls `Groups::kill_all` before
returning an interruption row. `Proc::stop` waits for its leader, then only
signals the group. Agent wait handles live in a workload-owned JoinSet.
Neither path establishes descendant cessation after workload cancellation.
Retain direct-child wait ownership and group records outside the abortable
workload and join cleanup before return.

The shared helper's `pid_alive` maps any `/proc` read error to dead. Use typed
observations and recorded start identities for the new proof. The existing
eventual standalone test is not matrix evidence. Real readiness alone does
not prove cleanup ordering: add a controlled completion fence.

## Project structure and touched files

| File | Implementation responsibility |
| --- | --- |
| `evals/src/main.rs` | Matrix latch, active-child handle, cancellable bootstrap, output validation/drainage, ordered output, suffix, help; local boundary tests. |
| `evals/src/runner.rs` | Shared resolver parsing, retained reapers/groups and cleanup fence; preserve standalone API/scoring; test-only seams. |
| `evals/tests/common/mod.rs` | Identity observation, readiness/control helpers, launch markers and failure-safe guards. |
| `evals/tests/report.rs` | Real matrix signals, output/bootstrap error controls, report assertions; helper entry point. |
| `evals/tests/runner.rs` | Preserve standalone regression; prove strengthened cleanup and direct reaping separately. |
| `evals/README.md` | Signals, exit/accounting, attempts meaning, bounds and platform/output limits. |
| `docs/decisions/README.md` | First implementation commit adds proposed 0062 index row for the existing inventory gate. |

Chief artifacts are only this directory's spec/plan/tasks and proposed 0062.
No report formula or root source edit; prefer existing-file tests and no new
production module.

## Data model and contracts

Keep TrialResult/schema 1 unchanged. Internal matrix state holds sticky
interruption/first signal, product position, aggregate flags, and zero or one
owned subprocess (bootstrap or trial). That owner retains child, PID,
bounded stdout reader and wait completion. Stream the product/refusals.

Trial cleanup retains each direct child's reaper/status and group until
both wait and cessation finish. The workload gets readiness/status access;
aborting it cannot abort reapers. Register each spawn before any await.
Completed reapers retain status for scoring; leader exit alone cannot retire
a group. Status subscriptions preserve server-health and agent-completion
behavior without competing waits on a child.

States: Running → Stopping → Cleaning → Accounting → Finished. Normal
completion loops Running; Stopping never admits a new trial. Pending
cancellation overrides an uncommitted normal row. Commitment is ordered
output enqueue after validated row and child wait; backpressure may defer
it but cannot block signal/cleanup observation. Enqueued rows are immutable.
Output failure uses the same stop/cleanup path, never detaches active work.

## Implementation approach

1. Install a local sticky cancellation state before bootstrap. Retain both
   signal streams through return. Biased checks at admission, commitment
   and final return settle simultaneous readiness explicitly.
2. Split binary resolution into shared path validation/artifact parsing and
   an async matrix build owner. Cargo keeps offline/locked arguments, runs
   once in its own group and drains incrementally, retaining the artifact
   path and bounded diagnostics. Keep the synchronous standalone resolver
   compatible. `spawn_blocking` around it alone cannot cancel Cargo.
3. Replace `run_child.output()` with spawn/reader/wait ownership. Forward
   SIGTERM once, retain wait, drain with a 64-KiB retention limit, validate
   exact single-row/labels/status, normalize cancellation after wait. Use
   a five-second post-exit EOF bound. ESRCH is a completion race; other
   signal errors diagnose without dropping ownership. kill_on_drop is not
   the cancellation protocol.
4. Refactor only runner lifecycle ownership. Keep reapers/groups across body
   abort/panic. Join stopped admission, TERM groups concurrently for five
   seconds, KILL survivors, then observe cessation/reap for five seconds.
   Existing normal cleanup can supply completed waits; the final fence
   covers every path racing interruption. No per-child/repeated grace reset.
5. Linux observation reads `/proc` membership for registered groups only,
   using start identity/state; unexplained errors fail. It does not derive
   a new kill inventory by ancestry. Retain group continuity while members
   exist and retire empty registrations permanently. Other Unix uses
   conservative kill(-pgid, 0)/ESRCH, diagnosing zombie timeout. No subreaper
   or unrelated grandchild wait.
6. Trial awaits its cleanup fence before returning a row. Matrix reaps it,
   commits one active error if interrupted, then refusal suffix. Failures
   cannot become pass. Keep matrix writes asynchronous/bounded and stderr
   drainage independent so output pressure cannot stall cancellation.
   Complete rows require progressing output; no universal exit timeout or
   competing parent kill timer is invented.
7. Update help/README with exit 2, ordering, refusal semantics, denominators
   and cleanup/platform/output limits. Implement linked tests before making
   the new cleanup claim.

## Test design

Real CLI tests in `evals/tests/report.rs` run matrix with a passing prefix,
active helper and suffix. Record per-launch unique markers (never overwrite),
trial/server/fixture/agent/descendant identities, and atomic readiness after
confirming liveness. Signal matrix PID only. Drain both pipes concurrently.
Scoped guards clean only the test's owned groups/children on failure.

Use test-only seams in existing owners for startup, admission, completion,
signal-install failure and cleanup-bound cases. Compile under cfg(test);
exercise production matrix/runner paths in a re-executed unit-test harness
when the integration CLI cannot see a seam. No production environment hold
or feature flag. Child acknowledges its cleanup hold with owned work live;
matrix acknowledges waiting. Release, observe cessation/direct reaps, then
acknowledge fence completion. A negative control dropping the wait fails by
missing the required ordered acknowledgement, not a timing guess. Public
CLI signal tests remain independent evidence.

TERM-resistant descendants and leader-first exit use real processes. Use
readiness/control events and deadlines, never fixed sleep synchronization.
Linux identity observations fail on unknown state; scope those tests to
Linux explicitly rather than passing on absent `/proc`. Conservative Unix
observation gets its own errno/error tests.

## Quickstart and validation

From `evals/`: `cargo test --locked --test report`,
`cargo test --locked --test runner`, `cargo fmt --all -- --check`,
`cargo clippy --all-targets --locked -- -D warnings`, then
`cargo test --locked` including oracle/null/isolation proof.

Run the complete [house-rule gates](../../docs/house-rules.md#gates--part-of-every-change)
with locked dependencies, exact coverage/selftest, publish/seal proofs,
digest-matched compile, licenses and fail-closed native-tls absence. Keep
CI-only proofs with their designated owners. Implement and independent
reviewers owe evals evidence separately; design-check is structural only.

Chief handoff commits precisely four authorized paths unsigned, checks HEAD's
file list/clean blobs, then runs
`bash bundles/self/scripts/design-check.sh check .` locally. The engine's
pinned check remains authoritative. Leave chief commit last. Do not claim
full gates green while the excluded index row or environment prerequisite
is outstanding.

## Risks

| Risk | Mitigation and accepted cost |
| --- | --- |
| Signal during spawn loses handle | No await before registration; cancel admitted child; boundary control. |
| Handler ordering drifts | Resolved binary and startup-order regression; future early spawn requires redesign. |
| Abort drops waits | Reapers outside workload, joined cleanup fence; separate standalone and matrix proofs. |
| Descendant outlives leader | Retain group; shared TERM/KILL budget and membership observation. |
| PID reuse/observation error | Record identities, retire empty groups, unknown means failure; hostile races unsupported. |
| Kernel cannot stop child | Explicit failure and unresolved-ownership diagnostic, no cleanup-success claim. |
| Repeated signals | Sticky handlers, no reset or parent forced kill. |
| Output pressure | Bounded concurrent drainage/output, independent cleanup, broken-output tests. |
| Large refusal suffix | Stream O(product) rows; document requested-slot attempts/output prerequisites. |
| Unix proof overstated | Linux-specific identity/zombie proof; conservative absence/failure elsewhere. |
| ADR index excluded | First implementation task adds proposed row and proves inventory; no full-gate success meanwhile. |
| Framework scope creep | Two existing production files, one trial, no dependency/schema/server changes. |

## Chief validation receipt — 2026-10-24

Design-only checks: relative links resolve; root fmt/clippy, digest-matched
`.forge/bin/brokkr compile --bundle bundles/self`, coverage selftest,
publish-guard proof, keyless seal proof, license check, shim integrity and
fail-closed native-tls absence passed. The binary digest matches tooling.json
(`e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`).

`cargo test --workspace --locked` and instrumented coverage both refused
`store_contract_docs`: the decision inventory and its refusal selftest see
`0062-matrix-cancellation.md: no index row`. T1 supplies that excluded file
change; these gates must be rerun on implementation. Run-local logs are in
`.forge/design-194-checks/`; no full-gate or implementation success is claimed.
The unchanged eval suite was not run by this design seat; T9 remains due.
