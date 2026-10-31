# Compact email view and field projections

Date: 2026-10-30. Status: design draft. Epic #164, issue #155 (P1, metric:
median tokens per success). Zero-Codex `claude-careful` lane (decisions 0098,
0099; review residuals rework under accepted 0102).

Decision: 0108 — [Compact email view and field projections](../../docs/decisions/0108-compact-email-view.md), proposed.

## Feature and rationale

Agents poll `swarmail_search_emails` and `swarmail_get_latest_email` far more
often than they read a body, and today every poll returns whole emails:
`text`, `html`, every link, every header. On a modern MCP revision the same
JSON is served twice (text block and `structuredContent`, decision 0082 §2).
That is the token cost this issue attacks.

The change: on the June and November MCP revisions, search and latest answer a
**compact view** by default — a handle, the headers an assertion needs, a
bounded text rendering, links with their anchor text, an attachment list and
the stored codes. Full bodies come on request through `swarmail_get_email`,
whose bytes do not change. A `fields` argument projects any of the four MCP
reads (search, latest, get_email, wait_for_email) to the members an agent
names. REST changes only where the issue asks: the inbox list gains
`view=compact`; every REST response without that key is byte-identical.

Attachments are the one input the stored record does not hold today, so they
are captured at ingest (decision 0005) into one nullable column and never
parsed from `raw` on a read.

## Base and the inherited merge check

This branch is stacked on #153 (PR #282, structured output) and #154 (PR #284,
anchor text and the focused waits), merged at `415a11c`, then main at
`f663c44`. The operator asked that no #153 structured-output assertion be lost
in the dispatcher's resolution of `tests/mcp_revisions/structured.rs`.

Design-time reading of `git diff 240ccf6 415a11c -- tests/mcp_revisions/structured.rs`:
fourteen removed lines. Nine are `pub(super)` visibility widenings of helpers
(`Wire`, `schema_ok`, `valid`, `march_view`, `structural`). The remaining five
are each replaced by an equal or stronger form:

- `legacy_bytes`' whole-transcript equality became `assert_appended` on line 0
  plus byte equality on lines `1..`. `assert_appended`
  (`tests/mcp_revisions/focused.rs:74`) strips the captured line's closing
  `]}}`, requires the served line to start with that exact byte prefix, parses
  the tail as exactly two appended descriptors and checks each against the
  March descriptor shape. That is the full byte prefix intake asked the verify
  seat to confirm, and it is stronger than a thirteen-tool equality a
  fifteen-tool catalogue could never satisfy.
- `names == NAMES` became `names[..13] == NAMES` plus `names[13..] == FOCUSED`.
- `served == projected` and `served == NAMES` became the same equalities over
  the union with the focused set; the `structural` comparison gained the two
  focused expectations stated independently in `focused::expected_output`.

Nothing was lost; nothing is restored. The implementation still proves it by
running the `mcp_revisions` and `focused_waits` suites green before touching
any file (task T0). Work that belongs only to #153 or #154 stays theirs.

## Panel rulings and costs

Both run-local positions were read (`.brokkr/state/design/positions/`).
`.brokkr/state/tasks/` is absent in this run; framing is the task block and
intake's resolved result. The rulings, in the order they matter:

1. **Read-time compact view from stored fields — adopt simplicity, reject
   robustness's materialized `compact_json` column.** Every input of the view
   is already a stored, restore-verbatim field: `text`/`html` (0031 §1),
   `link_metadata` (0094), `codes` (0005), and, after this change,
   attachments. The text rendering reuses `compat::snippet` byte for byte: a
   display-time rendering of a stored body, bounded to 8192 source bytes and
   200 collapsed characters, which accepted 0031 §3 names as *not*
   extraction. Robustness's claim that a read-time render returns the read
   plane to the extraction plane is wrong under that accepted carve-out, and
   its design still needed a render-on-read path for legacy `NULL` rows, so
   it would have carried two renderers, a migration, storage amplification
   and a version tag to buy a property (frozen historical rendering) the
   issue never asks for. Cost of the ruling: a future change to the snippet
   renderer changes the compact view of old mail. That is a view, not a
   stored fact; `get_email` and `raw` stay the truth. The hostile-body bound
   is the existing one (`SNIPPET_SOURCE_PREFIX_BYTES`, decision 0044 §5),
   already proven by a visited-work test in `src/compat.rs`, which the compact
   view inherits by identity.
2. **Attachments at ingest — adopt both.** `Email` carries no attachment
   list and `compat.rs` hard-codes none. Parsing MIME from `raw` on a read is
   the class 0005 forbids, so the list is built in `build_email`'s existing
   parse and committed in the same `record_insert` transaction as the row
   (0001, 0014). `NULL` is a row from before the column: unknown, never
   backfilled (the 0094 §2 semantics). Storage is simplicity's strict tuple
   form, restored through `link_metadata`'s refusing path made generic:
   damage refuses the restore rather than becoming a confident "no
   attachments". Robustness's `size` member is adopted: the decoded part
   length is free from the parser and lets an agent tell the right file
   arrived.
3. **Bounded presentation lists — adopt robustness's exact-total rule, at
   the focused waits' 250.** `anchors` and `attachments` are cut at 250
   entries, and a cut list carries `anchors_total` / `attachments_total`
   with the exact count. Simplicity's "let the 10 MiB bridge refuse" is
   honest but wastes the metric on a newsletter; silent truncation would be
   a lie. The stored fields `links` and `codes` are never cut: they are
   served as `Email` serializes them.
4. **`fields` selects, it does not order — reconcile.** Intake's AC asked
   for requested order. serde_json's map is a `BTreeMap` here
   (`Cargo.toml:46`, no `preserve_order`), and `structuredContent` is a
   `Value` placed inside a `Value` tree, so requested order would need
   either the crate-wide `preserve_order` feature (every reply's bytes
   change) or a parallel serializer for the structured member. Neither is
   worth an agent's ability to reorder keys. Ruling: the keys of a projected
   email are the requested names the mail carries, in the server's canonical
   (alphabetical) order every existing reply already uses; `id` is always
   kept; omission follows `Email`'s existing rules (absent `subject` is
   omitted, not `null`; 0082 §3's "preserve optional omission versus null").
   Cost: a request for a member the mail lacks yields no key, so "exactly the
   requested keys" holds modulo those omissions, and the test states which.
5. **No MCP `view` argument — adopt simplicity.** `fields: ["text"]` already
   returns a page of bodies and naming every member rebuilds the full value;
   `get_email` is the full read. A `view` grammar, a precedence table and a
   REST `view=full` are cut. Cost: an agent wanting a full search page names
   its members.
6. **March stays byte-frozen — adopt simplicity, reject compact on every
   revision.** 0082 §1 (the stack's own proposed contract) keeps March's
   descriptor and result bytes exactly; `legacy_bytes` pins it against a
   captured transcript; the default-version suites (`tests/mcp.rs`,
   `tests/p2.rs`, `tests/stdio.rs`) read bodies from search and latest at
   about two dozen sites. `fields` is admitted only on June/November; March
   answers `unknown argument: fields` as it does for any unpublished key.
   Cost: a client that never negotiates a version gets no compact view.
   `Revision::parse` falls back to November for any unknown supplied
   version, so every negotiating client gets it.
7. **REST: `view=compact` on the list route only — adopt simplicity.**
   The issue's REST criterion names exactly that key. GET
   `/api/v1/messages/{id}` and the thread routes have query domains accepted
   0074 §1 and 0048 §4 close to empty; adding keys there contradicts an
   accepted ruling and waits for operator acceptance (0045 §8), which is
   what blocked the earlier #155 attempt. `fields` on REST is not delivered:
   "`fields` projects any read" is read as the agent surface the metric
   measures. Cost: a REST caller projects nothing beyond compact. Fallback if
   the operator reads it otherwise: `fields` as a comma list on the list and
   await routes, a separate decision.
8. **Schemas: `required: ["id"]`, closed, request-aware wire proof — adopt
   simplicity's shape with robustness's proof.** Once `fields` exists, no
   member but `id` is always emitted, so a stronger static `required` would
   be false. One closed `projected_email` object (every member typed) is the
   item or root schema of the four tools; the `{}` vacuity control of #153
   survives because `id` stays required. The lost static claim is carried by
   a wire test that computes the exact key set per request. Cost: named in
   the plan's risks.
9. **Token metric — report it unmeasured.** No paid trial. The story reports
   the literal "not measured: no paid trial" and a bytes proxy from the pins,
   labelled bytes. The harness's own MCP client (`evals/src/mcp.rs`) pins
   `2025-03-26` and serves the oracle, not the agents under trial, which
   negotiate their own modern revisions; it stays as it is.

## Contract

### The compact view

A compact email is a JSON object with these members and no others. "Absent"
means the key is omitted, never `null`, unless the row says otherwise.

| Member | Value | Absent when |
|---|---|---|
| `id` | the handle: pass it to `swarmail_get_email` | never |
| `from` | the stored `EmailAddress` | unknown, as `Email` |
| `to` | the stored list | never (may be `[]`) |
| `subject` | stored | absent, as `Email` |
| `received_ms` | stored; pairs with the `since_ms` filter | never |
| `snippet` | `compat::snippet(email)` byte for byte: the text part, else the HTML rendered through `html_to_text`, from at most 8192 source bytes, whitespace runs collapsed, at most 200 characters, `...` appended when the output or the source was cut | never (may be `""`) |
| `codes` | the stored `codes`, uncapped | empty, as `Email` |
| `anchors` | `[{"url","anchor_text"}]` from `link_metadata`, ingest order, at most 250 | `link_metadata` is `None` (row before 0094) or `Some([])` |
| `anchors_total` | exact occurrence count | the list was not cut |
| `links` | the stored `links`, uncapped, as `Email` serializes them | `link_metadata` is `Some` (fresh rows) or the list is empty |
| `attachments` | `[{"filename","content_type","size"}]`, parser order, at most 250; `filename` and `content_type` nullable; `null` as a whole for a row from before the column | the list is known empty |
| `attachments_total` | exact count | the list was not cut |

`N` is 200 collapsed characters, the existing `SNIPPET_CHARS`; it is named in
decision 0108 and a change to it is an amendment, not an edit. `inbox`, `cc`,
`recipients`, `envelope_from`, `received_at`, `size`, `text`, `html`,
`message_id`, `in_reply_to`, `references` are reachable through `fields`.

### `fields`

- An argument of `swarmail_search_emails`, `swarmail_get_latest_email`,
  `swarmail_get_email` and `swarmail_wait_for_email` on June and November.
- An array of names from one table, published as the argument's `enum`:
  the 18 serialized `Email` members (`id`, `inbox`, `from`, `envelope_from`,
  `to`, `cc`, `recipients`, `subject`, `received_at`, `received_ms`, `size`,
  `text`, `html`, `links`, `codes`, `message_id`, `in_reply_to`,
  `references`) plus `snippet`, `anchors`, `attachments`. `raw` is not a
  name. A `_total` member rides with its list and is not named separately.
- Each name has exactly one shape wherever it appears.
- The projected email is the full `Email` value merged with the three
  derived members, retained to the requested names plus `id`. Omission
  follows the full view (an absent member stays absent). `fields: []` is
  ids only. Duplicates are harmless.
- Refusals, `-32602`, before any store read and before a wait parks:
  a non-array or a non-string element is `fields must be an array of
  strings`; an unknown name is `fields must be an array of known field names
  (unknown: <name>)`, naming the first unknown spelling.
- Not on the focused waits, `extract_*`, seed or the report. Not on March:
  `unknown argument: fields`.
- On `swarmail_wait_for_email`, the projection applies to the `emails` of the
  found arm; the timeout arm is unchanged.

### REST

`GET /api/v1/inboxes/{inbox}/messages?view=compact` answers the existing
`EmailList` envelope (`total`, `emails`) with compact items. `view` admits the
single value `compact`; any other value is the existing 400 naming `view`.
Without the key the handler path and the bytes are unchanged. No other route
gains a key.

### Attachments at ingest

- `Email.attachments: Option<Vec<Attachment>>`, `#[serde(skip)]`, so every
  existing serialization, feed and webhook payload keeps its bytes.
  `Attachment { filename: Option<String>, content_type: Option<String>, size: u64 }`.
- Built in `build_email` from mail-parser's outer `attachments()` iterator, in
  parser order: `filename` from `attachment_name()`, `content_type` as
  `type/subtype` of the part's Content-Type (absent when the header is), `size`
  the decoded part length. An attached message counts once; its inner parts
  are not listed. No payload is decoded beyond what the parser already did.
  A mail the parser cannot parse stores `None`; a parsed mail without
  attachments stores `Some([])`.
- One nullable column `attachments_json TEXT` in `ADDED_COLUMNS` and
  `COLUMNS`, written in the same insert transaction, deleted with the row by
  the existing prune delete. Stored as `[[filename|null, type|null, size], ...]`
  and restored through a strict reader: SQL `NULL` is unknown history; any
  other value must parse exactly or the restore refuses.

### Revisions, schemas and documents

- The `TOOLS` literal is March's pinned catalogue and does not change. The
  modern projection (`tool_descriptors`) replaces the search and latest
  descriptions with the compact wording, adds the `fields` property to the
  four tools' input schemas, and publishes one closed `projected_email`
  output object (`required: ["id"]`) as the item or root schema of the four
  tools. The wait keeps its `oneOf` arms; wrappers keep their members. The
  catalogue stays at 15.
- `unknown_argument` consults the revision's projection, so `fields` is
  known on June/November and unknown on March.
- `openapi.json` documents `view` on the list route and the compact item
  shape; README and `llms.txt` describe the compact default, `fields` and
  `view=compact`. The decision index gains the 0108 row.

## Frozen guarantees

- **Accept == stored (0001):** the attachment list is built inside the
  existing offloaded parse and committed in `record_insert`'s transaction;
  `insert` gains no await and no second write.
- **`await` subscribes before it checks (0002):** the wait engine is
  untouched; `fields` is refused before `park_await` and applied to the
  final snapshot.
- **Oldest-only pruning (0003):** untouched; the new column lives on the
  pruned row.
- **Extraction at ingest (0005, as amended by 0031 §3):** attachments are
  captured at ingest; links, anchors and codes are read from stored fields;
  the snippet is a bounded display rendering that derives nothing and writes
  nothing back.
- No dependency is added; the coverage floor stays 100% exact; no
  `coverage(off)`.

## Token metric

Median tokens per success is not measured in this delivery: no paid trial.
The story (`implementation.md`) reports the literal "not measured: no paid
trial" and a byte table from the pinned fixtures — March full item versus
November compact item, and the modern double-serialized reply — labelled as
bytes, never as tokens. Two reasons it may not move even when measured: the
eval prompts steer agents to `swarmail_wait_for_email`, which stays full by
default, and the four modern descriptors grow by the `fields` enum, a cost
paid once per discovery.

## Acceptance Criteria

1. **Compact byte pins.** Inline tests in `src/api.rs` pin the exact JSON of
   the compact view for: a text-only mail; an html-only mail; a multipart
   mail with text, html, two anchors and an attachment; a seeded mail whose
   collapsed text is exactly 200 scalars (no suffix) and 201 scalars (200 plus
   `...`), each containing 3-byte and 4-byte UTF-8 scalars at the cut; a
   historical row (`link_metadata: None`, `attachments: None`) serving
   `links` and `attachments: null`; an anchors list of 250 (no total) and 251
   (250 entries plus `anchors_total: 251`); the same for attachments. The
   snippet member equals `compat::snippet` for every fixture.
2. **MCP default and projection.** Over real HTTP and stdio on June and
   November, `swarmail_search_emails` and `swarmail_get_latest_email` return
   compact items in both the text block and `structuredContent`;
   `swarmail_get_email` without `fields` is byte-identical to the current
   modern baseline; `fields` on each of the four tools yields exactly the
   requested present names plus `id`, with a control that `text` and `html`
   never appear unless named; `fields: []` yields ids only; an unknown name
   refuses `-32602` naming it, before lookup (a missing id with a bad
   `fields` answers the refusal, not not-found) and before a wait parks (the
   await gauge does not move); a wrong type refuses by the stated sentence;
   March answers `unknown argument: fields` and `legacy_bytes` passes
   unmodified.
3. **REST unchanged unless asked.** Every existing REST pin passes
   unmodified; `view=compact` on the list route returns the `EmailList`
   envelope with the compact item bytes of criterion 1; `view=full` and an
   empty `view=` refuse 400 naming `view`; the served OpenAPI lists `view`
   on exactly the list route and the served-parameters tests hold.
4. **Attachments stored, restored, refused.** A real SMTP delivery with a
   named attachment is queryable with its list immediately after `250`; the
   same bytes are served after a store reopen; a file written before the
   column migrates and restores `None`, serving `attachments: null`; a
   corrupt column refuses the restore by the stated error; a pruned mail
   leaves no row.
5. **Schemas.** Every served modern output validates against its
   `outputSchema` in `tests/mcp_revisions/structured.rs` for every fixture of
   criterion 2, the `{}` vacuity control still fails, the catalogue is 15 and
   the OpenAPI pins keep deriving from the `Email` component with named
   exceptions for the four tools.
6. **Inherited merge check.** T0's suites pass on the untouched base and the
   story records the `assert_appended` reading above.
7. **Metric honesty.** The story carries the literal "not measured: no paid
   trial" and the byte table labelled as bytes.
8. **Gates.** Every gate in `docs/house-rules.md` § Gates clean with
   `--locked`, including `cargo test --manifest-path evals/Cargo.toml --locked`,
   the coverage gate at 100% exact with its selftest, the publish-guard and
   seal proofs and `cargo deny check licenses`. Commits unsigned; nothing
   pushed, merged, published or accepted.
