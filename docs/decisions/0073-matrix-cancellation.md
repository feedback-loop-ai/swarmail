# 0073 — Matrix cancellation waits for owned cleanup

- **Status:** accepted (operator, 2026-10-26)
- **Date:** 2026-10-24.
- **Issue:** Epic #192, #194.
- **Specification:** [matrix cancellation](../../specs/matrix-cancellation/spec.md).
- **Extends:** [0055 §§8–10](0055-agent-eval-harness.md), compatibly: explicit matrix interruption, completion and accounting semantics.
- **Preserves:** 0001–0005, schema 1, single-trial cleanup, report formulas, root coverage and dependencies.
- **Number:** proposed as 0062; renumbered to 0073 at integration, because 0062 and 0072 went to other epic #192 decisions first. Historical gaps are not reused.

## Context

The matrix hides its one active trial in `Command::output()` and handles no
signals. Trials start servers and agents in separate process groups; killing
a parent is not cleanup. Existing trial interruption aborts the body, sends
kill requests and returns without proving all owned descendants stopped.
The single-trial regression observes eventual death and cannot prove matrix
cancellation or cleanup-before-exit ordering.

## Proposed ruling

1. After valid inputs, subscribe to SIGTERM/SIGINT before any subprocess,
   including fallback Cargo. First observed signal latches cancellation;
   no later slot starts. A concurrently admitted child remains owned and
   is cancelled and awaited. Repeated signals cannot shorten or reset
   cleanup. Signal setup failure refuses the complete valid product.
2. Either matrix signal forwards SIGTERM once to its active trial PID.
   Matrix passes its resolved binary path, so trial signal handlers precede
   owned process creation; termination before handlers starts no children.
   Retain explicit child/read/wait ownership and reap the trial. No startup
   channel is required under that invariant. Fallback Cargo is independently
   owned/cancelled/reaped by matrix, using the same group cleanup policy.
3. Trial reapers/group registrations outlive aborted workload admission.
   Close admission and join the body, TERM remaining groups concurrently
   for one five-second grace, KILL survivors, then allow five seconds for
   cessation/reaping. Reap direct children and observe no running owned
   group members before successful cleanup completion. Leader exit, EOF
   and kill success are insufficient. Normal completion uses the same
   final fence so a cancellation race cannot inherit live descendants.
   Matrix has no shorter competing kill timer for its cleanup owner.
4. Linux observation distinguishes registered group membership, identity,
   disappearance, live state and zombies. Zombies are non-running; direct
   children still owe a wait. Other Unix uses conservative group absence
   (ESRCH), never `/proc` absence as proof. Adopted zombies may cause a
   conservative timeout there. Unknown observations and cleanup expiry
   are diagnosed harness failures, never successful cleanup claims.
5. Both handled matrix signals exit 2. Preserve full task → agent →
   repetition JSONL cardinality, completed prefix and schema 1. Pending
   cancellation wins simultaneous commitment. Active interrupted work
   produces one harness-error row after cleanup, with honest observed
   statuses and elapsed lifetime; absent usable output uses a fresh refusal
   ID and unknown statuses. Unstarted slots use fresh IDs, zero-time
   `refused_row` values and launch nothing. A signal observed after the
   final committed row still changes matrix exit to 2, not that row.
6. Accept exactly one matching, valid child row, bound retained output to
   64 KiB while draining and independently await the child; malformed,
   extra, oversized or incompatible output is a harness error. Cancellation
   and cleanup remain runnable under output pressure. Output failure stops
   admission and cleans active work. Full JSONL requires available,
   progressing stdout; forced death/abort/host failure cannot promise it.
7. Report attempts count requested result slots, including infrastructure
   refusals with no executed agent. Preserve success over pass+fail and
   passes/attempts. No new outcome, summary row, retry or partial-prefix
   exception. Cleanup bounds exclude scheduling/output latency; unkillable
   kernel tasks and escaped/hostile commands cannot satisfy unconditional
   bounded cessation. A bound failure leaves that guarantee unproved and
   must say so; this is lifecycle ownership, not confinement.

## Adopted and rejected positions

Adopt simplicity's sequential matrix, direct signal forwarding, existing
schema and one cleanup owner. Reject robustness's mandatory private startup
channel because the pinned-binary trial path registers handlers before any
owned spawn. The cost is a no-row early-death fallback and a mandatory
regression guarding that ordering; future pre-handler spawning invalidates
this choice.

Adopt robustness's retained reapers, bootstrap ownership, shared cleanup
budget and controlled ordering proof. Reject a matrix-only signal patch,
kill-and-return and eventual-death proof. The cost is a narrow runner
ownership change and additional real-process tests. No general supervisor,
parent-side second descendant registry, cgroup, subreaper, dependency or
root server edit follows from this issue.

Both positions retain full accounting. Reject partial JSONL or a new skipped
outcome: either would require consumer changes and potentially an accepted
amendment to 0055. The cost of preserving it is O(product) refusal output
and attempts that cannot be read as executed-agent counts.

## Proof and delivery

For each signal, a real matrix PID-only test establishes live readiness,
a passing prefix, active descendants and append-only suffix launch evidence.
An acknowledged cleanup hold proves the matrix waits while owned work is
live; release and observed cessation precede completion. A negative control
removing that await must fail. Linux identity-aware exit observations,
TERM-resistant descendants, startup/completion races, repeat signals,
bootstrap and output failures supplement that proof. Standalone cleanup
and oracle/null/isolation tests remain separate obligations.

The chief's authorized four-file commit excludes the decision index. The
first implementation task adds this proposed record to its index and proves
the inventory/status checks. Until then the full gate inventory is not
clean; the structural design check is not a verification verdict. Only the
operator can accept this proposal. Compatible work may proceed under the
house rules; no accepted ruling is amended in place.
