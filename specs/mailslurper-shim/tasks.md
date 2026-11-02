# Tasks — MailSlurper service-API shim

Each task is paired with the test that proves it. Run the full gate
inventory last (T14).

- [ ] **T1 — Decision index.** Add the 0118 row (`proposed`) to
  `docs/decisions/README.md`. *Proof:* the index/record agreement in
  `tests/store_contract_docs.rs` passes.
- [ ] **T2 — Config.** Add `--mailslurper-listen` /
  `SWARMAIL_MAILSLURPER_LISTEN` (no default) to `ServeArgs`, add
  `Config::mailslurper_listen`, and add `mailslurper_listen: None` to
  every `Config` literal (src harnesses, `tests/`, `benches/`). *Proof:*
  `tests/config_cli.rs` shows the flag and the env each set the field, and
  absence gives `None` (AC3).
- [ ] **T3 — Shared serve.** Extract `api::serve_app` from `api::serve`
  and make `origin_guard` `pub(crate)`, with no HTTP-plane change.
  *Proof:* the existing `tests/api.rs`, `tests/lifecycle.rs` and drain
  tests stay green unmodified.
- [ ] **T4 — Listener wiring.** Bind in `run_on` with the other binds,
  spawn only when set, key the guard to its own port when loopback, and add
  `RunningServer::mailslurper_addr`. *Proof:* `tests/mailslurper.rs`
  `off_by_default_and_absent_from_the_http_plane` (AC2, with a
  before/after count). In `tests/binary.rs`, the binary binds the env
  address, and an unbindable address exits non-zero before SMTP answers
  (AC3).
- [ ] **T5 — Guard.** Wrap the router in `origin_guard`. *Proof:*
  `tests/mailslurper.rs`
  `the_loopback_guard_refuses_foreign_hosts_and_origins`: a wrong-port
  Host, a foreign Host and a foreign Origin each get `403`, and a guarded
  `DELETE` deletes nothing (AC4).
- [ ] **T6 — Projection.** `SlurperItem`, `slurper_date` and
  `slurper_body`. *Proof:* inline unit tests in `src/compat.rs` for:
  - the key order and exact key set;
  - HTML before text before `""`;
  - recipients before header To;
  - the `""` sender;
  - the date format and the `i64::MIN` fallback.

  Wire test `the_item_shape_is_pinned_field_by_field` (AC5).
- [ ] **T7 — `GET /mail` paging and omission.** The closed query, page
  size 25, `totalPages`, empty-page omission and `page_work`. *Proof:*
  `tests/mailslurper.rs`:
  - `pages_are_newest_first_and_counted` (26 mails, pages 1–3, AC8);
  - `an_empty_store_omits_mail_items` (exact bytes);
  - `invalid_page_numbers_refuse_by_name` (`0`, `-1`, `x`).
- [ ] **T8 — Closed queries.** Refuse the search, sort and date keys, any
  unknown key, and any key on the other routes. *Proof:*
  `tests/mailslurper.rs` `every_unsupported_query_key_refuses_by_name`
  (AC9).
- [ ] **T9 — Page cap.** `slurper_page_body` with a budget parameter and
  the named refusal. *Proof:* inline unit test
  `the_slurper_page_budget_fits_exactly_and_refuses_the_next_byte` (AC12).
- [ ] **T10 — `GET /mail/{id}`, `/mail/{id}/message`, `/mailcount`.**
  *Proof:* `tests/mailslurper.rs`:
  - `one_mail_and_its_message_by_id` (AC5, AC6, with the `404`s);
  - `mailcount_sums_every_inbox` (AC7, including the empty case).
- [ ] **T11 — `DELETE /mail`.** Check the query, then the body, then the
  verdict; then call `clear_all_async`, answer the plain-text count, and
  map a failed commit to `503`. *Proof:*
  - inline unit tests for each `prune_verdict` arm;
  - `tests/mailslurper.rs` `every_destructive_refusal_deletes_nothing`:
    every AC10 input, each with a before/after count over both inboxes,
    and the age codes named in the reason;
  - `a_failed_wipe_commit_answers_503`, through
    `common::refusing_trigger` (AC11).
- [ ] **T12 — The recorded client, freshness, restart.** *Proof:*
  `tests/mailslurper.rs`:
  - `the_alkemio_client_recording_finds_and_deletes_mail` (AC1): the exact
    recorded headers, no `Content-Encoding`, the `href`-only link found by
    regex, bare `toAddresses`, body `2`, the empty envelope after, and
    both inboxes gone natively;
  - `every_250_is_listed_at_once` (AC13);
  - `an_id_survives_a_data_file_restart` (AC14).
- [ ] **T13 — Docs.** Update:
  - `docs/guide/compat-shims.md`: the MailSlurper section from 0118
    §§1–6, with the cluster recipe and the *recalled* list;
  - the quickstart and network-posture listener and env lists;
  - the `src/compat.rs` module doc and the `config.rs` flag doc.

  *Proof:* a docs-contract test next to the existing compat-shims checks
  pins the global-view sentence, `--mailslurper-listen` and each age-code
  refusal in the guide (AC15). `tests/agent_setup_docs.rs` and
  `tests/listener_contract_docs.rs` stay green.
- [ ] **T14 — Gates.** Run every house gate `--locked`:
  - fmt, clippy, test, and the `swarmail` binary build;
  - the evals fmt, clippy and test against that binary;
  - `brokkr compile --bundle .brokkr/recipes/self`;
  - `coverage-gate.sh` at exact 100%, and its `selftest`;
  - `publish-guard-test.sh` and `seal-branch-test.sh`;
  - `cargo deny check licenses`.

  *Proof:* every command exits 0, and
  `git diff --stat main -- Cargo.lock evals/Cargo.lock deny.toml charts .github scripts .brokkr`
  is empty (AC16).
