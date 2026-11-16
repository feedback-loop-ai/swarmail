# Tasks — mail comparison protocol v4, story A

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md). Ordered. Each group is
green (`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets
--locked -- -D warnings`, `cargo test --locked --test mail_compare`) and
committed unsigned, in the repository's style, before the next. Never push.
New tests live in `tests/mail_compare/v4.rs` unless a task says otherwise;
names below are the test functions to write. A test that reads `/proc`,
`/sys` or `/dev/shm` is Linux-only.

**State after review 1 (2026-11-16).** Groups T1 to T7 were built at head
`2852bd2`; the verify seat passed every gate there and review 1 found AC1 to
AC17 and AC19 met on fixtures. They are ticked on that basis and kept as the
record of what was built. T7.6 is not ticked: no real proof has passed on a
kernel. Where revision 1 changes what a T task built, the task says so.
**The work to do now is group R**, at the end, in its order. Each R task
names the review finding it answers (plan.md § *Revision 1* has the table).

## T1 Protocol 4, lock 5 and the records in the tree (V1–V7)

- [x] T1.1 Add `PROTOCOL_VERSION_4`, `PROTOCOL_V4_ID`, `LOCK_VERSION_5`; let
  `validate` know version 4 and hold it to protocol 3's rules except id,
  predecessor and reason; require `mailslurper` in `does_not_start`; extend
  `pairing` and `collection_ready`. — Tests:
  `v4_protocol_validates_with_its_id_predecessor_and_reason`,
  `v4_refuses_a_wrong_id_predecessor_or_reason`,
  `v4_keeps_every_v3_rule` (a missing target, rotation, capability record,
  product scope and `does_not_start` entry each refuse),
  `every_cross_pairing_of_protocols_and_locks_refuses` (the nine pairs).
  (AC1)
- [x] T1.2 Add the optional `Resources` fields and V2's exact-value rules;
  protocols 1 to 3 refuse each new field by name. — Tests:
  `v4_resources_block_refuses_each_absent_or_different_value` (one case per
  field), `earlier_protocols_refuse_each_v4_resources_field`,
  `v2_and_v3_history_bytes_round_trip_unchanged`. (AC2)
- [x] T1.3 The cost section, the 32-session bound, no wait profile, and
  `headline`. — Tests: `frozen_v4_cost_table_is_exact` (added, removed and
  each changed field), `v4_refuses_thirty_three_sessions`,
  `v4_refuses_a_wait_profile`, `v4_headline_is_required_and_is_cost_c8`. (AC3)
- [x] T1.4 The 150,000 cap rule. — Test:
  `v4_refuses_a_cap_other_than_150000` (Swarmail and one competitor). (AC4)
- [x] T1.5 `clients_sha256`, the freeze rule and the owed-section constant;
  the validator's inner entry takes the owed list. — Tests:
  `this_driver_refuses_a_frozen_v4_naming_the_owed_sections`,
  `with_nothing_owed_a_frozen_v4_still_needs_resolved_clients_and_inspected_capabilities`
  (three cases), `earlier_protocols_refuse_clients_sha256`. (AC5)
- [x] T1.6 Lock 5: `SourceLock::validate`, `cap_check` and `launch_checks`
  know the placeholder; lock 3 and lock 4 refuse it by name. — Tests:
  `lock_v5_requires_the_placeholder_at_each_cap_flag` (argv pair, `flag=`,
  `env:`), `lock_v5_refuses_a_literal_cap`,
  `lock_v5_refuses_the_placeholder_elsewhere`,
  `lock_v4_and_v3_refuse_the_placeholder`. (AC4)
- [x] T1.7 `cp` the active protocol and lock to
  `docs/evals/history/2026-11-07-protocol.v3.json` and
  `docs/evals/history/2026-11-07-sources.lock.v4.json`; record `sha256sum`
  of all four files before and after. Point `PROTOCOL_V3`, `SOURCES_V4` and
  the two `std::fs::read("docs/evals/protocol.json")` tests at the history
  copies. — Tests: extend the frozen-hash test in `eight_targets.rs` to the
  copies; `the_active_records_are_still_v3_until_the_v4_freeze` (active
  bytes equal the history copies). The whole existing suite stays green.
  (AC6)
- [x] T1.8 Write `docs/evals/draft/protocol.v4.json` and
  `docs/evals/draft/sources.lock.v5.json` (plan, *Draft data*). — Tests:
  `the_v4_draft_validates_and_is_not_ready_for_collection`,
  `the_v4_draft_differs_from_v3_only_where_0132_says` (key-by-key against the
  v3 bytes, with the allowed differences listed in the test),
  `lock_v5_differs_from_lock_v4_only_in_version_caps_and_three_launches`,
  `lock_v5_verifies_its_documents_and_pairs_with_the_draft`. (AC6)

## T2 `resources.rs` on fixtures (V8, V9, V11, V12, parsers of V13)

- [x] T2.1 Create `benches/mail_compare/resources.rs`; declare it in the
  bench and, by `#[path]`, in `tests/mail_compare.rs`. Detection and
  adoption over a root path; the non-Linux branch. — Tests:
  `detect_refuses_exactly_without_a_cgroup2_root`,
  `detect_refuses_exactly_without_each_controller`,
  `adoption_refuses_exactly_when_the_read_back_lacks_a_controller`,
  `adoption_refuses_an_occupied_scope_and_an_existing_driver_leaf`,
  `adoption_makes_the_driver_leaf_before_it_enables_controllers`,
  `off_linux_detection_is_the_exact_refusal` (non-Linux only). (AC7)
- [x] T2.2 `Leaf`: create, write the five limits, read them back, record
  them. — Tests: `a_leaf_records_the_five_limits_as_read_back`,
  `a_differing_read_back_refuses_naming_the_file`,
  `a_leaf_without_each_required_file_refuses_naming_it` (the five,
  `cgroup.kill`, `memory.peak`). (AC8)
- [x] T2.3 The flat `key value` parser and the single-value parser, and the
  `cgroup.procs`, `/proc/<pid>/cgroup` and `Cpus_allowed_list` readers. —
  Tests: `cgroup_files_parse_from_captured_text` (`cpu.stat`, `memory.stat`,
  `memory.events`, `memory.current`, `memory.peak`, `pids.current`,
  `cgroup.procs`), `malformed_cgroup_text_refuses`,
  `proc_cgroup_and_cpus_allowed_parse` (list form, range form, mixed). (AC9,
  AC10)
- [x] T2.4 The layout function. — Tests:
  `layout_takes_two_distinct_cores_and_idles_their_siblings` (an SMT map and
  a map without SMT), `layout_refuses_one_core`,
  `layout_refuses_when_no_client_cpu_is_left`,
  `layout_records_siblings_outside_the_set`. (AC9)
- [x] T2.5 The scratch check and the allocated-bytes walk. — Tests:
  `tmpfs_magic_refuses_and_disk_magic_passes` (pure),
  `a_tmpfs_scratch_refuses` (Linux, `/dev/shm`),
  `a_target_dir_scratch_passes` (Linux, `CARGO_TARGET_TMPDIR`),
  `allocated_bytes_sums_blocks_of_a_tree`. (AC10)

## T3 The launcher join, teardown and receipts (V6, V10, V14, V15)

- [x] T3.1 `LaunchSpec::from_pin` takes the cap and resolves
  `{max_messages}`; the wire run passes the protocol's cap, the adapters'
  launch mode the pin's own. — Tests:
  `the_v4_spec_carries_150000_at_each_cap_flag` (Mailpit, smtp4dev, Inbucket
  from the draft lock, and Swarmail's `--max-per-inbox`),
  `an_unresolved_placeholder_refuses_before_spawn`,
  `a_lock_v4_spec_is_unchanged_by_the_cap_argument`. (AC4)
- [x] T3.2 `Owned::launch` takes an optional leaf context: create the leaf,
  open `cgroup.procs` before the fork, join and set affinity in `pre_exec`
  with every return checked, read back `/proc/<pid>/cgroup` and
  `Cpus_allowed_list`, refuse and remove the leaf on a mismatch;
  `LaunchError::Leaf`. — Tests: `a_launch_without_a_leaf_never_builds_a_resources_context`
  (protocol 3 path), `a_join_read_back_mismatch_refuses_the_launch` (the
  comparison function on fixtures); the join itself is T7.2 to T7.4. (AC7,
  AC9)
- [x] T3.3 `cleanup` gains the cgroup step after the existing sequence, in
  the same budget; `CleanupFacts.cgroup_removed` and `memory_peak_bytes`;
  `proved()` for a leaf launch. — Tests:
  `a_leaf_that_never_empties_ends_at_the_budget_unproved_and_in_place`
  (fixture leaf), `the_kill_is_written_and_procs_read_once_even_past_the_budget`,
  `a_leaf_launch_without_cgroup_removed_is_not_proved`. (AC11)
- [x] T3.4 The `LaunchFacts` additions. — Tests:
  `v3_launch_and_cleanup_receipts_serialize_byte_for_byte` (against literal
  JSON captured from the current code before this group),
  `a_v4_launch_receipt_shows_the_cap_and_keeps_ports_and_paths_as_slots`.
  (AC11)

## T4 Rows, the cost chunk and the figures (V13, V16, V17)

- [x] T4.1 `ResourceRow` and the protocol 4 samples reader; planned windows;
  `Manifest.chunk` and `Manifest.host`. — Tests:
  `a_v4_run_directory_reads_both_row_families`,
  `a_resource_row_in_a_v3_run_directory_refuses`,
  `a_duplicate_an_unplanned_and_a_missing_window_each_refuse`,
  `a_v3_manifest_serializes_byte_for_byte`. (AC10)
- [x] T4.2 The sampler and the window snapshots (one sample at each bound;
  `oom_killed` from the `oom_kill` delta only; `affinity_held`). — Tests:
  `a_window_outcome_is_oom_killed_only_when_oom_kill_rose` (snapshots on
  fixtures: `max` alone, `oom_kill`, neither),
  `affinity_held_is_false_when_any_task_differs` (fixture `/proc` tree),
  `the_sampler_writes_a_row_per_tick_and_one_at_each_bound` (fixture leaf, a
  paused tokio clock, no sleep).
- [x] T4.3 `plan_v4`, `Chunk`, `run --chunk`, `targets --plan v4`. — Tests:
  `the_draft_plans_24_cost_chunks_in_rotation_order` (none for
  `mailslurper`), `chunk_names_round_trip_and_unknown_ones_refuse`,
  `protocol_4_needs_a_chunk_and_protocol_3_refuses_one`,
  `a_v4_collect_refuses_a_missing_or_dirty_commit`. (AC12)
- [x] T4.4 The cost chunk loop: leaves for warmup and measured instances,
  `ready`, `idle_ready` and `measure` windows, a target exit as rows and an
  outcome, the chunk deadline, the exit code. — Tests on the pure parts:
  `a_profile_past_the_chunk_deadline_is_unstarted_rows_and_a_chunk_deadline_window`,
  `a_chunk_with_any_unmeasured_window_exits_one`. The loop against a real
  target is T7.5. *(Revision 1: the deadline as built here bounds nothing in
  flight; R1 replaces it, and R2 and R3 replace what `skip_profile` writes.)*
- [x] T4.5 `stats`: nearest-rank median with minimum and maximum. — Test:
  `median_min_max_over_one_two_and_three_values`.
- [x] T4.6 `report`: version by protocol, the `cost`, `idle`, `startup` and
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

- [x] T5.1 `footprint` as a module function and a mode. — Tests:
  `footprint_reads_archive_layer_and_candidate_sizes`,
  `footprint_installed_bytes_include_the_runtime_tree`,
  `footprint_labels_an_unpinned_runtime_and_never_counts_it_as_zero`,
  `footprint_blocks_the_does_not_start_row`,
  `footprint_refuses_a_changed_executable`. (AC14) *(Revision 1: R5.1
  withdraws the installed size of archive rows.)*
- [x] T5.2 `hold` as a module function over generic input and output, and a
  mode; the same teardown on EOF, SIGTERM and SIGINT. — Tests:
  `hold_refuses_missing_flags_and_protocol_3`,
  `hold_refuses_bytes_on_stdin_by_name`; under the budget it is T7.4. (AC15)
- [x] T5.3 `admit_v4` and `--seal`. Build the fixture run directory in the
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

- [x] T6.1 Add the `collect-v4` case and usage text to
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

- [x] T7.1 Add `tests/mail_compare/v4-smoke.json` (a protocol 4 draft with
  one small profile) and a shared proof prelude: scratch under
  `CARGO_TARGET_TMPDIR`, detection that panics with V8's exact text. — Test:
  `the_v4_smoke_fixture_validates_as_a_draft`.
- [x] T7.2 `v4_real_oom_at_two_gib` (`#[ignore]`, Linux). — Proof: spec V22.1.
- [x] T7.3 `v4_real_cpu_held_within_two` and
  `v4_real_setsid_escapee_removed`. — Proof: spec V22.2 and V22.3.
- [x] T7.4 `v4_real_hold_swarmail_under_budget`. — Proof: spec V22.4.
- [x] T7.5 `v4_real_cost_chunk_seals_and_admits`. — Proof: spec V22.5.
- [ ] T7.6 Run each proof where the seat stands, alone, with the bounded
  command of V22. Where there is no delegation, keep the output that shows
  V8's exact refusal for all five, and say in the result notes that the five
  host runs are owed (plan, Risk 1). Where there is, keep each passing
  output with `/proc/loadavg` before and after. Raw output goes to
  `target/mail-compare-v4-resources/receipts/`, summarized in the notes;
  nothing under `target/` is committed. — Proof: the receipts. (AC18)
  *(Not done: all five receipts at `2852bd2` show V8's refusal. Superseded
  by R9.2 and R9.3.)*
- [x] T7.7 The v4 section of `docs/evals/README.md` (V23). — Tests: the
  existing documentation and decision-inventory tests
  (`cargo test --locked --test store_contract_docs`) pass; a grep in the
  notes shows the refusal text, the draft route and Reading 4's sentence.
  (AC19)
- [x] T7.8 Prove what did not change. — Proof:
  `git diff e23a8ba --stat -- src evals Cargo.toml Cargo.lock deny.toml` is
  empty; `git diff e23a8ba --name-status -- docs/evals` lists only added
  files and `README.md`; `sha256sum docs/evals/protocol.json
  docs/evals/sources.lock.json` equals the frozen constants. (AC19)
- [x] T7.9 Run every gate of `docs/house-rules.md` § Gates on the final
  tree, `--locked`, the coverage gate and its selftest, the publish-guard
  proof, the seal proof and `cargo deny check licenses` included, and read
  each recipe's compile output for the policy-sweep warning. — Proof: each
  command and its exit status in the notes; a failure is reported with its
  output, not retried into silence. (AC20)

## R Revision 1 — after review 1

Ordered. The same rule as above: each group green and committed unsigned
before the next; never push. Nothing under `src/`, `evals/`, the lockfiles,
`deny.toml`, the active or historical records or `docs/decisions/` changes.
Finding names (`I0`, `C2`, `G1`, …) are plan.md § *Revision 1*'s. Tests go in
`tests/mail_compare/v4.rs`. A test that launches a target uses the candidate
`swarmail` on a fixture scope, as the existing leaf-launch tests do; a
protocol 1 to 3 path must behave as before in every task.

### R1 The chunk deadline reaches the work in flight (V2, V16; `I0` `C4` `G2` `C6`)

- [x] R1.1 `protocol.rs`: `CHUNK_KILL_MS = 600000`, `CHUNK_MARGIN_MS =
  30000`, and V2's inequality in place of `1..=540000`; the refusal prints
  the computed ceiling. Set `chunk_deadline_ms` to 480000 in
  `docs/evals/draft/protocol.v4.json` and bring
  `tests/mail_compare/v4-smoke.json` under its own ceiling. — Tests:
  `v4_chunk_deadline_at_its_ceiling_passes_and_one_above_refuses`,
  `v4_chunk_deadline_ceiling_moves_with_the_startup_and_cleanup_budgets`,
  `v4_chunk_deadline_refusal_names_the_ceiling`,
  `the_v4_draft_states_a_chunk_deadline_of_480000`; the existing
  `v4_resources_block_refuses_each_absent_or_different_value` still passes.
  (AC2, AC6)
- [x] R1.2 `runner.rs`: compute `chunk_end` once in `run_chunk` from the run
  clock's origin and pass it to the warmup, the measured instance and the
  send-and-verify window. At or after it: launch no instance, open no
  window, begin no send, listing or fidelity read. Remove the
  between-profiles check it replaces. — Tests:
  `a_chunk_whose_deadline_has_passed_launches_nothing_and_seals_with_exit_one`
  (`run_chunk` with a clock whose origin is older than the deadline: no
  launch receipt, every profile not measured with `chunk_deadline`, every
  planned window present once, `index.json` written),
  `a_send_window_past_chunk_end_is_all_unstarted_and_lists_nothing` (one row
  per index, reason `chunk deadline`; verification `incomplete` with that
  reason; `measure` window `chunk_deadline`). (AC21)
- [x] R1.3 `runner.rs`: one helper that races a future against `chunk_end`;
  wrap each warmup connect and send, the idle wait, each measured connect
  and send, each listing and each fidelity read. A cut connect keeps a
  failed connect's disposition, a cut send is `ambiguous`, both with a
  reason naming `chunk deadline`; a cut session is dropped, never reused;
  the workers' deadline is `min(send_deadline, chunk_end)` and the reason
  says which ended it. Launches and `shutdown` are not wrapped. — Tests:
  `a_send_in_flight_is_cancelled_at_chunk_end` (a test-owned listener that
  accepts and never answers; the call returns before one request deadline;
  the index has exactly one row naming `chunk deadline`; the rows count the
  profile's messages exactly),
  `a_warmup_cut_by_chunk_end_launches_no_measured_instance`,
  `a_fidelity_read_cut_by_chunk_end_is_unread_not_a_mismatch` (visible rows
  without `fidelity_ok`; verification `incomplete`),
  `an_idle_wait_cut_by_chunk_end_keeps_its_counters_as_chunk_deadline`,
  `the_profiles_own_send_deadline_still_writes_send_deadline`,
  `a_protocol_3_run_has_no_chunk_end` (its rows for a slow listener are what
  they were). (AC21)
- [x] R1.4 `runner.rs`: `skip_profile` becomes the one "not measured" path:
  it takes the outcome, the reason (`chunk deadline`, `oom killed`,
  `target exited`) and the launch name (`none` when nothing was launched),
  writes the send rows, the verification row and the three never-opened
  windows (R3.1's shape). The chunk exits 1 when any window is not
  `measured`. — Tests: `a_not_measured_profile_writes_its_rows_for_each_cause`
  (three causes; rows, reasons, windows without counters),
  `the_chunk_goes_on_after_a_profile_that_was_not_measured`; rewrite
  `a_profile_past_the_chunk_deadline_is_unstarted_rows_and_a_chunk_deadline_window`
  to the new shape. (AC21, AC22)

### R2 A target that dies before readiness is measured (V15, V16; `G1` `C8`)

- [x] R2.1 `targets.rs`, `resources.rs`: the leaf teardown reads `oom_kill`
  from `memory.events` beside `memory.peak`, before the `rmdir`;
  `CleanupFacts` gains `oom_kills` and `exited_early` (optional, absent
  without a leaf); `LaunchError::NotReady` keeps the launch facts of a leaf
  launch. — Tests: `a_leaf_cleanup_receipt_carries_oom_kills_and_exited_early`
  (fixture leaf with `oom_kill 1`; a leader that exited before the
  teardown; a leader the driver had to stop),
  `a_not_ready_leaf_launch_keeps_its_launch_receipt`; the existing
  `v3_launch_and_cleanup_receipts_serialize_byte_for_byte` passes unchanged.
  (AC11)
- [x] R2.2 `runner.rs`: one pure function from the cleanup receipt (and
  whether a launch receipt exists) to the outcome of V16's table. — Test:
  `a_not_ready_launch_is_classified_from_its_receipts` (`oom_kills` ≥ 1
  with either `exited_early`; `exited_early` alone; neither; no launch
  receipt; an unproved cleanup with each of the above). (AC22)
- [x] R2.3 `runner.rs`: the warmup and the measured instance call it, push
  both receipts, and hand a target failure to R1.4's path instead of
  returning an infrastructure error. — Tests:
  `a_chunk_whose_target_exits_at_once_seals_with_exit_one_and_goes_on`
  (`run_chunk` on a fixture scope with a program that exits at once as the
  candidate: every profile `target_exited`, reason `target exited`, as many
  proved cleanups as launch receipts, the next profile attempted),
  `a_join_mismatch_is_still_exit_two_and_unsealed`,
  `a_protocol_3_target_that_exits_at_once_is_still_infrastructure`. (AC22)

### R3 No observation, no number (V13, V17; `C2` `G3` `C3` `I4`)

- [x] R3.1 `records.rs`, `runner.rs`: the window's counters, `affinity_held`
  and `scratch_bytes` become optional together; one constructor for an
  opened window and one for a never-opened one; the reader refuses a
  `measured` window without counters, a window with some counters and not
  others, and a `sample` row naming launch `none`. The row version stays 1.
  — Tests: `a_never_opened_window_serializes_null_counters_and_never_zero`,
  `the_reader_refuses_a_measured_window_without_counters`,
  `the_reader_refuses_a_window_with_partial_counters`,
  `the_reader_refuses_a_sample_of_launch_none`; the existing duplicate,
  unplanned and missing window tests pass. (AC10)
- [x] R3.2 `runner.rs`: read affinity from `/proc` at the end of `ready`,
  `idle_ready` and `measure` through one function that gives `None` for an
  empty task list; remove both literal `true`s. — Tests:
  `affinity_is_read_at_the_end_of_each_window_kind` (fixture `/proc` tree:
  all tasks on the pair, one task differing, no task; for each of the three
  kinds), and the existing `affinity_held_is_false_when_any_task_differs`.
  (AC10)
- [x] R3.3 `stats.rs`, `report.rs`: per-repetition figures are optional and
  follow V17's table; `spread` carries `n`; `mean` and the peaks lose their
  zero defaults; the idle and startup cells carry `ranked` and `shortfalls`
  from the function the cost cells use. — Tests:
  `a_repetition_that_was_not_measured_has_null_figures`,
  `one_of_three_not_measured_gives_a_median_over_two_with_n_two`,
  `three_not_measured_give_null_figures_and_no_zero` (walks the report JSON:
  no number 0 under the cost, idle, startup and headline keys of that cell),
  `idle_and_startup_cells_are_unranked_and_name_the_window_outcome` (one
  case each for `timeout`, `oom`, `target_exited`),
  `a_target_that_exited_in_its_idle_window_has_no_idle_figure`,
  `an_opened_oom_killed_measure_window_keeps_its_cpu_and_peak_unranked`;
  extend `median_min_max_over_one_two_and_three_values` with `n`. AC13's
  existing tests pass with their expected values unchanged. (AC13, AC23)
- [x] R3.4 `report.rs`: `timeout` from a `chunk_deadline` window, a send row
  `unstarted` at a deadline, or a verification the chunk deadline ended;
  `lost` not named from such a verification; rows `unstarted` because the
  target failed before readiness name that failure; a visible row without
  `fidelity_ok` makes the repetition inexact. — Tests:
  `a_verification_cut_by_the_chunk_deadline_names_timeout_not_lost`,
  `unstarted_rows_of_a_dead_target_do_not_name_timeout`,
  `an_unread_fidelity_is_not_exact_and_not_a_fidelity_mismatch`; the
  existing one-case-per-shortfall test passes. (AC23)

### R4 Validators and admission (V4, V6, V20; `I2` `I3` `G5` `I7` `I8` `Gi`)

- [x] R4.1 `protocol.rs`, `runner.rs`: protocol 4 refuses a `null` Swarmail
  cap by name, and a protocol 4 launch spec for Swarmail is an error without
  a cap, never `--max-per-inbox 0`. — Tests:
  `v4_refuses_a_null_swarmail_cap`,
  `a_v4_swarmail_spec_is_never_built_uncapped`; protocol 3's Swarmail spec is
  unchanged. (AC4)
- [x] R4.2 `protocol.rs`: in lock 5 the cap flag occurs exactly once in a
  launch, in any form. — Test: `lock_v5_refuses_a_second_cap_flag`
  (placeholder then literal, as an argv pair, as `flag=` and as `env:`).
  (AC4)
- [x] R4.3 `report.rs`: `admit_v4` requires `candidate_dirty: false` on
  every chunk, one executable digest per target across chunks, as many
  proved cleanup receipts as launch receipts per chunk, and refuses an empty
  plan by name before it indexes anything; the sealed manifest carries
  `protocol_status`. — Tests: `admit_v4_refuses_a_later_dirty_chunk`,
  `admit_v4_refuses_a_target_whose_digest_differs_between_chunks`,
  `admit_v4_refuses_a_cleanup_without_its_launch_receipt`,
  `admit_v4_refuses_an_empty_plan_without_panicking`,
  `the_sealed_manifest_names_a_draft_as_a_draft`. (AC16)
- [x] R4.4 Tests only: every absent-field refusal test of the v4 resources
  block and of `clients_sha256` asserts that the refusal text names the
  field, not only that validation failed. — Proof: the changed assertions in
  `v4_resources_block_refuses_each_absent_or_different_value` and its
  neighbours; each fails when the named field is swapped for another.
  (AC2, AC5)

### R5 `footprint` and `hold` (V18, V19; `G4` `C10` `C7`)

- [x] R5.1 `resources.rs`: for a row whose artifact is an archive,
  `installed_bytes` is `null`, `installed_basis` is
  `owed: the extracted archive is not walked`, and `executable_bytes` is the
  digest-checked executable's size; executable rows keep their sum. — Tests:
  `footprint_owes_the_installed_size_of_an_archive_row`,
  `footprint_never_publishes_an_executable_alone_as_installed` (over every
  row of a fixture with both artifact kinds);
  `footprint_installed_bytes_include_the_runtime_tree` passes for an
  executable row. (AC14)
- [x] R5.2 `runner.rs`: `hold` prints each endpoint as the `host:port` the
  target listens on, the POP3 listener's included. — Tests:
  `hold_prints_each_endpoint_as_the_address_it_listens_on` (a launch without
  a leaf; a connect to each printed address succeeds; a fixture with a POP3
  listener prints its address, not a word),
  `a_hold_ended_by_its_interrupt_tears_down_and_prints_a_proved_receipt`
  (the interrupt future resolves after the endpoints line; no leaf, no
  signal). (AC15)

### R6 `collect-v4` (V21; `I1` `C5` `I5` `I6` `I8` `T b`)

Bash 3.2 syntax only. The rig's fake tools gain what each test needs: a
`systemctl` scripted per call, a `sleep` that returns at once, an `nproc`
that answers `--all`, a `cargo` that logs, and a fake driver that can write
a marker and wait.

- [x] R6.1 After a chunk, poll `systemctl --user is-active` until inactive,
  for at most 10 s, before judging the scope. — Tests:
  `collect_v4_waits_for_a_scope_that_is_active_once_then_inactive` (the
  chunk is clean and the run goes on),
  `collect_v4_stops_a_scope_that_stays_active_and_counts_two` (the existing
  test, now through the wait; the kill and stop are logged after it). (AC17)
- [x] R6.2 Write `reproduce.txt` after the last chunk, before the seal. —
  Tests: `collect_v4_writes_reproduce_txt_only_after_the_last_chunk` (absent
  when the fake driver of chunk 1 runs),
  `an_incomplete_collect_v4_has_no_reproduce_txt`. (AC17)
- [x] R6.3 A trap on INT, TERM and HUP: during a chunk, SIGTERM to the
  chunk's `timeout`, wait until it is reaped, judge the scope, write the
  ledger entry with exit 2, start no later chunk, exit 2; outside a chunk,
  exit 2. The chunk runs in the background and is `wait`ed. — Test:
  `collect_v4_on_sigterm_ends_the_chunk_writes_its_ledger_and_exits_two`
  (the test waits, with a deadline, for the fake driver's marker, then
  signals the wrapper: ledger exit 2, one ledger file, no `index.json`, no
  `reproduce.txt`, the fake driver gone). (AC17)
- [x] R6.4 One bound prefix, `timeout --kill-after=30 600 taskset -c
  0-<n-1>`, on both `cargo` builds, `check`, `footprint`, every chunk and
  both admissions; refuse, before building, a shell whose `nproc` differs
  from `nproc --all`. — Tests:
  `collect_v4_runs_every_process_under_the_bound` (from the log: builds,
  `check`, `footprint`, a chunk, `--seal`, verify),
  `collect_v4_refuses_a_restricted_shell_by_name`,
  `collect_v4_chunk_timeout_is_chunk_kill_ms` (the logged `600` against the
  constant in `protocol.rs`). (AC17)
- [x] R6.5 Tests only, the cases AC17 lacked: —
  `collect_v4_counts_exit_three_and_a_kill_as_two` (3 and 137),
  `collect_v4_a_timeout_with_a_live_scope_is_killed_and_counted`,
  `collect_v4_a_seal_followed_by_a_refused_verify_exits_two`. (AC17)
- [x] R6.6 The v4 rig owns its git environment: the fixture's `git` calls
  and the wrapper it runs get `GIT_CONFIG_GLOBAL=/dev/null`,
  `GIT_CONFIG_NOSYSTEM=1` and no inherited `GIT_CONFIG_COUNT`,
  `GIT_CONFIG_KEY_*` or `GIT_CONFIG_VALUE_*`. — Test:
  `the_v4_rig_ignores_an_ambient_git_config_count` (the rig is built with
  `GIT_CONFIG_COUNT=1` set and no key; `git init` and the dry run succeed).

### R7 The draft's text (V7; `G6` `I8`)

- [x] R7.1 `docs/evals/draft/protocol.v4.json`: add two `unresolved`
  entries, one for GreenMail's REST read path (story B, 0132 acceptance
  answer 8; this draft reads POP3) and one for `wait_operation_ms` (story C
  restores it with its wait profiles); make the claim's sentence say "in
  this draft" for POP3. — Tests:
  `the_v4_draft_names_greenmail_rest_and_the_wait_budget_as_unresolved`;
  `the_v4_draft_differs_from_v3_only_where_0132_says` and
  `the_v4_draft_validates_and_is_not_ready_for_collection` pass. (AC6)

### R8 The README (V23; `C9` `Gi` `C8`)

- [x] R8.1 `docs/evals/README.md`, the v4 section: the sentences V23 lists
  for revision 1, and the **Owed** paragraph replaced by AC18b's statement
  (the host run gates the merge). — Tests:
  `cargo test --locked --test store_contract_docs` passes; a test
  `the_v4_readme_states_the_revised_sentences` greps the section for
  `siblings_outside_set`, `null`, `owed: the extracted archive`, "alive but
  not ready" and "frozen", and for the absence of "can never be admitted".
  (AC19)

### R9 Proofs and gates (V22; `C1`, `I` on AC18)

- [x] R9.1 Update the five proofs to V22's revised list: proof 1 asserts the
  teardown's `oom_kills`; proof 4 adds the hold ended by its interrupt;
  proof 5 asserts `affinity_held: true` on every window and adds the chunk
  whose candidate exits at once (`/bin/false`). Each still fails with V8's
  exact text without delegation. — Test:
  `the_five_real_proofs_are_ignored_and_linux_only` (reads the test file:
  the five names, `#[ignore]`, the `cfg`), and each proof's own run in
  R9.2. (AC18a)
- [x] R9.2 Run each proof where the seat stands, alone, with V22's command.
  Without delegation, keep the output showing V8's exact refusal for all
  five, and state in the result notes, in these words, that **AC18b is
  open and the story is not complete**. With delegation, keep each passing
  output with `/proc/loadavg` before and after. Raw output goes under
  `target/mail-compare-v4-resources/receipts/`; nothing under `target/` is
  committed. — Proof: the receipts and the sentence in the notes. (AC18a)
- [ ] R9.3 **Dispatcher, on the host, after R1 to R9.2 are committed and
  the gates are green; not a seat's task.** Run the five proofs one at a
  time under V22's command (`timeout 600 taskset -c 0-31`, a delegated
  scope, the test binary and not `cargo` inside it), at the head to be
  merged, with `/proc/loadavg` before and after each, and attach the
  passing output to the PR. A proof that fails goes back as a finding with
  its output; the PR is not merged. — Proof: the five passing outputs on
  the PR. (AC18b)
- [ ] R9.4 *(Every gate ran and passed but the brokkr compile of the shelf
  recipes: the pinned brokkr is not installed in the implement seat's box and
  nothing here changes `.brokkr/`; the verify seat and CI run it.)* Prove what did not change, then run every gate of
  `docs/house-rules.md` § Gates on the final tree, `--locked`, the coverage
  gate and its selftest, the publish-guard proof, the seal proof and
  `cargo deny check licenses` included, and read each recipe's compile
  output for the policy-sweep warning. — Proof:
  `git diff e23a8ba --stat -- src evals Cargo.toml Cargo.lock deny.toml` is
  empty; `git diff e23a8ba --name-status -- docs/evals` lists only added
  files and `README.md`; the frozen sha256 of the two active records; each
  gate's command and exit status in the notes, a failure reported with its
  output. (AC19, AC20)
