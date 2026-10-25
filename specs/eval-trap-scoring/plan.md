# Eval trap scoring implementation plan

Reconciled 2026-10-25 against accepted 0071 and the current integrated base.

## Technical context

Use the existing standalone unpublished Rust workspace in evals/, its own
lockfile and existing HTTP/MCP clients. Root code provides real SMTP/MCP.
No new dependency, workspace member, production seam or public CLI is needed.
Baseline: dc1822a45e4d2ae8326b44baf22818187a8320e9.

No plan-template file exists in the checked-in template directory. This plan
supplies the required context, constitution check, research, project structure,
data model, contracts, quickstart and risks inline. No auxiliary design files
are added under the chief's exact artifact limit.

## Constitution check

Preserve 0001–0005 by leaving root runtime unchanged and proving mail through
real SMTP and MCP. Each trial owns fresh loopback port-zero servers, unique
AUTH inboxes and private temporary artifacts. No shared clear_all, query-time
extraction, TLS bypass, storage inference from callbacks, or retained-mail
capacity too small for the planted alternatives. Failed/parked deliveries,
including decoys, remain infrastructure. No persistence, webhook durability,
hostile MIME or thousand-session throughput claim follows from these evals.

Accepted [0071](../../docs/decisions/0071-eval-trap-scoring.md) authorizes the
0055 §§1/2/9 amendment. Its accepted index row already exists. Validate that
agreement without rewriting accepted records or introducing another proposal.
Preserve #194 cancellation ownership and #196 closed child environments, safe
diagnostics, bounded stream capture and private artifact publication. Update
schema-sensitive fixtures without weakening their assertions or restoring older
runner/helper source wholesale. Candidate bundle routing remains unchanged.

The single authoritative root inventory is
[the charter](../../docs/house-rules.md#gates--part-of-every-change).
Every item remains required, including exact coverage and selftest,
publish/seal proofs, pinned bundle compilation, licenses and fail-closed
native-tls absence. Root coverage excludes evals under 0055: independently
run evals fmt, locked all-targets clippy and locked tests. No exclusion,
denominator/dependency change or protected-review bypass is authorized.

## Research and resolved questions

Read intake and both positions, decision index, 0001–0005, 0055, original eval
spec, scorer, fixture, state, runner, CLI, report and proof helpers. The
early goal return still permits recovered traps to pass in current code;
accepted 0071 now authorizes correcting that behavior.
Decoys can answer HTTP 200, so status codes cannot classify the trap.

The ordered events already carry lasting evidence. Private::claim can
refuse a known Decoy before token lookup; follow_link supplies null flow.
Preserve that path explicitly. Ordinary matches still require exact flow,
class, action, rejected status and provisioned Agent claimant.

Choose superseded_code alone, explicit v2 contracts and exact matching
without broader event-validation redesign. The spec records the losing
alternatives and costs. Existing metadata lacks a required scoring revision;
its package version and binary path do not prevent mixed report rows.
No unresolved research choice blocks this design.

## Project structure and files

| Owner | Planned work |
|---|---|
| evals/src/tasks.rs | Private scenario/event classifier; scan after validation before goal return; goals and catalogue v2 |
| evals/src/state.rs | Shared trap action constants and snapshot v2; no flags or fields |
| evals/src/fixture.rs | Replace matching action literals with constants; preserve public behavior and freeze |
| evals/src/runner.rs | Result v2 and comments; preserve process/cleanup/precedence |
| evals/src/main.rs | Preserve callers and #194/#196 ownership; update inline matrix probe schema fixtures and unsupported-version cases; safe diagnostic plumbing only if actual CLI proof requires it |
| evals/src/report.rs | Existing strict result validation rejects old/mixed input; preserve arithmetic/aggregate shape |
| evals/tests/scoring.rs | Policy, boundary, recovery and validation/version matrix |
| evals/tests/common/mod.rs | Independent bounded external helper modes through existing dispatch |
| evals/tests/proof.rs | Completed naive strategies, actual check replay, oracle/null/isolation |
| evals/tests/fixture.rs | Real producer evidence and HTTP action/freeze assertions |
| evals/tests/runner.rs, evals/tests/report.rs | V2 CLI/artifact/report and trap-bearing precedence proofs |
| evals/tests/boundary.rs | Retain dependency/publication proof; extend only for a concrete uncovered boundary |
| evals/README.md, specs/agent-eval-harness/spec.md | Dated active v2 scoring and historical replay without migration |
| specs/agent-eval-harness/plan.md and tasks.md | Reconcile obsolete active instructions, preserving historical entries |
| docs/decisions/README.md and tests/store_contract_docs.rs | Read/validate the existing exactly-one accepted 0071 row; no index edit planned |
| specs/eval-trap-scoring/{spec,plan,tasks}.md | This chief's only committed artifacts; 0071 remains unchanged authority |

## Data model

Keep Snapshot, Event, Facts, Delivery, Task and TrialResult field sets.
The immutable increasing Event.seq is already validated. Facts reflect real
transitions: a failed trial may have a true goal. No taint flag or second
event store.

A private match returns an existing FailureClass using scenario and snapshot
identity context. Ordinary matches use exact action/category/flow, rejected
status and Agent claimant. Sibling additionally permits provisioned Decoy
foreign-token refusals regardless of recorded flow, including null. No
matching by class alone and no configurable policy language.

Advance SNAPSHOT_SCHEMA, CATALOGUE_SCHEMA and RESULT_SCHEMA together to 2.
Update tests hardcoding 1 or treating 2 as unsupported. Unknown-version tests
must still use an actually unsupported version. Report parsing already calls
TrialResult::validate; its CLI must refuse old/mixed input before aggregation.
No historical selector/migration framework; old artifacts need the old binary.

## Contracts and implementation sequence

1. Verify packet checksums, accepted decision/index and pinned engine digest.
   Run the documentation inventory test; acceptance is already established.
2. Prepare independent bounded probes using existing test-helper dispatch and
   public SMTP-backed MCP/fixture HTTP. Read links/codes from mail and complete
   the real goal without run_oracle or private state. Await every response.
3. Materialize the locally available baseline commit
   dc1822a45e4d2ae8326b44baf22818187a8320e9 into ignored storage inside this
   workspace (for example by git archive). Never reset this candidate or access
   sibling worktrees. Compile baseline with its own source, versions and locked
   dependencies; carry only independent probe/helper additions, recorded as a
   patch with source hashes. Keep outputs separate from candidate artifacts.
   Existing build tooling is offline; install nothing. If an input is absent,
   name its exact path/revision rather than claiming a red result.
4. Retain actual baseline unwanted passes: source revision and helper patch,
   literal command/argv, raw zero child exits, healthy DATA 250 deliveries,
   exact events and recovered facts. Compile, version, timeout, startup or
   infrastructure failure is not red. Reuse the same public strategies on the
   candidate and retain the matching green receipts. Parent test assertions may
   inspect frozen admin snapshots; helpers never use them to solve a task.
5. Introduce shared constants and private trap scan between validation and
   goal scoring. Leave isolation/fallback intact. Update goals and all v2
   constants as one authorized contract change; never emit stricter scores
   labeled v1 in an intermediate release.
6. Add pure tables for all actions and both goal orders, exact nonmatches,
   earliest trap and recovery. Keep existing malformed-state refusal.
   Test expiry recovery deterministically, not by sleeping for expiration.
7. Land real naive controls for confirm forward/reverse, both sibling
   claimant paths, old/current OTP, Unicode subject, magic links, entity
   links/malformed query and all digest links. Assert zero child exit,
   DATA 250 receipts, trap evidence and recovered goal alongside failure.
8. Replay every healthy completed negative case through actual check.
   Exercise v1/mixed refusal, v2 accounting, nonzero-child-success absent
   traps, all oracle/null cases and overlapping isolation. Use existing lock
   and real request completion to test wrong/correct concurrency and freeze:
   pre-freeze events count, subsequent refusals cannot mutate snapshot.
9. Reconcile documentation and red/green receipts; run focused then full evals
   tests and all pinned gates/review. No model measurements or publication.

## Quickstart and validation

For the implementation candidate, from the repository root:

```bash
cargo build --offline --locked --bin swarmail
cargo test --locked --test store_contract_docs
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --locked --test scoring
cargo test --manifest-path evals/Cargo.toml --locked --test proof
cargo test --manifest-path evals/Cargo.toml --locked
```

The proof helper resolves the current checkout's server once and uses
literal external-command argv. Manual retained-state replay:

```bash
evals/target/debug/swarmail-evals check --task confirm-decoys --state PATH_TO_STATE_JSON
```

Replace PATH_TO_STATE_JSON with the actual retained v2 state. Completed link
enumeration must return fail/wrong_link with the CLI's failure exit. Snapshot
verdict equality does not mean full live-result equality.

Then execute every charter gate with digest-matched pinned brokkr/gate copies
and checked shim hashes. Retain source-bound commands, exits and logs.
Hermetic verify installs nothing. Preserve CI-only chart/history/selftest,
Docker, advisory, release-rate and port-80 obligations without claiming local
proof. #201 owns macOS eval evidence, the operator owns live settings/sealing,
and #221 owns namespaces; none creates a new local acceptance criterion. The
candidate bundles own the actual review routing and hard security hold; do not
copy obsolete reviewer counts into a new delivery requirement. Chief structural
preflight is advisory; the sequence's pinned check owns the design verdict.

## Risks

| Risk | Mitigation / accepted cost |
|---|---|
| Stale instructions undo accepted authority | Validate existing accepted 0071/index; reconcile active text, preserve historical receipts |
| Schema change weakens #194/#196 regression proof | Audit inline main.rs probe scripts and all synthetic/error/cancellation rows; use a truly unsupported version in refusal tests |
| Old/new experiments silently mix | Required v2 boundaries, report refusal, old binary for historical replay |
| Sticky rule punishes recovery or reset replay | Fixed mapping, supersession-only rule, non-target/replay controls |
| Null-flow Decoy claim escapes matching | Real producer proof and Agent/Decoy/null-flow scoring cases |
| Edited evidence passes | Existing validation plus documented nonmatches; no hostile-artifact integrity claim |
| Broken helper looks like trap resistance | Assert zero exit, DATA 250, traps and true goal before class |
| Digest first-link test succeeds accidentally | Enumerate 40 and assert 39 decoys plus one correct event |
| Freeze loses fire-and-forget actions | Await responses; real HTTP sequence and frozen-immutability proof |
| Real proof runtime/resources | Bound every helper, reuse server binary, isolate trial state, preserve infrastructure classification |
| Root green hides eval defects | Independent evals suites plus full pinned gates and protected review |
| Scope expands into known residuals | Retain unbounded event-log and agent-triggered infrastructure limitations; no cap, attribution redesign, lockout or root repair |

## Historical chief validation receipt — 2026-10-24

Preserved verbatim below as the earlier run's evidence, not current instructions.
The missing-index condition and proposed status were superseded before this
2026-10-25 design; see the spec's dated clarification.

Executed cargo test --locked --test store_contract_docs against these draft
artifacts: exit 101, 17 passed and 2 failed. Both
every_decision_file_has_exactly_one_resolving_index_row and
the_inventory_refuses_missing_dangling_duplicate_and_malformed_entries
reported the added 0071-eval-trap-scoring.md has no index row. This is the
known T1 integration obligation under the chief's exact four-file limit,
not a clean full-gate result. Artifact nonemptiness, required headings,
checkbox presence, proposed status and relative links were checked locally.
No behavioral red/green, full gate inventory, independent review, CI or ship
receipt is claimed by this design seat. The subsequent pinned design check
remains the sequence's authority.


## Chief validation receipt — 2026-10-25

Executed against base fa58d8b with only the three design artifacts changed.
Run-local commands, logs, status table, source HEAD and tested design patch are
retained under `.forge/epic192/issue198/chief-validation/`. Packet checksums,
the candidate-local engine digest and all pinned public-shim hashes passed.

- Root fmt, locked all-target clippy with denied warnings and full locked
  workspace tests passed; store_contract_docs passed all 25 tests, including
  the existing accepted 0071 inventory/status agreement.
- `.forge/bin/brokkr compile --bundle bundles/self` passed with zero unruled
  policy valuations. No routing change was made.
- The pinned coverage gate passed: 13,850/13,850 lines, 780/780 branches,
  1,508/1,508 functions; retained evidence is `target/coverage/run.cdg2rG`.
  Its selftest passed. Pinned publish-guard, seal-branch, design-check selftest
  and delivery-proof tests passed; `cargo deny check licenses` and the
  fail-closed locked native-tls absence proof passed.
- Independent evals fmt and locked all-target clippy passed. The first full
  locked test invocation exited 101: the existing
  `tests::matrix_stdout_backpressure_is_bounded_and_still_observes_signals`
  test failed at `evals/src/main.rs:2312` reading missing `signals.log`.
  The unchanged isolated test passed, then the complete locked suite with
  `--no-fail-fast` passed. The initial failure remains in `eval-tests.log`;
  this is an observed intermittent failure, not a demonstrated root cause or
  a repaired defect. Preserve and investigate it if it recurs during T9/T11.

These checks validate the current pre-implementation source and this design
handoff. They do not prove the future v2 matcher, completed naive red/green
controls, independent engine review or CI/operator-owned work. The structural
check of the final design commit is a separate advisory preflight; the
sequence's pinned final check owns its design verdict.
