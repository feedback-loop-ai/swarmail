# Spec — five fidelity closures: the record, the restore, the wire, the view (issues #39, #54, #55, #56, #57)

- **Run:** run 11 (chief architect, synthesizing the simplicity and
  robustness positions) · **Feature slug:** `fidelity-closures`
- **Issues:** [#39](https://github.com/feedback-loop-ai/swarmail/issues/39)
  (extraction fidelity, P1 — first), [#54](https://github.com/feedback-loop-ai/swarmail/issues/54)
  (restored state matches live state), [#55](https://github.com/feedback-loop-ai/swarmail/issues/55)
  (Mailpit/MailHog compat fidelity), [#56](https://github.com/feedback-loop-ai/swarmail/issues/56)
  (thread normalization + web UI), [#57](https://github.com/feedback-loop-ai/swarmail/issues/57)
  (documentation and comment drift — docs-only, its own commit inside the
  batch).
- **Decisions:** [0031](../../docs/decisions/0031-sender-honest-stored-bodies.md)
  (proposed — the stored record is sender-honest; amends 0005 with the
  display-rendering carve-out) and
  [0032](../../docs/decisions/0032-emptied-inboxes-are-removed.md)
  (proposed — an emptied inbox is gone on every path). Both are minted by
  this spec; see *The semantic changes* below for why two, not zero.
- **Base:** `ce2df23` (run 10b landed at `a4ff22c`, the repairs
  `81409d4`/`95d2823`, 0030 accepted). Every claim below was re-verified
  against this tree; where the issue's age shows, the tree wins.
- **Shape:** the whole batch rides the seven production files it names
  (`src/extract.rs`, `src/smtp.rs`, `src/persist.rs`, `src/store.rs`,
  `src/compat.rs`, `src/threads.rs`, `src/ui.rs` + `src/static/ui.js`) and
  the suites that already prove them. Zero new `src/` files, zero new test
  files, zero new routes except one, one declared dependency that is
  already compiled into the tree (`percent-encoding` 2.3.2, MIT OR
  Apache-2.0, in `Cargo.lock` via `form_urlencoded`). Two decision
  documents, both `proposed`.

## The problem

1. **Extraction lies about the document (#39).** `extract_links` scans raw
   source with no entity decoding (`src/extract.rs:6`), so an `href`
   holding `&amp;` is stored verbatim; the existing seed test uses a raw
   `&` and never exercises encoding (`tests/api.rs:314`). `dedup` sorts
   alphabetically (`src/extract.rs:11-15`), so `links[0]`/`codes[0]` are
   the alphabetically first value, not the mail's first — and in
   multipart/alternative the broken HTML variant can precede the honest
   text one. `extract_codes`' HTML leg runs `CODE_RE` over the raw HTML
   body (`src/extract.rs:47-55`), harvesting digits from `<style>` blocks
   and attribute values — a sink that invents a verification code
   (`333333` from a stylesheet) is worse than one that misses codes.
   `build_email` uses mail-parser's *synthesizing* accessors
   (`body_text(0)` converts HTML→text; `body_html(0)` converts text→HTML;
   `src/smtp.rs:1248-1256`), so a text-only mail is stored with an `html`
   body the sender never sent and a `<style>` color becomes a "code" —
   the stored record, and every surface mirroring it (REST, MCP, Mailpit,
   MailHog, webhook payloads), lies about the wire. 0005's placement is
   right; its *content* is leaking.
2. **Restored state is not live state (#54).** `Persist::open` seeds
   `next_seq` with `COALESCE(MAX(seq), 0)` (`src/persist.rs:90`) while
   `record_insert` consumes it pre-increment (`:123-124`): the first
   insert after every reopen reuses the last row's `seq`. `load()` orders
   by `seq` alone over an unindexed full scan of every `raw` BLOB
   (`:103`) — restore order is SQLite's tie-break to choose, and per-inbox
   order is exactly what 0003's pruning evicts by: a tie broken newest-
   first at the duplicate makes the first capped insert after a restart
   evict the **newest** mail. 0003 silently dies across a restart under a
   full inbox. And an inbox emptied by `delete` survives as an empty
   `VecDeque` — listed with count 0, POP3 `USER` accepts it
   (`inbox_exists`, `src/pop3.rs:604-608`) — while `clear`'s drain
   removes the entry (`src/store.rs:104-120`) and restore rebuilds
   entries only for inboxes with rows: three paths, two answers.
3. **Compat answers plausibly-wrong (#55).** A Mailpit client's
   `{"IDs": [...]}` delete 400s and deletes nothing — serde accepts only
   lowercase `ids` (`src/compat.rs:72-78`); a test's cleanup no-ops and
   later assertions pass against mail that should be gone. Envelope
   recipients are invisible: with `RCPT TO:<bcc@x.io>` and header
   `To: a@x.io`, `kind=to` search returns `total: 0` (`Filter::matches`
   chains only header to/cc, `src/store.rs:46-57`), MailHog `To`/`Raw.To`
   read headers only (`src/compat.rs:646, 658`), Mailpit `Bcc` is
   hardcoded `[]` (`:501, 547`) — a search for a *delivered* recipient
   answering zero is a lie about delivery. `limit=0` returns an empty
   page instead of the default (four `unwrap_or(50)` sites, `:97, 110,
   211, 264`); `/api/v1/message/latest` 404s; `query=to:foo@x.io`
   silently returns nothing instead of refusing the unsupported prefix;
   Mailpit search `total`/`unread` report the result count instead of
   mailbox totals. Two items are already resolved on this tree and are
   **out of scope**: the staged kind domain and the page caps riding the
   documented envelopes (0030 §3).
4. **The view mis-files mail and leaks the codes it holds (#56).**
   `strip_marker` handles only `re/fw/fwd` ASCII (`src/threads.rs:104-114`)
   and the subject fallback folds ASCII-only (`:171`), so `AW: Plan`,
   `RE : Plan` and `Über`/`über` land in separate threads. The UI
   double-escapes the title (`message_view` escapes, then `page` escapes
   again — `A &amp;amp; B` in the tab), inbox links are HTML-escaped but
   not URL-encoded (`a#b` links to `a`, `src/ui.rs:74`), a missing
   message returns 200 with a "not found" card, no CSP is sent — the
   sink's real payload is verification codes, and a hostile mail rendered
   in the sandboxed iframe can beacon `codes[0]` to an attacker URL the
   moment a human opens it — and `load()`'s fetch can paint over a newer
   SSE frame. The sound basics (textContent/esc, sandboxed iframe, `/raw`
   as `message/rfc822` with attachment disposition) are kept.
5. **The record's documents lie (#57).** The decision index lists
   accepted decisions as `proposed` and carries a fourth citations cell
   under a three-column header (`docs/decisions/README.md`); `deny.toml`
   allows licenses the house rules do not name; CONTRIBUTING.md cites the
   root bundle and a wrong test path; README shows knob defaults as bare
   ports (`SWARMAIL_SMTP_LISTEN=2525` fails startup — the values are
   `host:port` socket addresses), lists a nonexistent `tests/smoke.rs`,
   omits `tests/p2.rs`, and leaves `RUST_LOG` undocumented; openapi.json
   omits the 503s, `/api/v2/search`'s `limit`/`start` and assert's
   `since_ms`/`subject`; comments contradict their code — including
   `src/mcp.rs:245` against 0014's no-trace refusal. Every claim is
   re-verified at HEAD before fixing (the issue was assessed at
   `ac8c3261`; some claims are already stale and are dropped, not fixed).

## The positions, ruled

The panel wrote two contrarian positions (`.brokkr/state/design/
positions/{simplicity,robustness}.md`). Ruled here, not averaged. Both
agree on the skeleton — decode at the extraction boundary only, seed
`MAX(seq)+1`, one emptied-inbox rule reached by deletion, the `IDs` alias,
envelope recipients surfaced, refuse-loudly over plausible-wrong, the CSP,
escape-once, docs conform to code. Three genuine conflicts needed a chief:

**Ruling 1 — the codes haystack is mail-parser's `html_to_text`, not a
hand-rolled skipper.** Robustness demanded a hand-rolled bounded skipper
over `<style>`/`<script>`/attributes and refused anything that "adds a
supply chain"; simplicity proposed
`mail_parser::decoders::html::html_to_text` — public, already a declared
dependency (verified in the vendored 0.11.x source, `decoders/html.rs:38`),
battle-tested with its own unit tests pinning style/script/comment
stripping. **Adopted: `html_to_text`.** Robustness's premise (a new supply
chain) does not bind — nothing new enters the tree — and a hand-rolled tag
stripper is exactly the new machinery this batch refuses. **Cost of losing
the skipper:** the rendering boundary is mail-parser's, so its behavior
changes ride the dependency-bump review — which is already the posture for
`build_email`'s parse — and what counts as "text content" is pinned by our
tests on top of it, not by our own code. Robustness's *demand* (codes from
text content only, total and linear, hostile input degrades to fewer
extractions and never a `451`) is carried verbatim as an invariant on the
whole ingest pass.

**Ruling 2 — the two display fallbacks land, and 0031 owns the 0005
carve-out.** Simplicity prices two one-line fallbacks (`snippet`,
`plain_response` render `html_to_text(html)` when `text` is `None`), so a
Mailpit client keeps a text view for html-only mail. Robustness refuses
"query-time extraction or decoding" on the query plane and says a derived
view, if wanted, is "a new field with a new name, documented". Both are
right about their own frame: 0005's letter says "nothing re-parses raw
HTML at query time", and the fallback does re-render at request time.
**Adopted: the fallbacks, under a proposed amendment to 0005** —
[0031](../../docs/decisions/0031-sender-honest-stored-bodies.md) rules
that *display rendering is not extraction*: no links or codes are derived
from it, the precomputed-answer property 0005 protects is intact, and the
stored record is never written back. **Costs, named:** an html-only mail's
snippet/plain view now pays one bounded render per request (bounded by
0022's ≤ 50 MiB cap, on the loopback compat plane, outside the mutation
boundary) where ingest previously paid it once — the honest trade for
stopping the store from inventing bodies. **If review or the operator
refuses even that**, the fallback collapses to an empty view and the diff
shrinks by two lines; the stored truth is identical either way. That
posture is recorded in 0031 so review owns it explicitly.

**Ruling 3 — two decision documents, not zero.** Simplicity's ledger prices
"zero new decision documents" and rejects door K's permanent review burden.
The zero-doc arithmetic holds *inside 0005's own frame* — but door S is
still a public-contract change (what `text`/`html` report flips for
html-only and text-only mail across REST, MCP, Mailpit, MailHog and
webhook payloads), and the emptied-inbox rule flips observable answers
(listings shrink, POP3 `USER` refuses a vanished inbox). The charter
routes semantic changes through a `proposed` decision; the house rules
call an unlisted difference a trap, and this batch exists to close traps.
**Adopted: [0031](../../docs/decisions/0031-sender-honest-stored-bodies.md)
and [0032](../../docs/decisions/0032-emptied-inboxes-are-removed.md),
both `proposed`.** **Cost of losing simplicity's zero-doc position:** two
files and one operator acceptance each. **Cost of the alternative:**
a contract shift no document owns — the exact class #57 exists to close.
Extractor changes A1–A3 need *no* decision: 0005's own consequence clause
routes extractor quality through review, and 0005's ruling (extraction at
ingest) is preserved verbatim.

**Corrected against code (the tree wins):** both positions cite
`snippet`/`plain_response` comments claiming "ingest always populates
`text` for html mail" — those comments are the synthesis fiction in
writing and are rewritten with A4. Simplicity's "one declared dep already
compiled in the tree" is confirmed (`percent-encoding` 2.3.2 in
`Cargo.lock`; declaring it changes no package set). Robustness's "no
index on `seq`" is confirmed — `load()` is a full-scan sort of every raw
BLOB today.

## The design

### #39 — extraction fidelity (P1, first) — `src/extract.rs`, `src/smtp.rs`, `src/compat.rs`

- **A1 — entity decoding at the extraction boundary, HTML leg only.** A
  small total `decode_refs` beside the regexes: the five XML named refs
  (`amp`, `lt`, `gt`, `quot`, `apos`) plus decimal/hex numeric references.
  It is applied **per matched URL, after `URL_RE` finds it and before the
  trailing-punctuation trim**, and **only on the HTML leg** — decoding the
  whole source first would turn `&lt;` into `<` and corrupt match
  boundaries; decoding the text/plain leg would corrupt honest text
  ("use `&amp;` in your config" → `&`). `text/plain` stays byte-honest.
  **Degrade, never fail:** unknown names and out-of-range or surrogate
  numeric refs stay literal; the decoder never grows the string, never
  panics, and is linear — a hostile doc degrades to *fewer* extractions,
  never a `451` (the run-10 ruling that a panicking parse is a dead
  session, no `451` arm, stands, so the decoder must simply be total).
  **Cut:** no HTML5 entity table (2,231 entries; a data dependency) and
  no `html5ever` (MPL-2.0, license-blocked). The named refs that can
  appear in a well-formed URL are the XML five; an `&nbsp;` stays
  verbatim and the raw body still holds the truth (0005's own
  consequence). Named refs are matched case-sensitively (XML is).
- **A2 — first-occurrence document order.** `dedup` becomes an
  insertion-order seen-set dedup (no sort). `links[0]`/`codes[0]` are the
  mail's first values; the text leg is scanned before the html leg
  (existing loop), so in multipart/alternative the honest text variant's
  URL wins.
- **A3 — codes from rendered text only.** `extract_codes`' HTML leg runs
  against `mail_parser::decoders::html::html_to_text(html)` (Ruling 1) —
  codes can only come from rendered text nodes; `<style>`, `<script>`,
  comments and attribute values yield nothing. Precision over recall: a
  code held only in an attribute is a named gap; `333333` was a lie we
  stop telling.
- **A4 — sender-honest bodies (0031).** `build_email` replaces the
  synthesizing accessors with a two-arm match on the parsed part's type:
  a text part stores `text` and `html: None`; an html part stores `html`
  and `text: None`; anything else stores neither. The stored `text`/
  `html` fields hold **only what the sender sent**. The ripples are the
  two display fallbacks of Ruling 2 (`snippet` and `plain_response` render
  `html_to_text` at their own site when `text` is `None`) — display
  derivations, not extraction; no links or codes come from them. Every
  other consumer (`hog_mime`, `hog_message`'s `Content.Body`,
  `contains()`) already handles `None` honestly. **Named residual:** a
  needle matching *decoded* text (`a&b` against source `a&amp;b`) now
  hits only via the raw source's encoded form or not at all — the raw
  bytes are the truth; no machinery chases it. Tests that asserted the
  synthesis are updated in the same commit.

### #54 — restored state matches live state — `src/persist.rs`, `src/store.rs`, `src/pop3.rs`

- **B1 — the seed is `COALESCE(MAX(seq), 0) + 1`.** One character of
  arithmetic; **no persisted counter row** (a write on the hot insert
  path, or the same staleness question, for a fact the table already
  holds — MAX *is* the counter; the seed was merely off by the
  increment). **Cost accepted:** `seq` gaps are honest — a failed commit
  consumes a sequence (`:123-124` increments before the commit);
  ordering needs monotonicity, not density; gaps are not "repaired".
- **B2 — total restore order: `ORDER BY seq, rowid` + one idempotent
  index.** `CREATE INDEX IF NOT EXISTS emails_seq ON emails(seq)` in the
  schema (the `emails_inbox` pattern, `:51`) — created on every open, so
  old files gain the index at open, **no format change, no version gate,
  no migration matrix**. The index's `(seq, rowid)` ordering satisfies
  the new ORDER BY: restore stops dragging every `raw` BLOB through a
  temp b-tree sort, and restore order is deterministic for both new and
  legacy files (legacy files written by the buggy seed carry duplicate
  `seq`; `rowid` breaks those ties the same way every time). **Cost
  accepted:** one O(n log n) index build on the first open of a legacy
  file, riding the existing pre-bind restore path; subsequent opens are
  no-ops.
- **B3 — an emptied inbox is gone (0032).** One rule by **removal** on
  every path: `delete` removes the emptied inbox entry at the exact
  moment it already releases the watcher channel (the `list.is_empty()`
  condition at `src/store.rs:601-603`, plus the guard restructure — the
  `DashMap` `get_mut` guard is dropped before the `remove`); `clear`
  already removes it; restore already rebuilds only inboxes with rows.
  No tombstone table, no `meta` row, no restore-side special case —
  persisting emptiness adds a whole vocabulary to preserve an empty
  container. POP3 `USER` on a vanished inbox answers exactly like a
  never-seen one (`inbox_exists` is derived from the listing, so it
  follows automatically); 0021's watcher semantics are unchanged (same
  condition, same moment; the waiter's per-iteration re-subscribe on
  `Closed` is miss-free — 0002), and the issue-#14 `watcher_count` probe
  extends to the reopened store. 0001 holds: every change is synchronous
  SQL inside `open`/`load` or the existing mutation boundary — no await
  enters insert. 0003 holds: B2 changes the *determinism* of the order
  eviction reads, never which mails are candidates.

### #55 — compat fidelity — `src/compat.rs`, `src/store.rs`

- **C1 — `#[serde(alias = "IDs")]` on `IdsBody.ids`.** The whole delete
  fix; 0011's explicit-selector-or-400 rule is intact (absent, empty and
  wrong-typed selectors still refuse with the named sentence).
- **C2 — envelope recipients reach every recipient surface, from the
  stored field.** `email.recipients` (RCPT TO) is already a persisted
  model field — no schema, no ingest change: (a) `Filter::matches` chains
  `email.recipients` into the `to` leg — one owner for "any recipient",
  fixing `kind=to` search for every dialect at once (the filter's doc
  already promises "any recipient"; the header-only match was the bug);
  (b) `hog_message`'s `To` and `Raw.To` read the envelope as upstream
  MailHog fills them (header `To`/`Cc` stay in `Content.Headers`);
  (c) Mailpit's `Bcc` lists **envelope-only** recipients — a small helper
  (recipients minus header to/cc addresses) shared by `mailpit_summary`
  and `mailpit_message`.
- **C3 — refuse-loudly over plausible-wrong; README-list the rest.**
  *Fixed:* `limit=0` means "unset" → the default page at the four compat
  call sites (upstream answers the default; an empty page is a footgun —
  a client's "assert nothing" passes vacuously); `/api/v1/message/latest`
  exists, reusing `mailpit_message` over the newest stored mail (404 only
  when the store is empty — honest); Mailpit search `total`/`unread`
  report mailbox totals (the unfiltered scope count), not the result
  page's size; `query=to:…` is **refused by name** with a 400 in 0018 §3's
  sentence pattern (0025's discipline) — not implemented: Mailpit's
  prefix grammar (quoted phrases, multiple terms) is a new parser with
  new refusals for a compatibility nicety. *README-listed as known
  differences:* the malformed `limit`/`start` rejection shape (axum's
  plain-text 400 — fixing it means a custom query extractor for one
  status-code shape) and `/api/v1/delete-all`'s provenance (swarmail's
  alias, not an upstream route). The two resolved items (staged kind
  domain, page caps) are not re-litigated.

  **Addendum (implement seat, 2026-10-16).** The feature file's account
  of the staged kind domain said "an absent kind is upstream's
  `containing` default". Upstream answers a bare 400 for a missing kind
  too — MailHog-Server `api/v2.go`, `search()`: the whitelist
  `kind != "from" && kind != "to" && kind != "containing"` rejects the
  empty string before the query is read (the v1 API has no search
  route). Amended: `search_hog` requires `kind` present and one of
  from|to|containing; `kind=subject`'s bare 400 is unchanged.

### #56 — threads + web UI — `src/threads.rs`, `src/ui.rs`, `src/static/ui.js`

- **D1 — normalization: markers and Unicode case.** `strip_marker` gains
  the `aw` marker and tolerates whitespace between the word and the colon
  (`RE : Plan`); the marker list stays a `match` arm, not a data table.
  The subject fallback folds with `str::to_lowercase()` — std is already
  Unicode-aware; **no ICU, no folding crate**. **Named residual:**
  `to_lowercase()` is lowercase mapping, not full `to_case_fold` —
  `Über`/`über` merge, `ß` vs `ss` does not; the README claim is worded
  to match (#57's truth discipline). A false marker merge is bounded by
  0018 anyway: the subject fallback groups chain-less orphans only,
  never bridges a chain — its worst case misfiles a thread and cannot
  lie about mail.
- **D2 — the view tells the truth.** The subject is escaped **once**
  (`message_view` passes the raw subject to `page`; the `<h2>` keeps the
  escaped form); inbox hrefs are `utf8_percent_encode`d
  (`percent-encoding`, `NON_ALPHANUMERIC`) while the visible text keeps
  `esc`; a missing message answers **404** (the card body stays for
  humans; the status stops lying).
- **D3 — one CSP string, one choke point.** A single
  `<meta http-equiv="Content-Security-Policy">` constant interpolated
  into `page()`'s head — the one template every UI route flows through.
  No `tower-http`, no middleware layer. The directive set keeps the
  existing inline script/style and the SSE `EventSource` working and
  blocks remote loads (`default-src 'none'`, `script-src`/`style-src
  'unsafe-inline'`, `connect-src 'self'`, `img-src 'self' data:`; exact
  spelling is the implementer's verified detail). The srcdoc iframe
  inherits the embedding document's policy container, so mail bodies
  inside the sandboxed iframe lose remote `img`/`css` loads — the
  exfiltration defense for the codes the sink holds. 0030 §2's loopback
  posture is additive over, not weakened.
- **D4 — a four-line paint token.** A monotonic `paintSeq` in `ui.js`;
  the SSE listener bumps it; `load()` captures it before its `fetch` and
  abandons the paint if it moved. The newest frame always wins. **Cut:**
  `AbortController` plumbing, server-side sequence numbers — a client-
  side token is the whole race.

### #57 — the record — docs and comments only

- **E1 — verify every claim at HEAD before fixing; stale claims are
  dropped, not fixed** (including the already-stale CI "decision 0063"
  citation).
- **E2 — the decision index matches each file's recorded status**, and
  the table gains a real fourth header column for the citations cell
  every row already carries. The index is the only thing edited.
- **E3 — `deny.toml` conforms **down** to the house rules**: the
  allowances the house rules do not name (CC0-1.0,
  CDLA-Permissive-2.0, `Apache-2.0 WITH LLVM-exception`) are removed
  unless the tree genuinely needs one — a genuine need goes to the
  operator notes file, never a silent widen; CC-BY-4.0 is present,
  noted inline for data; the Zlib allowance stays (the house rules name
  it). `cargo deny check licenses` stays green.
- **E4 — CONTRIBUTING.md, README.md and openapi.json match the code at
  fix time** (bundle path, `tests/common/mod.rs`, "100% of lines,
  branches and functions", no `tests/smoke.rs`, `tests/p2.rs` listed,
  knob defaults as `host:port` socket addresses, `RUST_LOG` documented,
  the startup log's POP3 address consistent, openapi gaining the 503s on
  `DELETE /api/v1/messages` and `/api/v1/delete-all`,
  `/api/v2/search`'s `limit`/`start`, and assert's `since_ms`/`subject`).
- **E5 — comments match the code, never the reverse** (`src/chaos.rs:16`,
  `src/store.rs:158`, `src/pop3.rs:11`, `src/pop3.rs:503`,
  `src/mcp.rs:245` learning 0014's no-trace refusal wording,
  `src/lib.rs`'s startup comment, `benches/writer_stall.rs`'s expect
  messages). Nothing re-litigates the `smtp=` startup line (4daa332) or
  0027 §7. No new test files, no doc-reading linters, no CI doc-drift
  machinery; E2's row-by-row comparison ships as review evidence.

## The semantic changes

Two public-contract shifts, each owned by a `proposed` decision the
operator accepts post-review:

1. [0031](../../docs/decisions/0031-sender-honest-stored-bodies.md) —
   the stored `text`/`html` fields hold only what the sender sent (no
   synthesis); display-time rendering on the compat plane is not
   extraction (amends 0005 with that carve-out). Until accepted, the
   spec's A4 stands on the issue's own fork ("the synthesis is
   documented" — 0031 is the documentation).
2. [0032](../../docs/decisions/0032-emptied-inboxes-are-removed.md) —
   an emptied inbox is gone on every path (delete, clear, restore);
   listings shrink and POP3 `USER` refuses a vanished inbox exactly like
   a never-seen one. The issue itself offers both doors (removed or
   kept); removal is chosen and the choice is the decision's.

Everything else in the batch is applied, not amended: 0005's ingest
ruling (links/codes) is preserved; 0011's delete contract survives the
alias verbatim; 0029/0025/0018 §3 govern the new refusals; 0030 §2/§3 are
untouched; 0001/0002/0003 are re-proved, not weakened (B2 repairs 0003
*across a restart*; nothing weakens it).

## Acceptance Criteria

Each criterion is red-able before its fix and green after, with the proof
site named. Tests follow 0020 (no sleep-as-sync, no wall-clock validity,
chaos at the poles, fresh stores/inboxes, no cold-path harness needed —
every new branch is reachable on the green path by a direct request).

### #39 — extraction fidelity (P1, first)

- **AC-39.1** An SMTP-delivered HTML mail whose `href` holds `&amp;` — and
  one holding `&#38;` — stores the decoded URL in `links`. Named and
  numeric character references are decoded **only** on the HTML leg.
  *Proof:* a real-SMTP leg in `tests/api.rs` extending the seed at
  `:314`; inline units in `src/extract.rs`.
- **AC-39.2** A text/plain mail whose body contains the literal string
  `&amp;` stores it verbatim in `text` and extracts no decoded variant.
  *Proof:* `src/extract.rs` unit.
- **AC-39.3** `links` and `codes` keep first-occurrence document order
  while deduplicating; `links[0]`/`codes[0]` are the mail's first value,
  not the alphabetically first; in multipart/alternative the variant
  occurring first wins. *Proof:* `src/extract.rs` units.
- **AC-39.4** A mail whose only digit runs sit in a `<style>` block or an
  attribute value yields no codes. *Proof:* `src/extract.rs` units (and
  the live `&amp;` leg's `codes` field in `tests/api.rs`), including
  `src/extract.rs::inline_adjacent_codes_behave_as_the_renderer_renders` —
  the renderer's narrowings pinned: a code the renderer glues into a word
  is refused, never invented.
- **AC-39.5** Malformed references degrade: an unknown named ref or an
  out-of-range/surrogate numeric ref stays literal; the decoder never
  grows its input and never panics. *Proof:* `src/extract.rs` units.
- **AC-39.6** A text-only mail stores `html: null`; an html-only mail
  stores `text: null` — on the native REST and MCP planes, which keep the
  null (0031 §2); on the Mailpit and MailHog compat shapes an absent part
  is an **empty string** (`Text`/`HTML` map through `unwrap_or_default()`
  at `src/compat.rs:669–670`; MailHog's `Body` is one string,
  text-else-html, at `:757`), so a client there cannot distinguish "no
  part" from "empty part"; `raw` still holds the sender's bytes.
  *Proof:* `tests/api.rs` + the `build_email` units in `src/smtp.rs`
  (native planes); the compat mapping is recorded by 0031 §2, as
  amended — deliberately untested, a Go-shape change-detector would only
  pin it.
- **AC-39.7** For html-only mail, Mailpit's snippet and the plain shim
  still return a text view (the display fallback), and no links/codes are
  derived from that rendering. *Proof:* `tests/compat.rs`.

### #54 — restored state matches live state

- **AC-54.1** Open → insert → reopen → insert → reopen → insert leaves no
  duplicate `seq` values; the seed is `MAX(seq) + 1`. *Proof:*
  `src/persist.rs` unit test (reopen twice per the issue).
- **AC-54.2** Restore order is `(seq, rowid)` — deterministic even for a
  hand-built legacy fixture carrying duplicate `seq` — and the `seq`
  scan is indexed. *Proof:* `src/persist.rs` unit with a legacy fixture;
  insertion order asserted after reopen.
- **AC-54.3** An inbox emptied by `delete` is absent from the listing
  live **and** after a reopen; `clear`, `delete` and restore follow one
  rule. *Proof:* `tests/persistence.rs` (live + reopen parity).
- **AC-54.4** POP3 `USER` on a vanished inbox answers exactly like
  `USER` on a never-seen one; `watcher_count` returns to baseline after
  the emptied-inbox removal, including on a reopened store. *Proof:*
  `tests/pop3.rs` extensions.
- **AC-54.5** Old data files still open: the index creation is
  idempotent, no version gate fires, and `Store::insert` stays
  synchronous (no await entered the persist path). *Proof:* the existing
  open-refusal tests still pass (`src/persist.rs::open_fails_on_an_
  unusable_file`, `src/store/persist_tests.rs::open_refuses_an_unusable_
  data_file`) plus a reopen-with-index leg; the 0001 bench records no
  commit regression.

### #55 — compat fidelity

- **AC-55.1** `DELETE /api/v1/messages` with body `{"IDs": ["…"]}`
  deletes exactly the named messages (no 400); an absent, empty or
  wrong-typed selector still 400s with 0011's named sentence. *Proof:*
  `tests/compat.rs`.
- **AC-55.2** With `RCPT TO:<bcc@x.io>` and header `To: a@x.io`:
  `GET /api/v2/search?kind=to&query=bcc@x.io` returns `total == 1` (and
  `items.len() == 1`);
  MailHog `To`/`Raw.To` surface `bcc@x.io`; Mailpit `Bcc` lists
  `bcc@x.io` (envelope-only). Header to/cc stay in `Content.Headers`.
  *Proof:* `tests/compat.rs` real-SMTP leg.
- **AC-55.3** `limit=0` returns the default page (not an empty one) on
  all four compat pages; above 250 it still refuses (0030 §3). *Proof:*
  `tests/compat.rs`.
- **AC-55.4** `GET /api/v1/message/latest` returns the newest stored mail
  in Mailpit's message shape; 404 only when the store is empty. *Proof:*
  `tests/compat.rs`.
- **AC-55.5** Mailpit search `total`/`unread` report mailbox totals, not
  the result page's size. *Proof:* `tests/compat.rs`.
- **AC-55.6** A Mailpit search query with a `to:` prefix answers a named
  400 (0018 §3's sentence), never a silent empty result. *Proof:*
  `tests/compat.rs`.
- **AC-55.7** The README lists the malformed `limit`/`start` rejection
  shape and `/api/v1/delete-all`'s provenance as known differences.
  *Proof:* README inspection against the code.

### #56 — threads + web UI

- **AC-56.1** `AW: Plan`, `RE : Plan` and `Über`/`über` normalize to one
  thread key (and `Re: x` / `Re : x` / `aw: x` group with `x`); the
  `to_lowercase` residual (`ß` vs `ss`) is documented in the doc comment.
  *Proof:* `src/threads.rs` units on `normalize_subject`.
- **AC-56.2** A message title containing `&` appears escaped exactly
  once in the tab and the `<h2>` (no `&amp;amp;`). *Proof:* `tests/ui.rs`.
- **AC-56.3** An inbox named `a#b` links to `/ui/inbox/a%23b` (URL-encoded
  href, visible text unharmed). *Proof:* `tests/ui.rs`.
- **AC-56.4** A missing message answers 404 (status, not a 200 card).
  *Proof:* `tests/ui.rs`.
- **AC-56.5** The served shell carries a CSP that keeps inline
  script/style and same-origin SSE working and blocks remote loads
  inside mail bodies. *Proof:* `tests/ui.rs` asserts the policy on the
  served shell.
- **AC-56.6** A stale `load()` response cannot paint over a newer SSE
  frame (the paint token guards it). *Proof:* the served asset carries
  the guard (`tests/ui.rs` pattern); a real JS interleaving test would
  need a browser engine — not demanded.

### #57 — the record (docs-only)

- **AC-57.1** Every decision-index row matches its file's recorded
  status, and the table header has four columns matching the rows.
  *Proof:* row-by-row comparison, shipped as review evidence.
- **AC-57.2** `deny.toml` and `docs/house-rules.md` name the same
  license allowlist; `cargo deny check licenses` is green.
- **AC-57.3** CONTRIBUTING.md, README.md and openapi.json match the code
  at fix time (the E4 list); every E5 comment matches the code it
  describes. *Proof:* each comment read against its code; openapi.json
  greps.

### Batch-wide

- **AC-B.1** All gates green: `cargo fmt --all -- --check`, `cargo
  clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace --locked`, `brokkr compile --bundle
  bundles/self`, `TMPDIR=/home/valentin/scratch/cov-tmp bash
  scripts/coverage-gate.sh` (exact 100/100/100, missed == 0, no
  `coverage(off)`), `cargo deny check licenses`.
- **AC-B.2** No new `src/` file, no new test file, no new crate beyond
  the declared `percent-encoding` (already in the resolved graph,
  MIT OR Apache-2.0).
- **AC-B.3** 0001/0002/0003 re-proved, not weakened: the 0001 bench
  records no commit regression; the await path is untouched; the capped
  burst + restart leg evicts the oldest mail (B2's proof in practice).
