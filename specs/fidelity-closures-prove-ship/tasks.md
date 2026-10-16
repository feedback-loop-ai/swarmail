# Tasks — prove and ship the fidelity closures

Ordered for the phases that remain: **verify** (the gates and the
demonstrations), **review** (the red lines and the named item), **ship**
(the hermetic close). There is no implementation task: the five closures
are committed at `24df038`, and the do-not-redo rule binds every seat. Each
task names the test or artifact that proves it; a task that cannot be
proved is not a task. Tests obey 0020 — no sleep-as-sync, no wall-clock
validity, chaos at the poles, fresh stores and per-test inboxes, cold
paths only in parent-declared `src/<module>/<name>_tests.rs` harnesses.

## Verify — the gates

- [ ] 1. Run the four non-coverage gates in order — `cargo fmt --all --
  --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace --locked`, `brokkr compile --bundle
  bundles/self` — and record the outputs.
  **Proved by:** all four exit 0; the suite reports 470 tests
  (spec.md AC-G.1).

- [ ] 2. Run the license gate and the coverage gate, in that order, and
  record both outputs.
  **Proved by:** `cargo deny check licenses` exit 0 with only the
  recorded CDLA-Permissive-2.0 exception (AC-G.3); `TMPDIR=/home/
  valentin/scratch/cov-tmp bash scripts/coverage-gate.sh` reports exact
  100/100/100, missed == 0 (AC-G.2). The TMPDIR is absolute — the gate is
  never run on tmpfs, and never run twice without need.

- [ ] 3. Confirm the landed shape is still the landed shape.
  **Proved by:** `git diff --name-status --diff-filter=A 2cfe08c..24df038`
  is empty; `grep '^percent-encoding' Cargo.toml` shows the one declared
  dependency with its reason comment; `grep -c 'coverage(off)' src/` is 0
  (AC-G.4).

## Verify — the per-issue demonstrations (proof sites in spec.md)

- [ ] 4. **#39 extraction honesty:** the wire leg and the decoder units.
  **Proved by:** `cargo test --test api
  smtp_html_hrefs_decode_character_references` (the `&amp;`/`&#38;` hrefs
  store decoded links) and `cargo test --lib extract` —
  `named_refs_decode_per_url`,
  `numeric_refs_decode_and_unknown_names_stay_literal`,
  `decode_refs_is_total_and_never_grows`, `links_keep_document_order`,
  `codes_keep_document_order`, `codes_ignore_style_script_and_attributes`,
  `codes_render_from_visible_html_text` (AC-39.1…39.4).

- [ ] 5. **#39 sender-honest record:** the bodies hold only what the
  sender sent, and the two display fallbacks still serve html-only mail.
  **Proved by:** `cargo test --lib
  bodies_hold_only_what_the_sender_sent`, `cargo test --test mcp
  seed_with_both_bodies_stores_html_and_drops_text_over_mcp`, and
  `cargo test --test compat mailpit_message_plain_and_headers_endpoints`
  (AC-39.5).

- [ ] 6. **#54 restored state:** the seed, the order, the index, and the
  one emptied-inbox rule on every path.
  **Proved by:** `cargo test --lib persist` —
  `the_first_insert_after_a_reopen_never_reuses_a_seq`,
  `a_legacy_duplicate_seq_restores_in_write_order`,
  `the_restore_scan_reads_the_seq_index`,
  `a_pre_thread_data_file_migrates_and_keeps_its_mail`,
  `open_fails_on_an_unusable_file`; `cargo test --test persistence
  an_emptied_inbox_is_absent_from_the_listing_live_and_after_a_reopen`
  and `deletes_and_clears_survive_a_restart`; `cargo test --test pop3
  a_maildrop_vanished_between_user_and_pass_is_refused_at_pass` and
  `a_vanished_maildrop_is_refused_after_a_reopen_too`; the crate-private
  pin `cargo test --lib open_refuses_an_unusable_data_file` (AC-54.1…54.5).

- [ ] 7. **#55 compat fidelity:** the alias, the envelope, the pages, the
  latest route, and the named refusals.
  **Proved by:** `cargo test --test compat` —
  `the_ids_alias_deletes_and_a_wrong_typed_one_still_refuses`,
  `shim_deletes_go_through_the_store`,
  `an_envelope_only_recipient_surfaces_in_every_compat_shape`,
  `a_zero_limit_pages_the_default_on_every_compat_surface`,
  `the_shim_pages_refuse_over_the_cap_and_honor_the_boundary`,
  `the_latest_route_serves_the_newest_mail_and_404s_an_empty_store`,
  `a_to_prefixed_query_is_refused_by_name`,
  `mailpit_search_supports_each_kind_and_scopes` (AC-55.1…55.5).

- [ ] 8. **#56 threads + web UI:** the normalization, the escape, the
  link, the status, the CSP, and the paint token.
  **Proved by:** `cargo test --lib threads` —
  `the_aw_marker_and_a_padded_colon_strip`,
  `the_subject_fallback_folds_unicode_case`; `cargo test --test ui` —
  `an_ampersand_title_is_escaped_exactly_once`,
  `a_hash_inbox_name_is_url_encoded_in_the_href`,
  `message_view_renders_text_html_and_missing`,
  `the_shell_ships_a_content_security_policy`,
  `the_live_shell_guards_paints_with_a_token`; README's
  "Unicode-lowercased" wording read against `src/threads.rs:167–169`
  (AC-56.1…56.6).

- [ ] 9. **#57 the record:** docs and comments match the code at
  `24df038`.
  **Proved by:** the row-by-row decision-index comparison
  (`docs/decisions/README.md` — every row's status equals its file's
  recorded status; four-column header), the allowlist diff
  (`deny.toml` = house rules + the one operator-noted CDLA allowance,
  recorded in `.brokkr/state/run11-operator-edits.md`), the E4 greps
  (README knob defaults as `host:port`, no `tests/smoke.rs`,
  `tests/p2.rs` listed, `RUST_LOG` documented, openapi.json's 503s /
  `limit`/`start` / `since_ms`/`subject`) and the E5 comment reads
  (`src/chaos.rs:16`, `src/store.rs:158`, `src/pop3.rs:11`,
  `src/pop3.rs:503`, `src/mcp.rs:245`, `src/lib.rs`,
  `benches/writer_stall.rs`) — shipped as review evidence, no new tests
  (AC-57.1…57.3).

- [ ] 10. **The frozen three are re-proved by the standing suites,
  unmodified.**
  **Proved by:** `cargo test --test burst the_burst_guarantee` and
  `a_cap_sized_data_is_stored_as_soon_as_the_250_leaves` (accept ==
  stored), `await_waits_for_arrival` and the await family (subscribe
  before check), and `cargo test --test persistence
  a_refused_insert_is_never_exposed_as_accepted_mail` (the cap legs —
  oldest-only eviction across a restart) (AC-R.3).

## Review

- [ ] 11. Deliver the hand-off and rule the named item.
  **Proved by:** the panel reads `specs/fidelity-closures-prove-ship/
  spec.md` §§The design of record and the red lines + Acceptance Criteria;
  0031 §3's carve-out sites are counted (`grep -n html_to_text src/` —
  exactly `src/compat.rs:560`, `:801` and the `src/extract.rs` codes
  haystack; AC-R.1); verdicts are recorded in the journal and a
  security-hold stops the run (AC-R.2). Findings are line- and
  sentence-level; any repair follows plan.md's protocol (owning file,
  same suite, 0020, one `proposed` decision if semantic).

- [ ] 12. Route the operator's acceptances.
  **Proved by:** 0031 and 0032 leave review with status `proposed` and
  their evidence attached; the acceptance remains the operator's act —
  recorded in `.brokkr/state/run11-operator-edits.md` if the operator
  edits anything.

## Ship

- [ ] 13. Close the run hermetically.
  **Proved by:** `brokkr run`'s ship seat (`scripts/ship-seat.sh`) execs
  the recorded evidence — the six gate outputs, the targeted filters, the
  review verdicts, the decision documents — and the journal's result chain
  ends clean; nothing is pushed; the position files and operator notes
  stay uncommitted under `.brokkr/state/`.

- [ ] 14. If any demonstration above failed and was repaired: re-run the
  affected gates and commit the repair per plan.md's protocol before ship.
  **Proved by:** the repaired criterion's own test plus the gate set
  green; the commit message names the criterion and the owning file.
