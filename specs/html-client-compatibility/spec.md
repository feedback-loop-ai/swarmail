# HTML client compatibility

Date: 2026-10-26. Epic #251, issue #259. Design only; implementation follows.

Decision: 0090

[Proposed 0090](../../docs/decisions/0090-html-client-compatibility.md)
records these additive semantics through the shipped
[0085 report contract](../../docs/decisions/0085-deliverability-report.md).
Neither proposal is accepted by this design. The verified operator direction
requires lazy cached analysis of stored bytes, never SMTP analysis. Accepted
[0001](../../docs/decisions/0001-synchronous-insert-lossless.md),
[0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md),
[0003](../../docs/decisions/0003-oldest-only-pruning.md) and
[0005](../../docs/decisions/0005-extraction-at-ingest.md) remain intact.

## Feature and rationale

A program testing mail can inspect known HTML rendering risks, missing alt
attributes, image-only bodies, clipping risk and potential tracking pixels
through the existing deliverability report. One versioned embedded catalogue
makes the findings reproducible and reviewable offline. Empty findings mean
no enabled heuristic matched within its declared scope, never universal client
compatibility, successful delivery or absence of tracking.

Use existing Report/Finding/Location, Collector and Store-owned Service only.
No renderer, client-selector API, network lookup, new config, new dependency,
external scorer, persistent report, background scan or extra executor.
Stored raw/text/html/links/codes, persistence, fan-out and ordinary queries
remain unchanged. Authentication, later DNS policy and MCP upgrades are separate.

## Contract

### Catalogue and provenance

Embed `src/deliverability/html-rules.json` with existing serde/serde_json.
Aggregate `rule_set_id` is `swarmail-html/1`, including the unchanged
`demo.marker.v1` rule and its existing enablement/header behavior. The catalogue
owns this identity, all HTML IDs, severity, fixed hints, client scope, selector
kind/arguments and thresholds. Code implements a closed set of selector kinds;
changing a valid selector/threshold in test data changes evaluation without
changing Rust. No executable expressions, regex language or external files.

Every row carries a source title, URL, authored summary, basis type and basis
date. Initial rows are **Swarmail-authored heuristics dated 2026-10-26**, with
named reference guidance below; these are not downloaded upstream snapshots,
client-version measurements or claims of recent upstream verification. No
current client measurement snapshot was supplied locally. Do not invent one
or fetch one. Record that limitation in the tracked data and living docs.
No third-party dataset is copied; existing dependency/license policy stands.

Validate the immutable catalogue once on first lazy evaluation: closed shapes,
unique IDs, known kinds/severities, required nonempty hints/source/client labels,
valid selector arguments, positive size threshold and coherent dimensions.
Invalid catalogue data returns sanitized analysis failure, never an empty set.
Test the pure validator with local values, without mutable global fixtures.
Rule/data/parser/location/hint/order changes advance the aggregate identity.
A version-keyed expected exact catalogue snapshot in tests pins reviewed bytes; data
and its snapshot may only change together with the aggregate version after v1 ships.

The finite applicability matrix names Gmail web/mobile, classic Outlook for
Windows (Word renderer), and Apple Mail on macOS/iOS. The CSS rows target
classic Outlook; the video row targets Gmail and classic Outlook. General
accessibility/content/privacy rows apply to all three families. There is no
Apple-specific unsupported-feature row in v1; unlisted behavior is unassessed.
New Outlook is not classic Outlook. Findings are risk guidance, not a per-client
pass/fail certification.

| Exact ID | Severity | Data selector | Named reference |
|---|---|---|---|
| `html.css.position.v1` | warning | literal CSS property `position`, any nonempty value | Can I email, CSS position: https://www.caniemail.com/features/css-position/ |
| `html.css.display-flex.v1` | warning | literal `display` value `flex` or `inline-flex` | Can I email, CSS display:flex: https://www.caniemail.com/features/css-display-flex/ |
| `html.element.video.v1` | warning | active `video` start tag | Can I email, HTML video: https://www.caniemail.com/features/html-video/ |
| `html.img.alt-missing.v1` | warning | active `img` without an `alt` attribute | WHATWG HTML, alternative text: https://html.spec.whatwg.org/multipage/images.html#alt |
| `html.image-only.v1` | warning | active `img` present and no substantive text in this HTML part | Swarmail heuristic; Mailchimp Outlook guidance: https://mailchimp.com/help/my-email-looks-different-in-outlook/ |
| `html.gmail-clipping.v1` | warning | decoded HTML UTF-8 bytes `> 104448` | Mailchimp, Gmail is clipping my email: https://mailchimp.com/help/gmail-is-clipping-my-email/ |
| `html.tracking-pixel.v1` | info | remote HTTP(S) `img`, explicit integer width/height each in `[0,1]` | Swarmail heuristic; MDN Tracking pixel: https://developer.mozilla.org/en-US/docs/Glossary/Tracking_pixel |

Exact fixed hints, in the same row order:

1. `Positioned content may render poorly in classic Outlook for Windows; use a table-based fallback.`
2. `Flex layouts may render poorly in classic Outlook for Windows; use a table-based fallback.`
3. `Video may be removed in Gmail and classic Outlook for Windows; provide a linked image and text fallback.`
4. `Add descriptive alt text to informative images, or an empty alt attribute to decorative images.`
5. `Add meaningful HTML body text so the message remains useful when images are unavailable.`
6. `HTML exceeds the 102 KiB clipping-risk heuristic; shorten the HTML and test the result in Gmail.`
7. `A small remote image may be a tracking pixel; remove it or disclose tracking to recipients.`

Hints contain no sender HTML, URL, identifier or snippet. Source/client metadata
stays in the catalogue/docs; the public Finding shape gains no fields.

### MIME and source locations

Parse immutable raw bytes once per cold evaluation with existing mail-parser.
Walk outer-message multipart children in wire order with structural zero-based
child-index paths; root HTML has `[]`. Analyze every actual `text/html` body
leaf, including multiple alternatives, without synthesizing plain/HTML bodies.
Exclude any subtree marked `Content-Disposition: attachment`, any subtree
with a filename/name parameter (including inline filename-bearing parts), and
all attached `message/rfc822` bodies. Propagate exclusion to descendants before selecting eligible content; still
count their structural nodes for the part/depth budgets. Do not descend into
attached messages when counting outer-message nodes.
Inline HTML without a filename remains eligible. Text/plain and attached HTML
cannot generate HTML findings; a plain-text alternative does not cure its HTML
sibling's image-only rendering. This is analysis eligibility, not a change to
accepted stored-body/extraction selection.

All HTML findings use `Location::MimePart { path }`, one contribution per rule
per affected part regardless of occurrence count. Root demo remains its header
location. Finish through the existing sort/deduplicate owner. Preserve 256
pre-canonicalization contributions and 65,536 compact JSON bytes; overflow
fails the whole report. No unbounded vector of per-occurrence findings.

### Finite lexical model

One forward-moving HTML scan recognizes ASCII case-insensitive start/end tag
and attribute names, single/double-quoted and unquoted attribute values, and
source quotes protecting `>`. First occurrence of a duplicated attribute wins.
Decode attribute values and text runs once, after identifying source boundaries,
using the existing bounded semicolon-required entity decoder. Decoded `<` or
quotes never become structural delimiters. Invalid/unknown entities stay literal.

Comments (including hidden Outlook conditional comments), declarations and
processing instructions are inert. Script content is inert. Head/title/template
content does not count as body text or active images/elements; head style blocks
still contribute CSS checks. Template-contained styles are inert. Style
contents contribute CSS only. Nested
head/template suppression is tracked; valid closing tags restore normal mode.
Markup entry requires `<` followed by an ASCII letter, `/`, `!` or `?`.
Unrecognized literal `<` is text; recognized but unterminated tags, quoted
attributes, comments, raw script/style regions or suppression regions fail
analysis rather than hiding an unanalyzed remainder. Closed malformed constructs
have deterministic fixture-pinned lexical treatment; no browser tree repair.
Downlevel-revealed conditional content outside comments is ordinary markup.

Scan inline style attributes and style blocks as CSS declarations, not raw
substring searches. Respect CSS comments, quoted strings with escaped delimiters,
parentheses and braces; delimit property/value only in declaration context.
ASCII property/value matching ignores case, surrounding whitespace and a trailing
`!important`. `display` requires a single literal value token after that suffix.
Strings, comments, URLs/functions, selector text, ordinary text and attribute
values other than style cannot manufacture matches. Do not resolve cascade,
selectors, media applicability, variables, imports or CSS escaped identifiers.
Escaped identifiers are outside literal matching and do not trigger a rule;
unclosed lexical constructs fail. Rules inside syntactically recognized media
blocks are checked without deciding whether the media condition applies.

Visible-text approximation is a non-whitespace scalar in entity-decoded body
text outside suppressed regions; use Rust Unicode whitespace (including NBSP).
Alt/title attributes never count. No CSS visibility calculation: hidden text
can defeat image-only detection. Any active img counts, including a pixel; empty
HTML with no images does not warn. Present empty/whitespace/boolean alt passes
the missing-alt rule, reflecting an explicit decorative choice, not its quality.

Tracking dimensions use trimmed ASCII decimal digits only, no sign, fraction,
units or percent; leading zeros are allowed, numeric overflow is a nonmatch.
Both width and height attributes must decode to 0 or 1. Trim source whitespace
and compare HTTP(S) prefix case-insensitively after one entity decode; no URL
fetch or intent inference. CSS dimensions/visibility never override attributes.
Missing dimensions, larger images and cid/data/relative sources do not match.
A lone pixel may receive both image-only and tracking findings.

### Size, work and failure boundaries

Clipping uses the full eligible HTML part's UTF-8 byte length after MIME transfer
and charset decoding, before HTML entity decoding, including comments/styles.
Threshold is strictly greater than **104448 bytes (102 KiB)**. 104447 and 104448
pass; 104449 warns. This is Swarmail's deterministic interpretation of approximate
102 KB guidance, not Gmail's exact live boundary. Attachments, raw MIME/base64
size and character counts are different quantities. No total-message size rule
or SMTP rejection is introduced. A charset conversion can change this estimate.

Before MIME parsing, refuse raw input over 8,388,608 bytes (8 MiB). Limit parsed
outer-message parts to 256 including root and excluded subtree nodes; limit
MIME depth to 32 with root depth 0. Enforce limits during iterative traversal;
these post-parse checks do not claim to bound mail-parser's prior allocations.
Limit aggregate eligible decoded UTF-8 HTML to 4,194,304 bytes (4 MiB), checked
before HTML scanning. Bound lexical nesting (head/template and CSS brace/
parenthesis nesting) to 32, failing on the 33rd open level. Exact bounds pass
where other limits allow. Source cursors must progress; bounded entity probes,
constant catalogue size and no repeated suffix scans keep scanner work linear
in inspected bytes. No separate arbitrary token quota is needed.

These are workload limits, not RSS, fairness or wall-time guarantees. SMTP may
store larger messages under its existing cap; their reports can fail. Preserve
the demo's existing 65,536-byte root-header limit. MIME parser failure, detected
transfer-decode error on eligible HTML, or decoding that cannot yield a supported
text representation fails; never substitute empty HTML. Charset handling follows
the pinned mail-parser decoder, including its deterministic replacement behavior;
fixtures document that tolerated replacement is not a new lossless decode claim.
Any limit, malformed supported construct or catalogue failure yields existing
`AnalysisError::Failed`; publish no partial report, success cache or exact count.
Normal complete evaluation of the explicitly limited heuristic syntax is success;
unsupported rendering semantics are a declared gap, not a swallowed parse error.

### Service and adapter invariants

Preserve Weak object/config/version cache identity, 128 slots, four admitted
workers, cancellation ownership, ready-only zero-deadline reads and selected
snapshot semantics. No analysis in SMTP/insert/seed/restore/fan-out. An in-flight
selected read may finish after prune/delete; a fresh lookup cannot resurrect it.
All adapters share the same report and existing sanitized errors. REST/MCP
qualifiers retain inclusive severity, exact assertions including zero, minimum-one
waits and subscribe-before-scan with one deadline. Failure never qualifies a
message or silently removes it from an exact count. Retain current MCP JSON
text-block output and escaped UI hints; no protocol/schema changes.

## Panel adjudication — 2026-10-26

Read both run-local positions and the tracked [intake](intake.md); the legacy
run-local tasks directory is absent. Both supplied issue-packet checksums pass.
The following choices rule actual differences rather than averaging them:

| Issue | Ruling and losing-side cost |
|---|---|
| Architecture and scope | Adopt both members' existing collector/service and one immutable catalogue. Reject new framework, executor, dependencies and extractor rewrite. Pay a small analysis-local scanner rather than sharing a general parser with ingest. |
| Validation and hard limits | Adopt robustness's lazy catalogue validation, raw pre-parse cap, decoded-byte/part/depth bounds and failure-not-clean. Reject simplicity's stored-cap-only bound: some otherwise accepted large mail becomes unanalyzable. Reject robustness's extra token quota: bounded source and monotonic work suffice; retain deterministic work-counter proof. |
| Conditional comments and CSS escapes | Adopt simplicity's explicit limited literal model, while adopting robustness's string/comment/parenthesis awareness. Reject conditional-comment execution and escaped-identifier normalization. Known Outlook-only hidden markup and escaped declarations can be missed; docs and negative fixtures must expose that gap. No universal compatibility claim. |
| Clipping | Adopt robustness's decoded UTF-8 scope and 104448 threshold. Reject simplicity's pre-charset 102400 convention. Sizes 102401–104448 do not warn, and non-ASCII conversion can alter the estimate; simplicity's more conservative warning window is deliberately lost. |
| Pixels | Adopt simplicity's 0/1 integer HTML attributes, no CSS sizing. Reject robustness's positive-only/CSS dimension expansion: zero-sized beacons are included, CSS-only and hidden larger trackers are missed, and decorative small remote images can warn. |
| Image-only | Adopt simplicity's any-img rule, even for a beacon, and both members' empty-alt acceptance. Reject robustness's meaningful-image exclusion: a pixel-only body produces two actionable findings, without a second image classifier. |
| Provenance | Adopt named source guidance plus explicitly authored summaries. Reject any implied downloaded snapshot or current measured client matrix. The local packet lacks current empirical client results; v1 is a finite risk catalogue, not empirical certification. |
| Locations | Adopt both members' one rule/part finding. Reject per-occurrence offsets/new evidence fields. Consumers locate the part, then inspect its source to find each occurrence. |

## Acceptance Criteria

These are implementation obligations, proved locally unless explicitly assigned
below. Structural design validation does not prove the feature implemented.

1. **AC1 — Catalogue:** tracked embedded data contains all seven exact rows,
   version, selectors/thresholds, provenance and hints above. Pure validator
   negative cases fail; a changed selector/threshold changes outcomes. A test
   pairs every catalogue ID with tracked pass/fail fixtures and checks exact
   finding ID, severity, MIME path and hint. Cargo package listing includes
   data and fixtures. No new dependency or runtime network operation.
2. **AC2 — HTML/CSS:** positive inline/style-block/property/value/element cases
   and negative prose/comment/string/quoted-delimiter cases obey the lexical
   model. Mixed case, entities, !important, duplicated attributes, nested media,
   CSS functions and documented escape/conditional gaps have explicit witnesses.
   Truncated supported constructs fail with no partial successful result.
3. **AC3 — Images:** pairs and boundaries pin absent/empty/whitespace alt,
   body versus suppressed/entity whitespace, empty versus image-only HTML,
   plain-text alternative behavior, dimensions 0/1/2/overflow/missing/units,
   source schemes, entity-decoded schemes and CSS-only size exclusions.
4. **AC4 — MIME and clipping:** plain/attached/filename-bearing HTML is excluded;
   inline/nested/multiple HTML paths are stable. Base64, quoted-printable and
   non-UTF-8 charset fixtures prove decoded-byte size, with threshold −1/at/+1.
   Large size fixtures are generated from tracked recipes, not giant blobs.
5. **AC5 — Bounds and failure:** raw/decoded/part/depth/nesting limits have exact
   boundary witnesses; deterministic counters prove forward/bounded scanning
   on adversarial delimiters. Parse/decode/catalogue failures and 256-findings/
   65536-report-byte limits cannot return or cache partial success. A real
   report error makes exact assert/wait fail, never a partial count or clean mail.
6. **AC6 — Shared behavior:** real SMTP -> REST/MCP/UI and existing stdio tests
   show identical HTML findings, current text-block contract and escaped hints;
   inclusive severity, exact-zero asserts and minimum-one waits remain correct.
   Cold/warm/restart JSON is identical; equal keys reuse results and changed
   version/config cannot reuse old results.
7. **AC7 — Ownership:** retain real-protocol and owner-local proofs of immediate
   queryability after 250, unchanged raw/extracted fields, zero analysis on
   ingest/seed/restore, delivery during qualification, cancellation and prune/
   delete snapshots. Preserve four jobs/128 slots and subscribe-before-scan.
   Tests use per-owner seams/barriers, never sleep as synchronization.
8. **AC8 — Delivery:** living docs expose this version, applicability, sources,
   gaps and error boundaries; index 0090 as proposed during implementation.
   Preserve recipe routing and accepted decisions. Run all local house gates
   using the candidate-local pinned engine, exact coverage and its selftest,
   plus structural design/publish/seal proofs and native-tls absence. Commit
   unsigned, never push. CI owns macOS, network RustSec, Helm/chart/history,
   Docker, privileged port-80 and tag-release proofs; the operator owns live
   settings, acceptance, sealing and publication. None is claimed locally.
