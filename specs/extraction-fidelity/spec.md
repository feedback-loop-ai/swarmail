# Extraction fidelity

**Date:** 2026-10-18  
**Status:** Draft design; implementation and operator acceptance remain outstanding.  
**Scope:** Issues #74, #93 and #94, run 17.  
**Decision:** [0040 — extraction fidelity](../../docs/decisions/0040-extraction-fidelity.md), status **proposed**.

## Feature and rationale

An accepted mail must expose its actual MIME body representations and useful
precomputed links and codes. Today a fallback candidate hides a later genuine
body, the display renderer joins unrelated digits or exposes stylesheet and
attribute values, and decoding an already matched URL introduces delimiters
that remain in the stored link. These defects let a test assert on values the
sender did not supply, or miss values the sender did supply.

Repair these three derivations together inside the existing offloaded ingest
parse. SMTP 250 still follows synchronous storage; queries read stored fields.
Raw bytes, await subscription order, pruning and persistence ordering retain
their existing guarantees. This design changes extraction semantics and
clarifies decision 0031, so 0040 requires operator acceptance; this draft does
not accept it.

## Inputs and authority

Read both run-local positions, `.brokkr/state/run17-feature.txt`, and the full
issue bodies in `.brokkr/state/epic70/{74,93,94}.md`. The files in
`.brokkr/state/tasks/` frame older run-12 gate work; there is no run-17 intake
file there. The explicitly named run-17 framing and issue bodies govern this
design. Run-local evidence is not a deliverable or a prerequisite for reading
the requirements below.

## Panel rulings

| Conflict | Ruling and rejected alternative's cost |
|---|---|
| MIME candidate selection | Adopt both positions' complete scan of the parser's body candidates, using the existing iterators and actual PartType. Reject a fixed second-index patch and a recursive walk of all parts: the former still misses later bodies; the latter can promote attachments. Require the robustness position's attached message/rfc822 regression. |
| Code text buffering | Adopt robustness's regex per decoded text run. Reject simplicity's concatenated extraction buffer: it requires a second separator invariant and a full-body allocation. Cost accepted: regex entry per run and independent word boundaries at each tag. |
| HTML entry/close rules | Adopt robustness's ASCII tag entry, attribute-value quote state, bounded case-insensitive close matching, and deterministic work observations. These guard concrete losses and attribute leaks without a DOM. |
| Suppress head | Adopt simplicity and #93: suppress head, including attributed head. Reject robustness's proposed exception. Cost accepted: an omitted closing head can hide subsequent codes; allowing it would expose head text and violate the requested suppression rule. No implicit browser head closure is promised. |
| URL ordering | Adopt simplicity: decode the entire HTML source once, then apply the URL regex. Reject robustness's raw-match/decode/re-match pipeline: its first match misses encoded scheme characters, such as `https&#58;//x.io/`. Cost accepted: one bounded full-source allocation. This is a lexical URL scan, never parsing decoded HTML. |
| Named entities | Adopt robustness's in-tree standard mapping data and bounded lookup. Reject a new decoder dependency: only mapping data is needed, and the existing scanner must remain the sole decoding owner. Cost accepted: a provenance, attribution and regeneration obligation for roughly two thousand data entries. |
| Restored rows | Adopt simplicity's documented residual, explicitly allowed by #94. Reject robustness's extraction-version column, shared restore derivation and boot warning fallback. Cost accepted: old incorrect fields remain visible after every restart, with no automatic repair or per-row warning. Versioning and restore CPU/storage policy exceed this fresh-ingest repair. |
| Scope and proof | Adopt both positions' real SMTP regressions and unchanged reference-window pins. Adopt deterministic tokenizer totality/work tests, but do not call a finite enumeration a proof for all strings. Reject DOM/CSS rendering, generic extraction frameworks, query-time fallback, gate weakening and unrelated storage work. |

Correct two factual claims in the robustness position: `&Lt;` is a valid HTML5
name for U+226A; exact-case lookup must decode it. The standard's 2,231 keys
include 106 legacy semicolon-less aliases; the semicolon-required policy needs
all 2,125 semicolon-terminated mappings, including multi-scalar values. Those
aliases are deliberately not accepted syntax. Full mappings do not mean a
browser's complete context-dependent error recovery.

## Required behavior

### Sender body selection

Select the first genuine `PartType::Text` from the outer message's text-body
candidate list and the first genuine `PartType::Html` from its HTML-body list,
in parser order. Continue past fallback types. Nested multipart containers
belonging to the outer message are eligible; unrelated attachments and bodies
inside an attached `message/rfc822` are not. An empty genuine body remains
present; no match means absent. Never concatenate bodies or synthesize the
missing representation. Preserve the original raw message.

### Codes

Use `(?-u:\b)[0-9]{4,10}(?-u:\b)` for both representations. Return plain-text
codes if any exist; otherwise scan HTML. Preserve first occurrence order and
exact-string deduplication. ASCII boundaries deliberately allow an ASCII digit
run next to non-ASCII characters; non-ASCII digits themselves are never codes.

The HTML scanner recognizes structure before decoding references. Every tag,
comment or suppressed-region boundary ends a text run; scan each decoded run
independently. Preserve ordinary text and its whitespace. Decode once: encoded
markup is displayed text, never another tokenizer input.

Recognize tag entry only when `<` is followed by an ASCII letter, `/`, `!` or
`?`; otherwise keep it as text. Tag names and suppression closes are ASCII
case-insensitive, with a name ending at whitespace, `/` or `>`. A quote starts
a quoted attribute value only after `=` and optional whitespace; an apostrophe
inside an unquoted value is not a quote opener. Quoted `>` never ends a tag.
Attribute contents never become code text.

Suppress `head`, `template`, `script`, `style` and `title` contents, regardless
of attributes. Track nested head/template containers; an unrelated closing
tag cannot end suppression. Script/style/title use raw-text matching: only
their matching closing name with a valid delimiter ends that region, even if
their contents resemble comments or other tags. A self-closing spelling of a
suppressed HTML element still requires its closing tag. Comments end at
`-->`, including the abrupt `<!-->` and `<!--->` forms; other declarations
and processing-instruction spellings consume through the next `>`.
Unterminated recognized markup, quotes, comments or suppressed regions consume
the remainder. Codes emitted before the malformed construct remain available.
No DOM repair, CSS visibility or joining across inline elements is promised.

### References and links

One decoder serves HTML link source and emitted HTML code runs. Require a
semicolon and exact-case lookup of all standard named mappings, preserving
every scalar. Keep numeric behavior: valid decimal/hex Unicode scalars decode;
malformed, surrogate and out-of-range numerics remain literal. Keep valid
control scalars, without HTML5 replacement-character/C1 remapping. Unknown
names and missing semicolons remain literal. Never decode recursively.

Preserve `REF_MAX = 40` **characters after each ampersand**, `RefScanStats`,
the returned-stats seam and the two existing exact hostile-input pins. A named
lookup receives only the bounded token. Multi-scalar replacements may grow:
`&nGt;` is five input bytes and six output bytes. Replace the obsolete
never-grows claim with the table-checked bound `5 * output_bytes <= 6 *
input_bytes`; malformed copies and numeric decoding satisfy it too.

For links, scan literal plain text first, then once-decoded HTML source,
including attributes. Use the existing URL excluded-character class and
trailing punctuation policy, with ASCII case-insensitive `http`/`https`
schemes. Apply the URL pattern only after HTML decoding; decoded whitespace,
quotes and brackets end a link. Preserve decoded spelling, scheme/host case,
text-before-HTML order and exact-string first-occurrence deduplication. Two
case variants remain distinct. No URL normalization, validation or navigation
is added; these are heuristic candidates, not a browser equivalence promise.

### Historical data

`Persist::load` and the schema remain unchanged. Stored text/html/links/codes
restore verbatim, including historical synthesis and extraction errors. A new
delivery of the same raw mail receives the repaired derivation. Implementation
must append a dated clarification to 0031 without rewriting its ruling, and
record this limitation in release notes. Old rows are not retrospectively
covered by the fresh-ingest fidelity claim.

## Acceptance Criteria

1. **AC74 — representations:** Real SMTP tests send multipart/mixed text then
   HTML and the reversed order, assert both exact bodies and the HTML-only
   link immediately after 250. Also cover plain-only, HTML-only, empty genuine
   bodies, multipart/alternative, multiple genuine candidates (first wins),
   nested multipart, ordinary attachments, and a text-only outer message with
   an attached HTML-only message/rfc822 (outer HTML remains absent).
2. **AC74-raw:** For the mixed fixture REST `/raw` equals the accepted raw
   bytes, and POP3 RETR yields the same payload under the existing CRLF and
   transparency rules. No extraction or body-selection mutation reaches raw.
3. **AC93 — table:** Send each of these HTML-only inputs over real SMTP and
   read the complete codes array through REST:

   | HTML | Expected codes |
   |---|---|
   | `<table><tr><td>Order 1234</td><td>5678</td></tr></table>` | `["1234", "5678"]` |
   | `<div>Ref 2026</div><div>482913</div>` | `["2026", "482913"]` |
   | `<div>482935</div><div>is your code</div>` | `["482935"]` |
   | `<h1>Your code</h1><h2>482933</h2>` | `["482933"]` |
   | `<ul><li>Code</li><li>482936</li></ul>` | `["482936"]` |
   | `<table><tr><td>482937</td><td>expires soon</td></tr></table>` | `["482937"]` |
   | `<p><strong>482938</strong>is your code</p>` | `["482938"]` |
   | `<style type="text/css">p { color: #333333; }</style><p>Hello</p>` | `[]` |
   | `<script type="text/javascript">var t = 44445555;</script><p>Hello</p>` | `[]` |
   | `<img alt="x > 12345678" src="https://x.io/i.png"><p>Hello</p>` | `[]` |

4. **AC93-edges:** Test attributed head, nested template, title, comments,
   mixed-case closes and near misses, quoted/unquoted attributes, every EOF
   state, bare `<`, multibyte text, entity digits, plain-text preference and
   stable deduplication. Pin `x<i>555555</i>y` to `["555555"]`,
   `<b>48</b><b>2935</b>` to `["2935"]`, `&lt;style&gt;333333` to
   `["333333"]`, and plain `١٢٣٤` to `[]`. These are explicit heuristic
   boundary consequences, not claims to understand a sender's intended OTP.
5. **AC94 — table:** Real SMTP/REST tests cover all seven issue rows:
   `&lt;https://x.io/unsub?u=1&gt;` yields `https://x.io/unsub?u=1`;
   `&quot;https://x.io/q&quot;` yields `https://x.io/q`;
   `https://x.io/p&#41;` yields `https://x.io/p`;
   `https://x.io/a&#32;b` yields `https://x.io/a`;
   `https://x.io/n&nbsp;now` yields `https://x.io/n`;
   `https://x.io/a?b=1&AMP;c=2` yields `https://x.io/a?b=1&c=2`;
   `HTTPS://X.IO/upper` retains exactly that spelling. Use the original
   surrounding tags/attributes from #94 and assert whole arrays.
6. **AC94-edges:** Pin ordered multiple URLs separated by encoded delimiters,
   encoded scheme punctuation, trailing punctuation, duplicates across both
   bodies, case-distinct URLs, literal plain-text entities, unknown names,
   missing semicolons, malformed numerics and one-pass decoding. Validate all
   2,125 named mappings, including `AMP`, `LT`, `Lt`, `GT`, `QUOT`, `nbsp`,
   `fjlig`, `NotEqualTilde`, `ThickSpace` and growing `nGt`/`nLt`.
7. **AC-bounds:** Existing 500,000-scan pins remain exactly 19,999,180 and
   29,999,410 examined bytes. Extend local stats to observe tokenizer traversal
   and bounded close probes, with exact operation counts on repeated tags,
   bare `<` and raw-text near misses. Deterministic small-alphabet enumeration
   and explicit multibyte cases terminate without panic or invalid slicing.
   Review the monotonic-cursor argument; time thresholds are not the proof.
8. **AC-restore:** A legacy persisted fixture retains its stored derived
   fields across restart while a fresh delivery of the same raw source uses
   the repair. The dated 0031 clarification and release notes state this
   difference, including that restart does not repair old rows.
9. **AC-delivery:** At least one behavioral regression per child fails on the
   reviewed/current unchanged extraction and passes after repair. Record the
   baseline SHA, assertion and commands. All six house gates pass, with exact
   100/100/100 coverage, no excluded production code, no allowlist widening
   and no weakening of existing bounds. Design artifact validation alone is
   not feature completion.

## Dated clarifications

- **2026-10-18:** Retain head suppression despite the robustness objection;
  adopt full-source link decoding and a semicolon-required complete named
  mapping set; choose the documented historical-data residual. The preceding
  requirements supersede conflicting panel recommendations.
