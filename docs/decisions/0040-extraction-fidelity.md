# 0040 — Extraction fidelity

- **Status:** accepted (operator, 2026-10-19)
  Original draft status: proposed — acceptance is the operator's.
- **Date:** 2026-10-18
- **Issues:** #74, #93, #94 (run 17).
- **Amends:** [0031](0031-sender-honest-stored-bodies.md), its body-selection
  interpretation and §3's extraction from rendered text; clarifies the scope
  of sender honesty for historical persisted rows.
- **Preserves:** [0001](0001-synchronous-insert-lossless.md),
  [0002](0002-awaits-subscribe-before-checking.md),
  [0003](0003-oldest-only-pruning.md),
  [0005](0005-extraction-at-ingest.md), and
  [0022](0022-bounded-data-raw-bytes.md).
- **Specification:** [extraction fidelity](../../specs/extraction-fidelity/spec.md).

## Context

Parser body lists include presentation fallbacks. Testing only candidate zero
can discard a later genuine representation. The display renderer joins codes
across elements and misses attributed suppressed elements. Decoding only after
matching a URL leaves decoded delimiters inside the stored result. These are
public stored-field defects, not reasons to change storage or query ordering.

## Proposed ruling

1. Select the first actual Text/Html in the corresponding outer-message body
   candidate list. Nested multipart bodies count; attachments, including an
   attached message's bodies, do not. Preserve absence, empty genuine bodies
   and raw bytes; never synthesize or concatenate representations.
2. Codes come from plain text first, or from independently scanned HTML text
   runs if plain text yields none. Use ASCII digits and ASCII word boundaries,
   `(?-u:\b)[0-9]{4,10}(?-u:\b)`. Each markup boundary separates runs. Decode
   references in a run once, after structural recognition, then apply the
   regex. Preserve exact-string first-occurrence deduplication. The display
   renderer remains available only to 0031's display-time carve-out.
3. The small tokenizer suppresses comments and attributed or bare head,
   template, script, style and title. Track head/template nesting; raw-text
   closes are case-insensitive and name-delimited. Attribute quote state starts
   after `=`; a quoted `>` never exposes attribute digits. Recognize markup
   entry only after `<` followed by an ASCII letter, `/`, `!` or `?`; other
   less-than signs remain text. Unterminated recognized constructs suppress
   their remainder. No browser tree repair or implicit closing head is added.
   The spec defines abrupt comments, declarations and self-closing spellings.
4. Decode the whole HTML source once before URL matching, including attribute
   text. The decoded source is used lexically, never tokenized as HTML. Keep
   literal plain-text scanning, existing URL delimiters and punctuation trim,
   text-before-HTML order and exact-string deduplication. Match only HTTP(S)
   schemes case-insensitively and retain their spelling. This recognizes
   encoded scheme characters and all URLs separated by decoded delimiters.
5. One bounded decoder owns both callers. It accepts all 2,125 semicolon-ended
   HTML5 named mappings, including every scalar in multi-scalar mappings,
   using exact-case lookup. A required semicolon deliberately excludes the
   106 legacy no-semicolon aliases in the 2,231-key standard table. `&Lt;`
   decodes to U+226A; `&AMP;` decodes to ampersand. Numeric scalar validation
   stays as it is: invalid values remain literal, valid controls are retained,
   and no C1 remapping or replacement-character policy is introduced.
6. Preserve the 40-character reference window, `RefScanStats`, the returned
   stats seam and both exact scan/byte pins. Never search the unbounded suffix
   for each ampersand or recursively decode. Full mappings can expand bytes:
   `&nGt;` grows 5 to 6. Validate the finite table's maximum expansion and pin
   `5 * output_bytes <= 6 * input_bytes` instead of claiming no growth.
   Tokenizer cursor progress and bounded name probes must also be observable
   with local deterministic counters. No global mutable statistics are added.
7. Keep the standard named mapping data in-tree with a pinned source digest,
   provenance, applicable attribution/license notice and reproducible generation
   instructions. Generation is maintenance work, not a build/network step.
   A new decoder crate and a DOM parser are not required. The dependency
   allowlist remains unchanged; verify the data's license before importing it.
8. These repairs govern fresh ingest. Restore remains verbatim: old stored
   text/html/links/codes can retain synthesis and extraction mistakes forever.
   No schema/version stamp, restore reparse, write-back, query fallback or boot
   warning is added. Implementation appends a dated clarification citing this
   proposal to 0031 and release notes; it does not rewrite an accepted ruling
   or mark this proposal accepted.

## Alternatives and costs

The panel agrees on bounded candidate scanning and ingest ownership. The
chief adopts robustness's per-run code scan, precise lexical guards and
deterministic work observations, but rejects removing head suppression:
malformed mail with an omitted closing head may lose later codes. This is the
explicit cost of retaining #93's requested suppression rather than exposing
head text. Full browser recovery is beyond the contract.

The chief adopts simplicity's full-source URL decode and rejects a raw URL
prefilter followed by re-matching. The prefilter misses encoded scheme bytes;
the selected approach costs one extra linear source allocation. Links remain
heuristic candidates from source, including scripts/comments, not validated
browser destinations. Codes and links intentionally have different inputs.

The in-tree mapping table wins over simplicity's focused dependency. Its
maintenance and attribution bill is accepted to keep token interpretation
owned by the existing bounded scanner. It is data, not a second HTML parser.
No-semicolon legacy syntax and HTML numeric recovery remain outside the
contract; decode-once intentionally leaves `&amp;lt;` as literal `&lt;`.

The documented persistence residual wins over robustness's restore migration.
The losing proposal would repair old answers, but would introduce versioning,
a second derivation site and recurring startup CPU. The selected behavior is
explicitly allowed by #94 and preserves historical records; old wrong answers
are a real accepted limitation, not a claim that those answers are honest.

Tag boundaries can expose a fragment: `<b>48</b><b>2935</b>` yields `2935`,
never `482935`. ASCII boundaries can match `5678` after Arabic-Indic digits.
These are heuristic fragments present in decoded text, not reconstructed OTPs.
Two URLs differing only in scheme case remain two stored links. Every one of
these consequences must be pinned and documented rather than hidden behind
an absolute claim to reproduce what a browser or sender intended.

## Proof and delivery

The spec's acceptance criteria are normative: real SMTP/REST tests for all
issue rows, MIME attachment/raw/POP3 coverage, complete entity mappings,
deterministic malformed-input and work tests, and the restart residual.
At least one observed behavioral red per child precedes its passing repair.
Keep 0020's test placement and 0035's exact coverage discipline, with no gate
edits. All six house gates are owed by implementation. This decision is
proposed and the design files do not claim those repairs have shipped.

## Pointer — 2026-10-18 (proposed 0049)

[0049](0049-extraction-compat.md) §§1–4, also **proposed**, update §§2/4 here (issues
#136, #137, #139): a quoted attribute value is delimited by its source quote before it
is decoded, so a decoded quote is URL data there; balanced parentheses, brackets and
interior commas stay in a link; numeric references take ASCII digits only; text codes
are preferred only for direct `multipart/alternative` siblings; and a fresh mail keeps
at most 64 distinct codes. §5–§7's full 2,125-name table is still outstanding: the
batch-4 implementing seat had no WHATWG snapshot, and the interim subset is not claimed
complete.

## Pointer — 2026-10-18 (proposed 0049 §2, full table)

§5–§7's table is now complete: `src/extract/entities.rs` carries all 2,125
semicolon-ended names generated by `scripts/gen-entities.py` from the
operator-supplied, unmodified WHATWG `entities.json` (fetched 2026-10-18,
SHA-256 `d741d877ac77c4194c4ad526b5b4a19aef8dfe411ab840a466891cdbb9f362e6`,
CC-BY-4.0, retained as `scripts/data/html-entities.json`), with the
attribution and digest recorded in the module and an inline test checking
every mapping against the snapshot. The 106 legacy names without a
semicolon stay literal, as §5 rules. The previous pointer's "outstanding"
is superseded by this one; the status of both decisions is unchanged.

## Later-decision markers — 2026-10-19 (issue #144)

Appended 2026-10-19 by the issue #144 provenance audit. The operator accepted
this record and the later record below on 2026-10-19; the text above, both
0049 pointers included, stays as written, and their "proposed" and
"unchanged" status wording is historical. Each marker uses the later
record's own verb and scope. The amendment this record makes to 0031 now
carries a dated marker in [0031](0031-sender-honest-stored-bodies.md).

- *(Updated by [0049](0049-extraction-compat.md) (accepted 2026-10-19):
  §§2/4 — MIME-relative code preference, bounded codes and source-aware
  links — and extends §§5–7's full-table and numeric proof, as the two
  pointers above describe; §8 is preserved.)*
