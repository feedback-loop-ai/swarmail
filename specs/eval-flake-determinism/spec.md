# Eval flake determinism (issue #302)

Status: design, run `issue-302-make-the-flaky-eval-te-e944c222`, base main
`c336c95`, 2026-11-02.

## The feature

Three evals tests hard-stop the verify seat under seat load (several delivery
runs and cargo test binaries on one host) while passing on an idle host. Since
#201/#300 the verify seat runs the evals gates, so each flake fails delivery
runs that never touch `evals/`. This change makes all three deterministic by
removing the cause of each one. Every test keeps observing its subject.

| # | Test | Cause (ruled here) | Owner of the fix |
|---|---|---|---|
| 1 | `evals/src/runner.rs` `runner::test_probes::cleanup_observation_error_and_forced_expiry_fail_with_live_owned_processes` | The injected observation fault travels through one process-global slot, `CLEANUP_OBSERVATION_TARGET`. Owned groups clean up in parallel, so one group's store can overwrite another's. The fault then fails the wrong group, and the agent group escalates to SIGKILL and kills the TERM-proof survivor before `any(alive)` looks. | the `cfg(all(test, target_os = "linux"))` fault seam in `evals/src/runner.rs` |
| 2 | `evals/tests/runner.rs` `child_output_server_death_remains_a_harness_error_on_linux` | The failing line was the exit-0 witness. At a4f552d, `tests/runner.rs:1620` is `assert_eq!(row.agent_exit_status, Some(0), ..)`, and the quoted `service_exited_during_trial` text is that assertion's *message* (the captured stderr). So the classification was right. The helper SIGKILLs `serve` and then returns 0. When the runner reaps the server death first, it TERMs the agent group while the helper is still returning, and with the default disposition the helper dies of a signal. | the helper arm `unsafe_output_then_server_death` in `evals/tests/common/mod.rs` |
| 3 | `evals/tests/fixture.rs` `focused_tools_complete_link_and_code_traps_without_guessing`, digest-forty | **A test-data collision, not a product bug.** Every rig draws a random seed (`inputs: Default::default()`). The generator puts `DIGEST_ITEM` at the seeded index `right` (out of 40) *in place of* `digest_decoy_title(right)`. The negative control is hard-coded to `digest_decoy_title(1)` ("… (part 1)"). When `right == 1` the control is not in the mail, so a correct selector answers `timed_out` with 0 candidates. This happens 1 negative rig in 40, with or without load. | the digest-forty negative control in `evals/tests/fixture.rs` |

### Why case 3 is not a 0001/0002/0005 defect

The same rig's first call is a `timeout_ms: 0` wait with no anchor on the same
inbox, and the test requires it to answer `ambiguous`. It passed: the panic
came at the second call's `"{task}: {result}"` message. An empty inbox answers
`timed_out`. So before the failing call the digest was stored, its link
occurrences were extracted at ingest, and the scan saw them
([0001](../../docs/decisions/0001-synchronous-insert-lossless.md),
[0005](../../docs/decisions/0005-extraction-at-ingest.md),
[0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md)).
`near_misses` filters only on to/from/subject/since and never on the anchor
([0111](../../docs/decisions/0111-near-miss-timeouts.md)). Listing the digest
with `missed: []` therefore proves the mail arrived. It does not prove the
anchor matched. The PR states this classification and the falsifier: a
zero-timeout anchored wait whose anchor is among the pre-call candidates and
which still answers `timed_out` would be a product bug. The test now asserts
that falsifier on every run.

### Why

- The verify seat is the loaded host. A test that passes only on an idle host
  is a scheduler lottery, and 0020 §5 forbids such a test: chaos only at the
  poles, no sleep as synchronisation.
- Retries, longer sleeps, an ignored test or a pinned seed would turn the race
  into a pass and lose the subject.
- No product behaviour changes. Case 3 has no product bug, so root `src/` is
  untouched.

### Authorities

The accepted decisions this design relies on, cited as authority and not
declared by it:
[0001](../../docs/decisions/0001-synchronous-insert-lossless.md),
[0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md),
[0005](../../docs/decisions/0005-extraction-at-ingest.md),
[0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md),
[0055](../../docs/decisions/0055-agent-eval-harness.md),
[0056](../../docs/decisions/0056-signing-policy.md),
[0076](../../docs/decisions/0076-sequential-eval-trials.md),
[0111](../../docs/decisions/0111-near-miss-timeouts.md).
No frozen guarantee, public contract or ruling changes, so this design
declares no decision.

### Rulings on the intake criteria

Intake `6dba8890` was written before the causes were known. Four of its
criteria named the wrong remedy or the wrong files. They are restated below.
None of the restatements loosens what a test proves.

- **Intake AC1** asked for a release-file hold on the survivor. **Rejected.**
  The survivor already ignores TERM (`trap "" TERM; … exec sleep 1000`), and
  `launch_probe` already holds the test on the `agent-pids` readiness
  handshake. The only thing that kills the survivor is the post-TERM SIGKILL
  escalation, and a release file cannot block SIGKILL. The "equivalent real
  signal" the issue allows is to remove the path through which the fault is
  lost. **Cost to the losing side:** the issue's literal "Done when" wording
  is not followed. The PR must say why.
- **Intake AC2** asked that the services-survived check stop racing the
  server-death path. **Rejected as a cause.** The evidence (a4f552d `:1620`)
  shows the classification was correct. `services_survived` and
  `SERVICE_EXIT_GRACE` stay unchanged.
- **Intake AC3** is resolved as a test-data collision (above).
- **Intake AC5's file list** cannot hold the real fixes. Case 1's defect lives
  in the test seam outside `test_probes`, and case 2's lives in
  `evals/tests/common/mod.rs`. The list is restated in AC5 below.

## Acceptance Criteria

- **AC1. Case 1 observes a real survivor with no lost fault.**
  `CLEANUP_OBSERVATION_TARGET` is gone: `grep -n CLEANUP_OBSERVATION_TARGET
  evals/` finds nothing.
  - With `EVAL_TEST_CLEANUP_OBSERVE_ERROR` set, every owned group that still
    has members after its TERM fails its own cleanup at once. The error is
    the byte-identical string `cannot inspect owned process group <pgid>:
    injected observation error`, which `safe_cleanup_category` maps to
    `cleanup_observation_failed`.
  - No harness path can SIGKILL the recorded survivor before the test's own
    `cleanup_owned`.
  - Every assertion in the test body is kept with its condition unchanged:
    surviving runtime, direct agent reaped, `probe.owned.iter().any(alive)`,
    retained admission, retained HOME, checked-teardown refusal, the expected
    stderr category, guard cleanup, admission release, HOME disposal and
    clean exit.
  - The `any(alive)` message may gain detail: the owned identities and their
    alive state.
  - Proof: on the old seam, a forced interleaving (the agent group's store is
    overwritten by another group's) fails at `any(alive)`. On the new seam
    the same perturbation cannot apply, and the test passes.
- **AC2. Case 2's witness owns its exit status.** The
  `unsafe_output_then_server_death` helper ignores SIGTERM before it walks
  `/proc`, so it reports its own exit (0 = found and killed `serve`, 64 = did
  not) in either reaping order.
  - The test still requires all of: outcome `HarnessError`,
    `agent_exit_status == Some(0)`, the stderr line
    `swarmail-evals: harness error: service_exited_during_trial`, and the
    marker byte count.
  - `services_survived`, `SERVICE_EXIT_GRACE`, `TERM_GRACE` and the
    `ServerDied` arm are unchanged.
  - No sleep is added or lengthened. No retry is added.
  - Proof: with a temporary delay after the kill (uncommitted), the old
    helper fails the exit-0 assertion and the new helper passes. If the
    perturbation fires the outcome assertion instead, case 2 returns to
    design as a runner ordering bug, and this helper patch is not shipped.
- **AC3. Case 3's control is chosen from the observed mail, and the
  falsifier is asserted.**
  - For digest-forty's negative rig, the test reads the `anchor_text` values
    in the first (`ambiguous`) answer's `candidates`.
  - It uses the part-1 control if that title is present. Otherwise it uses
    `^Async drop RFC enters final comment period \(part 2\)$`, which is
    `digest_decoy_title(14)`. Exactly one index is displaced, so one of the
    two titles is always present.
  - It asserts that the chosen title is among the pre-call candidates and is
    not `DIGEST_ITEM`. The message names the task, `negative` and the chosen
    title.
  - Both calls keep `timeout_ms: 0`. The `selected` and `WrongLink`
    assertions and every positive-rig assertion are unchanged.
  - The random-seed rigs stay random. One extra digest-forty negative rig is
    added, with an explicit seed that the test derives deterministically so
    that `right == 1`. It exercises the displaced-control branch on every run.
    It must go red, in the reported shape (`timed_out`, 0 candidates), with
    the old hard-coded control.
  - The PR records the classification "test-data collision, not a product
    bug" with the evidence chain above. No root `src/` file changes.
- **AC4. Determinism under load is proven, and the counts are in the PR.**
  - The candidate `swarmail` and the evals test binaries are prebuilt, with
    target dirs on `/home`. The binaries are then run directly while at least
    4 parallel copies of the evals suite run as load.
  - Each test (`--exact`) passes at least **100/100** for cases 1 and 2 and
    at least **200/200** for case 3.
  - The whole evals suite passes once under the same load.
  - The command, the counts and the wall time are in the PR text. Nothing
    from the load proof is committed.
- **AC5. Scope.** Changes are limited to:
  - `cfg(all(test, target_os = "linux"))` code in `evals/src/runner.rs` (the
    fault seam and `test_probes`);
  - `evals/tests/common/mod.rs`;
  - `evals/tests/fixture.rs`.

  No root `src/`, `tests/`, `.github/`, `scripts/`, `.brokkr/`, toolchain or
  decision-status change. The release build of both crates is unaffected.
- **AC6. Gates.** Every gate in
  [house-rules § Gates](../../docs/house-rules.md#gates--part-of-every-change)
  is green with `--locked`. This includes:
  - the evals fmt, clippy and tests, with `SWARMAIL_BIN` set to the
    candidate's binary from Cargo's artifact record of
    `cargo build --locked --bin swarmail`;
  - `brokkr compile`;
  - the coverage gate (100 % exact, floor unchanged) and its selftest;
  - `publish-guard-test`, `seal-branch-test` and
    `cargo deny check licenses`.
- **AC7. Dependencies.** `Cargo.lock`, `evals/Cargo.lock` and `deny.toml`
  are unchanged. No new dependency.
- **AC8. Delivery.**
  - The implementation lands as unsigned commits on
    `repair/issue302-eval-flakes`, on top of this design commit.
  - No push, merge, publish or decision acceptance. No paid trials.
