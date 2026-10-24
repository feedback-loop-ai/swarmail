# Publish the chaos contract and prove refusal before mutation

Created: 2026-10-24

Status: Draft for implementation

Scope: Epic #192, issue #181; `epic192/rollup-a` with accepted 0070
(design input HEAD `594dc37`).

## Feature and rationale

Implement the object-only admission authorized by accepted 0070 and finish the
REST/MCP validation matrix. Serde-derived closed structs currently accept
positional arrays: chaos arrays can clear installed rules, and a five-string
seed array can insert mail. Shape refusal must precede mail construction,
insertion and configuration replacement. Valid object behavior and every
existing refusal's wording, path and envelope remain compatible.

OpenAPI chaos bounds already shipped in PR #217 and are present in this base.
Retain that publication and prove it through both constant/clause pins and a
real HTTP GET. Finish the populated-state, exact-path and public CA fidelity
proofs; existing tests are assets to strengthen, not missing work to duplicate.

## Authority and design evidence

[0070](../../docs/decisions/0070-object-only-chaos-and-seed.md) was accepted by
the operator on 2026-10-24. It authorizes the narrow exception to 0029 §§3/6
for map-only admission of ChaosConfig and non-null ChaosRule, and object-only
seed admission. Its historical blockage/proposal narrative is not a current
stop condition. This design implements that accepted semantic ruling; it
introduces no further ruling, proposed decision or acceptance.

Other accepted authorities are
[0025](../../docs/decisions/0025-shared-domain-validation.md),
[0029](../../docs/decisions/0029-supplied-values-are-validated-wherever-they-reach-a-wire.md),
[0041](../../docs/decisions/0041-webhook-boundary.md),
[0044](../../docs/decisions/0044-compat-validation.md), and
[0048](../../docs/decisions/0048-rest-webhook-validation.md).
Frozen [0001](../../docs/decisions/0001-synchronous-insert-lossless.md),
[0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md),
[0003](../../docs/decisions/0003-oldest-only-pruning.md), and ingest extraction
under [0005](../../docs/decisions/0005-extraction-at-ingest.md) are preserved.

Read both run-local positions at
`.brokkr/state/design/positions/{simplicity,robustness}.md` and the current
intake `.brokkr/state/tasks/epic-192-issue-181-publish-the-c-a8abe809.md`.
The issue body, prior outcomes and operator rulings in
`.forge/epic192/issue181/{README,operator-rulings}.md` both passed that
directory's SHA256SUMS. These ignored files remain evidence, not committed
artifacts. The committed criteria below stand without access to them.

## Clarification — 2026-10-24

Accepted 0070 supersedes this spec's earlier instruction to stop for operator
review of the precise chaos shape-reader exception. Production reader work
is now required. Already-shipped OpenAPI publication still needs proof; a
design commit or pre-existing passing suite does not discharge any criterion.

## Panel synthesis and rulings

Both positions agree on authority and the defect. There is no genuine dispute
about the input domain; the rulings below settle the cost tradeoffs rather
than inventing a compromise domain.

| Question | Ruling | Rejected alternative and cost |
| --- | --- | --- |
| Admission ownership | Adopt simplicity's owner-local readers at ChaosConfig, ChaosRule and SeedRequest, retaining derived map-field delegation and existing handlers/validators. Robustness concurs. | Reject a generic parser module, raw JSON preflight/reparse, typed MCP Args, newtypes, dependencies and setter redesign. Accept small private helper/literal duplication and hand-maintained OpenAPI. |
| Compatibility versus shortest visitor | Adopt both positions' requirement to preserve already-refused sequences as well as object/scalar diagnostics. Permit narrow private legacy diagnostic decoding, whose successful positional values can never reach a handler. | Reject unconditional visit_seq rejection or a deserialize_map shortcut if it changes old failures. The minimum-line implementation loses: extra private branches and exact-coverage tests are the necessary cost. |
| Proof strength versus fixture economy | Adopt robustness's immediate complete populated-state witness after every refusal; retain simplicity's existing test tables and transports. | Reject status-only, empty-state, count-only and final-read-only proofs. Pay for extra requests, snapshots and credential delivery checks; a small fixture is insufficient if destructive acceptance stays invisible. |
| CA fidelity and paths | Adopt generated loadable CA byte comparisons through PUT/GET, existing credential delivery proofs and independent literal complete diagnostic paths. | Reject new TLS policy, external preflight, new fidelity-only TLS fixture and response-derived/leaf-only expectations. Diagnostics do not promise future delivery; serde upgrades require deliberate compatibility review. |
| Failure boundaries | Adopt robustness's explicit preservation limits and simplicity's bounded implementation scope. | Reject outbox, persistence, scheduler, concurrency stress and extraction work. Existing suites remain required, but this change adds no durability, exactly-once, latency or session-wide chaos transaction promise. |

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
missing probability, wrong types/scalar null and closed-shape failures. Retain
the shipped correction distinguishing those bands; neither a universal 400 nor
a universal 422 claim is correct.

Extend `the_served_openapi_publishes_the_shared_domain` to pin numeric bounds
to the owning constants, required probability, both closures, null alternatives,
every grammar clause and both response descriptions/envelopes. A real HTTP GET
of `/openapi.json` must also prove that the published payload is the pinned
document. Deleting a required clause or bound must break the corresponding pin.

### AC2 — Both probability wires have complete refusal witnesses

Audit and complete the existing matrices for -1 and omitted probability on
both wires; reuse the already-present rows. Retain 0/100 success controls
and 101/255/256, fraction, string and scalar null refusals. Use each wire's
exact status/envelope from the table and the existing probability sentence
where applicable.

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
contents, including stored raw bytes through the existing raw endpoint.

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
| REST seed | Top-level empty/short and complete five-string positional arrays; array/object/null scalar fields; unknown keys | Exact original inbox count, IDs, contents and stored raw bytes; no extra inserted mail |
| REST webhooks | Object instead of list; array/scalar instead of target object; array/object/null instead of each string field; unknown keys | Complete installed list, including secrets and URL credentials hidden by diagnostics |
| REST chaos | Empty/short and complete positional config arrays; complete and malformed arrays at each of the four event-rule locations; array/object/null scalar fields; unknown event/rule keys | Complete nonempty multi-event config |
| MCP tools | Non-object arguments containers; array/object/null values at typed argument boundaries; unknown keys | Seeded default inbox for clear/seed refusals, complete chaos config for set_chaos refusals |

Permanently pin the 0070 witnesses: `[]`, `[null]`,
`[null,null,null,null]`, and `{"data":[0,"451 kept",0]}` for chaos;
`["sender@example.test","recipient@example.test","positional subject",
"positional body","<p>positional html</p>"]` for seed. Exercise complete
rule arrays at connect, mail_from, rcpt and data, plus already-refused short,
wrong-type and excess-element sequences. Before each refusal install at least
two distinguishable nonempty event rules or identifiable keeper mail. A short
array failure alone cannot prove object-only admission.

Successful admission at the three readers is map-only. Preserve original
struct names, field order, defaults, present_string behavior, duplicate and
unknown-field diagnostics, nested paths and existing error precedence. Use
raw request text for duplicate keys and order-sensitive fixtures so a JSON
value builder cannot erase the case. Capture existing failures before changing
readers, including short seed arrays' invalid-length diagnostic. The new
refusal for a formerly successful positional body names the expected
object/struct and applicable serde path without echoing supplied values.
Private diagnostic decoding may reproduce a legacy failure; it must never
pass a positional request to mail construction or a configuration handler.

Seed/webhook data failures retain value-free native JSON 400, chaos data
failures retain extractor 422, and MCP argument failures retain -32602.
Use a valid first webhook target followed by a malformed second target and
a valid earlier chaos event followed by a bad later event to expose partial
application. Pair webhook installed-state equality with existing real-sink
delivery evidence that the prior credentials and secret remain usable.
Retain positive omission/default, per-event null-clear, empty webhook-list
clear and valid object controls.

Implement the narrow reader exception under accepted 0070; do not request
acceptance again. Every existing refusal must retain its sentence, path and
envelope, while every formerly accepted positional body now refuses before
effects. A compatibility test failure remains implementation work, not
permission to weaken a pin or omit a row. A further semantic amendment is
outside this design's authority.

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

CI owns the house-listed chart/history/selftest, real Docker, network RustSec,
tag-only rate and privileged/port-80 platform evidence. Operator-only sealing
and live GitHub settings remain with the operator. No macOS, external-network
or namespace proof is added to local acceptance.

No store/persistence, SMTP/await/pruning/extraction, chaos setter, dispatcher,
dependency, license-allowlist, feature-flag or gate-policy changes are planned.
No push, merge, tag or publication is authorized. Author commits are unsigned;
operator acceptance and sealing remain separate actions.
