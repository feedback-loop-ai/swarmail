# Implementation plan — strict native queries continuation

Implements the bounded continuation in [spec.md](spec.md). The admission handlers and witnesses already exist; they are preservation obligations, not instructions to rebuild the feature. Design/check success is not implementation, gate or review success.

## Technical context

Single Rust crate; existing Axum RawQuery, serde/serde_urlencoded, Tokio, real HTTP/SMTP servers and SQLite fixtures. The two low repairs change static OpenAPI prose and assertions in the existing API integration suite. No runtime behavior, dependency, lockfile, feature, schema, configuration or query grammar changes.

No plan template exists in this checkout (`.specify/templates/plan-template.md` is absent). This plan supplies technical context, constitution check, project structure, research, data model, contracts, quickstart and risks directly, as the earlier plan did. No separate research/model/contract files are needed.

## Constitution check

Preserve accepted 0001–0005, 0011, ordered mutation/refusal/shutdown, domain admission, 0044/0048 closed sets and compat exceptions. Accepted 0045 §8 allows these compatible corrections; 0062 stays proposed, unamended and unaccepted. No new semantic ruling is needed. A concrete contradiction of an accepted contract requires operator resolution; frozen weakening is refused regardless of tests.

Use existing real HTTP/SMTP/SQLite integration tests under 0004/0020. Keep fresh servers, per-test inboxes and temporary databases, exact full-state witnesses, generated CA material and deterministic chaos at 0/100. No mocks, sleep synchronization, shared global mutable fixtures, cold-path exclusions or coverage denominator changes. Keep accepted 0058's five-office high-risk review and protected gate/security boundaries. Commit unsigned under 0056; never push.

The chief commit touches exactly this directory's spec.md, plan.md and tasks.md. No decision is created or modified. The final deterministic sequence check owns the design verdict.

## Research and resolved questions

- All seven recovered evidence checksums pass from repository root. The packet resolves the previous missing journal/issue evidence; its reports bind to old commit d1c6a73, not this baseline 9af6b28.
- Issue #197 explicitly names the ten operations in spec AC1–AC10. Current handlers already use local ViewQuery/parse_query; no missing parser/signature work remains. Keep clear-inbox admit before parse, seed json_body before parse, and parse before domain operations.
- `src/static/openapi.json` list GET 400 still contains `every native query shape is closed (decision 0044 §3)`. The correct list GET set is to/from/subject/since_ms/limit/offset; other methods on the same path have their own contract.
- `query_free_native_contracts_match_served_docs` already starts a real server, fetches OpenAPI and llms, and checks all ten empty-query operations plus README. Extend it in place. Its current inventory omits this list GET prose defect.
- The first successful global wipe in `query_free_native_operations_refuse_by_name` leaves exactly two messages: the absent-query and literal-empty-query matrix-control seeds. Both clear-target seeds and both successive matrix IDs have been deleted. The server has no competing producers. The next wipe removes its single new wipe-control seed.
- Existing owned webhook/chaos handles provide complete hidden configuration witnesses; existing SQLite rejecting-trigger/reopen tests provide durable no-effect witnesses. No new endpoint, receiver, seam, lock or persistence mechanism is warranted.
- The three historical info findings remain deliberately unchanged and recorded in the spec. The old panel neither fixed the lows nor found a spec defect; fresh reports must preserve those facts.

## Project structure and files

| File | Continuation work |
|---|---|
| `src/static/openapi.json` | Replace only list GET's stale 400 prose with its actual method-specific allowed set and existing refusal conditions. |
| `tests/api.rs` | Strengthen the existing served-doc test and change the first global wipe assertion to exact numeric two. |
| `specs/strict-native-queries/{spec,plan,tasks}.md` | Reconcile design and ordered continuation tasks; preserve dated old validation as historical evidence. |
| `README.md`, `src/static/llms.txt`, `tests/store_contract_docs.rs` | Inspect/run existing witnesses; no planned edits. |
| `tests/compat.rs`, existing runtime/lifecycle/burst suites | Execute existing preservation and trigger/reopen proofs; no planned edits. |
| `docs/decisions/0062-strict-native-queries.md`, decision index | Preserve byte-for-byte; proposed index row already exists. |

Do not edit src/api.rs, src/compat.rs, store, SMTP, extraction, webhook internals, dependencies, gates, tooling or recipe routing. Broader defects must be surfaced with concrete evidence rather than hidden in this repair.

## Implementation approach

1. Extend `query_free_native_contracts_match_served_docs` using its fetched `doc`. Add a positive pin for the list GET method and six-key statement, a negative pin for the escaped universal claim, an independent exact query-name comparison, and retention of required path parameter inbox. Preserve all ten existing operation checks and README/served-llms checks. Check effective parameters, including any path-level inheritance; do not copy expectations out of the document itself or snapshot the entire file.
2. Run that real-server test against the old OpenAPI sentence and retain the expected failure. Correct only the sentence and rerun green. Suggested prose: “A blank inbox, an over-cap limit, a malformed value or an unknown query key (named). GET /api/v1/inboxes/{inbox}/messages accepts only to, from, subject, since_ms, limit and offset.” Retain schemas, bounds, responses and DELETE's separate contract.
3. Replace `assert!(wiped["removed"].as_u64().unwrap() > 0)` with `assert_eq!(wiped["removed"], 2)` in the first wipe control. A brief fixture-count comment is allowed. Preserve the following exact-one control. No response-derived expected value or generalized helper.
4. Prove the strengthened control rejects a partial result using a temporary mutation confined to this test, for example change the observed response's removed value to one immediately before the exact assertion. Retain the failing test output and mutation diff, restore the test, then run green. This proves the assertion detects the reported weakness; it is not a claim of a simulated storage failure. Zero is likewise excluded by equality. Do not leave mutation code or a permanent assertion-syntax test.
5. Run focused and full preservation suites, then the entire house-rule gate inventory and pinned verifier obligations. Bind commands, exits and tooling identity to the resulting tree; preserve red receipts separately from green. Old receipts are historical, not reusable proof.
6. Obtain fresh accepted-0058 review on that verified tree: separate Astra correctness and guarantees, official DeepSeek Flash security (`deepseek-official`), Sonnet 5.5 tests and independent Fable 5.1. No fallback or DeepSeek Pro. Worst verdict/security hold and unknown-fact refusal remain. The engine, not this document, decides advancement.

## Data model

No persisted or request/response model change. Mail IDs, raw bytes, extracted fields, configuration secrets/filters/CA material and per-instance counters remain unchanged. This repair corrects prose about an existing query model and strengthens a deterministic test expectation.

## Contracts

Spec AC1–AC10 remain the original full issue contract. AC11–AC14 specify the bounded repair and current proof/review. No new migration or filter is introduced. Unsupported destructive selectors still refuse before effects with native JSON 400, literal method/template and decoded key; distinctive values are not echoed. Seed body syntax/media/size/data-shape failures remain before query; domain-invalid deserializable bodies remain after query. Preserve Origin/Host, path extraction, inbox admission and 404/503 precedence.

Excluded chaos/webhook PUT, selective ids-body DELETE, compat list/search, other aliases, MCP and health/metrics/UI/docs retain behavior. Existing strict methods, latest, assert's typed inert uncapped limit, raw fidelity, redaction and shutdown/commit failures remain. No whole-API strictness claim may replace method-specific documentation.

## Proof and quickstart

Start with these targeted commands, recording baseline red for the first and restored green for both:

```bash
cargo test --locked --test api query_free_native_contracts_match_served_docs -- --exact
cargo test --locked --test api query_free_native_operations_refuse_by_name -- --exact
cargo test --locked --test api
cargo test --locked --test compat
cargo test --locked --test store_contract_docs
```

The API suite retains decoder probes, exact two-inbox IDs/records/raw preservation, complete chaos/webhook settings and SMTP behavior, cap-one seed counters and precedence. Compat retains `a_blocked_delete_refuses_the_shim_deletes_with_503`: bad-query 400, valid-query fixed 503, exact records after stop/reopen. Full workspace gates retain burst, ordering, extraction, lifecycle and existing protocol proofs. Do not replace these witnesses with status/count-only assertions.

Then execute every gate in the sole [house-rules inventory](../../docs/house-rules.md#gates--part-of-every-change), with locked Cargo, digest-matched brokkr, pinned gate scripts and checked shims, literal zero missed lines/branches/functions plus coverage selftest, publish/seal proofs and license check. Also preserve fail-closed native-tls absence, design-check selftest and delivery tamper-boundary proof. Offline verification installs nothing. Missing tools or red gates are recorded failures, never waived by design success. Keep CI-only Helm/history/selftest, Docker persistence/context/volume, network RustSec, tag rate proof and port-80 exchange distinct; do not claim local execution.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Prose repair still implies filters on destructive methods. | Positive list-GET statement and six actual parameters; retain method-aware DELETE and excluded-writer checks. No runtime closure to make old prose true. |
| Weak assertion allows partial deletion. | Exact numeric two, temporary partial-response negative control, exact-one second wipe. Fresh isolated fixture makes the literal stable. |
| Refusal comes after effects or counts conceal wrong-record deletion. | Retain local parser ordering, exact full records/raw/counters and SQLite trigger/reopen witnesses. No mutate-then-undo implementation. |
| Redacted configuration hides lost credentials. | Complete owned target/config snapshots plus redaction and real SMTP chaos behavior. No new end-to-end TLS/credential-delivery proof is claimed. |
| Middleware/refactor changes seed, inbox or guard precedence. | No runtime edits; retained mixed-defect suite. Accept bounded body extraction before query rejection. |
| Historical clean reports or small diff bypass full review. | Five fresh candidate-bound 0058 results; old severities/facts retained. Unknown safety facts and security hold refuse. |
| A prose pin cannot recognize every misleading replacement. | Positive method/key expectations and actual parameter inventory plus review; accept local coupling rather than building a natural-language validator. |
| Crash, webhook hang or parallel burst is overclaimed. | Preserve commit-before-250, subscribe-before-scan and oldest-only pruning. Commit before a lost reply can yield retries/duplicates; memory-only storage is not restart-durable. Best-effort webhooks can lose notifications; no outbox, global budget or unlimited capacity promise. Existing suites own unchanged runtime behavior. |
| Hostile/trickling DATA or certificate expiry broadens scope. | Keep bounded DATA/raw fidelity and ingest extraction; complete-line trickler/resource residuals remain. Generated CA fixtures preserve bytes without claiming TLS renewal or disabling verification. |
| Design looks like completed implementation. | Pending tasks and separate historical/current receipts. Final sequence check proves structure only; implementation and repaired-tree gates/review remain outstanding. |

## Historical validation provenance

The following dated entry is preserved verbatim from the earlier design. Its paths/receipts describe that run only. The current candidate already has the 0062 index row; its former T10 is not a pending index edit. None of these old receipts proves this continuation.

## Design-stage validation — 2026-10-24

Formatting, all-targets locked Clippy, pinned bundle compilation, coverage selftest, publish-guard proof, seal-branch keyless proof, license check, fail-closed native-tls absence, shim integrity and design-validator selftest passed. The candidate-local `.forge/bin/brokkr` digest matches tooling.json; the initial PATH lookup failure was resolved by using that binary without installation.

Full locked tests and the full instrumented coverage run both fail the two decision-inventory tests in `tests/store_contract_docs.rs` because proposed 0062 has no decisions-index row yet. The design instruction permits exactly the three spec files and proposed decision in this commit, so the index update remains T10 rather than an unauthorized fifth file. No all-green or exact-coverage verdict is claimed. Initial 120-second test/coverage attempts timed out; subsequent longer runs establish the actual inventory failure. Run-local receipts are under `.brokkr/state/design/chief-gates/`; coverage also preserves `target/coverage/run.YpkNjM/run.log`. Implementation must index 0062 and run the complete gates afresh.


## Continuation design-stage checks — 2026-10-24

All seven recovered evidence checksums and all 19 store-contract documentation tests pass. The design diff changes only the three artifacts in this directory; no runtime, test, decision or policy edits were made. Gate attempts and command/exit receipts are in `.brokkr/state/design/continuation-gates/` for this run. These are design-stage candidate-copy diagnostics, not the later hermetic verifier's pinned execution receipt; `/runtime/bundle` is not exposed in this chief box.

Formatting, all-targets locked Clippy, coverage selftest, publish-guard proof, seal-branch keyless proof, license check, shim integrity, design-check selftest, delivery tamper-boundary proof and fail-closed native-tls absence passed. Full locked tests failed `tests/delivery_recipes.rs:371`, `fast_strategy_selectors_have_no_default_and_keep_one_two_or_five_reviewers`: chore's actual model array is `["luna"]`, while the test expects `["sol"]`. The instrumented coverage run failed the same test before an exact-coverage verdict; its log is `target/coverage/run.zpWvje/run.log`. This routing/test mismatch is in unchanged baseline files, outside the two low repairs. Do not relax the gate, silently rewrite routing or claim full test/coverage success. Surface the blocker to the engine/operator for its authorized disposition; final verification remains required.

`brokkr compile --bundle bundles/self` could not run: `brokkr` is absent from PATH and the earlier run's `.forge/bin/brokkr` is absent in this box. No replacement was installed, no tooling pin changed and no binary identity or compilation pass is claimed. The later verifier must have the recorded digest-matched binary. These limitations do not change the bounded design or authorize bypassing a required gate. Implementation, all repaired-tree gates and fresh five-role review remain outstanding.
