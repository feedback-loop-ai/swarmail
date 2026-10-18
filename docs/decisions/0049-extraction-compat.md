# 0049 — Source-aware extraction and consistent compat reads

- **Status:** proposed — acceptance is the operator's.
- **Date:** 2026-10-18.
- **Issues:** #136, #137, #138, #139; epic #120 batch 4.
- **Specification:** [extraction-compat](../../specs/extraction-compat/spec.md).
- **Updates proposed:** [0040](0040-extraction-fidelity.md) §§2/4
  (MIME-relative preference, bounded codes and source-aware links), extends
  §§5–7's full-table/numeric proof without relaxing provenance, preserves §8;
  [0044](0044-compat-validation.md) §§4/5/7 (uniform compat view order,
  complete response offload and wrapped-prefix refusal);
  [0038](0038-store-memory-and-timestamps.md) §4 (compat scoped-order exception).
- **Extends/clarifies accepted:** [0030](0030-loopback-plane-bounded-results.md)
  off-worker work scope and [0031](0031-sender-honest-stored-bodies.md)'s
  display/extraction distinction. Stored bodies and historical rows remain
  verbatim. No accepted decision text is rewritten or presumed accepted here.
- **Preserves:** 0001–0005, exact extraction/count pins, raw fidelity,
  the single ordered mutation boundary and all prior epic batches.

## Context

Whole-source decoding loses the difference between a source attribute's
closing quote and an encoded quote in its URL. A partial entity table and
signed numeric parsing violate the intended decoder contract. Text-first
codes are inappropriate when the chosen text is an unrelated footer.
Compat prefix checks miss wrapped syntax, scoped views use a different
order from latest, and single-message response work blocks async workers.
Unbounded code collection amplifies numeric mail into millions of strings.

## Proposed ruling

1. Recognize HTML source boundaries before decoding. Share existing tag
   quote-state recognition; decode quoted attribute values once and preserve
   their decoded quotes as URL data. Other source regions retain ordinary
   decoded delimiters. Preserve source-only links, including comments/raw
   elements and unfinished tails; code suppression does not suppress links.
   Keep text-before-HTML deduplication and HTTP(S) spelling. Permit interior
   commas, IPv6 brackets and matched parentheses; trim suffix punctuation
   and unmatched closers with linear work. No browser recovery or validation.
2. Complete the 2,125-name table from a pinned WHATWG snapshot with real
   SHA-256, CC-BY-4.0 attribution and reproducible generation, tested against
   every source mapping. No CPython substitution or subset completion.
   Numeric references require nonempty ASCII digits of the proper radix;
   retain scalar policy, semicolon requirement, decode-once, 40-character
   window, exact statistics and the 6/5 byte expansion bound.
3. Only selected bodies that are direct siblings under one
   multipart/alternative use text codes with HTML fallback when text yields
   none. Other selected bodies contribute in source byte-offset order. Keep
   body selection, filenames, empty/absent bodies, raw data and attachment
   exclusion unchanged; no persisted MIME ancestry or synthetic body.
4. Fresh ingest retains the first 64 distinct codes across that ordering,
   using a collector bounded before allocation, shared across runs/sources.
   Duplicate matches do not consume slots. Preserve all exact-count/work
   pins. SMTP, seeds and webhook payloads share the stored result. Historical
   arrays restore verbatim; no load-time truncation, migration, query-time
   derivation or write-back. Publish the cap and historical exception.
5. All compat scoped/global list/search views sort descending
   `(received_ms, id)` before paging; latest keeps its same maximum. Native
   inbox listings, POP3 numbering, eviction and restore sequence retain
   acceptance order. No in-memory sequence, timestamp or UUID changes.
   Independent requests may observe concurrent delivery differently. Keep
   latest's existing scope/filter errors and document tz's named 400.
6. Generic containing search on both dialects checks the existing ASCII
   identifier-colon rule after at most one leading `(`, one `-`/`!` and one
   double quote in that order. A closing wrapper or whitespace within the
   value cannot hide the field. Return a named 400 for the issue forms,
   including `(to:x)`. Preserve `://`, ordinary literals and explicit field-
   kind values; recognized wrappers are for refusal only, never execution
   or transformation of accepted queries. No general Mailpit grammar.
7. Use awaited existing offload for complete single-message projection,
   rendering, copying and serialization, including full/latest, both plain
   aliases, raw/header/latest aliases, download and native JSON/raw. Return
   finished responses, with no deferred JSON serialization on async workers.
   Release map guards and hold no mutation lock while rendering; preserve
   error, panic, content-type and raw-byte contracts. Keep existing page
   offload and MailHog's inclusive 64 MiB page budget.

## Alternatives, costs and residuals

The specification's panel table rules explicitly. Simplicity wins on direct
siblings, timestamp view order, timezone refusal, snapshot provenance and
verbatim historical restore. Robustness wins on common lexical ownership,
construction-site offload proof and covering raw/header/download siblings.
Both require bounded collection. Neither a new acceptance sequence nor a
CPython provenance shortcut is needed to satisfy the issue criteria.

Costs are real: nested alternative/related structures can expose extra HTML
codes; parse-start clock rollback can make latest differ from last accepted;
scoped compat sorting adds O(n log n) work; clients sending tz must change.
A 65th distinct OTP is omitted. Historical code arrays and all link arrays
remain uncapped beyond source size, so this is not a global-memory repair.
Roughly 600,000 short distinct URLs in 8 MiB can still amplify storage,
responses and webhooks. A link cap or historical cleanup requires separate
scope and proof. Pool saturation can delay parses and commits; success still
follows synchronous commit, so no acceptance durability guarantee is relaxed.

## Proof and delivery

Spec AC1–AC11 and the paired tasks require all issue fixtures, all mappings,
unchanged seven #94 rows and exact work pins, MIME ordered-array proofs,
named query refusals, deterministic timestamp disagreement/ties/restart,
real-server held-response progress, bounded collector/response fixtures,
historical-row preservation and all house gates. Every changed behavior
needs a red/green regression; a structural design check is not that proof.
Follow 0020 placement and 0035 coverage discipline, with no exclusions,
dependency-policy broadening or accepted-text edits. Implementation indexes
this proposed record and adds dated pointers; only the operator accepts it.
