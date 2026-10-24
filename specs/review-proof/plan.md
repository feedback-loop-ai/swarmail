# Plan: Required review proof

## Technical context

One Rust application crate, a standalone unpublished eval workspace, and
Python/Bash/JSON delivery resources. The external recorded Brokkr engine owns
dispatch, correlation, aggregation and policy; do not duplicate these algorithms.
No new dependency, application workspace member or data migration is planned.

Current tooling records base 731189f9097308c01ae4f2b4c9caea5f41224844,
binary SHA-256 ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f
and three ordered checksum-pinned patches. This is a read record, not this
seat's build attestation. Verify actual bytes against dispatched expectations.
There is a spec template but no plan template; this plan supplies the required
dialect sections explicitly.

## Constitution check

Preserve accepted 0001–0005, 0007/0013 as amended by 0057/0058, 0045 §8,
0050, 0055 and 0056. [0062](../../docs/decisions/0062-review-proof.md) is proposed,
not authority to contradict existing rulings. Preserve protected review,
security-only panel hold, witnessed security, unknown-fact refusal, bounded
returns, candidate binding, offline verification and exact coverage.

Chief commits exactly the new spec/plan/tasks and proposed 0062. It is the next
number after existing 0061. Indexing 0062 is a downstream implementation task,
outside this four-file commit. The intermediate design therefore makes no
full documentation-index or house-gate pass claim. Deterministic design check
rules this phase; there is no gate bypass.

## Research and current evidence

The complete packet, intake and both positions are accounted for in spec.md.
The Python script infers ROOT from its location, which breaks on relocation
or pinned mounting. It skips self, checks all implementers against Sol, uses
inequality negatives, terminalizes return targets and writes only a final
summary. Completion status is too weak an oracle.

verify-seat.sh already has resource preflight, public shim checks, launched
engine hashing, offline gates and typed fail results with exit zero.
The existing CI brokkr-compile job checks out source, applies ordered patches
and tests core policy, aggregation and candidate binding. Its post-build check
currently compares version only. Require actual recorded build/executable
provenance, not an inference from version equality.

Before any tooling repair obtain authentic recorded source through the owning
workflow, identify the process correlation owner and list its exact regression
filters from that checkout. Existing known filters read from CI are:

    cargo test --locked -p brokkr-core policy::tests
    cargo test --locked -p brokkr-runtime --lib aggregate
    cargo test --locked -p brokkr-runtime --lib candidate_binding_tests

The protocol-specific filter is a mandatory research output before patch/pin
change; it cannot honestly be invented from this checkout. Missing source,
build inputs or faithful owner interface stops the dependent work with exact
security/proof evidence. Retained framing does not satisfy original-issue/
journal inspection; recover or retain the explicit gap.

## Project structure and touched files

| Owner | Planned implementation |
| --- | --- |
| tests/codex_review_routing.py | Remove after moving the single algorithm; update real references. |
| bundles/self/scripts/codex-review-routing.py | Canonical literal oracle, real-engine cases, explicit roots, atomic receipts. Keep helpers here unless a distinct pinned helper is necessary. |
| bundles/self/scripts/verify-seat.sh | Require canonical resources and ordinary proof before pass, using launched digest-checked engine. |
| bundles/self/scripts/delivery-proof-test.sh | Add outer routing-dispatch; preserve default and eval-dispatch; fresh outer-candidate receipts. |
| bundles/self/bundle.json and resource closure | Confirm actual transitive resource walk; declare additions only if needed. Inherit verify intact in child recipes. |
| .github/workflows/ci.yml | Extend existing recorded-engine/proof owners with build identity, canonical routing and required outer proofs; retain failure artifacts. |
| docs/house-rules.md | Add required proof commands and explain inner/outer ownership in the single inventory. |
| tests/delivery_recipes.rs; tests/delivery_constitution.rs | Targeted structural route/wiring regressions supplemental to behavior. Preserve accepted Luna assertions. |
| agents/README.md; bundles/README.md | Correct current 0058 status. |
| evals/README.md | Restore current commands/agent/schema/failure reference; keep wired gates and honest limits. |
| specs/codex-review-routing/{spec,plan,tasks}.md; specs/eval-gates/tasks.md | Downstream dated status/receipt mapping, obsolete current pin/status correction, five-role continuation note. Not edited by chief. |
| docs/decisions/README.md | Index 0062 as proposed downstream. No acceptance or rewrite of accepted decisions. |
| bundles/self/tooling.json; bundles/self/tooling-patches/ | Conditional bounded owner repair with reviewed ordered patches and reproducible build provenance; no incidental upgrade. |
| scripts/apply-brokkr-patches.py | Preserve existing ordered validated patch owner unless a demonstrated defect requires bounded repair. |
| Ignored evidence | Source references, per-case traces, build and final verification/review receipts; never reconstructed journals. |

Adapters, role charters, accepted routes, policies, public shims and application/
eval behavior are preservation inputs. A proved defect beyond this plan returns
with precise scope evidence.

## Implementation approach

1. Build a complete disposition ledger and capture baseline identities. Recover
   authentic originals if accessible; explicitly label gaps. Inventory owner
   interfaces and defensive regression coverage before any clean claim.
2. Move the algorithm once. Define --candidate-root, --pin-root, --engine and
   --out; pin-root is the dispatched bundle root containing scripts, policy.json
   and tooling.json. Candidate root supplies configuration data only. Validate
   proof/resource/engine identity against dispatched expectations. Require a
   fresh output directory; remove old developer entry without a shim.
3. Implement a small literal accepted matrix. Normalize only presentation;
   actual and mutated data enter one checker with stable diagnostics. Separate
   real selector refusal from checker cases. Validate endpoint/model/effort and
   office, not alias alone. Keep exact legacy self expectations without imposing
   Codex-only restrictions on authorized legacy chains.
4. Connect controlled provider-boundary participants to resolved compositions.
   Retain production dispatcher/aggregator/protected policy. Assert calls,
   result identities, fields, rules and genuine return traces. Determine retry/
   fallback counts from recorded owner semantics. Generic fixture failures are
   infrastructure failures. Use actual nonterminal targets or recorded-core
   harness; never a local policy clone.
5. Establish identity through defensive owner tests with valid completion/retry/
   resume contrasts. If live, correct correlation once at its owner, review
   source/regressions and apply checked ordered patch/build mechanism. Record
   compiler, lock, source composite and binary identity; run known core/runtime
   filters plus discovered protocol tests. No digest bypass or consumer cleanup
   after admitting foreign events. If proof/repair unavailable, retain
   security=true and stop with exact evidence and bounded corrected proposal.
6. Wire ordinary proof into inherited pinned verify and existing CI. Verify
   remains offline with the launched recorded binary. CI establishes source,
   ordered patches and corresponding executable identity. Nonreproducibility
   remains a build/provenance gap, not permission to whitelist current output.
   Confirm candidate-local engine preflight before new dispatch.
7. Add outer routing-dispatch refusal/restoration to existing proof driver.
   Require it in owning outer CI/delivery invocation alongside current-candidate
   eval-dispatch. Neither mode is called by verify; default tamper proof stays
   nonrecursive. Each mutant needs its own diagnostic/typed fail and restored
   success. An unrelated inner failure cannot certify a negative case.
8. Restore documentation, index proposed 0062, and reconcile status only against
   evidence. Prepare candidate under existing policy, run every final-head
   gate/proof and obtain five-role protected review. Any later tracked edit
   creates a new candidate requiring fresh evidence; store execution receipts
   ignored rather than creating a post-verification tracked receipt commit.

## Data model and contracts

No application schema migration. Keep engine result envelope exactly result,
inputs, notes and existing vocabulary. Proof receipts are ignored evidence,
not new policy facts or acceptance tokens.

Each case records expected outcome, full outer HEAD/tree/dirty/mutation digest,
separate fixture identity, proof/helper hashes, engine base/ordered patches/
binary digest, recipe/strategy, participating offices/models and run/effect/
attempt/member/result identities, command/exit/timeout/budget, typed results/
aggregate, decisive rule/destination/trace and specific pass/fail/incomplete
cause. Write each receipt atomically even on assertion failure or timeout.
A final atomic summary enumerates the complete required case set; missing,
truncated or incomplete summary cannot pass.

Use fresh case-owned journals, scratch repos, outputs and child processes.
Only a one-run isolated fixture may use latest. Bound logs and subprocess
lifetimes, retain diagnostics without credentials, and reap owned children
on cancellation. Use rendezvous/barriers/deadlines, never sleep ordering.
No resume/reuse implementation: interrupted suites rerun in fresh directories.

## Quickstart and verification

Planned canonical developer interface after implementation:

    python3 bundles/self/scripts/codex-review-routing.py \
      --candidate-root "$PWD" --pin-root "$PWD/bundles/self" \
      --engine "$PWD/.forge/bin/brokkr" --out "$fresh_proof_dir"

This interface does not exist on design HEAD. Pinned verify uses equivalent
arguments with its dispatched bundle root. Developer invocation alone does not
discharge workflow execution.

Targeted existing tests and all four recorded-engine compiles:

    cargo test --locked --test delivery_recipes --test delivery_constitution --test store_contract_docs
    "$engine" compile --bundle bundles/self
    "$engine" compile --bundle bundles/codex
    "$engine" compile --bundle bundles/codex-careful
    "$engine" compile --bundle bundles/codex-fast

Run actual dispatched verify with engine-generated prompt and recorded binary,
checking typed result. Run pinned design selftest, default delivery tamper proof,
new outer routing-dispatch and existing outer eval-dispatch through their owning
outer workflow with fresh receipts. The full
[house inventory](../../docs/house-rules.md#gates--part-of-every-change)
remains authoritative, including root-artifact-bound full evals, locked/offline
operation, exact coverage/selftest, publish/seal and license/TLS proofs.
Chart/history/selftest, Docker, network advisory, tag-rate and privileged
port-80 checks keep CI-only ownership; this task publishes nothing.

For design, run the pinned validator against the last chief commit and its
selftest. This proves structure only. Full gates and high-risk review remain
downstream, not design claims.

## Risks

| Risk | Mitigation / accepted cost |
| --- | --- |
| Historical identity defect live or source unavailable | Preserve high/security evidence; owner proof/repair or stop. Gates cannot establish identity closure. |
| Relocation selects wrong algorithm | Explicit roots, pinned executable closure, actual dispatch/tamper proof. |
| Unrelated failure masquerades as refusal | Named diagnostics, typed facts/rules, reached-case set and positive contrasts. |
| Return fixture hides actual behavior | Genuine targets or recorded-core harness; terminalized fixture insufficient. |
| CI build identity differs | Record source/patch/compiler/binary provenance; mismatch refuses, no automatic digest admission. |
| Outer proof recursion/timeouts | Outer-only modes, bounded cleanup and measured integrated budget; timeout stays incomplete. |
| Old or parallel evidence reused | Full outer/fixture distinction, fresh isolated outputs and before/after identity checks. |
| Overbuild | Existing owners, one Python algorithm, no compatibility shim/framework or speculative runtime work. |
| Proposed record mistaken for acceptance | Clear status; index downstream; no full-gate claim for intermediate design. |
| Restored docs revive obsolete guidance | Compare deletion with current CLI/accepted 0055; retain current gates/artifact guidance. |
| Five roles called five independent models | Separate Astra identities and independent Fable; disclose model correlation. |
| Commit restriction/evidence mismatch | Chief four-file commit expressly requested; later commits follow ship policy, ignored receipts avoid post-verification edits. |
