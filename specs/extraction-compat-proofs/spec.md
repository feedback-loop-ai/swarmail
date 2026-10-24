# Feature specification: extraction and compatibility proof closure

Created: 2026-10-24
Status: Draft design; implementation and delivery evidence outstanding
Backlog: epic #192, issue #190 — the five bounded proof residuals
Run: epic-192-issue-190-close-the-ext-92295151

## Feature and rationale

Close the remaining proof obligations for the pinned entity table, complete
response offload, source-only containing search, ordered shim deletion and
empty-inbox removal. Users must be able to trust that the tests reject the
specific defects those contracts forbid. A green test name or historical
receipt is insufficient: serialization can stay offloaded while rendering
returns to the async worker, a table can have the right size and wrong
values, and a deletion test can pass without ever committing a prefix.

This is one Rust repository and a proof-only change. Existing tests already
cover the requested surfaces. Execute and falsify them first; strengthen only
a demonstrated proof gap. An unchanged production tree, and even unchanged
tests with fresh discriminating evidence, is a valid implementation outcome.
The request to strengthen the held test is satisfied only when that test
itself rejects the serialization-only defect, whether its existing assertions
suffice or need a local repair.

## Authority and preserved requirements

Accepted [0005](../../docs/decisions/0005-extraction-at-ingest.md),
[0040](../../docs/decisions/0040-extraction-fidelity.md) and
[0049](../../docs/decisions/0049-extraction-compat.md) govern extraction,
snapshot provenance, verbatim historical arrays and complete response work.
Accepted [0023](../../docs/decisions/0023-off-worker-commits.md) governs
awaited offload and panic symmetry; [0044](../../docs/decisions/0044-compat-validation.md)
governs compatibility queries. Preserve their accepted behavior, including
0049's dated compatible clarification.

Preserve synchronous insert and immediate DATA 250 visibility
([0001](../../docs/decisions/0001-synchronous-insert-lossless.md)),
subscribe-before-check ([0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md))
and oldest-only pruning ([0003](../../docs/decisions/0003-oldest-only-pruning.md)).
Explicit shim selectors ([0011](../../docs/decisions/0011-shim-delete-contract.md)),
ordered mutation/refusal behavior and the existing per-id shim loop remain.
[0015](../../docs/decisions/0015-boundary-failure-contract.md) supplies the
selective real SQLite failure technique and POP3 prefix authority; it is not
misrepresented as a new shim ruling. Empty-entry semantics remain those of
[0032](../../docs/decisions/0032-emptied-inboxes-are-removed.md).

No semantic amendment is proposed, so this design declares no new decision.
[0061](../../docs/decisions/0061-compat-webhook-proof.md) remains proposed:
its all-quote refusal normalization and independent link-order and
conditional-comment questions are outside this task. Preserve current
ordered prefix recognition, text-first links and ordinary comments.
Completing these proofs is not acceptance of 0061 or closure of all #190
or epic #192 work.

## Panel synthesis and rulings

The two current positions agree on the architecture and acknowledge that
the existing tests may suffice. There is no invented architectural conflict.
The following choices resolve their different emphases explicitly.

| Question | Ruling and position adopted | Alternative rejected and cost |
|---|---|---|
| Must implementation manufacture a diff? | Adopt simplicity's existing-owner approach and robustness's evidence-first condition. Keep already sufficient tests and comments. | Reject mandatory new test files or cosmetic edits. The accepted cost is a delivery whose value may be fresh evidence rather than additional code. |
| How much negative proof is necessary? | Adopt robustness's final-candidate, discriminating controls. Require full mapping, serialization-only and search-candidate mutants; the held-route evaluator must fail independently of its companion test. | Reject stopping at unchanged green tests. Simplicity also requests mutations; its optional same-cardinality experiment becomes mandatory here. Extra executions and recorded restoration are the accepted cost. |
| What observation machinery is justified? | Adopt simplicity's existing harness and cfg(test)-only invocation observations, retaining robustness's independent callback witness. Add only a demonstrated local proof repair. | Reject a work-site registry, second gate framework, permanent mutation runner or new dependency. Less generic reuse and bounded defect coverage are accepted. |
| How is the shim failure injected? | Adopt both positions' real selective SQLite fault on the middle of three ordered ids, with live/reopened/retry observations. | Reject an empty-prefix fixture, mock mutation seam or atomic batch transaction. Partial success remains visible and clients must retry idempotently. |
| Must the deadlock regression be renamed? | Adopt simplicity's existing explanatory comment; adopt robustness's precise account of recreation before reopen and process-level watchdog. | Reject cosmetic renaming as a requirement and an async timeout as a deadlock watchdog. The long existing name remains, and this test alone does not prove absence after restart without recreation. |
| Must broader robustness risks be repaired? | Preserve the frozen guarantees and existing error/resource policies while proving this bounded surface. | Reject outbox, admission, parser, migration, TLS, shutdown and capacity redesign. Existing blocking-pool contention, historical wrong arrays, link amplification and best-effort webhook residuals remain. |

## Acceptance Criteria

1. **AC1 — Full generator reproduction.** On the final candidate, execute
   the unchanged `python3 -I scripts/gen-entities.py scripts/data/html-entities.json`.
   Require exit 0 and byte-for-byte equality of all stdout with the complete
   `static NAMED` declaration, including its terminal newline. Reject empty,
   partial or normalized comparisons. Record source hashes, command, exit,
   byte count and equality. Preserve snapshot SHA-256
   `d741d877ac77c4194c4ad526b5b4a19aef8dfe411ab840a466891cdbb9f362e6`,
   provenance and attribution.

2. **AC2 — Independent mapping and refusal proofs.** Run
   `all_whatwg_entities_match_snapshot`, covering every one of the 2,125
   mappings and 106 legacy aliases. A temporary valid-scalar substitution
   that keeps table cardinality unchanged must compile and fail this test
   for mapping inequality; restore and rerun green. Feed a byte-modified
   snapshot to the same unmodified generator through stdin or private scratch:
   require nonzero exit, SHA refusal and zero stdout. Never change the real
   snapshot or its pin to make the control pass. Python remains offline
   maintenance/delivery tooling, not a Cargo build or test dependency.

3. **AC3 — Held real responses.** Run
   `held_single_message_builds_leave_smtp_and_health_serving` and
   `actual_lookup_and_build_leave_the_async_worker` on their current-thread
   runtimes. All eleven native/compat full, latest, plain, raw, header and
   download routes reach the hold before release. While held, SMTP greeting,
   health, an independent DATA transaction's 250 and immediate count readback,
   and the existing missing-id/latest 404 cases succeed without gate expiry.
   After explicit release, every route returns its expected body/content type,
   with eleven lookup and eleven completed-build observations. Callback
   observations independently establish actual lookup and projection placement.

4. **AC4 — Serialization-only discrimination.** Construct and inspect a
   successfully compiling temporary mutant that moves actual lookup and
   projection/render/copy work onto the async worker while leaving serialization
   and the old hold offloaded. Keep actual-site observations with actual work
   and preserve the independent callback-internal witness. Run the held-route
   test separately: it must fail from an intended placement assertion or
   hold/progress failure, not solely from a companion test. A compile failure,
   missing observation caused only by removing a seam, disconnected callback,
   or unrelated timeout is not proof. Run the companion independently too.
   Restore exact source bytes and rerun both green. If the held test remains
   green, repair its observation in the existing harness and repeat until
   this control is discriminating. Preserve full awaited responses, guard
   release, panic/error behavior and actual production offload.

5. **AC5 — Non-candidate search miss on both wires.** Execute
   `extracted_only_fields_miss_both_wire_searches` against real HTTP.
   Historical-style stored links/codes contain needles absent from every
   source candidate, including raw mail; native readback retains those arrays.
   Each array-only link/code needle returns 200/count 0 from both Mailpit and
   MailHog containing searches; subject/body controls return 200/count 1.
   A temporary addition of links/codes to search candidates must compile and
   make the regression fail; restoration must pass. No re-extraction,
   historical write-back, search expansion or validation-precedence change.

6. **AC6 — Durable nonempty shim prefix.** Execute
   `shim_delete_preserves_committed_prefix_across_reopen_and_retry` using
   ordered A/B/C ids and unrelated mail. A real SQLite trigger refuses only B.
   DELETE returns 503 with `storage commit failed`; A is absent and B, C and
   unrelated mail remain, both live and after reopen. Remove the fault and
   retry the same ids: exact success bodies are `{"removed":2}` then
   `{"removed":0}`; unrelated mail survives. These assertions must distinguish
   swallowed failure, continuing past B, rolling back A and an empty-prefix
   fixture. No transaction replacing the per-id loop, mock server or new
   production mutation seam. Graceful reopen is not hard-kill durability proof.

7. **AC7 — Named hang condition.** Execute
   `a_delete_that_empties_an_inbox_removes_the_entry_live_and_after_reopen`
   under an outer process deadline able to kill a synchronously wedged test.
   Its existing comment, or a clearer equivalent name/comment, must state that
   retaining the map guard while removing the emptied entry deadlocks on the
   same shard. Preserve watcher closure, listing absence, recreation and
   reopen assertions. State accurately that this fixture recreates the inbox
   before reopening; do not claim it proves absent-after-restart without
   recreation. Preserve the production guard-drop/removal ordering.

8. **AC8 — Honest final-candidate evidence and delivery.** For each targeted
   proof retain candidate/source identities, exact commands, executed test
   counts, statuses, relevant diagnostics and red/restored-green logs. Zero
   selected tests is failure. Restore all mutations before final validation;
   never commit a mutant. Any proof repair stays in the explicitly owned
   files in plan.md, with no new dependency, feature flag, coverage exclusion,
   gate edit or accepted-decision edit. Complete the authoritative
   [house-rule gates](../../docs/house-rules.md#gates--part-of-every-change),
   exact coverage/selftest, pinned proofs and fail-closed native-tls absence
   proof on the final candidate. Retain CI-only obligations separately.
   Engine policy requires all four independent reviews for the engine lane:
   Qwen Max correctness, Opus security, official DeepSeek Flash guarantees and
   DashScope GLM 5.3 tests; never Pro or fallback. No security hold is waived.

## Completion boundary

The chief delivers only this spec, plan and tasks as its final unsigned
design commit. The deterministic pinned check owns the design verdict.
Implementation, mutations, full gates and policy review remain downstream
obligations until actual receipts exist. Preserve historical evidence and
open operator questions. Never push, merge, tag or publish.
