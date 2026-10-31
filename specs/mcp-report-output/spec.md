# Complete structured MCP report output

Decision: 0105 — [Thirteen-tool structured output](../../docs/decisions/0105-mcp-report-output.md)

## Feature and rationale

Complete issue #153 on the current-main #152 integration: all thirteen tools
advertise useful output schemas and return structured successes on 2025-06-18
(June) and 2025-11-25 (November), while 2025-03-26 (March), including the
absent-version default, keeps its exact independently observed bytes.
The added deliverability read must describe the actual bounded report, not a
null schema or an unconstrained object. Keep the existing text block so a
consumer using only text can complete the same workflows.

Design baseline is 093ed64, following explicitly incomplete staging
dfa76635b2d1c75c07ba605c9a62c1cb88e1e98c. Local pre-#153 candidate
3a2eef948ed26392fe8ce2254f4b522964459a35 has tree
ce2e41e68d3a2d392db972b1699e0e35310943ff. These identities establish ancestry,
not completed delivery or independent approval. The supplied focused log records
17 passes and failures in revision_tool_annotations, structured::legacy_bytes
and structured::served_openapi_pins_and_negative_controls; it is historical
evidence, not a fresh test result.

The chief read both run-local positions and the resolved intake framing for
run follow-up-to-run-7244a9aa-which--5572d327. The issue153 packet's SHA256SUMS
verified: README.md b5cf69a9bfe6e704a57dfed5448b19dc92e97a75f9a80b475e8f6da11d6b958f;
operator-rulings.md b68560f6782247377e75e90698f95f33feef390cdbdb71ddd8c9a3cc0fbdfd9e.
The earlier 7244a9aa audit records no intake result; its final prose is not one.

This specification owns the follow-up. Historical
[#152 reconciliation](../mcp-version-negotiation/reconciliation.md) and
[#153 design](../mcp-structured-output/spec.md), with their dated plans, tasks
and receipts, remain unchanged. Proposed
[0082](../../docs/decisions/0082-mcp-structured-output.md) retains its historical
ledger clarification; its twelve-tool inventory describes that earlier candidate.
This proposal supplements that inventory for the current integration without
rewriting history or accepting 0079, 0080, 0082, 0085 or any other decision.

## Panel adjudication

| Question | Ruling and rejected position | Cost retained |
| --- | --- | --- |
| Production architecture | Adopt both positions' existing OUTPUT_SCHEMAS, tool_descriptors and text_result owners. Adopt simplicity's finite fragments; reject a generator, schema registry, new report service, SDK or production validator. | Literal duplication and explicit drift proofs remain maintenance work; modern JSON duplication increases response size. |
| Strict proof versus the smallest test change | Adopt robustness's exact name sets, independent expected payloads, all-branch schema checks and constraint mutations. Reject count-only repairs or updating goldens from this candidate. | More test code and independent baseline preparation are mandatory even though production changes are small. |
| All typed forms on the wire | Rule for simplicity's distinction: every tool needs real HTTP/binary-stdio success, while currently reserved Link coordinates and empty reports need independent typed serialization/schema witnesses. Reject robustness's suggestion that today's production collector can naturally emit every location/empty form. | There will be no production-binary receipt for unreachable forms. Label those cases typed-only; any later demand for such a receipt stays unmet, never silently waived. No production rule/flag/provider is added to manufacture it. |
| Historical documents | Reject robustness's request to repair the old #153 spec/plan/tasks and 0082 inventory in place. Preserve the operator's historical record; place the current normative obligations here and in 0105. | Readers follow a new scoped supplement. Implementation indexes it and updates current product descriptions without rewriting historical evidence. |
| OpenAPI and numeric fidelity | Adopt robustness's independent source pins and exact arithmetic, with simplicity's finite pattern vocabulary and existing facilities. Reject stripping an unsupported assertion or using f64 for both integer operands. | The finite evaluator gains deliberate, tested support for the emitted constraints; it still is not a general JSON Schema implementation. |
| Delivery evidence | Adopt both positions' independent baseline and protected review requirements. Reject treating a compile, local shipment, other candidate's settled member or upstream closure as this candidate's verdict. | Missing evidence and inherited HIGH security-relevant residuals can still prevent delivery after application tests pass. |

## Contract

### Revision, catalogue and representation

The exact catalogue and modern annotation mapping is:

| Tool | readOnlyHint | destructiveHint |
| --- | --- | --- |
| swarmail_search_emails | true | absent |
| swarmail_get_deliverability_report | true | absent |
| swarmail_get_email | true | absent |
| swarmail_get_latest_email | true | absent |
| swarmail_delete_email | false | true |
| swarmail_clear_inbox | false | true |
| swarmail_clear_all | false | true |
| swarmail_wait_for_email | true | absent |
| swarmail_extract_links | true | absent |
| swarmail_extract_codes | true | absent |
| swarmail_seed_email | false | true |
| swarmail_set_chaos | false | true |
| swarmail_clear_chaos | false | true |

For June and November each descriptor has a self-contained, reference-free
object-root outputSchema. Each success has exactly the existing JSON text block,
isError:false, and structuredContent equal to the captured original payload.
One exchange revision and one selected value supply both representations;
no parse-back, extra store read, repeated analysis, retry, rollback or validation
after effects. The existing twelve output contracts and their deliberate
OpenAPI projections remain in force: omitted optional Email/address strings
are not null, envelope_from is always present and nullable, received_ms is
signed, raw is excluded, historical code arrays have no invented fresh-ingest
cap, chaos emits every event key, and wait has separate closed found/timeout
arms. No transaction is invented between wait counts or chaos update/snapshot.

March descriptors, result/error bodies and text remain byte-identical to the
independent current-integration baseline. In particular, do not edit shared TOOLS
descriptions to fix modern wording and thereby alter March. The stale report
description claiming only the demo rule ships is corrected in modern projection
and current product documentation. Document the retained legacy wording as
historical; do not claim it accurately inventories current report rules.

### Deliverability schema

The report schema must match the actual served OpenAPI assertions after the
explicit projection below, and the existing typed serializers:

| Object/value | Required shape and assertions |
| --- | --- |
| Report | Closed object; required rule_set_id (string) and findings (array, maxItems 256). No const/enum for the example rule-set ID and no invented minimum string length. |
| Finding | Closed object; required rule_id, severity, location, fix_hint. rule_id is a string, minLength 1, pattern `^[!-~]+$`. fix_hint is a string with minLength 1, measured in characters. |
| Severity | String enum exactly info, warning, error. |
| Header location | Closed object; required kind, name, occurrence. kind is string const header. name is a string with exact OpenAPI pattern `^[\x21-\x39\x3b-\x40\x5b-\x7e]+$`. occurrence is explicit null or an integer 0..=18446744073709551615. Omission is invalid. |
| MIME location | Closed object; required kind, path. kind is string const mime_part; path is an array of integers 0..=18446744073709551615, including the empty root path. |
| Link location | Closed object; required kind, part_path, byte_offset. kind is string const link; part_path uses the same coordinate array; byte_offset is an integer 0..=18446744073709551615. No URL/body content is added. |

Location is oneOf those three exclusive tagged arms. Unknown members,
cross-variant mixtures, wrong tags, wrong severity, negative/fractional/overflow
coordinates, omitted occurrence and invalid string patterns refuse validation.
Do not narrow u64 coordinates to signed i64 or require a nonempty path.

The report producer separately enforces at most 65,536 compact JSON bytes,
canonical finding order and exact deduplication. These are behavioral contracts;
maxLength on an object/string cannot express encoded report bytes. Keep its
128 cache slots, four worker-held jobs, immutable selected mail, failure
classification and lazy analysis outside SMTP/ingest. Cache/job bounds do not
promise total RSS, latency or delivery success. Unknown authentication remains
not_evaluated, never a clean report. Extraction still occurs once at ingest.

### OpenAPI and finite validation

Pin independently inspected DeliverabilityReport, DeliverabilityFinding,
DeliverabilityLocation and DeliverabilitySeverity source structures before
reference resolution/projection, alongside the existing six component pins.
Resolve only the finite local OpenAPI references for this test comparison.
Emitted MCP schemas contain no references at all.

Projection may remove explicitly classified schema annotations (description,
title, examples, and the location discriminator whose dispatch is fully retained
by tagged oneOf); add explicit string type beside the three location constants.
Existing Email/address/chaos/wait projections remain explicit. Never recursively
drop arbitrary keys or assertions: properties' names are data. Do not change
REST constraints to make equality pass, drop requiredness/bounds/patterns, or
turn examples into assertions. Preserve legal chaos request defaults.

Extend the existing test-only evaluator for enum, maxItems, minLength, the two
inspected patterns, and full exact signed/unsigned integer bounds. Validate every
schema arm and keyword shape before testing a payload. Unknown keywords,
unsupported patterns, malformed schemas, invalid combinators and all references
continue to refuse, even in unused alternatives. minLength counts characters,
not UTF-8 bytes. Integral JSON floating values retain the existing integrality
semantics, with exact comparison to integer bounds; rounding must not admit
2^64. No saturating cast or narrowed bound is acceptable.

Independent positives and negatives cover each keyword and variant. Mutating
copied OpenAPI/schema/value expectations must detect dropped enum/requiredness/
closedness/pattern/maxItems, changed tag/wrapper, and numeric overflow. Recognition
of a keyword alone is not proof that its constraint is evaluated.

### Baseline and evidence classes

Keep tests/fixtures/mcp-structured-output/march.jsonl byte-for-byte, SHA-256
d2fafeebb5c245df9a53ee87456cc16f3373ecbb0d0875aa727be58e51838aa1, with its original
dc67f4a twelve-tool provenance. Add a distinctly named march-current-main.jsonl
captured from an independently built real pre-#153 #152 implementation that
contains the report tool. The available 3a2eef948ed26392fe8ce2254f4b522964459a35
is a local integration-baseline candidate, not a protected #152 acceptance receipt.

Capture actual HTTP bodies and real binary stdio lines using identical controlled
inputs, explicit March and omitted-version selection. Record source commit/tree,
build invocation, executable digest before/after, request identities, fixture
hash and exact allowed substitutions. Normalize only known generated IDs/times
after occurrence/location assertions; preserve escaping, key order and whitespace.
Do not parse/reserialize expected bytes or generate them from the changed #153
binary with fields removed. If authentic capture is unavailable, leave the
criterion unmet and report the missing evidence.

Real modern workflows require all thirteen successful tool names over HTTP and
binary stdio for each modern revision. Assert equal transcript lengths and case
identities before comparing rows. Obtain the seed ID using text alone. Compare
modern text to corresponding March strings and structuredContent to parsed text
plus independently specified expected values (for report cases, complete expected
findings and the served REST payload). Equality between two outputs of the same
formatter is insufficient.

Real stored-byte fixtures exercise the currently produced Header/MimePart forms,
header occurrence/null and info/warning/error demo severities. Independent typed
serialization fixtures exercise Link, empty report, empty paths and every u64
coordinate boundary. Existing cfg(test) owner seams may prove report failure or
typed shapes through real adapters, but must be labeled as fixture-driven.
No fabricated provider becomes production behavior. Exercise real empty/nonempty
search/extraction and wait found/timeout, historical Email fields and chaos
states. Returned links are untrusted data and are never followed.

### Errors, preserved guarantees and scope

Missing mail, failed/unavailable analysis, storage refusal, rate refusal and
invalid wait timeout retain their existing isError text; malformed arguments/
envelopes retain JSON-RPC errors. Neither has structured success content. Normal
wait timeout remains a successful timeout arm. Notifications stay effectful and
silent. Preserve lifecycle fences, mixed-revision isolation, March batches,
modern array refusal, correlation, 10 MiB bridge cap and C1–C6 deviations.
Negotiated dates and structured output do not certify full MCP conformance.

Preserve synchronous insert/queryability before SMTP 250, subscribe-before-scan
including resubscription, oldest-only pruning and ingest extraction (0001–0005).
Selected snapshots can outlive deletion; new reads select anew. Reply loss after
commit is uncertain execution, never automatic replay or rollback. Webhooks
remain best effort; memory-mode process death loses state. Structured JSON does
not make untrusted mail instructions trustworthy.

No new dependency, production module, public knob, storage/report evaluator,
background service, transport rewrite, schema engine, tool removal, raised bound,
coverage exclusion or license widening. Repair a further concrete integration
defect only within these owners and contracts; otherwise record it explicitly.

## Acceptance Criteria

- **AC1 — Catalogue and revisions:** Real tools/list on both modern revisions
  equals the exact thirteen-name/annotation table, with valid non-null object
  schemas and exact schema-name coverage. Mixed clients retain independent
  selection; March/omitted descriptors match the independent baseline.
- **AC2 — Report contract:** The closed report/finding/three-location schema and
  severity enum satisfy every assertion above. Typed positives cover each form,
  null/empty paths and zero/u64::MAX; negatives reject omissions, extras,
  variant mixtures, illegal names, empty IDs/hints, 257 findings and invalid
  coordinates. 256 findings and all legal bounds are accepted.
- **AC3 — Finite evaluator:** Selftests exercise each emitted assertion, all
  branches, malformed schemas and unsupported references/keywords/patterns.
  Exact numeric controls include 2^53 neighbors, i64::MAX, u64::MAX and 2^64,
  both integer encodings and integral floating JSON values; fractional/negative
  coordinates fail. Ambiguous oneOf fails; character length differs from bytes.
- **AC4 — OpenAPI:** Independent pins of the existing six and four report
  components match the actual served source before explicit projection.
  Complete projected structures equal MCP schemas; independent constraint
  mutations fail. No REST assertion is removed or changed to achieve equality.
- **AC5 — Wire success:** All thirteen tools succeed over HTTP and binary stdio
  in June and November, with exact successful-name sets, case identities and
  transcript lengths. Text-only workflows work; modern text equals March and
  structuredContent equals independently expected actual payloads. All success,
  empty, wait, omission/null, chaos and report evidence classes above are covered
  and distinguished honestly.
- **AC6 — Legacy baseline:** The new thirteen-tool fixture has independent
  #152 source/executable/capture provenance and exact March/default HTTP/stdio
  comparisons for successes and errors. The historical twelve-tool fixture
  and historical specs/0082 clarification are unchanged. No self-generated
  expected transcript substitutes for capture.
- **AC7 — Refusals and costs:** Report missing/invalid/not-found/unavailable/
  failed cases and existing storage/rate/shutdown/envelope/notification cases
  retain taxonomy, text, silence and no structured success. Real bridge witnesses
  preserve below-cap success, over-cap refusal, March-fit/modern-overflow,
  correlation and subsequent usability without replay, truncation or cap changes.
- **AC8 — Application invariants:** Existing real-protocol and deterministic
  owner witnesses for 0001–0005, report limits/order/cache/cancellation/deadlines,
  storage, wait, chaos, TLS/webhooks and #152 behavior pass unchanged or with
  narrowly justified additional coverage. No guarantee, report bound or rule
  semantics is weakened.
- **AC9 — Documentation and local delivery:** Modern report descriptions state
  current rule scope and revision-qualified output accurately; legacy bytes
  remain unchanged. Implementation indexes 0105 as proposed, records each
  acceptance witness/gap, commits unsigned, and passes every local house gate
  on the clean candidate, exact coverage/selftest and fail-closed native-tls
  absence included. A pinned-layer edit requires a freshly compiled run.
- **AC10 — Independent ownership:** The engine obtains five separate protected
  gate-class review results under the selected route. All historical findings
  below remain preserved with original severity/classification; unknowns,
  indeterminate members and security holds are never converted into passes.
  Missing eval coordination, actual baseline/client/CI/seal evidence remains
  explicitly outstanding. No shipment or decision acceptance is claimed here.

## Review residuals and independent prerequisites

Retain the complete prior-runs.json unchanged (SHA-256
f510506bc7b5264510a6eec637ee9a482fcc6abd4497fd9ce7a205d09173f6be); this prose is
an index of findings, not a replacement for the original notes. Mixed review
53f33089 concerned 1734d8d and had four settled members and indeterminate tests:

- Correctness, guarantees and security: HIGH inherited review-integrity/result-
  attribution residual. Subsequent proof cases and five-member runtime behavior
  were not established. Independent called the same residual LOW; preserve both
  classifications without averaging or downgrading the HIGH findings.
- Missing live Terra/Opus invocation, boxed work-tool and engine-consumed result
  evidence: correctness HIGH, guarantees MEDIUM, independent INFO. Compilation
  and office resolution were structural evidence only; provider presence unknown.
- Independent INFO: effective route comparison was manual/receipt-backed rather
  than in CI/gates; mixed route/office documentation omissions. Nit: office-limit
  comment discrepancy despite correct assertions. Retain all these findings.
- Keep member-specific environment/gate limitations and prior receipt claims
  distinct: correctness's sandbox-denied tests/coverage/selftest/publish reruns,
  guarantees's receipt review without mutating gates, independent's denied
  cargo/engine execution and security's lack of shell. None is a new pass here.

Do not reproduce review attacks, forge results, broaden review write access,
repair/re-pin the engine, or infer a pin change from upstream closure.
The selected route retains Terra 6.1 intake, Opus 5.5 implementation, separate
Astra correctness and guarantees, official DeepSeek Flash security, Claude Fable
independent and DSH Spark GLM-5.3-Flash tests; tests effort remains not applicable.
Five roles are separate results, not five independent model families. Only the
engine collects their current candidate verdicts; security hold remains binding.

Standalone eval suites require authentic host-wide same-user nonoverlap plus one
exact captured executable checked by digest before/after. A worktree-local lock
or boxed process list cannot prove host-wide exclusion. Report missing
coordination and do not run those suites when it is absent. No paid trials.
Actual #152 baseline and independent protected review, paid #151 baseline,
real Claude Code/Codex/MCP Inspector/Claude Desktop matrix, CI and operator seal
remain separate merge prerequisites. Local protocol tests prove neither client
compatibility nor tokens-per-success / invalid-argument/tool-misuse improvement
(with turns-per-task context). No push, merge, publication, tag, closure or
decision acceptance is authorized.
