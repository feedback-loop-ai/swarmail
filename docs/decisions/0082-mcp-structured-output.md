# 0082 — Revision-qualified structured MCP tool outputs

- **Status:** accepted (operator, 2026-11-02 — "accept all as recommended",
  given in chat to the dispatcher).
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

## Dated addendum — 2026-11-02 (at acceptance)

The operator accepted this record on 2026-11-02. The text above is unchanged
and remains the record of what was proposed. This addendum states what has
moved since it was written, checked against `main` at `c336c95`; it rules
nothing beyond the acceptance.

- **What the March freeze covers.** The 2025-03-26 freeze covers the tool
  descriptions that existed when it was written and the captured result
  bytes of those tools. It does not freeze report content, and it does not
  freeze what later records add: [0094](0094-focused-mail-waits.md)
  (`swarmail_wait_for_code`, `swarmail_wait_for_link`),
  [0110](0110-attachments-and-ics.md) (`swarmail_list_attachments`,
  `swarmail_get_attachment`) and [0112](0112-recipient-inbox-routing.md)
  (`swarmail_create_inbox`) append descriptors on every revision, March
  included; [0111](0111-near-miss-timeouts.md) adds `near_misses` to a
  timed-out wait on every revision; and [0107](0107-html-review-repair.md)
  moves the report's `rule_set_id` from `swarmail-bulk/1` to
  `swarmail-html/3` on March as on the modern revisions (the March fixture's
  provenance,
  `tests/fixtures/mcp-structured-output/march-current-main.provenance.json`,
  records that one-token edit). The served MCP description in
  `src/static/openapi.json` says the same: March keeps its thirteen existing
  catalogue entries and result bytes, and discovery appends the later tools
  after them, eighteen in all.
- **The inventory.** §3's twelve-tool inventory is historical;
  `tests/mcp_revisions.rs` pins the served catalogue as eighteen tools
  (`CATALOGUE`), the same names on all three revisions
  (`revision_tool_annotations`).
- **The merge hold was not kept.** "#152/#153 PRs stay unmerged until the
  baseline exists": #282 merged as `85d45ed` on 2026-10-31 and #288 as
  `42f04f2` the same day; the baseline was published on 2026-11-01
  (`f815d48`).
- **The metric.** The [2026-11-01 baseline](../evals/baseline-2026-11-01.md)
  records tokens per success as not measured.
