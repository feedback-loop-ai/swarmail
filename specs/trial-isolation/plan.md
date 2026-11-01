# Trial isolation implementation plan — the boundary is bubblewrap

Implements [spec.md](spec.md) as revised 2026-11-01; semantic activation is
governed by proposed [0115](../../docs/decisions/0115-trial-isolation.md)
and its *Revision 2026-11-01* section. This run replaces the never-executed
raw-syscall worker with a controller that invokes `/usr/bin/bwrap`, and
re-targets the live proofs. Concurrency stays off; no row version changes;
no production path reads a proof record. The 2026-10-31 plan stays in git
history; this file supersedes it.

## Technical context

A standalone Rust 2024 workspace, `evals/`, outside the measured root crate
and its coverage denominator (`evals/Cargo.toml`, operator ruling 2026-10-18,
decision 0055). Tokio subprocesses (the `process` and `signal` features are
already enabled), an Axum fixture, reqwest and real SMTP/MCP. `evals` already
depends on `libc =0.2.190`; the controller uses it for `close_range`,
`kill` and `geteuid` only. **No new dependency, workspace member or license
entry.** bubblewrap is a host prerequisite executed as a separate process: it
is never linked, vendored, downloaded or added to a lockfile, and `cargo deny`
neither sees it nor needs to.

What the seat observed on 2026-11-01: `/usr/bin/bwrap` is `bubblewrap
0.11.1`, mode `0755`, not setuid; `kernel.apparmor_restrict_unprivileged_userns`
reads 1; `/proc/self/uid_map` is `1000 0 1` (the seat is inside a user
namespace already); the production argv with `/bin/sh` as the command fails
with `bwrap: Creating new namespace failed: nesting depth or
/proc/sys/user/max_*_namespaces exceeded (ENOSPC)`, exit 1, nothing on
stdout. bwrap 0.11.1 lists `--die-with-parent` as "Kills with SIGKILL child
process (COMMAND) when bwrap or bwrap's parent dies", plus `--new-session`,
`--clearenv`, `--setenv`, `--size`, `--tmpfs`, `--chdir`, `--symlink`,
`--disable-userns` and `--assert-userns-disabled`. So the controller compiles
and its pure parts run here; the probe refuses `bwrap_probe` here; the live
path runs only on the host.

There is no plan template in this checkout; the spec-kit sections (research,
data model, contracts, quickstart) are inline. The design commit is exactly
this spec/plan/tasks and the revised proposed decision. Implementation and
documentation edits belong to later commits, not the chief's HEAD.

## Constitution check

Preserve accepted 0001–0005 (the spec says how each survives the boundary),
0055's app-state authority and real-subprocess path, 0071 trap precedence,
0073 matrix cancellation, 0075 safe diagnostics and closed environments, and
all of 0076. Accepted 0045 §8 separates compatible proof from contradictory
activation: overlapping trials, a new row version and any egress change wait
for operator acceptance of 0115. Root exact coverage stays 100% with zero
missed and no `coverage(off)`; this work does not touch `src/`, so the root
report is unchanged and `git diff main --stat -- src/` must stay empty. Apply
0020: deterministic readiness, generated fixtures, real protocols, chaos at
0/100 only, no sleep as synchronization (the live proofs use deadlines,
bounded `eventually` polls and process identities, never bare sleeps).
Review is the accepted 0098/0099 panel in a later phase. Commit unsigned under
0056; no push, merge or publication. `.github/`, `scripts/`, `.brokkr/`,
`deny.toml` and both lockfiles are untouched.

## Research and decisions

**Why bwrap and not the raw worker.** The worker is dead on this host by
`kernel.apparmor_restrict_unprivileged_userns=1`
(`boundary_unavailable:identity_map:EACCES`), the Ubuntu-family default. bwrap
carries an AppArmor profile that permits exactly the namespaces the boundary
needs, and it is distribution-maintained code: it maps the uid, builds a
tmpfs root, mounts `/proc` and `/dev`, marks binds `nosuid`, brings `lo` up,
runs a PID-1 reaper (which also removes the recorded zombie limit) and ties
the sandbox's life to its parent's. Keeping both mechanisms would be two
refusal vocabularies and a fallback temptation, so the worker is deleted.

**Why the probe argv is not the boundary.** `--ro-bind / /` makes the host
root visible read-only, and Linux allows `connect()` to a unix socket on a
read-only mount: `/run/user/<uid>` carries the forwarded ssh-agent, gpg-agent
and the session bus. The boundary binds the allowlist the branch already
resolves (`SYSTEM_ROOTS`, `resolve_source`, `bind_plan`, `--isolated-expose`).

**Why one preflight probe.** Judging the launch by "did a row arrive" alone
conflates a boundary outage with an inner panic. One run of the exact argv
with `/usr/bin/true`, bounded, before admission, names the outage
(`bwrap_probe`) before any server or agent exists; afterwards a silent exit
is a different named harness error (`isolated_trial_no_row`). Three stages
(exec, `--unshare-user -- true`, full argv) were cut: the full argv subsumes
the others and the extra names add nothing an operator can act on.

**Why no in-boundary self-check.** It is the trial grading itself, needs an
"I am inside" marker (new vocabulary), and duplicates checks bwrap performs
or refuses on. The live proof observes the topology from inside through a
scripted agent and attacks it from a sibling; those are the evidence.

**Why `--clearenv`.** The runner reads `PATH`, `SWARMAIL_BIN`, and `HOME` /
`CARGO_*` only for toolchain discovery, which `--isolated` refuses
(`isolation_requires_swarmail_bin`). Agents already get the closed
`RuntimeDirs` environment (0076 §4). So the sandbox needs exactly four
variables, and clearing the rest keeps the operator's host environment out of
every process inside at no cost to the runner.

**Why signal the runner, not bwrap.** bwrap does not forward signals, and
`--die-with-parent` delivers `SIGKILL`, which loses the interrupted row. The
controller, in host namespaces, sees every sandbox process with a host PID
and the runner's `/proc/<pid>/exe` is its own executable path; one `SIGTERM`
there is what the matrix sends an unisolated trial today (0073).

**Why `--die-with-parent` alone.** Its documented semantics cover bwrap's
parent dying. Parent-death signals follow the forking thread, so the
controller spawns from the main thread inside `block_on`, never from a
`tokio::spawn`ed task; `kill_on_drop` covers the orderly path.

**Why no `--disable-userns`.** On this host nesting is already denied by the
profile; the flag is an untested interaction with that profile; a nested
namespace is within-trial. Observed in AC-L2, not asserted; the operator may
add it after the first live run.

**Why the live proof launches A through the builder.** Nested bwrap is
refused by the profile, and plain `unshare` is what AppArmor restricts, so
the 2026-10-31 `unshare`/`setpriv` wrapper cannot run here. The trusted test
is A's controller through the same pure argv function the CLI uses; B is the
production path and holds the real host admission. Nothing in production
overlaps trials (0076 §1 intact).

**Why a void is retried.** Under separate network namespaces both trials
bind ephemeral ports on their own `127.0.0.1`; a numeric coincidence makes
the probe's routes 14/15 test B's own service. The probe already reports
that coincidence as route 90 (void); the test retries at most twice and
fails by name on the third, so the proof can never pass or fail on a
coincidence.

**Provider egress** is a feasibility checkpoint, not shipped code. 0115
carries the question with the options and their cost; `--isolated` refuses
every `--wrapper-profile` so no paid driver runs isolated.

## Project structure and ownership

| File / surface | Planned responsibility |
| --- | --- |
| `evals/src/isolation/linux.rs` | **Deleted.** |
| `evals/src/isolation.rs` | Keep: the precondition names, `Refusal` (now `Named` and `Boundary { step }` with no errno), `SYSTEM_ROOTS`, `resolve_source`, `bind_plan`, `Request`, `request`; `DEVICES` goes if `--dev /dev` leaves it unused. Delete: `ERRNO_NAMES`, `errno_name`, `Step`, `STEPS`, `remount_flags`, `Observed`, `verify`, `interfaces`, `status_field`, `Enter`, `enter`. Add: `BWRAP: &str = "/usr/bin/bwrap"`, `BWRAP_MIN`, `TMP_BYTES`, `parse_version`, `system_root_options` (ro-bind or symlink per host layout), `command(&Request, inner) -> Vec<OsString>` (the pure argv), and `launch(request, inner, bwrap: &Path)` (the controller: platform, absent, version, probe, admission, spawn, copy, signal, judge). Linux-only parts sit behind a platform-neutral `launch` whose non-Linux body refuses `platform` first. |
| `evals/src/main.rs` | `prepare_trial` keeps the precondition refusals; its `enter: Enter` parameter becomes `bwrap: &Path`. For `--isolated` it no longer continues into the `Trial` arm: `main` builds the runtime and `block_on`s `isolation::launch` on the main thread, prints the refusal or no-row row through `runner::refused_row` with the named reason, and exits with the launch's code. The `unsafe set_var(SWARMAIL_BIN)` goes. `inner_args(&Cmd)` lives beside `Cmd` and destructures `Cmd::Trial` without `..`. |
| `evals/src/runner.rs` | Unchanged production logic. `refused_row` is reused for the controller-side rows (`boundary_unavailable:<step>`, `isolated_trial_no_row`, `isolated_trial_interrupted`); no new fields. |
| `evals/tests/trial_isolation.rs` | Keep AC-S1, AC-S3, the hidden-flag and concurrency checks and the probe's negative control. The injected-`Enter` refusal test moves to `main.rs` unit tests as the parameter-seam tests. Delete `with_private_admission`. Rewrite AC-L1 (A through `isolation::command`, B through the CLI, void retry), AC-L2 (add the observer agent, the host-`/tmp` canary file and the canary descriptor), AC-L3 (controller `SIGKILL`, runner `SIGKILL`, controller `SIGTERM`). `#[ignore]` reason string updated. |
| `evals/tests/common/mod.rs` | Keep `boundary_unavailable`/`require_boundary` (now classifying `boundary_unavailable:<step>`), `process_descendants`, `proc_environ`, `victim_args`, `cross_trial_probe`, `probe_route`. Add the `boundary_observe` helper mode with its exit-code table, and `launch_victim(request)` that spawns bwrap from the builder's argv with the controller's stdio discipline. |
| `evals/README.md`, `evals/run-plan.md` | Revise the dated #221 section: mechanism, prerequisite, refusal set, proof command, sequential-still, no row claims isolation. |
| `docs/decisions/0115-trial-isolation.md` | The proposed record with its *Revision 2026-11-01* section, committed by the chief with this design. Index row unchanged. |
| `specs/trial-isolation/{spec,plan,tasks}.md` | This design. |

No root `src/`, `Cargo`, gate-script, policy, `.github/` or review-routing
change. Do not refactor the matrix-cancellation (0073) or child-environment
(0075/0076 §4) blocks incidentally.

## Data model and contracts

No wire contract changes in this run. `TrialResult` stays schema 2 with
required integer `concurrency: 1`; `results::read` and the report math are
untouched. The `execution`/`boundary` object of 0115 §4 is not added to any
struct, constructor, reader or report.

The boundary's own contract is internal and pure where tested:

- **The refusal vocabulary** is closed: the six precondition names (unchanged),
  `boundary_unavailable:{platform,bwrap_absent,bwrap_version,bwrap_probe}`,
  `isolated_trial_no_row`, `isolated_trial_interrupted`. A row never carries
  bwrap's text; bwrap's stderr is copied to the controller's stderr as a
  separate stream (0075 finite categories in the row, raw diagnostics beside
  it).
- **The version parser**: `bubblewrap <major>.<minor>.<patch>` with trailing
  text ignored, compared as a triple against `(0, 11, 1)`.
- **The argv builder** `command(&Request, inner: &[OsString]) -> Vec<OsString>`:
  the fixed prefix in the spec's order, the bind options from `bind_plan` and
  the host's `/bin`/`/lib`/`/lib64` layout, then `--`, then `inner`. The
  index of the first `--` is a function of the request only, never of
  `inner`.
- **The inner renderer** `inner_args(&Cmd) -> Vec<OsString>`: `trial`, every
  `Cmd::Trial` field rendered as its clap flag, `isolated` and
  `isolated_expose` dropped, the agent program replaced by the resolved
  absolute path, then `--` and the agent argv verbatim.
- **The launch judgement**: exit code mirrored when stdout carried a byte;
  `isolated_trial_no_row` otherwise; exit 2 on a signal death of bwrap.

The controller/sandbox boundary is a process relationship, not a message
protocol: no agent-controlled string reaches bwrap's option list; the inner
runner reads only its validated, re-rendered request.

## Quickstart and validation

From the repository root, with cached dependencies:

```bash
cargo build --locked --bin swarmail
cd evals
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
SWARMAIL_BIN="$PWD/../target/debug/swarmail" cargo test --locked
```

The last command runs AC-S1 through AC-S9 on every host, with the live proofs
shown as `ignored`. The live proof is the separate `--ignored` command in the
spec, run by the dispatcher on this host after the run. Then, from the root,
every command in `docs/house-rules.md` § Gates runs locked. CI owns the chart,
Docker smoke, RustSec, tag-rate and port-80 proofs; no CI change is made here.

Design-stage validation is structural (`design-check.sh`): exactly one touched
`specs/<slug>/`, its three files committed, non-empty and clean, an acceptance
heading in spec.md, a checkbox in tasks.md, a risks heading in plan.md, and the
declared `Decision: 0115` resolving to one committed, clean, proposed record
the spec links. The chief's result is `drafted`.

## Risks

| Risk | Mitigation / remaining cost |
| --- | --- |
| bwrap's semantics are trusted but unverified in the seat (`lo` up, `--die-with-parent` covers the parent case, the reaper outlives every process, setup failures never print to stdout) | The live proof exercises each one (AC-L2 topology, AC-L3 lifetimes). A wrong assumption is red and named, never a silent pass: the controller judges only "a row arrived" and the probe runs the real argv. |
| An unobserved option interaction on the host (for example `--size` before `--tmpfs`, or `--clearenv` ordering) | The argv is the probed one plus options whose `--help` text the seat read on 0.11.1; the probe runs it before admission, so a wrong option refuses `bwrap_probe` with bwrap's own stderr and the implementer fixes the argv, never bypasses it. |
| The floor `0.11.1` refuses Ubuntu 24.04's packaged 0.9.0 | Honest and named; lowering it is a one-line change that must carry a live proof on that version (spec, Required behaviour). |
| `--isolated` falls back to unisolated under pressure | No fallback path exists: the only outcomes are a named refusal and exit 2, a mirrored exit with a row, or a named no-row error. The bwrap path is a constant with no override; the parameter seam is reachable only from unit tests. AC-S2 proves every refusal launches no agent. |
| A test prints SKIP and shows `ok` (the fake pass) | Boundary tests are `#[ignore]`; run explicitly they probe and panic on an unavailable boundary. The gate inventory never passes `--ignored`. The seat itself shows `bwrap_probe` red, proving the path. |
| Misattribution: an inner panic after a green probe reports `isolated_trial_no_row` | Still a harness error, exit 2, no score. The panic hook's stderr line is forwarded and distinguishes it. A start handshake would add a descriptor and a protocol; deferred. |
| A same-uid agent forges its own trial's row through `/proc/<runner>/fd/1` | The existing within-trial `unconfined` limit (0115 ruling 6), unchanged from the unisolated case; sibling pipes are in another PID namespace and invisible. |
| Port coincidence between A's and B's private loopbacks voids AC-L1 | The probe reports the void (90); the test retries at most twice and fails by name on the third, never passing or failing on a coincidence. |
| `/tmp` mode or size does not satisfy the inner admission or a burst | `--tmpfs` is owned by the sandbox uid, so `0755` suffices for every write; if the first live run shows otherwise the fix is `--perms 1777` before `--tmpfs`, not a bypass. `ENOSPC` inside is the runner's own harness error. |
| Interrupt reaches bwrap, not the runner | The controller signals the runner's host PID found by a `/proc` walk; if none exists yet it kills bwrap and emits `isolated_trial_interrupted`. AC-L3 proves the row. |
| Spawning from a worker thread trips `--die-with-parent` early | The spawn happens on the main thread inside `block_on`; plan and tasks say so, and a code comment marks it. |
| The sandbox can still exhaust host CPU or memory | Stated plainly: an access boundary, not a resource boundary; only null, oracle and scripted agents run; sequential admission bounds it today; activation must revisit (0115). |
| Reading another process's `/proc/<pid>/environ` is a hostile technique | Used only inside the test suite against the suite's own victim (AC-S1 on every host; AC-L1 from the trusted test outside both boundaries), bounded to the victim runner's descendants, never other sessions. |
| Coverage: `isolation.rs` is outside the root report | True (`evals` is a separate workspace). The discipline is AC-S2/S4 covering every pure part and the controller's refusal arms, and the live proofs covering the rest; the honest limit is that the launch's success arm runs only on the host, stated in the spec. |
| CI facts about hosted ubuntu runners are unobserved | No `.github/` change here (held); CI proves the always-on tests and the macOS `platform` refusal; 0115 names the serialized fail-closed job as an operator follow-up. |
| Egress blocks any real driver | AC-L4 stays open and blocks activation; `--isolated` refuses wrapper profiles so no paid driver runs isolated; 0115 puts the one question to the operator. No paid trial is run. |
| Gates or reviewers unavailable | Preserve failed receipts; no substitution or success claim. The run must not report complete while AC-L1..L4 and AC-O1 are open. |
