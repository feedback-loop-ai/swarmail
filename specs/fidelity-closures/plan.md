# Plan — five fidelity closures

The implementation approach for [spec.md](spec.md). The whole batch rides
seven existing production files and the suites that already prove them:
**zero new `src/` files, zero new test files, zero new decision documents
beyond the two this spec mints, one declared dependency that is already
compiled into the tree.** Ground truth re-verified at `ce2df23`; every
line number below is from this tree.

## Technical context

- **`src/extract.rs` (76 lines) — the whole #39 extraction surface.**
  `URL_RE` (`:6`) and `CODE_RE` (`:9`) are `LazyLock` regexes; `dedup`
  (`:11-15`) is `sort + dedup` — the alphabetical-order bug;
  `extract_links` (`:18-31`) loops `[text, html]` and trims trailing
  punctuation; `extract_codes` (`:35-56`) prefers text then falls back to
  raw HTML. The two legs share one loop body, so the html-leg-only decode
  needs the leg's identity at the match site (a `bool` is enough — no
  signature change beyond the decoder call). The inline `#[cfg(test)]`
  module (`:58-76`) holds the two existing pins.
- **`mail_parser` 0.11.x is already a dependency** (`Cargo.toml:49`) and
  its public `decoders::html::html_to_text` (verified vendored,
  `src/decoders/html.rs:38`) strips style/script/template/comment
  content and emits text nodes — the A3 haystack and the A4-ripple
  renderer are the *same function*, one rendering boundary in the batch.
  The synthesizing accessors live in `build_email` (`src/smtp.rs:1248-
  1256`); the replacement is a match on the parsed message's body-part
  type (text part → `text`, html part → `html`, else `None`), keeping
  "first part of each kind" semantics. `build_seed_email` ends in
  `build_email`, so the seed path rides the same fix with zero edits.
- **`src/persist.rs` — the #54 surface.** The seed is `COALESCE(MAX(seq),
  0)` (`:90-92`) consumed pre-increment in `record_insert` (`:123-124`);
  `load()` is `SELECT {COLUMNS} FROM emails ORDER BY seq` (`:103`) over
  the `SCHEMA` batch (`:32-56`, where `emails_inbox` at `:51` is the
  index pattern to copy). `ADDED_COLUMNS` (`:59-63`) is the existing
  idempotent-migration precedent the index rides. The write path is one
  connection behind a `std::Mutex`, never across an await (`:44-46`).
  The open-refusal pins: `open_fails_on_an_unusable_file` (inline) and
  `open_refuses_an_unusable_data_file` (`src/store/persist_tests.rs`).
- **`src/store.rs` — `Filter::matches` (`:46-57`) chains `to`/`cc`
  headers only; `email.recipients` is the persisted envelope field
  (`recipients_json` in `COLUMNS`, `src/persist.rs:27-29`). The `delete`
  apply arm (`:595-607`) holds a `DashMap` `get_mut` guard when it
  releases the watcher on `list.is_empty()` (`:601-603`) — the container
  removal must drop that guard before `inboxes.remove` (DashMap
  deadlocks on nested shard access). `drain_inbox` (`:104-120`) is the
  clear-side precedent; restore rebuilds only inboxes with rows.
- **`src/compat.rs` — the #55 surface.** `IdsBody` (`:72-78`) needs one
  `#[serde(alias = "IDs")]`. The four page-staging sites are `:97, :110,
  :211, :264` (`params.limit.unwrap_or(50)`); `page_cap` is the shared
  0030 §3 refusal. `mailpit_summary` (`:472`, `Bcc` at `:501`) and
  `mailpit_message` (`:518`, `Bcc` at `:547`) hardcode `[]`;
  `hog_message`'s `To`/`Raw.To` at `:646, :658`; `snippet` (`:458`) and
  `plain_response` (`:683`) carry the "ingest always populates `text`"
  comments — rewritten with A4. The Mailpit routes registry is beside
  the other `api/v1` routes; `/api/v1/message/latest` is one new route
  reusing `mailpit_message` (the batch's only new route).
- **`src/threads.rs` — `normalize_subject` (`:89-101`), `strip_marker`
  (`:104-114`, ASCII-only word/colon, `re|fw|fwd`), and the subject fold
  `to_ascii_lowercase()` at `:171`. The marker match stays a `match`
  arm; whitespace tolerance is a `trim_start` before the colon check.
- **`src/ui.rs` + `src/static/ui.js`.** `esc` (`:34-41`); `page` escapes
  its `title` argument (`:43-48`) while `message_view` passes an
  already-escaped subject (`:139-146`) — the double escape; the inbox
  href interpolates `esc(&name)` into `href="/ui/inbox/{name}"` (`:74`);
  the missing-message arm returns a 200 card. `page()`'s head is the
  single choke point for the CSP meta. `ui.js`: `async function load()`
  (`:72`) and the `EventSource` feed (`:86`) — the paint token sits
  between them.
- **`percent-encoding` 2.3.2** is in `Cargo.lock` (`:1039-1040`) via
  `form_urlencoded` (serde_urlencoded → axum), MIT OR Apache-2.0.
  Declaring it in `Cargo.toml` adds no package to the graph;
  `cargo deny check licenses` stays green.
- **#57's files** (docs-only): `docs/decisions/README.md`,
  `deny.toml`, `CONTRIBUTING.md`, `README.md`, `openapi.json`,
  `.github/workflows/ci.yml`, `src/chaos.rs:16`, `src/store.rs:158`,
  `src/pop3.rs:11`, `src/pop3.rs:503`, `src/mcp.rs:245`, `src/lib.rs`
  (startup comment), `benches/writer_stall.rs` (expect messages).
  E1's rule binds every one: verify at HEAD first; stale claims are
  dropped, not fixed.

## Constitution check

- **0001 (accept == stored):** the index build rides the pre-bind
  `Persist::open` path; `record_insert`'s seed arithmetic is inside the
  existing synchronous commit; the part-type match and `decode_refs` run
  inside the offloaded parse, *before* the boundary. No await enters any
  insert/persist path. Re-proved by the existing burst/cap legs and a
  reopen leg; the 0001 bench records no regression.
- **0002 (subscribe before check):** untouched by all five. The B3
  watcher release keeps its condition and moment; waiters re-subscribe
  per iteration (the `Closed` discipline is already in `wait_for`).
- **0003 (oldest-only pruning):** B2 makes restore order *deterministic*
  — the eviction choice is untouched (same candidates, same
  `pop_front`). The duplicated-`seq` status quo is the actual 0003 leak
  across restart; the fix is the repair.
- **0005 (extraction at ingest):** preserved for links/codes; the two
  display fallbacks ride 0031 §3's carve-out (proposed). A1–A3 are
  extractor-quality changes 0005 routes through review.
- **0011 (shim delete contract):** the `IDs` alias only teaches serde the
  casing real Mailpit clients send; absent/empty/wrong-typed selectors
  still refuse with the named sentence.
- **0018 §3 / 0025:** every new refusal (`to:`-prefix 400, the alias's
  unchanged 400 sentence, the 404) names the rule in one sentence.
- **0029 (supplied values validated at the wire):** exercised, not
  amended — the `to:` refusal and the existing page caps.
- **0030 §2/§3:** the loopback guard, the bind posture and the 250 caps
  are untouched; the CSP is additive UI hardening; `limit=0 → default`
  then caps exactly as today.
- **0022:** the ≤ 50 MiB DATA cap bounds every new decode/render path.
- **0023:** the decode runs inside the existing offloaded parse — no new
  blocking-pool vocabulary, no panic arm (the decoder is total).
- **0020:** no cold-path harness — every new branch (decoder, refusals,
  404, alias, removal, paint token) is reachable on the green path by a
  direct request; all new tests are inline `#[cfg(test)]` or the existing
  `tests/*.rs` suites. No seams are added.
- **License policy:** one declared dep (`percent-encoding`, MIT OR
  Apache-2.0) — *"URL-encode inbox path segments using the encoder
  already compiled into the tree rather than hand-rolling one."*
  `deny.toml` conforms down to the house rules (#57 E3); no widen.

## Project structure

Files touched (complete list):

| File | Change |
|---|---|
| `src/extract.rs` | A1 `decode_refs` (html-leg, per-URL); A2 seen-set `dedup`; A3 `html_to_text` haystack; inline units |
| `src/smtp.rs` | A4 part-type match in `build_email` (replaces the synthesizing accessors); `build_email` units |
| `src/compat.rs` | A4 ripples (`snippet`, `plain_response` fallbacks + comment rewrites); C1 alias; C2 To/Raw.To/Bcc helper; C3 fixes (`limit=0`, `latest` route, `total`/`unread`, `to:` refusal) |
| `src/store.rs` | B3 emptied-inbox removal in the `delete` apply arm (guard restructure); C2 `Filter::matches` chains `recipients` |
| `src/persist.rs` | B1 seed `+ 1`; B2 `ORDER BY seq, rowid` + `emails_seq` index; units |
| `src/threads.rs` | D1 `aw` marker, whitespace-tolerant colon, `to_lowercase()` fold; units |
| `src/ui.rs` | D2 escape-once, percent-encoded hrefs, 404; D3 CSP meta |
| `src/static/ui.js` | D4 paint token |
| `src/pop3.rs` | E5 comment only (`:11`, `:503`) — `inbox_exists` follows the listing |
| `tests/api.rs` | AC-39.1/39.6 real-SMTP legs |
| `tests/persistence.rs` | AC-54.3 live/reopen parity |
| `tests/pop3.rs` | AC-54.4 USER parity + watcher probe |
| `tests/compat.rs` | AC-39.7, AC-55.1–55.6 |
| `tests/ui.rs` | AC-56.2–56.6 |
| `Cargo.toml` | declare `percent-encoding` |
| docs (#57) | `docs/decisions/README.md`, `deny.toml`, `CONTRIBUTING.md`, `README.md`, `openapi.json`, `.github/workflows/ci.yml`, five comment sites, `benches/writer_stall.rs` |
| `docs/decisions/0031-…md`, `0032-…md` | minted by this spec, status `proposed` |

No new `src/` module, no new test file, no new bench, no new route except
`/api/v1/message/latest`, no schema/format change (the index is
idempotent), no new knob.

## Data model

`emails` keeps its columns; one index is added:

```sql
CREATE INDEX IF NOT EXISTS emails_seq ON emails(seq);  -- idempotent, every open
```

- `next_seq := COALESCE(MAX(seq), 0) + 1` at open; `record_insert`
  keeps consume-then-increment (the `+1` moves to the seed, not the
  increment — a failed commit still consumes its sequence; gaps are
  honest).
- Restore order is `ORDER BY seq, rowid`; the index's internal
  `(seq, rowid)` ordering serves it directly, so legacy files with
  duplicate `seq` restore deterministically without a rewrite.
- No `meta` row for `next_seq`; no tombstone table for empty inboxes;
  no version-gate bump. Files written before this batch open unchanged.

## Contracts (wire deltas)

- **REST/MCP/Mailpit/MailHog/webhooks:** `text`/`html` become honest
  nulls when the sender sent only the other kind (0031); Mailpit
  `Snippet` and the plain shim render a text view for html-only mail.
- **Mailpit:** `DELETE /api/v1/messages` accepts `{"IDs": […]}`;
  `Bcc` lists envelope-only recipients; search `total`/`unread` are
  mailbox totals; `limit=0` returns the default page; `to:`-prefixed
  queries 400 with a named sentence; `GET /api/v1/message/latest`
  exists.
- **MailHog v2:** `To`/`Raw.To` surface envelope recipients; `kind=to`
  search matches envelope recipients (one `Filter` owner).
- **REST/MCP filters:** the `to` filter matches header to/cc **and**
  envelope recipients (the filter doc's "any recipient" made real).
- **POP3:** `USER` on an emptied inbox answers as a never-seen maildrop
  (0032).
- **UI:** 404 for a missing message; `Content-Security-Policy` meta on
  every UI route; URL-encoded inbox hrefs; escape-once titles.

## Quickstart (verify gate)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
brokkr compile --bundle bundles/self
TMPDIR=/operator-scratch/cov-tmp bash scripts/coverage-gate.sh   # exact 100/100/100
cargo deny check licenses
```

Targeted proofs inside `cargo test` (real servers, no mocks):
`--test api` + `--lib extract` (#39), `--test persistence` + `--test pop3`
+ `--lib persist` (#54), `--test compat` (#55), `--test ui` +
`--lib threads` (#56); #57 is gates + doc/openapi inspection with the
row-by-row status comparison shipped as review evidence.

## Risks

| Risk | Mitigation |
|---|---|
| **The XML-five entity table is not the HTML5 table.** An `&nbsp;` in an href stays verbatim; `&AMP;` (uppercase) is not decoded. | Accepted and named: raw is stored (0005's consequence), the framing's proof cases are `&amp;`/`&#38;`, and the alternative is a 2,231-entry data dependency for characters well-formed URLs do not carry. The decoder is one function; widening the table later is a one-arm diff. |
| **Honest bodies change what consumers see.** `text: null` / `html: null` flip across REST, MCP, Mailpit, MailHog and webhook payloads; tests asserting the synthesis break. | That is the feature (0031). Tests are updated in the same commit; the two display fallbacks keep Mailpit's human-visible views whole; `raw` remains the complete truth. The flip is a *listed* contract delta owned by a proposed decision the operator can refuse. |
| **The display fallbacks re-render at request time** — the 0005-letter tension the panel named. | 0031 §3 owns the carve-out (rendering ≠ extraction: no links/codes derived, nothing written back, bounded by 0022's cap, loopback plane, outside the boundary). If review refuses, the fallback collapses to an empty view — a two-line deletion, stored truth unchanged. |
| **`to_lowercase()` is lowercase mapping, not `to_case_fold`.** `Über`/`über` merge; `ß` vs `ss` does not. | Accepted: nothing in the framing tests full folding; the residual is one doc-comment sentence and the README wording is fixed to match (#57's truth discipline). ICU is a dependency for a case nobody demanded. |
| **`to:`-prefix queries answer 400 instead of parsing.** | Named refusal per 0018 §3/0025; README documents it; a later grammar implementation is a separate additive change. The plausible-wrong alternative (silently empty) is the bug being fixed. |
| **Legacy data files carry duplicate `seq`.** | B2's `rowid` tie-break plus the index makes their restore deterministic; B1 stops manufacturing new duplicates; no row rewrite, no format bump. The legacy fixture test pins the tie-break. |
| **The index build costs one O(n log n) pass** on the first open of a legacy file. | It rides the existing pre-bind restore path (one-time open latency; 0001 is untouched — no await near insert); subsequent opens are `IF NOT EXISTS` no-ops; the 0001 bench re-proves the commit path. |
| **The emptied-inbox rule flips observable answers** (listings shrink; POP3 `USER` refuses). | That is the fix (0032), not a regression: three paths converge instead of two lying to one. The operator owns the flip; the watcher probe (`watcher_count` baseline, extended to reopen) proves the release is miss-free. |
| **Honest `text: null` narrows search.** A needle matching decoded text (`a&b` vs `a&amp;b`) may miss where the synthesis used to hit. | Accepted and named: `raw` stays searchable byte-honestly (0022/0025 posture); adding a decode-on-search layer is query-time extraction, refused. |
| **`html_to_text` behavior is mail-parser's, not ours.** A 0.12 bump could change what the haystack strips. | It is already the parse boundary the tree trusts (`build_email`); its own unit tests pin style/script stripping; our tests pin the *observable* (no codes from CSS, snippet non-empty for html-only mail), so a bump that breaks the posture goes red at the gate. |
| **The CSP string breaks the UI's own script/SSE.** | The directive set keeps `'unsafe-inline'` script/style and `connect-src 'self'`; `tests/ui.rs` asserts the policy on the served shell and the existing UI legs prove the shell still serves; exact spelling is verified against the served page before landing. |
| **Two `proposed` decisions ride the batch** — if the operator refuses one, the diff is not a revert. | 0031's refusal path: the fallbacks collapse to an empty view; the record-honesty rule stands on the issue's own fork. 0032's refusal path: the *other* door (kept-across-restart) becomes the batch's B3, converging on persistence instead. Either way one rule lands; nothing lands half-ruled. |
| **#57's fixes drift while the batch lands** (docs verified at one commit, landed at another). | E1's rule is "verify at HEAD **at fix time**"; #57 lands last inside the batch (its own commit), so its edits see the final code; the openapi/comments re-check is the review checklist's last line. |
