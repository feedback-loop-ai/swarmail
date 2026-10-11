# Tasks — contract repairs #4/#5/#6

Ordered. Each task is paired with the real-protocol test that proves it
(real servers via `tests/common`, never mocks). Implement executes 1-10 in
order; task 10 is verify-only. The exact 100/100/100 gate is the arbiter:
if an arm measures uncovered, the arm is dead or under-tested and the fix
is a test — `coverage(off)` is forbidden.

- [ ] **1. Re-kind the two schema constants into objects** — in
  `src/mcp.rs:221-222`, turn `SCHEMA_EMAIL_ID` / `SCHEMA_INBOX_ONLY` into
  object `Value`s under the same names (e.g. `static LazyLock<Value>`,
  the incumbent `TOOLS` pattern), carrying the **verbatim** field names,
  types, descriptions and `required` arrays; the five `inputSchema` sites
  (`:231,235,237,244,246`) keep referencing them. No schemars, no schema
  builder, no runtime argument validation.
  *Test:* extend `initialize_and_tools_list` (`tests/mcp.rs:34`) — all
  twelve `inputSchema` assert `.is_object()` (the JSON *type*, not a
  substring); the five repaired tools pin `properties.id` /
  `properties.inbox` with their descriptions and the `required` arrays.
  Proves AC1.
- [ ] **2. Schema/handler parity test** — in `tests/mcp.rs`, call each of
  the five repaired tools plus `swarmail_seed_email` without its required
  argument and assert the documented error (`missing id` / `missing to`,
  per `src/mcp.rs:103-109,176`) — closing the drift class #5 is an
  instance of. Named residual, not repaired here: `swarmail_set_chaos`
  declares `probability` required but defaults it to 0 when absent (a
  pinned bare-call behavior; a separate contract change if ever wanted).
  *Test:* the parity walk itself (extends `tests/mcp.rs`; the existing
  `unknown_tool_and_missing_args_are_errors` covers the get_email case —
  add delete_email, extract_links, extract_codes, seed_email). Proves AC2.
- [ ] **3. Add the shared seed constructor and rewire REST** — in
  `src/smtp.rs`, add `pub fn build_seed_email` beside `build_email`
  (defaults `from = "fixture@swarmail.dev"`, `subject = "(no subject)"`,
  absent body → empty; raw shape byte-identical to today's REST,
  `src/api.rs:431-438`; html-wins; refusal of `CR`/`LF`/`NUL` in
  `to`/`from`/`subject` **before** any insert; ends in `build_email`).
  Rewire `seed_inbox` (`src/api.rs:424-442`) to it, returning
  `ApiResult<Json<Email>>` — `400` naming the field on refusal. The accept
  path (`src/smtp.rs:454-456`) gains zero diff.
  *Test:* existing REST seed tests stay green unchanged — the `seed()`
  helper (`tests/api.rs:8`), `seed_with_html_runs_the_extraction_pipeline`
  (`tests/api.rs:236`, the shape-stability proof) — plus new: a bare REST
  seed `{"to": …}` pinning the defaults, and the html-wins pin (both
  `text` and `html` given → html stored, text dropped). Proves AC3-AC5
  (REST side).
- [ ] **4. Wire MCP seed to the shared constructor** — `swarmail_seed_email`
  (`src/mcp.rs:174-189`) reads the advertised `html` argument, calls
  `build_seed_email`, answers `isError` with the same reason on refusal,
  keeps the `{id, links, codes}` response; keep per-key `arg_str` parsing
  (non-string args stay ignored-and-defaulted). State html-wins and the
  refusal in the tool description.
  *Test:* new in `tests/mcp.rs` — seed with `html` → `html` preserved
  verbatim and `links`/`codes` extracted; seed with `CR`/`LF`/`NUL` in
  `to`/`from`/`subject` → `isError`, store unchanged. Proves AC3, AC6
  (MCP side).
- [ ] **5. Cross-adapter parity test** — in `tests/mcp.rs` (it drives both
  surfaces): seed the identical fixture via REST `POST …/seed` and via MCP
  `swarmail_seed_email`, compare `text`, `html`, `subject`, `links`,
  `codes` for equality; `id`/`received_ms` are excluded by construction
  (clock and UUIDv7 draws — a parity test that touches them is a flake
  factory).
  *Test:* the parity test itself. Proves AC4.
- [ ] **6. Split the clamp into two named accessors** — in `src/api.rs`:
  rename `AwaitQuery::count()` → `min_one_count()` (`:142-144`, keeps
  `.unwrap_or(1).max(1)`), used only by `await_messages` (`:270`); add
  `exact_count()` = `self.count.unwrap_or(1)`, used only by
  `assert_messages` (`:307`). The `matched == want` compare (`:309`) and
  `Store::wait_for` are untouched. MCP `swarmail_wait_for_email` keeps its
  own min-one (`src/mcp.rs:145`); `swarmail_get_latest_email` (`:112`) is
  not harmonized.
  *Test:* new pins — `await?count=0` on an empty inbox answers `408`
  within a short timeout (never instant success) and `200 {matched: 1}`
  once one matching mail has landed (AC9); MCP `wait_for_email` with
  `count: 0` behaves as 1 (AC10). Extends `tests/api.rs` and `tests/mcp.rs`.
- [ ] **7. Exact-count assert tests** — extend
  `assert_endpoint_returns_ok_or_conflict` (`tests/api.rs:78`):
  `assert?count=0` on an empty inbox → `200 {ok, matched: 0}`; on a
  one-message inbox → `409 {expected: 0, matched: 1}`; filtered
  `count=0&to=absent@x` → `200` while other mail is present (the
  negative-assertion primitive); `assert` with no count → default 1
  (`409 {expected: 1, matched: 0}` on empty); `count=-1` → `400` (usize
  query parse refuses).
  *Test:* those five cases themselves. Proves AC7, AC8.
- [ ] **8. The chaos pair** — in `tests/api.rs`: set a `data` chaos rule
  that errors (`PUT /api/v1/chaos`), attempt a real SMTP delivery (the
  error line answers before insert, nothing stored), then
  `assert?count=0` → `200 {matched: 0}` — chaos rejects the mail and the
  assert confirms it, deterministically (no sleeps, no probabilistic
  flake; the seed path gains no chaos hook).
  *Test:* the pair test itself. Proves AC11.
- [ ] **9. Docs move with the contract** — `src/static/openapi.json` (seed
  `400` response + `SeedRequest` field descriptions: html preserved and
  supersedes text, CR/LF/NUL refused; assert `count` description states
  `0` asserts an empty inbox; await `count` description states minimum
  one), `src/static/llms.txt` (agent-loop: negative assertion via
  `assert?count=0`; seed row: html preserved), `README.md` (assert and
  seed endpoint-table rows, two rows only). In the same commit as the
  wire change.
  *Test:* the openapi/llms endpoints are served live — extend
  `tests/api.rs` doc tests (or the existing doc-serving assertions) to
  check the new description text is present at `/openapi.json` and
  `/llms.txt`. Proves AC12 (docs half).
- [ ] **10. Gates (verify-only)** — run the full floor: `cargo fmt --all
  -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
  `brokkr compile --bundle .`, `bash scripts/coverage-gate.sh` (exact
  100/100/100, every new arm covered), `cargo deny check licenses`. No new
  dependency appears; `deny.toml` and the gate script are untouched.
  *Test:* the six gates themselves; a red gate is a finding, not noise.
  Proves AC12 (gates half).
