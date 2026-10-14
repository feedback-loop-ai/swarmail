# Plan — shared domain validation

HOW for [spec.md](spec.md). Ordering: the domain layer first
(constant → validator), then REST, then MCP (the deletion), then the
docs and their pins, then the wire matrices, then the gate sweep.
Every task carries its proving test ([tasks.md](tasks.md));
`src/store.rs` and `src/persist.rs` are never opened.

## Technical context

- `src/chaos.rs` — `ChaosRule { probability: u8, error, delay_ms }`
  (`:21-35`), `ChaosConfig { connect, mail_from, rcpt, data }`
  (`:37-47`), `Chaos::set/clear/current` (an `RwLock<ChaosConfig>`
  swapped whole, in-memory only), `Chaos::check` (`:68-80`) whose
  `random_range(0..100) < rule.probability as u32` turns a stored 150
  into a silent always-fire. Inline `#[cfg(test)] mod tests` exists
  (`:83-118`) — the home of the validator's unit pins.
- `src/api.rs` — `MAX_AWAIT_TIMEOUT_MS` (`:63`, `pub(crate)`, already
  imported by `src/mcp.rs`: the shared-home precedent);
  `AwaitQuery { count, timeout_ms: Option<u64> }` (`:158-166`) with
  `timeout()`'s 0018 cap refusal (`:184-194`);
  `parse_query`'s `400 invalid … query` (`:311-313`) — REST's existing
  supplied-malformed answer; `bad_request` (`:51-53`); the `set_chaos`
  handler (`:483-489`) — `state.chaos.set(config)` with no validation,
  the REST defect; an inline `#[cfg(test)] mod tests` exists
  (`:628-687`); `openapi_json` serves `include_str!("static/openapi.json")`
  (`:620-622`).
- `src/mcp.rs` — the wait tool's `timeout_ms` parser (`:160`,
  `a.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(5000)` — the
  indicted line) above the 0018 cap check (`:161-169`); the
  `swarmail_set_chaos` staging (`:244-268`: presence → `as_u64` →
  `u8::try_from` → the local `> 100` arm → `PROBABILITY_REFUSAL`) and
  its read-clone-modify-`set` tail (`:270-277`); the `arg_usize`
  helper (`:75-77`); `error_result`/`text_result` envelopes
  (`:44-51`); the `TOOLS` `LazyLock` schema literals (`:292-348`,
  `timeout_ms` has `minimum: 0, maximum: 60000` and no default;
  `probability` has `minimum: 0, maximum: 100`). No inline test module
  yet — one is added for the schema pin.
- `src/static/openapi.json` — `ChaosRule.probability` is
  `{"type": "integer"}` with no bounds; the PUT documents only 200; the
  await endpoint already publishes `timeout_ms` `default: 5000,
  minimum: 0, maximum: 60000` with its 400.
- `tests/` — `tests/api.rs` (`chaos_roundtrip_and_invalid_body_is_a_bad_request`
  `:276-304`; the 0018 cap/`400` leg `:558-590`; the parked-await shape
  lives in `tests/lifecycle.rs` `a_parked_await_does_not_hold_stop_hostage`),
  `tests/mcp.rs` (`set_chaos_refuses_every_malformed_wire_value_by_name`
  `:1102-1152` — JSON-RPC `error.message` envelope;
  `wait_over_the_cap_is_refused_without_parking` `:1009-1027` —
  `isError` envelope; `the_schemas_publish_the_wire_bounds` `:1167-1190`;
  the parked-MCP-wait shape in `tests/lifecycle.rs`
  `a_parked_mcp_wait_does_not_hold_stop_hostage`).
- No third consumer: `Chaos::set` is called from exactly two places
  (`src/api.rs:487`, `src/mcp.rs:277`); `src/config.rs` is CLI args
  only; `src/stdio.rs` bridges to the same MCP handler. The two
  transports are the whole world for chaos rules.

## Constitution check

- **0001/0002/0003 (frozen)**: untouched. A refused wait registers no
  watcher (it is 0002's complement: refuse-before-subscribe, extending
  0018 §3); a rejected chaos PUT swaps nothing and evicts nothing; the
  store is not opened.
- **0014/0015 (the boundary)**: validation is pure and sits above the
  boundary; the one ordered mutation boundary and its failure contract
  are uncalled by this slice; `src/store.rs`/`src/persist.rs` end at
  zero diff.
- **0018 §3**: amended *through* 0025 (the 0019 pattern) — the accepted
  file gains its dated `Amended by` marker as a landing task; this
  slice never rewrites the accepted text. §3's tested shapes (the cap
  text, the MCP envelopes, the refusal-before-subscribe) are preserved
  byte-for-byte; the supplied-malformed refusal is the *extension*.
- **0020 (test placement)**: every new line is a deterministic branch —
  the validators, the cap-check delegation, the parser match — so
  everything stays inline and measured. No cold path exists in this
  slice (no line runs only while the bug is present), so no harness
  file is created; the two pins read `pub(crate)` constants, which a
  `tests/` test must not touch (rule 3) — hence inline (rule 1). No
  `coverage(off)`; no sleep-as-synchronization (the omission probes'
  300 ms pause is the established *sequencing* discipline of the 0018
  legs — the parked wait cannot answer before the stop fires, and the
  asserted outcome is the answer's shape, not a wall-clock validity).
  No shared global state (the validators take `&self` on the config
  under test; the wire probes use per-test inboxes and fresh servers).
- **Dependency policy / bundle**: no new dependency, no `deny.toml`
  edit, no bundle change; `brokkr compile --bundle bundles/self` stays
  green at HEAD.
- **Modular monolith**: zero new files under `src/`; the two homes are
  the modules that already own the types and the constants.

## Project structure — every file this slice touches

1. `src/chaos.rs` — `PROBABILITY_MAX`, `PROBABILITY_REFUSAL`,
   `ChaosRule::validate`, `ChaosConfig::validate`, inline unit pins.
2. `src/api.rs` — `DEFAULT_WAIT_TIMEOUT_MS`, `WAIT_TIMEOUT_REFUSAL`,
   `checked_wait_timeout_ms`; `AwaitQuery::timeout()` delegates;
   `set_chaos` validates then sets (`ApiResult`); inline pins (cap text
   + openapi).
3. `src/mcp.rs` — the wait parser's supplied-vs-omitted match + the
   isError mappings; `set_chaos`'s staging drops the local `> 100` arm
   and calls `rule.validate()?`; the wait schema gains `"default":
   5000`; a new inline test module pins TOOLS against the constants.
4. `src/static/openapi.json` — `ChaosRule.probability`
   minimum/maximum; PUT 400 documented.
5. `tests/api.rs` — the REST matrix (chaos rows, wait rows, the
   no-waiter probe, the omission probe).
6. `tests/mcp.rs` — the MCP matrix (refusal rows, boundary rows, the
   no-waiter probe, the omission probe, the served-schema default).
7. Landing tasks (implementer's commit, not the design commit):
   `docs/decisions/0018-…` gains the dated `Amended by 0025` marker;
   `docs/decisions/README.md` gains the 0025 row (status **proposed**).

## The design, per file

### `src/chaos.rs` — the domain rule and its words

```rust
/// The probability domain, ruled once (issue #32 slice 2, decision 0025):
/// an integer 0..=100. Both transports call validate() — REST's PUT for
/// every rule it would install, MCP's set_chaos for the rule it would
/// set — so the wires cannot drift again.
pub(crate) const PROBABILITY_MAX: u8 = 100;
pub(crate) const PROBABILITY_REFUSAL: &str =
    "probability must be an integer between 0 and 100";

impl ChaosRule {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.probability > PROBABILITY_MAX {
            return Err(PROBABILITY_REFUSAL.to_string());
        }
        Ok(())
    }
}

impl ChaosConfig {
    /// A PUT replaces the whole config: every present rule must be in
    /// the domain, in all four events, before any caller sets it.
    pub(crate) fn validate(&self) -> Result<(), String> {
        for rule in [&self.connect, &self.mail_from, &self.rcpt, &self.data]
            .into_iter()
            .flatten()
        {
            rule.validate()?;
        }
        Ok(())
    }
}
```

The mcp.rs staging keeps its presence/type/`u8` stages with the same
sentence (`ok_or(PROBABILITY_REFUSAL)?` — the const now lives here, one
owner), drops the local `> 100` arm, and calls `rule.validate()?` on the
constructed rule. Note the stage *order* for a doubly-invalid call: the
ceiling now runs after the `delay_ms` stage (single-invalid rows pin
each stage's sentence; no test pins the double-invalid order).

### `src/api.rs` — the wait default, the cap check, the REST refusal

```rust
/// The documented wait default (issue #32 slice 2, decision 0025):
/// named once, used by both parsers — REST's query default and MCP's
/// omitted `timeout_ms` arm.
pub(crate) const DEFAULT_WAIT_TIMEOUT_MS: u64 = 5_000;

/// The supplied-malformed refusal: one sentence, both transports (the
/// MCP parse stage reuses it before the value exists to check).
pub(crate) const WAIT_TIMEOUT_REFUSAL: &str =
    "timeout_ms must be an integer between 0 and 60000";

/// The one cap check for every wait surface (0018 §3, made shared by
/// 0025): over the cap is a refusal naming the bound, never a clamp.
pub(crate) fn checked_wait_timeout_ms(ms: u64) -> Result<Duration, String> {
    if ms > MAX_AWAIT_TIMEOUT_MS {
        return Err(format!("timeout_ms must be at most {MAX_AWAIT_TIMEOUT_MS}"));
    }
    Ok(Duration::from_millis(ms))
}
```

`AwaitQuery::timeout()` becomes
`checked_wait_timeout_ms(self.timeout_ms.unwrap_or(DEFAULT_WAIT_TIMEOUT_MS)).map_err(bad_request)`
— `unwrap_or` now touches only genuine omission, on both transports.
The handler:

```rust
async fn set_chaos(
    State(state): State<AppState>,
    Json(config): Json<ChaosConfig>,
) -> ApiResult<Json<ChaosConfig>> {
    // The domain rule is owned once (0025): validate before any effect,
    // so a rejected config leaves the active rules exactly as they were.
    config.validate().map_err(bad_request)?;
    state.chaos.set(config);
    Ok(Json(state.chaos.current()))
}
```

`get_chaos`/`clear_chaos` are unchanged; `GET` keeps returning the
current config as the roundtrip test pins it.

### `src/mcp.rs` — the deletion, the match, the envelope

```rust
// Omission takes the documented default (0025); a *supplied* value that
// does not parse is refused — never silently defaulted. `null` is
// supplied (the key is present): `Value::Null.as_u64()` is None, so it
// refuses with no special case — the same answer delay_ms already gives.
let timeout_ms = match a.get("timeout_ms") {
    None => crate::api::DEFAULT_WAIT_TIMEOUT_MS,
    Some(v) => match v.as_u64() {
        Some(ms) => ms,
        None => {
            return Ok(error_result(crate::api::WAIT_TIMEOUT_REFUSAL.to_string()));
        }
    },
};
// The cap check is the shared one; its refusal is the same isError
// envelope the cap refusal already has (0018).
let timeout = match crate::api::checked_wait_timeout_ms(timeout_ms) {
    Ok(t) => t,
    Err(msg) => return Ok(error_result(msg)),
};
```

Both refusals answer *before* `store.wait_for` — no waiter is ever
registered for a rejected call. The `swarmail_set_chaos` arm: the
staging keeps `PROBABILITY_REFUSAL` (now `crate::chaos::`) for its
presence/type/`u8` stages, the local `if probability > 100` arm is
deleted, and `rule.validate()?` runs after the rule is built (the
JSON-RPC `error` envelope is preserved — `?` into the handler's
`Result<_, String>` is exactly what the staging refusals already do).

The TOOLS literal for `swarmail_wait_for_email`'s `timeout_ms` gains
`"default": 5000` (its `minimum: 0, maximum: 60000` stay).

### `src/static/openapi.json`

- `components/schemas/ChaosRule/properties/probability`:
  `{"type": "integer", "minimum": 0, "maximum": 100}`.
- `paths["/api/v1/chaos"].put.responses`: add `"400"` whose description
  carries the shared sentence — mirroring the await endpoint's 400
  entry (`"probability must be an integer between 0 and 100 — refused,
  never stored (issue #32 slice 2, decision 0025)"`).
- Nothing else moves: the await `timeout_ms` already publishes
  `default/minimum/maximum`; `delay_ms`, `error` and the config shape
  are untouched (including the typed null-clears-a-rule semantics).

## Contracts — the wire after this slice

| Input | REST | MCP |
|---|---|---|
| chaos probability `0` / `100` | 200, stored | ok, stored |
| chaos probability `101` / `150` / `255` | 400 `{"error": "probability must be an integer between 0 and 100"}` — rules unchanged | JSON-RPC `error.message` = same sentence — rule not stored |
| chaos probability `"high"` / `1.5` / `null` / `256+` | extractor rejection (400 malformed JSON / 422 wrong-typed field) — rules unchanged | JSON-RPC `error.message` = the same sentence — rule not stored |
| wait timeout `0` | 200/408 immediate (real zero deadline) | `timed_out: true` at once |
| wait timeout `60000` | 200 satisfied mid-wait (existing 0018 leg) | parks — lifecycle's parked leg |
| wait timeout `60001` | 400 `timeout_ms must be at most 60000` | `isError` same text (existing 0018 leg) |
| wait timeout `-1` / `"invalid"` / `1.5` / `null` | 400 at the query parse (unchanged) | `isError` `"timeout_ms must be an integer between 0 and 60000"` — immediate, no waiter |
| wait timeout omitted | parks on the documented default → 408/200/503 per 0018 | parks on `DEFAULT_WAIT_TIMEOUT_MS` → the same three outcomes |

## Test placement & coverage arithmetic

- **Inline (0020 rule 1 — all deterministic):** `chaos.rs` — the
  poles/ceiling rows of `ChaosRule::validate`, and the walk of
  `ChaosConfig::validate` (all-four-absent → ok; all-four-present →
  ok; a bad rule in *each* of the four slots → refused with the shared
  sentence, proving the walk sees every event). `api.rs` —
  `checked_wait_timeout_ms`'s two arms + the exact cap text; the
  openapi pin (bounds, default, the PUT 400) against the constants.
  `mcp.rs` — the new inline module's TOOLS pin (`timeout_ms` default/
  min/max, `probability` min/max) against the same constants. New
  branches: 2 (validate) + 8 (the walk's four Option arms) + 1 (cap
  check) + 2 (the MCP match) + 1 (each refusal mapping) — every one is
  a row in the matrices below or a unit pin; the gate's vacuity
  detector is the backstop, never `coverage(off)`.
- **Wire (`tests/api.rs`, `tests/mcp.rs` — 0004, real servers):** the
  full matrix on both transports (see tasks), including the two
  discriminating probes: the malformed timeout against a *pre-seeded
  inbox* (immediate refusal — a silent default would answer Found),
  and the omission probes shaped on lifecycle's parked-wait legs
  (spawn → 300 ms sequencing pause → `stop()` → the fixed
  503/`isError` answer — proving the omitted value parks on the
  documented default rather than refusing or zeroing; the exact 5000
  rides on the shared constant and the schema pins).
- **Served-schema pin:** `tests/mcp.rs`'s existing
  `the_schemas_publish_the_wire_bounds` gains the served
  `"default": 5000` (literal style, as today); the constants live one
  inline test away in both modules.

## Quickstart

```bash
cargo test --lib chaos::            # the validator pins
cargo test --lib api::tests         # the cap check + openapi pin
cargo test --test api  chaos_       # the REST matrix
cargo test --test mcp  set_chaos_refuses wait_ the_schemas
cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings \
  && cargo test --workspace --locked \
  && brokkr compile --bundle bundles/self \
  && bash scripts/coverage-gate.sh && cargo deny check licenses
```

## Risks

1. **REST's dual envelope for a bad probability (400 domain vs
   422/400 type) reads as inconsistency to a client.** *Mitigation:*
   openapi documents the 400 with the shared sentence next to the PUT,
   and the wrong-*type* class keeps the extractor envelope every other
   endpoint already has; the ≥256 residual is written down in 0025 §Consequences.
   If the operator wants one status for the whole band, that is a
   decision, not a drive-by (0025 §Refused names the `u16` door).
2. **Explicit-`null` MCP callers break** (`timeout_ms: null` /
   `delay_ms`-style supplied null now refuse where `timeout_ms: null`
   previously waited 5000 ms). *Mitigation:* the refusal names the
   field and the domain — the fix for the caller is omitting the key;
   0025 §Consequences records the visible behavior change, and the
   sibling `delay_ms: null` refusal already behaves this way, so the
   surface is *more* uniform after the slice. No silent path remains.
3. **A matrix row misses an arm and the exact-100% gate refuses.**
   *Mitigation:* the arm↔test map above is explicit (each validator
   arm, walk slot, match arm and refusal mapping names its row); a
   refusal is answered by one more wire row or unit pin — never by an
   exclusion or a harness file. The walk's all-absent arm is covered
   by every existing config test that sets a subset.
4. **The refusal-sentence const move silently changes MCP's
   JSON-RPC error text.** *Mitigation:* the existing
   `set_chaos_refuses_every_malformed_wire_value_by_name` test pins the
   exact sentence byte-for-byte across all five stages and runs
   *unchanged* against the new owner — the const is moved, not retyped.
   The cap text is pinned the same way by the 0018 legs, which must
   pass without edits.
5. **The omission probes' 300 ms pause is mistaken for timing flake.**
   *Mitigation:* it is the established 0018 *sequencing* discipline
   (the parked wait cannot answer before the stop fires; the 5000 ms
   default dwarfs the pause), and the asserted outcome is the answer's
   *shape* (503/`isError shutting down`), not a wall-clock validity —
   a defaulted-to-zero or refused parser answers instantly with a
   different shape and fails the test deterministically.
6. **Scope pressure at review** (unify MCP envelopes, fix
   count/limit/since_ms, widen `u16`, persist chaos). *Mitigation:*
   each is ruled in the spec with its cost, and 0025 §Refused carries
   the same list — the reviewer's ask is answered by pointing at the
   ruling, and the residual map is the next slice's starting point.
7. **Landing-order drift of the decision marker.** *Mitigation:* the
   0018 `Amended by` marker and the README index row are explicit
   landing tasks in [tasks.md](tasks.md), the 0019 pattern — the
   accepted file is pointed at, never rewritten, and the design commit
   carries only the proposed decision and the spec.
