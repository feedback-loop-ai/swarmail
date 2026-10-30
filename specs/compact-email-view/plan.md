# Implementation plan — compact email view and field projections

Implements [spec.md](spec.md) under proposed
[0092](../../docs/decisions/0092-compact-email-view.md). Compatible work under
accepted 0045 §8: every accepted ruling is preserved; nothing here waits on
acceptance.

## Technical context

Single Rust crate on the existing Tokio/Axum/serde_json/rusqlite/mail-parser
stack; real SMTP, HTTP, stdio and SQLite in the tests. No new dependency, no
new production file, no feature flag, no seam. `.specify/templates/` is
absent; this plan supplies the context, constitution check, structure,
research, data model, contracts, quickstart and risks directly. The chief
commits spec.md, plan.md, tasks.md and the decision draft only.

## Constitution check

- 0001: the attachment list is derived inside `build_email` (already run on
  the blocking pool before `insert`) and written in `Persist::record_insert`'s
  transaction (`src/persist.rs:455-500`); `Store::insert`
  (`src/store.rs:539`) gains no await and no second write.
- 0002: the wait engine and `park_await` are untouched; `fields` is staged
  with the other arguments before the park.
- 0003: the prune delete already removes the whole row.
- 0005 as amended by accepted 0031 §3: the only read-time work is the
  existing bounded `snippet` and a copy of stored fields.
- 0020 placement: inline `#[cfg(test)]` modules for the builders, parsers and
  readers; `tests/*.rs` for wire shapes; no new harness file. No sleep as
  synchronization, no wall-clock assertions, fresh `Store::new` or per-test
  inboxes.
- 0044 §3 / 0074 §1 / 0048 §4: every argument and query shape stays closed;
  only the list query gains a key.
- 0082 §1: March bytes and descriptors unchanged; 0082 §4: schema literals
  live beside the catalogue and attach in the modern projection only.
- 0094 §2 semantics reused for the new nullable column.
- 0056: commit unsigned, never push.

## Research and chosen approach

**Snippet reuse.** `compat::snippet` (`src/compat.rs:1129`) is private; it
becomes `pub(crate)`. Its bound (`SNIPPET_SOURCE_PREFIX_BYTES`, 8192) and
length (`SNIPPET_CHARS`, 200) are the view's `N`; the existing visited-work
test (`src/compat.rs:1432-1460`) proves the bound, and the new pins assert
`compact.snippet == snippet(email)` so the proof transfers by identity.

**Where the view lives.** `src/api.rs` already owns the shared list shape
(`EmailList`, `DEFAULT_LIST_LIMIT`, `checked_page_size`) that `src/mcp.rs`
calls into. A borrowed `pub(crate) struct CompactEmail<'a>` with
`#[derive(Serialize)]` and `skip_serializing_if` on the optional members,
built by `CompactEmail::from(&Email)`, serializes without hand-written
branches. The two capped lists are cut with `take(250)` and their `_total`
members are `Option<usize>` set only when the source is longer.

**`fields` as a retained merge.** `serde_json::to_value(email)` is already
paid by every modern read; merging the three derived members and retaining
the requested names plus `id` is a handful of lines and needs no per-member
match. Order is serde_json's canonical map order (spec ruling 4). The name
table is one `const FIELD_NAMES: [&str; 21]` that both the published `enum`
and the refusal consult — one registry.

**Revision gating without a per-call clone.** `tool_descriptors(revision)`
clones the catalogue on every discovery; the modern projection is identical
for June and November, so a second `LazyLock` (`MODERN_TOOLS`) caches it
once and `unknown_argument(tool, args, revision)` reads the matching
catalogue. The March literal `TOOLS` is untouched.

**Attachments from the parser.** mail-parser 0.11's `Message::attachments()`
iterates the outer message's attachment parts in parser order;
`MimeHeaders::attachment_name()` and `content_type()` give the two labels and
`contents().len()` the decoded size (the parser decodes on parse; nothing is
decoded twice). An attached message is one part in that list; its inner parts
are not in the outer list. The classification is the parser's; the view
does not reclassify.

**Strict column reader.** `persist::link_metadata` (`src/persist.rs:596`)
becomes a generic `strict_column<T: DeserializeOwned>(index, raw)`; attachments
restore through it as `Vec<(Option<String>, Option<String>, u64)>` tuples,
which serde already refuses on wrong length, wrong type or a top-level
`null`. The map-only reader of 0097 §2 is not duplicated.

**REST.** `ListQuery` gains `view: Option<View>` with
`enum View { #[serde(rename = "compact")] Compact }`; serde_urlencoded's
unknown-variant error flows through the existing `invalid list query` 400
and names `view`. The handler branches once on `q.view`: the `None` path is
the existing code and the existing bytes.

**Schemas.** One `projected_email` closed object with every compact and
`Email` member typed, `required: ["id"]`, replaces `email` as the item or
root schema of the four tools in `OUTPUT_SCHEMAS`. `output_object` already
sets `additionalProperties: false`. The two existing pin tests gain named
exceptions for exactly those four names, as they already do for the wait
and the report.

## Data model

- `model::Attachment { filename: Option<String>, content_type: Option<String>, size: u64 }`
  (`Serialize`, `Clone`, `PartialEq`, `Debug`).
- `Email.attachments: Option<Vec<Attachment>>`, `#[serde(skip)]`; `None` is
  unknown history, `Some([])` known empty. Every construction site gains the
  member: `src/smtp.rs` (`build_email`), `src/persist.rs` (`row_to_email`),
  `src/model.rs`, `src/store.rs`, `src/compat.rs`, `src/webhook.rs` (3),
  `tests/mcp_revisions/structured.rs`, `tests/mcp_revisions/focused.rs`,
  `tests/webhook.rs`.
- Column `attachments_json TEXT` appended to `COLUMNS` (`src/persist.rs:69`),
  `SCHEMA` and `ADDED_COLUMNS` (`src/persist.rs:116`); the insert gains the
  22nd placeholder and `row_to_email` reads index 20.

## Contracts

- MCP (June/November): `swarmail_search_emails` and
  `swarmail_get_latest_email` items are `CompactEmail`; `fields` on the four
  reads as the spec states; descriptors as the spec states. March: unchanged.
- REST: `GET /api/v1/inboxes/{inbox}/messages?view=compact`; everything else
  unchanged.
- OpenAPI: `view` query parameter (`enum: [compact]`) on the list route; a
  `CompactEmail` component; the `EmailList.emails` items documented as
  `Email` or `CompactEmail`. The `Email` component is unchanged so the MCP
  pins keep deriving from it.
- Documents: README's MCP section and `llms.txt` describe the compact
  default, `fields` and `view=compact`; `docs/decisions/README.md` gains the
  0092 row.

## Project structure (files touched)

| File | Change |
|---|---|
| `src/model.rs` | `Attachment`; `Email.attachments` |
| `src/smtp.rs` | `build_email` builds the list from the outer parser list |
| `src/persist.rs` | column, insert placeholder, strict generic reader, migration entry |
| `src/compat.rs` | `snippet` → `pub(crate)`; test fixture gains the member |
| `src/api.rs` | `CompactEmail`, `View`, `ListQuery.view`, `list_messages` branch, OpenAPI tests |
| `src/mcp.rs` | compact default, `fields` staging and projection, `MODERN_TOOLS`, revision-aware `unknown_argument`, modern descriptions, `projected_email` schema |
| `src/static/openapi.json` | `view` parameter, `CompactEmail` component |
| `src/store.rs`, `src/webhook.rs` | construction sites |
| `tests/mcp_revisions/structured.rs`, `tests/mcp_revisions/focused.rs` | modern pins, projection proofs, schema validation, pin exceptions |
| `tests/api.rs`, `tests/persistence.rs`, `tests/smtp_edges.rs` (or `tests/p2.rs`) | REST `view=compact`, attachment round trip and SMTP proof |
| `README.md`, `llms.txt`, `docs/decisions/README.md` | documents and the index row |
| `specs/compact-email-view/implementation.md` | the story: T0 reading, byte table, metric line |

## Implementation sequence

1. T0: run the inherited suites on the untouched base; record exits and the
   `assert_appended` reading.
2. Attachments: model, ingest, column, reader, construction sites; unit and
   persistence tests; SMTP wire proof.
3. `CompactEmail` with its byte pins; `snippet` visibility.
4. MCP: `MODERN_TOOLS`, `fields` staging (before lookup and park), default
   compact on search/latest, projection on the four reads, descriptors and
   schemas; wire proofs on HTTP and stdio for June and November; March
   unchanged.
5. REST `view=compact`, OpenAPI, document updates, served-parameter tests.
6. The story with the byte table and the metric line; the decision index row.
7. Every gate, locked, including the evals crate; commit unsigned.

## Quickstart

```bash
cargo test --locked --test mcp_revisions --test focused_waits          # T0, before any edit
cargo test --locked compact_                                           # inline pins
cargo test --locked --test mcp_revisions modern_                       # modern wire proofs
cargo test --locked --test api view_compact                            # REST
bash scripts/coverage-gate.sh && bash scripts/coverage-gate.sh selftest
```

## Risks

- **Modern search/latest bytes change by default.** The issue demands it;
  every modern-revision test that read `text`/`html` from a search or latest
  item moves to `fields` or `get_email`. Mitigation: T0 inventories those
  sites (`tests/mcp_revisions/*.rs` only; the default-version suites are
  March and untouched) before the change.
- **Schemas weaken to `required: ["id"]` on four tools.** A static claim #153
  made is no longer true once members are selectable. Mitigation: the
  request-aware wire test asserts the exact key set per request and the `{}`
  control stays.
- **`fields` serializes the full mail, then discards.** CPU equals what every
  modern read pays today; the default compact path never serializes a body.
  Mitigation: none needed for the metric; a selective builder can replace the
  merge later without a contract change.
- **A snippet renderer change alters old mail's compact view.** Accepted as
  a view property (spec ruling 1); `get_email` and `raw` are the truth.
- **Hostile bodies.** Snippet work is bounded by the existing 8192-byte
  prefix; `anchors` and `attachments` are capped at 250 with exact totals;
  the stored `links` and `codes` are uncapped as today and the bridge's
  10 MiB refusal stays honest. Mitigation: the bound test transfers by the
  `compact.snippet == snippet` identity; the 250/251 pins cover the caps.
- **Migration of existing data files.** `ADDED_COLUMNS` already migrates
  nullable columns on open; `NULL` restores `None`. Mitigation: the old-file
  restore test and the corrupt-column refusal test.
- **Attachment classification is the parser's.** Inline images and attached
  messages appear in the outer list as mail-parser lists them. Mitigation:
  the fixture test names each case; the spec states that the view does not
  reclassify.
- **Token metric cannot be measured here.** Reported as "not measured: no
  paid trial" with a bytes proxy labelled as bytes; the harness's March
  client is the oracle's and stays.
- **Decision number.** 0092 is free on this tree and reserved for #155 by the
  index row of 0094; another candidate branch carries a different 0092.
  Mitigation: the operator resolves collisions at integration, as the index
  already records.
