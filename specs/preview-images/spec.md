# Inline and opt-in remote preview images

Issue #374 · Design synthesis · 2026-11-17

Decision: 0135 — [Sandboxed preview images](../../docs/decisions/0135-preview-images.md) (**proposed**).

## Feature and rationale

Display retained inline raster images in a separate sandboxed preview document.
Remote images, stylesheets and fonts start blocked; a deliberate action for the
current message permits those browser requests. This makes real mail readable
without turning selection into a tracking event. No attachment download route,
proxy, cache, persistent consent, dependency, schema or SMTP-path work is added.

## Authority and explicit synthesis

Both panel positions identify two conflicts. The existing parent CSP has no
frame-src/child-src, so default-src 'none' blocks the requested iframe src.
A child policy cannot override that. Propose adding only frame-src 'self' to
the parent, preserving every existing directive and the prohibition on external
console resources. The intake's literal unchanged-CSP criterion cannot coexist
with a working route-backed frame. This correction requires acceptance of
0135; this draft does not pretend the operator already ruled on it.

Stored attachments have neither CID nor disposition. Propose a bounded,
preview-only MIME identity lookup, emitting only retained bytes. This needs
an explicit exception to 0110 §2 as well as §7 and 0134 §3.
[0031 §3](../../docs/decisions/0031-sender-honest-stored-bodies.md) distinguishes
display from extraction; it does not override 0110.
[0005](../../docs/decisions/0005-extraction-at-ingest.md) remains authoritative:
no link/code/ICS answer is derived or persisted. Build against the proposal as
requested; hold merge for operator acceptance, including both conflicts.

Adopt simplicity's single route, retained-byte-only rendering, raster-only
allowlist, limited CID context, fixed outside notice and ephemeral consent.
Reject its unlimited parallel rendering and 50 MiB preview parse: robustness
wins with two jobs and smaller preview bounds. Cost: busy/size refusals for
otherwise valid stored mail. Adopt robustness's cancellation-safe admission,
snapshot and navigation suppression. Reject its redundant aggregate decoded
budget and per-reference diagnostic subsystem: inserted-byte accounting bounds
expansion, and hostile mail CSS can hide in-frame diagnostics. Cost: generic
omission explanations rather than per-image attribution. Reject SVG and CSS/
srcset CID expansion for this iteration; the fidelity limitation is visible.

## Acceptance Criteria

1. **AC1 — Route.** GET /ui/message/{id}/preview returns text/html;
   charset=utf-8 from one stored Email snapshot. Query is closed: absent or
   exactly one remote=1; malformed encoding, empty/other values, duplicate
   remote and unknown keys return named 400. HEAD has GET status/headers,
   including render/admission semantics, without body. Other methods return
   405; absent/pruned ids return existing 404 message-not-found. No HTML part
   returns a fixed inert 200 document. No CORS or server fetch is added.

2. **AC2 — Headers and framing.** Every preview success and controlled error
   has Cache-Control: no-store, X-Content-Type-Options: nosniff,
   Referrer-Policy: no-referrer, X-Frame-Options: SAMEORIGIN and HTTP CSP:
   `default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'; sandbox`.
   Remote mode changes only img-src to `data: https: http:`, style-src to
   `'unsafe-inline' https: http:` and adds `font-src https: http:`. Sandbox
   has no tokens. Guard refusals preserve their existing envelope. Parent
   policy gains only frame-src 'self', under the authority hold above.
   Console and legacy parents retain DENY; no other route gains a framing
   exception. Browser proof demonstrates the frame actually loads.

3. **AC3 — Stored identity.** Render Email.html, not a newly selected MIME
   body. Bounded raw parsing identifies outer-message leaf candidates in
   ingest order (including calendar candidates for ordinal accounting).
   Never descend into attached messages. Match stored candidate ordinal and
   validate normalized type, decoded size and SHA-256 against both candidate
   and retained content; emit only retained content. Never call
   attachments::extract, scan ICS, derive links/codes, write back or recover
   unavailable raw bytes. Unknown/labels-only lists, unstored/missing bytes,
   truncation, parse failure and identity mismatch leave CIDs unresolved.
   Restart preserves results; stored HTML/raw/extraction remain unchanged.

4. **AC4 — CID policy.** Allow image/png, image/jpeg, image/gif, image/webp
   only, ignoring parsed MIME parameters. Refuse SVG and explicit attachment
   disposition. Inline/absent disposition with CID qualifies. Trim MIME outer
   whitespace and one angle-bracket pair; reject empty IDs/controls. Recognize
   cid scheme ASCII-insensitively, HTML-decode attributes, strictly percent-
   decode identifier once as UTF-8, preserve plus, compare case-sensitively.
   Duplicate normalized IDs across all outer parts, including disallowed
   parts, make the ID ambiguous. No filename/Content-Location fallback.

5. **AC5 — Context and isolation.** Substitute only HTML img[src], supporting
   quoted/unquoted values and character references. No rewriting of comments,
   text, raw-text elements, SVG/MathML, href, srcset or CSS url values.
   Unsupported CID contexts stay broken. Use context-aware tokenization,
   not regex/global replacement. Remove base elements, meta refresh and
   resource hints (prefetch, preload, modulepreload, preconnect, dns-prefetch,
   prerender); neutralize anchor/area href and ping to prevent clicked frame
   navigation. Keep ordinary stylesheet links subject to CSP. Scripts,
   handlers, script URLs, forms, objects, nested frames, popups and top
   navigation remain refused in both modes. Unsafe parsing uncertainty yields
   a fixed inert document, never an unprocessed fallback. This is limited
   display transformation backed by CSP/sandbox, not a general sanitizer.

6. **AC6 — Bounds.** Inclusive constants: 8 MiB raw MIME lookup input,
   2 MiB UTF-8 source HTML, 256 KiB decoded bytes/image, 2 MiB inserted data-URI
   bytes/document, 4 MiB final HTML. Charge prefix plus base64 at every
   occurrence before allocation; repeated references consume budget. First-fit
   in source order, continuing after omissions; checked arithmetic. Above raw
   cap, skip lookup but display bounded HTML with unresolved CIDs. Above HTML
   or final-output cap return named 413; never parse a truncated prefix.
   Over-budget images remain unchanged. Unsafe/unparseable transformation
   returns fixed inert 200 saying preview unavailable. Fixed diagnostics count
   toward final output; no attacker-controlled diagnostics are interpolated.

7. **AC7 — Admission/races.** Two preview permits per service, shared across
   router clones, separate from ingest. Acquire without waiting before
   snapshot/worker submission; exhaustion returns 503 `preview busy; retry`.
   Blocking closure owns its permit through actual exit, even if caller
   disconnects. Render without store/mutation locks. Snapshot before prune
   may finish; lookup after prune returns 404. No eviction reservation/waiter.
   The cap bounds jobs, not all HTTP buffers or process memory.

8. **AC8 — Trusted UI.** Console and legacy use iframe src, never srcdoc,
   with bare sandbox set before src/attachment. Mail HTML never reaches the
   parent DOM; console mail text stays textContent-only. Always-visible
   trusted outside notice names missing/ambiguous IDs, unsupported type
   (including SVG), disposition/context, unavailable/mismatched stored bytes,
   raw lookup cap and byte caps with numeric values. Generic explanation is
   sufficient, but must not imply all images displayed. Legacy stays off-only.

9. **AC9 — Consent.** Button says “Load remote content”; adjacent warning
   says “Loading contacts the sender's hosts.” Explain viewer IP/credentials
   exposure and local/private host reachability. Click changes only selected
   frame to remote=1 and disables button. Selection change, including return
   to an earlier message, resets off. Preserve frame/consent across same-
   selection tab changes; enrichment/SSE must not reload it. Stale asynchronous
   results cannot transfer consent. No persistent preference.

10. **AC10 — Proof/boundary.** Real SMTP/HTTP tests prove replacements and
    byte equality, complete headers, refusals, no unreferenced/non-image bytes
    and unchanged projection/MCP results. Preserve 0043 guard ordering on all
    methods: hostile Host/Origin/null refuse, allowed/no-Origin reads pass,
    off-loopback authority rules unchanged. No opaque-origin bypass. Preserve
    console loads-nothing-from-outside test. Browser evidence with controlled
    origins proves visible CID pixels, zero default external loads, opt-in
    image/CSS/font loads, no script/form/automatic-navigation effects, framing
    boundaries and consent lifecycle. String/header tests alone do not prove
    browser behavior.

11. **AC11 — Delivery.** OpenAPI documents route, HEAD/query/refusals/limits.
    Update UI/network/attachment guides, llms.txt and v0.5.0 Unreleased notes.
    State browser networking, redirects/private hosts/credentials and mixed
    content/font-CORS limitations, no server proxy. During implementation add
    proposed index row and dated pointers to 0110/0134; accepted bodies stay.
    All house gates and CI-owned proofs remain required with exact coverage.
    No frozen guarantee, SMTP extraction, schema or dependencies change.

## Draft scope

This is a design, not implemented or verified behavior. Browser proof and
house gates remain delivery tasks; 0135 remains proposed until acceptance.
