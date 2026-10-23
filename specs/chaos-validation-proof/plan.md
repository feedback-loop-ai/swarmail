# Implementation plan — chaos validation proof

Implements [spec.md](spec.md); ordered proof obligations are in [tasks.md](tasks.md).
This is design material, not an implementation or gate receipt.

## Technical context

One Rust crate using the existing Axum/serde REST extraction, MCP JSON-RPC
staging, Tokio real servers and generated TLS fixtures. OpenAPI is static 3.1
JSON included by the API. No dependencies, build features or allowlist changes.
The checkout supplies `.specify/templates/spec-template.md` but no plan template;
this plan supplies the required technical context, constitution check, project
structure, research, data model, contracts and quickstart directly, following
existing repository plans without inheriting their historical signing policy.

## Constitution check

Accepted 0025/0029/0044/0048 already establish every behavior being published.
No semantic amendment is proposed; accepted decisions remain untouched.
0001–0003/0005 remain invariants. Validation still precedes any swap, insert,
clear or subscription. REST replaces a complete validated config; MCP stages
and sets one event. Do not move validation into new setters or normalize errors.
Tests use private real servers and wire calls, no mocks or sleeps as
synchronization. Use probability 0 sentinels and only 0/100 for executed chaos.
Inline schema pins remain measured under 0020; do not exclude code or lower
coverage. All local gates and external CI obligations in house rules remain
binding before delivery. No push, merge, tag, publication or issue closure.

Triage has selected the full `engine` lane. Retain the Luna xhigh implementation
recipe, GLM 5.3 test review, Flash-only DeepSeek route and protected review;
this seat does not redispatch providers or choose the next phase.

## Research and resolved questions

Both panel positions and the run's task framing were read against
`src/chaos.rs`, OpenAPI, existing schema pins and affected tests. Typed `u8`
extraction explains the 400/422 split; 422 already exists in the document.
Both chaos structs already deny unknown fields, probability lacks a default,
`error` uses supplied-string validation and delay defaults to zero. The schema
needs `required`, closed shapes, bounds and nullable event alternatives.
Choose `anyOf` containing the existing ChaosRule reference and `type: null`
for each event; do not use a nullable keyword from an older OpenAPI dialect.

The affected REST matrix only compares config after its loop; MCP's matrix
has omission but lacks negative probability and per-probe full-state proof.
The seed path checks can match Axum's fixed prefix, and the destructive MCP
witness starts empty. The generated-CA HTTPS fixture already owns the correct
fidelity test location. No runtime defect has been demonstrated by this seat.

The candidate-local engine was inspected at the task's exact absolute path:
`/home/valentin/source/feedback-loop-ai/swarmail-epic192-delivery/NEXT-181-luna1/.forge/bin/brokkr`.
Its SHA-256 is `e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`.
Recheck before subsequent engine use/provider dispatch; stop on absence or
mismatch, never substitute a PATH binary. The design's initial HEAD is
`9b15fda841f6ea304326df6df23adf6a3c2f77cb`, following recipe commit `450bef6`.

## Project structure and files

| File | Planned change and proof |
|---|---|
| `src/static/openapi.json` | Correct existing chaos components and PUT descriptions; no new component or runtime schema engine. |
| `src/api.rs` | Extend only `the_served_openapi_publishes_the_shared_domain` inline pins; compare probability/delay/octet constants, required fields, closure, null alternatives and accurate refusal descriptions. |
| `tests/api.rs` | Extend `chaos_wrong_typed_probability_keeps_the_extractor_envelope_and_the_rules`, `null_strings_refuse_on_rest_like_mcp` and `seed_and_webhook_data_errors_are_local_400s`. |
| `tests/mcp.rs` | Extend `set_chaos_refuses_every_malformed_wire_value_by_name` and `a_wrong_typed_argument_refuses_by_name`; use current real HTTP/RPC helpers. |
| `tests/webhook.rs` | Add GET fidelity/redaction assertions to `https_target_is_delivered_over_tls`; retain existing delivery and separate userinfo-redaction tests. |

Read-only owners: `src/chaos.rs`, REST/MCP handlers, seed string readers,
webhook validation/redaction and SMTP consumers. No new source/test files are
expected. Design commit contains exactly this directory's spec, plan and tasks;
positions remain gitignored evidence. No ADR/index edit is required.

## Implementation approach

1. Capture baseline behavior of the named tests and real error strings using
   the locked dependency tree. Record results honestly. Newly tested runtime
   behavior may already pass; schema gaps and weak assertions are distinct.
2. Update OpenAPI in place. State precise grammar in prose, not regex. Keep
   integer constraints machine-readable and disambiguate omission, scalar
   null refusal and nullable event clearing. Extend existing deterministic
   schema pins using independent expected shape/wording and owner constants.
   Retain the existing real served-document route test.
3. Add named negative and omitted cells to REST and negative to MCP. Install
   two distinct probability-0 event rules before each matrix, snapshot the
   complete config, and GET/compare after each refusal. Assert MCP error code
   and exact sentence. Preserve positive boundary/default/clear controls and
   existing malformed classes without introducing a shared matrix framework.
4. Replace target diagnostic substring checks with exact strings. Decode seed
   native JSON first and compare its `error`; compare chaos extractor text as
   text. Templates may insert known fixture field names and expected JSON kinds,
   but cannot parse the response to manufacture its own expected diagnostic.
   Include full dependency-owned wording/locations where returned. Keep wrong
   shape, syntax, media and size checks, with seed/webhook-local 400 mapping.
5. Seed one known default-inbox mail before malformed clear calls. Record ID,
   check count 1 and fetch/list that ID before and after each invalid clear.
   Other invalid tools must leave the same baseline intact. Never replenish
   mail between a destructive call and its postcondition check.
6. Reuse `mint_cert` and the current TLS sink. Preserve the submitted PEM for
   exact string comparison after PUT then GET, assert secret is null, then
   execute the existing SMTP/verified HTTPS exchange. Preserve userinfo-redaction
   tests without adding credentials to this TLS fixture or altering CA policy.
7. Validate assertion sensitivity in a disposable control where feasible:
   replacing a field-specific diagnostic with only the generic prefix must
   fail; clearing seeded mail before returning the correct error must fail.
   Document exact control substitutions and results. Restore all mutations;
   do not ship new production seams for these simple observable conditions.
8. Run focused suites, then all required local gates once on the final source.
   Retain failures and investigate without weakening the checks. Record source
   identity and external proof gaps for independent verify/review handoff.

## Data model and contracts

No persistent or runtime data model changes. ChaosRule remains required `u8`
probability, optional non-null-when-supplied string error and default-zero `u64`
delay, with shared domain validation. ChaosConfig has four optional rules;
REST omission or null leaves an event unset in the replacement. MCP updates
one named event and keeps its existing schema/clearing mechanism.

REST native domain 400 differs from plain-text extractor 422; seed/webhook
JSON data errors alone use the accepted local native 400 mapping. Webhook GET
keeps a verbatim public CA string and redacted secret/userinfo. Exact diagnostic
fixtures test current locked behavior, not a new promise that dependencies can
never change wording. No endpoint, serialization default or status changes.

## Quickstart and validation

Use this candidate's own `target/`; never point `CARGO_TARGET_DIR` at another
run. Focused commands (real protocols, no dependency installation):

```bash
cargo test --locked --lib the_served_openapi_publishes_the_shared_domain
cargo test --locked --test api
cargo test --locked --test mcp
cargo test --locked --test webhook
```

Then execute every gate in [the authoritative inventory](../../docs/house-rules.md#gates--part-of-every-change),
including exact coverage and selftest, publish/seal guard proofs, locked Rust
checks, license check and fail-closed native-tls absence. Use the digest-verified
absolute candidate engine for `compile --bundle bundles/self`. Verify uses
pinned scripts and checks dispatch-shim hashes; do not substitute edited shims.
Do not duplicate or abbreviate that inventory into a narrower gate list.

External evidence remains pending: Helm chart contract/history/bump selftest,
real-Docker persistence/context/volume smoke, network RustSec audit, real port-80
bridge exchange and applicable tag-only release-rate-5k proof. These are CI
obligations; do not trigger publication or fabricate receipts to close them.
Design-check proves artifact structure only, not AC implementation or delivery.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Documentation turns every out-of-range value into one status. | Pin 101–255 domain 400 separately from typed-extraction 422 classes; runtime unchanged. |
| Scalar null wording accidentally forbids event clearing. | Explicit OpenAPI 3.1 null alternatives and retained positive wire controls. |
| A regex or maxLength narrows valid reply text or permits injection. | Exact byte grammar prose, owner-constant pins and existing wire grammar/boundary tests; server is authoritative. |
| Correct error returned after partial mutation. | Two-event full-config snapshots and immediate per-probe checks; seeded count plus message identity for clear. |
| Seeded mail breaks old empty-store assumptions. | Private server, update terminal baseline assertions, keep malformed-tool legs individually verified. |
| Exact diagnostics churn on upgrades. | Independent literals from locked real responses; intentional review on dependency updates. Generic prefix cannot satisfy them. |
| CA normalization passes a TLS-only test. | Direct decoded-string equality including newlines, alongside real verified TLS and credential redaction. |
| Generated certificate invites expiry policy or time assertions. | Reuse per-run valid fixtures; no future-delivery or wall-clock validity promise. |
| Small schema diff encourages gate shortcuts or scope creep. | All house gates remain required; no new runtime work without evidence/reframing; no coverage exclusions. |
| A draft is reported as issue completion. | Report only drafted artifacts here; implementation, full verification, reviews and missing CI receipts remain explicit. |
