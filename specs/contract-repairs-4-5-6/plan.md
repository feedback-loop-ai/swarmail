# Plan — contract repairs #4/#5/#6 (object inputSchemas, one shared seed, exact-count asserts)

Implementation plan for `specs/contract-repairs-4-5-6/spec.md`. All anchors
below were re-verified against the working tree on
`feat/epic13-04-05-06-contract-repairs` this session.

## Technical context

- Single Rust crate; production under `src/`, real-protocol suites under
  `tests/` (`tests/api.rs` REST, `tests/mcp.rs` MCP-over-HTTP, shared
  helpers in `tests/common/`: `start()`, `http_json`, `smtp_send`).
  No mocks anywhere in the loop.
- The relevant modules: `src/mcp.rs` (TOOLS static + tool handlers),
  `src/api.rs` (`AwaitQuery`, `await_messages`, `assert_messages`,
  `seed_inbox`), `src/smtp.rs` (`build_email` at :523, the accept loop).
- Coverage is measured by `scripts/coverage-gate.sh` — literal integer
  100/100/100 lines/branches/functions, `--branch` on nightly,
  `coverage(off)` forbidden. Every new branch needs a wire-driven test or
  the gate refuses; that refusal is the gate working.
- Model/policy: unchanged (`glm-flash` pinned); no new dependency —
  `serde_json`, the existing regex/tree cover everything.

## Constitution check

| Guarantee | Effect of this wave |
|---|---|
| 0001 accept == stored | Untouched. Both seed paths end in synchronous `store.insert` and answer only after it; the shared constructor adds no await between build and insert; the hostile-field rejection happens **before** any insert. |
| 0002 await subscribes before it checks | Untouched. `Store::wait_for` (`src/store.rs:404-434`, subscribe at :411 before scan at :414) has zero diff; `assert` registers no watcher, before and after. |
| 0003 oldest-only pruning | Untouched. Seeds flow through ordinary `insert`; the assert path reads the post-prune truth under the same shard guard. |
| 0005 extraction at ingest | Strengthened. The shared constructor *ends* in `build_email` (`src/smtp.rs:569-570` extraction); MCP's advertised `html` finally reaches the extractor. |

Zero-diff demand (robustness seat, binding): `src/store.rs`,
`src/persist.rs`, and the SMTP accept path — the `250` sequence at
`src/smtp.rs:454-456` — are byte-identical. Editing `src/smtp.rs` to add
the seed constructor beside `build_email` does **not** touch the accept
loop; review checks the diff.

## Project structure — files touched

| File | Change |
|---|---|
| `src/smtp.rs` | Add one `pub fn build_seed_email(...)` beside `build_email` (defaults, html-wins raw shape, hostile-field rejection, `build_email` call). No change below the seed helper. |
| `src/api.rs` | Rename `AwaitQuery::count()` → `min_one_count()` (await site, :270); add `exact_count()` and use it in `assert_messages` (:307); rewire `seed_inbox` (:424-442) to `build_seed_email`, returning `ApiResult<Json<Email>>` (400 on refusal). `SeedRequest` unchanged. |
| `src/mcp.rs` | Re-kind `SCHEMA_EMAIL_ID`/`SCHEMA_INBOX_ONLY` (:221-222) into object `Value`s under the same names (five sites unchanged); seed tool calls `build_seed_email`, `isError` on refusal; seed tool description states html-wins + hostile-field refusal. |
| `src/static/openapi.json` | Seed: add the `400` response + `SeedRequest` field descriptions (html preserved & supersedes text; CR/LF/NUL refused). Assert: state `count=0` asserts an empty inbox. Await: state count is minimum-one. |
| `src/static/llms.txt` | Agent-loop section: negative assertion via `assert?count=0`; seed row: html preserved, html supersedes text. |
| `README.md` | Endpoint-table rows for `assert` (count=0) and `seed` (html preserved). Two rows, nothing else. |
| `tests/api.rs` | Extend `assert_endpoint_returns_ok_or_conflict` (:78) + new: exact-zero (empty/one-message/filtered), no-count default, `count=-1` 400, await?count=0 408 pin, seed 400 hostile fields, chaos.data → assert?count=0 pair. |
| `tests/mcp.rs` | Extend `initialize_and_tools_list` (:34) with object-type + constraint assertions and the required-arg parity walk; new: MCP html seed, cross-adapter parity, MCP hostile-field isError, wait count:0 pin. |

Not touched: `src/store.rs`, `src/persist.rs`, `src/extract.rs`,
`src/chaos.rs`, `deny.toml`, `scripts/coverage-gate.sh`, `bundle.json`,
`policy.json`. Fallback (pre-authorized in the spec): if implement finds a
real cohesion reason, the constructor may live in a ~20-line `src/seed.rs`
registered in `lib.rs` instead — the contract is *one owner*, not one file.

## Data model & contracts

No new types, no persisted-shape change (`Email` untouched; `raw`/`size`
byte-stable for every fixture shape that exists today). The one new
production function:

```rust
/// The shared seed policy (decision 0010): defaults, html-wins, hostile-
/// field rejection, then the ordinary ingest constructor. The SMTP sink
/// accepts hostile bytes; the fixture constructor refuses hostile fields.
pub fn build_seed_email(
    inbox: &str,
    to: &str,
    from: Option<&str>,
    subject: Option<&str>,
    text: Option<&str>,
    html: Option<&str>,
) -> Result<Email, String>
```

- Rejection: `to`/`from`/`subject` containing `'\r'`, `'\n'` or `'\0'` →
  `Err` naming the field, before any insert. Adapters surface it
  identically: REST `400 {error: ...}` via `bad_request`, MCP
  `error_result` (`isError: true`) with the same reason.
- Raw shape (byte-identical to today's REST, `src/api.rs:431-438`):
  `From: {from}\r\nTo: {to}\r\nSubject: {subject}\r\n` then, for `html`:
  `MIME-Version: 1.0\r\nContent-Type: text/html\r\n\r\n{html}\r\n`; else
  `\r\n{body}\r\n` with `text.unwrap_or_default()`. Recipients are
  `[to]` (both adapters pass exactly that today), envelope `from` as given.
- Wire contracts:
  - `POST /api/v1/inboxes/{inbox}/seed` — `200` full `Email` (unchanged
    shape), `400` on hostile fields (new response in openapi).
  - `swarmail_seed_email` — `{id, links, codes}` on success (unchanged
    shape), `isError` text on refusal; `html` argument now read.
  - `GET …/assert` — `200 {ok, inbox, matched}` iff
    `matched == exact_count()`; `409 {ok, expected, matched}` otherwise;
    absent `count` still defaults to 1; `count=-1` is a `400` from query
    parse (usize).
  - `GET …/await` and MCP `wait_for_email` — unchanged min-one behavior.

## Quickstart (how an operator sees the repair)

```bash
cargo run -- serve &   # :8025
# 1. The negative assertion now works:
curl -s 'localhost:8025/api/v1/inboxes/t/assert?count=0'          # {"ok":true,…,"matched":0}
curl -s -X POST localhost:8025/api/v1/inboxes/t/seed \
  -d '{"to":"a@b.io","text":"hi"}' >/dev/null
curl -s 'localhost:8025/api/v1/inboxes/t/assert?count=0'          # 409 {"expected":0,"matched":1}
# 2. Hostile fixture fields are refused (nothing stored):
curl -s -X POST localhost:8025/api/v1/inboxes/t/seed \
  -d '{"to":"a@b.io\r\nBcc: smuggled@x.io"}' -i                  # 400, inbox unchanged
# 3. Schemas are objects:
curl -s localhost:8025/mcp -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | jq '.result.tools[1].inputSchema | type'   # "object"
# 4. MCP html seeding extracts at ingest:
curl -s localhost:8025/mcp -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"swarmail_seed_email","arguments":{"to":"h@x.io","html":"<a href='"'"'https://q.io'"'"'>go</a> 555555"}}}' | jq -r '.result.content[0].text' | jq '{html, links, codes}'
```

## Implementation notes (per repair)

- **#4.** Define
  `static SCHEMA_EMAIL_ID: std::sync::LazyLock<Value> = …(json!({ … }))`
  (and `SCHEMA_INBOX_ID`→ inbox-only sibling) carrying the verbatim fields
  from today's strings (`src/mcp.rs:221-222`); `.clone()` at the five
  `inputSchema` sites. `json!` then emits objects. No schema-builder, no
  `schemars`, no runtime argument validation.
- **#5.** Adapters keep parsing local (REST's serde `SeedRequest`; MCP's
  `arg_str` reads — a non-string arg stays ignored-and-defaulted, not hard-
  rejected) and call `build_seed_email`, then `store.insert` the `Ok`
  email. Defaults move into the constructor (both adapters duplicate the
  same two literals today, `src/api.rs:429-430`, `src/mcp.rs:177-178`).
- **#6.** The handler compare `matched == want` (`src/api.rs:309`) is
  untouched. `min_one_count()` keeps `.unwrap_or(1).max(1)`;
  `exact_count()` is `self.count.unwrap_or(1)`. `swarmail_get_latest_email`
  (`src/mcp.rs:112`) is a fetch count — untouched, not harmonized.

## Risks

1. **The exact 100/100/100 gate refuses an uncovered new arm.** The
   constructor's arms (hostile ×3 fields, html Some/None, text
   None-default) and both accessors' arms must each be driven over the
   wire. *Mitigation:* the test map in `tasks.md` (every arm paired with
   the real-protocol test that drives it, `tests/api.rs` / `tests/mcp.rs`
   via `tests/common`); implement runs `scripts/coverage-gate.sh` before
   reporting; an uncovered arm means the arm is dead or under-tested — the
   fix is a test, never an exclusion (`coverage(off)` is forbidden).
2. **Cross-adapter parity flakes on volatile fields.** `id` (UUIDv7) and
   `received_ms` differ by construction. *Mitigation:* parity asserts
   content fields only (`text`, `html`, `subject`, `links`, `codes`) —
   pinned in the spec's AC4 and the task list.
3. **REST seed behavior change for hostile fields (`400` where a lying
   `200` was).** A caller seeding CR/LF in envelope fields gets refused.
   *Mitigation:* recorded as semantic in decision 0010 (status `proposed`);
   documented in openapi (the seed `400` response) and the tool
   description; per 0004's private test inboxes no other test's mailbox is
   affected.
4. **Exact-zero asserts race in-flight mail.** A caller asserting zero
   while its own delivery is in flight races itself. *Mitigation:* the
   docs' clear → act → assert recipe plus 0004 isolation and 0001
   synchronicity; the chaos.data → `assert?count=0` pair test pins the
   deterministic case. Exact counts hold only under the per-inbox cap
   (0003, as documented).
5. **Two min-one clamps drift back into one.** After the split, REST await
   and MCP wait each carry min-one; a future "cleanup" could re-merge them.
   *Mitigation:* semantics-named accessors (`min_one_count` /
   `exact_count`), pin tests (AC9/AC10), and decision 0010 stating the
   ruling.
6. **Raw-shape drift breaks existing callers.** The constructor must leave
   the text-only path byte-compatible with today's REST raw while making
   MCP's `html` real. *Mitigation:* `seed_with_html_runs_the_extraction_
   pipeline` (`tests/api.rs:236`, unchanged) is the shape-stability proof;
   the cross-adapter parity test pins identical construction; the html-wins
   pin covers the two-body case.
7. **Docs and wire drift apart.** A repaired contract with stale docs is
   the same lie-class as a string schema. *Mitigation:* openapi.json,
   llms.txt, README rows and the seed tool description move in the same
   commit (task 8), scoped to the three repaired contracts.
8. **Decision numbering collision with the unmerged wave-3 branch.**
   `feat/epic13-03-14-watcher-atomicity` carries an unrelated provisional
   0010. *Mitigation:* noted in decision 0010's Numbering line; the
   operator renumbers at acceptance if both land. No action in this wave.
