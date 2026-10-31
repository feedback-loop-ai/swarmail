# Focused wait review repair

Date: 2026-10-28. Epic #164, issue #154; full-story follow-up.
Status: design draft, not implementation completion or a protected verdict.

Decision: 0097 — [Focused review repair](../../docs/decisions/0097-focused-review-repair.md).

## Purpose and evidence

Make persisted focused-link provenance obey its promised shape, correct current
compatibility claims, and reconcile the provisional MCP/eval stack without losing
focused waits, report output or proof obligations. A restored malformed occurrence
must not become a plausible link answer. Root-workspace success cannot establish
execution of the separate eval crate.

The commissioned candidate is 5dc3bc94dfa56d9fcf80efc6587a6de22d6a84a1,
tree 7da304fe0ff71283ac9f03f8b1a91956f066044e, on provisional #153
25b367de5e7920666bebd96a7c5216f60c8cfd1a. The chief read both run-local
positions, the complete five-member protected panel, and the intake at
`specs/issue154-review-followup/intake.md`; `.brokkr/state/tasks/` is absent.
Intake and positions remain uncommitted evidence.

The panel file `.forge/evidence/issue154-focused-review-stop/panel-event.json`
matches SHA-256
b1ebc081b3ede7669f1d7de9b8251676b7426c8c25ee38c136fdb8382b39891a.
All three issue154 packet checksums match when checked from their packet directory.
The chief checked engine bytes against tooling.json:
ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f.
These authenticate supplied identities, not conclusions or fresh implementation
passes. No attacks or eval trials were executed.

| Finding | Classification and disposition |
| --- | --- |
| Correctness C1 / guarantees G1 | One MEDIUM non-security restore defect; object-only occurrences/string-only sources with real load/startup proof. |
| Guarantees G2 | LOW non-security prose defect; correct live claims and retain captures. |
| Correctness C2 / independent 1 | MEDIUM non-security inherited standalone-eval build/execution gap; reconcile and obtain authentic coordinated proof. |
| Correctness C3 / guarantees G3; security and independent carry | HIGH security-relevant inherited review-integrity/result-attribution residual; preserve classification and ownership. |

Independent's old optimistic AC2 assessment does not override C1/G1. Tests'
historical clean result does not establish the missing canonical fixture run.
No old verification or protected verdict transfers to this successor.

## Scope and authorities

Retain the complete [focused-mail-waits contract and AC1–AC10](../focused-mail-waits/spec.md)
and [proposed 0094](../../docs/decisions/0094-focused-mail-waits.md).
Historical specs, captures and accepted decision bodies remain unchanged.
Preserve the [charter](../../docs/house-rules.md): synchronous accept==stored,
subscribe-before-check, oldest-only pruning, ingest extraction, 0012/0014/0015
mutation/failure ordering, 0020 test placement and 0045 §8. A proposal grants
no accepted-ruling exception. No frozen guarantee may be weakened.

Keep the commissioned roster: Terra 6.1 intake, Opus 5.5 implementation, Astra
correctness/guarantees, independent Claude, official Flash security and DSH Spark
GLM-5.3-Flash tests through the established Claude/Codex/DSH route. No route,
capability, fallback, engine pin or protected-boundary change is commissioned.

## Explicit synthesis

Adopt simplicity's two type-local readers and unchanged persistence bridge.
Adopt robustness's real-file controls, startup propagation, full-story preservation
and proof ownership. Both reject a framework; there is no disagreement to average.

Rule for simplicity on implementation breadth: no generic JSON validator,
migration, public DTO, dependency, repair service, resource cap or shared wait-loop
refactor. Cost: focused/report loops stay separately maintained, and malformed
metadata can stop startup until operator repair. Reject a Value prepass because
it can collapse duplicate keys. Reject short-array-only controls: the defective
decoder already refuses those.

Rule for robustness on delivery breadth: decoder-only completion loses.
Reconciliation, transport/revision controls and canonical trap execution remain
required even when they dominate the repair. Cost: delivery can remain incomplete
after storage tests pass; missing coordination is not a waived criterion.
Existing real-server regressions can prove preserved failure owners without a
new crash/chaos/concurrency framework.

Rule for accepted v2 semantics when eval conflict arms disagree. Preserve current
strict provenance, scoring, retention and cleanup; reject the historical v1
fallback. Cost: historical evidence needs its historical reader/binary. Preserve
#154's additional focused fixture where 4806ab8 lacks it.

Reject whole-parent-file copies and receipt-only tip imports as reconciliation.
Cost: inspect ancestry and adapt tests to fifteen tools. Reject a private lock or
new filtered-launch escape as proof of host-wide coordination. An authentic
coordinated full suite may supply the focused receipt without a new launch feature.

## Required behavior

Stored link_metadata_json is SQL NULL for unavailable history, or a JSON array
(including known-empty []) of objects with exactly four required nonduplicate
members: string url, nullable-string anchor_text, nullable-string rel, and string
source exactly text or html. Nullable members must be present; null differs from
empty string. Preserve serialization, order, repetition and exact strings. This
is representation validation, not URL normalization, fetching or destination trust.

Malformed non-NULL values propagate the existing column-19 Text conversion
failure through Persist::load and startup before readiness. Never skip rows,
substitute empty/unknown metadata, re-extract raw mail or backfill historical rows.
Preserve the insert/prune/counter transaction and commit-before-visibility/fan-out.

README, served llms text and OpenAPI must say the existing thirteen March/default
descriptor entries and existing result bytes are preserved while discovery
appends two focused entries, totaling fifteen. Retain old captures. June/November
schemas have object roots; successful text and structuredContent use one captured
value. REST-backed schemas use served OpenAPI projections; focused MCP-only schemas
have independently stated expectations. No full-conformance/client-success claim.

For #152, Host/Origin admission runs first. Admitted OPTIONS /mcp returns 405,
Allow: POST and no Access-Control-* even with an invalid version header. Hostile
Origin refuses first. Other non-POST invalid-version refusals, POST body-limit/
parse precedence, shared router budget and notification/error vocabulary remain.
Accepted 0043 §4 governs; retain the documented modern-header exception.

Focused waits retain full-snapshot ambiguity before 250-entry presentation,
acceptance-order newest-code selection and older provenance, independent filters,
and one monotonic deadline. Subscribe precedes capture and Closed resubscription;
Lagged rescans. An ineligible evaluation crossing expiry triggers one fresh
terminal capture; zero timeout scans once. Keep report qualification, cold
analysis and failure behavior under its separate owner.

## Reconciliation ledger

Observed inputs, not final reviewed upstream prerequisites:

| Source | Commit | Tree and required disposition |
| --- | --- | --- |
| #153 assertion correction | 393247c2e22a0f418db24ec720a6935d6547fa20 | 09ad1b537d693bb303cd2d1ba591572459b8abc8; assert schema type equals object separately from absent root ref over fifteen tools. |
| #153 later tip | df8f610d805b8a4ff11b89e6265f5dd0b69ab1e0 | 3d504e2f35df6f799dc8489e9a40fa0e41cbca1d; receipts at tip; integrate applicable 8d205d7/4227aef unavailable-report/refusal/notification controls. |
| #152 follow-up | 7f2e2129e36577f4c65703c70e20d35a484e6baf | 46fbd52c08d419e4c0fd8bbfa92bdc78285759f6; design tip; inspect f0b2b14 runtime repair and later served-prose corrections. |
| Eval integration | 4806ab8aa5416bbd2ff8f8e09621b33628f3bbab | c8a2c196bbf31b591d4fc0b348d184c29f196e42; reconcile conflicts/provenance/lifecycle/captured-binary support without transplanting historical specs or gate ownership. |
| Locally visible main | 39a04d0f092d7a0a55dafc684fe2df4cfa81c497 | a0eddfe5e50a9c7aff240bca5a3e7995b2b9a06a; local observation, not remote freshness. |

Recheck available heads at integration; record source and final commit/tree
identities and unavailable eventual reviewed heads. No prior verdict transfers.

Visible refs contain collisions at 0090 (including eval-candidate-proof and
mcp-options-admission), 0091 (including mcp-report-output and mixed-work-seats),
and 0092 (compact-email-view, mcp-candidate-proof and fixture-response-bounds).
#156 uses 0093; this stack uses provisional 0094. Other candidates allocate
0095-deterministic-spam-rules and two 0096 routing names. 0097 is next free above
the observed allocations; recheck before integration. The panel shorthand
“0092 OPTIONS” refers to that lineage's mcp-candidate-proof, not its 0090 runtime
repair. Keep distinct provenance. Collision disposition and acceptance are the
operator's; never silently overwrite or renumber accepted history.

## Acceptance Criteria

- **AC1 — Strict load:** actual temporary SQLite controls independently reject a
  complete four-position occurrence and otherwise valid occurrences with tagged
  object sources, both text and html, through Persist::load with column-19 Text
  conversion failure. Retain malformed JSON, JSON null, wrong outer/occurrence
  shapes, duplicate/missing/extra keys, wrong scalar/nullable types and unknown
  source controls. SQL NULL, [], and populated valid objects for both sources load
  with exact order, strings and provenance.
- **AC2 — Real startup:** each new alternate representation refuses real run_on
  startup before readiness for metadata conversion, not lock contention. Release
  prior owners, restore valid populated bytes, then start/query a real server and
  prove focused results/provenance. Retain pre-column migration, repeated restart,
  acceptance-sequence and failed-insert-at-cap witnesses. Serde-only tests cannot
  satisfy AC1/AC2.
- **AC3 — Prose/compatibility:** all three current documentation surfaces state
  additive discovery and the OpenAPI/MCP-only distinction. Served-description,
  independent March/default prefix/result captures, exact fifteen-name and both
  modern revisions' HTTP/binary-stdio schema/value tests pass. Do not regenerate
  old goldens or change existing Email/REST/webhook projections.
- **AC4 — Stacked MCP behavior:** reconcile applicable #153/#152 ancestry with
  object-root assertions, report unavailable/refusal/notification controls,
  OPTIONS/Host/Origin/version precedence and shared router-budget tests passing.
  Preserve report and focused behavior across revisions/transports; record source
  identities and remaining eventual upstream review/reconciliation prerequisites.
- **AC5 — Actual eval tree:** resolve every inherited conflict in
  evals/src/{artifacts,provenance,results}.rs and evals/tests/{identity,provenance}.rs
  against 4806ab8. Locked standalone test compilation and affected regressions
  prove this tree, strict v2/provenance/privacy/cleanup and current scoring.
  Preserve the focused fixture. Root gates and scratch compilation do not count.
- **AC6 — Coordinated traps:** execute
  focused_tools_complete_link_and_code_traps_without_guessing only under authentic
  host-wide standalone-eval admission retained through cleanup. Record authority,
  mechanism, ownership, actual source/tree/captured binary, command, outcome and
  cleanup. Real SMTP/MCP/application actions select the good link/newest code;
  independent WrongLink/StaleCode controls fail; ambiguity takes no action.
  A private lock, serial invocation or launcher source is not evidence. Without
  coordination leave this criterion and #201 unproved; a focused pass alone
  never proves #201 suite-wide coordination.
- **AC7 — Full story:** rerun original 0094 AC1–AC10 witnesses: mixed attribute
  ownership, ingest order/legacy bytes, source-proportional work, entry-251
  ambiguity, code order/age extremes, per-tool independent filter counterexamples,
  deterministic subscribe/Closed/Lagged/terminal/zero-timeout/shutdown schedules,
  and SMTP/seed/prune/commit-failure/report/webhook/TLS controls. Preserve local
  errors and correlated 10 MiB bridge refusal without retries of effects or
  dropping a modern representation.
- **AC8 — Decisions/residual ownership:** inventory collisions and index 0097 as
  proposed during implementation without historical rewrites or acceptance.
  Carry **HIGH security-relevant inherited review-integrity/result-attribution
  residual**, has_security_residual=true, max_residual_severity=high. Correctness,
  guarantees and independent may carry it; security owns its verdict and any hold.
  Preserve the existing must-fail witness without reproducing attacks, widening
  access, re-pinning or claiming upstream merges change pinned engine bytes.
- **AC9 — Fresh gates:** bind receipts to final committed candidate/tree and checked
  engine/shims. All charter gates, exact zero-missed coverage and selftest,
  publish/seal proofs, licenses and native-tls absence, plus required pinned
  design/delivery proofs, pass under their authorized owners. Failures and
  unavailable gates remain open; no historical pass substitutes.
- **AC10 — Final ownership:** engine verification and all five protected members
  assess the final candidate independently. Keep a missing-proof ledger for
  coordination/#201 macOS, unavailable eventual upstream reviews/reconciliation,
  #151 paid baseline and actual clients, CI platform/network/chart/Docker/port-80
  evidence and operator sealing. No full completion while required proof is
  missing. No paid trials, push, merge, publication, tag, closure or acceptance.

## Accepted costs and delivery limits

Per-mail occurrence count, source-proportional storage/amplification and complete
snapshot work remain unbounded by the 250-entry display cap. Truncation must not
manufacture uniqueness. Lexical labels imply neither browser rendering nor
trust. Started work can outlast timeout/cancellation; no global CPU deadline,
memory bound or retention lease follows.

Commit-before-250 can still lose its reply and cause duplicate retries;
memory-only state and best-effort webhooks can be lost. Preserve complete DATA,
ordered mutation/refusal and post-commit fan-out without an outbox, new commit-stage
chaos, disabled TLS verification or a global concurrency controller. Missing proofs
are obligations, not accepted availability costs.

This design yields drafted only. The sequence check owns the artifact verdict;
engine verification and protected review own later delivery verdicts.
