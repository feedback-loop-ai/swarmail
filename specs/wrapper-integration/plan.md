# Wrapper integration implementation plan

## Technical context

This is a local Git integration and documentation repair. The root Rust crate
and independent `evals/` workspace already contain the wrappers and their
proofs. No new source, package, dependency, lock, configuration or schema is
needed. The workspace has a spec template but no plan template; the prescribed
technical context, constitution, structure, research, data model, contracts,
quickstart and risks are supplied here within the three-artifact limit.

Design input HEAD: `f9df13e05be85fac459ae95c0378682fdc5452ab`.
Wrapper preservation baseline: `6a67f7e`.
Supplied local main: `89581763b8b4f4ddbc1a800c7cdb37dae0f2e3ff`.
Engine SHA-256: `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
The engine digest and complete issue-packet checksums were verified in design.

## Constitution check

No new semantic decision: the operator has already allocated the existing
proposal to 0083. Keep it proposed and preserve its runtime provisions.
Accepted 0001–0005, 0050 proof ownership, 0055's standalone eval boundary and
0056 unsigned history remain unchanged. Use current candidate recipe routing.
The local merge is the specific authorized exception to the task's general
no-merge delivery prohibition; no remote merge, fetch, rebase or push is needed.
Do not read remote services, journals or sibling worktrees, install tools, run
paid agents, add acceptance, lower coverage or widen the license policy.

The chief authors and commits only `specs/wrapper-integration/spec.md`,
`plan.md` and `tasks.md`. Implementation performs the merge and documentation
repair afterward. Design-check validates this chief commit, not an older design
selected by mtime. Required implementation proofs remain outstanding even if
the structural design check passes.

## Research and resolved choices

Both positions and both generations of intake were read. The current
[integration intake](../../evals/issue151-integration-intake.md) supersedes the
older index exclusion. The original issue README has no prior-run entries;
use the supplied task and [receipt](../../evals/issue151-implementation.md),
never invented journal outcomes. No missing input blocks this design.

Local main contains #243 plus #234: incoming executable diffs affect the test
modules in `evals/src/artifacts.rs` and `evals/src/fixture.rs`, and the tracked
Markdown boundary tests. They are required incoming proof repairs, not authored
wrapper changes. Dropping these changes or cherry-picking only README text
would fail the integration intent. Runtime failure-mode analysis from robustness
is satisfied by unchanged code plus existing real-protocol and coverage proofs;
it does not justify new lifecycle or storage mechanisms.

## Project structure and touched files

| Owner/path | Implementation action |
|---|---|
| `evals/README.md` | Resolve the merge by keeping the #243 tracked authority link/boundary section and all #151 wrapper guidance. |
| `docs/rulings/epic-192.md` and main's other tracked link repairs | Retain incoming #243 content exactly; do not introduce links into ignored packets. |
| `evals/src/artifacts.rs`, `evals/src/fixture.rs`, `evals/tests/boundary.rs` | Retain incoming proof changes alongside existing candidate work; no additional executable changes. |
| `docs/decisions/0083-cli-wrapper-launch.md` | Rename the existing wrapper proposal, correct title/allocation and add dated integration scope; preserve proposed status and runtime contract. |
| `docs/decisions/README.md` | Add exactly one resolving proposed row for 0083. |
| `specs/cli-wrapper-launch/{spec,plan,tasks}.md` | Correct proposal references and obsolete present-tense scope; preserve detailed launch requirements and dated history. |
| `evals/wrappers.md`, `evals/issue151-implementation.md` | Correct proposal references, preserving factual prior verdicts; date the newly authorized integration ownership. |
| Other tracked references found by inventory | Correct only references to this proposal; leave unrelated allocations and accepted history intact. |
| Existing integration intake/implementation receipt | Record tested commits, command outcomes and remaining ownership without changing historical exits. |

All other main changes enter through ancestry as supplied. No hand-authored
root runtime, manifest/lock, gate, recipe or wrapper edit is authorized.

## Data model and contracts

There is no new runtime data model, API or protocol. The documentation identity
migration has one final wrapper decision file, one proposed index row and
resolving references. Do not leave an alias under the collided number. Record
historical failures as, for example, "the wrapper proposal, now 0083, lacked an
index row in the earlier run" rather than converting the old receipt to a pass.

The existing private credential selection, eight EVAL inputs, fixed six-model
launch recipes, required canonical tool-surface metadata, strict current readers,
sequential controls and cleanup lifetime are unchanged. Only observed offline
proof can be claimed. A dry-run is a plan, not a baseline or provider session.

## Implementation sequence

1. Require a clean index/worktree; record the current design HEAD, supplied
   main OID and merge base in ignored evidence. Recheck authority/engine digests.
   Inventory the baseline paths with `git ls-tree -r --name-only 6a67f7e evals`.
2. Merge the recorded locally available main tip without fetching and without
   signing. Preserve both parents. Resolve `evals/README.md` by section; neither
   whole-parent choice is correct. Inspect the result against both parents and
   their merge base. Preserve #234/#243 proofs and all candidate wrapper work.
3. Rename the existing wrapper proposal to the allocated 0083 path, correct its
   number/title/allocation, and add its unique proposed index row. Search tracked
   files for the old wrapper filename and textual decision citations; correct
   all references to this proposal without globally replacing the bare number.
   Reconcile obsolete index/#243 ownership with a dated 2026-10-26 integration
   statement in the existing spec/plan/tasks and proposal. Preserve recorded
   failure exits and earlier dated clarifications. Do not accept any decision.
4. Establish byte preservation with `git diff --exit-code 6a67f7e --` over
   `evals/src/wrappers.rs`, `evals/src/main.rs`, `evals/src/baseline.rs`,
   `evals/agents-baseline.json`, `evals/tests/wrappers.rs`,
   `evals/tests/wrappers_launch/`, `evals/tests/common/` and
   `evals/tests/baseline.rs`. These include the subcommand entrypoints and
   helper scripts; no separate tracked wrapper shell entrypoints exist in the
   inspected baseline. Extend the inventory if enumeration finds another owner.
   Check remaining executable differences against the incoming main patch.
5. Commit the integrated repair unsigned before the full proof run. Run every
   command below, retaining full output and exit status with the tested OID.
   If a gate fails, retain the exact failure and diagnose within scope; do not
   weaken tests, claim completion, or silently expand behavior. Reprove affected
   gates after any authorized repair. A transport timeout requires reading the
   recorded process completion and exit, not inferring a verdict.
6. Append a dated receipt with command results and external owners, commit it
   unsigned and verify tracked cleanliness. Record which tested commit the
   documentation-only receipt follows; repeat documentation/inventory checks
   on the final receipt. Any executable or gate-input repair requires the
   corresponding fresh proofs. Keep the design chief's HEAD untouched until
   its immediate deterministic check has run.

## Quickstart and verification

Use `CARGO_NET_OFFLINE=true`, the candidate-local engine and pinned script
owners. The authoritative house inventory is
[the charter](../../docs/house-rules.md#gates--part-of-every-change); the
[integration intake](../../evals/issue151-integration-intake.md#verify-commands)
instantiates every exact command, including supplemental and eval proofs.
Do not execute unchecked public shims as proof owners. The required invocation
set is reproduced here for implementation execution, not a new gate policy:

```bash
export CARGO_NET_OFFLINE=true
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum --check --strict
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --locked --test store_contract_docs
.forge/bin/brokkr compile --bundle bundles/self
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --locked
cargo test --manifest-path evals/Cargo.toml --locked --test boundary
cargo test --manifest-path evals/Cargo.toml --locked --test wrappers
cargo test --manifest-path evals/Cargo.toml --locked --test baseline
cargo test --manifest-path evals/Cargo.toml --locked --test proof
```

Record commands separately so a failed check cannot suppress the remaining
inventory. Bundle compile must have zero unruled valuations and no policy-sweep
warning. Execute native-tls absence in a separate shell, preserving the exit:

```bash
out=$(cargo tree -i native-tls 2>&1); code=$?
if [ "$code" -eq 0 ]; then exit 1
elif printf '%s' "$out" | grep -q 'did not match any packages'; then exit 0
else exit 1; fi
```

Run the actual command using a known-absent output path (do not remove any
existing operator data just to make it absent):

```bash
test ! -e /tmp/issue151-integration-dry-run-only
cargo run --manifest-path evals/Cargo.toml --locked -- baseline --config /private/wrappers.json --output /tmp/issue151-integration-dry-run-only --dry-run
test ! -e /tmp/issue151-integration-dry-run-only
```

Capture and parse the emitted JSON to check AC5 cardinalities, deadline,
concurrency, ordered controls/validation/paid/report steps and output paths.
Use the existing baseline refusal/no-read/no-launch tests as the accompanying
negative proofs. No provider CLI, live credential file, benchmark or paid trial
is needed. The full eval workspace remains mandatory even if direct targets
pass. Root Cargo does not run it.

For this design commit only, run the pinned `design-check.sh check .` and
selftest; keep exactly these three artifacts committed and clean. The required
house run at this stage still exposes the carried inventory/coverage failures
until implementation repairs them; report actual outcomes without certifying
the future integration. Final engine validation alone rules the design phase.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| README resolution drops wrapper guidance or #243 links | Resolve by section and compare both parents; run full boundary tests. Accept manual integration review. |
| Number substitution corrupts #152 or historical facts | Search the specific wrapper proposal and contextual citations; require one proposed 0083 row; preserve dated failure exits. Accept old external links breaking; no alias under the wrong number. |
| New design duplicates functional requirements | This spec owns integration only; the launch spec keeps its detailed contract. Reconcile old scope statements during implementation. |
| Unchanged wrappers interact differently with incoming proof changes | Require byte comparisons and complete root/eval proofs on committed merged content; do not reuse old passes. Accept full gate runtime. |
| Tracked link passes only with local ignored evidence | Retain #243's tracked owner, scanner and negative controls; never fabricate missing packets or link these artifacts into delivery state. |
| Partial command output is mistaken for completed verification | Keep tested OID, command, completion marker, exit and full log; unfinished processes remain unproved. |
| Gate failure tempts broader changes or weaker assertions | Diagnose within authorized integration; record exact remaining work if an unrelated repair is needed. No floor, denominator, assertion, dependency or routing change. |
| Offline stand-ins are mistaken for authenticated interoperability | State operator-owned six-model/tool-inventory smoke and paid baseline; retain tool asymmetry and unconfined limits. CI/#201 owns platform/network evidence; #221 owns namespaces. |
| Frozen runtime guarantees drift during the merge | Compare executable changes against both parents, preserve existing real-protocol tests and exact coverage; no new storage, webhook, pruning, extraction or cleanup semantics. |
