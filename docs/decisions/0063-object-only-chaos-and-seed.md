# 0063 — Refuse positional REST seed and chaos bodies before mutation

- **Status:** proposed — awaiting the operator; not implementation authority.
- **Date:** 2026-10-24.
- **Scope:** Epic #192, issue #181, implementation of
  [chaos-contract](../../specs/chaos-contract/spec.md), AC6 / tasks T2–T3.
- **Proposes amendment to:** accepted
  [0029](0029-supplied-values-are-validated-wherever-they-reach-a-wire.md)
  §3 and §6's explicit prohibition of custom REST chaos deserialization,
  narrowly for object-shape admission of `ChaosConfig` and `ChaosRule`.
- **Clarifies:** object-shaped seed admission under accepted
  [0048](0048-rest-webhook-validation.md) §2; preserves its native data-error
  400 versus chaos's extractor plain-text 422 distinction.
- **Preserves:** 0001–0005, 0025's probability domain and scalar-null versus
  typed-event-null distinction, domain validation before swap, and existing
  diagnostics for requests already refused. No accepted decision is edited.

## Observed defect

Real HTTP probes at design HEAD `ff155e62c0ce37046aff03c8906b7779352f3b00`
(the integrated `d681ccc` baseline plus the design commit) confirmed that
serde-derived structs accept positional arrays despite `deny_unknown_fields`.
A short array refusal is insufficient evidence of object-only decoding.

Each chaos probe first installed this nonempty configuration by real PUT:

```json
{"rcpt":{"probability":0,"error":"451 prior","delay_ms":2},"data":{"probability":100,"error":"554 prior","delay_ms":3}}
```

GET before each probe confirmed both rules. Observed requests and effects:

| Method and path | JSON body | Observed result and immediate GET witness |
| --- | --- | --- |
| PUT `/api/v1/chaos` | `[]` | 200; all four event fields became null |
| PUT `/api/v1/chaos` | `[null]` | 200; all four event fields became null |
| PUT `/api/v1/chaos` | `[null,null,null,null]` | 200; all four event fields became null |
| PUT `/api/v1/chaos` | `{"data":[0,"451 kept",0]}` | 200; data became probability 0, error `451 kept`, delay 0; rcpt became null |
| POST `/api/v1/inboxes/shape181/seed` | `["sender@example.test","recipient@example.test","positional subject","positional body","<p>positional html</p>"]` | 200; inserted a new ID with positional subject and HTML; exact message total increased from 1 to 2 |

The seed inbox already held identifiable keeper mail. Empty and one-string
seed arrays returned native JSON 400 with the existing `invalid length 1,
expected struct SeedRequest with 5 elements` diagnostic; immediate GETs
retained the original ID and total 1. The complete five-string array inserted
mail while retaining that keeper. The chaos 200 replies agreed with GET:
the clearing replies contained four null event fields, and the rule-array
reply contained the installed `data` rule. These are mutation witnesses,
not merely successful parsing or empty-to-empty comparisons.

## Why implementation stopped

The issue and committed design require object-only shape refusal. Accepted
0029 §3 explicitly says chaos keeps serde's own 422 rejection with **“no
custom deserializer”**, and §6 expressly rejects a custom deserializer for
REST chaos events. Accepted 0048 authorizes the webhook shape reader and
local seed/webhook 400 mapping, but explicitly preserves chaos's existing
mapping; it does not expressly amend that chaos implementation constraint.

The current `Json<ChaosConfig>` extractor's derived visitor accepts sequences
for the config and its rules; the handler therefore receives validated
ordinary structs and performs the swap. There is no separate handwritten
sequence-acceptance branch to delete. `deny_unknown_fields` already exists
on both types. Type-owned domain validators cannot distinguish an array from
an equivalent object once deserialization has finished.

A map-only reader would directly meet the requested domain but would require
the explicit exception below. Moving to raw bytes or `Json<Value>` with a
shape precheck would instead add a second parse/admission path and require
choosing new array-refusal wording and path behavior. Neither is silently
inferred as authority to override 0029 or the issue's unchanged-diagnostic
constraint. Existing acceptance of arrays is evidence of the gap, not a
ruling to retain it. The design's T3/AC6 stop rule applies.

## Proposed operator ruling

1. Native REST seed and chaos config bodies must be objects. A non-null
   chaos event rule must also be an object. Reject positional arrays at
   each of those locations before construction, insertion or config swap,
   including empty, short and otherwise complete valid sequences.
2. Permit the smallest object-only admission readers at these existing
   owners. Specifically amend 0029's no-custom-deserializer constraint only
   for chaos config/rule shape admission; keep derived map-field decoding
   or equivalent delegation for already-supported object inputs. Do not
   introduce a generic validation framework, a third writer or a new domain
   validator. This proposal does not prescribe an implementation before
   operator acceptance.
3. Preserve every existing refusal sentence, path and envelope for inputs
   that already refused. The newly refused sequence class has no existing
   refusal wording to preserve; its diagnostics must name the expected
   object/struct and the existing serde path where applicable, without
   echoing supplied values. Seed keeps native JSON 400 and chaos keeps
   extractor plain-text 422. No global normalization or webhook policy
   change is authorized.
4. Keep valid object omissions/defaults, event-level null clears, scalar
   null refusals, 0/100 acceptance, native 400 for representable 101–255,
   extractor 422 for negative/overflow/missing/wrong-type probability and
   MCP's existing -32602 messages. Rejected inputs preserve populated state.
5. After acceptance, resume all six issue criteria, add permanent real-wire
   regressions for the observed arrays, pin exact existing paths, and prove
   the complete house gates and required independent four-provider panel.
   Proposal status alone does not authorize the exception.

## Evidence and delivery status

Baseline targeted suites passed: API 64, MCP 44, webhook 27 tests; the
existing shared-domain OpenAPI unit test also passed. Temporary real-server
observational probes used `tests/common/mod.rs` and
`cargo test --locked --test api issue181_complete_positional_array_probe -- --nocapture`.
Their success means the observations completed, not that AC6 passed.
The temporary test was removed and the original test source restored
byte-for-byte after recording observations.

Run-local receipts and the exact temporary reproducer are under
`.forge/issue181/`: `baseline-wire.log`, `baseline-openapi.log`,
`positional-probe.log` and `api-with-probe.rs`. The request/response/state
facts necessary for operator review are retained above; ignored receipts
are supplementary and are not fabricated journal history.

Implementation is **blocked**, not complete. OpenAPI, production code and
permanent tests remain unchanged. The six acceptance criteria remain open;
the complete house gate inventory and independent review have not run on
an implemented candidate. The implementation seat does not launch later
phases. No push, merge, tag or publication occurred.
