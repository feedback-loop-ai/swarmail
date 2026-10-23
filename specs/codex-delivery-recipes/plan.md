# Plan — Codex delivery recipes

## Technical context

Implement [spec.md](spec.md) under accepted 0057 in the existing single Rust
crate. Implementation is authored but uncommitted: four composed bundles,
JSON offices/adapters/policy, Markdown charters/docs, shell proof seats, Rust
integration regressions and CI wiring. Finish only demonstrated non-pinned
test/document gaps. No production dependency, package, mail schema, API,
feature flag or runtime change is needed.

Use `.forge/bin/brokkr`, whose frozen tooling record identifies source
`597f4868e3fb971b62e6a368a2d8aa23271f4531` and binary SHA-256
`744ea1f3c459d08f5b102659541e3349182bccd91f597ce7906d49cbed431ed4`.
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
| `.github/workflows/ci.yml` | Prepared four-composition compile loop; check all four are covered |
| `README.md`, `agents/README.md`, `docs/house-rules.md`, `docs/decisions/README.md` | Authorized docs; finish non-pinned launch/evidence guidance |
| `docs/decisions/0057-codex-delivery-recipes.md`, dated markers in `0007`, `0009`, `0013` | Commit authorized record/markers without accepted-body rewrites or invented acceptance |
| `.forge/tasks/`, `.brokkr/state/tasks/`, both position locations, `.forge/results/`, real journal/ledger | Ignored run-local evidence; never stage or fabricate historical records |
| `.forge/epic192/{bootstrap-v1,bootstrap-evidence,policy-probe}` | Archived history, baseline logs and independent evaluator evidence; retain provenance/limits |

Implementation reviews an explicit file list including all fast additions;
no wildcard staging substitutes for review. Chief stages only its three files.
The old exclusion of fast work is replaced by the current authorized inventory.
No `src/`, Cargo manifest/lock, allowance or coverage-code changes.

## Research and current observations — 2026-10-23

Chief read current framing, both positions, the decision index and
0007/0009/0013/0017/0050/0056/0057; inspected compositions, fast policy/charters,
tooling, recipe tests, realm/alias and design/verify/ship scripts. These are
inspection observations, not final execution receipts.

- Fast has no selector default. Chore uses Flash, feature Qwen/Opus,
  design/engine four members. Whole design/verify/ship seats are inherited.
- Policy bounds oversized returns and stops third-visit classification.
  Only chore review has `spec_defect` reclassification. Broader/full residual
  admission lacks fixes/scope predicates. Test real aggregation, external tree
  checks and a supported feature-scope refusal. Charter monotonicity is not
  table proof.
- Ship records discrepancies yet emits ready/shipped labels. Determine whether
  engine facts enforce dirty/drift and ledger/HEAD refusals in actual dispatch.
  This is an unresolved execution boundary, not a demonstrated bypass or waiver.
- Existing probe receipt/summary reports 16,388 cases, zero failures, exact
  recorded resolver/core `Machine::evaluate`, explicitly core-only. Chief read
  those reports but has not established full source/dependency provenance,
  final-candidate binding, aggregation or live dispatch. T05–T07 retain these
  obligations; the reported count alone proves no delivery verdict.
- Intake reports prelaunch compile valuations of 38 for self/codex/careful
  and 304 for fast, no unruled rows or warnings. These reports and old pre-fast
  logs are not final-candidate proof. All four must actually compile.
- Legacy framing/position mirrors coexist with `.forge` journals because
  frozen charters require them. State-only `.brokkr` can affect discovery;
  prove host dispatch/restart and standalone resolution without deleting
  evidence, fabricating adopter configuration or editing pinned charters.
- Frozen descriptions mention signing or old routes. 0056/0057 and actual
  execution control. Record cosmetic fixes for a future pin; a behavior or
  contract delivery mismatch is a blocking repair.

Historical observations from `7b47790` stay in Git and bootstrap-v1 evidence;
they are not reasserted for this candidate. This design claims no full gate
or live reviewer success.

## Implementation approach

1. Reconcile actual pin and authorized inventory, preserving all pending work.
   Compare bytes, model/effort resolution, inheritance, helper origins, symlink
   and realm. Confirm 0057 provenance and dated markers. Report necessary
   repairs before any completion result.
2. Extend existing recipe/constitution suites only for uncovered obligations:
   fast hands/efforts/charter scope, historical self routes, Opus/Flash high,
   omitted DashScope effort, standalone realm/journal behavior and preserved
   protected gates. Use disposable wrong-value controls for demonstrated gaps,
   never frozen originals. No locally cloned policy evaluator.
3. Compile/discover all four with the recorded engine. Compare named/explicit
   identities and inherited script origin; resolve standalone without siblings.
   Retain raw logs/exits and inspect warnings even when compile exits zero.
4. Validate the exact-core probe's source SHA, dependency/lock subset, resolved
   input hashes and cases. Supplement gaps through that recorded core outside
   application dependencies. Cover strategies, absent/null/wrong-typed selector,
   safety/scope/fixes facts, all valid severities, first-match precedence,
   clean/residual admission, holds, visit limits and unknown stops.
5. Obtain actual engine probes for 1/2/4 selection, aggregation, consumption,
   scope refusal and restart/counters. Reuse valid receipts or use minimally
   scoped disposable probes without epic launches or frozen-original changes.
   Exercise one hold among clean members, missing/partial output, unknown
   strategy, first/repeated scope growth, third visit and feature scope defect.
   Verify the newly ruled strategy selects subsequent work/review. If no
   permitted proof exists, leave it outstanding. Selection probes are not
   complete live deliveries.
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
| Feature defect or non-chore residual fixes slip through | Supported refusal plus aggregation/tree checks; report frozen repair if absent |
| Inheritance relocates relative proof script | Inherit complete seats; prove resolved origin and exec mount; no copies |
| Structural/core checks masquerade as execution | Separate structural, evaluator, dispatch and live receipts |
| Ship labels conceal discrepancies | Real failure probes, fact provenance and clean close-out; new pin if broken |
| Legacy mirrors affect host discovery | Preserve evidence; prove standalone/host restart; report path repair |
| Provider/effort differs from declaration | Live selection receipts; no DashScope flag, fallback or Pro; outages stop |
| Dsh cannot restrict tools natively | Honest conduct limitation, fixes facts and tree checks; drift refuses |
| Stale receipts or checkbox edits prove wrong candidate | Validate input identity; final verify after edits; later receipts ignored |
| Frozen resource needs repair | Stop for operator-prepared new pin/run; no alternate engine |
| Eight runs collide or exhaust resources | Separate mutable state, reserve overlaps, throttle and rerun integration; no scheduler |
| Gates expose epic runtime defects | Preserve exact coverage/dependencies; report failure and defer scoped repair |
