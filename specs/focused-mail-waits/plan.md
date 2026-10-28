# Implementation plan — focused mail waits

Implements [spec.md](spec.md) under proposed
[0094](../../docs/decisions/0094-focused-mail-waits.md).

## Technical context

One production Rust crate with Tokio, Axum, serde, regex, rusqlite and the existing
reqwest URL parser. Evals is a separate existing crate outside production coverage.
Add no dependency, feature flag, schema framework or allowlist entry. The requested
`.specify/templates/plan-template.md` is absent; this file supplies the required
technical context, constitution check, structure, research, data model, contracts,
quickstart and risks. All writes/commits use authorized workspace hands.

This four-file design commit is not implementation evidence. The sequence check
owns the design verdict. Historical specs and receipts stay unchanged, including
earlier #152/#153 artifacts and the historical patch's #154 designs. Corrections
belong in 0094 and separate new evidence, not imported historical gate receipts.

## Constitution check

Preserve synchronous insertion/250, subscribe-before-check, oldest-only pruning
and ingest extraction (0001–0005). Parsing stays outside mutation/map locks on
the existing offload path (0030); seed shares build_email (0010). Keep honest
bodies, text-first links, conditional comments, MIME code preference and historical
arrays (0031/0049/0061). Retain closed inputs/errors (0048/0074), 0079/0080's
operator-retained deviations, 0082/0091 output and current report behavior.
Proposed status grants no amendment to accepted rulings (0045 §8).

Follow 0020: branch-stable tests remain inline/measured; concrete cold failure
paths belong in a declared cfg(test) sibling harness. Public protocol tests need
no private seams. Use fresh Stores/private inboxes, instance-local barriers and
controlled time, chaos 0/100 and generated TLS fixtures. No production test knobs,
coverage exclusions, blind test movement, new global mutable seams or sleep
synchronization. Exact coverage and protected owners remain unchanged.

## Research and historical reconciliation

Store::wait_for currently delegates to wait_for_report, which owns lazy report
qualification and distinct timeout semantics. The historical patch replaces an
older wait_for; transplanting it would discard report behavior. Add one narrow
focused owner beside it, reusing subscribe/list/offload and adapter shutdown/park.
Accept a second small lifecycle loop with paired tests, not another registry.

The patch contains source-span bookkeeping and controlled-clock final-scan
repairs after review b5564ae6. Adapt their intent to current Regions, LinkRegions,
tag, walk, work counters and conditional comments. Retain the occurrence index
for unquoted href even when a later quoted rel/title flushes an ordinary region.
Neither historical passes nor ordinal assumptions prove current behavior.

| Historical area | Disposition |
| --- | --- |
| Extraction and ownership repair | Adapt current tokenizer; independently prove mixed attribute order, exact old arrays and work bounds. |
| SQLite migration/restore | One column/transaction; explicit required nullable fields, which ordinary Option decoding may not enforce. |
| Wait wrapper/final scan | Reimplement terminal classification in focused owner; preserve report owner/count wrappers. |
| Tools/admission/results | Reconcile with thirteen current tools and shared formatter; omit historical separately sampled inbox total. |
| Email/REST/webhook/OpenAPI enrichment | Reject broad publication. Skip internal metadata in shared serialization and prove old outputs. |
| March fixture edits | Reject replacement goldens. Freeze captures, compare old descriptor subsequence, independently specify additions. |
| Old design/0084/receipts | Evidence only. 0094 records corrections without editing historical specs or importing verdicts. |
| Trap/filter/lifecycle tests | Adapt substantive consumers and repairs with stale/decoy controls; no standalone receipt without coordination. |

Main/OPTIONS identities are in the spec. The base includes newer report, MIME,
bounded-work and structured-output behavior. Reconcile reviewed heads again
before delivery. Local tips expose the 0092 #155/OPTIONS collision and allocated
#156/0093. No local tip exposed 0094; use it provisionally and recheck at integration.

## Project structure and files touched

| Path | Responsibility |
| --- | --- |
| specs/focused-mail-waits/{spec,plan,tasks}.md | This design; implementation may add a separate current receipt. |
| docs/decisions/0094-focused-mail-waits.md | Proposed contract and historical corrections. |
| src/model.rs | Internal skipped optional vector and closed occurrence/source types. |
| src/extract.rs | Source-owned occurrences, labels, legacy projection and work proofs. |
| src/smtp.rs | Shared SMTP/seed construction records metadata once. |
| src/persist.rs | Nullable migration, transactional insert and strict new-column restore. |
| src/store.rs | Focused snapshot lifecycle, test-only instance-local coordination and constructors. |
| src/store/focused_tests.rs | Cold-path diagnostics only, declared cfg(test) if needed under 0020. |
| src/mcp.rs | Two descriptors/admission arms, selectors/results, finite schemas/annotations. |
| tests/mcp.rs; tests/persistence.rs; tests/p2.rs | Real SMTP/seed/filter/persistence and preservation witnesses. |
| tests/mcp_revisions.rs; tests/mcp_revisions/structured.rs | Fifteen-tool parity, finite evaluator, real HTTP/stdio and legacy proofs. |
| tests/fixtures/mcp-structured-output/ | Separate focused expectations/provenance; preserve captures/source pins. |
| tests/stdio.rs; tests/burst.rs; tests/rate.rs; tests/webhook.rs; tests/deliverability.rs | Retain/extend concrete invariant, byte-bound and report witnesses. |
| evals/tests/fixture.rs | Local trap consumers and independent application negatives. |
| README.md; src/static/llms.txt | Current tool contract/examples/history/bounds/limitations. |
| docs/decisions/README.md | Index 0094 as proposed during implementation after collision check. |

Email constructors in compat/webhook/test fixtures may need mechanical internal
field initialization, not new API output. OpenAPI assertion-bearing schemas stay
unchanged and pinned; only current descriptions may need alignment. No planned
changes to Cargo, bundles, engine, review, gates, transport, report evaluator,
UI or deployment.

## Data model and contracts

Add LinkMetadata { url, anchor_text, rel, source } and optional link_metadata to
Email, skipped by its serializer. Fresh Some([]) differs from historical None.
Keep occurrences in lexical order and derive legacy URLs from that ingest result.
No new sequence, index, backfill or query-time body parser is needed.

Nullable link_metadata_json uses explicit strict decoding for required nullable
members, unknown fields, enum/type and JSON-null refusal. Use existing serde
facilities plus a scoped representation check; do not change old from_json
fallback policy. Keep record_insert's plan -> commit -> memory -> fan-out order.

Evaluators inspect Arc<Email> snapshots outside Store guards. Compile selectors
before parking. Parse components only from stored URLs. Build bounded display
lists while counting/selecting all input; request-local distinctness may still
require O(unique URLs) memory. Avoid per-email tasks and raw/body deep copies.
Sample response time once. Preserve exact strings, unknowns and sender semantics.

Spec owns fields/variants/ranges. Finite output fragments stay beside the MCP
catalogue. No runtime validation after effects, parse-back, reread or retry.
The formatter owns one Value for text and modern structuredContent. Cross-field
counts/order/truncation require producer assertions beyond schema checks.

## Focused lifecycle algorithm

1. Validate all inputs and compile patterns before readiness park. Adapter races
   the focused future against shutdown using the current mechanism.
2. Establish one deadline before subscription; subscribe before first capture.
3. Before each capture classify terminal = now >= deadline. Capture all filtered
   retained mail in native order and release guards.
4. Await existing offload eligibility/selection work, retaining the receiver.
   Selected or ambiguous snapshot returns immediately.
5. If unsuccessful and terminal, return timed_out once. Otherwise, if expired,
   go to step 3 for one fresh terminal snapshot.
6. Otherwise await receiver/deadline. Wake/lag return to step 3; Closed first
   resubscribes. No wake/channel recreation resets the deadline.

Advancing time during terminal evaluation cannot cause another final scan. Zero
timeout scans once. Report waits keep ready-only/analysis-unavailable behavior
and never start cold analysis on expiry. Focused evaluation may exceed timeout
and observe a later commit; no hard latency deadline or reservation is promised.

## Implementation and proof sequence

Establish baseline identities and freeze fixtures first. Add model/extraction
and exact source fixtures, wire through shared ingest, then persist with migration
and failure proofs. Implement focused lifecycle/interleavings before MCP adapters.
Add complete-snapshot selection with bounded construction, admission and schemas,
then real-wire filters/all-revision workflows. Finish trap consumers, current
documentation and a separate candidate-bound proof receipt.

The active-await metric proves admission, not actual subscription/rescan. Use
per-Store barriers for lifecycle ordering. Public wire tests use acknowledged
deliveries and terminal outcomes that change under foreign selection or premature
completion; no private seams from integration tests. since_ms tests use controlled
stored stamps or an observed receive-time boundary, never SMTP Date headers or
sleep assumptions. Every filter gets its own failing counterexample.

Extraction tests combine attribute orders, repeated/adjacent destinations and a
quoted neighbour URL. For final scan, hold ineligible nonterminal evaluation,
commit eligible mail, advance controlled time and release; require fresh capture.
Advance time during terminal evaluation to prove termination. Force Closed/lag
and stop after actual park. Negative controls reintroduce only the scoped product
defect in disposable fixtures, never attacks on review/protected tooling.

## Quickstart and verification

Use installed offline tools through workspace hands. Verify evidence, shim hashes
and .forge/bin/brokkr against bundles/self/tooling.json. Its observed SHA-256 is
ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f. No download,
install or re-pin is commissioned. Start with:

```bash
export CARGO_NET_OFFLINE=true
cargo test --locked --test mcp
cargo test --locked --test mcp_revisions
cargo test --locked --test persistence
cargo test --locked --test deliverability
cargo test --locked --test p2
```

Run every command in the single
[house gate inventory](../../docs/house-rules.md#gates--part-of-every-change),
recording failures and continuing independent checks, using pinned owners:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
```

Check shim digests first. Require no policy-sweep warnings and zero missed lines,
branches/functions, never a rounded percent. Prove native-tls absence with cargo
tree --locked -i native-tls failing specifically because no package matched; other
errors do not establish absence. Use existing coverage reservation, not a private
replacement. Retain commands/statuses/log digests/commit/tree. Changes after proofs
require appropriate reruns and fresh engine binding.

The fixture command is cargo test --manifest-path evals/Cargo.toml --locked
--test fixture. Execute only with authentic required suite-wide coordination and
captured executable identities. That prerequisite remains unproved under #201:
record the gap, not a private lock, fabricated receipt or paid trial. A valid
local fixture result proves the application fixture, not the standalone full
suite or actual-client/model matrix.

Chief checks exactly four committed files and artifact structure; the sequence's
pinned final check alone owns designed/fail. Implementation commits unsigned;
engine verify and five protected members rule that candidate. CI retains platform,
network/chart/Docker/port-80 evidence. Reconciliation, #151 baseline, actual clients,
operator sealing and merge retain their separate owners. No push.

## Risks

| Risk | Mitigation and accepted limit |
| --- | --- |
| Wrong href gets a plausible label | Source identity across ordinary/quoted regions, exact attribute-order/adjacent/repeated fixtures. Conservative unknown for unsupported markup. |
| Parsing amplification | Source-proportional counters, one active label/owner, no body copies; occurrences remain uncapped beyond source size. No global-memory claim. |
| Corruption hides ambiguity | Strict new-column refusal including missing-member negatives; damaged rows cost startup availability. Legacy decoder unchanged. |
| Missed arrival during CPU work | Terminal classification before capture and controlled evaluation delays; completion can exceed timeout under shared pool contention. |
| Report regression | Separate focused owner and paired report qualification/ready-cache/failure/deadline tests; accept small lifecycle duplication. |
| First-page false uniqueness | Full semantic scan and entry-251/historical large-code controls. Display bounded, snapshot/set work is not. |
| Newest still stale for action | Explicit age/sender/older candidates and since_ms/private-inbox examples; no inferred expiry. |
| Legacy payload drift | Skipped field, frozen independent captures, fresh-ingest REST/webhook/March witnesses. General reads remain URL-only. |
| Self-certifying schema test | Independent expected values/negatives, fifteen names, exact variants and existing OpenAPI pins; no general-validator claim. |
| Huge response | Keep 10 MiB correlated refusal/continuation and both modern representations; one entry can still exceed it, no retry. |
| Crash/chaos/cap/fan-out/TLS | Preserve insert transaction, complete DATA, oldest-only pruning, offload and best-effort webhooks; generated TLS fixtures. Post-commit lost replies can cause resend duplicates; memory-mode and notification loss remain. |
| Stack/number collision | Reconcile reviewed heads/main and 0092 collision/0093 allocation; 0094 remains provisional. |
| Evidence overclaim | Carry HIGH inherited review-integrity residual; current-tree receipts only, protected owners and external holds. No attack reproduction or engine workaround. |
