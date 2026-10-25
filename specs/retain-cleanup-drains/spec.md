# Retain failed-cleanup drains

Date: 2026-10-25. Epic #192 / issue #199, AC3 follow-up to reviewed port
`d973b0eed2605524f3b44eb21f6388593e3e1982`.

## Feature and rationale

A failed bounded cleanup must keep every outstanding output task owned until
actual completion on a surviving runtime. Admission and private runtime state
must outlive those tasks. A cleanup error reports an unproved cleanup; it does
not authorize another trial or disposal of state still in use.

`OwnedGroup::cleanup_once` takes the whole drain vector and currently drops the
timed-out handle and unvisited suffix when it returns an error. Tokio detaches
those tasks. `ensure_cleanup` then finds no handles to retain. Unlike ordinary
`drain`, captured-helper and Cargo stdout tasks do not independently capture
admission/private state, so they expose the ownership loss after child reaping,
group cessation and stderr completion. This is the supplied review's MEDIUM;
the regression must establish the failure causally before claiming it fixed.

## Authority and scope

The checksum-verified issue199 README, operator-rulings and review packet govern
this follow-up. Intake is committed at
[the current framing](../../.forge/tasks/epic-192-issue-199-meet-ac3-befo-882d93dc.md);
`.brokkr/state/tasks/` is absent. Both run-local positions were read in
`.brokkr/state/design/positions/{simplicity,robustness}.md`. All eight packet
checksums and the candidate-local engine digest match. Packet and panel paths
are run-local evidence, not new tracked dependencies or external proof claims.

This restores the existing [0055 lifecycle and sequential addendum](../../docs/decisions/0055-agent-eval-harness.md)
and [sequential-trials R3/AC3](../eval-sequential-trials/spec.md).
[0076](../../docs/decisions/0076-sequential-eval-trials.md) remains proposed and
unchanged. There is no new public contract, semantic exception or amendment to
a ruling, so no new decision is drafted or declared. The operator already chose
sequential refusal and fresh HOME; namespace isolation remains #221.

Implementation is limited to runner cleanup ownership and necessary causal tests.
Keep frozen 0001–0005, scoring, provenance, privacy, result versions/concurrency,
matrix accounting, dependencies, allowlist, root source, gates and routing intact.
The independent review's packet-link LOW and main-probe coordination LOW are
outside this path. Other admission/error-arm/barrier INFO findings remain recorded
in the supplied review, not silently marked fixed.

## Panel synthesis and rulings

Adopt simplicity's local restoration of the pending handle and iterator suffix
into the existing owner. Adopt robustness's explicit proof of retained ownership
through both specialized stdout paths, the unvisited suffix, and failed-cleanup
admission on a surviving runtime. The positions agree on production design;
there is no genuine conflict to invent or average.

Rule against per-task lifetime captures as the sole repair: they leave the
owner's claimed handle-retention path empty and require every future output
branch to remember the policy. Cost of rejecting that alternative: tests must
prove the common transfer, and its mutex/iterator logic gains a small branch.
Reject a new supervisor, registry, wrapper hierarchy, dependency, abort policy,
retry, timeout extension or comment-only correction. These either enlarge the
scope or fail the existing through-drains requirement.

Reconcile minimal production code with substantial causal proof: choose both,
not a weaker test to make the patch smaller. Explicitly adopt the robustness
requirement to release the first retained handle while a later one stays pending;
a single-handle test cannot prove suffix retention. Reject the review's suggested
post-probe-exit one-line admission assertion: runtime teardown is already an
excluded case. Cost: a surviving-runtime return/completion handshake is needed.
Robustness's storage/TLS/webhook scenarios are preservation checks covered by
existing house gates, not new feature work or acceptance criteria.

Accept that a permanently pending task can retain admission and private state
indefinitely on a surviving runtime. The cached error still arrives within the
existing cleanup budget; recovery does not turn it into success. Runtime teardown,
SIGKILL/abort and host failure retain their documented limits and require operator
cleanup before reuse. This does not provide same-uid confinement or safe resume.

## Acceptance Criteria

1. **AC1 — Complete ownership transfer.** On the first bounded drain timeout,
   retain that handle and every later unjoined handle, in order, before returning
   the existing error. Joined handles are not polled again. Keep the one cleanup
   owner, existing absolute deadlines and JoinError treatment; never hold a
   synchronous mutex across await. No abort/detach-to-release shortcut is allowed.
2. **AC2 — Real helper and Cargo red/green proof.** Separate cases traverse actual
   captured-helper and Cargo stdout tasks with real child pipes and acknowledged
   pending work through the real bounded-drain timeout. Before asserting lifetime,
   establish successful direct-child reaping, empty owned group, finished ordinary
   stderr, published cleanup error and failure-owner progress. Drop every caller,
   Proc/ProcessWait/Groups and external admission/private-state strong reference.
   On the still-live runtime, pending stdout must cause exactly
   `concurrent_trials_refused` and keep the private directory present. Release it,
   acknowledge task completion and owner disposal, then prove admission succeeds
   and the directory is removed. Tests compile against unchanged production logic
   at d973b0e and fail at the admission/lifetime assertion, then pass with the fix.
   Setup timeouts, compilation failures, zero-match filters and sandbox errors
   are not red evidence. Record exact test names, commands and failure assertions.
3. **AC3 — Unvisited suffix.** With at least two pending unjoined tasks, force the
   first wait to expire, then finish the first while the later task stays pending.
   Admission and private state remain retained until the later task completes.
   No ordinary drain or test-held strong reference may mask the missing owner.
   A focused test-only handle seam may supplement, never replace, AC2's real paths.
4. **AC4 — Failed-cleanup LOW closure.** Extend
   `cleanup_observation_error_and_forced_expiry_fail_with_live_owned_processes`
   or its existing probe family for both observation failure and forced expiry.
   After a failed verdict, acknowledge live owned work on a surviving runtime,
   remove unrelated owners, and assert exact admission refusal and private-state
   retention. Identity-aware teardown and actual completion permit admission.
   A temporary early-release negative control must fail the refusal assertion in
   both modes; remove it before final gates. Preserve existing Pending/entry and
   checked survivor/teardown controls.
5. **AC5 — Reporting and lifetime compatibility.** The same cached cleanup error
   is observable after retained work finishes, without renewed cleanup budgets.
   Preserve fixed TERM/KILL behavior, direct waits, cancellation/dropped waiter,
   finalization fence, bounded output and finite error categories. Rerun
   `unproved_worker_cleanup_stops_matrix_with_full_refusal_suffix` to close the
   same-path INFO by proving unchanged harness-error/stop-admission/suffix behavior.
   Existing sequential, cleanup, trap, privacy and provenance controls stay green.
6. **AC6 — Candidate evidence.** Commit implementation and regressions unsigned;
   record candidate HEAD/tree and fresh local house gates, exact 100/100/100 root
   coverage and selftest, plus standalone eval fmt/clippy/full locked tests with
   default threading. Use the pinned `.forge/bin/brokkr`, unchanged recipe routing
   and verified pinned scripts. Older receipts do not prove this candidate. A
   failure on an unrelated known finding is reported with its scope conflict,
   never hidden by weakening a gate or widening the repair.

These criteria govern implementation completion. This design commit proves only
the specification's structural validity; no new regression is claimed executed.
CI owns charter platform/network/privileged checks and #201 macOS eval evidence.
The operator owns live GitHub settings, hard-death recovery, sealing and decision
acceptance. Those are ownership records, not local acceptance conditions.
