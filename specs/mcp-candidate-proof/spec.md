# Independently verify the integrated MCP candidate

Decision: 0092 — [Dated candidate corrections](../../docs/decisions/0092-mcp-candidate-proof.md).

## Feature and rationale

Assess epic #164 issue #152 candidate
`ab807436aa9bab8ebe21c49f6dea3481bb74eebe`, tree
`371a1db870910b61977e38e8be9ebff3e8f1df47`, and its explicitly identified
successors with fresh verification and five protected review results. The
candidate combines thirteen-tool annotations, accepted 0043's OPTIONS admission,
real HTTP/stdio witnesses, a shared limiter witness and the requested mixed
work-seat route. Source inspection and historical passing receipts cannot
establish that this integrated tree passes or that its review is trustworthy.
The intended runtime delta is zero; a demonstrated defect permits only a bounded,
accepted-contract-compatible repair with a regression in its existing owner.

This design adds a correction record because observable contract claims need
qualification. It does not accept a decision or commission general MCP
conformance. Supported revisions remain 2025-03-26, 2025-06-18 and 2025-11-25.
Historical specs remain byte-for-byte history, including their stale assertions.

## Framing and authority

On 2026-10-28, both run-local simplicity and robustness positions were read.
The requested `.brokkr/state/tasks/` directory is absent. The supplied task and
current intake receipt `.forge/results/a9852698-2906-4acb-82fe-96a8978eaf90.json`
provide the framing; unrelated `.forge/tasks/` files supply no authority. These
paths identify ignored evidence, not tracked dependencies or fabricated history.

Preserve accepted decisions [0001](../../docs/decisions/0001-synchronous-insert-lossless.md),
[0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md),
[0003](../../docs/decisions/0003-oldest-only-pruning.md),
[0004](../../docs/decisions/0004-per-test-inboxes-via-smtp-auth.md),
[0005](../../docs/decisions/0005-extraction-at-ingest.md),
[0043](../../docs/decisions/0043-mcp-origin-boundary.md),
[0045](../../docs/decisions/0045-delivery-repairs.md),
[0057](../../docs/decisions/0057-codex-delivery-recipes.md) and
[0058](../../docs/decisions/0058-codex-review-routing.md), including dated amendments.
Proposed [0079](../../docs/decisions/0079-mcp-version-negotiation.md),
[0080](../../docs/decisions/0080-mcp-source-reconciliation.md),
[0081](../../docs/decisions/0081-glm-flash-seats.md),
[0085](../../docs/decisions/0085-deliverability-report.md),
[0090](../../docs/decisions/0090-mcp-options-admission.md) and
[0091](../../docs/decisions/0091-mixed-work-seats.md) retain their statuses.
0090/0091 collide with independent candidates; their final allocation and every
acceptance belong to the operator. 0092 is the next unused local number above
0091 for this new correction, provisionally, not a reassignment of either one
or a claim of a host-wide reservation. The operator reconciles it at integration.

## Panel adjudication

The positions agree on runtime ownership and reject speculative infrastructure;
there is no genuine conflict about introducing sessions, retries or another
limiter. These rulings resolve their different priorities without inventing
an opposing recommendation.

| Question | Ruling and adopted position | Rejected alternative and cost of the ruling |
| --- | --- | --- |
| Implementation scope | Adopt simplicity's zero-runtime-delta default and existing test owners. Adopt robustness's causal and transient-effect evidence requirements. | Reject speculative hardening and redundant suites. A demonstrated defect can still force a small repair and fresh proof cycle. |
| Correction owner | Adopt robustness's explicit dated home for catalogue, OPTIONS and W9 corrections. Reconcile simplicity's preference for existing 0090 by referencing it from one new locally provisional correction for this design's semantic claims. | Reject a disclaimer alone and rewriting history. Pay one extra proposal and operator numbering reconciliation; do not duplicate the full clause ledger. |
| Method precedence | Preserve accepted 0043 guard-first OPTIONS 405 over universal modern invalid-header 400. Keep the existing handler branch. | The source's modern header MUST remains unmet for OPTIONS. It is additional to C1–C6 and is not operator-approved C7. Universal version middleware loses because it contradicts accepted precedence. |
| Limiter and annotations | Adopt both positions' single catalogue, per-router fixed window and report-as-read classification. | Reject per-peer fairness, global counters and report redesign. Clients share contention; adjacent windows can burst; cache work still consumes resources. |
| Proof and review | Adopt robustness's exact-tree evidence and five separate protected results; adopt simplicity's existing receipt/proof machinery. | Reject historical-green substitution and another panel framework. Tool/provider failures or high residuals can stop delivery despite a small diff. |
| Eval and integrity | Adopt both positions' unresolved HIGH review-integrity concern and unproved host-wide eval coordination. | Reject attack reproduction, engine re-pin and local-lock workaround. Full eval and any untrustworthy review remain unavailable; a source merge proves neither. |

## Required observable contract

The whole-plane Host/Origin guard precedes dispatch. Guard-admitted OPTIONS
returns 405 with `Allow: POST`, no `Access-Control-*`, regardless of absent,
valid, unsupported, malformed or duplicate version headers. Other non-POST
methods return 405 for absent/supported headers and 400 for invalid headers;
HEAD retains HTTP body suppression. POST retains bounded-body and parse
precedence before header admission and effects. Initialization negotiates from
its body after header admission; HTTP revision is immutable per exchange.

The bridge retains its single lifecycle owner, validated initialization result,
initialize/initialized fences, returned-version headers, explicit failure
recovery and concurrent operational exchanges. Modern arrays, including
notification-only arrays, refuse before dispatch; March batches retain existing
concurrency, reply order and unordered effects. No retries or stronger outcome
guarantee follows when a transport reply is lost after a commit.

The catalogue contains exactly thirteen tools. Modern annotations mark seven
reads: search, get, latest, wait, extract links, extract codes and deliverability
report. Six mutations are delete, clear inbox, clear all, seed and set/clear
chaos; they carry `readOnlyHint:false, destructiveHint:true`. Reads carry only
`readOnlyHint:true`. Preserve legacy descriptors and the existing modern wait
wording adjustment. Lazy report-cache fill changes no stored mail or ingest
extraction. Hints grant no authorization. Mail text and links remain untrusted
data; #153 structured outputs and a general schema validator remain out of scope.

One router-owned `Arc<ToolRate>` admits 1024 `tools/call` attempts per one-second
monotonic fixed window across connections, revisions, notifications and March
batch members. Charge before argument staging and effects. Exhaustion yields
the existing text `isError` result; notifications remain silent. Control and
discovery remain available. This is neither a rolling rate nor fairness,
host-wide exclusion, a global resource bound or an SMTP/REST limiter.

## Acceptance Criteria

1. **AC1 — Identity and honest evidence.** Every executed gate/review records
   commit, tree, command or invocation, exit/verdict and receipt identity. Record
   baseline separately from the design/documentation successor and any repair.
   Verify/review the final frozen successor; changed trees invalidate prior
   certification. Old receipts and inspected test names are never passes.
2. **AC2 — HTTP admission.** Fresh real-server `revision_transport_methods` and
   `revision_headers_and_isolation` prove the method/header matrix above, invalid
   Origin precedence and POST parse/body precedence, without handler effects on
   refusals. No CORS or media-type restriction is introduced.
3. **AC3 — Negotiation and lifecycle.** Fresh `tests/mcp_revisions.rs` and
   `tests/stdio.rs` prove all three revisions, isolation, metadata validation,
   returned headers, fences, explicit recovery, concurrency and modern whole-array
   refusal. Transient mutation/subscription observers stay zero on admission
   refusal and have live positive controls; mailbox equality alone is insufficient.
   Legacy batch and forwarding contracts remain.
4. **AC4 — Exact catalogue.** `revision_tool_annotations` proves all thirteen
   names and exact seven/six hint split over real HTTP and the binary bridge for
   both modern revisions, plus legacy descriptors and existing schema shape.
   `revision_tool_schemas_and_text_safety` proves closed inputs and inert text
   projection without claiming immunity to model prompt injection.
5. **AC5 — Shared admission.** Fresh `tool_rate_uses_exact_monotonic_windows`,
   `exhausted_tool_rate_refuses_before_effects_and_keeps_control_available` and
   `the_router_owns_one_tool_rate_across_connections_revisions_and_members`
   establish exact boundary/refill, shared accounting including staging failures,
   silence, control access and zero transient mutation/subscription/chaos effects
   on exhaustion. Use causal seams, not sleep-based exhaustion.
6. **AC6 — Descriptions and corrections.** Served OpenAPI and llms.txt agree with
   observed wire behavior and link both the historical per-revision register and
   the new correction. Retain C1–C6 by actor/source strength and separately identify
   OPTIONS' modern header-400 MUST mismatch. Decision 0092 qualifies historical
   W7 counts, header claims and W9 proof limits with old locations/current witness
   names. Historical `specs/mcp-version-negotiation/` files remain unchanged.
7. **AC7 — Guarantees.** Fresh SMTP visibility, workspace persistence/burst,
   deterministic wait/prune and extraction tests preserve completed storage before
   250, subscribe-before-scan, oldest-only pruning and ingest extraction. Report
   analysis stays separate. Preserve per-test inboxes, no mocks, 0020 placement
   and exact coverage. An accepted-ruling contradiction blocks; a proposal cannot
   waive it.
8. **AC8 — Gates.** The verifier executes the entire current
   [house-rule inventory](../../docs/house-rules.md#gates--part-of-every-change)
   with required `--locked`, exact zero missed lines/branches/functions, coverage
   selftest, publish/seal proofs and fail-closed native-tls absence. Check shims,
   digest-matched candidate-local engine and transitive pinned proof chain. Include
   intake's focused MCP/stdio/recipe tests, design-check selftest and delivery proof.
   No assertion, pin, dependency policy or review boundary is relaxed;
   unavailable/failed checks preclude success.
9. **AC9 — Route and protected results.** Confirm Terra 6.1 intake, Opus 5.5
   implementation and five separate protected results on the final candidate:
   Astra correctness, official DSH Flash security, Spark GLM Flash tests, Astra
   guarantees and independent Claude Fable. Keep distinct charters/invocations,
   read-only classes, worst-verdict aggregation, unknown-fact refusal, hard holds
   and no unauthorized fallback. Two Astra roles are not two model families.
   Compile/effective-route proof is separate from actual review results.
10. **AC10 — Review integrity.** Carry the inherited HIGH concern into relevant
    review packets and the disposition. Only independently supported admissible
    evidence can establish its disposition. No attacks are reproduced; an upstream
    merge or focused route pass cannot resolve it. If trustworthy review cannot
    be established or a high/security hold persists, stop without claiming approval
    or treating it as a safe medium residual.
11. **AC11 — External limits.** Record full standalone eval as unavailable while
    #201's host-wide coordination scope remains unproved. Evidence must cover all
    relevant runners and cleanup lifetimes before any later full-eval claim; a
    worktree lock or sequential recipe is insufficient. Preserve #151's merge hold
    and separately list owed Claude Code, Codex, MCP Inspector and Claude Desktop
    receipts, CI-only platform/network/chart/Docker/port-80 checks and tag-only
    release proofs. No paid trial or external acceptance is implied.
12. **AC12 — Delivery boundaries.** Keep proposals proposed, record number
    collisions for the operator, and commit only this design's three files plus
    one proposal as the final chief commit. Positions remain ignored. The
    deterministic final check owns advancement. No push, merge, publish, tag,
    issue closure, decision acceptance or claimed completed verification follows
    from the chief's `drafted` result.

## Completion boundary

This design supplies requirements, not gate or review receipts. Downstream work
must separate passed, failed, unavailable, not-run and CI/operator-owned
obligations. An honest blocker report preserves the assessment without satisfying
unfinished acceptance criteria or making the overall delivery complete.
