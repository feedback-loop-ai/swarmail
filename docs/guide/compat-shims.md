# MailHog / Mailpit / MailSlurper shims

[← README](../../README.md) · [Guide index](README.md)

Point an existing client at the HTTP port and it works — the MailHog and
Mailpit shapes test suites actually hit, answered from one store. A third
shape, MailSlurper's service API, is served on a listener of its own that
you enable ([below](#mailslurper-service-api)):

| Endpoint | Behavior |
|---|---|
| `GET /api/v1/messages` | Mailpit's summary envelope, newest first — the receive time, then id, as `latest` uses, scoped or not; `?inbox=` scopes, `?limit=`/`?start=` page; each `Snippet` is built from at most the first 8192 bytes of its body |
| `GET /api/v1/message/{id}` | Mailpit's full message shape (`Text`, `HTML`, `ReturnPath`, …) |
| `GET /api/v1/message/latest`, `…/latest/raw`, `…/latest/headers` | the newest mail — the maximum receive time, then id — in Mailpit's message shape, as its raw source, or as its header map; `?inbox=` scopes it to one inbox (404 when that inbox is empty, whatever the others hold), no `?inbox=` means the whole store; a nonblank `?query=` is a `400` — search, then fetch the id |
| `GET /api/v1/message/{id}/plain`, `GET /api/v1/messages/{id}/plain` | the text part as `text/plain; charset=utf-8` — html-only mail is rendered to text at request time |
| `DELETE /api/v1/messages` | Mailpit's `{"ids": [...]}` delete (`IDs` accepted alike) — the named ids and nothing else; every other body (absent, empty, unparseable, `{"ids": []}`) is a `400` that deletes nothing — including MailHog's bodyless wipe (decision 0011) |
| `DELETE /api/v1/delete-all` | swarmail's own explicit wipe of every inbox — not an upstream route |
| `GET /api/v1/search` | Mailpit search: `kind=from\|to\|subject\|containing`, `?query=` required; the envelope's `messages_unread` is the match count |
| `GET /api/v2/messages` | MailHog v2 `{total, count, start, items}` with `data.Message` shapes; a page whose JSON would pass 64 MiB is refused whole by name |
| `GET /api/v2/search` | MailHog search: `kind=from\|to\|containing` present and one of the three, as upstream requires; a bare `400` on an unknown kind, an absent kind or an empty query; `?limit=`/`?start=` page |
| `GET /api/v1/messages/{id}/download`, `GET /api/v1/message/{id}/raw` | the RFC 5322 source as `message/rfc822` |

**Deliberate omissions** — swarmail has no counterpart and a shim would
lie: attachments and inline images (the fields exist, the arrays are
empty), read/unread state, tags (always `[]`), the send API (mail arrives
by SMTP), Mailpit's `mime/part` routes, `latest` on the plain/attachment
routes, and the `to:`-style query prefixes: in a containing search, every
whitespace-separated word that opens with a field name and a colon
(`to:`, ` from:alice`, `plan subject:x`, an unknown `foo:`) answers a named
`400` — the prefix grammar is not implemented; swarmail takes `kind=`.
Both shims remove every double quote from each word for refusal recognition
only, then look past one leading `(` and one exclusion `-` or `!`
(decision 0061, accepted 2026-10-26). Thus `-to:x`, `!from:alice`,
`"to:x"`, `-"subject:x"`, `(to:x)`, `"-to:x"`, `"!from:alice"`,
`"-subject:a b"`, `to":x` and `"to":x` refuse by the field's name instead of
silently matching nothing. A URL's `://` is exempt, so `https://host/path`
(and `-https://host/path`) stays a literal search, as do `-word` and
`"a phrase"`, but other colon words (`mailto:x`, `"note:x"`) refuse too.
Accepted queries execute with their original quotes and markers; no quoting,
grouping or negation grammar is implemented. Supported explicit-kind values
stay literal (Mailpit's `subject` extension remains outside MailHog's kinds).
swarmail's own single-message route is
`/api/v1/messages/{id}`; the Mailpit-style singular `/api/v1/message/{id}`
is the shim.

**Known differences** — the shapes are close but not byte-identical, and
the divergence is the honest part: a malformed `limit=`/`start=` answers
axum's plain-text `400` (not Mailpit's JSON error envelope — matching it
means a custom query extractor for one status-code shape); a blank
`?inbox=` is a `400`, never the whole store. The sender the shims report —
MailHog `From`, `Raw.From` and `kind=from`, Mailpit `ReturnPath`, the
synthesized `Return-Path` — is the SMTP envelope sender, as upstream's is
(`""` and `<>` for a bounce); only mail stored before swarmail kept the
envelope falls back to the header `From`. Mailpit's `From` and the native
`from` filter stay on the header. Every compat list and search, scoped by
`?inbox=` or not, and `latest` order by the receive time, then the id —
so at a stable store `messages?inbox=x&limit=1` is `latest?inbox=x`. The
receive time is stamped when parsing starts, so a large mail accepted after
a small one can sort behind it; the native inbox list, POP3 numbering and
pruning keep acceptance order, and can disagree with the compat views.
`latest` does not read Mailpit's `tz`: `?tz=` is a `400` naming it, like any
other unknown key there.

## MailSlurper service API

A suite that reads and clears mail through MailSlurper's service API (for
example Alkemio's `@alkemio/tests-lib`: `getVerificationLink()` and
`deleteMailSlurperMails()` against `MAIL_SLURPER_ENDPOINT`) can point at
Swarmail by changing that one variable. Enabling it is the operator's choice,
made by starting the server with a flag, and it is **off by default**.

**It is a global view.** MailSlurper has one mailbox, so this shim has no
inbox selector: `GET /mail` and `GET /mailcount` read **every inbox**, and
`DELETE /mail` with `{"pruneCode":"all"}` **clears every inbox**. Enabling
it collapses per-inbox isolation for the clients of this listener: such a
client sees every session's mail, and its cleanup deletes every session's
mail. Run one Swarmail (one process and one data file) per suite that uses
the MailSlurper API. There is no `?inbox=` extension, because a scoped read
beside a global wipe would be half an isolation story. Per-inbox caps still
evict the oldest mail of each inbox only (decision 0003), so a mail can
leave between two reads, and `GET /mail/{id}` is a `404` for a mail that was
accepted and later evicted.

```bash
swarmail serve --mailslurper-listen 127.0.0.1:4437     # or SWARMAIL_MAILSLURPER_LISTEN
MAIL_SLURPER_ENDPOINT=http://localhost:4437/mail npm test     # the consumer, unchanged
docker run --rm --stop-timeout 30 -p 1025:1025 -p 8025:8025 -p 4437:4437 \
  -e SWARMAIL_MAILSLURPER_LISTEN=0.0.0.0:4437 ghcr.io/feedback-loop-ai/swarmail:main
```

**Where it is served.** `--mailslurper-listen <host:port>`
(`SWARMAIL_MAILSLURPER_LISTEN`) has no default: unset means no socket and no
route. The address is yours, so the listener is loopback only if you write a
loopback address. It sits behind the same Host/Origin guard as the HTTP
plane, keyed to its own port ([network posture](network-posture.md)), and
speaks plain HTTP: terminate TLS at your ingress. **The HTTP port serves
none of these routes, under any prefix** (`GET /mail` there is a `404`). An
ingress that publishes the shim under a prefix, such as
`https://test-alkem.io/mailslurper-api/mail`, strips the prefix and forwards
to the listener's root.

On Kubernetes, enable it through the chart's env contract
(`--set config.SWARMAIL_MAILSLURPER_LISTEN=0.0.0.0:4437`) and publish port
4437 with a Service of your own. The chart does not expose the port yet, and
with `networkPolicy.enabled` its policy does not admit 4437 either (add an
`ingressPeers` entry and a policy for the port):

```yaml
apiVersion: v1
kind: Service
metadata: { name: swarmail-mailslurper }
spec:
  selector:
    app.kubernetes.io/name: swarmail
    app.kubernetes.io/instance: <your release name>
  ports: [{ name: mailslurper, port: 4437, targetPort: 4437 }]
```

| Endpoint | Behavior |
|---|---|
| `GET /mail` | `200 application/json`, `{"mailItems":[…],"totalPages":n,"totalRecords":n}` over every inbox, newest first (the receive time, then id); `?pageNumber=` pages (1-based, default 1), 50 to a page; `totalRecords` counts the whole mailbox, not the page; a page past the end is `200` with the totals and no `mailItems`; **`mailItems` is omitted whenever the page is empty**, so an empty mailbox is exactly `{"totalPages":0,"totalRecords":0}`; `pageNumber` of `0`, a negative number or a non-integer is a `400` naming it |
| `GET /mail/{id}` | one item (the shape below); an unknown id is a `404` |
| `GET /mail/{id}/message` | the item's `body` as `text/html; charset=utf-8`, with `Content-Security-Policy: sandbox` and `X-Content-Type-Options: nosniff` (the HTML is the sender's); an unknown id is a `404` |
| `GET /mailcount` | `{"mailCount":n}` over every inbox |
| `DELETE /mail` | with the JSON body `{"pruneCode":"all"}`: clears every inbox, then `200 text/plain; charset=utf-8` with the number of mails removed (`2`) |

The item, in this key order: `id`, `dateSent` (the receive time as
`YYYY-MM-DD HH:MM:SS` in UTC), `fromAddress` (the SMTP envelope sender, as
the other shims report it), `toAddresses` (the envelope recipients as bare
address strings, so a Bcc'd recipient is listed; the header `To` addresses
only for a row that recorded no envelope), `ccAddresses` (the header `Cc`
addresses, bare), `subject` (the stored subject, HTML-escaped as MailSlurper
serves it: `&` `<` `>` `"` `'` become `&amp;` `&lt;` `&gt;` `&#34;` `&#39;`),
`body` (the stored HTML when the mail has one,
else its stored text, else `""`, exactly as decoded at ingest: not
re-rendered, unescaped or sanitized, so an `href`-only link stays literal)
and `attachments` (always `[]`).

**The wipe refuses by name.** Only a request with `Content-Type:
application/json`, no query string and a body that is an object whose sole
key `pruneCode` is exactly the string `"all"` deletes anything. MailSlurper's
age-based codes `60plus`, `30plus` and `2wksplus` are each a `400` that names
the code: swarmail prunes only the oldest mail past a per-inbox cap (decision
0003) and adds no time-keyed eviction. Everything else (an absent, empty or
non-JSON body, `{}`, `"ALL"`, an unknown code, a non-string, an array, an
extra key, another content type, any query key) is a `400` that deletes
nothing. A failed commit is `503 storage commit failed`, never a count.

**Known differences** (decision 0118):

- *Not served* (a plain `404`): `/mail/{id}/messageraw`, attachment
  download, `/pruneoptions`, `/version`, login and logout; there is no CORS.
- *Omitted item fields*: `xmailer`, `mimeVersion`, `boundary`,
  `contentType`, `transferEncoding` and `bccAddresses`. Swarmail does not
  store these headers as fields, and inventing them or parsing MIME at
  request time would be wrong.
- *Closed query*: MailSlurper's search, sort and date parameters
  (`message`, `from`, `to`, `start`, `end`, `orderby`, `dir`), and any other
  key, are a `400` naming the key rather than being ignored. The other
  routes take no query at all.
- *Fixed paging*: the page size is 50, upstream's own, and cannot be changed (no flag, env or `?limit=`; a client that wants more uses `pageNumber`). The Alkemio client reads page 1 only. A page whose
  JSON would pass 64 MiB is refused whole, by name, with a pointer to `GET
  /mail/{id}`.
- *`dateSent` is the receive time, in UTC*: the format is upstream's
  (`YYYY-MM-DD HH:MM:SS`), but upstream formats the message's parsed `Date`
  header in the server's local zone. Swarmail stamps receive time at ingest
  (decision 0005) and reads no header at request time.
- *Order is the receive time*: the list is newest first by receive time,
  then id. Upstream orders by `dateSent` descending. A sender with a skewed
  clock therefore sorts differently here, and the mail a suite has just
  triggered is always the newest one, which is the mail its link scan wants
  first. Once the store holds more than 50 mails the two orders can put
  different mail on page 1.
- *A page past the end is a `200`* with the totals and no `mailItems`.
  Upstream may answer `404 Page number out of range` (recalled, see below).
- *An unknown id is a `404`*; upstream answers `500`.
- *`ccAddresses` is an extra key*: upstream's item, as the review of this
  shim read it, carries no such key. A JSON client ignores it.
- *A list page over 64 MiB is a `400`*, refused whole by name (never
  truncated, decision 0044). Fifty items under 64 MiB is the normal case; a
  client that does not handle the `400`, such as the Alkemio helper, fails.
- *Identity encoding*: the client's `Accept-Encoding` is a preference, and
  responses are never compressed.
- *Errors* are swarmail's JSON `{"error": …}` envelope (a malformed query is
  axum's plain text), not MailSlurper's plain text.
- *Not in `/metrics`*: requests to this listener are not counted in
  `swarmail_http_requests_in_flight`, which reports the HTTP plane's requests.
- *`subject` is HTML-escaped, `body` is not*: the item's `subject` goes
  through the five substitutions of Go's `html.EscapeString` (`&` →
  `&amp;`, `<` → `&lt;`, `>` → `&gt;`, `"` → `&#34;`, `'` → `&#39;`), once,
  on `GET /mail` and `GET /mail/{id}`, so a subject that already reads
  `&amp;` is served `&amp;amp;`. Only `"` → `&#34;` is observed (the Alkemio
  trial compares subjects with it); the other four are inferred from
  MailSlurper being Go, not read from its source. The native API, Mailpit
  and MailHog keep the raw subject, and search matches the raw subject.
  Escaped bytes count toward the 64 MiB page. The addendum of 2026-11-04 to
  decision 0118 records this and is proposed until the operator accepts it.
- *Ingest still counts*: a mail is listed the moment its SMTP `250` is read,
  and a wipe that follows may remove it. That is what "all" means.
- *Reported from upstream source by the review of this shim*, not
  re-checked by the operator yet: the page size of 50, the routes, the age
  codes `60plus`, `30plus` and `2wksplus`, the `dateSent` format and its
  local-zone, `Date`-header origin, `toAddresses` from `RCPT TO`, the
  `{"mailCount":n}` envelope and the `500` for an unknown id.
- *Still recalled, not read from source*: the `pageNumber` spelling, the
  `/message` content type, the search-key names, the plain-text delete answer
  and the past-the-end `404`. The fields the Alkemio client reads
  (`mailItems`, `totalRecords`, `subject`, `body`, `toAddresses`, and the
  empty-mailbox omission of `mailItems`) are from the operator's
  description of that client. Check the rest against
  `mailslurper/mailslurper` before you rely on them (decision 0118 §7).
