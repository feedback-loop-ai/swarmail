# Ordered tasks — compact email view and field projections

Each task is checked only with its paired proof. Every test runs against real
servers (0004); inline modules stay measured (0020). Nothing is `complete`
while a box is open.

- [x] **T0 — Prove the stacked base before any edit (AC6).** On the untouched
  tree run the inherited suites and record the `assert_appended` reading from
  the spec in `implementation.md`; inventory the modern-revision test sites
  that read `text`/`html` from search or latest.
  **Test:** `cargo test --locked --test mcp_revisions --test focused_waits`
  exits 0 at `f663c44`; `legacy_bytes`, `modern_all_tools_http_and_stdio` and
  `served_openapi_pins_and_negative_controls` are named green in the story.
- [x] **T1 — Capture attachments at ingest (AC4).** `model::Attachment`,
  `Email.attachments` (`#[serde(skip)]`), `build_email` fills it from the
  outer parser list; every construction site gains the member; existing
  serialization bytes unchanged.
  **Test:** inline `src/smtp.rs` test: a multipart fixture with a named PDF,
  an inline image and an attached message yields the three entries in parser
  order with the message counted once; a text-only mail yields `Some([])`;
  an unparsable body yields `None`; `serde_json::to_value(email)` carries no
  `attachments` key.
- [x] **T2 — Store and restore the column strictly (AC4).** `attachments_json`
  in `COLUMNS`, `SCHEMA`, `ADDED_COLUMNS` and the insert; a generic strict
  reader shared with `link_metadata`; `row_to_email` restores tuples.
  **Test:** inline `src/persist.rs` tests: round trip of `Some(list)`,
  `Some([])` and `None`; a file created without the column migrates and
  restores `None`; `UPDATE emails SET attachments_json='[["a"]]'` refuses the
  restore with `FromSqlConversionFailure` at the column index; `cargo test
  --locked --test persistence` and the burst prune tests stay green with no
  orphan row.
- [x] **T3 — The compact view and its byte pins (AC1).** `api::CompactEmail`
  from `&Email`; `compat::snippet` becomes `pub(crate)`; capped `anchors` and
  `attachments` with `_total` members.
  **Test:** inline `src/api.rs` pins, byte-exact JSON strings, for text-only,
  html-only, multipart with two anchors and one attachment, 200 and 201
  collapsed scalars with `é`, `雪` and `😀` at the cut, the historical row
  (`links`, `attachments: null`), 250 and 251 anchors, 250 and 251
  attachments; and `compact.snippet == compat::snippet(email)` on every
  fixture, which carries the existing prefix-bound proof.
- [x] **T4 — MCP compact default and `fields` (AC2, AC5).** `MODERN_TOOLS`;
  revision-aware `unknown_argument`; `fields` staged before lookup and before
  `park_await`; search and latest serve `CompactEmail` on June/November; the
  four reads project; modern descriptions, `fields` property with the 21-name
  `enum`, `projected_email` schema with `required: ["id"]`.
  **Test:** `tests/mcp_revisions/structured.rs` over HTTP and stdio on June
  and November: compact items in text and `structuredContent`; `get_email`
  without `fields` byte-identical to the current modern baseline; exact key
  sets for `fields` on each tool with the body-leak control; `fields: []`
  ids only; unknown name `-32602` "fields must be an array of known field
  names (unknown: nope)" on a missing id; wrong type refusal sentence; the
  await gauge unchanged after a refused wait; every served output validates
  against its schema; `{}` still invalid; catalogue 15; March: `unknown
  argument: fields` and `legacy_bytes` unmodified; pin tests pass with the
  four named exceptions.
- [x] **T5 — REST `view=compact` (AC3).** `ListQuery.view`, `View::Compact`,
  the handler branch, OpenAPI `view` parameter and `CompactEmail` component.
  **Test:** `tests/api.rs`: `view=compact` returns `{total, emails:[compact]}`
  byte-pinned against T3's fixtures; the no-key list is byte-identical to the
  existing pin; `view=full` and `view=` answer 400 naming `view`; the inline
  served-parameters test lists `view` on the list route only; the single
  message, thread and feed routes still take no query.
- [x] **T6 — Documents and the index row.** README MCP section, `llms.txt`,
  `docs/decisions/README.md` row for 0108 (status proposed), release notes if
  the release process asks.
  **Test:** `cargo test --locked --test listener_contract_docs --test
  store_contract_docs` and the OpenAPI pin tests stay green; the README and
  `llms.txt` name `fields`, `view=compact` and the compact default on modern
  revisions.
- [x] **T7 — The story and the metric (AC6, AC7).** Write
  `specs/compact-email-view/implementation.md`: T0's reading and exits, the
  byte table (March full item, November compact item, modern reply with both
  representations) labelled as bytes, and the literal "not measured: no paid
  trial".
  **Test:** the byte figures come from T3/T4's pinned fixtures (`len()` of
  the pinned strings asserted in the tests), not from prose.
- [ ] **T8 — Every gate, locked (AC8).** (Every gate ran green in the implement seat except `brokkr compile`, whose pinned binary is not installed in that sandbox; see `implementation.md`. Left open for the verify seat.) `cargo fmt --all -- --check`;
  `cargo clippy --workspace --all-targets --locked -- -D warnings`;
  `cargo test --workspace --locked`; `cargo test --manifest-path
  evals/Cargo.toml --locked`; `brokkr compile --bundle .brokkr/recipes/self`;
  `bash scripts/coverage-gate.sh` and `selftest`;
  `bash scripts/publish-guard-test.sh`; `bash scripts/seal-branch-test.sh`;
  `cargo deny check licenses`. Commit unsigned; never push.
  **Test:** each command's exit 0 recorded in the story; coverage 100% exact
  (missed == 0) with no `coverage(off)` and no dependency change.
