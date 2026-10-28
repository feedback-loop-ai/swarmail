# Focused mail waits

Date: 2026-10-28. Epic #164, issue #154. Design on a provisional stack.

Decision: 0094 — [Focused mail waits](../../docs/decisions/0094-focused-mail-waits.md), proposed.

## Feature and rationale

Agents need a code belonging to the newest eligible delivery and a link whose
stored context matches their request. Reading the first message or guessing a
URL from a first page hides stale codes and competing destinations. Add
`swarmail_wait_for_code` and `swarmail_wait_for_link`, using occurrence metadata
extracted once at ingest and complete retained snapshots. Ambiguity is an
explicit successful answer; it never silently chooses an application action.

This additive public tool and persistence contract preserves
[0001](../../docs/decisions/0001-synchronous-insert-lossless.md),
[0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md),
[0003](../../docs/decisions/0003-oldest-only-pruning.md),
[0004](../../docs/decisions/0004-per-test-inboxes-via-smtp-auth.md) and
[0005](../../docs/decisions/0005-extraction-at-ingest.md). Compatible work may
proceed under [0045 §8](../../docs/decisions/0045-delivery-repairs.md); no proposal
accepts itself or permits weakening those guarantees.

## Evidence and authority

The commissioned base is commit `25b367de5e7920666bebd96a7c5216f60c8cfd1a`,
tree `91c8ada3f36d396e824c06f800f6a501bf577bec`; both were checked. It is
provisional #153, not a merged or protected-reviewed prerequisite. Available
local main is `39a04d0f092d7a0a55dafc684fe2df4cfa81c497`. The separate OPTIONS
repair is visible at `f0b2b14b5cbbac663dbdb5349c8a0a600e7a95d9`; a visible commit
is not proof of final reviewed disposition. Final delivery must reconcile
reviewed #153, #152 OPTIONS and then-current main and rerun proofs on that tree.

Both run-local simplicity and robustness positions were read. The requested
`.brokkr/state/tasks/` is absent; framing comes from the supplied task and intake
result `27784653-e22a-49fe-9a5f-1b8c920c45a1`. All supplied issue154 SHA256SUMS
entries passed independently:

| Evidence | SHA-256 |
| --- | --- |
| operator-rulings.md | 5b8da87b897306a313f673b134edd1a56e0f477117a07618e3ec345449a6c6f9 |
| README.md | 299376b35dd3f56652f5d008a5e16c00691c93dc6bd892202d6e52d4678157e1 |
| review-b5564ae6.md | 8a38a8f3b6016237aa901cae0180239e5e2956e8e88ffa528523da4586e83695 |

Historical `issue154-after-153.patch` independently matches
`e84cfe9c8049c0e43ba6b664838a13f6f5b5d8eeb36af1569ff932d5610cd691`. Its
functional changes, later repairs and all five historical reviews are evidence,
never current verdicts. Its 0084 allocation, fixture replacements and broad Email
serialization changes are not imported. No tracked link points into run-local
state. The later 2026-10-26 Phase 1 ruling authorizes #154 development on #153,
held for #151's operator-owned baseline and possible reprioritization.

The historical-spec ruling requires corrections through decisions, preserving
old specs. Packet acceptance provenance for 0079/0080/0082 and their locally
proposed headers remain distinct; this design changes neither statuses nor
accepted history. 0094 supplements
[0082](../../docs/decisions/0082-mcp-structured-output.md) and
[0091](../../docs/decisions/0091-mcp-report-output.md)'s older tool inventories.

Local branch-tip decision inventories were checked. #155 owns proposed
0092-compact-email-view on `epic192/next-155`; `repair/issue152-options-mixed`
independently carries proposed 0092-mcp-candidate-proof, an existing collision.
#156 owns proposed 0093-near-miss-timeouts on `epic192/next-156`. No local branch
tip exposed 0094. Use 0094 provisionally, recheck concurrent allocations before
implementation/delivery, and leave resolution to the operator. Do not replace or
accept #155/0092 or reuse historical 0084 and reserved gaps.

## Panel rulings

| Dispute | Ruling and cost |
| --- | --- |
| Public Email metadata | Adopt simplicity: internal persisted occurrences exposed by the new link tool. Reject robustness's modern Email enrichment and March-specific Email projection. Existing reads/webhooks/feeds retain their shapes; consumers needing labels must use the new tool. This avoids multiple new public projections. |
| Shared wait refactor | Adopt robustness's narrow focused path beside the report owner. Reject simplicity's immediate unification. A small duplicated lifecycle costs maintenance and paired tests, but avoids imposing focused terminal evaluation on cold report deadlines. Both new tools share one focused owner and the existing watcher/offload/shutdown mechanisms. |
| Ownership and restore | Adopt both panels' source-to-occurrence ownership and robustness's strict corruption refusal. Reject equal-URL/latest-cursor attribution and silent corrupt fallback. Pay bookkeeping and damaged-column startup refusal rather than conceal ambiguity. |
| Timeout | Adopt both panels' terminal classification before capture. Reject timeout from an old snapshot after awaited eligibility. Pay one extra scan and possible completion beyond timeout_ms; no hard CPU deadline or strict arrival cutoff is promised. |
| Presentation | Adopt full-snapshot selection, exact totals and 250 entries per list. Adopt simplicity's omission of a separately sampled unfiltered total; keep same-snapshot eligible-email matched count. Reject first-page uniqueness and unbounded lists. Narrower filters recover omitted entries; item bounds do not bound bytes/work. |
| Proof | Adopt robustness's independent corruption, interleaving, filter and negative controls. Reject reducing tests to shrink the diff. Adopt simplicity's finite schema literals/existing dependencies over generators, runtime validators or services; accept explicit drift-proof maintenance. |

No DOM, CSS/accessibility inference, trust scoring, URL fetch, background
extraction, cache, durable wait/outbox, retention redesign, #155 compact views
or #156 near-miss diagnostics belongs here.

## Ingest and persistence contract

`Email.link_metadata` is an internal optional ordered vector, skipped by the
existing Email serializer on every surface/revision. Fresh SMTP/shared seed
construction sets Some(vector), including Some([]). Historical rows use None;
they retain stored links, codes, bodies and raw bytes without backfill, reparse,
truncation or rewriting across repeated restarts.

Each persisted occurrence is a closed object with all four members required:
`url: string`, `anchor_text: string|null`, `rel: string|null`,
`source: "text"|"html"`. Missing nullable members are corrupt, not implicitly
null. SQL NULL alone means unavailable history. Invalid non-NULL JSON, JSON null,
wrong shapes/types, unknown fields or source values fail restore through the
existing corruption channel. A valid empty vector means known no occurrences.

Retain occurrences before exact-string deduplication, text first then HTML source
order. Identical destinations with different labels/rel/origins remain separate.
First-occurrence URL projection equals legacy fresh links exactly. Preserve code
MIME ordering and the 64-distinct fresh-ingest cap, historical array exceptions,
first genuine bodies, attachment exclusion, direct-alternative preference,
conditional-comment URL fidelity, punctuation and quoted-attribute handling under
[0031](../../docs/decisions/0031-sender-honest-stored-bodies.md),
[0049](../../docs/decisions/0049-extraction-compat.md) and accepted
[0061](../../docs/decisions/0061-compat-webhook-proof.md).

Use the existing tokenizer on original source bytes. Tag/attribute names are
ASCII case-insensitive. Only a complete unsuppressed a start tag establishes an
anchor. First href/rel win; a valueless attribute is present-empty. Attribute
order cannot alter attribution. Label/rel attach only to the exact occurrence
wholly owned by that href, whose once-decoded ASCII-trimmed value equals one
whole extracted URL. Multi-URL/malformed hrefs remain unlabelled. Ordinary
regions flushed before later quoted attributes retain ownership; neighbour URLs
and repeated destinations cannot steal it.

One active lexical anchor collects descendant text, decodes each run once,
collapses/trims ASCII whitespace and preserves Unicode/case. Inline tags add no
space; start/end p, div, li, tr, h1–h6 and start br add separators. Existing
suppressed head/template/script/style/title content and comments add no text.
Finalize on closing a, replacement a or end of source; incomplete start tags
never create anchors. Self-closing spelling follows current lexical start
semantics. Known empty label is "", unknown is null. Rel decodes once without
token/whitespace normalization; missing differs from empty. Text-part,
source-only, conditional-comment and unattributable URLs keep null label/rel.
No alt text or browser rendering is inferred. Bound association/label work by
source size; do not copy a long label onto every malformed-attribute URL.

Extraction finishes in offloaded build_email shared by SMTP/seed under
[0010](../../docs/decisions/0010-exact-count-asserts-and-the-shared-seed.md), before
the mutation boundary. One nullable SQLite JSON column uses the existing
migration owner. Metadata, mail, sequence, oldest-only evictions and counters
commit together before memory apply and nonblocking fan-out. Failed commits
create no mail/index/counter/wake trace; SMTP 250 still follows storage success.

## Inputs and wait semantics

Both tools accept only optional inbox, to, from, subject, since_ms, timeout_ms;
link additionally accepts anchor_text, host, path. Omitted inbox is default;
supplied names use current verbatim validation. To matches to/cc/envelope
recipient addresses, from matches stored From address, subject matches stored
subject, using existing case-insensitive substring semantics. since_ms is an
inclusive signed-i64 parse-start timestamp lower bound. Filters AND together.
No count, limit, offset, TTL, report predicate or fresh-only option is added.
Timeout defaults to 5000 ms, permits 0, and caps at 60000.

All closed-shape, supplied-null/type, filter and selector validation finishes
before readiness parking or watcher registration. Unknown keys, invalid names,
filter types and selectors retain named value-free JSON-RPC -32602; timeout
shape/domain errors retain existing text isError refusal. Rate admission,
negotiation and operational error taxonomy stay unchanged.

Selectors use Rust regex search syntax, each at most 1024 UTF-8 bytes, compiled
once per request with a 1 MiB compiled-size limit. Empty patterns are legal but
cannot match unavailable values. Anchor/path default to case-sensitive Unicode
mode; host defaults to ASCII-insensitive bytes mode with Unicode disabled.
Supported inline flags retain library semantics; never lowercase patterns.
All supplied selectors must match the SAME occurrence.

Parse stored URLs locally only when a host/path selector needs it. Host uses the
existing URL parser's spelling without credentials/port, including its IDNA and
bracketed IPv6 behavior. Path is serialized path without query/fragment or extra
percent/entity decoding. An invalid URL or absent needed component fails that
selector. Unconstrained candidates need no parse. Exact host intent uses
`^example[.]test$`; search is not implicit equality. No network operation occurs.

The focused owner subscribes before initial capture and resubscribes before
capture after Closed. Wakes/lag rescan retained truth. Retain the receiver during
offloaded eligibility, releasing map/mutation guards before work. Set one
monotonic deadline before initial subscription. Classify each scan terminal
BEFORE capture. Eligible selected OR ambiguous results return immediately.
An ineligible nonterminal scan finishing after expiry takes one fresh terminal
snapshot. That evaluation returns selected/ambiguous/timed_out once; advancing
time again cannot cause endless rescans. Zero timeout has one terminal scan.
Unusable wakes never renew the deadline. Shutdown races the call and releases
its readiness park; already-started blocking work may finish later.

Preserve wait_for/wait_for_report, count=0/minimum-one wrappers, report failures,
zero-timeout ready-only access, cache/worker bounds and expiry without new cold
analysis. Focused tools never analyze reports. Final snapshots may include a
commit slightly after the nominal deadline; retained Arc snapshots may finish
after pruning. No reservation, durable wait or exact arrival cutoff is promised.

## Selection and output contract

Use one complete filtered retained snapshot before display caps. Native per-inbox
acceptance order, including seq/rowid restore order, defines newest; timestamps,
UUIDs and compat sorting cannot override it. Code eligibility means nonempty
stored codes. Within each email deduplicate codes preserving order for this
response only. One distinct code in the newest eligible email selects; multiple
codes there are ambiguous, never falling back. Older eligible emails contribute
codes in acceptance order then stored order; equal codes across emails retain
separate attribution.

Link eligibility means at least one matching occurrence. Exactly one distinct
exact URL across all eligible emails selects; several are ambiguous. Different
spellings stay distinct even when parsing normalizes them equally. Candidates
retain all matching occurrences, newest email first then occurrence index.
Historical None adapts stored links with null anchor/rel/source and legacy array
indices. Fresh empty metadata never falls back to legacy links.

A code candidate has exactly code, email_id, received_ms, age_ms, sender. A link
candidate has exactly url, email_id, link_index, anchor_text, rel, source,
received_ms, age_ms, sender. Strings/nulls follow the stored definitions; source
also permits null for historical adapters. Sender is stored Email.from or
explicit null: required address and optional name with existing EmailAddress
omission rules. It can reflect the current envelope-sender fallback; it is NOT
guaranteed header-only identity. Do not reparse or infer a sender.

Sample response wall time once. Compute age_ms = max(0, now_ms - received_ms)
with widened arithmetic, without signed overflow, represented in 0..u64::MAX.
received_ms spans i64. Age is parse-start age, not code expiry or acceptance
order. Counts/link_index are nonnegative integers within the serialized u64
domain. Old matching mail may return immediately; callers own inbox isolation
and a suitable since_ms boundary for their application action.

All objects are closed. Every common member below is required in every status.
Status is exactly selected, ambiguous or timed_out. Matched counts eligible
emails in this snapshot, not occurrences or unfiltered inbox mail.

| Tool | Common fields | Selected-only field |
| --- | --- | --- |
| code | status, matched, candidates, candidates_total, candidates_truncated, older_candidates, older_candidates_total, older_candidates_truncated | selected: code candidate |
| link | status, matched, candidates, candidates_total, candidates_truncated, distinct_urls | selected_url: string |

Code candidates describes the newest eligible email; older_candidates describes
older eligible emails. Link candidates describes matching occurrences and
distinct_urls counts exact destinations. Cap each array independently at its
first 250 entries, computing exact totals/selection over ALL input. Truncated
is true iff total exceeds displayed length. Bound presentation construction
during scanning, not after constructing every response object. Complete scans
and a distinct-URL set still cost work/memory proportional to retained input.

Selected code requires candidates_total=1 and selected=candidates[0]; ambiguity
requires candidates_total>1. Selected link requires distinct_urls=1; ambiguity
requires distinct_urls>1 even when a second URL occurs beyond entry 250.
Selected/ambiguous require matched>=1. Ambiguous/timed_out OMIT selected or
selected_url. Timeout has matched=0, empty arrays, zero totals/distinct_urls and
false truncation. No independently sampled inbox total, cursor or byte truncation
is added. The unchanged 10 MiB bridge cap can refuse even one huge entry: preserve
correlation/continuation without dropping structuredContent or retrying actions.

## Revision contract

Thirteen current tools plus these two are exactly fifteen. Preserve existing
descriptor order/entries; append the additions. June 2025-06-18 and November
2025-11-25 advertise complete finite object-root outputSchema for every tool,
with readOnlyHint=true for both additions. Every success uses one captured Value
for text JSON and matching structuredContent, with isError:false. Errors remain
separate and notifications silent. Close variants, require emitted members,
preserve exact ranges/null/omission and 250-item caps; cross-field counts/order/
truncation require producer proofs as well as schemas.

March 2025-03-26 and absent-version defaults stay text-only. Existing results and
descriptor entries retain independently captured pre-change bytes, including
fresh mail carrying metadata. tools/list changes only through additive discovery:
compare the original thirteen-entry subsequence, not full fifteen-tool byte
identity. Preserve both existing March captures and provenance; add independent
new-tool expectations. Never rewrite old goldens to manufacture compatibility.
Keep 0091 report unions, numeric bounds, OpenAPI pins and finite evaluator
unknown-assertion refusals. Negotiation is not full MCP conformance or actual
client validation.

## Acceptance Criteria

1. **AC1 — Extraction:** exact ordered occurrences and legacy arrays pass through
   SMTP and seed for text/HTML, repeated URLs/different labels, rel, missing/empty
   labels, nested/block/br text, both attribute orders, unquoted href before
   quoted rel/title, neighbouring URLs, malformed/suppressed/conditional comments,
   Unicode/one-decode punctuation and selected MIME versus attachments. Existing
   code/link order and work counters remain valid; metadata is source-proportional.
2. **AC2 — Persistence:** genuine old-schema migration, SQL NULL versus [], fresh
   nonempty metadata, corrupt non-NULL representations (including missing nullable
   fields), exact historical bytes/arrays and two restarts pass. Failed insert
   under cap pressure leaves visible/durable state and wake counts unchanged.
3. **AC3 — Codes:** newest acceptance-order code, duplicate/older provenance,
   newest-multicode ambiguity without fallback, future/negative timestamps and
   overflow-safe age pass. Reversed timestamp/ID and restart witnesses prove order.
4. **AC4 — Links/bounds:** same-occurrence AND, parsed components, exact-host
   controls, unknown labels, regex limits, no matches, repeated single URL and
   multiple URLs pass. A second URL beyond entry 250 still makes ambiguity; every
   list has exact totals/truncation. Historical large code arrays prove caps
   follow complete semantic evaluation.
5. **AC5 — Admission/lifecycle:** invalid inputs register nothing. Deterministic
   per-Store barriers/clock prove subscribe-window delivery, Closed resubscribe,
   lag, ineligible floods, fresh terminal capture after awaited scan crosses
   expiry, zero timeout and shutdown after actual park. No 1 ms assumption,
   sleep synchronization or immediate task-finished assertion counts as proof.
6. **AC6 — Wire filters/isolation:** real SMTP/MCP proves to/from/subject/since_ms
   independently for EACH tool, with unique counterexample identities, stored
   timestamp boundaries and positive/negative controls. Acknowledged post-park
   unusable/foreign delivery is proved with owner interleavings plus terminal
   wire outcomes; target eligible mail alone completes selection.
7. **AC7 — Protocol/schema:** exact fifteen-name parity and all focused success
   variants plus history/errors pass over HTTP AND binary stdio for BOTH modern
   revisions. Independent schema negatives reject wrong status, missing/extra
   fields, bad nulls/ranges and selected-on-ambiguity. All thirteen old tools,
   report forms/pins, modern text equality, mixed exchanges, errors, notifications
   and independent March/default bytes remain correct.
8. **AC8 — Traps:** real local application consumers use returned code/URL to
   complete stale-code and look-alike-link tasks. Independently selecting stale
   or decoy candidates fails the application action/trap assertion; status-only
   assertions are insufficient. Fixture evidence remains separate from suite-wide
   standalone eval, paid models and actual-client receipts.
9. **AC9 — Preservation/reconciliation:** report waiting/analysis and SMTP DATA,
   chaos/stop/TLS/raw, webhook and oldest-only retention witnesses remain green.
   Reconcile reviewed #153, separate #152 OPTIONS, current main and decision
   collisions before final delivery, preserving historical artifacts.
10. **AC10 — Proof ownership:** execute all local house gates and supplemental
    pinned design/delivery proofs against the committed implementation with
    logs/statuses/tree identity. Exact coverage has zero missed lines/branches/
    functions and passes selftest. Engine verify and five protected reviewers
    supply their own verdicts; missing evidence remains owed.

## Delivery and limitations

Carry the inherited HIGH security-relevant review-integrity/result-attribution
residual unchanged. Do not reproduce attacks, widen protected boundaries, forge
results, re-pin the engine or infer an unpublished fix. Retain Terra 6.1 intake,
Opus 5.5 implementation, Astra correctness/guarantees, inherited independent
Claude and official Flash security, and DSH Spark GLM-5.3-Flash tests. The chief
does not substitute for protected members.

Standalone eval suite-wide same-user coordination remains unproved under #201;
a private lock or fixture pass is not a substitute or receipt. #151's paid
baseline is operator-owned and a phase-one merge prerequisite. No paid trials.
CI owns platform/network/chart/Docker/port-80 proofs; actual-client receipts and
operator seal/merge remain separate. Track stale_code/wrong_link task success
and later tokens-per-success/invalid-tool-use rates without claiming performance,
token, model-success or prompt-injection-safety improvements here.

No push, merge, publication, tag, issue closure or decision acceptance is
commissioned. This design yields only drafted; implementation/delivery remain
unproved. Historical passes cover their named trees only.
