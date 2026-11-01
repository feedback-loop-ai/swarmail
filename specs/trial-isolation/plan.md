# Trial isolation implementation plan — preparation on main

Implements [spec.md](spec.md); semantic activation is governed by proposed
[0115](../../docs/decisions/0115-trial-isolation.md). This run prepares the
worker and proves everything provable without a namespace. Concurrency stays
off; no row version changes; no production path reads a proof record.

## Technical context

A standalone Rust 2024 workspace, `evals/`, outside the measured root crate
and its coverage denominator (`evals/Cargo.toml`, operator ruling 2026-10-18).
Tokio subprocesses, an Axum fixture, reqwest and real SMTP/MCP. `evals`
already depends on `libc =0.2.190`; the boundary uses it and the raw
`syscall`, `unshare`, `close_range`, `umount2`, `sigprocmask`, `sigwait`,
`waitpid` and `prctl` bindings it exposes, plus `SYS_pivot_root` and
`SYS_clone` for the namespaced child. No new dependency, workspace member or
license entry. Do not add `bwrap`/`unshare` as a runtime dependency because a
binary is on PATH.

The box the delivery seat runs in is already inside a user namespace
(`/proc/self/uid_map` is `1000 0 1`), `unshare -Urn true` returns `ENOSPC`,
`close_range` and `pivot_root` bindings are present in the locked libc, and
`kernel.apparmor_restrict_unprivileged_userns` reads 1. So the boundary code
compiles and its pure parts run here; its live path does not, and every
namespace-dependent test skips by name.

There is no plan template in this checkout; the spec-kit sections (research,
data model, contracts, quickstart) are inline. The design commit is exactly
this spec/plan/tasks and the one proposed decision. Implementation and
documentation edits belong to later commits, not the chief's HEAD.

## Constitution check

Preserve accepted 0001–0005, 0055's app-state authority and real-subprocess
path, 0071 trap precedence, 0073 matrix cancellation, 0075 safe diagnostics
and closed environments, and all of 0076. Accepted 0045 §8 separates
compatible proof from contradictory activation: overlapping trials, a new row
version and any egress change wait for operator acceptance of 0115. Root exact
coverage stays 100% lines/branches/functions with zero missed and no
`coverage(off)`; this work does not touch `src/`, so the root report is
unchanged and `git diff main --stat -- src/` must stay empty. Evals keeps its
own 0055 proofs. Apply 0020: deterministic readiness, generated fixtures, real
protocols, chaos at 0/100 only, no sleep as synchronization. Review is the
accepted 0098/0099 panel in a later phase, not an extra panel here. Commit
unsigned under 0056; no push, merge or publication.

## Research and decisions

The inspected candidate (`evals/src/runner.rs`) allocates each child's state
with `RuntimeDirs::create` under a `tempfile` root in `/tmp` at mode `0700`
(`runner.rs:509-530`), and `admission.rs` holds `/tmp/swarmail-evals-<uid>/
trial.lock` by `try_lock`. Both are uid-scoped and cooperative: 0076 already
records that they are not confinement. The trap scorer, fixture identity
header and failure precedence are unchanged by this work.

The one feasibility candidate is a Linux user/mount/PID/network/IPC namespace
worker mapping back to the initiating uid, with minimal privilege. The chief
chose the clone-based topology (controller in host namespaces, runner as PID 1
of the new PID namespace) over simplicity's `unshare`-then-`fork` because the
process that becomes the runner must itself be PID 1 for the kernel to reap
the trial on its death; `unshare(CLONE_NEWPID)` only affects children of the
caller. The clone is done once, single-threaded, from `trial --isolated`
before any Tokio runtime is built, so no async-fork hazard exists.

Provider egress is a feasibility checkpoint, not shipped code. A private
network namespace has loopback only; a real driver cannot reach its provider.
0115 asks the operator one question with the chief's recommendation (a
controller-side CONNECT relay with an explicit allowlist over an inherited
socket). Until then `--isolated` refuses every `--wrapper-profile`, so a paid
driver cannot run isolated and score a provider outage as a model `gave_up`.

## Project structure and ownership

| File / surface | Planned responsibility |
| --- | --- |
| `evals/src/isolation.rs` (new) | The pure, testable parts: the errno-name table, the bind allowlist resolver, the ordered step list, and the request validation that refuses `SWARMAIL_BIN`-unset / artifacts / wrapper / bad-expose. `#[cfg(target_os = "linux")]` for the syscall body; a `cfg` stub refuses with `ENOSYS` elsewhere. One `src/isolation/boundary_tests.rs` if a cold refusal path needs a sibling harness (decision 0020 rule 2). |
| `evals/src/lib.rs` | One `pub mod isolation;` line. |
| `evals/src/main.rs` | The hidden `--isolated` flag and `--isolated-expose` on `Cmd::Trial`; about 25 lines that, when set, run `isolation::enter` before `tokio::runtime::Builder` and continue into the existing `Trial` arm on success, or emit the refusal row and exit 2. The entry is taken as a parameter so a test can inject a failing one (AC-S2). |
| `evals/src/runner.rs` | Unchanged production logic. `refused_row` is reused for the refusal; no new fields. `git diff main --stat -- evals/src/runner.rs` is expected empty except possibly a `pub(crate)` visibility widening for a seam the sibling harness needs. |
| `evals/tests/trial_isolation.rs` (new) | The always-on leak baseline (AC-S1), the precondition refusals (AC-S3), and the `#[ignore]` live proofs (AC-L1..L3), with the probe-and-fail skip helper (AC-S6). |
| `evals/tests/common/mod.rs` | Extend with the idle-victim handshake (already present as `held_idle`), bounded descendant discovery (already present as `process_descendants`), a `/proc/<pid>/environ` reader, and the probe helper. Controller-owned; grants no sibling capability. |
| `evals/README.md`, `evals/run-plan.md` | A dated section: the hidden flag, its refusals, the host proof command, sequential-still, no row claims isolation. |
| `docs/decisions/0115-trial-isolation.md` | The proposed record itself, committed by the chief with this design. |
| `docs/decisions/README.md` | Its index row, added in implementation task T01 (the 0076 precedent: the chief commits only the decision body; the index row lands with the implementation so the chief HEAD stays the four design files). |
| `specs/trial-isolation/{spec,plan,tasks}.md` | This design. |

Read `fixture.rs`, `oracle.rs`, `mcp.rs` and the scorer to confirm effects;
no change to their public authentication or scoring is planned. No root
`src/`, `Cargo`, gate-script, policy or review-routing change. Do not refactor
the matrix-cancellation (0073) or child-environment (0075/0076 §4) blocks
incidentally.

## Data model and contracts

No wire contract changes in this run. `TrialResult` stays schema 2 with
required integer `concurrency: 1`; `results::read` and the report math are
untouched. The `execution`/`boundary`/`concurrency` object of 0115 §4 and §5
is drafted in the decision and the spec but is **not** added to any struct,
constructor, reader or report here; it lands as a new row version chosen by
the operator at activation, with the same fail-closed reading of older rows
that `results.rs` already applies to unknown versions.

The boundary's own contract is internal and pure where tested:

- An **errno-name table**: a total function from the `c_int` errnos the
  boundary can report to stable lowercase names, exhaustive and closed, so a
  diagnostic is `boundary_unavailable:<operation>:<ERRNO-name>` with no raw
  string interpolation (0075 finite categories).
- A **bind allowlist**: the fixed read-only sources plus the resolved
  absolute paths of the runner, Swarmail and agent executables and any
  `--isolated-expose` files. Resolution refuses a relative, missing,
  non-regular or symlinked target before any clone.
- An **ordered step list**: a value listing the setup steps in order, asserted
  by a unit test to place identity maps first, the `/proc` mount before the
  pivot, descriptor closure and privilege drop before the self-check, and the
  parent-death signal after the last credential change.

The controller/init boundary is a process relationship, not a message
protocol: no agent-controlled string selects a host path, namespace or
privileged operation; the init reads only its validated request through the
existing `--worker-context` channel, which this run does not extend.

## Quickstart and validation

From the repository root, with cached dependencies:

```bash
cargo build --locked --bin swarmail
cd evals
SWARMAIL_BIN="$PWD/../target/debug/swarmail" cargo fmt --all -- --check
SWARMAIL_BIN="$PWD/../target/debug/swarmail" cargo clippy --locked --all-targets -- -D warnings
SWARMAIL_BIN="$PWD/../target/debug/swarmail" cargo test --locked
```

The last command runs AC-S1 through AC-S6 on every host, with the live proofs
showing as `ignored`. The live proof is the separate `--ignored` run named in
the spec, on a namespace-capable host. Then, from the root, every command in
`docs/house-rules.md` § Gates runs locked: `fmt`, `clippy --locked`,
`cargo test --workspace --locked`, `brokkr compile --bundle
.brokkr/recipes/self`, `scripts/coverage-gate.sh` and its `selftest`,
`scripts/publish-guard-test.sh`, `scripts/seal-branch-test.sh`, and
`cargo deny check licenses`, plus the fail-closed `native-tls` absence proof.
CI owns the chart, Docker smoke, RustSec, tag-rate and port-80 proofs.

Design-stage validation is structural (`design-check.sh`): exactly one touched
`specs/<slug>/`, its three files committed, non-empty and clean, an acceptance
heading in spec.md, a checkbox in tasks.md, a risks heading in plan.md, and
the declared `Decision: 0115` resolving to one committed, clean, proposed
record the spec links. The sequence's pinned checker rules the attempt; the
chief's result is `drafted`.

## Risks

| Risk | Mitigation / remaining cost |
| --- | --- |
| The namespace code lands unexecuted in this seat | Every failure refuses by name; the code has no caller outside the hidden flag and the `#[ignore]` proofs; the spec and 0115 say plainly the worker is unproven. The first live run will find setup bugs (locked mount flags, a masked `/proc`); that is expected, not a regression. |
| `--isolated` falls back to unisolated under pressure | There is no fallback path by construction: the single return is success or a named refusal and exit 2. AC-S2 proves a forced failure launches no agent. |
| A test prints SKIP and shows `ok` (the fake pass) | Namespace tests are `#[ignore]`; run explicitly they probe and **panic** on an unavailable boundary. The gate inventory never passes `--ignored`, so seats stay green with the proofs shown as ignored. |
| The index row is a fifth file in the chief commit | It is not: the chief commits only the four design artifacts; the index row is implementation T01, matching the accepted 0076 delivery (its T1 added the index row). The decision is still discoverable by its `Decision: 0115` link from the spec, which is what `design-check.sh` validates. |
| Reading another process's `/proc/<pid>/environ` is a hostile technique | It is used only inside the test suite against the suite's own idle-victim child, to establish the leak the feature exists to close (AC-S1). It is authorized security testing of this harness, bounded to the victim runner's descendants, never other sessions. |
| The clone-based PID-1 topology is subtle | The clone is single-threaded before any runtime; the controller only forwards signals and mirrors status; a tiny reaping init is deferred activation work and named as a known zombie limit, not silently assumed away. |
| A row implies a trial was isolated | No row changes; `confinement` stays `"unconfined"`; no `boundary` field exists until the operator accepts a version. The report cannot claim isolation. |
| Egress blocks any real driver | AC-L4 stays open and blocks activation; `--isolated` refuses wrapper profiles so no paid driver runs isolated; 0115 puts the one question to the operator. No paid trial is run. |
| Coverage: isolation.rs is outside the root report | True (`evals` is a separate workspace), so the exact-100% gate does not force its lines to execute. The discipline is replaced by AC-S1..S4 covering the pure parts and the live proofs covering the rest; the honest limit is that the syscall body is unexecuted here, stated in the spec. |
| Incidental edits to reserved cancellation/environment blocks | The boundary sets HOME inside the private tmpfs via the existing `RuntimeDirs` map; no edit to the 0073 cancellation or 0075/0076 §4 environment code. If an edit becomes necessary it is recorded as an integration dependency, not hidden in a refactor. |
| Gates or reviewers unavailable | Preserve failed receipts; no substitution or success claim. The run must not report complete while AC-L1..L4 and AC-O1 are open. |
