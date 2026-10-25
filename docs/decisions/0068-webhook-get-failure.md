# 0068 — Document webhook GET preparation failure

- **Status:** accepted (operator, 2026-10-24)
- **Date:** 2026-10-24
- **Scope:** Epic #192 issue #195 continuation, public OpenAPI status contract.
- **Specification:** [webhook pool review](../../specs/webhook-pool-review/spec.md).
- **Clarifies:** the existing GET failure surface alongside accepted [0041](0041-webhook-boundary.md) §3 and [0048](0048-rest-webhook-validation.md) §3 diagnostics; no accepted clause is replaced.
- **Preserves:** accepted 0001–0005, 0023/0030, 0041/0048, 0045 §8, 0050 §4 and 0058. Existing [0067](0067-webhook-ca-safety.md) stays proposed.
- **Number:** 0068 is the next number after the highest existing record, 0067; historical gaps are not reused.

## Context

GET already awaits public response preparation and returns a fixed value-free
500 when its blocking job cannot join. The served OpenAPI response map lists
only 200/403; PUT already lists its own 500. Correctness and guarantees found
this documentation omission in the recovered review. That review also reported
a true security residual: repeated unauthenticated GET projection shares the
blocking pool with SMTP parsing and Store mutation. Its low severity and reference
to proposed 0067 do not accept that residual.

## Proposed clarification

1. Document GET `/api/v1/webhooks` 500 as response-preparation failure with fixed
   `webhook response preparation failed` in the existing native error envelope.
   State awaited off-worker preparation on GET itself. Preserve GET 200/403,
   PUT's existing failure-before-swap contract and all schema shapes. This
   records reachable behavior; it introduces no new failure mode or status.
2. Preserve public complete certificate diagnostics, raw installed input,
   unrelated-section/mixed-PEM admission, complete offline cache-neutral
   validation, single PUT swap and direct verified delivery policy. No admission
   limit, overload status, cache/executor or shutdown policy is selected.
3. Evaluate API-induced resource contention using the specification's real
   HTTP/SMTP capacity and cancellation tests. A finite synthetic hold proves
   coupling and recovery, not a natural attack threshold or safety bound. The
   32-entry cache and 22.1-second network/backoff sum are not global work limits.

## Authority and stopping rule

This proposed clarification does not amend an accepted ruling and does not
accept 0067, itself, or an availability/security residual. Compatible contract
repair and investigation may proceed under accepted 0045 §8. If a concrete
remedy contradicts an accepted clause, draft its precise proposed amendment
and stop dependent implementation for the operator. A frozen weakening is
refused regardless of any proposal or test result.

Accepted 0050/0058 remain the authority: only the authorized security office
resolves its finding, unsafe/unknown facts stop, and the engine lane requires
five separate current results including official DeepSeek Flash security.
Never Pro, unauthorized fallback or author-made safety facts. The chief's
`drafted` result and structural design check do not clear protected review.

## Alternatives, costs and proof

A documentation-only closure loses to explicit GET contention/cancellation
proof because the old delivery-held test leaves the API path unmeasured.
A dedicated executor or new admission policy loses to bounded investigation
because it introduces unresolved ownership, fairness and shutdown contracts.
The cost is extra deterministic harness work and potentially a held delivery;
no runtime availability remedy is claimed.

The served-contract test must fail on removing GET 500 and pass on restoration;
existing live join-failure tests pin exact value-free errors. Spec AC2–AC8
require preserved behavior, causal capacity/cancellation evidence, full gates,
current candidate/executable binding and protected security evaluation. Only
the operator accepts this record. Implementation may index it as proposed;
this design commit contains only this record and the three specification files.
