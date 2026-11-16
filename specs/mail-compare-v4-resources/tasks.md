# Tasks — mail comparison protocol v4, story A

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md). Ordered. Each group is
green (`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets
--locked -- -D warnings`, `cargo test --locked --test mail_compare`) and
committed unsigned, in the repository's style, before the next. Never push.
New tests live in `tests/mail_compare/v4.rs` unless a task says otherwise;
names below are the test functions to write. A test that reads `/proc`,
`/sys` or `/dev/shm` is Linux-only.

## T1 Protocol 4, lock 5 and the records in the tree (V1–V7)

- [ ] T1.1 Add `PROTOCOL_VERSION_4`, `PROTOCOL_V4_ID`, `LOCK_VERSION_5`; let
  `validate` know version 4 and hold it to protocol 3's rules except id,
  predecessor and reason; require `mailslurper` in `does_not_start`; extend
  `pairing` and `collection_ready`. — Tests:
  `v4_protocol_validates_with_its_id_predecessor_and_reason`,
  `v4_refuses_a_wrong_id_predecessor_or_reason`,
  `v4_keeps_every_v3_rule` (a missing target, rotation, capability record,
  product scope and `does_not_start` entry each refuse),
  `every_cross_pairing_of_protocols_and_locks_refuses` (the nine pairs).
  (AC1)
- [ ] T1.2 Add the optional `Resources` fields and V2's exact-value rules;
  protocols 1 to 3 refuse each new field by name. — Tests:
  `v4_resources_block_refuses_each_absent_or_different_value` (one case per
  field), `earlier_protocols_refuse_each_v4_resources_field`,
  `v2_and_v3_history_bytes_round_trip_unchanged`. (AC2)
- [ ] T1.3 The cost section, the 32-session bound, no wait profile, and
  `headline`. — Tests: `frozen_v4_cost_table_is_exact` (added, removed and
  each changed field), `v4_refuses_thirty_three_sessions`,
  `v4_refuses_a_wait_profile`, `v4_headline_is_required_and_is_cost_c8`. (AC3)
- [ ] T1.4 The 150,000 cap rule. — Test:
  `v4_refuses_a_cap_other_than_150000` (Swarmail and one competitor). (AC4)
- [ ] T1.5 `clients_sha256`, the freeze rule and the owed-section constant;
  the validator's inner entry takes the owed list. — Tests:
  `this_driver_refuses_a_frozen_v4_naming_the_owed_sections`,
  `with_nothing_owed_a_frozen_v4_still_needs_resolved_clients_and_inspected_capabilities`
  (three cases), `earlier_protocols_refuse_clients_sha256`. (AC5)
- [ ] T1.6 Lock 5: `SourceLock::validate`, `cap_check` and `launch_checks`
  know the placeholder; lock 3 and lock 4 refuse it by name. — Tests:
  `lock_v5_requires_the_placeholder_at_each_cap_flag` (argv pair, `flag=`,
  `env:`), `lock_v5_refuses_a_literal_cap`,
  `lock_v5_refuses_the_placeholder_elsewhere`,
  `lock_v4_and_v3_refuse_the_placeholder`. (AC4)
- [ ] T1.7 `cp` the active protocol and lock to
  `docs/evals/history/2026-11-07-protocol.v3.json` and
  `docs/evals/history/2026-11-07-sources.lock.v4.json`; record `sha256sum`
  of all four files before and after. Point `PROTOCOL_V3`, `SOURCES_V4` and
  the two `std::fs::read("docs/evals/protocol.json")` tests at the history
  copies. — Tests: extend the frozen-hash test in `eight_targets.rs` to the
  copies; `the_active_records_are_still_v3_until_the_v4_freeze` (active
  bytes equal the history copies). The whole existing suite stays green.
  (AC6)
- [ ] T1.8 Write `docs/evals/draft/protocol.v4.json` and
  `docs/evals/draft/sources.lock.v5.json` (plan, *Draft data*). — Tests:
  `the_v4_draft_validates_and_is_not_ready_for_collection`,
  `the_v4_draft_differs_from_v3_only_where_0132_says` (key-by-key against the
  v3 bytes, with the allowed differences listed in the test),
  `lock_v5_differs_from_lock_v4_only_in_version_caps_and_three_launches`,
  `lock_v5_verifies_its_documents_and_pairs_with_the_draft`. (AC6)

## T2 `resources.rs` on fixtures (V8, V9, V11, V12, parsers of V13)

- [ ] T2.1 Create `benches/mail_compare/resources.rs`; declare it in the
  bench and, by `#[path]`, in `tests/mail_compare.rs`. Detection and
  adoption over a root path; the non-Linux branch. — Tests:
  `detect_refuses_exactly_without_a_cgroup2_root`,
  `detect_refuses_exactly_without_each_controller`,
  `adoption_refuses_exactly_when_the_read_back_lacks_a_controller`,
  `adoption_refuses_an_occupied_scope_and_an_existing_driver_leaf`,
  `adoption_makes_the_driver_leaf_before_it_enables_controllers`,
  `off_linux_detection_is_the_exact_refusal` (non-Linux only). (AC7)
- [ ] T2.2 `Leaf`: create, write the five limits, read them back, record
  them. — Tests: `a_leaf_records_the_five_limits_as_read_back`,
  `a_differing_read_back_refuses_naming_the_file`,
  `a_leaf_without_each_required_file_refuses_naming_it` (the five,
  `cgroup.kill`, `memory.peak`). (AC8)
- [ ] T2.3 The flat `key value` parser and the single-value parser, and the
  `cgroup.procs`, `/proc/<pid>/cgroup` and `Cpus_allowed_list` readers. —
  Tests: `cgroup_files_parse_from_captured_text` (`cpu.stat`, `memory.stat`,
  `memory.events`, `memory.current`, `memory.peak`, `pids.current`,
  `cgroup.procs`), `malformed_cgroup_text_refuses`,
  `proc_cgroup_and_cpus_allowed_parse` (list form, range form, mixed). (AC9,
  AC10)
- [ ] T2.4 The layout function. — Tests:
  `layout_takes_two_distinct_cores_and_idles_their_siblings` (an SMT map and
  a map without SMT), `layout_refuses_one_core`,
  `layout_refuses_when_no_client_cpu_is_left`,
  `layout_records_siblings_outside_the_set`. (AC9)
- [ ] T2.5 The scratch check and the allocated-bytes walk. — Tests:
  `tmpfs_magic_refuses_and_disk_magic_passes` (pure),
  `a_tmpfs_scratch_refuses` (Linux, `/dev/shm`),
  `a_target_dir_scratch_passes` (Linux, `CARGO_TARGET_TMPDIR`),
  `allocated_bytes_sums_blocks_of_a_tree`. (AC10)

## T3 The launcher join, teardown and receipts (V6, V10, V14, V15)

- [ ] T3.1 `LaunchSpec::from_pin` takes the cap and resolves
  `{max_messages}`; the wire run passes the protocol's cap, the adapters'
  launch mode the pin's own. — Tests:
  `the_v4_spec_carries_150000_at_each_cap_flag` (Mailpit, smtp4dev, Inbucket
  from the draft lock, and Swarmail's `--max-per-inbox`),
  `an_unresolved_placeholder_refuses_before_spawn`,
  `a_lock_v4_spec_is_unchanged_by_the_cap_argument`. (AC4)
- [ ] T3.2 `Owned::launch` takes an optional leaf context: create the leaf,
  open `cgroup.procs` before the fork, join and set affinity in `pre_exec`
  with every return checked, read back `/proc/<pid>/cgroup` and
  `Cpus_allowed_list`, refuse and remove the leaf on a mismatch;
  `LaunchError::Leaf`. — Tests: `a_launch_without_a_leaf_never_builds_a_resources_context`
  (protocol 3 path), `a_join_read_back_mismatch_refuses_the_launch` (the
  comparison function on fixtures); the join itself is T7.2 to T7.4. (AC7,
  AC9)
- [ ] T3.3 `cleanup` gains the cgroup step after the existing sequence, in
  the same budget; `CleanupFacts.cgroup_removed` and `memory_peak_bytes`;
  `proved()` for a leaf launch. — Tests:
  `a_leaf_that_never_empties_ends_at_the_budget_unproved_and_in_place`
  (fixture leaf), `the_kill_is_written_and_procs_read_once_even_past_the_budget`,
  `a_leaf_launch_without_cgroup_removed_is_not_proved`. (AC11)
- [ ] T3.4 The `LaunchFacts` additions. — Tests:
  `v3_launch_and_cleanup_receipts_serialize_byte_for_byte` (against literal
  JSON captured from the current code before this group),
  `a_v4_launch_receipt_shows_the_cap_and_keeps_ports_and_paths_as_slots`.
  (AC11)

## T4 Rows, the cost chunk and the figures (V13, V16, V17)

- [ ] T4.1 `ResourceRow` and the protocol 4 samples reader; planned windows;
  `Manifest.chunk` and `Manifest.host`. — Tests:
  `a_v4_run_directory_reads_both_row_families`,
  `a_resource_row_in_a_v3_run_directory_refuses`,
  `a_duplicate_an_unplanned_and_a_missing_window_each_refuse`,
  `a_v3_manifest_serializes_byte_for_byte`. (AC10)
- [ ] T4.2 The sampler and the window snapshots (one sample at each bound;
  `oom_killed` from the `oom_kill` delta only; `affinity_held`). — Tests:
  `a_window_outcome_is_oom_killed_only_when_oom_kill_rose` (snapshots on
  fixtures: `max` alone, `oom_kill`, neither),
  `affinity_held_is_false_when_any_task_differs` (fixture `/proc` tree),
  `the_sampler_writes_a_row_per_tick_and_one_at_each_bound` (fixture leaf, a
  paused tokio clock, no sleep).
- [ ] T4.3 `plan_v4`, `Chunk`, `run --chunk`, `targets --plan v4`. — Tests:
  `the_draft_plans_24_cost_chunks_in_rotation_order` (none for
  `mailslurper`), `chunk_names_round_trip_and_unknown_ones_refuse`,
  `protocol_4_needs_a_chunk_and_protocol_3_refuses_one`,
  `a_v4_collect_refuses_a_missing_or_dirty_commit`. (AC12)
- [ ] T4.4 The cost chunk loop: leaves for warmup and measured instances,
  `ready`, `idle_ready` and `measure` windows, a target exit as rows and an
  outcome, the chunk deadline, the exit code. — Tests on the pure parts:
  `a_profile_past_the_chunk_deadline_is_unstarted_rows_and_a_chunk_deadline_window`,
  `a_chunk_with_any_unmeasured_window_exits_one`. The loop against a real
  target is T7.5.
- [ ] T4.5 `stats`: nearest-rank median with minimum and maximum. — Test:
  `median_min_max_over_one_two_and_three_values`.
- [ ] T4.6 `report`: version by protocol, the `cost`, `idle`, `startup` and
  `headline` keys, ranking and shortfalls, the blocked row, the owed
  `mib_per_1k_retained`. — Tests:
  `cost_figures_equal_independent_arithmetic`,
  `a_zero_divisor_is_null_never_zero_or_infinite`,
  `an_inexact_repetition_unranks_the_cell_with_its_shortfall` (one case per
  shortfall name), `an_unmeasured_window_unranks_the_cell`,
  `affinity_widened_is_a_label_and_does_not_change_ranking`,
  `peak_and_mean_come_from_the_samples_inside_the_window`,
  `idle_ready_is_the_mean_of_its_last_two_seconds`,
  `the_headline_is_the_cells_of_cost_c8_and_owes_mib_per_1k_retained`,
  `the_blocked_row_has_no_number`,
  `a_v4_report_is_version_3_and_a_v3_report_is_still_version_2`. (AC13)

## T5 `footprint`, `hold` and `admit-v4` (V18–V20)

- [ ] T5.1 `footprint` as a module function and a mode. — Tests:
  `footprint_reads_archive_layer_and_candidate_sizes`,
  `footprint_installed_bytes_include_the_runtime_tree`,
  `footprint_labels_an_unpinned_runtime_and_never_counts_it_as_zero`,
  `footprint_blocks_the_does_not_start_row`,
  `footprint_refuses_a_changed_executable`. (AC14)
- [ ] T5.2 `hold` as a module function over generic input and output, and a
  mode; the same teardown on EOF, SIGTERM and SIGINT. — Tests:
  `hold_refuses_missing_flags_and_protocol_3`,
  `hold_refuses_bytes_on_stdin_by_name`; under the budget it is T7.4. (AC15)
- [ ] T5.3 `admit_v4` and `--seal`. Build the fixture run directory in the
  test (sealed chunk directories written through `RunDir`, ledger files,
  footprint). — Tests: `a_complete_fixture_run_seals_and_then_verifies`, and
  one refusal test per case of AC16:
  `admit_v4_refuses_a_missing_chunk`, `…_an_extra_chunk`,
  `…_an_unsealed_chunk`, `…_two_run_ids`, `…_a_dirty_candidate`,
  `…_differing_host_facts`, `…_a_wrong_limit`, `…_a_missing_join_read_back`,
  `…_a_wrong_cap`, `…_cgroup_removed_false_or_absent`,
  `…_a_ledger_exit_two`, `…_an_unclean_scope`, `…_a_second_seal`,
  `admit_v4_names_the_path_of_an_edited_figure`,
  `admit_run_still_refuses_a_protocol_4_directory`. (AC16)

## T6 `collect-v4` (V21)

- [ ] T6.1 Add the `collect-v4` case and usage text to
  `scripts/compare-mail.sh`, bash 3.2 syntax only; the driver override for
  tests. Extend the wrapper rig with a fake driver, `systemd-run` (execs its
  command), `systemctl` (scripted active or inactive), and a log. — Tests:
  `collect_v4_dry_run_prints_24_commands_in_plan_order_and_starts_nothing`,
  `collect_v4_refuses_the_default_protocol_and_names_the_draft_route`,
  `collect_v4_names_each_preflight_refusal` (missing tool, existing output,
  too few CPUs, dirty candidate),
  `collect_v4_nests_timeout_taskset_then_the_scope` (from the logged
  command), `collect_v4_stops_on_a_chunk_exit_two`,
  `collect_v4_counts_a_timeout_as_two`,
  `collect_v4_counts_a_sealless_exit_zero_as_two`,
  `collect_v4_stops_a_scope_that_outlives_its_chunk_and_counts_two`,
  `collect_v4_full_fake_run_writes_24_ledgers_seals_and_verifies`,
  `collect_v4_refuses_off_linux` (non-Linux only), and the existing usage
  test names `collect-v4`. (AC17)

## T7 Real proofs, docs and gates (V22–V24)

- [ ] T7.1 Add `tests/mail_compare/v4-smoke.json` (a protocol 4 draft with
  one small profile) and a shared proof prelude: scratch under
  `CARGO_TARGET_TMPDIR`, detection that panics with V8's exact text. — Test:
  `the_v4_smoke_fixture_validates_as_a_draft`.
- [ ] T7.2 `v4_real_oom_at_two_gib` (`#[ignore]`, Linux). — Proof: spec V22.1.
- [ ] T7.3 `v4_real_cpu_held_within_two` and
  `v4_real_setsid_escapee_removed`. — Proof: spec V22.2 and V22.3.
- [ ] T7.4 `v4_real_hold_swarmail_under_budget`. — Proof: spec V22.4.
- [ ] T7.5 `v4_real_cost_chunk_seals_and_admits`. — Proof: spec V22.5.
- [ ] T7.6 Run each proof where the seat stands, alone, with the bounded
  command of V22. Where there is no delegation, keep the output that shows
  V8's exact refusal for all five, and say in the result notes that the five
  host runs are owed (plan, Risk 1). Where there is, keep each passing
  output with `/proc/loadavg` before and after. Raw output goes to
  `target/mail-compare-v4-resources/receipts/`, summarized in the notes;
  nothing under `target/` is committed. — Proof: the receipts. (AC18)
- [ ] T7.7 The v4 section of `docs/evals/README.md` (V23). — Tests: the
  existing documentation and decision-inventory tests
  (`cargo test --locked --test store_contract_docs`) pass; a grep in the
  notes shows the refusal text, the draft route and Reading 4's sentence.
  (AC19)
- [ ] T7.8 Prove what did not change. — Proof:
  `git diff e23a8ba --stat -- src evals Cargo.toml Cargo.lock deny.toml` is
  empty; `git diff e23a8ba --name-status -- docs/evals` lists only added
  files and `README.md`; `sha256sum docs/evals/protocol.json
  docs/evals/sources.lock.json` equals the frozen constants. (AC19)
- [ ] T7.9 Run every gate of `docs/house-rules.md` § Gates on the final
  tree, `--locked`, the coverage gate and its selftest, the publish-guard
  proof, the seal proof and `cargo deny check licenses` included, and read
  each recipe's compile output for the policy-sweep warning. — Proof: each
  command and its exit status in the notes; a failure is reported with its
  output, not retried into silence. (AC20)
