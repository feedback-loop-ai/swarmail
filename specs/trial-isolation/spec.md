# Isolate concurrent eval trials — the boundary is bubblewrap

Date: 2026-11-01 (third revision, after the first live proof on the host
failed; the 2026-10-31 design commit and the two earlier 2026-11-01
revisions stay in history). Issue #221 (split from #199), Epic #192.
Branch `repair/issue221-trial-isolation` (PR #308), main a4f552d merged in.
Lane: `claude-careful` (accepted 0098/0099; residual rework under accepted
0102 and its spec-defect addendum).
Operator direction, 2026-11-01: **fix it; merge only once the live proof
passes on the host.** The boundary stays bubblewrap exactly as the second
revision drew it; what this revision changes is the controller on the host
side of it and the tests that hold it.

Decision: 0115 — [Trial separation and observed concurrency](../../docs/decisions/0115-trial-isolation.md)
(proposed; its dated *Revision 2026-11-01* section, the *After the first
review* subsection and the *After the live proof* subsection below it govern
where they disagree with the 2026-10-24 ruling and the 2026-10-31
integration).

## Feature and rationale

A trial must not read another trial's prompt, mail, process data or HOME, and
must not use another trial's MCP or fixture endpoints to alter its business
state. Today that holds only by cooperation: accepted
[0076](../../docs/decisions/0076-sequential-eval-trials.md) admits one trial
per uid, gives every child a fresh `0700` HOME under `/tmp`, and documents
that trials are sequential. Both trials are the same uid, discretionary
access control is uid-based, and a process that outlives its trial can watch
the next one even at concurrency 1. Sequential is an admission policy; it is
not isolation, and this spec never lets the two be confused.

The 2026-10-31 design built the boundary itself: a `clone` into fresh
user/mount/PID/network/IPC namespaces, `uid_map`, `pivot_root`, and a
sixteen-step init. That path is dead on this host and on most Ubuntu-family
hosts: `kernel.apparmor_restrict_unprivileged_userns=1` makes the live proof
refuse `boundary_unavailable:identity_map:EACCES`, and the code never
executed anywhere. bubblewrap (`bwrap`) traverses that restriction through
its AppArmor profile (`bwrap-userns-restrict`): the dispatcher's out-of-seat
probe on this host, with `--unshare-user --unshare-pid --unshare-net
--unshare-ipc --die-with-parent`, gave uid 1000, an empty private `/tmp`,
loopback only and pid 2; on this host the dispatcher also observed that
processes inside cannot create further user namespaces (an observation about
this host's profile, not a property of the argv). This design keeps the
boundary's **contract** and replaces its **mechanism**: the trial's
controller invokes `/usr/bin/bwrap` with a fixed argv, and the kernel and
bwrap own the namespaces. The raw-syscall worker is deleted (done at
bdc504f).

What still holds, unchanged from the design on record:

- the whole trial (runner worker, Swarmail, fixture, every agent) sits inside
  one boundary;
- the trial gets a private `/tmp`, HOME and `/proc`;
- escape descriptors are closed;
- an unavailable boundary refuses by name before any server or agent exists
  (`boundary_unavailable:<step>`), with no unisolated fallback;
- trials stay sequential (accepted 0076) until the live proof passes;
  concurrency comes back only behind it and behind the operator's acceptance
  of 0115.

Concurrency stays off. The public `--concurrency` refusal, admission and every
0076 row rule are untouched. No production path reads a "proof record" to
unlock overlap (accepted 0045 §8 while 0115 is proposed).

## What the live proof found, and where each finding is answered

The dispatcher ran the proof command on this host on 2026-11-01 (Linux,
`/usr/bin/bwrap` 0.11.1, `kernel.apparmor_restrict_unprivileged_userns=1`
with the `bwrap-userns-restrict` profile). **The boundary worked:** the probe
ran, bwrap accepted `--as-pid-1` beside `--die-with-parent`, and the inner
runner started Swarmail and the fixture inside the namespace as its PID 1.
The isolated trial still failed end to end, in the controller, and the
dispatcher's strace summary (kept outside the repository; its summary in the
task is authoritative for seats) is the evidence. Each finding below has a
task in [tasks.md](tasks.md) and a proving test, and [plan.md](plan.md)
says which finding each change answers.

| # | Finding (source) | Root cause, read against the code | Answered by |
| --- | --- | --- | --- |
| P1 | **Row handoff fails.** The inner runner wrote a complete 2502-byte row to its fd 1 at +2.8 s and exited 1 (correct: the null agent fails); bwrap exited 1 at once; the controller printed `isolated_trial_no_row` about 10 s later (dispatcher) | `main` passes `&mut std::io::stdout().lock()` and `&mut std::io::stderr().lock()` inside the `match` scrutinee; those temporaries live to the end of the `match`, so the main thread holds both process-wide reentrant locks across `rt.block_on(run_isolated(..))`. `pump` copies each chunk through `spawn_blocking(write_through)`, which takes `stdout().lock()` on a **blocking-pool thread**; that thread parks on the lock for good, so neither pump reaches EOF, each drain times out at `DRAIN_BOUND` (5 s + 5 s = the 10 s gap), the byte count collapses to 0 and the verdict is `NoRow`. The pipe, the sweep (it starts at fd 3) and `--new-session` (it only calls `setsid`) are correct | Required behaviour, *The controller*: a current-thread runtime shared with the tests, the pumps write from the main thread, no std lock outlives its statement; the read-side byte count; AC-S2, AC-S10, AC-S11 |
| P2 | **The controller never exits.** After its row it sleeps on a futex forever: three threads, all in `futex_do_wait`, no children (58 min in the proof, 3 h 16 min under the host gate, killed at 76 s under strace) | `return ExitCode::from(code)` drops the multi-thread runtime **before** the scrutinee temporaries; the runtime's drop joins the blocking pool without a timeout, and its two workers are parked on the locks the main thread still holds. That leaves main and the two writers: three threads, no children | *The controller*, step 7: every wait bounded, `shutdown_timeout`, a controller that has written its row returns from `main`; the bwrap envelope; AC-S2, AC-S10, AC-S11 |
| P3 | **The suites hang where bwrap works.** `cargo test --locked` in `evals/` never finishes on such a host: `boundary_unavailable()` runs a real `trial --isolated --agent null` through an unbounded `.output()` while holding the cross-process `trial_scope()`, and every other scoped test (`isolated_preconditions_refuse_through_the_real_cli`, the hostile probe, the observer, the leak) waits behind it; the verify seat stayed green only because bwrap refuses inside the seat (dispatcher) | The suite's boundedness depended on the code under test, and nothing in the gate inventory ever exercised a delivered row | AC-S12: one bounded, file-backed CLI runner for every isolated child; AC-S6's two-suite skip rule; AC-S10's always-on handoff test |
| P4 | **False sentences.** The spec said the whole proof command "completes in under five minutes on this host" and that the proof "has not yet run on any host" (positions) | Both were claims without a run behind them | The live-proof paragraph: what the one run showed, the per-child bound and the 20-minute acceptance limit; the floor's evidence sentence |
| P5 | **A timed-out drain reads as no row** (robustness position, latent) | `bytes` is the pump's return value, so a drain that expires after copying a row reports 0 | *The controller*, step 6: the count is kept on the read side as bytes arrive and survives an expired drain; AC-S2 |
| P6 | **bwrap's wait is unbounded** (robustness position; the operator's direction "every wait in the controller must be bounded") | `supervise` awaits `child.wait()` with no limit; a runner that ignores its deadline holds the host admission forever | *The controller*, step 6: the envelope; the second synthesis row *Supervisor bound* reversed below; AC-S2 |

## What the review returned, and where each finding is answered

The panel reviewed bdc504f and returned the run to design with
`spec_defect: true`. Every finding above low, every security finding, and
the ordinary findings the implementer must act on are answered here; each
has a task in [tasks.md](tasks.md) and a proving test, and
[plan.md](plan.md) says which finding each change answers.

| # | Finding (member, severity) | Answered by |
| --- | --- | --- |
| R1 | bwrap's default PID-1 reaper sits inside the boundary as a same-uid, dumpable process; `/proc/1/mem` and `/proc/1/fd` may reach the host `/proc` and sibling A; nothing checks it; ptrace policy is not a stated prerequisite (security, medium, **the spec defect**) | `--as-pid-1`: no reaper exists (Required behaviour, the argv); AC-L2's descriptor walk across every process inside; `/proc/1/mem` and `ptrace_scope` recorded as observations; 0115 *After the first review* |
| R2 | 0115 promises no paid driver can score a provider outage as `gave_up`, but only `--wrapper-profile` is refused; any external argv runs inside the no-network boundary (security, medium/low, contract) | the `isolation_external_agent_unsupported` precondition (Required behaviour, Entry); AC-S3; 0115's egress paragraph rewritten to what is enforced |
| R3 | the evals suite does not compile on macOS: `find_victim_agent`, `boundary_observe`'s `unshare`, `ProcessIdentity` uses in `trial_isolation.rs`, one `main.rs` entry test and the `/etc` layout test are ungated, so the required `test (macos-latest)` check is red (guarantees HIGH; independent, tests MEDIUM) | AC-S1 rescoped to Linux, AC-S6's macOS half restated, AC-S7's CI clause; cfg gating per the repo's pattern (plan) |
| R4 | AC-L1's void covers only same-role port coincidences; a cross-role or SMTP/POP3 coincidence reports a false breach instead of voiding (correctness, independent, tests, guarantees; low) | AC-L1's LISTEN-port void rule |
| R5 | the spec says the proof command has run on 0.11.1; it has never run (guarantees) | Required behaviour, the floor's evidence sentence |
| R6 | AC-L2 says `/home` is absent; the observer cannot check that because the bound checkout files need their parent directories (correctness, guarantees) | AC-L2 restated: nothing under `/home` but bound files and their empty parents |
| R7 | the spec asserts nested user namespaces are impossible inside; AC-L2 only observes (guarantees) | Feature and rationale, AC-L2: an observation on this host |
| R8 | precondition order in `isolate()` deviates from the spec's order (independent, low) | Required behaviour, Entry; AC-S3's two-fault case |
| R9 | an absolute agent program reached through a symlinked directory is bound at the canonical path but executed at the given path (independent, low) | Required behaviour, argv rules; AC-S4 |
| R10 | AC-S4 calls one fixed example a property test (correctness, independent; info) | AC-S4: a table over hostile tokens in every position |
| R11 | AC-S5/S8 say `git diff main`; the local `main` ref is stale (correctness; info) | AC-S5, AC-S8: `git diff a4f552d` |
| R12 | the oracle trial in the proof carries no `--deadline-ms` while the docs say every trial is bounded at 30 s (tests; info) | Live proof command paragraph; AC-L2 |
| R13 | bwrap's stderr forwarding is never asserted (tests; info) | AC-S2's `bwrap_probe` case asserts it |
| R14 | `admission::root_for`'s doc comment describes the deleted raw worker (independent; info) | plan, project structure |

## Panel synthesis — adopted, rejected, reconciled

The first synthesis table (bwrap over the raw worker, the constant path and
floor, the allowlist argv without a `/` bind, `--clearenv`, no in-boundary
self-check, one preflight probe, signalling the runner on interrupt,
admission held through reaping, the descriptor sweep, no row schema change,
no egress machinery, no `.github/` change, the live-proof arrangement) stands
as committed at 3ecd4f8 except where a row below replaces it. The second
panel argued the review return; this is the ruling.

| Question | Ruling | Losing side and its cost |
| --- | --- | --- |
| R1: the PID-1 reaper | **Simplicity:** add `--as-pid-1` so bwrap installs no reaper; the runner is PID 1 of the trial's PID namespace. The attack needs a same-uid process inside that holds a descriptor leading out; with the reaper gone, every process inside is the runner or what it spawned, all exec'd behind the controller's close-on-exec sweep. AC-L2 gains a check aimed at the attack itself: no descriptor of any process inside reaches the host `/proc`. `/proc/1/mem` openability and `kernel.yama.ptrace_scope` are **recorded, not asserted**: PID 1 is now the trial's own runner, and reading one's own runner is within-trial (0115 ruling 6, unconfined). The first synthesis row *Lifetime chain — no `--as-pid-1`* is reversed. | **Robustness's non-dumpable init** (`--isolated-init`, `PR_SET_DUMPABLE`, a second sweep, fork, `kill(-1)`, reap) is rejected: it is the hand-built PID-1 vocabulary the operator's direction retired, a new hidden mode with signal, reaping and coverage obligations, and it protects only a trial's own init from the trial's own agent. **Its `ptrace_scope ≥ 1` prerequisite** is rejected: Yama restricts `PTRACE_MODE_ATTACH` (`/proc/<pid>/mem`, ptrace) and not `PTRACE_MODE_READ` (`/proc/<pid>/fd`), so it would narrow which hosts qualify without closing the route the finding names. Cost of the ruling: (a) a `SIGTERM` to the controller that lands between bwrap's exec and the runner installing its 0073 handler is dropped by the kernel (an init without a handler ignores signals from an ancestor namespace), so that trial ends at its `--deadline-ms` instead; bounded, and never a false row; (b) grandchildren orphaned inside re-parent to the runner, which reaps only its own children, so they stay zombies until the runner exits and the kernel empties the namespace; bounded by one trial; (c) if the first live run shows bwrap 0.11.1 refuses `--as-pid-1` beside `--die-with-parent`, the probe refuses `bwrap_probe` by name, and the fallback argued then with evidence is the init, never the default reaper. |
| R2: the paid-driver promise | **Robustness, by a cheaper lever:** `--isolated` refuses every non-builtin agent program as `isolation_external_agent_unsupported` unless its resolved path is one of the files the caller named with `--isolated-expose`. The live proof's scripted agents are already exposed that way; no new flag, one new refusal name. 0115's sentence becomes what the code enforces. | **Simplicity's narrowing** of the sentence is rejected: a decision that promises what the code does not enforce will be believed, and the egress question is open, so an external driver has no supported use under `--isolated` until the operator rules. **Robustness's `--isolated-test-agent` flag** is rejected as a second lever where one exists. Cost of the ruling: an operator who writes `--isolated-expose /usr/bin/<driver> -- /usr/bin/<driver> …` deliberately runs a driver that cannot reach its provider and gets an honest failure row; the documentation says an isolated row is a boundary-proof row, not an eval result, until AC-L4 is ruled. |
| R3: macOS compile | **Both:** `#[cfg(target_os = "linux")]` on every Linux-only helper and every test or signature that names one, following `tests/report.rs`, `runner.rs` and `wrappers_launch/mod.rs`; the `/etc` layout test asserts the host's own `symlink_metadata` rather than `Tree::Directory`; the `cfg(not(linux))` platform tests stay as macOS's real proof; AC-S1's leak proof is **Linux-only**. The seat cannot build for Darwin (`ring` needs an Apple toolchain), so the macOS compile is CI-only evidence and the implementer audits by name. | None. No cross-compile gate is added: the toolchain it needs cannot be installed offline. |
| R4: the void rule | **Both:** inside B's private network namespace every loopback `LISTEN` socket is B's own by construction, so before routes 14 and 15 the probe reads `/proc/net/tcp` and `/proc/net/tcp6` and voids (90) when A's MCP or fixture port is among B's listening ports. The pairwise same-role comparisons are deleted, not extended. | None. |
| Supervisor bound on bwrap's wait | **Reversed by the third ruling below (P6).** The second revision rejected it because no finding asked for it; the live proof and the operator's direction of 2026-11-01 ("every wait in the controller must be bounded") now ask for it. | The second revision's row stays in history as the position the live proof overturned. |
| Burst-at-the-cap trial inside the boundary | **Rejected:** 0003 is proven by the root suite against the unchanged server; the boundary adds no pruning path. | Robustness §4. Cost: none to the frozen guarantee. |
| R5–R14 | **Both:** adopted as written in the table above. AC-S4's "property test" becomes a table over hostile tokens in every argv position; `/home` is restated to what the observer can prove; nested user namespaces and `/proc/1/mem` are observations; the floor's evidence sentence says the proof is outstanding; `git diff` binds to a4f552d. | None. |

### The third ruling — after the live proof

The third panel argued the live-proof failure, not the boundary. Both
positions read the same cause (P1/P2) off the code and the trace, and they
agree on the two-suite skip rule and on one bounded test-side runner; they
part on how much of the controller to rebuild. This is the ruling.

| Question | Ruling | Losing side and its cost |
| --- | --- | --- |
| How many defects | **Simplicity:** P1 and P2 are one defect, the std locks held across the trial, and the fix removes its cause. **Robustness, in part:** the unbounded `child.wait()` (P6) is a separate hole the operator's direction closes now, not at activation. | None: both are answered. |
| The controller's runtime and its writers | **Simplicity:** the isolated arm runs on a **current-thread** runtime built by one production entry, `isolation::run`, that the unit tests call too, so tests and production share the code that was wrong; `pump` writes its chunk directly (no `spawn_blocking`, no per-chunk copy), so every write is on the main thread, which is the only thread that may hold the std locks; `prepare_trial`'s result is bound by `let` before it is matched, so no lock guard outlives the statement that took it; after `block_on`, `shutdown_timeout(DRAIN_BOUND)` and `return`. **Robustness's rule survives as a stated rule:** no `Stdout`/`Stderr` lock guard may be held across `.await` or `block_on`, anywhere in the controller, and AC-S11 is its proof. | **Robustness's owned-sink plumbing** (`out`/`err` writers threaded into `launch`/`supervise`) is rejected: the pumps are spawned tasks and would need `'static` sinks, which is new plumbing to fix a scope bug. **Robustness's `std::process::exit` after the row** is rejected: with a bounded shutdown nothing is left to wait on, and an explicit exit would hide a future blocking destructor instead of making the always-on test (AC-S10) catch it. Cost of the ruling: a consumer that stops reading the controller's stdout blocks the copy on the reactor thread, where the drain timers cannot preempt it. That is the unisolated runner's own contract (`print_line` is the same blocking write), the row is a few KB under one pipe buffer, and the test side reads to files; it is named here and accepted. |
| The bound on bwrap's wait (P6) | **Both, reconciled.** The number is **simplicity's**: the trial's deadline (`--deadline-ms`, else the task's declared `deadline_ms`) plus the matrix's existing 300 000 ms grace, the literal in `run_child_assigned` lifted into one shared constant, so there is one envelope in the harness, not two. The **seam is robustness's**: the bounds are a parameter of `launch` (a `Bounds` value beside the `bwrap` path; production fills it from the request, tests pass milliseconds), because a stand-in bwrap cannot prove a 300 s bound without sleeping (0020 §5). On expiry: `start_kill()` bwrap (its `--die-with-parent` `SIGKILL`s PID 1 and the kernel empties the namespace), reap within `DRAIN_BOUND`, print one fixed stderr line, and judge by the **existing** outcome match: no byte, `isolated_trial_no_row`; a byte and a signal death, exit 2. No new row reason. | **Robustness's `deadline + 30 s` with a 60 s floor and 300 s ceiling** is rejected as a second number set. **Its process-group `SIGKILL`** is rejected: `--new-session` puts the sandbox in its own session, so `kill(-pgid)` reaches only bwrap, which `start_kill` already does; `--die-with-parent` is the mechanism. **Simplicity's "no seam, one constant"** is rejected because it leaves the envelope unprovable in the seat. Cost of the ruling: a runner that hangs past its deadline holds the host admission for up to deadline + 300 s (5.5 min for a 30 s trial); the test side bounds its children tighter (AC-S12), and the matrix never runs isolated. |
| The byte count (P5) | **Robustness:** the count is kept on the read side as bytes arrive, so an expired drain never turns a delivered row into `NoRow`. | None. One shared counter; no new type. |
| The test-side runner (P3) | **Simplicity:** one helper spawns every isolated CLI child with stdout and stderr on **files** in the test's tempdir (no reader thread can block, no pipe can fill), polls `try_wait` to a hard deadline, and on expiry `SIGKILL`s the child and panics by name with everything it wrote. `ChildGuard::wait_with_output` becomes that bounded wait. The bound is **90 s** per child: 5 s version + 10 s probe + a 30 s trial deadline + the runner's own teardown inside + 5 s + 5 s drains + slack. | **Robustness's 120 s and reader threads** are rejected: longer, and threads are what hangs. Cost: a slow host that takes more than 90 s for a 30 s trial fails by name and the bound is raised with that evidence; the worst-case run time is stated in the live-proof paragraph rather than promised small. |
| What proves the fix | **All three tests, from both positions.** The always-on handoff test around the real CLI (AC-S10; both positions); the seat-runnable re-exec red test that holds the stdout lock as `main` did (AC-S11; simplicity); the stand-in bwrap that writes a row and then hangs, and the one that writes nothing and hangs, under a millisecond envelope (AC-S2; robustness). | None. |
| The skip rule | **Both:** always-on tests that need the boundary print `SKIP trial-isolation: boundary_unavailable:<step>` and **return** where it is unavailable; the ignored live proofs print the same line and **fail**. The spec states both sentences side by side (AC-S6). | None. |
| Documents | **Both:** only the facts the run changed; 0115 stays proposed and gains one dated subsection. | None. |

## Required behaviour

**Entry.** `swarmail-evals trial --isolated …` (hidden flag) validates its
request exactly as today. The precondition refusals are checked **in this
order**, each before any boundary work, and the first one hit names the row:
`concurrency_unsupported`, `isolation_requires_swarmail_bin`,
`isolation_artifacts_unsupported`, `isolation_wrapper_unsupported`,
`isolation_expose_invalid`, `isolation_external_agent_unsupported`,
`isolation_source_invalid`. `SWARMAIL_BIN` and the agent program are resolved
only after the artifacts, wrapper and expose checks (R8). On any refusal the
trial prints one `refused_row` harness-error row and exits 2 before any
server or agent exists. There is no fallback path.

`isolation_external_agent_unsupported` (R2): under `--isolated` the agent is
a builtin (`oracle`, `null`) or an external program whose canonical path is
one of the canonical `--isolated-expose` files. Anything else refuses. The
builtins and the exposed scripted agents are the only programs that run
inside until the operator rules on provider egress (AC-L4); a real model
driver cannot be run isolated by accident, and an isolated row is a
boundary-proof row, not an eval result. The matrix never renders
`--isolated`, and a unit assertion says so.

**Boundary refusals, a closed set.** Each is `boundary_unavailable:<step>`
with `<step>` one of:

- `platform` — any non-Linux target. **macOS refuses by name**, since bwrap
  is Linux-only; CI's macOS `test` job compiles the evals suite and runs the
  `cfg(not(target_os = "linux"))` refusal tests. That is all macOS proves.
- `bwrap_absent` — `/usr/bin/bwrap` is missing, not a regular file, or not
  executable.
- `bwrap_version` — `/usr/bin/bwrap --version` did not finish within the
  bound, did not print exactly `bubblewrap <major>.<minor>.<patch>` (trailing
  text ignored), or is below `0.11.1`.
- `bwrap_probe` — the exact production argv with `/usr/bin/true` as the
  command did not exit 0 within the bound. bwrap's stderr is forwarded
  unchanged so the operator sees the kernel's reason (`ENOSPC`, `EACCES`,
  `EPERM`) or bwrap's own option error (for example a bwrap that rejects
  `--as-pid-1`); it is never interpolated into the row (0075 finite
  categories).

The errno-name table is deleted: bwrap reports its own errors in its own
words on stderr. The version floor is `0.11.1` because it is the only
version on which the boundary has been observed to work: the proof command
ran once on this host on 2026-11-01 (P1–P4), the probe passed, bwrap 0.11.1
accepted the full argv, `--as-pid-1` beside `--die-with-parent` included,
and the runner ran inside as PID 1 of the namespace. That run failed in the
controller, not in the boundary, so the floor is confirmed for the launch
and the live criteria AC-L1..L3 stay outstanding until the command passes.
Every flag used, `--as-pid-1` included, is older than 0.9. Lowering the floor
(Ubuntu 24.04 packages 0.9.0) is an ordinary one-line change that must carry
a live proof on that version. The floor is a compatibility statement, not a
security attestation of a distribution's patch level. The version and probe
are checked on every launch, never cached.

**The argv** is one pure function over the validated request, golden-tested:

```
/usr/bin/bwrap
  --unshare-user --unshare-pid --unshare-net --unshare-ipc
  --die-with-parent --new-session --as-pid-1
  --clearenv
  --setenv PATH /usr/bin:/bin --setenv HOME /tmp --setenv TMPDIR /tmp
  --setenv SWARMAIL_BIN <swarmail_bin>
  --size 536870912 --tmpfs /tmp
  --ro-bind /usr /usr
  [--ro-bind /bin /bin | --symlink usr/bin /bin]        (per host layout)
  [--ro-bind /lib /lib | --symlink usr/lib /lib]
  [--ro-bind /lib64 /lib64 | --symlink usr/lib64 /lib64]
  --ro-bind /etc /etc
  --ro-bind <runner_exe> <runner_exe>
  --ro-bind <swarmail_bin> <swarmail_bin>
  [--ro-bind <agent> <agent>]
  [--ro-bind <expose> <expose> ...]
  --proc /proc --dev /dev
  --chdir /tmp
  -- <runner_exe> trial <the request re-rendered without --isolated and
     --isolated-expose> [-- <agent argv>]
```

`--as-pid-1` (R1) is the one argv change of this revision. bwrap then
installs no reaper: the process it execs, the runner, is PID 1 of the trial's
PID namespace, and there is no same-uid process inside that the runner did
not spawn. bwrap's `--die-with-parent` still delivers `SIGKILL` to that PID 1
when bwrap or the controller dies, which an init cannot ignore, and when PID
1 exits the kernel kills and reaps everything left in the namespace before
bwrap's `wait` returns, so admission-through-reaping (0076 §3) is unchanged.
Signals other than `SIGKILL`/`SIGSTOP` reach a namespace init from outside
only once it has installed a handler; the runner installs its 0073
`SIGTERM`/`SIGINT` handlers when the trial arm starts, and the window before
that is an accepted cost (synthesis table, R1).

Implementation note, 2026-11-01 (implement seat, kept): the private `/tmp`
is mounted **before** the file binds, not after as first drawn. bwrap
applies options in order, so a tmpfs mounted last hides any bound file that
lives below `/tmp` on the host; mounted first, such a file is bound into the
sandbox's own tmpfs. The golden test records this order.

Rules: a system tree that is a symlink on the host is recreated as the same
symlink, a missing one is skipped (today's `SYSTEM_ROOTS` semantics, rendered
as bwrap options); every bind is read-only and bwrap makes it `nosuid`; the
first `--` is at a fixed position and everything after it is runner-owned, so
no agent argument can become a bwrap option; the inner request is
re-rendered from the parsed `Cmd::Trial` fields (never by filtering the raw
argv, which would strip an agent's own `--isolated`) and proven by a clap
round trip over a request with every field populated; **the agent program
inside is the same canonical path the controller binds** (R9): the
controller canonicalizes the program's directory once and uses that path
both in the `--ro-bind` and in the inner argv, so a program reached through
a symlinked directory runs where it was bound (a symlinked final component
still refuses `isolation_source_invalid`, as today). The `/tmp` bound is the
512 MiB the design on record used; an `ENOSPC` inside is the runner's own
harness error, never a pruned inbox.

**The controller** is the `swarmail-evals` process the operator or matrix
started. It stays in the host namespaces and never runs the trial arm:

1. refuses the preconditions above, synchronously, in their named order,
   and binds that result before matching on it: **no `Stdout` or `Stderr`
   lock guard outlives the statement that took it, and none is held across
   an `.await` or a `block_on`** (P1);
2. checks `platform`, `bwrap_absent`, `bwrap_version`, then runs the
   `bwrap_probe`, each bounded (5 s for version, 10 s for the probe);
3. acquires the host admission (0076 §1; `concurrent_trials_refused` as
   today) and holds it until bwrap is reaped — which, with the runner as PID
   1, is after the kernel has emptied the namespace — or until the envelope
   below has expired and the bounded reap after it has run out;
4. builds a **current-thread** tokio runtime in one production entry,
   `isolation::run`, which the unit tests call as well, and from its main
   thread spawns bwrap with `process_group(0)`, `kill_on_drop`, null stdin,
   piped stdout and stderr, and the `close_range` CLOEXEC sweep in
   `pre_exec`; copies both pipes to its own stdout and stderr **from that
   same thread** (the pumps write directly; no `spawn_blocking`), so
   bwrap's raw diagnostics appear beside the row and never in it, and no
   writer can park on a lock another thread holds (P1);
5. on `SIGTERM`/`SIGINT`, finds the runner's host PID (the descendant of
   bwrap whose executable is the runner; with `--as-pid-1` that is bwrap's
   direct child) and sends it one `SIGTERM`; if no runner exists yet, kills
   bwrap and prints the `isolated_trial_interrupted` row;
6. waits for bwrap **under an envelope** of the trial's deadline
   (`--deadline-ms`, else the task's declared `deadline_ms`) plus the
   matrix's 300 000 ms grace, one shared constant (P6). When the envelope
   expires it `start_kill()`s bwrap (whose `--die-with-parent` `SIGKILL`s
   PID 1, and the kernel empties the namespace), waits `DRAIN_BOUND` for the
   reap, and prints one fixed line on stderr,
   `swarmail-evals: isolated trial envelope expired; bwrap killed`. It then
   drains both pipes, each bounded by `DRAIN_BOUND`, with the stdout byte
   count kept **on the read side as bytes arrive**, so an expired drain
   never reads as no row (P5). The verdict is the existing one: if at least
   one byte reached stdout, the controller exits with bwrap's exit code
   (bwrap propagates the runner's); if none did, it prints the
   `isolated_trial_no_row` harness-error row and exits 2; if bwrap died by a
   signal, the envelope kill included, exit 2. No new row reason exists;
   the finite 0075 set is unchanged;
7. shuts the runtime down with `shutdown_timeout(DRAIN_BOUND)` and
   **returns from `main`** with that code. Every wait in the controller is
   one of: the version bound, the probe bound, the envelope, the bounded
   reap, the two drain bounds and the shutdown bound; the admission never
   waits (`try_lock`). A controller that has written its row exits (P2).

The bounds (`Bounds { envelope, drain }`) are a parameter of `launch`
beside the bwrap path, filled by production from the request and by the
stand-in tests in milliseconds; like the path, no environment variable,
flag or cfg selects them.

The inner process is the same binary running the unchanged `trial` arm as
PID 1 of its namespace: it takes its decorative admission on the private
`/tmp`, builds its runtime and runs the trial. `SWARMAIL_BIN` arrives by
`--setenv`.

**Inside the boundary nothing else changes.** Same catalogue, scorer, fixture
identity rule, failure precedence, `confinement:"unconfined"`, cleanup owner
and admission-through-disposal discipline (0076 §3). The trusted initiating
user, the controller, the kernel, bwrap and any unisolated host process stay
outside the hostile-agent threat model; agents launched inside sibling trials
are inside it. An agent reading its own runner's `/proc/1` is within-trial,
exactly as reading the runner's `/proc/<pid>` is in an unisolated trial
(0115 ruling 6). A hostile agent still shares the host kernel and can still
consume host CPU and memory: bwrap sets no cgroup limits unprivileged, so
the boundary is an access boundary, not a resource boundary, and nothing
sells it as one. Timing through the host clock is a covert channel, not an
access path. A kernel or bwrap bug defeats all of this.

**Frozen guarantees under the boundary.** No proxy sits between the agent
and Swarmail, so an SMTP `250` is still a synchronous, queryable insert
(0001). The fixture and Swarmail share one private loopback, so the
long-poll still subscribes before it scans (0002). Each trial prunes its own
store oldest-only; a sibling cannot evict (0003). Extraction runs at ingest
inside the boundary (0005). Root `src/` is not touched.

**Host prerequisite, not a dependency.** bubblewrap is LGPL-2.0-or-later,
executed as a separate process and never linked, vendored or shipped. It
appears in neither lockfile; `deny.toml` does not change; no Rust crate is
added (`libc` is already pinned in `evals/`). 0115 states the prerequisite
for the operator: Linux, `/usr/bin/bwrap` ≥ 0.11.1, and a host policy that
lets bwrap create user, PID, network and IPC namespaces (Ubuntu's
`bwrap-userns-restrict` profile, or the sysctl lowered). No ptrace policy is
a prerequisite (R1 ruling); `kernel.yama.ptrace_scope` is recorded by the
live proof as an observation. Accepting 0115 is accepting that prerequisite.

**Matrix.** Untouched: sequential, unisolated, refusing concurrency as today,
and never rendering `--isolated`.

**Documentation.** `evals/README.md` and `evals/run-plan.md` revise their
dated #221 sections only where the live run changed the facts: the proof
ran once on 2026-11-01, the boundary worked and the controller lost the row
and hung; every wait in the controller is now bounded and the envelope is
named; a controller that has written its row exits; every isolated test
child is held under a 90 s bound; the always-on suite skips by name without
bwrap and the ignored suite fails by name; the dispatcher runs the proof
with a hard 20-minute limit. Nothing else changes.

## What stays outstanding after this run, and how to prove it

| Outstanding | Owner | How it is settled |
| --- | --- | --- |
| AC-L1..L3 hostile live proof | the dispatcher, on this host, after the run | the command below, green, within the dispatcher's hard 20-minute limit; its first run on 2026-11-01 failed in the controller (P1, P2), not in the boundary |
| AC-L4 provider egress | operator ruling on 0115's question, then a host proof | a negative test that the chosen path cannot reach the host fixture port or a sibling address |
| AC-O1 operator ruling on 0115 (including the bwrap prerequisite) | operator | acceptance or rejection recorded in `docs/decisions/` |
| CI boundary job | operator (`.github/` is held) | a serialized ubuntu job that provides a bubblewrap at or above the floor (stock 24.04 apt gives 0.9.0, below it), checks the floor and policy, runs the same command and fails closed |
| Concurrency re-enabled | a later change after AC-L1..L4 and AC-O1 | a new row version chosen then; not this run |

**The live proof command**, from the repository root on this host:

```bash
cargo build --locked --bin swarmail
cd evals && SWARMAIL_BIN="$PWD/../target/debug/swarmail" \
  cargo test --locked --test trial_isolation -- --ignored --nocapture --test-threads=1
```

It is deterministic (null and oracle builtins, `/usr/bin/sleep`, the
scripted probe and observer from the test executable; no provider, no
network beyond the private loopbacks; no paid call; a port coincidence voids
by the LISTEN rule and is retried, never scored), bounded and
self-describing (each test probes first and fails by name when the boundary
is unavailable). Bounded means, exactly: **every** isolated trial in it, the
null probe and the oracle signup included, carries `--deadline-ms` of at
most 30 000 (R12); **every** CLI child it starts is held by the bounded
runner of AC-S12 at 90 s and is `SIGKILL`ed and failed by name past that;
the controller's own envelope (deadline + 300 s) sits behind the test's
bound; and no test starts more than five such children (the hostile proof
with its two void retries), so no single test runs past 7.5 min even if
every child of it hangs. The dispatcher runs the command with a **hard
20-minute limit**: that is the acceptance gate of this PR, and a run in
which every child times out exceeds it and fails, by the dispatcher's kill
after the earlier tests have already failed by name. The healthy duration
is not promised here; the first passing run records it in tasks.md's
status. **AC-L1, AC-L2 and AC-L3 are proven only by that command on a host
whose bwrap can create the namespaces.** It ran once, on 2026-11-01, and
failed with P1/P2 before any live criterion was reached. The seat cannot
run it: `/usr/bin/bwrap` 0.11.1 is present in the seat and refuses with
`ENOSPC` (the seat is already inside a user namespace), so there every live
proof prints `SKIP trial-isolation: boundary_unavailable:bwrap_probe` and
fails, as AC-S6 requires.

## Acceptance Criteria

Criteria marked **S** are proven in this seat; where a criterion is
Linux-only it says so, and on macOS it reduces to compiling. Criteria marked
**L** are live boundary proofs: `#[ignore]` with the named reason, red when
asked for and unavailable, outstanding until the host command above passes.
**O** is the operator's.

- **AC-S1 — The leak is live, not alleged (Linux).** An always-on test, on
  Linux, holds an idle signup victim; a bare same-uid helper, given only the
  victim runner's PID, finds the victim agent among that runner's
  descendants, reads its `EVAL_*` environment from `/proc`, completes signup
  through real MCP and the real fixture, and the victim row is `pass`. The
  negative control for the probe (routes 10, 91, 30) stays always on, on
  Linux. Both are `cfg(target_os = "linux")` because they read `/proc`
  (R3); macOS has no `/proc` and proves nothing here rather than passing
  vacuously, per the `tests/report.rs` precedent.
- **AC-S2 — Refusal before launch, through a parameter seam.** The launch
  takes the bwrap path **and its bounds** as function parameters; no
  environment variable, flag or cfg selects either. Unit tests pass, in
  turn: an absent path (refuses `bwrap_absent`); a generated script printing
  `bubblewrap 0.9.0` (refuses `bwrap_version`); a script printing
  `bubblewrap 0.11.1` that writes `bwrap: <marker>` to stderr and exits 1
  for anything else (refuses `bwrap_probe`, **and the controller's captured
  stderr contains that marker while the row does not**, R13); a script that
  passes the version and the probe and then exits 1 with nothing on stdout
  (`isolated_trial_no_row`). In every case the row is `harness_error`, exit
  is 2, stderr names the reason, no admission was taken before the probe
  passed, and the agent's sentinel file was never created. The sentinel
  agent in these tests is named by `--isolated-expose`, so the external-agent
  precondition admits it and the stand-in is reached. **The bounds are
  proven the same way (P5, P6), through `isolation::run` with millisecond
  bounds:** a stand-in that passes the version and the probe, writes a row
  to its stdout and then sleeps forever yields `Exited(2)` within the
  envelope plus the drain bound, the stand-in is dead afterwards, and the
  controller's stderr carries the fixed envelope line; one that writes
  nothing and sleeps forever yields `NoRow` the same way; one that writes a
  row and exits 0 while a grandchild keeps its stdout open past the drain
  bound yields `Exited(0)`, never `NoRow`, because the count is read-side.
  Every one of these tests is a real deadline on a child that never ends by
  itself (0020 §5: chaos at the poles), and each runs in the seat.
- **AC-S3 — Precondition refusals are real CLI runs, in the named order.**
  `trial --isolated` without `SWARMAIL_BIN`, with `--artifacts-dir`, with a
  `--wrapper-profile`, with an `--isolated-expose` that is not an absolute
  regular file, **with an external agent program that is not exposed**
  (`isolation_external_agent_unsupported`, R2), and with an unresolvable
  program each emit one harness-error row with the named reason and exit 2,
  and no agent starts. `--isolated --concurrency 2` refuses
  `concurrency_unsupported` first. A two-fault case proves the order (R8):
  `--isolated --artifacts-dir <dir> -- /missing/program` refuses
  `isolation_artifacts_unsupported`, not `isolation_source_invalid`; and
  `--isolated --isolated-expose /missing -- /bin/sh` refuses
  `isolation_expose_invalid`, not `isolation_external_agent_unsupported`.
  An exposed external program (`--isolated-expose <p> -- <p>`) passes the
  precondition and reaches the controller. A unit assertion proves the
  matrix never renders `--isolated`.
- **AC-S4 — Pure parts are unit-tested.** The version parser accepts
  `bubblewrap 0.11.1`, `bubblewrap 0.11.1-ubuntu1` and `bubblewrap 1.0.0`,
  refuses `bubblewrap 0.11.0`, `bubblewrap 0.9.0`, an empty string and
  garbage. The golden argv for a request with an agent and one exposed file
  is exactly the list above: `--as-pid-1` present, no `/` bind, no `-try` or
  `--unshare-all`, no `--share-net`, `--clearenv` before every `--setenv`,
  `--size` immediately before `--tmpfs /tmp`, the first `--` immediately
  before the runner executable. **A table test** (R10) places each of
  `--ro-bind`, `--share-net`, `--as-pid-1`, `--`, `--isolated` and
  `--chdir` at every index of the agent argv and shows the index of the
  first `--` is the same as for the benign argv and none of the tokens
  precedes it. The inner-args clap round trip over a request with every
  `Cmd::Trial` field populated yields the same request minus `isolated` and
  `isolated_expose`, with the agent program canonical; **a program reached
  through a symlinked directory renders inside as the canonical path the
  bind uses** (R9); the renderer destructures `Cmd::Trial` without `..` so a
  new field fails to compile until it is rendered. The bind allowlist tests
  (`resolve_source`, `bind_plan`) stand; the host-layout test asserts
  against the host's own `symlink_metadata` of each system root, so it is
  true on merged-usr Linux and on macOS, where `/etc` is a symlink (R3).
- **AC-S5 — Concurrency stays off.** Every existing refusal test in
  `evals/src/runner.rs` and `evals/tests/` stays green; `git diff a4f552d
  --stat -- src/` is empty (R11); no production path reads a proof record;
  every row still carries `concurrency: 1`.
- **AC-S6 — Skip discipline, and what each platform proves.** Two suites,
  two rules, stated side by side (P3). **The ignored live proofs:** every
  test that proves AC-L1..L3 is
  `#[ignore = "trial-isolation: needs /usr/bin/bwrap >= 0.11.1 able to create user namespaces"]`
  and `cfg(target_os = "linux")`. When run explicitly it probes with a real
  null-agent `trial --isolated` (under the bounded runner, with
  `--deadline-ms 30000`); on refusal it prints
  `SKIP trial-isolation: boundary_unavailable:<step>` and **fails**. It never
  passes without the boundary. The gate inventory never passes `--ignored`.
  **The always-on tests:** a test that needs a working boundary (AC-S10, and
  any always-on test that runs a real `trial --isolated` past the
  preconditions) probes the same way, under the same bound; where the
  boundary is unavailable it prints the same `SKIP trial-isolation:
  boundary_unavailable:<step>` line and **returns**, asserting nothing about
  the boundary; where it works it completes within its bound and asserts.
  A test whose subject is the *absence* of the boundary
  (`an_unavailable_boundary_refuses_by_name_and_never_falls_back`,
  `an_exposed_external_agent_passes_the_precondition_and_reaches_the_boundary`)
  prints `SKIP trial-isolation: boundary available` and returns on a host
  where it works. So the always-on suite finishes within bounded time on a
  seat without bwrap and on a host with it, and no named SKIP is ever an
  `ok` in the live proof. On macOS the evals suite **compiles** and the `platform` refusal is proven
  by the `cfg(not(target_os = "linux"))` tests in `isolation.rs`, `main.rs`
  and `trial_isolation.rs`; a Linux unit test proves the platform-neutral
  refusal function. Nothing else is claimed for macOS (R3).
- **AC-S7 — Gates.** Every command in `docs/house-rules.md` § Gates passes
  locked, plus the standalone evals `fmt`, `clippy --all-targets` and
  `cargo test --locked`. CI-owned obligations are not claimed; the macOS
  compile of the evals suite is CI's `test (macos-latest)` evidence, which
  the seat cannot produce (no Apple toolchain for `ring`), so the implementer
  audits every `cfg(target_os = "linux")` item for ungated uses by name
  (R3). No new dependency; neither lockfile nor `deny.toml` changes.
- **AC-S8 — Authority.** `docs/decisions/0115-trial-isolation.md` is
  committed with status `proposed`, its index row unchanged; its *Revision
  2026-11-01* section and the *After the first review* subsection name
  `--as-pid-1` and why, the external-agent refusal, the observations
  (`/proc/1/mem`, `ptrace_scope`, nested user namespaces), the macOS scope
  and the CI facts (stock 24.04 apt gives 0.9.0, below the floor); its
  *After the live proof* subsection records the 2026-11-01 run (the boundary
  worked, the controller lost the row and hung), the bounded-controller
  contract (every wait bounded, the envelope, exit after the row), the
  two-suite skip rule and the 20-minute acceptance limit, and corrects "has
  not run" to what ran. Accepted decisions, the dependency allowlist, gate
  scripts, `.github/` and review routing are unchanged; `git diff a4f552d --
  docs/decisions/0055-agent-eval-harness.md
  docs/decisions/0076-sequential-eval-trials.md` is empty (R11).
- **AC-S9 — The raw worker is gone.** Kept as proven at bdc504f:
  `grep -rn "pivot_root\|SYS_clone\|uid_map" evals/src` is empty; the
  `root_for` doc comment no longer describes it (R14).
- **AC-L1 — The boundary holds against the same helper.** Victim A is an
  isolated `/usr/bin/sleep` signup trial launched by the trusted test
  through the library argv builder, held under the deadline; the test reads
  A's coordinates from the host `/proc` (same uid, host namespaces). Trial B
  runs the real `trial --isolated --isolated-expose <test exe>` with the
  AC-S1 probe as its agent and A's coordinates in argv. The probe first
  reads `/proc/net/tcp` and `/proc/net/tcp6` inside B and returns 90 (void)
  if A's MCP or fixture port is a `LISTEN` port there, since every listener
  inside B's network namespace is B's own (R4); it also voids on a prompt or
  HOME path coincidence. The same-role pairwise comparisons are gone. It
  then tries, in order: discovery of any `/proc` process carrying A's
  environment (10), A's runner and agent `/proc` entries (11), A's prompt
  file (12), A's HOME (13), A's MCP authority (14), A's fixture authority
  (15), and completing A's signup (16); exit 0 means every route was denied.
  The test retries a void at most twice and fails by name on the third. The
  test then kills A's agent from outside; A's row is `fail` in signup's
  declared idle class with no accepted confirmation, never `pass`, `budget`
  or `harness_error`. A passing victim turns this test red.
- **AC-L2 — Fresh HOME, own services, and the observed topology.** A real
  child observes a fresh writable HOME; a sentinel from the previous isolated
  trial is absent; the oracle passes signup inside the boundary through its
  own MCP and fixture, with `--deadline-ms 30000` (R12). A scripted observer
  agent, run as an isolated trial's agent, reports by exit code the first
  violated check among: uid is the caller's; `/proc` lists no host PID and
  `/proc/1/exe` is the runner executable, so bwrap installed no reaper (R1);
  `/proc/net/dev` lists exactly `lo`
  and a connect to a port the observer itself bound succeeds; `/root`,
  `/run/user`, `/var/tmp`, the other host trees, the operator's HOME
  dotfiles and a canary file the test created under the host `/tmp` are
  absent, **and nothing exists under `/home` except the bound files and the
  directories on the path to them** (R6); `/tmp` is a tmpfs of the bounded
  size; the environment holds no key beyond the four set by the argv and
  those the runner sets; no descriptor at or above 3 is open in the
  observer beyond its own stdio and the canary the controller held is
  absent; **for every process readable in the sandbox's `/proc` (PID 1, the
  runner, included) no entry of `/proc/<pid>/fd` opens as a directory that
  contains any of the host PIDs the test passed**, so no descriptor inside
  reaches the host `/proc` (R1); `CapEff` is zero and `NoNewPrivs` is 1. It
  **records without asserting**, on stderr: whether `unshare(CLONE_NEWUSER)`
  succeeds inside (R7), whether `/proc/1/mem` opens read-write, and the
  host's `kernel.yama.ptrace_scope` as the test read it (R1). The `/proc/1/exe`
  check is what proves the reaper is gone; the descriptor walk is what would
  catch its escape route if it were not.
- **AC-L3 — Lifetime is bound on both ends.** `SIGKILL` to the controller
  leaves no trial process alive on the host; `SIGKILL` to the runner (from
  the host) leaves no agent alive and the controller ends with exit 2 and
  the `isolated_trial_no_row` row; `SIGTERM` to the controller, sent after
  the agent exists, produces the runner's own interrupted row, exit 2 and no
  survivors; no orphan from trial A is alive when trial B starts. With
  `--as-pid-1` the runner is the namespace init, so these also prove that
  `--die-with-parent`'s `SIGKILL` reaches an init, that the runner's handler
  receives the forwarded `SIGTERM`, and that bwrap's `wait` returns only
  after the namespace is empty (admission-through-reaping, 0076 §3).
- **AC-L4 — Driver egress.** Open. The operator's ruling on 0115's question,
  then a negative proof that the chosen transport cannot reach the host
  fixture port or a sibling trial's address. Offline oracle passes alone do
  not satisfy it. Until then `isolation_external_agent_unsupported` keeps
  every real driver out.
- **AC-S10 — The row comes out and the controller exits, always on
  (Linux).** An always-on, non-ignored test,
  `an_isolated_null_trial_hands_over_its_row_and_exits`, runs the real CLI
  `trial --isolated --task signup --agent null --deadline-ms 30000` through
  the bounded runner of AC-S12, which passes only by actual exit. Where the
  boundary works it requires exactly one row on stdout, the null agent's
  own (outcome is not `harness_error`, no `isolated_trial_no_row` and no
  `boundary_unavailable` on stderr), the exit code the runner's own (1),
  and the controller gone within the bound; on the pre-fix controller this
  test fails at its deadline with the named message instead of hanging.
  Where the boundary is unavailable it prints
  `SKIP trial-isolation: boundary_unavailable:<step>` and returns (AC-S6).
- **AC-S11 — The deadlock is red in the seat (Linux).** A unit test in
  `isolation.rs`, `a_controller_holding_its_own_stdout_lock_still_delivers_the_row_and_exits`,
  re-executes the test binary (the `interrupt_child` pattern): the child
  takes `std::io::stdout().lock()` on its main thread as `main` did, calls
  the production entry `isolation::run` against a stand-in bwrap that
  passes the version and the probe, prints a row and exits 1, and writes
  the result to a file. The parent polls the child to a hard deadline,
  `SIGKILL`s it and fails on expiry, and otherwise requires `Ok(Exited(1))`.
  It is a separate process because in-process the stuck writer would hold
  the lock libtest's reporter needs and the suite would hang instead of
  failing. The implementer shows this test red against the pre-fix `pump`
  and runtime (the commit body or the implement notes carry the red run).
- **AC-S12 — Every isolated CLI child is bounded.** One helper in
  `evals/tests/common` spawns a CLI child with stdout and stderr on files
  in a tempdir, polls `try_wait` to a hard deadline of 90 s, and on expiry
  `SIGKILL`s it and panics `controller outlived its row after 90 s` with
  everything it wrote; `ChildGuard::wait_with_output` is that bounded wait.
  `boundary_unavailable()`, `require_boundary()`, `trial()` when called with
  `--isolated`, `launch_victim`, `idle_trial`, `idle_victim`,
  `isolated_helper_with` and every other `trial --isolated` invocation in
  `trial_isolation.rs`, ignored or not, go through it; a `grep` for
  `.output()` and `.wait_with_output()` on an isolated command outside the
  helper is empty. A test of the helper itself holds a child that never
  exits under a 200 ms bound and shows the named panic and no survivor.
- **AC-O1 — Operator ruling on 0115.** Outstanding by construction; now
  includes the host prerequisite and the `--as-pid-1` argv.

No push, merge, tag, publication, issue closure or decision acceptance belongs
to this work. Drafting this design is not implementation success.

## History

**2026-10-31 implementation.** Built and green on every host: AC-S1, AC-S3,
AC-S5 and the probe's negative control. Built and never executed: the
clone/`pivot_root` worker, errno table, step order, remount arithmetic,
`Observed`/`verify`, the `Enter` seam, and the test-side `unshare`/`setpriv`
private-admission wrapper. Observed on the host after that run:
`boundary_unavailable:identity_map:EACCES` under
`kernel.apparmor_restrict_unprivileged_userns=1`.

**2026-11-01 first bwrap revision (3ecd4f8, implemented at bdc504f).**
Deleted the never-executed parts, built the bwrap controller, argv and
refusal set, rewrote the live proofs. The verify seat passed every gate on
Linux. The review panel returned it: the default PID-1 reaper inside the
boundary, the unenforced paid-driver sentence, the macOS compile break, the
incomplete void rule and the smaller findings in the table above. This
second revision answered them; the earlier design commits remain in history
as the spec the code was first built to.

**2026-11-01 second bwrap revision (5a5b9b6, implemented at f1a30a7).**
`--as-pid-1`, the external-agent refusal, the macOS gating, the LISTEN
void, the observer's descriptor walk. The verify seat passed every gate on
Linux; bwrap refuses inside the seat, so only the refusal paths ran there.
The dispatcher then ran the live proof on the host for the first time: the
boundary worked and the controller lost the runner's row and never exited
(P1–P4). This third revision answers that; the second revision's commits
remain in history.
