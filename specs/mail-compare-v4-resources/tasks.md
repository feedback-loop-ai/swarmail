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
Group R was the work after review 1. Each R task names the review finding
it answers (plan.md § *Revision 1* has the table).

**State after review 2 (2026-11-16).** Group R was built at head `c2041b9`;
the verify seat passed every gate there, and review 2 found security and
tests clean and AC1 to AC17, AC19 and AC21 to AC23 met as far as fixtures
show, with the residual findings plan.md § *Revision 2* lists. R1 to R9.2
are ticked on that basis and kept as the record. R9.3 and R9.4 are not
ticked and are superseded by S7.2 and S7.3: no real proof has passed on a
kernel, and the gates are run again on the final tree. **The work to do now
is group S**, at the end, in its order. Where group S replaces what an R
task built (R5.1's owed rule, R6.4's `600`), the S task says so.

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
  with `B=$(git merge-base HEAD origin/main)` (the merge base, not
  `e23a8ba`, once main is merged in), `git diff "$B" --stat -- src evals
  Cargo.toml Cargo.lock deny.toml` is empty; `git diff "$B" --name-status --
  docs/evals` lists only added
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

- [x] R5.1 *(Revision 2: this owed rule contradicted 0132 §4.5 and §5.6;
  S1.2 replaces it.)* `resources.rs`: for a row whose artifact is an archive,
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
- [x] R6.4 *(Revision 2: the `600` let a step run 630 s; S3.1 makes it
  `560`.)* One bound prefix, `timeout --kill-after=30 600 taskset -c
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
- [ ] R9.3 *(Not done; superseded by S7.2 at the revised head.)*
  **Dispatcher, on the host, after R1 to R9.2 are committed and
  the gates are green; not a seat's task.** Run the five proofs one at a
  time under V22's command (`timeout 600 taskset -c 0-31`, a delegated
  scope, the test binary and not `cargo` inside it), at the head to be
  merged, with `/proc/loadavg` before and after each, and attach the
  passing output to the PR. A proof that fails goes back as a finding with
  its output; the PR is not merged. — Proof: the five passing outputs on
  the PR. (AC18b)
- [ ] R9.4 *(Superseded by S7.3 on the final tree.)* *(Every gate ran and passed but the brokkr compile of the shelf
  recipes: the pinned brokkr is not installed in the implement seat's box and
  nothing here changes `.brokkr/`; the verify seat and CI run it.)* Prove what did not change, then run every gate of
  `docs/house-rules.md` § Gates on the final tree, `--locked`, the coverage
  gate and its selftest, the publish-guard proof, the seal proof and
  `cargo deny check licenses` included, and read each recipe's compile
  output for the policy-sweep warning. — Proof:
  with `B=$(git merge-base HEAD origin/main)` (the merge base, not
  `e23a8ba`, once main is merged in), `git diff "$B" --stat -- src evals
  Cargo.toml Cargo.lock deny.toml` is empty; `git diff "$B" --name-status --
  docs/evals` lists only added
  files and `README.md`; the frozen sha256 of the two active records; each
  gate's command and exit status in the notes, a failure reported with its
  output. (AC19, AC20)

## S Revision 2 — after review 2

Ordered. The same rule as above: each group green and committed unsigned
before the next; never push. Nothing under `src/`, `evals/`, the lockfiles,
`deny.toml`, the active or historical records or `docs/decisions/` changes,
and in this revision neither draft file and not the smoke fixture. Finding
names (`2G1`, `2C2`, `2I4a`, …) are review 2's, as plan.md § *Revision 2*
cites them. Tests go in `tests/mail_compare/v4.rs`. A protocol 1 to 3 path
must behave as before in every task. Where a task replaces a test of group
R, it says so; no other existing test changes its assertions.

### S1 Every row has an installed size, and the headline carries it (V17, V18, V20; `2G1`, `2I5.5`)

- [x] S1.1 `resources.rs`: one walk of a directory that returns the bytes
  and count of its regular files and the sha256 of its listing (one line
  `<sha256 of the file>  ./<relative path>` and a newline per file, sorted
  by the path's bytes); a symbolic link and a directory count for nothing
  and are not listed; a file name that is not UTF-8 or holds a newline or a
  backslash refuses by name. `tree_bytes` becomes this walk or calls it. —
  Tests: `a_tree_walk_counts_each_regular_file_once_and_no_link`,
  `a_tree_digest_equals_the_listing_the_test_builds_by_hand` (the test
  writes the lines itself from V18's format and hashes them; it does not
  call the function under test for the expected value),
  `a_tree_digest_changes_when_a_file_is_added_removed_or_changed`,
  `a_tree_walk_refuses_a_name_sha256sum_would_escape` (newline, backslash;
  the non-UTF-8 name on Linux only). (AC14)
- [x] S1.2 `resources.rs`, `footprint`: an archive row's installed size is
  the walk of its extraction root, which is `--bin`'s path with the pin's
  `member` path removed from its end; a `--bin` that does not end with the
  member refuses by name. Every walked directory is listed in
  `installed_trees` with `kind` (`runtime`, `archive`), `files`, `bytes`
  and `sha256`; a row that walks none has an empty list. Remove the
  `ArtifactKind::Archive` null arm, `executable_bytes` and the unused
  `parts` vector of host paths; `installed_basis` is fixed text naming the
  rule that applied. — Tests:
  `footprint_sizes_an_archive_row_from_its_extraction_root` (the member, two
  more files, a nested directory and a link: the sum, the count, the
  executable counted once), `footprint_lists_each_walked_tree_with_its_kind_and_digest`
  (an archive row and an executable row with a runtime),
  `footprint_refuses_a_bin_that_does_not_end_with_the_pinned_member`,
  `footprint_never_publishes_null_zero_owed_or_a_host_path` (walks every
  value of the document of a fixture with every artifact kind: no
  `installed_bytes` that is `null` or 0, no string containing `owed`, no key
  `executable_bytes`, no string containing the fixture's directory). This
  replaces `footprint_owes_the_installed_size_of_an_archive_row`;
  `footprint_never_publishes_an_executable_alone_as_installed` and
  `footprint_installed_bytes_include_the_runtime_tree` still pass. (AC14)
- [x] S1.3 `resources.rs`, `footprint`: after every row's executable and
  trees are known, refuse by name, giving both targets, when one target's
  walked tree contains another target's executable or walked tree, the
  candidate included; compare canonical paths. — Test:
  `footprint_refuses_two_targets_that_share_a_tree` (another competitor's
  executable inside an archive root; the candidate inside it; a runtime
  tree inside an archive root; the same directory reached through a
  symbolic link; and two separate directories pass). (AC14)
- [x] S1.4 `report.rs`: `Headline.installed` in the run-level report, one
  entry per launched target (`installed_bytes`, `installed_files`,
  `installed_basis`) copied from the run's `footprint.json`, the
  does-not-start row blocked as elsewhere; absent from a chunk's report.
  `admit_v4` requires the file's protocol id to be the run's and one row
  per launched target with a positive integer `installed_bytes`, and
  refuses naming the target or the protocol. — Tests:
  `the_run_headline_carries_each_targets_installed_size`,
  `a_chunk_report_has_no_installed_key`,
  `admit_v4_refuses_a_footprint_without_a_launched_targets_row`,
  `admit_v4_refuses_a_null_or_zero_installed_size` (both, each naming the
  target), `admit_v4_refuses_a_footprint_of_another_protocol`,
  `admit_v4_names_the_path_of_an_edited_installed_cell`,
  `a_footprint_changed_after_the_seal_fails_the_verify`; the fixture run
  directory's `footprint.json` gains real rows, and
  `a_complete_fixture_run_seals_and_then_verifies` still passes. (AC16)

### S2 `CHUNK_KILL_MS` is the instant of the SIGTERM (V2; `2G2`, `2C4`)

- [x] S2.1 `protocol.rs`: `CHUNK_KILL_MS = 560000`, documented as the
  instant the wrapper's `timeout` sends SIGTERM; V2's inequality unchanged.
  The draft and the smoke fixture are not edited: both state 480000 with
  30 s and 10 s budgets, which is the new ceiling. — Tests:
  `v4_chunk_deadline_ceiling_is_480000_for_the_draft_budgets`,
  `the_v4_draft_and_the_smoke_fixture_sit_at_their_ceiling_and_validate`;
  the three ceiling tests of R1.1 pass with their expected values derived
  from the constant, and any test fixture that built a protocol above the
  new ceiling is brought under it. Proof beside the tests:
  `git diff c2041b9 -- docs/evals/draft tests/mail_compare/v4-smoke.json`
  is empty. (AC2, AC6)

### S3 `collect-v4` (V21; `2G2`, `2G3`, `2C3`, `2G7`–`2G9`, `2I4a`–`c`)

Bash 3.2 syntax only. The rig's fakes gain what each test needs: a
`systemctl` scripted per call that can print a state word, print nothing
with a non-zero status, or write a marker and wait; a `cargo` and a driver
that can write a marker and wait in any mode; a `git` fixture with a
`main`, a side branch and, in one test, a `refs/remotes/origin/main`.

- [x] S3.1 One prefix, `timeout --kill-after=30 560 taskset -c 0-<n-1>`,
  and one helper that starts a command under it in the background, records
  it as the running step and waits until it is reaped. Both `cargo` builds,
  both `targets` calls, `check`, `footprint`, every chunk and both
  admissions go through it. A step whose output the wrapper reads (the
  driver's path from `cargo bench --no-run`, the competitor list, the plan)
  writes to a file that is read afterwards, never through `$(…)`. — Tests:
  `collect_v4_runs_every_process_under_the_bound` (extended: both builds,
  both `targets` calls, `check`, `footprint`, a chunk, `--seal`, verify,
  each under `timeout --kill-after=30 560 taskset`),
  `collect_v4_chunk_timeout_is_chunk_kill_ms` (the logged `560` against the
  constant), `no_collect_v4_timeout_line_carries_600`; every existing
  assertion on `timeout --kill-after=30 600` moves to `560`. (AC17)
- [x] S3.2 Every `systemctl` call has its own `timeout`: 1 for
  `is-active`, 3 for `kill`, 3 for `stop`. — Tests:
  `collect_v4_bounds_every_systemctl_call` (from the log: no `systemctl`
  line of a run without its `timeout`, and the three numbers),
  `collect_v4_sums_stay_within_ten_minutes` (reads the prefix's two numbers
  and the three `systemctl` bounds from the log and the poll count and
  interval from the script, and requires 560 + 30 + 1 + 3 + 3 ≤ 600 and
  560 + 20 × 1.5 + 3 + 3 ≤ 600 as computed from them). (AC17)
- [x] S3.3 The scope is judged from the word `is-active` prints. Only
  `inactive` and `failed` are clean; a non-zero status alone decides
  nothing. After a sealed chunk: at most twenty queries half a second
  apart, clean at the first of the two words, any other answer asked again,
  unclean after the twentieth (kill, stop, the chunk counts as 2). After a
  failed chunk: one query and no wait; unless it answers one of the two
  words, kill and stop, ledgered unclean. — Tests:
  `collect_v4_waits_for_a_scope_that_is_active_once_then_inactive` (kept),
  `collect_v4_reads_a_failed_scope_as_gone`,
  `collect_v4_asks_again_after_a_query_that_says_nothing`,
  `collect_v4_never_calls_an_unknown_scope_clean` (three forms, each twenty
  times: `active`; no output with a non-zero status; an unknown word; each
  ends killed, stopped, `scope_clean` false, exit 2),
  `collect_v4_does_not_wait_for_a_failed_chunks_scope` (a 124: exactly one
  query before the `kill` and the `stop`; no `kill` when it answers
  `inactive`); `collect_v4_stops_a_scope_that_stays_active_and_counts_two`
  and `collect_v4_a_timeout_with_a_live_scope_is_killed_and_counted` still
  pass. (AC17)
- [x] S3.4 Signals. The handler only records the interrupt and sends
  SIGTERM to the running step's `timeout`; it exits 2 itself only when no
  step is running and no chunk is owned. A chunk is owned from before it
  starts until its ledger entry is written: its pid and unit are not
  cleared when it is reaped. The main flow reads the interrupt after every
  step and, for a chunk, again just before the ledger entry: an interrupted
  chunk is judged as a failed one, ledgered with exit 2, and the mode exits
  2; an interrupted step of another kind ends the mode with exit 2, naming
  the interrupt; no step starts once an interrupt is recorded. — Tests:
  `collect_v4_on_sigterm_ends_the_chunk_writes_its_ledger_and_exits_two`
  (kept), `collect_v4_on_sigterm_during_a_build_exits_two_at_once`,
  `collect_v4_on_sigterm_during_check_footprint_and_admission_exits_two_at_once`
  (one case each; the fake writes a marker and would run on; the test waits
  for the marker with a deadline, signals, and requires exit 2 before the
  fake's own duration has passed and the fake gone afterwards),
  `collect_v4_on_sigterm_during_the_scope_judgement_still_writes_the_ledger`
  (the fake `systemctl` writes a marker and waits; after the signal the
  chunk's ledger file exists with exit 2, a scope that was not gone is
  killed and stopped, and no later chunk started). No test sleeps to place
  the signal. (AC17)
- [x] S3.5 The candidate is on `main`: for a real run, refuse unless `HEAD`
  is an ancestor of, or equal to, `refs/heads/main`, and of
  `refs/remotes/origin/main` where it exists; refuse by name when neither
  exists; fetch nothing; `--dry-run` skips it. The refusal for a dirty tree
  keeps its text. — Tests: `collect_v4_refuses_a_clean_branch_that_is_not_main`,
  `collect_v4_refuses_a_branch_with_main_merged_in`,
  `collect_v4_accepts_an_earlier_commit_of_main_and_its_tip` (detached
  `HEAD` in both), `collect_v4_refuses_a_head_origin_main_does_not_have`
  (naming `refs/remotes/origin/main`),
  `collect_v4_refuses_when_no_main_ref_exists`,
  `collect_v4_dry_run_does_not_check_main`; the rig's default fixture is on
  `main`, so every other test is unchanged. (AC17)

### S4 The chunk and the join (V10, V13, V16; `2C2`, `2G6`, `2I2`, `2I5.1`, `2G11`, `2I5.6`)

- [x] S4.1 `runner.rs`: at or after `chunk_end` no `QUIT` is begun and the
  session is dropped; a `QUIT` in flight is cut by the existing helper.
  Before `chunk_end`, and on every protocol 1 to 3 path, a session ends as
  it does today. — Tests, against a test-owned listener that records every
  line it receives:
  `a_reused_session_past_chunk_end_is_dropped_without_quit`,
  `a_fresh_session_past_chunk_end_sends_no_quit_and_keeps_its_accepted_row`,
  `a_quit_in_flight_is_cut_at_chunk_end` (a listener that never answers
  `QUIT`; the call returns before one request deadline has passed),
  `before_chunk_end_a_session_still_ends_with_quit`,
  `a_protocol_3_session_still_ends_with_quit`. Each places `chunk_end` by
  construction, as R1.3's tests do. (AC21)
- [x] S4.2 `runner.rs`: one function gives a never-opened window its
  outcome in V13's order: `oom_killed` when the instance's leaf shows
  `oom_kill` ≥ 1, else `target_exited` when the leader has exited, else the
  cause in hand. The cut, never-opened `measure` window uses it. — Test:
  `a_never_opened_window_keeps_the_order_of_outcomes` (fixture leaf with
  `oom_kill 1` and a live leader; with `oom_kill 1` and an exited leader;
  with `oom_kill 0` and an exited leader; with neither: `oom_killed`,
  `oom_killed`, `target_exited`, `chunk_deadline`). (AC10)
- [x] S4.3 `resources.rs`, `targets.rs`: the join comparison requires the
  whole cgroup path of the `0::` line to equal the path of the leaf the
  driver made; the receipt's `joined.cgroup` stays the leaf's name. — Tests:
  `a_join_under_another_parent_with_the_leafs_name_is_a_mismatch`,
  `a_join_at_the_leafs_own_path_passes_and_the_receipt_keeps_the_name`;
  `a_join_read_back_mismatch_refuses_the_launch` and
  `a_target_that_exits_before_its_join_is_read_is_still_target_exited`
  still pass. On a kernel it is real proofs 2 to 4. (AC9)

### S5 Admission and the idle cell (V15, V17, V20; `2G5`, `2I3`, `2G10`, `2G11`, `2I5.4`)

- [x] S5.1 `report.rs`: `admit_chunk` no longer refuses a chunk without
  launch receipts; the pairing and every other check stay. — Tests:
  `admit_v4_admits_a_sealed_chunk_that_launched_nothing` (a fixture run in
  which one chunk is the output of
  `a_chunk_whose_deadline_has_passed_launches_nothing_and_seals_with_exit_one`'s
  shape: it seals and verifies),
  `a_run_with_a_zero_launch_chunk_still_refuses_a_missing_cleanup_elsewhere`.
  (AC16)
- [x] S5.2 `targets.rs`, `report.rs`: the cleanup receipt of a leaf launch
  carries `leaf`, the name in its launch receipt, absent without a leaf;
  admission requires each cleanup to name the leaf of the launch it is
  paired with and no leaf to be named twice in a chunk. — Tests:
  `a_leaf_cleanup_receipt_names_its_leaf`,
  `admit_v4_refuses_a_cleanup_that_names_another_launchs_leaf`,
  `admit_v4_refuses_two_cleanups_of_one_leaf`,
  `admit_v4_refuses_a_leaf_cleanup_without_a_leaf_name`;
  `v3_launch_and_cleanup_receipts_serialize_byte_for_byte` passes
  unchanged. (AC11, AC16)
- [x] S5.3 `report.rs`: an idle cell's `ranked` requires current memory,
  `anon` and the scratch footprint on every repetition; a startup cell's
  rule is unchanged. — Test: `an_idle_cell_ranks_only_on_all_three_values`
  (one repetition lacking each of the three in turn, and one complete
  cell); AC23's existing tests pass unchanged. (AC23)

### S6 The README (V23; `2G1`–`2G3`)

- [x] S6.1 `docs/evals/README.md`, the v4 section: V23's revision 2
  sentences. The paragraph that says an archive row's installed size is
  owed goes; in its place, that every row has an installed size and the
  headline carries it, how to extract an archive for it, what the tree
  digest is with the command that reproduces it, and Reading 9's limit.
  `timeout --kill-after=30 560` and V21's three sums replace each `600`
  the section states for `collect-v4`; the candidate rule names `main`'s
  history and the refs read; a scope is clean only on `inactive` or
  `failed`. The proofs' command keeps `timeout 600`. — Tests:
  `cargo test --locked --test store_contract_docs` passes;
  `the_v4_readme_states_the_revised_sentences` is updated to require
  `installed_trees`, `sha256sum`, `--kill-after=30 560`, `merge-base`
  and "`inactive` or `failed`", and the absence of
  `owed: the extracted archive` and of `--kill-after=30 600`. (AC19)

### S7 Proofs and gates (V22; `2C1`, `2G4`, `2I1`)

- [x] S7.1 Bring the proofs to this revision and run them where the seat
  stands. Proof 5 also asserts that each cleanup receipt names the leaf of
  its launch receipt; proofs 2 to 4 exercise S4.3 as they are.
  `the_five_real_proofs_are_ignored_and_linux_only` passes. Run each proof
  alone with V22's command. Without delegation, keep the output showing
  V8's exact refusal for all five and write in the result notes, in these
  words, **AC18b is open and the story is not complete**, whatever the
  typed result is. With delegation, keep each passing output with
  `/proc/loadavg` before and after. Raw output goes under
  `target/mail-compare-v4-resources/receipts/`; nothing under `target/` is
  committed. — Proof: the receipts at this revision's head and the sentence
  in the notes. (AC18a)
- [ ] S7.2 **Dispatcher, on the host, after S1 to S7.1 are committed and
  the gates are green; not a seat's task.** Run the five proofs one at a
  time under V22's command (`timeout 600 taskset -c 0-31`, a delegated
  scope, the test binary and not `cargo` inside it), at the head to be
  merged, with `/proc/loadavg` before and after each, and attach the
  passing output to the PR. Output from any earlier head does not count:
  this revision changes code the proofs exercise. A proof that fails goes
  back as a finding with its output; the PR is not merged. — Proof: the
  five passing outputs on the PR. (AC18b)
- [ ] S7.3 Prove what did not change, then run every gate of
  `docs/house-rules.md` § Gates on the final tree, `--locked`, the coverage
  gate and its selftest, the publish-guard proof, the seal proof and
  `cargo deny check licenses` included, and read each recipe's compile
  output for the policy-sweep warning. A gate the seat's box cannot run is
  named with the reason and left to the verify seat; it is not reported as
  passed. — Proof:
  with `B=$(git merge-base HEAD origin/main)` (the merge base, not
  `e23a8ba`, once main is merged in), `git diff "$B" --stat -- src evals
  Cargo.toml Cargo.lock deny.toml` is empty; `git diff "$B" --name-status --
  docs/evals` lists only added
  files and `README.md`; the frozen sha256 of the two active records; each
  gate's command and exit status in the notes, a failure reported with its
  output. (AC19, AC20)
