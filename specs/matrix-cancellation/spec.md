# Recover the matrix cancellation proof

Date: 2026-10-24. Epic #192, issue #194. Recovery run:
`continue-epic-192-issue-194-from-8d385d4d`, continuing stopped
`epic-192-issue-194-verification--cae786a0`.

Decision: 0062 — [Matrix cancellation and cleanup completion](../../docs/decisions/0062-matrix-cancellation.md), already committed and proposed; acceptance is the operator's.

## Feature and rationale

SIGTERM or SIGINT sent only to the matrix PID must stop admission, cancel
its active trial, and await owned cleanup, with complete truthful schema-1
accounting when output remains available. This recovery retains that
contract and the implementation now at
`1315846de12787f78a6f66b1885bbbc2aff72786`, following original implementation
`1a2f36c` and recovery design `1843c55`. It reconciles the remaining proof
work with the repaired successor; it does not implement cancellation again.

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
This design commit necessarily creates a successor of 1315846; later evidence
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

The current simplicity and robustness positions agree on runtime ownership.
Their dispute is how much proof repair is necessary. Rule as follows rather
than averaging the requirements or copying stale findings from 1a2f36c.

- Adopt simplicity's sequential matrix, one active trial and one cleanup
  owner, with robustness's retained reapers and observed cessation. Reject
  a second supervisor, parent-side production descendant registry, new
  startup protocol, cgroup, subreaper or dependency. Cost: cancellation waits
  for its cleanup owner; escaped/hostile processes remain unsupported.
- Adopt simplicity's reuse of the repaired matrix Pending/bypass control
  and cfg(test) migration at 1315846. Reject describing those repairs as
  still absent. Cost: independent execution and ordinary dev/release proof
  remain due; implementer claims cannot close them.
- Rule for robustness's concrete runner proof repair over stopping after
  matrix proof. Simplicity makes this conditional on mapping existing
  evidence; inspected runner tests only establish liveness at entry and
  eventual death. Require actual cleanup-future Pending acknowledgement,
  a matching bypass falsifier and cessation/direct reaps before return for
  both signals, panic and server death. Cost to the narrower approach:
  additional local test seams and execution, even if runtime code is sound.
- Adopt robustness's trap-ready acknowledgement: the matrix shell helper
  currently publishes its PID before installing traps. Publish complete
  readiness atomically after traps are active. This is a test-helper fix,
  not a new trial startup protocol. Cost: one additional readiness fact;
  file existence alone is no longer an adequate signal trigger.
- Adopt robustness's bounded guard and survivor-ownership repairs.
  MatrixProbe currently takes its child out before an unbounded wait;
  runner fallback depends on the original leader still existing. Require
  bounded ordinary and failure waits, early identity retention and teardown
  after leader-first exit or owner bypass. Reject relying on longer Drop
  delays alone. Cost: explicit test-owned identity records and failure-path
  controls; no duplicate production supervisor follows from this ruling.
- Reconcile the shared cleanup budget with concurrent implementation:
  groups must not incur serial five-plus-five windows, and repeated signals
  cannot restart cleanup. Map actual deadline creation and reuse; do not
  call independently scheduled deadlines mathematically identical. If a
  reproducer shows renewed or multiplied windows, repair it within 0062.
- Adopt both positions' existing schema 1, exit 2, immutable queued prefix
  and complete ordered refusal suffix. Reject a partial-prefix exception,
  new outcome, retry or report denominator change. Cost: O(product) output
  and attempts including unexecuted slots; no universal wall-time promise.
- Retain this directory and proposed 0062. The robustness failure analysis
  of mail acceptance, webhooks, caps, waits, chaos, hostile bytes and TLS
  establishes preservation constraints, not root-server implementation work.
  Reject engine/pin/journal changes and gate weakening. Cost: all evals
  proofs, house gates and protected four-lens review remain required.

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
- **AC2:** Prove both completion links independently: matrix awaiting the
  active child and runner awaiting owned cleanup. Hold known live owned
  work; acknowledge the actual awaited future returning Pending before
  release, then observe cessation and direct reaps before completion.
  For each signal, bypassing that same fence must violate the same ordering
  predicate with live work at early completion; restoring it must pass.
  Runner controls also cover panic and server death. A marker before await,
  owner liveness or post-exit eventual death cannot establish this criterion.
  Shell-helper readiness is published completely after trap installation.
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

- **AC9:** Ordinary dev/release executables ignore former test hold and
  failure-injection controls. Internal controls stay cfg(test)-only; ordinary
  PID-only CLI proof remains separate. Guards retain child handles and owned
  identities before fallible assertions, use a 30-second post-release normal
  wait deadline, and bound failure teardown separately while allowing the
  full cleanup window plus margin. Readiness failure, assertion unwind,
  stalled completion, bypassed owner and leader-first exit are exercised.
  No unbounded wait or signalling of unrelated/recycled identities; forced
  teardown and unresolved survivors explicitly fail the test.
- **AC10:** Fresh evidence records candidate revision and engine digest.
  The final candidate passes independent evals checks, every canonical house
  gate and the selected four-lens review with the protected security hold.
  No stale receipt, journal rewrite, push, merge or issue close is used.

## Recovery evidence and delivery boundary

2026-10-24, earlier recovery chief (1843c55): recorded the exact-1a2f36c
issue regression as 1 passed, 14 filtered, under `.forge/design-194-recovery/`.
That historical receipt does not prove this successor. The earlier findings
of ordinary-build controls and a three-second guard describe that revision,
not the current cfg(test) migration. The original missing 0062 index row is
also historical; the current index already contains it.

2026-10-24, continuation chief: read the actual prior VERIFY-FAIL at
`.forge/results/a5167df6-a43b-4830-967d-dea3e16c0433.json`, both current panel
positions, all three intake framings and current source. Independently hashed
`.forge/bin/brokkr` against tooling.json. Confirmed matrix actual-wait control
and cfg(test) repair in source; confirmed helper trap ordering, unbounded
MatrixProbe ordinary wait, runner eventual-death proof and leader-dependent
fallback concerns. These are source findings, not executed test failures.
The implementer receipt for 1315846 is not independent verification. No
missing journal or later verdict is reconstructed.

The chief commits exactly this directory's spec.md, plan.md and tasks.md.
Already committed, clean 0062 stays declared and proposed; this recovery
changes no runtime/public contract and needs no new numbered decision.
No production change occurs in this phase. Local checks are documented in
run-local evidence; only the sequence's pinned deterministic check owns the
design verdict. Implementation, independent full verification and protected
review remain outstanding in [tasks.md](tasks.md); drafted is not delivery
success. The design commit must remain HEAD.
