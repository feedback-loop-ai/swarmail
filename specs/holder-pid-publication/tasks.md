# Tasks — holder.pid publication (issue #334)

Ordered. Each task names the test that proves it. All code is in
`evals/src/main.rs` `mod tests`.

- [ ] **T1 — Strict PID-line helper.** Add `complete_pid_line(&[u8]) -> Option<i32>`
  (spec criterion 6) and the one-shot `published_pid(label, path) -> i32`
  wrapper, whose panic message names the label, the path and the bytes.
  *Proof:* new inline test `holder_pid_line_is_accepted_only_when_complete`
  covers every input in spec criterion 7 with its exact expected `Option`.
- [ ] **T2 — Rewrite the test body first (RED).** In
  `matrix_post_exit_inherited_pipe_is_bounded_and_holder_is_reaped_by_the_test`,
  add the either-or bounded wait, the "absent before staging" assertion,
  the exact-one staged-entry check, the release write, the bounded publish
  wait, the single `published_pid` read, the equality check against the
  staged PID and the zero-remaining-temp check (plan §3; spec criteria 3–6,
  9). Leave every later assertion unchanged (criterion 8).
  *Proof:* run it against c95386e's script text. It fails with
  "published without staging". Record that output.
- [ ] **T3 — Atomic writer with a staging gate (GREEN).** In the script's
  `holder` branch, write `$EVAL_TEST_MATRIX_HOLDER_PID.tmp.$$`, then touch
  `.entered`, spin on `.release`, `mv` onto the final name and `exit 0`
  (spec criteria 1–2). Add no env var and no controls field.
  *Proof:* `cd evals && SWARMAIL_BIN=<candidate> cargo test --locked matrix_post_exit_inherited_pipe`
  passes.
- [ ] **T4 — Confirm bounded teardown while the gate holds.** Read `Drop`
  and confirm that `record_descendants()` plus `cleanup_owned()` reap a
  shell held at the gate. Change no code unless the reading shows a gap.
  *Proof:* T3's test passes, and the full evals suite run in T6 leaves no
  `eprintln!` teardown diagnostic for this test.
- [ ] **T5 — Surface check.** Confirm that `git diff --stat c95386e` touches
  only `evals/src/main.rs` (plus this design) and that no manifest, lockfile,
  `deny.toml`, `src/`, `scripts/`, `.github/`, `.brokkr/` or decision file
  changed (spec criterion 10).
  *Proof:* `git diff --name-only c95386e..HEAD` lists only those paths.
- [ ] **T6 — Every house gate, `--locked`** (spec criterion 11): root fmt,
  clippy `-D warnings`, `cargo test --workspace`, `cargo build --bin swarmail`;
  evals fmt, clippy `--all-targets -D warnings`, and
  `SWARMAIL_BIN=<candidate> cargo test` (the full suite); the recipe compile
  loop (verify's if brokkr is not on PATH, and said so);
  `scripts/coverage-gate.sh` and `selftest`; `scripts/publish-guard-test.sh`;
  `scripts/seal-branch-test.sh`; `cargo deny check licenses`.
  *Proof:* every command exits 0. Report each result.
