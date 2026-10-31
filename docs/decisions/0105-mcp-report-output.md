# 0105 — Complete the modern thirteen-tool report output contract

- **Status:** proposed
- **Date:** 2026-10-28
- **Scope:** Epic #164 issue #153, current-main #152 integration follow-up.
- **Specification:** [MCP report output](../../specs/mcp-report-output/spec.md).
- **Supplements:** proposed [0082](0082-mcp-structured-output.md)'s historical
  twelve-tool inventory and [0085](0085-deliverability-report.md)'s deferred
  structured report output; neither proposal is accepted or rewritten.
- **Preserves:** 0001–0005, accepted 0029/0039/0044/0047 contracts, proposed
  0079/0080 operator-retained C1–C6 and the historical-spec ruling.
- **Number:** drafted as provisional 0091; renumbered to 0105 at
  integration on 2026-10-30, after main's #163 delivery took 0091–0093 and
  the #152 delivery took 0103–0104.

## Context and authority

The operator commissions thirteen-tool structured outputs on the current
integration, exact March compatibility and independent proof. The staging
candidate dfa7663 is explicitly incomplete; its twelve-entry output catalogue
leaves the actual report descriptor's outputSchema null. The current formatter
already owns one payload and the modern read-only report annotation is correct.
The old fixture/workflow/projections cover twelve tools, and the finite evaluator
does not yet express the report's assertions or u64 upper bounds.

The proposed public output contract needs a decision even though production
ownership stays unchanged. Only the operator accepts it. Compatible work may
proceed under 0045 §8; no accepted ruling is contradicted and no frozen guarantee
may be weakened. This directive does not accept any earlier proposal.

## Proposed ruling

1. Keep all thirteen tools, using the spec's exact name/annotation table.
   June and November descriptors have complete self-contained object-root
   outputSchema; successes retain their JSON text block and isError:false and
   add matching structuredContent from the same captured value. March and its
   absent-version default retain independently captured bytes. Errors and
   notifications retain their distinct existing contracts.
2. Extend the finite schema mapping beside the existing MCP catalogue.
   The report/finding/location objects are closed; required fields, three
   severities, tagged Header/MimePart/Link union, explicit-null header occurrence,
   empty paths, exact u64 range, 256 findings and exact string constraints follow
   the specification's inspected OpenAPI contract. Preserve the existing twelve
   serializer projections. No fixed rule-set/rule-id enum, extra public switch,
   dependency, runtime validator, second read, re-analysis or retry is added.
3. Keep encoded-report bytes/order/dedup and bounded lazy cache/worker behavior
   as separately tested producer constraints. Do not invent a JSON Schema
   keyword to claim the 65536 compact-byte limit. Lazy analysis remains outside
   ingest/SMTP and never recomputes stored extraction.
4. Extend only the test evaluator's emitted vocabulary, with all-branch schema
   validation, exact numeric comparisons and independent positive/negative
   controls. Unknown assertions/patterns, malformed schemas and references
   refuse. Pin four report OpenAPI components alongside the existing six before
   explicit projection; discard only classified annotations, retaining every
   assertion and legal request default.
5. Require all thirteen real successes over HTTP and binary stdio for both
   modern revisions, exact case lengths/name sets, independent expected payloads
   and modern-text/March equality. Production-reachable report forms receive
   real stored-byte/adaptor witnesses. Reserved Link/empty forms receive
   independently constructed typed serialization/schema witnesses, clearly
   labeled; never change production rules or mislabel them as binary receipts.
6. Preserve the original twelve-tool March fixture and historical specifications.
   Add an independently captured thirteen-tool baseline from actual pre-#153
   #152 code with source/tree/build/executable/raw-byte provenance. The local
   3a2eef9 candidate can establish local compatibility but cannot discharge the
   separate actual #152 baseline and protected-review prerequisite. Unavailable
   independent evidence stays an unmet criterion.
7. Correct stale report descriptions only in modern projection/current product
   docs while preserving March bytes. This proposal is the current inventory
   supplement: existing 0082, including its historical-ledger clarification,
   and all old #152/#153 artifacts remain intact. Implementation adds the index
   row and current witness pointers; acceptance remains the operator's.

## Alternatives ruled and costs

Adopt both positions' single-value/existing-owner design and independent baseline.
Choose robustness's strict constraint/negative-control proof over count/fixture
refresh: the extra test work is required evidence, not optional hardening.
Choose simplicity's finite implementation over a generator/registry/runtime
schema engine: maintain duplicated literals and explicit drift proofs.

Choose simplicity's typed-versus-wire distinction over robustness's demand for
every reserved location through today's production rules. Cost: no production
Link/empty-report receipt exists; typed proofs do not claim one, and any broader
wire requirement remains explicitly outstanding. Production output need not
change merely to make a test emit a legal future variant.

Reject robustness's in-place rewrite of historical #153 artifacts and 0082's old
inventory. The operator's historical-spec ruling takes precedence. Cost: a new
scoped supplement and index pointer rather than one rewritten history.

Modern replies duplicate JSON and can exceed the unchanged 10 MiB bridge cap
where March fits. Keep real boundary/correlation/continuation witnesses, not
truncation, cap increases or replay. Smaller pages may help but one email may
still exceed the cap. Snapshot reads, shared resources, best-effort webhooks,
in-memory process loss and uncertain post-commit reply outcomes remain. No
availability, exactly-once, rollback or full MCP conformance promise is added.

## Proof, residuals and disposition

Spec AC1–AC10 and the paired tasks own exact catalogue/schema, OpenAPI projection,
finite evaluator, independent March, real transport, report/invariant/error and
complete local gate proofs. Exact 100% coverage/selftest, locked dependencies,
publish guard, seal proof, permissive licenses and pinned execution remain owed
on the clean unsigned implementation commit. A design commit is not shipment.

All original findings of mixed review 53f33089 remain unchanged historical
evidence, including HIGH inherited security-relevant review-integrity/result-
attribution residuals, member-specific missing live-work evidence, documentation/
manual-proof limitations and indeterminate tests. No fresh verdict follows from
an earlier local shipment or another candidate's settled member. Do not reproduce
attacks, forge results, widen reviewer writes or re-pin the engine.

Retain the selected Terra 6.1 intake/Opus 5.5 implementation and five separate
protected gate-class results: Astra correctness and guarantees, official Flash
security, Claude independent and local DSH Spark GLM-5.3-Flash tests. Missing
standalone-eval host-wide same-user nonoverlap or exact captured executable
digests is reported, never fabricated. No paid trials.

Actual #152 baseline/protected review, paid #151 baseline, actual client receipts,
CI and operator seal remain independent merge prerequisites. No push, merge,
publication, tag, issue closure or acceptance is authorized.
