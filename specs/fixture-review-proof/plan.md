# Implementation plan — fixture review proof

## Technical context

This is a verification continuation for a single root Rust crate and the
independently locked evals workspace. Existing serde/serde_json/reqwest/Tokio
owners already implement the helper. Expected production delta: zero. Existing
real-child/HTTP tests and source evidence provide the proof surfaces; no new
library, dependency, framework, scheduler or result schema is needed.

The requested `.specify/templates/plan-template.md` is absent. This plan supplies
its required technical context, constitution check, structure, research, data
model, contracts and quickstart directly.

## Constitution check

Preserve accepted 0001–0005 and their amendments, exact root coverage and
regression placement under 0020, proof ownership under 0050, standalone proof
under 0055, routing under 0058 with the narrow recorded 0081 direction, source
attribution under 0063 and admission under 0076. No accepted ruling is weakened.
No new semantic change is proposed. Existing proposal statuses remain intact.
The chief commits only these three design files, unsigned; positions, results
and execution evidence stay run-local. Full verification and protected review
are later phase obligations, not facts created by a structural design check.

## Research and resolved questions

The intake, both positions, issue snapshot and prior receipt index agree on
scope. The original helper (c5807fc), metadata change (97dd874), positional
repair (7169583), integration (7bdd484) and description correction (09852d8)
all belong to review. `7bf779f..HEAD` is useful integration context but omits
original helper/governance history; it is not the review boundary.

Source inspection confirms IgnoredAny on Call._meta, closed Call/Arguments,
two-slot positional guard, presence-preserving body deserialization, bounded
stdio and finite tool-error projection. Existing actual_stdio_helper tests
are the proof owners. The tip only corrects standalone-wiring claims and two
report descriptions; #201 CI integration is outside this continuation.

The prior snapshot records event_count 701 and __member-schema-invalid__.
The retained index lists local passes with protected review still remaining.
Proposed 0081 and tracked GLM intake retain the earlier correlation failure.
None is current proof. Historical absence cannot be cured by manufacturing
receipts or executing an attack demonstration.

## Project structure and files

| Paths | Work and owner |
| --- | --- |
| specs/fixture-review-proof/spec.md, plan.md, tasks.md | Only files in this chief's commit; contract, procedure and paired proof tasks. |
| evals/src/fixture_http.rs; evals/tests/wrappers.rs and launch tests; evals/wrappers.md | Inspect helper and launch contracts; reuse real-process tests. Repair only a demonstrated scoped defect. |
| evals/src/provenance.rs, artifacts.rs, results.rs, wrappers.rs, runner.rs; evals/tests/common/; remaining evals/tests/ | Inspect merge resolutions; execute complete attribution, reader, privacy, lifecycle, replay and admission proofs. |
| evals/Cargo.toml; .github/workflows/; pinned verify resources selected by bundles/self/ | Compare standalone-check statements with actual owners; no #201 wiring change. |
| src/mcp.rs, src/ui.rs and report owners; tests/mcp.rs, ui.rs, store_contract_docs.rs | Check descriptions and real tools/list parity against shipped report behavior. |
| src/store.rs, smtp.rs, extract.rs, webhook.rs and wait/report owners; tests/burst.rs, p2.rs, rate.rs | Review full integrated guarantee/lifecycle boundaries and run root proofs. Preservation scope, not redesign authorization. |
| bundles/codex-careful/, inherited bundles, agents/, adapters/, bundles/self/tooling.json and scripts/ | Inspect pinned routes/proof identities; engine alone dispatches protected review. No routing or tooling mutation. |
| specs/glm-flash-seats/intake.md, implementation.md; docs/decisions/0081-glm-flash-seats.md | Read retained governance evidence without changing or reproducing it. |
| .forge/evidence/ under a unique run-owned leaf | Later phases retain commands/exits, reports, build JSON, binary/digest and evidence index. Existing receipts are preserved. |

## Implementation approach

1. Record clean starting HEAD/tree, ancestry and full integration scope. After
   the design commit, bind every proof to the actual descendant. Record engine
   digest, tooling patch hashes, checked shims and dispatched bundle identities.
   Verify offline and install nothing; do not substitute PATH tooling.
2. Inspect the helper, integrated provenance/result readers and root guarantee
   owners. Reuse their current tests. For a demonstrated compatible defect,
   retain its failure, make the smallest owner-local repair with a meaningful
   regression, commit under the later phase's authority, and rerun affected
   proofs and review. Semantic/routing/engine changes exceed this design.
3. Run the entire local inventory through engine-dispatched pinned verify
   resources. Check public shims against their pins; never let mutable dispatch
   shims select the proof algorithm. Keep supplemental design/delivery proofs.
4. Establish suite availability through the permitted same-user boundary.
   Wait on occupied admission without overlapping another focused/full suite.
   Preserve existing parent-test coordination; do not acquire the production
   trial guard around the suite. If activity/cleanup cannot be established,
   report the gap and leave standalone verification outstanding.
5. Capture the exact candidate's root executable and source/build association.
   Prefer the successful root-test Cargo artifact; a fresh explicit locked
   root build can supply the artifact with its own recorded source context.
   Copy to a unique path outside mutable target outputs, remove write bits,
   record the absolute path/digest, and use it through SWARMAIL_BIN. Run
   standalone fmt/clippy/full tests separately; hash immediately before/after
   the full test command and retain its exit independently. Do not run coverage
   or another build against the captured path. Tests intentionally exercising
   missing/override/build resolution keep their own selection controls.
6. Hand the complete candidate/evidence to the existing protected Careful phase.
   It must invoke all five offices and retain actual schema-valid member and
   aggregate records. Reviewers assess the full source and inherited finding
   independently. Existing engine recovery owns malformed-result handling.
   No chief-authored review results or replacement policy evaluator is allowed.
7. Record the inherited finding as unresolved unless the independent protected
   disposition cites sufficient candidate/engine-bound evidence. Neither a
   newer upstream repair nor focused route success establishes boundary safety.
   If required evidence is unavailable under authorized means, retain the high
   finding and let existing policy block ship. Do not reproduce the defect.
8. Distinguish local receipts, CI obligations and operator-owned evidence.
   Preserve earlier attempts. Only policy-authorized ship may record delivery;
   no issue action, push, merge, tag, publication or decision acceptance follows.

## Data model and contracts

No runtime data model or public contract changes. Use the existing evidence
owners rather than a new attestation subsystem. A run evidence index records:
actual commit/tree and cleanliness, engine/bundle/patch/shim identities,
command and exit with complete log path, Cargo artifact/source association,
captured executable path and before/after hashes, suite admission/cleanup
observations, and protected member/aggregate references with resolved routes.
Use actual unavailable facts where evidence is missing; invent no digest,
result, clean vote or provenance. Preserve privacy controls and never copy
credentials into receipts. Evidence indexes summarize original artifacts and
cannot replace them or authenticate a historical claim on their own.

The spec's AC1–AC11 are the acceptance oracle. The implementation phase proves
local behavior; pinned verify owns gate verdicts; protected review owns member
results, aggregation and security hold. The chief's drafted result and a local
structural check cannot advance the sequence or certify delivery.

## Quickstart and validation

The following lists the intake's required proof commands for later owners,
not executed receipts. Use separate bounded logged invocations, retaining each
exit even when nonzero. Long-running work must retain completion state; an
interrupted tool response is not a pass.

```bash
export CARGO_NET_OFFLINE=true
sha256sum .forge/bin/brokkr
sha256sum --check --strict bundles/self/scripts/shims.sha256
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

Compare the measured engine digest to tooling.json, including all recorded
patch hashes. The paths above identify proof owners; actual verify dispatch
uses the pinned resource copies. Coverage must report zero missed lines,
branches and functions with unchanged denominator and no exclusions. Preserve
any documented build diagnostic settings without weakening instrumentation.
Compilation must have no unruled valuations or policy-sweep warnings.

For both Cargo.toml and evals/Cargo.toml, retain output/status from
`cargo tree --manifest-path <manifest> --locked -i native-tls`.
Only nonzero exit with the specific package-absent diagnostic proves absence;
exit zero, resolution failure or other error refuses. Capture the expected
nonzero safely rather than allowing errexit to erase the distinction.

After the admission check, use these standalone/build commands:

```bash
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings
cargo build --offline --locked --bin swarmail --message-format=json-render-diagnostics
# Retain Cargo JSON and clean before/after source observations; select the
# successful swarmail binary artifact and preserve it in a unique read-only path.
# Set absolute SWARMAIL_BIN and a newly recorded EVAL_EXECUTABLE_DIGEST_FILE.
sha256sum --check --strict "$EVAL_EXECUTABLE_DIGEST_FILE"
cargo test --manifest-path evals/Cargo.toml --locked
sha256sum --check --strict "$EVAL_EXECUTABLE_DIGEST_FILE"
```

The full suite includes helper metadata/closed-shape/GET/overflow tests,
wrapper launch/tools-list parity, boundary, identity, provenance, proof,
baseline, replay and sequential-admission controls. Focused diagnostics may
use the actual_stdio_helper_ filter, root `--test mcp`, `--test ui` and
`--test store_contract_docs`; they never replace full suites and focused evals
require the same admission check. No paid provider trial or benchmark is needed.

After the chief commit, run the pinned design validator's local check:
`bash bundles/self/scripts/design-check.sh check .`. The sequence's final
check owns the design verdict. Later phases still owe all ACs, even if this
structural check passes. CI retains the complete charter's platform/network,
Helm/chart/history, Docker, real port-80 and tag-only responsibilities.

## Risks

| Risk | Mitigation and accepted limit |
| --- | --- |
| Old receipts silently certify the new HEAD | Record descendant commit/tree and source observations; preserve historic receipts separately; renew affected proofs/review on change. |
| Metadata changes broaden serde positional admission or leak context | Keep named discard and two-slot guard; exact full-stream/HTTP capture and closed-shape negatives; no recursive POST scrubbing. |
| Source attribution is inferred from filename or current HEAD | Preserve actual Cargo artifact/source observations and immutable copied bytes; hash before/after. This is consistency evidence, not hostile-writer attestation. |
| Another suite causes refusal or contaminates evidence | Observe availability, wait, preserve stable locks and parent-test ownership, retain contention, obtain a fresh uncontended run. Unknown cleanup prevents safe-resume claims. |
| Root green hides standalone or merge regressions | Execute separate standalone fmt/clippy/full tests; inspect integrated provenance/readers and original helper history. |
| Governance finding gets closed by compilation or unsupported opinions | Require independent protected disposition plus concrete candidate/engine evidence; retain high/block ship when absent. No exploit demonstration or routing workaround. |
| Malformed review result becomes a synthetic clean vote | Retain actual five results and aggregate through authorized engine recovery; missing/malformed evidence refuses. |
| Broader robustness discussion becomes unrequested redesign | Keep current Store/wait/webhook/cleanup owners and documented limits; only demonstrated compatible repairs, no new queue/retry/framework. |
| Flaky proofs hide regressions | Fresh inboxes, ephemeral ports, generated certificates, causal seams and owned child/listener cleanup; no assertion weakening, sleep-as-sync or mid-band chaos. |
| Verification seat cannot observe host-wide activity or required evidence | State the precise inaccessible fact and leave that criterion outstanding; do not infer exclusive execution or waive a gate. |
| Local proof is reported as baseline, CI or shipment | Keep owner-specific receipts and explicit outstanding items; no external publication or paid measurement claim. |
