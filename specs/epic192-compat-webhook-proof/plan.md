# Implementation plan — D3 compat and webhook proofs

Date: 2026-10-23. Run `epic192-first-wave-lane-d-compat-39ce98ea`; branch `epic192/lane-d-attempt3`; starting HEAD `874b4a464dd3ffb59e4e431b5e82947c70bc93dd`. [spec.md](spec.md) owns AC1–AC14 and panel rulings; [tasks.md](tasks.md) pairs current work with proof. [0061](../../docs/decisions/0061-compat-webhook-proof.md) stays proposed. D1/D2 outcomes remain recorded. This continues inherited compatible implementation, not a new architecture or completed delivery.

## Technical context

One Rust crate, Tokio/Axum, literal regex search, reqwest/rustls, SQLite and existing tokenizer/entity decoder. No new dependencies, features, schemas, config, queues or licenses. Reuse invocation/store-local observations. Inline tests remain measured; existing cold failure/watchdog paths stay in declared `src/compat/page_work_tests.rs` under 0020. No sleep synchronization or scheduler-dependent elapsed correctness ceiling.

No plan template exists in the inspected repository inventory or `.specify/templates`; this plan supplies technical context, constitution check, structure, research, data model, contracts and quickstart directly.

## Research and evidence identity

Both current positions and D3 intake agree: compatible repairs already exist. Source inspection confirms `stage_search` inside both awaited page closures, full Find/Build work inside `render_one`, and `execute_request` stripping reqwest URL metadata before bounded cause formatting. Real-client DNS/TLS fixtures and complex-needle/extracted-only/prefix wire regressions exist. NAMED lives in `src/extract/entities.rs`, not its parent module.

The retained `implementation-identity.json` under `.forge/epic192/compat-webhook-proof/` names code commit `a09a5483f1afb114ea09c21caf748c0b246a4608`. Audit all nine hashes before reusing receipts. Provenance inputs:

| Input | SHA-256 |
|---|---|
| scripts/gen-entities.py | 80541b78d56ec6ac6318adefc5bb8033f2c487fec375ad9118339b3856fb36d8 |
| scripts/data/html-entities.json | d741d877ac77c4194c4ad526b5b4a19aef8dfe411ab840a466891cdbb9f362e6 |
| src/extract/entities.rs | 4334e2e8f0af1ba8a6c0a65fa0b716f51a7ee70867018e350ff0b4d154cc6335 |

D3 intake records fresh isolated generator exit 0 and 60,746-byte complete NAMED equality. It does not independently establish tampered-snapshot refusal or wrong-scalar evaluator failure. Preserve historical receipts with original commit, cwd, tool and timestamp. Historical workspace exit 101 and coverage exit 1 named the missing index row; parent has now repaired it. Fresh final-tree results remain required. The archive below records original failures, not current blockers or passes.

The separate historical `prefix-skipped.json` used compat source digest `2ce18c7dd9e5fe38bebacd13c1443c09485a91337da9f576e33a74e6700a9dc7`, while current compat is `ae67feaa8cc8c67fd42d7f312c51563d8dbf1b9cf9c1ef11c05ad970b55af0f5`. Its red/restored-green remains historical; D08 must renew that owned-path mutation on current bytes before claiming current negative proof. This is distinct from the 13-mutant final-source summary.

## Constitution check

Preserve 0001–0005, synchronous insert/queryability before SMTP 250, single-writer protection, ordered FULL commit, oldest-only pruning, subscribe-before-check including after closure, fresh-ingest extraction, raw/source bodies and verbatim historical arrays. Keep complete awaited offload and panic symmetry. No detached acceptance, webhook dependency on acceptance, rollback change or guard retained during response construction.

0045 §8 allows compatible work under proposed 0061; it does not authorize AC10–AC12. No dormant alternative. Only the operator accepts exact amendments. All house gates and zero-missed lines/branches/functions remain required; no exclusions, denominator change or dependency widening. Full engine route retains Qwen Max correctness, Claude Opus security, official DeepSeek Flash guarantees and DashScope GLM 5.3 tests. Provider failure stays visible; no Pro/fallback, fabricated review, security-hold waiver or policy shortcut.

## Project structure and edit boundaries

| Path | D3 role and allowed edits |
|---|---|
| specs/epic192-compat-webhook-proof/{spec,plan,tasks}.md | Chief synthesis; later current evidence/disposition. |
| docs/decisions/0061-compat-webhook-proof.md | Reserved proposal, never acceptance. |
| src/compat.rs | Existing awaited staging/render owners; demonstrated compatible fixes/local observations. |
| src/webhook.rs | Sole execute/redact/format owner; deterministic real-client failure proof. |
| tests/compat.rs | Refusal, misses, prefix/reopen/retry and fresh-ingest extraction wire proofs. |
| tests/webhook.rs | Real delivery controls; no #178 sink/budget refactor. |
| scripts/gen-entities.py | Generator proof/fix only if evidence requires it. |
| src/extract.rs | NAMED regeneration/equality only; preserve inherited algorithm repairs. |
| src/store.rs | Existing emptied-entry regression comment only; G/parent owns broader work. |

Read/test but do not newly edit `src/compat/page_work_tests.rs`, `src/extract/entities.rs`, `src/smtp.rs`, `tests/smtp_edges.rs`, `tests/api.rs`, `docs/decisions/0049-extraction-compat.md` or `specs/extraction-compat/**`. Inherited compatible bytes/addenda remain. Essential edits or fresh temporary mutations there need a concrete parent request. Parent's proposed 0061 index repair is done; do not edit the index again. No other lane/worktree, baseline, recipe, gate, shim, snapshot/license or bundle edit.

## Implementation approach

1. Audit clean HEAD, inherited source/evidence identities and proposed status. Resolve mismatched receipts before reuse. Record D3 separately; no launch or invented original journal history.
2. Preserve lowering/escaping/compilation inside each awaited page closure before scan, validation precedence and public search_needle. Admitted fixture: 60,000 ASCII k bytes (Mailpit URI 60,021; Hog 60,037). A 414/EOF/panic is not compiler-refusal proof. Run actual compiler-site current-thread proof and independently challenge each caller's placement.
3. Preserve complete render_one lookup/latest, projection/render/copy/build and finished serialization. Actual Find/Build observations distinguish work from the old gate. Retain all eleven routes, body/type/404, released map guards, SMTP/health progress and independent DATA 250 queryability. The serialization-only mutant must retain serialization AND old gate on the pool while hoisting actual lookup/build; owning evaluator must fail.
4. Keep execute_request as sole redaction boundary. Real reqwest consumes a per-client resolver refusal and fresh loopback untrusted certificate. Identify DNS and UnknownIssuer, retain useful bounded causes, exclude plain/encoded username/password. Fixture metadata deliberately carries original userinfo after Basic-auth construction so removing without_url tests confidentiality. Preserve functional Basic auth, verified rustls, no proxy/redirect, cache isolation/retries/budgets.
5. Retain link/code sentinels absent from every source/raw field; assert populated arrays, both wire misses and ordinary hits. Retain ordered A/B/C real SQLite refusal on B: named 503, A gone, B/C/unrelated intact live/reopen with prior Store released; remove trigger, retry original ids to remove 2 then 0. No atomic batch or mock store.
6. Preserve named map-guard hang comment and executable regression. Retain seven nested-URL rows, curly-scalar trimming, #136/encoded quotes, decode-once/source-only and exact-work controls. Fresh algorithm/harness defects go to parent, not an implicit ownership expansion.
7. Execute generator and compare complete NAMED; test altered-snapshot refusal on a run-local copy (nonzero, empty stdout). Run unchanged full-mapping evaluator. Audit inherited wrong-scalar red/exact-restoration-green evidence against source/tool identities. If insufficient, request parent authority for entity-module mutation; leave proof open meanwhile. Do not regenerate an equal table or alter attribution.
8. Audit historical negative proofs by mutation, bytes, failing assertion/exit and restored digest/green rerun. Renew owned-path mutations when evidence is absent/stale or a repair changes the path. Compilation failure is not behavioral red. Fresh unowned mutations require parent ownership. Restore exact bytes on every outcome without blanket reset or another checkout.
9. Record each finding and rerun every required final-tree gate with pinned tooling. Complete compatible owned fixes; refer unrelated/unowned gate defects to parent with exact output. No skipping or lowered gates. Actual independent review/verify/ship follows through configured engine; this chief launches none and claims no verdict.
10. Keep AC10–AC12 pending. R4.2 authority permits refusal-only normalization, never execution rewrite. Link order and conditional comments need separate exact rulings AND file ownership. Retain requested fixtures. No whole #190/epic completion without resolved criteria and actual delivery proof.

## Data model and contracts

No persistent change. Stored links/codes remain ordered arrays derived once, historical values verbatim. Matcher stays request-owned, no cache. BodyOrder remains codes-only. Direct URL attributes preserve encoded quotes; embedded delimiters and curly trim keep inherited behavior. Ordinary comments retain the explicitly unresolved Outlook output.

Keep per-dialect errors/precedence/caps and Hog's inclusive 67,108,864-byte page budget. Shim503 can leave a committed prefix; retries converge. Webhooks remain best effort: lag, slots, shutdown and crash can lose notifications; lost replies can duplicate remote processing. No outbox or global memory/CPU bound. Query/fragment exposure is unchanged; userinfo cannot leak.

## Quickstart and validation

These are obligations, not D3 execution receipts. Use private D3 target, assigned CPU group D48–63/16 CPUs and launcher-installed tooling. Targeted entry points:

```bash
cargo test --locked --test compat
cargo test --locked --test webhook
cargo test --locked --test smtp_edges
cargo test --locked --test api smtp_issue_136_link_fidelity
cargo test --locked --lib compat::page_work_tests
cargo test --locked --lib dns_failure_causes_keep_no_userinfo
cargo test --locked --lib tls_verification_failure_causes_keep_no_userinfo
cargo test --locked --lib extract::
cargo test --locked --lib all_whatwg_entities_match_snapshot
cargo test --locked --lib a_delete_that_empties_an_inbox_removes_the_entry_live_and_after_reopen
cargo test --locked --test store_contract_docs
python3 -I scripts/gen-entities.py scripts/data/html-entities.json
```

Capture mutation patch/hash, command, behavioral red assertion/exit, restoration hash and owning evaluator green. Inventory: needle-panic, compile-worker, compile-worker-hog, leak-dns, leak-tls, links-searched, codes-searched, serialization-only, prefix-false-success, prefix-skipped, prefix-resurrected, whole-attribute-tolerance, old-curly-trim, entity-wrong-scalar, plus altered-snapshot refusal. Historical final summary has 13 mutations: audit separately retained skipped-prefix evidence instead of claiming that summary contains all 14 source mutants.

The single gate inventory remains [house rules](../../docs/house-rules.md#gates--part-of-every-change): fmt, locked workspace clippy/tests, pinned brokkr compile, coverage/selftest, publish/seal proofs, licenses. Also configured design-check selftest, delivery tamper-boundary, recipe structural and fail-closed native-tls absence. Expected Cargo no-match text plus nonzero exit proves absence; arbitrary Cargo errors do not. Record exact commands, resolved proof/tool paths/digests, source identities, exits/logs and explicit zero missed lines/branches/functions.

Immutable baseline: `/home/valentin/source/feedback-loop-ai/swarmail/.forge/epic192/start-now/eight-lanes/baseline/bundles/codex-fast`. Verify/ship must use its actual pinned chain; historical equal-copy receipts are not D3 dispatch. If chief workspace cannot see baseline, candidate design preflight is only local; sequence deterministic pinned check owns the verdict. Never substitute version-equal engine or change baseline/shims. No candidate script repair is assigned.

CI-only chart/history/selftest, real Docker persistence/context/volume smoke, network RustSec, tag-only release-rate-5k and real port80 exchange remain separate. Original journal/live self-bundle/release evidence stays external, never fabricated. No release edit requires local chart/package change. No push/merge/tag/publication/comments/issue closure.

## Risks

| Risk | Mitigation and remaining cost |
|---|---|
| Historical evidence mistaken for current success | Preserve original identity/date, audit hashes/logs, renew final-tree gates; index repair proves no gate. |
| Old plan grants broader ownership | Current path table controls; preserve inherited source, send precise extra edits/mutations to parent. |
| Proposal mistaken for acceptance | AC10–AC12 remain open with fixtures; exact operator provenance for each ruling. |
| Generic400 or misplaced probe passes defect | Exact compiler refusal and actual work sites; separate caller and serialization-only mutants. |
| DNS/TLS wrong arm or already sanitized request | Per-client resolver, generated cert, cause classification, original metadata and leak mutants. |
| Bad mutant blocks its cleanup | Existing cold harness independent watchdog/coordination, no performance ceiling or sleep sync. |
| Prefix proof covers no commit | Fail B after A, verify suffix/unrelated live/reopen, retry 2 then 0 and distinct mutants. |
| Table length hides wrong mapping | Complete equality, real wrong-scalar evaluator and exact restoration; preserve snapshot/license. |
| Pool saturation, amplification, webhook loss | Preserve bounds/ordering; disclose costs without new admission/durability promises. |
| Store/SMTP stress regressions | Existing cap/watcher/commit/hostile-DATA/burst/rate suites; no acceptance/store algorithm edits. |
| Unknown residual/provider failure bypasses policy | Actual reviewers/exec verdicts only; security holds and unknown/unsafe residual rules stay hard stops. |

## Chief validation — D3, 2026-10-23

The chief's bounded checks are recorded separately under `.forge/epic192/compat-webhook-proof/design-d3/`: current documentation-contract suite (19 tests), artifact structure/relative links, all nine inherited implementation/input digests and byte-identical historical plan archive. These are design checks, not full house-gate, coverage, independent-review or ship receipts. The immutable baseline script is not mounted inside this workspace; candidate design-check preflight after commit cannot stand in for the sequence's pinned final check. No production source changed.

## D3 implementation handoff — 2026-10-23

This section records the implementation phase at D3, after chief commit `ff58cb3` and parent index repair `874b4a4`. It supersedes no historical receipt or operator ruling. Compatible implementation is locally proved; whole #190 and epic192 remain open. Actual protected verify/review/ship are subsequent engine-owned phases, not verdicts supplied by this seat.

The only new source diff is in `src/webhook.rs`'s inline tests: split each DNS/TLS compound `assert!(class && detail)` into two assertions. Both failure-class and useful-cause requirements remain mandatory. The first fresh coverage command actually refused four short-circuit branches: lines12564/12564, branches734/738, functions1390/1390. Its report remains `target/coverage/run.Zk4tG4/` and its exit1 receipt is `implement-d3/coverage.json`. No production implementation, coverage script, exclusion, dependency, license, budget or accepted behavior changes. The repaired source passes exact coverage: **lines12564/12564, branches730/730, functions1390/1390**, zero missed in each dimension, report `target/coverage/run.oZdZr6/{lcov.info,coverage-summary.json}`.

### Current proof and findings

All paths below are under `.forge/epic192/compat-webhook-proof/` unless explicitly repository-relative. `implement-d3/final-evidence.json` records final source/report digests, all exact commands/exits, log paths/hashes and environment. The nine inherited source/input hashes were audited in `implement-d3/identities.json`; only the assertion-only webhook test rewrite changes an inherited hash. Historical plan archive and receipts stay unchanged.

| Finding / AC | D3 disposition and actual evidence |
|---|---|
| #170 2.1 / AC1 | Named complex-needle400 and MailHog bare400 wire tests pass in the full workspace run. The identical-source needle-panic behavioral101/restored0 receipt is audited, not relabeled as a fresh mutation. |
| #170 2.2 / AC2 | Both actual compiler-site placement and held SMTP/health progress tests pass. Independent Mailpit/MailHog compile-worker behavioral101/restored0 receipts match unchanged source. No new aggregate CPU or latency bound claimed. |
| #170 2.3 / AC3 | Real per-client DNS refusal and generated untrusted loopback TLS/UnknownIssuer tests pass, with useful causes and absent plain/encoded userinfo. Fresh `implement-d3/leak-dns.json` and `leak-tls.json` each show behavioral101, exact restoration and green0 against final assertion bytes. Basic-auth/proxy/redirect/CA/cache/retry controls pass in the workspace run. |
| #190 22.1 / AC4 | Extracted-only link/code arrays are populated, both wire searches miss each distinct sentinel absent from raw, and ordinary candidates match. Separate unchanged-source links-searched/codes-searched mutants are audited red/restored-green. |
| #190 22.2 / AC5 | Fresh isolated generator exit0 reproduces the complete60,746-byte NAMED block; altered snapshot exits1 with zero stdout. `implement-d3/generator.json` records commands/input hashes. Full mapping evaluator passes. Historical same-cardinality wrong-scalar101/exact-restoration0 is audited against identical generator/snapshot/table and reconstructed patch digest; no unowned source mutation or table edit performed. |
| #190 22.3 / AC6 | Actual Find/Build thread observations and eleven-route held-response/body/type/404 checks pass, including independent DATA250/queryability and guard-release controls. Historical serialization-only mutant keeps the old gate and serializer offloaded while hoisting lookup/projection; its behavioral101/exact-restoration0 is audited at identical source. |
| #190 22.4 / AC7 | Selective SQLite refusal on B in A/B/C produces named503, persistent committed A prefix, intact B/C/unrelated mail and convergent retries2 then0. Stale skipped-prefix evidence is replaced for current proof by fresh `implement-d3/prefix-skipped.json`: behavioral101 on live A still present, exact restoration and green0. Distinct false-success/resurrection historical mutants remain audited. |
| #190 22.5 / AC8 | Existing emptied-entry comment names retained-map-guard same-shard deadlock/hang; its live/reopen unit regression passes. Store bytes remain unchanged. |
| #190 R4.1 / AC9 | All seven nested-quote rows pass fresh SMTP/REST readback, together with #136/encoded-quote, source-only/decode-once and exact-work controls. Inherited compatible fix/addendum preserved; whole-attribute-tolerance historical red/restored-green audited. No new extractor algorithm edit. |
| #190 R4.2 / AC10 | **OPEN.** Preserve accepted0049§6 ordered wrapper recognition. Refusal-only all-double-quote normalization requires exact operator acceptance of proposed0061§1. No authority supplied or inferred. |
| #190 R4.3 / AC11 | **OPEN.** Independent MIME-part links stay accepted text-first. Operator link-order ruling and parent extraction/SMTP ownership are required before changing to source order. |
| #190 R4.4 / AC12 | **OPEN.** Conditional comments retain ordinary-comment behavior and the specified truncated-prefix output. Operator must authorize quoted-attribute treatment or explicitly accept that named debt. Neither ruling exists. |
| #190 R4.5 / AC13 | Inherited scalar-safe trailing U+201D/U+2019 trimming and mixed/interior/balanced punctuation/work controls pass; unchanged-source old-curly-trim historical red/restored-green audited. |
| AC14 local implementation proof | Every required local house command and supplemental proof returns0 on the corrected source, as recorded below. Engine-owned baseline dispatch, protected reviews and ship remain outstanding. No story/epic closure claimed. |

`implement-d3/historical-mutation-audit.json` verifies all13 historical receipts against original/current source, reconstructed mutant hash, behavioral failing assertion/log and restored-green log. Final proof uses11 unchanged-source historical cases plus3 fresh cases (DNS/TLS leakage and skipped prefix), covering all14 distinct source mutations. Snapshot tampering is separate. No historical receipt is rewritten or promoted to current execution. The initial long workspace response timed out at the transport while the command continued; completed per-command receipts, not the timeout, establish exits. First-run coverage1 stays recorded.

### Final source gates

All commands run in the D3 candidate, with offline Cargo,16 build jobs, private D3 target and CPU48–63. Each row has exact command, UTC interval, exit0 and SHA256-bound log under `implement-d3/final-gates/<name>.json`:

| Receipt name | Exact command (all exit0) |
|---|---|
| fmt | `cargo fmt --all -- --check` |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| workspace | `cargo test --workspace --locked` |
| compile | `brokkr compile --bundle bundles/self` |
| coverage | `bash scripts/coverage-gate.sh` |
| coverage-selftest | `bash scripts/coverage-gate.sh selftest` |
| design-selftest | `bash bundles/self/scripts/design-check.sh selftest` |
| delivery-tamper | `bash bundles/self/scripts/delivery-proof-test.sh` |
| publish | `bash scripts/publish-guard-test.sh` |
| seal | `bash scripts/seal-branch-test.sh` |
| licenses | `cargo deny check licenses` |
| recipes | `cargo test --locked --test delivery_recipes` |
| native-tls | Python invokes `cargo tree --locked -i native-tls`; requires its nonzero exit AND exact `did not match any packages` diagnostic. Recorded Cargo exit101 proves expected absence, not arbitrary Cargo failure. |

The resolved launcher-supplied `.forge/bin/brokkr` digest remains `e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`. Candidate canonical proof/scanner/shim digests still equal the retained baseline identity; no proof bytes changed. These local runs use unchanged candidate copies because the immutable external baseline is not mounted in this seat. They do not claim actual engine-owned baseline verify/ship dispatch. The configured engine supplies those phases and the fixed Qwen Max/Opus/official DeepSeek Flash/GLM5.3 independent lenses; no manual replacement, provider fallback or security-hold waiver.

### Preservation and remaining obligations

Accepted0001–0005, synchronous Store::insert and250-queryability, subscribe-before-check, oldest-only pruning, single-writer/FULL ordered commit, ingest-only extraction, source/raw/historical truth and shim committed-prefix behavior remain intact. No dependency/allowlist, exclusion, public shim, gate, bundle, adapter, charter, baseline or unowned source edits. Proposed0061 remains proposed; parent index repair is preserved.

D01–D14 local implementation tasks are proved; D15 awaits actual engine-owned verify/review/ship, and D16–D19 retain authority-dependent AC10–AC12. Original journal absence and live-self-bundle/release evidence remain external. CI-only chart/history/selftest, real-Docker persistence/context/volume smoke, network RustSec audit, tag-only release-rate-5k and real port80 exchange are unproved by these local runs. No release change requires local chart/package work. No cross-lane implementation request is newly necessary; parent/operator still own exact semantic rulings and eventual integration/seal/landing. No push, merge, rebase, tag, publication, GitHub message or issue closure occurred.

## Historical evidence archive — pre-D3

Everything below records earlier attempts at original identities and is retained verbatim, including failed gates and then-unresolved index request. It is not a D3 instruction or fresh pass. Sections above supersede old planning/ownership and index-repair demands. Parent874b4a4 repaired the row; D3 final-tree proof remains outstanding. Old designed receipt validates only its old chief commit.

## Implementation handoff — 2026-10-23

Compatible code commit: `a09a5483f1afb114ea09c21caf748c0b246a4608`, unsigned
(`%G? = N`). Chief identity remains `c498682`; deterministic design check
returned `designed` in
`.forge/results/6c006808-b705-4b58-9051-3c3cbf9f38ec-check.json`.
This implementation returns **broken**: required full gates refuse the
parent-owned decision-index omission. This is neither oversized scope nor
a frozen-guarantee refusal. No successful verify/review/ship receipt or
whole #190/epic completion is claimed.

### Finding-by-finding disposition

| Finding / AC | Implementation and actual proof | Remaining obligation |
|---|---|---|
| #170 2.1 / AC1 | `too_complex_needle_refuses_on_the_wire`: 60,000 ASCII k characters; URI bytes 60,021 Mailpit / 60,037 MailHog. Exact Mailpit JSON 400 `Error: the search needle is too complex`, Hog bare 400. Existing empty/kind/prefix/literal controls stay green. Actual `.expect()` mutation fails this wire evaluator. The earlier 250,000-character probe got HTTP 414 and is explicitly not compiler-refusal evidence. | Full delivery gates. |
| #170 2.2 / AC2 | Both production handlers lower/escape/compile inside the awaited existing page offload, before scan/build. `needle_compilation_leaves_the_async_worker` observes actual compiler-site entry on a current-thread runtime while real SMTP/health progress. Separate Mailpit and MailHog worker-compilation mutations fail. | No new aggregate CPU/admission bound claimed. |
| #170 2.3 / AC3 | Real reqwest connector consumes per-client resolver refusal and loopback TLS verification failure (`UnknownIssuer`, not TCP refusal). Fresh certificate validity derives from the run's date. Shared production `execute_request` formats informative bounded causes after `without_url`. Distinct raw/percent-encoded username/password sentinels and userinfo are absent. Tests restore original URL metadata after Basic-auth construction so omitted redaction really leaks; independent DNS/TLS mutations fail. Existing webhook suite green. | DNS proof is controlled resolver failure through reqwest, not OS/public-DNS outage. No confidentiality residual identified by these tests; independent review remains. |
| #190 22.1 / AC4 | `extracted_only_fields_miss_both_wire_searches`: populated link/code arrays absent from every source/raw field. Both real shims miss each sentinel and hit ordinary subject/body controls. Independent links/codes-haystack mutations fail the respective assertion. | Full delivery gates. |
| #190 22.2 / AC5 | Reuse actual prior generator equality (60,746 bytes, exit 0) and altered-snapshot refusal (exit 1, empty stdout) only after three identical-input hash checks. Independent evaluator catches same-count AElig U+00C6→U+00C7; exact source restored to SHA-256 `4334e2e8f0af1ba8a6c0a65fa0b716f51a7ee70867018e350ff0b4d154cc6335`, same evaluator green. | Historical T02a now checked with dated provenance. T11/T12 stay open; no fresh-download claim. |
| #190 22.3 / AC6 | Eleven real response routes report actual lookup and completed build observations via scoped per-store gate metadata, separate from the old hold. Direct Find/Build callback proof also observes actual thread. Full body/content-type/404 controls, SMTP/health progress and an independent DATA 250/immediate queryability pass. Mutant moves real Mailpit lookup/projection to the worker while actual JSON serialization AND old gate remain on the pool; held-route evaluator fails, restoration passes. | Full delivery gates; no Store field, feature flag or map guard retained during build. |
| #190 22.4 / AC7 | Real A/B/C DELETE with SQLite trigger refusing B alone: named HTTP 503, A absent, B/C and unrelated inbox present live and after releasing old Store/reopening. Remove trigger; original-selector retries remove 2 then 0. False-success and committed-prefix resurrection mutations fail wire/live evaluator; complete restart/retry evaluator restores green. | Existing ordered per-id mutation unchanged. |
| #190 22.5 / AC8 | Existing store regression comment names retained-map-guard entry-removal deadlock/hang; unchanged executable live/reopen test passes. | Store production code untouched; G's ownership preserved. |
| #190 R4.1 / AC9 | All seven issue rows plus leading encoded-quote control pass fresh SMTP→REST exact arrays in `nested_attribute_urls_and_curly_suffixes_survive_fresh_ingest`. Leading-value tolerance; embedded candidates use ordinary quote delimiters and second CSS URL survives. Old whole-value tolerance fails the wire evaluator. Encoded quotes, unchanged #136, decode and exact work controls pass. | Compatible clarification appended under 0049 §1; full delivery gates. |
| #190 R4.2 / AC10 | Existing ordered wrapper recognition preserved; chief's explicit authority finding recorded in proposed 0061 §1. | OPEN: exact operator amendment. Five quote-first/interior-quote forms are not claimed fixed or tested as refused. |
| #190 R4.3 / AC11 | Text-first independent-part links and codes-only BodyOrder unchanged; exact fixture/order alternatives retained in spec. | OPEN: operator link-order ruling; no source-order link wiring. |
| #190 R4.4 / AC12 | Ordinary comments, including specified Outlook truncation, unchanged; complete fixture/options retained in spec. | OPEN: exact repair or accepted-retention ruling; no inferred acceptance. |
| #190 R4.5 / AC13 | Remove U+201D/U+2019 as whole scalars. Exact removed-byte controls 4/9/0/3, interior curly quotes, IPv6/balanced parentheses; fresh plain/HTML SMTP readback. Old trimmer fails the wire evaluator, restoration passes. | Compatible clarification appended under 0049 §1; full delivery gates. |
| AC14 | Every required command attempted; actual results below. | OPEN: full suite and coverage failure, independent phases and CI-only/external evidence. |

### Evidence and identities

Run-local root: `.forge/epic192/compat-webhook-proof/`. Each `gate-*.json`
records exact argv, cwd, relevant environment, UTC start/end, exit, log path
and log SHA-256. `implementation-identity.json` binds source hashes to the
code commit. `mutations-summary.json` validates all **13** final-commit
mutation receipts: original == restored source digest, behavioral exit 101,
restored evaluator exit 0. Each named mutation has `.patch`, `.json`,
`-red.log` and `-restored.log`; compile errors explicitly do not count as
behavioral red. `mutate.py` and `gates.py` are the actual executed drivers.
An earlier group reached the workspace-command watchdog after five cases;
it restored source, and every authoritative receipt was rerun in the final
group. The interrupted group is not claimed as a completed pass.

The inherited generator receipt remains in
`.forge/tasks/epic192-first-wave-lane-d-compat-0b5b1027.md` and current triage.
Its date/provenance remains historical; no rerun was used to invent a date.
Three current input hashes still equal the Research table above.

`baseline-identity.json` records the immutable source root, exact SHA-256
and byte equality of proof chain/scanner/resources, actually compared through
the host's read-only view. All compared pairs equal. Workspace hands do not
mount the external baseline: local commands execute unchanged byte-identical
candidate copies, using the exact paths recorded below. Installed
`.forge/bin/brokkr` is byte-identical to the launched baseline binary,
SHA-256 `e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`;
no version-equal replacement. No proof/bundle/policy/recipe/shim bytes changed.
These are local implementation proofs, not fabricated engine-owned baseline
verify/ship dispatch receipts.

Final compatible-source suites: `cargo test --locked --lib` → **0, 328
passed** (`lib-final.log`), `cargo test --locked --test compat` → **0, 32
passed** (`compat-final.log`). Final fmt/clippy → 0. Those source bytes equal
the code commit; every final mutation restores the same bytes.


### Mandatory gate and preservation receipts

| Command | Exit | Receipt and log under run-local root |
|---|---:|---|
| `cargo fmt --all -- --check` | 0 | `gate-fmt.json`, `gate-fmt.log` |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | `gate-clippy.json`, `gate-clippy.log` |
| `cargo test --workspace --locked` | 101 | `gate-workspace.json`, `gate-workspace.log` |
| `.forge/bin/brokkr compile --bundle bundles/self` | 0 | `gate-compile.json`, `gate-compile.log` |
| `bash scripts/coverage-gate.sh` | 1 | `gate-coverage.json`, `gate-coverage.log` |
| `bash scripts/coverage-gate.sh selftest` | 0 | `gate-coverage-selftest.json`, `gate-coverage-selftest.log` |
| `bash bundles/self/scripts/design-check.sh selftest` | 0 | `gate-design-selftest.json`, `gate-design-selftest.log` |
| `bash bundles/self/scripts/delivery-proof-test.sh` | 0 | `gate-delivery-tamper.json`, `gate-delivery-tamper.log` |
| `bash scripts/publish-guard-test.sh` | 0 | `gate-publish.json`, `gate-publish.log` |
| `bash scripts/seal-branch-test.sh` | 0 | `gate-seal.json`, `gate-seal.log` |
| `cargo deny check licenses` | 0 | `gate-licenses.json`, `gate-licenses.log` |
| `cargo test --locked --test delivery_recipes` | 0 | `gate-recipes.json`, `gate-recipes.log` |
| `Fail-closed cargo tree --locked -i native-tls shell probe (exact argv in receipt)` | 0 | `gate-native-tls.json`, `gate-native-tls.log` |
| `cargo test --locked --test api smtp_issue_136_link_fidelity` | 0 | `gate-api136.json`, `gate-api136.log` |
| `cargo test --locked --test webhook` | 0 | `gate-webhook.json`, `gate-webhook.log` |
| `cargo test --locked --test smtp_edges` | 0 | `gate-smtp-edges.json`, `gate-smtp-edges.log` |
| `cargo test --locked --lib a_delete_that_empties_an_inbox_removes_the_entry_live_and_after_reopen` | 0 | `gate-store-hang.json`, `gate-store-hang.log` |

The two full-suite failures are precisely
`store_contract_docs::every_decision_file_has_exactly_one_resolving_index_row`
and `store_contract_docs::the_inventory_refuses_missing_dangling_duplicate_and_malformed_entries`:
`0061-compat-webhook-proof.md: no index row`. Coverage's instrumented suite
hits the same failure; the gate exits 1 and preserves
`target/coverage/run.C721ey/run.log`. It produces **no trusted exact 100%
verdict or zero-missed-lines/branches/functions proof**. No test was skipped
and no denominator/gate was lowered. Instrumentation uses the installed
`nightly-2026-10-22`; ordinary Cargo uses launcher-mounted rustc 1.98.0.
Offline gates use the private target and 16 build jobs, per receipts.

Native-TLS absence is the expected nonzero Cargo no-match refusal,
`package ID specification native-tls did not match any packages`. The shell
probe accepts only that text with nonzero Cargo exit, not arbitrary failure.
Bundle compile exits 0: 76 policy valuations, zero unruled, no policy-sweep
warning. Selftests confirm refusal/pass arms; seal proof reports seven cases
and ten mutants, keyless. No operator-key signing proof is inferred.

### Discriminating final-commit mutations

| Mutation / receipt basename | Owning evaluator | Red → restored exit |
|---|---|---|
| `needle-panic` | `cargo test --locked --test compat too_complex_needle_refuses_on_the_wire` | 101 → 0 |
| `compile-worker` | `cargo test --locked --lib compat::page_work_tests::needle_compilation_leaves_the_async_worker` | 101 → 0 |
| `compile-worker-hog` | `cargo test --locked --lib compat::page_work_tests::needle_compilation_leaves_the_async_worker` | 101 → 0 |
| `leak-dns` | `cargo test --locked --lib dns_failure_causes_keep_no_userinfo` | 101 → 0 |
| `leak-tls` | `cargo test --locked --lib tls_verification_failure_causes_keep_no_userinfo` | 101 → 0 |
| `links-searched` | `cargo test --locked --test compat extracted_only_fields_miss_both_wire_searches` | 101 → 0 |
| `codes-searched` | `cargo test --locked --test compat extracted_only_fields_miss_both_wire_searches` | 101 → 0 |
| `serialization-only` | `cargo test --locked --lib compat::page_work_tests::held_single_message_builds_leave_smtp_and_health_serving` | 101 → 0 |
| `prefix-false-success` | `cargo test --locked --test compat shim_delete_preserves_committed_prefix_across_reopen_and_retry` | 101 → 0 |
| `prefix-resurrected` | `cargo test --locked --test compat shim_delete_preserves_committed_prefix_across_reopen_and_retry` | 101 → 0 |
| `whole-attribute-tolerance` | `cargo test --locked --test compat nested_attribute_urls_and_curly_suffixes_survive_fresh_ingest` | 101 → 0 |
| `old-curly-trim` | `cargo test --locked --test compat nested_attribute_urls_and_curly_suffixes_survive_fresh_ingest` | 101 → 0 |
| `entity-wrong-scalar` | `cargo test --locked --lib all_whatwg_entities_match_snapshot` | 101 → 0 |


### Parent integration and remaining obligations

`docs/decisions/README.md` is outside lane D's ownership; the chief explicitly
requested a parent-owned index row. Add exactly one resolving row for
`0061-compat-webhook-proof.md`, retaining **proposed** status, for example:

```markdown
| [0061](0061-compat-webhook-proof.md) | Compat refusal recognition and extraction repair boundaries | proposed — awaiting the operator | (#170/#190, lane D) |
```

Then rerun both documentation tests, every full house gate including exact
coverage/selftest, and the immutable configured verify chain on the final
integrated candidate. Only the operator can accept 0061 §1 or rule separately
on R4.3/R4.4. R4.2 is not silently reclassified as an implementation detail.
All three criteria remain open, owned follow-up work, never waived or closed.

Four configured independent review lenses and deterministic verify/ship
remain engine-owned and outstanding. No reviewer was invoked/substituted by
this seat; no security hold is overridden. CI-only chart verification/history/
selftest, Docker persistence/context/anonymous-volume smoke, RustSec advisory
audit, tag-only release-rate-5k and real port-80 exchange are unproved here.
No release edit calls for a local chart-bump proof. Historical snapshot and
canonical active journal are unchanged; no import/supersede/release receipt.

Changed code paths: src/compat.rs, src/compat/page_work_tests.rs,
src/extract.rs, src/webhook.rs, tests/compat.rs, src/store.rs (comment only).
Documentation: this spec tree, the three historical extraction-compat files,
and the dated 0049 addendum. No SMTP lifecycle, Store algorithm, entity data,
package attribution, Cargo/dependency/license, Docker, #178 helper or gate/
recipe work. A/E/G ownership stays intact; parent owns index/integration.

Accepted 0001–0005 remain: synchronous insert/queryability before SMTP 250,
subscribe-before-check awaits, oldest-only pruning and fresh-ingest
extraction. Existing single-writer/FULL commit, source-body/raw truth,
verbatim historical arrays, linear scan accounting, code cap, partial shim
committed prefix, no-proxy/no-redirect rustls verification and error contracts
remain. Uncapped link amplification, aggregate blocking-pool contention and
best-effort webhook loss stay existing costs, with no new resource/delivery
promise. Only compatible extraction corrections land; 0061 stays proposed.
Unsigned commits; no push, merge, tag, publication, external comment or issue
closure. The documentation/evidence commit follows the code commit; final
hashes and result-file path are in the engine handoff.


## Implementation retry — 2026-10-23 (4537c5cc)

The automatic `IMPL-BROKEN-RETRY` starts at unchanged clean HEAD
`201be49d1b654bd2be569d0e3270d84cce050aea`; no parent index repair or new
operator amendment was supplied. Re-read the triage framing, chief artifacts,
actual deterministic `designed` result and previous handoff before proceeding.
No new implementation is required by this retry's evidence.

New receipts live under
`.forge/epic192/compat-webhook-proof/implement-retry-4537c5cc/`:

- `retained-evidence-audit.json` confirms all nine implementation/input hashes
  still match, all 13 behavioral-red/restored-green mutation receipts match
  their summaries and current restored source bytes, and all 17 gate-log
  hashes match their retained receipts. These are retained executions, not
  fresh gate/mutation passes. The inherited generator proof is not repeated.
- `cargo test --locked --test store_contract_docs` freshly exits **101**:
  **17 passed, 2 failed**, both naming
  `0061-compat-webhook-proof.md: no index row`. Exact command, environment,
  timestamps, exit and log SHA-256 are in `store-contract-docs.json`; actual
  output is `store-contract-docs.log`.

The parent integration request above is still actionable: add the one
**proposed** 0061 index row in `docs/decisions/README.md`. That path remains
outside this lane's edit ownership; neither delete the chief's required ADR
nor change a gate/test to hide its missing index entry. The earlier full
workspace/coverage refusals remain unresolved. Repeating unaffected full
gates cannot repair the reproduced prerequisite; this retry reruns the
owning evaluator and retains the other receipts without relabeling them.
After parent repair, all required full gates and immutable configured
verification still require a clean final-tree result.

Return **broken** for the unresolved gate failure. This is not an oversized
scope finding or a frozen-guarantee refusal. T14/T15 and R4.2/R4.3/R4.4
remain open exactly as above; no whole-story, review/ship, CI-only, operator
acceptance or publication claim is added. Existing compatible code and all
accepted contracts remain unchanged.


## Operator-ruling continuation — 2026-10-26

Current authority and scope come from the operator's supplied Decision rulings
of 2026-10-26 and the current implement task, as framed in
[intake-2026-10-26.md](intake-2026-10-26.md). They supersede this plan's earlier
pending-authority and ownership limits for AC10–AC12. Earlier evidence,
failures and unresolved-at-that-time statements above remain historical.

Use src/compat.rs for refusal-only normalization and src/extract.rs for
source-aware conditional-comment attributes. Keep src/smtp.rs unchanged
after the temporary source-order negative proof. Use tests/compat.rs for
all four named wire/reopen regressions and measured extractor unit controls
for lexical boundaries/decode-once. Change only 0061's current status/index
row and append 0061/0049 acceptance/ruling provenance; accept nothing else.
Update the current spec and task disposition and commit the evidence ledger.

The current recipe routing, dependency tree, gate/shim bytes, extraction
call sites, synchronous insert, watcher order and pruning remain unchanged.
House and configured supplemental gates run on the final candidate with
the checked local .forge/bin/brokkr; exact coverage and its selftest remain
mandatory. The [proof ledger](proof-2026-10-26.md) records commands, results
and immutable source identities. Protected phases are still engine-owned;
CI/operator-owned evidence is listed in the current spec without new local
acceptance criteria.
