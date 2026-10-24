# Implementation plan — coverage selftest reservation follow-up

Updated: 2026-10-24. Implements [spec.md](spec.md), preserving AC1–AC9 and
adding AC10–AC15. [tasks.md](tasks.md) pairs each obligation with its proof.
This replaces the initial implementation plan with investigation, bounded
repair and revalidation of existing code; it certifies no completed task.

## Technical context

The canonical gate is Bash with `set -euo pipefail`. Explicit resolution uses
parent `mkdir -p`, one plain leaf `mkdir`, and ownership 0. The selftest has
FIFO releases, nested resolver subshells, child status files and merged-log
matching. Conditional shell execution can disable errexit; worker completion
and resolver outcome need separate observations. The retained verification
failure has not been causally reproduced in this seat.

Existing compiler fixtures use jq, Cargo, installed coverage nightly,
cargo-llvm-cov and the standalone Rust scanner. One measured application crate
remains unchanged. No new dependency, Rust line or test seam is needed.
No plan template exists in `.specify/templates/`; the requested technical
context, constitution, project structure, research, data model, contracts and
quickstart are provided inline.

## Constitution check

Preserve accepted 0020/0035/0045/0050, all frozen guarantees and extraction at
ingest. Strengthening this proof changes no semantic contract and requires no
new decision. Scanner, application, non-selftest gate logic, dependencies,
fixed shims/checksums, policies and recipes remain preservation boundaries.
Production repair requires the evidence and scope return in AC8 and the dated
follow-up ruling before implementation; it is not a hidden conditional permit.

Retain codex-fast's recorded `engine` classification and all four configured
reviews under 0057. The engine dispatches them; this seat does not substitute
its own review. Keep security hold, no fallback or DeepSeek Pro, unsigned
commits under 0056, and no push, merge, tag or publication.

Recheck candidate-local `.forge/bin/brokkr` against
`bundles/self/tooling.json` before compile or dispatch; the recorded SHA-256 is
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
Version equality or a sibling binary does not prove the pin. Hermetic verify
executes the pinned transitive proof chain and checks public shim integrity.

## Project structure and files touched

| Path | Role and permitted change |
|---|---|
| `specs/coverage-selftest/spec.md` | Requirements, preserved criteria and dated rulings |
| `specs/coverage-selftest/plan.md` | Investigation, repair boundary and validation |
| `specs/coverage-selftest/tasks.md` | Ordered tasks paired with tests |
| `bundles/self/scripts/coverage-gate.sh` | Downstream repair only inside selftest helpers/concurrent arm |
| `.brokkr/state/tasks/`, `.brokkr/state/design/positions/`, `.forge/results/` | Ignored framing, positions and receipts; never stage as design artifacts |
| `target/coverage/selftest.XXXXXX` | Existing unique disk-backed fixture root, endpoints and records |

Only the three spec artifacts belong in the chief commit. No permanent fixture
crate, helper executable, separate shell test or additional research/contract
file is needed. Use the existing outer fixture lifetime, ensuring children are
joined before any outer cleanup runs. All other files are inspection inputs.

## Research and implementation approach

1. Preserve the failed receipt and identify baseline `16841aa` and reviewed
   #173 ancestor `ff9bf8a`. Reproduce through the actual canonical definitions
   and full accumulated selftest context. Record focused traces of paths,
   Bash options, relevant command/function resolution, individual statuses,
   outputs and leaf lifetime without dumping unrelated environment or secrets.
   Compare a controlled direct race through the same resolver. A reduced pass
   is a comparison, not discharge of the full-context failure.
2. Identify what permits both successes: selected paths, inherited overrides,
   status capture, descriptor interference or leaf deletion/replacement. Do
   not assume an EXIT-trap fault. Correct only the demonstrated selftest path.
   A reproduced production resolver fault triggers scope return to reconcile
   AC8 and dependent documents before production edits. If unreproduced, record
   uncertainty rather than labeling a synchronization patch causally verified.
3. Keep coordination local to two children under one root. Use private
   endpoints and dynamically allocated descriptors or equivalent child
   isolation; never overwrite inherited harness descriptors. Each child
   acknowledges the start boundary. Release only after both acknowledgements;
   children check the token before calling the actual resolver. Either kernel
   scheduling order and either winner are valid.
4. Bound readiness, release and completion with a local deadline/watchdog
   using the existing shell environment. Cover FIFO opens/writes as well as
   reads and waits. Explicitly check setup and I/O even under conditional
   errexit. On failure stop and reap only the fixture's tracked children,
   close endpoints and clean only after joins. Document the chosen failure
   bound in the fixture; it is not a wall-clock validity assertion. No new
   dependency, reusable process manager or timing-based success condition.
5. Capture resolver status in a subshell because `refuse` exits. Report the
   actual winner path/ownership only after successful resolution; no trailing
   successful command may mask failure. Workers write fresh complete records
   and signal record-delivery success separately. Parent validates both worker
   waits and records before requiring {0,1}. Match stable literal refusal
   category plus exact path in only the losing child's output. Reuse existing
   assertion helpers locally; no merged-log attribution or new result library.
6. Keep the leaf until both outcomes and ownership assertions finish. Write
   sentinel bytes, invoke actual cleanup with observed ownership, and retry
   the retained path through the real resolver. Assert named category/path and
   unchanged leaf/sentinel. Reuse existing retry/ownership tests where possible;
   the outer caller owns final root cleanup after joins.
7. Exercise the actual local proof with child-local controls: death before
   readiness; failure after release/missing completion; unrelated nonzero
   result; refusal text only in the other child's output. Each must fail and
   leave no children or damaged descriptors. Observe control verdicts without
   resetting the real parent's failures. Restore the evidenced original fault
   in an isolated control for red, then show green after repair. No duplicate
   resolver, production injection hook or broad mutation framework. Follow
   focused controls with the full canonical selftest.
8. Revalidate AC1–AC9, separate application coverage and all required gates.
   Byte-compare scanner and non-selftest script regions to `ff9bf8a`, and audit
   follow-up scope against `16841aa`. Preserve actual final-candidate verify
   and review receipts; baseline and draft checks cannot certify later edits.

## Data model and contracts

No application schema, API or persistence changes. Each of two fixture
identities binds a child PID, readiness observation, checked release, captured
output, resolver status and complete result record. The parent separately
observes worker completion. Both identities carry the same exact leaf.

Lifecycle: setup -> both ready -> both released -> both joined -> validated
outcomes -> ownership/retry assertions -> caller cleanup. Any infrastructure
failure takes bounded stop-and-join followed by a failed proof. A resolver
refusal is expected only when its worker completes correctly and its own
output proves the refusal category/path.

Success means resolver statuses {0,1}, actual winner ownership 0 and exact
path, and attributed loser refusal. Retained explicit reservations after
failure/crash belong to the caller. Atomic mkdir covers cooperating callers
retaining the namespace, not hostile external deletion, distributed locking
or automatic stale-target recovery.

## Quickstart and validation

For the chief's documentation change run `git diff --check`, then after
commit `bash bundles/self/scripts/design-check.sh check .` as structural
preflight. Keep the design commit at HEAD for the sequence's final pinned
check. Structural validation certifies no implementation or runtime gate.

Downstream, run `bash -n bundles/self/scripts/coverage-gate.sh`, focused
actual-function reproduction and red/green controls, then
`bash scripts/coverage-gate.sh selftest`. Separately run
`bash scripts/coverage-gate.sh` for actual coverage. Leave the explicit target
unset for automatic allocation or use a never-created leaf; do not reuse a
cached/precreated target or bypass the free-space check.

Execute the entire sole inventory in
[house rules § Gates](../../docs/house-rules.md#gates--part-of-every-change)
against the final candidate: fmt, locked clippy/tests, digest-matched bundle
compile, exact coverage, coverage selftest, publish-guard, keyless seal and
licenses. Retain positive counts and integer covered == count in all three
dimensions, real protocol tests and offline locked compiler fixtures. Also
execute `bundles/self/scripts/design-check.sh selftest`,
`bundles/self/scripts/delivery-proof-test.sh`, strict shim digest checks and
fail-closed native-tls absence. Only nonzero
`cargo tree --locked -i native-tls` with `did not match any packages` proves
absence; other failures do not. Record command/status, candidate/pin and log
paths in ignored evidence. CI-only obligations retain their charter owner.

The engine retains Qwen Max correctness, Claude Opus security, official
DeepSeek Flash guarantees and DashScope GLM 5.3 tests reviews, without
fallback. Design-seat checks replace none of those dispatches. Failed,
blocked and not-run gates/reviews stay outstanding.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Readiness patch conceals the two-success cause | Require full-context causal evidence and fault-restored red/green; unreproduced residual stays unresolved. |
| Resolver fault conflicts with AC8 | Return with reproduction and reconcile scope before editing production; accept explicit design return. |
| Worker errors look like refusal | Separate completion from resolver status, require fresh records and per-child category/path; wrong-error and wrong-log controls. |
| Child death blocks FIFO or wait forever | Bound every phase, check tokens and stop/reap before cleanup; accept small local protocol maintenance. |
| Descriptor collision or conditional errexit hides failure | Allocate/isolate descriptors and explicitly check I/O; test actual conditional calling context. |
| Early cleanup permits two reservations or removes evidence | Neither child deletes the leaf; parent joins, checks ownership/sentinel/retry, then cleans its root. |
| Controls contaminate later arms | Child-local overrides/counters, fresh records and checked joins; never reset parent failure count. |
| Repair expands into #175 or weakens #173/#174 | Scope/byte checks and full retained proof audit; no scanner/runtime/dependency/policy or denominator/floor change. |
| Added checks cost time and tool coupling | Keep shell controls local, reuse compiler fixtures, permit either winner; no exhaustive schedule matrix. |
| Draft check is mistaken for delivery success | Leave downstream tasks unchecked until evidenced; deterministic sequence owns design verdict, actual gates/reviews own delivery. |
