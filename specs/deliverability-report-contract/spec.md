# Deliverability report contract

Date: 2026-10-26. Status: design draft, current-main scope corrected at re-intake. Epic #251, issue #252.

Current framing: [current-main intake](intake-current-main.md). The operator
explicitly withdrew the earlier #153 prerequisite; this active draft is revised
under that instruction, without rewriting earlier runs or accepting 0085.

Decision: 0085 — [Lazy deliverability reports](../../docs/decisions/0085-deliverability-report.md), proposed; acceptance belongs to the operator.

## Feature and rationale

Programs need one stable explanation of a captured message that they can inspect,
render and use in assertions. Add a typed, versioned report, computed only when
requested from the exact stored bytes. REST, MCP, the message page and predicates
share its evaluator and cache. This phase specifies the feature; it does not
claim that the implementation or its behavior tests already exist.

The operator's #251 ruling dated 2026-10-26 settles lazy cached analysis outside
SMTP. This is a separate analysis product: stored bodies, links, codes, thread
fields and envelope provenance remain ingest-owned and unchanged. The founding
[0001](../../docs/decisions/0001-synchronous-insert-lossless.md),
[0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md),
[0003](../../docs/decisions/0003-oldest-only-pruning.md) and
[0005](../../docs/decisions/0005-extraction-at-ingest.md) remain authority.

Only an artificial demonstration rule ships. Empty findings mean “no findings
from this rule set,” never “deliverable,” “spam-free,” or authentication success.
DNS policy (#253), authentication, substantive content/spam checks, external
scorers, new dependencies and license widening are outside this story.

## Evidence and dependencies

Both supplied issue-packet files passed their SHA256SUMS. README digest:
`258493ffee3b828847c1236d1c28fb690a263f6273a1b57d4c58749055b1cf41`;
operator-rulings digest:
`5b8da87b897306a313f673b134edd1a56e0f477117a07618e3ec345449a6c6f9`.
The packet's prior-outcomes section is empty. The run-local simplicity and
robustness positions were both read. The legacy tasks directory is absent;
[intake.md](intake.md) is the framing. No external source, journal or sibling
worktree was consulted. The tracked [rulings](../../docs/rulings/epic-192.md)
copy predates the supplied #251 ruling; the dated authority above is transcribed
from the verified packet, not inferred from that older copy.

**Current-main scope, operator correction of 2026-10-26:** implement independently
of epic #164. MCP uses main's existing 2025-03-26 tools contract: report JSON in
a text content block. There is no design or completion dependency on decisions
0079/0080/0082 or on #152/#153 implementation artifacts. Add the report's
`structuredContent`/`outputSchema` through the shared owner when #153 lands,
as a later, small follow-up; that follow-up is outside this issue's completion
criteria. No protocol negotiation or newer-revision conformance is claimed here.
The earlier IMPL-BLOCKED outcome records the withdrawn brief, not a current
missing input. All required inputs for this framing are present locally.

## Contract

A report is `{rule_set_id, findings}`. Each finding has exactly `rule_id`,
`severity`, `location`, `fix_hint`. Severity is `info`, `warning`, or `error`,
ranked in that order. Rule IDs are stable nonempty ASCII identifiers; hints are
nonempty fixed English strings for the initial rule. No timestamps, arrival IDs,
cache flags, timing, scores or locale-dependent text enter the report.

Location is a tagged object, with unsigned integer coordinates:

| kind | Fields and identity |
|---|---|
| `header` | `name`: lowercase ASCII root-header field name; `occurrence`: zero-based occurrence among that name's raw fields, or JSON null for a missing header. |
| `mime_part` | `path`: child-index array, root `[]`; children in wire order, including attachments. An encapsulated message root is child 0 of its containing message/rfc822 part. |
| `link` | `part_path`: the same part coordinates; `byte_offset`: zero-based first source byte of the URL token within that part's transfer-decoded payload, before charset/entity decoding. No URL is echoed. |

Link coordinates describe source bytes, not the ingest extractor's deduplicated
links or a new extractor. This issue implements typed variants and fixtures;
real MIME/link rules are later work. A future provider unable to establish a
source offset must report the containing MIME part instead of fabricating one.

Canonical order is the tuple `(rule_id, kind rank, typed location coordinates,
severity rank, fix_hint)`, using ASCII/UTF-8 byte order for strings, numeric order
for numbers and lexicographic numeric order for paths. Kind ranks are header,
mime_part, link. Header coordinates compare name then occurrence (null before
numbers); link coordinates compare path then offset. Remove exact duplicate
findings. Any change to rule membership/meaning, hints, parser interpretation,
location identity or ordering advances `rule_set_id`.

Initial version: `swarmail-demo/1`. Rule `demo.marker.v1` inspects only the first
root `X-Swarmail-Demo` field. After unfolding continuation lines to one space and
trimming ASCII space/tab, an exact lowercase value `info`, `warning` or `error`
emits one finding with that severity, header location
`{kind:"header",name:"x-swarmail-demo",occurrence:0}`, and hint
`Remove the X-Swarmail-Demo header.` Any other value, missing field or disabled
rule emits none. Later occurrences never override the first. The typed immutable
configuration is `{demo_enabled: bool}`, default true; no public configuration
endpoint or CLI flag is added solely for the demo.

The raw header scan accepts CRLF and LF, stops at the first empty line or EOF,
compares field names ASCII-case-insensitively, ignores non-field lines and orphan
continuations, and associates continuation lines only with the preceding valid
field. No charset conversion or lossy UTF-8 conversion precedes matching. A
malformed line terminates that preceding-field association. Non-ASCII values do
not match the three tokens. The complete header block is limited to 65,536
source bytes (including its terminating empty line when present); exceeding it
returns an analysis failure, even if a marker appeared earlier. Body size and
body syntax do not affect this rule. This bounded byte scan does not parse HTML.

## Execution, cache and failures

One report service per Store owns at most 128 cache slots and four admitted
blocking evaluations. Each key uses a Weak reference to the selected immutable
`Arc<Email>`, validated by upgrade plus `Arc::ptr_eq`, exact configuration by
value and rule-set id. No public message id, pointer integer, TTL or lossy digest
can establish identity. Cache state never enters Email, persistence or mutation
callbacks. Lookup always starts with a live store selection, before cache access.

Same-key misses serialize through one slot; a successful evaluation is reused.
An active or caller-held slot cannot be evicted. Evict the oldest unpinned slot
when full; if every slot is pinned, return unavailable. A job reserves one of
four permits before offload, with no queued blocking job on refusal. The worker
owns its permit and slot guard until completion, even if the caller cancels.
Waiting same-key callers use an async slot lock, not blocking-pool threads.
Ordinary failure leaves the slot empty; panic retains Store::offload's existing
unwind behavior and releases guards. No new panic-to-success conversion.

Retain at most 256 findings and 65,536 compact JSON report bytes per slot;
limits are checked without returning a truncated successful report. This bounds
cache payload, not total process RSS, caller-held responses, mail snapshots or
transport waiters. The demonstration emits at most one finding. No claim of
hard CPU isolation or bounded total store memory is made.

A request selected before deletion/clear/pruning may finish against its held
snapshot, just like existing reads. A request beginning after removal must fail
its live lookup; cache entries cannot resurrect it. Weak cache entries retain no
raw mail. Same-id replacements and restore create different objects; recovered
owned Emails mutated and reinserted cannot validate an old Weak identity. No
report changes oldest-first retention. Cache loss on restart costs recomputation.

REST uses existing JSON error shape: 404 for missing mail, 503 with fixed
`deliverability analysis unavailable` for capacity refusal, and 503 with fixed
`deliverability analysis failed` for analysis/size failure. Neither exposes raw
mail or parser diagnostics. Failure is never an empty report or a clean match.

## Surfaces and predicates

- `GET /api/v1/messages/{id}/deliverability`: report JSON, closed empty query
  domain; existing middleware/path/query admission precedes lookup/evaluation.
- MCP `swarmail_get_deliverability_report`: required string `id`, no other keys.
  On 2025-03-26 use the existing plain text result containing report JSON and
  existing `isError` operational failures; malformed arguments use `-32602`.
  Return report JSON through the existing text_result helper; do not add
  structuredContent/outputSchema or revision negotiation in this issue.
  Add structuredContent/outputSchema when #153 lands in a later, small follow-up
  through its shared result/schema owner. No resource or second formatter.
- Existing message UI page: server-rendered report section with rule-set id,
  severity, rule id, locator and hint; explicit empty and unavailable/failed
  states. Escape all strings and render link locators as text. No list warming,
  background polling, dashboard, new client bundle or automatic URL fetch.
- Existing REST assert/await and MCP wait gain optional
  `no_findings_at_or_above=info|warning|error`. Absent means existing behavior.
  Invalid, blank, differently cased, null or wrong-typed supplied values refuse
  before subscription or analysis. The option is not admitted on list/count.

The predicate qualifies each ordinary-filter candidate iff its successful report
has no finding at or above the inclusive threshold. An empty report qualifies.
Exact-count assert counts qualifying messages, including zero; it is not a
universal claim that every inbox message is clean. Wait still requires at least
one, including supplied count=0. Empty inboxes cannot satisfy waits. Preserve
newest-first store listing order, existing count/page caps and inert assert
limit. Qualify before limiting returned matches. Ordinary total fields retain
their existing unfiltered meaning.

Extend the existing wait owner. Validate and freeze inputs, subscribe, snapshot
ordinary candidates, release store guards, then analyze. Keep the subscription
through analysis; delivery notifications need not coincide with report
completion. Lag rescans; Closed resubscribes before scanning. One original
deadline and the existing shutdown race govern all slot waits and evaluations.
Never put analysis in synchronous Filter::matches or hold a map/mutation guard
across it. Scan candidates sequentially, not one spawned task per message.

For the new predicate only, an unfinished analysis scan at the deadline is 503
`deliverability analysis unavailable` (MCP operational error), not a fabricated
matched count. A completed insufficient scan followed by deadline expiry keeps
the existing timeout shape/count. A zero timeout allows a ready complete scan,
but cannot wait for cold work. Shutdown retains existing shutdown errors. Exact
asserts fail on any candidate analysis failure, never count partial successes.
All counts describe selected snapshots, not a whole-store transaction.

## #253 extension seam

Use a pure collector over raw bytes and immutable typed effective inputs, followed
by the one canonicalizer. A local fixture appends a second rule's findings to
prove the seam. No registry, dynamic plugin trait or resolver ships. #253 must
supply its complete immutable observation/policy snapshot and include its identity
in the typed cache key; changing external facts cannot reuse an old-input report.
It owns refresh/wakeup, expiry and operational-failure policy before introducing
live DNS. Future authentication must similarly identify envelope/client inputs.
No ambient network/time reads are permitted inside this issue's evaluator.

## Panel adjudication

| Conflict | Ruling and cost of rejecting the alternative |
|---|---|
| Message-owned slot versus bounded Weak cache | Adopt robustness's store-owned Weak identity and explicit bounds. Reject simplicity's Email field/reset-at-insert approach: it would enlarge constructors and require ingest ownership resets. Pay cache eviction/admission code and possible 503/recomputation; avoid coupling derived state to public Email clones/persistence. |
| Final liveness recheck versus selected snapshot | Adopt simplicity's existing snapshot boundary; reject robustness's post-analysis recheck. Pay the possibility that a previously selected read completes after deletion. Do not imply whole-store atomicity or new retention guarantees. Fresh lookups still refuse. |
| Missing Subject versus artificial marker | Adopt robustness's marker, limited to the first occurrence. Reject simplicity's missing-subject warning: it looks like a substantive mail-quality rule and exercises only warning. Pay one deliberately synthetic documented header. |
| Ordinal parts/links versus structural identity | Choose structural part paths; replace both proposed occurrence-based link schemes with an explicit source-byte offset. Pay a coordinate contract and provider fixtures; avoid an unspecified future tokenizer defining occurrence identity. |
| Unbounded shared offload versus bounded derived work | Adopt four permits, 128 slots and explicit limits, using existing offload. Reject an unbounded demo service and also a dedicated executor/general scheduler. Pay extra tests and busy responses; shared-pool/OS contention is still possible. |
| Separate pending UI/wait/provider machinery | Adopt simplicity's server rendering, one wait loop and static collector. Reject loading/polling jobs, persistent reports and generic provider frameworks. First view pays analysis latency; future heavy rules must justify their own design. |

Both positions' typed output, full input identity, truthful failures, strict
admission, no SMTP analysis and no dependency widening are retained. The
operator's later correction removes the newer-MCP prerequisite; current-main
text results are the complete MCP scope for this issue.

## Acceptance Criteria

These criteria prove the implementation locally on current main; they are not
claims that design has implemented it. Concrete test ownership is in
[tasks.md](tasks.md).

1. **AC1 — Typed contract:** fixtures pin all severity/location shapes, nullable
   header occurrence, empty/nonempty reports, source offsets, ordering and exact
   deduplication. Demo tests pin first-header/unfolding/invalid-byte behavior,
   fixed rule id/hint/version, inclusive size boundary and oversize failure.
2. **AC2 — Determinism:** identical raw bytes, effective config and version yield
   identical compact report bytes across independent instances, cold/warm reads
   and restart, despite changed arrival metadata. Changed config/version cannot
   reuse an old result. Rule-disable and severity fixtures demonstrate changes.
3. **AC3 — Ingest preservation:** SMTP DATA 250 precedes any report evaluation and
   permits immediate byte-exact retrieval; SMTP, seed and restore evaluate zero
   reports. Subsequent reporting leaves stored extraction/persistence unchanged.
   Existing burst/rate, chaos refusal and commit-failure proofs remain green.
4. **AC4 — Freshness/lifetime:** same-id replacement, clone/mutate/reinsert,
   recovered-owned-email reinsertion, delete/clear/prune and restart never yield
   a stale report through a new lookup. A selected pre-removal snapshot can
   finish. Cache entries do not retain raw messages or alter oldest-only pruning.
5. **AC5 — Resource/failure behavior:** deterministic barriers prove same-key
   coalescing, four-job refusal, 128-slot eviction/all-pinned refusal, report
   size/findings limits, cancellation retaining capacity, guard release on panic,
   failure retry and shutdown/deadline interruption. Failures never become clean
   reports. SMTP can commit and be queried while report workers are held.
6. **AC6 — REST:** real-server response/schema parity, 404, fixed 503s and closed
   query validation; invalid input invokes no evaluator. Served OpenAPI covers
   report variants, bounds, predicate domains and all documented statuses.
7. **AC7 — MCP:** real HTTP and stdio expose the same report as REST using
   main's 2025-03-26 contract: one text block containing report JSON and
   isError=false on success; existing isError operational failures and -32602
   malformed-argument errors remain. tools/list advertises the required id
   input and closed argument shape; tests cover empty/nonempty findings,
   missing mail, invalid inputs and sanitized analysis failures. The report
   descriptor has no outputSchema and its result has no structuredContent.
   Existing initialization/protocol behavior remains pinned. The later #153
   follow-up is documented and is not a prerequisite or local criterion.
8. **AC8 — UI:** real message pages display nonempty, empty and failure states
   with escaped fields; link locators do not create links. Reading the page does
   not modify stored fields or warm the inbox list.
9. **AC9 — Assert/wait:** real REST/MCP tests pin each threshold, exact-count
   assert including zero, minimum-one wait, filter-before-page, newest-first
   output, mismatch, timeout, shutdown, invalid admission and failure-not-clean.
   Owner-local interleavings prove subscribe-before-scan, delivery during
   analysis, lag recovery and Closed re-subscription without sleep synchronization.
10. **AC10 — Extension and delivery:** a deterministic additional-input fixture
    proves collector, serialization, threshold and cache-key agreement. No DNS,
    authentication/content/spam rules or external scorer is present. Current
    docs and proposed decision/index reflect the contract. Every local house
    gate and verifier addition passes on the implementation commit with exact
    zero missed lines/branches/functions and coverage selftest.

CI owns macOS, network RustSec, Helm/chart history, Docker, privileged port-80
exchange and tag-only release-rate proofs. Live hosting settings, namespaces,
sealing, publication and decision acceptance remain operator/CI owned, not new
local criteria. Recipe routing and protected review remain as this candidate
ships them. This seat never pushes, merges, tags, publishes or accepts decisions.
