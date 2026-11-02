# Focused waits: the newest code, the one link

[← README](../../README.md) · [Guide index](README.md)

`swarmail_wait_for_code` and `swarmail_wait_for_link` (decision
[0094](../decisions/0094-focused-mail-waits.md), proposed) wait like
`swarmail_wait_for_email` — the same `inbox`, `to`, `from`, `subject` and
inclusive `since_ms` filters, `timeout_ms` defaulting to 5000 and capped at
60000, subscribe-before-scan, one deadline — and answer a decision instead of
whole emails. Both are read-only; neither fetches, renders or trusts a link.

```
$ curl -s -X POST localhost:8025/mcp -H 'content-type: application/json' \
    -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"swarmail_wait_for_link",
         "arguments":{"inbox":"quickstart","anchor_text":"^Confirm","host":"^app[.]test$"}}}'
… {"status":"selected","selected_url":"https://app.test/confirm/1","distinct_urls":1,"matched":1,
   "candidates":[{"url":"https://app.test/confirm/1","anchor_text":"Confirm your account",
   "rel":null,"source":"html","link_index":1,"email_id":"…","received_ms":…,"age_ms":…,
   "sender":{"address":"app@app.test"}}],"candidates_total":1,"candidates_truncated":false}
```

- **Codes:** the newest eligible email *in acceptance order* (not by stamp or
  id) decides. One distinct code there is `selected`, with its `email_id`,
  `sender`, `received_ms` and `age_ms`; several codes there are `ambiguous` —
  never resolved from older mail. Older eligible emails' codes are listed as
  `older_candidates`, each with its own provenance. `age_ms` is the age of
  the parse-start stamp, not code expiry; `sender` is the stored From, which
  can be the envelope fallback. Old matching mail answers at once: use a
  private inbox or a `since_ms` from before your action.
- **Links:** `anchor_text`, `host` and `path` are Rust regex *searches* (at
  most 1024 UTF-8 bytes and 1 MiB compiled each) that must all match the
  **same** occurrence. `host` is the parsed host (no credentials or port,
  IDNA as punycode, IPv6 bracketed) matched ASCII case-insensitively by
  default — a search, so an exact host is `^app[.]test$`; `path` is the
  serialized path without query or fragment. Exactly one distinct URL
  across every matching occurrence is `selected_url`; two spellings or two
  destinations are `ambiguous`, with every candidate's anchor text, `rel`
  and `text`/`html` source. Labels are lexical, decoded once and recorded at
  ingest: inline tags join, block tags and `br` separate, suppressed content
  and comments are omitted; an unknown label (text-part, historical or
  unattributable links) is `null` and never matches an `anchor_text`.
- **Shapes:** `status` is `selected`, `ambiguous` or `timed_out` (nothing
  eligible by the deadline — a normal result). `matched` counts eligible
  emails. Each list shows at most 250 entries, chosen after the whole
  snapshot is evaluated, with an exact `*_total` and a `*_truncated` flag;
  a winner appears only when `selected`. Selection is complete, so its cost
  grows with the retained inbox; narrow filters to reach omitted entries.
- **History:** mail stored before decision 0094 keeps its `links`; its
  candidates carry `null` label, `rel` and `source` and are never
  re-extracted. Email reads, feeds and webhooks are unchanged — occurrence
  metadata is reachable only through `swarmail_wait_for_link`.

- **Near misses on a timeout (decision [0111](../decisions/0111-near-miss-timeouts.md), proposed):**
  a timed-out `swarmail_wait_for_email`, `swarmail_wait_for_code` or
  `swarmail_wait_for_link` over an inbox that holds mail also carries
  `near_misses`, read from the store at the timeout with no extra retention:
  `arrivals` (the retained mail the wait did not answer with, newest first,
  at most 10, with exact `arrivals_total` and `arrivals_truncated`),
  `nearest_subject` (an entry or `null`) and `last_arrival_ms` (the
  `received_ms` of the newest retained mail, whatever `since_ms` says). An
  entry is `email_id`, `received_ms`, `from`, `to` (at most 5 distinct
  To/Cc/envelope addresses, exact `to_total`), `subject` and `missed` — the
  supplied `to`, `from` and `subject` filters that mail fails, so a sibling
  address reads `["to"]` and a typo `["subject"]`. Strings are clipped to
  256 bytes. `swarmail_wait_for_email` lists only mail its filter refused;
  the focused waits list every retained mail, `missed: []` meaning it passed
  your filters but carried no code or no link the selectors leave. A
  supplied `since_ms` limits `arrivals`. `nearest_subject` is a hint, not a
  selection: it is the mail that missed the subject alone and shares the
  most character pairs with it, and you still read its `to`. **No
  `near_misses` key means the inbox holds nothing**, so an empty-inbox
  timeout keeps its exact bytes. The read is of the wait's own inbox only;
  it is not access control (MCP is unauthenticated, decision
  [0004](../decisions/0004-per-test-inboxes-via-smtp-auth.md)).
