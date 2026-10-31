# HTML review repair

Date: 2026-10-28. Epic #251, issue #259; prerequisite to PR #268.

Decision: 0107 — [proposed repair contract](../../docs/decisions/0107-html-review-repair.md).

## Purpose and authority

Repair the shipped HTML collector's false CSS warnings, false URL failures and
reachable empty-message failure, and replace its cursor-distance claim with a
falsifiable inspected-work proof. Passing shell gates do not close unmet
acceptance criteria. This is design; implementation and fresh review remain.

Source candidate: a49c702; 3f7dd88 adds only the
[repair intake](../html-client-compatibility-review-repair/intake.md). The legacy
run-local tasks directory is absent. Both positions and the full supplied review
packet were read; all four SHA256SUMS pass. Run-local evidence is not a tracked
link authority; the intake preserves its provenance and eight obligations.
[0085](../../docs/decisions/0085-deliverability-report.md) and
[0106](../../docs/decisions/0106-html-client-compatibility.md) remain proposed.
0107 records repair semantics, accepting neither proposal. Compatible work follows
accepted [0045 §8](../../docs/decisions/0045-delivery-repairs.md) and the operator's
#251 lazy-analysis direction. Historical specs remain records. Preserve accepted
0001–0005, including ingest extraction as amended by 0031/0040/0049.

## Panel adjudication

The positions agree on the principal repairs; no disagreement is invented.
Adopt their shared local-collector approach and settle remaining choices:

| Choice | Ruling and cost |
|---|---|
| CSS ownership | Adopt robustness's explicit declaration/component frames and simplicity's existing scanner owner. Include balanced braces, parentheses and square brackets in values. Reject blanket custom-property/block suppression. Pay delimiter state and boundary tests; genuine enclosing/following declarations cannot disappear. |
| URL recovery | Adopt both positions' literal URL repair with the grammar below. Reject disabling comments in all functions or treating the INFO defect as optional. Malformed URL tokens fail; browser recovery remains outside scope. |
| Work proof | Adopt robustness's operation accounting and original-source denominator, with simplicity's small counters. Reject cursor-only proof, generic tracing and permanent sabotage mode. Pay local arithmetic and a reproducible temporary mutation experiment. |
| Empty message | Adopt both positions' HTML-only exact-empty exception. Reject documentation-only failure and aggregate early success. Pay a version transition and composition/wire proofs; future authentication still reports unknown or failure honestly. |
| Breadth | Adopt simplicity's existing owners. Robustness's lifecycle envelope becomes preservation tests and explicit limits, not new cache/executor/webhook/persistence work. Shared-pool contention and selected snapshots remain. |
| Records | Use short slug html-review-repair and new proposed 0107 as this task requires, rather than reusing only the long intake directory as the positions preferred. Link intake, preserve old specs, add dated proposed pointers during implementation. Pay an additional decision/index entry. |

Reject a CSS engine, DOM/AST renderer, variable/cascade evaluation, new dependency,
generic cost framework, public schema change and runtime work quota. No rule is
disabled, renamed or weakened to close a finding. Existing heuristic gaps never
excuse the concrete valid component-value or URL cases.

## Contract

### Declaration and component context

Inline style starts in declaration context; stylesheets start in rule-list
context. Recognized rule blocks establish declarations; media containers retain
rule lists. A colon establishes property/value only within a declaration, not
selector text. Preserve existing supported media/selector behavior without
claiming general CSS nesting or selector resolution.

Within a value, braces, parentheses and square brackets are balanced components.
Internal colons/semicolons cannot create declarations or end the enclosing one.
Strings, escapes and comments protect delimiters. Matching close restores the
owner; only its top-level separator/block close ends a declaration. Unmatched,
mismatched or unclosed supported delimiters fail. All active CSS components,
blocks and functions share cap 32; the implicit inline root costs no level.
HTML suppression retains its existing separate 32-level bound.

Preserve the outer property and opaque value presence: a complex nonempty
position value still follows its property-only selector. Inner position text
in --unused is never a declaration. Do not concatenate omitted components into
a literal such as flex. Literal display matching, !important, case/whitespace
and decode-once behavior retain their meaning. No variable resolution.

Both `<p style="--unused:{position:absolute};color:red">Hello</p>` and
`<style>.x{--unused:{position:absolute};color:red}</style><p>Hello</p>` yield no
position finding. A following genuine position:absolute produces exactly the
existing warning, hint and MIME-part location.

### Literal URL tokens

Recognize ASCII-case-insensitive literal `url(` at a token boundary: the
preceding character cannot continue an identifier or escape; myurl( is ordinary.
Do not normalize escaped identifiers or permit whitespace between url and (.
Consume optional CSS whitespace after (. A quote selects existing quoted-string/
function handling. Otherwise unquoted URL data makes slash-star, braces,
brackets, colons and semicolons literal, not comments/declarations. The first
unescaped ) closes it. The opening parenthesis consumes one shared depth level.

In unquoted data, backslash protects the next non-newline scalar; hex escapes
consume one to six hex digits and optional terminating CSS whitespace (CRLF is
one terminator). This protects structure without decoding identifiers or fetching
URLs. Trailing CSS whitespace is allowed only before ); non-whitespace after it
is malformed. Unescaped quote or (, non-printable control (NUL, U+0001–0008,
U+000B, U+000E–001F, U+007F), trailing backslash, escaped newline or missing close
fails. CSS whitespace is TAB, LF, FF, CR or space. Ordinary functions/comments
retain their behavior; outside unterminated slash-star still fails.

Thus `background:url(https://x.test/*/a.png)` is valid and produces no findings
by itself with substantive body text, leaving a following real declaration
visible. No network access or browser recovery is implied.

### Actual inspected work

Keep cursor progress/displacement as a separate invariant. Count work at the
operations performing it: HTML/CSS byte/scalar reads, failed/successful prefix
and terminator probes, tags/attributes, raw ends, inert regions, strings/escapes,
components, declaration splitting/trimming/matching, suppression stacks, image
attributes and entity decoding. Repeated operations are charged repeatedly.
Library scans, copies, normalization and fixed lookup may use documented
conservative charges per invocation with justified inspected-prefix/length
bounds. Charging the whole remaining suffix for each early successful search
would itself be quadratic accounting. State hashing/lookup assumptions; do not
claim deterministic bounds on randomized collisions or allocator internals.

Before adversarial fixtures, implementation records an operation census and
derives numeric fixed C/K for `W <= C*n + K`: n is eligible HTML UTF-8 bytes
before entity decoding. A per-part proof composes over at most 256 parts with
fixed per-message overhead; K covers empty parts. Include failures; helper-only
tests name their input domain. Derive constants from pass multiplicities, fixed
catalogue, 32-level stacks, 40-character reference window including construction
lookahead, bounded entity lookup and 6/5 decoded expansion, never measurements.
Preserve exact legacy RefScanStats fields/meaning and decoder semantics.

Use small local integer counters with justified range at admitted sizes; the
same scanner/decoder runs in production and tests. No globals, atomics, locks,
per-byte counting allocations, feature flags or new operational rejection.
This proves the lexical charging model, not hardware instructions, MIME-parser
internals, RSS, latency, fairness or future uninstrumented changes.

Temporarily mutate an actual helper to perform real counted repeated-suffix
inspection. The same fixed-bound regression must fail while cursor progress
passes. Preserve mutation diff/command/red-log hashes, restore exact source,
rerun green. Fabricated counter increments, a second scanner or timing cannot
satisfy this. No permanent sabotage path ships.

### Empty raw and composition

After catalogue validation/raw admission, exactly zero raw bytes mean no eligible
HTML parts. Only HTML returns success with no findings before MIME parse.
Whitespace and arbitrary nonempty parse failures receive no exemption. Demo,
other collectors and Collector bounds/finish still run; no aggregate early return.

On this historical candidate the zero-byte report is exactly
`{"rule_set_id":"swarmail-html/2","findings":[]}`. This describes enabled demo/
HTML rules, not authentication. Later main integration must retain bulk and every
other collector's honest unknown/failure behavior, with fresh combined-candidate
verification and review.

### Version, bounds and ownership

Advance to swarmail-html/2 across constant/catalogue/config/cache/fixtures/living
docs. Keep seven HTML IDs, severities, hints, locations, thresholds, demo rule and
public schema. Add dated proposed repair pointers to 0085/0106; accept neither.
Retain authored provenance/date validation and negatives.

Keep raw 8 MiB, eligible decoded aggregate 4 MiB, 256 outer parts, MIME depth 32,
shared CSS depth 32, 256 contributions, 65,536 report bytes and root-header limits.
No partial/truncated success, cached failure, dropped failed predicate candidate
or partial exact count. Preserve four worker-held permits, 128 Weak-identity/
config/version slots, cancellation ownership, selected snapshots, one subscribed
wait loop/deadline. Stored bytes/bodies/links/codes stay unchanged; analysis stays
lazy/offloaded, absent from SMTP/seed/restore/fan-out, with no runtime network.

## Review disposition

| Original finding/severity | Required disposition |
|---|---|
| Correctness MEDIUM / independent 2 LOW: value braces | Repair ownership, paired full evaluator/wire reports; AC1. |
| Correctness LOW / independent 5 INFO: work proof | Actual inspections plus falsification, progress separately; AC2. |
| Independent 1 LOW: empty SMTP mail | HTML-only exception and real wire/composition proof; AC3. |
| Independent 3 LOW: index row | Remove separating blank line, proposed status/table membership; AC5. |
| Independent 4 INFO: URL/comment | Repair token context with real regression; AC4, no skip. |
| Independent 6 INFO: fixed provenance/date | Retain validator/negatives; future provenance needs code/version review, no arbitrary data-only extensibility promise; AC5/6. |
| Independent 7 INFO: strict failures | Retain documented unclosed head/recognized tag and stray closer failures with negatives. Accept reduced analysability; valid components/URLs are not exempted; AC5/7. |
| Independent 8 INFO: displaced header witness | Restore distinct header-overflow and malformed-HTML wire sanitized failures, retries and failure-not-clean proof; AC5. |

Preserve original non-security attributions. Prior clean security/guarantees/tests
reviews are historical, not fresh clearance.

## Acceptance Criteria

1. **AC1 — CSS ownership.** Real HTML evaluator and SMTP-report tests cover both
   exact examples and genuine following declarations, nested components/internal
   separators, strings/comments/escapes, selector colons, media/entities and
   complex outer position values. Assert complete reports, warning qualification,
   exact hint and root/nested MIME location. Opaque components cannot fabricate
   display literals. Shared depth 32 passes/33 fails. Rule suppression or mere
   success assertions do not satisfy this criterion.
2. **AC2 — work proof.** Publish census/numeric C/K derivation before measurement.
   Predetermined multiple sizes cover long quoted/unquoted attributes, near raw
   closers/terminators, inert regions, ampersand/unknown/multibyte references,
   long/nested CSS and malformed tails through real helpers. Fixed work bounds
   and separate progress pass; counted repeated-suffix mutation fails that bound
   with progress green; restored source passes. Keep decoder exact stats/6/5
   proofs and document counter range/model exclusions.
3. **AC3 — empty SMTP.** Immediate DATA terminator gets 250; lookup and exactly
   zero /raw bytes precede analysis. Pin complete REST/MCP/UI, cold/warm equality,
   exact-one/zero assertion semantics and minimum-one waits with empty-inbox
   control. Owner tests prove later fixture contribution survives empty HTML and
   its overflow still fails. Invalid catalogue/nonempty parser failure remain
   failures. No unknown-authentication success, including later integration.
4. **AC4 — URLs.** Exact reviewed URL passes inline/style-block evaluator and
   SMTP-report tests; following declaration still warns. Pin uppercase/token
   boundaries, quoted/unquoted, protected delimiters, hex escapes, whitespace,
   malformed controls/delimiters/EOF. Outside unclosed comments and ordinary
   function comments preserve behavior; failures are atomic and uncached.
5. **AC5 — all findings.** Implementation ledger names tests/receipts for every
   row. 0106/0107 are contiguous table members marked proposed. Fixed provenance
   and strict lexical negatives remain. Distinct header-overflow and malformed
   HTML wire tests prove sanitized failure, failure-not-clean and retry; retain
   owner-local coverage. Documentation cannot close AC1–4 defects.
6. **AC6 — version.** Exact JSON/catalogue/config/cache fixtures use v2; rule
   identities/hints/severities/schema remain. Retain seven pairs, MIME eligibility,
   charset/clipping boundaries, version/config cache isolation and packaged data.
   README/llms/OpenAPI and proposed pointers agree.
7. **AC7 — preservation.** Existing size/depth/finding/header/report boundaries,
   no-truncation/failure/retry and cache/cancellation/snapshot/ready-only/wait
   interleavings pass. Real burst/p2/rate preserve 0001–0005. No changed stored
   extraction, SMTP analysis, new dependency, weaker gate, security relabel,
   optional skip or severity change.
8. **AC8 — fresh proof.** Defect red/green and every local
   [house gate](../../docs/house-rules.md#gates--part-of-every-change) pass on the
   repaired committed candidate with HEAD/tree/engine-bound logs/hashes. All
   coverage dimensions have missed == 0; selftests, pinned engine/shims, package
   inclusion and fail-closed native-tls absence pass. Follow 0020; no broad test
   migration, sleep-as-sync or exclusion. CI owns macOS, RustSec, Helm/chart/
   history, Docker, privileged port-80/tag proofs; operator owns acceptance,
   sealing/settings/publication. Design does not claim implementation completion.
