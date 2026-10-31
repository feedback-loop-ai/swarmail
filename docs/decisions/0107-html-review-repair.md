# 0107 — Repair HTML declaration context, inspected work and empty-message reports

- **Status:** proposed
- **Date:** 2026-10-28
- **Scope:** Epic #251 issue #259 independent review repairs before PR #268.
- **Updates proposed:** [0085](0085-deliverability-report.md) report identity/composition and [0106](0106-html-client-compatibility.md) lexical, empty-input and proof details; neither is accepted here.
- **Authority:** operator's #251 lazy cached analysis direction and repair task; compatible work under accepted [0045 §8](0045-delivery-repairs.md).
- **Preserves:** 0001–0005, 0031/0040/0049 stored extraction/raw fidelity, 0020 test discipline, existing report schema/bounds/cache/wait ownership. No accepted ruling is amended.
- **Number:** 0107 follows highest recorded 0106 on this historical candidate. Earlier reserved/missing numbers are not reused; later integration reconciles any independent reservation without rewriting accepted history.

## Context

Shipped a49c702 interprets braces in unused custom-property values as declarations
and warns about position that is not applied. It also mistakes slash-star inside
an unquoted URL for an unterminated comment. SMTP accepts zero-byte messages but
HTML fails their reports through mail-parser. Work tests measure cursor distance,
which cannot detect helper rescans. Original findings are non-security; this
proposal issues no security clearance, waiver or acceptance.

## Decision proposed

The normative [repair spec](../../specs/html-review-repair/spec.md#contract)
pins lexical grammar/version/proof. Its [adjudication](../../specs/html-review-repair/spec.md#panel-adjudication)
records adopted/rejected alternatives and costs.

1. Advance aggregate identity to swarmail-html/2; preserve rule IDs, severities,
   hints, locations, thresholds and public report schema. Constant, catalogue,
   cache/config fixtures and living docs move together.
2. Track rule-list/declaration/component ownership. Braces, parentheses and
   brackets inside values cannot manufacture declarations. Restore context on
   close; following declarations and property-only matches on complex outer
   values remain visible. All CSS components/functions/blocks share cap 32.
   No variable/cascade/selector evaluation or escaped-identifier normalization.
3. Recognize literal case-insensitive url( at token boundaries. In unquoted URL
   data slash-star is literal; escapes protect delimiters. Follow the spec's
   exact whitespace/hex-escape/invalid-control/quote/open-parenthesis/EOF outcomes.
   Outside comments and quoted functions retain semantics. Malformed constructs
   fail atomically; no browser recovery or network fetch.
4. Exactly empty raw after catalogue/raw admission contributes no HTML findings
   before MIME parsing. Only HTML receives this exception: other collectors and
   finish still run. Nonempty parse failures remain failure-not-clean. Historical
   demo/HTML returns empty v2, making no authentication claim. Later main must
   preserve bulk/unknown-auth outcomes, then reverify/review the combined candidate.
5. Count actual lexical operations including helpers/entity decoding with local
   bounded counters and justified conservative charges. Keep progress/legacy
   decoder statistics separate. Derive fixed numeric C/K from a census before
   fixtures for W <= C*n + K, where n is pre-entity eligible HTML bytes, with
   explicit part composition. Include failures, lookup/window/lookahead and 6/5
   expansion. Prove actual counted rescanning fails the same assertion, restore
   and prove green. The proof covers the declared lexical model, not MIME-parser,
   allocator/hash-collision internals, RSS, latency or future uncounted work.
6. Keep lazy off-SMTP analysis, fixed raw/decoded/part/depth/finding/report/header
   budgets, no truncation, success-only cache, worker-owned cancellation, one
   qualified wait owner, exact counts/minimum-one waits. Stored raw/body/link/code
   data and ingest extraction remain untouched.
7. Retain fixed authored provenance/date validation and strict malformed failures
   with negatives. Restore header-overflow wire proof beside malformed HTML.
   Repair the 0106 Markdown row; all proposals remain proposed. Dated pointers
   in 0085/0106 explain repairs without changing historical specifications.

## Alternatives and costs

Both positions support the local repair. Choose explicit bounded frames and
operation accounting over blanket suppression/cursor-only proof; pay additional
state/arithmetic/coverage. Choose a temporary real rescanning mutation over a
permanent fault hook; pay a controlled reproducible red/green run. Choose
HTML-only empty success over documentation-only failure or aggregate bypass;
pay version/fixture/cache/doc updates and composition proof.

Reject full CSS engine, new dependency, generic tracing, extra executor/cache/
wait loop and public quota. Strict malformed input remains unanalyzable; unmodeled
client/CSS behavior remains outside the finite authored heuristic. Four workers
promise no fairness/latency; MIME structural checks follow parsing. No limit
waives an unmet criterion. A new short repair directory/record satisfies this
chief's artifact contract without editing the historical feature specification.

## Proof and disposition

[AC1–AC8](../../specs/html-review-repair/spec.md#acceptance-criteria),
[plan](../../specs/html-review-repair/plan.md) and [tasks](../../specs/html-review-repair/tasks.md)
require full evaluator/real-wire reports, composition, decoder/work bounds,
negative-control receipts, preserved ownership/boundaries and fresh house gates.
The prerequisite index commit keeps proposed 0106/0107 inside the table. The
chief's final commit contains exactly this record and three design files.
Implementation and independent review remain outstanding. CI/operator proofs
retain their named owners. Only the operator accepts decisions; no push, merge,
publication or acceptance is authorized by this proposal.

## Combined bulk and HTML identity — proposed integration note

The earlier `swarmail-html/2` identity described the historical HTML-only
repair candidate. The combined bulk and HTML candidate uses `swarmail-html/3`
because its inputs, findings and empty-message failure behavior differ.
The schema and rule IDs remain stable; the catalogue and cache identity move
with the aggregate version. This note is proposed, not an acceptance.

## March baseline and the report identity — proposed integration note

Merging #282 onto this candidate moved one byte range of the thirteen-tool March
baseline: the seed report row's `rule_set_id`, from `swarmail-bulk/1` to
`swarmail-html/3`. That row gains no finding. The baseline was edited by that
one token and its provenance records the edit; it was not re-captured.

This note reads 0105 §1 ("March ... retain independently captured bytes") as
pinning the March envelope, order, escaping and text-only shape, not freezing
report content across rule-set versions. 0085 requires a new id for these rule
changes, 0105 §2 and its specification admit no fixed rule-set value, and 0105
§1 and 0106 §7 give every adapter one report, so serving the old id on March
would misstate what ran. New HTML findings reach March text-block clients on
mail that triggers them, under the new id. That reading is proposed, not an
acceptance; the operator may rule otherwise.
