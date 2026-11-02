# MailHog / Mailpit shims

[← README](../../README.md) · [Guide index](README.md)

Point an existing client at the HTTP port and it works — the two shapes
test suites actually hit, answered from one store:

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
