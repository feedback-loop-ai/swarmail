# Stop active eval trials on matrix interruption

Date: 2026-10-24. Epic #192, issue #194. Design only; implementation and
verification remain the unchecked work in [tasks.md](tasks.md).

Decision: 0062 — [Matrix cancellation and cleanup completion](../../docs/decisions/0062-matrix-cancellation.md), proposed; acceptance is the operator's.

## Feature and rationale

A SIGTERM or SIGINT sent to the eval matrix PID must stop further trial
admission, cancel its active trial, and await that trial's owned process
cleanup before the matrix exits. Today `matrix` awaits `Command::output()`
without a signal path; the trial owns separate agent/server process groups.
Killing only the matrix or trial leader does not establish cleanup.

Triage classified this as `engine`. Retain that classification: process
ownership, race ordering and observable result contracts require the full
route even though the matrix remains sequential. Intake and both panel
positions under `.brokkr/state/` are evidence, not committed authority.

[0055 §§6–10](../../docs/decisions/0055-agent-eval-harness.md) governs the
Unix lifecycle, schema 1, complete product accounting and report formulas.
This design clarifies matrix interruption under those rules and strengthens
the cleanup completion boundary without weakening any accepted ruling.
[0001](../../docs/decisions/0001-synchronous-insert-lossless.md),
[0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md),
[0003](../../docs/decisions/0003-oldest-only-pruning.md) and
[0005](../../docs/decisions/0005-extraction-at-ingest.md) are unchanged.

## Panel rulings and their costs

- Adopt both positions' sticky cancellation, one active trial, exit **2**,
  schema-1 refusal suffix and immutable completed prefix. Reject partial
  JSONL, exit 130/143, a new outcome or summary row. Cost: cancellation
  still writes O(requested product) rows; attempts include unexecuted slots.
- Adopt simplicity's direct SIGTERM forwarding for either matrix signal;
  reject robustness's mandatory private cancellation channel/handshake.
  Matrix resolves the server binary first and passes it to every trial.
  With that path, trial handler registration precedes owned child creation:
  pre-handler termination leaves no grandchildren; post-registration
  cancellation enters cleanup. Cost: an early trial can produce no row,
  requiring a truthful fallback. This choice requires a regression for
  that ordering and becomes invalid if trial startup later spawns earlier.
- Adopt robustness's durable child/reaper ownership and observed cessation;
  reject a matrix-only patch and reliance on merely awaiting today's trial.
  The current abort/kill-all/return path does not prove descendants stopped.
  Cost: a narrow `runner.rs` ownership repair is required, while standalone
  cleanup and scoring remain intact.
- Adopt robustness's cancellation of the matrix's fallback Cargo build.
  Signal installation only around `run_child` leaves an owned subprocess
  outside the contract. Cost: an async matrix resolver with shared parsing;
  no change to the standalone resolver's public behavior.
- Adopt simplicity's single existing cleanup owner and no general
  supervisor framework, parent group inventory, cgroups, subreaper,
  recursive descendant discovery, dependencies or new production module.
  Reject a competing matrix timer that kills the trial first. Cost: forced
  matrix death and escaped/hostile processes remain outside the guarantee.
- Adopt robustness's controlled cleanup hold in addition to immediate
  post-exit observations. Reject an eventual PID disappearance test as
  sufficient evidence. Cost: small test-only lifecycle seams and explicit
  platform observation rules; the existing single-trial test remains
  separate evidence.

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
cancellation, and committed that row to ordered output. Committed prefix
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

## Delivery boundary

This chief commits exactly spec.md, plan.md, tasks.md and proposed 0062.
The required `docs/decisions/README.md` proposed index row is an explicit
first implementation task: the exact-file design constraint excludes it
from this commit. Until it lands, the existing decision-inventory test is
expected to refuse; this is not a waived gate or an implementation-complete
claim. The sequence's pinned check alone owns the design verdict. No push.
