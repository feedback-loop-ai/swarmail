# Webhook CA test hang (issue #331)

Created: 2026-11-07. Status: draft. Issue #331.
Run: `issue-331-intermittent-ci-hang-i-60ce6b3b`. Base: `c95386e`.
Recipe: `claude-careful`. Design panel: simplicity and robustness positions
(run-local, `.brokkr/state/design/positions/`), synthesized here.

No decision record. This change sets no new rule. It applies accepted
[0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md)
§5 (bounded waits, no sleep as synchronization) and works inside accepted
[0078](../../docs/decisions/0078-ca-executor-ownership.md) §1 (nonblocking
admission: "Contention ... refuses", no pending queue). The reserved number
0127 is left unused.

## The feature

Two CI jobs hung until their timeout on 2026-11-04:

- `test (macos-latest)` on #328, with three `webhook::ca_construction_tests`
  tests reported as running for more than 60 s;
- `coverage` on #327, which printed nothing for 60 minutes.

This change removes the cause of the hang. It also bounds every wait in the
three named tests, so that a future regression fails within seconds and names
what it waited for, instead of using up the job timeout.

## The cause (ruled)

Both panel members read the code on `c95386e`. The simplicity member confirmed
the mechanism with a reverted scratch test. One test is the root:
`ca_workers_survive_runtime_replacement`
(`src/webhook/ca_construction_tests.rs:1993`).

1. The test spawns **two** `redacted_targets` jobs on the same
   `Webhooks::scope` before it waits for either one to enter its build. It
   does this in phase 1 (`first`/`second`) and again in phase 2 (the
   `for _ in 0..2` loop).
2. Admission is nonblocking by accepted contract. `Scope::admit` uses
   `try_lock` (`src/webhook/ca_executor.rs:16`), and so does the executor
   state (`:262`). The worker that runs the first job takes the same scope
   mutex in `execute_envelope` (`scope.live()`, `:209`). If the second job's
   `admit()` lands inside that window, the job is refused with `Unavailable`.
   That is correct production behaviour under 0078 §1.
3. A refused job never reaches `build_ca_client`, so its `CaBuildPause` stays
   queued in the global `CA_BUILD_PAUSE`. The `entered` sender stays alive
   inside a `static`, so the receiver never resolves and never closes. The test
   then waits forever on its bare `entry_b.await` (or `entry.await`).
4. The test holds `CACHE_TESTS` (`src/webhook.rs:1003`) for its whole body.
   That lock is a fair FIFO tokio mutex, and every other CA test waits on it
   with no deadline. `cancelled_delivery_build_counts_work_without_late_publication`
   and `cancelled_get_retains_charge_without_blocking_smtp` were therefore
   **victims**: they were queued on the lock, not hung themselves. This matches
   the report exactly: three tests running for more than 60 s, then silence.

All the other executor users in the lib test binary already admit one job at a
time: they wait for each `entered` before they spawn the next job. They all
hold `CACHE_TESTS`, and no test outside it reaches the global executor. The
`entered` send happens inside `job.run`, strictly after the worker's
`scope.live()` temporary has been dropped. So once the test waits for entry
before admitting the next job, the collision cannot happen.

The coverage hang cannot be attributed from its log. It is consistent with the
same test: the gate runs the same lib binary under instrumentation, which
widens the window.

## Requirements

- **R1. Fix the cause.** In both phases, `ca_workers_survive_runtime_replacement`
  admits sequentially: it spawns job N, waits until that job's build has
  entered, and only then spawns job N+1. Its claim stays the same: both workers
  are held and then abandoned, the runtime is replaced, and the same two worker
  threads are reused.
- **R2. A refusal is a named failure.** Every wait for a CA build's `entered`
  in the three named tests races the event against the job's own completion and
  against a deadline. If the job finishes first, the test fails at once with a
  message that names the stage, says the admission was refused before the build
  entered, and gives the job outcome and `ca_executor::jobs_for_test()`. If the
  deadline fires first, the message names the stage and the deadline.
- **R3. Every wait in the three named tests is bounded and named.** This covers:
  - `ca_workers_survive_runtime_replacement`;
  - `cancelled_delivery_build_counts_work_without_late_publication`;
  - `cancelled_get_retains_charge_without_blocking_smtp`, including the shared
    `projection_capacity_case` it runs, which also bounds its sibling
    `all_ca_workers_held_smtp_persists_before_release`.

  Each wait gets one of three tiers:
  - **seam handshakes** (oneshots, `JoinHandle`s, `server.stop()`, the SMTP
    `QUIT` reply): their own 5 s deadline, with a message that names what they
    waited for;
  - **each `block_on` phase body** (ordinary loopback HTTP and SMTP steps): one
    30 s envelope that names the test and the phase;
  - **runtime teardown**: a bounded, named teardown (R5).
- **R4. The serial lock is bounded and named.** The three named tests take
  `CACHE_TESTS` through one helper. If the lock is not acquired within 120 s,
  the helper fails with a message that names the lock and says a predecessor CA
  test is stuck. The sync test stops using `blocking_lock()`.
- **R5. Teardown is a bounded wait.** Where the three tests drop a runtime, they
  do it through a helper. The helper drops the runtime on a separate thread and
  waits for completion for at most 5 s. If it does not finish, it fails with a
  message that names the test's runtime and the teardown. `Drop` is the one
  wait that no `tokio::time::timeout` can reach.
- **R6. Contracts do not change.** There are no production code changes:
  - `src/webhook.rs` and `src/webhook/ca_executor.rs`, including their inline
    `#[cfg(test)]` seams, are unchanged;
  - there is no new seam, no new dependency, and no change to `scripts/`,
    `.brokkr/`, `.github/`, `deny.toml` or either `Cargo.lock`;
  - 0001–0005 and 0078 are untouched.

## Acceptance Criteria

- **AC1 (cause).** In `ca_workers_survive_runtime_replacement`, no
  `redacted_targets` job is spawned while an earlier job of the same phase has
  not yet signalled `entered`. This holds in phase 1 and in phase 2. The test
  still asserts that the two held threads differ, that both phase-2 threads are
  among the phase-1 pair, and that the post-stop GET returns 503.
- **AC2 (regression, deterministic).** A new harness test,
  `refused_admission_fails_the_entry_wait_by_name`, forces the 0078 §1
  contention refusal at the pole. A helper thread holds the target scope's
  `live()` guard while a `redacted_targets` job is admitted. The test asserts:
  - the entry-wait helper returns its refusal error, which contains
    `admission refused before its CA build entered`, well inside the 5 s
    deadline;
  - the job result is `Err(Unavailable)`;
  - `jobs_for_test() == 0`.

  After the guard is released, the test repeats the wait on a fresh PEM and
  target and asserts the success arm:
  - the helper returns the worker `ThreadId`;
  - `jobs_for_test() == 1` while the build is held;
  - after release, `finished` resolves, the job is `Ok`, and
    `jobs_for_test()` returns to 0.

  The test uses no sleep.
- **AC3 (red on old).** A negative control is recorded in the implement
  receipt: the AC2 schedule with c95386e's wait shape (a bare `entered.await`
  in place of the helper). Run as
  `timeout 120 cargo test --locked --lib refused_admission_fails_the_entry_wait_by_name`,
  it exits 124 (hang). The committed shape passes in under 5 s. Afterwards the
  tree is restored to the committed shape.
- **AC4 (bounded waits).** The bodies of the three named tests and
  `projection_capacity_case` contain no bare `.await` on a oneshot receiver, a
  `JoinHandle`, `CACHE_TESTS`, `server.stop()` or the SMTP `QUIT` reply. Each of
  these goes through a named deadline helper. Each `block_on` phase body is
  enclosed in a named 30 s envelope.
- **AC5 (serial lock).** The three named tests acquire `CACHE_TESTS` only
  through the bounded helper (120 s, a named message). A harness test holds the
  lock and calls the helper with a short injected deadline. It asserts that the
  panic message names `CACHE_TESTS` and the stuck predecessor.
- **AC6 (teardown).** The three named tests end every runtime through the
  bounded teardown helper. A harness test parks a `spawn_blocking` task on a
  channel, calls the helper with a short injected deadline, and asserts the
  named teardown failure. It then releases the task, so the leaked teardown
  thread finishes before the test returns.
- **AC7 (no production change).** The following diffs are empty:
  - `git diff c95386e -- src/webhook.rs src/webhook/ca_executor.rs Cargo.toml Cargo.lock evals/Cargo.lock deny.toml scripts .brokkr .github`.

  Only `src/webhook/ca_construction_tests.rs` changes under `src/`.
- **AC8 (gates).** Every house gate in
  [house-rules § Gates](../../docs/house-rules.md#gates--part-of-every-change) is
  clean, `--locked`. This includes the evals crate against the candidate binary,
  `coverage-gate.sh` (100 % exact) and its selftest, the publish-guard proof, the
  seal proof and `cargo deny check licenses`. If the sandbox has no `brokkr`
  binary, the bundle compile is run in verify and reported as such.
- **AC9 (no masking).** The change adds no retry of an admission, raises no
  existing deadline (the 5 s `wait_ca_idle` and the 20 s `BuildWatchdog` keep
  their values), and leaves out `--test-threads`.

## Out of scope (ruled; see plan.md "Panel rulings")

- Changing `try_lock` to `lock` in `Scope::admit` or `Executor::reserve`. This
  contradicts accepted 0078 §1 and would need an operator amendment to that
  decision.
- An RAII pause guard that replaces `BuildWatchdog`. It touches measured inline
  seams in `src/webhook.rs` and is not the cause.
- Making the coverage gate stream slow-test names. The gate is the pinned
  bundle's script (0050 §1) in the operator-hold class. This is a follow-up for
  the operator.
- Per-test executor capacity. It would change 0078 §3 process ownership.
- A file-wide sweep of every other `CACHE_TESTS` user.
