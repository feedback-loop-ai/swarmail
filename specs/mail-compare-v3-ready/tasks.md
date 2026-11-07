# Tasks — mail comparison protocol v3, ready to collect

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md). Ordered; each step is
green before the next. Every real or capture command uses V11's bounded form
(`timeout 600 taskset -c 0-$(( $(nproc)/4 - 1 ))`, `--test-threads=1`,
`/proc/loadavg` before and after). Commit unsigned after each group, in the
repository's style; never push.

## T0 Receipt (V1)

- [ ] T0.1 `sha256sum -c SHA256SUMS` in every
  `.compare-bins/2026-11-07-supply/` subdirectory and at its top level, and
  re-digest the earlier `.compare-bins/2026-11-07*` artifacts the lock
  names. — Proof: the receipt table in the notes; any mismatch refuses that
  row.
- [ ] T0.2 Extract each archive needed (zips, tarballs, the jar, layers
  `f350948e…` and the executable's layer, the source tarball) into its own
  new directory under `target/compare-dl/`; list before extracting (no
  absolute or `..` member); scripts in `target/compare-tools/`; `python3 -I`
  only. — Proof: `git ls-files .compare-bins target` empty; listing output in
  the notes.
- [ ] T0.3 Prove the loader layer is the topmost manifest layer holding
  `lib/ld-musl-x86_64.so.1`, and that `musl/ld-musl-x86_64.so.1` equals that
  layer's member. — Proof: the layer listing and digests in the notes.

## T1 Vocabulary, each arm with its refusal tests (V3–V7, V10)

- [ ] T1.1 `TargetKind::MailslurperSourceBuild`, `ALL` → 9,
  `Provenance::SourceBuild`, `ProductScope::SourceBuild`, `official_home`,
  and the `targets.rs` match arms. — Tests: `parse` round-trips nine names;
  `mailslurper_source_build_is_neither_mailslurper_nor_the_shim`;
  `products_match` derives `SourceBuild`; nine-rotation and nine-capability
  validation.
- [ ] T1.2 `SourceBuild` validation (download `{home}/tree/{commit}`,
  observed build provenance on the tagged commit with `BuildEvidence`). —
  Tests: `source_build_refuses_unresolved_build_provenance`,
  `source_build_refuses_provenance_on_another_commit`,
  `source_build_refuses_missing_build_evidence`,
  `source_build_refuses_other_download`.
- [ ] T1.3 `REPORTED_VERSIONS`. — Tests:
  `reported_version_accepted_for_mailslurper_rows_at_release_1_15_0`,
  `reported_version_refused_for_another_target`,
  `reported_version_refused_for_another_tag`,
  `version_output_naming_neither_version_is_refused`.
- [ ] T1.4 `Protocol.does_not_start` (v3 only), `plan()` skip,
  `declared_competitors` omission, report row. — Tests:
  `does_not_start_refuses_undeclared_target`,
  `does_not_start_refuses_target_without_pin`,
  `does_not_start_refuses_cite_outside_pin`,
  `does_not_start_refused_on_v2`,
  `plan_yields_no_coordinate_for_a_does_not_start_target`,
  `report_row_for_does_not_start_has_label_and_no_number`.
- [ ] T1.5 `RUNTIMES` table, loader layer in captured manifest, launcher
  entry from the table, `RUNTIME_LICENSES` + `MIT`. — Tests:
  `runtime_refused_on_a_third_target`,
  `loader_runtime_refuses_missing_member`,
  `loader_runtime_refuses_layer_absent_from_manifest`,
  `runtime_license_off_the_list_is_refused`,
  `permissive_list_is_exactly_the_five`,
  `loader_launch_execs_the_pinned_loader_with_the_binary_as_artifact`;
  existing GreenMail runtime tests stay green.
- [ ] T1.6 `DigestBasis::RegistryChecksum`, `REGISTRY`, `DocKind::Checksum`,
  `release_agrees` asset skip under that basis only, `verify_files` checksum
  bytes. — Tests: `registry_download_outside_home_is_refused`,
  `registry_basis_on_another_target_is_refused`,
  `registry_checksum_document_disagreeing_is_refused`,
  `registry_pin_without_checksum_document_is_refused`,
  `registry_checksum_record_must_name_the_sha256_url`,
  `upstream_release_pin_with_unlisted_asset_is_still_refused`.
- [ ] T1.7 `DocKind::GitCommit` route in `tag_chain` and the git tree
  rebuild. — Tests: `tree_rebuild_matches_authored_git_mktree_fixture`
  (fixture bytes and id authored with `git mktree`, committed),
  `edited_tree_entry_fails_the_rebuild`,
  `git_commit_naming_another_tree_is_refused`,
  `git_commit_for_another_commit_is_refused`,
  `commit_attributed_blob_absent_from_tree_is_refused`; Mailpit/MailHog's
  by-commit route unchanged (`mailpit_mailhog_v3_documents_byte_unchanged`).
- [ ] T1.8 `Attribution::{Member, Output, Supplied}` with `SOURCE_ARCHIVES`.
  — Tests: `member_refuses_escaping_path`, `member_refuses_bad_digest`,
  `member_refuses_archive_outside_the_allowed_set`,
  `output_refuses_empty_argv`, `supplied_refuses_empty_record`,
  `supplied_refuses_record_without_document_digest`.
- [ ] T1.9 Diff audit. — Proof: `git diff 2c28b8b --
  benches/mail_compare/protocol.rs` reviewed line by line; the notes list
  each widened condition (only the five ruled arms) and confirm no `out.push`
  removed; `git diff 2c28b8b -- tests/` deletes no test function.

## T2 Pins and documents (V2–V7)

- [ ] T2.1 `mailslurper` (official): zip, member, lightweight chain
  (ref, commit, root tree), LICENSE/README (`Commit`), API source (`Member`
  of the source tarball), failure output (`Supplied`), launch and
  `scratch_file`/`no_cap_observed` storage. — Test:
  `current_lock_v4_validates_and_verifies`.
- [ ] T2.2 `mailslurper-source-build`: the cgo executable, same chain
  (byte-copied documents under its own directory), `BuildEvidence`
  (`Supplied`). — Same test.
- [ ] T2.3 `smtp4dev`: release artifact, lightweight chain, `LICENSE.md`
  (`Commit`, blob `ac0f7a32…`, `BSD-3-Clause`), setup/API documents
  (`Member`/`Output`), launch from the captured `--help`. — Same test.
- [ ] T2.4 `inbucket`: lightweight chain observed; LICENSE/README → `Commit`;
  nested files → `Member`; help → `Output`; release slice → `Api`. — Same
  test.
- [ ] T2.5 `greenmail`: registry pin, annotated chain with tag document,
  `Checksum` document, JRE runtime as captured in 0130's 2026-11-07
  addendum, openapi (`Member` of the jar), licence. — Same test.
- [ ] T2.6 `alkemio-mailslurper`: image pin (index, manifest documents;
  executable layer and member), loader runtime, licence/API/setup documents
  (`Image`), plain rendered config. If its licence is not permissive: S11.
  — Same test, plus `alkemio_name_is_not_mailslurper`.
- [ ] T2.7 Smoke fixture follows the pins; a test name that becomes false is
  renamed one-for-one and listed in the notes. — Test: the smoke fixture
  test.

## T3 Exploratory real pass (before the freeze; untimed, unmeasured)

- [ ] T3.1 Run every V9 real test and `real_member_and_output_documents_match`
  against the pinned lock, bounded, one at a time; fix launch facts from
  their output (each correction cited in 0130). S9/S10 stop the row. —
  Proof: command, exit, wall time and loadavg per test in the notes.

## T4 Freeze (V8)

- [ ] T4.1 Protocol v3: nine targets, nine rotations, nine capability and
  product records, `does_not_start`, claim statement additions,
  `status: frozen`, `unresolved: []`. — Proof: `cargo bench --locked --bench
  mail_compare -- check --protocol docs/evals/protocol.json --sources
  docs/evals/sources.lock.json` exits 0;
  `committed_protocol_v3_is_ready_for_collection` (calls `collection_ready`
  and `verify_files`).
- [ ] T4.2 Record both files' sha256. — Test:
  `frozen_v3_and_lock_v4_bytes_are_the_recorded_ones`; history byte tests
  stay green.

## T5 Recorded real proofs, against the frozen bytes (V9, V11)

- [ ] T5.1 `real_<t>_smtp_to_native_api_round_trip`,
  `real_<t>_hostile_round_trip`, `real_caps_hold_<t>` for `smtp4dev`,
  `inbucket`, `greenmail`, `mailslurper-source-build`,
  `alkemio-mailslurper`. — Proof: each passes; notes per V11.
- [ ] T5.2 `real_mailslurper_official_does_not_start`. — Proof: passes
  (non-zero exit, stub line, no listener, cleanup).
- [ ] T5.3 `real_member_and_output_documents_match`. — Proof: passes.
- [ ] T5.4 No run artefact. — Proof: `git status --porcelain docs/evals`
  empty after T5; no v3 run directory exists. Any failure here: S12.

## T6 Record (V12)

- [ ] T6.1 0130 addendum "2026-11-07 (operator rulings, pins and freeze)",
  additions only. — Proof: `git diff 2c28b8b -- docs/decisions/0130*` has
  no `-` lines; status line unchanged.
- [ ] T6.2 `docs/evals/README.md` v3 section. — Proof: names every row's
  labels and the open items; no comparison number (reviewed in the notes).

## T7 Gates and report (V13, V14)

- [ ] T7.1 Every house gate `--locked` (root and evals fmt/clippy, `cargo
  test --workspace`, candidate binary, evals tests with `SWARMAIL_BIN` from
  Cargo's artifact record, coverage gate and selftest, publish-guard and seal
  proofs, `cargo deny check licenses`); brokkr compile reported as verify's
  if absent. — Proof: each command's exit in the notes.
- [ ] T7.2 `git diff c95386e -- src/ Cargo.toml Cargo.lock evals/Cargo.lock
  deny.toml` empty. — Proof: the empty output in the notes.
- [ ] T7.3 Notes: criteria passed with outputs, loads, collection not run,
  PR held for the operator and why.

## R Review return (2026-11-07)

T0–T7 were delivered at `c5a784c`, and verify passed on that tree. They are
not redone, except that R8 reruns T5's receipts. Every R task leaves
`docs/evals/protocol.json` and `docs/evals/sources.lock.json` byte-unchanged:
`frozen_v3_and_lock_v4_bytes_are_the_recorded_ones` stays green after each
task, and so does `cargo bench --locked --bench mail_compare -- check
--protocol docs/evals/protocol.json --sources docs/evals/sources.lock.json`.
A task that would break either is S14. Load bounds are as in V11. Commit
unsigned after each group.

- [ ] R1 Condition (f) takes its closed shape. `collection_ready` skips the
  tagged-source gap only when `!t.provenance().has_tagged_source()`, where
  `Provenance::has_tagged_source()` is false only for `ThirdPartyImage`.
  The condition no longer depends on `pin.image.is_none()`.
  - Tests: `official_pin_still_needs_an_observed_tagged_source_but_an_image_pin_does_not`
    and `image_pin_refuses_observed_tagged_source_or_build_provenance` stay
    green.
  - Add `has_tagged_source_is_false_only_for_the_image_provenance`, which
    covers all nine targets.
- [ ] R2 Condition (g)'s remaining refusal. Add
  `skipped_asset_list_still_checks_tag_commitish_and_date`: a copy of the
  committed lock with one release-metadata field edited (tag, commitish,
  date) is refused, both for GreenMail (registry basis) and for
  `mailslurper-source-build`.
  - Test: the new test, plus
    `upstream_release_pin_with_unlisted_asset_is_still_refused` and V4's four
    `source_build_refuses_*` tests, all green.
- [ ] R3 `does_not_start_matches` refuses a cite whose attribution is not
  `Supplied` or `Output`. First confirm that the committed cite
  (`sources/mailslurper/mailslurper-official-failure.txt`) is `Supplied`.
  - Test: `does_not_start_cite_must_be_failure_evidence`, run over a copy of
    the committed protocol and lock whose cite is the row's
    `Commit`-attributed README: refused. The committed pair: `Ok`.
- [ ] R4 Labels and the seal.
  - `report::row_labels` gives smtp4dev
    `runtime: Some("self-contained .NET on host libraries (unpinned)")`.
  - `admit_run` compares `["throughput", "waits", "blocked", "labels"]` when
    the protocol is version 3, and `["throughput", "waits"]` otherwise.
  - Tests: `report_labels_name_every_unpinned_runtime` (the smtp4dev and
    source-build runtime labels) and
    `admit_run_refuses_a_v3_report_with_an_edited_blocked_row` (a dropped
    blocked row, and a blocked cell given a number: both refused; the
    untampered report admits). The existing `admit_run` tests stay green.
- [ ] R5 One launch owner, and the flags.
  - Add `Protocol::launch_order(rep)`. `runner::plan`, `runner::run` and
    `prepare` use it, and `grep -n 'does_not_start(' benches/` shows no other
    filter. The grep output goes in the notes.
  - Move `flags` and a pure `launch_specs(p, s, bins, runtimes)` from
    `benches/mail_compare.rs` into `runner.rs` without changing behaviour.
    `prepare` keeps its file and scratch I/O.
  - Tests:
    - `plan_yields_no_coordinate_for_a_does_not_start_target`, tightened to
      the exact count `(|targets| − 1) × per-target coordinates`;
    - `run_never_launches_a_does_not_start_target` (the candidate alone plus
      one `does_not_start` competitor with no spec; `Clean`, and the sealed
      report carries the blocked row; Linux-gated as the existing real smoke);
    - `flags_repeat_bin_and_runtime_only`;
    - `launch_specs_take_two_runtimes`;
    - `launch_specs_refuse_a_bin_for_a_does_not_start_target`;
    - `launch_specs_refuse_a_runtime_given_twice_for_one_target`.
  - Proof of the call path: one rerun of the wrapper smoke (`scripts/compare-mail.sh
    local`, the nine-target smoke fixture), V11-bounded, unmeasured. Its exit
    status is quoted. Exit 1 on the decoded-body fidelity cells is the
    expected, recorded outcome; any other failure is a defect.
- [ ] R6 Test honesty.
  - Replace `real_hostile`'s `eprintln!` for the decoded-body rows with
    assertions: `..leading dot` kept, `<br />` per newline. In
    `real_round_trip`'s multipart arm, assert HTML == `<p>html part</p>`.
  - Rename `eight_target_smoke_fixture_is_a_valid_draft_that_cannot_launch_yet`
    one-for-one to a true name (for example
    `eight_target_smoke_fixture_is_a_valid_draft_that_launches`).
  - Delete the `started.elapsed() < startup` assertion in
    `real_mailslurper_official_does_not_start`.
  - Proof: the touched real tests pass in R8. The rename is listed in the
    notes and in 0130's addendum.
- [ ] R7 Alkemio burst probe. Run `real_caps_hold_alkemio_mailslurper` once
  with Alkemio removed from `loses_under_burst`, as `timeout 600 taskset -c
  0-$(( $(nproc)/4 - 1 )) cargo test --locked --test mail_compare --
  --ignored --exact eight_targets::real_caps_hold_alkemio_mailslurper
  --test-threads=1`, with the loadavg before and after.
  - If 600/600 are listed, keep the change.
  - If not, restore the batching and record the listed count.
  - Proof: the command, exit status, wall time, loadavg and listed count in
    the notes and in R9's addendum.
- [ ] R8 Receipts at the final head. After R1–R7 are committed, rerun every
  V9 real test and `real_member_and_output_documents_match`, one at a time and
  V11-bounded, against the unchanged frozen bytes.
  - Proof: a table with each test's command, `git rev-parse HEAD`,
    `HEAD^{tree}`, wall time, exit status and loadavg. `git status --porcelain
    docs/evals` is empty, and no v3 run directory exists.
  - Any failure here is S12: report it, and never edit the frozen bytes.
- [ ] R9 The record (V12′).
  - Add 0130's addendum "2026-11-07 (review return)", additions only. It
    covers: nine; conditions (f), (g) and (h) with their derivations and tests,
    and the sentence superseding "for the operator" and the open sixth
    condition; the GreenMail REST correction of record; the smtp4dev runtime
    wording; the Alkemio cap basis from R7; the `server_search` sentence; the
    musl holder; R6's rename; R8's receipt table.
  - Append the 0091 §4 pointer line, add the note to the 0130 index row, fix
    the `wire.rs` comment, and update the README v3 section.
  - Proof: `git diff c5a784c -- docs/decisions/0130* docs/decisions/0091*`
    has no `-` line; both statuses are unchanged.
- [ ] R10 Gates and report.
  - Run every house gate `--locked`, as in T7.1, under `taskset`. Report the
    brokkr compile as verify's if `brokkr` is absent.
  - `git diff c95386e -- src/ Cargo.toml Cargo.lock evals/Cargo.lock
    deny.toml` is empty.
  - `git diff 2c28b8b -- benches/mail_compare/` is audited against V10's
    table, row by row, in the notes.
  - Proof: each command's exit status in the notes. The notes also say that
    collection was not run and that the PR is held for the operator.
