# Tasks — eight-target mail comparison (#340)

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md) · Decision: [0130](../../docs/decisions/0130-eight-target-mail-comparison.md) (proposed)

Work in order. Each task names the test that proves it. "Real" tests are
`#[ignore]` and **fail** without their variables; they never skip. Every
measurement uses the bounded form shown in plan.md's quickstart, which caps it
at `nproc/4` CPUs and 600 s and records `/proc/loadavg` before and after.
Commit unsigned in small commits, and never push. If a stop trigger S1–S6
fires, stop that row and report it; the other rows proceed.

## Phase 0 — prerequisites and records

- [ ] **T0 Locate the bytes (S6).**
  - Check whether `swarmail-landing/compare-bins/2026-11-07*` is visible. Copy
    each artifact into its own fresh `mktemp -d` under `target/compare-dl/`
    (not `/tmp`) and verify its sha256 against the dispatcher's recorded
    digest before any use.
  - Record the full Alkemio index digest from the labelled source.
  - Note which rows can be pinned and which stay pending.
  - *Proof:* a digest receipt listing path, expected digest and observed
    digest, quoted in the delivery notes. A missing directory is reported, not
    worked around.
- [ ] **T1 Decision records (AC1).**
  - Add 0130's row to `docs/decisions/README.md`.
  - Add a dated 2026-11-07 pointer addendum to 0091 (§4, target count) and to
    0093 (§1, lightweight, image and runtime pins). Each cites #340's
    paraphrase "#163 approved and widened" and says it is a paraphrase.
  - *Proof:* `git diff c95386e -- docs/decisions/0091* docs/decisions/0093*`
    shows only `+` lines. The decision-index inventory gate passes.
- [ ] **T2 History bytes (AC2).**
  - Copy `docs/evals/protocol.json` and `docs/evals/sources.lock.json` to
    `docs/evals/history/<first-commit-date>-protocol.v2.json` and
    `…-sources.lock.v3.json`. Get each date from `git log --diff-filter=A`.
  - *Proof:* the test `history_v2_v3_bytes_are_the_c95386e_bytes` compares
    sha256 against constants taken at `c95386e`.

## Phase 1 — records and validators

- [ ] **T3 Targets (R1).** Add the five variants, the `V2` and `ALL` sets and
  `provenance()`. Move every validator off `official_repository()`'s `Option`
  meaning.
  - *Proof:* `target_names_round_trip_and_unknown_refused`.
  - *Proof:* `v2_protocol_and_v3_lock_refuse_each_new_target` (five cases).
  - *Proof:* `alkemio_name_is_not_mailslurper` (serde names, source dirs and
    labels are distinct; nothing matches `tests/mailslurper.rs`).
- [ ] **T4 Lock version gate and pairing (R2, AC3).**
  - Accept `lock_version` 3 or 4. The v3 gate refuses every v4 field, and the
    v4 gate requires the fields its pin kind needs.
  - Enforce the pairing protocol v2 ↔ lock v3 and protocol v3 ↔ lock v4.
  - *Proof:* `lock_v3_refuses_each_v4_field`, one case per field: `ref_kind`,
    `source.image`, `runtime`, `listeners`, `launch_env`, `config_file`,
    `pending`, `checked_members`.
  - *Proof:* `lock_v4_requires_ref_kind`.
  - *Proof:* `protocol_lock_pairing_refused_across_versions`.
  - *Proof:* `history_v3_lock_still_validates_verifies_and_pairs_with_v2`.
- [ ] **T5 Lightweight chain (AC4).** Add `RefKind`. For a lightweight pin,
  the captured ref must have `object.type == "commit"` and
  `object.sha == commit`, and the pin carries no tag object, signature or
  `git_tag` document. The tree rules stay as in v3. The v4 tag-shape check is
  equality with the ref plus a closed shape list.
  - *Proof:* `lightweight_pin_validates_from_authored_captures`.
  - *Proof:* `lightweight_pin_refuses_tag_object_signature_or_git_tag_doc`.
  - *Proof:* `lightweight_ref_of_type_tag_or_other_commit_refused`.
  - *Proof:* `lightweight_truncated_tree_or_missing_document_refused`.
  - *Proof:* `v4_tag_shapes_accept_release_bare_v_and_refuse_latest_branch_rc`.
  - *Proof:* `annotated_mailpit_mailhog_pins_unchanged_under_v4`.
- [ ] **T6 Image source (AC5).**
  - Add `PinSource::Image` and the `ImageIndex`/`ImageManifest` documents.
  - In `verify_files`: sha256(index bytes) == `index_digest`, the index names
    `manifest_digest` for the declared platform, and the manifest names
    `layer_digest`.
  - An image pin's `tagged_source` and `binary_build_provenance` must be
    `Unresolved`.
  - *Proof:* `image_pin_validates_content_addressed_chain`.
  - *Proof:* `image_pin_refuses_each_digest_mismatch_and_missing_manifest`.
  - *Proof:* `image_pin_refuses_observed_tagged_source_or_build_provenance`.
- [ ] **T7 Runtime pin and licenses (AC13, 0130 §6).**
  - Add `Runtime` with `checked_members`, and `artifact.checked_members`.
  - Add the runtime license list, holding exactly the JRE's expression,
    separate from `PERMISSIVE`. Leave `PERMISSIVE` unchanged.
  - *Proof:* `runtime_license_checked_against_runtime_list_only`.
  - *Proof:* `permissive_list_is_unchanged` (a constant equality).
  - *Proof:* `runtime_pin_refused_on_target_other_than_greenmail`.
- [ ] **T8 Listeners, env and config in the lock (AC6/AC7 structure).**
  - Use a closed role set, each role at most once, with `smtp` always present
    and exactly one read role.
  - Use per-role slots `{role_addr}`/`{role_port}` plus `{data_dir}`,
    `{scratch}` and `{artifact}`. The template must use every declared role
    and no undeclared one.
  - `launch_env` names are upper-snake, and the launcher-owned names are
    refused. `config_file` names have no `/` and no `..`.
  - *Proof:* `listener_roles_closed_unique_and_template_complete`.
  - *Proof:* `launch_env_refuses_owned_and_malformed_names`.
  - *Proof:* `config_file_name_confined`.
- [ ] **T9 Pending entries (S6).** A competitor entry is either `pin` or
  `pending`, never both and never neither. A pending entry is valid structure
  and is skipped by `verify_files`. `collection_ready` and launch refuse it by
  name.
  - *Proof:* `pending_entry_validates_but_refuses_collection_and_launch`.
- [ ] **T10 Protocol v3 (AC8/AC9).**
  - Accept `PROTOCOL_VERSION` 3, with v2 still valid. Add the v3 sections:
    `capabilities` with fixed keys and `products[].scope`.
  - A citation must name a document or reviewed-source path of that target's
    pin. `scope` must agree with `provenance()` and the pin source.
  - v3's id must differ from every historical id.
  - Throughput, wait, corpus, budgets and resources follow v2's rules.
  - *Proof:* `protocol_v3_requires_all_eight_and_each_section`.
  - *Proof:* `capability_without_or_with_foreign_citation_refused`.
  - *Proof:* `product_scope_must_match_provenance`.
  - *Proof:* `v3_reusing_v2_id_refused`.
  - *Proof:* `v3_cap_below_workload_refused`.
  - *Proof:* `v2_protocol_refuses_v3_sections`.
  - *Proof:* `frozen_only_when_nothing_unresolved_and_no_pending`.

## Phase 2 — lifecycle, decoders, POP3

- [ ] **T11 N-listener launch and exact listening set (R4, AC6).**
  - Reserve one port per declared role and substitute it. After readiness,
    read the group's listening sockets from `/proc` and refuse any address
    outside the declared set (`UndeclaredListener`), with cleanup.
  - `binds()` must be `Owned` for every listener. `CleanupFacts.listeners`
    records each role.
  - *Proof:* `undeclared_listener_refused_and_cleaned` (a scripted child
    through the existing `#[cfg(test)]` seam, which also binds an extra
    loopback port).
  - *Proof:* `cleanup_reports_each_listener_and_conjunction`.
  - *Proof:* the existing Swarmail lifecycle tests, unchanged and green.
- [ ] **T12 Env, config file and checked members at launch (AC7, AC13).**
  - Render `launch_env` and `config_file` into the owned scratch before spawn.
  - Digest the artifact, the runtime `checked_members` and the jar at launch,
    and again in `unchanged()` at seal.
  - *Proof:* `config_file_rendered_in_scratch_and_removed`.
  - *Proof:* `launch_env_slots_substituted`.
  - *Proof:* `checked_member_or_jar_mismatch_refuses_launch`, using authored
    files in a fresh test directory.
- [ ] **T13 Per-target readiness (R4).** SMTP `220` comes first, then an HTTP
  listing `200` or, for GreenMail, a POP3 `+OK`, all before the startup
  deadline.
  - *Proof:* `pop3_readiness_needs_greeting` (scripted responder).
  - *Proof:* `http_readiness_unchanged_for_v2_targets`.
- [ ] **T14 POP3 client (R6, AC11).** Add it to `wire.rs` on tokio:
  - `USER`/`PASS`, `STAT`, `UIDL`, `TOP n 0`, `RETR`, `QUIT`, and never `DELE`;
  - a fresh session per scan;
  - a line cap, a message cap at `max_response_bytes`, the request deadline,
    and dot-unstuffing;
  - `STAT` versus `UIDL` disagreement is `Incomplete`.
  - *Proof:* the AC11 cases against a scripted responder:
    `pop3_dot_unstuffing`, `pop3_err_each_step`, `pop3_long_line`,
    `pop3_oversized_message`, `pop3_stall_hits_deadline`,
    `pop3_cancel_mid_retr_leaves_no_task`, `pop3_never_sends_dele`,
    `pop3_stat_uidl_mismatch_incomplete`. All use the paused clock, and none
    sleep as synchronization.
- [ ] **T15 Decoders (R5, AC10).**
  - Lift page decoding to `decode_page(kind, &Value, start)`.
  - Add list, find and fetch arms for MailSlurper, smtp4dev, Inbucket and
    Alkemio, and route GreenMail through POP3. Cite each path to a captured
    document.
  - Where `server_search` is not `supported`, `find` is `list_all` filtered by
    token, under the same page bounds.
  - Inbucket's unpaged listing is one bounded response.
  - *Proof:* the tests `decode_<target>_{complete,truncated,wrong_start,
    repeated_id,moving_total,oversized,malformed}`, over independently authored
    bodies in `tests/mail_compare/captures/`.
  - *Proof:* `hostile_bodies_round_trip` (dot-leading lines, an 8-bit subject,
    a 998-octet header, the boundary string in the body) through `parse_raw`.
  - *Proof:* `client_side_find_labelled_and_complete`.
- [ ] **T16 Runner and report (R3).** Build `LaunchSpec` from either lock
  version. The report refuses inputs that span protocol ids and labels each
  row: source kind, scope, read path, search method, storage profile and
  resource profile.
  - *Proof:* `report_refuses_merged_v2_v3_runs`.
  - *Proof:* `row_labels_present_for_each_target`.
  - *Proof:* the existing three-target smoke test, run against the v3 history
    lock pairing.

## Phase 3 — pins, protocol, wrapper, docs

- [ ] **T17 Captures and pins (AC12, AC14).**
  - Write `docs/evals/sources/<target>/` captures and the v4 lock entries
    from verified bytes only: release, ref and tree, API and setup documents,
    reviewed sources, `--help` and version output, defaults cited, and caps
    overridden.
  - Extract Alkemio's binary from its layer into a fresh directory, using
    `tar` and/or `python3 -I` with scripts kept outside the directory.
  - Unpack the JRE the same way.
  - Rows without bytes are `pending` entries.
  - *Proof:* `current_lock_v4_validates_and_verifies` (CI-run).
  - *Proof:* `mailpit_mailhog_v3_documents_byte_unchanged`.
  - *Proof:* the real tests
    `real_<target>_smtp_to_native_api_round_trip` for each of the five, plus
    `real_greenmail_pop3_under_pinned_jre` and
    `real_caps_hold_at_workload_count`.
- [ ] **T18 Protocol v3 file (AC8/AC9).**
  - Write `docs/evals/protocol.json` v3 with the eight targets, rotations,
    capabilities, scopes, copied workloads, and a `change_reason` citing 0130.
  - Write `tests/mail_compare/eight-target-smoke.json`, a draft smoke.
  - Set `frozen` only if no row is pending and nothing is unresolved.
  - *Proof:* `current_protocol_v3_shape_and_v2_workloads_equal` (a
    field-by-field diff against the history v2).
  - *Proof:* `check` exit status as AC9 states, run in a test.
- [ ] **T19 Driver `targets` subcommand and wrapper (R8, AC15).**
  - Add `--bin NAME=PATH` and keep the `--mailpit`/`--mailhog` aliases.
  - Derive the required set from `mail_compare targets`. The other modes stay
    unchanged.
  - *Proof:* `wrapper_requires_every_declared_competitor`.
  - *Proof:* `wrapper_refuses_unknown_bin`.
  - *Proof:* `wrapper_aliases_still_work`.
  - *Proof:* `wrapper_paid_modes_refuse_as_before`. Each runs the real script
    with no paid inputs and no network.
- [ ] **T20 README (AC18).** Add the v3/lock v4 pointer section, with no
  comparison claim, the row labels and the open list.
  - *Proof:* review; `grep` finds no number or claim comparing tools in the
    new section.

## Phase 4 — bounded smoke and gates

- [ ] **T21 Bounded smoke (AC16, AC17).** Run each named command from the
  quickstart under `timeout 600 taskset -c 0-$((nproc/4-1))`, recording loadavg
  before and after.
  - *Proof:* a receipt in the delivery notes giving command, wall time, exit
    status and loadavg for each command. Each output is labelled smoke, and
    nothing is admitted.
- [ ] **T22 House gates (AC19).** Run the full inventory `--locked`:
  - fmt and clippy, root and evals;
  - `cargo test --workspace`;
  - `--test mail_compare`, and `-- --ignored` where the bytes are visible;
  - the candidate `swarmail` binary build;
  - the evals tests with `SWARMAIL_BIN` set from Cargo's artifact record;
  - the coverage gate and its selftest;
  - publish-guard and seal proofs;
  - `cargo deny check licenses`.
  Then run `git diff c95386e -- Cargo.toml Cargo.lock evals/Cargo.lock
  deny.toml src/`.
  - *Proof:* every command exits 0 and the diff is empty. Report the bundle
    compile as verify-owned if brokkr is absent. Report that the PR is held
    for the operator (because of `scripts/` and the amendments to accepted
    0091 §4 and 0093 §1).
