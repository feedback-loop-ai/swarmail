# 0090 — MCP OPTIONS admission and modern version headers

- **Status:** proposed — awaiting the operator
- **Date:** 2026-10-28
- **Scope:** Epic #164, issue #152; correction to current contract documentation.
- **Related:** accepted [0043 §4](0043-mcp-origin-boundary.md); proposed [0079](0079-mcp-version-negotiation.md) and [0080](0080-mcp-source-reconciliation.md).
- **Number:** next free on this candidate base; reconcile independently developed proposals before integration.

## Proposed clarification

Accepted 0043 §4 requires a guard-admitted OPTIONS request to return 405 with
Allow: POST and no CORS headers. This applies regardless of the supplied
MCP-Protocol-Version header. Other non-POST methods reject invalid, duplicate,
or unsupported version headers with 400; POST retains body and parse precedence.
The modern protocol's invalid-version-header 400 requirement is therefore unmet
for OPTIONS. This mismatch is separate from the operator-chosen C1–C6 deviations
recorded in 0080; it is not an operator acceptance of another deviation or a
claim of full revision conformance. The origin guard still runs first.

The current OpenAPI and llms.txt descriptions state this behavior. Historical
specification files remain historical; this proposal records the correction
without changing or accepting earlier decisions. Real HTTP tests exercise
OPTIONS with valid, invalid and duplicate version headers and the guard.
