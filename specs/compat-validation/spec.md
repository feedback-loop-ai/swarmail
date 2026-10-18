# Specification: compat and validation repairs

Created: 2026-10-18. Status: drafted for implementation; no implementation or
operator acceptance is claimed. Scope: run 21, issues #91, #95, #96, #99, #100
of epic #70 in the single swarmail repository.

Authority: the run-21 feature commission and the five issue bodies. The
`.brokkr/state/tasks/` directory contains older run-12 framing, not a run-21
task. Both current panel positions were read; their run-local evidence is
resolved below. Semantic amendments are proposed in
[0044](../../docs/decisions/0044-compat-validation.md); accepted decisions
remain unchanged. Only the operator may accept 0044.

## Feature and rationale

A parallel test harness must receive its own latest mail, get a refusal for
a misspelled assertion, and continue talking SMTP while another client reads
large compat pages. SMTP envelope provenance must survive restart without
inventing facts about old rows. Subject-only threading must recognize the
localized reply markers in #100 without changing explicit header chains.

The feature repairs the existing owners of those behaviors. It preserves
synchronous insert-before-250, subscribe-before-check, oldest-only pruning,
ingest-time extraction, native sender filtering, and the 250-item page cap.
No new dependency, configuration knob, authentication scheme, query engine,
store index, global memory controller or threading persistence is introduced.

## Panel synthesis and rulings

| Disagreement | Ruling and cost |
|---|---|
| Robustness's sender `None` for both bounce and legacy vs simplicity's provenance distinction | Adopt `envelope_from: Option<String>`: `None` is unknown legacy data, `Some("")` is a known null reverse path. Reject fallback to header From for a fresh bounce: it invents an envelope. Cost: three states must be tested and preserved through SQLite. |
| Robustness says MCP chaos `error: null` should be accepted | Reject: 0025 §2 exempts only per-event `Option<ChaosRule>`, not `ChaosRule.error`. Adopt simplicity's REST repair: present null strings refuse; omitted strings retain defaults. Cost: REST callers using null for omission must omit the key. |
| Typed MCP argument structs vs one schema-derived unknown-key check | Adopt simplicity's check against the existing tool descriptors, for every tool, plus closed native query structs. This retains 0029's staging owners. Reject a second typed argument registry. Cost: schema/runtime agreement needs a test. |
| Concurrent batch collection mechanism | Adopt a bounded `join_all` over the already admitted maximum 16 members (robustness); simplicity's ordered futures are equally sound but unnecessary here. Replies keep member order; effects do not. No spawned tasks or new semaphore. Cost: dependent mutations must be separate client requests. |
| Robustness calls the awaits registry an admission bound | Reject the premise: the 4,096-name cumulative metrics registry folds into `_uncounted`; it does not refuse concurrent waits. Preserve that behavior. The batch cap bounds one batch only. Cost: aggregate HTTP concurrency and watcher memory remain unbounded by this feature. |
| Filtered latest and deque-tail optimization | Adopt simplicity's explicit nonempty-query refusal and one-pass maximum over all scoped messages. Reject `run_search().first()` and robustness's `back()` shortcut: receive timestamps are sampled before parsing, not at commit (0038). Cost: O(n) scanning remains and filtered latest requires search followed by ID fetch. |
| Borrowed serialization alone vs a page byte budget | Adopt both borrowed views and robustness's additional budget, but measure actual serialized bytes with a capped writer, not guessed field multipliers. Cost: some formerly attempted MailHog pages now refuse; there is no guaranteed fitting positive limit for one enormous item. |
| Search with byte regex to eliminate invalid-UTF-8 copies | Retain the existing string regex and lossy raw semantics, but short-circuit on the first hit. Reject the byte-regex change because replacement-character searches would change. Cost: when raw is reached, one malformed raw message can still require a temporary lossy allocation. |
| Closed Mailpit prefix list vs every field-shaped prefix | Adopt simplicity's lexical refusal, with an explicit URL exception. A closed list would silently accept an unknown `foo:` field. Cost: literal colon words also refuse; this search does not implement quoting or field syntax. URLs containing `://` remain literal. |
| Localized subjects and optional numbered markers | Adopt the finite localized list, Unicode-safe slicing, and whitespace collapse agreed by both. Decline numbered `Re[n]:`, locale detection and persisted keys. Cost: numbered markers remain documented literal subjects and orphan keys can change. |

## Requirements and contracts

### #91 — request validation and pagination

`POST /mcp` retains its streaming 2 MiB body limit. It parses bytes:
malformed JSON or invalid UTF-8 gives HTTP 400, JSON-RPC -32700, null ID.
A body exceeding 2,097,152 bytes gives HTTP 413, -32600, null ID, with the
limit named. Never read an unbounded body to determine its size. Existing
Host/Origin refusals remain outside this envelope and unchanged.

A non-object single value gives HTTP 200, -32600, null ID. Within an
admitted batch, each non-object member (including nested arrays) contributes
one such error. Missing-ID object notifications still execute and stay
silent. Preserve explicit null/duplicate IDs. Empty or over-16 arrays refuse
as one null-ID -32600 before any member runs. Valid replies stay in input
order; notification-only envelopes stay empty HTTP 202. The shared classifier
also governs stdio: malformed members require replies and contribute null-ID
positions to transport errors. No jsonrpc-version validation is added.

Poll admitted members concurrently, with each wait retaining its own maximum
60,000 ms deadline, shutdown behavior and subscribe-first ordering. Sixteen
waits must not accumulate sixteen serial wait intervals. This bounds the
waiting component, not scheduling, serialization, storage or whole HTTP wall
time. There is no batch transaction or mutation ordering guarantee.

Every MCP tool refuses unknown argument keys with -32602 naming the key
before effects; descriptors publish `additionalProperties: false`. Preserve
the existing set-chaos wording and unknown-tool code. Native `ListQuery`,
flat `AwaitQuery` (including assert), and the new inbox pagination query
reject unknown keys with HTTP 400 JSON `error` naming the key. Preserve
assert's exact-count exemption and existing count=0 meanings.

Optional seed strings (`from`, `subject`, `text`, `html`) and chaos rule
`error` accept omission, refuse present null/non-string on REST and MCP,
and name the field. REST's wrong-shaped JSON remains 422; MCP's argument
error remains -32602. Per-event REST chaos null still clears that event.
No rejected seed, chaos change, clear, or wait has an effect or registers a
waiter. Refuse a client-supplied inbox that is empty after Unicode whitespace
trimming on MCP, native HTTP, UI, compat selectors and SMTP/POP3 username
admission. Omission still selects `default` where documented. Preserve
nonblank names verbatim; do not rename historical rows. An absent path
segment may retain the router's 404; a decoded blank segment must refuse.

MCP search gains nonnegative integer `offset`, default 0, uncapped like REST;
total is calculated before paging. New inbox pages at `/api/v1/inboxes` and
`/` use `limit` (default 100, allowed 0..250) and `offset` (default 0,
nonnegative, uncapped), sorted by the existing name order. The native JSON
array shape stays. Limit 0 yields an empty page; the UI supplies a usable
default-page link and does not emit zero-step next links. UI previous/next
links preserve positive limit and use checked arithmetic. Over-cap pages
refuse, never clamp. This bounds returned rows, not name enumeration/sorting.

### #95 — scoped latest

Mailpit `/api/v1/message/latest`, `/api/v1/message/latest/raw`, and
`/api/v1/message/latest/headers` share one validated resolver. Optional inbox
restricts the candidates; absence means global. The winner is the maximum
`(received_ms, id)` in scope. A scope with no mail yields 404 even if another
inbox has mail. A nonempty trimmed `query` yields HTTP 400 JSON error naming
`query`; absent/whitespace query is unused. The latest query shape accepts
only inbox/query. Ordinary ID lookup keeps its existing behavior. Hold the
chosen Arc throughout rendering; no cross-request snapshot promise is made.
The three required routes are the extent of this repair: plain/download and
attachment aliases are not added to the latest contract.

### #96 — compat work and memory

Mailpit snippets inspect at most `SNIPPET_SOURCE_PREFIX_BYTES = 8192` source
bytes per item for both text and HTML, cutting down to a UTF-8 boundary.
Prefer genuine text; otherwise render only that HTML prefix with the existing
renderer. Collapse Unicode whitespace and emit at most 200 characters plus
`...`; inspect at most the 201st collapsed output character. Add `...` when
either the source prefix or output is truncated, including a blank prefix.
Large leading markup/whitespace can produce a short preview. Stored bodies,
links/codes and the full plain/raw views remain intact.

All four compat list/search handlers move scan, filtering, page selection,
summary/body construction and JSON serialization into awaited `Store::offload`.
They return finished JSON bytes. Staging and refusals precede expensive scans;
no map guard crosses an await or is held while rendering/serializing. No
mutation lock is acquired for the read work. Compat default 50, limit=0 as
default, maximum 250, uncapped start and staged kind behavior are preserved.

MailHog v2 list/search retain all wire fields including `Content.MIME` and
top-level `MIME`. Serialize borrowed views into one output buffer; no owned
page-sized `Value` tree or body clones. A malformed raw source may use one
item's temporary lossy string, released before the next item. Enforce
`MAX_COMPAT_PAGE_BYTES = 67_108_864` (64 MiB) on the actual JSON bytes,
including escaping, repeated MIME fields and envelope overhead. Exactly the
budget is allowed. An attempted next byte refuses the entire response with
HTTP 400 JSON `error`:

`compat page exceeds byte budget 67108864; reduce limit or fetch raw by id`

No truncated success or partial body is sent. The limit applies to these two
MailHog pages, not single-message/raw responses or Mailpit summary pages.
It is a fixed limit, not a new option or a replacement for the 250-item cap.
Do not claim a global RAM bound: retained store Arcs, full-scope search,
temporary lossy raw strings and simultaneous requests remain additional costs.

`contains` returns after its first matching field, using the existing field
order, escaped regex, case semantics, compilation-size refusal and lossy raw
fallback. Raw remains the last candidate and actual search is never truncated.

### #99 — envelope provenance and compat contracts

Add nullable `Email.envelope_from` and SQLite `envelope_from TEXT` together.
Fresh SMTP stores the syntax-checked reverse path, including `Some("")` for
`MAIL FROM:<>`, before header fallback; shared REST/MCP seeds populate it via
the same builder. Old rows restore `None`; there is no backfill or MIME
reparse. Fresh files, populated older files, repeat opens and new writes all
preserve ordering, counters and raw bytes through the existing migration and
commit boundary. JSON mirrors expose the additive nullable field.

One compat accessor uses envelope_from when known, otherwise historical
header From. Apply it to MailHog From/Raw.From/kind=from, Mailpit ReturnPath,
and synthesized Return-Path. Fresh null reverse paths never use header From.
Native `Filter.from` continues matching header From. Preserve `Filter.to`
matching To/Cc headers plus envelope recipients; document that it is not a
header-presence assertion in README, OpenAPI, MCP descriptions and llms text.

For generic compat containing searches, after trimming, examine each
whitespace-delimited token. A leading ASCII identifier
`[A-Za-z][A-Za-z0-9_-]*:` is unsupported field syntax, refused with HTTP 400
and that identifier named (case-insensitive recognition, preserve supplied
spelling in the message). Exempt a token whose same colon is followed by
`//`, so `https://host/path` is literal; interior URL colons are not rescanned
as token starts. This includes unknown fields, leading spaces, mixed case,
and later tokens. There is no quoting/escaping grammar. Explicit MailHog
`kind=to|from|subject` continues taking a literal value with the existing
staging/refusal behavior; it is not parsed again as generic field syntax.

Rename Mailpit search `messages_unread_count` to `messages_unread`; all mail
remains unread. Document `/api/v1/delete-all` as swarmail's explicit global
wipe extension and bodyless MailHog DELETE messages as refused by 0011.

### #100 — subject fallback

Repeated leading markers recognize Re, Fw, Fwd, Aw, SV, VS, Antw, WG, TR, RV,
Odp, R, I, ENC, RES, Ответ and 回复, with Unicode lowercase comparison,
optional Unicode whitespace before the delimiter, and ASCII or full-width
colon. Every removed marker strictly shortens the slice; all slices use
character boundaries. Collapse all remaining whitespace runs to one ASCII
space, trim, then use the existing lowercase fallback key. Empty results use
the existing singleton key. Numbered Re[2]/Re[10]/Aw[2] stay literal.
Keep original subjects and all Message-ID/In-Reply-To/References precedence.

## Acceptance Criteria

- **AC1 (#91):** Real HTTP probes pin malformed JSON, invalid UTF-8, exact
  body-limit boundary and chunked over-limit input without Content-Length;
  scalars/null/strings/nested arrays and mixed batches produce the specified
  codes, HTTP statuses, IDs and JSON media type. Empty/17-member batches
  execute no effects. Notifications execute once with no reply.
- **AC2 (#91):** Shared classifier and real stdio tests preserve ordered
  duplicate/null IDs and invalid-member null placeholders on successful and
  failed exchanges. Concurrent waits plus a satisfying seed in one batch
  complete without serial timeout; a deterministic readiness test observes
  every wait configured at 60,000 ms registered before releasing deliveries,
  proving their waiting intervals overlap. Existing per-wait deadline tests
  and above-60,000 refusal pins remain green, as do shutdown and `_uncounted`
  folding tests; no flaky elapsed-time upper-bound assertion is substituted.
- **AC3 (#91):** Every tool rejects a sentinel unknown key, including
  zero-argument tools. A typo clear cannot clear default; typo assert cannot
  pass; typo await cannot subscribe. Native unknown-query errors name the
  key. Missing/null/wrong-type/valid-string tables cover every optional seed
  string and chaos error, with unchanged counts/config on refusal and
  per-event null clearing preserved.
- **AC4 (#91):** With 251+ messages MCP offset reaches older mail, preserves
  total, and returns empty beyond end; malformed/negative/null offset refuses.
  With 251+ named inboxes, API/UI pages are sorted, disjoint and complete when
  traversed; default, 0, 250, 251, huge offset and UI links are pinned. Blank
  inbox tests cover every admitting surface and prove no stored blank inbox,
  waiter, rename, or destructive effect.
- **AC5 (#95):** Two-inbox tests cover all three latest routes, absent/empty
  scope, global scope and named query refusal. Controlled fixtures reverse
  insertion and received-time order within one inbox and pin equal-time ID
  ties. `/raw` is byte-exact; headers belong to that resolved mail. Ordinary
  IDs still work. The implementation retains only the running winner.
- **AC6 (#96):** Instrumented snippet tests show inspection bounded by 8192
  source bytes for long text, HTML and all-whitespace inputs; multibyte cuts,
  entities, unfinished tags, no body and exactly 200/201 output characters
  obey the truncation contract without mutation of stored fields.
- **AC7 (#96):** A per-instance cfg(test) gate parks compat response building
  on a current-thread runtime. Real SMTP returns its 220 and health responds
  while the gate is still held; release and join finish every request. A
  cleanup deadline independent of that runtime prevents mutant deadlock.
  Run concurrent Mailpit/MailHog lists/searches through this proof.
- **AC8 (#96):** MailHog text-only/HTML-only/multipart/missing-body and invalid
  UTF-8 fixtures preserve the complete JSON shape. A capped-writer test at
  small injected budgets proves exact fit and next-byte refusal, including
  JSON escapes and duplicate MIME. A real over-64-MiB page refuses by name;
  smaller pages or raw-by-ID remain usable. Review verifies no body-sized
  Value tree; record isolated before/after peak-memory and SMTP greeting
  measurements without platform-dependent timing assertions.
- **AC9 (#96/#99):** An early-field match avoids visiting raw; raw-only and
  invalid-byte replacement-character searches keep their results. Prefix
  tests cover known and unknown identifiers, spaces, mixed case and later
  tokens; literal URLs and explicit kind values remain searchable. Invalid
  kind/page inputs retain existing staging and cap errors.
- **AC10 (#99):** Real SMTP header/envelope divergence, null reverse path,
  native header filter and compat envelope filter are pinned; both seed
  transports populate the field. A populated old-schema fixture migrates
  without loss, restores unknown rather than fabricated envelope values,
  accepts new mail, and reopens idempotently. Restart preserves all three
  sender states; restore query-plan, lock/sidecar and 5 s pragma tests pass.
- **AC11 (#99):** Native REST/MCP recipient tests match envelope-only Bcc;
  docs/schema tests pin that predicate, nullable sender field, unread key,
  wipe refusal, latest limitations, snippet/budget costs and batch ordering.
- **AC12 (#100):** Table-driven cases cover every listed marker, repeated
  markers, both colons, uppercase Cyrillic, tabs/NBSP/multiple spaces, lone
  colon/high codepoint, marker-only and numbered nonmarkers. Header-bearing
  counterparts keep their exact keys and chain membership, independent of
  subject; orphan variants coalesce on the wire.
- **AC13 (delivery):** At least one behaviorally failing-before/passing-after
  regression per child is recorded; compilation failure is not a red proof.
  All six house gates pass, including exact 100/100/100 coverage, with no
  exclusions, weakened bounds, dependency changes or accepted-ADR rewrites.

## Boundaries and accepted limitations

This is a design handoff, not implementation. Existing latest/plain alias
gaps, numbered subject markers, aggregate memory exposure, full-scope scan
costs, blocking-pool contention and lack of cross-request snapshots are
explicitly retained. The new page budget can refuse one large item even at
limit=1; fetch its raw source directly. New strict validation can reject
clients that previously relied on ignored fields or null-as-omission.
