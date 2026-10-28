# HTML review repair implementation

This ledger implements proposed 0091; 0085/0090/0091 remain proposed.
The source packet's four SHA256SUMS passed. Behavioral reds against original v1
semantics are preserved in the run receipt, without a version-only mismatch.
The 0090 table repair and proposed 0091 index were already supplied by a0c9d11.

## Operation census and fixed bound (before adversarial measurements)

One inspection unit is a byte/scalar read, a prefix byte comparison, a copied
byte, or a conservative charge for the listed library operation. Repeated
invocations pay repeatedly, including failure. Counters are local u64 values;
there is no quota, rejection, atomics, per-byte allocation or second scanner.
Progress counters remain separate from inspections.

| Owner | Charged operations and maximum multiplicity |
|---|---|
| Work::byte/prefix | Every helper byte read including unsuccessful get; every prefix probe charges pattern length + 1, even mismatch. |
| HTML/text | Entry/lookahead and text reads; decoded whitespace classification charges full decoded length + 1 even early success. Final seven-rule dispatch charges 32. |
| tag | Name/attribute/value/whitespace reads at get; lowercase name once; lowercase/hash/key comparison three attribute lengths + 1; each first value decoded once. Duplicate values are still scanned but not decoded. |
| raw_end | Every candidate charges the two-byte prefix + 1; potential names charge name length + 2 (script/style/title <= 6), including failed boundary probes. Closing tag uses the same counted tag helper. |
| terminated/inert_end/string_end | Each terminator candidate charges its full <=3-byte prefix + 1; inert/quoted bytes use counted gets. No whole-suffix charge for an early successful search. Escapes skip a byte without inspecting it. |
| HTML rule/image/suppression | Per parsed tag 512 + 64*name length covers fixed rule/name comparisons, <=32 suppression names (<=8 bytes), copying and fixed-key probes. Per img remote check 64 + 8*sum(src,width,height lengths) covers trim, prefix, digit validation and numeric parse, including failures. |
| CSS dispatch/components | Eight units per loop cover repeated delimiter reads, bounded frame operations, sentinel/copy; counted entry/lookahead, <=4-byte next_char and copied escape length. Each URL prefix charges six. A byte enters one ordinary/comment/string/URL region. Active depth <=32; no stack traversal per byte. |
| CSS declarations/media | Each completed disjoint segment charges 64*(length+1), covering split, trim, backwards important search, seven rules and finite literal value lists (shipped maximum two). Media recognition charges 3*segment length+7. Components emit one opaque sentinel and do not concatenate literal tokens. |
| URL | Counted whitespace/data/escape reads, next scalar bytes, <=6 hex digits and optional CRLF lookahead. Quoted detection may revisit leading whitespace once through ordinary handling; no unbounded suffix probe. |
| entity decoder | Each find('&') charges inspected prefix incl. ampersand or full failed suffix + 1; each copied prefix/final suffix charges its length. Every parse charges window construction including the 41st scalar lookahead (<=164 bytes), window terminator search (<=160+1), and, when terminated, 5+14*(token bytes+1)+6 for prefixes, numeric validation/parse or <=13 binary-search comparisons in the fixed sorted 2,125-name table and output (<=6 bytes). Legacy RefScanStats.scans/bytes retain exact old meaning. |

Derivation, independent of measurements: decoder references cost at most
164+161+5+14*161+6 = 2590 units each, plus search/copy and loop
constants. Each reference begins at a distinct source ampersand. Thus
4096*d+1 bounds a decode invocation of d source bytes, including empty/failure.
Decoded output is <=6*d/5 (the existing full-table/numeric/literal proofs).

Outside decoding, HTML costs <=1024*n+256: each byte has at most the
tag/raw/terminator and classification passes above; fixed tag charges can be
assigned to distinct '<' bytes, name/value charges to their disjoint regions.
This deliberately overbounds name probes, 32-entry suppression and all seven
rules. CSS costs <=256*d+128 per invocation: dispatch/URL/strings <32*d,
disjoint declaration segments <=128*d+64 (including separators), media <10*d,
and bounded entry/final/frame constants fit the remainder. These charges also
bound an aborted prefix; unfinished segments are not matched.

Attribute and text decoding consume disjoint HTML regions, total <=n, in at
most n+1 calls. CSS bodies/style attributes consume disjoint regions, and only
attribute decoding can expand them: total CSS input <=6*n/5, at most n calls.
The combined conservative coefficient is
4096 + 1 + 1024 + 256*6/5 + 128 = 5556.2.
Round up once, before fixtures, to **C=8192, K=4096** per HTML part:
**W <= 8192*n + 4096**, where n is pre-entity decoded eligible HTML UTF-8
bytes. No coefficient is fitted to measurements. Helper-only tests use their
own original input length; direct CSS and decoder charges obey the same bound.

For p eligible parts, sum gives W <=8192*N+4096*p. With N<=4,194,304
and p<=256, W<=34,360,786,944, far below u64::MAX (and below 2^36).
Fixed message catalogue/part admission and MIME parse are outside this lexical
counter, explicitly bounded by the existing part/raw limits. Empty raw invokes
no lexer. No RSS, latency, allocator or MIME-parser-internal bound is claimed.
Hashing uses length charges under the standard HashMap probe model; randomized
collision behavior, allocator internals and future uninstrumented operations
are excluded. The deterministic claim is for the declared lexical charging model.

## Finding disposition and evidence

The implementation keeps the original non-security attributions; it grants no
security clearance or decision acceptance. Every row is addressed as follows:

| Original finding | Disposition and executable witness |
|---|---|
| Correctness MEDIUM / independent 2 LOW: custom-property braces | Explicit Rules/Declarations/Component frames, matching delimiters and shared depth. reviewed_custom_properties_preserve_declaration_context and component_values_keep_outer_and_following_declarations_visible assert complete evaluator reports, outer/following declarations, media/selector/entity/opaque values, mixed depth 32/33 and nested path [1,0]. css_component_and_literal_url_pairs_survive_smtp_reports asserts full root reports, hints, warning/error qualification and unchanged stored bytes over SMTP/REST/MCP/UI. |
| Correctness LOW / independent 5 INFO: work proof | inspected_work_has_a_fixed_linear_bound_separate_from_progress drives the actual helpers at sizes 64/512/4096/32768, including malformed tails, entity windows, near terminators/raw closers, long attributes and CSS. Legacy extract reference scan/byte/window/table expansion tests remain unchanged. Census/bound above and actual rescanning experiment below replace displacement as the work proof. |
| Independent 1 LOW: zero-byte SMTP message | HTML-only exact-empty branch after admission. reviewed_empty_raw_has_no_html_parts pins v2. empty_html_keeps_later_collectors_and_finish_bounds proves later contributions survive and contribution/report-byte excess still fails. empty_smtp_message_has_an_exact_composable_report sends an immediate terminator, observes 250, message lookup and exactly zero raw bytes before analysis; checks complete cold/warm REST/MCP/UI, exact-one/zero (and mismatches), minimum-one waits with empty-inbox control. report_stdio_text_contract now includes true zero bytes. catalogue_is_closed_and_purely_validated keeps invalid-catalogue/whitespace parser failure controls. No authentication collector exists here and no authentication result is invented. |
| Independent 3 LOW: misplaced 0090 row | Prerequisite a0c9d11 removed the separator and indexed 0091. Both remain contiguous proposed table members; design-check and the final receipt check this structurally. No second cosmetic index edit is necessary. |
| Independent 4 INFO: slash-star in unquoted URL | Literal URL token helper fixes both inline and stylesheet paths. reviewed_unquoted_url_keeps_comment_characters_literal and literal_urls_protect_data_without_disabling_ordinary_comments cover exact URL, following warning, boundaries, case, quotes, scalar/hex escapes, whitespace/CRLF/CR, invalid controls/EOF and outside/ordinary function comments. Real SMTP report pairs cover it. malformed_html_is_retried_and_never_cached now includes malformed URL after an earlier position match: no partial success or cached failure. |
| Independent 6 INFO: hardcoded provenance/date | Retain authored_heuristic and 2026-10-26 validation with both negative mutations in catalogue_is_closed_and_purely_validated. This finite v2 catalogue is authored guidance, not arbitrary data-only extensibility; future provenance/schema changes require Rust review and aggregate version discipline. |
| Independent 7 INFO: strict malformed failures | Retain strict unclosed head/recognized tag, stray closer and unterminated supported construct failures in malformed_supported_syntax_fails_and_nesting_is_bounded, plus new mismatched component and URL negatives. Living docs name the cost. This records the specified limited analysability, not an exemption for valid components/URLs. |
| Independent 8 INFO: displaced header wire witness | report_failure_is_never_clean_and_ui_keeps_message now separately sends header overflow and malformed HTML to isolated real SMTP inboxes; both have repeated exact sanitized 503s, failed assert/await/MCP and intact UI/list behavior. Existing report_failed_scan_is_not_exact_or_clean/demo_header_budget_is_exact owner tests remain. |

## Falsification and delivery evidence

The temporary mutation inserted a real counted repeated-suffix read loop into
terminated, using Work::byte and black_box on the read byte. It did not change
cursor movement or fabricate increments. The **same** fixed-bound test first
passed its progress assertion, then failed at n=32775:
W=537149483 > 268496896. Restoring the exact source bytes made it green.
No fault hook or sabotage path ships. The run-local work-mutation.json receipt
preserves original/restored source hashes, patch hash, exact test command and
red/green log hashes; behavioral reds have their own receipt. This evidence is
run-local data, not a tracked link into a delivery journal.

The initial exact-coverage attempt refused at 16797/16798 lines and 1146/1148
branches (all 1742 functions covered). Its evidence is retained. Valid quoted
URLs inside a component and hex-escape CR-without-LF cases execute the missing
outcomes; no denominator exclusion, test migration or gate edit was introduced.
Fresh final gates remain mandatory: completion is determined by the implementation
receipt's exit codes and integer coverage equality, never this ledger alone.

The final implementation receipt binds every gate command, exit and log hash
to committed HEAD/tree and the candidate-local engine digest. It includes the
house inventory, coverage selftest, publish/seal proofs, license check, engine/
shim integrity, design/delivery selftests, package inclusion and fail-closed
native-tls absence. The prior chief/reviewer receipts remain historical.
Existing burst/p2/rate, MIME/charset/clipping/budget and worker/cache/wait/restart
proofs run unchanged in the full suite. Seven rule IDs, hints, severities,
thresholds and locations are unchanged; aggregate v2 constant/catalogue/full
snapshots/config/cache and living docs move together. No dependencies were added.

CI owns macOS, RustSec, Helm/chart/history, Docker, privileged port-80 and tag
release proofs. The operator owns acceptance, settings, sealing and publication.
This seat never pushes/merges/accepts. Main integration and later eval/PR work
are deliberately subsequent: retain main's bulk and every collector's honest
unknown-authentication/failure outcomes on empty raw, then verify and independently
review the combined candidate. This historical run provides no fresh clearance
for that future tree.

Integration note: the proof table above describes the original HTML-only candidate. In the combined bulk-checklist candidate, exactly empty stored raw contributes no HTML findings, but the bulk metadata collector rejects it; REST, MCP and UI expose that failure rather than a clean report. The combined tests exercise both the HTML collector and the full failure path.
