# Implementation plan — eval provenance port

Date: 2026-10-25. Base aec4b46, intake 1823f55. Implements [spec.md](spec.md)
and proposed [0063](../../docs/decisions/0063-eval-provenance.md).

## Technical context

One unpublished Rust 2024 crate in the standalone evals workspace, with Tokio
subprocesses, Unix process groups and real loopback SMTP/MCP/HTTP. Reuse shipped
ring =0.17.14 for SHA-256: it is already root-locked and reuses the vetted
permissive dependency closure. Only evals/Cargo.toml and its offline lock change.
No RNG package, root dependency, allowlist or gate change.

No plan-template.md exists in this checkout. This plan supplies the requested
technical context, constitution check, project structure, research, data model,
contracts, quickstart and risks within the four authorized design artifacts.

## Constitution check

Preserve 0001 accept == stored, 0002 subscribe before scan, 0003 oldest-only
pruning, 0004 real protocol testing and 0005 ingest extraction. Root runtime
owners have no implementation work here. Fixture UTC never replaces monotonic
expiry/deadlines or TLS time. Tests use fresh inboxes, deterministic seams and
chaos only at probability 0/100; no sleeps as synchronization or weakened bounds.

Accepted 0071 controls business-v2 semantics. Restore reserved proposed 0063
with one proposed index row in T1; preserve 0055's accepted historical body and
0073/0075's proposed status. Any old 0055 compatibility addendum must be clearly
labeled superseded by 0071 beside a dated integration explanation. No new
acceptance, v3 business schema, legacy scorer or migration API. Keep sequential
trials and fresh HOME under the operator's #199 ruling.

Keep current candidate bundles/routing, protected review/security hold, 0050
pinned proof chain, exact coverage denominator and 0056 unsigned commits. The
engine chooses transitions; no push/merge/tag/publication/closure.

## Research and resolved questions

All 53 evidence hashes and pinned engine bytes match. Six patches cover the
feature, boundary repairs, acknowledged cleanup and source-observation signal
latch. Do not apply their obsolete business versions. Listed textual conflicts
are evals/README.md, main.rs and fixture/scoring tests; semantic conflicts also
affect runner/state/tasks/report/proof. Integrate into current owners.

Both panels favor those owners. Require strict provenance-bearing v2 input and
historical binaries for records lacking it; missing facts cannot masquerade as
a valid new producer. Typed bounds and hashing serve truthful evidence, not
hostile-host attestation. The v2 scorer remains the sole business authority.
Regenerate actual catalogue/definition vectors, not just old fixture labels.

Use shipped semantic SHA-256 choices, public-argv framing and bounded envelopes
as spec R1–R4. Fixed seed plus fixture time reproduces public difficulty; separate
entropy preserves authority. Unknown claimants become null at recording; the
writer guard cannot erase known Decoy evidence or change historical v2 scoring.
A retained child/parent conflict is incomplete evidence and exit 2, without a
handshake or overwrite. These are explicit rulings, not deferred research.

## Project structure and files

| Files | Work and owner |
|---|---|
| evals/build.rs, evals/git_context.rs, evals/src/identity.rs | Build-time harness facts, measured executable/owned source identity, private fingerprints, cleared owned helpers and launch rechecks. |
| evals/src/provenance.rs, evals/src/results.rs, evals/src/lib.rs | Typed availability, exact choice/argv derivation and strict new v2 reader; register modules, omit v1 adapters. |
| evals/src/main.rs | Extend current CLI/matrix/check; bounded public context before native argv, preassigned IDs/seeds, existing signal/commit owners and source-helper cleanup latch. |
| evals/src/runner.rs | Identity/public inputs, one finalizer after cleanup, current Groups/RuntimeDirs, private publication and acknowledged controls. |
| evals/src/fixture.rs, evals/src/mail.rs | Public seed/time choices and dates, separate CSPRNG secrets, canonical claims with unchanged routes. |
| evals/src/state.rs | Writer-only claim guard; current v2 snapshots with no reader/scorer membership change. |
| evals/src/tasks.rs, evals/src/report.rs | Current catalogue/scorer identity and shared strict reader; preserve trap precedence, v1/mixed refusal and arithmetic. |
| evals/src/artifacts.rs | Share current private no-clobber primitive; typed plan/metadata/index, single finalization, bounded offline replay. |
| evals/tests/{identity,provenance}.rs, evals/tests/data/ | Shipped independent vectors/boundary controls including patches 04/06; actual v2 catalogue/result and historical-refusal fixtures. |
| evals/tests/{fixture,scoring,proof,runner,report,boundary}.rs, evals/tests/common/mod.rs | Preserve controls/cleanup and add composed replay, privacy canaries and exact v2 expectations. |
| evals/Cargo.toml, evals/Cargo.lock, evals/README.md | Eval-only root-version dependency and current CLI/contract documentation. |
| docs/decisions/README.md, docs/decisions/0055-agent-eval-harness.md | T1 index and dated historical integration; no accepted-body/status rewrite. |
| specs/eval-provenance/{spec,plan,tasks}.md, docs/decisions/0063-eval-provenance.md | Design and later truthful progress/test mapping. |

mcp.rs/oracle.rs retain no_proxy and real protocols. Root src/, manifests/lock,
deny.toml and bundles/ are outside implementation scope.

## Data model and contracts

Spec R1–R7 is normative. TrialResult remains business schema 2 with original
execution fields and required typed provenance: fixture, matrix coordinates,
task/catalogue/scorer, harness/target/agent identity and effective limits.
Availability separates measured, owned-build, declared and unavailable facts;
private source fingerprints/paths and unknown claims never enter rows.

Catalogue/snapshot/result versions stay 2; scorer identity is business-score-v2.
Metadata/plan/index/replay and public projection retain independent version 1.
Metadata binds exact result/state/diagnostics bytes; index binds plan and ordered
metadata. Original execution outcome remains separate from business replay.

Symmetric bounds: 4096 retained cells; 8 MiB plan/index; 64 KiB metadata/result/
diagnostics (row newline included); 16 MiB state; 1 MiB retained agent input;
1024-byte public strings; 64 JSON levels. Child capture stays 64 KiB and drains
overflow. Validate every matrix cell before output, including semantic checks
after hashing. Require fixed relative paths, no symlinks and regular opened files.

Parent owns plan/index and committed rows; worker owns trial retention. Bounded
public CLI context precedes native argv after `--`, with no environment widening
or caller-authored attestation. Worker rechecks bytes. Metadata and matrix index
are last. Parent fallback is post-reap and only without a child leaf. Conflicting
immutable evidence prevents index completion, never authorizes replacement.

## Implementation approach

1. Repair index/integration documentation, inventory baseline tests and recover
   all six patches' final behavior without importing old business versions.
2. Port pure strict result/provenance types, independent v2 vectors and exact
   seed/argv framing. Explicitly refuse pre-provenance v2 and v1/mixed results.
3. Port identity into existing cleared helper/Cargo ownership. Subscribe before
   spawn; source errors still clean up and latch signals before later admission.
4. Thread seed/time through fixture/mail and canonicalize shared claims. Retain
   real token ownership, route facts, all provisioned roles and known decoys.
5. Share publication primitive; finalize once after cleanup with writer guard
   before payload and metadata last. Requested retention failure is infra.
6. Extend current sequential matrix with complete accounting/bounded output.
   Check retained equality after cancellation arbitration; stop requests use
   existing cleanup and never reset its budgets.
7. Add offline replay through tasks::score. Validate whole matrix before output;
   any second reading pass must revalidate bytes and bindings.
8. Compose #194/#196/#198 and shipped tests, finish docs and execute every gate
   on committed work. Record exact counterparts for narrow test relocations.

## Acceptance-to-test map

New targets here are required work, not claimed to exist at this base.

| Criteria | Discriminating proof |
|---|---|
| AC1 | identity/provenance: build revision vs runtime cwd, worktree/packed refs, dirty/archive/rebuild, dirty-to-dirty edits, aliases/mismatches/replacement and every error-row availability. |
| AC2, AC9, AC12 | runner/report/provenance: native argv, independent private/public digest vectors, whole env maps, HOME/PWD/config/proxy canaries at all sinks; invalid declarations fail before launch. |
| AC3 | provenance/fixture: independent Unicode/duplicate-coordinate vectors; equal fresh-process projections, selected changed seeds, controlled private entropy; real Date/DTSTAMP/MIME/invite/OTP. |
| AC4 | provenance/report/main: real two-task × oracle/null × two-repetition retained matrix, unique IDs/seeds/order, shared roots, malformed/multiple/oversized/wrong-ID/exit/crashed workers and stdout failure. |
| AC5 | artifacts/runner: pre-payload modes, both umasks, collisions/partial writes/marker/index failures, unchanged unowned bytes/modes and cleanup despite failure. |
| AC6, AC12 | provenance/artifacts/scoring: binary-free replay, altered/swapped bytes and recomputed semantic mismatches, wrong scorer/catalogue, every at/over bound/path/key refusal, invalid last cell produces zero rows; original infra/budget preserved, absent/ineligible state unscorable. |
| AC7 | scoring/proof/report/fixture plus actual retained CLI: all v2 traps beat recovered goals, both orders, known-Decoy null flow, expiry-only/reset/non-target recovery, v1 snapshot and v1/mixed report refusals. |
| AC8 | fixture/scoring/provenance: live OTP and nonnumeric claim canaries through every route family; known decoys intact, admin/writer guard failure and historical v2 unknown-claim compatibility. |
| AC10, AC13 | main/runner/provenance: both signals, Pending/release/bypass, source failure during owned cleanup/no later Cargo, acknowledgement, immutable retained-child conflict, index failure and timeout fence. |
| AC11 | store_contract_docs, eval boundary and all gates: exact-one proposed index/status, dated documentation, root-version dependency closure, current routing and fresh committed-tree receipts. |

## Baseline test preservation inventory

All tests listed below exist at this design base. Their destination is the same
file/name unless a narrow helper relocation is recorded with its exact new
counterpart during implementation. Preserve assertion strength, helper cleanup,
negative controls and full-suite execution. Only v2/API/provenance expectations
may change. Historical v1 acceptance goldens are replaced with explicit refusal controls,
as required by accepted 0071. Preserve report arithmetic using strict v2 rows. This inventory intentionally includes
pre-#194 controls so broad green cannot hide a dropped regression.

### evals/src/main.rs

- `matrix_probe_entrypoint` → same owner/name.
- `matrix_completion_fence_is_observed_and_its_negative_control_exits_early` → same owner/name.
- `matrix_probe_readiness_unwind_releases_and_reaps_owned_processes` → same owner/name.
- `matrix_owner_test_controls_cover_setup_admission_commit_and_final_row` → same owner/name.
- `pending_matrix_decisions_observe_queued_signals_and_retain_spawned_children` → same owner/name.
- `mixed_signals_during_matrix_cleanup_do_not_bypass_the_retained_wait` → same owner/name.
- `matrix_spawn_failure_keeps_all_rows_and_unknown_agent_statuses` → same owner/name.
- `matrix_stdout_backpressure_is_bounded_and_still_observes_signals` → same owner/name.
- `matrix_post_exit_inherited_pipe_is_bounded_and_holder_is_reaped_by_the_test` → same owner/name.
- `matching_trial_row_requires_one_bounded_valid_matching_row_and_exit_pair` → same owner/name.
- `capture_stdout_drains_beyond_its_retention_limit` → same owner/name.
- `matrix_worker_requires_one_matching_schema_row_and_exit` → same owner/name.
- `matrix_row_reader_caps_storage_and_drains_overflow` → same owner/name.

### evals/src/runner.rs

- `runner_probe_entrypoint` → same owner/name.
- `runner_guard_finish_failure_entrypoint` → same owner/name.
- `runner_guard_finish_rejection_fails_in_isolated_process_and_rescues` → same owner/name.
- `runner_cleanup_await_pending_and_bypass_controls_cover_signals_and_faults` → same owner/name.
- `mixed_signals_after_cleanup_budget_creation_keep_one_owner_and_fixed_budgets` → same owner/name.
- `cleanup_observation_error_and_forced_expiry_fail_with_live_owned_processes` → same owner/name.
- `server_death_during_the_trial_is_a_harness_error` → same owner/name.
- `a_runner_panic_cleans_up_and_still_emits_one_row` → same owner/name.
- `trial_runtime_owner_survives_aborted_body` → same owner/name.
- `runner_probe_failure_teardown_handles_owner_and_leader_exit` → same owner/name.
- `trial_pre_handler_control_runs_before_owned_children_exist` → same owner/name.
- `artifact_leaf_and_temporary_are_private_before_payload` → same owner/name.
- `artifact_leaf_refuses_existing_directories_files_and_symlinks` → same owner/name.
- `artifact_temp_name_collision_preserves_the_unowned_file` → same owner/name.
- `artifact_final_name_appears_only_after_complete_private_payload` → same owner/name.
- `artifact_write_failure_cleans_only_its_private_partial_file` → same owner/name.
- `artifact_publication_refuses_existing_targets_without_clobbering` → same owner/name.
- `admin_transport_read_and_parse_failures_are_value_free` → same owner/name.
- `cargo_runtime_owner_survives_cancelled_preparation` → same owner/name.
- `admin_error_categories_discard_injected_response_text` → same owner/name.
- `executable_resolution_uses_only_the_supplied_original_context` → same owner/name.
- `default_toolchain_homes_resolve_relative_home_at_the_caller_boundary` → same owner/name.
- `cargo_record_reader_bounds_lines_and_keeps_draining_after_overflow` → same owner/name.

### evals/tests/report.rs

- `helper_agent` → same owner/name.
- `the_matrix_runs_the_whole_product_and_aggregates_exits` → same owner/name.
- `matrix_workers_are_closed_and_child_output_is_not_published` → same owner/name.
- `matrix_preserves_non_utf8_path_discovery_through_the_worker_cli` → same owner/name.
- `the_matrix_keeps_counting_after_a_harness_error_and_exits_two` → same owner/name.
- `a_missing_swarmail_binary_still_emits_the_whole_product` → same owner/name.
- `the_matrix_refuses_bad_inputs_before_running_anything` → same owner/name.
- `the_report_separates_harness_errors_from_the_success_rate` → same owner/name.
- `empty_and_all_error_inputs_give_null_estimates_and_bad_rows_refuse` → same owner/name.
- `matrix_sigterm_and_sigint_wait_for_the_active_trials_owned_cleanup` → same owner/name.
- `privacy_cancellation_preserves_owned_runtime_and_suffix_accounting` → same owner/name.
- `a_broken_matrix_stdout_stops_after_the_owned_trial_finishes` → same owner/name.
- `normal_matrix_binary_ignores_test_control_environment_variables` → same owner/name.
- `matrix_signal_cancels_and_reaps_the_fallback_cargo_group` → same owner/name.

### evals/tests/runner.rs

- `artifacts_are_private_under_permissive_and_restrictive_umasks` → same owner/name.
- `helper_agent` → same owner/name.
- `an_external_agent_gets_literal_argv_and_identical_prompt_and_env` → same owner/name.
- `relative_and_bare_agent_programs_resolve_at_the_caller_boundary` → same owner/name.
- `fixture_gets_only_its_private_runtime_map_and_generated_admin_input` → same owner/name.
- `swarmail_gets_only_its_fixed_runtime_map` → same owner/name.
- `readiness_discards_an_oversized_preannouncement_flood` → same owner/name.
- `readiness_continues_draining_after_the_first_valid_announcement` → same owner/name.
- `automatic_build_uses_a_cleared_named_preparation_map` → same owner/name.
- `automatic_build_accepts_explicit_homes_without_caller_home` → same owner/name.
- `oversized_cargo_record_is_drained_and_fails_without_echo` → same owner/name.
- `agent_argv_stays_literal_and_wrapper_config_is_explicit` → same owner/name.
- `two_trials_use_fresh_ports_and_identities` → same owner/name.
- `a_spawn_failure_is_a_harness_error_with_null_statuses` → same owner/name.
- `direct_trial_preserves_non_utf8_later_arguments` → same owner/name.
- `missing_program_and_bad_clap_values_do_not_echo_input_text` → same owner/name.
- `artifact_write_failures_use_safe_categories` → same owner/name.
- `a_swarmail_that_never_comes_up_is_a_harness_error` → same owner/name.
- `invalid_inputs_are_one_harness_error_row_with_the_supplied_labels` → same owner/name.
- `a_deadline_is_budget_and_reaps_the_agents_descendants` → same owner/name.
- `an_agent_that_succeeds_then_crashes_still_passes_with_its_real_status` → same owner/name.
- `child_output_canaries_are_drained_without_publication` → same owner/name.
- `child_output_server_death_remains_a_harness_error_on_linux` → same owner/name.
- `a_noisy_agent_is_drained_and_does_not_hang_the_trial` → same owner/name.
- `normal_trial_binary_ignores_the_former_runner_fault_control` → same owner/name.
- `sigterm_interrupts_cleans_up_and_emits_one_row` → same owner/name.
- `a_retained_snapshot_reproduces_the_verdict_and_bad_state_refuses` → same owner/name.
- `a_delivery_parked_after_provisioning_outranks_the_business_check` → same owner/name.
- `a_delivery_parked_after_provisioning_outranks_the_deadline` → same owner/name.
- `the_catalogue_is_served_as_schema_version_two` → same owner/name.
- `cleanup_escalates_after_a_leader_exits_and_a_descendant_ignores_term` → same owner/name.
- `loopback_clients_ignore_caller_proxy_settings` → same owner/name.

Also retain all existing fixture/scoring/proof/boundary tests and common helper
ownership. Root tests/delivery_recipes.rs remains unchanged and runs in the
house suite. Do not weaken mcp.rs/oracle.rs no_proxy or the helpers' cleanup.

### Current #198 and final shipped repairs

Keep these current names and assertion strength in addition to the inventory:

- report::v2_report_refuses_v1_and_mixed_rows_before_aggregation.
- scoring::every_named_trap_precedes_goals_in_both_orders_and_unrelated_refusals.
- scoring::expiry_and_nontarget_recovery_remain_recoverable_and_isolation_keeps_its_fallback.
- scoring::invalid_or_failed_trap_bearing_snapshots_still_refuse_before_scoring.
- scoring::the_expected_replay_refusal_is_evidence_in_reset_reuse_only.
- fixture::concurrent_trap_and_goal_linearize_before_freeze_and_stay_frozen.
- proof::completed_naive_controls_recover_the_goal_but_lose_the_trial.

Completed naive controls must still exit zero, receive healthy real deliveries
and recover true goals while failing with their exact traps. Retain forty-link/
39-decoy and both-order controls; add retained actual CLI replay without changing
scoring. Catalogue goals disclose traps. Preserve all other current v2 tests.

Patch 04's identity/provenance boundary controls, patch 05's cleanup ownership
acknowledgement and patch 06's source-observation cleanup/signal latch are part
of the port. Keep both-signal causal hold/release/bypass proofs. No assertion
removal, looser bounds, mock protocol or sleep-as-sync adaptation.

## Quickstart

After implementation, build with cargo build --manifest-path evals/Cargo.toml
--offline --locked. A public agents.json for this example contains
`[{"label":"oracle","argv":[]},{"label":"null","argv":[]}]`.
Use a fresh root and substitute generated IDs for the placeholders below.

```bash
EVAL_EXE=evals/target/debug/swarmail-evals
"$EVAL_EXE" matrix --tasks signup,otp --agents agents.json --trials 2 \
  --seed 0000000000000000000000000000000000000000000000000000000000000001 \
  --fixture-time 2026-10-24T00:00:00Z --artifacts-dir /tmp/eval-evidence
"$EVAL_EXE" check --matrix /tmp/eval-evidence/<matrix-id>
"$EVAL_EXE" check --artifacts-dir /tmp/eval-evidence/<matrix-id>/trials/<trial-id>
"$EVAL_EXE" check --task otp --state /tmp/eval-evidence/<matrix-id>/trials/<trial-id>/state.json
```

Mixed oracle/null execution and replay normally exit 1. Retain seed AND fixture
time for public-layout reproduction. Opaque target source can be unavailable.
Replay does not rerun the model or certify old files safe to publish. Historical
v1 and pre-provenance v2 results need their matching historical reader.

## Verification and ownership

Run offline without installing anything; record command, exit and private log
path. The charter remains the sole house inventory; this execution list adds
intake supplements and the independent eval suite. Engine bytes must match and
bundle policy-sweep warnings refuse. Exact coverage means zero missed lines,
branches and functions plus passing selftest. Old receipts verify no new tree.

```bash
export CARGO_NET_OFFLINE=true
export PATH="$PWD/.forge/bin:$PATH"
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum -c -
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
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --workspace --locked
```

Run the pinned verify-seat's fail-closed native-tls proof too; arbitrary Cargo errors do not prove absence:

```bash
if out=$(cargo tree -i native-tls 2>&1); then
    exit 1
elif printf '%s' "$out" | rg -q 'did not match any packages'; then
    :
else
    exit 1
fi
```

After porting, run focused locked eval identity/provenance/fixture/scoring/proof/
runner/report/boundary targets and the two named cleanup fence controls in the
inventory. Restore temporary mutations before full gates. Evals remains outside
root coverage; its standalone suite is mandatory. No new exclusion is authorized.

CI owns macOS eval evidence through #201, privileged port-80, chart/history/
selftests, Docker smoke, network RustSec and tag-only rate proof. The operator
owns live settings and sealing; #221 owns namespaces. These are ownership
records, not new local acceptance criteria or execution claims. Candidate
bundles alone select review; no historical routing is imported.

The chief runs requested gates and records actual exits. The exact four-file
limit leaves the proposed decision's index row for T1: inventory/coverage failure
until then is not waived. Structural design validation and drafted are distinct
from full-gate/implementation success. Implementation and verify still owe fresh
committed-candidate proofs after T1 and the port.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Old patches undo traps | Keep current score/trap_class and #198 controls; regenerate actual v2 vectors and use business-score-v2 throughout. |
| Missing provenance silently passes | Closed required new v2 reader; pre-provenance rows refuse without fabricated fields; historical reader remains necessary. |
| Claim privacy disables sibling trap | Keep all provisioned roles and Decoy null-flow evidence; writer guard stays out of readers/scorer. |
| Helper failure loses cancellation | Owned cleanup on every source result; both-signal acknowledged/release/bypass tests prevent later Cargo admission. |
| Retained child disagrees with row | Preserve bytes, exit 2 and omit index; accept incomplete evidence instead of a handshake. |
| Source facts overclaim | Embedded revision, private before/after source fingerprint, measured/rechecked bytes and finite unavailable reasons; no adversarial TOCTOU claim. |
| Private argv/environment leaks | Closed maps, native execution separate from opted-in digest, fixed diagnostics and independent canaries. |
| Seed derives authority | Semantic public keys and separate CSPRNG; controlled entropy tests, fresh IDs/UID, real deadlines; no identical-wire/outcome promise. |
| Write/replay boundaries regress | Shared pre-payload modes/no-clobber writer, bounded original-byte parser and full-matrix validation; bad later-cell controls. |
| Gate debt hidden by draft | Explicit T1 index repair, actual gate receipts and full implementation rerun; old green cannot cover this base. |
| Scope/routing grows | No root/runtime/gate/engine changes; six patches supply behavior, current bundles supply routing. |
