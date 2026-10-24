# Continuation plan — matrix cancellation

Implements [spec.md](spec.md), retaining proposed
[0062](../../docs/decisions/0062-matrix-cancellation.md). The chief changes
only these three design artifacts; subsequent implementation closes proof
holes and repairs only defects exposed by the specified falsifiers.

## Technical context

Rust 2024 standalone `evals/` workspace, existing Tokio process/signal/I/O,
libc, serde and tempfile. Sequential matrix, one active trial, registered
Unix process groups and independently retained reapers. No new dependency,
feature, manifest or lock change. Evals has its own mandatory suite under
0055; root exact coverage does not measure it. No plan template exists in
this checkout; this plan supplies the required dialect sections directly.

Inspected baseline: `1315846de12787f78a6f66b1885bbbc2aff72786`, following
`1a2f36c` implementation and `1843c55` recovery design. This chief commit
creates a successor even though executable source stays unchanged. Record
actual candidate revisions throughout, never relabel exact-1a2f36c evidence.
The run remains engine-class with independent verification and four protected
review lenses; the engine, not this plan, selects phase transitions.

## Constitution check

Preserve accepted 0001–0005, 0055 lifecycle/schema/report contracts and 0020
readiness/test-placement discipline. 0062 remains proposed; accepted 0045 §8
allows compatible work, not contradiction of accepted rulings. This proof
repair adds no semantic ruling, so no new ADR is needed. A discovered accepted
contract conflict requires an operator ruling before contradictory work;
frozen guarantees cannot weaken. No root source, gate, recipe, policy,
coverage boundary or license allowance change. Commits unsigned under 0056;
no push, merge, issue close or fabricated journal/result history.

## Research and current evidence

Read both current panel positions, current and predecessor intake framings,
prior VERIFY-FAIL, decisions and source. The failure was host executable
identity, not an executed signal failure. The candidate-local executable's
SHA-256 is `e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`,
matching tooling.json. Invoke its absolute path; install nothing and change
no pins. The prior chief's exact-1a2f36c green regression and 1315846
implementer claims remain historical, not new verification.

Current matrix has actual child-wait Pending and bypass controls, separately
for TERM and INT. Current holds/injection are cfg(test)-only. Preserve those
repairs. Current source still publishes the shell helper PID before trap
installation, takes MatrixProbe's child out before an unbounded wait, and
proves runner cleanup using entry/liveness followed by eventual death.
Runner fallback also depends on finding the original group leader alive.
These findings justify the narrow repairs below without asserting production
cleanup is broken. Map every acceptance criterion to executable evidence;
unmapped or unexecuted proof remains outstanding.

## Project structure and touched files

| Path | Work and evidence |
| --- | --- |
| `specs/matrix-cancellation/{spec,plan,tasks}.md` | This chief's exact commit scope; reconciled current design and paired obligations. |
| `evals/src/main.rs` | Keep runtime child ownership and cfg(test) Pending/bypass controls; repair helper readiness, MatrixProbe bounded waits and failure ownership. |
| `evals/src/runner.rs` | Add local actual-cleanup Pending/bypass proof; strengthen Probe survivor teardown; inspect deadline reuse and retained reapers. Runtime edits require a reproducer. |
| `evals/tests/report.rs` | Preserve ordinary CLI PID-only cancellation, prefix/active/suffix accounting, aggregation and exit-time descendant assertions. |
| `evals/tests/common/mod.rs` | Reuse atomic readiness, launch markers, identity observation and scoped failure cleanup. |
| `evals/tests/runner.rs` | Preserve standalone signal, deadline, panic/server-death and ordinary lifecycle behavior. |
| `evals/tests/proof.rs` | Run real oracle/null/isolation proof; edit only for a demonstrated issue-local test defect. |
| `evals/README.md` | Clarify only demonstrated divergence from retained interruption, queue-commit and output-limit semantics. |

Inspect `evals/src/report.rs`, TrialResult and refused_row as consumers;
keep formulas/schema unchanged. 0062 and its existing index entry stay intact.
Prefer owning cfg(test) modules; a sibling test harness requires a concrete
cold-path reason under 0020. Do not export test APIs through the normal
library: binary-unit cfg(test) does not enable the library's cfg(test).

## Data model and contracts

Retain schema-1 TrialResult, unique IDs and observed agent statuses. Matrix
owns sticky stop state, ordered slot, aggregate status, output writer and at
most one bootstrap/trial child. Trial owns registered group identities,
shared cleanup results and direct reapers beyond abortable admission.
Register ownership before an await; abort and join workload before cleanup
inventory; normal completion also awaits that inventory's cleanup.

Running → Stopping → Cleaning → Accounting → Finished remains the interrupted
path. First observed signal closes admission; concurrently admitted work
remains owned. Both signals forward TERM once and exit 2. Repeated signals
cannot discard ownership or restart cleanup. Remaining groups clean
concurrently under five-second TERM and five-second cessation/reap budgets;
no serial per-group multiplication or shorter matrix owner-kill timer.

Commit is row enqueue after child reap, validation and pending-signal check;
write/flush acknowledgement is separate and is not crash durability. Prefix
bytes never change. Active interruption is one harness_error after cleanup,
with usable identity/observed statuses and elapsed lifetime. Without usable
output, use a fresh refusal ID and unknown statuses. Unstarted ordered slots
are zero-time refusals without launches. Preserve full cardinality, ordinary
2/1/0 precedence and report attempts/success formulas. Bound retained child
output at 64 KiB while draining independently. Progressing available stdout
is required for complete accounting; output failure retains cleanup ownership.

## Implementation approach

1. Record baseline revision/status, digest equality and current AC-to-test
   inventory before source edits. Reproduce the real CLI issue regression
   and current matrix causal control separately. Do not infer runner proof
   from the shell-helper matrix control or old receipts.
2. Publish helper readiness only after TERM/INT traps are installed. Write
   complete PID/identity content to a temporary file and rename, or validate
   an equally complete acknowledgement. Signal only after observing that
   fact; exercise a controlled pre-ready hold to prove no early signal.
3. Retain MatrixProbe's child in its guard until a deadline-bound try_wait
   completes. Record owned identities before assertions can unwind. Release
   holds idempotently without panicking in Drop. Bound normal post-release
   completion at 30 seconds; on failure allow TERM/cleanup a further
   15 seconds, then identity-checked forced teardown with a five-second
   observation/reap deadline. Never use an unconditional final wait that can
   hang forever. Forced teardown or unresolved identity/survivor is a test
   failure; kernel refusal cannot become a fictitious successful reap.
4. Test cleanup must work after owner bypass or leader-first exit. Retain
   enough verified member/group identity evidence to clean only test-owned
   work even when the original leader is gone. Revalidate before signalling,
   retire empty/recycled groups and diagnose unknowns. Exercise readiness
   failure, assertion unwind and stalled completion using isolated helper
   processes; drain their output independently. This test ownership record
   is not a second production supervisor.
5. At the runner's actual awaited cleanup path, hold live owned work, poll
   that future and acknowledge Pending. Release only after observing this
   event. Before return, assert direct reaper completion and immediate
   non-running owned identities. Apply to interruption by each signal,
   panic and server death; preserve normal cleanup evidence. A cfg(test)
   bypass of the same fence must produce early completion with live work
   and fail the same ordering predicate. Restore the fence and rerun. Do
   not use a marker before await, a separate toy supervisor or an eventual
   check after completion as the causal proof.
6. Preserve the matrix's existing actual-child-wait control and separately
   run ordinary-binary CLI tests with real server/fixture/agent descendants,
   passing prefix, ready active slot, PID-only signal and append-only launch
   evidence. Immediate non-running observations at exit complement both
   causal controls; a later eventual check cannot earn the claim.
7. Map boundary and failure tests before adding cases: pre-admission,
   pre-handler, spawn, between-row, simultaneous commit, final-row and mixed
   repeated signals; TERM resistance, leader-first exit, observation failure
   and expiry; real fallback Cargo, setup refusal and once-only offline/
   locked resolution; malformed/extra/oversized output, exit mismatch,
   inherited pipes, backpressure and broken stdout. Verify deadline creation
   and sharing with deterministic seams: concurrent operations need not
   have mathematically identical timestamps, but cannot restart or serialize
   the cleanup budget. Repair runtime only where a reproducer fails 0062.
8. Validate ordinary dev/release binaries ignore former test environment
   controls. Keep seams cfg(test)-only with no feature flag or normal-library
   API. Preserve standalone runner, schema/report and oracle/null/isolation
   proof. Record command, revision, digest and exit for all checks; retain
   failures. Independent hermetic verification and protected review must
   evaluate the actual final candidate, not this design receipt.

## Test map and quickstart

AC1/3: from `evals/`, run
`cargo test --locked --test report matrix_sigterm_and_sigint_wait_for_the_active_trials_owned_cleanup`.
AC2: run owner-unit matrix
`matrix_completion_fence_is_observed_and_its_negative_control_exits_early`
and the strengthened runner Pending/bypass controls for both signals and
fault paths. The negative case must demonstrably violate the predicate the
restored positive case satisfies. Keep readiness and teardown falsifiers
(AC9) separate so a flaky helper cannot masquerade as a causal result.

AC4–7: retain named boundary/failure tests and add missing falsifiers under
the owning compilation; record the exact mapping in run-local evidence.
AC8/9: run `cargo test --locked --test report`,
`cargo test --locked --test runner`, `cargo test --locked` (all owner units
and proof.rs), `cargo fmt --all -- --check`, and
`cargo clippy --all-targets --locked -- -D warnings` from `evals/`, plus
ordinary dev/release former-control probes. Use real subprocesses/protocols,
observed readiness and deadlines, never mocks or sleep-based synchronization.

AC8/10: execute every [authoritative house gate](../../docs/house-rules.md#gates--part-of-every-change),
with exact coverage and selftest, publish/keyless-seal proofs, licenses,
pinned shim integrity and fail-closed native-tls absence. Compile with
`/home/valentin/source/feedback-loop-ai/swarmail-epic192-delivery/NEXT-194-fast/.forge/bin/brokkr compile --bundle bundles/self`.
Hermetic verify runs pinned implementations and installs nothing. CI owns
its listed chart/history/selftest, Docker, advisory, tag-rate and port-80
proofs; local checks cannot claim those. Protected correctness/security/
guarantees/tests review remains due, with the security hold intact.

Chief checks artifact links/diff/cleanliness and commits exactly the three
spec artifacts unsigned. Run the structural design check against that HEAD;
leave it the last commit. Only the sequence's pinned check owns the design
verdict. Run-local logs and the mandatory drafted result stay uncommitted.

## Risks

| Risk | Mitigation and accepted cost |
| --- | --- |
| Stale findings cause a rewrite | Start at actual 1315846, preserve cfg(test)/matrix controls and map existing proofs. |
| False causal green | Actual Pending acknowledgement, same-predicate bypass and immediate cessation/reap assertions at both links. |
| Signal before trap or partial PID read | Atomic complete trap-ready acknowledgement; controlled pre-ready case. |
| Failure guard drops ownership or hangs | Guard retains child/identities, bounded normal/failure waits and tested unwind/timeout paths. |
| Leader exits, descendant survives | Member identity evidence survives leader loss; targeted fallback and survivor diagnostics. |
| PID/group reuse or unknown observation | Revalidate identities, retire empty groups, fail closed; never broad process-name kills. |
| Serial or renewed cleanup budgets | Concurrent cleanup and shared result reuse; deterministic deadline/repeated-signal checks. |
| Runtime test-control leakage | Owner-local cfg(test) and ordinary dev/release former-control probes. |
| Backpressure hides signals or loses ownership | Independent drainage/output, irreversible queue commitment and broken-output tests. |
| Refusal suffix or scheduling extends exit | O(product) accounting and explicit output limits; no universal matrix wall-time promise. |
| Linux proof overstated | Linux identity/zombie observations; conservative Unix absence and unsupported escape limits. |
| Wrong engine or stale delivery receipts | Absolute digest-matched engine, candidate-bound logs, independent verify and protected review. |
