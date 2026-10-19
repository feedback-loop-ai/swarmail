# Deterministic commit-at-stop regression

Date: 2026-10-19. Epic #120 batch 5, re-drive 5; issue #143.
Base: `9d19fff` on `epic120-delivery-constitution`.
Status: design draft for implementation and deterministic validation.

## Feature and rationale

Make `an_in_flight_commit_neither_holds_stop_nor_half_lands` establish its
interleaving explicitly while retaining real SMTP, a real running server and
real SQLite persistence. An admitted blocking mutation must remain held until
the test releases it: stop returns while it is held, the socket closes without
DATA acceptance, and release completes exactly one consistent mutation.

The current test has two independent scheduling races. Its delivery metric is
live during parsing, before the mutation passes shutdown admission. Also, its
external SQLite writer can outlive the connection's five-second busy timeout;
a refused insert cannot land when that writer is eventually released. Both
produce a permanently empty landing tail on a correct tree. Its comments
describe refusal although its last assertion requires successful completion.

The controlling scope is the operator's re-drive-5 resume instruction. The
older framings in `.brokkr/state/tasks/`, the batch-5 intake and resume-point-3
intake are historical context, not instructions to repeat completed work.
The issue bodies and comments for #123, #124, #125, #140 and #143 remain the
batch's acceptance baseline. This supplement replaces only the timing-based
proof of the named #143 regression; [delivery-gates](../delivery-gates/spec.md)
continues to own the other batch requirements and its dated clarifications.

## Panel rulings

1. **Adopt simplicity's existing per-store gate and owning harness.** Invoke
   `run_pre_mirror_gate` once from `Store::record`, after the stopping refusal
   and before the mirror call, under the existing mutation boundary. Remove
   the two clear-specific invocations. Move only the named regression to
   `src/store/persist_tests.rs`; keep all seam code `#[cfg(test)]`. Its cold
   timeout diagnostics and private seam justify that placement under 0020.
2. **Adopt robustness's two-race diagnosis and separate failure proofs.** Gate
   entry must mean shutdown admission has passed, not merely that DATA or
   parsing is in flight. Stop must be observed returning while the mutation
   remains held. Zero rows after release is a failure, never an allowed
   refusal outcome for this controlled successful-persistence fixture.
3. **Reject robustness's public busy-timeout Config field and commit metric.**
   A 60-second timeout increases the scheduling window but still has an expiry;
   it does not make lifetime depend on the test's explicit release. A new
   tunable and metric add public contracts to repair a test. The gate supplies
   a durable commit-stage handshake without those additions. Cost of this
   ruling: this test no longer exercises an actual SQLite busy-handler wait,
   and operators gain no new commit-stage metric. Existing real-lock refusal
   tests and the five-second pragma/default-override tripwire remain required.
4. **Reconcile the harness objection without importing the integration suite.**
   Do not `#[path]`-include `tests/common/mod.rs` into the library test unit.
   Keep the small wire driver and bounded diagnostics local to the existing
   persistence harness. Cost: a small amount of local SMTP test plumbing;
   no generic test framework, shared global state or production helper API.
5. **Reject window shrinking and weaker assertions.** Reordering lock release,
   raising timeouts, serializing tests, retrying until green, skipping the test
   or allowing an empty landing tail cannot satisfy this design. Adopt both
   positions' demand for mutant evidence and loaded instrumented execution.

## Contract and decision scope

This is a test repair, not a semantic amendment. No new decision is declared:
it preserves [0018](../../docs/decisions/0018-shutdown-observed-waits-and-explicit-bounds.md)
as amended by [0024](../../docs/decisions/0024-shutdown-refusal-at-the-boundary.md),
the whole-mutation offload of
[0023](../../docs/decisions/0023-off-worker-commits.md), and the harness/seam
rules of [0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md).
The admitted-before-stop case is already expressly permitted by 0024 §3.
[0050](../../docs/decisions/0050-delivery-gates.md) remains proposed and unchanged.

Shipping configuration, metrics, shutdown logic, mutation ordering and the
five-second SQLite busy bound stay unchanged. Preserve synchronous
commit-before-acceptance, subscribe-before-scan, oldest-only pruning and
ingest-time extraction (0001–0005); the server fence, awaits cap, single-writer
lock, extraction counts, and all prior batch repairs remain constraints.
No dependencies, coverage exceptions, feature flags, charts or delivery-policy
changes are authorized. Further fixes require a concrete verify/review finding.

## Acceptance Criteria

- **AC1 — Admission is explicit.** The moved test retains its leaf name and
  waits for an entered handshake from the per-instance gate after `record`'s
  shutdown check, while the mutation boundary is held. There is exactly one
  invocation site. No delivery-metric poll, external writer lock, sleep or
  retry loop establishes this interleaving.
- **AC2 — Stop is independent.** With the gate still held, the real server's
  `stop()` returns under the existing fence-derived 12-second observation
  budget (10-second fence plus two-second scheduling allowance). The DATA
  reply is EOF/reset or a non-250 refusal, never acceptance; both SMTP and
  HTTP listeners refuse connections, as in the original test. A helper panic,
  timeout or failed task join is not converted into a valid protocol refusal.
- **AC3 — No early visibility.** Before releasing the gate, independent
  read-only SQLite inspection finds zero emails and unchanged persisted
  counters; memory and its counters also contain no applied mail. Reads must
  not acquire the held mutation lock or introduce a deadlock. No fan-out
  notification for the mail has arrived.
- **AC4 — Completion is mandatory and consistent.** Subscribe before sending
  DATA. After the stopped/empty assertions, release the gate and await the
  existing post-apply notification under a named diagnostic deadline. Memory
  and SQLite then contain exactly one identical mail (id, inbox, envelope,
  raw content), inserted count increases by one and dropped count stays zero.
  The store remains alive through these checks; no replay or second send is
  used to obtain the required row.
- **AC5 — Failure unwinds.** A release guard exists before the task can enter
  the gate, and releases on assertion failure, cancellation and timeout.
  Every connect/read/write/join and gate-entry/completion observation is
  bounded and identifies its step. No blocking channel receive runs on an
  async worker. A deliberate red execution exits without hanging runtime
  teardown or leaving its temporary database/WAL/SHM files.
- **AC6 — Each property can fail.** Record separate compiled mutants and
  their expected diagnostics: stop joins the held SMTP mutation; an admitted
  mutation is discarded after release; memory is applied before the gated
  mirror call; DATA is acknowledged before the awaited commit. The named
  regression must fail at the corresponding stop, completion, early-visibility
  or acceptance assertion. Compilation failure, outer process timeout or an
  unrelated failing test does not count. Restore each mutant and rerun green.
- **AC7 — Loaded proof.** Record at least 20 consecutive passes of the exact
  moved test under synthetic CPU workers covering every available core, with
  normal test parallelism. Also pass the full instrumented coverage gate
  under that load. Record core/affinity discovery, worker count, commands,
  candidate revision, statuses and cleanup. Retain every failed attempt;
  a retry-selected sample does not satisfy the criterion.
- **AC8 — Existing protections and gates remain.** Existing gate installers,
  persistence ordering/panic/refusal tests, queued-stop tests and SQLite busy
  default/override pins pass. The final implementation passes the complete
  house gate inventory, including exact 100/100/100 coverage, both script
  proofs, pinned warning-free compile and fail-closed native-tls absence.
- **AC9 — Honest delivery.** Chief commits only this spec, plan and tasks in
  one new signed commit. Implementation appends signed commits and records
  AC6–AC8 execution evidence under ignored run state, without claiming prior
  receipts prove the repaired tree. No rewrite, squash, push, PR or issue
  closure; the branch remains checked out and the tracked tree clean.
