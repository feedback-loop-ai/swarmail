# Implementation plan — POP3 allocation warm-up

Implements [spec.md](spec.md). This phase commits design; implementation,
mutation experiments and final feature verification remain ordered work.

## Technical context

One Rust crate, standard global allocator forwarding to `System`, two
const-initialized thread-local cells, and an RAII disarming guard. The
existing `src/pop3/alloc_tests.rs` cold-path harness is declared under
`#[cfg(test)]` by `src/pop3.rs`. It can call the crate-private store seam.
No extra crate, feature, platform conditional or runtime API is needed.
There is no plan template under `.specify/templates/`; the required
technical context, constitution, structure, research, model, contracts and
quickstart are supplied here.

## Constitution check

The implementation leaves production code, wire contracts and frozen
guarantees byte-identical. Accepted 0020 permits this already-declared
harness and requires causal synchronization. Its tests still exercise
production sizing and real RETR/session paths. No source moves into an
excluded harness, no new exclusion is added, and exact coverage remains
100% in all three dimensions. No dependency or license change is justified.
No decision amendment is needed; existing proposed records are not accepted
by this work. Signed append-only history and the protected verify/review
sequence remain mandatory.

## Research and resolved choices

The counter's own cells have neither a lazy destructor nor heap state.
`Barrier::wait` occurs twice inside the control's scope. Darwin's lazy
pthread boxes explain the observed 112-byte excess, and a fresh Store's
snapshot gate is separately uninitialized: insert locks `mutation`, not
`snapshot_gate`. Directly invoking an empty `run_snapshot_gate` warms only
test setup without listing mails or sizing payloads. Record any available
local toolchain-source corroboration with the implementation receipt; do
not call the CI failure log an allocation-stack trace.

One extra barrier round exercises the actual mutex and waiting path before
the observed rounds. Both threads participate. This creates a phase
boundary without assuming a scheduler order. A generic primitive warm-up
in `observe` cannot prepare these instances, and a complete snapshot
rehearsal would weaken first-call allocation detection.

## Project structure and files

| Path | Responsibility |
|---|---|
| `src/pop3/alloc_tests.rs` | Only implementation edit: two setup steps, local lazy-fixture proof, comments explaining the boundary. |
| `specs/pop3-allocation-warmup/{spec,plan,tasks}.md` | Chief's three committed artifacts; later record actual outcomes without declaring unrun work done. |
| `.brokkr/state/epic120/` | Ignored patches, mutation outputs, restored-green receipts and gate/platform status. Never stage. |
| `src/pop3.rs`, `src/store.rs` | Read-only implementation references; temporary production mutants are fully reverted. |

## Data model and measurement contract

Keep `Tally`, `Counting`, `ARMED`, `TALLY`, `Armed` and `observe` semantics.
Preparation occurs while disarmed; only then does `observe` reset and arm,
call its closure exactly once, read the tally and disarm. Return measured
buffers to the caller and destroy them after the scope. Existing unwind
cleanup continues to own panic exits.

The new proof uses two local `OnceLock<Box<[u8; 64]>>` instances. A cold
initializer allocates under observation. The second initializer runs
before observation; its measured closure calls `get_or_init` again, reads
the retained buffer and creates a 1000-byte `Vec` with exact capacity.
Return that Vec and inspect both fixtures after observation. Use
`std::hint::black_box` on allocations/accesses as needed, matching the
existing harness's discipline. This exercises actual allocation, with no
manufactured tally, global mutable fixture, TLS destructor or mock allocator.
The initializer must still be reachable inside the warm scope so deleting
preparation produces a counted allocation rather than an unwrap panic.

## Implementation sequence

1. Add a preliminary `barrier.wait()` to both participants before their
   existing measured-round protocol. Keep the worker allocation after the
   first measured round and before the second. Keep the exact tally and
   post-scope disarming assertion unchanged.
2. After building the snapshot fixture and before `observe`, call
   `store.run_snapshot_gate()` with its default empty hook. Do not move the
   seam, install a hook, or run any production traversal beforehand.
3. Add the cold/warm local lazy-fixture proof and document why these two
   scopes prepare harness synchronization while sizing and RETR do not.
   Comments describe Linux's proof limit and Darwin's supported diagnosis.
4. Execute focused tests under normal parallel execution and single-thread
   harness execution. Inspect `--list` and receipts to ensure every named
   proof runs. These checks supplement the phase-boundary reasoning;
   repeated luck is not the synchronization mechanism.
5. Apply AC5 mutations one at a time. The sizing mutant must use a real
   observable allocation inside `payload_len` (for example black-box a
   temporary cloned payload), not simply alter a pin. Capture each patch
   and failing assertion. Deleting the new proof's preparation is a third,
   separate mutation; it does not replace breaking the two original tests.
   Restore exact original files before running the green suite and gates.
6. Run final gates, append signed implementation commits, and record
   Linux/macOS statuses separately. The operator's assistant owns pushing
   and collecting the macOS CI result; this seat cannot close that gap.

## Validation and quickstart

```bash
cargo test --workspace --locked --lib pop3::alloc_tests -- --list
cargo test --workspace --locked --lib pop3::alloc_tests
cargo test --workspace --locked --lib pop3::alloc_tests -- --test-threads=1
cargo test --workspace --locked --lib pop3::alloc_tests::the_counter_sees_this_threads_allocations_and_no_other -- --exact
cargo test --workspace --locked --lib pop3::alloc_tests::the_snapshot_sizes_its_maildrop_without_a_per_mail_allocation -- --exact
```

Use the actual new test name for its exact mutation command. Require one
executed test for each exact command and the intended assertion diagnostic.
The full suite below includes the real protocol tests; no mock server is
substituted. Final source-tree inventory:

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
cargo deny check licenses
brokkr compile --bundle bundles/self
```

Verify uses the pinned script copies, checks dispatch shims, proves
`native-tls` absent fail-closed, and uses the brokkr binary whose digest is
recorded in `bundles/self/tooling.json`. It installs nothing. Do not override
TMPDIR or weaken a failing gate. Report command/environment refusals as
outstanding, never passed. macOS proof is the existing CI job on the final
candidate; no new workflow or local emulation is planned.

## Risks

- **Toolchain setup changes.** Narrow preparation may expose a new macOS
  allocation later. Accept a loud exact-pin failure and diagnose its path;
  never pre-run production work or introduce an allowance to hide it.
- **Incomplete barrier warm-up.** Preparing only a mutex may miss the
  condition variable. Complete a real round on the same barrier with both
  participants before measured rounds; retain the worker-allocation ordering.
- **Synthetic proof overclaim.** A local OnceLock proves scope ordering on
  Linux, not Darwin's implementation. Keep macOS explicitly pending CI.
- **A warm scope could become vacuous.** Keep its explicit 1000-byte
  allocation, cold exact control, and preparation-removal mutant. Retain
  independent counter-disabled and production-sizing mutants.
- **Optimizer or drop placement distorts counts.** Make allocation values
  observable and keep retained buffers alive until after disarming; inspect
  the full tally, not just request counts, in the mechanism test.
- **Mutation leakage or misleading red output.** Save reversible patches,
  require named assertion failures and nonzero test counts, restore before
  each next mutant and final gates, and inspect the staged diff.
- **Signing or local execution unavailable.** No unsigned fallback and no
  claimed pass. Preserve concrete artifacts/evidence and report the blocker.
  The design result is only `drafted`; implementation is still outstanding.
