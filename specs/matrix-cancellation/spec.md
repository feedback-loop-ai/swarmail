# Close the matrix cancellation review residual

Date: 2026-10-24. Epic #192, issue #194. Residual-repair run:
`triage-and-continue-epic-192-iss-d9c1d0eb`, following completed
`continue-epic-192-issue-194-from-8d385d4d`.

Decision: 0073 — [Matrix cancellation and cleanup completion](../../docs/decisions/0073-matrix-cancellation.md), already committed, clean and proposed; acceptance is the operator's.

## Feature and rationale

Close the recorded cancellation proof residual and the returned selector-test
blocker without changing the implemented SIGTERM/SIGINT contract. Inspected
baseline is `20c20b0d2a37e87d1e7ebc0a9281573155888a74`, including residual
implementation `e95f3d9`, design `18f5acc`, merged main and accepted 0058.
The returned framing and both current positions supersede the earlier
29a6615-baseline work list. T1–T8 record predecessor implementation evidence;
their controls must be reproduced, not implemented again. T9/T10 remain open.

The returned blocker is in `tests/delivery_recipes.rs`: its shared fast
selector test still expects Sol/high, while all four selectors choose
`codex-fast-implementer`, whose singleton model is Luna/xhigh. Accepted
[0057's dated model clarification](../../docs/decisions/0057-codex-delivery-recipes.md)
and the dedicated Luna/no-fallback test settle that contract. Correct the
existing expectation and bind each selector to that exact office. Do not
change the recipe to satisfy the stale test or derive expectations from the
configuration under test. Existing one/two/five-reviewer checks remain.

The oversized return `.forge/results/ddf345e5-9ab0-41eb-8524-e7e3303ed0f7.json`
records root test and coverage failure at that assertion. This is a test
oracle repair under accepted policy, not a new policy decision. Retain all
existing residual controls and require current-candidate verification.
Further runtime edits require a concrete recorded residual-specific failure;
no cancellation redesign or new public contract is requested. Fresh chief
checks also reproduce an inconsistent teardown-control failure (AC12), which
belongs to the existing AC9 proof scope and must not be hidden by a retry.

The intake names aggregate review sequence 714. Read all four retained
`.forge/results/3e202f8a-5be6-48a3-89f1-1e58415614a7-{correctness,tests,security,guarantees}.json`
results and the predecessor ledger/ship receipt before synthesis. Correctness
and tests report residual, maximum medium, no security residual, no applied
fixes and no spec defect; security and guarantees report clean. The ledger
records residual without notes; ship binds its close-out to `137e7e9`.
The boxed `.forge/forge.db` is empty and `.brokkr/state/forge.db` unavailable;
no aggregate result is retained. Exact sequence-to-member binding remains
unverified. These member findings define this bounded repair; no missing
journal event or current-candidate gate receipt is invented.

Accepted [0055](../../docs/decisions/0055-agent-eval-harness.md) remains the
lifecycle/schema/report authority. Existing proposed 0073 describes the
compatible interruption contract retained here; this proof repair adds no
semantic ruling and needs no new decision. Accepted
[0045 §8](../../docs/decisions/0045-delivery-repairs.md) permits compatible
implementation while proposed, never contradiction of accepted rulings.
Frozen [0001](../../docs/decisions/0001-synchronous-insert-lossless.md),
[0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md),
[0003](../../docs/decisions/0003-oldest-only-pruning.md), AUTH and
[0005](../../docs/decisions/0005-extraction-at-ingest.md) remain untouched.

Triage remains `engine`: concurrency, process identity and bounded failure
proofs require accepted [0058](../../docs/decisions/0058-codex-review-routing.md)'s
five separate high-risk results: Astra correctness, official DeepSeek Flash
security, Sonnet 5.5 tests, Astra guarantees and independent Fable 5.1.
DeepSeek Pro and unauthorized fallback are forbidden. Protected review,
worst verdict and hard security hold remain. The engine selects transitions;
[0050](../../docs/decisions/0050-delivery-gates.md) binds design to the chief's
HEAD and verification to executed evidence on the actual candidate.

## Panel rulings and their costs

Read both returned run-local positions, `simplicity.md` and `robustness.md`,
and the updated intake framing. They now agree on the immediate repair;
do not invent a disagreement or carry withdrawn advice forward as current.

- Adopt simplicity's in-place selector correction and robustness's exact
  selected-office binding. All four cases must name `codex-fast-implementer`,
  resolve to exactly `["luna"]` and use `efforts.luna == "xhigh"`. Retain the
  adapter/provider and no-fallback checks. Reject a generic routing validator,
  new fixture family, weakened "some Codex model" assertion, or Sol rollback.
  Cost: local explicit assertions must be maintained when the operator next
  changes the contract; a policy rollback would contradict accepted 0057.
- Adopt simplicity's restriction against reimplementing completed T1–T8 and
  robustness's requirement to reproduce their causal lifecycle proofs. A
  smaller source patch does not reduce the delivery argument to a model test.
  Reject treating prior checked tasks or predecessor clean security review as
  current proof. Cost: real-process suites and five-role review latency remain.
- Reconcile the prior Full-channel disagreement explicitly: simplicity has
  withdrawn optional deletion for this return. Preserve the current writer.
  Robustness's distinction between real stdout pressure and synthetic channel
  fullness governs claims. Cost: retaining possibly redundant machinery;
  deletion without a demonstrated residual defect would expand scope.
- Adopt robustness's limits on forced expiry: the current seam returns an
  error before the ordinary deadline loop. It proves failure propagation with
  real owned work, not the real clock branch or an unkillable kernel task.
  Require causal owner/event evidence for no deadline renewal; logged budget
  constants alone are insufficient. Cost: honest proof limits remain visible
  to reviewers rather than being concealed by a green test name.
- Adopt both positions' sequential matrix, retained waits/reapers and single
  cleanup owner per group. Reject a startup handshake, second registry,
  supervisor framework, cgroup/subreaper or root-server hardening absent a
  concrete residual failure. Cost: conservative unknown/expiry failures and
  explicit unsupported escape/kernel limits; ownership is not confinement.
- Adopt the existing spec directory and proposed 0073. No new semantic ADR,
  schema, outcome, report denominator, dependency or coverage exception is
  warranted. Preserve O(product) refusal output and attempts including
  unstarted slots. Broader failure analysis supplies preservation constraints.
- Rule for the accepted 0058 high-risk panel despite the tiny immediate patch:
  delivery-gate assertions and the enclosing concurrency/cleanup residual
  exclude a cheaper route. Cost: five separate results; the rejected shortcut
  would leave the current lifecycle and delivery evidence independently unreviewed.

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

- **AC1 — Ordinary signal regression:** retain separate PID-only SIGTERM and
  SIGINT cases using the ordinary CLI, real Swarmail/fixture/agent and live
  descendants after a passing prefix. Assert exit 2, exact ready identity
  count, no suffix launch and non-running supported identities at exit.
- **AC2 — Completion fences:** preserve both actual awaited-future Pending
  acknowledgements and matching bypass falsifiers: matrix child wait for
  both signals; runner cleanup wait for both signals, panic and server death.
  Hold live work, observe Pending, release, prove direct reaps and cessation
  before return. Bypass must violate that same predicate before test rescue.
  Entry markers or eventual post-exit death cannot replace these proofs.
- **AC3 — Accounting:** both ordinary signal cases preserve prefix bytes
  exactly, ordered cardinality, unique IDs, one truthful active error and
  zero-time refusal suffix. Real report ingestion retains attempts including
  refusals, passes/(pass+fail), passes/attempts and error counts. Keep queued
  row immutability, final-row exit 2, normal 2/1/0 aggregate precedence and
  ordinary-error continuation. Keep the already removed tautological signal
  assertion absent.
- **AC4 — Admission and repeated signals:** deterministic controls leave a
  signal pending at the real `pending_matrix_signal` check between rows and
  before enqueue; a helper consuming it and pre-latching interruption does
  not qualify. Cover before-spawn cancellation with no issuance and an
  already-issued concurrent spawn retained/cancelled/awaited, with no suffix
  launch. Retain pre-admission, pre-handler, simultaneous-completion and
  final-row controls. Assert early no-row fallback has a fresh unique ID,
  unknown raw statuses and actual attempted-child elapsed lifetime bracketed
  by acknowledged events, distinct from unstarted zero-time rows. With live
  owned work and actual cleanup Pending, acknowledge TERM then INT then TERM,
  including repeats after budget creation: one forwarded TERM, one cleanup
  instance per group, unchanged deadlines, concurrent group cleanup, exit 2,
  direct reaps and no supported running identities at return.
- **AC5 — Cleanup failures:** retain resistant-descendant and leader-first
  controls. Narrow owning cfg(test) observation-error and forced-expiry
  controls must reach explicit harness failure/diagnostics with real owned
  processes. Unknown membership is never absence; kill success, leader exit
  and EOF are never sufficient cleanup proof. Test direct waits separately.
  Do not manipulate host /proc permissions or manufacture unkillable tasks.
  The existing forced-expiry seam proves error propagation before the ordinary
  timeout loop; it does not prove the real deadline branch or kernel survival.
- **AC6 — Bootstrap/spawn:** retain signal-setup refusal, real fallback Cargo
  cancellation and complete missing-binary/invalid-input behavior. Count
  fallback invocations to prove once-only offline/locked resolution. Using
  the existing test-only executable override with valid Swarmail resolution,
  force actual trial spawn failure; assert honest refusal/status/cardinality
  and normal aggregation without inventing a stop-on-any-spawn-error policy.
- **AC7 — Output bounds:** a real open, never-read pipe reaches actual blocked
  stdout acknowledgement; cancellation remains observable and the absolute
  five-second no-progress bound yields exit 2 and a captured diagnostic.
  Observe deadline identity so later signals cannot renew it. Separately a
  reaped child with a test-owned descriptor holder reaches the five-second
  post-exit EOF bound and harness error; independently verify the reap and
  clean the holder. Retain 64-KiB retention, invalid/extra/incompatible output
  controls and EPIPE, capturing its stderr. Full-channel synthetic controls
  do not stand in for these cases; record supported-path reachability before
  claiming that arm is covered. Only unavailable output may prevent complete
  JSONL, never cleanup ownership. Do not enlarge production queueing for tests.
- **AC8 — Preservation and isolation:** all standalone interruption, deadline,
  panic, server-death, normal cleanup, oracle/null/isolation and report tests
  pass. Every added seam has an installing test and ordinary dev/release
  binaries ignore all corresponding controls. Use owning cfg(test) modules,
  fresh temp state, isolated environments and acknowledged readiness; no
  mocks, sleep-as-sync, feature flags, new dependencies or lock changes.
  Root production code, accepted guarantees, report math, schemas and gates
  are unchanged. The root test-only correction in AC11 is explicitly in scope.
- **AC9 — Honest test teardown:** retain handles/identities before fallible
  assertions and use checked finish on positive paths. Forced rescue,
  unreaped direct children or unresolved survivors fail the test. An isolated
  helper intentionally requiring forced cleanup exits unsuccessfully and its
  parent asserts that result, not just stderr. Drop performs bounded emergency
  cleanup without a second unwind panic. Keep readiness failure, assertion
  unwind, stalled completion, owner-bypass and leader-first controls. Retain
  30-second post-release ordinary wait, then 15-second TERM allowance and
  five-second forced observation/reap phase; never an unbounded final wait,
  unrelated/recycled group signal or a rescue timer preempting product cleanup.
- **AC10 — Earned delivery evidence:** map every retained member finding to
  test name, command, revision and exit. Correct the old output-coverage claim
  with a dated superseding entry; retain original receipts. Task boxes close
  only on executed evidence. Run independent locked evals checks and the full
  authoritative house gate inventory including exact coverage/selftest,
  publish/keyless-seal proofs, pinned shim integrity and fail-closed native-tls
  absence. Verify against actual candidate and matching engine digest. Require
  the five separate accepted 0058 review results and protected security hold.
  CI-only duties stay with CI. No journal fabrication, push, merge or issue close.
- **AC11 — Settled fast-selector contract:** the existing shared selector
  regression covers chore/feature/design/engine, each pinned to the exact
  `codex-fast-implementer` office, singleton `["luna"]` and `luna/xhigh`, with
  class-specific diagnostics. Keep exact case sets, absent/unknown-default
  refusal, independent one/two/five-review mappings, dedicated Luna adapter
  resolution to `gpt-6-luna` and no fallback. Applicable assertions reject a
  different office, Sol/high, wrong provider/model/effort or a fallback chain.
  Reproduce the old expectation failure before correction, then pass the
  shared test, dedicated Luna test and complete delivery_recipes suite with
  command/revision/exit evidence. No recipe, adapter or policy edit is needed.
- **AC12 — Teardown proof reproducibility:** investigate and repair the
  current `runner_probe_failure_teardown_handles_owner_and_leader_exit`
  timeout waiting for cleanup-hold entry. It failed in the full evals suite
  and in isolation at 0b522d9, then passed in a subsequent full-suite run.
  Preserve those receipts; a later pass does not discharge the failure.
  Establish the causal order between actual cleanup-future Pending, cleanup
  task/hold entry, bypass return and leader exit using acknowledged events.
  The negative control must still prove owner exit with live owned work and
  checked-finish refusal before identity-safe rescue. Retain both actual-Pending
  fences and their bypass falsifiers. Neither sleeps, longer timeouts, weaker
  assertions, disabled tests nor a second runtime cleanup owner are remedies.
  Require the isolated regression and full locked evals suite to pass after
  the local repair, plus a deterministic control demonstrating the repaired
  ordering. No production defect is inferred solely from this harness timeout.

## Historical evidence and current delivery boundary

The following dated entries describe predecessor revisions, not current gaps
or fresh verification. Their old engine digest and source findings must not
be applied to the current candidate.

2026-10-24, earlier recovery chief (1843c55): recorded the exact-1a2f36c
issue regression as 1 passed, 14 filtered, under `.forge/design-194-recovery/`.
That historical receipt does not prove this successor. The earlier findings
of ordinary-build controls and a three-second guard describe that revision,
not the current cfg(test) migration. The original missing 0073 index row is
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

2026-10-24, residual chief: inspected both positions, current intake,
all four retained member results, predecessor ledger/ship, decisions and the
owning source. The actual-Pending fences, trap-ready publication and bounded
wait repairs now exist; their predecessor absence is historical. Remaining
work is AC4/5/6/7/9 falsifiers and AC3/10 evidence corrections above. Current
local `.forge/bin/brokkr` hashes to
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`, matching
current `bundles/self/tooling.json`; predecessor e525babf receipts are not
fresh evidence. The chief's design-only successor requires its own evidence.

2026-10-24, returned synthesis at 20c20b0: read both revised positions, returned
framing, all four retained member results and oversized ddf345e5 result.
Inspected the stale shared assertion, dedicated Luna regression, implementer
configuration and current residual controls. The implementation map
`.forge/implement-194/t1-map-d9c1d0eb.md` records T1–T8 evidence at e95f3d9;
it supersedes the older output-coverage claim, not current verification.
The current work is AC11/AC12 plus reproduction of AC1–AC10, not adding
those controls anew. Exact seq-714 aggregate binding remains unavailable. No
historical receipt is promoted, bounded return counts are not reset, and
0073 remains proposed. Current chief checks are recorded separately under
`.forge/design-194-selector/`; their results cannot close future repair tasks.

2026-10-24, chief execution at initial draft 0b522d9: the shared-selector test
and root workspace tests exit 101 on actual Luna versus expected Sol; coverage
exits 1 at that same instrumented test. The dedicated Luna regression passes.
All other house gates, shim/native-tls checks, design check/selftest and delivery
boundary proof pass. Evals fmt/clippy pass; the first locked suite and isolated
teardown test exit 101 waiting for runner cleanup hold entry. A subsequent
`cargo test --locked --no-fail-fast` passes. These are inconsistent observations,
not proof that the defect disappeared or a claim of a production failure.
AC12 makes this existing AC9 failure explicit. Receipts are in
`.forge/design-194-selector/{summary,followup}.json` and sibling logs; later
documentation changes do not relabel their executed revision. T9/T10 and
current verification remain open. The final chief commit changes only design
artifacts, and its own structural check is recorded separately.

Commit exactly this directory's spec.md, plan.md and tasks.md. Existing 0073
stays clean, committed, declared and proposed; no new decision is needed.
Only the sequence's pinned deterministic check owns the design verdict.
Run-local checks/results stay uncommitted. Drafted does not mean implemented,
verified or reviewed: outstanding repair is tracked in [tasks.md](tasks.md).
Leave the design commit at HEAD.
