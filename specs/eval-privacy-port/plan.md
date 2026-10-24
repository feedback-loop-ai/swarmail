# Implementation plan — eval privacy port

## Technical context

Standalone unpublished Rust evals crate, Tokio Unix subprocesses, reqwest,
serde and already-locked tempfile 3.27.0. Linux and macOS are supported Unix
targets. Root manifests/locks and standalone coverage boundary remain unchanged
under accepted 0055. Continue from unreviewed handoff 529d916 and intake a87fb96,
preserving the already-ported #194 runner/main architecture. Linux proof is due
here; macOS eval proof belongs to #201 under spec AC13, not to this seat.

The usual .specify/templates/plan-template.md is absent in this workspace.
This plan supplies the required technical context, constitution check, project
structure, research, data model, contracts, quickstart and risks directly.
No replacement template or extra design package is needed.

## Constitution check

Preserve synchronous accept/store, subscribe-before-check, oldest-only pruning,
AUTH inbox routing and ingest extraction (0001–0005). No root-server edits.
Follow 0020 for test-only seams and deterministic real-process/wire proofs.
Keep exact root coverage, dependency allowlist and 0050 pinned proof chain.
0055 permits this compatible strengthening while 0075 is proposed; any actual
accepted-rule contradiction waits for the operator. Neither 0073 nor 0075 is
accepted by this work. Commit unsigned, never push (0056).

The chief commit contains exactly spec.md, plan.md, tasks.md in this directory
and docs/decisions/0075-eval-privacy.md. It deliberately excludes the index and
earlier owning specs to obey the exact design-commit boundary. The proposed 0075
index row already exists: T1 verifies it rather than duplicating it. T1 also
reconciles the dependent current platform sentences in evals/README.md and
specs/agent-eval-harness/{spec,plan}.md, outside this four-file design commit;
those sentences are superseded by spec AC13 and are not an additional blocker.
The intake framing stays untouched; position files stay ignored. Structural
design validation is distinct from implementation, hermetic verify and review.

## Research and settled choices

All eleven packet checksums and .forge/bin/brokkr digest matched:
ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f.
Read the packet README and operator-rulings, both current positions, framing.md,
decision index and accepted 0055/0058 plus proposed 0073/0075. The packet's
relative SHA256SUMS paths must be checked from its directory. The earlier root
invocation's missing-file results are not packet corruption; the corrected
packet-directory invocation passes every entry.

The current source already has RuntimeDirs::apply, retained runtime owners,
private no-clobber publication, all three no_proxy builders and the integrated
ownership/cancellation tests named below. The previous implementation gaps are
historical rationale, not instructions to recreate those changes. Historical
127-test and eleven-mutation summaries require candidate/assertion binding;
neither aggregate counts nor digest equality proves fresh execution.

Adopt the current panel synthesis in spec.md: preserve existing owners and
code unless a criterion fails, keep complete Linux evidence, and apply the
operator's #201 ownership correction consistently. No new parser framework,
supervisor, dependency, raw-output option or proof schema is needed. Keep all
existing bounds and explicit wrapper authentication.

## Project structure and touched files

| Owner | Existing responsibility; permitted repair only after a demonstrated failure |
| --- | --- |
| evals/src/runner.rs | Native caller lookup and closed role/preparation maps; private runtime owners; typed drains; safe errors/Admin; metadata; private publication; existing inline seams and tests |
| evals/src/main.rs | Native CLI/worker handoff, safe parsing/panic/error publication; closed worker/build commands; bounded row validation while retaining signal/admission/wait/output owners |
| evals/src/mcp.rs, evals/src/oracle.rs | Direct no_proxy HTTP; audit error propagation at publishing callers |
| evals/tests/common/mod.rs | Explicit test-owned wrappers/config and byte-aware probes/collectors |
| evals/tests/runner.rs, report.rs, proof.rs and owning inline tests | Verify every privacy test and preserve every cancellation proof, exact environments and discriminating controls |
| evals/tests/fixture.rs, boundary.rs, scoring.rs; other existing inline tests | Retain all regression tests and real-protocol behavior; edit only if helper signature adaptation requires it |
| evals/README.md; specs/agent-eval-harness/spec.md and plan.md | T1 aligns current platform ownership with AC13 and audits maps/output contract; preserve dated clarification history |
| docs/decisions/0075-eval-privacy.md; docs/decisions/README.md | Correct proposed proof ownership; validate the existing exact-one proposed index row, edit index only if defective; no accepted-decision rewrite |
| specs/eval-privacy-port/spec.md, plan.md, tasks.md | Owning contract, test mapping and honest task/receipt updates |

No root production/manifests/locks, bundles, shims, policy or dependencies change.
Do not recreate continuation specs from patches 0004–0008 or replay the already
upstream Sol/Luna assertion repair. Any necessary additional file must have an
explicit scoped reason; it cannot relax these boundaries.

## Data model and contracts

CallerEnv captures only native discovery inputs (original cwd/PATH, selected
binary/tool/cache locators). RuntimeDirs owns fresh private per-role paths and
constructs the exact common map; role additions remain explicit. TrialRequest's
program/argv representation and worker command handoff use OsString/PathBuf
without lossy round trips. JSON agent specs stay textual at ingestion.

Readiness stores one typed loopback announcement and completion state; Cargo
stores a required executable path plus overflow/error/EOF state; matrix retains
its existing bounded bytes/status state. None becomes a raw diagnostic queue.
Safe diagnostic categories are finite internal facts, not a public schema
migration. Keep TrialResult schema 1, report formulas and unknown-status semantics.

RuntimeDirs must be owned outside run_body or retained alongside registered group
resources. Build fallible command environments before spawn; register ownership
synchronously once spawned without a cancellable gap. Release directories only
after their established wait/drain/group-cleanup fences. Matrix runtime owners
outlive worker wait and settled output; Cargo owners outlive cleanup_cargo.

Private publication uses a fresh exclusive directory, exact pre-payload modes,
exclusive private temporary, complete write and tempfile persist_noclobber.
Normalize modes through owned handles; preserve existing ancestors and unrelated
entries. Do not convert complete-file atomicity into a durability claim.

## Implementation sequence

1. Reconcile the #201 ruling in this design and proposed 0075; validate the
   existing index row. T1 then aligns the three dependent current documentation
   sentences outside the chief commit. Preserve dated historical receipts and
   explain the reassigned former blocker without turning it into a pass.
2. Bind every AC and named baseline/packet test below to the committed handoff
   and actual assertions. Existing unchecked tasks mean proof is due, not that
   the implementation is absent. Record a concrete failing criterion/test before
   any code repair; otherwise keep source and tests unchanged.
3. Verify complete runtime/preparation maps, native CLI/worker handoff, generated
   PWD, lazy toolchain-home defaults, once-per-matrix offline/locked custom-target
   build and normal dev/release fault-control inertness.
4. Verify typed readiness and Cargo bounds, discard drains, safe CLI/Admin/panic/
   cleanup publication, all-client direct HTTP, null metadata and worker variants.
   Preserve actual waits/statuses, EOF bounds and cancellation under pressure.
5. Verify exclusive private artifacts before payload, adverse umasks, no-clobber
   collisions, partial-write failures and ownership-specific cleanup on Linux.
6. Execute all retained #194 and integrated privacy tests: both signals, admission/
   commitment races, actual-Pending holds/releases and bypass controls, Cargo
   ownership, private-directory lifetimes, exact rows and report denominators.
7. Audit the eleven supplied mutation receipts against source/test regions and
   candidate lineage using the existing map below. Reproduce missing, stale or
   insufficient intended-assertion failures and restored-green controls; retain
   valid evidence. Restore all mutations before final gates.
8. Repair only demonstrated defects at their current owners and rerun affected
   proofs plus all final gates. The independent hermetic verify and five careful
   review results belong to their phases; this chief supplies no review verdict.

## Requirements-to-proof map

| Criteria | Named proof families below |
| --- | --- |
| AC1–2, AC16, AC18 | Complete environment, native argv, lookup, wrapper auth, explicit-home and matrix worker tests |
| AC3, AC5, AC15 | Automatic/custom-target preparation, Cargo parser/overflow, readiness flood tests |
| AC4, AC12, AC17, AC20 | Child canaries, safe CLI/Admin errors, proxy success, matching-row variants |
| AC6–7, AC24–25 | All #194 inline and wire lifecycle/accounting tests; full proof/fixture/scoring suites |
| AC8, AC19 | Independent exposure, Cargo-overflow and native-worker red/restored-green controls |
| AC9 | store_contract_docs inventory/status tests plus live-link/map review |
| AC10–14 | Pinned final-candidate Linux gates/receipts, dated #201 platform-ownership citation and five careful reviews |
| AC21–23 | Pre-payload, failure/collision helper tests and real adverse-umask failed trial |
| AC26 | Complete named baseline/packet mapping below, reconciled with final test listing |

The #194 AC1–AC12 obligations remain: ordinary signals → AC24/25; completion
fences → AC24; accounting → AC25; admission/repeat signals → AC24/25;
cleanup failures → AC24; bootstrap/spawn → AC6/24; pressure/EOF → AC5/24;
isolation/teardown → AC7/24; earned evidence → AC10/11/14; already-settled
routing assertion → unchanged full root tests; reproducible teardown → AC24.
Do not copy historical checked boxes as proof of this port.

## Named retained/adapted test inventory

The following inventory is generated from all baseline eval test declarations
and every eval file version referenced by the eight verified patches. B means
baseline #194 candidate; P means packet #196 (including retained earlier tests).
Each arrow names the required destination. Identity arrows preserve the test
and its discriminating assertions; helpers may change as described above.
Earlier server-death/panic integration cases move to test-only inline owners,
where #194's compiled-only controls can exercise them without production flags.
The two older matrix parser tests remain distinct adapted tests over current
64 KiB owners; they do not replace #194's tests. This is the preserved required mapping; audit it against 529d916 and the final
candidate rather than inferring either missing code or executed proof from it.

| Origin | Required source test | Planned retained/adapted target |
| --- | --- | --- |
| B | evals/src/fixture.rs::a_superseded_code_is_stale_even_before_expiry | evals/src/fixture.rs::a_superseded_code_is_stale_even_before_expiry |
| B | evals/src/fixture.rs::an_expired_code_is_stale_and_a_fresh_one_verifies | evals/src/fixture.rs::an_expired_code_is_stale_and_a_fresh_one_verifies |
| B | evals/src/fixture.rs::invite_times_compare_in_basic_utc | evals/src/fixture.rs::invite_times_compare_in_basic_utc |
| B | evals/src/mail.rs::a_unicode_subject_becomes_bounded_encoded_words | evals/src/mail.rs::a_unicode_subject_becomes_bounded_encoded_words |
| B | evals/src/mail.rs::dot_stuffing_doubles_leading_dots_and_normalizes_line_ends | evals/src/mail.rs::dot_stuffing_doubles_leading_dots_and_normalizes_line_ends |
| B | evals/src/mail.rs::rendered_lines_stay_under_the_smtp_limit_and_carry_the_attachment | evals/src/mail.rs::rendered_lines_stay_under_the_smtp_limit_and_carry_the_attachment |
| B | evals/src/main.rs::capture_stdout_drains_beyond_its_retention_limit | evals/src/main.rs::capture_stdout_drains_beyond_its_retention_limit |
| B | evals/src/main.rs::matching_trial_row_requires_one_bounded_valid_matching_row_and_exit_pair | evals/src/main.rs::matching_trial_row_requires_one_bounded_valid_matching_row_and_exit_pair |
| B | evals/src/main.rs::matrix_completion_fence_is_observed_and_its_negative_control_exits_early | evals/src/main.rs::matrix_completion_fence_is_observed_and_its_negative_control_exits_early |
| B | evals/src/main.rs::matrix_owner_test_controls_cover_setup_admission_commit_and_final_row | evals/src/main.rs::matrix_owner_test_controls_cover_setup_admission_commit_and_final_row |
| B | evals/src/main.rs::matrix_post_exit_inherited_pipe_is_bounded_and_holder_is_reaped_by_the_test | evals/src/main.rs::matrix_post_exit_inherited_pipe_is_bounded_and_holder_is_reaped_by_the_test |
| B | evals/src/main.rs::matrix_probe_entrypoint | evals/src/main.rs::matrix_probe_entrypoint |
| B | evals/src/main.rs::matrix_probe_readiness_unwind_releases_and_reaps_owned_processes | evals/src/main.rs::matrix_probe_readiness_unwind_releases_and_reaps_owned_processes |
| P | evals/src/main.rs::matrix_row_reader_caps_storage_and_drains_overflow | evals/src/main.rs::matrix_row_reader_caps_storage_and_drains_overflow |
| B | evals/src/main.rs::matrix_spawn_failure_keeps_all_rows_and_unknown_agent_statuses | evals/src/main.rs::matrix_spawn_failure_keeps_all_rows_and_unknown_agent_statuses |
| B | evals/src/main.rs::matrix_stdout_backpressure_is_bounded_and_still_observes_signals | evals/src/main.rs::matrix_stdout_backpressure_is_bounded_and_still_observes_signals |
| P | evals/src/main.rs::matrix_worker_requires_one_matching_schema_row_and_exit | evals/src/main.rs::matrix_worker_requires_one_matching_schema_row_and_exit |
| B | evals/src/main.rs::mixed_signals_during_matrix_cleanup_do_not_bypass_the_retained_wait | evals/src/main.rs::mixed_signals_during_matrix_cleanup_do_not_bypass_the_retained_wait |
| B | evals/src/main.rs::pending_matrix_decisions_observe_queued_signals_and_retain_spawned_children | evals/src/main.rs::pending_matrix_decisions_observe_queued_signals_and_retain_spawned_children |
| BP | evals/src/oracle.rs::entity_decoding_and_anchor_selection | evals/src/oracle.rs::entity_decoding_and_anchor_selection |
| BP | evals/src/oracle.rs::labelled_urls_and_ics_properties | evals/src/oracle.rs::labelled_urls_and_ics_properties |
| B | evals/src/report.rs::agent_files_refuse_bad_shapes | evals/src/report.rs::agent_files_refuse_bad_shapes |
| B | evals/src/report.rs::nearest_rank_uses_ceil_minus_one | evals/src/report.rs::nearest_rank_uses_ceil_minus_one |
| B | evals/src/report.rs::wilson_matches_known_values | evals/src/report.rs::wilson_matches_known_values |
| P | evals/src/runner.rs::admin_error_categories_discard_injected_response_text | evals/src/runner.rs::admin_error_categories_discard_injected_response_text |
| P | evals/src/runner.rs::artifact_final_name_appears_only_after_complete_private_payload | evals/src/runner.rs::artifact_final_name_appears_only_after_complete_private_payload |
| P | evals/src/runner.rs::artifact_leaf_and_temporary_are_private_before_payload | evals/src/runner.rs::artifact_leaf_and_temporary_are_private_before_payload |
| P | evals/src/runner.rs::artifact_leaf_refuses_existing_directories_files_and_symlinks | evals/src/runner.rs::artifact_leaf_refuses_existing_directories_files_and_symlinks |
| P | evals/src/runner.rs::artifact_publication_refuses_existing_targets_without_clobbering | evals/src/runner.rs::artifact_publication_refuses_existing_targets_without_clobbering |
| P | evals/src/runner.rs::artifact_temp_name_collision_preserves_the_unowned_file | evals/src/runner.rs::artifact_temp_name_collision_preserves_the_unowned_file |
| P | evals/src/runner.rs::artifact_write_failure_cleans_only_its_private_partial_file | evals/src/runner.rs::artifact_write_failure_cleans_only_its_private_partial_file |
| P | evals/src/runner.rs::cargo_record_reader_bounds_lines_and_keeps_draining_after_overflow | evals/src/runner.rs::cargo_record_reader_bounds_lines_and_keeps_draining_after_overflow |
| B | evals/src/runner.rs::cleanup_observation_error_and_forced_expiry_fail_with_live_owned_processes | evals/src/runner.rs::cleanup_observation_error_and_forced_expiry_fail_with_live_owned_processes |
| P | evals/src/runner.rs::default_toolchain_homes_resolve_relative_home_at_the_caller_boundary | evals/src/runner.rs::default_toolchain_homes_resolve_relative_home_at_the_caller_boundary |
| P | evals/src/runner.rs::executable_resolution_uses_only_the_supplied_original_context | evals/src/runner.rs::executable_resolution_uses_only_the_supplied_original_context |
| B | evals/src/runner.rs::mixed_signals_after_cleanup_budget_creation_keep_one_owner_and_fixed_budgets | evals/src/runner.rs::mixed_signals_after_cleanup_budget_creation_keep_one_owner_and_fixed_budgets |
| B | evals/src/runner.rs::runner_cleanup_await_pending_and_bypass_controls_cover_signals_and_faults | evals/src/runner.rs::runner_cleanup_await_pending_and_bypass_controls_cover_signals_and_faults |
| B | evals/src/runner.rs::runner_guard_finish_failure_entrypoint | evals/src/runner.rs::runner_guard_finish_failure_entrypoint |
| B | evals/src/runner.rs::runner_guard_finish_rejection_fails_in_isolated_process_and_rescues | evals/src/runner.rs::runner_guard_finish_rejection_fails_in_isolated_process_and_rescues |
| B | evals/src/runner.rs::runner_probe_entrypoint | evals/src/runner.rs::runner_probe_entrypoint |
| B | evals/src/runner.rs::runner_probe_failure_teardown_handles_owner_and_leader_exit | evals/src/runner.rs::runner_probe_failure_teardown_handles_owner_and_leader_exit |
| B | evals/src/runner.rs::trial_pre_handler_control_runs_before_owned_children_exist | evals/src/runner.rs::trial_pre_handler_control_runs_before_owned_children_exist |
| B | evals/tests/boundary.rs::evals_is_its_own_unpublished_workspace_and_the_root_does_not_package_it | evals/tests/boundary.rs::evals_is_its_own_unpublished_workspace_and_the_root_does_not_package_it |
| B | evals/tests/boundary.rs::every_evals_package_is_already_in_the_root_lock | evals/tests/boundary.rs::every_evals_package_is_already_in_the_root_lock |
| B | evals/tests/fixture.rs::a_delivery_failing_after_provisioning_replays_as_a_harness_error | evals/tests/fixture.rs::a_delivery_failing_after_provisioning_replays_as_a_harness_error |
| B | evals/tests/fixture.rs::a_stale_code_sits_beside_the_current_one | evals/tests/fixture.rs::a_stale_code_sits_beside_the_current_one |
| B | evals/tests/fixture.rs::a_support_reply_needs_the_mailed_challenge_and_a_body | evals/tests/fixture.rs::a_support_reply_needs_the_mailed_challenge_and_a_body |
| B | evals/tests/fixture.rs::a_transient_refusal_is_parked_and_retried_once_cleared | evals/tests/fixture.rs::a_transient_refusal_is_parked_and_retried_once_cleared |
| B | evals/tests/fixture.rs::confirm_sits_beside_unsubscribe_and_report | evals/tests/fixture.rs::confirm_sits_beside_unsubscribe_and_report |
| B | evals/tests/fixture.rs::every_action_is_bound_to_its_identity | evals/tests/fixture.rs::every_action_is_bound_to_its_identity |
| B | evals/tests/fixture.rs::freezing_stops_every_mutation | evals/tests/fixture.rs::freezing_stops_every_mutation |
| B | evals/tests/fixture.rs::sibling_and_unicode_decoys_land_in_the_same_inbox | evals/tests/fixture.rs::sibling_and_unicode_decoys_land_in_the_same_inbox |
| B | evals/tests/fixture.rs::slow_mail_is_held_until_released_and_the_first_wait_times_out | evals/tests/fixture.rs::slow_mail_is_held_until_released_and_the_first_wait_times_out |
| B | evals/tests/fixture.rs::state_reads_are_read_only_and_redacted | evals/tests/fixture.rs::state_reads_are_read_only_and_redacted |
| B | evals/tests/fixture.rs::the_admin_plane_needs_the_bearer_and_the_public_plane_has_none_of_it | evals/tests/fixture.rs::the_admin_plane_needs_the_bearer_and_the_public_plane_has_none_of_it |
| B | evals/tests/fixture.rs::the_digest_has_exactly_forty_links_and_one_is_right | evals/tests/fixture.rs::the_digest_has_exactly_forty_links_and_one_is_right |
| B | evals/tests/fixture.rs::the_invite_attachment_and_its_download_are_the_same_ics | evals/tests/fixture.rs::the_invite_attachment_and_its_download_are_the_same_ics |
| B | evals/tests/fixture.rs::the_marketing_url_is_entity_encoded_and_only_its_decoding_completes | evals/tests/fixture.rs::the_marketing_url_is_entity_encoded_and_only_its_decoding_completes |
| B | evals/tests/proof.rs::every_task_fails_with_null_and_its_declared_idle_class | evals/tests/proof.rs::every_task_fails_with_null_and_its_declared_idle_class |
| B | evals/tests/proof.rs::every_task_passes_with_the_oracle | evals/tests/proof.rs::every_task_passes_with_the_oracle |
| B | evals/tests/proof.rs::helper_agent | evals/tests/proof.rs::helper_agent |
| B | evals/tests/proof.rs::two_concurrent_isolation_trials_both_pass | evals/tests/proof.rs::two_concurrent_isolation_trials_both_pass |
| B | evals/tests/proof.rs::wrong_actions_fail_in_their_observable_classes | evals/tests/proof.rs::wrong_actions_fail_in_their_observable_classes |
| B | evals/tests/report.rs::a_broken_matrix_stdout_stops_after_the_owned_trial_finishes | evals/tests/report.rs::a_broken_matrix_stdout_stops_after_the_owned_trial_finishes |
| BP | evals/tests/report.rs::a_missing_swarmail_binary_still_emits_the_whole_product | evals/tests/report.rs::a_missing_swarmail_binary_still_emits_the_whole_product |
| BP | evals/tests/report.rs::empty_and_all_error_inputs_give_null_estimates_and_bad_rows_refuse | evals/tests/report.rs::empty_and_all_error_inputs_give_null_estimates_and_bad_rows_refuse |
| B | evals/tests/report.rs::helper_agent | evals/tests/report.rs::helper_agent |
| P | evals/tests/report.rs::matrix_preserves_non_utf8_path_discovery_through_the_worker_cli | evals/tests/report.rs::matrix_preserves_non_utf8_path_discovery_through_the_worker_cli |
| B | evals/tests/report.rs::matrix_signal_cancels_and_reaps_the_fallback_cargo_group | evals/tests/report.rs::matrix_signal_cancels_and_reaps_the_fallback_cargo_group |
| B | evals/tests/report.rs::matrix_sigterm_and_sigint_wait_for_the_active_trials_owned_cleanup | evals/tests/report.rs::matrix_sigterm_and_sigint_wait_for_the_active_trials_owned_cleanup |
| P | evals/tests/report.rs::matrix_workers_are_closed_and_child_output_is_not_published | evals/tests/report.rs::matrix_workers_are_closed_and_child_output_is_not_published |
| B | evals/tests/report.rs::normal_matrix_binary_ignores_test_control_environment_variables | evals/tests/report.rs::normal_matrix_binary_ignores_test_control_environment_variables |
| BP | evals/tests/report.rs::the_matrix_keeps_counting_after_a_harness_error_and_exits_two | evals/tests/report.rs::the_matrix_keeps_counting_after_a_harness_error_and_exits_two |
| BP | evals/tests/report.rs::the_matrix_refuses_bad_inputs_before_running_anything | evals/tests/report.rs::the_matrix_refuses_bad_inputs_before_running_anything |
| BP | evals/tests/report.rs::the_matrix_runs_the_whole_product_and_aggregates_exits | evals/tests/report.rs::the_matrix_runs_the_whole_product_and_aggregates_exits |
| BP | evals/tests/report.rs::the_report_separates_harness_errors_from_the_success_rate | evals/tests/report.rs::the_report_separates_harness_errors_from_the_success_rate |
| BP | evals/tests/runner.rs::a_deadline_is_budget_and_reaps_the_agents_descendants | evals/tests/runner.rs::a_deadline_is_budget_and_reaps_the_agents_descendants |
| BP | evals/tests/runner.rs::a_delivery_parked_after_provisioning_outranks_the_business_check | evals/tests/runner.rs::a_delivery_parked_after_provisioning_outranks_the_business_check |
| BP | evals/tests/runner.rs::a_delivery_parked_after_provisioning_outranks_the_deadline | evals/tests/runner.rs::a_delivery_parked_after_provisioning_outranks_the_deadline |
| BP | evals/tests/runner.rs::a_noisy_agent_is_drained_and_does_not_hang_the_trial | evals/tests/runner.rs::a_noisy_agent_is_drained_and_does_not_hang_the_trial |
| BP | evals/tests/runner.rs::a_retained_snapshot_reproduces_the_verdict_and_bad_state_refuses | evals/tests/runner.rs::a_retained_snapshot_reproduces_the_verdict_and_bad_state_refuses |
| P | evals/tests/runner.rs::a_runner_panic_cleans_up_and_still_emits_one_row | evals/src/runner.rs::a_runner_panic_cleans_up_and_still_emits_one_row |
| BP | evals/tests/runner.rs::a_spawn_failure_is_a_harness_error_with_null_statuses | evals/tests/runner.rs::a_spawn_failure_is_a_harness_error_with_null_statuses |
| BP | evals/tests/runner.rs::a_swarmail_that_never_comes_up_is_a_harness_error | evals/tests/runner.rs::a_swarmail_that_never_comes_up_is_a_harness_error |
| P | evals/tests/runner.rs::agent_argv_stays_literal_and_wrapper_config_is_explicit | evals/tests/runner.rs::agent_argv_stays_literal_and_wrapper_config_is_explicit |
| BP | evals/tests/runner.rs::an_agent_that_succeeds_then_crashes_still_passes_with_its_real_status | evals/tests/runner.rs::an_agent_that_succeeds_then_crashes_still_passes_with_its_real_status |
| BP | evals/tests/runner.rs::an_external_agent_gets_literal_argv_and_identical_prompt_and_env | evals/tests/runner.rs::an_external_agent_gets_literal_argv_and_identical_prompt_and_env |
| P | evals/tests/runner.rs::artifact_write_failures_use_safe_categories | evals/tests/runner.rs::artifact_write_failures_use_safe_categories |
| P | evals/tests/runner.rs::artifacts_are_private_under_permissive_and_restrictive_umasks | evals/tests/runner.rs::artifacts_are_private_under_permissive_and_restrictive_umasks |
| P | evals/tests/runner.rs::automatic_build_accepts_explicit_homes_without_caller_home | evals/tests/runner.rs::automatic_build_accepts_explicit_homes_without_caller_home |
| P | evals/tests/runner.rs::automatic_build_uses_a_cleared_named_preparation_map | evals/tests/runner.rs::automatic_build_uses_a_cleared_named_preparation_map |
| P | evals/tests/runner.rs::child_output_canaries_are_drained_without_publication | evals/tests/runner.rs::child_output_canaries_are_drained_without_publication |
| P | evals/tests/runner.rs::child_output_server_death_remains_a_harness_error_on_linux | evals/tests/runner.rs::child_output_server_death_remains_a_harness_error_on_linux |
| B | evals/tests/runner.rs::cleanup_escalates_after_a_leader_exits_and_a_descendant_ignores_term | evals/tests/runner.rs::cleanup_escalates_after_a_leader_exits_and_a_descendant_ignores_term |
| P | evals/tests/runner.rs::direct_trial_preserves_non_utf8_later_arguments | evals/tests/runner.rs::direct_trial_preserves_non_utf8_later_arguments |
| P | evals/tests/runner.rs::fixture_gets_only_its_private_runtime_map_and_generated_admin_input | evals/tests/runner.rs::fixture_gets_only_its_private_runtime_map_and_generated_admin_input |
| BP | evals/tests/runner.rs::helper_agent | evals/tests/runner.rs::helper_agent |
| BP | evals/tests/runner.rs::invalid_inputs_are_one_harness_error_row_with_the_supplied_labels | evals/tests/runner.rs::invalid_inputs_are_one_harness_error_row_with_the_supplied_labels |
| P | evals/tests/runner.rs::missing_program_and_bad_clap_values_do_not_echo_input_text | evals/tests/runner.rs::missing_program_and_bad_clap_values_do_not_echo_input_text |
| B | evals/tests/runner.rs::normal_trial_binary_ignores_the_former_runner_fault_control | evals/tests/runner.rs::normal_trial_binary_ignores_the_former_runner_fault_control |
| P | evals/tests/runner.rs::oversized_cargo_record_is_drained_and_fails_without_echo | evals/tests/runner.rs::oversized_cargo_record_is_drained_and_fails_without_echo |
| P | evals/tests/runner.rs::readiness_continues_draining_after_the_first_valid_announcement | evals/tests/runner.rs::readiness_continues_draining_after_the_first_valid_announcement |
| P | evals/tests/runner.rs::readiness_discards_an_oversized_preannouncement_flood | evals/tests/runner.rs::readiness_discards_an_oversized_preannouncement_flood |
| P | evals/tests/runner.rs::relative_and_bare_agent_programs_resolve_at_the_caller_boundary | evals/tests/runner.rs::relative_and_bare_agent_programs_resolve_at_the_caller_boundary |
| P | evals/tests/runner.rs::server_death_during_the_trial_is_a_harness_error | evals/src/runner.rs::server_death_during_the_trial_is_a_harness_error |
| BP | evals/tests/runner.rs::sigterm_interrupts_cleans_up_and_emits_one_row | evals/tests/runner.rs::sigterm_interrupts_cleans_up_and_emits_one_row |
| P | evals/tests/runner.rs::swarmail_gets_only_its_fixed_runtime_map | evals/tests/runner.rs::swarmail_gets_only_its_fixed_runtime_map |
| BP | evals/tests/runner.rs::the_catalogue_is_served_as_schema_version_one | evals/tests/runner.rs::the_catalogue_is_served_as_schema_version_one |
| BP | evals/tests/runner.rs::two_trials_use_fresh_ports_and_identities | evals/tests/runner.rs::two_trials_use_fresh_ports_and_identities |
| B | evals/tests/scoring.rs::a_fact_without_its_delivery_receipt_does_not_pass | evals/tests/scoring.rs::a_fact_without_its_delivery_receipt_does_not_pass |
| B | evals/tests/scoring.rs::a_failed_or_parked_fixture_delivery_is_unscorable | evals/tests/scoring.rs::a_failed_or_parked_fixture_delivery_is_unscorable |
| B | evals/tests/scoring.rs::a_satisfied_predicate_passes_even_after_wrong_actions | evals/tests/scoring.rs::a_satisfied_predicate_passes_even_after_wrong_actions |
| B | evals/tests/scoring.rs::each_scenario_maps_to_its_flow_and_only_retry_uses_chaos | evals/tests/scoring.rs::each_scenario_maps_to_its_flow_and_only_retry_uses_chaos |
| B | evals/tests/scoring.rs::held_mail_and_a_retried_delivery_score_normally | evals/tests/scoring.rs::held_mail_and_a_retried_delivery_score_normally |
| B | evals/tests/scoring.rs::isolation_forbids_a_foreign_presentation_even_when_all_confirmed | evals/tests/scoring.rs::isolation_forbids_a_foreign_presentation_even_when_all_confirmed |
| B | evals/tests/scoring.rs::malformed_or_mismatched_snapshots_refuse | evals/tests/scoring.rs::malformed_or_mismatched_snapshots_refuse |
| B | evals/tests/scoring.rs::rendering_fills_every_placeholder_without_rescanning_values | evals/tests/scoring.rs::rendering_fills_every_placeholder_without_rescanning_values |
| B | evals/tests/scoring.rs::the_catalogue_is_fifteen_tasks_in_three_tiers_plus_isolation | evals/tests/scoring.rs::the_catalogue_is_fifteen_tasks_in_three_tiers_plus_isolation |
| B | evals/tests/scoring.rs::the_expected_replay_refusal_is_evidence_in_reset_reuse_only | evals/tests/scoring.rs::the_expected_replay_refusal_is_evidence_in_reset_reuse_only |
| B | evals/tests/scoring.rs::the_first_refused_action_names_the_class_else_the_idle_class | evals/tests/scoring.rs::the_first_refused_action_names_the_class_else_the_idle_class |
| B | evals/tests/scoring.rs::unknown_enum_values_and_fields_refuse_at_deserialization | evals/tests/scoring.rs::unknown_enum_values_and_fields_refuse_at_deserialization |
| B | evals/tests/scoring.rs::validation_refuses_every_malformed_declaration | evals/tests/scoring.rs::validation_refuses_every_malformed_declaration |

Additional required focused names already present in the handoff; verify the
whole conjunction rather than using existence as proof:

| Target | Required assertions |
| --- | --- |
| evals/tests/runner.rs::loopback_clients_ignore_caller_proxy_settings | Dead HTTP/HTTPS/ALL proxy, actual oracle completion; parent-admin directness and complete role observations; all reqwest builders audited |
| evals/tests/report.rs::privacy_cancellation_preserves_owned_runtime_and_suffix_accounting | Acknowledged noisy active worker cancellation; private dirs remain through cleanup, then release; exact prefix/active/suffix, no later launch, safe failure facts |
| evals/src/runner.rs::admin_transport_read_and_parse_failures_are_value_free | Actual exercised transport/read/invalid-JSON failures with independent canaries; no URL/body/error text |
| evals/src/runner.rs::cargo_runtime_owner_survives_cancelled_preparation | Real owned Cargo group and drain/cleanup fence retain runtime dirs; no early release |
| evals/src/runner.rs::trial_runtime_owner_survives_aborted_body | Live owned descendants and acknowledged cleanup Pending after body join still have private runtime dirs |

No new test is fulfilled merely by an empty scan, missing artifact or timeout.
Use real processes/protocols, explicit execution and completion markers and
independent byte counts. Count both noisy streams and drain test-driver stdout/
stderr concurrently with bounded capture/incremental canary scanning; otherwise
restoring a leak might only deadlock a test pipe. Avoid global environment/umask
mutation. Keep fixtures generated, chaos 0/100 and ordering acknowledged.

## Quickstart and mutation receipts

From repository root, focused iterations use:

~~~bash
cargo test --manifest-path evals/Cargo.toml --locked --test runner
cargo test --manifest-path evals/Cargo.toml --locked --test report
cargo test --manifest-path evals/Cargo.toml --locked --test proof
cargo test --manifest-path evals/Cargo.toml --locked admin_error_categories_discard_injected_response_text
cargo test --manifest-path evals/Cargo.toml --locked artifact_
cargo test --manifest-path evals/Cargo.toml --locked matrix_completion_fence_is_observed_and_its_negative_control_exits_early
cargo test --manifest-path evals/Cargo.toml --locked runner_cleanup_await_pending_and_bypass_controls_cover_signals_and_faults
~~~

Each mutation is temporary, scoped and recorded with exact source/test region,
candidate/tree, command, OS, actual exit and intended failed assertion:

| Mutation | Focused command suffix after cargo test --manifest-path evals/Cargo.toml --locked | Required red failure |
| --- | --- | --- |
| Remove runtime env_clear in one observed role | --test runner an_external_agent_gets_literal_argv_and_identical_prompt_and_env (and the corresponding fixture/server/worker probe for each role) | Complete-map/canary exposure assertion |
| Restore child raw output publication | --test runner child_output_canaries_are_drained_without_publication | Canary scan after counted completion and real Fail/GaveUp artifacts |
| Restore binary metadata/raw error rendering | --test runner missing_program_and_bad_clap_values_do_not_echo_input_text; separately child_output_canaries_are_drained_without_publication | Explicit path/error or metadata-null assertion, independently bound |
| Remove Cargo overflow rejection | --test runner oversized_cargo_record_is_drained_and_fails_without_echo | Oversize refusal assertion after successful wrapper writes/Cargo completion |
| Restore String-only worker boundary | --test report matrix_preserves_non_utf8_path_discovery_through_the_worker_cli | Native execution/argument-byte assertion |
| Restore permissive writer / move chmod after payload | --test runner artifacts_are_private_under_permissive_and_restrictive_umasks; separately artifact_leaf_and_temporary_are_private_before_payload | Exact mode and pre-payload assertions respectively |
| Bypass awaited cleanup | Existing two named fence tests above | Product exits while acknowledged live cleanup remains; then identity-safe rescue |

Run the same commands after restoration; unrelated compilation failure, missing
launch, blocked collector or rescue-only success is not a red proof. Keep #194
test guard checked-finish/identity-aware rescue semantics and negative controls
for guard rejection, observation failure and forced expiry. Full restored
candidate gates still follow these focused proofs.

## Final verification and review

One authoritative gate inventory remains docs/house-rules.md § Gates. Verify
engine/tooling/shim identity first; execute pinned script copies through the
hermetic proof chain (0050), not unchecked candidate dispatch shims. The public
inventory commands below identify their equivalent pinned proofs:

~~~bash
sha256sum .forge/bin/brokkr
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
bash scripts/seal-branch-test.sh
cargo deny check licenses
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --workspace --locked
~~~

Use the pinned verify seat's native-tls absence proof: cargo tree -i native-tls
must fail specifically with "did not match any packages"; success or any other
error refuses. Exact coverage means zero missed lines, branches and functions.
All standalone eval targets, including inline, boundary, fixture, runner, report,
scoring and proof, execute. Never infer eval success from root green.

All Linux preparation/toolchain/custom-target and selected artifact-primitive
receipts remain mandatory. The 2026-10-24 operator ruling linked in spec AC13
places evals outside CI on every OS until #201 and assigns macOS eval preparation,
toolchain, clippy/tests and artifact execution to #201's CI gating. Those macOS
receipts do not block #196. Linux inspection/cross-compilation and historical
summaries do not establish macOS execution; do not claim such execution here.
CI separately owns charter chart/history/selftest, Docker smoke, network RustSec,
tag release-rate and privileged port-80 proofs. Record ownership without querying
CI or inventing receipts. Retain every Linux proof and all five careful reviews.

After implementation/verification, use the existing bundles/codex-careful route:
separate Astra correctness, official Flash security, Sonnet 5.5 tests, Astra
guarantees and Fable 5.1 independent results, with protected review/worst verdict/
security hold and no unauthorized fallback. This sole design seat does not
simulate those reviews. Candidate changes require affected proof and final gates
again. No push, merge, tag, publication, issue closure or decision acceptance.

## Risks

| Risk | Mitigation and accepted limit |
| --- | --- |
| Privacy helper outlives neither body nor descendant | Retain runtime resources at existing cleanup owners; actual Pending/cancellation lifetime tests |
| Removal of output retention removes protocol/lifecycle evidence | Separate typed readiness/Cargo state from discard drains; preserve waits, EOF and numeric failure facts |
| Cleared env breaks #194 test controls | Explicit test-owned wrappers/config install controls after observing launch maps; keep cfg(test)-only seams and normal dev/release inertness |
| Matrix port regresses native argv or cancellation | Native OsString handoff; retain all #194 race/bound/fence tests and 64 KiB bound |
| Flood tests pass on nonexecution or deadlock | Count completed writes, acknowledge completion, bounded concurrent collection, intended-assertion mutations and below-limit controls |
| Restrictive umask prevents access; permissive mask exposes bytes | Normalize newly owned directory/file before payload via handles, exact-mode tests in isolated subprocesses |
| Collision clobbers another writer or temp cleanup removes a sentinel | Exclusive leaf/temp, persist_noclobber, ownership-specific cleanup and file/link/directory sentinel tests |
| Loss of diagnostics/ambient authentication surprises users | Document finite facts, private-file wrapper migration and prebuilt binary option; no raw-output escape hatch |
| macOS preparation or primitive differs | #201 owns actual macOS CI execution under the dated ruling in AC13; do not infer portability from Linux green, and preserve conservative non-Linux cleanup behavior |
| More code weakens coverage/dependency constraints | Existing owners and locked dependencies only; exact root gate unchanged, eval suite separately mandatory |
| Apparent cross-feature contradiction | Stop exact incompatible choice for operator; no weakening either suite or accepted ruling |
| Draft or prior green mistaken for verified handoff | Checkboxes remain fresh proof obligations; distinguish code already in 529d916, current Linux receipts, #201-owned macOS work, structural validation and five independent reviews |
