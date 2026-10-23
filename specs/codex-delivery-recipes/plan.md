# Plan — Codex delivery recipes

## Technical context

Implement [spec.md](spec.md) under accepted 0057 in the existing single Rust
crate. Implementation is authored but uncommitted: four composed bundles,
JSON offices/adapters/policy, Markdown charters/docs, shell proof seats, Rust
integration regressions, a stdlib-only Python source-patch helper and CI wiring.
Finish only demonstrated non-pinned test/document gaps. No production dependency, package, mail schema, API,
feature flag or runtime change is needed.

Use `.forge/bin/brokkr`, whose frozen tooling record identifies source
base `597f4868e3fb971b62e6a368a2d8aa23271f4531` PLUS the ordered patch
`bundles/self/tooling-patches/0001-preserve-review-residual-facts.patch`, SHA-256
`5d4da2217135c5a2ca14354ae9907623cf67ff0076b078277cb77a09ad3020b5`,
and binary SHA-256
`517c22680beaaf354f6ee3d01e4b964669ecd030a3ea4d9d4554d88e50c83bf0`.
This is explicitly patched source, never a clean-base identity. The archived
base executable must not be selected.
Verify bytes before execution; version 0.12.0 alone is insufficient.
Prepared tools include Rust 1.98/stable, nightly 2026-10-22 with llvm-tools,
cargo-llvm-cov 0.9.1, cargo-deny and locked caches. Availability is a prerequisite,
not proof of passed gates. Offline verify installs nothing.

There is no repository plan-template file (searched `.specify/` and tracked
paths). This plan supplies the technical context, constitution check, project
structure, research, data model, contracts and quickstart required by
`dialects/speckit/design.md`, without adding scaffold or extra artifacts.

## Constitution check

The bootstrap takes `engine` through full design and four reviewers. 0057
already authorizes direct bug lanes; implementing its terms needs no new ADR.
Preserve 0001–0005, protected independent review, hard security hold,
explicit-safe residual admission, and one transitive pinned proof owner under
0007/0050. Preserve 0017's delivered-result and immutable-pin boundaries and
0056's unsigned-author/no-push discipline. Historical accepted bodies stay
intact; existing dated 0057 markers are implementation inputs.

All frozen resources remain byte-identical, even prose under `bundles/`.
If an obligation needs a frozen-resource change, record its path, failing
evidence and required repair; the assistant must stop and prepare another
pin/run. No workaround, different engine or digest waiver.

## Project structure and touched files

| Paths | Responsibility and write boundary |
|---|---|
| `specs/codex-delivery-recipes/{spec,plan,tasks}.md` | Chief's entire commit; later implementation evidence edits precede final verification |
| `bundles/{self,codex,codex-careful,codex-fast}/`, `bundles/README.md` | Prepared compositions, policy, shared design, proofs, tooling and docs; review/commit frozen bytes unchanged |
| `agents/codex-*.json`, `agents/charters/codex-{triage,fast-implementer,bug-reviewer}.md` | Existing work/review overlays, triage, scope and singleton security contract; frozen |
| `agents/review-correctness.json`, `adapters/dsh.json`, inherited offices/adapters/charters | Effort correction, route declarations and inherited contracts; frozen |
| `recipes`, `realms.json` | Relative `recipes -> bundles` alias and standalone realm/journal; preserve prepared discovery inputs |
| `tests/delivery_recipes.rs`, `tests/delivery_constitution.rs` | Structural regressions; extend only missing non-pinned obligations |
| `bundles/self/tooling-patches/0001-preserve-review-residual-facts.patch` | Prepared aggregation repair and upstream regressions; frozen source input |
| `scripts/apply-brokkr-patches.py`, `tests/tooling_patches.rs` | Authorized stdlib helper and nine real-Git tests; no application dependency |
| `.github/workflows/ci.yml` | Apply exact ordered patches, run engine core/aggregation tests, build composite, compile all four compositions |
| `README.md`, `agents/README.md`, `docs/house-rules.md`, `docs/decisions/README.md` | Authorized docs; finish non-pinned launch/evidence guidance |
| `docs/decisions/0057-codex-delivery-recipes.md`, dated markers in `0007`, `0009`, `0013` | Commit authorized record/markers without accepted-body rewrites or invented acceptance |
| `.forge/tasks/`, `.brokkr/state/tasks/`, both position locations, `.forge/results/`, real journal/ledger | Ignored run-local evidence; never stage or fabricate historical records |
| `.forge/epic192/{bootstrap-v1,bootstrap-evidence,policy-probe,pin-repairs,assistant-review-probe,provider-smoke}` | Existing ignored receipts; retain provenance/limits |
| `.forge/epic192/active-engine.json`, `.forge/epic192/history/` | Activation identity and immutable recovered audit snapshot/receipt/ruling index; never use the snapshot as an active journal |

Implementation reviews an explicit file list including all fast additions;
no wildcard staging substitutes for review. Chief stages only its three files.
The old exclusion of fast work is replaced by the current authorized inventory.
No `src/`, Cargo manifest/lock, allowance or coverage-code changes.

## Research and current observations — 2026-10-23

Chief read current `64fd181b` framing, both new positions, the decision
index and 0007/0009/0013/0017/0050/0056/0057. Direct spot checks covered the
fast policy, ship/check scripts, tooling and source aggregate, patch helper,
test/CI inventory and named receipts. Binary and patch hashes match the
identity above. These are inspection findings, not final gate or live review
verdicts. Historical observations from `7b47790` and `8cb8bb9` remain in Git
and stopped-run evidence; obsolete observations are not current claims.

| Evidence inspected | What it establishes; remaining limit |
|---|---|
| Current fast policy | Clean/safe-residual scope returns cover chore/feature/design/engine; all ship allows require false scope/fixes. Third triage visit and repeated oversized stop. Aggregate defaults can still erase member uncertainty. |
| Declared composite `aggregate.rs` | `review_panel` emits false fixes unconditionally; scope joins with `as_bool().unwrap_or(false)`. An earlier mandatory validator has not been established. This is a blocking source-level finding, not a new CLI bypass claim. |
| `assistant-review-probe/patched-release-receipt.json` | Reports four malformed residual CLI cases changing completion to stop, two valid completions, seven aggregate and 33 core tests. Exec fixtures, not live providers; scope/fixes matrix remains unproved. Existing three compiling-mutant red/restored-green evidence is reusable only with checked identities. |
| `policy-probe/receipt.json`, `policy-inputs.sha256` | Reports 58,268 evaluations, zero failures, recorded resolver/core and source/lock provenance. Chief's checksum check matched tables/bundles/patch, failed tooling.json only. Receipt is staged/unactivated with old tooling digest and a historical missing-journal statement. Preserve it; explicitly reconcile activation via active-engine/build records and current resolved identities, or rerun the existing harness. It does not prove aggregation or dispatch. |
| Current ship script and `pin-repairs/ship-probe-receipt.json` | Script emits shipped only on clean matching close-out, exits 2 otherwise. Earlier receipt's engine was the archived base: unchanged-script regression evidence only. Ready may carry discrepancy notes and is not completion; repeat required dispatch boundaries on the active composite. |
| `provider-smoke/summary.json` | Qwen/GLM/official Flash returned HTTP 200; Opus CLI 2.1.309 exited 0. Availability only; no Brokkr dsh dispatch or protected review. No old Claude version is live proof. |
| `history/snapshot-receipt.json` and intake's ruling inventory | Recovered snapshot records integrity OK and 67 runs; supplied ruling index records 24 rulings/96 notes. Read-only audit evidence; no run/resume/operator/import, remote mutation, inferred acceptance or supersession. |
| Prepared tests/helper/CI | Twelve recipe tests include real-shell ledger controls; nine real-Git helper tests cover exact source/root, tracked cleanliness, strict JSON, safe paths, all hashes before mutation and buffered order. CI applies the helper and runs actual engine regressions. Inspection is not this revision's execution. |

Prelaunch sweeps report 38 valuations for self/codex/careful and 304 for fast,
zero unruled and no warnings. Final-candidate compilation still needs receipts.
Legacy `.brokkr` framing/positions coexist with `.forge` journals under the
frozen charters; prove canonical discovery without deleting evidence or
inventing adopter configuration. Frozen script prose suggesting push/merge
never overrides 0056 or this task. Cosmetic descriptions can wait for another
pin; behavior/contract mismatches cannot be excused as prose.

## Implementation approach

1. Reconcile actual pin and authorized inventory, preserving all pending work.
   Compare bytes, model/effort resolution, inheritance, helper origins, symlink
   and realm. Confirm 0057 provenance and dated markers. Report necessary
   repairs before any completion result. First resolve the blocking member-
   fact obligation below before expensive final gates or live provider review.
2. Discharge the member-fact blocker first using the exact engine and disposable
   fixtures in existing evidence locations. On singleton/paired/four-member
   clean and residual paths, submit absent inputs, omitted/null/wrong-typed
   scope/fixes, true scope/fixes, conflicting members and explicit-false
   controls. Observe the mandatory validation boundary and consumed inputs,
   aggregate and final destination. Unknowns stop; known defects cannot ship;
   witnessed edits cannot become false, including edits committed to a clean
   tree. Retain hold and above-medium precedence controls. If an earlier
   validator does not refuse, report `crates/brokkr-runtime/src/bundle/aggregate.rs`
   (or its member validator), the declared patch and tooling identity as the
   required new-pin repair. Stop; do not alter those bytes or add a bypass.
   Supplement actual 1/2/4 selection, missing/partial/stale/misassociated output,
   first/repeated scope growth, all-strategy scope returns and third-visit
   refusal. Restart must preserve run/pin/attempt/visits and use the newly ruled
   strategy without downgrade. Selection fixtures are not live deliveries.
3. Extend existing recipe/constitution suites only for uncovered obligations:
   fast hands/efforts/charter scope, historical self routes, Opus/Flash high,
   omitted DashScope effort, standalone realm/journal behavior and preserved
   protected gates. Use disposable wrong-value controls for demonstrated gaps,
   never frozen originals. Run the existing nine helper tests as well;
   preserve exact-HEAD/clean tracked source, all-hashes-before-mutation and
   verified-buffer ordering. No locally cloned policy evaluator. A later
   patch apply failure may leave earlier patches applied: refuse that checkout
   for build/certification; the helper does not promise rollback or attest
   untracked files. No additional patch-management framework.
4. Compile/discover all four with the recorded engine. Compare named/explicit
   identities and inherited script origin; resolve standalone without siblings.
   Retain raw logs/exits and inspect warnings even when compile exits zero.
5. Validate the exact-core probe's base-plus-patch source, dependency/lock subset,
   resolved input hashes and cases. Account explicitly for the stale tooling
   digest and staged metadata using the activation/build chain; otherwise
   rerun the same harness on the active executable. Never rewrite old receipts.
   Supplement gaps through that recorded core outside
   application dependencies. Cover strategies, absent/null/wrong-typed selector,
   safety/scope/fixes facts, all valid severities, first-match precedence,
   clean/residual admission, holds, visit limits and unknown stops.
6. Prove ship refusal boundaries through the recorded engine in disposable
   contexts: dirty index/worktree, changed HEAD and empty/missing/mismatched
   ledger. Capture fact origin and final rule. Core rows or happy-path stdout
   do not establish engine enrichment. Missing refusal requires a new pin;
   do not edit ship here.
7. Finish operator guidance on classification, launch, eight-run isolation,
   integration, dsh limits, unexercised variants and repair ownership. Use
   existing journal/log/ledger evidence, not another evidence system.
8. Run the complete local inventory and supplementary proofs on final intended
   bytes. Reconcile implementation-owned evidence/tasks, commit all authorized
   work unsigned with rationale, and bind receipts to the candidate. Any later
   tracked change needs renewed candidate proof. Gate failure grants no scope
   to repair epic runtime code.
9. Actual inherited offline verify proves the committed candidate with pinned
   helpers and checked shims. Then protected review receives four live member
   results with authorized routes, stable tree and valid worst-verdict handling.
   No fallback; provider failure stays outstanding. Do not require another full
   paid careful delivery merely for an effort variant.
10. Actual ship renders the genuine journal and closes with nonempty ledger,
    matching HEAD and directly checked clean tree/index. Later receipts remain
    ignored; do not dirty the proved candidate to tick future boxes. The result
    is a verified local branch for operator sealing, never a push or merge.

## Data model and contracts

No mail migration or public protocol change. Existing recipe composition is
the configuration model; `strategy` selects the ruled lane. Triage owns
framing, journal owns visit/attempt facts, and policy owns transitions.
Results reach the engine through assigned files, not final chat prose.
Missing, partial, stale or wrongly associated output cannot count as success.
Preserve pin/run/attempt identity across restart.

Chief writes exactly `result`, `inputs`, `notes` with `result: drafted` to
its assigned path through workspace hands, atomically after committing and
checking artifacts. It does not select the next phase. The final deterministic
exec validates chief's HEAD. Review is read-only; the single bug reviewer or
panel security member owns holds in its respective lane.

Source provenance comprises base SHA, ordered patch paths/digests, compiler
and executable digest. The helper validates every record/hash before mutation
and applies those verified buffers in order. Review facts belong to each
member before joining; missing scope/fixes cannot acquire safe aggregate
defaults. Journals must preserve both member evidence and consumed aggregate.
Candidate identity includes HEAD and tree across verify/review/ship; clean
status alone cannot detect a reviewer committing an unauthorized edit.

Receipts identify candidate, pin/engine, run/attempt, command/provider,
exit/verdict and result/log location without credentials. These contents are
not a new durable schema. Track implementation evidence before final verify;
later results remain ignored. A checked task needs evidence; future unchecked
phase tasks at implementation commit are intentional and are assessed from
subsequent receipts during close-out.

## Quickstart and validation

Stay on the prepared branch. Compare the executable SHA-256 against frozen
`bundles/self/tooling.json` before use; refuse mismatch. Then:

```bash
cargo test --locked --test delivery_recipes
cargo test --locked --test delivery_constitution
cargo test --locked --test tooling_patches
.forge/bin/brokkr compile --bundle bundles/self
.forge/bin/brokkr compile --bundle bundles/codex
.forge/bin/brokkr compile --bundle bundles/codex-careful
.forge/bin/brokkr compile --bundle bundles/codex-fast
.forge/bin/brokkr recipes show codex
.forge/bin/brokkr recipes show codex-careful
.forge/bin/brokkr recipes show codex-fast
```

Execute the complete single [house inventory](../../docs/house-rules.md#gates--part-of-every-change)
with the recorded engine on PATH. Do not duplicate or reduce that inventory.
Actual verify checks public shims and runs pinned implementations. Supplementary
proofs include design selftest, tamper-boundary and fail-closed native-tls.
CI retains chart/history/selftest, real Docker, advisory, tag-rate and real
port-80 ownership; do not claim them locally without receipts.

After recipes are reviewable, future launches may use
`.forge/bin/brokkr run --recipe codex-fast --db .forge/forge.db --feature '<scoped story>'`
from each isolated worktree. `codex` and `codex-careful` remain explicit full
route choices. Reserved `.forge/epic192/wave1.json` and `feature-A.txt` through
`feature-H.txt` are not launched by chief or as bootstrap acceptance. Reserve
overlaps, separate writable targets/cache overlays/coverage, journals,
ports/inboxes/data files and evidence, and throttle heavy gates to resources.
Integration reconciles overlaps and reruns gates. Eight is the trial target
and ceiling, not a throughput guarantee.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Cheap classification misses sensitive work | Evidence burden, scope review, full-lane examples and bounded escalation; uncertainty stops |
| Restart resets counters or uses stale strategy | Actual journal/restart/selection proof; no downgrade; missing proof blocks delivery |
| Aggregate erases unknown scope or witnessed fixes | Blocking member-level 1/2/4 exact-engine probes before certification; preserve candidate identity, stop for new pin if no mandatory validator exists |
| Inheritance relocates relative proof script | Inherit complete seats; prove resolved origin and exec mount; no copies |
| Structural/core checks masquerade as execution | Separate structural, evaluator, dispatch and live receipts |
| Ship labels conceal discrepancies | Real failure probes, fact provenance and clean close-out; new pin if broken |
| Legacy mirrors affect host discovery | Preserve evidence; prove standalone/host restart; report path repair |
| Provider/effort differs from declaration | Live selection receipts; no DashScope flag, fallback or Pro; outages stop |
| Dsh cannot restrict tools natively | Honest conduct limitation, fixes facts and tree checks; drift refuses |
| Stale tooling digest or checkbox edits prove wrong candidate | Explicit activation/source/input reconciliation or rerun; preserve old receipts; final verify after edits; later receipts ignored |
| Ordered patch application fails halfway | Refuse partial source for build; use a fresh exact checkout; no rollback or untracked cleanliness claim |
| Frozen resource needs repair | Stop for operator-prepared new pin/run; no alternate engine |
| Eight runs collide or exhaust resources | Separate mutable state, reserve overlaps, throttle and rerun integration; no scheduler |
| Gates expose epic runtime defects | Preserve exact coverage/dependencies; report failure and defer scoped repair |
