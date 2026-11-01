# Trial isolation tasks — the boundary is bubblewrap

Ordered. Each task names the test that proves it. AC, R, P and Q references
point to [spec.md](spec.md) as revised 2026-11-01 for the fourth time,
after the second review. A proposed decision is not authority to activate a
contradictory contract (accepted 0045 §8); concurrency stays off and no row
version changes. Tasks closed by earlier commits that this revision keeps
stay closed; the live proof's findings were T21–T28 (done at 4f3ba15); the
second review's findings are T29–T33, in the order the implementer does
them; T10 is reopened behind them, and T06–T08 and T20 stay open until the
dispatcher's host run.

- [x] **T01 — Carry the decision and its index row (AC-S8, AC-O1).**
  `docs/decisions/0115-trial-isolation.md` stays `proposed`, now with its
  *After the live proof* subsection (committed by the chief with this
  design); the README index row stands. **Test:** `design-check.sh` resolves
  `Decision: 0115` to one clean proposed record the spec links;
  `git diff a4f552d -- docs/decisions/0055-agent-eval-harness.md
  docs/decisions/0076-sequential-eval-trials.md` is empty; the README has
  exactly one 0115 row.
- [x] **T02 — Establish the leak live, before any boundary (AC-S1).** Kept.
- [x] **T12 — Delete the raw-syscall worker (AC-S9).** Done at bdc504f.
- [x] **T03 — The pure bwrap parts (AC-S4).** Done at bdc504f, revised at
  f1a30a7. The argv does not change in this revision: the golden test stays
  byte-identical.
- [x] **T04 — The controller and the parameter seam (AC-S2, AC-S3, AC-S5).**
  Done at bdc504f and f1a30a7; T21–T24 rebuild the controller's runtime,
  writers and waits behind the same seam.
- [x] **T05 — Skip discipline for the live proofs (AC-S6).** Done; T26
  adds the always-on half of the rule.
- [x] **T13–T18 — The first review's findings (R1–R14).** Done at f1a30a7
  and confirmed by the live run (bwrap 0.11.1 accepted `--as-pid-1`; the
  runner was PID 1 inside).
- [x] **T19 — Documentation (AC-S8).** Done at f1a30a7; T27 revises the
  facts the live run changed.
- [x] **T11 — Egress question to the operator (AC-L4, AC-O1).** Carried in
  0115; unchanged.
- [x] **T21 — Reproduce the deadlock red in the seat (P1, P2; AC-S11).**
  Before changing production code, add
  `a_controller_holding_its_own_stdout_lock_still_delivers_the_row_and_exits`
  and its re-exec child half to `isolation.rs`'s controller tests: the
  child takes `std::io::stdout().lock()` on its main thread, runs the
  production entry (T22's `isolation::run`; until T22 lands, the same
  current-thread-plus-`launch` shape `main` will use) against the stand-in
  that prints a row and exits 1, and writes the result to a file; the
  parent polls to a 30 s deadline, `SIGKILL`s and fails on expiry, else
  requires `Ok(Exited(1))`. **Test:** the test is red on the pre-fix
  `pump`/runtime (the child times out or reports `NoRow`); the red run is
  recorded in the commit body or the implement notes; it is green after
  T22–T23.
- [x] **T22 — One production entry, a current-thread runtime, no lock
  across the trial (P1, P2).** In `isolation.rs` add `Bounds` and
  `pub fn run(request, inner, bwrap, bounds)` building
  `new_current_thread().enable_all()`, `block_on(launch(..))`, then
  `shutdown_timeout(bounds.drain)`; `launch` takes `bounds`. In `main.rs`
  bind `prepare_trial`'s result with `let` before the `match`, build no
  runtime in the isolated arm, and call `isolation::run` with
  `Bounds::for_trial(deadline)` where the deadline is `--deadline-ms` else
  `tasks::find(task).deadline_ms`; delete the tests' private `run` helper
  and route `interrupt_child` and `through_the_controller` through the
  production entry with test bounds. **Test:** every existing controller
  and entry test passes through `isolation::run`; `grep -n "new_multi_thread" evals/src/main.rs`
  shows only the non-isolated arm; `grep -n "stdout().lock()\|stderr().lock()" evals/src/main.rs`
  shows no use inside a `match` scrutinee or across a `block_on`.
- [x] **T23 — The pumps write on the driving thread, and count on the read
  side (P1, P5).** `pump` calls `write_through` directly (no
  `spawn_blocking`, no per-chunk `to_vec`) and adds each read to a shared
  `Arc<AtomicU64>`; `supervise` judges by that counter after the drains.
  **Test:** T21 goes green; `a_row_on_stdout_is_mirrored_with_the_exit_code_it_came_with`
  and `a_silent_exit_after_a_good_probe_is_a_named_no_row` still pass; a
  new stand-in case where bwrap writes a row, backgrounds a grandchild that
  holds stdout open past `bounds.drain` (200 ms in the test) and exits 0
  yields `Exited(0)`, never `NoRow`.
- [x] **T24 — Bound bwrap's wait (P6; AC-S2).** Lift the matrix's
  `300000` into `pub const TRIAL_GRACE_MS` in `isolation.rs` and use it in
  `run_child_assigned`; in `supervise`, wait for bwrap under
  `bounds.envelope`; on expiry `start_kill()`, reap within `bounds.drain`,
  print `swarmail-evals: isolated trial envelope expired; bwrap killed` on
  stderr, and fall through to the existing verdict. **Test:** a stand-in
  that passes the version and the probe, writes a row and `exec`s
  `/bin/sleep 60` yields `Exited(2)` within `envelope + drain` (200 ms +
  200 ms in the test), the stand-in's PID is gone or a zombie afterwards,
  and the controller's stderr (read through the re-exec pattern or a
  captured `Sink`) carries the fixed line; the same stand-in writing nothing
  yields `NoRow`; the matrix's existing envelope tests pass unchanged with
  the shared constant.
- [x] **T25 — One bounded, file-backed runner for every isolated CLI child
  (P3; AC-S12).** In `tests/common/mod.rs`: `ChildGuard` owns its output
  files and a deadline; `bounded(command, bound) -> ChildGuard`;
  `wait_with_output` polls `try_wait` to the deadline, returns an `Output`
  built from the files, and on expiry `SIGKILL`s, reaps and panics
  `controller outlived its row after 90s` with both files' contents;
  `ISOLATED_CLI_BOUND = 90 s`. Route `boundary_unavailable()` (now with
  `--deadline-ms 30000`), `trial()` with `--isolated`, `launch_victim`, and
  in `trial_isolation.rs` `isolated_helper_with`, `idle_trial`,
  `idle_victim`, the hostile proof's trial B and the oracle trial through
  it. **Test:** a unit test of `bounded` with a child that `exec`s
  `/bin/sleep 60` under a 200 ms bound panics with the named message
  (caught by `catch_unwind`) and leaves no such process; `grep -n "\.output()\|\.wait_with_output()" evals/tests/trial_isolation.rs evals/tests/common/mod.rs`
  shows no isolated command outside the helper; the always-on suite
  finishes in the seat as before.
- [x] **T26 — The always-on handoff test and the two-suite skip rule (P3;
  AC-S6, AC-S10).** Add `an_isolated_null_trial_hands_over_its_row_and_exits`
  (Linux, not ignored): through `bounded`, `trial --isolated --task signup
  --agent null --deadline-ms 30000`; where `boundary_unavailable()` names a
  step, print `SKIP trial-isolation: boundary_unavailable:<step>` and
  return; otherwise require exactly one row whose outcome is not
  `harness_error`, no `isolated_trial_no_row` and no `boundary_unavailable`
  on stderr, and exit code 1. Make every always-on test that needs the
  boundary print the same SKIP line before returning, and make
  `an_unavailable_boundary_refuses_by_name_and_never_falls_back` and
  `an_exposed_external_agent_passes_the_precondition_and_reaches_the_boundary`
  print `SKIP trial-isolation: boundary available` and return where the
  boundary works. Revise the module doc to state both rules, the 90 s bound
  and the 20-minute limit. **Test:** in the seat the new test prints its
  SKIP and passes within seconds; on the host (the dispatcher's always-on
  run) it passes by actual exit with the null agent's row; `require_boundary`
  still panics after its SKIP line in the ignored suite.
- [x] **T27 — Documentation and the corrected sentences (P4; AC-S8).**
  Revise the dated #221 sections of `evals/README.md` and
  `evals/run-plan.md`: the proof ran once on 2026-11-01, the boundary
  worked, the controller lost the row and hung and is now bounded at every
  wait and exits after its row; every isolated test child is held at 90 s;
  the always-on suite skips by name without bwrap and the ignored suite
  fails by name; the dispatcher's 20-minute limit is the acceptance gate.
  **Test:** a requirement-to-observation read against the spec and 0115; no
  "under five minutes", no "has not yet run", no claim that the live
  criteria passed.
- [x] **T28 — Seat run of both suites (AC-S6, AC-S7).** Run the always-on
  evals suite locked in the seat and show AC-S10 skipped by name and every
  other test green within bounded time; run the `--ignored` command and
  show every live proof red with `SKIP trial-isolation: boundary_unavailable:bwrap_probe`,
  each within its bound. **Test:** both receipts in the implement notes
  with wall-clock durations.
- [x] **T29 — Judge the reap, speak 0076 §3's line (Q1; AC-S13).** In
  `isolation.rs` add `enum Cleanup { Proved, Unproved }`,
  `pub const CLEANUP_UNPROVED` (the exact `swarmail-evals: harness error:
  trial_cleanup_unproved\n` line), `fn envelope_report(Cleanup) ->
  &'static str` and `fn verdict(status, seen, abandoned) -> Launched` (the
  existing match, lifted unchanged). `supervise` returns `(Launched,
  Cleanup)`: `Unproved` exactly when the envelope fired and
  `timeout(bounds.drain, child.wait())` ran out, `Proved` otherwise; the
  envelope arm writes `envelope_report(cleanup)` where it wrote
  `ENVELOPE_EXPIRED`. No `Launched` variant, no row reason, no `conclude`
  change. **Test:** inline, `envelope_report(Proved)` equals
  `ENVELOPE_EXPIRED` and contains no `trial_cleanup_unproved`;
  `envelope_report(Unproved)` is `ENVELOPE_EXPIRED` then `CLEANUP_UNPROVED`,
  the cleanup line once and after the envelope line; `CLEANUP_UNPROVED`
  equals the literal the matrix parser matches; the `verdict` table
  (`(Err, 0, false)` → `NoRow`; `(Err, 9, false)` → `Exited(2)`;
  `(Ok(code 1), 9, false)` → `Exited(1)`; `(Ok(signal), 9, false)` →
  `Exited(2)`; `(Ok(code 0), 9, true)` → `Interrupted`); in `main.rs`,
  `capture_worker_category` fed `envelope_report(Unproved)` returns
  `trial_cleanup_unproved`; `an_expired_envelope_says_so_on_stderr_in_one_fixed_line`
  additionally asserts `stderr.matches("trial_cleanup_unproved").count() == 0`;
  every existing controller test still passes.
- [x] **T30 — Retain the admission on an unproved reap (Q1; AC-S13).** In
  `admission.rs` add `pub(crate) fn retain(self)` that `std::mem::forget`s
  the lock file (doc: 0076 §3; released only by process exit; the
  process-scoped limit). In `launch`: `drop(admission)` on `Proved`,
  `admission.retain()` on `Unproved`. **Test:** `retain_child` (re-exec
  half: reads `ADMISSION_TEST_ROOT`, `Admission::at(root, uid)`, `retain`,
  asserts a second `Admission::at(root, uid)` is
  `Err("concurrent_trials_refused")`, exits 0) and
  `a_retained_admission_is_released_only_by_the_process_exit`: spawns the
  child with a tempdir root, polls `try_wait` to a 30 s deadline,
  `SIGKILL`s and panics by name on expiry, requires exit 0, then
  `Admission::at(root, uid)` succeeds. Confirm the `Launched`-to-`retain`
  wiring by reading `launch`: exactly one `retain` call, on the `Unproved`
  arm.
- [x] **T31 — Correct the `Sink` comment (Q3).** The doc comment on `Sink`
  says the streams are written on the thread that drives the runtime, the
  only thread that may take the std locks, never from a blocking task.
  **Test:** `grep -n "off the async threads" evals/src/isolation.rs` is
  empty; `grep -n "spawn_blocking" evals/src/isolation.rs` is empty.
- [x] **T32 — Documentation (Q1; AC-S8).** One sentence in the
  bounded-controller paragraph of `evals/README.md` and `evals/run-plan.md`:
  a reap the controller cannot prove within its bound is the named
  `trial_cleanup_unproved` harness error on stderr, the admission is held to
  the controller's exit, exit 2, and an operator confirms cleanup before
  resuming — never an ordinary no-row. **Test:** a requirement-to-observation
  read against spec step 3 and 0115's *After the second review*; no claim
  that the live criteria passed; `git diff a4f552d --
  docs/decisions/0076-sequential-eval-trials.md` empty; 0115's *Amends if
  accepted* line unchanged.
- [x] **T33 — Seat run of both suites after T29–T32 (AC-S6, AC-S7).** Run
  the always-on evals suite locked in the seat and show AC-S10 skipped by
  name and every other test green within bounded time; run the `--ignored`
  command and show every live proof red with
  `SKIP trial-isolation: boundary_unavailable:bwrap_probe`, each within its
  bound. **Test:** both receipts in the implement notes with wall-clock
  durations.
- [ ] **T06 — Hostile live proof, boundary holds (AC-L1).** As at f1a30a7,
  under `bounded`. **Test:** on this host, the probe exits 0 (all routes
  denied), A's row after its agent is killed from outside is `fail` in
  signup's declared idle class, never `pass`/`budget`/`harness_error`; a
  leak or a third void makes it red; no child outlives 90 s.
- [ ] **T07 — Fresh HOME, own services and the observed topology (AC-L2).**
  As at f1a30a7, under `bounded`. **Test:** on this host, the observer exits
  0 (or 100/101/102 with its observations) and the oracle passes signup
  inside; a nonzero code names the first violated check.
- [ ] **T08 — Lifetime bound on both ends (AC-L3).** As at f1a30a7, under
  `bounded`. **Test:** on this host, `SIGKILL` to the controller leaves no
  trial process alive; `SIGKILL` to the runner's host PID leaves no agent
  alive and the controller exits 2 with the `isolated_trial_no_row` row;
  `SIGTERM` to the controller after the agent exists yields the runner's
  interrupted row, exit 2 and no survivors; no orphan of A is alive when B
  starts.
- [ ] **T10 — Gates (AC-S7), reopened after T29–T32.** Run the full house gate inventory locked plus
  the standalone evals `fmt`, `clippy --all-targets` and `cargo test
  --locked`. **Test:** command/exit/log receipts for every required check
  bound to the candidate, literal zero-missed root coverage with its
  selftest, publish-guard and seal proofs, `cargo deny check licenses` and
  the fail-closed native-tls absence; neither lockfile nor `deny.toml`
  changed; `git diff a4f552d --stat -- src/` empty; CI-owned obligations
  retained, not claimed; the run does not report complete while AC-L1..L4
  and AC-O1 are open.
- [ ] **T20 — Live proof outstanding (AC-L1..L3).** The dispatcher runs the
  one `--ignored` command on this host after the run with a hard 20-minute
  limit; that run is the acceptance gate of PR #308 and the only green that
  closes T06–T08. **Test:** the host run's receipt, green, with its measured
  duration recorded here by whoever runs it.

Status 2026-11-01 (chief, after the first live proof): T21–T28 are open for
the implement seat, in that order; T06–T08 and T20 behind them, proven only
by the dispatcher's one command on this host. The earlier status notes of
the implement seat (bdc504f, f1a30a7) stand in git history.

Status 2026-11-01 (implement seat): T21–T28 are done. Receipts:

- T21 red before the fix: with `a_controller_holding_its_own_stdout_lock_still_delivers_the_row_and_exits`
  added and production code untouched, `cargo test --locked --lib a_controller_holding`
  failed after 30.01 s with "the controller outlived its row while holding the
  stdout lock" (the child never wrote its verdict: the pump's blocking writer
  parked on the lock and the runtime's drop joined it). Green after T22–T23.
- T22–T24: `isolation::run`, `Bounds`, `TRIAL_GRACE_MS`, direct pump writes with
  a read-side count and the envelope on bwrap's wait; the unit tests of the
  controller all go through `isolation::run`; new cases for the envelope
  (row-then-hang `Exited(2)`, nothing-then-hang `NoRow`, the fixed stderr
  line once, row-then-grandchild-holds-stdout `Exited(0)`).
- T25–T26: `common::bounded`, `ISOLATED_CLI_BOUND = 90 s`, `isolated_output`;
  every isolated CLI child and the victim's bwrap go through it; new
  `an_isolated_null_trial_hands_over_its_row_and_exits` and
  `a_child_that_outlives_its_bound_fails_by_name_and_is_killed`.
- T28 in the seat: `cargo test --locked --test trial_isolation -- --nocapture
  --test-threads=1` finished in 5.85 s (8.2 s wall with the build): 14 passed, 7
  ignored, the handoff test printed `SKIP trial-isolation:
  boundary_unavailable:bwrap_probe`. The same with `-- --ignored` finished in
  1.78 s: 7 failed, each after `SKIP trial-isolation:
  boundary_unavailable:bwrap_probe`. The whole evals `cargo test --locked`
  passed (about 10 min, dominated by the unrelated runner and wrapper suites).
- T10 in the seat: root fmt, clippy, test, `cargo build --locked --bin swarmail`,
  evals fmt and clippy, the coverage gate (exact 100/100/100) and its selftest,
  the publish-guard proof, the seal-branch proof and `cargo deny check licenses`
  are green; `git diff a4f552d --stat -- src/` is empty; neither lockfile nor
  `deny.toml` changed. `brokkr compile --bundle .brokkr/recipes/self` was **not
  run**: the pinned brokkr binary is not installed in this seat and the seat
  installs nothing.
- T06–T08, T20: open, proven only by the dispatcher's one `--ignored` command.

Status 2026-11-01 (chief, after the second review): the review panel
confirmed T21–T28 and returned the run on Q1 (the reap the controller
cannot prove released the admission as proved and said nothing, against
accepted 0076 §3) beside Q2–Q7. T29–T33 are open for the implement seat, in
that order, with T10 reopened behind them; T06–T08 and T20 stay open and are
proven only by the dispatcher's one command on this host. The implementer
reads these tasks, not the review's notes; nothing in 4f3ba15's controller
fix is reopened.

Status 2026-11-01 (implement seat, after the second review): T29–T33 are
done. Receipts (seat; bwrap refuses here with `ENOSPC`, so only refusal paths
and stand-ins ran):
- T29/T30/T31: `Cleanup`, `CLEANUP_UNPROVED`, `envelope_report`, `verdict`
  and `Admission::retain` implemented; `launch` calls `retain` on
  `Unproved` and `drop` on `Proved`, one call each. Tests:
  `the_reap_judgement_speaks_the_same_line_the_matrix_parses`,
  `the_verdict_table_exits_two_whatever_reached_stdout_when_not_reaped`,
  `the_isolated_controllers_unproved_report_is_the_category_the_matrix_parses`
  (main.rs), `a_retained_admission_is_released_only_by_the_process_exit`
  (re-exec), and the envelope-line test now asserts zero
  `trial_cleanup_unproved` on the `Proved` pole. The grep for
  `off the async threads` and `spawn_blocking` is empty. The two admission
  tests take `trial_scope` on Linux: the existing rendezvous test failed once
  under load when a concurrent fork briefly held the lock descriptor.
- T32: one sentence in `evals/README.md` and `evals/run-plan.md`.
- T33: `cargo test --manifest-path evals/Cargo.toml --locked` finishes (about
  8 min in this seat, every isolated child bounded), 7 boundary tests
  ignored/skipped by name; the `--ignored` command takes 1.8 s and every live
  proof is red with `SKIP trial-isolation: boundary_unavailable:bwrap_probe`.
  One unrelated flake seen once under load and green on re-run:
  `child_output_server_death_remains_a_harness_error_on_linux`.
- T10: root and evals `fmt`, `clippy -D warnings`, `cargo test --workspace
  --locked`, `cargo build --locked --bin swarmail`, `cargo deny check
  licenses`, `scripts/coverage-gate.sh` (exact 100/100/100) and its
  `selftest`, `publish-guard-test.sh` and `seal-branch-test.sh` are green.
  `brokkr compile --bundle .brokkr/recipes/self` was **not run**: the pinned
  brokkr binary is not installed in this seat. T06–T08 and T20 stay open for
  the dispatcher's host run.
