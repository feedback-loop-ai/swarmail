# Tasks — MailSlurper service-API shim

Each task is paired with the test that proves it. T1–T14 were built in
`8f36519` and verified at `439eba5`; their boxes are checked. T15–T21 are
the review return of 2026-11-02 ([plan.md](plan.md) § Review return), and
T21 reruns the full gate inventory last.

- [x] **T1 — Decision index.** Add the 0118 row (`proposed`) to
  `docs/decisions/README.md`. *Proof:* the index/record agreement in
  `tests/store_contract_docs.rs` passes.
- [x] **T2 — Config.** Add `--mailslurper-listen` /
  `SWARMAIL_MAILSLURPER_LISTEN` (no default) to `ServeArgs`, add
  `Config::mailslurper_listen`, and add `mailslurper_listen: None` to
  every `Config` literal (src harnesses, `tests/`, `benches/`). *Proof:*
  `tests/config_cli.rs` shows the flag and the env each set the field, and
  absence gives `None` (AC3).
- [x] **T3 — Shared serve.** Extract `api::serve_app` from `api::serve`
  and make `origin_guard` `pub(crate)`, with no HTTP-plane change.
  *Proof:* the existing `tests/api.rs`, `tests/lifecycle.rs` and drain
  tests stay green unmodified.
- [x] **T4 — Listener wiring.** Bind in `run_on` with the other binds,
  spawn only when set, key the guard to its own port when loopback, and add
  `RunningServer::mailslurper_addr`. *Proof:* `tests/mailslurper.rs`
  `off_by_default_and_absent_from_the_http_plane` (AC2, with a
  before/after count). In `tests/binary.rs`, the binary binds the env
  address, and an unbindable address exits non-zero before SMTP answers
  (AC3).
- [x] **T5 — Guard.** Wrap the router in `origin_guard`. *Proof:*
  `tests/mailslurper.rs`
  `the_loopback_guard_refuses_foreign_hosts_and_origins`: a wrong-port
  Host, a foreign Host and a foreign Origin each get `403`, and a guarded
  `DELETE` deletes nothing (AC4).
- [x] **T6 — Projection.** `SlurperItem`, `slurper_date` and
  `slurper_body`. *Proof:* inline unit tests in `src/compat.rs` for:
  - the key order and exact key set;
  - HTML before text before `""`;
  - recipients before header To;
  - the `""` sender;
  - the date format and the `i64::MIN` fallback.

  Wire test `the_item_shape_is_pinned_field_by_field` (AC5).
- [x] **T7 — `GET /mail` paging and omission.** The closed query, the page
  size, `totalPages`, empty-page omission and `page_work`. *Proof:*
  `tests/mailslurper.rs`:
  - the paging test (pages 1–3, AC8). It was built for 25 to a page and
    26 mails; T15 and T16 revise it to 50 and 51;
  - `an_empty_store_omits_mail_items` (exact bytes);
  - `invalid_page_numbers_refuse_by_name` (`0`, `-1`, `x`).
- [x] **T8 — Closed queries.** Refuse the search, sort and date keys, any
  unknown key, and any key on the other routes. *Proof:*
  `tests/mailslurper.rs` `every_unsupported_query_key_refuses_by_name`
  (AC9).
- [x] **T9 — Page cap.** `slurper_page_body` with a budget parameter and
  the named refusal. *Proof:* inline unit test
  `the_slurper_page_budget_fits_exactly_and_refuses_the_next_byte` (AC12).
- [x] **T10 — `GET /mail/{id}`, `/mail/{id}/message`, `/mailcount`.**
  *Proof:* `tests/mailslurper.rs`:
  - `one_mail_and_its_message_by_id` (AC5, AC6, with the `404`s);
  - `mailcount_sums_every_inbox` (AC7, including the empty case).
- [x] **T11 — `DELETE /mail`.** Check the query, then the body, then the
  verdict; then call `clear_all_async`, answer the plain-text count, and
  map a failed commit to `503`. *Proof:*
  - inline unit tests for each `prune_verdict` arm;
  - `tests/mailslurper.rs` `every_destructive_refusal_deletes_nothing`:
    every AC10 input, each with a before/after count over both inboxes,
    and the age codes named in the reason;
  - `a_failed_wipe_commit_answers_503`, through
    `common::refusing_trigger` (AC11).
- [x] **T12 — The recorded client, freshness, restart.** *Proof:*
  `tests/mailslurper.rs`:
  - `the_alkemio_client_recording_finds_and_deletes_mail` (AC1): the exact
    recorded headers, no `Content-Encoding`, the `href`-only link found by
    regex, bare `toAddresses`, body `2`, the empty envelope after, and
    both inboxes gone natively;
  - `every_250_is_listed_at_once` (AC13);
  - `an_id_survives_a_data_file_restart` (AC14).
- [x] **T13 — Docs.** Update:
  - `docs/guide/compat-shims.md`: the MailSlurper section from 0118
    §§1–6, with the cluster recipe and the *recalled* list;
  - the quickstart and network-posture listener and env lists;
  - the `src/compat.rs` module doc and the `config.rs` flag doc.

  *Proof:* a docs-contract test next to the existing compat-shims checks
  pins the global-view sentence, `--mailslurper-listen` and each age-code
  refusal in the guide (AC15). `tests/agent_setup_docs.rs` and
  `tests/listener_contract_docs.rs` stay green.
- [x] **T14 — Gates.** Run every house gate `--locked`:
  - fmt, clippy, test, and the `swarmail` binary build;
  - the evals fmt, clippy and test against that binary;
  - `brokkr compile --bundle .brokkr/recipes/self`;
  - `coverage-gate.sh` at exact 100%, and its `selftest`;
  - `publish-guard-test.sh` and `seal-branch-test.sh`;
  - `cargo deny check licenses`.

  *Proof:* every command exits 0, and
  `git diff --stat main -- Cargo.lock evals/Cargo.lock deny.toml charts .github scripts .brokkr`
  is empty (AC16).

## Review return (2026-11-02)

- [ ] **T15 — Page size 50.** Set `SLURPER_PAGE_SIZE` to 50 in
  `src/compat.rs`, fix the `slurper_list` doc comment ("50 to a page"),
  and make any inline unit test that hard-codes 25 or 26 follow the
  constant. Production code changes only in that constant, so the cut,
  `div_ceil` and overflow arms keep their tests. *Proof:* the inline
  page-cut, overflow and budget tests in `src/compat.rs` pass, and the
  coverage gate stays at exact 100% (T21).
- [ ] **T16 — AC8 at upstream's boundary.** Rewrite the paging wire test
  in `tests/mailslurper.rs` and rename it
  `pages_cut_at_fifty_newest_first_and_counted`. Send 51 mails over both
  inboxes (real SMTP, sequential). Assert:
  - a `pageNumber`-less `GET /mail`, the consumer's request, holds exactly
    the 50 newest ids in `(received_ms, id)` order;
  - `pageNumber=2` holds exactly the oldest id;
  - both pages carry `totalPages` 2 and `totalRecords` 51;
  - the union of the two pages is the 51 sent ids, none repeated;
  - `pageNumber=3` is exactly `{"totalPages":2,"totalRecords":51}`.

  Write the literal `50` once, as upstream's page size, not by reading the
  shim's constant. Keep the `0`, `-1` and `x` refusals. *Proof:* the
  renamed test passes, and it fails against a 25-item page (page 1 would
  hold 25, not 50).
- [ ] **T17 — Docs and the decision index.** Update:
  - `docs/guide/compat-shims.md`: the title "MailHog / Mailpit /
    MailSlurper shims" (keep the file name), an intro naming three shapes,
    the paging row and *Fixed paging* at 50, and the AC15 known
    differences (receive-time UTC `dateSent` and the `Date` header in local
    time upstream; receive-time order and upstream's `dateSent` DESC; the
    past-the-end `200`; the unknown-id `404` against upstream's `500`;
    `ccAddresses` as an extra key; the listener's requests absent from
    `swarmail_http_requests_in_flight`, named exactly; a list page over 64
    MiB as a `400` the consumer cannot handle). The recalled list is
    split as 0118 §7 splits it;
  - `src/static/llms.txt`: the HTTP plane's only cross-inbox wipe is
    `delete-all`, and the opt-in MailSlurper listener's `DELETE /mail
    {"pruneCode":"all"}` is the other (0118);
  - `README.md`: the feature line and the compatibility-layer line name
    MailSlurper and its opt-in listener;
  - `docs/guide/repo-layout.md`: `src/compat.rs` holds the
    MailHog/Mailpit/MailSlurper shims;
  - `docs/decisions/README.md`: the 0118 row says it amends 0004's and
    0011's sentences, only if accepted. The status stays `proposed`.

  *Proof:* the MailSlurper docs-contract test in `tests/mailslurper.rs`
  also pins the title, "50" in the paging line, and the scoped `llms.txt`
  sentence (read from `src/static/llms.txt`). The existing `/llms.txt`
  byte-equality test in `tests/api.rs`, `store_contract_docs`,
  `agent_setup_docs` and `listener_contract_docs` stay green (AC15).
- [ ] **T18 — The listener's build leaves the worker (tests F1).** In
  `src/compat/page_work_tests.rs`, extend
  `held_compat_page_builds_leave_smtp_and_health_serving`. Enable the
  listener on `127.0.0.1:0` in its config, and add `GET /mail` on the
  listener as a fifth held build. If the gate requires a site
  observation, the MailSlurper build reports it through the existing
  `render_site`/`observe_work_thread` path, exactly as the other list
  builds do. Placement follows 0020 rule 2: the gate's expiry is the
  harness's cold path. *Proof:* while all five builds are held, a new SMTP
  peer is greeted, `/healthz` answers and no gate expires; release
  finishes the listener's request with a `200` (AC17). The test is red if
  the slurper page is built inline, because the fifth build never enters
  the gate on a blocking thread.
- [ ] **T19 — Named deadlines in the wire client (tests F3).** Run
  `req()`'s connect, write and read-to-EOF in `tests/mailslurper.rs`
  under a named `tokio::time::timeout` at the house helpers' deadline
  (`tests/common/mod.rs`'s `HELPER_DEADLINE` scale, issue #102). A timeout
  panics with the method, path and step. *Proof:* every test in
  `tests/mailslurper.rs` still passes unchanged in its assertions (AC18).
  No test depends on the deadline for validity (0020 rule 5).
- [ ] **T20 — A wipe survives restart.** Extend
  `an_id_survives_a_data_file_restart`: after the item is found on the
  restarted server, `DELETE /mail {"pruneCode":"all"}`, stop, restart on
  the same data file, then `GET /mail` is exactly
  `{"totalPages":0,"totalRecords":0}`, and the native API shows both inboxes
  empty. *Proof:* the extended test (AC14).
- [ ] **T21 — Gates and the hold.** Rerun every house gate `--locked`, as
  T14 lists them, on the revised tree: fmt, clippy, test, the binary
  build, the evals crate against that binary, `brokkr compile`, the
  coverage gate at exact 100% and its selftest, the publish-guard and seal
  proofs, and `cargo deny check licenses`. Carry plan.md's ship-notes
  HOLD line into the PR body's first line. *Proof:* every command exits 0;
  `git diff --stat main -- Cargo.lock evals/Cargo.lock deny.toml charts .github scripts .brokkr`
  is empty; and `docs/decisions/0118-mailslurper-shim.md` still reads
  `- **Status:** proposed` (AC16).

