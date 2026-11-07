# holder.pid is published only once its PID is complete (issue #334)

## Feature

The evals test
`matrix_post_exit_inherited_pipe_is_bounded_and_holder_is_reaped_by_the_test`
(`evals/src/main.rs`, in `mod tests`) drives the inline trial-helper script in
`holder` mode. That mode backgrounds a `sleep 1000` that inherits the trial's
stdout. It then writes the sleep's PID to `$EVAL_TEST_MATRIX_HOLDER_PID`
(`holder.pid` in the probe's per-test tempdir) and exits. The test waits until
`holder.pid` exists and reads it at once.

The writer is `printf '%s\n' "$!" > "$EVAL_TEST_MATRIX_HOLDER_PID"`. The shell
opens the final name, which creates and truncates it, **before** `printf`
writes. So `holder.pid` exists at size 0 for a window. The reader's
`is_file()` wait can land in that window, and then
`read_to_string(..).trim().parse().unwrap()` panics on the empty string. That
is the 2026-11-05 failure in a full `cargo test --locked` run of `evals/`.

The same script already publishes its other identity file, `trial.pid`,
correctly: it writes `$ready.tmp.$$` and then `mv`s it onto the final name in
the same directory. The holder file is the one writer that does not follow the
script's own convention.

This change makes `holder.pid` follow it. The file is written under a
temporary name and renamed into place, so "the file exists" implies "its PID
line is complete" by construction. That is the invariant `trial.pid`'s reader,
`record_ready_identity`, already relies on. A staging gate between the
temporary write and the rename lets a test stop the writer at the one point
that matters and check, deterministically, that nothing is published early.
The reader checks the published line strictly, once, and fails with a message
that names `holder.pid` and its path. It never retries a parse.

## Why this shape (the chief's rulings)

- **The cause is fixed at the writer.** Reader-side "retry until it parses" is
  refused. With an atomic writer, a reader that waits for completeness has
  nothing to wait for. Its only effect would be to hide a writer that loses
  atomicity again, and that is the "retry that hides a race" the task forbids.
  This rules for the simplicity position. Cost to robustness: the reader
  never waits for completeness. If a future writer regresses, the result is a
  loud, named failure, not a tolerated one.
- **The reader stays strict, but checks only once.** The published bytes must
  be exactly one newline-terminated line of ASCII decimal digits, with a value
  in `1..=i32::MAX`. This adopts robustness's predicate as an assertion, not
  as a wait condition. A truncated prefix (for example `1` of `12345`) is
  rejected rather than recorded as an owned identity, so `cleanup_owned` can
  never signal an unrelated process group. Cost to simplicity: one small,
  pure helper and its table test.
- **The regression proof is the staging gate, in the shape of the existing
  pre-trap gate.** The pre-trap gate is an `entered` marker plus a spin on
  `release`. The test waits until *either* the staging marker or `holder.pid`
  exists, then asserts that `holder.pid` is absent. On c95386e the staging
  marker never appears and `holder.pid` always does, so the assertion fails on
  every run: probability 1, decision 0020 §5's pole. The failure names the
  actual defect, publication without staging, not just a missing seam. On the
  fix, `holder.pid` cannot exist before release, so the assertion passes on
  every run. This beats simplicity's "wait only for `.entered`" (a 20 s
  timeout that names the seam, not the bug). It also beats robustness's
  impersonation RED, which never runs the old code.
- **No decision record.** This applies a convention the same script already
  follows. It sets no new realm rule, so reserved number 0128 stays unused.
- **No hold-free proof of the rename by load or repetition.** The proof is
  deterministic, so a loop adds nothing and only loads the host.

Authorities: [0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md)
(accepted: test-writing disciplines, seams stay `#[cfg(test)]`) and
[0055](../../docs/decisions/0055-agent-eval-harness.md) (accepted: `evals/` is
outside the root coverage denominator). Frozen guarantees 0001–0003 and
0005 are not touched: no production code under `src/` changes.

## Acceptance Criteria

1. **Atomic publication.** In the trial-helper script's `holder` branch,
   the PID line is written to `"$EVAL_TEST_MATRIX_HOLDER_PID.tmp.$$"` (same
   directory) and published with `mv` onto `$EVAL_TEST_MATRIX_HOLDER_PID`.
   No redirection ever opens the final name.
2. **Staging gate.** Between the completed temporary write and the `mv`, the
   holder branch creates `"$EVAL_TEST_MATRIX_HOLDER_PID.entered"` and spins
   until `"$EVAL_TEST_MATRIX_HOLDER_PID.release"` exists, in the pre-trap
   gate's shape. It adds no new environment variable and no
   `MatrixProbeControls` field: `holder` mode has exactly one caller.
3. **Ordering proof (RED on c95386e, GREEN on the fix).** The test waits
   through the existing bounded `eventually` (20 s) until the staging marker
   or `holder.pid` exists. The message names both paths. It then asserts
   that `holder.pid` does not exist, with a message that names `holder.pid`,
   its path and the missing staging. On c95386e this assertion fails on every
   run. On the fix it passes on every run.
4. **Staged content is complete before publication.** While the gate holds,
   the tempdir contains exactly one entry whose name starts with
   `holder.pid.tmp.`, and its bytes pass the strict PID-line check.
5. **Rename, not copy.** After the test writes the release file, it waits
   (bounded `eventually`, with a message that names `holder.pid` and its path)
   until `holder.pid` is a file. It then reads `holder.pid` exactly once. The
   bytes pass the strict check and equal the staged PID, and no
   `holder.pid.tmp.*` entry remains (exact count 0).
6. **Strict, total, single-shot reader.** A pure helper
   (`complete_pid_line(&[u8]) -> Option<i32>`) accepts only one non-empty
   run of ASCII decimal digits followed by a single `\n`, with a value in
   `1..=i32::MAX`. Every other input returns `None`, and the helper never
   panics. The test turns `None` into a panic whose message names `holder.pid`
   (or the staged file), its path and the bytes it read. The read and the
   parse have no bare `unwrap()`. Neither is retried, and the strict check is
   never used as a wait condition.
7. **Predicate table test.** An inline test pins the helper:
   - `b""`, `b"1234"`, `b"\n"`, `b"1234\n5678\n"`, `b" 1234\n"`,
     `b"1234 \n"`, `b"12a4\n"`, `b"0\n"`, `b"-5\n"`, `b"+5\n"`, `b"2147483648\n"`
     and `b"99999999999999999999\n"` are all `None`;
   - `b"1234\n"` is `Some(1234)` and `b"2147483647\n"` is `Some(i32::MAX)`.
8. **The test's meaning is unchanged otherwise.** Its name is unchanged. These
   checks stay as they are: holder identity via `matrix_identity`,
   `record_identity`, `record_ready_identity(trial.pid)`, the 10 s EOF-marker
   wait and its diagnostic, `probe.wait().success()`, the trial-reaped and
   holder-alive assertions, the stderr text, and the explicit
   `cleanup_owned()` reap and its final assertion. No trap or other shell
   code kills the holder.
9. **Waits.** Every new or changed wait goes through the existing 20 s
   `eventually` and names what it waited for. No timeout is lengthened, no
   sleep is used as synchronization and no test-level retry is added.
10. **No dependency or surface change.** `Cargo.toml`, `Cargo.lock`,
    `evals/Cargo.toml`, `evals/Cargo.lock` and `deny.toml` are unchanged.
    Nothing under `src/`, `scripts/`, `.github/` or `.brokkr/` changes. No
    decision record is added or edited.
11. **Gates.** Every house gate in `docs/house-rules.md` § Gates is clean,
    `--locked`. That includes the evals fmt, clippy and test suite run against
    the candidate's `swarmail`, the coverage gate and its selftest, the
    publish-guard and seal proofs, and `cargo deny check licenses`. If brokkr
    is not on the implementer's PATH, the bundle-compile gate is reported as
    verify's.
