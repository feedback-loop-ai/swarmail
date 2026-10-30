# 0079 — Revision-qualified MCP negotiation and admission

- **Status:** proposed
- **Date:** 2026-10-26
- **Scope:** Epic #164, issue #152.
- **Specification:** [MCP revision negotiation](../../specs/mcp-version-negotiation/spec.md).
- **Amends:** [0029](0029-supplied-values-are-validated-wherever-they-reach-a-wire.md) §2 initialization/batch scope and §4 bridge metadata; [0044](0044-compat-validation.md) §§1–2 batch admission scope; [0047](0047-storage-rpc.md) §§3–4 classification and bridge refusals for modern revisions only.
- **Preserves:** 0001–0005, 0029/0044 RPC edge taxonomy, 0039 steady-state concurrency, 0043 Host/Origin guards, and 0047's accepted 2026-10-24 supplied-ID amendment recorded in proposed 0062.

## Authority

The issue direction explicitly authorizes negotiating 2025-06-18 and 2025-11-25
alongside 2025-03-26 and retaining batches only for 2025-03-26. The operator's
2026-10-26 ruling authorizes delivery now with an unmerged-PR hold until #151's
baseline exists. Neither instruction accepts this proposal or other proposals.
The verified packet digests and evidence limits are recorded in the linked spec;
no tracked link points into run-local state. Historical accepted decision bodies
remain unchanged. The index records both 0079 and 0080 as proposed.
The later 2026-10-26 '#152 conformance' ruling chooses to retain C1–C6 as
explicit deviations; #152 claims version negotiation, not full revision conformance.

## Proposed contract

The specification's finite negotiation, transport, lifecycle and annotation tables
are the contract of this proposal. Supported single initialize strings return
themselves; unsupported strings return newest-supported 2025-11-25; omission keeps
2025-03-26; supplied non-string versions refuse -32602. Legacy batch initialization
retains its constant legacy result without transitioning bridge state. Modern
initialization uses a single message.

HTTP stays stateless: absent protocol-version header selects legacy, one exact
supported header selects its revision, and duplicate/malformed/unsupported headers
refuse HTTP 400/-32600/null after bounded body parsing and before any effect. A
single initialize negotiates from its body after header validation, even when a
valid supported header differs. No session registry or historical initialization
enforcement is claimed. Required client headers are emitted by the bridge;
existing endpoint media-type leniency is retained.

Both modern revisions reject every top-level array before member dispatch and
notification silence: HTTP 200/-32600/null, one error naming the revision. Shared
classification makes the bridge reject locally and preserves one null-ID error
for oversized modern arrays. Legacy batch cap, concurrent execution, input reply
order, duplicate/null slots and effectful notification silence stay unchanged.
Revision belongs to an exchange, never to shared server state.

The bridge owner fences single initialize and modern initialized exchanges,
installs only a validated matching successful result, and propagates its returned
revision on subsequent POSTs. Failed negotiation cannot release queued modern
input as implicit legacy. The spec pins recovery, early-call, repeated-init and
notification behavior. Normal operational exchanges remain concurrent with
existing framing, response forwarding, deadlines, limits and EOF draining.

Project modern annotations from the existing catalogue: reads get readOnlyHint
true; mutations get false and destructiveHint true, including seed (which can
prune) and set/clear chaos (which replace/remove configuration). Legacy descriptors
stay unchanged; #153 structured outputs are outside scope.

## Synthesis, costs and rejected alternatives

Adopt simplicity's existing owners and refusal of optional session infrastructure,
plus robustness's immutable exchange revision, initialization fence and zero-effect
proofs. A request-header-only bridge loses because queued input can race the
handshake; its apparent simplicity omits lifecycle behavior. A server-only batch
check loses because all-notification errors can be swallowed. Global version state
mixes clients. A session registry adds admission, locking, expiry, shutdown and
exact-coverage obligations without a demonstrated need.

Chosen costs: stateless HTTP trusts the request's claimed revision/history;
non-string single initialize versions lose permissive defaulting; modern clients
lose batch execution; a modern bridge refuses out-of-order calls; legacy batched
initialize cannot negotiate modern mode. Initialization fences briefly stop new
admission but do not block completion of existing waits. The hand-rolled
identity-only HTTP client and uncertain mutation outcome after transport failure
remain; no retries, rollback or exactly-once promise is added.

No storage, pruning, extraction, SMTP, webhook, dependency, coverage or routing
redesign follows. Guarantees remain synchronous insert/query visibility,
subscribe-before-scan, oldest-only pruning and extraction at ingest.

## Evidence and delivery — historical negotiation run

AC1–AC8 and paired tasks require real HTTP/binary stdio tests and all house gates.
Normative snapshots for both modern revisions' lifecycle, HTTP, message schema and
annotations are missing; the spec names their source identifiers. This proposal
fixes an implementable contract without claiming those sources were verified or
that June's contract proves every November delta. CI/operator source reconciliation
and four actual client receipts remain external obligations. Claude Code, Codex,
MCP Inspector and Claude Desktop must have green recorded results before claiming
issue compatibility. Local fixtures are not those clients.

Only the operator accepts decisions, seals or merges. This draft does not lift
#151's merge hold or require its baseline as a local implementation input.

## Source-availability correction — 2026-10-26

The missing-source statement above records the earlier run. The operator has now
supplied official March, June and November sources at upstream commit
`dae694f61a31f7a3ec2af3aff709e11f70f61e48`; the verified manifest and clause
ledger are [recorded here](../../specs/mcp-version-negotiation/reconciliation.md).
Proposed [0080](0080-mcp-source-reconciliation.md) records the new reconciliation
and, under the operator's later 2026-10-26 ruling, the operator-chosen C1–C6
deviations in the [per-revision register](../../specs/mcp-version-negotiation/reconciliation.md#exact-conflict-register).
Neither proposal is accepted by this correction. Full revision conformance is
not claimed. The four actual-client receipts remain operator-owned and owed;
#151's wrappers and the epic exit matrix determine the deviations' practical impact.
