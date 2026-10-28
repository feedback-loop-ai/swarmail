# 0082 — Revision-qualified structured MCP tool outputs

- **Status:** proposed
- **Date:** 2026-10-26
- **Scope:** Epic #164, issue #153; implemented additive contract, refining the allocated proposal.
- **Specification:** [Structured MCP outputs](../../specs/mcp-structured-output/spec.md).
- **Framing:** [Intake](../../specs/mcp-structured-output/intake.md).
- **Builds on:** proposed [0079](0079-mcp-version-negotiation.md) and
  [0080](0080-mcp-source-reconciliation.md), without accepting either.
- **Preserves:** 0001–0005, accepted RPC/error contracts and the operator's
  retained C1–C6 deviations. No amendment to those behaviors is proposed.

## Context and authority

The checksum-verified issue153 packet and operator's 2026-10-26 direction
commission structured tool output on supported modern revisions, keeping March
bytes. The spec records packet digests and missing upstream snapshot limits.
Only the operator accepts decisions. Refining this allocated proposal does not
allocate 0081 or accept any proposal. Accepted historical bodies stay unchanged;
this additive contract is compatible with their guarantees.

#152's temporary absence of outputSchema/structuredContent is superseded when this
story is implemented. Its negotiation, lifecycle, annotations, errors, rate limit
and documented C1–C6 deviations remain. A negotiated date and structured outputs
do not certify full revision conformance.

## Proposed contract

1. For Revision::June (2025-06-18) and Revision::November (2025-11-25), every
   existing tool advertises a self-contained object-root outputSchema and every
   successful tool reply carries conforming structuredContent. Keep the single
   JSON text block and isError:false. March, including missing-version defaults,
   retains descriptor/result bytes exactly.
2. Thread the selected exchange Revision through existing dispatch to the success
   formatter. Serialize the original owned Value once for text and include that
   same value as structuredContent for modern replies. No parse-back, wrapper,
   re-fetch, retry or runtime validation after effects. Error helpers/RPC failures
   remain separate with no structured success data; notifications stay silent.
3. The spec's twelve-tool inventory defines required fields/variants. Close owned
   objects, require always-emitted members, preserve optional omission versus null
   and signed received_ms. Raw stays excluded; historical code arrays gain no cap
   or format restrictions. Wait uses closed oneOf found/timeout arms. Do not invent
   transactions between separately observed counts or chaos update/snapshot state.
4. Keep finite schema literals/fragments beside the existing MCP catalogue and
   attach only in its modern projection. No schema engine, new production module,
   generator, SDK, runtime validator or dependency is justified. Input-schema
   ownership remains unchanged.
5. Pin shared constraints to served OpenAPI with the spec's exact output projections:
   requiredness, optional-null removal, closedness, nonnegative counts, wrappers
   and wait variants are explicit. Check source shapes before projecting them.
   Shared chaos request defaults remain legal. No REST runtime or assertion-bearing
   OpenAPI schema change is commissioned; description alignment is owed.
6. Use a finite test-only evaluator checking all schema branches and rejecting
   unknown assertions, malformed schemas and nonlocal/unresolved/cyclic references.
   Prove it with independent negative controls and real HTTP/stdio all-tool outputs;
   it is not a general JSON Schema validator. Self-comparison or schemas accepting
   arbitrary empty objects are not evidence.
7. Keep the bridge's 10 MiB cap and existing failure mapping. Duplicated JSON can
   exceed it where March text fits. Prove this with real traffic; document smaller
   practical pages and that one email can still exceed the cap. No truncation,
   omission of a representation, limit increase or mutation replay hides the cost.

## Synthesis, alternatives and costs

Adopt both positions' single captured value, existing owners and exchange-local
revision. Choose simplicity's finite mapping and OpenAPI projections over
robustness's preferred shared response-schema correction. The cost is continued
looseness in REST descriptions and maintained projection proofs; broad correction
risks shared request semantics without a required runtime benefit.

Adopt robustness's closed objects, exact serializer presence/null rules and extra
size/error proofs. Reject permissive optional-null fields retained merely for
literal REST equality; they admit malformed output. The cost is a stricter new
MCP output contract and deliberate schema changes for future fields.

Adopt the bounded evaluator with robustness's all-branch and negative controls.
Reject a schema dependency/framework for this vocabulary; accept helper maintenance
and a limited validity claim. Reject runtime validation, reply decorators,
store/transport redesign and retry/rollback machinery. Existing storage, SMTP,
webhook and TLS witnesses keep ownership; add representation proofs around them.

Text duplication and schemas increase encoded size, memory and possibly tokens.
Lost replies can follow committed mutations; no exactly-once, rollback, durable
outbox or in-memory restart guarantee follows. JSON escaping preserves untrusted
mail data, not a promise of prompt-injection protection.

## Proof and ownership

Spec AC1–AC10 and paired tasks require all tools/variants, negative controls,
OpenAPI pins, raw March fixtures, text-only consumers, mixed revisions, modern
errors and real stdio size boundaries. Preserve insert-before-250, subscribe-
before-scan, oldest-only pruning and ingest extraction. All house gates and exact
coverage/selftest remain owed on the committed implementation; a design does not
claim feature completion.

Measure **tokens-per-success** and **invalid-argument/tool-misuse rates**, with
turns-per-task context. Protocol tests establish neither metric improvement nor
actual-client compatibility. The operator's #151 baseline and later matrix own
measurements and receipts. CI owns platform/network/chart/Docker/port-80 proofs.
#152/#153 PRs stay unmerged until the baseline exists. No paid trials, route changes,
push, merge, publication, issue closure or acceptance are authorized.

## Implementation witness

The [implementation receipt](../../specs/mcp-structured-output/implementation.md)
names the real HTTP/stdio, raw legacy, OpenAPI, error and cap witnesses. The
bounded evaluator refuses all references because the emitted schemas are fully
inlined. No decision status or operator ownership changes.

## Historical reconciliation ledger

The #152 source-reconciliation specification records the #152 candidate as it
was reviewed and remains unchanged under the operator's historical-spec ruling.
The #153 plan's proposed edits to that older ledger are superseded by this
proposal: the structured output and schema behavior in the contract above is
revision-qualified, and March results remain unchanged. The earlier ledger's
entries describing structured output as absent are historical, not a claim
about the combined candidate. The #153 real-protocol witnesses are in
`tests/mcp_revisions/structured.rs`; external clients remain responsible for
their own validation. This clarification accepts no proposal.
