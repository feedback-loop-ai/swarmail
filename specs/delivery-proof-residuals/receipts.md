# NEXT-171 recovery receipt index — 2026-10-24

Run: `epic-192-issue-171-recovery-on-t-181c6f9c`. This is fresh codex-fast
triage and implementation, not a linked continuation. The committed design
and proposed 0072 are preserved; only the operator accepts the proposal.

Raw run artifacts are retained under the ignored path
`.forge/epic192/next-171/epic-192-issue-171-recovery-on-t-181c6f9c/`.
Verify the AC1 receipt with its adjacent `ac8/receipt.sha256`; verify every
AC1 evidence file against `ac8/evidence.sha256`. The receipt records the
candidate commit/tree, exact self and codex-fast manifests/policies, engine
binary, recorded source base and patches, harness lock/source, expected and
actual JSONL, exits and output hashes.

## AC1 — pinned evaluator / old T08

**Proved for the receipt's identified candidate.** `ac8/run.sh` invokes the
real `brokkr_runtime::bundle::compose::resolve` and
`brokkr_core::policy::Machine::from_table/evaluate` APIs. Its 105 unique cases
assert exact rule ID, destination and severity, or exact refusal problem text.
It covers supported severities and booleans, absent/null/malformed facts,
invalid-security precedence, both safety facts absent, security hold, clean
routes, and current codex-fast scope/fixes controls. The permissive UNSAFE to
ship/flagged mutation fails 17 assertions; the restored table passes.
`aggregation.log` records 12 actual Brokkr aggregation regressions, including
absent/null/wrong-typed residual and scope/fixes facts remaining unknown.
`self-compile.log` is exit 0 without the residual policy warning.

Identity in `ac8/receipt.json`: engine SHA-256
`e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`;
self bundle/policy/tooling digests
`2a5fa3696dac122822102316041022850d1630641987b96156a2f4f784bdb265`,
`652a064814e42a3d1e0e5fc5ecc600de5241da7e5cace8d73e6bf087ef15a47f`,
`e6eaeff867eb71fab5a1ce93f4ff1d46ae9deb2944b517867787a63a966f3fa3`;
recorded source base `597f4868e3fb971b62e6a368a2d8aa23271f4531`, followed by the
three patch hashes in the receipt. Root source/build evidence hashes verified.
The workspace's current source-composite recheck exits 1 because the copied
source Git checkout references an external alternate object directory outside
the workspace. That limitation is recorded, not represented as a successful
recheck. The harness nevertheless built and ran against the retained copied
core and the candidate-local binary matches the tracked tooling digest.

## AC2 — exact live `bundles/self` design/retry/pass

**Open.** No current live self dispatch receipt exists. This seat has only the
Brokkr workspace capability and cannot launch nested model providers or create
a canonical journal. The prior
`epic-192-issue-171-live-self-bun-c69c9fa3` report says DSH transport failures
parked before design; it is historical context, not this run's evidence or a
qualifying receipt. No alternate recipe, codex-fast run or evaluator harness
is substituted.

## AC3 — digest and dispatch boundaries

The final `cargo test --locked --test delivery_constitution` receipt is in
`gates.json`. It pins the tracked self bundle, policy, tooling, seven-member
shim manifest, canonical design-check, verify-seat and delivery-proof test
script; checks every public shim against the manifest; asserts launched-engine
identity and delivery-proof call text; and forbids both `bash scripts/` and
`bash ./scripts/` in active verify code. Four bounded mutations (appended
bytes, disabled digest comparison, disabled delivery-proof call and literal
candidate dispatch) must be recorded under `ac3-mutants/`; each is expected to
fail the fixed digest pin and the original verify-seat bytes must be restored.

## AC4 — 0050 §2 dated marker

**Documented, not repaired.** The dated addendum in 0050 links proposed 0072
and the retained `ac8/comparator-observation.json`. That observation invokes
the actual pinned comparator with a synthetic textual release-shape example;
it records that the equal-indentation `if true` to `if false` shape is
admitted, while no-coverage and changed-body controls refuse. It does not
claim compiler or real-build equivalence, change comparator bytes, or grant
an exception to 0050 §2.

## T08 / R04 / D04

- **T08:** complete for its AC8 evaluator obligation, by the AC1 receipt,
  aggregation regressions, warning-free pinned compile and passing named
  structural test.
- **R04:** open. No actual pinned verify-seat combined integrity/uncovered
  coverage red receipt and covered control, and no actual design retry
  dispatch, were produced. The canonical journal is absent and the fleet
  snapshot is dated 2026-10-23. It assigns overlapping verify-seat hunks to
  #172 and #206; current reservations and NEXT-187 coordination were not
  refreshed, so this run does not edit the canonical gate scripts.
- **D04:** open. It covers the broader original delivery-gates AC1–AC13.
  The missing actual dispatches and separate coverage, #143, and CI proofs
  remain assigned to their authorized owners; this recovery does not claim
  those obligations.
