# The REST API — await, don't poll

[← README](../../README.md) · [Guide index](README.md)

`:8025` — the native API. The verbs a test actually needs:

| Verb | What it does |
|---|---|
| `GET /api/v1/inboxes` | the live inbox list, sorted by name — `limit` (default 100, ≤ 250, refused above) and `offset` page it |
| `GET/DELETE /api/v1/inboxes/{inbox}/messages` | list newest-first (add `view=compact` for the compact agent view without bodies — see below); wipe the inbox |
| `GET /api/v1/inboxes/{inbox}/await` | **block until arrival** — `to`/`from`/`subject`/`since_ms` filters, `count` (minimum one, even `count=0`), `limit`/`count` ≤ 250 (refused, never clamped), `timeout_ms` (≤ 60000: refused with 400 above, never clamped; a stopping server answers 503 `shutting down` instead of parking) |
| `GET /api/v1/inboxes/{inbox}/assert` | exact-count assertion — `count=0` asserts an empty filter; never clamped — 200 or a named failure |
| `GET /api/v1/inboxes/{inbox}/threads[/{key}]` | conversations, oldest first, stable keys |
| `GET /api/v1/inboxes/{inbox}/feed` | SSE: one snapshot, then one event per accepted mail |
| `POST /api/v1/inboxes/{inbox}/seed` | a fixture without SMTP — full parse + extraction; `html` preserved verbatim (supersedes `text`), CR/LF/NUL envelope fields refused |
| `GET /api/v1/messages/{id}` | one full mail |
| `GET /api/v1/messages/{id}/raw` | the RFC 5322 source as `message/rfc822` |
| `GET /api/v1/messages/{id}/deliverability[?reachability=true]` | the lazy deliverability report; `reachability=true` adds the opt-in link probe on a server started with `--link-reachability public\|loopback` (400 otherwise) — see [Deliverability reports](deliverability.md) |
| `DELETE /api/v1/messages/{id}` | remove one mail |
| `GET /api/v1/chaos` | inspect active chaos rules |
| `DELETE /api/v1/chaos` | clear active chaos rules |
| `GET /api/v1/webhooks` | inspect redacted targets |
| `DELETE /api/v1/webhooks` | clear webhook targets |
| `DELETE /api/v1/delete-all` | clear every inbox |

Every mail carries `text`, `html`, `links`, `codes` — extracted at ingest —
and `envelope_from`, the SMTP `MAIL FROM` reverse path: `""` for a bounce's
null sender `<>`, `null` only for mail stored before swarmail kept it (it is
never backfilled from the header `From`).
`codes` holds at most the first **64 distinct** codes, in extraction order
(decision [0049](../decisions/0049-extraction-compat.md)): a
text part and an HTML part that are direct siblings in one
`multipart/alternative` contribute the text's codes (the HTML's only when
the text has none); otherwise both bodies contribute in source order, so a
mailing-list footer after an HTML-only mail cannot hide its code. Mail
stored before the cap keeps its longer array verbatim — nothing is
re-extracted or truncated on restore. `links` is uncapped (bounded only by
the body): a quoted attribute's link ends at the attribute's own closing
quote, so `href="…?name=O&#39;Brien&amp;t=1"` stays whole, and balanced
parentheses, IPv6 brackets and interior commas are kept while trailing
sentence punctuation and unmatched closers are trimmed.
`GET /api/v1/inboxes/{inbox}/count` is the exact-count read;
`/openapi.json` serves the whole contract (3.1).

**Request bodies are capped at 2 MiB.** Every REST body — the seed, the
webhook PUT, chaos, the shims' `DELETE /api/v1/messages` — is read through
axum's default limit of 2,097,152 bytes. One byte more is refused before
parsing with axum's own plain-text `413` (`text/plain`, `Failed to buffer
the request body: length limit exceeded`), not the JSON `error` envelope,
and nothing is stored or applied. A larger fixture goes in over SMTP,
whose cap is 50 MiB. `POST /mcp` has the same 2,097,152-byte limit but
answers in JSON-RPC: HTTP 413 with a null-id `-32600` naming the limit
(see [MCP](mcp.md)).

The filters are exact about what they match. `to` is a case-insensitive
substring of **any recipient** — the `To` and `Cc` headers *and* the SMTP
envelope recipients — so a Bcc'd address matches, and `assert?to=…&count=0`
is not a "this address is in no header" check. `from` is the header `From`
address. Closed native query rules are method-specific: the existing
list/count/await/assert/thread/feed routes accept only their documented
keys; an unknown key is a `400` naming the route and key. These ten native
operations take no query keys: `DELETE /api/v1/inboxes/{inbox}/messages`,
`DELETE /api/v1/delete-all`, `GET` and `DELETE /api/v1/messages/{id}`,
`GET /api/v1/messages/{id}/raw`, `GET` and `DELETE /api/v1/chaos`,
`GET` and `DELETE /api/v1/webhooks`, and
`POST /api/v1/inboxes/{inbox}/seed`. Unsupported selectors (for example
`?subject=keep`, `?inbox=a`, or `?event=data`) and typos receive a native
JSON `400` naming the method, route and decoded key before the operation
reads or changes data/configuration. An empty-valued or bare key also
refuses; no query and a literal empty query keep their current behavior.
Seed's JSON syntax/media/size/data-shape errors retain precedence over a bad
query; a deserializable but domain-invalid seed body is checked after query
admission. Use the body-based `DELETE /api/v1/messages` selector to remove
specific ids. The Mailpit/MailHog list and search shims keep their upstream
tolerance of unknown keys — `/api/v1/messages?inbx=` is a typo that answers
the whole store — while the `latest` shims refuse them. This rule does not
change compat aliases, selective DELETE, chaos/webhook PUT, health, metrics,
UI or document routes.
The seed and webhook bodies are closed too (`subjet`, `secrt` refuse by
name). In JSON bodies an omitted optional field takes its default while a
present `null` is refused with the field named — on the seed's `from`,
`subject`, `text` and `html`, a webhook's `inbox`, `secret` and `ca_pem`,
and on a chaos rule's `error` alike (only a whole chaos event set to
`null` means "clear it"). Seed and webhook JSON data errors are `400` with
a JSON `error` (decision 0048): unknown keys, missing fields, wrong types,
nulls and wrong top-level shapes. Malformed JSON syntax stays the extractor's
plain-text `400`, missing/wrong media type its plain-text `415`, and
oversize its plain-text `413`. Chaos JSON data errors stay the extractor's
`422`. One inbox-name rule holds on every
surface — REST, MCP, UI, the shims, webhook inbox filters, SMTP AUTH and
POP3 `USER`: a blank name (empty after trimming whitespace) is refused, and
so is one carrying any control character (`inbox must not contain control
characters`); any other name is kept exactly as given, padding included, so
`" pad "` names the same inbox everywhere and never the inbox `pad`.
Previously stored rows are not renamed: a control-bearing name stored
before this rule still lists in `/api/v1/inboxes`, but its by-name routes
refuse it; its mail still shows in the unscoped `GET /api/v1/messages`
and is removed by id (`DELETE /api/v1/messages/{id}`, or `DELETE
/api/v1/messages` with an `ids` list). Nothing promises such a name ages
out.
