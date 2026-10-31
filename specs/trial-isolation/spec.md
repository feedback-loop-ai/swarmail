# Isolate concurrent eval trials — preparation on main

Date: 2026-10-31. Issue #221 (split from #199), Epic #192. Base: main a3cff51.
Lane: `claude-careful` (accepted 0098/0099; residual rework under accepted 0102).
Operator direction, 2026-10-30: **preparation only**.

Decision: 0110 — [Trial separation and observed concurrency](../../docs/decisions/0110-trial-isolation.md)
(proposed; carried from the design on record, branch
`epic192/issue199-current-0058` at 518c52d where it was 0062, with a dated
*Current integration* section that governs where main has moved).

## Feature and rationale

A trial must not read another trial's prompt, mail, process data or HOME, and
must not use another trial's MCP or fixture endpoints to alter its business
state. Today that holds only by cooperation: accepted
[0076](../../docs/decisions/0076-sequential-eval-trials.md) admits one trial
per uid, gives every child a fresh `0700` HOME under `/tmp`, and documents
that trials are sequential. Both trials are the same uid, discretionary access
control is uid-based, and a process that outlives its trial (an agent that
detaches itself) can watch the next one even at concurrency 1. Sequential is
an admission policy; it is not isolation, and this spec never lets the two be
confused.

The design on record selects one whole-trial Linux boundary — private user,
mount, PID, network and IPC namespaces around the runner, Swarmail, the
fixture and every agent — as the single feasibility candidate. Distinct host
uids were rejected there and stay rejected here: given that rejection,
namespaces are the only remaining mechanism that turns identity into
separation, so the worker is not gold-plating; it is the price of concurrency.

This run **prepares** that worker and proves everything provable without a
namespace. Delivery seats cannot create namespaces: the 2026-10-31 seat is
already inside a user namespace (`/proc/self/uid_map` reads `1000 0 1`),
`unshare -Urn true` fails with `ENOSPC`, and
`kernel.apparmor_restrict_unprivileged_userns` reads 1. The branch recorded
`EPERM` in its box. Neither errno is assumed by the code.

Concurrency stays off. The public `--concurrency` refusal, admission and every
0076 row rule are untouched. Nothing in this run reads a "proof record" to
unlock overlap: that would contradict accepted 0076 §2 while 0110 is proposed
(accepted 0045 §8), and a file is forgeable, host-scoped and goes stale.

## Panel synthesis — what was adopted, rejected and reconciled

| Question | Ruling | Losing side and its cost |
| --- | --- | --- |
| Shape of the worker | **Simplicity:** one module entered in `trial --isolated` before the Tokio runtime exists, then the unchanged runner arm; no re-exec verb, no versioned lifecycle pipes, no proof record. | Robustness's controller pipes and re-exec. Cost: the controller cannot attest the boundary in this run; the row carries no `boundary` field until the operator accepts a row version (0110 §4). |
| Process topology | **Robustness, narrowed:** the controller stays in host namespaces, holds the host admission, forwards signals and mirrors the exit status; the trial init is PID 1 of its own PID namespace so the kernel tears the trial down when it dies, and it dies when the controller dies. | Simplicity's "unshare in the outer process, then fork" drags the controller into the trial's namespaces. Cost of the ruling: one clone with namespace flags instead of `unshare`+`fork` (three processes would otherwise be needed to make the runner PID 1). |
| Writable state | **Robustness:** a fresh size-bounded tmpfs mounted inside the boundary is the only writable path; the trial root is never a bind of a host directory; `/tmp` inside is that tmpfs, so the existing `RuntimeDirs` map lands there unchanged. | Nothing lost: 0076 §4's fresh HOME is kept, not rewritten. |
| Immutable inputs | **Simplicity's allowlist, extended by one named list:** read-only, nosuid binds of `/usr`, `/bin`, `/lib`, `/lib64`, `/etc`, a fixed set of `/dev` nodes, a fresh `/proc`, and the runner, Swarmail and agent executables at their own absolute paths. A hidden, repeatable `--isolated-expose <absolute regular file>` binds one more read-only file (the proof needs the test executable the helper script execs). | A mask list or a writable checkout (robustness's "edit the harness for later trials" route). Cost: an agent that needs anything outside the allowlist refuses by name; nothing is guessed. |
| Descriptors and privileges | **Both:** the complement is closed (`close_range` from 3), not a hand list; no-new-privs; empty capability sets and bounding set; nondumpable controller and init. Ordering is fixed so the parent-death signal is set after the last credential change. | None. |
| Verification | **Simplicity for this run:** the init self-checks before anything else runs — it is PID 1, `/proc` lists only itself, `/proc/net/dev` lists only `lo`, the old root is detached, effective capabilities are empty, no-new-privs is set. **Robustness for activation:** the controller's own readlink of the init's namespace links becomes the source of any row field. | Cost: a bug in setup is caught by the same binary that runs the trial, not by an outside observer. Acceptable while no agent has started and no row claims isolation. |
| Admission inside | The controller acquires the host admission before cloning and holds it until the trial is reaped. The inner `run_trial` still acquires one on the private `/tmp`; it is always granted and is **not** the host reservation. Documented, not hidden. | Threading a "skip admission" flag through `run_trial`. Cost of the ruling: one decorative lock inside. |
| Fixture control plane | **Reject robustness invariant 7 as inapplicable:** the runner that drives the fixture lives inside the boundary with it; nothing crosses the network namespace. | None. |
| Reaping orphans | The runner is PID 1; a detached grandchild whose parent exits becomes a zombie it does not reap, bounded by the namespace's lifetime. Recorded as a known limit; a tiny init between controller and runner is activation work if the live proof shows it matters. | Robustness's full init. Cost: zombies of daemonizing agents persist until teardown. |
| Artifacts and wrapper profiles | `--isolated` refuses `--artifacts-dir` (no writable host path enters the boundary; controller-side staging is activation work), refuses every `--wrapper-profile` (egress is open, below), and refuses an unset `SWARMAIL_BIN` (no cargo build inside). | Robustness's controller-owned artifact staging. Cost: isolated trials retain no artifacts in this run. |
| Row schema, `execution` object, event ledger | **Rejected for this run** (0110 Current integration 1–2). | Cost: the row cannot show a trial was isolated. Acceptable while overlap is off. |
| Egress (AC5 of the design on record) | **Both positions agree:** nothing is built; `--isolated` refuses wrapper profiles; 0110 asks the operator one question with a recommendation. AC-L4 stays open and blocks activation however green the hostile proof goes. | Cost: no real model driver can run isolated yet. |
| Live baseline of the leak | **Simplicity:** one always-on test makes an idle victim pass through a same-uid helper that reads the victim agent's environment from `/proc`, then uses real MCP and the real fixture. Runs on every host, no namespace needed. | Cost: a test that documents the attacker able to win, in the repository, on purpose — without it a later denial is cooperating-oracle evidence. |
| Skip discipline | `#[ignore = "trial-isolation: needs unprivileged user namespaces"]` is the one honest skip cargo has. Run explicitly, each such test probes first with a real `trial --isolated` null-agent run; if that refuses, it prints `SKIP trial-isolation: <reason>` and **fails**. | Intake C5's "records status unavailable in its evidence" is narrowed to that line plus the probe's harness-error row; no sidecar. A test that prints SKIP and returns `ok` is the fake pass the operator forbids. |

## Required behaviour

**Entry.** `swarmail-evals trial --isolated …` (hidden flag) validates its
request exactly as today, then, still single-threaded and before any Tokio
runtime exists, acquires the host admission and enters the boundary. On
success the init process builds the runtime and runs the unchanged `Trial`
arm. On any failure, in either process, the trial prints one `refused_row`
harness-error row and exits 2 with the reason
`boundary_unavailable:<operation>:<ERRNO>` on stderr, before any server or
agent exists. The errno is the name from a fixed table (0075 safe
diagnostics: finite categories and numeric facts). There is no fallback path.

**Refused before any namespace work**, each by name:
`isolation_requires_swarmail_bin` (`SWARMAIL_BIN` unset),
`isolation_artifacts_unsupported` (`--artifacts-dir`),
`isolation_wrapper_unsupported` (any `--wrapper-profile`),
`isolation_expose_invalid` (an exposed path that is not an absolute regular
file), and `boundary_unavailable:platform:ENOSYS` on non-Linux targets.
`concurrency_unsupported` and `concurrent_trials_refused` keep their existing
meaning and precede all of these.

**Boundary, in order, inside the init:** identity uid/gid maps with
supplementary groups denied; mounts made private; the bounded tmpfs root with
the allowlisted read-only binds, the named executables at their own paths and
a fresh `/proc` mounted while the host `/proc` is still visible; pivot into
the new root and detach the old one; `lo` up; descriptors from 3 closed;
no-new-privs; capabilities emptied including the bounding set; nondumpable;
parent-death signal with a parent re-check; the self-check; signals
unblocked. Every step that fails names itself and its errno.

**Controller.** Holds the host admission, blocks `SIGTERM`/`SIGINT`/`SIGCHLD`,
forwards `TERM`/`INT` to the init, reaps it, and ends the same way it ended:
exit code for an exit, re-raised signal for a signal. It never builds a
runtime and never writes a row; the init prints the row to the inherited
stdout. Agents never share that descriptor: the runner already pipes every
child's stdout and stderr.

**Inside the boundary nothing else changes.** Same catalogue, scorer, fixture
identity rule, failure precedence, `confinement:"unconfined"`, cleanup owner
and admission-through-disposal discipline (0076 §3). The trusted initiating
user, the controller, the kernel and any unisolated host process stay outside
the hostile-agent threat model; agents launched inside sibling trials are
inside it. Timing correlation through the host clock is a covert channel,
not an access path, and is accepted. A kernel bug defeats all of this; the
boundary is never sold as a defence against one.

**Matrix.** Untouched: sequential, unisolated, refusing concurrency as today.

**Documentation.** `evals/README.md` and the #151 run plan
(`evals/run-plan.md`, present on main) gain a dated section stating the
hidden flag, what it refuses, the host proof command, and that trials remain
sequential and that no row claims isolation.

## What stays outstanding after this run, and how to prove it

These need a host or CI runner that permits unprivileged user namespaces, or
the operator. The run must not report complete while they are open.

| Outstanding | Owner | How it is settled |
| --- | --- | --- |
| AC-L1..L3 hostile live proof | host/CI | the command below, green |
| AC-L4 provider egress | operator ruling on 0110's question, then a host proof | a negative test that the chosen path cannot reach the host fixture port or a sibling address |
| AC-O1 operator ruling on 0110 | operator | acceptance or rejection recorded in `docs/decisions/` |
| Concurrency re-enabled | a later change after AC-L1..L4 and AC-O1 | a new row version chosen then; not this run |

The live proof command, from the repository root on a capable host:

```bash
cargo build --locked --bin swarmail
cd evals && SWARMAIL_BIN="$PWD/../target/debug/swarmail" \
  cargo test --locked --test trial_isolation -- --ignored --nocapture --test-threads=1
```

The test's own probe is the only precondition. `unshare -Urn true` exiting 0
is a hint, not a check: it creates no mount-table or PID-namespace state and
can succeed where a `/proc` mount or a bind remount is later denied. On a host
with `kernel.apparmor_restrict_unprivileged_userns=1` the operator lifts that
restriction the way the port-80 job lowers the port floor, with a sysctl. No
GitHub runner has been observed by this run; this spec does not promise CI
works. Wiring a serialized, namespace-capable CI job is #201's surface and
the required checks are the operator's. Eight parallel jobs each creating user
namespaces would plausibly hit the very `ENOSPC` this seat saw.

## Acceptance Criteria

Criteria marked **S** are proven in this seat on every host. Criteria marked
**L** are live namespace proofs: `#[ignore]` with the named reason, red when
asked for and unavailable, outstanding until the host proof above passes.
**O** is the operator's.

- **AC-S1 — The leak is live, not alleged.** An always-on test holds an idle
  signup victim trial; a bare same-uid helper, given only the victim runner's
  PID, finds the victim agent among that runner's descendants, reads its
  `EVAL_*` environment from `/proc`, completes signup through real MCP and the
  real fixture, acknowledges, and the victim is released. The victim row is
  `pass`. Discovery is limited to the victim runner's descendants, never other
  sessions. This is AC2 of the design on record, established on every host.
- **AC-S2 — Refusal before launch, through a seam.** The trial entry takes the
  boundary entry as a function parameter. A unit test passes one that fails
  with `clone`/`ENOSPC`; the emitted row is `harness_error`, stderr names
  `boundary_unavailable:clone:ENOSPC`, exit is 2, and the agent's sentinel
  file was never created. No environment variable or mock selects this.
- **AC-S3 — Precondition refusals are real CLI runs.** `trial --isolated`
  without `SWARMAIL_BIN`, with `--artifacts-dir`, with a `--wrapper-profile`,
  and with an `--isolated-expose` that is not an absolute regular file each
  emit one harness-error row with the named reason and exit 2, and no agent
  starts. `--isolated --concurrency 2` refuses `concurrency_unsupported`
  first.
- **AC-S4 — Pure parts are unit-tested.** The errno-name table maps every
  errno the boundary can report and nothing else; the bind allowlist resolves
  exactly the named sources and refuses a missing, relative, non-regular or
  symlinked executable; the step order is a fixed list that places the
  parent-death signal after the last credential change and the `/proc` mount
  before the pivot.
- **AC-S5 — Concurrency stays off.** Every existing refusal test in
  `evals/src/runner.rs` and `evals/tests/` stays green; `git diff main --stat
  -- src/` is empty; no production path reads a proof record; every row still
  carries `concurrency: 1`.
- **AC-S6 — Skip discipline.** Every namespace-dependent test is
  `#[ignore = "trial-isolation: needs unprivileged user namespaces"]`. When
  run explicitly it probes with a real null-agent `trial --isolated`; on
  refusal it prints `SKIP trial-isolation: boundary_unavailable:<op>:<ERRNO>`
  and fails. It never passes without the boundary. The gate inventory never
  passes `--ignored`.
- **AC-S7 — Gates.** Every command in `docs/house-rules.md` § Gates passes
  locked, plus the standalone evals `fmt`, `clippy --all-targets` and
  `cargo test --locked`. CI-owned obligations are not claimed. No new
  dependency; `evals` already depends on `libc =0.2.190`.
- **AC-S8 — Authority.** `docs/decisions/0110-trial-isolation.md` is
  committed with status `proposed` and an index row; accepted decisions,
  the dependency allowlist, gate scripts and review routing are unchanged;
  `git diff main -- docs/decisions/0055-agent-eval-harness.md` and
  `0076-sequential-eval-trials.md` are empty.
- **AC-L1 — The boundary holds against the same helper.** Victim A runs
  `trial --isolated` with `/usr/bin/sleep` held under the deadline; the
  trusted test reads A's coordinates from the host `/proc`. Trial B runs
  `trial --isolated` with the AC-S1 helper as its agent and A's coordinates in
  argv. The helper attempts discovery and direct access to A's prompt, HOME
  sentinel, `/proc` entries, MCP port and fixture URL and reports through its
  exit status: 0 means every route was denied, N names the first route that
  worked. The test then kills A's agent from outside; A's row is `fail` in
  signup's declared idle class with no accepted confirmation, never `pass`,
  `budget` or `harness_error`. A passing victim turns this test red.
- **AC-L2 — Fresh HOME and own services through the boundary.** A real child
  observes a fresh writable HOME; a sentinel from the previous isolated trial
  is absent; the oracle passes signup inside the boundary through its own
  MCP and fixture.
- **AC-L3 — Lifetime is bound on both ends.** Killing the controller leaves no
  trial process alive on the host; killing the init leaves no agent alive;
  an interrupted trial emits its interrupted row and the controller ends with
  the same signal. No orphan from trial A is alive when trial B starts.
- **AC-L4 — Driver egress.** Open. The operator's ruling on 0110's question,
  then a negative proof that the chosen transport cannot reach the host
  fixture port or a sibling trial's address. Offline oracle passes alone do
  not satisfy it.
- **AC-O1 — Operator ruling on 0110.** Outstanding by construction.

No push, merge, tag, publication, issue closure or decision acceptance belongs
to this work. Drafting this design is not implementation success.
