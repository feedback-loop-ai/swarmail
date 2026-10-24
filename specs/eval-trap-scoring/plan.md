# Eval trap scoring implementation plan

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

0062 amends 0055 §§1/2/9 and requires operator acceptance before conflicting
scoring/prompts/version changes. The index repair and baseline probes are
compatible work. Accepted records stay unchanged. Add the proposed 0062 index
row in the next seat, outside the exact four-file design commit. Do not
suppress the resulting documentation-gate gap.

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
early goal return permits recovery after traps by accepted contract.
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
| evals/src/main.rs | Existing check/report callers; adjust only if diagnostics need plumbing |
| evals/src/report.rs | Existing strict result validation rejects old/mixed input; preserve arithmetic/aggregate shape |
| evals/tests/scoring.rs | Policy, boundary, recovery and validation/version matrix |
| evals/tests/common/mod.rs | Independent bounded external helper modes through existing dispatch |
| evals/tests/proof.rs | Completed naive strategies, actual check replay, oracle/null/isolation |
| evals/tests/fixture.rs | Real producer evidence and HTTP action/freeze assertions |
| evals/tests/runner.rs, evals/tests/report.rs | V2 CLI/artifact/report and trap-bearing precedence proofs |
| evals/tests/boundary.rs | Retain dependency/publication proof; extend only for a concrete uncovered boundary |
| evals/README.md, specs/agent-eval-harness/spec.md | Dated active v2 scoring and migration after acceptance |
| specs/agent-eval-harness/plan.md and tasks.md | Reconcile obsolete active instructions, preserving historical entries |
| docs/decisions/README.md | First compatible follow-up: one proposed 0062 index row |
| specs/eval-trap-scoring/{spec,plan,tasks}.md and docs/decisions/0062-eval-trap-scoring.md | This chief's only committed artifacts |

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

1. Add the proposed index row and prove documentation inventory. Prepare
   baseline-compatible probes in isolated run-local storage without modifying
   the triage baseline.
2. Extend existing helper dispatch in those isolated probes. Read public mail
   via MCP, enumerate links or select the wrong message/code, then reach the
   correct goal using public content. Reuse transport utilities, not
   run_oracle as a solution. Await every HTTP response.
3. Execute baseline pure/real probes. Retain command, source revision, exits,
   delivery receipts, events, true facts and unwanted pass. A compile or
   version refusal is not red evidence. Do not modify baseline version numbers
   to manufacture a comparison.
4. Record operator acceptance before incompatible work. Absent acceptance,
   leave semantic implementation open rather than changing fixture facts to
   evade the boundary.
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

After acceptance and implementation, from the repository root:

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
proof. The full engine lane requires four independent review lenses and hard
security hold. Chief structural preflight is advisory; the sequence's pinned
check owns the design verdict.

## Risks

| Risk | Mitigation / accepted cost |
|---|---|
| Implementation precedes authority | Acceptance task blocks changed scoring, prompts and versions |
| Four-file chief commit leaves unindexed ADR | First follow-up index task and inventory proof; no full-gate success claim |
| Old/new experiments silently mix | Required v2 boundaries, report refusal, old binary for historical replay |
| Sticky rule punishes recovery or reset replay | Fixed mapping, supersession-only rule, non-target/replay controls |
| Null-flow Decoy claim escapes matching | Real producer proof and Agent/Decoy/null-flow scoring cases |
| Edited evidence passes | Existing validation plus documented nonmatches; no hostile-artifact integrity claim |
| Broken helper looks like trap resistance | Assert zero exit, DATA 250, traps and true goal before class |
| Digest first-link test succeeds accidentally | Enumerate 40 and assert 39 decoys plus one correct event |
| Freeze loses fire-and-forget actions | Await responses; real HTTP sequence and frozen-immutability proof |
| Real proof runtime/resources | Bound every helper, reuse server binary, isolate trial state, preserve infrastructure classification |
| Root green hides eval defects | Independent evals suites plus full pinned gates and protected review |

## Chief validation receipt — 2026-10-24

Executed cargo test --locked --test store_contract_docs against these draft
artifacts: exit 101, 17 passed and 2 failed. Both
every_decision_file_has_exactly_one_resolving_index_row and
the_inventory_refuses_missing_dangling_duplicate_and_malformed_entries
reported the added 0062-eval-trap-scoring.md has no index row. This is the
known T1 integration obligation under the chief's exact four-file limit,
not a clean full-gate result. Artifact nonemptiness, required headings,
checkbox presence, proposed status and relative links were checked locally.
No behavioral red/green, full gate inventory, independent review, CI or ship
receipt is claimed by this design seat. The subsequent pinned design check
remains the sequence's authority.
