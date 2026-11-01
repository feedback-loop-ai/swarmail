# Trial isolation implementation plan — the boundary is bubblewrap

Implements [spec.md](spec.md) as revised 2026-11-01 after the review
return; semantic activation is governed by proposed
[0115](../../docs/decisions/0115-trial-isolation.md), its *Revision
2026-11-01* section and the *After the first review* subsection. The
controller, argv, refusal set, live-proof arrangement and documentation built
at bdc504f stand; this revision changes one argv token, adds one precondition
refusal, completes the void rule, gates the Linux-only tests, extends the
in-boundary observer, and fixes the small drifts the panel named. Concurrency
stays off; no row version changes; no production path reads a proof record.
The earlier plans stay in git history; this file supersedes them.

## Technical context

A standalone Rust 2024 workspace, `evals/`, outside the measured root crate
and its coverage denominator (`evals/Cargo.toml`, decision 0055). Tokio
subprocesses, an Axum fixture, reqwest and real SMTP/MCP. `evals` already
depends on `libc =0.2.190`; the controller uses it for `close_range`, `kill`
and `geteuid`, the observer for `statfs`, `fork`, `unshare` and `geteuid`.
**No new dependency, workspace member or license entry.** bubblewrap is a
host prerequisite executed as a separate process: never linked, vendored,
downloaded or added to a lockfile.

What the seat observed on 2026-11-01: `/usr/bin/bwrap` is `bubblewrap
0.11.1`; its `--help` lists `--as-pid-1` ("Do not install a reaper process
with PID=1") beside `--die-with-parent` and `--new-session`;
`kernel.yama.ptrace_scope` reads 1; the production argv fails here with
`ENOSPC` (the seat is already inside a user namespace), exit 1, nothing on
stdout. So the controller compiles and its pure parts run here; the probe
refuses `bwrap_probe` here; the live path runs only on the host. The seat
has no Apple toolchain, so `cargo clippy --target aarch64-apple-darwin` stops
at `ring`'s build script: the macOS compile is CI-only evidence.

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
missed and no `coverage(off)`; this work does not touch `src/`, so
`git diff a4f552d --stat -- src/` must stay empty. Apply 0020: deterministic
readiness, generated fixtures, real protocols, chaos at 0/100 only, no sleep
as synchronization. Commit unsigned under 0056; no push, merge or
publication. `.github/`, `scripts/`, `.brokkr/`, `deny.toml` and both
lockfiles are untouched.

## Research and decisions — each change and the finding it answers

**R1 — `--as-pid-1`, and why not an init.** The finding: bwrap's default
reaper is a same-uid, dumpable process inside the boundary, and the review
believes it holds a host-`/proc` descriptor reachable through `/proc/1/fd`
(a `PTRACE_MODE_READ` check Yama does not restrict) or `/proc/1/mem`. The
seat cannot read bwrap's source or the cited commit, so the design does not
argue the claim; it removes the process. With `--as-pid-1` bwrap execs the
runner directly as PID 1; every other process inside is one the runner
spawned behind the controller's close-on-exec sweep. The custom
non-dumpable init the robustness position proposed is rejected: it is the
hand-built PID-1 code the operator retired, it adds a hidden mode with
signal forwarding, reaping and coverage obligations, and it would protect
only the trial's own init from the trial's own agent, which 0115 ruling 6
already calls within-trial. A `ptrace_scope` prerequisite is rejected
because it does not guard the `/proc/<pid>/fd` route and would only narrow
the hosts that qualify. What `--as-pid-1` changes and what is accepted: a
`SIGTERM` that lands before the runner's handler is installed is dropped and
the trial ends at its deadline; orphans re-parent to the runner and stay
zombies until the namespace is emptied; `pick_runner` finds the runner as
bwrap's direct child by the same walk. The observer gains the descriptor walk
across every process inside and records `/proc/1/mem` openability and
`ptrace_scope` without asserting, so the security claim is falsifiable on
the host. If bwrap 0.11.1 rejects the option beside `--die-with-parent`, the
probe refuses `bwrap_probe` with bwrap's own words; the next step is argued
with that evidence, never a return to the default reaper.

**R2 — refuse external agents under `--isolated`.** 0115 promised that no
paid driver can score a provider outage as `gave_up`; the runner accepted any
external argv. The promise is now enforced by one precondition: a non-builtin
agent program must be one of the `--isolated-expose` files, compared on
canonical paths. The live proof's scripted agents (the test executable, the
sentinel scripts of the stand-in tests) are already, or trivially, exposed; a
real driver is not. A separate `--isolated-test-agent` flag was rejected as a
second lever for the same purpose, and narrowing the sentence was rejected as
an unenforced promise. The matrix never renders `--isolated`; one assertion
in the existing matrix argv tests pins it.

**R3 — macOS compile.** The pattern exists in the repo (`tests/report.rs`,
`runner.rs`, `wrappers_launch/mod.rs`): gate the `/proc`-reading helpers and
every test or signature that names them with `cfg(target_os = "linux")`.
`#[ignore]` does not help; ignored tests still compile. The shared
`tests/common/mod.rs` is included by nine binaries, so an ungated Linux
symbol there breaks all of them. The `/etc` layout test becomes
layout-aware instead of hard-coding `Tree::Directory`. The existing
`cfg(not(target_os = "linux"))` platform tests stay as macOS's proof. The
implementer audits by name because the seat cannot build for Darwin.

**R4 — the LISTEN-port void.** Inside B's private network namespace every
loopback listener is B's own, so "is A's port one of B's LISTEN ports" is
the complete coincidence test; it replaces the same-role comparisons rather
than extending them, and cannot go stale when B gains a listener.

**R5, R7, R6, R11, R12 — evidence and wording.** The floor's evidence
sentence says the proof is outstanding; nested user namespaces are an
observation on this host; `/home` is restated to what the observer proves
(and the observer now walks `/home`); `git diff` binds to the merged base
a4f552d because the local `main` ref is stale; the oracle trial passes
`--deadline-ms 30000` so the boundedness sentence is true.

**R8, R9, R13, R14 — small drifts.** `isolate()` resolves `SWARMAIL_BIN`
and the agent program after the artifacts, wrapper, expose and external-agent
checks, so the first-named refusal wins; the inner argv renders the agent
program as the canonical path the bind uses; the `bwrap_probe` stand-in test
asserts bwrap's stderr marker appears in the controller's stderr and not in
the row; `admission::root_for`'s comment stops describing the deleted worker.

**R10 — the hostile-argv table.** Six tokens at every index of the agent
argv, asserting the first `--` index equals the benign one and no token
precedes it. No property-testing crate: that would need an operator ruling.

**Rejected without a finding behind it.** The controller-side supervisor
bound (robustness §2.2): the runner's `--deadline-ms` and 0071 bound the
trial, PID-1 exit empties the namespace, and `D` state defeats any userspace
timeout; reconsidered at activation. The burst-at-the-cap trial inside the
boundary: 0003 is the root suite's proof against the unchanged server.

**Provider egress** is a feasibility checkpoint, not shipped code. 0115
carries the question with the options and their cost; `--isolated` now
refuses every external program that is not an exposed scripted agent.

## Project structure and ownership

| File / surface | Planned responsibility |
| --- | --- |
| `evals/src/isolation.rs` | `argv`: insert `--as-pid-1` after `--new-session` (R1). Add `EXTERNAL_AGENT_UNSUPPORTED` and the membership check in `request` after `EXPOSE_INVALID` and before `SOURCE_INVALID` (R2); `request` therefore compares the canonical agent against the canonical expose list. Doc comments on `supervise`/`launch` say the runner is PID 1. Tests: golden argv updated; the hostile-token table (R10); `the_host_layout_is_read_from_the_host` asserts against `symlink_metadata` (R3); the precondition-order table gains the new name. |
| `evals/src/main.rs` | `isolate()`: check `concurrency`, then the artifacts, wrapper, expose and external-agent facts, and only then resolve `SWARMAIL_BIN` and the agent program (R8), restructuring `isolation::request` into stages if that is the cleanest way to keep the order in one place. Canonicalize the agent program once and pass that path to both the bind and `inner_args` (R9). Gate `every_refused_boundary_is_a_named_harness_error_and_no_agent_runs` to Linux (R3). Stand-in tests expose their sentinel agent; the `bwrap_probe` case asserts the stderr marker (R13). The two-fault order cases and the exposed-external acceptance case (AC-S3). One assertion that matrix argv never contains `--isolated`. |
| `evals/src/runner.rs` | Unchanged production logic. `BUILTINS` is the builtin set the precondition consults. |
| `evals/src/admission.rs` | `root_for` doc comment (R14). |
| `evals/tests/common/mod.rs` | Gate `find_victim_agent`, `boundary_observe`, `launch_victim` and the `ProcessIdentity` helpers to Linux (R3). `cross_trial_probe`: read `/proc/net/tcp{,6}` LISTEN ports before routes 14/15, void on membership, delete the same-role comparisons (R4). `boundary_observe`: `/proc/1/exe` equals the runner; the `/home` walk; the descriptor walk over every readable `/proc/<pid>/fd` for a directory containing a passed host PID (new code 50); record `/proc/1/mem` and `ptrace_scope` on stderr (R1, R6). |
| `evals/tests/trial_isolation.rs` | Gate the live tests, their helpers and the always-on `/proc` tests to Linux (R3); keep the `cfg(not(linux))` platform test. Pass `--deadline-ms 30000` to the oracle trial (R12). Pass the runner path to the observer. Add the external-agent precondition and the two-fault order cases to the CLI precondition test (AC-S3). Add `--isolated-expose <test exe>` wherever the test executable is the agent. |
| `evals/README.md`, `evals/run-plan.md` | Revise the dated #221 section: `--as-pid-1`, the external-agent refusal and "an isolated row is not an eval result", macOS compiles and proves the platform refusal only, the proof has not yet run, stock 24.04 apt bwrap is below the floor. |
| `docs/decisions/0115-trial-isolation.md` | The proposed record with its *After the first review* subsection, committed by the chief with this design. Index row unchanged. |
| `specs/trial-isolation/{spec,plan,tasks}.md` | This design. |

No root `src/`, `Cargo`, gate-script, policy, `.github/` or review-routing
change. Do not refactor the matrix-cancellation (0073) or child-environment
(0075/0076 §4) blocks incidentally.

## Data model and contracts

No wire contract changes. `TrialResult` stays schema 2 with required integer
`concurrency: 1`. The `execution`/`boundary` object of 0115 §4 is not added.

The boundary's internal contract:

- **The refusal vocabulary** is closed: the seven precondition names (the six
  of bdc504f plus `isolation_external_agent_unsupported`),
  `boundary_unavailable:{platform,bwrap_absent,bwrap_version,bwrap_probe}`,
  `isolated_trial_no_row`, `isolated_trial_interrupted`. A row never carries
  bwrap's text.
- **The version parser**: unchanged.
- **The argv builder**: the fixed prefix now includes `--as-pid-1`; the
  index of the first `--` is a function of the request only.
- **The inner renderer**: the agent program is the canonical path the bind
  uses.
- **The launch judgement**: unchanged.
- **The observer's exit codes**: 40 bad arguments, 41 uid, 42 `/proc`
  (host PID visible, or PID 1 is not the runner), 43 interfaces, 44 loopback
  unusable, 45 a host path present (including a stray entry under `/home`),
  46 tmpfs, 47 environment, 48 stray descriptor in the observer, 49
  capabilities, **50 a descriptor of some process inside reaches the host
  `/proc`**, 100 nested user namespace granted (recorded; the test treats
  100 as an observation, not a failure).

## Quickstart and validation

From the repository root, with cached dependencies:

```bash
cargo build --locked --bin swarmail
cd evals
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
SWARMAIL_BIN="$PWD/../target/debug/swarmail" cargo test --locked
```

The last command runs AC-S1 through AC-S9 on Linux, with the live proofs
shown as `ignored`. The live proof is the separate `--ignored` command in the
spec, run by the dispatcher on this host after the run. Then, from the root,
every command in `docs/house-rules.md` § Gates runs locked. CI owns the
macOS compile, chart, Docker smoke, RustSec, tag-rate and port-80 proofs; no
CI change is made here.

Design-stage validation is structural (`design-check.sh`): exactly one
touched `specs/<slug>/`, its three files committed, non-empty and clean, an
acceptance heading in spec.md, a checkbox in tasks.md, a risks heading in
plan.md, and the declared `Decision: 0115` resolving to one committed, clean,
proposed record the spec links. The chief's result is `drafted`.

## Risks

| Risk | Mitigation / remaining cost |
| --- | --- |
| bwrap 0.11.1 rejects `--as-pid-1` beside `--die-with-parent`, or an init without a reaper behaves differently from what the spec assumes (R1) | The probe runs the real argv before admission and refuses `bwrap_probe` with bwrap's own stderr; AC-L3 proves the lifetime chain and AC-L2's `/proc/1/exe` check proves the reaper is gone. A wrong assumption is red and named, never a silent pass; the fallback is argued with evidence, never the default reaper. |
| An early `SIGTERM` to the controller is dropped because the runner, as init, has no handler yet | Accepted: the trial ends at its `--deadline-ms`, no false row. AC-L3's `SIGTERM` case sends after the agent exists. |
| Zombies re-parented to the runner inside | Bounded by one trial: PID-1 exit empties the namespace before bwrap's `wait` returns. |
| The review's escape route exists in some other form (a descriptor bwrap leaks into the process it execs) | AC-L2's descriptor walk over every process inside goes red (50) by name; `/proc/1/mem` and `ptrace_scope` are recorded for the operator. |
| The exposed-external rule is used to run a real driver by hand | A deliberate `--isolated-expose <driver>`; the row is an honest failure, documented as not an eval result; the matrix never renders `--isolated` (unit assertion). |
| The macOS compile is still broken after gating | CI-only evidence; the implementer audits every Linux-gated name for ungated uses and the `cfg(not(linux))` tests stay. If CI is red the fix is more gating, never a weakened criterion. |
| A port coincidence still misreports | The LISTEN rule is complete inside a private netns by construction; the two-retry bound and named third failure stand. |
| The floor `0.11.1` refuses Ubuntu 24.04's packaged 0.9.0, so a stock CI runner cannot run the boundary | Honest and named; lowering it is a one-line change that must carry a live proof on that version; the operator's CI follow-up chooses a pinned bubblewrap or a floor decision. |
| `--isolated` falls back to unisolated under pressure | No fallback path exists; the bwrap path is a constant with no override; AC-S2 proves every refusal launches no agent. |
| A test prints SKIP and shows `ok` | Boundary tests are `#[ignore]` and Linux-only; run explicitly they probe and panic on an unavailable boundary. |
| Misattribution: an inner panic after a green probe reports `isolated_trial_no_row` | Still a harness error, exit 2, no score; the panic hook's stderr line is forwarded beside the row (now asserted by the R13 test). |
| A same-uid agent forges its own trial's row through `/proc/1/fd/1` | Within-trial (0115 ruling 6), unchanged from the unisolated case; sibling pipes are in another PID namespace and invisible. |
| The sandbox can still exhaust host CPU or memory | An access boundary, not a resource boundary; only null, oracle and exposed scripted agents run; sequential admission bounds it today. |
| Coverage: `isolation.rs` is outside the root report | AC-S2/S3/S4 cover every pure part and the controller's refusal arms; the live proofs cover the rest; the launch's success arm runs only on the host, stated in the spec. |
| Egress blocks any real driver | AC-L4 stays open and blocks activation; the external-agent refusal keeps drivers out; 0115 puts the one question to the operator. No paid trial is run. |
| Gates or reviewers unavailable | Preserve failed receipts; no substitution or success claim. The run must not report complete while AC-L1..L4 and AC-O1 are open. |
