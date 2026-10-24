# Publish chaos constraints and complete validation proofs

Issue #181 (epic #192), findings 13.1–13.6. Design date: 2026-10-24.
Status: drafted; implementation, verification, review and external CI evidence remain pending.

## Feature and rationale

Make the served OpenAPI describe the chaos domain already enforced by the
server, and make existing real-protocol tests demonstrate the missing validation
cells and refusal-before-effect guarantees. Clients need accurate bounds and
statuses. Maintainers need tests that detect lost configuration, erased mail,
missing field diagnostics and altered public CA text, even when status codes
and successful TLS delivery look correct.

This is a documentary correction and stronger evidence for existing semantics.
It introduces no new accepted input, refusal, status, storage behavior or public
promise. No new decision is declared. Authorities are accepted
[0025](../../docs/decisions/0025-shared-domain-validation.md) (shared domain,
transport envelopes and schema pins),
[0029](../../docs/decisions/0029-supplied-values-are-validated-wherever-they-reach-a-wire.md)
(reply grammar, delay cap, argument staging and public CA text),
[0041](../../docs/decisions/0041-webhook-boundary.md) (credential redaction),
[0044](../../docs/decisions/0044-compat-validation.md) (null refusal and closed
arguments), and [0048](../../docs/decisions/0048-rest-webhook-validation.md)
(seed/webhook data-error 400 exception and diagnostic response shapes).
A discovered need to change these semantics requires explicit reframing and a
proposed decision; this design does not authorize such a change.

## Panel synthesis and rulings

Read both run-local positions, `simplicity.md` and `robustness.md`, under
`.brokkr/state/design/positions/`, and intake
`.brokkr/state/tasks/epic-192-next-181-luna-xhigh-del-6b91af9b.md`.
Triage already chose `engine`; retain its full design/review route.

| Question | Ruling and cost |
|---|---|
| Small repair versus broader hardening | Adopt both panels' five-file documentation/test scope and existing validation owners. Reject new validators, deserializers, schema generation, dependencies and envelope normalization. Cost: static schema duplication and transport differences remain, guarded by focused pins. |
| Status-only tests versus stateful refusal proof | Adopt robustness's complete configuration comparison after every refused request, with two populated events; simplicity also supports per-probe comparison. Reject a single terminal snapshot. Cost: more real GET calls; benefit: a later request cannot hide a partial replacement. |
| Re-seed before every destructive probe versus retain one fixture | Choose simplicity's one seed on a private server, with a nonzero count and retained identity established before and immediately after each probe. Reject robustness's repeated seeding as unnecessary once each prior refusal is checked. Cost: later probes depend on earlier assertions; a failure stops the sequence. No reseeding may hide deletion. |
| Reply grammar prose versus optional regex | Choose precise prose plus numeric schema constraints. Reject an added regex and a character-count substitute for the octet cap. Cost: OpenAPI-only validators cannot fully enforce the byte grammar; the server remains authoritative. Avoid introducing newline-anchor or Unicode mismatches. |
| Loose diagnostic matching versus locked exact fixtures | Adopt both panels' full-string assertions on the decoded native error or extractor text. Reject generic-prefix or field-substring checks and a general diagnostic parser. Cost: dependency wording changes require deliberate fixture review. |
| CA fidelity versus wider TLS policy | Adopt byte-faithful GET equality within the existing generated-CA HTTPS test; retain credential-redaction proofs. Reject PEM normalization, expiry/hostname policy at PUT, network preflight and a new round-trip DTO. Cost: GET is still diagnostic, not replayable configuration, and CA loadability cannot guarantee later delivery. |
| Robustness's wider failure analysis | Preserve its limits as scope constraints, not new implementation obligations: no durable webhook outbox, exactly-once delivery, indefinite mail retention, global resource bound or persisted chaos configuration is promised. Reject storage, SMTP and dispatcher refactors here; inherited best-effort notification loss remains. |

## User scenarios

1. A REST client reads OpenAPI and can distinguish a valid chaos rule, a
   domain refusal and an extraction refusal before interpreting the response.
   Schema pins and real REST/MCP matrix tests independently prove this slice.
2. An operator submits malformed configuration or a malformed clear request.
   The existing rules and already-stored message survive each refusal. Seeded
   state comparisons prove this independently of response wording.
3. An operator diagnoses webhook trust using GET. The public CA string is
   identical to the submitted string while credentials remain redacted. A real
   generated-CA HTTPS test proves fidelity and continued delivery separately.

## Acceptance Criteria

- **AC1 — Chaos schema (13.1):** Served `ChaosRule` and `ChaosConfig` have
  `additionalProperties: false`. `ChaosRule` requires `probability`, an integer
  with minimum 0 and maximum 100. `delay_ms` is an integer with minimum 0,
  maximum 300000 and omission default 0. All four event properties permit a
  rule or JSON null using OpenAPI 3.1 schema vocabulary; null clears an event.
  Optional omitted events in a REST replacement remain unset.
- **AC2 — Reply domain (13.1):** `error` documents a string of at most 510
  UTF-8 octets, no CR or LF, three leading ASCII digits with the first 4 or 5,
  and ASCII space as the fourth byte if any text follows the code. Bare `451`
  and `451 ` remain legal; Unicode text after the space remains legal within
  the byte limit. Omission uses the existing stage default; supplied null is
  refused. Do not encode `maxLength: 510` as the byte rule. Deterministic pins
  tie published bounds to `MAX_DELAY_MS` and `MAX_REPLY_LINE_OCTETS`.
- **AC3 — REST statuses (13.2):** OpenAPI's probability description and PUT
  responses agree: integers 101–255 yield native JSON 400; negative, omitted
  required probability, null, non-integer/wrong type and beyond-u8 values
  yield extractor plain-text 422. Preserve the existing explicit 422 response.
  Unknown event/rule keys also yield 422. Per-event null clearing is explicitly
  excluded from scalar-null refusal wording. Malformed JSON remains extractor
  400, media type 415 and body size 413. No runtime status normalization.
- **AC4 — Probability matrix (13.3):** Real REST and MCP tests explicitly name
  negative (`-1`) and omitted probability cells. REST proves 422/extractor text;
  MCP proves HTTP 200, JSON-RPC `error.code == -32602`, and exact message
  `probability must be an integer between 0 and 100`. MCP's existing omitted
  cell stays. Before each sequence install two distinguishable valid rules at
  probability 0; after EVERY rejection compare the entire GET configuration
  to its successful baseline snapshot. Retain existing wrong-type, null,
  overflow, domain-boundary, unknown-key, delay and grammar coverage, along
  with valid 0/100 and null-event controls. No mid-band chaos is executed.
- **AC5 — Public CA fidelity (13.4):** PUT the per-run generated valid CA in
  the existing HTTPS test, then GET `/api/v1/webhooks` and compare the decoded
  `ca_pem` string byte-for-byte, including line endings/trailing newline.
  Check GET still withholds the secret; preserve the existing URL-userinfo
  redaction proof. Real verified HTTPS delivery still succeeds. Do not trim,
  parse/re-emit or normalize the compared PEM, or claim full GET-to-PUT replay.
- **AC6 — Exact diagnostic paths (13.5):** For `{"to":7}`, null and numeric
  values of each optional seed string (`from`, `subject`, `text`, `html`), and
  chaos `data.error: null`, assert the exact actual error string against an
  independent expected literal/template established from locked real-server
  responses. Seed uses the decoded native JSON `error` with HTTP 400; chaos
  uses plain-text 422. Each expectation must contain the actual field path
  and expected/found JSON kinds. Generic Axum prefix alone must fail. Keep
  no-insert/no-arm and omission/null-clearing controls; do not expose values
  or credentials or synthesize expectations from the observed response.
- **AC7 — Destructive refusal (13.6):** On a private real server seed a known
  message in `default`, record its identity and establish exact count 1. Before
  and immediately after each numeric/null `swarmail_clear_inbox` probe verify
  count 1 and the same retained message. The refusal is HTTP 200/-32602 naming
  `inbox`. A clear followed by the correct error must fail the proof. Keep
  other malformed-tool probes valid with the now-nonempty baseline.
- **AC8 — Boundaries and evidence:** Preserve synchronous accept/store,
  subscribe-before-check, oldest-only pruning and ingest extraction. Add no
  dependency, allowlist change, runtime validation owner or policy bypass.
  Preserve recipe ancestors `450bef65c581b3a4791cf94dbc145912814bd6ed` and
  `9b15fda841f6ea304326df6df23adf6a3c2f77cb`. Implementation/verify must execute
  the complete [house-rules gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
  and fail-closed native-tls proof with candidate-local output and the pinned
  engine. Record missing external CI evidence explicitly; this design does
  not close #181 or certify implementation.

## Scope and honest limits

Expected implementation changes: `src/static/openapi.json`, deterministic
inline pins in `src/api.rs`, and existing tests in `tests/api.rs`, `tests/mcp.rs`
and `tests/webhook.rs`. No migration, new endpoint or new framework. Existing
runtime behavior may already pass new matrix/fidelity tests; that is additional
contract coverage, not a fabricated runtime bug fix. Stronger assertions need
an explicit negative control where feasible to demonstrate their sensitivity.
