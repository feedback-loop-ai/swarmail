# Implementation plan — compat, webhook and extraction proofs

Date: 2026-10-23. Design only. The [spec](spec.md) owns requirements; [tasks](tasks.md) pairs work with proof. Proposed [0061](../../docs/decisions/0061-compat-webhook-proof.md) separates compatible repairs from an unaccepted R4.2 amendment. R4.3/R4.4 are independently pending. No runtime, mutation or delivery-gate result is claimed by this plan.

## Technical context

One Rust crate; Tokio/Axum servers, regex literal compilation, reqwest/rustls delivery, SQLite persistence, existing tokenizer/entity decoder. No new dependencies, features, config, schema or license allowances. Real SMTP/HTTP clients and isolated SQLite files are already available. Default tests stay inline and measured; cold watchdog paths remain in the declared src/compat/page_work_tests.rs harness under accepted 0020. Test-only observations must be request/store-local, never shared mutable global state.

No plan template exists in the inspected .specify/templates or repository file inventory; this plan supplies the required technical context, constitution check, structure, research, data model, contracts and quickstart directly. The unrelated workspace-oriented spec template does not introduce another repository into this single-crate task.

## Research and current evidence

At base 168ac299e0fe16ecdd8fa0e0218cb7bc4e0a261c:
- src/compat.rs search_mailpit/search_hog call stage_search before page_work. search_needle is also public bench surface; preserve its signature/semantics. Mailpit errors use JSON key error with the complete existing sentence, including the Error: prefix; MailHog maps its own failed staging to bare 400.
- render_one already wraps Find/Build in Store::offload; the existing gate runs after find. That placement alone cannot prove both callbacks remain off-worker after refactoring.
- post_json executes its built request then formats with_causes after without_url. reqwest may transfer URL credentials into Basic auth before execution, so simply removing without_url may be an inert mutation.
- LinkRegions::value currently applies ATTR_URL_RE to the entire decoded attribute value; trim_url removes only ASCII bytes. NAMED actually lives in src/extract/entities.rs.
- delete_messages awaits per-id deletion and stops at first failure. Existing wire coverage uses one doomed id/first-row refusal. The emptied-entry test is a_delete_that_empties_an_inbox_removes_the_entry_live_and_after_reopen.
- Accepted 0049 §6's ordered wrapper rule is explicit; R4.2 changes it. This plan does not infer acceptance from the unsupported-prefix intent in 0044.

The prior-run intake at .forge/tasks/epic192-first-wave-lane-d-compat-0b5b1027.md records actual generator execution (exit 0, 60,746 bytes, complete NAMED byte equality) and a snapshot-plus-space refusal (exit 1, zero stdout, rejected digest d2772fd6c4b6368df9b1b9510b9b750196c12dfaedc74159630bcfd535735e50). The current intake mirrors it. Chief rechecked these hashes with sha256sum, exit 0:

| Input | SHA-256 |
|---|---|
| scripts/gen-entities.py | 80541b78d56ec6ac6318adefc5bb8033f2c487fec375ad9118339b3856fb36d8 |
| scripts/data/html-entities.json | d741d877ac77c4194c4ad526b5b4a19aef8dfe411ab840a466891cdbb9f362e6 |
| src/extract/entities.rs | 4334e2e8f0af1ba8a6c0a65fa0b716f51a7ee70867018e350ff0b4d154cc6335 |

Reuse that actual dated receipt while inputs match; do not rerun only to rename its date. It does not prove the unexecuted wrong-table mutation or any runtime criterion. Do not claim the issue comment's fresh network download as a local receipt.

## Constitution check

Preserve accepted 0001–0005; ordered synchronous mutation and FULL commit; single-writer lock; subscribe-before-check even after channel closure; acceptance-order pruning; genuine source bodies and fresh-ingest extraction; historical restore verbatim. Offload is awaited, not detached, and does not promise a latency ceiling or admission budget. Search scans and response construction hold no map guard or mutation lock.

0045 §8 allows compatible code under a proposed record, not contradictory code. Apply R4.1/R4.5 as explicit compatible lexical/punctuation clarifications; hold R4.2's normalization until its exact amendment is accepted; hold R4.3/R4.4 as instructed. Accepted bodies remain historical. New 0061 stays proposed; no index/acceptance-marker edits outside ownership. Request a parent-owned index entry if needed by the implementation's documentation checks.

All mandatory house gates remain required before implementation success, with --locked wherever applicable, exact missed == 0 on lines/branches/functions, no exclusions or denominator changes. The full engine lane retains independent Qwen Max correctness, Claude Opus security, official DeepSeek Flash guarantees and DashScope GLM 5.3 tests; no fallback or Pro. No nested run, publication, push, merge, tag, issue close or external comment.

## Project structure and touched files

| Path | Responsibility / boundary |
|---|---|
| specs/epic192-compat-webhook-proof/{spec,plan,tasks}.md | The three chief artifacts; later record accurate clarification/evidence without erasing pending obligations. |
| docs/decisions/0061-compat-webhook-proof.md | Proposed R4.2 amendment, compatible clarification boundaries and separate operator residuals. |
| src/compat.rs | Move staging into existing offload, preserve dialect mapping; local compiler/Find/Build observations if needed; R4.2 only after acceptance. |
| src/compat/page_work_tests.rs | Strengthen actual-work placement proof, eleven real routes, compiler/runtime coordination and cold cleanup. |
| tests/compat.rs | Complex needle wire refusal, extracted-only misses, selective persistent prefix/restart/retry, conditional R4.2 matrix. |
| src/webhook.rs | Production execute/format owner and real-client internal fixtures; one private factoring seam only if required. |
| tests/webhook.rs | Reuse existing real TLS/client fixtures where public access suffices. No #178 sink/budget helper work. |
| src/extract.rs | Leading-value versus nested URL delimiters, UTF-8 suffix trim, deterministic exact-work and fidelity tests. |
| src/smtp.rs | Only exact extraction fixtures / ingest wiring required here. No accept/drain lifecycle edits; BodyOrder link wiring waits for R4.3. |
| tests/smtp_edges.rs | New exact extraction SMTP/REST cases if existing owned fixtures cannot house them. Existing tests/api.rs #136 runs unchanged. |
| src/store.rs | Existing emptied-entry test comment only; G/parent owns all additional store/seam work. |
| scripts/gen-entities.py | Proof/fix only if evidence identifies a generator defect; none currently established. |
| src/extract/entities.rs | Temporary wrong-mapping proof input, exact restoration; no permanent edit planned/authorized by current evidence. |
| specs/extraction-compat/{spec,plan,tasks}.md; docs/decisions/0049-extraction-compat.md | Implementation appends dated corrections/proof links, retaining historical dates and bodies; only evidence-backed task completion. |

No Cargo, README license, Docker notice, entity snapshot, policy, tooling, bundle, recipe, shim or shims.sha256 edits. A owns package attribution; E owns SMTP lifecycle; G owns broader store work. Send unsupported changes to parent rather than expanding ownership. The design commit itself touches only its three artifacts plus 0061, never the historical spec directory.

## Implementation approach

1. Freeze evidence identity: record base/current SHA, current authority, ownership and input hashes. Preserve prior stopped run evidence. Establish named tests and run-local receipts under .forge/epic192/compat-webhook-proof/ with commands, exit codes and hashes. No journal writes or fabricated live-phase receipts.
2. Search: retain dialect kind/cap/scope/prefix validation order. Move stage_search into the existing page_work closure before run_search; return a fallible finished result. Mailpit preserves ApiError, MailHog retains separate bare staging refusal and named OverBudget mapping. One blocking hop carries lowering/compilation, scan, projection and serialization. Keep public search_needle and simple case-fold literal semantics.
3. Establish a transport-reachable over-complex fixture by measuring request/encoded size and exercising the real handler; report actual size, not an assumption about the 250,000-k helper fixture. If no admitted literal reaches the existing compiler budget, report a concrete AC1 feasibility residual instead of lowering a production limit or using a parser error as the refusal.
4. Instrument compiler placement at actual invocation, scoped to the operation; exercise it through the production search path on a current-thread runtime. A per-invocation callback/test-only wrapper may observe thread identity and coordinate hold/release. Do not add Store fields. If a store change is essential, request parent/G ownership explicitly while completing independent work.
5. Rendering: exercise render_one's actual Find/Build callback execution with thread identity observations, and retain real-route lookup/latest/projection/render/copy/serialization evidence. Reuse existing per-store gate as coordination, not as the sole assertion. Hold actual operations, observe real SMTP greeting/health, verify completed responses and immediate queryability after an independent DATA 250. Keep cold watchdogs in the existing harness; their role is terminating a bad mutant, not measuring performance. Ensure the serialization-only mutant leaves the old gate offloaded and still fails.
6. Webhook: begin with the existing production path. If deterministic client selection needs a seam, extract one private execute-and-format function taking client/request inside webhook.rs, called by post_json and the fixtures. Keep the boundary builder's rustls/no-proxy/no-redirect policy. A request-local reqwest resolver reports a controlled resolution failure consumed by the real connector; assert it was invoked. A loopback rustls sink uses generated certificates and a client that rejects the intended certificate; assert verification failure rather than generic TCP close. Keep informative cause strings. For negative proof, explicitly restore the original fixture URL metadata before the real formatter if Basic auth extraction otherwise makes the mutation inert.
7. Non-candidate search: use the existing public Store insertion on an isolated test server with two extracted-only sentinels and distinct ordinary candidate controls. Assert actual stored arrays and both wire search counts. No injection endpoint or query-time extraction.
8. Prefix proof: create a unique persistent server/database and ordered A/B/C records. Install a real SQLite trigger conditioned on B, inside tests/compat.rs; preserve A's deletion and B/C after 503 and restart. Release server/Store before reopening, retain sidecars under the temp guard, drop the trigger, retry the original list twice and verify an unrelated inbox. Mutate only lane-owned shim behavior for swallowed-error and prefix-loss proof; if a proposed mutant needs store ownership, coordinate rather than editing G's work.
9. Extractor: retain source-boundary recognition before decode. Decode each quoted value once. Apply tolerant matching only to the leading URL after optional whitespace and ordinary delimiters to later/deeper candidates; do not accidentally apply tolerant matching to every find_iter result. Keep monotone source/candidate cursors and visit the second CSS URL. Extend trim_url by whole Unicode scalar suffix widths for U+201D/U+2019, keeping the one-counting-pass punctuation balance and byte-based counters. Preserve every encoded-quote, raw-region and exact-work control. Add all AC9 fixtures via owned SMTP/REST tests.
10. Entity proof: run the existing independent all_whatwg_entities_match_snapshot evaluator green, replace one mapping with a wrong scalar without changing table cardinality, run it red, restore byte-identical source and rerun green. Back up only the exact file, use unconditional restoration, record hashes and check diff afterward. No blanket git reset/checkout, no second checkout, no change to snapshot or attribution.
11. Documentation: name the map-guard hang in its existing comment. Append dated 0049 §1 and historical spec corrections only after actual proof. Keep old receipt dates and distinguish generator equality, tampered snapshot, wrong mapping and runtime verification. Do not mark whole T11/T12 done on a generator receipt alone. Preserve unresolved R4.2/R4.3/R4.4 prominently.
12. After exact authority arrives, implement only the accepted clause: R4.2 refusal-only normalized token with original execution input untouched; R4.3 transient BodyOrder link wiring and source-order proof; R4.4 the specifically accepted conditional-comment treatment or explicit operator-accepted retention documentation. These are conditional follow-up tasks, not prerequisites for independent compatible work or authority granted here.

## Data model and contracts

No persistent data model changes. Email.links/codes are existing stored ordered arrays derived once; historical rows stay verbatim. The transient Matcher remains request-owned and drops after the response; no regex cache. BodyOrder remains codes-only until R4.3 authority. Direct quoted URL values retain encoded quote characters; embedded nested quotes delimit their URLs. Ordinary comments continue to expose ordinary decoded source, including the pending Outlook case.

Mailpit and MailHog retain distinct refusal envelopes, validation precedence, page caps and the inclusive 67,108,864-byte MailHog page budget. Proposed R4.2 changes only recognition/refusal, not execution. Shim DELETE is an ordered partial commit; a 503 does not imply rollback. Webhooks remain best effort with unchanged retries, sink budgets, direct delivery and diagnostic cause depth. Credentials remain valid internally; public URL userinfo is absent. Queries/fragments retain their existing exposure.

## Failure behavior and preservation tests

| Stress/failure | Behavior to preserve / evidence |
|---|---|
| Crash after commit before SMTP reply | Stored mail may be retried/duplicated; never acknowledge uncommitted mail. Existing persistence/hard-kill suites remain required; no durability claim for memory-only restart. |
| Crash/reopen during failed shim batch | Exactly committed prefix absent, suffix present, retry convergent; new real SQLite/wire proof. |
| Hanging webhook or certificate failure | Existing timeout/retries/drop ledger and best-effort loss; no awaited webhook in acceptance or store lock. Preserve existing real webhook suite. |
| Cap under burst | Oldest acceptance-order eviction inside ordered mutation; view timestamp order never changes pruning. Preserve burst/rate suites and exact counts with cap disabled where required. |
| Hostile DATA / chaos | Bounded byte-honest reader, complete DATA boundary, parse/extract before insert, no insertion on EOF/refusal; chaos only 0/100 in tests. Preserve SMTP edge suites; no lifecycle edits. |
| Many simultaneous sessions | Unique inboxes, ephemeral ports, unique database dirs, per-request/store seams, no global environment mutation; no elapsed latency correctness ceiling. |
| Historical malformed extracted rows | Restore unchanged; repairs affect fresh ingest only. No migration or hidden read-time repair. |

## Quickstart and validation

These commands are planned, not execution receipts. Use the launcher's private target and installed pinned toolchain/engine; do not install or substitute a version-equal binary.

Targeted proof entry points (resolve exact individual selectors into the receipt):
```bash
cargo test --locked --test compat
cargo test --locked --test webhook
cargo test --locked --test smtp_edges
cargo test --locked --test api smtp_issue_136_link_fidelity
cargo test --locked --lib compat::page_work_tests
cargo test --locked --lib extract::
cargo test --locked --lib all_whatwg_entities_match_snapshot
cargo test --locked --lib a_delete_that_empties_an_inbox_removes_the_entry_live_and_after_reopen
```

Run each negative mutation separately against its owning real evaluator/wire test, restore exact bytes even on test failure, and rerun the same selector green. Capture mutation patch/hash, command, exit status, intended failing assertion, restoration digest and final diff. Compile failure alone is not a behavioral red. Never claim a mutation that did not affect sensitive input or actual work placement.

Final implementation inventory:
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
bash scripts/seal-branch-test.sh
cargo deny check licenses
```

Configured verify additionally executes design-check selftest, delivery tamper-boundary proof, recipe structural tests and fail-closed native-tls absence proof. Only the expected cargo-tree no-match refusal establishes absence; first distinguish successful dependency resolution from arbitrary Cargo failures. The immutable baseline recipe is /home/valentin/source/feedback-loop-ai/swarmail/.forge/epic192/start-now/eight-lanes/baseline/bundles/codex-fast; launched engine is its baseline checkout's .forge/bin/brokkr. Verify executes pinned copies against the candidate and checks public shims. Record actual resolved script/binary paths and SHA-256 digests, results and logs. No assigned gate repair exists; if later explicitly assigned, both baseline and repaired candidate proofs/mutations are owed.

CI-only chart/history/selftest, Docker persistence/context/volume smoke, RustSec network audit, tag-only release-rate-5k and port-80 bridge remain separate obligations. Local tests do not establish them. Root journal and historical snapshot remain outside this seat's mutation scope. Parent integrates lanes and reruns gates; this lane never merges.

## Risks

| Risk | Mitigation and remaining cost |
|---|---|
| R4.2 is incorrectly treated as accepted | Explicit 0061 §1 authority gate; preserve ordered recognition until ruling. Five forms remain an open defect and must be disclosed. |
| R4.3/R4.4 disappear from scope | AC11/AC12 and conditional tasks stay open with exact fixtures and operator options; no whole-story completion. |
| One offload changes error precedence/type | Preserve dialect validation order and test exact JSON/bare errors, cap/scope/prefix controls and public compiler behavior. |
| Large query hits HTTP parser instead of compiler | Prove transport reaches intended compiler branch, record admitted size and exact error; report feasibility gap honestly. |
| Placement probe travels with serialization and lies | Observe actual compile/Find/Build sites and require serialization-only offload mutant red. |
| Holding a bad mutant deadlocks test | Non-runtime watchdog cleanup, event/thread assertions, existing cold harness; no sleep or performance ceiling. |
| TLS/DNS fixture flakes or proves wrong arm | Generated certificates, request-local resolver, real connector, explicit failure classification and independent useful cause assertion. |
| Credential mutation is inert after Basic auth extraction | Restore metadata containing fixture credentials deliberately before actual formatter; require exact confidentiality assertion red. |
| Literal normalization affects execution | Gated refusal-only temporary token, original accepted query retained, literal/URL/explicit-kind controls. |
| Nested URL fix breaks encoded quotes or linearity | Source boundaries first, decode once, leading-only tolerance, all AC9 rows, legacy pins and exact work counters. |
| UTF-8 trim panics / undercounts | Remove full scalar widths, assert bytes and balanced/interior punctuation, restore-old-trim negative. |
| Prefix proof only sees first-row failure | At least three ordered ids, trigger B only, live/restart assertions, retry plus unrelated inbox. |
| Entity evidence is stale or fabricated | Match three hashes; preserve inherited date; actual independent wrong-mapping evaluator and restoration. |
| Shared blocking pool saturation / link amplification | Keep no-lock awaited work; disclose unbounded aggregate contention and source-size link amplification. No new admission/memory guarantee. |
| Seam edits cross G/E/A ownership or shrink coverage | Invocation-local compat/webhook seams; store comment-only; SMTP extraction-only; no coverage exclusions or packaging changes. Escalate exact extra path to parent. |
| Proposal/artifact pass mistaken for verified shipping | Distinguish drafted, deterministic designed, runtime evidence, full gates, independent reviews and operator acceptance at handoff. |

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
