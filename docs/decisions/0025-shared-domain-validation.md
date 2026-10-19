# 0025 — Shared domain validation: a supplied malformed value refuses, an omitted optional value keeps its default

- **Status:** accepted (operator, 2026-10-14 — reviewed by the completion run's full panel and the slice-2 run's panel; no security residual)
- **Date:** 2026-10-14
- **Supersedes:** none
- **Amends:** [0018](0018-shutdown-observed-waits-and-explicit-bounds.md)
  §3 — in place, by the 0019 precedent: 0018 carries the dated `Amended
  by` marker (a landing task of this slice), and this document carries
  the amendment. §3 ruled that *wire bounds are refusals, never clamps*;
  this decision extends the refusal posture from the *bound* to the
  *supplied malformed value*, and makes one shared validator — not two
  parallel checks — the owner of the rule both transports enforce.
- **Preserves:** [0014](0014-one-ordered-mutation-boundary.md) and
  [0015](0015-boundary-failure-contract.md) — validation is pure and
  sits above the boundary; a refusal happens before any boundary effect,
  and `src/store.rs`/`src/persist.rs` end at zero diff.
  [0001](0001-synchronous-insert-lossless.md)/[0002](0002-awaits-subscribe-before-checking.md)/[0003](0003-oldest-only-pruning.md)
  are untouched: no wait is registered by a refused call, nothing is
  accepted that is not validated, no rule is evicted or half-swapped.
- **Issue:** [#32 slice 2](https://github.com/feedback-loop-ai/swarmail/issues/32)

## Context

The two transports apply different validation rules. Wire probes against
the current server:

| Input | REST | MCP |
|---|---|---|
| Chaos probability `150` | `PUT /api/v1/chaos` returns 200 and stores 150 (`src/api.rs` `set_chaos`; `ChaosRule.probability: u8` — the silent-100 behaviour of `Chaos::check`'s `random_range(0..100) < p as u32`) | `swarmail_set_chaos` refuses it |
| Wait timeout `-1` | HTTP 400 (the query parse refuses before the handler) | Accepted; silently defaults to 5000 ms |
| Wait timeout `"invalid"` | HTTP 400 | Accepted; silently defaults to 5000 ms |

The silent MCP fallback is established by the parser:
`a.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(5000)` treats a
*supplied malformed* value exactly like *omission* (`src/mcp.rs`, the
wait tool). The strict side is already the decided posture: 0018 §3 made
MCP `swarmail_set_chaos` refuse absent, non-integer, over-`u8` and >100
probabilities by name — and the published contracts already promise it.
`src/static/llms.txt` says "`probability` must be an integer 0..=100";
`src/static/openapi.json`'s await `timeout_ms` pins
`default: 5000, minimum: 0, maximum: 60000`. REST and the MCP timeout
parser are the laggards: the docs were ahead of the code, and openapi's
`ChaosRule` publishes a *boundless* `probability` with no 400 on the
PUT — the receipt that "kept in sync by convention" already failed once.

Swarmail is test infrastructure: a `200` that stored a rule the caller
did not ask for, or a tool result that silently waited a timeout the
caller did not choose, is the control-plane version of a `250` for mail
that was never stored. The accepted set and the refusal semantics of
both transports must be one domain, or the mock lies to one of its
callers — greenly.

## Decision

*(Amended by [0029](0029-supplied-values-are-validated-wherever-they-reach-a-wire.md),
accepted 2026-10-15: §1's residual list is closed, §4's envelope posture is
extended, not rewritten, and §5's "refused call registers no waiter" is
carried to every new refusal.)*

1. **One domain validator per rule, owned by the type it constrains;
   both transports call it.** `ChaosRule::validate` and
   `ChaosConfig::validate` live in `src/chaos.rs` (the rule's home);
   the wait cap-check lives beside the `MAX_AWAIT_TIMEOUT_MS` constant
   that `src/mcp.rs` already imports (`src/api.rs`). No
   `src/validation.rs`, no newtype, no envelope rewrite: the transports
   keep their own parse layers and their own envelopes, and the *domain*
   layer is the one shared thing. `ChaosConfig::validate` walks all four
   events — a REST PUT replaces the whole config, so a partial check
   would accept a bad rule in any slot it forgot to look at.
2. **Supplied vs omitted is the distinction, and `unwrap_or` may never
   touch a supplied value.** A key that is *absent* takes the documented
   default; a key that is *present but unparseable* is refused, naming
   the field and the accepted range. JSON `null` is **supplied** (the
   key is present in the object; `Value::Null.as_u64()` is `None`), so
   `timeout_ms: null` and `probability: null` refuse — the same answer
   the sibling `delay_ms` staging already gives an explicit `null`
   today. REST's query wire has no null type: the literal string `null`
   is a supplied malformed value and keeps its parse-time 400. The one
   place this ruling does **not** reach: `ChaosConfig`'s own
   `Option<ChaosRule>` fields keep serde's typed null→None (an explicit
   `{"data": null}` clears that event's rule) — that is the field's
   declared type carrying a typed instruction, not a malformed scalar
   being silently reinterpreted.
3. **The default and the bounds each have exactly one owner.**
   `DEFAULT_WAIT_TIMEOUT_MS` (5000) is named once and used by both
   parsers; `checked_wait_timeout_ms` is the one cap check, its refusal
   text byte-identical to 0018's ("timeout_ms must be at most 60000");
   the supplied-malformed sentence ("timeout_ms must be an integer
   between 0 and 60000") and the probability sentence ("probability must
   be an integer between 0 and 100") are `pub(crate)` constants beside
   their validators, reused by the transport staging so no second copy
   of any sentence exists. The MCP `swarmail_wait_for_email` schema
   gains the `"default": 5000` that REST's OpenAPI already publishes.
4. **The refusal *text* is shared; the refusal *envelope* stays each
   tool's own.** REST answers range refusals with `400` + `{"error": …}`
   (its existing envelope, via `bad_request`); wrong-*type* bodies keep
   the extractor's rejection — malformed JSON 400, a well-formed body
   with a wrong-typed field 422 — the pre-existing envelope of every
   endpoint, unchanged here. MCP `swarmail_set_chaos` keeps answering
   argument refusals with the JSON-RPC `error` object (0018's tested
   shape); MCP `swarmail_wait_for_email` keeps answering its timeout
   refusals — the cap *and* the new supplied-malformed refusal — as
   `isError` tool results, the envelope its cap refusal already has.
   Unifying the MCP envelopes would rewrite decided 0018 assertions for
   no acceptance line.
5. **Refusal precedes every effect.** Chaos validates before the swap:
   a rejected PUT never calls `Chaos::set`, so the active rules are
   unchanged *by construction, not by rollback*. Waits refuse before
   subscribe: 0018 §3's refusal-before-subscribe extends from the
   over-cap value to the supplied-malformed one — a rejected wait
   registers no waiter, and the probe that proves it is a malformed
   `timeout_ms` against an inbox that already holds a matching message
   (the issue's own trick): the refusal answers immediately, where a
   silent default would have answered Found.
6. **The contract documents are pinned to the constants by test, not
   codegen.** openapi.json gains the `probability` minimum/maximum and
   documents the PUT's 400; inline `#[cfg(test)]` pins (decision 0020
   rule 1 — branch-stable, deterministic, measured) assert the served
   openapi.json and MCP schemas equal `PROBABILITY_MAX`,
   `DEFAULT_WAIT_TIMEOUT_MS` and `MAX_AWAIT_TIMEOUT_MS`. The fourth
   copy of each number can drift only by failing CI. A generated-schema
   approach is refused: it costs a build dependency to fix a drift two
   test files close.

## Consequences

- REST probability 101–255 now answers `400` with the shared sentence —
  a loud breaking change for any client that relied on storing `150`
  under a contract the published schema never stated and the MCP
  transport never allowed. That is the slice doing its job.
- MCP `swarmail_wait_for_email` with a supplied `-1`, `"invalid"`,
  `1.5` or `null` timeout now answers an `isError` result naming the
  domain, instead of silently waiting 5000 ms. Callers that filled
  every field of a template with explicit `null`s now get a named
  refusal and must omit the key — the visible cost of this ruling, and
  the point of it.
- `timeout_ms: 0` and `probability: 0/100` stay legal on both wires;
  the poles are pinned, never clamped.
- **Residual — the same disease, unframed.** `arg_usize(...).unwrap_or(1)
  .max(1)` (`swarmail_get_latest_email`'s count), `unwrap_or(50)`
  (`swarmail_search_emails`' limit), `unwrap_or(1)` (the wait's count)
  and `as_i64` swallowing a malformed `since_ms` silently default
  supplied malformed values on MCP, while REST's urlencoded parse 400s
  the same inputs. The framing's acceptance names only probability
  0–100 and timeout 0–60000 ms; each added parameter costs a full
  two-transport matrix under the exact coverage gate, so this slice
  does not take them. The next slice inherits this map, not a mystery;
  the shape to generalize is the supplied-vs-omitted match.
- **Residual — the REST ≥256 envelope.** `ChaosRule.probability` stays
  `u8`, so REST answers `256+` with the extractor's type rejection
  (422, serde's message) rather than the domain sentence: the field
  width owns that band, an accident of representation, not a domain
  rule. MCP answers `256` with the domain sentence (its staging refuses
  the `u8` overflow by name). Both wires *reject* every out-of-domain
  value; the texts are byte-identical through 255. Widening the field
  to `u16` would move the accident to 65536 and launder a status code
  through the type system; if the operator wants one envelope for the
  whole ≥256 band, that is its own decision.
- **Trigger clause — a third writer.** Only the two transports install
  chaos rules today (`Chaos::set` is called from the REST handler and
  the MCP handler and nowhere else; there is no chaos config file). If
  a third writer appears, the validator moves from the call sites into
  the setters — `Chaos::set` itself, or a constructor-only newtype — so
  the invariant becomes structural rather than ordered-at-every-door.
  Until then, a `Result`-returning `Chaos::set` would churn every call
  site and blur the refuse-before-effect ordering §5 pins.

## Refused

- **Two parallel checks kept in sync by convention** — the drift the
  issue is titled after; the boundless openapi.json is the receipt that
  the convention already failed once.
- **A `src/validation.rs` module** owning the rules and the constants:
  a new file kept at literal 100% forever, holding a rule two files
  away from the type it constrains, and moving `MAX_AWAIT_TIMEOUT_MS`
  — touching the shape 0018 cites — for zero acceptance lines. Both
  homes are already the cross-transport import points.
- **`Probability`/`WaitTimeoutMs` newtypes with constructor-only
  construction and a `Json<Value>` REST handler.** A custom
  `Deserialize` (or hand-built config from a raw `Value`) changes
  REST's rejection envelope for the wrong-*type* class — a wire change
  the slice does not frame — ripples through every fixture naming
  `probability`, and adds a dozen branches under the exact-100% gate
  to buy an invariant the two-caller world already has. The slash in
  "one domain type/validator" is satisfied by the validator half.
- **Validation inside `Chaos::set`** — a failable effect function
  blurs the refuse-before-effect ordering §5 pins and churns every
  call site; see the §Consequences trigger clause.
- **Unifying the MCP refusal envelopes** (all-`isError`, or
  all-JSON-RPC-error) — decided 0018 test shapes; the text is what is
  shared, not the envelope.
- **Chaos persistence, a chaos config file, count/limit/since_ms
  fixes, and any edit to `src/store.rs`/`src/persist.rs`** for this
  slice — the first two are new features, the third is recorded as a
  residual with its file:line map, and the boundary ends at zero diff.

## Later-decision markers — 2026-10-19 (issue #144)

Appended 2026-10-19 by the issue #144 provenance audit, after the operator's
2026-10-19 ruling accepting 0034–0050 and 0055. The text above, the dated
0029 marker included, stays as the accepted historical ruling; each marker
names a later record with that record's own verb and scope, and nothing
above is rewritten by it.

- *(Amended by [0048](0048-rest-webhook-validation.md) (accepted
  2026-10-19): §4, for seed `POST` and webhook `PUT` only. Their bodies are
  closed (seed's five keys; webhook `url`, `inbox`, `secret`, `ca_pem`),
  optional webhook strings default only on omission while `null` and wrong
  types refuse, and every JSON data error there is a value-free HTTP `400`
  with the native envelope, including former `422` refusals. Other routes
  keep their mapping, chaos's `422` included (0048 §2).)*
