# Extraction and compat repairs

Date: 2026-10-18. Epic #120, batch 4; issues #136, #137, #138, #139.
This is a design, not an implementation or gate receipt. Semantic authority
is proposed [0049](../../docs/decisions/0049-extraction-compat.md);
acceptance remains the operator's.

## Resume confirmation — 2026-10-18

Run `resume-point-read-first-epic-120-2a606e73` confirms this existing
`extraction-compat` design. Preserve signed design `4801165`, implementation
`d350d5b` and supplied-data commit `2987ee2` on the checked-out
`epic120-extraction-compat` branch. Add signed commits; do not recut the
branch, rewrite history or repeat completed implementation work.

Decision: 0049 — the existing [proposed decision](../../docs/decisions/0049-extraction-compat.md)
continues to cover this feature. This confirmation adds proof detail, not
new semantics, so no new decision is drafted. Both 0040 and 0049 are still
proposed; the panel's description of 0040 as accepted is rejected against
the decision file and index. Only the operator can accept them.

The remaining implementation scope is T02 (#136 AC2), the two dependent
T03/AC1 rows, and their provenance/status reconciliation and final-tree
verification. T01 and T04–T09 remain the prior implementation's completed
work. Its gate receipts are historical evidence, not receipts for the
forthcoming full table. The task directory contains historical run-12
framing only; the current intake result and explicit resume commission
govern this confirmation. The batch-3 resume framing is unrelated.

The operator-supplied, unmodified WHATWG snapshot is
`scripts/data/html-entities.json`, fetched 2026-10-18 from
`https://html.spec.whatwg.org/entities.json`, SHA-256
`d741d877ac77c4194c4ad526b5b4a19aef8dfe411ab840a466891cdbb9f362e6`.
Design inspection verified the digest, 2,231 keys, 2,125 semicolon-ended
names, 106 legacy aliases, codepoint/character agreement and matching
semicolon counterparts. This resolves the acquisition blocker; it does not
make the current 71-name Rust table complete.

### Resume panel rulings

| Question | Ruling | Cost or rejected claim |
|---|---|---|
| Scope and ownership | Adopt both positions' T02 completion through the existing generator, static table and bounded decoder. | No parser redesign, substitute data or isolated hand additions of eacute/rarr. Historical stored extraction remains verbatim. |
| Generator integrity | Adopt robustness's standard-library SHA-256 refusal before generation, with ASCII, unique-name, count and scalar checks. | Slightly more maintenance tooling than simplicity's unchanged-generator path; deliberate future snapshot updates must update the pin. No Rust hashing dependency. |
| Independent proof | Adopt simplicity's every-mapping lookup/decode proof and robustness's codepoint-derived whole-table equality, checked against JSON characters. | Extra inline measured test code and a test-only embedded snapshot; no Python during Rust tests. Table-self tests alone are insufficient. |
| Additional bounds | Adopt robustness's full-table no-ASCII-digit assertion and all-106 legacy cases, retaining exact window/work/expansion pins. | These are preservation checks, not new baseline defects. |
| Snapshot identity | Reject robustness's byte-length pin as an integrity guarantee; use the actual digest at regeneration and explicit digest verification in delivery. | Cargo tests alone do not authenticate snapshot bytes; source equality also cannot detect coordinated edits of both inputs. No claim otherwise. |
| Legacy syntax | Adopt both positions' semicolon-required contract. Reject robustness's claim that it matches browsers on every relevant link case. | HTML permits context-dependent bare matches, including some attribute cases. Swarmail deliberately leaves all bare forms literal; full browser recovery is outside this resume. |
| Packaged tests | Adopt simplicity's exact-file Cargo include for the JSON fixture. | The package grows by the snapshot's size; no broad scripts inclusion, duplicate fixture or build-time fetching. |

## Framing and rationale

Original design baseline: main `067ab91`, which contains batches 1–3 and descends from the
review baseline `2a325c9`. Read evidence: both run-local panel positions,
the complete #120 and #136–#139 issue snapshots including their embedded
review evidence, and `.brokkr/state/tasks/`. That directory contains only
historical run-12 tasks; they do not replace this commission's framing.

At that baseline, encoded quotes truncate attribute URLs, the named-entity table is
incomplete, and an unrelated MIME footer can displace the desired code.
Unsupported compat search syntax can make assertions pass vacuously; scoped
lists and latest disagree. Large single-message responses occupy async
workers, while numeric bodies allocate a string per extracted code. Repair
these defects at their existing owners, preserving ingest-time extraction
and the previous batches' storage, validation and delivery repairs.

## Original panel rulings (confirmed)

| Question | Adopted ruling and rejected alternative | Cost of the ruling |
|---|---|---|
| HTML boundaries | Adopt both panels' shared lexical boundary owner: recognize source attributes before decoding, and scan other source regions separately. Reject another independent attribute parser and whole-source decode before boundary recognition. | A lexical scanner remains heuristic on malformed HTML; shared scanner changes need all existing exact work pins. |
| Entity provenance | Adopt simplicity's pinned WHATWG `entities.json` snapshot, generated table, digest and CC-BY-4.0 attribution. Reject robustness's CPython substitute and interim-subset completion escape. | Snapshot acquisition is a prerequisite; unavailable provenance leaves #136 incomplete, never permission to fabricate data or call the subset complete. |
| MIME preference | Adopt simplicity's direct sibling rule, exactly as #137 requests. Reject robustness's shared-alternative-ancestor rule and filename filtering. | Nested `alternative[text, related[html]]` merges both selected bodies in source order; HTML-only extra codes may appear. An alternative containing a misleading text stub still prefers that stub. |
| Compat order | Adopt simplicity's descending `(received_ms, id)` view order everywhere in compat. Reject robustness's new acceptance sequence and reject merely documenting two compat orders. | Scoped lists sort O(n log n); clock rollback can make latest differ from most recently accepted. Native inbox order, POP3 and eviction retain acceptance order. |
| Search wrappers | Adopt refusal after limited wrapper recognition, including the issue's `(to:x)` example. Reject robustness's decision to leave that known vacuous form literal. | Literal colon-bearing words inside these wrappers may now refuse. No general Mailpit grammar is offered. |
| Time zones | Keep `tz` a named 400 on latest and document it, as simplicity proposes and #138 permits. Reject accepting an inert display option. | Clients that always send `tz` must remove it. |
| Response work | Adopt robustness's extension through existing raw/header/download aliases as well as the four required render cases. Reject a README-only stall residual and simplicity's narrower route set. | More blocking-pool hops and tests; pool contention with ingest remains possible. |
| Code bound | Adopt both panels' bounded distinct collector, 64 across selected sources. Adopt simplicity's fresh-ingest-only scope and preserve verbatim restore. Reject collect-then-truncate and robustness's load-time truncation. | Historical oversized arrays remain; a real code after the first 64 distinct candidates is omitted. No global memory guarantee follows. |
| Links | Reject robustness's additional 256-link cap and generic bounded-collection framework. Record the exposure here and in product documentation. | Links remain bounded only by source size: roughly 600,000 short distinct URLs can fit in 8 MiB and expand stored arrays, responses and webhook payloads. |

## Required behavior

### Source-aware links and complete references (#136)

Preserve literal plain-text scanning, text-before-HTML ordering, HTTP(S)-only
case-insensitive scheme recognition without spelling normalization, and exact
string first-occurrence deduplication. In HTML, use the existing tokenizer's
quote state to delimit a quoted attribute in the original source. Decode
that span once; decoded quotes/apostrophes are URL data, while decoded
whitespace and angle-bracket delimiters retain #94's termination behavior.
Other regions decode once before ordinary URL matching. Never tokenize
decoded markup. Preserve source links in unquoted attributes, comments,
head/template and raw-text elements; code suppression is not link suppression.
An unterminated recognized construct gives the link consumer its remaining
source once, using ordinary matching; it must not silently discard that tail.

Permit interior commas, bracketed IPv6 hosts and matched parentheses. Trim
trailing sentence punctuation and unmatched closing parentheses/brackets
with bounded linear work. No URL validation, normalization or network access.

Generate all 2,125 semicolon-ended named mappings from a pinned WHATWG
snapshot; retain all scalars, exact case, sorted unique names and binary
search. Record the actual snapshot SHA-256, source URL/date, attribution,
license and reproducible generation command. Numeric references require a
nonempty ASCII decimal or hexadecimal digit sequence after `#` or `#x`/`#X`.
Signs, non-ASCII digits, overflow and non-scalars stay literal; valid controls
keep the existing policy. Preserve decode-once, the 40-character window,
the returned statistics seam, both exact scan/byte pins, and the finite-table
expansion bound `5 * output_bytes <= 6 * input_bytes`.

The 106 bare legacy aliases remain literal in both decoder contexts. For
every legacy name `n`, `&n` and `&n=1` remain unchanged; `&n;x` decodes
the semicolon-ended mapping followed by `x`. Unknown `&notit;` stays
literal: no longest-prefix recovery. This is the deliberately narrower
contract of 0040 §5 and 0049 §2, not a claim that HTML requires semicolons
universally. Record WHATWG's title/author, source URL/date, CC-BY-4.0 license
URL and transformation in the generated module: the retained JSON is
unmodified; the derived table excludes bare aliases and strips delimiters.

### MIME-relative codes and bounded collection (#137, #139)

Select the same first genuine text and HTML body candidates as today; retain
their source identities transiently. Only direct children of the same
`multipart/alternative` earn text preference with HTML fallback when text
has no codes. Otherwise scan both selected bodies in source byte-offset
order. Single-part messages scan their one body. Do not expand selection to
all parts, skip inline filenames, synthesize bodies or include attachments.

The shared collector retains at most the first 64 distinct code strings in
the resulting order, across HTML runs and MIME sources. Duplicates consume
no slot. Bound allocations while collecting, never after an unbounded Vec
or set has been built. Retain ASCII 4–10 digit runs, run boundaries and
suppression rules. Existing exact-count/work pins are mandatory; continuing
a scan after the collector fills is allowed, allocating further codes is not.
SMTP and seed ingest persist the same capped arrays and feed them to webhooks.
Restore preserves historical stored bodies, links and codes verbatim; queries
never re-extract, truncate or write back historical arrays.

### Compat refusal and order (#138)

Both generic containing searches refuse the existing ASCII `identifier:`
form after recognizing at most one leading parenthesis, one exclusion
marker (`-` or `!`), and one leading double quote, in that order. Matching
closing wrappers do not hide the field; detection also works when a quoted
value spans whitespace tokens. This is refusal-only preprocessing: do not
strip wrappers from accepted literal queries or execute grouping/negation.
Keep the colon-followed-by-`//` URL exception, literal `-word`, and explicit
`kind=to|from|subject` literal values. Report HTTP 400 naming the field.

All Mailpit and MailHog list/search scopes use descending `(received_ms, id)`
before pagination. Latest uses the same maximum over its requested scope;
empty scope remains 404, filtered latest remains a named 400. At a stable
store, an unfiltered list's first item equals latest. Concurrent requests
are not an atomic snapshot. `tz` remains unsupported and named by the 400.

### Off-worker single-message responses (#139)

Await the existing `Store::offload` mechanism for full-message projection,
HTML-to-text rendering, large copies and serialization to finished response
bytes. Returning `Json<Value>` or `Json<Arc<Email>>` for later serialization
on the async worker is insufficient. Cover Mailpit by-ID and latest full
message, both plain aliases, raw/header/latest aliases, MailHog download,
and native single-message JSON and raw. Release map guards before rendering;
never acquire a mutation lock for it. Preserve response shapes, statuses,
content types, raw bytes and existing offload panic behavior. Offloading is
not a new response/admission budget or a guarantee against pool saturation.

## Acceptance Criteria

1. **AC1 (#136):** Real SMTP ingest and REST readback yield whole URLs for
   literal `O'Brien`, `O&#39;Brien`, `O&#x27;Brien` with the token suffix,
   `&quot;x&quot;`, `caf&eacute;`, and `go&rarr;`. Wikipedia `Foo_(bar)`,
   `http://[::1]:8080/p`, and `w_100,h_100/x.jpg` survive whole; unmatched
   sentence wrappers and trailing commas trim. Cover both quote delimiters,
   encoded schemes, decoded spaces, malformed tails and source-only links.
2. **AC2 (#136):** Test every one of the 2,125 names against the pinned source
   mappings, including multi-scalar values; regeneration matches committed
   data. Pin sorting, uniqueness, digest/provenance, case and expansion.
   `&#+52;` and signed hex stay literal in decoder/link input and cannot
   manufacture `4829` from `&#+52;&#+56;&#+50;&#+57;`.
   The offline generator refuses a digest mismatch before emitting a table.
   A Rust test independently derives all expected values from JSON
   `codepoints`, checks `characters`, and compares the whole sorted slice
   with `NAMED`, lookup and actual decoding. Pin 2,231 source keys, 2,125
   mappings and 106 identical legacy counterparts; exercise the literal
   legacy cases above, and prove no named value contains an ASCII digit.
   A changed table value with unchanged count must fail. Rust tests require
   neither Python nor network; the exact JSON fixture ships in the crate.
3. **AC3 (#136 preservation):** All seven #94 rows and both exact reference
   scan/byte pins remain unchanged and green. Preserve code tokenizer work
   pins and add deterministic one-pass link-region/trim accounting over
   malformed, multibyte and long punctuation inputs. Reads perform no extraction.
4. **AC4 (#137):** The supplied mixed/alternative-HTML plus footer fixture
   yields exactly `["121212", "2026"]`; the HTML plus inline filename text
   fixture yields exactly `["333334", "444445"]`. Real SMTP tests pin both.
   Pin reversed mixed order, direct alternatives with text preference and
   empty-code fallback, and nested related-under-alternative as a merge.
   Existing #74 body selection, alternative, attachment, absence/empty and
   raw-byte preservation tests remain green.
5. **AC5 (#138):** Each of `-to:`, `!from:`, `"to:x"`, `-"subject:x"`,
   `(to:x)` and a quoted field with spaces answers a named 400 on both generic
   containing searches. Positive controls for URLs, `-word`, ordinary quoted
   literals and explicit field-kind values preserve literal behavior.
6. **AC6 (#138):** A deterministic reverse-stamp/acceptance fixture and an
   equal-stamp/different-ID fixture prove identical descending order for
   scoped/global compat list/search, including page boundaries, and latest's
   first-item agreement. Prove it after restore too. Native inbox/POP3 order
   and oldest-only pruning keep their existing acceptance-order pins.
7. **AC7 (#138):** Real HTTP tests pin latest's `tz` named 400 and existing
   empty-scope/filter refusals. README known differences states wrapper
   refusal, the compat order, native-order distinction and unsupported `tz`;
   `Store::latest` documentation no longer claims every listing shares it.
8. **AC8 (#139):** Extend the current-thread real-server held-render proof
   to every required route/alias. While actual projection/render/serialization
   is held on the blocking pool, SMTP greets and health answers before the
   test gate expires; release yields the correct full body and headers. The
   seam must guard response construction, not just lookup. Preserve 404s and
   raw fidelity and include HTML-only plain rendering and native serialization.
9. **AC9 (#139):** Fresh SMTP and seed mails with 0/64/65 distinct codes,
   duplicates around the boundary, HTML run accumulation and mixed MIME
   accumulation prove a combined maximum of 64 in first-occurrence order.
   Collector observations prove no more than 64 retained distinct values or
   unbounded intermediate allocation. Persist/restart and a real webhook
   delivery preserve those arrays. A historical >64-code row stays verbatim
   through restore and reads, explicitly proving the documented exception.
10. **AC10 (#139):** For a fixed 8 MiB ASCII fixture comprising 1,048,576
    distinct seven-digit numbers separated by spaces, pin exactly the first
    64 codes. Their serialized array is at most 641 bytes (the general
    ten-digit array bound is 833). A one-mail native list is <9 MiB, a
    Mailpit summary page is <16 KiB, and a MailHog limit=1 page is <40 MiB;
    each retains its existing complete shape. Native lists serialize full
    Email bodies, not summaries. Full native/Mailpit bodies and MailHog duplication remain body-sized;
    do not claim body-independent total responses. Existing MailHog 64 MiB
    inclusive refusal proof stays green.
11. **AC11 (delivery):** Each changed behavior has a regression red against
    `2a325c9` behavior and green after repair, with issue-to-test mapping in
    the signed implementation commit. Preservation checks are labeled as
    such. All house gates, exact 100/100/100 coverage, script selftests and
    native-tls absence proof pass on the final implementation tree; verify
    owns the pinned bundle compile. No gate/dependency-policy weakening.

## Scope limits

No schema, acceptance sequence, runtime dependency, DOM, query grammar,
timezone engine, global memory/admission control, historical migration or
new executor. Links and old arrays remain resource exposures. Preserve the
single-writer lock, 5 s busy bound, awaits-registry cap, envelope sender,
webhook/bridge/Origin behavior, prior validation/delivery repairs, chart
startupProbe/knobs and publish guard. No issue closure or claim that the
remaining epic delivery-policy findings are resolved by this batch.


## Dated compatible correction — 2026-10-23, epic192 lane D

For #190 R4.1, quote tolerance applies to a URL starting a decoded quoted
attribute value after optional whitespace; embedded CSS/prose URLs use quote
delimiters and successive URLs remain separate. R4.5 removes trailing U+201D
and U+2019 as whole scalars, with byte-accurate trim counters. These compatible
clarifications preserve 0049 §1's source-before-decode and fresh-ingest rules,
with historical arrays unchanged. Exact SMTP/REST rows, accounting controls,
preservation tests and red/restored-green mutations are in the
[lane D handoff](../epic192-compat-webhook-proof/plan.md#implementation-handoff--2026-10-23).

The eleven-route held-response proof observes actual lookup/build sites and
completed responses; a serialization-only offload mutant with lookup and
projection on the async worker fails. Generator provenance now has actual
identical-input prior-run evidence and an independent wrong-mapping proof.
Full-gate/delivery tasks remain open; no exact coverage or whole-story
completion follows. Ordered prefix recognition, text-first links and ordinary
conditional-comment behavior remain pending exact R4.2/R4.3/R4.4 rulings.
