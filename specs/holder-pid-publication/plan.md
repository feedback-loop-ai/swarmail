# Plan — holder.pid publication (issue #334)

Spec: [spec.md](spec.md).

## Technical context

- **Language and surface:** Rust 2024. The change is test-only, inside
  `evals/src/main.rs` `mod tests`: the inline trial-helper shell script in
  `matrix_probe_config`, one new pure helper, one rewritten test body and one
  new inline test. All of it is under `#[cfg(target_os = "linux")]`, like the
  surrounding matrix-probe code.
- **Workspace:** `evals/` is standalone and unpublished, and sits outside the
  root coverage denominator (decision 0055). No root crate code changes.
- **Dependencies:** none are added. `tempfile`, `libc` and `serde_json` are
  already dev-reachable in `evals/`.
- **Platform:** Linux only, the same as the existing `/proc`-based identity
  proofs.

## Constitution check

| Rule | Status |
|---|---|
| Frozen guarantees 0001–0003, and 0005 | Untouched. No `src/` change. |
| 0020 §5: no sleep as synchronization; chaos only at the poles | The gate is a file handshake. Every wait is the bounded `eventually`. The RED fails with probability 1 and the GREEN passes with probability 1. |
| 0020 §4: seams are `#[cfg(test)]`-only | The gate lives in a script that only `#[cfg(test)]` code writes. No feature flag. |
| Per-test isolation | Every path is under the probe's own `tempfile::tempdir()`. `$$` keeps the temporary name unique. |
| Dependency policy | No change to manifests, lockfiles or `deny.toml`. |
| Decision records | None. This applies an existing script convention (`trial.pid`'s tmp + `mv`). No new rule, so 0128 stays unused. |
| Merge holds (0056 addendum) | None triggered: no `scripts/`, `.github/`, `.brokkr/`, lockfile or decision-status change. |

## Approach

### 1. Writer (shell, `holder` branch of the trial-helper script)

```sh
if [ "$EVAL_TEST_MATRIX_TRIAL_MODE" = holder ]; then
  sleep 1000 2>/dev/null &
  holder_tmp="$EVAL_TEST_MATRIX_HOLDER_PID.tmp.$$"
  printf '%s\n' "$!" > "$holder_tmp"
  touch "$EVAL_TEST_MATRIX_HOLDER_PID.entered"
  while [ ! -e "$EVAL_TEST_MATRIX_HOLDER_PID.release" ]; do :; done
  mv "$holder_tmp" "$EVAL_TEST_MATRIX_HOLDER_PID"
  exit 0
fi
```

`printf`'s redirection is closed before `touch` runs, so when the marker
appears the staged file is complete. A `mv` within one directory is
rename(2), which is atomic: `holder.pid` either does not exist or holds the
full line. The gate is unconditional in `holder` mode because the mode has
exactly one caller (`grep '"holder"'`). An opt-in env var would add a name
and a `MatrixProbeControls` field for no caller.

### 2. Reader helper (Rust, `mod tests`)

```rust
#[cfg(target_os = "linux")]
fn complete_pid_line(bytes: &[u8]) -> Option<i32> {
    let digits = bytes.strip_suffix(b"\n")?;
    if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) { return None; }
    std::str::from_utf8(digits).ok()?.parse::<i32>().ok().filter(|pid| *pid > 0)
}
```

This is a pure function and total over its input. The all-digits check comes
before `parse`, because `str::parse::<i32>` accepts a leading `+`. A
one-shot wrapper, for example
`fn published_pid(label: &str, path: &Path) -> i32`, reads the file once with
`std::fs::read` and panics with a message naming the label, the path and the
bytes when the read fails or the check returns `None`. It is called twice:
for the staged file and for `holder.pid`. It is never a wait condition.

### 3. Test body (`matrix_post_exit_inherited_pipe_is_bounded_and_holder_is_reaped_by_the_test`)

Replace the old `eventually(.. is_file)` and the `unwrap()` read with:

1. `eventually(format!("holder.pid staging at {entered} or publication at {holder}"), || entered.is_file() || holder_file.exists())`;
2. `assert!(!holder_file.exists(), "holder.pid at {path} was published without staging its complete PID first (no {entered})")`.
   This is the RED line on c95386e;
3. list the tempdir and keep the entries whose names start with
   `holder.pid.tmp.`. Assert exactly one, then
   `staged = published_pid("staged holder.pid", &that_entry)`;
4. write `holder.pid.release`;
5. `eventually(format!("holder.pid published at {path}"), || holder_file.is_file())`;
6. `holder_pid = published_pid("holder.pid", &holder_file)`, then assert
   `holder_pid == staged`, then assert zero `holder.pid.tmp.*` entries remain;
7. leave everything after that unchanged, from
   `matrix_identity(holder_pid).expect("holder identity")` on.

### 4. New inline test `holder_pid_line_is_accepted_only_when_complete`

A table over the inputs in spec criterion 7, asserting the exact
`Option<i32>` for each input. It has no file I/O, no clock and no fixture.

### Teardown if the test panics while the gate holds

`Drop` runs `record_descendants()` while the trial shell and the `sleep` are
both live descendants of the probe, so both become owned. It then SIGTERMs
the probe, and `cleanup_owned()` escalates SIGTERM, then SIGKILL, over the
owned groups, each step bounded at 5 s. The spinning shell is therefore
reaped within bounded teardown, and `Drop` needs no change. The
implementer confirms this by reading `Drop`; no new code is required.

## Files touched

- `evals/src/main.rs`: the script's holder branch, the helper and wrapper,
  the test body, and the new table test.
- `specs/holder-pid-publication/*`: this design.

## Research and rejected alternatives

- **Reader retry until it parses** (offered by the issue; robustness D2 as a
  wait). Rejected: it masks the writer's race and any future regression of
  it.
- **Writer fix with no proof** (`mv` only). Rejected: no deterministic RED.
- **Wait only for `.entered`** (simplicity). Superseded: on old code it is a
  20 s timeout that names the seam. The either-or wait fails at once and
  names the defect.
- **An impersonation RED of the predicate** (robustness). Rejected as the
  RED of record because it never runs the old code. The table test stays as
  the reader's unit proof.
- **A trap that kills the holder.** Refused: it would destroy the
  descriptor-inheritance property the test exists for.
- **Longer timeouts, sleeps, test retries, load loops.** Refused by the task
  and by 0020 §5.

## Data model and contracts

`holder.pid` is a test-fixture file. Its contract becomes: if it exists, it
holds exactly one `\n`-terminated decimal PID. `holder.pid.entered` and
`holder.pid.release` form the gate. The temporary file is
`holder.pid.tmp.<shell pid>`. No public or wire contract changes.

## Quickstart (implementer's RED/GREEN)

```bash
cargo build --locked --bin swarmail
BIN=$(cargo build --locked --bin swarmail --message-format=json | jq -r 'select(.executable!=null).executable')
# RED: new test body against c95386e's script. Stash the shell hunk only, or
# check out the script text from c95386e into a WIP commit.
(cd evals && SWARMAIL_BIN=$BIN cargo test --locked matrix_post_exit_inherited_pipe -- --exact --nocapture)  # expect: "published without staging"
# GREEN: with the fix
(cd evals && SWARMAIL_BIN=$BIN cargo test --locked matrix_post_exit_inherited_pipe)
(cd evals && cargo test --locked holder_pid_line_is_accepted_only_when_complete)
```

Record the RED output in the delivery notes. No load proof is needed or
planned. If anyone runs an optional sanity repetition, it must be sequential,
the one filtered test only, at most 10 runs within 10 minutes, and its load
must be reported.

## Risks

- **The gate couples `holder` mode to its single test.** Mitigation: the mode
  has one caller. If a second caller appears, an opt-in env var is a small
  change at that time.
- **A busy spin while the gate holds.** Mitigation: the test releases the
  gate right after its checks, which takes milliseconds. This is the same
  spin the pre-trap gate already uses.
- **A panic while the gate holds leaves a spinning shell.** Mitigation:
  `Drop`'s `record_descendants()` and the bounded `cleanup_owned()` SIGKILL
  path reap it (see Teardown above).
- **The RED depends on the old script never creating `.entered`.** That is
  true by construction on c95386e. If the old trial never ran, the
  either-or wait would time out, which is still RED and names both paths.
- **The strict reader rejects a future format change.** This is intended. It
  fails loudly, naming the file, the path and the bytes.
- **A stale `holder.pid.tmp.*` left by a killed shell.** It lives in the
  per-test tempdir and is removed with it. The exact-count check runs only in
  the probe's own fresh directory.
- **An orphaned `sleep 1000` if the probe dies before publishing.** It is
  bounded and cheap, and the descendant recording in `Drop` is the net.
  This is an accepted loss, unchanged from before.
- **A rename across filesystems would not be atomic.** Mitigation: the
  temporary name is derived from the final path, so both are always in the
  same directory.
