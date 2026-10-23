# Implementation plan — Codex delivery recipes

Implements [spec.md](spec.md) under accepted
[0057](../../docs/decisions/0057-codex-delivery-recipes.md).

## Technical context

Single Rust crate with real-protocol integration suites; this feature changes
recipe configuration, tests and documentation only. Existing JSON/test
facilities suffice: no dependency or production Rust addition. The repository
has a spec template but no plan template; this plan supplies the dialect's
technical context, constitution check, structure, research, data model,
contracts and quickstart directly.

Use `.forge/bin/brokkr`, source
`597f4868e3fb971b62e6a368a2d8aa23271f4531`, SHA-256
`744ea1f3c459d08f5b102659541e3349182bccd91f597ce7906d49cbed431ed4`,
recorded in frozen `bundles/self/tooling.json`. Rust 1.98, stable,
nightly-2026-10-22 with LLVM tools, cargo-llvm-cov and cargo-deny are supplied.
Verify installs nothing and has no network. Version equality is insufficient.

## Constitution check

0057 already supplies routing authority; no new ruling is proposed.
0007/0050 require one transitively pinned proof chain; 0009 requires chief's
committed design followed by deterministic validation; 0013 retains independent
review; 0017 requires usable result delivery; 0056 requires unsigned commits
and no push. Preserve all frozen guarantees and exact coverage.

Existing bootstrap changes are authorized inputs, including frozen files.
Implementation may commit their existing bytes but may not repair them in
place. New non-pinned tests/docs are the only planned edits. Determine actual
pin membership before editing: documentation under a frozen bundle is also
frozen. Required pinned repair is a finding for an operator-prepared new run,
never authority to change this pin or claim completion.

## Project structure and touched files

| Paths | Responsibility and write boundary |
|---|---|
| `specs/codex-delivery-recipes/{spec,plan,tasks}.md` | Chief's only committed files; implementation may reconcile evidence before its final commit. |
| `bundles/codex/bundle.json`, `bundles/codex-careful/bundle.json` | Existing compositions: review and commit unchanged frozen bytes. |
| Original framed `agents/codex-*.json` files, `adapters/dsh.json` | Existing overlays/routes: review and commit unchanged frozen bytes. |
| `bundles/self/bundle.json`, `bundles/self/tooling.json`, `bundles/self/scripts/ship-seat.sh` | Existing shared design, engine identity and ledger changes: review and commit unchanged frozen bytes. |
| `bundles/self/policy.json`, other pinned scripts, base offices/charters/adapters | Read/execute as authority; no edits or proof copies. |
| `recipes`, `realms.json` | Existing relative alias and standalone realm/journal map: review, test and commit. |
| `tests/delivery_recipes.rs` | Add only missing self-route, realm/journal and effort assertions using existing test facilities. |
| `tests/delivery_constitution.rs` | Retain/run existing structural checks; do not mistake them for evaluator execution. |
| `.github/workflows/ci.yml` | Review/commit the already-authored three-recipe compile step. |
| `README.md`, `agents/README.md`, `docs/house-rules.md`, `docs/decisions/README.md` | Review existing changes; finish narrowly necessary non-pinned operator/evidence documentation. |
| `bundles/README.md` | Review/commit existing bytes only: this documentation is within the frozen bundle tree. Put new observations in this spec or other non-pinned documentation. |
| `docs/decisions/0057-codex-delivery-recipes.md`, existing dated addenda to `0007` and `0013` | Implementation commits accepted authority and amendment markers; no historical-body rewrite or invented acceptance. |
| `.forge/intake/`, `.forge/design/`, `.forge/results/`, `.forge/ledger/`, actual journal | Ignored evidence, never staged; preserve legacy panel copies under `.brokkr/state/`. |

The original office inventory is `codex-intake`, `codex-implementer`,
`codex-careful-intake`, `codex-careful-implementer`,
`codex-position-simplicity`, `codex-position-robustness`,
`codex-chief-architect`, and `codex-review-{correctness,security,guarantees,tests}`.
Do not stage later fast-lane additions through a wildcard or `git add -A`.

No `src/`, Cargo manifest/lock, dependency allowlist or coverage code changes.
Another necessary executable repair stays outstanding rather than expanding
this bootstrap into epic remediation.

## Research and observations — 2026-10-23

Chief read both positions and canonical intake; position mirrors compare
identically and the legacy intake directory is absent. Current executable
SHA-256 matches tooling. Explicit compile of self/codex/codex-careful and
default `recipes show` for both Codex variants exit 0. Each reports 38 policy
valuations over 14 groups, zero unruled, without policy-sweep warnings.
Named and explicit lookup produce equal respective recipe digests. The
compiled Flash model pin identifies `deepseek-official`.

Full command stdout, stderr and exits are retained under
`.forge/design/chief-evidence/` as `compile---bundle-bundles-*.{stdout,stderr,exit}`
and `recipes-show-*.{stdout,stderr,exit}`. `baseline.json` records 422
pre-existing tracked/untracked non-ignored files for chief's preservation
check; it is a local change detector, not a substitute run pin.
These pre-implementation observations prove neither final-candidate gates nor
live dispatch. Full house gates, evaluator execution, live reviewers, actual
verify and ship remain outstanding; actual design check follows chief.

A later preservation check found concurrent external edits to
`docs/decisions/0009-agent-council-hybrid.md`, 0057, the decision index and
house rules, plus new fast-lane files. A subsequent check also found external changes to
`.github/workflows/ci.yml`, `README.md`, `agents/README.md`,
`bundles/README.md` and `tests/delivery_recipes.rs`. These later bytes were
absent from the captured bootstrap inventory. Do not treat the current contents of broad directories
as the authorized original snapshot. Preserve external work, report the
mismatch and reconcile exact file/hunk scope with run preparation; this chief
commits only its three design files. The two-recipe design does not implement
or certify new triage/fast lanes. No earlier compile receipt attests them. Because one changed document is
under `bundles/`, compare actual run manifest membership rather than assume
that a documentation edit is harmless to the pin. Required pin drift cannot
be repaired or waived by this design.

Research obligations:

1. **Path conflict.** Actual host dispatch/resume must resolve correctly after
   frozen charters required legacy position output. Workspace lookup success
   narrows the risk but does not amend 0057 or prove another environment.
   Report a future charter-path repair; do not delete evidence, fabricate a
   `.brokkr` configuration or modify the pin.
2. **Evaluator access.** Locate the recorded engine's supported evaluator or
   existing harness. Verify-seat invokes tamper proof but source inspection
   does not establish the separate 0050 truth table. If needed, use a
   disposable harness linked to the exact recorded engine core/source outside
   the production dependency graph. No cloned policy logic, fetched verifier,
   digest bypass or frozen-input mutation. If interface/source access is
   unavailable, leave the execution proof outstanding.
3. **Descriptions/results.** Frozen self bundle/policy prose says ship signs;
   the bundle also describes shared design as GLM. Accepted 0056/0057 and
   measured wiring control. Record these stale descriptions for correction
   before a future pin; they alone do not prove signing or wrong dispatch.
   Check each dispatched result against its actual receiver and inspect real
   ship ledger/head consistency. A success-shaped result label alone does not
   prove clean close-out. Required executable/contract repairs need a new run.

## Implementation approach

1. Inventory authorized bootstrap diffs against actual run inputs and pin.
   Review concrete model/effort resolution, overlays, inheritance, alias,
   standalone realm and ledger redirection. Check accepted-body preservation
   and dated addenda. Record discrepancies with path, required repair and
   proving test before any claim of readiness.
2. Extend `tests/delivery_recipes.rs` only for missing obligations: historical
   self work/review chains, standalone realm/journal resolution and uncovered
   reviewer effort/provider-default settings. Existing tests cover inheritance,
   alias, work hands and most models; do not duplicate them. Resolve realm
   references in an isolated layout without sibling realms rather than infer
   resolution from strings. Use existing integration-test facilities; relevant
   wrong-value fixtures may be disposable, never edits to frozen originals.
3. Finish non-pinned documentation for launch prerequisites, isolation/ceiling,
   unexercised variants, dsh conduct versus confinement, legacy-path and stale
   description findings, and provider/gate failure reporting. Do not introduce
   another gate inventory, ledger format or evidence service.
4. Run recipe/constitution suites and compile/discovery with the recorded
   engine. Execute actual evaluator proofs with rule IDs and destinations:
   above-medium stops first; explicit false security plus valid at-most-medium
   severity may advance; the final unconditional unsafe rule catches remaining
   security/unknown facts. Cover true/false, every valid severity, absent,
   null and wrong-typed facts on both axes, malformed severity and invalid
   security with above-medium severity. Also prove clean routes, aggregation
   and security hold. Preserve receipts separately from compile sweeps.
5. Execute every local house gate on the candidate, retaining literal exact
   coverage and all refusal proofs. No tool installation, policy mutation or
   candidate-selected verifier. Reconcile implementation-owned tasks from
   evidence; commit all authorized bootstrap and needed non-pinned tests/docs
   unsigned with rationale. Missing mandatory local proof or required pinned
   repair precludes implementation completion.
6. The engine's actual verify dispatch must execute the inherited offline
   seat and transitive pinned helpers on that committed candidate. Record
   mounted script origin, launched digest, checked shims and every result.
   Manual shell execution supplements but cannot replace this dispatch.
7. The protected panel supplies all four live provider routes and usable
   member results. Capture actual endpoint/model selection without secrets,
   particularly official Flash; preserve tree stability, worst-verdict and
   no fallback. Provider failures remain outstanding. A second paid full
   delivery solely for the unexercised careful variant is not required.
8. Actual inherited ship renders this run's journal into the explicit ledger
   path. Require a nonempty ledger, matching repository head and directly
   checked clean worktree/index, plus real engine close-out evidence. Never
   reconstruct old journals or call a discrepancy complete. No push, merge
   or eight-workstream launch occurs here.

## Data model and contracts

No mail schema, public protocol or migration changes. Recipe names select
compositions; offices bind inherited charters, singleton models and efforts;
review retains its lenses/results; policy alone decides transitions.

Existing receipts identify run/attempt, command/seat, candidate head,
engine/bundle identity, provider/model, result and location. This describes
evidence contents, not a new schema. Chief writes exactly `result`, `inputs`,
`notes`, with `drafted`, to its assigned result path through workspace hands;
use a sibling temporary file and rename after artifacts are committed and
checked. Only the final deterministic check advances design. Other seats
must satisfy their actual receiving contracts; partial/missing output never
becomes success from final prose.

Implementation task statuses describe evidence available at its final commit.
Future verify/review/ship tasks stay pending then; subsequent results belong
in the actual journal/ledger. Do not mutate the reviewed tree just to tick
boxes after verification. A later tracked update requires normal delivery and
renewed candidate evidence. Close-out must inspect later receipts directly.

## Quickstart and validation

Stay on `epic192-codex-recipes`. No nested delivery or workstream launch in
this design. For implementation validation:

```bash
export PATH="$PWD/.forge/bin:$PATH"
sha256sum .forge/bin/brokkr
cargo test --locked --test delivery_recipes
cargo test --locked --test delivery_constitution
brokkr compile --bundle bundles/self
brokkr compile --bundle bundles/codex
brokkr compile --bundle bundles/codex-careful
brokkr recipes show codex
brokkr recipes show codex-careful
```

Compare the digest to frozen tooling first, then execute the complete single
[house gate inventory](../../docs/house-rules.md#gates--part-of-every-change).
Actual verify checks shims and runs dispatched pinned implementations. Preserve
coverage/design selftests, tamper proof, publish/seal proofs and fail-closed
native-tls absence. CI-only chart/history, Docker, advisory, tag rate and
real port-80 proofs retain their owner.

After bootstrap is reviewable, operator launches use the documented
`brokkr run --recipe codex --db .forge/forge.db --feature '<scoped story>'`
(or `codex-careful`) from each isolated worktree. This is a future operator
quickstart, not evidence or authorization for chief to launch workstreams.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Inheritance changes script origin despite matching JSON | Retain complete base sequences; require composed compile and actual mounted exec provenance. |
| Legacy path changes host discovery | Preserve evidence/current lookup receipts; require actual dispatch/resume and report charter correction for a future pin. |
| Frozen descriptions mislead | Record discrepancy here; accepted decisions and measured execution control. No in-run byte repair. |
| Route label differs from live provider | Require non-secret actual selection receipts. Failure stops; no Pro or author fallback. |
| Analysis is lost without result delivery | Require engine-consumed member results and atomic chief output; no inference from prose. |
| Compile misses unknown-input policy behavior | Execute recorded evaluator truth table and first-match IDs; unavailable access remains outstanding. |
| Dsh reach exceeds read-only conduct | State limitation, retain charters/tree-drift checks and refuse observed drift; no invented confinement guarantee. |
| Coverage or another inherited gate fails | Preserve exact 100% lines/branches/functions and refusal proofs; report failure without floor waiver or epic fixes. |
| Ledger text/result label masks discrepancy | Require real journal, nonempty ledger, matching head, clean tree and real close-out; report necessary frozen repair. |
| Concurrent external additions broaden the framed bootstrap | Compare against original inventory and actual pin; preserve external work, report exact drift, and require scope/pin reconciliation before delivery. No wildcard staging of fast-lane files. |
| Eight runs exhaust resources | Ceiling only; isolated targets/private cache overlays, reserved overlaps and reduced concurrency when needed; no scheduler scope. |
| Evidence updates change verified candidate | Commit implementation evidence before verify; later ignored receipts stay in journal/ledger; reverify any tracked change. |
