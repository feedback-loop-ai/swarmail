# Recovery plan — matrix cancellation

Implements [spec.md](spec.md); retains proposed
[0062](../../docs/decisions/0062-matrix-cancellation.md). Candidate source
already implements the lifecycle. This plan audits and repairs its proof;
it does not schedule a second implementation of cancellation.

## Technical context

Rust 2024 standalone `evals/` workspace; existing Tokio process/signal/I/O,
libc, serde and tempfile. Sequential matrix and one active trial. No new
crate, feature, dependency, manifest or lock changes. Under accepted 0055,
evals remains outside the root exact-coverage report and owes its own suite.
No plan template exists in this checkout; required sections are supplied here.

Base candidate: `1a2f36c2b28e5e0bbce740c71485aeac39567b4d`. The current
recovery is engine-class. The predecessor is concluded, not resumable by
rewriting history. Use the current run's design/check/implementation/verify/
review flow as selected by engine policy. Four independent correctness,
security, guarantees and tests lenses remain mandatory, with no fallback
provider or security-hold bypass. Never push, merge or close the issue.

## Constitution check

Preserve 0001–0005 and 0055. Reuse proposed 0062 without accepting it or
changing its semantics. Test strengthening and removal of accidental test
controls do not create a new contract (0050 §6). Any conflict discovered with
accepted authority requires an operator ruling before contradictory work.
Keep 0020's real-process, deterministic readiness and cfg(test)-only seam
discipline. Root coverage remains exact 100%; no exclusions, extra root
workspace members or license widening. Keep gate/recipe/policy files intact.
Unsigned commits only (0056). Engine transitions and journal records are
never fabricated by this seat.

## Research and evidence

Both recovery positions and both intake framings were read. Current source
confirms that matrix installs handlers before resolution, retains its active
child and reader separately, forwards SIGTERM once for either matrix signal,
and waits for the child. Runner retains groups/reapers beyond the abortable
body, aborts and joins admission before cleanup, and fences normal completion
too. Those mechanisms should be preserved unless a reproducer finds a defect.

The previous verifier refused host digest `408c3c4d…a086`, not cancellation.
The local `.forge/bin/brokkr` was independently hashed and matches tooling's
`e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`.
Resolve its absolute candidate-local path for every boxed compile/check;
version or PATH equivalence is insufficient. Do not install or edit tooling.

The exact-candidate issue regression passed during this chief step (see spec).
It still lacks an acknowledgement that the matrix wait future is pending
while cleanup is held. `MatrixGuard::wait` releases before waiting, and its
Drop allows only three seconds before killing matrix. Production main/runner
also read test environment hold/injection variables; this violates the
existing test-only plan and can indefinitely stop cleanup. These are the
scoped recovery repairs. No negative-control execution is claimed yet.

Historical 2026-10-24 chief receipt: the earlier design's root tests and
coverage refused an absent 0062 index row; other listed design checks passed.
Candidate 1a2f36c supplies the row. Earlier `.forge/design-194-checks/` logs
remain historical and must not be reused as current receipts. The old
preimplementation tasks are replaced by the recovery tasks, not marked done
from implementer assertions.

## Project structure and touched files

| Path | Recovery responsibility |
| --- | --- |
| `specs/matrix-cancellation/{spec,plan,tasks}.md` | Only files in this chief commit; current framing, rulings, requirements and paired work. |
| `evals/src/main.rs` | Remove production test injection/holds; cfg(test)-only matrix wait, admission/commit/startup controls and test harness re-entry; preserve runtime ownership. |
| `evals/src/runner.rs` | Remove production cleanup/pre-handler holds; local test-only cleanup/reaper controls; preserve handler ordering and shared cleanup. |
| `evals/tests/report.rs` | Ordinary-binary PID-only signals/accounting, bounded guard waits and failure cleanup; migrate internal controls into owning unit tests. |
| `evals/tests/common/mod.rs` | Unique readiness/launch evidence, identity-aware observation and scoped cleanup helpers. |
| `evals/tests/runner.rs` | Standalone signal, panic, deadline, server-death, resistant descendant and reaping regressions. |
| `evals/tests/proof.rs` | Existing oracle/null/isolation suite, run unchanged unless a demonstrated local proof defect needs repair. |
| `evals/README.md` | Align existing contract with queued-row commitment, limits and ordinary binary behavior where clarification is needed. |

Inspect `evals/src/report.rs` and TrialResult/refused_row as consumers; preserve
schema and formulas. 0062 and its index remain unchanged. Root `src/`,
accepted decisions, manifests/locks and delivery machinery are outside scope.
Prefer existing owning test modules; a sibling test harness is permitted only
when a concrete cold-path test warrants 0020's placement rule.

## Data model and contracts

Retain TrialResult/schema 1. Matrix holds sticky interruption, position,
aggregate flags and at most one owned bootstrap/trial process. Trial records
retain direct-child wait/status and registered group identity until cleanup
finishes; aborting workload must not abort those owners. Register spawns
before any await. Retire empty groups and never deliberately signal recycled
identities. Linux records group/session/start/state, with zombies non-running;
direct children still owe wait. Other Unix uses conservative group absence.

Keep Running → Stopping → Cleaning → Accounting → Finished. Ordinary
completion loops Running. First observed signal closes admission forever.
A concurrently spawned child remains owned. Pending signal at commitment
wins over an unqueued normal result. Enqueue is irreversible; write/flush
acknowledgement precedes continuing, and a signal during writing changes the
exit verdict without rewriting the line. Output failure closes admission and
retains cleanup ownership; unavailable output limits complete JSONL.

After cleanup, interrupted active work has one harness-error row with usable
identity/observed agent exits and measured lifetime. No usable row means fresh
refusal identity and unknown exits. Unstarted slots have zero-time refusals,
fresh IDs and no launch. Preserve exact task → agent → repetition order,
exit 2 on either handled signal, normal precedence 2/1/0 and report formulas.

## Implementation approach

1. Preserve the exact-candidate receipt and source findings. Before source
   changes, record HEAD and worktree status; after every repair, record the
   successor revision. Chief docs-only HEAD is already a successor, even
   though executable source is unchanged. Never claim final proof of 1a2f36c.
2. Remove normal-build `EVAL_TEST_MATRIX_SIGNAL_SETUP_FAIL`, admission,
   commit, final, cleanup and pre-handler controls. Introduce only cfg(test)
   local seams. No public control socket, production environment hold or
   feature flag. Unit-test helper re-entry runs the owner compiled with its
   seams; integration CLI tests run the ordinary binary independently.
   Evals library cfg(test) is not enabled merely because its binary is being
   tested: place runner controls/tests in the runner's own test compilation
   and matrix controls/tests in main's test compilation. Do not expose test
   APIs through a normal library build to bridge that boundary.
3. Build the causal fence test around the actual owner futures. After real
   owned-work readiness and held cleanup acknowledgement, explicitly poll
   the matrix wait path while completion is unavailable; require Pending
   and emit an acknowledgement from that pending observation. An entry
   marker before awaiting is insufficient. Gate release on this event,
   observe group cessation/direct waits, then allow completion. Use the
   analogous local runner control to prove cleanup precedes trial return.
   Together with the ordinary CLI test these establish both links.
4. Add a negative control that removes/bypasses the actual awaited completion
   fence and must fail the same ordered-event/Pending assertion. Run for
   SIGTERM and SIGINT separately. A test-only alternate path must exercise
   the same orchestration; do not build an unrelated miniature supervisor.
   If a temporary source mutation is used, isolate it, record the exact diff
   and failure, restore it and rerun the unmutated proof before committing.
   Mere missing markers, longer sleeps or repeated greens cannot stand in
   for demonstrably failing the completion-fence removal.
5. Make guards release all holds, stop admission and wait within an explicit
   outer deadline. Allow at least the complete five-second TERM and
   five-second cessation/reap window after release, plus a stated scheduling/
   output margin; use a 30-second post-release test deadline. Drain output
   independently. On expiry, diagnose the identities still present and
   clean only test-owned groups after revalidating identity, then reap owned
   direct children. A forced test cleanup is a test failure, never a product
   cleanup success. Retain identities before assertions can unwind; test
   readiness failure, assertion unwind and deadline cleanup themselves.
6. Map every AC to executable proof (tasks below), reuse valid tests and fill
   actual gaps. Keep setup, pre-handler, admission/spawn/completion/final-row,
   repeated signals, bootstrap and output controls when migrating seams.
   Restore ordinary runtime paths first; do not remove coverage to avoid
   the migration. Repair production ownership only on a concrete reproducer.
7. Run independent evals proofs and all canonical gates against final source,
   then independent hermetic verify and four-lens review. Keep logs tied to
   revision, executable digest, command and exit; failures remain outstanding.
   No candidate script or local assertion selects an engine transition.

## Test design and proof map

Public CLI proof sends each signal only to matrix PID after a passing prefix
and observed live trial/server/fixture/agent/descendant readiness. Use fresh
scratch paths, AUTH inboxes, loopback port zero and append-only launch markers.
Assert immediate identity-aware cessation at matrix exit, trial wait and
no suffix launch, then exact rows and real report ingestion (AC1/3).

Internal re-executed tests supply causal pending-future controls and real
owned processes for both completion fences (AC2); deterministic readiness
and bounded deadlines replace sleep-dependent synchronization. Scoped helper
processes contain any environment state. Keep mixed repeated-signal and
startup/commit controls (AC4), escalation/leader-first/observation/budget
failures (AC5), real cancellable bootstrap plus setup refusal (AC6), and
bounded drain/invalid output/backpressure/broken stdout controls (AC7).

Standalone and full oracle/null/isolation tests prove preserved behavior
(AC8). Check ordinary dev and release paths with former test variables set:
they must not create hold acknowledgements or inject failure; compilation
boundaries and focused tests prove the seams are absent (AC9). Failure-path
guard tests must reap their owned direct children and report unknown
observations as failures. Restrict Linux /proc proofs explicitly; portable
observation errors must not be treated as absence.

## Quickstart and validation

From `evals/`, run the issue regression first:
`cargo test --locked --test report matrix_sigterm_and_sigint_wait_for_the_active_trials_owned_cleanup`.
Then `cargo test --locked --test report`, `cargo test --locked --test runner`,
`cargo test --locked` (including owner unit tests and oracle/null/isolation),
`cargo fmt --all -- --check`, and
`cargo clippy --all-targets --locked -- -D warnings`.
Run the explicit AC2 negative control and its restored positive case as well.

Run every [canonical house gate](../../docs/house-rules.md#gates--part-of-every-change),
with locked dependencies, exact coverage and its selftest, publish/keyless-seal
proofs, license check, pinned shim integrity and fail-closed native-tls absence.
Use the absolute digest-matched candidate-local Brokkr for compile. Hermetic
verify executes pinned canonical implementations, installs nothing, and owes
its own receipts; local chief checks do not replace it. CI-only chart/history/
selftest, Docker, RustSec, tag rate proof and port-80 obligations stay with CI.

Chief checks links/diff/committed cleanliness, commits exactly the three spec
artifacts unsigned, then runs `bash bundles/self/scripts/design-check.sh check .`.
Leave that commit last. The sequence's pinned check alone owns advancement.
Run-local logs and the mandatory drafted result are not committed.

## Risks

| Risk | Mitigation and accepted cost |
| --- | --- |
| Passing old test hides missing causal proof | Explicit Pending observation, ordered release and fence-removal negative control; extra test work. |
| Seam compiled into ordinary binaries | cfg(test) at each owning compilation, ordinary dev/release tests with former variables; separate CLI proof. |
| Test fails and kills matrix too soon | Release holds, 30-second outer deadline, owned-identity fallback cleanup and reaping; fail on escalation. |
| Signal/spawn race loses ownership | No await before registration, sticky stop, pre-handler invariant and boundary tests. |
| Group leader exit or EOF mistaken for cessation | Retain group/reapers, observe running members and direct waits independently. |
| Reused identity or unknown observation | Revalidate owned identity, retire empty groups, diagnose unknowns; no unrelated signalling. |
| Repeated signals multiply cleanup or kill owner | Shared five-plus-five windows, no reset or competing matrix timer. |
| Backpressure hides signals or loses rows | Independent bounded drainage, irreversible queue boundary, output-error tests and explicit output limit. |
| Large refusal suffix slows exit | Stream O(product) rows; no universal ten-second matrix promise. |
| Linux proof presented as universal | Explicit Linux identity/zombie proof; conservative Unix absence and stated unsupported escapes. |
| Wrong executable or stale receipts | Absolute local engine plus digest; fresh revision-bound logs and hermetic verify. |
| Recovery grows into redesign | Narrow reproducer-backed repairs; preserve schema, root code, tooling and dependencies. |
