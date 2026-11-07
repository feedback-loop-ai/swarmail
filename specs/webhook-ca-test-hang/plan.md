# Plan: webhook CA test hang (issue #331)

Spec: [spec.md](spec.md). Base `c95386e`. Harness-only change.

## Technical context

- Rust crate, tokio, with `macros` already enabled (`select!` is available).
- No new dependency.
- The only touched file is `src/webhook/ca_construction_tests.rs`. It is a
  decision 0020 rule-2 harness: it is declared `#[cfg(test)] mod
  ca_construction_tests;` in `src/webhook.rs`, and its harness name keeps it out
  of the coverage report. New helpers therefore add nothing to the 100 %
  denominator. The production lines they execute are already covered.
- The seams used are existing ones only:
  - `install_ca_build_pause`;
  - `ca_executor::jobs_for_test`;
  - `Scope::live` (a `pub(crate)` method), reached through the `Webhooks::scope`
    field;
  - `CACHE_TESTS`.

## Constitution check

| Rule | Status |
|---|---|
| 0001 accept == stored, 0002, 0003, 0005 | untouched. No production code changes, and no wait comes near the store commit |
| 0078 §1 nonblocking admission ("Contention ... refuses") | preserved. The fix stops the *test* from relying on a refused admission. The refusal arm itself is asserted (AC2) |
| 0078 §3 process-owned workers survive runtime replacement | preserved and still proved (AC1) |
| 0020 §2 harness placement / §4 seams / §5 disciplines | applied: chaos at the poles (the guard is held or it is not), no sleep as synchronization, bounded waits, exact counts |
| Dependencies / `deny.toml` | unchanged |
| Operator-hold paths (`scripts/`, `.brokkr/`, `.github/`, lockfiles) | untouched, so the PR is not in the hold class |
| New rule → decision record | none set; 0127 left unused |

## Panel rulings

The two positions agree on the diagnosis and on the bounded lock. On the
points where they conflict, these are the rulings.

1. **Cause.** Adopted the simplicity diagnosis (contention refusal leaves
   `entered` pending forever; the other two tests were lock victims), because it
   was confirmed on `c95386e` and matches every reported symptom. Robustness
   item 2 (resolve the wait on refusal as well as on entry) is **adopted** as
   R2/AC2. It is the same `select!` both members proposed.
2. **Robustness item 5: `try_lock` → `lock` in `Scope::admit` /
   `Executor::reserve`.** **Rejected.** Accepted 0078 §1 states "Contention ...
   refuses". The task forbids changing an accepted contract without stopping, and
   the sequential fix removes the only collision the tests cause.
   - What robustness loses: in production, a microsecond collision still returns
     a 503 to a client that had a free worker. That is the documented 0078
     behaviour.
   - If the operator wants that changed, it needs a 0078 amendment, as its own
     story.
3. **Robustness's "foreign test holds a global slot" cause.** Checked and
   **rejected as a live cause.** Every lib-binary user of the global executor
   holds `CACHE_TESTS`: the `ca_construction_tests` tests, the
   `client_for(Some(..))` tests in `webhook.rs`, and `executor_tests`, which
   builds private `Executor`s. `api.rs` tests do not reach `/api/v1/webhooks`.
   The wrapper tests `all_ca_workers_held_*` and `cancelled_get_*` take the lock
   inside `projection_capacity_case`. The cost of being wrong is not a hang:
   R2's helper names the refusal at once.
4. **Robustness item 1: bound teardown.** **Adopted in a changed form (R5).**
   `Runtime::shutdown_timeout` returns no verdict, and a silent leak is not a
   bounded wait. Instead, a `teardown(what, runtime)` helper drops the runtime
   on a spawned std thread and waits on a channel with `recv_timeout`. That
   gives a named failure. Robustness's R0 is kept as AC6, but it tests the
   helper's verdict, not tokio.
   - What simplicity loses: about 15 harness lines, for a wait that 0078 §3
     makes very unlikely today. They are kept because the issue asks for every
     wait to be bounded, and `Drop` is the one wait no timeout can reach.
5. **Wait granularity.** Adopted the simplicity tiers over robustness's "wrap
   every await":
   - named 5 s deadlines on seam handshakes;
   - a 30 s named envelope per `block_on` phase;
   - the 120 s lock.

   Wrapping every `reqwest` step separately buys no better attribution than the
   phase name. The deadlines nest so that the most specific one fires first:
   5 s < 30 s < 120 s.
6. **Serial lock deadline: 120 s (simplicity) over 60 s (robustness).**
   `CACHE_TESTS` is fair FIFO, so a queued waiter legitimately waits for the sum
   of every predecessor's run. A 60 s bound risks false failures under coverage
   on macOS. The value only attributes a stuck predecessor; it is not a
   correctness bound.
7. **Robustness item 4: RAII pause guard replacing the 20 s `BuildWatchdog`.**
   **Rejected.** The pause seam is inline `#[cfg(test)]` code in
   `src/webhook.rs`. Under 0020 §1 that code is measured, so a new owner-gone
   branch must run on every coverage run, and it is not the cause.
   `BuildWatchdog` already releases on `Drop` (on unwind as well), and
   `assert_unused` makes a watchdog release a failure, not a hidden success.
   What robustness loses: the wall-clock release thread stays as a backstop.
8. **Robustness item 6 / simplicity C6: coverage-gate slow-test reporter.**
   **Deferred, recorded as an operator follow-up.** Both members agree. The gate
   is the pinned bundle's script (0050 §1, checksum-pinned shims), so changing it
   moves the PR into the operator-hold class. Making slow-test reporting a gate
   contract would be a new rule, and so a decision. With the tests bounded, a
   recurrence unwinds with a named panic, and the gate already prints the run
   log's tail on failure (`coverage-gate.sh` `tail -n 40 "$run_dir/run.log"`).
   What the issue loses: a hang caused by some *other*, still-unbounded test
   under coverage stays unnamed until that test is bounded too.
9. **The sweep of every other `CACHE_TESTS` user.** **Cut** (simplicity). The
   others already admit sequentially and are not affected by this cause. The
   helpers will exist for the next bounded conversion.

## Project structure

Only `src/webhook/ca_construction_tests.rs` changes. Its new harness helpers sit
near the top of the file, beside `BuildWatchdog`:

- `const SEAM: Duration = 5 s`, `PHASE: Duration = 30 s`,
  `SERIAL: Duration = 120 s`, `TEARDOWN: Duration = 5 s`.
- `async fn within<T>(what: &str, limit: Duration, fut) -> T`. It panics with
  `timed out after {limit:?} waiting for {what}`.
- `async fn entered_or_refused(what, entered: &mut Receiver<ThreadId>, job: &mut JoinHandle<Result<Charged, TargetPreparationError>>) -> Result<ThreadId, String>`.
  It runs `tokio::select!` over three arms:
  - **entered:** `Ok(id)`. A closed sender also gives an `Err` naming it.
  - **job finished:** `Err("{what}: admission refused before its CA build entered ({outcome}; {jobs_for_test()} CA jobs busy)")`.
    Format only the variant name, never the payload; 0078 §5 says never format
    payloads.
  - **`tokio::time::sleep(SEAM)`:** `Err("{what}: CA build not entered within 5s")`.
    The sleep is a deadline arm, not synchronization.
- `async fn serial_within(limit) -> tokio::sync::MutexGuard<'static, ()>` and
  `serial()` (120 s). The message is: `CACHE_TESTS serial lock not acquired
  within {limit:?}: a predecessor CA test is stuck; its own named deadline should
  have fired first`.
- `fn try_teardown(what, runtime, limit) -> Result<(), (String, mpsc::Receiver<()>)>`.
  It does `std::thread::spawn(move || { drop(runtime); tx.send(()) })` and then
  `rx.recv_timeout(limit)`. On expiry it returns the message `{what}: runtime
  teardown did not finish within {limit:?}; a blocking task is still held`
  together with the still-pending completion receiver.
- `fn teardown(what, runtime)` calls `try_teardown(.., TEARDOWN)` and panics
  with that message. The tests use `teardown`; only the AC6 probe uses
  `try_teardown`, so it can wait for completion after releasing its task.

## Implementation approach

1. Add the helpers.
2. `ca_workers_survive_runtime_replacement` (answers the cause, R1–R5):
   - Build the first runtime **before** installing pauses. Acquire the lock with
     `runtime.block_on(serial())`. A tokio guard on a `static` is runtime
     independent, so it outlives both runtimes.
   - Phase 1: spawn `first` and wait `entered_or_refused("phase 1 job A", ..)`.
     Only then spawn `second` and wait for its entry. Install each pause
     immediately before spawning its job.
   - Abort both jobs, then do `within("phase 1 cancelled job A", SEAM, first)`.
   - Bound `server.stop()` by its name.
   - Wrap the whole body in `within("ca_workers_survive_runtime_replacement phase 1", PHASE, ..)`.
   - Replace `drop(runtime)` with `teardown(..)`.
   - Phase 2: the same sequential admission in the loop (spawn, then wait for
     entry, then the next spawn). Bound `done_*`, `finished`, each job join and
     `server.stop()` with SEAM. Bound the body with PHASE, then call `teardown`.
3. `cancelled_delivery_build_counts_work_without_late_publication`:
   - Bind the runtime.
   - Replace `CACHE_TESTS.lock().await` with `serial().await`.
   - Wrap `entered` with `entered_or_refused` (the job is `waiter`).
   - Bound `waiter.await` after the abort.
   - Add a PHASE envelope and `teardown`.
4. `projection_capacity_case`:
   - Use `serial()`.
   - Wrap `entered` with `entered_or_refused` (job: the `get` handle; its
     outcome is the HTTP status).
   - Bound with SEAM: `dropped`, `get.await` (both arms), `queued`, `finished`,
     the `QUIT` reply and `server.stop()`.
   - Add a PHASE envelope and `teardown`.

   Keep the existing 5 s and 3 s timeouts.

   For a `get` job the "job finished" arm reports the `JoinHandle` outcome. The
   helper is generic over the job's output with a `fn(&T) -> String` describer,
   so that GET (HTTP status) and `redacted_targets` (variant name) share it.
5. Regression tests (all in the same harness file, each under `serial()`):
   - `refused_admission_fails_the_entry_wait_by_name` (AC2).
     - A std thread clones the target `Scope`, takes `live()`, sends "held" and
       waits for "release".
     - The test waits for "held" with `within`, so no std guard is held across
       an `.await` and `clippy::await_holding_lock` stays quiet.
     - It spawns `redacted_targets` and asserts the refusal arm, the
       `Unavailable` result and the busy count of 0.
     - It releases the thread and joins it with a bound.
     - The success arm uses a **fresh** `ca_pem()` and target, because the
       refused job's pause stays queued under the old PEM.
   - `serial_lock_wait_names_the_stuck_predecessor` (AC5). Hold `serial()`,
     spawn `serial_within(200 ms)` and assert its panic text through the
     `JoinError` payload.
     - The deadline expiring is the event under test, at the pole: the lock is
       certainly held.
     - The test holds the real `CACHE_TESTS` itself, so the probe waits on a
       holder that cannot finish. It needs no second mutex and no second test.
   - `runtime_teardown_wait_names_the_held_blocking_task` (AC6).
     - Park `spawn_blocking(move || rx.recv())`, wait for a started signal, and
       call `try_teardown("probe", rt, 200 ms)`.
     - Assert the `Err` message, then send the release so the teardown thread
       ends, and wait `recv_timeout(SEAM)` on the returned completion receiver.
       That wait is bounded, and it expects the teardown to finish.
6. Negative control (AC3): temporarily swap the helper call in the AC2 test for
   a bare `entered.await` and run the named timed command. Record the exit code
   124 and the wall time in the implement receipt, then
   `git checkout -- src/webhook/ca_construction_tests.rs`. Do not commit it.

## Research / data model / contracts / quickstart

- Research: the cause section of the spec. The scratch confirmation is in the
  simplicity position (run-local evidence).
- Data model: none.
- Contracts: none changed (R6).
- Quickstart:
  `cargo test --locked --lib webhook::ca_construction_tests`, then the full
  gate inventory.
- Load proof: none is needed, because the forced contention is deterministic.
  If the implement seat chooses to show stability anyway, bound it as follows
  and report the load:
  `taskset -c 0-$(( $(nproc)/4 - 1 )) timeout 600 cargo test --locked --lib webhook::ca_construction_tests -- --test-threads=$(( $(nproc)/4 ))`
  (at most nproc/4 workers, at most 10 min).

## Risks

| Risk | Mitigation |
|---|---|
| The fix masks the race rather than removing it | The cause is the test admitting while its own worker holds the scope. Sequential admission removes that window, because `entered` fires after `live()`'s temporary has dropped. AC2 asserts that the refusal arm is real and named. No retry is added (AC9). |
| The 120 s lock bound fails a legitimately slow queue | The tiers nest (5 s < 30 s < 120 s), so a holder fails on its own named deadline first. FIFO fairness bounds the queue to the tests ahead of a waiter. The value only attributes. |
| A 30 s phase envelope is too tight under macOS coverage | Normal phases take a few seconds. A firing envelope names the test and phase, and reviewers can raise only that constant, with evidence. The 5 s seam deadlines match the existing file convention. |
| The red-on-old proof is a negative control, not a test that compiles on `c95386e` | Unavoidable when the defect is the test's own wait shape. The control is a named, timed command with an exit code in the receipt (AC3), the way 0078's negative controls are recorded. |
| `teardown` leaks a thread when it fails | Only on the failure path, in a test process. The AC6 probe releases its task and joins the thread. In production nothing changes. |
| `select!` on `&mut JoinHandle` consumes the result, so the test cannot also `.await` the job later | The helper returns the outcome inside the `Err`. On the success arm the handle is untouched, and the caller later aborts it or joins it with a bound. |
| A refused job's stale pause stays queued under its PEM | PEMs are fresh per test (`ca_pem()`), and the AC2 success arm uses a new PEM, so a stale entry can never match. |
| A sibling test still has an unbounded wait | Accepted residual (ruling 9). The helpers make the next conversion mechanical. The coverage-gate reporter is the operator follow-up (ruling 8). |
| A gate breaks on clippy (`await_holding_lock`, `type_complexity`) | The guard is held on a std thread, never across `.await`. Add a type alias for the job handle if clippy asks. |
