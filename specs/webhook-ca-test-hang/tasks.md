# Tasks: webhook CA test hang (issue #331)

Spec: [spec.md](spec.md). Plan: [plan.md](plan.md).
Every task touches only `src/webhook/ca_construction_tests.rs`, except T9
(gates) and T8 (a temporary negative control, never committed).

- [ ] **T1. Add the bounded-wait helpers.**
  - Add the constants `SEAM` (5 s), `PHASE` (30 s), `SERIAL` (120 s) and
    `TEARDOWN` (5 s).
  - Add `within`, `entered_or_refused`, `serial_within`/`serial`, and
    `try_teardown`/`teardown`, with the messages given in plan.md.

  *Proof:* T2, T3 and T4 exercise every helper arm. `cargo clippy --all-targets
  --locked -- -D warnings` stays clean.
- [ ] **T2. Regression: a refused admission fails the entry wait by name**
  (`refused_admission_fails_the_entry_wait_by_name`).
  - A std thread holds the target scope's `live()` guard while
    `redacted_targets` is admitted.
  - Assert the following: the helper returns `Err` containing `admission
    refused before its CA build entered` within the deadline; the job result
    is `Err(Unavailable)`; `jobs_for_test() == 0`.
  - Release the guard. On a fresh PEM and target, assert the success arm:
    `Ok(ThreadId)`; `jobs_for_test() == 1` while held; `finished` resolves; the
    job is `Ok`; the busy count returns to 0.

  *Proof:* the test itself (AC2) and the negative control in T8 (AC3).
- [ ] **T3. Bounded serial lock** (`serial_lock_wait_names_the_stuck_predecessor`).
  - Hold `serial()` and call `serial_within(200 ms)` in a spawned task.
  - Assert that the panic payload names `CACHE_TESTS` and the stuck
    predecessor.

  *Proof:* the test itself (AC5).
- [ ] **T4. Bounded teardown** (`runtime_teardown_wait_names_the_held_blocking_task`).
  - Park a `spawn_blocking` task on a channel, then call
    `try_teardown("probe", rt, 200 ms)`.
  - Assert the `Err` message.
  - Release the task, then wait `recv_timeout(SEAM)` for the teardown to
    complete.

  *Proof:* the test itself (AC6).
- [ ] **T5. Fix the cause in `ca_workers_survive_runtime_replacement`.**
  - Admit sequentially in phase 1 and phase 2: spawn, then wait
    `entered_or_refused`, then the next spawn. Install each pause just before
    its job.
  - Acquire the lock through `runtime.block_on(serial())`, with the runtime
    built first.
  - Bound with SEAM: every oneshot, every job join and each `server.stop()`.
  - Wrap each phase in a named PHASE envelope, and replace `drop(runtime)` with
    `teardown`.
  - Keep every existing assertion: distinct threads, phase-2 threads among the
    phase-1 pair, the post-stop 503, `scope.live().is_none()`, and both
    watchdogs unused.

  *Proof:* the test passes. AC1 and AC4 are checked by reading the diff, and
  `cargo test --locked --lib webhook::ca_construction_tests::ca_workers_survive_runtime_replacement`
  is green.
- [ ] **T6. Bound `cancelled_delivery_build_counts_work_without_late_publication`.**
  - Bind the runtime and take the lock through `serial()`.
  - Wait for entry through `entered_or_refused` (the job is `waiter`), and bound
    `waiter.await` after the abort.
  - Add a PHASE envelope and `teardown`. Keep the existing 5 s timeouts.

  *Proof:* the test passes (AC4).
- [ ] **T7. Bound `projection_capacity_case`**, which covers
  `cancelled_get_retains_charge_without_blocking_smtp` and
  `all_ca_workers_held_smtp_persists_before_release`.
  - Take the lock through `serial()`, and wait for entry through
    `entered_or_refused` (the job is the GET handle, described by HTTP status).
  - Bound with SEAM: `dropped`, both `get.await`s, `queued`, `finished`, the
    `QUIT` reply and `server.stop()`.
  - Add a PHASE envelope and `teardown`. Keep the existing 5 s and 3 s timeouts.

  *Proof:* both wrapper tests pass (AC4).
- [ ] **T8. Negative control, red on the old wait shape** (AC3).
  1. Temporarily replace the T2 helper call with c95386e's bare
     `entered.await`.
  2. Run
     `timeout 120 cargo test --locked --lib refused_admission_fails_the_entry_wait_by_name`
     and record exit 124 and the wall time.
  3. Restore with `git checkout -- src/webhook/ca_construction_tests.rs` (after
     committing T1–T7 first, so the restore is exact).
  4. Rerun the committed test: green in under 5 s.

  *Proof:* the receipt lines (command, exit code, time) in the implement result.
- [ ] **T9. Check that production is untouched, then run every house gate locked** (AC7, AC8, AC9).
  1. Check that `git diff c95386e -- src/webhook.rs src/webhook/ca_executor.rs
     Cargo.toml Cargo.lock evals/Cargo.lock deny.toml scripts .brokkr .github`
     is empty.
  2. Check that no existing deadline value was raised: `wait_ca_idle` stays
     5 s, `BuildWatchdog` stays 20 s, and no `--test-threads` is added.
  3. Run, in order:
     - `cargo fmt --all -- --check`
     - `cargo clippy --workspace --all-targets --locked -- -D warnings`
     - `cargo test --workspace --locked`
     - `cargo build --locked --bin swarmail`
     - the evals crate's fmt, clippy and `SWARMAIL_BIN=<candidate> cargo test --locked`
     - `bash scripts/coverage-gate.sh` (100 % exact)
     - `bash scripts/coverage-gate.sh selftest`
     - `bash scripts/publish-guard-test.sh`
     - `bash scripts/seal-branch-test.sh`
     - `cargo deny check licenses`
     - the `brokkr compile` loop over `.brokkr/recipes/*/`. If the sandbox has
       no brokkr binary, report this gate as deferred to verify, not broken.

  *Proof:* every gate exits 0. Gate output is in the implement result.
- [ ] **T10. Record the follow-up.** Add a sentence to the implement result
  notes that the coverage-gate slow-test reporter (plan ruling 8) and the 0078
  §1 contention-vs-capacity question (ruling 2) are operator follow-ups and are
  not done here.

  *Proof:* the implement result notes contain both items.
