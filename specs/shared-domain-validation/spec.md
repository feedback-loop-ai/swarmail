# Shared domain validation: REST and MCP refuse the same domain — a supplied malformed value is refused, an omitted optional value keeps its default

Issue: #32 slice 2 (design run `close-slice-2-of-issue-32-https--b7363ba3`)
Decision: **0025** — [shared domain validation: supplied malformed refuses, omission keeps its default](../../docs/decisions/0025-shared-domain-validation.md) (status *proposed*, awaiting the operator).

The wires disagree today, measured at HEAD: `PUT /api/v1/chaos` accepts any
`u8` probability (`src/api.rs` `set_chaos` — validate-then-set is missing, so
`150` is stored and behaves as a silent 100 in `Chaos::check`'s
`random_range(0..100) < p as u32`), while `swarmail_set_chaos` refuses it —
and `swarmail_wait_for_email` silently defaults a *supplied* malformed
`timeout_ms` to 5000 ms (`a.get("timeout_ms").and_then(|v| v.as_u64())
.unwrap_or(5000)`), while REST's query parse refuses the same inputs with
400. The published contracts already promise the strict rule —
`llms.txt`: "`probability` must be an integer 0..=100"; openapi's await
`timeout_ms`: `default 5000, minimum 0, maximum 60000` — while openapi's
`ChaosRule` publishes a boundless `probability` and documents no 400 on the
PUT. REST and the MCP timeout parser are behind their own docs.

This slice is not "add input checks". It is: **the accepted set and the
refusal semantics of both transports are one domain** — one validator per
rule, called by both transports, with supplied-malformed values refused and
omitted optional values keeping their documented default — because a `200`
that stored a rule the caller did not ask for, or a tool result that
silently waited a timeout the caller did not choose, is the control-plane
version of a `250` for mail that was never stored. Frozen guarantees are
untouched: the slice is pure control-plane validation above the boundary
(`store.rs`/`persist.rs` at zero diff), ordered refuse-before-effect.

## What the panel agreed on

Both seats measured the same tree and converged on the bones: the layering
*transport parse → domain validation → effect*, with the parse layer left
per-transport (unifying it means custom `Deserialize` and envelope churn
beyond the framed change); validation *before* any effect, so a rejected
chaos PUT never touches the write lock and a rejected wait registers no
waiter; 0018 §3's refusal-before-subscribe extended from the over-cap value
to the supplied-malformed one; the wait default and cap-check owned once
and shared through the constant MCP already imports; openapi's missing
`probability` bounds *added*, not generated; the count/limit/since_ms
asymmetry recorded with file:line, not fixed. The conflicts below are
where they genuinely split; each is ruled, with the losing side's cost.

## Rulings

Position files: `.forge/design/positions/{simplicity,robustness}.md`.
Line references locate by shape; `main` has moved only in comments and
tests since the reviewed commit 66fab48.

**R1 — The validator is a method on the type it constrains; no newtype, no
new file, no envelope rewrite** *(simplicity wins the structure;
robustness wins the invariant's wording, the docs pin and the residual
map)*.

`src/chaos.rs` gains `ChaosRule::validate` and `ChaosConfig::validate`
(walking all four events — a PUT replaces the whole config, so a partial
check would accept a bad rule in a slot it forgot to look at), and the
probability sentence becomes a `pub(crate)` constant beside them that
MCP's staging reuses — one owner of the rule *and* the words. REST's
`set_chaos` becomes `ApiResult<Json<ChaosConfig>>`:
`config.validate().map_err(bad_request)?` before `state.chaos.set(config)`.
MCP deletes its local `> 100` arm and calls `rule.validate()?` on the
constructed rule; `mcp.rs`'s net lines go negative.

*Cost of the losing side (robustness's `Probability`/`WaitTimeoutMs`
newtypes constructed only through the validator, and the `Json<Value>`
REST handler that builds the config by hand)*: a custom `Deserialize` (or
hand-built config from a raw `Value`) changes REST's rejection envelope
for the wrong-*type* class — a wire change the slice does not frame — and
ripples through every fixture naming `probability`; the diff adds a dozen
branches under the exact-100% gate to buy an invariant the two-caller
world already has (`Chaos::set` is called from exactly two places — the
two transports — and `src/config.rs` has no chaos path; stdio bridges to
the same MCP handler). The newtype is the right design *when a third
constructor path appears* (a chaos config file), and 0025 records that
trigger. `Chaos::set` itself stays total — validation lives at the
handler, before the effect; putting it inside `set` would return
`Result` from an effect function and blur the refuse-before-effect
ordering the acceptance pins.

**R2 — The wait timeout: one default, one cap-check, one
supplied-vs-omitted match, beside the constant MCP already imports**
*(both seats converge on the shape; the location is simplicity's, and the
deletion is the point)*.

`src/api.rs` gains `DEFAULT_WAIT_TIMEOUT_MS` (5000 — named once; today the
default is hardcoded twice) and `checked_wait_timeout_ms(ms) ->
Result<Duration, String>` beside `MAX_AWAIT_TIMEOUT_MS` (`src/mcp.rs`
already imports that constant — the shared-home precedent). REST's
`AwaitQuery::timeout()` becomes a delegation mapped through
`bad_request`; its refusal text is byte-identical to 0018's. MCP's
indicted `unwrap_or(5000)` becomes the explicit match — `None =>
DEFAULT_WAIT_TIMEOUT_MS`, `Some(v)` unparseable → the shared
supplied-malformed sentence — and the wait tool's input schema gains
`"default": 5000`, which REST's OpenAPI already publishes. The
supplied-malformed refusal maps to the same `isError` envelope the cap
refusal already has in the same tool.

*Cost of the losing side (robustness's `src/validation.rs` owning the
rules and the constants)*: a new file kept at literal 100% of lines,
branches and functions forever, holding a rule two files away from the
type it constrains, and moving `MAX_AWAIT_TIMEOUT_MS` — touching the
shape 0018 cites — to buy nothing the two existing homes do not already
give. Both homes *are* the cross-transport import points.

**R3 — `null` is supplied: it refuses** *(simplicity wins; robustness's
null == omission is refused)*.

The MCP match is `None => default, Some(v) => v.as_u64().ok_or(refusal)`:
`Value::Null.as_u64()` is `None`, so an explicit `"timeout_ms": null`
falls into the supplied arm and refuses — with no special case. This is
also the tree's existing answer for `delay_ms: null`, which the sibling
staging refuses today; adopting null == omission for `timeout_ms` would
fork null semantics inside one tool surface, and applied to `delay_ms`
(as robustness's position explicitly does) it would flip a *current*
refusal into a silent default — the disease the issue is titled after,
returning through the null door.

*Cost of the losing side (robustness's null == omission)*: one extra
`Some(Value::Null) => default` arm in every parser, a second null
vocabulary inside `swarmail_set_chaos`/`wait_for_email`, and template-style
callers keep getting silent 5000 ms for a value they explicitly sent.
Under this ruling those callers get a named refusal and must omit the key
— a visible behavior change, recorded in 0025, and the point of the
slice. Scope note, pinned in 0025: `ChaosConfig`'s own `Option<ChaosRule>`
fields keep serde's typed null→None (an explicit `{"data": null}` clears
that event's rule) — the field's declared type carrying a typed
instruction, not a malformed scalar being reinterpreted. REST's query wire
has no null type: the literal string `null` is supplied-malformed and
keeps its parse-time 400.

**R4 — The refusal *text* is shared; the refusal *envelope* stays each
tool's own** *(both seats agree the envelopes stay transport-specific;
this rules the tree's two existing MCP envelopes into the slice instead of
"fixing" them)*.

REST: range refusals → `400` `{"error": …}` via `bad_request`; malformed
JSON → 400 and a well-formed body with a wrong-typed field → 422, both
the extractor's own pre-existing rejection (the envelope of every
endpoint; the slice does not unify it). MCP `swarmail_set_chaos`:
argument refusals keep the JSON-RPC `error` object — 0018's tested shape,
pinned by `set_chaos_refuses_every_malformed_wire_value_by_name`, which
runs against the new validator owner *unchanged*. MCP
`swarmail_wait_for_email`: timeout refusals — the cap *and* the new
supplied-malformed refusal — keep the `isError` tool-result shape its cap
test pins. *Cost of the losing shape (one MCP envelope for all argument
failures)*: rewriting six decided 0018 assertions and erasing the
protocol-level vs tool-level distinction for no acceptance line.

**R5 — REST answers two statuses for a bad probability, and that is
accepted** *(simplicity's priced risk wins; robustness's single-status
path is refused with R1)*.

Through the domain layer, REST refuses 101–255 with `400` + the exact
sentence MCP refuses with. The wrong-*type* class — string, fraction,
`null`, and `256+` (which does not fit the field's `u8`) — is refused by
the extractor with its own message and status. Both wires reject every
out-of-domain value; the texts are byte-identical through 255.

*Cost of the losing side*: single-status would need the `Json<Value>`
rewrite (R1) or a `u16` widening of `probability` so `256+` reaches the
validator — laundering a status code through the type system and moving
the accident boundary to 65536. The residual is named in 0025: REST
answers the type layer's message for `256+`, MCP answers the domain
sentence for all of `101+`. Accurate on both wires; byte-identical only
through 255. If the operator wants one envelope for the whole band, that
is its own decision.

**R6 — Refusal before effect, proven by the issue's own trick**
*(both seats; grounded in 0015 and 0018, with 0002's complement stated)*.

Validation is pure and precedes every effect. REST validates before
`Chaos::set` — the active rules are unchanged *by construction, not by
rollback*. MCP refuses before `store.wait_for` — a rejected wait
registers no waiter. The discriminating probe: a malformed `timeout_ms`
against an inbox that *already holds a matching message* must answer the
refusal immediately — if the parser still fell back, the wait's first
scan would answer Found. Deterministic, no sleeping, no cold path. The
GET-after-rejected-PUT assertion (REST) and the successful set's
read-back (MCP) pin "rules unchanged" from the wire.

**R7 — `count`/`limit`/`since_ms` are named, not fixed** *(both seats
converge; the spec draws the boundary explicitly)*.

`arg_usize(...).unwrap_or(1).max(1)` (`swarmail_get_latest_email`'s
count), `unwrap_or(50)` (`swarmail_search_emails`' limit), `unwrap_or(1)`
(the wait's count) and `as_i64` swallowing a malformed `since_ms` carry
the same disease on MCP, while REST's urlencoded parse 400s the same
inputs. The acceptance names only probability 0–100 and timeout
0–60000 ms; each added parameter costs a full two-transport matrix under
the exact gate. 0025 records the map; the next slice inherits a map, not
a mystery.

**R8 — Docs: two edits, two pins, and zero llms.txt/README edits**
*(simplicity's measurement + robustness's pin, placed per 0020 rule 1)*.

`src/static/openapi.json`: `ChaosRule.probability` gains
`minimum`/`maximum`, and the PUT documents its `400` (mirroring the await
endpoint's 400 shape). The MCP `swarmail_wait_for_email` schema gains
`"default": 5000` on `timeout_ms`. `llms.txt` and the README already
promise the domain REST now honors — REST catches up to its own published
contract; zero edits there. The pins are inline `#[cfg(test)]` tests
(0020 rule 1 — branch-stable, deterministic, measured; a `tests/` test
cannot read the `pub(crate)` constants without a forbidden seam): one in
`src/api.rs` asserting openapi.json's bounds equal
`PROBABILITY_MAX`/`DEFAULT_WAIT_TIMEOUT_MS`/`MAX_AWAIT_TIMEOUT_MS`, one
in `src/mcp.rs` asserting the TOOLS schema does the same.
`tests/mcp.rs`'s existing served-schema pin (literal style) gains the
default. The fourth copy of each number can drift only by failing CI.

## Residual — named, not fixed

- REST `≥256` answers the extractor's type rejection (422, serde's
  message), not the domain sentence — R5, recorded in 0025.
- MCP `count`/`limit`/`since_ms` silently default supplied malformed
  values — R7, the file:line map in 0025.
- A third chaos writer would bypass the call-site validators — 0025's
  trigger clause names the move (into `Chaos::set` or a constructor-only
  newtype) that makes the invariant structural.
- `ChaosConfig`'s own `Option<ChaosRule>` null handling is unchanged —
  R3's scope note.

## Acceptance Criteria

1. **One chaos validator, both wires.** `ChaosRule::validate` (in
   `src/chaos.rs`) is the only probability rule in the crate; REST's
   `set_chaos` and MCP's `swarmail_set_chaos` both call it. Probability
   `0` and `100` are accepted on both transports; `101`, `150` and `255`
   are refused on both with the same sentence — REST `400`
   `{"error": "probability must be an integer between 0 and 100"}`, MCP
   the JSON-RPC error message with the same text. A rejected PUT/set
   leaves the active rules exactly as they were: REST — `GET
   /api/v1/chaos` byte-identical before and after; MCP — the next
   successful set's read-back shows the refused rule never stored.
2. **One wait-timeout check, both wires.** `checked_wait_timeout_ms`
   (in `src/api.rs`) is the only cap check; `AwaitQuery::timeout()` and
   the MCP wait tool both use it. `60000` is accepted on both (REST 200
   satisfied mid-wait, MCP parks — the 0018 legs); `60001` refused on
   both with the byte-identical cap text (existing tests now flow
   through the shared function); `0` is legal and answers an immediate
   `Timeout`/408 — never a clamp.
3. **Supplied malformed refuses; omission keeps its default.** MCP
   `swarmail_wait_for_email` with a supplied `-1`, `"invalid"`, `1.5`
   or `null` `timeout_ms` answers an `isError` tool result
   `"timeout_ms must be an integer between 0 and 60000"` — never a
   silent 5000 ms. An *omitted* `timeout_ms` parks with the documented
   default (proven by the 0018 shutdown-race shape: not answered
   instantly, resolved by `stop()` with the fixed `shutting down`
   text) — never refused. REST's `timeout_ms=-1`/`invalid` keep their
   parse-time 400; the default `5000` is named once
   (`DEFAULT_WAIT_TIMEOUT_MS`) and used by both parsers.
4. **A rejected wait registers no waiter.** The malformed-timeout probe
   against an inbox that already contains a matching message answers
   the refusal immediately on both transports — never Found; the seeded
   mail is untouched and still listable.
5. **A rejected chaos configuration leaves the active rules unchanged.**
   A refused PUT/set applies nothing — including a partial config whose
   *other* slots are valid (the walk sees all four events) — and the
   previously armed rules keep firing exactly as before.
6. **One owner per sentence and per number.** The probability sentence
   lives in `src/chaos.rs`; the timeout sentences live beside
   `MAX_AWAIT_TIMEOUT_MS` in `src/api.rs`; no second copy of any bound
   or sentence exists anywhere in the crate. The inline pins hold
   openapi.json (`probability` minimum/maximum, the PUT 400, the await
   `timeout_ms` default/minimum/maximum) and the MCP tool schemas
   (`timeout_ms` gains `"default": 5000`) equal to the constants;
   `tests/mcp.rs`'s served-schema pin includes the default. Drift fails
   CI.
7. **Envelopes unchanged in kind.** REST keeps `{"error": …}` + status
   codes (400 domain / 400-syntax+422-type extractor); MCP keeps the
   JSON-RPC `error` object for `set_chaos` arguments and `isError` tool
   results for wait refusals. No new envelope vocabulary anywhere.
8. **The gates and the boundary hold at HEAD.** `cargo fmt --all --
   --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test
   --workspace --locked`, `brokkr compile --bundle bundles/self`,
   `bash scripts/coverage-gate.sh` (exact 100/100/100 — every new arm
   of the validators and the match executed by the matrix) and
   `cargo deny check licenses` are green; zero new files under `src/`,
   zero new dependencies; `src/store.rs` and `src/persist.rs` at zero
   diff; no `coverage(off)`.
