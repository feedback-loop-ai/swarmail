# Isolate concurrent eval trials — the boundary is bubblewrap

Date: 2026-11-01 (revision of the 2026-10-31 design; the first design commit
and its files stay in history). Issue #221 (split from #199), Epic #192.
Branch `repair/issue221-trial-isolation`, main a4f552d merged in.
Lane: `claude-careful` (accepted 0098/0099; residual rework under accepted
0102 and its spec-defect addendum).
Operator direction, 2026-11-01: **redesign the boundary around bubblewrap.**

Decision: 0115 — [Trial separation and observed concurrency](../../docs/decisions/0115-trial-isolation.md)
(proposed; its dated *Revision 2026-11-01* section governs where it disagrees
with the 2026-10-24 ruling and the 2026-10-31 integration).

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
refuse `boundary_unavailable:identity_map:EACCES`, and the code has never
executed anywhere. bubblewrap (`bwrap`) traverses that restriction through
its AppArmor profile (`bwrap-userns-restrict`): the dispatcher's out-of-seat
probe on this host, with `--unshare-user --unshare-pid --unshare-net
--unshare-ipc --die-with-parent`, gave uid 1000, an empty private `/tmp`,
loopback only and pid 2, and processes inside cannot create further user
namespaces. This revision keeps the boundary's **contract** and replaces its
**mechanism**: the trial's controller invokes `/usr/bin/bwrap` with a fixed
argv, and the kernel and bwrap own the namespaces. The raw-syscall worker is
deleted.

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

## Panel synthesis — adopted, rejected, reconciled

| Question | Ruling | Losing side and its cost |
| --- | --- | --- |
| Mechanism | **Both:** invoke bubblewrap; delete `evals/src/isolation/linux.rs`, the errno table, the step order, the remount arithmetic and the in-init self-check. | None. About nine hundred lines nobody can run are removed. |
| Locating and pinning bwrap | **Both:** the constant absolute path `/usr/bin/bwrap`, floor `0.11.1`, no `PATH` lookup, no environment or flag override. Tests reach other binaries only through a function parameter. | A path override. Cost: an operator with bwrap elsewhere gets a named refusal and must file a decision, not set a variable. |
| The argv | **Simplicity's shape, robustness's rules:** explicit `--unshare-*` (never `--unshare-all`, never a `-try` unshare), `--die-with-parent`, `--new-session`, the existing allowlist of read-only system trees and executables at their own paths, a size-bounded `--tmpfs /tmp`, and **never `--ro-bind / /`**. The dispatcher's probe proved capability, not a boundary: a read-only host root exposes `/run/user/<uid>` (the forwarded ssh-agent, gpg-agent and D-Bus sockets are connectable on a read-only mount), the operator's HOME and credentials. | Robustness's `/run/*` executable normalization and minimal `/etc` subset. Cost of the ruling: the sandbox learns the checkout path string (nothing at it is reachable but the bound files); a whole read-only `/etc` is what the unisolated trial already sees and keeps NSS, TLS roots and locale working without seat-untestable guesses. |
| Environment | **Robustness:** `--clearenv` then exactly `PATH`, `HOME`, `TMPDIR`, `SWARMAIL_BIN`. The runner reads nothing else when `SWARMAIL_BIN` is set (toolchain discovery is the only `HOME`/`CARGO_*` reader and is refused under `--isolated`); agents already get a closed environment from the runner (0076 §4). | Simplicity's pass-through. Cost: a test hook that travels by environment variable does not reach an isolated runner; the live proofs use exit codes and rows, not hooks. |
| In-boundary self-check | **Simplicity:** none, and no hidden `--in-boundary` marker. A self-check is the trial grading itself; 0115 already says namespace flags are not evidence. The topology is **observed** from inside by a scripted agent in the live proof (AC-L2) and **attacked** from a sibling in AC-L1. | Robustness's self-check. Cost: a wrong argv on a host nobody has probed is caught by the live proof, not by every production launch. Accepted while the argv is a golden-tested constant and bwrap refuses rather than skips an unavailable namespace. |
| Nested user namespaces | **Simplicity:** no `--disable-userns`/`--assert-userns-disabled`. Under the Ubuntu profile nesting is already denied; on this host the flags are an untested interaction with that profile; a nested namespace sees nothing outside its own trial. AC-L2 observes and reports whether `unshare(CLONE_NEWUSER)` succeeds inside, without asserting. | Robustness. Cost: on a host without the profile a hostile agent can own a nested namespace inside its own trial; that is within-trial, which 0115 ruling 6 already calls unconfined. The operator may add the flag after the first live run shows it traverses the profile. |
| Preflight | **Robustness, narrowed to one stage:** after the request refusals and before the host admission, the controller runs the exact production argv with `/usr/bin/true` as the command, bounded by a deadline. Failure refuses `boundary_unavailable:bwrap_probe` before any server or agent exists. The launch stays authoritative. | Simplicity's "judge by stdout bytes only". Cost of the ruling: one extra bwrap spawn per isolated trial (milliseconds). Gain: a boundary outage and an inner crash are different named harness errors. |
| Lifetime chain | **Simplicity:** `--die-with-parent` already kills the sandbox when bwrap **or bwrap's parent** dies (bwrap 0.11.1 `--help` says so verbatim); no controller-side `PR_SET_PDEATHSIG`. The controller spawns bwrap on its main thread (parent-death signals follow the forking thread) with `process_group(0)` and `kill_on_drop`. Killing the runner ends bwrap's PID-1 reaper, and the kernel kills everything left in the PID namespace. | Robustness item 6. Cost: none proven; AC-L3 is the evidence either way. |
| Interrupts | **Simplicity, made explicit:** bwrap does not forward signals. On `SIGTERM`/`SIGINT` the controller finds the runner's **host** PID (the descendant of bwrap whose `/proc/<pid>/exe` is the runner executable) and sends it one `SIGTERM`; the runner emits its own interrupted row and cleans up exactly as an unisolated trial does (0073). If no runner exists yet, the controller kills bwrap and emits the `isolated_trial_interrupted` harness-error row. | Robustness's "forward to bwrap and prove it forwards". Cost: one `/proc` walk in the controller; no `--as-pid-1`, `--info-fd`, `--json-status-fd` or signal protocol. |
| Admission | **Both:** the controller acquires the host admission (0076 §1) after the probe and holds it until bwrap is reaped. The inner runner's admission on its private `/tmp` is always granted and decorative, documented as such. | None. |
| Descriptors | **Both:** `stdin` null; `stdout` and `stderr` piped and copied byte for byte to the controller's own; a `pre_exec` `close_range(3, ~0, CLOSE_RANGE_CLOEXEC)` sweep so nothing above 2 survives the exec. The live proof opens a canary descriptor in the controller and the inside observer must not see it. | None. |
| Row schema, `execution` object | **Rejected for this run** (0115 Current integration 1–2, unchanged). `confinement` stays `"unconfined"`. | Cost: the row cannot show a trial was isolated. Acceptable while overlap is off. |
| Provider egress | **Both:** nothing is built. `--share-net` is rejected outright (it puts the trial's Swarmail and fixture on the host loopback). `pasta`/`slirp4netns` and the controller-side CONNECT relay each cost a new binary or relay code plus a descriptor that survives the sweep; the operator chooses or declines. `--isolated` refuses every `--wrapper-profile`. AC-L4 stays open and blocks activation. | Cost: no real model driver can run isolated yet. |
| CI | **Both:** no `.github/` change in this run (held for the operator under 0056's addendum). CI proves the always-on tests on ubuntu and the `platform` refusal on macOS; whether a hosted ubuntu runner has bwrap, which version apt gives it, and whether its policy permits the namespaces are **unobserved**. | Robustness's serialized fail-closed job. Cost: CI never proves the boundary until the operator adds that job; the host command is the only live proof. |
| Live proof arrangement | **Simplicity:** victim A is launched by the trusted test through the library's own argv builder (the test is A's controller; no host admission); hostile B goes through the real `trial --isolated` CLI and takes the real host admission. The dead `unshare`/`setpriv` wrapper is deleted. **Robustness's correction adopted:** under per-trial network namespaces A and B may legitimately share port numbers; the probe's "own service equals victim's port" void is real and the test retries a void at most twice, by name. | None lost; nested bwrap is refused by the profile, so no other arrangement runs here. |

## Required behaviour

**Entry.** `swarmail-evals trial --isolated …` (hidden flag) validates its
request exactly as today. The precondition refusals keep their names and
order: `concurrency_unsupported`, then `isolation_requires_swarmail_bin`,
`isolation_artifacts_unsupported`, `isolation_wrapper_unsupported`,
`isolation_expose_invalid`, `isolation_source_invalid`. On any refusal the
trial prints one `refused_row` harness-error row and exits 2 before any
server or agent exists. There is no fallback path.

**Boundary refusals, a closed set.** Each is `boundary_unavailable:<step>`
with `<step>` one of:

- `platform` — any non-Linux target. **macOS refuses by name**, since bwrap
  is Linux-only; CI's macOS `test` job proves it live.
- `bwrap_absent` — `/usr/bin/bwrap` is missing, not a regular file, or not
  executable.
- `bwrap_version` — `/usr/bin/bwrap --version` did not finish within the
  bound, did not print exactly `bubblewrap <major>.<minor>.<patch>` (trailing
  text ignored), or is below `0.11.1`.
- `bwrap_probe` — the exact production argv with `/usr/bin/true` as the
  command did not exit 0 within the bound. bwrap's stderr is forwarded
  unchanged so the operator sees the kernel's reason (`ENOSPC`, `EACCES`,
  `EPERM`); it is never interpolated into the row (0075 finite categories).

The errno-name table is deleted: bwrap reports its own errors in its own
words on stderr. The version floor is `0.11.1` because it is the only
version the boundary has been observed to work on (the dispatcher's probe
and the proof command run on it); every flag used is older than 0.9.
Lowering the floor (Ubuntu 24.04 packages 0.9.0) is an ordinary one-line
change that must carry a live proof on that version. The floor is a
compatibility statement, not a security attestation of a distribution's
patch level. The version and probe are checked on every launch, never
cached.

**The argv** is one pure function over the validated request, golden-tested:

```
/usr/bin/bwrap
  --unshare-user --unshare-pid --unshare-net --unshare-ipc
  --die-with-parent --new-session
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

Implementation note, 2026-11-01 (implement seat): the private `/tmp` is
mounted **before** the file binds, not after as first drawn. bwrap applies
options in order, so a tmpfs mounted last hides any bound file that lives
below `/tmp` on the host (a test wrapper, an operator's agent script); mounted
first, such a file is bound into the sandbox's own tmpfs. The argv is otherwise
as drawn, and the golden test records this order.

Rules: a system tree that is a symlink on the host is recreated as the same
symlink, a missing one is skipped (today's `SYSTEM_ROOTS` semantics, now
rendered as bwrap options); every bind is read-only and bwrap makes it
`nosuid`; the first `--` is at a fixed position and everything after it is
runner-owned, so no agent argument can become a bwrap option; the inner
request is re-rendered from the parsed `Cmd::Trial` fields (never by
filtering the raw argv, which would strip an agent's own `--isolated`) and
proven by a clap round trip over a request with every field populated; the
agent program is the absolute path the controller resolved (today's
`adopt`). The `/tmp` bound is the 512 MiB the design on record used; an
`ENOSPC` inside is the runner's own harness error, never a pruned inbox.

**The controller** is the `swarmail-evals` process the operator or matrix
started. It stays in the host namespaces and never runs the trial arm:

1. refuses the preconditions above, synchronously, as today;
2. checks `platform`, `bwrap_absent`, `bwrap_version`, then runs the
   `bwrap_probe`, each bounded (5 s for version, 10 s for the probe);
3. acquires the host admission (0076 §1; `concurrent_trials_refused` as
   today) and holds it until bwrap is reaped — the structural form of 0076
   §3's admission-through-reaping, since bwrap's PID-1 reaper exits only
   after every process in the namespace is gone;
4. spawns bwrap from its main thread with `process_group(0)`,
   `kill_on_drop`, null stdin, piped stdout and stderr, and the `close_range`
   CLOEXEC sweep in `pre_exec`; copies both pipes to its own stdout and
   stderr;
5. on `SIGTERM`/`SIGINT`, signals the runner as described in the synthesis
   table, once;
6. when bwrap exits: if at least one byte reached stdout, the controller
   exits with bwrap's exit code (bwrap propagates the runner's); if none did,
   it prints the `isolated_trial_no_row` harness-error row and exits 2. If
   bwrap died by a signal, exit 2.

The inner process is the same binary running the unchanged `trial` arm: it
takes its decorative admission on the private `/tmp`, builds its runtime and
runs the trial. `SWARMAIL_BIN` arrives by `--setenv`; the controller's
`unsafe set_var` is gone.

**Inside the boundary nothing else changes.** Same catalogue, scorer, fixture
identity rule, failure precedence, `confinement:"unconfined"`, cleanup owner
and admission-through-disposal discipline (0076 §3). The trusted initiating
user, the controller, the kernel, bwrap and any unisolated host process stay
outside the hostile-agent threat model; agents launched inside sibling trials
are inside it. A hostile agent still shares the host kernel and can still
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
added (`libc` is already pinned in `evals/` for `kill` and `close_range`).
0115 states the prerequisite for the operator: Linux, `/usr/bin/bwrap` ≥
0.11.1, and a host policy that lets bwrap create user, PID, network and IPC
namespaces (Ubuntu's `bwrap-userns-restrict` profile, or the sysctl
lowered). Accepting 0115 is accepting that prerequisite.

**Matrix.** Untouched: sequential, unisolated, refusing concurrency as today.

**Documentation.** `evals/README.md` and `evals/run-plan.md` revise their
dated #221 sections: the mechanism is bwrap, the prerequisite, the closed
refusal set, the host proof command, trials remain sequential, no row claims
isolation.

## What stays outstanding after this run, and how to prove it

| Outstanding | Owner | How it is settled |
| --- | --- | --- |
| AC-L1..L3 hostile live proof | the dispatcher, on this host, after the run | the command below, green |
| AC-L4 provider egress | operator ruling on 0115's question, then a host proof | a negative test that the chosen path cannot reach the host fixture port or a sibling address |
| AC-O1 operator ruling on 0115 (including the bwrap prerequisite) | operator | acceptance or rejection recorded in `docs/decisions/` |
| CI boundary job | operator (`.github/` is held) | a serialized ubuntu job that installs a pinned bubblewrap, checks the floor and policy, runs the same command and fails closed |
| Concurrency re-enabled | a later change after AC-L1..L4 and AC-O1 | a new row version chosen then; not this run |

**The live proof command**, from the repository root on this host:

```bash
cargo build --locked --bin swarmail
cd evals && SWARMAIL_BIN="$PWD/../target/debug/swarmail" \
  cargo test --locked --test trial_isolation -- --ignored --nocapture --test-threads=1
```

It is deterministic (null and oracle builtins, `/usr/bin/sleep`, the
scripted probe and observer from the test executable; no provider, no
network beyond the private loopbacks; no paid call), bounded (every isolated
trial carries `--deadline-ms` of at most 30 000; every child is held by
`ChildGuard`; the whole command completes in under five minutes on this
host) and self-describing (each test probes first and fails by name when the
boundary is unavailable). **AC-L1, AC-L2 and AC-L3 are proven only by that
command on a host whose bwrap can create the namespaces.** The seat cannot
run it: `/usr/bin/bwrap` 0.11.1 is present in the seat and refuses with
`ENOSPC` (the seat is already inside a user namespace), so there every live
proof prints `SKIP trial-isolation: boundary_unavailable:bwrap_probe` and
fails, as AC-S6 requires.

## Acceptance Criteria

Criteria marked **S** are proven in this seat on every host. Criteria marked
**L** are live boundary proofs: `#[ignore]` with the named reason, red when
asked for and unavailable, outstanding until the host command above passes.
**O** is the operator's.

- **AC-S1 — The leak is live, not alleged.** Unchanged and kept: an always-on
  test holds an idle signup victim; a bare same-uid helper, given only the
  victim runner's PID, finds the victim agent among that runner's
  descendants, reads its `EVAL_*` environment from `/proc`, completes signup
  through real MCP and the real fixture, and the victim row is `pass`. The
  negative control for the probe (routes 10, 91, 30) stays always on.
- **AC-S2 — Refusal before launch, through a parameter seam.** The launch
  takes the bwrap path as a function parameter; no environment variable,
  flag or cfg selects it. Unit tests pass, in turn: an absent path (refuses
  `bwrap_absent`); a generated script printing `bubblewrap 0.9.0` (refuses
  `bwrap_version`); a script printing `bubblewrap 0.11.1` that writes
  `bwrap: …` to stderr and exits 1 for anything else (refuses
  `bwrap_probe`); a script that passes the version and the probe and then
  exits 1 with nothing on stdout (`isolated_trial_no_row`). In every case
  the row is `harness_error`, exit is 2, stderr names the reason, no
  admission was taken before the probe passed, and the agent's sentinel file
  was never created.
- **AC-S3 — Precondition refusals are real CLI runs.** Unchanged:
  `trial --isolated` without `SWARMAIL_BIN`, with `--artifacts-dir`, with a
  `--wrapper-profile`, and with an `--isolated-expose` that is not an
  absolute regular file each emit one harness-error row with the named
  reason and exit 2, and no agent starts. `--isolated --concurrency 2`
  refuses `concurrency_unsupported` first.
- **AC-S4 — Pure parts are unit-tested.** The version parser accepts
  `bubblewrap 0.11.1`, `bubblewrap 0.11.1-ubuntu1` and `bubblewrap 1.0.0`,
  refuses `bubblewrap 0.11.0`, `bubblewrap 0.9.0`, an empty string and
  garbage. The golden argv for a request with an agent and one exposed file
  is exactly the list above: no `/` bind, no `-try` or `--unshare-all`, no
  `--share-net`, `--clearenv` before every `--setenv`, `--size` immediately
  before `--tmpfs /tmp`, the first `--` immediately before the runner
  executable. A property test over agent argv containing `--ro-bind`,
  `--share-net`, `--` and `--isolated` shows none of them precede the first
  `--`. The inner-args clap round trip over a request with every
  `Cmd::Trial` field populated yields the same request minus `isolated` and
  `isolated_expose`, with the agent program absolute; the renderer
  destructures `Cmd::Trial` without `..` so a new field fails to compile
  until it is rendered. The bind allowlist tests (`resolve_source`,
  `bind_plan`) stand as they are.
- **AC-S5 — Concurrency stays off.** Every existing refusal test in
  `evals/src/runner.rs` and `evals/tests/` stays green; `git diff main --stat
  -- src/` is empty; no production path reads a proof record; every row still
  carries `concurrency: 1`.
- **AC-S6 — Skip discipline.** Every boundary-dependent test is
  `#[ignore = "trial-isolation: needs /usr/bin/bwrap >= 0.11.1 able to create user namespaces"]`.
  When run explicitly it probes with a real null-agent `trial --isolated`;
  on refusal it prints `SKIP trial-isolation: boundary_unavailable:<step>`
  and **fails**. It never passes without the boundary. The gate inventory
  never passes `--ignored`. The `platform` refusal is proven by a
  `cfg(not(target_os = "linux"))` test that runs in CI's macOS job only, and
  by a Linux unit test of the platform-neutral refusal function.
- **AC-S7 — Gates.** Every command in `docs/house-rules.md` § Gates passes
  locked, plus the standalone evals `fmt`, `clippy --all-targets` and
  `cargo test --locked`. CI-owned obligations are not claimed. No new
  dependency; neither lockfile nor `deny.toml` changes.
- **AC-S8 — Authority.** `docs/decisions/0115-trial-isolation.md` is
  committed with status `proposed`, its index row unchanged; its *Revision
  2026-11-01* section names the bwrap prerequisite, the closed refusal set,
  the macOS refusal, the unobserved CI facts and the egress question.
  Accepted decisions, the dependency allowlist, gate scripts, `.github/`
  and review routing are unchanged; `git diff main --
  docs/decisions/0055-agent-eval-harness.md
  docs/decisions/0076-sequential-eval-trials.md` is empty.
- **AC-S9 — The raw worker is gone.** `evals/src/isolation/linux.rs`, the
  errno-name table, `Step`/`STEPS`, `remount_flags`, `Observed`/`verify`,
  the `Enter` fn-pointer seam and the test-side `with_private_admission`
  wrapper no longer exist; `grep -rn "pivot_root\|SYS_clone\|uid_map" evals/src`
  is empty.
- **AC-L1 — The boundary holds against the same helper.** Victim A is an
  isolated `/usr/bin/sleep` signup trial launched by the trusted test
  through the library argv builder, held under the deadline; the test reads
  A's coordinates from the host `/proc` (same uid, host namespaces). Trial B
  runs the real `trial --isolated` with the AC-S1 probe as its agent (the
  test executable bound by `--isolated-expose`) and A's coordinates in argv.
  The probe tries, in order: discovery of any `/proc` process carrying A's
  environment (10), A's runner and agent `/proc` entries (11), A's prompt
  file (12), A's HOME (13), A's MCP authority (14), A's fixture authority
  (15), and completing A's signup (16); exit 0 means every route was denied.
  A port or path coincidence with B's own services voids the run (90); the
  test retries a void at most twice and fails by name on the third. The
  test then kills A's agent from outside; A's row is `fail` in signup's
  declared idle class with no accepted confirmation, never `pass`, `budget`
  or `harness_error`. A passing victim turns this test red.
- **AC-L2 — Fresh HOME, own services, and the observed topology.** A real
  child observes a fresh writable HOME; a sentinel from the previous isolated
  trial is absent; the oracle passes signup inside the boundary through its
  own MCP and fixture. A scripted observer agent, run as an isolated trial's
  agent, reports by exit code the first violated check among: uid is the
  caller's; `/proc` lists no host PID; `/proc/net/dev` lists exactly `lo` and
  a connect to a port the observer itself bound succeeds; `/home`, `/root`,
  `/run/user`, `/var/tmp` and a canary file the test created under the host
  `/tmp` are absent; `/tmp` is a tmpfs of the bounded size; the environment
  holds no key beyond the four set by the argv and those the runner sets; no
  descriptor at or above 3 is open beyond its own stdio and the canary the
  controller held is absent; `CapEff` is zero and `NoNewPrivs` is 1. It also
  records, without asserting, whether `unshare(CLONE_NEWUSER)` succeeds
  inside.
- **AC-L3 — Lifetime is bound on both ends.** `SIGKILL` to the controller
  leaves no trial process alive on the host; `SIGKILL` to the runner (from
  the host) leaves no agent alive and the controller ends with exit 2 and
  the `isolated_trial_no_row` row; `SIGTERM` to the controller produces the
  runner's own interrupted row, exit 2 and no survivors; no orphan from trial
  A is alive when trial B starts.
- **AC-L4 — Driver egress.** Open. The operator's ruling on 0115's question,
  then a negative proof that the chosen transport cannot reach the host
  fixture port or a sibling trial's address. Offline oracle passes alone do
  not satisfy it.
- **AC-O1 — Operator ruling on 0115.** Outstanding by construction; now
  includes the host prerequisite.

No push, merge, tag, publication, issue closure or decision acceptance belongs
to this work. Drafting this design is not implementation success.

## History — what the 2026-10-31 implementation left on the branch

Built and green on every host: AC-S1, AC-S3, AC-S5 and the probe's negative
control. Built and never executed: `evals/src/isolation/linux.rs` (the
clone/`pivot_root` body), the errno table, step order, remount arithmetic,
`Observed`/`verify`, the `Enter` seam, and the test-side `unshare`/`setpriv`
private-admission wrapper. Written and never run: AC-L1..L3. Observed on the
host after that run: `boundary_unavailable:identity_map:EACCES` under
`kernel.apparmor_restrict_unprivileged_userns=1`. This revision deletes the
never-executed parts and re-targets the live proofs at bwrap; the earlier
design commit remains in history as the spec that code was built to.
