# 0090 — Versioned HTML client compatibility heuristics

- **Status:** proposed
- **Date:** 2026-10-26
- **Scope:** Epic #251 issue #259; additive HTML findings through shipped 0085.
- **Authority:** operator's #251 direction, 2026-10-26: lazy cached analysis of stored bytes, never SMTP work; built-in deterministic rules. Number 0090 is assigned by this task; earlier reserved numbers are not reused.
- **Preserves:** accepted 0001–0003, 0005/0031/0040/0049 ingest and raw-byte guarantees, exact predicates and current adapter contracts. No accepted ruling is amended or accepted here. 0085 remains proposed.

## Decision proposed

The normative [specification](../../specs/html-client-compatibility/spec.md#contract)
pins literal rule IDs, hints, source guidance, parser scope, locations, thresholds
and errors. Its [adjudication](../../specs/html-client-compatibility/spec.md#panel-adjudication--2026-10-26)
records adoption/rejection of both contrarian positions and the losing costs.

1. Extend the existing pure collector with seven embedded, data-driven HTML
   heuristics under `swarmail-html/1`; retain the demo rule. Closed selectors,
   severity, hints, client scopes, thresholds and explicitly authored provenance
   live in tracked JSON. Existing serde and mail-parser suffice. No imported
   client-support dataset, claimed empirical snapshot or runtime network access.
2. Analyze actual outer-message HTML body leaves once per cold report, with
   structural MIME paths. Exclude attachment/filename subtrees and attached
   messages; never synthesize bodies or alter stored extraction. One finding
   per rule/part enters the existing canonicalizer; public schema is unchanged.
3. Use finite source-aware HTML/CSS lexical scanning with bounded entity decode.
   Quotes/comments/strings protect structure. Hidden conditional comments,
   escaped CSS identifiers, cascade/layout and CSS visibility are explicit gaps.
   Incomplete supported lexical constructs fail, rather than returning clean.
4. Missing alt means absent attribute; present empty alt passes. Image-only is
   any active img with no substantive lexical body text, including a lone pixel.
   Potential trackers are HTTP(S) images with both integer HTML dimensions 0
   or 1; CSS sizing and remote metadata are outside this heuristic.
5. Clipping risk is strictly more than 104448 decoded UTF-8 HTML bytes per part,
   before entity decoding. This 102 KiB convention interprets approximate named
   guidance; it is not a live Gmail invariant or an SMTP size limit.
6. Limit raw analysis to 8 MiB before MIME parse, aggregate eligible decoded HTML
   to 4 MiB, outer parsed MIME parts to 256, MIME root-relative depth to 32 and
   lexical nesting to 32. Retain header/report/finding limits from 0085. These
   bound workloads, not RSS or latency. Explicit failures cannot yield partial
   success, clean qualification, a success cache entry or partial exact count.
7. Keep the existing Weak identity/config/version cache, four worker permits,
   128 slots, worker-owned cancellation and selected snapshots. No SMTP/seed/
   restore/fan-out analysis, extra cache, executor or wait owner. Current REST,
   MCP text-block and escaped UI consumers share the same typed report/errors.

## Alternatives and costs

Choose robustness's catalogue validation and pre-parse/structural/decoded work
bounds over relying only on the storage cap: some accepted large mail cannot
be analyzed. Choose its 102 KiB UTF-8 clipping convention over simplicity's
100 KiB pre-charset convention: the intervening range does not warn and charset
conversion affects the estimate. Keep simplicity's literal CSS/comment scope,
attribute-only pixels and any-img image-only rule: Outlook hidden markup and
CSS-only trackers can be missed; a lone beacon can produce two findings.

Reject rendering infrastructure, a generic rules engine, new dependencies,
per-occurrence HTML locations and changes to ingest extraction. Pay one small
analysis-local lexer and part-level diagnostics. Adopt authored, attributed
heuristics rather than claiming locally unavailable client-version measurements.
All limitations must remain visible in documentation and negative fixtures.

## Proof and disposition

[Acceptance criteria](../../specs/html-client-compatibility/spec.md#acceptance-criteria)
and [paired tasks](../../specs/html-client-compatibility/tasks.md) require every
rule's pass/fail fixture, exact work/size boundaries, failure-not-clean,
real-protocol parity, preserved frozen guarantees and all local house gates.
CI/operator-only proofs retain their owners. The proposed index row is supplied
by a separate prerequisite commit because the inventory gate requires it now.
The final chief commit is restricted to this decision and the three design files. Only the operator may accept this proposal. Compatible work under
the supplied #251 direction and 0045 §8 can proceed; frozen weakening cannot.

## Repair implementation pointer — 2026-10-28 (proposed 0091)

[0091](0091-html-review-repair.md), still **proposed**, records the compatible
review repair: swarmail-html/2 preserves CSS component ownership and literal URL
data, treats exact-empty raw only as no HTML parts (other collectors/limits
still run), and proves counted helper inspections separately from progress.
The [implementation ledger](../../specs/html-review-repair/implementation.md)
names regressions, original non-security findings and proof boundaries. The
historical text above remains as recorded; no proposal is accepted here. Later
main integration must preserve bulk/unknown-authentication semantics and receive
fresh combined verification/review.
