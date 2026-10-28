# 0090 — Mixed work seats and protected review

- **Status:** proposed — awaiting the operator
- **Date:** 2026-10-28
- **Scope:** operator-requested mixed-opus-terra route for issue #163.
- **Related:** accepted [0057](0057-codex-delivery-recipes.md), [0058](0058-codex-review-routing.md); proposed [0081](0081-glm-flash-seats.md).
- **Number:** next free on this candidate base; independent candidates use this number and require reconciliation.

## Proposed route

The operator requested Codex GPT-6.1 Terra for intake, Claude Opus 5.5 for
implementation, and Astra review. The additional explicit mixed-opus-terra
recipe extends Careful, overrides those two work seats, and retains five
protected review roles: Astra correctness and guarantees, Claude Fable
independent review, official DSH Flash security, and Spark GLM-5.3 Flash tests.
Separate invocations, security holds, read-only review boundaries, worst
verdict and no-fallback policy remain intact. Existing pinned runs and aliases
retain their original meanings. Compiling a route proves neither provider
availability nor a completed review.

This record proposes a narrow addition to the work-seat routes described in
0057 and 0058. It does not accept itself or 0081; authority and final numbering
remain with the operator. Resolve any conflict with accepted rulings before
landing, and preserve all delivery gates and application guarantees.
