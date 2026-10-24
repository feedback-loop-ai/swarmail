# Recover the matrix cancellation proof

Date: 2026-10-24. Epic #192, issue #194. Recovery run:
`epic-192-issue-194-verification--cae786a0`.

Decision: 0062 — [Matrix cancellation and cleanup completion](../../docs/decisions/0062-matrix-cancellation.md), already committed and proposed; acceptance is the operator's.

## Feature and rationale

SIGTERM or SIGINT sent only to the matrix PID must stop admission, cancel
its active trial, and await owned cleanup, with complete truthful schema-1
accounting when output remains available. This recovery retains that
contract and its implementation at candidate
`1a2f36c2b28e5e0bbce740c71485aeac39567b4d`. It replaces the predecessor's
preimplementation framing with a fresh proof and narrowly scoped repairs.

The concluded run `epic-192-issue-194-stop-active-e-b454f70a` stopped at
VERIFY-FAIL because the launched host Brokkr digest was unrecorded. That
refusal is not an executed cancellation failure. The candidate-local
`.forge/bin/brokkr` matches `bundles/self/tooling.json`, SHA-256
`e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`.
Invoke that absolute executable; do not install, change pins, reopen the
concluded journal or relabel earlier receipts.

Current triage remains `engine`: [0057](../../docs/decisions/0057-codex-delivery-recipes.md)
requires full design, hermetic verification and four independent review
lenses. The engine selects transitions. [0050](../../docs/decisions/0050-delivery-gates.md)
binds design to the chief's HEAD and verification to actual executed proofs.
This design commit necessarily creates a successor of 1a2f36c; later evidence
must identify its actual revision. Exact-candidate regression evidence is
separate from successor verification.

[0055 §§6–10](../../docs/decisions/0055-agent-eval-harness.md) remains the
accepted authority for lifecycle, schema, accounting and report formulas.
0062 already proposes the compatible interruption semantics retained below;
this recovery introduces no new public contract or amendment and therefore
needs no new numbered decision. Only the operator can accept 0062.
[0045 §8](../../docs/decisions/0045-delivery-repairs.md) permits compatible
work while proposed. Frozen [0001](../../docs/decisions/0001-synchronous-insert-lossless.md),
[0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md),
[0003](../../docs/decisions/0003-oldest-only-pruning.md), AUTH routing and
[0005](../../docs/decisions/0005-extraction-at-ingest.md) are unchanged.

## Panel rulings and their costs

These rulings synthesize the recovery positions, not the earlier panel.
Both recovery positions agree on the ownership design and the need to close
the proof gap. There is no incompatible proposal to average: the governing
tradeoff is minimal recovery work versus sufficient causal evidence.

- Adopt simplicity's existing sequential matrix, one active child, direct
  PID forwarding and one trial cleanup owner. Adopt robustness's retained
  reapers, observed cessation and concurrent shared cleanup budget. Reject
  a rewrite, second supervisor, matrix kill deadline, cgroup, subreaper,
  parent-side duplicate kill registry or dependency. Cost: cleanup can delay
  matrix exit; escaped or hostile processes remain outside the guarantee.
- Rule for robustness's causal proof requirement over a tool-identity-only
  recovery. Simplicity also identifies this concrete gap: the existing test
  checks matrix liveness at a cleanup marker, then releases before waiting.
  Preserve AC2 and require an executable negative control. Cost: a narrow
  test repair and successor evidence are necessary even if the old test
  passes; exact-1a2f36c green cannot close the issue.
- Adopt simplicity's removal of normal-build environment holds and failure
  injection. Reconcile this with robustness's demand for boundary coverage
  through cfg(test)-only controls in re-executed test harnesses. Public CLI
  tests still exercise the ordinary binary. Reject deleting the boundary
  proofs or adding feature flags. Cost: separate CLI and internal causal
  tests and more deliberate helper entry points.
- Adopt robustness's failure-safe teardown correction. A three-second
  guard deadline must not kill the trial's parent before its five-plus-five
  cleanup window. Reject unbounded ordinary waits. Cost: recorded process
  identities, a bounded outer test deadline and explicit survivor diagnostics;
  test failure must not silently leak into other runs.
- Adopt both positions' handler-before-owned-spawn invariant instead of a
  mandatory startup handshake. Cost: pre-handler termination can yield no
  row, requiring an honest fallback and a regression guarding spawn order.
- Adopt both positions' schema 1, exit 2, immutable prefix and full ordered
  refusal suffix. Reject partial output as normal cancellation behavior,
  new outcomes, retries or report denominator changes. Cost: O(product)
  output and attempts that include unexecuted slots. Unavailable output is
  an explicit limit, never permission to fabricate statuses.
- Retain the existing spec directory and proposed 0062. Reject new recipe,
  engine, root-server, gate or accepted-decision changes. Brokkr identity
  preflight already has a local solution. Cost: full independent gates and
  protected four-lens review remain due; panel passes cannot substitute.

## Required behavior

### R1 — Admission and interruption

After valid input parsing, install SIGTERM and SIGINT subscriptions before
any subprocess, including fallback Cargo. Installation failure starts none,
prints a useful stderr diagnostic, emits refusal rows for the valid product,
and exits 2. Invalid inputs still exit 2 with no rows.

The first observed signal permanently latches stopping. Check pending
signals before bootstrap, before every trial spawn, at result commitment,
and before matrix return. When completion and cancellation are ready at
one decision point, cancellation wins. A spawn already issued concurrently
with observation is admitted: retain its handle immediately, cancel it,
and await it. No subsequent slot launches. Further signals remain handled
through cleanup and accounting; they neither restart the clock nor bypass
cleanup. Ordinary harness errors without interruption still continue.

Cancel the active trial once with SIGTERM to its PID, not a process-name or
matrix-group kill. Retain and reap it; do not cancel the wait by dropping
its future. A concurrently exited child is still reaped. A forwarding error
other than already-exited is diagnosed, forces exit 2, and does not release
ownership or start another trial; continue awaiting the owner. There is no
promise of finite recovery from a kernel that refuses signalling owned work.

### R2 — Cleanup completion

Within the trial, child handles/reaper tasks and owned group records must
outlive cancellation or panic of the workload. Stop and join workload
admission, then terminate all remaining owned groups concurrently. Allow
one shared five-second TERM grace, then SIGKILL survivors and allow one
shared five-second cessation/reaping window. Neither budget multiplies by
agent/server count or resets on a repeated signal. Existing earlier forced
termination may finish sooner. The same completion fence must cover normal
completion racing cancellation, so a completed leader cannot leave a live
registered descendant behind.

A successful cleanup requires reaping direct children and observing no
running members of the owned groups. Group leader exit, pipe EOF and a
successful kill syscall are insufficient. A zombie is non-running, but a
zombie direct child still owes a wait. Grandchildren adopted by another
process need not be reaped by this harness. Retire observed-empty groups;
never deliberately signal a recycled group identity or unrelated process.

The matrix reaps the trial only after the trial's cleanup operation finishes;
it has no shorter independent forced-kill deadline. Bootstrap Cargo has its
own group and is cancelled/reaped by the matrix using the same budget and
completion rules. Ten seconds is the cleanup operation budget once stopped
admission reaches its cleanup owner, not a universal matrix wall-clock
promise: scheduling, final output availability and suffix cardinality add
latency. Bound failure or observation failure is a named cleanup failure,
exit 2 and harness-error accounting, never evidence of successful cessation.
No finite bound can guarantee death of an uninterruptible kernel task.

Linux is the explicit descendant-ordering proof platform: inspect only
members of registered owned groups via `/proc`, distinguish start identity,
non-running zombie states, disappearance and observation errors. On other
supported Unix platforms, use conservative group-existence checks: only
ESRCH proves absence, EPERM/unknown does not. This may time out on adopted
zombies; diagnose that conservative failure rather than call it success.
Do not interpret absent `/proc` as successful Unix-wide proof. No Windows
support, hostile daemon escape or new confinement claim is introduced.

### R3 — Result commitment and JSONL

Only matrix writes matrix stdout. A slot becomes committed when matrix has
reaped its child, accepted exactly one valid matching result, checked pending
cancellation, and enqueued that row to ordered output. Enqueue is the irreversible
commit boundary; the writer then acknowledges write/flush completion. A
signal during that write changes the exit verdict, never rewrites a queued
line. Flush is not crash-durable storage. Committed prefix
rows are immutable. Uncommitted active work overtaken by cancellation emits
exactly one `harness_error`/`harness_error` row after cleanup. Preserve a
usable trial ID and actual observed agent statuses; normalize a raced
pass/fail to harness error. Its wall time includes actual lifetime through
cleanup. With no usable result, create a fresh refusal ID and leave statuses
unknown, using measured active elapsed time. Never derive agent status from
the trial process's exit status.

Unstarted elements emit existing `refused_row` values in task → agent →
repetition order: requested labels, unique fresh IDs, zero wall time, null
singular status, empty child exits, harness error and `unconfined`. No process
starts for these elements. Missing-binary failure retains complete accounting.

Retain at most 64 KiB of trial result output while continuing to drain;
overflow, malformed or extra nonempty lines, invalid schema, wrong labels,
or an incompatible child exit/result pair refuse that slot. Normal trial
exit must match its row's 0/1/2 mapping; a usable row from a signal-terminated
trial may only supply identity/status evidence for a harness-error row.
Child wait and stdout drainage are independent: neither EOF nor a row means
cleanup finished. After child exit, a pipe that still cannot finish within
five seconds is an output failure, not permission to invent a valid result.

Either handled signal makes matrix exit **2**, even with a wholly passing
prefix or when observed after the final row and before return. Without
interruption, preserve aggregate precedence 2 before 1 before 0. Emit one
concise matrix interruption diagnostic with the unstarted-slot count to
stderr; cleanup errors may add specific diagnostics. stdout contains JSONL
only. Report accepts the rows without changing schema or formulas: attempts
means requested result slots, not necessarily executed agents; success over
pass+fail and passes/attempts retain their existing meanings.

### R4 — Output and failure limits

Cancellation observation and cleanup must remain runnable while stdout is
backpressured. Use bounded ordered asynchronous output; do not hold a
blocking stdout lock in the cancellation owner. Broken output stops admission,
cleans active work and exits 2; complete JSONL cannot be promised then.
Writable, progressing output and handled interruption are prerequisites for
complete accounting. SIGKILL, process abort, host failure, daemonizing or
hostile commands and unkillable kernel tasks remain explicit residuals under
0055. Do not conceal a cleanup timeout behind a passing eventual check.

## Acceptance Criteria

- **AC1:** For SIGTERM and SIGINT separately, signal only a real matrix PID
  after an observed passing prefix and live active trial/server/fixture/
  agent/descendant readiness. Matrix exits 2, launches no suffix element,
  and all recorded supported owned processes are non-running at exit.
- **AC2:** Hold cleanup at an acknowledged test-only boundary while a known
  owned process is live. An acknowledged matrix wait state must precede
  release; it cannot report completion. Release, observe cessation/reaping,
  then permit the completion fence. Removing the awaited fence must fail
  this control. Immediate identity-aware observation at exit supplements
  this ordering proof; no post-exit eventual-death loop earns the claim.
- **AC3:** Both signal cases produce the exact requested ordered cardinality,
  unique IDs, unchanged prefix, one truthful active error and zero-time
  refusal suffix; no extra stdout lines. Real `report` ingestion verifies
  attempts, errors and unchanged formulas. A recorded suffix launch marker
  is forbidden even if that process later dies.
- **AC4:** Controlled boundary tests cover cancellation before first
  admission, before trial handlers, during spawn ownership, between rows,
  at simultaneous completion, after the final row, and repeated mixed
  signals during cleanup. No later launch, lost handle or reset deadline;
  the early no-row trial fallback is honest.
- **AC5:** Real TERM-resistant descendants and a leader that exits before
  its descendant exercise escalation and retained group ownership. Direct
  children are reaped. Observation errors and forced cleanup-budget expiry
  produce explicit harness errors, never a successful cleanup assertion.
- **AC6:** Signal setup failure starts no process and accounts for all valid
  slots. A ready real fallback-build subprocess and descendant are cancelled
  and awaited without starting any trial. Preserve once-only offline/locked
  build resolution and complete missing-binary accounting.
- **AC7:** Spawn failure, invalid/extra/oversized child output, exit mismatch,
  inherited pipe delay, stdout backpressure and broken output cannot drop
  ownership. Cancellation still reaches cleanup; only output-unavailable
  cases may lack complete JSONL, with exit 2 and a diagnostic.
- **AC8:** Existing standalone interruption, deadline, panic, server-death,
  oracle/null/isolation, normal matrix aggregation, ordinary-error continuation,
  invalid-input and missing-binary tests remain green. Root source, frozen
  guarantees, report math, schemas, manifests/locks, dependencies and gates
  remain unchanged. Run independent evals checks and the full canonical
  house-rule gates; root green cannot stand in for evals evidence.

- **AC9:** Ordinary dev/release executables contain no effective test hold or
  failure-injection control. Boundary tests run cfg(test)-only seams, including
  AC2's negative control; real PID-only CLI coverage remains. Failed readiness,
  assertion and deadline paths release holds, finish owned cleanup or report
  identified survivors, reap direct children, and never signal unrelated work.
- **AC10:** Fresh evidence records candidate revision and engine digest.
  The final candidate passes independent evals checks, every canonical house
  gate and the selected four-lens review with the protected security hold.
  No stale receipt, journal rewrite, push, merge or issue close is used.

## Recovery evidence and delivery boundary

2026-10-24: before editing these artifacts, the chief ran
`cargo test --locked --test report matrix_sigterm_and_sigint_wait_for_the_active_trials_owned_cleanup`
from `evals/` on exact 1a2f36c: 1 passed, 14 filtered. Run-local revision and
log are under `.forge/design-194-recovery/`. This proves the existing
regression passes, not AC2's missing causal control or full verification.
Source inspection confirms ordinary-build test controls and the three-second
MatrixGuard teardown; no mutation result is claimed.

The chief commits exactly this directory's spec.md, plan.md and tasks.md.
The already committed, clean 0062 remains declared and proposed; its index
row is present at 1a2f36c. The previous missing-index gate refusal is historical,
not a reason to add it again. No production changes occur in this phase.
The sequence's pinned deterministic check alone owns the design verdict.
Implementation, full verification and reviews remain outstanding as paired
in [tasks.md](tasks.md); drafting is not delivery success.
