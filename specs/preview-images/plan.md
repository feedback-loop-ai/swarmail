# Plan — preview images

[Spec](spec.md) · [Tasks](tasks.md) · [Proposed 0135](../../docs/decisions/0135-preview-images.md)

## Technical context and research

Rust/axum, existing mail-parser/base64/percent-encoding and SHA-256 helper;
embedded plain JavaScript/CSS. No package or license widening. Local origin/main
at launch is 0449622dc7490b32106bfcaef37bea89982df967; highest decision 0134.
0135 is next free. This is a local-ref observation, not a network fetch.

Read both panel positions, intake, decision index, 0005/0031/0043/0110/0134,
network posture and relevant UI/attachment code. src/ui.rs confirms missing
frame-src and blanket DENY. Attachment lacks CID/disposition; ingest records
candidate ordinals, at most 250 entries and 1 MiB retained content. Read-time
identity lookup needs the explicit 0110 exception. No browser runner exists
in house gates; Rust source assertions cannot prove browser enforcement.

.specify/templates contains only spec-template.md, no plan template. Required
research, data model, contracts and quickstart are provided inline below.

## Constitution check

0001–0003: no mutation, acknowledgement, wait or pruning change. A held Arc
is a snapshot, not an eviction reservation. 0005/0031: display only; stored
links/codes/ICS stay ingest-owned. 0110/0134: explicit proposed exceptions,
merge held for acceptance. 0043: unchanged whole-plane guard, no CORS. Issue
#56: textContent and two bare sandboxes. 0020: stable unit tests inline;
concurrency cold paths in declared cfg(test) sibling harness. No sleeps as
synchronization, mid-band chaos or expiring certificate fixtures. Exact
coverage and dependency policy stand.

## Project structure

| Files | Responsibility |
|---|---|
| src/ui.rs | Preview router outside parent DENY; controlled-response headers; proposed parent frame-src addition; legacy src/notice. |
| src/ui/preview.rs (new, declared by ui.rs) | UI-private bounded renderer, CID matching, source-span transformation, constants and stable unit tests. |
| src/api.rs and service/router construction owning AppState | One two-permit Arc semaphore per service; retain guard ordering. |
| src/static/ui.js | Selection-owned frame/consent, trusted warning, refresh preservation. |
| src/static/ui.css | Only needed button/note layout; no external assets. |
| tests/ui.rs, tests/api.rs | SMTP/HTTP route, CSP, guard, console and OpenAPI regression proofs. |
| src/ui/preview_tests.rs if cold paths require it | Declared cfg(test) harness for cancellation, prune race and worker failures. |
| src/static/openapi.json, src/static/llms.txt | Preview contract and separation from projection/tools. |
| docs/guide/threads-and-ui.md, network-posture.md, attachments.md | Limits, consent and browser exposure. |
| docs/releases/v0.5.0.md | Unreleased feature/privacy note. |
| docs/decisions/README.md, 0110-attachments-and-ics.md, 0134-ui-projection-route.md | Proposed index entry and dated pointers only. |

The design commit touches exactly three spec files and new 0135. The index
and old-record pointers are implementation tasks to honor the exact-file
instruction. Dated amendment text is already in 0135. Accepted bodies stay.

## Implementation approach and data model

1. Shared closed-query parsing, deliberate GET/HEAD behavior. Try-acquire an
   owned permit, read one Arc<Email>, move both into spawn_blocking. Joining
   a failed worker returns fixed 500 with preview headers, without mail text.
   Request cancellation cannot release a permit owned by the worker.
2. Check HTML/raw sizes before respective parsing. Build temporary CID index
   from outer parts; count duplicate identities even among ineligible parts.
   Reproduce ingest candidate ordinal accounting without extract/ICS calls.
   Validate bounded eligible candidate type/size/hash against retained bytes.
   Never guess filenames or emit newly decoded raw bytes. Borrow retained
   content; destroy lookup data at completion. No new persisted/public model.
3. Use context-aware source spans. Inspect extract.rs tokenizer/entity logic
   for pure lexical primitives only; never call link/code extraction. Any
   factoring requires unchanged extraction-output proofs. Track comments,
   raw text, quotes and foreign content. Decode entities for URI interpretation,
   never re-tokenize decoded text as markup. Generated URIs contain fixed
   allowed MIME types and base64, safely quoted. Duplicate src attributes
   are not rewritten. If malformed syntax prevents reliable navigation
   suppression, return the fixed inert fallback.
4. Remove refresh/base/hints and neutralize anchor navigation in the same
   pass. Preserve stylesheet links subject to CSP. CSP/sandbox enforce
   scripts/forms/objects/frames; source deletion is not the security boundary.
   Malformed/foreign-content browser fixtures must prove no automatic escape.
5. Charge each inserted URI before encoding, checked arithmetic throughout.
   Fixed inert documents and outside notices avoid another metadata endpoint
   or attacker-controlled console diagnostics. No cache.
6. Route-specific response layer covers successes, errors and method fallback
   inside the existing plane guard. Preview SAMEORIGIN/frame-ancestors self
   is separate from parent DENY. Parent adds only frame-src self, explicitly
   proposed in 0135 rather than silently treating unchanged-CSP as satisfied.
7. Console owns current-selection generation, one frame and consent. Set
   sandbox before src; async completions check generation. Hide/reuse frame
   across tabs and enrichment, reset on selection change. Legacy uses default
   route with notice and no remote button.

## Contracts

AC1–AC7 pin the wire/data contract. Controlled errors follow existing JSON
error conventions: 400 names bad query, 404 message not found, 405 unsupported
method, 413 names preview byte budget, 503 preview busy; retry, 500 preview
rendering failure. Content-Type matches actual body; headers apply to all.
HEAD strips body. Raw/part-limit omission is partial 200; source/output limit
is 413; no HTML/unsafe transformation is fixed inert 200. No mail text is
interpolated into errors.

## Synthesis rulings and costs

Initial design: no review-return findings were provided. AC2 answers both
panels' CSP defect; AC3 answers their missing-identity/authority defect.
Literal unchanged parent policy is impossible with iframe src because
frame-src falls back to default-src none. The proposal corrects that
criterion on evidence; it grants no implementation or merge waiver.

Robustness wins admission, smaller input bounds and navigation suppression;
accept extra permit tests and unavailable previews for large mail. Simplicity
wins fixed outside notes, img src only, no dependencies/persistent fields and
one inserted-byte aggregate cap. Accept less fidelity and no per-image
omission attribution. Both agree SVG is refused. Reject simplicity's no-module
preference: a UI-private transform module separates bounded rendering from
shell construction, with every compiled line still measured.

Reject adding a browser framework/toolchain dependency here. Use an already
available browser for a reproducible manual proof with controlled local
fixtures and request logs. If unavailable, record proof outstanding and arrange
it before feature verification; static assertions never substitute for it.

## Risks

- Authority: 0135 explicitly proposes parent-CSP and 0110 identity exceptions.
  Hold merge for operator acceptance; accepted text is not rewritten.
- Parser differential: browser parsing can disagree with a small scanner.
  Explicit contexts, conservative fallback and malformed/entity/foreign-content
  fixtures plus browser evidence are mandatory; do not expand contexts silently.
- Resources: two jobs and input/output caps bound work, not all allocations,
  HTTP buffers or process memory. Compressed byte caps do not bound raster
  pixel memory. No constant SMTP-latency claim.
- Cancellation: spawn_blocking outlives callers; owned permit stays until
  actual completion, proved with deterministic barriers.
- Identity drift: ordinal/type/length/hash checks fail closed across parser
  upgrades. Old/unavailable parts stay unavailable rather than invented.
- Privacy: remote opt-in exposes browser IP/credentials under browser policy,
  follows redirects and may reach private hosts. No server address filter is
  implied; mixed content/font CORS may still refuse loads. Warn and document.
- Evidence: browser version, fixture and observed request logs must be recorded;
  no successful browser claim without actual execution.
- Number collision: integration rechecks 0135 and updates all links together
  if assigned a different number.

## Validation and quickstart

Build candidate; start isolated SMTP/HTTP using existing test rig. Send
multipart/related mail with unique CID/retained PNG and remote image, CSS and
font from a second local origin. Open Preview; confirm visible inline pixels
and zero second-origin requests. Click warning button; observe image/CSS/font
requests. Switch away/back; confirm off. Run malicious fixture in both modes:
refresh/base/hints, scripts/handlers/forms, anchors, nested frames and parent
access attempts. Use DevTools/network logs and controlled request counters,
not internet services or sleep-based validity assertions. Open preview URL
directly to verify response sandbox; test third-origin framing refusals.

Each task pairs a proving test. Run the entire
[house gate inventory](../../docs/house-rules.md#gates--part-of-every-change),
including locked root/evals checks against candidate binary, every recipe,
coverage/selftest, publish/seal proofs, licenses, native-tls absence and
CI-owned checks. This design-only draft claims no implementation gates or
browser proof.
