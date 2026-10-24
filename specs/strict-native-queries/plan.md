# Implementation plan — strict native queries

Implements [spec.md](spec.md) and proposed [0062](../../docs/decisions/0062-strict-native-queries.md). This is a design artifact, not an implementation or gate verdict.

## Technical context

One Rust crate using existing Axum RawQuery, serde/serde_urlencoded, Tokio, real HTTP/SMTP servers and SQLite fixtures. `api::ViewQuery {}` already denies unknown fields; `api::parse_query` produces the native JSON error. Both are accessible to compat.rs. No new dependencies, features, lockfile changes, persistent fields or configuration settings.

The checkout has a spec template but no `.specify/templates/plan-template.md`. This plan supplies the required technical context, constitution, structure, research, data model, contracts and quickstart, following established repository plans. Separate research/contracts/data-model files are unnecessary for this bounded change.

## Constitution check

Preserve accepted 0001–0005, 0011, 0014/0015/0024, 0025/0029, 0041, 0044, 0048 and 0049. Query admission precedes mutation and rendering/build work. Origin/Host middleware stays first. Body mappings stay local. Compatible extension under 0045 §8 does not authorize contradicting accepted rulings; 0062 remains proposed.

Use real HTTP/SMTP, isolated servers for global/configuration effects, per-test inboxes, ephemeral ports and owned temporary database paths. Follow 0020: public wire tests belong in existing integration suites, deterministic unit pins remain measured, and only concrete cold diagnostics justify sibling harnesses. No mocks, sleep synchronization, probabilistic chaos, fixed-date certificate validity assertions, global mutable fixtures, exclusions or reduced denominators.

Only the three spec artifacts and new proposed decision belong in this design commit. Implementation indexes 0062 as proposed. Never rewrite accepted decisions or claim acceptance; commit unsigned and never push.

## Research and resolved questions

- Read intake and both positions. The ten-row matrix is the explicit scope; no external issue contents were fetched.
- Router and handler inspection confirms all ten omit query parsing. Native delete-all resides in compat.rs.
- Thread/feed handlers already demonstrate the empty-shape pattern.
- Four configuration GET/DELETE handlers return bare Json today; they need fallible signatures and Ok wrapping.
- Seed captures `Result<Json<SeedRequest>, JsonRejection>` and calls json_body before its builder. BodyError already accepts ApiError.
- RunningServer.webhooks and Webhooks::current expose owned complete targets, with public fields. Compare fields or serialization locally; no new derive, seam, receiver or credential-exposing endpoint.
- `tests/compat.rs::a_blocked_delete_refuses_the_shim_deletes_with_503` uses `common::refusing_trigger` and stop/reopen. Extend this existing fixture.
- `served_openapi` and `documents` in tests/api.rs and existing inline OpenAPI tests support method-aware pins. Use independent expected method/key sets, not a generated runtime registry.

## Project structure and files

| Path | Planned ownership |
|---|---|
| `src/api.rs` | RawQuery and parser calls in nine handlers; broaden ViewQuery comment; four fallible signatures; existing inline contract pins if useful. |
| `src/compat.rs` | Shared empty parser in native delete_all only. |
| `tests/api.rs` | Wire matrix, exact mail/configuration preservation, seed/precedence and served contract proofs. |
| `tests/compat.rs` | Delete-all refusals, two-inbox controls, rejecting-trigger/restart proof and compat preservation. |
| `README.md`, `src/static/llms.txt`, `src/static/openapi.json` | Method-specific query/refusal contracts, migration, compatibility and seed precedence. |
| `docs/decisions/README.md` | Index 0062 as proposed during implementation. |

No production changes to store, persistence, webhook/chaos internals, SMTP, extraction, middleware, gates, bundle policy or manifests. Reuse test helpers; change shared helpers only for concrete duplication in these tests.

## Implementation approach

1. Add the focused destructive-selector real-wire regression first. Record its baseline failure using isolated data, so the existing bug cannot damage unrelated tests.
2. Add RawQuery to all ten signatures, before seed's body extractor. Use `let ViewQuery {} = parse_query(raw.0.as_deref().unwrap_or(""), "METHOD /route/template")?;`. In compat reuse api::ViewQuery and api::parse_query; do not relocate delete-all or duplicate parsing. Existing strict-route labels stay unchanged.
3. Parse after inbox admission on clear, after json_body on seed, and before all domain work elsewhere. Preserve successful statements, offloading and mutation calls. Wrap configuration successes in Ok and return ApiResult<Json<...>>.
4. Broaden only ViewQuery's explanatory comment. No type rename, custom extractor or map scanner; existing libraries retain decoding and escaping ownership.
5. Add the proof fixtures below and run relevant suites without relaxing established validation.
6. Update all three public contract surfaces together; add method-specific pins and index 0062 as proposed.
7. Execute the authoritative full gate inventory and pinned verifier obligations. Record unavailable tools/failures honestly; do not install replacements, reduce scope or infer all-green from targeted tests.

## Data model

No persisted schema or migration. Stored mail, inbox names, counters, ChaosConfig and WebhookTarget are unchanged. Request staging only gains the existing empty ViewQuery. Full target configuration remains internal; public GET/PUT echoes still redact secret and URL userinfo and retain CA material.

## Contracts

The spec's method matrix and precedence table are normative. OpenAPI models each method separately. Absence of query parameters cannot alone express rejection: descriptions explicitly state unknown-key 400 and response definitions include it. Retain path parameters and existing success/404/503 and seed 413/415 responses as applicable.

Labels use templates, not actual paths or URIs. Test method, route, decoded key and absence of a sentinel value without matching the full serde sentence. Empty-key refusal uses the same parser. Do not prescribe a winning bad key or a new malformed-percent rule.

## Proof design

### Wire matrix and mail

Use a small method table for status/envelope/label checks and separate readable fixtures for effects. Every row gets meaningful selector, typo, empty value and no-query/empty-query controls. Add focused encoded/bare/repeated/empty-key probes, not a cross-product of unrelated defects.

Seed two inboxes with distinct IDs/content. Capture complete message JSON by ID and raw bytes; compare canonical sets keyed by ID rather than relying on unspecified ordering. Compare both inboxes after each refused destructive request. Valid controls assert exact removals and response shapes. GET/raw refusals return JSON; valid raw stays byte-identical.

Extend the rejecting-delete-trigger fixture: bad-query delete-all returns 400 while a valid control returns fixed 503. Stop/reopen and compare exact original IDs/content in both inboxes. This proves refusal avoids a failing commit and preserves durable data without new transaction machinery.

### Configuration and seed

Install nonempty chaos, snapshot every field, refuse DELETE and compare. Use a retained probability-100 DATA failure to prove the rule still acts over real SMTP with no insertion. Clear validly and observe restored normal delivery; probabilities are only 0/100.

Install multiple valid webhook targets with differing filters, secrets, URL userinfo and generated valid CA material. Compare all fields and target order via owned srv.webhooks.current snapshots after the real HTTP refusal. Separately pin public redaction. Valid clear empties targets with its existing response. No new TLS server or credential delivery assertion; keep the existing delivery suite.

For seed, snapshot exact records, inbox names and per-instance insertion/pruning counters on a quiescent server. Refuse valid-body requests to existing/absent inboxes, then compare. Successful controls prove ingest/extraction and immediate queryability. Generic HTTP request metrics are not insertion counters.

### Precedence and preserved behavior

Pair bad queries with missing IDs for GET/raw/DELETE, retaining no-query 404 controls. Pair them with bad inbox clear and existing exposed-listener Origin/Host fixtures. For seed pair unknown query with syntax/media/size/data failures, then a deserializable domain-invalid body. Assert original body status/envelopes win except for the last pair, where query wins. Do not alter json_body or body limits.

Retain strict-query/cap/inbox/assert/latest, tolerant compat list/search, selective-delete body, raw fidelity and shutdown/commit tests. Shared paths never license changing those accepted methods.

### Documentation and receipts

Check all ten served OpenAPI operations, effective query parameters including path-level inheritance, retained path parameters, explicit refusal descriptions and 400s. Pair contract pins with actual requests. README and served llms pins cover scope, compatibility, precedence and unsupported selectors without depending on paragraph wrapping.

Record baseline red and repaired green separately from preservation controls already green. Execute every new production line/branch under exact coverage. Full local gates remain the [house-rules inventory](../../docs/house-rules.md#gates--part-of-every-change); verify uses pinned copies and checks shims. CI-owned chart/Docker/advisory/release/port-80 obligations remain distinct. Design-check validates artifacts only.

## Quickstart

Run `cargo test --locked --test api` and `cargo test --locked --test compat`, then all authoritative gates. Against a disposable server with two seeded inboxes:

```bash
curl -i -X DELETE 'http://127.0.0.1:8025/api/v1/delete-all?inbox=a'
curl -i -X DELETE 'http://127.0.0.1:8025/api/v1/inboxes/a/messages?subject=keep'
curl -i -H 'Content-Type: application/json' --data '{"to":"a@x.io","text":"code 123456"}' 'http://127.0.0.1:8025/api/v1/inboxes/new/seed?typo='
```

After implementation each returns named 400, with prior data/configuration intact and no new seed inbox. These probes supplement exact-state tests.

## Risks

| Risk | Mitigation and accepted residual |
|---|---|
| Junk-query callers break. | Explicit 400 and migration; no permissive switch. |
| Middleware closes compat/PUT behavior. | Local calls and method-specific tests/docs; excluded writers retain their current query residual. |
| Error occurs after mutation; counts hide wrong-record deletion. | Parse first, compare exact IDs/records/raw/configuration, and use commit-failure/restart proof. No rollback. |
| Redacted echoes hide credential loss. | Complete owned internal snapshots plus public redaction; no new end-to-end credential proof. |
| Diagnostics disclose values. | Template labels, existing unknown-field parser, sentinel tests, JSON escaping, no raw-query logging. Keys remain intentionally named. |
| Seed precedence or resource promise changes. | Body mapping first and mixed-defect tests. Bounded extraction still precedes refusal; no new CPU/concurrency bound. |
| Concurrent test traffic invalidates snapshots. | Dedicated servers, quiescent fixtures, per-instance counters and owned paths; no sleeps/global resets. Valid independent requests can change state. |
| Certificates expire in tests. | Generate valid CA material per run; no wall-clock validity assertions or new HTTPS-delivery obligation. |
| Crash/commit/delivery claims overreach. | Keep existing boundary: commit-before-reply can duplicate on retry; memory-only data is not durable. Webhooks can lose best-effort deliveries on hang, expiry, capacity, lag, shutdown or crash; no outbox. |
| Burst pruning, lost wakeups, hostile DATA or chaos scheduling regresses. | No subsystem edits; retain burst/lifecycle/protocol suites. Oldest-only retention, subscribe-before-check, bounded DATA and ingest extraction remain. No new total-session bound or per-session chaos snapshot. |
| Tooling failure is confused with design success. | Preserve receipts, report limitations, keep artifacts clean and leave final pinned design verdict to its seat. Implementation acceptance requires every gate. |

## Design-stage validation — 2026-10-24

Formatting, all-targets locked Clippy, pinned bundle compilation, coverage selftest, publish-guard proof, seal-branch keyless proof, license check, fail-closed native-tls absence, shim integrity and design-validator selftest passed. The candidate-local `.forge/bin/brokkr` digest matches tooling.json; the initial PATH lookup failure was resolved by using that binary without installation.

Full locked tests and the full instrumented coverage run both fail the two decision-inventory tests in `tests/store_contract_docs.rs` because proposed 0062 has no decisions-index row yet. The design instruction permits exactly the three spec files and proposed decision in this commit, so the index update remains T10 rather than an unauthorized fifth file. No all-green or exact-coverage verdict is claimed. Initial 120-second test/coverage attempts timed out; subsequent longer runs establish the actual inventory failure. Run-local receipts are under `.brokkr/state/design/chief-gates/`; coverage also preserves `target/coverage/run.YpkNjM/run.log`. Implementation must index 0062 and run the complete gates afresh.
