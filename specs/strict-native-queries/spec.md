# Repair strict native query documentation and proof

- **Created:** 2026-10-24; continuation synthesis and returned-scope reconciliation 2026-10-24.
- **Status:** Draft for bounded continuation implementation; final verification/review outstanding.
- **Scope:** Epic #192, issue #197; feature slug `strict-native-queries` retained.
- **Triage:** ENGINE. Destructive admission remains high-risk despite the small repair.
- **Baseline:** current synthesis starts at `dd043c6c739dad611bc74224c3726903831f8ae7`, following returned design `f56ea03`, implementation `9af6b28`, current main and accepted 0058. All three authorized implementation files already have pending edits, including path requiredness and Luna/xhigh assertions. Preserve them for subsequent proof and review; this chief commits only the three design artifacts.

## Feature and rationale

The ten native operations below already reject unsupported queries before domain effects. This continuation repairs two confirmed low residuals: the list GET's OpenAPI 400 description still claims universal native strictness, and one deterministic global wipe test accepts any positive removal count instead of exactly two. Correct the published account and strengthen its proof without changing HTTP behavior. The return also authorizes correcting two stale Sol/high expectations in the existing fast-strategy regression test: accepted [0057](../../docs/decisions/0057-codex-delivery-recipes.md) requires Luna/xhigh for fast implementation, and accepted [0058](../../docs/decisions/0058-codex-review-routing.md) preserves that choice. Existing routing already complies; change the test, not the routing.

Accepted [0048 §5](../../docs/decisions/0048-rest-webhook-validation.md) requires method-specific documentation; accepted [0010](../../docs/decisions/0010-exact-count-asserts-and-the-shared-seed.md) supports exact isolated assertions. The original public-contract extension is recorded in existing [0074](../../docs/decisions/0074-strict-native-queries.md), still **proposed**, untouched and unaccepted. This continuation introduces no semantic change and declares no new decision. Under accepted [0045 §8](../../docs/decisions/0045-delivery-repairs.md), compatible work can proceed without treating a proposal as accepted. Stop for the operator only if a concrete fix requires changing an accepted semantic contract; never weaken a frozen guarantee.

The existing admission feature prevents a selector typo from silently widening destructive scope: `DELETE /api/v1/delete-all?inbox=a` must refuse, not wipe every inbox. Preserve that behavior and all original AC1–AC10 below. The migration section describes the original feature, not a new compatibility change in this continuation.

## Evidence and issue reconciliation — 2026-10-24

Verified all seven root-relative entries in `.forge/epic192/issue197/recovered-review-evidence/SHA256SUMS`, then read README, issue-197.json, exact old-run-ledger.md and all four original reports before synthesis. An initial invocation from inside the packet directory failed path resolution; the corrected repository-root check passed all entries without modifying evidence. Read both `.brokkr/state/design/positions/{simplicity,robustness}.md` and the current/previous intake framings. Run-local evidence stays ignored; this spec carries the necessary facts independently.

Old run `epic-192-issue-197-reject-unknow-e287e0df` shipped `d1c6a738e98fc9548a08a877f42020ac8a1f8319` (design `0cbcb23`) under a pre-0058 panel. Correctness/security were clean; tests/guarantees retained maximum low residual, `has_security_residual=false`, `fixes_applied=false`, `spec_defect=false`. The ledger's gate/shipment receipts concern that old tree. Current implementation commit IDs differ; no historical receipt certifies this candidate.

The earlier evidence-limited intake stopped for unavailable journal/issue evidence. The recovered packet resolves that gap without fabricating a journal or resetting returns. Its issue body confirms all four issue criteria: ten empty-query methods (AC1–2/6–7), destructive-selector named refusals with unchanged data (AC3–4), configuration refusals with unchanged settings (AC5), and method-specific README/llms/OpenAPI kept in step by tests (AC9/11). AC8 and AC10 preserve the wider accepted validation/proof boundary.

## Exact method scope

Every row accepts **zero query keys**. Its diagnostic label is the literal method plus route template, without interpolating path values.

| Method | Route | Handler |
|---|---|---|
| DELETE | `/api/v1/inboxes/{inbox}/messages` | `api::clear_inbox` |
| DELETE | `/api/v1/delete-all` | `compat::delete_all` |
| GET | `/api/v1/messages/{id}` | `api::get_message` |
| DELETE | `/api/v1/messages/{id}` | `api::delete_message` |
| GET | `/api/v1/messages/{id}/raw` | `api::get_raw` |
| GET | `/api/v1/chaos` | `api::get_chaos` |
| DELETE | `/api/v1/chaos` | `api::clear_chaos` |
| GET | `/api/v1/webhooks` | `api::get_webhooks` |
| DELETE | `/api/v1/webhooks` | `api::clear_webhooks` |
| POST | `/api/v1/inboxes/{inbox}/seed` | `api::seed_inbox` |

The recovered issue #197 body explicitly names these ten operations. This existing matrix satisfies its bounded empty-query scope; the title does not authorize closing excluded writers. The handlers already implement this matrix on the continuation baseline.

No query and a literal empty query preserve behavior. Any key produced by the existing form-query decoder is unsupported, including bare, empty-valued, encoded, repeated and empty keys such as `=value`. Retain the existing parser's handling of malformed escapes and separator-only input; introduce no new URI grammar or blacklist.

For otherwise admitted requests, return HTTP 400 and `{"error":"invalid METHOD /route/template query: ..."}`. Name the decoded offending key, preserve JSON escaping, and do not echo its value, a full URI or stored credentials. Do not promise all bad keys are listed, which wins among multiple keys, or serde's complete incidental sentence. Sensitive text deliberately used as a key is still named.

## Admission and precedence

Keep router Origin/Host protection and path extraction ahead of handler validation. For requests reaching a handler:

| Operation | Required order |
|---|---|
| Inbox clear | Existing `admit(&inbox)` → query parse → existing clear |
| Message GET/raw/DELETE | Query parse → lookup/render or deletion |
| Chaos/webhook GET/DELETE | Query parse → configuration read or clear/replacement |
| Delete-all | Query parse → existing global clear |
| Seed | Captured JSON extraction and `json_body(body, "seed")` → query parse → offloaded seed construction/domain validation → insertion |

A bad query wins over missing-ID 404 or storage commit 503; valid queries keep those errors. Invalid inbox clear keeps its admission error. Seed syntax 400, wrong/missing media 415, size 413 and named native data-shape 400 beat a simultaneously bad query. A deserializable body with invalid envelope/domain values instead gets the query refusal before the builder runs. Seed may receive and deserialize its already-bounded body before query refusal; rejection before all allocation or body receipt is not promised.

Refuse before domain work; never mutate then roll back. Refusal performs no store mutation, insertion/pruning counter increment, inbox creation, waiter registration, seed construction or configuration replacement. Ordinary HTTP request accounting can occur. Exact unchanged-state assertions use isolated quiescent fixtures; independently valid concurrent requests may change state.

## Preserved contracts and exclusions

Retain native list/count/await/assert/inbox-page/thread/feed shapes, bad-value refusals, caps, blank/control inbox and null/omission rules. Assert's typed `limit` remains inert and uncapped. Compat list/search remain tolerant, including GET `/api/v1/messages`; latest remains strict. Selective DELETE `/api/v1/messages` retains [0011](../../docs/decisions/0011-shim-delete-contract.md)'s nonempty `ids` body rule and gains no query selectors.

Chaos/webhook PUT, selective DELETE, other compat aliases, MCP, health, metrics, UI and document routes are outside this repair. Preserve their behavior. Documentation must name closed methods rather than claim universal native-query refusal. Ignored selectors on excluded writers remain a separate contract question, not a defect claimed fixed here.

Keep synchronous accept/store, subscribe-before-check, oldest-only pruning, ingest extraction, ordered commit/refusal/shutdown, emptied-inbox semantics, raw bytes, off-worker rendering, webhook redaction and best-effort delivery unchanged. No store/persistence/SMTP/extraction redesign, dependencies, license changes, validation subsystem, new filters, confirmation/force/dry-run switches, cache or global resource budget.

## Returned-scope clarification — 2026-10-24

The revised intake `.brokkr/state/tasks/continue-epic-192-issue-197-afte-58097c18.md` absorbs an oversized implementation return without resetting triage visits or downgrading ENGINE. Its retained full-test and coverage logs fail `fast_strategy_selectors_have_no_default_and_keep_one_two_or_five_reviewers` at the Sol expectation. Static inspection confirms both stale assertions: the loop resolves each chore/feature/design/engine office, then expects `["sol"]` and `efforts["sol"] == "high"`, while every selected implementation office already declares Luna/xhigh. These failures remain failures, not a historical pass or a newly attributed spec-defect finding.

This clarification supersedes the earlier two-file restriction and unresolved-test disposition, while preserving all dated diagnostics. Permit exactly one additional implementation file, `tests/delivery_recipes.rs`, to restore the accepted expectation for all four selected offices. The two low repairs already present in `src/static/openapi.json` and `tests/api.rs` remain pending implementation proof; at that returned design, the served-doc test still needed an explicit `required: true` path-parameter assertion. The current diff includes it; execution proof is still owed. No recipe, office, adapter, policy, tooling pin or gate edits are authorized.

At the returned design, the candidate-local `.forge/bin/brokkr` was absent. Accepted 0057 requires its digest to match `bundles/self/tooling.json` before provider dispatch. The launch environment must supply and prove that exact candidate-local binary; neither a sibling/PATH substitute nor changing the pin resolves the requirement. That was a setup block, not a semantic decision or permission to waive compilation. The current reconciliation below supersedes its availability statement; full repaired-tree verification and five-role review remain outstanding.

## Current evidence reconciliation — 2026-10-24, run 77052a74

Read both current panel positions and all three intake framings, then reconcile their claims against HEAD and the pending diff. All seven recovered hashes pass from repository root. Current `.forge/bin/brokkr` exists and hashes to `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`, matching `bundles/self/tooling.json`. This resolves the observed availability/identity gap only; it does not establish launch, compilation or reviewer dispatch. Keep candidate-local preflight before subsequent dispatch. Earlier absence and failed-test receipts remain dated history.

The pending diff already contains the required inbox-path assertion and both selected-office Luna/xhigh corrections. They are implementation proposals to validate, not missing edits to recreate. AC1–AC15 remain intact. No new decision is declared: method-specific documentation, exact counts and the settled model assertions implement accepted authority without changing semantics.

## Continuation panel synthesis and rulings — 2026-10-24

Both current positions independently advocate the same three-file repair. There is no genuine disagreement about model choice, runtime scope or retention of the historical info findings. Their emphasis on minimal changes and complete evidence is reconciled explicitly below; no artificial compromise widens the feature.

| Position or tension | Chief ruling and cost |
|---|---|
| Both: three local repairs; robustness: retain the full destructive-admission proof. | Adopt the three-file boundary and the entire existing exact-state, configuration, persistence and precedence proof. Reject counts-only/status-only substitutions. Cost: full test and five-role review time despite the small diff. |
| Both: local served-doc assertions including path requiredness. | Adopt independent exact six-key names, positive method-specific prose, negative stale-claim pin and explicit `inbox` `required: true`. Reject names-only and prose-only proof. Cost: coupled assertions must be deliberately maintained with future approved contract changes. |
| Simplicity's minimal machinery alongside robustness's failure scenarios. | Simplicity wins the implementation boundary: no registry, generic OpenAPI resolver, parser abstraction, rollback, receiver/TLS fixture or introspection seam. Robustness wins preservation of existing full snapshots and failure-path tests. Cost of rejecting extra machinery: no new end-to-end credential-delivery, durability or general prose-verification guarantee. |
| Both: exact destructive count. | Require literal numeric two for the first wipe and one for the subsequent wipe. Reject positive/ranged or response-derived expectations. Cost: fixture changes require deliberate assertion maintenance. |
| Both: restore Luna/xhigh across selected offices. | Change both stale expectations in the existing four-strategy loop, retaining `office(...)` resolution. Reject changing routing to Sol/high, accepting alternatives, deleting effort checks or testing only the named office. Cost: the regression is tied to the accepted routing and must change if the operator later changes it. |
| Both: retain three info findings; optional narrow cleanup allowed by intake. | Retain all three unchanged and disclosed; no helper/envelope cleanup is needed for the framed defects. Cost: known non-blocking test weaknesses remain; separate seed-sentinel and preservation checks remain required. |
| Both: reconcile existing artifacts and preserve historic facts. | Reuse this feature directory, preserve AC1–AC15, replace obsolete current scope and retain dated receipts explicitly as history. Reject a second routing spec or retroactive green claims. Cost: readers must distinguish old receipts from current candidate proof. |
| Proposed 0074 versus operator stop. | Adopt both positions: these corrections preserve accepted rulings under 0045 §8. No new ADR, amendment or acceptance. A concrete semantic contradiction requires operator resolution; frozen weakening remains refused. |
| Simplicity of local repair versus tooling identity/full assurance. | Neither position permits reduced assurance. Keep the candidate-local engine prerequisite, all gates and all five separate review results. Cost: a future identity mismatch, missing tool or provider failure blocks certification without changing application scope; the earlier observed binary absence is now resolved. |

## Acceptance Criteria

1. **AC1 — Ten named refusals.** Real HTTP tests cover every matrix row with a meaningful unsupported selector and typo, using nonempty and empty values. Each returns 400, JSON `error`, its exact method/route label and decoded key, excluding a distinctive supplied value. GET/raw refusals return error JSON instead of normal payloads.
2. **AC2 — Decoder and valid controls.** Real HTTP probes cover `%69nbox`, a bare key, repeated unknown keys and `=value`; each refuses through the existing parser. Do not prescribe a winning repeated occurrence. Every row has no-query and empty-query controls preserving status/shape. Raw success remains byte-identical.
3. **AC3 — Exact mail preservation.** Seed distinct messages in two inboxes; snapshot IDs, complete records and raw bytes. After each refused inbox/global/individual DELETE both snapshots remain identical. Valid controls then remove precisely the intended scope and preserve removal/deleted responses and missing-ID 404.
4. **AC4 — No commit on refusal.** Retain and execute the existing real SQLite rejecting-delete-trigger fixture for delete-all: bad query gets named 400, valid control gets fixed 503, and exact original records/IDs survive stop/reopen. No store/persistence implementation change.
5. **AC5 — Complete configuration preservation.** Install nonempty complete chaos with probabilities 0/100 and multiple webhook targets carrying secret, URL userinfo, inbox filter and valid CA material. Refused DELETEs preserve every field. Compare complete webhook snapshots via the existing server handle; separately retain redacted GET checks. A retained DATA rule at 100 causes real SMTP refusal and no insertion; valid clear restores normal delivery. Valid webhook clear retains its response and empties targets.
6. **AC6 — Seed has no effects.** Valid body plus unknown query, against existing and absent inboxes, preserves exact records, creates no inbox and moves no insertion/pruning counters. Valid no-query/empty-query seed follows ingest and is immediately queryable with extracted fields. Observe isolated per-instance counters, not process-global totals.
7. **AC7 — Defined precedence.** Pin unknown-query/missing-ID for message GET/raw/DELETE; blank/control inbox admission for clear; Origin/Host guards before query; seed bad-query combinations with syntax, media, size, data-shape and deserializable domain/envelope defects. Preserve the order and envelopes stated above.
8. **AC8 — Existing validation preserved.** Existing strict-query, invalid-value, caps, inbox, inert assert-limit, strict latest, tolerant compat list/search, selective-delete body and shutdown/commit refusal suites remain green. No method-sharing expansion or filters.
9. **AC9 — Accurate published contracts.** README, served llms.txt and OpenAPI name all ten empty query sets, named 400s, empty-value refusal, unchanged absent/empty query behavior, unsupported destructive selectors and seed precedence. Preserve compat exceptions and existing nonempty closed sets. Each OpenAPI operation retains path parameters/responses, declares no query parameters, describes refusal explicitly and includes 400. Method-aware served-contract tests cover all rows; README/llms pins cover the boundary.
10. **AC10 — Complete implementation proof.** Preserve the historical destructive-selector red/green account with its original candidate binding; do not present it as a newly executed receipt. Record fresh red/green proof for the continuation repairs in AC11–AC12 and AC15. Satisfy every [house-rules gate](../../docs/house-rules.md#gates--part-of-every-change), exact missed == 0 coverage, fail-closed native-tls absence and pinned verifier obligations. Separate local from CI-owned receipts. No exclusions, denominator reduction, dependency widening or push; artifact validation is not implementation proof.

11. **AC11 — Repair the escaped OpenAPI claim.** The served list GET's 400 description names `GET /api/v1/inboxes/{inbox}/messages` and only `to`, `from`, `subject`, `since_ms`, `limit`, `offset`; it retains blank-inbox, over-cap, malformed-value and named unknown-key refusal. Remove the universal native-query claim. The existing served-doc regression positively pins the scoped statement, independently compares the exact six query parameter names and retained `inbox` path parameter with `required == true`, and rejects the old universal sentence. It fails on the old prose and passes on the correction. Existing schemas, bounds, response shapes, ten-method pins and README/served-llms checks remain.
12. **AC12 — Exact destructive success control.** `query_free_native_operations_refuse_by_name` asserts numeric `removed == 2` for its first successful global wipe. A partial result of one (or zero) cannot pass. The following literal-empty-query wipe still asserts exactly one. Retain a controlled temporary mutation failure receipt and a restored green run; no mutation remains in the delivered tree.
13. **AC13 — Honest scope and provenance.** Implementation edits are limited to `src/static/openapi.json`, `tests/api.rs` and the two stale model/effort expectations in `tests/delivery_recipes.rs` (AC15). No runtime, dependency, gate, recipe, office, adapter, tooling-pin, policy or decision changes. The old correctness/security reports remain clean historical reports; tests/guarantees remain historical low residuals with no security residual, no fixes applied and no spec defect. Retain the three info findings below. Fresh fix/review facts must identify their own tree. Decision 0074 remains byte-for-byte unchanged and proposed.
14. **AC14 — Fresh high-risk review.** After all mandatory gates on the repaired tree, obtain five separate accepted-0058 results: Astra correctness, official DeepSeek Flash security via `deepseek-official`, Sonnet 5.5 tests, Astra guarantees, independent Fable 5.1. Astra offices are separate invocations. No DeepSeek Pro, fallback or author substitution. Bind review to the verified candidate; changes require renewed affected proof and binding. Unknown/malformed facts cannot become clean; worst verdict and hard security hold remain binding. No push, merge, tag, publication, issue closure or decision acceptance.

15. **AC15 — Restore the settled fast implementation assertion.** In `fast_strategy_selectors_have_no_default_and_keep_one_two_or_five_reviewers`, resolve the implementation office through each existing chore/feature/design/engine selector and require exactly `["luna"]` with `efforts["luna"] == "xhigh"`. Both stale assertions are corrected. Preserve all adjacent selector/no-default, reviewer/provider, protected-gate, Sol/medium triage and Luna/adapter checks. The original stale test fails against the unchanged accepted office; the corrected test passes and independently rejects an isolated selected-office model change to Sol or effort change to high. Restore every temporary mutation; the full delivery-recipes suite passes on the final tree. These are test corrections, never authorization to change routing.

## Historical findings retained

The original issue's destructive-query finding was **medium**, its documentation finding **low**. The old run's remaining two findings were **low**: universal OpenAPI wording and `removed > 0`. Repairing those now does not rewrite their historic severity or attribute a fix to the old reviewers.

The original test report's three **info** findings remain: `assert_eq!(query_value, "")` compares a loop constant; `assert_named_query_refusal_with_body` lacks its sibling's general supplied-value non-echo assertion (separate seed sentinel coverage exists); malformed seed syntax precedence is pinned negatively without a positive syntax-envelope assertion. Retention is deliberate, not an inference that they were fixed. No old report identified a spec defect or applied fixes.

## Migration

Remove unsupported query keys. For selective removal use the existing documented selective operation and its body; queries do not scope unconditional clears. No-query callers need no changes. The compatibility break deliberately replaces silently broader work with named refusal.
