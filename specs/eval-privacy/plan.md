# Verification plan — eval privacy continuation

Date: 2026-10-24. Implements the continuation obligations in [spec.md](spec.md).
The contracts of proposed [0062](../../docs/decisions/0062-eval-privacy.md) remain
unchanged. This replaces the original implementation plan's stale future work;
it does not turn reported historical passes into current receipts.

## Technical context

Standalone unpublished Rust 2024 eval workspace with its own lock, Tokio process
ownership, reqwest, Serde, Clap and tempfile; root remains a single Rust crate.
Unix Linux/macOS scope, real SMTP/HTTP/MCP proofs, no new dependencies. Evals is
separately measured under 0055: root green does not prove it. The current host is
Linux. Preserve c232602 and cf3aa2f; default implementation action is no code edit.
Only a spec template is present under `.specify/templates/`; no plan template
exists. Required research, data model, contracts and quickstart are embedded here.

## Constitution check

Preserve synchronous accept/store, subscribe-before-check, oldest-only pruning,
AUTH inbox routing and ingest extraction under 0001–0005. Keep 0020 deterministic
test placement and real protocol discipline, 0050 pinned proof ownership, 0055
eval semantics, 0056 unsigned commits and 0057 protected delivery. Proposed 0062
is not operator acceptance; an accepted-ruling contradiction waits under 0045 §8.
No frozen weakening is permitted even with a proposal and green tests.

This chief changes exactly `specs/eval-privacy/{spec,plan,tasks}.md`. Existing 0062
and its index row are already committed. No new decision is needed because this
continuation preserves the semantic contract. Keep this design commit last for
the sequence's deterministic check. Chief returns drafted, never a design or
verification verdict. Full gates and review remain downstream obligations.

## Research and resolved questions

Read both current positions, both intake records, decision index and relevant
rulings, existing design and c232602's diff. The observed fix maps Admin transport,
read, JSON and non-200 response errors to finite categories. Its new HTTP test
exercises non-200 JSON; the custom-target regression now checks a built executable.
These observations identify proofs to run, not proof outcomes.

RuntimeDirs::apply remains the shared runtime environment owner; main::run_child
reuses it for matrix workers, adds resolved SWARMAIL_BIN and drains stderr.
Adopt simplicity's local ownership/no redesign and robustness's complete claim-to-
evidence binding. No new receipt cache, sanitizer, provider adapter, process
supervisor or policy evaluator. Named preparation selectors remain intentional
inputs; arbitrary caller environment never becomes a compatibility fallback.

## Project structure and touched files

| Path | Role and allowed action |
|---|---|
| specs/eval-privacy/spec.md, plan.md, tasks.md | Only committed chief edits; reconcile continuation requirements and proof tasks. |
| evals/src/runner.rs | Preserve Admin categories, RuntimeDirs, preparation, bounded stream handling and lifecycle; inspect only if an existing proof exposes a defect/gap. |
| evals/src/main.rs | Preserve matrix caller, row validation, safe CLI/publication; same bounded repair rule. |
| evals/tests/runner.rs, report.rs, proof.rs, fixture.rs; evals/tests/common/mod.rs | Execute existing controls; narrow regression changes only when a failing proof demonstrates a defect under the downstream route. |
| evals/README.md; specs/agent-eval-harness/ | Verify documentation agrees with the owning contract; no speculative rewrites. |
| docs/decisions/0062-eval-privacy.md and README.md | Existing proposed record/index, verify status and links without acceptance or duplicate records. |
| bundles/self/tooling.json; .forge/bin/brokkr | Identity inputs; candidate-local binary must match recorded SHA-256 before execution. |
| Dispatched pinned scripts and public shims | Proof owners and integrity inputs; no edits/bypasses to obtain a pass. |
| .forge/ run-local evidence/results; .brokkr/state/ framing/positions | Preserve historical records; new receipts remain uncommitted and distinguishable. Never fabricate journal events. |

No root production, lock, manifest, dependency allowance, policy, recipe or gate
change is planned. A failure outside this bounded scope returns through the engine
route; do not silently enlarge scope or modify tests merely to accept bad behavior.

## Data model and contracts

No schema/CLI changes. Preserve TrialRequest, schema-1 TrialResult, secret-free
Snapshot, explicit labels, actual agent exits, matrix product cardinality, scoring
precedence and confinement:unconfined. Keep literal argv and the eight generated
agent inputs, fixture-only secret and explicit wrapper-based authentication.

The spec owns exact runtime/preparation maps, per-child private directories,
8192-byte stream/readiness bounds, first typed readiness slot, 1 MiB Cargo/matrix
protocol bounds, zero raw diagnostic retention and capped safe facts. Keep owners
alive through drain/cleanup. No global environment mutation, retained transcripts
or test-control map exception. Admin body collection is not proved byte-bounded;
owned local fixture trust and unconfined filesystem/network limits remain explicit.

Evidence uses existing run-local facilities rather than a new schema/framework.
Keep command, exit, tested commit/tree, platform, engine hash, pinned resource
identity and log path together; label historical versus freshly executed evidence.
Only synthetic canaries enter proof records. Do not dump caller environment or
credentials. Record partial/interrupted executions as incomplete. Bind any admitted
historical mutation control to its precise tested source/test and explain continued
relevance; otherwise reproduce the missing control narrowly, restoring the source.

## Execution approach and quickstart

1. Verify ancestry and clean starting candidate; record final candidate identity
   after this documentation commit, engine digest and OS. Compare `.forge/bin/brokkr`
   against tooling.json; use its absolute path for dispatched checks. Do not use a
   sibling/host binary, rebuild the engine or bypass a mismatch.
2. Run the focused c232602 controls, then separate eval fmt, all-target locked
   clippy and the full locked suite including proof.rs. Require actual test counts
   and helper execution markers. Check role maps, wrapper auth, independent output
   canaries, final/temporary artifacts, bounded drains, readiness, failure outcomes,
   cleanup, matrix cardinality and real oracle/null/wrong-action/isolation controls.
3. Execute the complete current [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
   through its owning pinned route. Keep exact zero-missed line/branch/function
   coverage and selftest, design selftest, delivery tamper-boundary proof,
   publish/seal keyless proofs, licenses and fail-closed native-tls proof. Execute
   recorded-engine compile without the policy-sweep warning refusal; actual design,
   verify dispatch and policy-evaluator proofs are separate requirements. Never
   replace the evaluator with a hand-written approximation. Hermetic verify is
   offline and installs nothing. Previous reported greens do not skip these gates.
4. On failure, preserve the failing command/log/source first. Distinguish a setup
   or missing-evidence block from a demonstrated implementation defect. Repair only
   the latter, preferring restoration of the existing owner or removal of an
   unnecessary path before a small local guard. Add a discriminating regression,
   rerun affected suites and all required gates on the final candidate. No raw
   error output, inheritance fallback or reduced gate to restore compatibility.
5. Preserve existing admissible mutation controls, reproduce only missing necessary
   ones with synthetic values and no production candidate left mutated. Run all
   protected review lenses on the verified candidate and retain security hold.
   Tests: GLM 5.3; task-authorized Haiku only on documented dispatch failure.
   Guarantees: official DeepSeek Flash, never Pro. Correctness/security remain.
   Record identities and outcomes; an adverse verdict follows the engine return
   route rather than selecting a different model.

Focused command examples from root (offline cache must already be available):

```bash
cargo test --offline --locked --manifest-path evals/Cargo.toml admin_error_categories_discard_injected_response_text
cargo test --offline --locked --manifest-path evals/Cargo.toml --test runner automatic_build_uses_a_cleared_named_preparation_map
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --offline --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings
cargo test --offline --manifest-path evals/Cargo.toml --locked
cargo test --offline --manifest-path evals/Cargo.toml --locked --test proof
.forge/bin/brokkr compile --bundle bundles/self
```

These examples are not receipts and do not replace pinned dispatch. Root gate
commands remain solely in the house inventory. Any local structural design check
is a preview; the sequence's final pinned check owns advancement. Reviewers
reproduce eval evidence under 0055; the chief does not certify downstream work.

## Supported-platform and CI handoff

T4 retains macOS system/rustup/toolchain preparation as explicitly outstanding
until a macOS runner executes it with candidate-bound evidence. Linux-only success
cannot check that task or prove full supported-platform completion. Continue independent Linux
proofs while retaining this open obligation. Do not assert
that current CI already runs this specific test. An unavailable macOS runner is a
reported evidence gap, not permission to remove support or weaken acceptance.

Separately attribute the charter's chart contract/history/selftest, real-Docker
persistence/kill/volume/context smoke, network RustSec audit, tag-only release-rate
and publication prerequisites, and privileged port-80 proof to their CI owners.
Report actual receipts or outstanding status, not blanket CI-green claims. No push,
merge, issue close, signing or live measurement is authorized by this design.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Stale receipts certify a changed candidate. | Record identities and fresh mandatory execution; preserve historical evidence separately. Accept rerun cost. |
| Correct hash is confused with dispatch proof. | Actual pinned compile, design/verify dispatch and evaluator/tamper tests with checked shims; no alternate executable. |
| Canary absence is vacuous. | Require helper markers, full role maps, generated inputs and useful failure categories; preserve attributable red/green controls. |
| HTTP regression is overgeneralized. | It covers non-200 JSON; broader suite evidence must cover other claimed publication paths. No hostile-body memory-bound claim. |
| Closed environment breaks toolchain/auth. | Real custom-target and wrapper tests, named locators, retain macOS gap; never restore ambient inheritance. |
| Output suppression hides outcomes or stalls pipes. | Safe category/status assertions, independent stream canaries, flood/deadline/cleanup and matrix controls. Accept reduced diagnostic detail. |
| Unnecessary audit becomes a redesign. | Run existing owners/tests first; bounded evidence-led repair only, no new framework/dependency. |
| Review fallback changes verdict or independence. | Record GLM dispatch failure before task-specific Haiku; adverse reviews never trigger fallback; preserve security hold. |
| Linux green becomes a macOS/full-delivery claim. | T4 remains open; distinct supported-platform and charter CI evidence in handoff. |
| Proposal mistaken for permission to weaken guarantees. | 0062 stays proposed; preserve 0001–0005 and use 0045 §8 for any conflict. |
