# Implementation plan: tooling digest reconciliation

## Technical context

One Rust crate; the owning Rust integration test hashes seven pinned bundle
resources, checks the seven public shim members and retains dispatch/engine
identity assertions. This work changes one expected string, with no new
production lines, dependencies, configuration, data model or public contract.
The available `.specify/templates/` contains a spec template but no plan
template; this plan supplies the charter's required plan sections directly.

## Constitution check

Preserve synchronous accept/store, subscribe-before-check, oldest-only pruning
and ingest extraction (0001–0005). Preserve fixed independent proof identities
and actual pinned execution (0050), full engine routing and independent review
(0057), the exact coverage floor and dependency policy. Commit unsigned under
0056 and never push. The work enforces existing rules, so it requires no new
ADR and grants no acceptance to proposed 0072. Required checks remain in the
single house-rules inventory; a design draft is not a green implementation.

## Research and evidence

Both run-local positions and the intake framing agree on the old/new hashes,
one-literal scope and engine classification. Chief independently confirmed
tracked/worktree equality and the new tooling digest. All seven shim manifest
checks passed. The candidate-local engine hashes to the recorded
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
The tooling source remains `731189f9097308c01ae4f2b4c9caea5f41224844`, with its
three ordered patch records unchanged. These observations establish identity,
not execution, external review, live delivery or historical receipt applicability.

## Project structure and files

| Path | Role and permitted change |
|---|---|
| `specs/tooling-digest-sync/{spec,plan,tasks}.md` | Chief's only committed artifacts. |
| `tests/delivery_constitution.rs` | Implementation replaces only the tooling expected literal in the named test. |
| `bundles/self/tooling.json` | Read-only source of the reviewed bytes; independently hash the Git blob and working file. |
| `bundles/self/scripts/shims.sha256` and its seven members | Read-only integrity inputs; retain exact membership and hashes. |
| `bundles/self/scripts/` | Read-only canonical proof scripts, including verify/design and their helpers. |
| `.forge/bin/brokkr` | Read-only candidate-local recorded executable for compilation. |
| `.forge/epic192/next-171-pinfix/chief-checks/` | Ignored local check commands, outputs and exits; no synthetic live receipt. |
| `specs/delivery-proof-residuals/receipts.md` | Read-only outstanding AC2/R04/D04 authority. |

## Implementation approach

1. Recheck the starting tree, tracked/worktree tooling hash and candidate-local
   engine identity. Refuse automatic pin refresh if bytes differ from this
   design. Preserve any unrelated operator work; do not stage it.
2. Replace exactly one occurrence of the old literal in the owning test.
   Compare the result to the prior file with that exact replacement applied;
   this proves every other assertion/helper is byte-for-byte preserved. Check
   the implementation diff contains only this file and this literal.
3. Run the targeted regression and complete owning suite, then every required
   house gate with locked dependency resolution where specified. Check the
   public shim manifest; hermetic verify executes its pinned canonical scripts,
   never candidate-selected proof code. Preserve and execute existing negative
   and tamper proofs rather than adding a duplicate test harness.
4. Retain exact commands, candidate/input identities, exits and logs in the
   run's ignored evidence. Report failure or unavailable prerequisites honestly.
   Let the existing engine route perform independent review; no lane rewrite,
   nested provider dispatch or substituted reviewer is part of implementation.

## Data model and contracts

No application model, protocol, persistence format, CLI, policy schema or
coverage contract changes. The test's reviewed expected digest is still its
independent oracle. The phase result file has exactly `result`, `inputs` and
`notes`; chief's result is `drafted`, with optional typed facts only in inputs.
No fake live-run record or edited journal is permitted.

## Quickstart and verification

Run from the candidate root after the one-literal implementation:

```bash
git show HEAD:bundles/self/tooling.json | sha256sum
sha256sum bundles/self/tooling.json
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo test --locked --test delivery_constitution pinned_bundle_shims_and_proof_dispatch_are_digest_bound -- --exact
cargo test --locked --test delivery_constitution
```

Run every command in [the house gate inventory](../../docs/house-rules.md#gates--part-of-every-change),
using the recorded `.forge/bin/brokkr` for bundle compilation. Verification
checks its digest before dispatch, rejects policy-sweep warnings, executes the
canonical coverage/selftest, publish/seal proofs and design/delivery selftests,
and proves native-tls absence fail-closed. Keep CI-only checks assigned there.
Do not install from offline verify or change inputs to conceal a red verdict.

For this design handoff, commit exactly the three design artifacts and keep
that commit last. Run `bash bundles/self/scripts/design-check.sh check .` as a
local structural precheck; the sequence's pinned final check owns the verdict.
No implementation literal is changed during chief. Pre-implementation gates
may therefore expose the known stale assertion; record their actual results
without representing a design draft as a verified implementation.

## Chief verification — 2026-10-24

All house gate commands were invoked on the design-only tree, together with
its targeted regression, native-tls absence, design selftest and delivery
component proof. Formatting, Clippy, recorded-engine compile (no policy-sweep
warning), coverage selftest, publish proof, seal proof, license check,
native-tls absence, design selftest and delivery proof exited zero.

The targeted test and workspace tests exited 101 at the unchanged tooling
literal; the owning suite reported six passing tests and one failing test.
Coverage exited 1 when its instrumented run hit the same assertion, before
reporting exact counts. These are expected pre-implementation failures, not
green final-tree verdicts. No exact coverage, full-suite success or live-self
proof is claimed. Subsequent implementation must rerun the required gates.

Actual commands, exits, elapsed times and log SHA-256 values are retained in
`.forge/epic192/next-171-pinfix/chief-checks/results.json`, with adjacent logs.
The coverage refusal retains `target/coverage/run.qzRRQb/run.log`.
Only design prose and run-local records changed during chief; tooling, scripts,
tests and product code remain unchanged. The local structural check follows
this design commit; the engine's pinned final check remains authoritative.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Blessing moved or unreviewed input | Independently hash tracked bytes and compare working bytes; reconcile drift before changing the fixed expected literal. Review provenance remains supplied by the task. |
| Weakening or widening the guard | Exact one-replacement comparison, preserve six other pins and all seven shims/assertions; no generated oracle, fallback or bulk refresh. Manual synchronization remains necessary. |
| Hash agreement mistaken for engine execution | Candidate-local binary identity plus canonical gate execution and retained actual exits. Compilation alone does not close live proofs. |
| Small diff used to bypass review | Preserve recorded engine classification and full protected review; accept its execution/provider cost. Official DeepSeek Flash only where assigned, never Pro/fallback. |
| Unrelated gate failure or unavailable tool | Retain diagnostics and leave the proof outstanding; do not widen this correction into script/product fixes or claim success. |
| False #171 closure or reuse of stale receipts | Preserve AC2/R04/D04 and proposed 0072 status; tie new evidence to current inputs and make no live-self claim. |
| Design selected from stale artifacts | Chief HEAD touches one spec directory, exactly three clean committed files; local structural check followed by the pinned sequence check. |
