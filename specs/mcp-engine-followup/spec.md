# MCP engine follow-up

Created: 2026-10-28. Status: draft design. Scope: epic #164, issue #152.
Run: `follow-up-stopped-run-636c4ba9-f-88f22317`.
Baseline commit: `9317cd145033b23980888fb9803f2b4e5766b7d8`.
Baseline tree: `8080a66c1ee55d1f89287306ef85e6f129466395`.

## Feature and rationale

Resume assessment of the integrated MCP candidate now that its exact local
engine is provisioned. The previous two implementer results stopped for missing
engine availability. The feature is a fresh, candidate-bound proof of the
existing implementation through the requested mixed route. Default runtime delta
is zero; repair only a demonstrated defect compatible with accepted rulings.
An implementation phase does not itself require another source edit.

The legacy `.brokkr/state/tasks/` directory is absent. Framing is the supplied
follow-up task and current intake receipt
`.forge/results/de9b0cf8-608f-4fdb-a238-f764148f8724.json`, not unrelated tasks.
Both current positions under `.brokkr/state/design/positions/` were read.
Those files and the checksummed previous results are ignored run evidence.
Historical specs and prior missing-engine reports retain their original meaning.
This design commit creates a successor to the baseline, requiring new proof.

No semantic contract or ruling changes in this follow-up, so no new decision is
commissioned. The existing proposed [0104 correction](../../docs/decisions/0104-mcp-candidate-proof.md)
owns the thirteen-tool, OPTIONS and shared-budget qualifications. It supplements
[0079 negotiation](../../docs/decisions/0079-mcp-version-negotiation.md),
[0080 reconciliation](../../docs/decisions/0080-mcp-source-reconciliation.md) and
[0090 OPTIONS](../../docs/decisions/0101-mcp-options-admission.md), without
accepting them or rewriting the [historical register](../mcp-version-negotiation/reconciliation.md).
The route is already recorded in proposed [0103](../../docs/decisions/0103-mixed-work-seats.md)
and [0081](../../docs/decisions/0081-glm-flash-seats.md). Lazy report analysis
remains scoped by proposed [0085](../../docs/decisions/0085-deliverability-report.md).
A newly discovered semantic change requires a separate proposed decision and
scope reconciliation; a contradiction of accepted policy waits for the operator
under [0045 §8](../../docs/decisions/0045-delivery-repairs.md).

## Panel adjudication

Both positions actually require fresh full verification and protected review.
Robustness's rejected “compile and reuse old green results” is a tempting
shortcut, not simplicity's submitted recommendation. There is no invented panel
disagreement about that obligation. Adopt their common zero-runtime-delta scope
and robustness's causal evidence standard together.

| Question | Ruling and rejected alternative | Cost retained |
| --- | --- | --- |
| How much new design and infrastructure? | Adopt simplicity's three required artifacts and existing correction owner. Reject another ADR/ledger, permanent receipt schema, protocol service or executor for a tooling-availability follow-up. | Current proof depends on reading existing contract owners; no new protocol capability is gained. |
| How much verification? | Adopt robustness's final-successor gate/review bindings and live observer controls. Reject treating either old passes or fresh compile as correctness/protection. Simplicity also requires this. | Full verification and five reviews cost work even with zero runtime edits; delivery may remain blocked. |
| Broad robustness failure catalogue? | Retain it as review/test-selection guidance over current store, SMTP, wait, prune and webhook owners. Reject turning every hypothetical failure into new production guards or tests without a demonstrated gap. | No new exactly-once, durable webhook, lifetime, fairness or global-resource guarantee. Existing limitations remain. |
| OPTIONS versus blanket invalid-version 400? | Accepted [0043 §4](../../docs/decisions/0043-mcp-origin-boundary.md) wins. Preserve guard-first OPTIONS 405 and the existing method owner; reject universal version middleware. | The modern invalid-header MUST remains unmet for OPTIONS, separately from C1–C6. It is not an accepted C7 or full conformance. |
| Stronger limiter/session behavior? | Adopt both positions' one catalogue, router-owned fixed window and exchange-owned revision/lifecycle. Reject per-client fairness maps, rolling refill, session registry, SSE, stronger IDs and retries. | Shared contention, boundary bursts, stateless history claims and uncertain outcomes after a lost reply remain. |
| Eval/protection gaps? | Adopt robustness's explicit blocking evidence distinctions and simplicity's exclusion of private-lock workarounds/full eval. Reject attack reproduction, access widening or inference of integrity from compilation. | HIGH review integrity and #201 coordination remain unresolved; a green delivery cannot be promised. |

## Acceptance Criteria

These criteria govern downstream implementation/verification/review. A chief's
`drafted` result certifies only committed design artifacts, not completion of
these unchecked obligations or a protected verdict.

1. **AC1 — Identity and engine.** Record actual commit/tree, clean tracked state,
   command or invocation and outcome for every proof group. Verify candidate-local
   `.forge/bin/brokkr` against `bundles/self/tooling.json` and SHA-256
   `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f` before use.
   Execute self and mixed-opus-terra compile with that exact absolute binary;
   both must exit zero without unruled policy valuations. No fetch, substitute,
   build or pin change. Check historical checksums but never attribute old gate
   results to this run. Reverify and review the final frozen successor after
   tracked changes, including documentation.
2. **AC2 — HTTP admission.** Real HTTP proves the Host/Origin guard runs before
   dispatch; hostile Origin refuses before effects. Guard-admitted OPTIONS is
   405 with `Allow: POST` and no `Access-Control-*` for absent, supported,
   unsupported, malformed and duplicate version headers. Other non-POST invalid
   version headers retain 400. POST preserves bounded-body/parse precedence.
   Use the existing matrix and observers; do not substitute final-state-only
   checks for transient-effect refusal.
3. **AC3 — Revision and lifecycle.** Real HTTP and binary stdio prove all three
   revisions (2025-03-26, 2025-06-18, 2025-11-25), immutable exchange revisions,
   initialize/initialized fences, validated installed metadata, failure recovery
   and retained operational concurrency. Modern arrays refuse before member
   effects and notification suppression. March retains concurrent members and
   reply order, without an effect-order guarantee. Existing mutation/subscription
   observers have positive controls, with chaos-state checks where applicable.
4. **AC4 — Catalogue.** Independent expected-name/hint assertions over HTTP and
   the real binary prove exactly thirteen distinct tools: seven modern reads
   including `swarmail_get_deliverability_report`, six destructive mutations.
   Legacy descriptors, closed inputs and text safety remain as implemented.
   Tests must not derive their expected classification from the production
   catalogue. Lazy report cache fill changes neither stored mail nor ingest
   link/code extraction; read-only hints imply neither free computation nor
   authorization, and mail remains untrusted data.
5. **AC5 — Shared admission.** Deterministic owner tests and production-router
   real-connection tests prove one 1024-attempt monotonic one-second fixed window
   across clients, revisions, notifications and March members. Charge before
   staging/effects. Exact boundary/refill proofs use existing test-owned time;
   no timing-dependent 1024-request exhaustion loop. Refusal yields no transient
   mutation, subscription or chaos effect; control/discovery remain available.
6. **AC6 — Honest current contract.** Served OpenAPI and llms.txt retain links to
   both the historical register and proposed 0104, and match current wire tests.
   Preserve C1–C6 and distinguish the additional OPTIONS source MUST mismatch.
   0104 qualifies historical W7, header and W9 assertions without turning source
   presence into passed tests. Preserve all proposal statuses and provisional
   numbering; operator alone resolves collisions and accepts decisions.
7. **AC7 — Guarantees and repairs.** Workspace real-protocol/deterministic proofs
   preserve insert before SMTP 250, subscribe before scan including re-entry,
   oldest-only pruning, per-test inbox isolation and ingest extraction (0001–0005).
   Keep persistence/memory ordering, shutdown admission and best-effort fan-out.
   Any repair stays in its existing owner, has a discriminating regression and
   fresh successor-bound proof. Frozen weakening is refused; no accepted-ruling
   conflict is excused by a proposal. With no demonstrated defect, record no
   runtime repair rather than manufacturing implementation work.
8. **AC8 — Full gates.** All locally required checks in the sole
   [house inventory](../../docs/house-rules.md#gates--part-of-every-change) must
   pass on the final candidate: required locked builds/tests, exact zero missed
   lines/branches/functions, coverage selftest, publish/seal proofs and licenses
   included. Preserve transitive pinned execution, checked shims and fail-closed
   native-tls absence. Include focused MCP/stdio/recipe proofs, design-check
   selftest and the authorized delivery proof. A zero-test filter, unavailable
   proof or failed check is not a pass. If an obligation would require prohibited
   attack reproduction, leave it explicitly unmet and blocking; never weaken it.
9. **AC9 — Route and review.** Resolve Terra 6.1 intake and Opus 5.5 implementation
   through the inherited mixed recipe. Require five separate protected results
   bound to the final verified candidate: Astra correctness, official DSH
   DeepSeek Flash security, Spark GLM-5.3 Flash tests, Astra guarantees and
   independent Claude Fable. Preserve distinct invocation IDs and charters,
   gate/read-only boundaries, exact provider identities, Spark effort not
   applicable, singleton chains/no unauthorized fallback, worst verdict,
   unknown-fact refusal and hard security holds. Two Astra roles are not two
   independent model families. Compilation proves neither dispatch nor protection.
10. **AC10 — Integrity disposition.** Carry the inherited HIGH review-integrity
    residual unresolved into review and disposition. No attack reproduction,
    protection-access expansion, synthetic closure or downgrade. An unresolved
    high/security residual or inability to establish trustworthy protected review
    prevents delivery completion, even when application gates pass.
11. **AC11 — External limits.** Full standalone eval stays unavailable until #201
    establishes host-wide participation/coordination through cleanup. Private
    locks, isolation and sequential local runs do not prove it. Separate failed,
    unavailable, not-run, CI-only and operator-owned evidence. Preserve four owed
    actual-client receipts (Claude Code, Codex, MCP Inspector, Claude Desktop),
    #151's merge hold and CI platform/network/RustSec/Helm/Docker/port-80 and
    tag-only release ownership. No paid trials, push, merge, tag, publication,
    issue closure or decision acceptance. Commit authorized tracked changes unsigned.

## Dated design evidence — 2026-10-28

Chief checked the exact engine digest and all five historical checksum entries,
then freshly compiled self and mixed-opus-terra on the clean baseline above;
both exited zero with 76 valuations over 14 groups and zero unruled. Bundle
digests were respectively `87ee9e9af5460a9883fda8c5076866aed5227b92c4ee0fa9a464cec742ac943b`
and `df8a7f55405b68f048d803010493e5b618951d5ccf82e22f61ba209e4dd4b430`.
Receipts are ignored run-local evidence under `.forge/evidence/chief-engine-followup/`.
This resolves the observed local compile prerequisite for this seat only.
Executable digest equality is not independent source/build provenance. Full
application gates, protected review and standalone eval were not run by chief;
none is inferred from the previous implementer's results or panel compiles.
