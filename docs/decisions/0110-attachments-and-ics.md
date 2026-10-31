# 0110 — Attachments listed and fetched from stored bytes, ICS fields at ingest

- **Status:** proposed — awaiting the operator
- **Date:** 2026-10-31
- **Scope:** Epic #164, issue #159, on main `d892acb` (the merged MCP work of
  #152, #153 and #154).
- **Specification:** [Attachments and ICS](../../specs/attachments-and-ics/spec.md);
  [plan](../../specs/attachments-and-ics/plan.md),
  [tasks](../../specs/attachments-and-ics/tasks.md).
- **Supplements:** the modern tool inventories of proposed
  [0082](0082-mcp-structured-output.md), [0105](0105-mcp-report-output.md)
  and [0094](0094-focused-mail-waits.md) (two appended read-only tools);
  extends ingest-owned extraction under accepted
  [0005](0005-extraction-at-ingest.md) compatibly; states a new bounded
  memory exposure beside accepted [0038](0038-store-memory-and-timestamps.md).
- **Authority relied on:** accepted [0001](0001-synchronous-insert-lossless.md)
  (the attachment work runs inside the parse that precedes the commit),
  accepted [0022](0022-bounded-data-raw-bytes.md) (DATA is bounded at 50 MiB
  before the parser sees it; `/raw` serves the stored bytes), accepted
  [0031 §1](0031-sender-honest-stored-bodies.md) (the record is sender-honest),
  accepted [0040](0040-extraction-fidelity.md) (attachment parts never become
  the body), accepted [0045 §8](0045-delivery-repairs.md) (compatible work may
  land while this record is proposed).
- **Preserves:** 0001–0005, 0010, 0014, 0020, 0030, 0056, the 0082 §1 March
  byte freeze, every existing Email serialization and every REST response.
- **Number:** 0106 and 0107 are held by open PR #292, 0108 by the unmerged
  #155 branch and 0109 by `origin/main`; 0110 is the next number no checked
  tip holds. Recheck at integration; only the operator renumbers.

## Proposed ruling

1. **Attachments are extracted at ingest, inside `build_email`, before the
   insert.** For every candidate part of the outer message — the parser's
   attachment list plus any bare calendar part (`text/calendar`,
   `application/ics`, or a filename ending `.ics`) in part order — ingest
   records the filename, the ASCII-lowercased `type/subtype`, the decoded
   size in octets, the lowercase-hex SHA-256 of the decoded bytes, whether
   the bytes are stored, and for a calendar part the ICS fields of §4. An
   attached message is one entry; its inner parts are not walked. The walk,
   the hashes and the ICS scan are total: nothing in them can fail the
   mail, so an SMTP `250` still means stored (0001).

2. **Fetch reads stored bytes; nothing re-parses `raw` at query time.** The
   decoded bytes of listed parts are kept, first-fit in list order, under
   one per-message budget `MAX_ATTACHMENT_STORED_BYTES` = 1 MiB (1 048 576
   octets): a part whose size is at most the remaining budget is stored and
   charged; a larger one is listed with its true size and hash and
   `stored: false`, and the budget stays for later, smaller parts. Fetching
   an unstored part refuses by name, naming the size, the budget and the
   REST `/raw` route that still serves the whole mail (0022). The bytes are
   never truncated: a partial body would not match the listed hash. This is
   0005 read literally — list, fetch and ICS fields are served from stored
   fields only — and it keeps the served bytes equal to the hashed bytes
   across parser upgrades.

3. **The caps are pinned constants, never configuration.**

   | Constant | Value | Bounds |
   |---|---|---|
   | `MAX_ATTACHMENTS` | 250 | entries listed and considered for storage per mail; the exact candidate total is still recorded |
   | `MAX_ATTACHMENT_STORED_BYTES` | 1 MiB | decoded bytes retained per mail, summed over stored parts; also the largest fetchable part |
   | `MAX_LABEL_BYTES` | 1024 | every stored string: filename, content type, each ICS field |
   | `MAX_ICS_BYTES` | 1 MiB | the largest calendar part the ICS scanner reads; a larger one yields no fields |

   1 MiB is chosen from the stdio bridge's unchanged 10 MiB response cap
   (0029 §4): a modern result carries the JSON twice (text block and
   `structuredContent`), and a worst-case UTF-8 text payload escapes to four
   times its size inside the text block, so one fetch answers in at most
   about 6 MiB and never trips the bridge. A larger budget would make fetch
   refuse on the bridge for exactly the payloads it stored. A knob would
   double the branch matrix under the exact gate and make the pinned value
   unobservable.

4. **ICS fields are read from the first `VEVENT` only, literally.** After
   RFC 5545 unfolding (CRLF to LF, then `LF SP` and `LF HT` removed), the
   scanner reads properties that sit directly inside the first
   `BEGIN:VEVENT … END:VEVENT`, tracking nesting so a `VTIMEZONE`'s
   `DTSTART` or a `VALARM`'s `SUMMARY` is never reported as the event's.
   `DTSTART`, `DTEND` and `UID` are stored verbatim with the property's
   `TZID` parameter beside the two times; `SUMMARY` is RFC 5545 text
   unescaped. No timezone resolution, no instant conversion, no `RRULE`,
   attendee or organizer fields: a wrong offset is worse than the
   calendar's own text, and no timezone database is in the tree. A calendar
   part with no `VEVENT`, invalid UTF-8 or more than `MAX_ICS_BYTES` yields
   `ics: null`; a malformed one never panics and never refuses the mail.
   `UID` is the one field beyond the issue's three: the invite task reads
   it, and it is the event's identity.

5. **SHA-256 comes from the ring provider rustls already links**, through
   rustls's public cipher-suite hash provider, with known-answer tests
   (NIST `abc` and the empty string) proving it is SHA-256. No crate enters
   the tree and `Cargo.lock` is unchanged. A hand-rolled digest is refused
   as a security regression; `sha2` would be a new dependency and is not
   added without the operator. The operator may separately rule that `ring`
   (already a direct dependency of the evals crate at the locked version)
   becomes a direct dependency of the root crate; that cleanup changes the
   manifest only, adds no crate, and is not needed for this change.

6. **Persistence is two nullable columns in the insert's transaction.**
   `attachments_json` holds a closed object `{total, parts}` and
   `attachment_bytes` the concatenation of the stored parts' bytes in list
   order; both ride `record_insert` with the mail row, the prune deletes and
   the counters, and both are added through `ADDED_COLUMNS` so a data file
   written by an older release opens. SQL `NULL` in both is unknown — a row
   from before the columns, or a mail the parser could not read — served as
   a named refusal and never backfilled. Any other value restores strictly
   or refuses the restore: a damaged object, an unknown or missing member,
   a `NULL` in one column but not the other, or stored sizes that do not sum
   to the blob's length is corruption, never "no attachments".

7. **Two read-only MCP tools, appended.** `swarmail_list_attachments {id}`
   answers `{id, total, truncated, attachments}`; `swarmail_get_attachment
   {id, index}` answers `{index, filename, content_type, size, sha256,
   encoding, content}` with `encoding` `utf-8` for text that is valid UTF-8
   with no control bytes beyond TAB, LF and CR, else `base64`. Both carry a
   closed `outputSchema` on the June and November revisions and
   `readOnlyHint: true`; March keeps every earlier byte and gains the two
   descriptors text-only (0082 §1, 0094 §7). Malformed arguments refuse
   `-32602` by name before any store read; an unknown mail, an unknown
   attachment list, an index past the list and an unstored part are
   `isError` text results. No REST route, no compat field, no webhook
   change: `Email.attachments` and the bytes are `#[serde(skip)]`, so every
   existing read, feed, webhook and March capture keeps its bytes.

8. **Memory exposure, stated.** Each stored mail may retain up to 1 MiB of
   decoded attachment bytes beside its raw bytes, in memory and in the data
   file. 0038's rule stands: there is no global ceiling; the per-inbox cap
   bounds count, not bytes. The transient decode is unchanged: mail-parser
   decodes every part while parsing, bounded by the 50 MiB DATA cap (0022);
   this change bounds what is retained, not what the existing parse
   allocates for the duration of one ingest. *Implementation note:* the
   recorded entries are retained too — at most 250 of them, each with a
   filename, a type and (for a calendar part) six ICS fields of at most
   `MAX_LABEL_BYTES` plus a 64-character digest, so at most 8 256 bytes
   each and about 2 MiB of labels per mail beside the 1 MiB of bytes.

9. **The metric.** The invite and ICS eval tasks run offline only — the
   evals crate's fixture and oracle tests, extended with a rig test that
   lists and fetches the real invite through the two tools. The live agent
   metric is reported as **not measured: no paid trial**, and the report
   states that the catalogue's `invite-time` task does not exercise the MCP
   attachment tools (0055 §4); an attachment-only task needs its own id and
   baseline label and is not added here.

## Why

Agents asked for the invoice, the calendar invite or the report the mail
carries get, today, only a raw MIME download to decode themselves. Listing
type, size and hash lets an agent decide before it pays for bytes; fetching
from stored bytes answers in one call with the bytes the hash describes;
ICS fields at ingest answer the invite question the way links and codes are
answered — once, identically on every surface. The budget keeps the whole
feature inside the bridge cap and inside a stated memory bound.

## Alternatives and consequences

- *Re-decode `raw` at fetch under the report's job permits.* Rejected. It
  reopens 0005 for a convenience read, needs permit plumbing and a busy
  refusal, lets a parser upgrade move a part or change its bytes under a
  persisted hash, and its cold arms (permit exhaustion, hash mismatch) are
  reachable only through seams. Cost of the ruling: up to 1 MiB more per
  mail in memory and on disk, and no fetch for parts over the budget.
- *Store up to 10 MiB per mail in a child table.* Rejected. 10 MiB cannot
  be answered through the 10 MiB bridge once doubled and escaped, and a
  child table adds a second delete site to every prune, delete and clear.
  Cost of the ruling: large attachments are metadata-only through MCP.
- *`sha2` as a new dependency.* Not refused; not needed. It would block the
  change on a ruling the tree already answers. Cost: one unusual helper
  that reaches SHA-256 through a cipher-suite struct, commented.
- *A per-line ICS cap.* Rejected: the input is bounded by `MAX_ICS_BYTES`
  and every stored field by `MAX_LABEL_BYTES`; a third constant bounds
  nothing further.
- *REST list and fetch routes.* Deferred. The epic measures the agent
  surface; `/raw` already serves the bytes; a REST contract is a separate
  decision with its OpenAPI, refusal and `tests/api.rs` obligations.
- *Rendering HTML attachments to text in fetch.* Out of scope. The
  renderer exists once (`compat`'s `html_to_text`, 0031 §3) and #155 owns
  its sharing; fetch returns the part's own bytes.
- *Timezone resolution for `DTSTART`.* Rejected; see §4.

## Reconciliation with the unmerged #155 (proposed 0108 §5)

#155 adds `model::Attachment { filename, content_type, size }`,
`Email.attachments`, `attachments_json` as an array of three-tuples and
`attachment_list` in `smtp.rs`. This record keeps those names and widens the
entry with `sha256`, `stored` and `ics`, and the candidate set with bare
calendar parts. Whichever lands second reconciles the stored shape: a
strict reader must accept every shape ingest ever wrote and nothing else,
and 0108 §5's "outer parser list" wording gains the calendar arm. Neither
change is authority over the other; the operator rules on both.
