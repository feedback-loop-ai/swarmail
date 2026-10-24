# Plan: runner cleanup probe synchronization

## Technical context

The affected crate is standalone `evals/`, Rust 2024, with its own locked
workspace. Existing std and Tokio Unix sockets, async I/O and synchronization
are sufficient. The regression is Linux-only and uses real subprocesses,
`/proc` identity observations and the real `run_trial`. No new dependency or
platform layer is needed. Root exact coverage does not measure evals (0055).

## Constitution check

Follow [house rules](../../docs/house-rules.md), accepted 0020 placement and
seam discipline, and 0055 lifecycle ownership. Every executable change is
`cfg(test)`-gated in the owning module; no production cleanup timer or fence
changes. Preserve frozen guarantees, ingest extraction, dependency allowlist,
coverage denominator and candidate bundle routing. No decision is introduced
or accepted. Commits are unsigned and local; no push or sealing.

## Research and resolved questions

Inspection confirms `OwnedGroup::cleanup_once` owns the hold before
`TERM_GRACE`/`KILL_WINDOW` begin. `Groups::cleanup_pids` samples a real
`JoinSet::join_next` future, records Pending, optionally bypasses, and otherwise
awaits it. The preliminary sample is shared by positive and bypass paths and
cannot alone prove the await. `runner_probe_entrypoint` owns its runtime and
calls real `run_trial`; it is the appropriate place to report actual return.
`Probe` already owns child handles, identities and rescue; extend this owner.

`wait_for_cleanup_pending` and marker files have other callers. Preserve their
behavior unless a narrow compatible change is required; switch the named
regression to its handshake. Avoid blanket replacement of readiness polling,
budget tests or teardown bounds. Existing startup readiness may retain its
hang guard, but cannot serve as evidence for the cleanup-order verdict.

No plan template exists in this candidate's `.specify/templates/`; this plan
provides the requested technical context, constitution check, structure,
research, data model, contracts and quickstart inline rather than inventing
unavailable template requirements or adding extra design artifacts.

## Project structure and touched files

| File | Responsibility |
| --- | --- |
| `evals/src/runner.rs` | The only executable edit: owner-local test seam, actual await instrumentation, child return observation, inline `Probe` controller/teardown and regression matrix. |
| `specs/runner-cleanup-probe/spec.md`, `plan.md`, `tasks.md` | This chief's exact three-file design commit. |
| `specs/runner-cleanup-probe/verification.md` (implementation phase) | Commands, case/mode evidence, precise mutant patch/failure/restoration and production-equivalence method/results. Commit this evidence before handing off implementation. |
| `.forge/epic192/issue225/` and `.brokkr/state/` | Read-only authority/run evidence; position files remain uncommitted. |
| `.forge/tasks/` or `target/` | Run-local logs and compiler comparison outputs; do not stage build artifacts. |

No separate research, model or contract files are needed for this private
protocol; their content is below. Existing `framing.md` and intake receipt
stay intact. Do not add a decision or edit its index.

## Data model and private contract

One short-path Unix-domain endpoint is created in each probe's temporary
directory before child launch. The probe retains the directory, listener,
connections and release state through teardown. Pass the selector only via
that child's `Command::env`. Use async child-side I/O; never block runtime
workers on a parent response. Reuse existing dependencies and ownership.

A small fixed message format carries owner identity or cleanup-wait identity
where needed. Use distinct causal observations:

| Observation / command | Meaning |
| --- | --- |
| `pre-entry(owner)` / `permit-entry` | Delayed mode reached a barrier before hold entry and before cleanup budgets start; permission is retained for later owners too. |
| `held(owner)` / `release` | The hold's release latch is armed and the owner is held until retained per-probe release. |
| `sample-pending(wait)` | Existing preliminary poll of the real cleanup join returned Pending, including bypass controls. |
| `await-pending(wait)` | The continuation actually awaiting that same join observed Pending; a detached task or IPC read cannot produce this evidence. |
| `returned` / `ack-return` | Real `run_trial` returned; the test runtime remains alive for observation until acknowledgement. This is separate from OS child exit. |

Names describe obligations, not a public/versioned protocol. Buffer events or
store monotonic state; receiving late must not lose a notification. Accept
independent owner events in any order and correlate them with existing probe
identities. Never assume the first held PID is the agent or that only one
cleanup owner exists. Release covers current and future holders; closing the
controller must unblock or fail their test waits, not strand them. Keep
stdout/stderr and descendant descriptors separate from this channel.

## Implementation approach

1. Install the control endpoint before launching the probe. Register readers
   before signals or injected-fault release. Retain old marker outputs for
   sibling tests. The new named path waits on the handshake, not those files.
2. In the `cfg(test)` hold, optionally await entry permission after reporting
   the checkpoint. Arm the release latch, publish hold entry and await release.
   All of this precedes the existing production budget construction. Both
   early release and late-arriving cleanup owners must work.
3. Preserve the real preliminary Pending sample and bypass option. Instrument
   polling of the actual awaited continuation to publish awaited Pending
   without inserting a separate test wait in place of the fence. The positive
   controller accumulates held/Pending events; it also handles actual return
   immediately and rejects return before its required state. Instrumentation
   must leave an early-return path observable when the fence is mutated away.
4. Immediately after real `run_trial` returns, publish `returned` before the
   test-only acknowledgement gate. Keep the runtime alive until acknowledgement
   so independently spawned cleanup and reapers remain observable. This gate
   never counts as cleanup Pending. Separate actual runner return from probe
   child termination in `Probe`; the completion predicate still requires
   cessation and direct-child reaping. Share that predicate between positive
   and bypass assertions. Keep final child wait and `finish` obligations.
5. Replace the named regression's 50 ms non-exit and two-second bypass oracle.
   Positive cases collect causal state, assert live recorded work, release,
   observe return/cessation/reaping, acknowledge return and finish without
   rescue. Bypass cases observe actual return with the hold closed, reject
   completion while work is live, then release/acknowledge and perform checked
   teardown. EOF/malformed protocol is a harness failure; an early return in a
   positive case is a semantic assertion failure, never a timeout fallback.
6. Run all four positive and two bypass cases in ordinary and delayed modes
   inside the same named test. In delayed mode receive `pre-entry`, deliberately
   withhold permission across that acknowledged checkpoint, then grant it.
   This forces entry to depend on the controller even on an idle machine.
   Emit case/mode/checkpoint evidence; do not require a minimum duration.
7. Extend `Probe` failure teardown to release entry, cleanup and return gates
   before its existing bounded rescue and retain endpoint ownership throughout.
   Exercise teardown with a controlled assertion failure/disconnection in an
   isolated inline probe test, preserving PID/start-time/session checks. Test
   buffered-before-receive events, multiple/late owners and EOF handling through
   installed seams; keep the protocol minimal enough to exercise every seam.
8. Mutate the positive path's actual awaited continuation to early `Ok(())`
   after the preliminary Pending sample (or remove the corresponding real
   outer fence while retaining owner startup). The targeted test must fail on
   actual early return or live work at completion. A timeout means the observer
   is inadequate and must be repaired. Save the exact diff/output, restore,
   and rerun the identical filter. Do not commit the mutant.

## Production equivalence

Before editing, save canonical compiler-expanded non-test runner output from
baseline `f873523` using the workspace's compiler/configuration. After editing,
repeat with identical target, features, profile and compiler; compare bytes
with `cmp` and record hashes and exact commands. Compare the compiler's runner
module representation, not a regex removal of `cfg(test)` text. If compiler
output contains unrelated generated metadata, use a documented compiler-based
module extraction applied identically to both outputs; do not normalize away
code differences. Review that all executable diff hunks are test-only and
that the non-test cleanup loop is untouched. Missing local comparison support
must be named precisely, not hidden by a source-only assertion.

## Quickstart and verification

Run offline, install nothing. Use the exact Linux filter (one test must run):

```bash
export CARGO_NET_OFFLINE=true
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --workspace --locked
cargo test --manifest-path evals/Cargo.toml --locked --lib runner::test_probes::runner_cleanup_await_pending_and_bypass_controls_cover_signals_and_faults -- --exact --nocapture
```

The named test itself selects both ordinary and delayed modes. Use the same
filter for the temporary mutant and restored pass; capture nonzero semantic
failure and exactly one executed test. A zero-test run is invalid proof.

Run the [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
and the pinned verify seat's additional proofs on the committed candidate:

```bash
sha256sum .forge/bin/brokkr
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
```

Require engine digest
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`
before execution. A bundle compile warning containing `lets the run go on
without reading` fails even with exit zero. Use pinned script copies after
shim validation, per verify-seat policy. Also run the fail-closed native-tls
absence proof from [framing](framing.md): a failed `cargo tree -i native-tls`
only proves absence when it reports `did not match any packages`.
Retain exact coverage summary/lcov and every command/exit; all three missed
counts must be zero. CI/operator proofs remain with their existing owners.
The design check runs after this chief's last commit; its structural verdict
is distinct from implementation and gate completion.

## Risks

| Risk | Mitigation / accepted cost |
| --- | --- |
| Instrumentation accidentally supplies the missing await | Publish Pending only from real awaited polling and return before the observation gate. Require assertion-level mutant rejection. |
| Lost event, late owner or competing hold waiters | Retained per-probe state, reception installed before trigger, identity correlation and release-all semantics; exercise early delivery and late entry. |
| Runtime destruction masks bypass or return gate masks cleanup | Observe actual return before gating; inspect live identities then; separately await process exit and reaping. |
| New IPC wait strands failure teardown | Own all endpoints in `Probe`, release/close all gates first, and prove checked identity-safe rescue on injected failure. |
| Scheduling still exhausts a hang bound | Report a harness hang honestly; never interpret timeout as successful waiting or mutant detection. No sleep-based interleaving proof. |
| Shared helper change breaks unrelated tests | Preserve marker paths and run the complete evals suite, including budget/leader-first/runtime-lifetime probes. |
| Root coverage gives false confidence about evals | Keep inline seams exercised, run standalone evals checks, explicitly preserve the 0055 boundary. |
| Full evals suite repeats baseline matrix-stdout flake | Retain first failure and any diagnostic rerun; do not silently retry to success or expand #225 into that repair. |
| More private protocol code and doubled control cases | Limit to one module and existing facilities; no generic framework, dependency or production instrumentation. |
