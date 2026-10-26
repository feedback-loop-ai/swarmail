# 0089 — Source-citing bulk-sender checklist through the report contract

- **Status:** proposed
- **Date:** 2026-10-26
- **Scope:** Epic #251 issue #257, additive rules through proposed 0085.
- **Authority:** supplied #251 operator ruling and issue #257 task brief.
- **Preserves:** accepted 0001–0005, report/cache/wait ownership and all gates.
- **Number:** 0089 was assigned and created at intake; reserved gaps are not reused.

## Proposed ruling — chief synthesis

The normative IDs, severities, hints, syntax, coordinates, authentication truth
tables and analysis limits are in the
[specification](../../specs/bulk-sender-checklist/spec.md#contract).
This elaborates intake, accepting neither this proposal nor 0085.

1. Add seven checks through the existing lazy report: one-click structure,
   Message-ID, Date, sender plain body, DMARC presence, From alignment and
   passing aligned DKIM-or-SPF. Set swarmail-bulk/1; retain demo.marker.v1 and
   its flag. Preserve serialized shapes, collector/cache, wait owner and adapters.
2. A pass emits nothing; failure emits its ID. Auth Unknown emits a distinct
   *.not_evaluated.v1 ID with fixed "Not evaluated:" hint. Header defects and
   auth failure/unknown are error; plain-text advice is warning. Production
   defaults Unknown, excluding every message at every existing threshold.
   Exact zero assertions and minimum-one waits retain their original meanings.
3. Validate complete original singleton headers using current RFC 5322 syntax;
   obsolete-only forms are unsupported by this checklist, not universally invalid.
   One-click requires a complete list with a supported HTTPS URI and exact
   RFC 8058 Post token. Syntax cannot prove DKIM header coverage, endpoint
   operation or Message-ID uniqueness. Date uses calendar validity, never freshness.
4. Genuine outer-message plain text includes empty/default/nested/encoded body
   leaves, excluding attachment ancestry and embedded messages and respecting
   digest defaults. Never synthesize from HTML. This is presence advice, not
   equivalence or an unconditional provider mandate.
5. Consume internal immutable three-state facts: policy presence, valid From,
   DKIM pass/alignment and SPF pass/alignment. Presence accepts valid p=none.
   Alignment is valid From AND either alignment; authenticated sender is valid
   From AND either passing aligned mechanism. False wins AND, True wins OR,
   otherwise unresolved combinations stay Unknown. #255 owns verification,
   domains/alignment, provenance and selected-mail binding. Sender headers cannot
   supply evidence. Include all effective facts/policy identity in Inputs equality
   alongside Weak selected-mail identity. #253/#255 own expiry/refresh/wakes.
6. Keep job/cache/report bounds and worker-held cancellation ownership. Add
   inclusive metadata limits: 8,388,608 raw bytes, 65,536 per header block,
   262,144 aggregate header bytes, 256 outer MIME entities, depth 32 (root zero).
   Complete bounded structural inspection precedes a report; exhausted budgets
   or ambiguous/truncated MIME produce existing sanitized failure, never partial
   success. No SMTP limit change, decoded-body work, durable derived state or
   separate executor.
7. Fixed new hints cite supplied Gmail/Yahoo and applicable RFC references and
   never echo sender content. Root header occurrences are null/0/1 for absence/
   invalid singleton/duplicate witness; MIME advice uses root path []. URLs
   are citations, not live verification.
8. Every logical rule gets passing/failing real SMTP/report fixtures. Auth pairs
   evaluate immutable trusted inputs on a real server through a private seam;
   injected Findings do not prove rule logic. Public fixtures prove unknowns
   despite spoofed headers. Preserve protocol parity, frozen guarantees,
   cache/worker tests and exact coverage. No dependency or routing change.

## Alternatives ruled and costs

The [dated adjudication](../../specs/bulk-sender-checklist/spec.md#panel-adjudication--2026-10-26)
records both positions. Adopt simplicity's one owner and narrow scope; choose
robustness's separate error-level unknown IDs over same-ID warnings. Pay more IDs
and no positive production qualification until genuine evidence arrives.
Choose per-mechanism facts over precombined booleans: pay a larger fixture matrix
while leaving #255 verification out of scope.

Choose explicit structural budgets/complete metadata inspection over raw size
plus potentially truncated parsing: pay boundary tests and unavailable analysis
for some accepted mail. Choose occurrence 1 over aggregate 0 for duplicates.
Keep the narrow syntax and presence-only scope: obsolete legal mail can be flagged
and plain bodies may be empty or not equivalent. No provider acceptance promise.

Synchronous acceptance, subscribe-before-scan, oldest-only pruning and ingest
extraction remain. No stronger durability, webhook delivery, retention or resource
isolation follows. Selected reads may finish after deletion; fresh ones cannot.
Four jobs still share the existing blocking pool/OS. Compatible implementation
may proceed under accepted 0045 §8; an accepted-ruling contradiction needs operator
disposition and frozen weakening is refused.

## Proof and disposition

[Tasks](../../specs/bulk-sender-checklist/tasks.md) pair every change with a proof;
AC1–AC10 and the [plan](../../specs/bulk-sender-checklist/plan.md) govern handoff
and complete local gates. The existing index row stays proposed. CI/operator
proofs remain theirs. This is a design proposal, not feature completion,
decision acceptance or publication authorization.
