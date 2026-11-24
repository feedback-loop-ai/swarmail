# Implementation ledger: eval trial reaping

Part of #408. Recorded 2026-11-24 on branch `fix/eval-test-leak-408`, Linux
x86_64. Every command ran in the seat's box, which has its own PID namespace,
so each process census below shares one shell invocation with the run it
checks. Candidate: `cargo build --locked --bin swarmail --message-format=json`
reported `target/debug/swarmail`. `SWARMAIL_BIN` was set to that path for
every evals command.

Census used at each boundary:

```bash
ps -eo pid,ppid,pgid,sid,stat,args |
  awk '$6 ~ /swarmail$|swarmail-evals$/ && ($7=="serve"||$7=="fixture"||$7=="trial")'
# plus /proc/<pid>/cwd, field 22 of /proc/<pid>/stat (start time), and
# ls -d /tmp/swarmail-eval-*
```

## Reproduction before the fix (HEAD 43004f6f)

| Command (from `evals/`) | Result | Survivors after |
|---|---|---|
| `cargo test --locked --test baseline` | 8 passed, 37.48 s | none |
| `cargo test --locked --test paid_baseline` | 5 passed, 96.66 s | one `serve` and one `fixture` |
| `cargo test --locked --test paid_baseline one_command_gates_on_controls_defers_stale_logins_and_resumes_by_slot -- --exact` | 1 passed, 96.28 s | the same pair |

Before each run the census was empty. After the isolated run:

```
8103 ppid 1 pgid 8103 Sl  target/debug/swarmail serve --smtp-listen 127.0.0.1:0 --pop3-listen 127.0.0.1:0 --http-listen 127.0.0.1:0 --max-per-inbox 100000 --route-domain swarmail.test
     cwd=/tmp/swarmail-eval-G725qh/swarmail start=267647552
8234 ppid 1 pgid 8234 Sl  evals/target/debug/swarmail-evals fixture --smtp 127.0.0.1:39271 --task newsletter-findings --trial-id … --swarmail-http …
     cwd=/tmp/swarmail-eval-G725qh/fixture start=267647566
/tmp/swarmail-eval-G725qh: agent-0/{cache,config,data,home,state,tmp,work} fixture/ swarmail/
```

This matches the dispatcher's report exactly: the same pair, orphaned to init,
from one trial directory holding `agent-0/`. Each pair was cleaned up when the
box's PID namespace closed.

**Exact test.** The leak comes from
`paid_baseline::one_command_gates_on_controls_defers_stale_logins_and_resumes_by_slot`,
which #402 (dd4f67a0) added.

**Lifecycle boundary and mechanism.** The test stops two processes with
SIGKILL. The leak comes from the second kill, which hits the trial worker
(`swarmail-evals trial`) during a held `dsh dashscope` launch. `spawn_extended`
in `evals/src/runner.rs` starts `serve` and `fixture` with `process_group(0)`.
Each one leads its own group and has no parent-death signal (only the bwrap'd
agent has `--die-with-parent`). A SIGKILLed worker never reaches
`stop_servers`, and its handles never drop, so both servers reparent to init.
The coordinator's handled refusal (`has no proof of owned cleanup`) is correct
under 0137, which says forced death can leave descendants. Nothing in the test
ended them.

The first kill, which hits the coordinator, does not leak processes. The
orphaned worker finishes on its own and stops its servers. The fixed test now
proves this with `wait_clear`, which passes. That kill does abandon the
coordinator's `/tmp/swarmail-eval-matrix-*` runtime root for the worker. Once
the process leak was fixed, that root was the only thing left behind (seen
after the first green run of the fix). It is a likely source of the older
stray `swarmail-eval-*` directories, and the test now removes it.

No handled runner exit leaked. `baseline` ran its real trials, which are the
controls of the hard and expert tiers, and left no survivors. The isolated
named test leaked only through the forced worker death. Production code is
unchanged.

## Fix (test-owned, per the spec)

- `evals/tests/common/session.rs` (new): `isolate` starts a baseline command
  as the leader of a fresh session. `OwnedSession` records the leader's
  identity (pid and start time). Its members are every process in that session
  that started no earlier than the leader, plus their descendants. A process
  table it cannot read, or a session id reused by another leader, counts as
  missing ownership evidence and fails the check.
  - `assert_clear` reports each survivor (pid, state, start, PPid, NSpid, cwd,
    cmdline) before ending it, then fails.
  - `wait_clear` waits for natural exit, observed through pidfds, within one
    deadline.
  - `terminate` sends SIGKILL through a pidfd and re-verifies start time and
    session after opening it. It observes each exit within one shared 5 s
    deadline and rescans for late members.
  - `live_trial` checks the held launch: one `trial` worker under the leader,
    with the candidate's `serve` and the harness's `fixture` as its children.
    Their cwds must be sibling `swarmail/` and `fixture/` directories of one
    `swarmail-eval-*` root directly under `SHARED_TMP`. Paths are resolved with
    `artifacts::resolve_root` before `safe_root`. The worker's
    `swarmail-eval-matrix-*` runtime root is read from its HOME.
  - Drop works as the backstop: a live member fails the test. When the test is
    already panicking, Drop only rescues, so the first failure keeps its
    message.
- `evals/tests/common/paid.rs`: `Running::start` and `run` start their command
  in an owned session. `Running::finish` and `run` assert the session is clear
  once the command exits. `finish_owned` hands the session back for the two
  forced deaths.
- `paid_baseline.rs`, coordinator death: `live_trial` runs before the kill.
  Then the existing busy and refusal checks run, the hold is released and
  admission goes idle. After that, `wait_clear` observes the worker family's
  exit, the test asserts the trial scratch is gone and removes the abandoned
  runtime root. Confirmation comes after all of this. Score bytes and slot
  counts are unchanged.
- `paid_baseline.rs`, worker death: `live_trial` runs, then `kill_worker`
  kills the pinned identity. This replaces the private `trial_worker` /proc
  walk. The no-closure check, the unconfirmed refusal and the zero-launch
  checks stay as they were. Then `terminate()` runs, then the separate
  `assert_clear()`, then `dispose_scratch` (which asserts clear again).
  Confirmed resume runs only after that, and the retained partial attempt is
  untouched.
- `survivors()` now also prints start time and cwd. Its existing proof in
  `trial_isolation.rs` asserts both.

## Guard fails with the fix reverted

Mutation: in `paid_baseline.rs`, `family.terminate();` was commented out (only
the worker-death cleanup action). Command: the isolated `--exact` run above.
Result: exit status 101 after 34.48 s, panicking at
`tests/common/session.rs:96`:

```
baseline: owned processes outlived it
pid 8113 … PPid 1 … cwd /tmp/swarmail-eval-XwsFuW/swarmail cmdline …/target/debug/swarmail serve --smtp-listen 127.0.0.1:0 …
pid 8244 … PPid 1 … cwd /tmp/swarmail-eval-XwsFuW/fixture cmdline …/swarmail-evals fixture --smtp 127.0.0.1:40385 --task newsletter-findings …
```

On that failing run the census after the run was empty, because the rescue
ended both servers. Only the scratch directory of the failing run remained,
since disposal is deliberately left to the passing path. The line was then
restored with `git checkout` and the test passed again.

## After the fix

| Command (from `evals/`) | Result | Survivors / `swarmail-eval-*` after |
|---|---|---|
| isolated `--exact` named test | 1 passed, 95.91 s | none / none |
| `cargo test --locked --test baseline` | 8 passed, 37.62 s (37.48 s before) | none |
| `cargo test --locked --test paid_baseline` | 5 passed, 95.72 s (96.66 s before) | none |
| `cargo test --locked --no-fail-fast` (whole evals suite) | 622 passed, 0 failed, 7 ignored, 1457 s | none / 0 (0 before) |

The census was empty before every run. The added wall time is within noise:
in-binary times moved by −0.9 s and +0.1 s with warm builds. The new checks
are one /proc scan per finished baseline command, plus pidfd waits that return
when the processes exit.

An earlier full-suite run, made before the second commit, also left no
survivors and no `swarmail-eval-*` directories. Its single failure was the
`survivors` format assertion, which the second commit fixes.

## House gates

All clean on Linux unless listed otherwise:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo test --workspace --locked`: 2218 passed, 0 failed
- `cargo build --locked --bin swarmail`
- evals: fmt, clippy `--all-targets --locked -D warnings`, and the tests above
- `scripts/coverage-gate.sh`: exact 100/100/100
- `scripts/coverage-gate.sh selftest`: pass
- `scripts/publish-guard-test.sh`: OK
- `scripts/seal-branch-test.sh`: 7 cases and 10 mutants passed
- `cargo deny check licenses`: licenses ok

Not run here:

- `brokkr compile` over `.brokkr/recipes/*`: the pinned brokkr 0.12.0 binary
  is not installed in this box, and it cannot be fetched offline. This change
  touches no `.brokkr/` path. The verify seat and CI run it.
- macOS: `cargo check --target aarch64-apple-darwin` cannot build `ring`'s C
  code here (no clang). The cfg paths were reviewed by hand. Every Linux-only
  item carries its own `#[cfg(target_os = "linux")]`. Cross-platform methods
  hold their cfg inside the body, and the non-Linux `live_trial`,
  `kill_worker`, `dispose_scratch` and `dispose_runtime` are `unreachable!`
  stubs, like the old `trial_worker`, because their only caller is ignored on
  macOS. CI's macOS job is the proof of compilation and execution.

No held path (`scripts/`, `.github/`, `.brokkr/`, locks, `deny.toml`) was
changed, and no dependency was added. libc's `SYS_pidfd_open` and
`SYS_pidfd_send_signal` are in the locked libc 0.2.190.
