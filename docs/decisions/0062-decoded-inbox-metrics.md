# 0062 — Decoded inbox metric identity

- **Status:** proposed — acceptance is the operator's.
- **Date:** 2026-10-24
- **Issues:** Epic #192, issue #202.
- **Amends:** [0043 §6](0043-mcp-origin-boundary.md), only its preservation of raw-path versus decoded-inbox metric keys.
- **Preserves:** [0047 §2](0047-storage-rpc.md) cap/overflow, [0046](0046-smtp-pop3-edges.md) verbatim admitted identities, and frozen guarantees 0001–0003 plus ingest extraction under 0005.
- **Spec:** [Decoded inbox metrics](../../specs/decoded-inbox-metrics/spec.md).

0062 is the next sequential number after the highest record present, 0061;
earlier numbering gaps are left untouched.

## Context

The HTTP request middleware keys its gauge from a raw selected URI segment.
The await handler uses Axum's decoded `Path<String>` for both await metrics.
Thus a parked space/Unicode inbox has inconsistent identity across related
rows. 0043 intentionally preserved this distinction, so changing it needs
an explicit amendment rather than silently treating it as a settled bug fix.

## Proposed ruling

1. Keep existing path selection and middleware scope. Percent-decode only
   the selected inbox segment, exactly once, with strict UTF-8 conversion.
   A successful conversion is the HTTP request gauge's key on every
   currently recognized inbox-scoped route. Preserve Unicode, case, padding,
   literal plus and malformed-percent treatment; encoded slash is data.
2. On invalid UTF-8, use the complete original segment unchanged for request
   accounting. Leave Axum extraction, refusal status/body, validation and
   no-await-registration behavior unchanged. Do not use lossy decoding or
   add a rejection policy in metrics code.
3. Preserve middleware order and RAII lifetimes, total ownership including
   refusals/scrapes, family names and escaping, MCP identities and 0047's
   cumulative cap/reserved LF overflow. Do not add compatibility alias rows,
   counters, configuration, dependencies or registry resets.
4. Valid wire aliases intentionally coalesce under the decoded key. Consumers
   polling an encoded HTTP label must use the decoded selector. Invalid-byte
   fallback remains potentially ambiguous with a valid literal-percent name:
   `%FF` refused and `%25FF` admitted can both account under `%FF`. Metrics
   indicate request activity, not successful validation or a commit receipt.
5. Under [0045 §8](0045-delivery-repairs.md), runtime implementation that
   contradicts accepted 0043 waits for the operator to accept this proposal.
   Design checks and the issue request do not constitute acceptance. Upon
   acceptance, this narrowly supersedes the historical HTTP-plane spec and
   plan's raw-key preservation; retain historical text with dated pointers.

## Alternatives, panel ruling and costs

Adopt both panels' local existing-helper design, including simplicity's
existing-file proof and robustness's conditional process-isolation fallback.
There is no runtime disagreement to average. Choose the smaller architecture;
require explicit raw-fallback collision documentation and stable, parallel-safe
proof obligations. Use exclusive names with demonstrated cap headroom, falling
back to process isolation only when necessary. This costs a fixture audit and
bounded task ownership rather than new production code.

Reject lossy decoding (invented identities), whole-path/repeated decoding
(data becomes structure or a different name), render-only decoding (duplicate
samples), dual labels (ambiguous/double attribution), and handler-owned
accounting (missed refusals and changed lifetime). Accept the selector
migration and linear decoding/allocation cost. Broader delivery failure
analysis does not justify storage, SMTP, webhook, chaos or TLS changes.

## Proof and handoff

The linked spec's AC1–AC6 are normative for implementation: exact helper
fixtures; two real concurrent encoded space/Unicode awaits; one parsed
scrape with all three decoded rows and no encoded aliases; successful seed/
join and gauge cleanup with cumulative retention; unchanged invalid-UTF-8
refusal; regression sensitivity to restoring the old helper; full gates.
Separate metric families do not gain an atomic-snapshot guarantee. Stable
parked requests make the assertion meaningful. Preserve subscribe-before-scan.

This design commits only the three owning spec files and this proposal.
A later documentation task indexes the record with its actual status and,
after acceptance, adds scoped historical pointers and selector guidance.
No acceptance, implementation or live refusal reproduction is claimed here.
