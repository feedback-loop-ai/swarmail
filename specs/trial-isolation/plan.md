# Trial isolation implementation plan — the boundary is bubblewrap

Implements [spec.md](spec.md) as revised 2026-11-01 for the third time,
after the first live proof on the host failed in the controller; semantic
activation is governed by proposed
[0115](../../docs/decisions/0115-trial-isolation.md), its *Revision
2026-11-01* section and the *After the first review* and *After the live
proof* subsections. The boundary built at bdc504f and revised at f1a30a7
(argv with `--as-pid-1`, refusal set, external-agent precondition, void
rule, observer, Linux gating) stands untouched: the live run showed it
works. This revision rebuilds the host side of it — how the controller
drives bwrap, copies the row and exits — and the test side that holds it.
Concurrency stays off; no row version changes; no production path reads a
proof record. The earlier plans stay in git history; this file supersedes
them.

## Technical context

A standalone Rust 2024 workspace, `evals/`, outside the measured root crate
and its coverage denominator (`evals/Cargo.toml`, decision 0055). Tokio
`=1.53.2` with `rt-multi-thread` (which carries `rt`, so a current-thread
runtime needs no feature change — the unit tests already build one),
`process`, `signal`, `time`, `io-util`; `libc =0.2.190` for `close_range`,
`kill` and `geteuid`. **No new dependency, workspace member, feature or
license entry.** bubblewrap is a host prerequisite executed as a separate
process.

What the host showed on 2026-11-01 (the dispatcher's strace, summarized in
the task; seats cannot read the trace file): with `/usr/bin/bwrap` 0.11.1
under `kernel.apparmor_restrict_unprivileged_userns=1` and the
`bwrap-userns-restrict` profile, the probe passes, bwrap accepts the full
argv, the runner is PID 1 of the namespace and starts Swarmail and the
fixture inside; the runner writes a complete 2502-byte row to its fd 1 and
exits 1; bwrap exits 1; the controller prints `isolated_trial_no_row` about
10 s later and then sleeps on a futex forever with three threads and no
children. What the seat shows: bwrap refuses `ENOSPC` (already inside a
user namespace), so only the refusal paths and the stand-in tests run here.
The seat has no Apple toolchain; the macOS compile is CI-only evidence, as
before.

There is no plan template in this checkout; the spec-kit sections (research,
data model, contracts, quickstart) are inline. The design commit is exactly
this spec/plan/tasks and the revised proposed decision. Implementation and
documentation edits belong to later commits, not the chief's HEAD.

## Constitution check

Preserve accepted 0001–0005 (the spec says how each survives the boundary;
nothing in this revision touches the sandbox or root `src/`), 0055's
app-state authority and real-subprocess path, 0071 trap precedence, 0073
matrix cancellation, 0075 safe diagnostics and finite categories (no new row
reason; the envelope's stderr line is a fixed diagnostic, not a row field),
and all of 0076 (admission is still held through reaping; the envelope's
expiry is the one case where it is released after a bounded reap instead).
Accepted 0045 §8: this is compatible repair of a proposed design, not
activation. Root exact coverage stays 100% with zero missed and no
`coverage(off)`; `git diff a4f552d --stat -- src/` must stay empty. Apply
0020: every new test is a real deadline on a child, chaos at the poles (a
stand-in that always hangs, one that always exits), no sleep as
synchronization, per-test tempdirs. Commit unsigned under 0056; no push,
merge or publication. `.github/`, `scripts/`, `.brokkr/`, `deny.toml` and
both lockfiles are untouched.

## Research and decisions — each change and the finding it answers

**P1 and P2 — the std locks, the blocking writers and the runtime drop.**
`main.rs` builds the isolated arm's `match` on
`prepare_trial(&cli.command, …, &mut std::io::stdout().lock(), &mut
std::io::stderr().lock())`. Rust keeps a `match` scrutinee's temporaries
alive to the end of the `match`, so the main thread holds both reentrant
std locks across `rt.block_on(run_isolated(..))` and the `return` that
drops `rt`. `isolation::pump` copies every chunk through
`tokio::task::spawn_blocking(write_through)`, and `write_through` takes
`std::io::stdout().lock()` on a blocking-pool thread, which is not the
owner, so it parks on the futex with the row in hand. Neither pump reaches
EOF; `supervise`'s two `DRAIN_BOUND` joins expire (5 s + 5 s: the 10 s gap);
`bytes` is `0`; the verdict is `NoRow`, which `conclude` prints from the
main thread through the reentrant lock it owns, which is why the
harness-error row appears and the runner's row never does. Then `return`
drops the multi-thread runtime **before** the scrutinee temporaries; the
runtime's drop joins the blocking pool with no timeout; its two workers are
parked on the locks `main` still holds. Main plus two writers, all in
`futex_do_wait`, no children: the trace. The pipe, the `close_range(3..)`
sweep and `--new-session` (`setsid` only) are correct and are not changed.
The unit tests passed because their private `run` built a current-thread
runtime and held no lock: production and test ran different code.

Three changes close it, all in the controller: (a) `prepare_trial`'s result
is bound by `let` before it is matched, so the guards die at the end of
that statement; (b) one production entry, `isolation::run(request, inner,
bwrap, bounds) -> Result<Launched, Refusal>`, builds a
`new_current_thread().enable_all()` runtime, `block_on(launch(..))`s and
ends with `shutdown_timeout(bounds.drain)`; `main` calls it and passes the
result to the unchanged `conclude`, and the unit tests' private `run` is
deleted in favour of it; (c) `pump` calls `write_through` directly, so on
the current-thread runtime every write happens on the main thread, which
may take the std locks, and `spawn_blocking` and the per-chunk `to_vec`
go. Robustness's owned-sink plumbing is rejected (the pumps are spawned
tasks and would need `'static` sinks) and so is `std::process::exit` (a
bounded shutdown leaves nothing to wait on, and AC-S10 must be able to
catch a future blocking destructor rather than have it hidden). The rule
that survives, for review: no `Stdout`/`Stderr` guard outlives its
statement or is held across `.await` or `block_on`; AC-S11 is its proof.

**P6 — the envelope.** `supervise` awaited `child.wait()` with no bound.
The operator's direction bounds every wait, so bwrap's wait gets an
envelope equal to the matrix's: the trial's deadline (`--deadline-ms`, else
`tasks::find(task).deadline_ms`) plus the 300 000 ms grace that
`run_child_assigned` names, lifted into one `pub const` both use. The bound
is a parameter (`Bounds { envelope, drain }`, filled by `main` from the
request and by the stand-in tests in milliseconds) because a 300 s bound
cannot be proven by a stand-in without sleeping. On expiry: `start_kill()`
bwrap (`--die-with-parent` `SIGKILL`s PID 1; the kernel empties the
namespace), `timeout(drain, child.wait())`, one fixed stderr line, then the
existing outcome match. No new row reason. Robustness's floor/ceiling
formula is rejected as a second number set; its process-group `SIGKILL` is
rejected because `--new-session` puts the sandbox in its own session, so
`kill(-pgid)` reaches only bwrap, which `start_kill` already does.

**P5 — the read-side count.** `pump` gets an `Arc<AtomicU64>` it adds to as
bytes arrive; `supervise` reads it after the drains, so an expired drain
never collapses a delivered row to `NoRow`. The stand-in case is a bwrap
that writes a row and exits 0 while a backgrounded grandchild keeps its
stdout open: `Exited(0)`, never `NoRow`.

**P3 — the bounded test-side runner and the two suites.** The suite hung
because `boundary_unavailable()` ran a real isolated null trial through an
unbounded `.output()` while holding `trial_scope()`, and every scoped test
queued behind it. One helper, `bounded(command, bound) -> ChildGuard`,
spawns with stdout and stderr on files in the guard's tempdir (no reader
thread, no pipe to fill), and `ChildGuard::wait_with_output` polls
`try_wait` to the deadline, then `SIGKILL`s and panics by name with the
child's output. Every isolated CLI child in `trial_isolation.rs` and in
`common` goes through it, ignored or not. The bound is 90 s: 5 s version,
10 s probe, a 30 s deadline, the runner's own teardown inside, two 5 s
drains and slack. Always-on tests that need the boundary print the named
SKIP and return where it is unavailable; the ignored proofs print it and
fail (`require_boundary` is unchanged in meaning). Two tests whose subject
is the absent boundary print `SKIP trial-isolation: boundary available` and
return on a host where it works. The null probe trial carries
`--deadline-ms 30000` so the arithmetic above is true of it too.

**What proves it.** Three tests, from both positions. (1) AC-S10, always
on, Linux: `an_isolated_null_trial_hands_over_its_row_and_exits` runs the
real CLI through the bounded runner; on the host it requires the null
agent's own row and exit, on the seat it skips by name; on the pre-fix
controller it fails at 90 s instead of hanging. (2) AC-S11, the seat red
test in `isolation.rs`: a re-executed child holds `stdout().lock()` on its
main thread as `main` did and runs `isolation::run` against the row-and-
exit-1 stand-in; the parent polls to a deadline and requires
`Ok(Exited(1))`. Red against the pre-fix `pump`/runtime, which the
implementer shows before fixing (the commit body or the implement notes).
(3) AC-S2's bounds cases through `isolation::run` with millisecond bounds:
row-then-hang (`Exited(2)`, the stand-in dead, the envelope line on
stderr), nothing-then-hang (`NoRow`), row-then-grandchild-holds-stdout
(`Exited(0)`).

**P4 — the sentences.** The spec's "under five minutes" and "has not yet
run" are replaced by what ran, the per-child bound, the per-test worst case
and the dispatcher's 20-minute limit; the floor's evidence sentence says
the launch is confirmed on 0.11.1 and the live criteria are still open.
`evals/README.md` and `evals/run-plan.md` follow.

**Rejected, with the cost.** An argv change of any kind (the golden test
stays byte-identical; the boundary is not implicated); a second row channel
(`--json-status-fd`, an inherited fd, a socketpair, a row file: the pipe
delivered every byte); a watchdog thread or `shutdown_background`; a new
harness-error name for the expired envelope (an expired envelope is a
missing row, and `isolated_trial_no_row` already says that); a `BWRAP`
override to aim the real CLI at a fake (0115 forbids the lever); any
dependency, feature or `src/` change; un-ignoring the live proofs; any
`.github/` change (the CI boundary job stays the operator's follow-up).

## Project structure and ownership

| File / surface | Planned responsibility |
| --- | --- |
| `evals/src/isolation.rs` | `pub struct Bounds { envelope: Duration, drain: Duration }` with `Bounds::for_trial(deadline_ms)` = deadline + `TRIAL_GRACE_MS` and `DRAIN_BOUND`; `pub const TRIAL_GRACE_MS: u64 = 300_000` (the matrix's literal, now shared). `pub fn run(request, inner, bwrap, bounds) -> Result<Launched, Refusal>`: current-thread runtime, `block_on(launch)`, `shutdown_timeout(bounds.drain)`. `launch` takes `bounds`. `pump(from, sink, counter)` writes through directly and adds to the shared counter; `spawn_blocking` deleted. `supervise`: `tokio::time::timeout(bounds.envelope, child.wait())` inside the `select!` loop; on expiry `start_kill`, bounded reap, the fixed stderr line; drains bounded by `bounds.drain`; verdict from the read-side count. Doc comments say the pumps write on the driving thread and why. Tests: the private `run` helper deleted in favour of `isolation::run` with test bounds; the three bounds cases; AC-S11's re-exec red test and its child half; `interrupt_child` calls `isolation::run`. |
| `evals/src/main.rs` | `let prepared = prepare_trial(..)` bound before the `match`; the isolated arm calls `isolation::run(&request, &inner, Path::new(isolation::BWRAP), isolation::Bounds::for_trial(deadline))` with the deadline resolved as the matrix resolves it, then `conclude`; no runtime is built in `main` for the isolated arm. `run_child_assigned` uses `isolation::TRIAL_GRACE_MS` in place of its literal. `through_the_controller` (the entry tests) calls the same production path with test bounds. |
| `evals/src/runner.rs`, `tasks.rs` | Unchanged. `tasks::find(task).deadline_ms` is the fallback deadline the envelope uses, as in the matrix. |
| `evals/tests/common/mod.rs` | `ChildGuard` gains its output files (a `TempDir` it owns, `stdout`/`stderr` paths) and a `deadline`; `pub fn bounded(command, bound) -> ChildGuard` is the one spawn path; `wait_with_output` polls `try_wait` to the deadline, builds the `Output` from the files, and on expiry `SIGKILL`s, reaps and panics `controller outlived its row after <bound>` with both files' contents; `Drop` unchanged in meaning. `pub const ISOLATED_CLI_BOUND: Duration = 90 s`. `boundary_unavailable()` runs the null probe with `--deadline-ms 30000` through `bounded`; `require_boundary` unchanged; `trial()` routes through `bounded` when its args carry `--isolated`; `launch_victim` through `bounded`. A test of `bounded` itself with a `sleep` child under 200 ms. |
| `evals/tests/trial_isolation.rs` | Every `trial --isolated` child through `bounded` (`isolated_helper_with`, `idle_trial`, `idle_victim`, the hostile proof's trial B, the oracle trial). New always-on `an_isolated_null_trial_hands_over_its_row_and_exits` (AC-S10). The two absent-boundary tests print `SKIP trial-isolation: boundary available` and return where the boundary works; the always-on tests that need it print the named SKIP and return where it does not. Module doc: the two rules side by side, the 90 s bound, the 20-minute limit. |
| `evals/README.md`, `evals/run-plan.md` | The dated #221 section: the proof ran once on 2026-11-01 and failed in the controller; the bounded-controller contract; the two-suite rule; the 90 s bound and the 20-minute acceptance limit. |
| `docs/decisions/0115-trial-isolation.md` | The proposed record with its *After the live proof* subsection, committed by the chief with this design. Index row unchanged. |
| `specs/trial-isolation/{spec,plan,tasks}.md` | This design. |

No root `src/`, `Cargo`, gate-script, policy, `.github/` or review-routing
change. Do not refactor the matrix-cancellation (0073) or child-environment
(0075/0076 §4) blocks incidentally: `run_child_assigned` changes one literal
to the shared constant and nothing else.

## Data model and contracts

No wire contract changes. `TrialResult` stays schema 2 with required integer
`concurrency: 1`. The `execution`/`boundary` object of 0115 §4 is not added.

The boundary's internal contract, as revised:

- **The refusal vocabulary** is closed and unchanged: the seven precondition
  names, `boundary_unavailable:{platform,bwrap_absent,bwrap_version,bwrap_probe}`,
  `isolated_trial_no_row`, `isolated_trial_interrupted`. An expired envelope
  is reported through those (`isolated_trial_no_row`, or exit 2 beside the
  runner's own row) plus one fixed stderr line that carries no bwrap text
  and no number the row could be confused with.
- **The bounds**: `Bounds { envelope, drain }`; production `envelope` =
  trial deadline + `TRIAL_GRACE_MS`, `drain` = `DRAIN_BOUND` (5 s); the
  version and probe bounds stay the constants they are.
- **The launch judgement**: unchanged in shape; the byte count is read-side.
- **The entry**: `isolation::run` is the only way production drives
  `launch`; `launch` stays `pub` for the test that needs the future itself
  (none today) and for `launch_victim`'s argv builder, which does not call
  it.
- **The test runner**: `bounded(command, bound) -> ChildGuard`; output on
  files; `wait_with_output` passes only by exit.

## Quickstart and validation

From the repository root, with cached dependencies:

```bash
cargo build --locked --bin swarmail
cd evals
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
SWARMAIL_BIN="$PWD/../target/debug/swarmail" cargo test --locked
```

The last command runs AC-S1 through AC-S12 on Linux, with the live proofs
shown as `ignored`; on a host with a working bwrap it must now finish in
bounded time with AC-S10 green, and in the seat with AC-S10 skipped by
name. The live proof is the separate `--ignored` command in the spec, run
by the dispatcher on this host after the run under a hard 20-minute limit.
Then, from the root, every command in `docs/house-rules.md` § Gates runs
locked. CI owns the macOS compile, chart, Docker smoke, RustSec, tag-rate
and port-80 proofs; no CI change is made here.

Design-stage validation is structural (`design-check.sh`): exactly one
touched `specs/<slug>/`, its three files committed, non-empty and clean, an
acceptance heading in spec.md, a checkbox in tasks.md, a risks heading in
plan.md, and the declared `Decision: 0115` resolving to one committed, clean,
proposed record the spec links. The chief's result is `drafted`.

## Risks

| Risk | Mitigation / remaining cost |
| --- | --- |
| The diagnosis is read from the code and the trace summary, not reproduced by this seat | It explains every observation (the 10 s gap, three futex threads, no children, green unit tests). AC-S11 reproduces it deterministically without bwrap and must be shown red before the fix; AC-S10 and the dispatcher's live proof are the final word. If the proof still shows `isolated_trial_no_row`, the bounded runner fails it fast and by name, and the next step is evidence, not another layer. |
| A consumer that stops reading the controller's stdout blocks the copy on the reactor thread | Accepted and named: the unisolated runner has the same contract, the row is under one pipe buffer, the test side reads to files. |
| The envelope (deadline + 300 s) holds the host admission for minutes on a wedged runner | Accepted: one shared number with the matrix; the test side bounds its children at 90 s; the matrix never runs isolated. A `D`-state process defeats any userspace bound; the controller still exits, and the orphan is the kernel's. |
| The 90 s child bound is too tight on a slow host | A false timeout is red by name with the child's output attached; the bound is raised with that evidence, never removed. |
| The worst-case live proof (every child hangs) exceeds 20 minutes | Per test at most five children (7.5 min); the earlier tests have already failed by name when the dispatcher kills the run, so the failure is attributable. A healthy run is far shorter; its duration is recorded in tasks.md by the first passing run. |
| `shutdown_timeout` abandons a thread that is still blocked | Only possible if a future change reintroduces a cross-thread lock; AC-S11 and AC-S10 turn that red. Returning from `main` still exits the process. |
| The always-on handoff test runs a real isolated trial in CI | On hosted ubuntu bwrap is absent or below the floor, so it skips by name; where it is present and works, it is bounded at 90 s. |
| `trial_scope()` serializes the always-on tests behind one slow isolated child | Bounded now at 90 s per child; the scope is released on panic as the guard drops. |
| Two always-on tests change meaning on the host (they assert the absent boundary) | They print `SKIP trial-isolation: boundary available` and return; their subject is proven in the seat and on hosted CI, where bwrap refuses. |
| The matrix's envelope literal is shared and someone changes it for one caller | One `pub const` with a doc comment naming both callers; the matrix's envelope tests still pass unchanged. |
| bwrap 0.11.1 rejects `--as-pid-1` beside `--die-with-parent` (R1) | Closed by the live run: bwrap accepted the full argv and the runner was PID 1 inside. |
| `--isolated` falls back to unisolated under pressure | No fallback path exists; the bwrap path and the bounds are parameters with no override; AC-S2 proves every refusal launches no agent. |
| A test prints SKIP and shows `ok` in the live proof | The ignored proofs panic after the SKIP line (`require_boundary`); only the always-on tests return. |
| Coverage: `isolation.rs` is outside the root report | AC-S2/S3/S4/S11 cover every pure part, the controller's refusal arms and the bounds; AC-S10 covers the delivered-row arm on any host with bwrap; the live proofs cover the rest. |
| Gates or reviewers unavailable | Preserve failed receipts; no substitution or success claim. The run must not report complete while AC-L1..L4 and AC-O1 are open. |
