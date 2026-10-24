# Reject unsupported native query keys

- **Created:** 2026-10-24
- **Status:** Draft for implementation
- **Scope:** Epic #192, issue #197; Swarmail only.
- **Triage:** ENGINE, already ruled by TRIAGE-ENGINE. Admission and destructive behavior retain the full design/review lane; patch size does not downgrade the classification.
- **Evidence:** intake in `.brokkr/state/tasks/epic-192-issue-197-reject-unknow-e287e0df.md` and both run-local `design/positions/simplicity.md` and `robustness.md`. Those ignored files are evidence, not required dependencies of this specification.

Decision: 0062 — [proposed closed query admission](../../docs/decisions/0062-strict-native-queries.md).

## Feature and rationale

A client sending `DELETE /api/v1/delete-all?inbox=a` currently wipes every inbox. An unsupported subject selector on inbox clear, or a selector on configuration DELETE, is silently discarded. Success conceals the mismatch between the client's apparent intent and the operation performed.

Close the empty query shapes of the ten operations below. Unsupported selectors and typos receive a named native JSON 400 before domain reads or effects. Valid requests keep their behavior. This extends accepted [0044 §3](../../docs/decisions/0044-compat-validation.md) and [0048 §§4–5](../../docs/decisions/0048-rest-webhook-validation.md); it creates no filtering capability.

This changes a public contract, so 0062 is explicitly proposed despite preserving accepted rulings. Operator acceptance is separate. Compatible implementation may proceed under [house rules](../../docs/house-rules.md); any discovered contradiction requires resolution before the contradictory implementation.

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

The intake derived this matrix from handlers; no fuller issue-body route list was supplied or fetched. This matrix is the design's explicit bounded scope.

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

## Panel synthesis and rulings

| Position or conflict | Ruling and cost |
|---|---|
| Both propose ten local calls to the existing parser. | Adopt. Keep one empty shape and parser; accept repetition so admission order remains visible. Reject middleware, custom extractors, registries and per-route empty structs. Excluded methods retain tolerance. |
| Simplicity limits new decoder semantics; robustness requests encoded/repeated/bare/empty-key probes. | Adopt the probes against existing serde behavior, including empty-key refusal; reject a new decoder policy. Cost: a small test matrix, without a new malformed-encoding grammar. |
| Simplicity favors minimal fixtures; robustness asks for exact records, hidden configuration, persistence and failure-path evidence. | Robustness wins on proof strength. Require exact snapshots and one existing rejecting-commit/restart fixture. Counts or redacted GET equality cannot prove non-mutation. Cost: additional test setup and execution. |
| Hidden webhook proof through delivery versus an existing owned handle. | Choose simplicity's `srv.webhooks.current()` comparison of every stored target field after real HTTP refusal. Source confirms the handle and public fields. Reject new receiver/TLS/auth machinery here; cost: no new end-to-end credential delivery proof, which existing delivery tests retain. |
| Earliest query rejection versus seed body-error preservation. | Keep body precedence, as both ultimately recommend. Reject query-first middleware; accept bounded body extraction. Robustness's broad “before work” promise applies to domain work. |
| Robustness's failure analysis versus widening the repair. | Carry residuals into plan risks; retain existing burst/lifecycle/protocol/webhook proofs. Add a deterministic chaos behavioral check because it directly proves retained active rules. No unrelated subsystem work. |
| Compatible extension might be treated as needing no decision. | Draft 0062 because a public contract changes. Neither panel nor chief accepts it; no accepted decision is rewritten. |

## Acceptance Criteria

1. **AC1 — Ten named refusals.** Real HTTP tests cover every matrix row with a meaningful unsupported selector and typo, using nonempty and empty values. Each returns 400, JSON `error`, its exact method/route label and decoded key, excluding a distinctive supplied value. GET/raw refusals return error JSON instead of normal payloads.
2. **AC2 — Decoder and valid controls.** Real HTTP probes cover `%69nbox`, a bare key, repeated unknown keys and `=value`; each refuses through the existing parser. Do not prescribe a winning repeated occurrence. Every row has no-query and empty-query controls preserving status/shape. Raw success remains byte-identical.
3. **AC3 — Exact mail preservation.** Seed distinct messages in two inboxes; snapshot IDs, complete records and raw bytes. After each refused inbox/global/individual DELETE both snapshots remain identical. Valid controls then remove precisely the intended scope and preserve removal/deleted responses and missing-ID 404.
4. **AC4 — No commit on refusal.** Extend the real SQLite rejecting-delete-trigger fixture for delete-all: bad query gets named 400, valid control gets fixed 503, and exact original records/IDs survive stop/reopen. No store/persistence implementation change.
5. **AC5 — Complete configuration preservation.** Install nonempty complete chaos with probabilities 0/100 and multiple webhook targets carrying secret, URL userinfo, inbox filter and valid CA material. Refused DELETEs preserve every field. Compare complete webhook snapshots via the existing server handle; separately retain redacted GET checks. A retained DATA rule at 100 causes real SMTP refusal and no insertion; valid clear restores normal delivery. Valid webhook clear retains its response and empties targets.
6. **AC6 — Seed has no effects.** Valid body plus unknown query, against existing and absent inboxes, preserves exact records, creates no inbox and moves no insertion/pruning counters. Valid no-query/empty-query seed follows ingest and is immediately queryable with extracted fields. Observe isolated per-instance counters, not process-global totals.
7. **AC7 — Defined precedence.** Pin unknown-query/missing-ID for message GET/raw/DELETE; blank/control inbox admission for clear; Origin/Host guards before query; seed bad-query combinations with syntax, media, size, data-shape and deserializable domain/envelope defects. Preserve the order and envelopes stated above.
8. **AC8 — Existing validation preserved.** Existing strict-query, invalid-value, caps, inbox, inert assert-limit, strict latest, tolerant compat list/search, selective-delete body and shutdown/commit refusal suites remain green. No method-sharing expansion or filters.
9. **AC9 — Accurate published contracts.** README, served llms.txt and OpenAPI name all ten empty query sets, named 400s, empty-value refusal, unchanged absent/empty query behavior, unsupported destructive selectors and seed precedence. Preserve compat exceptions and existing nonempty closed sets. Each OpenAPI operation retains path parameters/responses, declares no query parameters, describes refusal explicitly and includes 400. Method-aware served-contract tests cover all rows; README/llms pins cover the boundary.
10. **AC10 — Complete implementation proof.** Record an initial failing destructive-selector wire regression and repaired green result. Satisfy every [house-rules gate](../../docs/house-rules.md#gates--part-of-every-change), exact missed == 0 coverage, fail-closed native-tls absence and pinned verifier obligations. Separate local from CI-owned receipts. No exclusions, denominator reduction, dependency widening or push; artifact validation is not implementation proof.

## Migration

Remove unsupported query keys. For selective removal use the existing documented selective operation and its body; queries do not scope unconditional clears. No-query callers need no changes. The compatibility break deliberately replaces silently broader work with named refusal.
