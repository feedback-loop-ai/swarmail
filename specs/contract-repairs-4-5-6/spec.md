# Spec — Contract repairs #4/#5/#6: object inputSchemas, one shared seed, exact-count asserts

Epic: feedback-loop-ai/swarmail#13, wave 5 (issues #4, #5, #6). Branch:
`feat/epic13-04-05-06-contract-repairs`.

Decision: 0010 —
[docs/decisions/0010-exact-count-asserts-and-the-shared-seed.md](../../docs/decisions/0010-exact-count-asserts-and-the-shared-seed.md)
(status `proposed`; acceptance is the operator's).

## The problem — three published contracts that lie

Each repair brings a contract surface back in line with what the other
surface and the docs already promise. All anchors re-verified on the working
tree this session.

1. **#4 — schema of the wrong JSON type.** `SCHEMA_EMAIL_ID` /
   `SCHEMA_INBOX_ONLY` (`src/mcp.rs:221-222`) are `&str` constants; `json!`
   serializes a `&str` as a JSON *string*, so five of the twelve tools —
   `swarmail_get_email`, `swarmail_delete_email`, `swarmail_clear_inbox`,
   `swarmail_extract_links`, `swarmail_extract_codes` (sites
   `src/mcp.rs:231,235,237,244,246`) — publish `inputSchema` as a string.
   The MCP spec requires an object; a string was never a valid schema.
   Nothing in `tests/` asserts schema shape, so nothing would go red if a
   schema regressed again.
2. **#5 — a schema that advertises an argument the handler drops.**
   `swarmail_seed_email` publishes an `html` argument (`src/mcp.rs:250`) but
   the handler reads only `text` (`src/mcp.rs:179-180`): an HTML fixture
   seeded over MCP stores `html: null`, with no extracted links or codes —
   a silent violation of decision 0005's "every surface answers
   identically". Meanwhile REST's seed endpoint (`src/api.rs:424-442`)
   handles HTML. Two raw builders exist today (`src/api.rs:431-438`,
   `src/mcp.rs:180`); the duplication *is* the bug.
3. **#6 — an assert that cannot assert the empty inbox.** `GET
   /api/v1/inboxes/{inbox}/assert?count=0` clamps to one
   (`AwaitQuery::count()` applies `.max(1)`, `src/api.rs:142-144`), so the
   negative assertion — "nothing from this sender arrived" — is impossible:
   on an empty inbox the endpoint answers `409 {expected: 1, matched: 0}`
   because the requested zero is clamped to one. The OpenAPI document
   already promises an "exact match
   count" (`src/static/openapi.json`, assert summary); the code is what
   lies. The same clamp is load-bearing on the wait side: `await?count=0`
   must keep waiting for at least one mail, or it becomes an instant
   success against an empty inbox.

## Ruling — how the two positions were reconciled

The panel's positions (`.forge/design/positions/{simplicity,robustness}.md`,
both `pass`) agree on the shape of all three repairs. They conflict in four
places; this spec rules rather than averages.

**Adopted from both.** #4 re-kinds the two `&str` constants into object
`Value`s under the *same two names*, five call sites unchanged; no
`schemars`, no schema-builder, no runtime argument validation. #5 moves seed
construction into ONE shared owner for both adapters, HTML preserved
verbatim, html-wins precedence pinned, parsing kept local per adapter,
parity asserted on content fields only (`text`, `html`, `subject`, `links`,
`codes` — never `id`/`received_ms`, which are clock and UUIDv7 draws and a
flake factory). #6 splits the clamp's two semantics instead of deleting it:
a minimum-one WAIT and an exact-count ASSERT, `Store::wait_for` untouched,
`assert` never subscribing. Zero diff to `src/store.rs`, `src/persist.rs`
and the SMTP accept path — the `250` sequence at `src/smtp.rs:454-456` is
byte-identical before and after this wave.

**Ruled against simplicity: the seed constructor rejects hostile envelope
fields.** The simplicity seat's budget has no validation; the robustness
seat demands rejecting `CR`/`LF`/`NUL` in `to`/`from`/`subject`. Adopted,
because the wave's own contract is "identical fixtures produce identical
content and extracted links regardless of adapter", and a fixture whose
envelope field smuggles a header (`to: "a@b\r\nBcc: x"`) lands as a mail
that lies about itself — defeating the assert primitive #6 repairs, through
an unauthenticated fixture surface. The review phase has a dedicated
security member (decision 0009); knowingly shipping a documented-then-
declined header-smuggling hole would be a self-inflicted security-hold. The
asymmetry is the design and is stated in decision 0010: the SMTP sink keeps
accepting hostile bytes (a capture problem), the fixture constructor refuses
hostile *fields* (a fixture-contract problem). Cost over simplicity's
budget: one validation arm, surfaced identically (REST `400`, MCP
`isError`) and pinned on both surfaces — and a REST behavior change for
hostile fields, recorded as semantic in decision 0010.

**Ruled against simplicity: the accessors are named by semantics.**
Simplicity keeps `count()` and adds `exact_count()`; robustness wants
`min_one_count()`/`exact_count()`. Adopted the rename: call sites must read
as their contract, and the two-clamp drift risk (REST `await` and MCP
`wait_for_email` both carrying min-one after the split) is exactly the
mistake class this wave exists to kill — greppable names plus pin tests are
the mitigation. Cost: one renamed private method, two call sites.

**Ruled against simplicity: a decision document.** Simplicity argues a
paragraph in the spec suffices. Refused: #6 flips observable endpoint
behavior (`409→200` on an empty inbox, `200→409` on a one-message inbox),
#5 changes MCP seed behavior and adds the rejection posture, #4 changes
published wire shapes — public contracts, and the WAIT-min-one /
ASSERT-exact split plus the sink/fixture asymmetry are rulings a future
cleanup could silently re-merge. Decision 0010 records both rulings with
status `proposed`.

**Ruled against robustness: no new `src/seed.rs`.** The shared seed
constructor lives **beside `build_email` in `src/smtp.rs`** — seed *is* the
ingest pipeline minus the wire: build a raw message, hand it to
`build_email`, and `build_email` is already there with both adapters
importing that module. A new file is a permanent citizen of the exact
100/100/100 denominator for a capability one `mod` away. The epic's "one
module for REST and MCP" is satisfied: one module owns seed construction
(today there are two builders). Fallback, pre-authorized: if implement
finds a real cohesion reason, a ~20-line `src/seed.rs` registered in
`lib.rs` may carry it instead — the contract is *one owner*, not one file.
Cost of refusing robustness's preference: the fixture-validation policy
lives in a module chartered for the wire protocol; mitigated by a doc
comment marking the constructor as the shared seed policy and by decision
0010 naming it.

**Reconciled: docs move with the contract, scoped to what the wave makes
true.** Simplicity verified no doc becomes *false* (openapi documents
`SeedRequest.html`, `count` default 1 with no minimum; the assert summary
already says "exact match count"). Robustness is right that the newly
*stated* contract must live where agents read it. Scope: `openapi.json`
gains the seed `400` response and field-semantics descriptions, and states
`count=0` on assert / minimum-one on await; `src/static/llms.txt` gains the
negative-assertion sentence and the html-preservation note in the agent
loop; README's endpoint-table rows for assert/seed gain the same two
truths; the `swarmail_seed_email` tool description states html-wins. Nothing
beyond the three repaired contracts.

## What changes

### #4 — object inputSchemas, proven by type

- `SCHEMA_EMAIL_ID` / `SCHEMA_INBOX_ONLY` become object values (e.g.
  `static LazyLock<Value>`, the pattern `static TOOLS` already establishes
  in the same file) carrying the **verbatim** field names, types,
  descriptions and `required` arrays the strings hold today; the five
  `inputSchema` sites keep referencing the same names.
- A test-time parity check closes the schema/handler drift class #5 is an
  instance of: every published `inputSchema` is an object, and every
  `required` argument of the six tools this wave owns is actually enforced
  by its handler (missing `id` → error, `src/mcp.rs:103-109` shape; the
  `set_chaos` probability residual is named under AC2). This is a test,
  not a runtime validator — runtime validation against `inputSchema` is
  refused (a new
  capability, a new failure mode per tool call, a new 100% branch burden
  for nothing the issue names).
- Honest residual, named: schema/handler agreement is enforced by the test,
  not by the compiler. Generating schemas from typed structs (`schemars`)
  is refused — a new dependency and a codegen step for twelve small static
  shapes.

### #5 — one owner, one shape, one validation

- One shared constructor beside `build_email` in `src/smtp.rs` owns:
  the defaults (`from = "fixture@swarmail.dev"`, `subject = "(no
  subject)"`, absent body → empty), the raw shape (when `html` is present:
  `MIME-Version: 1.0` + `Content-Type: text/html` + the HTML body
  **preserved verbatim**; otherwise the plain-text body — byte-identical to
  today's REST raw, `src/api.rs:431-438`), html-wins precedence when both
  `text` and `html` are given, the hostile-field rejection, and the
  `build_email` call (extraction stays at ingest, 0005).
- Both adapters call it; neither builds raw bytes again. REST keeps its
  serde `SeedRequest` (`src/api.rs:414-421`) and full-`Email` response; MCP
  keeps per-key `arg_str` parsing (non-string args stay ignored-and-
  defaulted, not hard-rejected) and its `{id, links, codes}` response.
  Identical *construction*; parsing stays local.
- Rejection: `CR`, `LF` or `NUL` in `to`, `from` or `subject` refuses the
  seed **before** any insert — REST answers `400` with the offending field,
  MCP answers `isError` with the same reason. Nothing is stored. The SMTP
  accept path is untouched: it keeps storing hostile bytes, only oversize
  gets `552` (`src/smtp.rs:429-434`).
- Seeding both `text` and `html` stores the HTML and drops the text —
  today's REST incumbent (`src/api.rs:431-435`), now shared and *stated*
  (tool description, `SeedRequest` description, llms.txt). Multipart/
  alternative for both-given is refused: a new parse surface and changed
  `raw`/`size` for every two-part fixture — the shape-stability break the
  framing forbids.

### #6 — two named accessors, two pinned semantics

- `AwaitQuery::min_one_count()` = `count.unwrap_or(1).max(1)` — used only
  by `await_messages` (`src/api.rs:270`). `await?count=0` keeps waiting for
  at least one mail; it never becomes an instant success.
- `AwaitQuery::exact_count()` = `count.unwrap_or(1)` — used only by
  `assert_messages` (`src/api.rs:307`). The handler's `matched == want`
  compare (`src/api.rs:309`) is untouched: it already answers `0 == 0` with
  `200 {ok, matched: 0}` and `1 != 0` with `409 {expected: 0, matched: 1}`.
  Absent `count` still defaults to 1 (OpenAPI documents `default: 1`).
- "Exact" means the real **filtered** match count — `store.count(&inbox,
  &q.filter())` is already what the handler compares. `assert?count=0&to=
  absent@x` is the negative-assertion primitive; `assert?count=0` is its
  unfiltered special case. Dropping the filter would contradict the
  existing 409 body's `matched` semantics.
- `Store::wait_for` is untouched (subscribe before scan, `src/store.rs:411`
  → `:414`); the clamp never moves into the store — `wait_for(want=0)`
  already behaves honestly, the min-one rule is query semantics. `assert`
  registers no watcher, before and after.
- MCP `swarmail_wait_for_email` keeps its own min-one clamp
  (`src/mcp.rs:145`) — pinned by test so no future cleanup re-merges it.
  `swarmail_get_latest_email`'s `.max(1)` (`src/mcp.rs:112`) is a fetch
  count, not a wait: not part of this repair, not harmonized.

## What does not change

- **0001 (accept == stored):** both seed paths end in synchronous
  `store.insert` and answer only after it — the shared constructor adds no
  await between build and insert. Nothing stored means nothing answered.
- **0002 (await subscribes before it checks):** `wait_for` ordering
  untouched; `assert` is a synchronous snapshot compare with no watcher.
- **0003 (oldest-only pruning):** seeds flow through ordinary `insert` and
  are prunable like any mail; exact counts hold for mail younger than the
  cap — documented consequence, not a bug.
- **0005 (extraction at ingest):** the shared constructor *ends* in
  `build_email`; the repair makes MCP's advertised `html` actually reach the
  extractor instead of being dropped.
- **No new dependencies** (`deny.toml` untouched); no `coverage(off)`;
  `scripts/coverage-gate.sh` untouched — every new branch ships with a
  real-protocol test, and the exact 100/100/100 gate refusing an uncovered
  arm is the constitution working.
- Known honest losses stand unchanged, none made worse: a retry after a
  crash in the post-commit/pre-response window is a duplicate fixture (new
  UUIDv7, one mailbox, caller's clear-before-reseed); unbounded webhook
  in-flight tasks under sustained hostile bursts; exact counts only under
  the cap; unbounded DATA *time* on the plaintext path.

## Acceptance Criteria

- **AC1.** `tools/list` publishes `inputSchema` as a JSON **object** for all
  twelve tools; the five repaired tools carry exactly today's constraints
  (`properties.id`/`properties.inbox` with their descriptions, `required:
  ["id"]` / no required on inbox-only). Proven by extending
  `initialize_and_tools_list` in `tests/mcp.rs` over a real server — a
  substring check would pass on the buggy shape, so the test asserts the
  JSON type.
- **AC2.** For the five repaired tools and `swarmail_seed_email`, calling
  the tool without its `required` argument yields the documented error
  (`missing id` / `missing to`) — the schema/handler parity test in
  `tests/mcp.rs`. Named out-of-scope residual: `swarmail_set_chaos`
  declares `probability` required but defaults it to 0 when absent; that is
  a documented bare-call behavior pinned by an existing test, and refusing
  it is a separate contract change this wave does not make.
- **AC3.** `swarmail_seed_email` with `html` stores a mail whose `html` is
  the argument verbatim, with `links`/`codes` extracted at ingest, and
  answers `{id, links, codes}` — proven in `tests/mcp.rs`.
- **AC4.** The identical fixture seeded via REST and via MCP produces
  identical `text`, `html`, `subject`, `links`, `codes` (content fields
  only; `id`/`received_ms` excluded by construction) — the cross-adapter
  parity test in `tests/mcp.rs`.
- **AC5.** Seeding both `text` and `html` (either adapter) stores HTML and
  drops text; html-wins is pinned on both surfaces.
- **AC6.** A seed whose `to`, `from` or `subject` contains `CR`, `LF` or
  `NUL` is refused before insert: REST `400` naming the field, MCP
  `isError` with the same reason, inbox count unchanged — pinned on both
  surfaces.
- **AC7.** `assert?count=0` answers `200 {ok: true, matched: 0}` on an
  empty inbox and `409 {expected: 0, matched: 1}` on a one-message inbox;
  with a filter it tests the filtered count (`count=0&to=absent@x` → 200
  while other mail is present). Proven in `tests/api.rs`.
- **AC8.** `assert` without `count` still defaults to 1, and `count=-1` is
  a `400` (usize query parse refuses) — `tests/api.rs`.
- **AC9.** `await?count=0` keeps minimum-one: on an empty inbox it waits
  and answers `408` within a short timeout — never an instant success —
  and answers `200 {matched: 1}` once one matching mail has landed.
- **AC10.** MCP `swarmail_wait_for_email` with `count: 0` behaves as 1
  (pin for the second min-one clamp).
- **AC11.** The chaos pair is deterministic: with a `data` chaos rule that
  errors, a delivery attempt stores nothing and `assert?count=0` passes
  — the regression test robustness demands, in `tests/api.rs`.
- **AC12.** `src/store.rs`, `src/persist.rs` and the SMTP accept path
  (the `250` sequence, `src/smtp.rs:454-456`) are byte-identical; no new
  dependency; all six gates green including exact 100/100/100 coverage with
  every new branch driven over the real protocol; and the docs that state
  the repaired contracts moved in the same commit (`openapi.json`,
  `llms.txt`, README's assert/seed rows, the seed tool description).
