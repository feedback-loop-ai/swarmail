# Publish the chaos contract and prove refusal before mutation

Created: 2026-10-24

Status: Draft for implementation

Scope: Epic #192, issue #181; integrated baseline `d681ccc`.

## Feature and rationale

Publish the already accepted chaos validation domain in the served OpenAPI
document and close the REST/MCP validation matrix with meaningful state
witnesses. A caller must be able to distinguish domain refusals from extractor
refusals, retain public CA bytes through diagnostics, and trust that refused
requests cannot insert mail, clear an inbox, or replace configuration.

The rollup already contains the original #178 implementation and #179 helpers.
The active #178 follow-up is docs-only. Reuse those changes. This design changes
documentation and regression evidence; it neither defines a new accepted input
domain nor authorizes an error-envelope change.

## Authority and design evidence

Accepted authorities are [0025](../../docs/decisions/0025-shared-domain-validation.md),
[0029](../../docs/decisions/0029-supplied-values-are-validated-wherever-they-reach-a-wire.md),
[0044](../../docs/decisions/0044-compat-validation.md), and
[0048](../../docs/decisions/0048-rest-webhook-validation.md). Frozen
[0001](../../docs/decisions/0001-synchronous-insert-lossless.md),
[0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md), and
[0003](../../docs/decisions/0003-oldest-only-pruning.md) remain unchanged.
No new semantic ruling is made, so no new decision is declared.

Both run-local positions were read:
`.brokkr/state/design/positions/simplicity.md` and
`.brokkr/state/design/positions/robustness.md`.
The legacy `.brokkr/state/tasks/` directory is absent; the intake result
identifies `specs/chaos-validation-matrix/framing.md`, which supplies the six
criteria below. This spec incorporates that framing without committing the
intake or panel evidence.

## Panel synthesis and rulings

The panel agrees on the accepted contract and identifies the same unverified
positional-array risk. There is no substantive disagreement about a new domain
to resolve. The choices below resolve the tradeoffs in proof scope explicitly.

| Question | Ruling | Rejected option and cost of the ruling |
| --- | --- | --- |
| Validation ownership | Adopt simplicity's existing type-owned validators, wire parsers, local test tables and #179 helpers; robustness concurs. | Reject schema generation, a shared validation framework, typed MCP Args, new dependencies and setter redesign. Small literal table duplication and hand-maintained OpenAPI remain. |
| Evidence of no mutation | Adopt robustness's complete populated-state comparison after every refusal, including secrets hidden by GET. Simplicity's reuse of existing tests is retained only with these stronger witnesses. | Reject error-only, empty-to-empty and end-of-loop-only evidence. More setup and real requests are the necessary cost; fewer assertions would miss destructive defaults and partial application. |
| CA fidelity | Adopt simplicity's generated loadable PEM and byte comparisons through PUT/GET; retain robustness's existing credential delivery proof for refused replacements. | Reject a new HTTPS scenario solely for fidelity, expiry checks, network preflight and replayable redacted configuration. Diagnostics do not establish future TLS success. |
| Exact paths | Adopt both positions' literal per-row paths and existing wording, including containing-object/root errors. | Reject field-name substring checks, response-derived expectations and error normalization. Dependency changes may require deliberate review of these pins. |
| Array-shaped structs | Adopt both positions' complete positional-array probes before any boundary repair. Robustness's requirement for an actual state witness rules out a docs-only shortcut if a decided shape is accepted. | Reject inferring object-only decoding from `deny_unknown_fields` or a short-array failure. An incompatible repair blocks implementation for operator review, rather than silently widening this issue. |
| Broader guarantees | Adopt robustness's preservation of acceptance, subscribe ordering, oldest-only pruning, ingest extraction and best-effort bounded webhook dispatch; adopt simplicity's scope limit. | Reject new crash recovery, outbox, scheduler, concurrency stress or parser projects here. Existing full-suite proofs remain required; this matrix establishes no new durability, latency or exactly-once promise. |

## Contract to publish and preserve

REST whole-config PUT validates all events before one swap. MCP stages and
validates before its atomic single-event update; it does not replace other
events or create a multi-operation transaction.

| Probability input | REST chaos PUT | MCP set_chaos |
| --- | --- | --- |
| Integer 0 through 100 | Accepted | Accepted |
| Integer 101 through 255 | HTTP 400, native JSON error | HTTP 200, JSON-RPC error -32602 |
| Negative integer, integer at least 256, fraction, string, scalar null, missing probability | HTTP 422, existing extractor plain text | HTTP 200, JSON-RPC error -32602 |

Both domain refusals and all MCP probability staging refusals retain exactly
`probability must be an integer between 0 and 100`. REST extractor messages
remain their existing serde/axum text. Malformed JSON retains its own syntax
refusal, not the probability-domain envelope.

A ChaosRule requires probability; omitted delay is zero, omitted error uses
the stage's default failure. Supplied scalar null is not omission. An omitted
event or explicit per-event null is unset in a REST replacement; explicit
`{"data":null}` remains legal. Empty/omitted MCP arguments retain their
tool-specific documented defaults. Webhook empty lists still clear.

The error string has no CR or LF, begins with three ASCII octets matching
`[45][0-9][0-9]`, and its fourth octet, if present, is a space. A bare code
such as `451` is legal. Its UTF-8 encoding is at most 510 octets before the
server appends CRLF. A character-count maximum alone cannot express this
byte limit. Delay is an integer from 0 through 300000 milliseconds.

## Acceptance Criteria

### AC1 — Published chaos contract is executable

The served OpenAPI 3.1 document publishes required probability with integer
0..100 bounds, integer delay with minimum 0 and maximum 300000, its omission
default, and every error-grammar clause above. Both ChaosRule and ChaosConfig
declare `additionalProperties: false`. Each event schema permits a rule or
null using OpenAPI 3.1 schema composition, preserving typed clears.

The chaos PUT operation itself documents native JSON 400 for representable
101..255 probabilities, invalid error grammar and excessive representable
delay, and extractor plain-text 422 for negative/unrepresentable probability,
missing probability, wrong types/scalar null and closed-shape failures. Remove
the misleading claim that every out-of-range probability gets 400; do not
replace it with an equally misleading universal 422 claim.

Extend `the_served_openapi_publishes_the_shared_domain` to pin numeric bounds
to the owning constants, required probability, both closures, null alternatives,
every grammar clause and both response descriptions/envelopes. A real HTTP GET
of `/openapi.json` must also prove that the published payload is the pinned
document. Deleting a required clause or bound must break the corresponding pin.

### AC2 — Both probability wires have complete refusal witnesses

Add REST -1 and omitted probability, and MCP -1, to the existing matrices.
Retain 0/100 success controls and 101/255/256, fraction, string, scalar null
and MCP omission refusals. Use each wire's exact status/envelope from the
table and the existing probability sentence where applicable.

Install a distinguishable nonempty configuration before each refusal, capture
the complete read-back, and compare it immediately after that refusal. Include
multiple events so replacement or loss of an unrelated MCP event is detected.
Accepted endpoints must retain their intended values without clamp, wrap or
default. Rules whose firing is exercised use only probability 0 or 100.

### AC3 — Public ca_pem survives diagnostic round-trip byte-for-byte

PUT a loadable per-run generated CA PEM to the real webhook endpoint. Retain
the submitted string and compare both the PUT echo and subsequent GET
`ca_pem` values with `as_bytes()` equality, including all whitespace and
newlines. Retain secret-null redaction and URL-userinfo removal assertions.
This is fidelity of the public field, not replayability of the diagnostic
object or proof of certificate validity, handshake success or future delivery.

### AC4 — Every refused clear faces mail it could destroy

Before the MCP clear refusal matrix, seed `default` on an isolated server
with identifiable content. Refuse wrong scalar/null/array/object inbox values,
unknown argument keys, and non-object arguments containers (array, scalar,
null). Pin HTTP 200, JSON-RPC -32602 and the exact existing relevant messages.
Immediately after each refusal, prove exact count plus identical IDs and
contents, including raw bytes where the existing helpers expose them.

An object is a valid arguments container; its unknown keys or wrong-typed
members are the refusal cases. Retain successful omitted/empty-arguments
default controls separately, after the refusal witness or on another server.

### AC5 — Data refusals pin exact existing paths and preserve wording

Seed, webhook and chaos matrix rows carry literal expected paths, bounded at
the path separator so a leaf-name occurrence cannot pass. Cover nested chaos
event/field paths, indexed webhook target/field paths, root shape failures,
unknown keys and missing-field locations. For example, a wrong-typed chaos
probability identifies `data.probability`; a second target's wrong CA field
identifies `[1].ca_pem`. Missing probability may identify its containing rule,
not the missing leaf. Record the actual accepted location form without
inventing one for root failures.

Pin status, envelope and existing diagnostic wording alongside the path;
source line/column decoration need not become a new contract. Keep seed and
webhook errors value-free, and retain credential/secret sentinels proving
non-disclosure. Do not change production error wording to satisfy a test or
derive expected paths from returned text. Syntax 400, media-type 415 and
body-size 413 controls remain distinct.

### AC6 — Wrong shapes refuse before insert or configuration replacement

Exercise the following applicable cases with real requests and an immediate,
nonempty before/after state witness for each added refusal:

| Boundary | Required shape cases | State that must survive |
| --- | --- | --- |
| REST seed | Top-level empty/short and complete five-string positional arrays; array/object/null scalar fields; unknown keys | Exact original inbox count, IDs and contents; no extra inserted mail |
| REST webhooks | Object instead of list; array/scalar instead of target object; array/object/null instead of each string field; unknown keys | Complete installed list, including secrets and URL credentials hidden by diagnostics |
| REST chaos | Empty/short and complete positional config arrays; array instead of event rule; array/object/null scalar fields; unknown event/rule keys | Complete nonempty multi-event config |
| MCP tools | Non-object arguments containers; array/object/null values at typed argument boundaries; unknown keys | Seeded default inbox for clear/seed refusals, complete chaos config for set_chaos refusals |

Probe otherwise valid complete positional values, including
`[null,null,null,null]` as a config and `[0,"451 kept",0]` as an event rule.
A short array refusing for missing fields does not prove object-only parsing.
Seed positional probes use all five valid strings in the struct's field order.

Seed/webhook data failures retain value-free native JSON 400, chaos data
failures retain extractor 422, and MCP argument failures retain -32602.
Use a valid first webhook target followed by a malformed second target and
a valid earlier chaos event followed by a bad later event to expose partial
application. Pair webhook installed-state equality with existing real-sink
delivery evidence that the prior credentials and secret remain usable.
Retain positive omission/default, per-event null-clear, empty webhook-list
clear and valid object controls.

If any probe succeeds against a decided refusal, record input, response and
state effect. A repair is allowed only if it preserves all accepted semantics
and diagnostics. If it requires a new interpretation, custom chaos
deserialization contrary to 0029, different wording/envelopes or an accepted
ruling amendment, stop implementation and record the precise proposed decision
for operator review. Do not omit the failing row, mark AC6 satisfied, or treat
current implementation behavior as authority.

## Delivery requirements and limits

All six criteria and every gate in the
[house inventory](../../docs/house-rules.md#gates--part-of-every-change)
must be proved on the implementation candidate, including real protocol
suites, exact zero-missed coverage, coverage selftest, publish/seal proofs and
fail-closed native-tls absence. Preserve the pinned verification chain and
distinguish CI-only evidence. Tests obey accepted 0020: isolated servers,
no mocks, no sleep synchronization, no probabilistic validity assertions and
no coverage exclusions.

Retain the full independent panel under
[0057](../../docs/decisions/0057-codex-delivery-recipes.md): DashScope Qwen Max
correctness, Claude Opus security, official DeepSeek Flash guarantees,
DashScope GLM 5.3 tests. No DeepSeek Pro or fallback. This design seat does not
launch later-phase reviewers or claim their outcomes.

No store/persistence, SMTP/await/pruning/extraction, chaos setter, dispatcher,
dependency, license-allowlist, feature-flag or gate-policy changes are planned.
No push, merge, tag or publication is authorized. Author commits are unsigned;
operator acceptance and sealing remain separate actions.
