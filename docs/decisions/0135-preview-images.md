# 0135 — Bounded inline images and opt-in remote preview content

- **Status:** proposed
- **Date:** 2026-11-17
- **Scope:** Issue #374.
- **Specification:** [preview images](../../specs/preview-images/spec.md),
  [plan](../../specs/preview-images/plan.md),
  [tasks](../../specs/preview-images/tasks.md).
- **Amends on acceptance:** [0110](0110-attachments-and-ics.md) §§2/7 and
  [0134](0134-ui-projection-route.md) §3, by dated addenda below.
- **Authority:** operator direction of 2026-11-17 to build a same-origin
  inline-image preview and explicit remote toggle against a proposed record,
  acceptance before merge. That direction does not accept this record or
  the additional parent-CSP/identity reconciliation below.
- **Preserves:** 0001–0003; ingest extraction under
  [0005](0005-extraction-at-ingest.md); display/storage distinction of
  [0031](0031-sender-honest-stored-bodies.md); whole-plane
  [0043](0043-mcp-origin-boundary.md). No SMTP or persistence work.
- **Number:** next free on local origin/main
  0449622dc7490b32106bfcaef37bea89982df967 at launch, highest 0134.
  Integration checks collisions again.
- **Hold:** operator acceptance before merge; never self-accept.

## Context and proposed ruling

srcdoc inherits console CSP and cannot resolve CIDs. Stored attachment
metadata lacks CID/disposition and no route serves retained bytes for display.

1. Add GET /ui/message/{id}/preview, HEAD with identical status/headers and
   no body. Closed query: absent or one remote=1. Render stored HTML only;
   no new extraction answer, stored write, attachment endpoint, raw fallback,
   server fetch or persistent consent.
2. Permit bounded raw-MIME parsing solely for CID/disposition/candidate
   ordinal association to retained parts. Validate normalized type, size and
   SHA-256 against retained content; emit only retained bytes. No ICS/link/
   code extraction or attachments::extract call. Ambiguity, legacy/unavailable
   bytes and mismatches fail closed. This explicitly excepts 0110 §2; it is
   not a claim the existing no-reparse rule already permitted it.
3. Only referenced inline/absent-disposition PNG/JPEG/GIF/WebP in HTML img src
   qualify. Refuse SVG and CSS/srcset CID substitution. IDs compare exactly
   after MIME whitespace/bracket normalization and strict once-only URL
   decoding; duplicate IDs are ambiguous. Spec AC3–5 pins complete rules.
4. Bounds: two fail-fast jobs per service owned through blocking-worker exit;
   8 MiB raw lookup input, 2 MiB source HTML, 256 KiB decoded image, 2 MiB
   inserted data URIs charged each occurrence, 4 MiB final output. No waiting
   job queue. Above raw cap skip identity lookup; above source/output cap
   return 413; busy is 503. One snapshot; no render-time store lock.
5. Preview has its own HTTP CSP, default-none, data-only images and inline
   styles. remote=1 widens only images/styles/fonts to HTTP/HTTPS, retaining
   image data and inline styles. Always bare CSP sandbox, script/object/frame
   refusal, base-uri none, form-action none, frame-ancestors self; nosniff,
   no-referrer, no-store and SAMEORIGIN. Bare iframe sandbox stays. Suppress
   refresh/base/resource hints and neutralize anchor/area navigation in the
   display transform. Preserve guard/no-CORS. Spec AC2 states exact policies.
6. **Parent-CSP correction requiring acceptance:** add only frame-src 'self'
   to the console/legacy policy. Existing default-src none otherwise blocks
   iframe src; the literal unchanged-policy requirement cannot coexist with
   a working separate route. Preserve every existing directive and external
   resource prohibition. Never permit remote frames or add sandbox tokens.
   Keep parent DENY; preview alone admits same-origin framing. Acceptance
   of this record rules this explicit correction.
7. Default off per selection; trusted “Load remote content” button warns
   “Loading contacts the sender's hosts.” Disable after click, reset on
   selection change, reject stale responses and reuse frame across tabs/
   enrichment. Legacy uses off-only route. Outside notice names every omission
   class/numeric bound. Requests originate in viewer's browser, may carry
   credentials, follow redirects and reach private hosts; no public-address-
   only promise. Mixed content/font CORS may still refuse loads.

## Dated addenda carried by this record — 2026-11-17

These amendments take effect only on operator acceptance. Accepted source
bodies and statuses stay unchanged.

- **To 0110 §2:** retain stored-byte-only list/fetch and ingest extraction.
  Permit solely the bounded preview MIME identity/disposition lookup to
  validate association to retained image bytes. Never serve freshly decoded
  raw bytes, recover unstored content, re-extract answers or write back.
  Parser drift refuses display instead of altering stored truth.
- **To 0110 §7:** beside metadata projection, one read-only UI preview may
  carry bounded referenced inline raster bytes as data URIs. No generic REST
  attachment list/fetch is authorized. Existing MCP, compact/full Email
  serialization and serde(skip) remain unchanged.
- **To 0134 §3:** projection still serves no bytes/hashes and keeps its exact
  shape. “Attachment bytes stay MCP-only” gains only this preview exception;
  no widening of projection or other surfaces.

Implementation appends dated proposed pointers to both amended records and
adds the proposed index row. Do not rewrite accepted paragraphs or accept
proposals. This design commit contains only the three requested spec files
and this record; amendment text is present here now.

## Alternatives, costs and proof

Adopt both panels' stored-byte-only, raster-only single-route approach.
Reject ingest CID fields (no-SMTP-work constraint), filename guesses and raw
recovery (invent availability). Cost: old, unstored or mismatched images fail.

Robustness's render cap/smaller bounds defeat simplicity's unlimited parallel
reads: accept busy/large-preview refusals. Simplicity's trusted generic note
and inserted-byte budget defeat per-reference diagnostics and redundant
aggregate decoded accounting: accept less attribution. Reject SVG and broader
CID grammar to bound implementation/proof surface.

Require SMTP/HTTP contract tests, exact-cap and cancellation proofs, browser
request/pixel/isolation evidence and full house gates. String tests cannot
prove a working frame or browser isolation. Parsing, raster pixel decode and
slow-client buffers still cost resources; two jobs is not a global memory or
SMTP-latency guarantee. A committed proposal is not feature verification.

## Implementation note — 2026-11-17 (delivery run, still proposed)

The implementation follows §§1–7 and records these readings. None widens a
ruling above; each is narrower or equal.

- **Removal is a superset.** Every `<meta>` is removed, not only refresh,
  because the response header carries the charset. Every `<link>` is removed
  unless all its `rel` tokens are `stylesheet` or `alternate` and one is
  `stylesheet`. `ping`, `referrerpolicy` (a per-element override of
  `no-referrer`) and SMIL's `attributeName` are dropped from every element,
  and `href`/`xlink:href` from `a` and `area`. A removed tag leaves `<!---->`,
  so the text on either side cannot join into a new tag.
- **Parser differential.** Any `<a`, `<area`, `<base`, `<link`, `<meta`,
  `<set` or `<animate` opener that the scan does not read as a start tag —
  inside a comment, raw text, CDATA or an attribute value — is written as
  `&lt;name`. That text means the same in each of those contexts and can
  never open a tag. A browser that tokenizes the source differently from the
  scan therefore still meets no such tag the scan did not see. `noscript` is
  markup, as in a document without scripts.
- **Fallback trigger.** The fixed inert document answers when a tag is still
  open at the end of the source. The unprocessed HTML is never served.
- **Admission.** One two-permit semaphore per `ui::router()` (one per
  service), taken with `try_acquire_owned` before the snapshot. The blocking
  closure owns the permit and drops it after the render returns.
- **Console.** The preview frame lives in its own slot and is hidden, not
  removed, on another tab, so tab changes and enrichment never reload it.
  Consent (`remoteFor`) resets whenever the selection changes. The
  standalone `/ui/message/{id}` page frames the default route only.
- **Browser proof outstanding.** The delivery box has no runnable browser
  (the `firefox` binary there is a snap stub, and there is no Chromium or
  network to install one). The browser evidence that AC10 and task T8
  require has not been produced, and header and string tests do not
  substitute for it. [The procedure](../../specs/preview-images/browser-proof.md)
  lists what to run on a machine with a browser before verification.

