# 0092 — Dated corrections for the integrated MCP candidate

- **Status:** proposed
- **Date:** 2026-10-28
- **Scope:** epic #164 issue #152 candidate ab807436aa9bab8ebe21c49f6dea3481bb74eebe; documentation and proof attribution.
- **Design:** [spec](../../specs/mcp-candidate-proof/spec.md), [plan](../../specs/mcp-candidate-proof/plan.md), [tasks](../../specs/mcp-candidate-proof/tasks.md).
- **Number:** next unused local number above 0091, provisional only. 0090 and 0091 already collide with other candidates; this record neither replaces them nor allocates final numbers. The operator reconciles numbering and alone accepts decisions.
- **Authority:** accepted [0043 §4](0043-mcp-origin-boundary.md), [0045 §8](0045-delivery-repairs.md), [0057](0057-codex-delivery-recipes.md) and [0058](0058-codex-review-routing.md); current operator-directed independent assessment. No accepted ruling is amended.

## Why this record exists

The integrated candidate has thirteen catalogue entries, an OPTIONS precedence
repair and a new shared-router wire witness. Historical reconciliation prose
predates those changes. Preserve that history and qualify its current use here.
These are source observations, not fresh test results, accepted policy changes,
a remote-source audit or permission to claim full MCP conformance.

## Dated corrections — 2026-10-28

Locations below identify the baseline's
[historical ledger](../../specs/mcp-version-negotiation/reconciliation.md).
Line numbers aid lookup on ab80743; named rows remain the durable identifiers.

| Historical assertion | Current qualification and witness |
| --- | --- |
| W7, line 49, says twelve schemas/names/hints; March/June/November listing rows at 536/544/552 say twelve tools; annotation summary at 686 says six reads. | `src/mcp.rs::TOOLS` and `tool_descriptors` expose thirteen entries. June/November have seven reads including `swarmail_get_deliverability_report`, and six mutations. Legacy descriptors stay unchanged. `tests/mcp_revisions.rs::revision_tool_annotations` compares exact names/hints on HTTP and the real binary; `revision_served_contract` checks the served catalogue. Fresh execution remains required. Proposed [0085](0085-deliverability-report.md) keeps lazy derived cache fill separate from stored-mail mutation and ingest extraction. |
| Header summary at 114 and June/November Protocol Version Header rows, particularly invalid-header MUST rows at 347/497, say blanket 400 or “conforms; non-POST refusal repaired.” | Guard-admitted OPTIONS returns 405, `Allow: POST`, no `Access-Control-*` for every version-header shape. Accepted 0043 §4 controls; hostile Origin is refused first. Other non-POST invalid headers return 400; POST retains body/parse precedence. Modern server invalid-version-header 400 MUST is unmet for OPTIONS. Proposed [0090](0090-mcp-options-admission.md) records this separate mismatch. `revision_transport_methods` and `revision_headers_and_isolation` require fresh wire execution. |
| W9, line 51, says shared-budget exhaustion is not proved on the wire. | The candidate adds `src/api.rs::tests::the_router_owns_one_tool_rate_across_connections_revisions_and_members`, exercising production router wiring over real connections with test-owned window state and transient observers. It complements `tool_rate_uses_exact_monotonic_windows` and `exhausted_tool_rate_refuses_before_effects_and_keeps_control_available`. Its presence supersedes an inference that no such witness exists; it does not discharge the earlier finding until fresh execution and protected review establish the result on the identified tree. |

The shared window is per router, 1024 attempts per monotonic second, before
staging/effects, including notifications and March members. It is not a rolling
limit, per-client fairness, host-wide coordination or a global resource bound.
Other planes and control/discovery retain current admission. Report reads can
consume bounded computation despite not mutating mail.

## Retained source differences and costs

[0080](0080-mcp-source-reconciliation.md) and the historical register retain
C1–C6 for March, June and November with actor/strength distinctions:

- C1: response-shaped input retains HTTP 200 with RPC refusal by operator
  direction; accepted envelope/ID rules do not compel that HTTP status.
- C2: supplied-ID tolerance and owned-output schema mismatches, including
  November omitted-versus-null error ID, remain visible.
- C3: JSON/SSE advertisement contrasts with identity-JSON parsing and missing
  general external SSE/session duties; a paired stateless server does not prove
  the general client's conditional obligations.
- C4: tolerant envelope/lifecycle forwarding and March-only batched initialize
  remain distinct from modern whole-array refusal.
- C5: unknown-tool/staging/wait taxonomy retains an example/SHOULD departure;
  it must not be promoted into a mandatory source conflict.
- C6: complete malformed MCP envelopes may be forwarded verbatim to stdout;
  validating initialization metadata before state installation does not cure it.

OPTIONS is an additional modern server MUST mismatch grounded in accepted 0043,
not a seventh operator-chosen deviation and not a proposal accepting one. Current
served descriptions must link this correction alongside the historical register.
Do not rewrite historical specs, recopy the whole ledger, add protocol machinery
or claim that negotiated dates certify conformance. Four actual-client receipts
remain owed; #151's merge hold remains. The cost is retained client incompatibility
and one discoverable correction with provisional numbering.

## Evidence and review boundary

The design adopts simplicity's existing owners and minimal changes, and
robustness's explicit correction and exact-tree proof requirements. Generic
version middleware loses to accepted OPTIONS precedence; extra fairness/session/
retry infrastructure loses to the bounded assessment scope. Pay deterministic
proof/review costs while retaining contention, boundary bursts, stateless history
claims and uncertain outcomes after lost transport replies.

Every proof must identify commit/tree and execution outcome. A design or doc
successor is a different candidate. Require all current house gates and five
protected results: Astra correctness, official DSH Flash security, Spark GLM
Flash tests, Astra guarantees and independent Fable. The requested Terra 6.1 /
Opus 5.5 work route is recorded in proposed [0091](0091-mixed-work-seats.md);
[0081](0081-glm-flash-seats.md) records tests routing. Neither is accepted here.
Compile success is not dispatch, review approval or evidence of protection.

Carry the inherited HIGH review-integrity concern unresolved. Do not reproduce
attacks, relax review boundaries or infer resolution from an upstream source
merge. Independent admissible evidence and the protected review path own any
disposition; unknown or high/security residuals block. The historical dispatcher
failure in 0081 is not erased by focused route tests.

Full standalone eval is unavailable while #201's host-wide coordination scope
and participation through cleanup remain unproved. No local lock, sequential
recipe or isolated worktree supplies that proof. Separate CI-owned proofs,
operator client receipts and failed/unavailable gates explicitly. No paid trial,
release action, push, merge, issue closure or acceptance follows. 0001–0005,
exact coverage, pinned proofs and hard holds remain in force.
