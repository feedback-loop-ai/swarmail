# Tasks — web UI run console (issue #355)

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md) · Decision: [0132](../../docs/decisions/0132-ui-projection-route.md) (proposed).
Ordered. Each task names the test that proves it. Write the test first and see it fail where that is possible. Existing tests in `tests/ui.rs` and `src/ui.rs` are not edited (AC2).

## Phase 1 — Records

- [x] T1. Add the 0132 row to `docs/decisions/README.md` (status `proposed`, citing issue #355 and the operator's 2026-11-16 ruling). **Proof:** `cargo test --locked --test store_contract_docs` (`every_decision_file_has_exactly_one_resolving_index_row`), AC32.
- [x] T2. Append one `## Dated pointer — 2026-11-16 (proposed 0132)` section after the last line of each of the 0110, 0094, 0126 and 0027 records, as plan § 6 words it. Change nothing above it. **Proof:** `git diff main -- docs/decisions/0110-attachments-and-ics.md docs/decisions/0094-focused-mail-waits.md docs/decisions/0126-spf-dmarc-alignment.md docs/decisions/0027-living-listeners-bounded-sessions.md` shows only added trailing lines; `cargo test --locked --test store_contract_docs` and `--test deliverability` (the accepted-called-proposed check) pass. AC32.

## Phase 2 — Read-only fields

- [x] T3. Add `Store::inbox_summaries()` in `src/store.rs` (name, count, back-of-deque `received_ms`), sorted by name. **Proof:** new inline test `inbox_summaries_carry_the_newest_retained_stamp` in `src/store.rs`: two inserts give the second mail's stamp; under a cap of 1 the survivor's stamp; after `clear` the inbox is absent. `cargo test --locked --lib store::`.
- [x] T4. Give `InboxSummary` the always-serialized `last_received_ms` and make `inbox_page` return `Vec<InboxSummary>` from T3; adapt `src/ui.rs`'s one caller. **Proof:** new `tests/api.rs` test `the_inbox_list_carries_the_last_received_stamp` (AC8); the existing `list_inboxes_names_and_counts` and the paging tests pass unmodified.
- [x] T5. Add `emails_inserted_total` and `emails_dropped_total` to the `ok` and `degraded` `/healthz` bodies; leave the stopping arm alone. **Proof:** new `tests/api.rs` test `healthz_reports_the_store_counters` (fresh, two mails, cap of 1, equal to `/metrics`; AC5); new inline `src/api.rs` test `a_degraded_healthz_still_carries_the_counters` (AC6); the four existing `healthz_*` unit tests and `a_healthz_inside_its_handler_at_the_stop_is_answered_503_stopping` pass unmodified.

## Phase 3 — The projection route

- [x] T6. Add the `MessageProjection` types and `impl From<&Email>` beside the compact view in `src/api.rs`, with dedicated structs (no `rel`, `sha256`, `stored`, `uid`, no content) and `PRESENTATION_CAP`. **Proof:** new inline test `the_projection_is_capped_and_reads_only_stored_fields` (251 occurrences, a 300-total list of 250 parts, empty `raw`; unknown lists serialize as `null`; AC22).
- [x] T7. Add `get_projection` and the route `GET /api/v1/messages/{id}/projection` with the closed `ViewQuery {}` query and the `404`. **Proof:** new `tests/api.rs` tests `the_projection_serves_what_ingest_stored` (multipart mail over real SMTP with a chosen `EHLO`; AC18), `a_seeded_mail_projects_no_smtp_client` (AC19), the AC20 member-set assertions inside `the_projection_serves_what_ingest_stored` and `the_projection_refuses_unknown_ids_queries_and_methods` (404, 400 naming route and key, 405 for `POST`/`PUT`/`DELETE`, no `access-control-` header; AC21).
- [x] T8. Prove that no existing serialization moved. **Proof:** new `tests/api.rs` test `no_other_surface_gains_the_projected_fields` (message, list, thread view and one feed frame for the T7 mail contain none of `link_metadata`, `smtp_client_ip`, `smtp_helo`, `attachments`; AC23); `model::tests::the_observed_envelope_never_serializes` passes unmodified.
- [x] T9. Document the route, the `MessageProjection` schema, `InboxSummary.last_received_ms` and the `/healthz` counters in `src/static/openapi.json`; add the paragraph to `src/static/llms.txt`. **Proof:** new `tests/api.rs` test `openapi_documents_the_projection_and_the_two_fields` (AC24); `machine_docs_are_served_from_the_binary` (the dangling-`$ref` walk), `--test store_contract_docs` and `--test agent_setup_docs` pass.

## Phase 4 — The page

- [x] T10. Move the stylesheet to `src/static/ui.css` (prototype rules, plan § 5 edits, the legacy classes) and embed it in `page()`. **Proof:** new `tests/ui.rs` test `the_stylesheet_shows_one_pane_at_phone_width_and_has_both_themes` (AC29); every existing message-page test passes unmodified (AC3).
- [x] T11. Write `console(…)` and `rail(…)` in `src/ui.rs`; route `/`, `/ui/inbox/{inbox}` and `/ui/inbox/{inbox}/thread/{key}` through them; keep the tagline literal, `#view` and its data attributes, the pager, the percent-encoded hrefs and the `400` refusals. **Proof:** new `tests/ui.rs` tests `every_console_route_serves_the_three_panes_under_one_status_bar` (AC1, AC4's ids), `the_rail_carries_its_page_and_marks_the_open_inbox` (AC9's attributes) and `the_no_inboxes_sentence_is_chosen_by_the_server` (AC10); `cargo test --locked --test ui` with every existing test unmodified (AC2); `cargo test --locked --lib ui::`.
- [x] T12. Assert the absence of demo controls. **Proof:** new `tests/ui.rs` test `the_console_ships_no_demo_controls` (AC7).

## Phase 5 — The script

- [x] T13. Rewrite `src/static/ui.js` sections 1–5 of plan § 4: bindings, helpers, formatting, `readJson`, the status tick with the `document.hidden` and in-flight guards, the listener, counter, chaos and live paints. **Proof:** new `tests/ui.rs` test `the_status_bar_is_wired_to_healthz_chaos_and_the_feed` (AC4's script half).
- [x] T14. Inboxes: the poll built from `data-limit`/`data-offset`, `paintRail`, the filter, `lastSeen` with `sessionStorage` and the dot rule, the unavailable state. **Proof:** new `tests/ui.rs` test `the_rail_polls_its_own_page_and_opens_one_feed` (AC9's script half, AC10's script strings).
- [x] T15. Enrichment: `cache`, `queue`, `pump`, `enrich`, the retry on a tick, the `404` state. **Proof:** new `tests/ui.rs` test `enrichment_names_each_route_once_and_never_probes` (AC11, AC12).
- [x] T16. Stream: `paint` with the header, "newest N of M", thread labels, rows with codes, link count, attachment and severity chips, the fresh marker, first-paint selection, the three states, the read-failure state in `load()`. **Proof:** new `tests/ui.rs` test `the_stream_states_are_distinct_strings` (AC13); the existing `the_live_shell_guards_paints_with_a_token`, `inbox_view_is_a_live_shell_wired_to_the_api_and_feed` and `thread_view_is_a_live_shell_for_one_conversation` pass unmodified.
- [x] T17. Clear inbox: the armed state, the one `DELETE`, the failure state. **Proof:** new `tests/ui.rs` test `the_only_write_is_the_confirmed_clear` (AC14); the existing wire test of `DELETE /api/v1/inboxes/{inbox}/messages` in `tests/api.rs` passes unmodified.
- [x] T18. Reader: envelope and client rows, "Extracted at ingest" with copy controls and the null fallbacks, the authentication row, the four tabs, the sandboxed preview, the findings list, the standalone link, the three failure states. **Proof:** new `tests/ui.rs` tests `the_reader_labels_ingest_and_query_time_data_apart` (AC15) and `the_preview_frame_is_sandboxed_without_tokens` (AC16).
- [x] T19. "For your test" snippets with `location.origin`, `shellSegment` and `JSON.stringify`. **Proof:** new `tests/ui.rs` test `the_snippets_name_real_tools_and_quote_safely` (AC17: every `swarmail_…` name in the script is in the server's `tools/list`).

## Phase 6 — Safety proofs

- [x] T20. The forbidden-construct sweep over the served script. **Proof:** new `tests/ui.rs` test `the_script_has_no_markup_sink_no_style_write_and_no_mail_href` (AC25).
- [x] T21. Hostile mail end to end. **Proof:** new `tests/ui.rs` test `hostile_mail_never_reaches_the_shell_as_markup` (payloads through subject, From name, `EHLO`, filename, calendar summary, anchor text and body; the hostile inbox name in `data-inbox`; AC26).
- [x] T22. The policy and the no-external-load sweep. **Proof:** new `tests/ui.rs` tests `every_ui_route_ships_the_one_policy_byte_for_byte` (AC27) and `the_console_loads_nothing_from_outside` (AC28); the existing `the_shell_ships_a_content_security_policy` passes unmodified.

## Phase 7 — Gates and report

- [x] T23. Run the whole inventory of `docs/house-rules.md` § Gates on the final tree, `--locked`, including the coverage gate, its selftest, the publish-guard proof, the seal-branch proof, `cargo deny check licenses`, the recipe compile loop and the `evals/` gates against the candidate's own binary. **Proof:** each command's exit status, recorded in the delivery (AC30); `git diff main --stat` for AC31.
- [ ] T24. Walk plan § Manual review in a browser and record the result of each step as manual review, with the browser used. Do not report any of it as a passed gate. **Proof:** the delivery's report (AC33).
- [ ] T25. State in the delivery that the pull request is held for the operator's acceptance of 0132, and that two other branches also carry a proposed 0132 for the operator to renumber. **Proof:** the delivery's report; spec § header "Hold".
