# Tasks — five fidelity closures

Ordered per the framing: **#39 first** (the P1 headline), then **#54**,
**#55**, **#56**, **#57** (docs-only, its own commit inside the batch).
Each task names the test that proves it; the coverage gate (exact
100/100/100, missed == 0) is the arbiter of "proved". Tests follow 0020:
no sleep-as-sync, no wall-clock validity, chaos at the poles, fresh
stores and per-test inboxes; no new test files — inline `#[cfg(test)]`
modules and the existing suites carry everything, and no new branch has a
cold path (each is reachable on the green path by a direct request).

## #39 — extraction fidelity (P1, first)

- [ ] 1. Add a total, linear `decode_refs` to `src/extract.rs`: the five
  XML named refs (case-sensitive) plus decimal/hex numeric references,
  decoded **per matched URL, after `URL_RE`, before the trailing-punct
  trim, on the HTML leg only**. Unknown names and out-of-range/surrogate
  numerics stay literal; the decoder never grows its input and never
  panics. `text/plain` stays byte-honest.
  **Proved by:** `src/extract.rs` inline units — `&amp;`/`&#38;` decode
  in the html leg; a literal `&amp;` in the text leg is untouched; the
  malformed corpus (unknown name, `&#55296;` surrogate, `&#1114112;`,
  `&#xZZ;`) stays literal; `&#0;`/`&#38;` never grow the string.

- [ ] 2. Replace `dedup`'s `sort + dedup` with an insertion-order seen-set
  dedup (no sort): `links[0]`/`codes[0]` are the mail's first values; the
  text leg scans before the html leg (existing loop order).
  **Proved by:** `src/extract.rs` units — first-occurrence order across
  both legs, dedup preserved, multipart/alternative case (the honest text
  variant's URL first).

- [ ] 3. Route `extract_codes`' HTML leg through
  `mail_parser::decoders::html::html_to_text` before `CODE_RE` — codes
  come from rendered text nodes only.
  **Proved by:** `src/extract.rs` units — a `<style>p{color:#333333}`
  block yields no code; a code in rendered text is still found; a code
  only in an attribute is a named miss (precision over recall).

- [ ] 4. Replace `build_email`'s synthesizing accessors
  (`body_text(0)`/`body_html(0)`, `src/smtp.rs:1248-1256`) with a
  part-type match: text part → `text`, html part → `html`, else `None`.
  The stored record holds only what the sender sent (0031 §1).
  **Proved by:** `src/smtp.rs` `build_email` units (text-only → `html:
  None`; html-only → `text: None`; alternative → both, sender's own) and
  `tests/api.rs` legs — a text-only SMTP mail answers `html: null` and an
  html-only mail answers `text: null` on REST.

- [ ] 5. Add the two display fallbacks of 0031 §3: `snippet`
  (`src/compat.rs:458`) and `plain_response` (`:683`) render
  `html_to_text(html)` at their own site when `text` is `None`; rewrite
  their "ingest always populates `text`" comments to state the rule.
  **Proved by:** `tests/compat.rs` — an html-only mail's Mailpit snippet
  and `/api/v1/message/{id}/plain` return a text view derived from the
  stored html; `links`/`codes` are unchanged by it.

- [ ] 6. Extend the `tests/api.rs` seed leg (`:314`) with the real-SMTP
  `&amp;`/`&#38;` cases (AC-39.1) and assert `codes` stays free of CSS
  digits on that wire.
  **Proved by:** `cargo test --test api` (real server, per-test inbox)
  and `cargo test --lib extract`.

## #54 — restored state matches live state

- [ ] 7. Seed `next_seq` at `COALESCE(MAX(seq), 0) + 1`
  (`src/persist.rs:90`); keep `record_insert`'s consume-then-increment
  (failed commits still consume a sequence; gaps are honest).
  **Proved by:** a `src/persist.rs` unit — open → insert → reopen →
  insert → reopen → insert asserts no duplicate `seq` (twice per the
  issue).

- [ ] 8. Make restore total: `load()` becomes `ORDER BY seq, rowid`
  (`:103`) and the schema gains `CREATE INDEX IF NOT EXISTS emails_seq
  ON emails(seq)` (the `emails_inbox` pattern). No format change, no
  version gate.
  **Proved by:** a `src/persist.rs` unit with a hand-built legacy fixture
  carrying duplicate `seq` (tie-break pinned to `rowid` order) plus an
  insertion-order-after-reopen leg; the existing
  `open_fails_on_an_unusable_file` /
  `open_refuses_an_unusable_data_file` pins still pass; the 0001 bench
  leg (`tests/burst.rs` cap-sized DATA) re-proves the synchronous commit.

- [ ] 9. Make `delete` remove the emptied inbox entry at the moment it
  already releases the watcher channel (`src/store.rs:601-603`): drop
  the `get_mut` guard before `inboxes.remove` (DashMap nesting). One
  rule: delete, clear and restore all end an emptied inbox (0032).
  **Proved by:** `tests/persistence.rs` — the inbox is absent from the
  listing live **and** after a reopen; `clear` and restore parity legs.

- [ ] 10. Assert the POP3 face of the rule: `USER` on a vanished inbox
  answers exactly like `USER` on a never-seen one, and the
  issue-#14 `watcher_count` probe returns to baseline — including on a
  reopened store.
  **Proved by:** `tests/pop3.rs` extensions (real server): the emptied
  maildrop's `USER`/`PASS` refusal, the watcher baseline live and after
  reopen.

## #55 — compat fidelity

- [ ] 11. Add `#[serde(alias = "IDs")]` to `IdsBody.ids`
  (`src/compat.rs:72-78`); 0011's rule is intact (absent/empty/wrong-typed
  still 400 with the named sentence).
  **Proved by:** `tests/compat.rs` — a `{"IDs": ["…"]}` delete removes
  exactly the named mails; the three refusal legs still answer 400.

- [ ] 12. Surface envelope recipients from the stored field: chain
  `email.recipients` into `Filter::matches`' `to` leg
  (`src/store.rs:46-57`, doc comment updated); `hog_message`'s `To` and
  `Raw.To` read the envelope (`src/compat.rs:646, 658`); Mailpit `Bcc`
  lists envelope-only recipients via one helper shared by
  `mailpit_summary` (`:501`) and `mailpit_message` (`:547`).
  **Proved by:** `tests/compat.rs` real-SMTP leg — `RCPT TO:<bcc@x.io>`,
  header `To: a@x.io`: `kind=to&query=bcc@x.io` → `total >= 1`, Hog
  `To`/`Raw.To` surface `bcc@x.io`, Mailpit `Bcc` lists it, and header
  to/cc stay in `Content.Headers`.

- [ ] 13. Fix the compat one-liners: `limit=0` means unset → the default
  page at the four staging sites (`:97, :110, :211, :264`; above 250
  still refuses); `GET /api/v1/message/latest` exists, reusing
  `mailpit_message` (404 only on an empty store); Mailpit search
  `total`/`unread` report mailbox totals (the unfiltered scope count).
  **Proved by:** `tests/compat.rs` — `limit=0` returns the default page
  and `limit=251` refuses (0030 §3); `latest` returns the newest mail in
  Mailpit's shape and 404s empty; the search envelope's totals match an
  unfiltered count, not the page size.

- [ ] 14. Refuse the Mailpit `to:`-prefix query by name — a 400 in 0018
  §3's sentence pattern (0025's discipline), never a silent empty result.
  **Proved by:** `tests/compat.rs` — the exact refusal sentence on a
  `query=to:…` search; plain queries and `kind=` staging unaffected.

## #56 — threads + web UI

- [ ] 15. Extend `strip_marker` (`src/threads.rs:104-114`) with the `aw`
  marker and whitespace tolerance between the word and the colon
  (`RE : Plan`); fold the subject fallback with `str::to_lowercase()`
  (`:171`); document the lowercase-vs-case-fold residual in the doc
  comment.
  **Proved by:** `src/threads.rs` units on `normalize_subject` —
  `AW: Plan`, `RE : Plan`, `aw: Plan`, `Über`/`über` group; `Re[2]:` and
  bare words stay alone; `ß` vs `ss` documented as the residual.

- [ ] 16. Fix the view: `message_view` passes the **raw** subject to
  `page` (escape exactly once; the `<h2>` keeps `esc`); inbox hrefs are
  `utf8_percent_encode(&name, NON_ALPHANUMERIC)` while visible text keeps
  `esc`; a missing message answers 404.
  **Proved by:** `tests/ui.rs` — `A & B` renders `A &amp; B` once in tab
  and `<h2>` (no `&amp;amp;`); an inbox `a#b` links to
  `/ui/inbox/a%23b` and resolves; a bogus id answers 404.

- [ ] 17. Add the CSP meta to `page()`'s head — the single choke point:
  `default-src 'none'`, inline script/style allowed, `connect-src
  'self'`, `img-src 'self' data:` (exact spelling verified against the
  served shell; remote loads inside mail bodies blocked).
  **Proved by:** `tests/ui.rs` — the served shell carries the policy;
  the existing UI legs prove script/SSE still serve.

- [ ] 18. Guard the SSE/fetch race: a monotonic `paintSeq` token in
  `src/static/ui.js` — the SSE listener bumps it, `load()` captures it
  before its `fetch` and abandons the paint if it moved.
  **Proved by:** `tests/ui.rs` asserts the served asset carries the
  guard (the existing asset-assert pattern; no browser engine in the
  loop).

## #57 — the record (docs-only; lands last, its own commit)

- [ ] 19. **Verify every #57 claim at HEAD before fixing; stale claims
  are dropped, not fixed.** Then: the decision index matches each file's
  recorded status and gains a real fourth header column
  (`docs/decisions/README.md`); `deny.toml` conforms down to
  `docs/house-rules.md` (remove CC0-1.0 / CDLA-Permissive-2.0 /
  `Apache-2.0 WITH LLVM-exception` unless the tree needs one — a genuine
  need goes to `.brokkr/state/run11-operator-edits.md`, never a silent
  widen; CC-BY-4.0 present, noted inline; Zlib stays — the house rules
  name it).
  **Proved by:** the row-by-row status comparison shipped as review
  evidence; `cargo deny check licenses` green; an allowlist diff against
  the house-rules sentence.

- [ ] 20. Fix the prose: `CONTRIBUTING.md` (`--bundle bundles/self`,
  `tests/common/mod.rs`, "100% of lines, branches and functions"),
  `README.md` (no `tests/smoke.rs`; `tests/p2.rs` listed; knob defaults
  as `host:port` socket addresses — a bare `SWARMAIL_SMTP_LISTEN=2525`
  shown as failing form; `RUST_LOG` documented; the startup log's POP3
  address consistent; the #55 known-differences list; the
  `to_lowercase` residual wording), `openapi.json` (the 503s on
  `DELETE /api/v1/messages` and `/api/v1/delete-all`,
  `/api/v2/search`'s `limit`/`start`, assert's `since_ms`/`subject`),
  and drop the stale `.github/workflows/ci.yml` citation.
  **Proved by:** each claim read against the code at fix time;
  openapi.json greps; the gates.

- [ ] 21. Fix the comments to match the code, never the reverse:
  `src/chaos.rs:16`, `src/store.rs:158`, `src/pop3.rs:11`,
  `src/pop3.rs:503`, `src/mcp.rs:245` (0014's no-trace refusal wording),
  `src/lib.rs`'s startup comment, `benches/writer_stall.rs`'s expect
  messages. Nothing re-litigates the `smtp=` startup line (4daa332) or
  0027 §7.
  **Proved by:** each comment read against its code; fmt/clippy/test
  green (comment-only diff, coverage unchanged).

## Batch close

- [ ] 22. Run the full gate set and the targeted proofs
  (`--test api`, `--lib extract`, `--test persistence`, `--test pop3`,
  `--lib persist`, `--test compat`, `--test ui`, `--lib threads`), confirm
  coverage is exact 100/100/100 with zero new files to defend, confirm
  `cargo deny check licenses` green with only `percent-encoding`
  declared, and re-check 0001/0002/0003 pins (burst/cap leg, await legs,
  capped-burst-then-restart eviction order).
  **Proved by:** the gate commands in plan.md's quickstart, all green.
