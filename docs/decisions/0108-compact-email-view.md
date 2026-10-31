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
