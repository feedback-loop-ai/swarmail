# 0044 — Validate requests and bound compat page work without losing provenance

- **Status:** proposed — acceptance is the operator's.
- **Date:** 2026-10-18.
- **Scope:** run 21, issues #91, #95, #96, #99, #100.
- **Spec:** [compat-validation](../../specs/compat-validation/spec.md).
- **Proposes amendments to:** [0029](0029-supplied-values-are-validated-wherever-they-reach-a-wire.md)
  §§2/4/7 (non-object replies, concurrent batches and closed tool arguments),
  [0030](0030-loopback-plane-bounded-results.md) §4 (short-circuit search),
  [0031](0031-sender-honest-stored-bodies.md) §3 (snippet cost per item).
  Extends 0030 §3 with a distinct MailHog response-byte bound while retaining
  its 250-item cap. Updates proposed [0039](0039-stdio-concurrency.md) §§2–4's
  scalar-silence and sequential-server-batch descriptions.
- **Preserves:** 0001–0005, 0011, the ordered mutation boundary, 0025's
  null/omission distinction, raw byte fidelity, and native Filter.from.

## Context

The existing control plane can silently discard invalid batch members and
ignore destructive-tool typos. Latest lookup leaks across requested inboxes.
Compat summaries scan full bodies and MailHog builds duplicate owned bodies
before serialization. Envelope senders are not persisted, and localized
subject markers split orphan threads. These are observable contracts and
accepted-ruling changes, so code repairs alone are insufficient authority.
Accepted documents are left intact; this record does not mark them amended
or claim acceptance. The linked specification fixes every boundary below.

## Proposed ruling

1. Parse bounded MCP body bytes. Invalid JSON/UTF-8 yields HTTP 400/-32700;
   streaming overflow above 2 MiB yields HTTP 413/-32600 naming the limit;
   both have null ID. A non-object parsed value is invalid, not a
   notification: answer HTTP 200/-32600/null, once per invalid admitted
   member. Share classification with stdio, including null-ID transport
   error positions. Object notifications keep effectful silence; empty and
   over-16 batches refuse before effects. No version-member check is added.
2. Poll at most 16 admitted batch members concurrently and collect replies
   in input order. Each wait owns its existing <=60 s deadline; batch wait
   time is not their serial sum. Effect order is unspecified. This amends
   0039's explicitly sequential HTTP-batch sentence, rather than pretending
   its concurrency across stdio lines already ruled server batches. The
   registry's 4,096-name cap only folds cumulative metrics into `_uncounted`;
   it neither limits active waits nor rejects admission. No global admission
   bound or total wall-clock latency guarantee is created.
3. Close every MCP tool argument shape using its existing descriptor's key
   set, and native list/await/assert/inbox-page queries using serde's closed
   structs. Unknown keys refuse by name before effects. This closes 0029
   §7's named typo residual without adding a typed Args framework. Supplied
   null seed strings and ChaosRule.error refuse on both planes; only the
   already-decided per-event ChaosConfig null means clear. Page inbox API
   and UI results at default 100, maximum 250, with offset and the existing
   native zero-size meaning. Reject blank-after-trim inbox names at admission,
   without renaming nonblank values or rewriting old rows.
4. Latest is maximum (received_ms, id) over every message in the requested
   scope, with a single retained Arc, no sorted page and no mutation lock.
   It is not the deque's tail: parse-start time can differ from commit order.
   Apply one resolver to Mailpit message/raw/headers; empty scope is 404,
   nonempty trimmed query is a named 400, no inbox means global. Plain and
   attachment aliases are not added. Separate calls can observe different mail.
5. Snippets inspect a UTF-8-safe prefix of at most 8192 source bytes per
   item, for text and HTML, and emit 200 collapsed characters plus a
   truncation suffix. Up to 250 items means up to 250 bounded prefix renders,
   not 0031's claimed one render per page request. Full plain views keep
   their single-body render; neither path extracts links/codes or writes back.
   All four compat list/search builds, including serialization, run through
   awaited Store::offload without map guards held during rendering.
6. MailHog v2 pages keep their complete shape but serialize borrowed views
   into one capped buffer. The maximum actual JSON output is 67,108,864 bytes,
   inclusive, with escaping and duplicate MIME counted. Refuse the whole
   page at the first byte beyond it: HTTP 400 JSON error `compat page exceeds
   byte budget 67108864; reduce limit or fetch raw by id`. Do not clamp items
   or emit partial JSON. The spec's budget applies to MailHog v2 list/search
   only. One item may exceed it; raw-by-ID remains available. This is not a
   global memory bound. The writer counts bytes actually emitted, not an
   estimate that could omit fields or undercount escaping.
7. Generic contains stops on its first hit while retaining existing regex
   folding and lossy raw semantics. Invalid raw may still allocate a temporary
   string when reached; byte-regex replacement is declined. After trimming,
   generic containing queries reject every whitespace-token-leading ASCII
   identifier `[A-Za-z][A-Za-z0-9_-]*:` by name except a colon followed by
   `//`. URLs remain literal; other colon words may refuse. Explicit
   kind=to/from/subject values retain literal staging. No query grammar is
   implemented and unknown identifiers cannot silently return zero matches.
8. Store envelope_from as nullable text: None means unknown historical
   provenance, Some("") means a known null reverse path, other Some values
   are the actual envelope. Capture at ingest, including seeds; persist in
   the same ordered insert, migrate additively without backfill. Compat
   sender projections/filter use this field with header fallback only for
   None. Native from remains header-based. Existing To/Cc/envelope-recipient
   matching stays and is documented on every affected plane. Rename the
   unread key to messages_unread and correct wipe documentation under 0011.
9. Extend only the view-time subject fallback with the spec's finite marker
   set, Unicode-safe delimiter slicing and whitespace collapse. Header-based
   threading stays authoritative. No persisted derived key, alias table,
   numbered marker grammar or historical subject rewrite is introduced.

## Alternatives, consequences and proof

The spec's synthesis table records adoption and rejection of both panel
positions, including the two erroneous robustness premises about null and
the registry. The added byte budget is chosen over borrowed-only output:
one copy of a potential 55 GB response is still too large. Its cost is a new
compat refusal and a small writer with exact-boundary tests. Estimated
preflight size/fit suggestions are declined because JSON escaping and one
oversized item make them unreliable. Responses are bounded; store memory,
full-scope candidate lists, per-item lossy conversions, and request concurrency
are not. Blocking-pool contention with ingest remains possible.

Unknown keys, null-as-omission, literal colon words and oversized pages lose
their former permissive behavior. Filtered latest is explicitly unavailable;
clients search and fetch an ID. Legacy sender fallback is approximate but
is never stored as provenance. Orphan thread keys can change. These costs
are public documentation requirements, not hidden implementation choices.

Spec AC1–AC13 require real HTTP/SMTP/stdio proof, migration and restart tests,
deterministic offload and batch concurrency tests, one behavioral red per
child and all six gates. Follow accepted 0020 placement and the 0035 gate;
no test-only seam ships in production, no floor or denominator is lowered.
The design commit includes only this proposed record and its three spec
artifacts. Indexing this record and updating product docs belongs to
implementation; operator acceptance remains a separate act.

## Pointer — 2026-10-18 (proposed 0046)

[0046](0046-smtp-pop3-edges.md) §4, also **proposed**, updates §3 here (issue #128).
The shared inbox rule also refuses any Unicode control character, with one sentence on
every plane ("inbox must not contain control characters"). Webhook inbox filters, SMTP
AUTH and POP3 `USER` now use the same rule. A nonblank padded name stays verbatim
everywhere, and stored rows are not renamed.

## Pointer — 2026-10-18 (proposed 0047)

[0047](0047-storage-rpc.md) §§2–3, also **proposed**, update §§1/2 here (issues #130,
#132): an `id`-less object is a notification only with a string `method` — any other is
`-32600` with a null id, alone or in a batch — and the cumulative registry folds into a
line-feed overflow label instead of `_uncounted`, which stays an ordinary inbox.
