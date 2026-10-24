# Implementation plan — chaos contract

## Technical context

One Rust crate; axum REST, JSON-RPC MCP, serde request decoding, hand-maintained
OpenAPI 3.1, rustls webhook delivery, and real-server integration tests.
Start from current main plus #181's accepted 0070/spec/intake (design input
`dc32761`), including merged #197 and shipped OpenAPI chaos publication.
Reapply the checksum-verified five-file implementation from
`.forge/epic192/issue181/0001-Refuse-positional-JSON-request-bodies.patch`;
reconcile its API helper overlap with main instead of replacing main's tests.
No dependency or build-matrix expansion is needed.

The requested plan-template file is absent from `.specify/templates/`;
this plan supplies the house-required technical context, constitution check,
structure, research, data model, contracts and quickstart directly. It is
governed by [the spec](spec.md), with six acceptance criteria and the
operator-accepted scope of 0070.

## Constitution check

Accepted 0025/0029 own the shared validators, scalar omission/null distinction,
REST 400/422 split and plane-specific envelopes. Accepted 0044/0048 own
closed inputs, value-free seed/webhook data 400s, diagnostic redaction and CA
loadability. Accepted 0070 narrowly permits the required object-admission
readers, amending 0029 §§3/6; preserve all other semantics and diagnostics.
Cite 0070 as authority, without a new decision or editing accepted records.

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
| `src/static/openapi.json` | Retain shipped bounds, grammar, closures, null alternatives and operation-local 400/422 descriptions; edit only if the completed pins reveal a missing decided clause |
| `src/api.rs` | Implement owner-local SeedRequest object admission with legacy diagnostic compatibility; complete inline OpenAPI pins; preserve handler and BodyError mapping |
| `src/chaos.rs` | Implement object-only ChaosConfig/ChaosRule admission with derived-equivalent field decoding, unchanged serialization/domain validators and measured compatibility tests |
| `tests/api.rs` | Reconcile the supplied patch with #197: retain its tuple snapshot and named-query helpers, all query/state/precedence/documentation assertions; add explicit count witnesses and the REST probability, served-document, shape and exact-path proofs |
| `tests/mcp.rs` | Complete existing probability rows; strengthen clear/container/typed-argument tables and exact refusal messages with immediate state comparisons |
| `tests/webhook.rs` | Add PEM echo/GET byte equality and shape/path rows with complete installed-state witnesses and retained credential-delivery proof |
| `tests/common/mod.rs` | Reuse existing real transport/seed helpers without planned changes |
| `src/mcp.rs`, `src/webhook.rs` | Read as regression authorities; no production parser/dispatch changes are planned |

Expected implementation owners are exactly `src/api.rs`, `src/chaos.rs`,
`tests/api.rs`, `tests/mcp.rs` and `tests/webhook.rs`; the existing OpenAPI
file needs no edit unless a decided clause is absent. No new production
module or test binary is planned. The chief's commit contains only
`spec.md`, `plan.md` and `tasks.md` in this directory. Already-committed
intake framing and ignored panel positions supply evidence; they are not
changed or added by this design commit.

## Research and implementation sequence

1. Audit existing API/MCP/webhook tests and run the baseline. Capture literal
   old diagnostics before modifying readers: scalar/root, missing, duplicate,
   unknown, nested, short/wrong-type/excess sequence and order-sensitive
   cases. Use raw request text where field order or duplicate keys matter.
   Existing negative/missing probability rows and CA GET proof should be
   strengthened in place, not re-added under new names.
2. Add permanent real-HTTP regressions for 0070's documented positional
   bodies. Observe the expected baseline failures with populated state;
   distinguish these failures from already-refused compatibility controls.
3. Reapply the patch's owner-local readers described below and reconcile
   `tests/api.rs` manually at the helper overlap. Do not wholesale replace
   current files or use a blind three-way result. Retain main's tuple
   `inbox_snapshot` and adapt the patch's `.count` assertions to explicit
   count GETs. Keep `assert_named_query_refusal` and
   `assert_named_query_refusal_with_body` unchanged in behavior. Make shape
   regressions, literal legacy diagnostics and every #197 test pass. Keep
   typed Json extraction, seed's local 400 and chaos's extractor 422.
4. Audit the already-shipped OpenAPI against AC1 and complete measured pins
   using MAX_DELAY_MS, MAX_REPLY_LINE_OCTETS and PROBABILITY_MAX. Check both
   null alternatives, complete grammar clauses, response descriptions and
   required/closed object fields. Extend the real GET proof of that document.
5. Complete the remaining probability, clear, shape/path and CA echo tables.
   Compare full populated state immediately after each refused request.
   Keep legal object/default/null-clear controls separate from witnesses.
6. Run targeted suites and all house gates on the final implementation, commit
   the implementation before handoff, and leave independent review to the
   existing engine route. Repair spec/plan/tasks together if evidence changes
   the approach; do not infer feature completion from structural validation.

### Reader design and diagnostic compatibility

At each of ChaosConfig, ChaosRule and SeedRequest, successful complete REST
JSON extraction must come only from a map. Keep the public type, serialization,
validators and mutation boundary; use private owner-local field readers with derived decoding
or equivalent delegation. Retain serde's original struct name, field order,
closed keys, defaults and present_string behavior. Field helpers are wire
implementation details, not new domain models or persistent data.

A minimal visitor may delegate map decoding via serde's map access adapter.
Its dispatch must also preserve old scalar expectations and previously refused
sequence diagnostics. In particular, replacing deserialize_struct with
unconditional deserialize_map, or immediately returning an object error from
every visit_seq, can change legacy invalid-length/wrong-type/excess-element
errors. Preserve those through the smallest necessary private compatibility
reader. For a complete sequence, return the new expected-struct refusal.
For excess elements, the supplied borrowed-sequence probe leaves the next
JSON value unread and may return intermediate fields so serde_json emits
its existing trailing-element syntax refusal. This staging is private to
that parser boundary: only completed map admission may reach a handler.
It is not a general-purpose object-only API for arbitrary deserializers.
Test excess arrays over real HTTP at root and all four rule locations,
including nested/order cases; an intermediate visitor result is not evidence
of extractor success. This diagnostic staging performs no mail build,
ingest extraction, insertion or swap.
For previously refused inputs, preserve the original failure's precedence,
path and envelope; helper names must not leak. Baseline/after wire comparisons
are the proof, not the visual similarity of the visitor.

Use the current serde/serde_json dependencies. No Json<Value>, raw-body
preflight, serialize/reparse round trip, manual parallel domain validator or
general parser framework is planned. Any small reuse between owners must
actually reduce code while preserving the complete compatibility contract.
The implementation may choose the narrow delegation details based on measured
proof; it may not simplify by weakening old diagnostic assertions. New reader
branches need measured deterministic tests, with public proofs in tests/api.rs
and placement following 0020. Existing setters and extraction stay untouched.

### Current-main ordering and helper reconciliation

Retain `seed_inbox` ordering: `json_body(body, "seed")`, empty-domain
`parse_query`, `Store::offload(build_seed_email)`, awaited `insert_async`.
Retain `set_chaos`: complete typed extraction, validate every event, single
whole-config swap. No rollback or speculative mutation is allowed.

Main's `inbox_snapshot` returns `(message_list_json, raw_byte_vectors)`;
keep that signature and all #197 tuple consumers. Full list equality covers
IDs, order, total and records; raw vectors follow the same ID order. Add
explicit count GETs for #181's count witness before and after each refusal.
One server/inbox per matrix and no concurrent writers make these ordinary
GETs sufficient; no transactional snapshot endpoint is warranted.

Run the entire API suite, not only new test names. Specifically retain
`query_free_native_operations_refuse_by_name` and
`native_query_refusal_preserves_error_precedence`, with their existing
state and documentation checks. Add the mixed-defect AC6 seed control
(positional body plus bad query), and retain the domain-invalid object plus
bad-query control. Body-shape errors win first; query errors beat domain
construction. Preserve every other #197 route's strictness unchanged.

## Data model and state witnesses

There is no stored-schema migration or new domain model; private field readers
exist only during decoding. ChaosRule retains
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
| AC6 | The same API/MCP/webhook suites, including complete/excess positional arrays, valid-first/invalid-later entries, complete snapshots and legal controls; all #197 tests plus mixed-defect seed precedence |

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
cargo test --locked --test api query_free_native_operations_refuse_by_name
cargo test --locked --test api native_query_refusal_preserves_error_precedence
```

Then execute the entire current
[house inventory](../../docs/house-rules.md#gates--part-of-every-change)
against the final implementation candidate. That link is the authoritative
list; do not substitute the targeted commands for any gate. Use locked Cargo
invocations and `.forge/bin/brokkr compile --bundle bundles/self`. Before
using that engine, require SHA-256
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`,
also recorded in `bundles/self/tooling.json`. Require exact zero-missed
lines/branches/functions, coverage selftest, publish and keyless seal proofs,
and the license check. Keep the pinned verifier's shim checks, design selftest
and delivery-proof tests, plus its fail-closed native-tls absence proof:
a successful reverse lookup is a failure; only the explicit no-matching-package
result proves absence; any other tool error fails closed.

The verify seat runs offline, installs nothing, checks candidate shims and
executes the trusted pinned bundle copies. A local preflight is not the
hermetic verification verdict. CI-only chart/history/selftest, real Docker,
RustSec, release-rate and privileged port-80 checks remain CI evidence.

The engine, after implementation/verification, owns independent review via
this candidate's unchanged `bundles/` routing. Read the selected bundle and
its agent definitions rather than copying an older roster. Standard codex
uses Fable correctness, official Flash security and Sonnet tests; careful
and fast design/engine use Astra correctness, Flash security, Sonnet tests,
Astra guarantees and independent Fable. Retain required results, protected
review and hard security hold; no ad hoc fallback or DeepSeek Pro. Do not
push, merge, tag, publish, close issues or accept decisions.

## Preserved failure boundaries

- A final SMTP DATA 250 still follows committed/queryable mail. A crash after
  commit but before reply can cause a retry duplicate; memory-only storage is
  not durable. This reader repair adds no idempotency or storage transaction.
- Webhooks remain post-store and best-effort with bounded per-target work,
  finite retries and request timeout. Hung targets, lag, shutdown and exhausted
  retries do not justify an outbox or blocking insertion here.
- Oldest-only pruning stays inside ordered insertion. A malformed seed must
  never insert even transiently and evict a keeper. Continued valid churn can
  evict old mail; tests own isolated inboxes and an adequate cap.
- Chaos retains per-stage snapshots, not retroactive cancellation or one
  session-wide snapshot. DATA refusal still occurs before build/insert; bounded
  SMTP reads and ingest-time extraction remain unchanged.
- Waiters still subscribe before scanning, including resubscription. Parser
  validation moves under no store lock and adds no aggregate latency promise.
- Generated CA fixtures prove byte fidelity and loadability. They add no PUT
  expiry/hostname policy or future TLS success guarantee; verification stays on.

## Risks

| Risk | Mitigation |
| --- | --- |
| A shorter custom reader changes old errors or leaks helper names | Capture legacy wire diagnostics first; preserve original names, defaults and error precedence via narrow private delegation; assert scalar, short/excess sequence, duplicate, unknown and nested failures before/after |
| Sequence staging or parser drift admits a value | Prove the complete serde_json/axum extractor rejects every sequence, especially the excess-element branch relying on its outer parser check; keep adapters private, prohibit non-JSON reuse, renew real-HTTP compatibility/state proofs on parser changes |
| Old patch displaces merged #197 | Keep main's tuple helper and both query helpers; adapt only #181 callers, retain all #197 assertions and test body-before-query-before-construction precedence |
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
