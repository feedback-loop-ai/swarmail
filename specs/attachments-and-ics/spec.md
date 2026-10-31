# Attachments and ICS at ingest

Date: 2026-10-31. Epic #164, issue #159. Base: main `d892acb` (the merged MCP
work of #152, #153 and #154), on the zero-Codex `claude-careful` recipe
(accepted 0098/0099, residual rework under accepted 0102).

Decision: 0110 — [Attachments listed and fetched from stored bytes, ICS fields at ingest](../../docs/decisions/0110-attachments-and-ics.md), proposed.

The restore and compact-view contract below was reconciled with #155 on
2026-11-01 after review (0110's dated note); no behaviour changed.

## Feature and rationale

An agent handling mail needs to know what a message carries before it pays
for bytes, and then needs the bytes of one part — an invoice, a calendar
invite, a report — without decoding MIME itself. Today the only path is the
REST `/raw` download of the whole message. This change adds two read-only MCP
tools, `swarmail_list_attachments` and `swarmail_get_attachment`, served from
fields extracted once at ingest, and parses the key fields of a calendar
(ICS) part — start, end, summary, and the event's UID — at the same moment,
the way links and codes are parsed (accepted
[0005](../../docs/decisions/0005-extraction-at-ingest.md)).

The design preserves every frozen guarantee:
[0001](../../docs/decisions/0001-synchronous-insert-lossless.md) (the walk,
the hashes and the ICS scan are total and run inside the offloaded parse that
precedes the synchronous insert, so a `250` still means stored),
[0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md) (the
waits are untouched) and
[0003](../../docs/decisions/0003-oldest-only-pruning.md) (the new columns ride
the same prune transaction; nothing changes which row is evicted). Compatible
implementation may proceed under
[0045 §8](../../docs/decisions/0045-delivery-repairs.md); the proposal does not
accept itself.

## Synthesis of the panel

Both positions were read in full (`.brokkr/state/design/positions/`, run-local
and uncommitted). The intake framing came from the intake result, since
`.brokkr/state/tasks/` does not exist in this run. The rulings, and what each
losing side costs:

| Question | Simplicity | Robustness | Ruling |
|---|---|---|---|
| Where the bytes come from at fetch | re-decode `raw` under the report's job permits, hash-mismatch guard | stored decoded copies, 10 MiB per mail, child table | **Stored copies** (robustness's principle) under a **1 MiB per-mail budget** in one BLOB column (simplicity's scale). 0005 stays literal; no permit plumbing; no cold hash-mismatch arm. Simplicity loses nothing it cared about except parts over 1 MiB, which it also refused. Robustness loses 9 MiB of budget to the bridge cap arithmetic (0110 §3) and its child table to a second delete site on every prune. |
| Count cap | 250 stored entries, `truncated` = `len == 250` | 1000 scanned, 100 listed | **250 listed entries plus the exact candidate total** stored beside them, so truncation is exact and never over-reports. One constant, not two. |
| Per-line ICS cap | none | 64 KiB | **None.** Input is bounded at 1 MiB and every field at 1024 bytes; a third constant bounds nothing. |
| SHA-256 | rustls's ring hash provider, no manifest change | `sha2` under operator ruling, else ship without the hash | **rustls's ring provider** with known-answer tests. A hash is required by the issue; a ruling that blocks the delivery is not needed because the tree already links SHA-256. The cost is one unusual helper, commented. |
| ICS scope | first `VEVENT`, depth-tracked, verbatim times with `TZID` | same | **Adopted from both**, plus `UID` (the invite task reads it). |
| Listed parts | parser attachments plus bare calendar parts | parser walk | **Parser attachments plus bare calendar parts.** A `text/calendar` body with no disposition is a real invite shape and never reaches the attachment list otherwise. |
| Public surface | two MCP tools, no REST | MCP tool plus REST route | **MCP only.** `/raw` already serves bytes (0022); a REST contract is a separate decision. |
| HTML attachment rendering | `rendered_text` on fetch | — | **Out of scope.** The one renderer is `compat`'s and #155 owns its sharing; fetch returns the part's own bytes. |
| Caps as config | constants | constants | **Constants**, both agreed. |
| Backfill of old rows | never | never | **Never**, both agreed (0094's precedent). |

One claim in the robustness position is corrected rather than adopted: a
streaming hash cannot make a 50 MiB message "cost cap plus constant memory",
because mail-parser decodes every part while parsing. The transient decode is
the existing, 50 MiB-bounded ingest behaviour (0022); this change bounds what
is retained (0110 §8).

## Contract

### Ingest

`build_email` records, for the outer message, every candidate part in part
order: a part in the parser's attachment list, or a part whose content type is
`text/calendar` or `application/ics`, or whose filename ends `.ics` (ASCII
case-insensitive). An attached `message/rfc822` is one entry; its inner parts
are not walked. For each of the first `MAX_ATTACHMENTS` (250) candidates:

| Member | Value |
|---|---|
| `filename` | the parser's attachment name (disposition filename, else the content-type name), or null; at most `MAX_LABEL_BYTES` (1024), cut at a character boundary |
| `content_type` | ASCII-lowercased `type/subtype`, or the bare type when the header has no subtype, or null when the part has no content type; same cap |
| `size` | the decoded part length in octets — the bytes fetch returns, which for a text part are the parser's UTF-8 decode |
| `sha256` | lowercase hex SHA-256 of exactly those bytes |
| `stored` | true when the part was charged to the budget (below) |
| `ics` | the object of "ICS fields" for a calendar-shaped part with a first `VEVENT`; otherwise null |

The exact candidate count is kept as `total`. Beyond the 250th candidate only
the count grows; nothing is hashed, scanned or stored.

**Budget.** `MAX_ATTACHMENT_STORED_BYTES` is 1 MiB (1 048 576). Walking the
listed entries in order, a part whose `size` is at most the remaining budget is
stored and the remaining budget is reduced by `size`; a larger part is not
stored and the budget is unchanged (first-fit). A part of exactly the whole
budget is stored. All arithmetic is on octet counts with no allocation from a
claimed length.

`Email.attachments` is `Some(list)` for every parsed mail, `Some([])` when
there are no candidates, and `None` when the parser could not read the mail
(restore adds the labels-only list of "Persistence"); it and the bytes are
`#[serde(skip)]`. Seeded mail shares `build_email`
(0010) and so gets the same treatment.

### ICS fields

The scanner reads a calendar-shaped part's decoded content when it is valid
UTF-8 of at most `MAX_ICS_BYTES` (1 MiB); otherwise `ics` is null. It unfolds
(CRLF to LF, then every `LF SP` and `LF HT` removed), splits on LF, and
tracks nesting: scanning starts at the first `BEGIN:VEVENT`, every nested
`BEGIN:` raises depth and its `END:` lowers it, properties are read only at
depth one, and `END:VEVENT` stops the scan. Property names compare ASCII
case-insensitively; the value starts after the first `:` outside a
double-quoted parameter value; the first occurrence of a name wins.

| Field | Value |
|---|---|
| `dtstart`, `dtend`, `uid` | the property value verbatim (a `Z` suffix stays as sent) |
| `dtstart_tzid`, `dtend_tzid` | the `TZID` parameter of that property, quotes stripped, else null |
| `summary` | the value with RFC 5545 text escapes resolved (`\\`, `\;`, `\,`, `\n`, `\N`) |

Each field is capped at `MAX_LABEL_BYTES` and is null when absent. No
timezone resolution, no conversion to an instant, no `RRULE`, `ATTENDEE`,
`ORGANIZER` or `METHOD`, and only the first `VEVENT`. Malformed input never
panics and never refuses the mail.

### Tools

Both tools are appended after the two focused waits (0094 §7's pattern), are
`readOnlyHint: true` on June and November with a closed `outputSchema`, and are
text-only on March, which keeps every earlier descriptor and result byte
(0082 §1). Arguments are validated before any store read: a missing or
non-string `id`, a non-integer or negative `index`, and an unknown key refuse
`-32602` by name (0044 §3). Store-state refusals are `isError` text results.

**`swarmail_list_attachments {id}`** answers
`{"id", "total", "truncated", "attachments": [entry…]}`; `entry` is
`{index, filename, content_type, size, sha256, stored, ics}` with `index` the
position in the list and `ics` the object above or null; `truncated` is
`total > attachments.len()`. The answer is the longest prefix of the stored
entries whose compact JSON fits `MAX_LIST_JSON_BYTES` (3 MiB; 0110 §3), so
labels spelled to escape can list fewer than 250; labels that need no escaping
never do, and an entry cut this way keeps its index for
`swarmail_get_attachment`. An unknown mail answers `message {id} not found`;
a mail whose list is unknown or labels-only (0110 §6) answers the named
refusal `the attachments of message {id} are unknown: …`.

**`swarmail_get_attachment {id, index}`** answers
`{"index", "filename", "content_type", "size", "sha256", "encoding",
"content"}`. `encoding` is `utf-8` and `content` the text when the bytes are
valid UTF-8 containing no control byte other than TAB, LF and CR; otherwise
`encoding` is `base64` and `content` the standard base64 of the bytes. Refusals,
in order: unknown mail; unknown or labels-only list; `index` past the list; a part not stored
— the sentence names the part's size, the 1 MiB budget and the
`GET /api/v1/messages/{id}/raw` route. Bytes are never truncated.

### Persistence

Two nullable columns are added through `ADDED_COLUMNS` and written in
`record_insert`'s transaction: `attachments_json TEXT`, a closed object
`{"total": n, "parts": [ {filename, content_type, size, sha256, stored, ics} … ]}`
with every member required (nullable members present as null, 0097 §2's
rule), and `attachment_bytes BLOB`, the concatenation of the stored parts' bytes
in list order (empty, not NULL, when nothing is stored). Ingest writes both as
NULL exactly when `attachments` is `None`.

Restore is strict and accepts exactly three shapes (0110 §6); none is ever
backfilled:

| `attachments_json` | `attachment_bytes` | Restores as | Compact view | Both tools |
|---|---|---|---|---|
| NULL | NULL | `None`, unknown: a row from before the columns, or a mail the parser could not read | `"attachments": null` | the unknown refusal |
| begins `[`: an array of `[filename, content_type, size]` three-tuples, both labels nullable (0108 §5's first listing) | NULL | a labels-only list: `total` is the tuple count; no hash, bytes or ICS fields | the labels, as #155 served them; `[]` is known empty and omitted | the unknown refusal |
| the closed object above, at most 250 parts and no more than `total` | a blob exactly as long as the stored parts' sizes sum | the list, each stored part's bytes split from the blob in list order | the recorded labels | list and fetch |

Every other pair refuses the restore with the column named: a malformed
tuple array, a tuple array beside a blob, a malformed or open object, too many
parts, or a NULL `attachments_json` beside a blob names `attachments_json`;
any value not beginning `[` beside a NULL blob, or a blob whose length is not
the stored sizes' sum, names `attachment_bytes`.

### The compact view's attachments (reconciliation with #155)

The compact view of 0108 — modern MCP search and latest, `fields` naming
`attachments` on the four modern reads, and REST `view=compact` — reads its
`attachments` and `attachments_total` from this list (0110, reconciliation
with #155). Relative to #155, for mail ingested from this change on, its
entries also include bare calendar parts (`text/calendar`, `application/ics`
or a `.ics` filename, in part order), and `filename` and `content_type` are
cut to `MAX_LABEL_BYTES` at a character boundary. `content_type` is now
ASCII-lowercased by ingest, which moves no byte at the locked mail-parser
0.11.9 because the parser already lowercases the type and subtype. `size` is
the same decoded length, at most 250 entries show, and `attachments_total` is
still the exact count. Rows restored as unknown or labels-only serve exactly
what #155 served them.

### What does not change

Every existing full Email serialization, every REST response without `view`
(0108 §4), SSE feed item, webhook payload, compat page, March descriptor and
result byte; the one existing output that moves is the compact view's
attachments, as the section above states. The DATA path, the
50 MiB message cap, the watchers, the prune rule and the restore order. No
dependency, feature flag or configuration knob is added; `Cargo.lock` is
unchanged.

## Acceptance Criteria

- **AC1 — list.** For a real SMTP-delivered mail with N candidate parts
  (`tests/mcp.rs`): `total` is N, each entry carries `filename`,
  lowercased `content_type`, `size` in decoded octets and a 64-character
  lowercase-hex `sha256`; the NIST `abc` vector and the empty-string vector
  prove the digest inline. A bare `text/calendar` body with no disposition is
  listed; an attached `message/rfc822` is one entry. An unknown id answers
  `isError`.
- **AC2 — fetch.** For a base64 part and a quoted-printable part, the fetched
  `content` decodes to bytes whose length is the listed `size` and whose
  SHA-256 (checked independently in the evals crate with its own `ring`
  against the fixture's invite, and inline against the known vectors) is the
  listed hash. A binary part answers `base64`; a text part answers `utf-8`;
  a text part with a NUL byte answers `base64`. An `index` past the list, a
  negative or non-integer `index` and an unknown id answer the stated
  refusals, the first by `-32602`.
- **AC3 — caps pinned.** `MAX_ATTACHMENTS`, `MAX_ATTACHMENT_STORED_BYTES`,
  `MAX_LABEL_BYTES`, `MAX_ICS_BYTES` and `MAX_LIST_JSON_BYTES` are single named constants in one
  module with the values of 0110 §3, and the tool descriptions state the 250
  and 1 MiB figures. On the wire: a single part of exactly 1 048 576 octets
  is stored and fetched whole; one of 1 048 577 is listed with `stored: false`
  and fetch refuses naming the size, the budget and the raw route; two parts
  of 524 288 octets are both stored and a following 1-octet part is not;
  a 1 MiB part followed by a 300-octet ICS leaves the ICS unstored but with
  its `ics` fields present; a mail with 251 one-line parts lists 250 entries
  with `total: 251` and `truncated: true`, and a mail with exactly 250 lists
  250 with `truncated: false`; a 1025-byte filename is cut at a character
  boundary to at most 1024 bytes.
- **AC4 — hostile attachment.** A 50 MiB-cap mail with thousands of minimal
  parts, and one with a 40 MiB base64 part, each get `250`, are queryable at
  once, list under the caps, and retain at most 1 MiB of attachment bytes —
  asserted through the data file's `attachment_bytes` length and the list's
  `stored` flags, not a claim about the parser's transient allocation, which
  0022 already bounds. No cold path needs a seam; all arms are reachable with
  generated mail on the real wire.
- **AC5 — ICS at ingest.** A `text/calendar` attachment with folded lines, a
  `VTIMEZONE` carrying its own `DTSTART`, a `VALARM` carrying its own
  `SUMMARY`, a `TZID` on `DTSTART`, a `Z`-suffixed `DTEND`, an escaped
  `SUMMARY` and two `VEVENT`s yields exactly the first event's verbatim
  `dtstart` with its `dtstart_tzid`, `dtend`, unescaped `summary` and `uid`;
  `application/ics` and a `.ics`-named `application/octet-stream` part are
  scanned too; a calendar with no `VEVENT`, one over 1 MiB and one that is not
  UTF-8 yield `ics: null`; a malformed calendar still answers `250` and the
  mail is queryable. Unit tests in the scanner's module cover every branch.
- **AC6 — no query-time reparse.** `tests/persistence.rs` restarts the server
  and reads the list and a fetched part back byte for byte from the restored
  row; a row with both columns `NULL` restores, serves `"attachments": null`
  in the compact view and answers the unknown refusal; a tuple-array row of
  0108's first listing with `attachment_bytes` `NULL`, populated or `[]`,
  restores labels-only, serves its labels in the compact view as #155 did and
  answers the unknown refusal from both tools; a damaged `attachments_json`,
  a malformed tuple array, a tuple array beside a blob, a NULL
  `attachments_json` beside a blob, an `attachments_json` not beginning `[`
  beside a NULL blob and a blob of the wrong length refuse the restore naming
  the column (across `tests/persistence.rs` and `src/persist.rs`'s tests).
  The fetch handler holds no
  parser call and no `raw` read — a source-level assertion in the handler's
  tests, as `compat`'s renderer carve-out is pinned.
- **AC7 — frozen guarantees.** `tests/burst.rs`, `tests/p2.rs`,
  `tests/lifecycle.rs`, `tests/stdio.rs` and `tests/mcp.rs` stay green; an
  ICS mail is listed and fetched immediately after its `250`; a mail that
  trips the per-inbox cap still evicts the oldest and its columns go with the
  row.
- **AC8 — MCP surface.** Both descriptors declare `inputSchema` and, on June
  and November, a closed `outputSchema` and `readOnlyHint: true`; March
  carries neither and keeps its earlier bytes; `tests/mcp_revisions.rs`'s
  catalogue grows to seventeen tools with eleven reads; every success
  carries matching `text` and `structuredContent`; the stdio bridge delivers
  a 1 MiB fetch under its 10 MiB cap in both encodings, and the list answer
  for 250 parts of eight 1024-byte labels under that cap on every revision,
  for labels of control bytes and of backslashes, with `truncated: true` and
  the exact `total`; 250 parts of 1024-byte plain labels list all 250.
- **AC9 — metric.** The invite and ICS tasks run offline only: the evals
  crate's existing tests plus one fixture-rig test that waits for the real
  invite, lists it, matches `ics.uid` and `ics.dtstart` to the oracle's
  `ics_property` of the downloaded ICS, fetches `invite.ics`, checks the bytes
  and the hash with the crate's own `ring`, and accepts the invitation. The
  two tool names join `SWARMAIL_TOOLS`; the catalogue, oracle and task prompts
  are unchanged. The report says **not measured: no paid trial** and that
  `invite-time` does not exercise the attachment tools (0055 §4).
- **AC10 — gates and honesty.** Every gate of
  [house rules § Gates](../../docs/house-rules.md#gates--part-of-every-change)
  runs locked and clean, the evals crate's tests included, with exact 100%
  coverage and no `coverage(off)`; `benches/ingest.rs` is run before and after
  and the numbers are reported as measured, never asserted; `Cargo.lock` and
  `deny.toml` are unchanged; README and `llms.txt` document the two tools, the
  five constants and the metadata-only answer for parts over the budget.
