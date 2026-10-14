# Tasks — shared domain validation

Ordered. Every task names the test that proves it; no task is done
until its proving test runs green against the change and red against
its absence (for the pins: until the served doc and the constants can
no longer drift without the test failing). The gates (`fmt --check`,
`clippy -D warnings`, `test --workspace --locked`, `brokkr compile`,
`coverage-gate.sh` — exact 100/100/100, `deny check licenses`) are the
final task's exit condition and re-run at HEAD before reporting.

- [ ] **The domain rule and its words in `src/chaos.rs`.** Add
  `pub(crate) const PROBABILITY_MAX: u8 = 100;` and
  `pub(crate) const PROBABILITY_REFUSAL: &str = "probability must be an
  integer between 0 and 100";` beside `ChaosRule`; add
  `ChaosRule::validate(&self) -> Result<(), String>` (refusing
  `> PROBABILITY_MAX` with the const) and
  `ChaosConfig::validate(&self)` walking all four `Option`s through it.
  No other file compiles differently yet.
  *Proving test* (inline `#[cfg(test)]`, 0020 rule 1): 0 and 100
  validate `Ok`; 101, 150, 255 return `Err(PROBABILITY_REFUSAL)`
  exactly; the walk accepts an all-four-present in-domain config and an
  all-absent one, and refuses a bad rule planted in *each* of the four
  slots (connect, mail_from, rcpt, data) with the shared sentence —
  every `Option` arm of the walk taken on every run.
- [ ] **The wait default, the shared cap check and the malformed
  refusal in `src/api.rs`.** Add
  `pub(crate) const DEFAULT_WAIT_TIMEOUT_MS: u64 = 5_000;` and
  `pub(crate) const WAIT_TIMEOUT_REFUSAL: &str = "timeout_ms must be an
  integer between 0 and 60000";` beside `MAX_AWAIT_TIMEOUT_MS`; add
  `pub(crate) fn checked_wait_timeout_ms(ms: u64) -> Result<Duration,
  String>` (over-cap → `Err(format!("timeout_ms must be at most
  {MAX_AWAIT_TIMEOUT_MS}"))`, else the `Duration`); rewrite
  `AwaitQuery::timeout()` as
  `checked_wait_timeout_ms(self.timeout_ms.unwrap_or(DEFAULT_WAIT_TIMEOUT_MS)).map_err(bad_request)`.
  *Proving test* (inline): the cap arm returns the exact existing text
  for 60001 and the `Duration::from_millis(0/60_000)` Ok arms for 0 and
  60000; the existing 0018 legs (`await_timeout_over_the_cap…` in
  `tests/api.rs`, `wait_over_the_cap_is_refused_without_parking` in
  `tests/mcp.rs`) pass **unchanged** — the text survived the move.
- [ ] **REST chaos validates before it sets** — `set_chaos` becomes
  `ApiResult<Json<ChaosConfig>>` running
  `config.validate().map_err(bad_request)?` before `state.chaos.set(config)`.
  `get_chaos`/`clear_chaos` untouched.
  *Proving test* (wire, `tests/api.rs`
  `chaos_probability_out_of_domain_is_refused_and_leaves_the_rules_unchanged`):
  arm a valid data rule; PUT `{"data": {"probability": 101/150/255}}`
  each answers `400` with body exactly
  `{"error": "probability must be an integer between 0 and 100"}`;
  `GET /api/v1/chaos` is JSON-identical before and after the rejected
  PUTs; `probability: 0` and `100` PUTs answer 200 (the poles, on the
  domain layer); a mixed partial config with one bad slot
  (`{"rcpt": {"probability": 150}}` after arming data) is refused and
  the data rule survives the GET.
- [ ] **The REST wrong-type rows keep the extractor's envelope, and the
  rules stay untouched.** No code expected beyond the handler's new
  signature — this task pins the class so no later change "fixes" it.
  *Proving test* (wire, same test or its neighbour): PUT bodies
  `{"data": {"probability": "high"}}`, `{"probability": 1.5}`,
  `{"probability": null}`, `{"probability": 256}` and `not json` each
  answer the extractor rejection (400 for malformed JSON, 422 for a
  well-formed body with a wrong-typed field — assert the status class,
  not serde's message) and `GET /api/v1/chaos` is unchanged after each.
- [ ] **MCP `swarmail_set_chaos` routes the ceiling through the shared
  validator.** Import `PROBABILITY_REFUSAL` from `crate::chaos` for the
  presence/type/`u8` staging; delete the local `if probability > 100`
  arm; call `rule.validate()?` after the rule is built (before the
  read-clone-modify-`set` tail). Net lines in `src/mcp.rs` negative.
  *Proving test* (wire): `set_chaos_refuses_every_malformed_wire_value_by_name`
  in `tests/mcp.rs` passes **with no edit to its assertions** — the
  JSON-RPC `error.message` is byte-identical through the new owner;
  add the rows `probability: null`, `101`, `255` to its table (same
  sentence, `null` included — supplied is supplied).
- [ ] **MCP's set read-back proves a refused rule is never stored.**
  *Proving test* (wire, `tests/mcp.rs`): on a fresh server, refuse
  `{"event": "data", "probability": 150}` (JSON-RPC error), then set
  `{"event": "rcpt", "probability": 50}` — the tool result's current
  config shows `rcpt` with probability 50 and `data` null: the refused
  rule was never stored (the set tail is the read-back; no new tool).
- [ ] **The MCP wait parser: supplied-vs-omitted, refused before any
  subscribe.** Replace `a.get("timeout_ms").and_then(|v|
  v.as_u64()).unwrap_or(5000)` with the explicit match (`None` →
  `DEFAULT_WAIT_TIMEOUT_MS`; `Some` unparseable →
  `Ok(error_result(WAIT_TIMEOUT_REFUSAL))`); map
  `checked_wait_timeout_ms`'s `Err` to the same `error_result`; both
  refusals run before `store.wait_for`. Add `"default": 5000` to the
  wait tool's `timeout_ms` schema literal.
  *Proving test* (wire, `tests/mcp.rs`
  `a_supplied_malformed_timeout_refuses_and_registers_no_waiter`): seed
  a matching mail into an inbox; for arguments
  `"timeout_ms": "invalid"`, `-1`, `1.5`, `null` the tool answers
  `isError: true` with text `"timeout_ms must be an integer between 0
  and 60000"` immediately — never the Found shape (a silent default
  would have answered Found against the seeded mail), and the mail is
  still listable afterwards.
- [ ] **The omission keeps the documented default on both transports.**
  No new logic — this pins the `None` arms.
  *Proving test* (wire, shaped on lifecycle's parked legs): REST — spawn
  `GET /api/v1/inboxes/{box}/await?to=nobody@x.io` (no `timeout_ms`) on
  an empty inbox, 300 ms sequencing pause, `srv.stop()`, assert the
  request answers `503 {"error": "shutting down"}` (a refused or
  zeroed default would have answered 400/408 before the stop);
  MCP — the twin with an omitted `timeout_ms`, asserting the `isError`
  `"shutting down"` result. No wall-clock validity is asserted; the
  answer's shape is the discriminator.
- [ ] **Boundary rows on the domain: 0 and 60000 legal, 60001 refused,
  on both transports.** *Proving test* (wire): REST `timeout_ms=0` on an
  empty inbox answers `408` immediately; `timeout_ms=60000` is the
  existing satisfied-mid-wait leg (unchanged); `60001` the existing 400
  leg (unchanged). MCP `timeout_ms: 0` on an empty inbox answers
  `timed_out: true` at once; `60001` the existing `isError` leg
  (unchanged); `60000` parks — lifecycle's parked-MCP leg (unchanged).
- [ ] **The served docs pin to the constants.** openapi.json:
  `ChaosRule.probability` gains `{"type": "integer", "minimum": 0,
  "maximum": 100}` and the PUT gains its `400` response entry with the
  shared sentence; `src/api.rs`'s inline tests pin the served
  openapi.json's probability bounds, PUT 400 and await
  `timeout_ms` default/minimum/maximum against `PROBABILITY_MAX` /
  `DEFAULT_WAIT_TIMEOUT_MS` / `MAX_AWAIT_TIMEOUT_MS`; `src/mcp.rs`
  gains its inline `#[cfg(test)] mod tests` pinning the TOOLS schema
  (`timeout_ms` default/min/max, `probability` min/max) against the
  same constants; `tests/mcp.rs`'s
  `the_schemas_publish_the_wire_bounds` gains the served
  `"default": 5000` assertion.
  *Proving test*: the pins themselves — red when a schema literal and
  a constant disagree (drift fails CI, not production), green at HEAD.
- [ ] **Landing doc tasks (the implementer's commit — never the design
  commit's file set).** Add the dated `Amended by 0025` marker to
  `docs/decisions/0018-shutdown-observed-waits-and-explicit-bounds.md`
  (0019 pattern: §3's refusal posture extends to supplied-malformed
  values via the shared validator) and the 0025 row to
  `docs/decisions/README.md` (status **proposed**).
  *Proving test*: `grep -n "0025" docs/decisions/0018-*.md
  docs/decisions/README.md` resolves to exactly the marker and the
  index row; no accepted decision's status text changed.
- [ ] **The gate sweep at HEAD.** *Proving test*:
  `cargo fmt --all -- --check && cargo clippy --all-targets -- -D
  warnings && cargo test --workspace --locked && brokkr compile
  --bundle bundles/self && bash scripts/coverage-gate.sh && cargo deny
  check licenses` all green; `git diff --stat` shows `src/store.rs`
  and `src/persist.rs` at zero diff, no new files under `src/`, and
  `GIT_CONFIG_COUNT=0 git grep -n 'coverage(off)' -- 'src/'` empty.
