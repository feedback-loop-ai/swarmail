# Tasks: eval flake determinism (issue #302)

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md). Do the tasks in order.
Each task names the test that proves it. A red-before task is an uncommitted
perturbation; its result goes in the PR text.

## Case 1: lost observation fault (AC1)

- [ ] **T1. Red before (uncommitted).** On the unchanged seam, force the agent
  group's slot store to be overwritten by another owned group's store. For
  example, temporarily make the agent group await a notification after its
  store until the `serve` group has stored. Run `cargo test --locked --lib
  runner::test_probes::cleanup_observation_error_and_forced_expiry_fail_with_live_owned_processes
  -- --exact`.
  - *Proof:* it fails at `any(alive)`. Record the output for the PR.
  - If it does not go red, stop and return to design with the evidence.
- [ ] **T2. Fix.** In `evals/src/runner.rs`:
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

- [ ] **T3. Red before (uncommitted).** In the old helper, add a temporary
  delay between the `serve` kill and `return 0`. Run `cargo test --locked
  --test runner child_output_server_death_remains_a_harness_error_on_linux --
  --exact`.
  - *Proof:* it fails the `agent_exit_status == Some(0)` assertion. Record
    which assertion fired.
  - If the outcome assertion fires instead, stop and return to design as a
    runner ordering bug.
- [ ] **T4. Fix.** In `evals/tests/common/mod.rs`, at the top of the Linux
  block of `unsafe_output_then_server_death`, add `libc::signal(SIGTERM,
  SIG_IGN)` with its SAFETY comment.
  - *Proof:* with the T3 delay still applied, the test passes. With the delay
    removed, it passes again.
  - *Proof:* the `git diff` of `evals/src/runner.rs` around
    `services_survived`, `SERVICE_EXIT_GRACE` and `TERM_GRACE` is empty.

## Case 3: digest-forty control (AC3)

- [ ] **T5. Forced-collision rig.** In `evals/tests/fixture.rs`:
  - add `Rig::with_inputs`, with `Rig::new` delegating to it;
  - add the bounded deterministic seed search for `right == 1`;
  - add a forced digest-forty negative iteration that still uses the **old**
    hard-coded part-1 control.
  - *Proof (red before):* `cargo test --locked --test fixture
    focused_tools_complete_link_and_code_traps_without_guessing -- --exact`
    fails on the forced rig with `timed_out`, 0 candidates and the digest
    listed in `near_misses`. That is the reported shape, and it confirms the
    derivation.
- [ ] **T6. Fix.** Choose the digest-forty negative control from the first
  `ambiguous` answer's `anchor_text` set: part 1 if present, otherwise part 2.
  Add the presence and not-`DIGEST_ITEM` assertion with a named message. Keep
  `timeout_ms: 0`, `selected`, `WrongLink` and all positive assertions
  verbatim.
  - *Proof:* the T5 test passes, including the forced rig, which now selects
    part 2 and records a real `WrongLink`.
  - *Proof:* `git diff --stat -- src/ tests/` (root) is empty.

## Proof under load (AC4)

- [ ] **T7. Load proof (PR text only).** Prebuild the candidate `swarmail` and
  the evals test binaries, with every target dir on `/home`. Start at least 4
  parallel copies of the full evals test binaries as load, then run each
  target with `--exact`:
  - case 1 at least 100 times;
  - case 2 at least 100 times;
  - case 3 at least 200 times;
  - the full evals suite once under the same load.
  - *Proof:* every target passes all of its runs (100/100, 100/100 and
    200/200), and the full suite passes. The command, the counts and the
    wall time go in the PR. Nothing from the proof is committed.

## Gates and delivery (AC5–AC8)

- [ ] **T8. Scope audit.**
  - *Proof:* `git diff --name-only c336c95..HEAD -- . ':!specs'` lists only
    `evals/src/runner.rs`, `evals/tests/common/mod.rs` and
    `evals/tests/fixture.rs`.
  - *Proof:* every `evals/src/runner.rs` hunk is under
    `cfg(all(test, target_os = "linux"))` or inside `mod test_probes`.
  - *Proof:* `git diff c336c95 -- Cargo.lock evals/Cargo.lock deny.toml` is
    empty.
- [ ] **T9. House gates, locked.** Run every gate in docs/house-rules.md §
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
- [ ] **T10. Commit.** Make one unsigned implementation commit in the repo's
  style. Its body states the three causes, including that case 3 is a
  test-data collision and not a product bug.
  - *Proof:* `git log -1 --format='%G?'` prints `N`.
  - *Proof:* nothing is pushed.
  - *Proof:* the PR text carries the T1, T3, T5 and T7 evidence and the
    falsifier statement.
