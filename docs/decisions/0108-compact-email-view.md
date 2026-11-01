# 0108 — Compact email view and field projections

- **Status:** proposed — awaiting the operator
- **Date:** 2026-10-30
- **Scope:** Epic #164, issue #155, on #153 (PR #282) and #154 (PR #284), both now on main.
- **Specification:** [Compact email view](../../specs/compact-email-view/spec.md);
  [plan](../../specs/compact-email-view/plan.md), [tasks](../../specs/compact-email-view/tasks.md).
- **Supplements:** proposed [0082](0082-mcp-structured-output.md),
  [0105](0105-mcp-report-output.md) and [0094](0094-focused-mail-waits.md)
  (the modern tool inventory and its output schemas); extends ingest-owned
  extraction under accepted [0005](0005-extraction-at-ingest.md) compatibly.
- **Authority relied on:** accepted [0031 §3](0031-sender-honest-stored-bodies.md)
  (display-time rendering is not extraction), accepted
  [0044 §5](0044-compat-validation.md) (the snippet's bounded prefix),
  accepted [0074 §1](0074-strict-native-queries.md) and
  [0048 §4](0048-rest-webhook-validation.md) (empty native query domains).
- **Preserves:** 0001–0005, 0010, 0014, 0020, 0030, 0056, the 0082 §1 March
  byte freeze and every existing REST response without `view`.
- **Number:** drafted as 0092, renumbered to 0108 at the merge of main: main's 0092 is
  official-mail-comparison, and open PR #292 holds 0106 and 0107.

## Proposed ruling

1. **Compact by default on modern MCP reads.** On the June (2025-06-18) and
   November (2025-11-25) revisions, `swarmail_search_emails` and
   `swarmail_get_latest_email` answer compact items: `id`, `from`, `to`,
   `subject`, `received_ms`, `snippet`, `codes`, `anchors` (or `links` for a
   row stored before 0094), `attachments`, and the exact `_total` of a cut
   list. `swarmail_get_email` keeps its full bytes. March, including the
   missing-version default, keeps every descriptor and result byte (0082 §1).
2. **`N` is 200.** The snippet is `compat::snippet` byte for byte: at most
   8192 source bytes, whitespace collapsed, at most 200 characters, `...` when
   cut. Changing the length or the prefix bound amends this decision.
3. **`fields` selects members.** On the four modern reads (search, latest,
   get_email, wait_for_email) an array of names from the published 21-name
   table retains those members plus `id`; keys stay in the server's canonical
   order; omission follows the full view; an unknown name or a wrong type
   refuses `-32602` by name before any store read and before a wait parks.
   March answers `unknown argument: fields`.
4. **REST gains `view=compact` on the inbox list and nothing else.** The
   single-message, thread and feed query domains stay empty (0074 §1, 0048 §4).
5. **Attachments are captured at ingest.** `build_email` records filename,
   content type and decoded size for every part of the outer parser list, in
   parser order, into `Email.attachments` and the nullable column
   `attachments_json`, committed in the insert's transaction. SQL `NULL` is
   unknown, served as `attachments: null`, never backfilled: a row from before
   the column, or a fresh mail the parser could not read (`build_email` stores
   `None`). The two are not told apart on the wire. `content_type` is
   `type/subtype`, or the bare type when the header has no subtype. Any other stored value restores strictly or refuses the
   restore. No read parses `raw`.
6. **Presentation lists are bounded with exact totals.** `anchors` and
   `attachments` show at most 250 entries; a cut list carries its exact
   count. The stored `links` and `codes` are served uncapped as today.
7. **Schemas.** One closed `projected_email` object with `required: ["id"]`
   is the item or root output schema of the four reads; a request-aware wire
   test proves the exact key set per request.
8. **Metric reporting.** A delivery that cannot run the paid trial reports
   "not measured: no paid trial" and may cite byte sizes from its pins,
   labelled as bytes.

## Why

Polling is the agent's hot path and bodies are its cost; a compact default
with a projection moves the tokens-per-success metric without a second
renderer, a second storage format or a changed REST surface. The stored
record already holds every input but the attachment list, and 0031 §3 already
rules that rendering a stored body for display is not extraction.

## Alternatives and consequences

- *Materialize a versioned compact column at insert.* Rejected: it buys a
  frozen historical rendering the issue does not ask for, at the cost of a
  migration, storage amplification and a second render path for `NULL` rows.
- *Compact on March too.* Rejected: contradicts 0082 §1 and the captured
  transcript; clients that never negotiate keep what they have.
- *`fields` on REST and `view` on the single-message route.* Deferred: the
  empty domains are accepted rulings; a separate decision can open them.
- *Requested key order.* Rejected: needs crate-wide `preserve_order` or a
  parallel serializer; the agent gains nothing.

## Dated addendum — 2026-11-01: what #159 changed

*A dated addendum, not a rewrite: the text above is unchanged and this record
is still proposed. It states only what the code on `main` does after #159
(PR #301, squash `19df8ad`; proposed [0110](0110-attachments-and-ics.md)).
Line numbers are those of `main` at `a4f552d`.*

**Unchanged.**

- The compact item's members (§1), and each attachment entry's three members
  `content_type`, `filename` and `size`: `AttachmentEntry`
  (`src/api.rs:1127–1133`) and the `projected_email` attachment item schema
  (`src/mcp.rs:1016–1037`). The fields #159 records beside them — `sha256`,
  `stored`, `ics` — reach neither the compact view nor a projection; only
  0110's two new tools serve them.
- The 21-name `fields` table (§3, `src/api.rs:1226–1248`), the projection
  (`src/api.rs:1278–1300`) and the four projecting reads (`PROJECTING_READS`,
  `src/mcp.rs:501–506`). The two new tools take no `fields`.
- The 250-entry presentation cap with an exact `attachments_total` (§6,
  `src/api.rs:1118`), `null` for an unknown list and omission of a known-empty
  one (`src/api.rs:1135–1139`).
- REST changes only by `view=compact` (§4). `Email.attachments` is still
  `#[serde(skip)]` (`src/model.rs:333–338`), so no full Email serialization
  carries it, and no read parses `raw`.

**No longer exactly true in §5.**

1. *Which parts are listed.* §5 says "every part of the outer parser list, in
   parser order". Ingest now lists every leaf part of the outer message that
   is in the parser's attachment list **or** is calendar-shaped
   (`text/calendar`, `application/ics`, or a filename ending `.ics`), never a
   multipart container, in part order (`src/attachments.rs:86–94`,
   `103–145`). A bare `text/calendar` alternative, which the parser does not
   call an attachment, now appears in the compact `attachments`
   (`src/api.rs:1386`). For the parts the parser lists, nothing moved:
   `attachments_are_listed_at_ingest_in_parser_order` (`src/smtp.rs:2040`)
   still asserts the same six entries, sizes included.
2. *What is recorded.* `Email.attachments` is now `Option<AttachmentList>`
   (`src/model.rs:183–193`, `338`): the exact candidate `total` and at most
   the first 250 parts (`MAX_ATTACHMENTS`), each carrying `sha256`, `stored`,
   `ics` and any retained bytes beside the three labels
   (`src/model.rs:152–162`). Ingest calls `crate::attachments::extract`
   (`src/smtp.rs:1945`); `attachment_list` is gone.
3. *Labels.* `filename` and `content_type` are cut to 1024 bytes
   (`MAX_LABEL_BYTES`) at a character boundary, and ingest ASCII-lowercases
   `content_type` itself (`src/attachments.rs:66–84`). The locked
   mail-parser 0.11.9 already lowercases it, so that moves no byte (0110,
   reconciliation); the cut touches only labels over 1 KiB.
4. *`attachments_total`.* It is the recorded `total` when that exceeds 250
   (`src/api.rs:1195–1197`), not the length of the held list, because ingest
   keeps at most 250 parts. On the wire it is the same exact count.
5. *Storage.* `attachments_json` no longer holds the tuple array this
   record's code wrote. Ingest writes the closed object `{total, parts}` and a
   second nullable column, `attachment_bytes` (the stored parts' bytes,
   concatenated), both in the insert's transaction (`src/persist.rs:120–134`,
   `499–503`, `628–637`). Unknown is SQL `NULL` in *both* columns, served as
   `attachments: null` and never backfilled.
6. *Restore.* "Any other stored value restores strictly or refuses the
   restore" now admits exactly three shapes (`src/persist.rs:696–763`): both
   `NULL`; a tuple array with `attachment_bytes` `NULL` — a row written under
   §5 before #159 — restored labels-only (`src/persist.rs:670–694`), whose
   compact view serves its labels as before (`[]` known empty, omitted) while
   both attachment tools answer unknown (`src/mcp/attachments.rs:39`); and the
   closed object beside a blob as long as its stored parts' sizes. Anything
   else refuses the restore by its column (`tests/persistence.rs:1994–2040`,
   `2094–2170`).

Where the code differs from the text above, it follows proposed 0110 (§§1, 6
and its reconciliation with #155). This addendum records facts; amending this
record's ruling is the operator's.
