# Implementation plan — chaos contract

## Technical context

One Rust crate; axum REST, JSON-RPC MCP, serde request decoding, hand-maintained
OpenAPI 3.1, rustls webhook delivery, and real-server integration tests.
Start at the integrated `d681ccc` rollup with the existing #178 behavior and
#179 helpers. No dependency or build-matrix expansion is needed.

The requested plan-template file is absent from `.specify/templates/`;
this plan supplies the house-required technical context, constitution check,
structure, research, data model, contracts and quickstart directly. It is
governed by [the spec](spec.md), with six acceptance criteria and its explicit
operator-review stop condition.

## Constitution check

Accepted 0025/0029 own the shared validators, scalar omission/null distinction,
REST 400/422 split and plane-specific envelopes. Accepted 0044/0048 own
closed inputs, value-free seed/webhook data 400s, diagnostic redaction and CA
loadability. Publication and stronger tests require no semantic amendment.
Do not add a Decision declaration for an existing authority.

Frozen synchronous acceptance, subscribe-before-scan and oldest-only pruning
stay untouched; extraction remains at ingest. Validation precedes insert or
swap, never rollback. No third chaos writer is introduced. Webhook validation
does not dial, take a delivery-cache lock, disable TLS verification or add
expiry policy. Existing best-effort dispatch and storage guarantees remain.

Use deterministic inline contract pins under 0020; real public-wire cases
belong in existing integration suites. Do not move measured code/tests to
evade coverage. All changes remain subject to the single house gate inventory.

## Project structure and intended file changes

| File | Work |
| --- | --- |
| `src/static/openapi.json` | Publish bounds, complete grammar, required probability, closed objects, null-capable events, and the operation's exact 400/422 distinctions |
| `src/api.rs` | Extend existing inline OpenAPI constant/clause pins; no handler rewrite planned |
| `tests/api.rs` | Extend REST probability, served-document, seed/chaos shape and exact-path tests with per-row populated-state comparisons |
| `tests/mcp.rs` | Add negative probability; strengthen clear/container/typed-argument tables and exact refusal messages with immediate state comparisons |
| `tests/webhook.rs` | Add PEM echo/GET byte equality and shape/path rows with complete installed-state witnesses and retained credential-delivery proof |
| `tests/common/mod.rs` | Reuse #179 transport/seed helpers; edit only if an indispensable small missing helper is demonstrated |
| `src/chaos.rs`, existing request readers in `src/api.rs`, `src/mcp.rs`, `src/webhook.rs` | Inspect owners; any production boundary repair is conditional on first reproducing a violation and satisfying the spec's stop rule |

No new production module or test binary is planned. The chief's commit contains
only `spec.md`, `plan.md` and `tasks.md` in this directory. The untracked
intake framing and ignored panel positions are evidence, not design artifacts.

## Research and implementation sequence

1. Inventory the existing tests named below and run their baseline. Inspect
   the accepted parser/error owners. For root, missing-field and unknown-key
   errors, capture existing wire responses and extract the exact path form
   into literal expectations; preserve wording.
2. Before designing any repair, add real-wire complete positional-array probes
   to the owning integration tests. Probe all-four-slot chaos configs, complete
   event rules and five-string seeds, alongside empty/short arrays. Install
   state that silent acceptance would visibly change. Save any failing input,
   response and before/after state in ignored run evidence.
3. If a shape violation is reproduced, first seek a compatible repair at its
   existing admission owner, deleting unintended acceptance where possible.
   Do not introduce a generic visitor/validator framework or convert chaos to
   seed/webhook's JSON error mapping. Stop and record an operator proposal if
   preserving the existing diagnostics and 0029's no-custom-deserializer
   ruling cannot be demonstrated. Never continue by deleting the requirement.
4. Update OpenAPI and its measured pins together. Use
   `MAX_DELAY_MS`, `MAX_REPLY_LINE_OCTETS` and `PROBABILITY_MAX` for numeric
   drift checks, literal grammar clauses for description checks, and exact
   operation-local response lookups. For OpenAPI 3.1, express each nullable
   event as `anyOf` with a ChaosRule reference and `{"type":"null"}`;
   assert both alternatives rather than relying only on prose.
5. Extend existing wire tables with local tuple expectations. Make every
   refused mutation prove unchanged state immediately; reinstall the known
   chaos baseline before each row. Add successful controls separately so a
   legal mutation cannot invalidate a later refusal witness.
6. Add the generated PEM diagnostic test, complete the exact-path/shape matrix,
   then run targeted suites and the entire verification chain. Repair the
   owning artifacts if new evidence changes the plan. No requirement is
   discharged merely because the structural design validator passes.

## Data model and state witnesses

There is no stored-schema migration or new data model. ChaosRule retains
`u8 probability`, optional string error and `u64 delay_ms`; ChaosConfig
retains four optional events. REST replaces the full config after validation;
MCP changes one event under its existing lock after staging.

Use at least two distinguishable event rules at probability 0/100 for state
witnesses, with different valid error/delay fields. Compare complete GET JSON
and exact success statuses, not only the changed field.

Seed identifiable mail into each isolated destructive-test inbox before the
matrix; for MCP clear specifically use `default`, because malformed values
could otherwise silently target it. Retain IDs and full readable contents/raw
bytes plus exact count. Compare after every row; a count alone misses replacement.

Webhook refusal tests reuse `installed(&srv)` to compare all four target
fields, including secrets and credential-bearing URLs, alongside real HTTP
responses. A redacted GET cannot prove those values survived. Reuse
`install_prior` and `assert_prior_delivers` for the existing real-sink
credential/secret proof. Do not add a mock delivery path.

## Contracts and proof ownership

| Criteria | Existing test homes and proof |
| --- | --- |
| AC1 | `src/api.rs::tests::the_served_openapi_publishes_the_shared_domain` for schema/constants/clauses; existing `tests/api.rs` OpenAPI HTTP GET test for the served payload |
| AC2 | `tests/api.rs::chaos_probability_out_of_domain_is_refused_and_leaves_the_rules_unchanged` and `chaos_wrong_typed_probability_keeps_the_extractor_envelope_and_the_rules`; existing `tests/mcp.rs` probability/domain tables |
| AC3 | `tests/webhook.rs` generated CA helpers (`mint_cert`/`unrelated_ca_pem`) and diagnostic assertions; compare original PEM bytes in PUT and GET |
| AC4 | `tests/mcp.rs::a_wrong_typed_argument_refuses_by_name` and `a_malformed_arguments_container_refuses_before_any_effect`; seed before the matrix and remove redundant empty-state assertions |
| AC5 | `tests/api.rs::seed_and_webhook_data_errors_are_local_400s`, chaos extractor tables and `tests/webhook.rs::webhook_unknown_and_null_fields_preserve_configuration`; literal exact paths plus unchanged refusal wording |
| AC6 | The same API/MCP/webhook suites, including complete positional-array probes, valid-first/invalid-later entries, complete snapshots and legal default/null-clear controls |

Implement exact-path checks by matching the entire path segment with its
delimiter or the full existing stable diagnostic prefix. Never calculate
expected paths from the response or just search for a leaf name. Root failures
that have no field path are pinned by their existing root form. Keep line/column
formatting outside the newly pinned contract. Seed/webhook diagnostics stay
value-free even when malformed fields contain credential-like sentinels.

The current OpenAPI unit test reads `include_str!`, despite its name; it
does not alone prove HTTP serving. Extend/reuse the real GET test to compare
the parsed served document with the embedded pinned document, or assert the
same relevant schemas and chaos PUT responses through that real endpoint.

## Quickstart and verification

Run the focused real-server suites after the changed cases:

```bash
cargo test --locked --test api --test mcp --test webhook
cargo test --locked --lib the_served_openapi_publishes_the_shared_domain
```

Then execute the entire current
[house inventory](../../docs/house-rules.md#gates--part-of-every-change)
against the final implementation candidate. That link is the authoritative
list; do not substitute the targeted commands for any gate. Use locked Cargo
invocations, the candidate-local digest-matched brokkr, exact zero-missed
lines/branches/functions, coverage selftest, publish and keyless seal proofs,
and the license check. Keep the pinned verifier's shim checks, design selftest
and delivery-proof tests, plus its fail-closed native-tls absence proof:
a successful reverse lookup is a failure; only the explicit no-matching-package
result proves absence; any other tool error fails closed.

The verify seat runs offline, installs nothing, checks candidate shims and
executes the trusted pinned bundle copies. A local preflight is not the
hermetic verification verdict. CI-only chart/history/selftest, real Docker,
RustSec, release-rate and privileged port-80 checks remain CI evidence.

The engine, after implementation/verification, owns the requested four
independent review seats: DashScope Qwen Max correctness, Claude Opus security,
official DeepSeek Flash guarantees and DashScope GLM 5.3 tests. Failure of a
provider does not authorize fallback or DeepSeek Pro. Do not push, merge, tag
or publish.

## Risks

| Risk | Mitigation |
| --- | --- |
| Complete positional arrays may deserialize despite closed structs | Probe complete valid arrays first on real wires with populated state; retain failing evidence and stop for operator review if a compatible repair cannot preserve the rulings |
| Publishing a universal probability status would contradict 0025 | Pin 101..255 domain 400 separately from negative/overflow/missing/type 422 and MCP -32602 |
| Schema closure could accidentally prohibit event null clears | Use OpenAPI 3.1 null alternatives and positive real PUT controls; scalar null remains refused |
| Error-path tests could be brittle or vacuous | Use independent literal complete path segments and existing stable wording; deliberately review dependency drift, without pinning incidental line/column numbers |
| A refusal could already have destroyed state | Seed default mail; compare count, IDs and contents after each row; compare full multi-event configs and unredacted webhook state |
| Redacted diagnostics could conceal credential loss | Retain installed-state comparisons and existing authenticated real delivery proof |
| CA test could become time-dependent or broaden TLS policy | Generate loadable CA per run, compare bytes without handshake/expiry assumptions, reuse the existing TLS stack and keep verification enabled |
| Hand-maintained prose could drift from constants | Pin each numeric owner and each grammar clause, and prove the same document is served |
| Global config or default-inbox tests could interfere | Each matrix owns a fresh server and mailbox; no sleeps, global mutable test state or stochastic firing |
| Large proof scope could become architectural churn | Reuse the existing test files/helpers and validators; no new dependency, generalized runner or coverage denominator reduction |
| Green design validation could be mistaken for feature completion | Keep tasks unchecked until their implementation evidence exists; record preflight failures honestly; require all six criteria, house gates and independent review later |
