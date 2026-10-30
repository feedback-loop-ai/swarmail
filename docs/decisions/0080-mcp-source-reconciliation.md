# 0080 — MCP source reconciliation and operator-chosen deviations

- **Status:** proposed
- **Date:** 2026-10-26
- **Scope:** Epic #164, issue #152; operator-directed reframing after run 998279d4.
- **Specification:** [MCP version negotiation](../../specs/mcp-version-negotiation/spec.md).
- **Supplements:** proposed [0079](0079-mcp-version-negotiation.md), which stays proposed.
- **Records:** the operator's 2026-10-26 choice to retain C1–C6; no further runtime amendment is commissioned.
- **Related accepted contracts:** [0029 §2](0029-supplied-values-are-validated-wherever-they-reach-a-wire.md), [0044 §1](0044-compat-validation.md), [0047 §3](0047-storage-rpc.md) and [0039 §§2/6](0039-stdio-concurrency.md).
- **Preserves:** 0001–0005, accepted history, current runtime, all delivery gates and both proposal statuses.

## Context and authority

#152 provides negotiation of 2025-03-26, 2025-06-18 and 2025-11-25, with batching
for 2025-03-26 only. It does not claim full revision conformance. The official
source snapshots are supplied at commit `dae694f61a31f7a3ec2af3aff709e11f70f61e48`;
the [spec manifest](../../specs/mcp-version-negotiation/spec.md#source-manifest)
and existing clause ledger preserve their hashes. Older missing-source notes
record an earlier run and are superseded as current evidence limitations.

The source-reconciliation review stopped at REVIEW-RESIDUAL-UNSAFE, spec_defect
true, with no security residual. Its independent member found C1 over-attributed.
The operator's subsequent 2026-10-26 '#152 conformance' ruling explicitly chooses
to keep C1–C6 as documented deviations. The verified ruling provenance is in the
[spec](../../specs/mcp-version-negotiation/spec.md#authority-and-provenance).
The client matrix (#151 wrappers, then epic exit criteria) determines practical
impact. This records the operator's choice, not acceptance of this proposal.

The earlier proposed HTTP-400/empty-body amendment and instruction to repair
every compatible deviation are superseded for this run. Updating existing 0080
is the commissioned correction; no new decision number is needed. 0079 and 0080
remain proposed. Accepted decisions retain their historical bodies and statuses.
No rule waives a source MUST merely because behavior is deliberately retained.

## Operator-directed disposition

Retain all six deviations, with each revision's observable behavior, source
strength/actor, current authority, operator disposition, witness and cost in the
single [exact register](../../specs/mcp-version-negotiation/reconciliation.md#exact-conflict-register).
The [spec's register requirements](../../specs/mcp-version-negotiation/spec.md#required-contract-and-deviation-register)
define its observations. The register records retention for March, June and
November individually and preserves the clause/schema ledger and W1–W9 proofs;
it replaces the earlier prospective repair/defer framing.

C1 particularly separates envelopes from status. 0029 §2 defines -32600 and
carried IDs; 0044 §1 explicitly pins HTTP 200 for non-object values; 0047 §3
classifies notifications. They do not compel HTTP 200 for response-shaped
objects. The independent review's observation about HTTP 400 with the existing
envelope corrects the older necessity claim. The operator chooses to retain
200/-32600 instead, not because that status is forced by those decisions. No
HTTP-400 repair, empty-body replacement or pending-request service is authorized.
All revisions' empty-202-or-HTTP-error transport obligation remains visible.

The register also preserves C2's supplied IDs and owned output mismatches,
including November absent-versus-null error ID; C3's JSON/SSE advertisement but
identity-JSON parser and unsupported external session duties; C4's tolerant
legacy sender/envelope compatibility and March-only batched initialize; C5's
unknown-tool -32601 versus example -32602, staging and wait split, without
upgrading an example/SHOULD departure into a MUST; and C6's complete malformed
MCP envelopes forwarded to stdout while state installation validates metadata.
March admitted batch positions differ from June/November whole-array refusals.
The shared register must enumerate all three revisions explicitly.

No stricter IDs/envelopes, replay cache, SSE/session machinery, general validator,
SDK, retry, schema change or runtime repair follows. Preserve headers, limits,
statuses, annotations, tools, rate budget and existing behavioral tests. Public
claims and served descriptions must match the bounded claim and link the register.

## Synthesis, alternatives and costs

Adopt simplicity's existing files and one detailed register. Adopt robustness's
removal of contradictory prospective instructions and per-revision actor/strength
precision. Reject a disclaimer-only edit: its small diff would leave a later seat
commissioned to change the wire. Reject new protocol machinery, a repeated source
audit or a second policy registry: their ownership and proof costs buy nothing
for this operator-directed wording correction. The spec records the full panel
synthesis and prices each ruling.

Retained deviations may limit client compatibility. A negotiated date is not a
conformance certificate; JSON-only paired operation does not satisfy a general
client's SSE/session duties; valid JSON is not necessarily valid MCP. C5's
example-level difference is weaker than C1/C2/C3/C4/C6 mandatory mismatches.
The operator accepts retention as the direction for this work, not an all-green
client matrix or a decision status transition.

Preserve insert-before-250, subscribe-before-scan, oldest-only pruning and ingest
extraction. Lost replies can follow committed mutations; webhook delivery stays
best effort. No retry, rollback, exactly-once, durable outbox or broader guarantee.

## Evidence and ownership

Spec AC1–AC6 and paired tasks require source-grounded documentation review,
unchanged behavioral witnesses, served-description checks, structural OpenAPI
comparison and every house gate with exact zero missed coverage on the committed
candidate. The [implementation receipt](../../specs/mcp-version-negotiation/implementation.md)
distinguishes current framing/description proofs from historical runtime repairs.
Public descriptions link the shared register and withhold full revision conformance.

Original issue AC3 stays operator-owned and owed for Claude Code, Codex,
MCP Inspector and Claude Desktop. Local wire tests are not those receipts. CI
owns platform/network/chart/Docker/port-80 proofs. #151's baseline merge hold,
sealing and merge remain operator work. No push, publication, issue closure,
paid trial or decision acceptance is authorized.

## Implementation clarification — 2026-10-26 (historical)

The following describes the previous source-reconciliation implementation, not
new work. Its word 'conflict' records source mismatch; current retention and
attribution follow the operator disposition above.


The [ledger](../../specs/mcp-version-negotiation/reconciliation.md) now records
all scoped source clauses and schema symbols. C3 also covers the general
client's conditional session-ID duties: the paired server assigns none, and the
retained parser/forwarding contract does not implement stateful external peers.
C2 includes error envelopes with null IDs, not just success envelopes. C6 is a
separate concrete upstream-forwarding conflict, not an exemption for this
server's own output. Required initialize metadata is checked before installing
state while complete replies remain verbatim.

A compatible rate repair adds one per-router 1024-attempt, one-second monotonic
window for tools/call, including notifications and March batch members. It
refuses with a text isError before effects, retaining notification silence,
control/discovery access and existing active-wait/concurrency semantics. There
is no peer map, timer task, retry, Store change or dependency. The clause ledger
prices the shared budget and the exact tests prove refill and transient refusal.
Output safety is context-specific: owned tools project typed text/JSON data,
omit raw mail, never render HTML, execute resources or fetch returned links;
client/model consumers must preserve the untrusted-data boundary. This is not a
claim that prompt injection can be solved by JSON escaping.
