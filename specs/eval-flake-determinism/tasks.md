# Tasks: eval flake determinism (issue #302)

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md). Do the tasks in order.
Each task names the test that proves it. A red-before task is an uncommitted
perturbation; its result goes in the PR text.

T1–T10 were done at `c3b4ab7` (implement and verify results of this run) and
are checked; their text is corrected where the 2026-11-02 review return
changed the criterion. The open work is the revision tasks R1–R6 at the end.

## Case 1: lost observation fault (AC1)

- [x] **T1. Red before (uncommitted).** On the unchanged seam, force the agent
  group's slot store to be overwritten by another owned group's store. For
  example, temporarily make the agent group await a notification after its
  store until the `serve` group has stored. Run `cargo test --locked --lib
  runner::test_probes::cleanup_observation_error_and_forced_expiry_fail_with_live_owned_processes
  -- --exact`.
  - *Proof:* it fails at `any(alive)` at a measured, probabilistic rate (about
    1 in 4 on the `c3b4ab7` run). Record the rate and output for the PR.
  - If it never goes red, stop and return to design with the evidence.
- [x] **T2. Fix.** In `evals/src/runner.rs`:
  - delete `CLEANUP_OBSERVATION_TARGET`, the CAS block in `group_members`, and
    the store in `cleanup_once`;
  - add the direct-return block (plan § Case 1), keeping the error string
    byte-identical;
  - extend only the `any(alive)` assertion message.
  - *Proof:* `grep -rn CLEANUP_OBSERVATION_TARGET evals/` is empty.
  - *Proof:* the T1 test passes, and every other `runner::test_probes` test
    passes.
  - *Proof:* `git diff` of the test body shows no changed assertion
    condition.

## Case 2: exit-0 witness and reaping order (AC2)

- [x] **T3. Red before (uncommitted).** In the old helper, add a temporary
  delay between the `serve` kill and `return 0`. Run `cargo test --locked
  --test runner child_output_server_death_remains_a_harness_error_on_linux --
  --exact`.
  - *Proof:* it fails the `agent_exit_status == Some(0)` assertion. Record
    which assertion fired.
  - If the outcome assertion fires instead, stop and return to design as a
    runner ordering bug.
- [x] **T4. Fix.** In `evals/tests/common/mod.rs`, at the top of the Linux
  block of `unsafe_output_then_server_death`, add `libc::signal(SIGTERM,
  SIG_IGN)` with its SAFETY comment.
  - *Proof:* with the T3 delay still applied, the test passes. With the delay
    removed, it passes again.
  - *Proof:* the `git diff` of `evals/src/runner.rs` around
    `services_survived`, `SERVICE_EXIT_GRACE` and `TERM_GRACE` is empty.

## Case 3: digest-forty control (AC3)

- [x] **T5. Forced-collision rig.** In `evals/tests/fixture.rs`:
  - add `Rig::with_inputs`, with `Rig::new` delegating to it;
  - add the bounded deterministic seed search for `right == 1`;
  - add a forced digest-forty negative iteration that still uses the **old**
    hard-coded part-1 control.
  - *Proof (red before):* `cargo test --locked --test fixture
    focused_tools_complete_link_and_code_traps_without_guessing -- --exact`
    fails on the forced rig with `timed_out`, 0 candidates and the digest
    listed in `near_misses`. That is the reported shape, and it confirms the
    derivation.
- [x] **T6. Fix.** Choose the digest-forty negative control from the first
  `ambiguous` answer's `anchor_text` set: part 1 if present, otherwise part 2.
  Add the presence assertion with a named message (the not-`DIGEST_ITEM`
  conjunct added at `c3b4ab7` is removed by R2). Keep
  `timeout_ms: 0`, `selected`, `WrongLink` and all positive assertions
  verbatim.
  - *Proof:* the T5 test passes, including the forced rig, which now selects
    part 2 and records a real `WrongLink`.
  - *Proof:* `git diff --stat -- src/ tests/` (root) is empty.

## Proof under load (AC4)

- [x] **T7. Load proof (PR text only).** Prebuild the candidate `swarmail` and
  the evals test binaries, with every target dir on `/home`. Start load per
  spec AC4 (done at `c3b4ab7` with 32 CPU hogs plus 4 looping root
  real-protocol test binaries, because evals copies serialise on 0076's
  admission lock), then run each target with `--exact`:
  - case 1 at least 100 times;
  - case 2 at least 100 times;
  - case 3 at least 200 times;
  - the full evals suite once under the same load.
  - *Proof:* every target passes all of its runs (100/100, 100/100 and
    200/200), and the full suite passes. The command, the counts and the
    wall time go in the PR. Nothing from the proof is committed.

## Gates and delivery (AC5–AC8)

- [x] **T8. Scope audit.**
  - *Proof:* `git diff --name-only c336c95..HEAD -- . ':!specs'` lists only
    `evals/src/runner.rs`, `evals/tests/common/mod.rs` and
    `evals/tests/fixture.rs`.
  - *Proof:* every `evals/src/runner.rs` hunk is under
    `cfg(all(test, target_os = "linux"))` or inside `mod test_probes`.
  - *Proof:* `git diff c336c95 -- Cargo.lock evals/Cargo.lock deny.toml` is
    empty.
- [x] **T9. House gates, locked.** Run every gate in docs/house-rules.md §
  Gates:
  - root fmt, clippy, tests and `cargo build --locked --bin swarmail`;
  - evals fmt, clippy, and `SWARMAIL_BIN=<artifact-record binary> cargo test
    --locked`;
  - `brokkr compile --bundle .brokkr/recipes/self`;
  - `bash scripts/coverage-gate.sh` and `bash scripts/coverage-gate.sh
    selftest`;
  - `bash scripts/publish-guard-test.sh`;
  - `bash scripts/seal-branch-test.sh`;
  - `cargo deny check licenses`.
  - *Proof:* every command exits 0. Record the outputs in the verify evidence.
- [x] **T10. Commit.** Make one unsigned implementation commit in the repo's
  style. Its body states the three causes, including that case 3 is a
  test-data collision and not a product bug.
  - *Proof:* `git log -1 --format='%G?'` prints `N`.
  - *Proof:* nothing is pushed.
  - *Proof:* the PR text carries the T1, T3, T5 and T7 evidence and the
    falsifier statement.

## Revision tasks (review return on `c3b4ab7`, 2026-11-02)

- [ ] **R1. Confirm the spec-only corrections need no code** (answers the
  guarantees spec defect: 0094/0111 proposed, 0045 §8 basis).
  - *Proof:* `git diff c3b4ab7 HEAD --stat` before R2 lists only
    `specs/eval-flake-determinism/`, and `grep -n "0111\|0094"
    docs/decisions/README.md` still shows both as proposed.
- [ ] **R2. Delete the constant-only conjunct** (answers correctness INFO on
  AC3; independent INFO on the tuple literal). In `evals/tests/fixture.rs`
  `digest_forty_control`:
  - change the assertion to `anchors.contains(&title(part).as_str())` alone,
    with the message naming the task, `negative` and the chosen title, and
    stating it must be among the pre-call candidates;
  - add a one-line comment on the digest-forty tuple's decoy literal saying
    `digest_forty_control` replaces it in every digest-forty negative rig;
  - change nothing else: both calls keep `timeout_ms: 0`; the `selected`,
    `WrongLink` and positive-rig assertions stay byte-identical.
  - *Proof:* `grep -n "DIGEST_ITEM" evals/tests/fixture.rs` no longer shows a
    `!=` comparison; `cd evals && SWARMAIL_BIN=<artifact-record binary> cargo
    test --locked --test fixture
    focused_tools_complete_link_and_code_traps_without_guessing -- --exact`
    passes, including the forced `right == 1` rig.
  - *Proof:* `git diff c3b4ab7 -- evals/` touches only
    `evals/tests/fixture.rs`, and only `digest_forty_control` and the comment.
- [ ] **R3. Falsifier still bites** (AC3). Temporarily (uncommitted) force `part = 1` in
  `digest_forty_control`, leaving the presence assertion in place.
  - *Proof:* the same test goes red on the forced rig at the presence
    assertion, with the named message, before any MCP call times out.
    Revert and record the output for the PR.
- [ ] **R4. Load proof on the shipped head** (answers the AC4 low residual).
  Load per spec AC4: `nproc` CPU hogs plus at least 4 looping root
  real-protocol test binaries, prebuilt, run directly, target dirs on `/home`;
  sample `/proc/loadavg` during the measurement.
  - *Proof:* case 3 passes at least 200/200 and the whole evals suite passes
    once, both on the R2 head, with the 1-minute load average ≥ `nproc`.
  - *Proof:* case 1 and case 2 counts from `c3b4ab7` (100/100 each) are
    reused only because R2's diff proof shows `evals/src/` and
    `evals/tests/common/` unchanged; otherwise re-run both at ≥100.
- [ ] **R5. Gates and commit** (AC5–AC8).
  - Run every gate of T9 again on the R2 head; every command exits 0.
  - The T8 scope audit still holds against `c336c95`.
  - Commit R2 unsigned in the repo's style; the body says the conjunct
    compared two constants and `WrongLink` is the runtime proof.
  - *Proof:* `git log -1 --format='%G?'` prints `N`; nothing is pushed.
- [ ] **R6. PR evidence** (AC9; answers the correctness and independent
  notes that the deviations must reach the PR).
  - *Proof:* the implement result hands ship every AC9 item: the case-3
    classification and falsifier, the release-file rejection, case 1's
    measured red-before rate, case 2's red-before and residual, case 3's
    forced-rig red-before and R3 output, the R4 command, counts with the head
    of each, load average and wall time with the 0076 reason, and the
    0094/0111-proposed / 0045 §8 statement.
