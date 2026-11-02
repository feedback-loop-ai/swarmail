# Attachments: list them, fetch one, read the invite

[← README](../../README.md) · [Guide index](README.md)

`swarmail_list_attachments {id}` and `swarmail_get_attachment {id, index}`
(decision [0110](../decisions/0110-attachments-and-ics.md), proposed) answer
from what ingest recorded once, at `250`, the way links and codes are: nothing
re-parses the raw message at query time. Both are read-only, MCP-only (no REST
route, no change to any Email, feed, webhook or compat payload), and refuse a
malformed argument by name (`-32602`) before the store is read.

```
$ ... "name":"swarmail_list_attachments","arguments":{"id":"0192…"}
{"id":"0192…","total":2,"truncated":false,"attachments":[
 {"index":0,"filename":"invite.ics","content_type":"text/calendar","size":412,
  "sha256":"9f2c…","stored":true,
  "ics":{"dtstart":"20261102T150000","dtstart_tzid":"Europe/Berlin",
         "dtend":"20261102T160000","dtend_tzid":"Europe/Berlin",
         "summary":"Planning, Q4","uid":"evt-1@x.io"}},
 {"index":1,"filename":"report.pdf","content_type":"application/pdf",
  "size":3145728,"sha256":"41ab…","stored":false,"ics":null}]}
```

Each entry carries the filename, the lowercased content type, the decoded size
in bytes and the lowercase-hex SHA-256 of exactly those bytes. A candidate is
a part of the parser's attachment list or any calendar-shaped part
(`text/calendar`, `application/ics` or a `.ics` filename), so a bare
`text/calendar` body with no disposition is listed too; an attached message is
one entry. `swarmail_get_attachment` answers `{index, filename, content_type,
size, sha256, encoding, content}`: `encoding` is `utf-8` when the bytes are
valid UTF-8 with no control byte beyond tab, newline and carriage return, else
`base64`. The bytes are never truncated, so they always match the listed hash.

A calendar part's `ics` holds `dtstart`, `dtend` (each with its `TZID`
parameter), `summary` and `uid` from the **first `VEVENT` only**, verbatim —
a `VTIMEZONE`'s `DTSTART` or a `VALARM`'s `SUMMARY` is never the event's —
with only `summary` unescaped. There is no timezone resolution: a wrong offset
is worse than the calendar's own text. A calendar with no `VEVENT`, invalid
UTF-8 or more than 1 MiB has `ics: null`.

The caps are constants, not configuration:

| Constant | Value | Bounds |
|---|---|---|
| `MAX_ATTACHMENTS` | 250 | entries listed per mail (`total` is the exact count, `truncated` says more exist) |
| `MAX_ATTACHMENT_STORED_BYTES` | 1 MiB | decoded bytes kept per mail, first-fit in list order; also the largest fetchable part |
| `MAX_LABEL_BYTES` | 1024 | every stored string (filename, type, each ICS field), cut at a character boundary |
| `MAX_ICS_BYTES` | 1 MiB | the largest calendar part the scanner reads |
| `MAX_LIST_JSON_BYTES` | 3 MiB | one list answer's entries as compact JSON, so it stays under the stdio bridge's 10 MiB cap once escaped |

Labels spelled to escape heavily can therefore list fewer than 250 entries, a
prefix of the list, with `truncated: true` beside the exact `total`; a stored
part cut this way stays fetchable by its index. Labels that need no escaping
always list all 250.

A part that does not fit the remaining budget is **metadata only**: it is
listed with its true size and hash and `stored: false`, and fetching it
answers an error naming its size, the 1 MiB budget and
`GET /api/v1/messages/{id}/raw`, which still serves the whole mail. Each stored
mail may therefore hold up to 1 MiB of decoded attachment bytes beside its raw
bytes, plus at most about 2 MiB of recorded labels (250 entries of at most
8 KiB each: filename, type and the ICS fields), in memory and in the data file;
as before, there is no global ceiling.
Mail stored by an earlier release, including a mail whose compact `attachments`
were recorded before these tools, answers "unknown" and is never backfilled.
