# Tasks: implicit TLS (SMTPS) on the SMTP listener

Do the tasks in order. Each is paired with the test that proves it. Every
test runs against a real `run_on` server with a real rustls client. The
certificate is generated per run, there are no sleeps used as sync, and
chaos runs at probability 100 (0020).

- [ ] **T1: index row for 0116.** Add the `docs/decisions/README.md` row
  for [0116](../../docs/decisions/0116-implicit-tls-smtp.md), status
  `proposed`, with a note that the number is provisional.
  *Proof:* `cargo test --test store_contract_docs` passes (index and
  record agree).
- [ ] **T2: the setting.** Add `SmtpConfig::implicit_tls` (default
  `false`), the `ServeArgs::smtp_implicit_tls` flag and env
  `SWARMAIL_SMTP_IMPLICIT_TLS`, and the `From<&ServeArgs>` mapping. Add the
  new field to the ServeArgs literal in `tests/config_cli.rs`.
  *Proof (AC10):* a new `tests/config_cli.rs` test covers four cases. No
  flag gives `false`. `--smtp-implicit-tls` gives `true`.
  `SWARMAIL_SMTP_IMPLICIT_TLS=true` gives `true`.
  `SWARMAIL_SMTP_IMPLICIT_TLS=false` gives `false`. Set env through the
  existing config_cli pattern, without racing other tests.
- [ ] **T3: fail closed without a pair.** Add
  `smtp::require_tls_for_implicit` and the `IMPLICIT_TLS_NEEDS_PAIR`
  message. Call the check from `run_on` before any `listen` and as
  `serve`'s first statement.
  *Proof (AC9), three tests:*
  - `tests/starttls.rs::implicit_tls_without_a_pair_refuses_to_serve`:
    `run_on` returns `Err` with exactly the message.
  - An inline `src/smtp.rs` test calls `serve` with `implicit_tls` and
    `tls = None`. It returns that error at once, with no accept.
  - `tests/binary.rs`: `swarmail serve --smtp-implicit-tls` (no pair)
    exits non-zero with the message on stderr.
- [ ] **T4: one handshake helper.** Lift the STARTTLS deadline block into
  `tls_handshake(acc, raw, deadline)`. The STARTTLS arm calls it, with
  behaviour unchanged.
  *Proof:* the existing `tests/starttls.rs` suite passes unmodified,
  including `a_stalled_starttls_handshake_is_dropped_within_the_deadline`
  and `a_client_that_refuses_the_cert_ends_the_session_and_the_server_serves_on`.
- [ ] **T5: implicit starting state.** In `handle_conn`, when `tls` is
  `Some` and `implicit_tls` is set, run `tls_handshake` on the accepted
  socket. Build `Conn::Tls` directly, without `take_plain`, and enter
  `session` with `TlsOffer::Active`. Move the chaos `Connect` check after
  the transport is chosen. Leave `serve`'s cap branch untouched. Add the
  client helpers `implicit_tls(..)` and `implicit_tls_handshake_fails(..)`
  to `tests/common/mod.rs`, and `start_implicit(..)` to
  `tests/starttls.rs`.
  *Proof (AC1):*
  `implicit_tls_delivers_to_the_auth_named_inbox_and_is_stored_at_250`.
  The client starts TLS on connect and reads `220` over TLS, then sends
  AUTH PLAIN as a per-test inbox, MAIL, RCPT and DATA. The body contains
  `0xFF`, a NUL, a dot-stuffed line, a subject and a link. Immediately
  after the `250`, the API shows exactly one message, with raw bytes equal
  to the unstuffed payload and the extracted subject, from and link
  present.
- [ ] **T6: no STARTTLS inside implicit TLS.**
  *Proof (AC2):*
  `starttls_is_neither_advertised_nor_accepted_inside_implicit_tls`.
  - The EHLO lines over implicit TLS equal a plain listener's EHLO lines,
    with `250-STARTTLS` absent.
  - `STARTTLS` gets `554 5.5.1 TLS already active`, and a second `AUTH`
    gets `503`.
  - A further delivery without re-AUTH lands in the original AUTH inbox.
- [ ] **T7: routing unchanged.**
  *Proof (AC3):* `implicit_tls_routes_by_recipient_ahead_of_auth`. With
  `route_domain` set and AUTH as `a`, a delivery to `x@<label>.<domain>`
  is stored in `<label>`, and `a` stays empty.
- [ ] **T8: await subscribes before it checks.**
  *Proof (AC4):* `an_await_issued_first_sees_an_implicit_tls_delivery`.
  Issue `GET /api/v1/inboxes/<fresh>/await?count=1&timeout_ms=…` and
  confirm it is pending. Then deliver over implicit TLS. The await
  returns the message, bounded by its own deadline with no sleep.
- [ ] **T9: stalled handshake.**
  *Proof (AC5):* `a_stalled_implicit_handshake_is_dropped_at_the_timeout`.
  Set `tls_handshake_timeout` to 750 ms. A raw TCP client sends nothing.
  A read at deadline/5 is still pending. Then EOF with zero bytes arrives
  within 5× the deadline. A fresh implicit client then delivers.
- [ ] **T10: plaintext client refused.**
  *Proof (AC6):* `a_plaintext_client_on_the_implicit_tls_port_gets_no_smtp_reply`.
  With a 10 s deadline, a raw TCP client sends `EHLO x\r\n`. The
  connection closes well before the deadline. The bytes read are empty or
  start with `0x15` (a TLS alert), and never contain `220`. A fresh
  implicit client then delivers.
- [ ] **T11: Connect chaos inside TLS.**
  *Proof (AC7):* `connect_chaos_reaches_an_implicit_client_inside_tls`.
  Install `PUT /api/v1/chaos` with a `connect` rule, probability 100,
  error `451 4.3.0 chaos`.
  - An implicit client handshakes and reads exactly that line, decrypted.
  - A raw TCP client on the same port reads bytes that never contain
    `451 4.3.0 chaos` (its non-handshake ends in rustls's refusal).
- [ ] **T12: cap refusal is 0027's.**
  *Proof (AC8):* `the_session_cap_refusal_is_unchanged_in_implicit_mode`.
  Set `max_sessions = 1`.
  - Hold one implicit session; its greeting has been read.
  - A raw TCP second connection reads exactly
    `421 4.7.0 Too many connections, closing transmission channel\r\n`,
    then EOF.
  - `implicit_tls_handshake_fails` holds for a TLS client in the same
    position.
  - QUIT the held session. A new implicit client then delivers. As in
    `tests/smtp_edges.rs`, reconnect against the cap without sleeps.
- [ ] **T13: wording (B8).** Update the help text of `--tls-cert` and
  `--tls-handshake-timeout-ms`, the `SmtpConfig::tls_handshake_timeout`
  doc and the `src/smtp.rs` module doc so they cover implicit TLS.
  *Proof:* `cargo test --test config_cli` and `--test binary` pass. Where
  `tests/binary.rs` or `tests/listener_contract_docs.rs` pin help text, the
  updated pin passes. `cargo clippy -D warnings` is clean.
- [ ] **T14: guide (B9).** Add an "Implicit TLS (SMTPS)" subsection to
  `docs/guide/smtp.md`. It covers:
  - the flag and env, off by default;
  - the required pair;
  - one mode per process, with no STARTTLS offered;
  - the shared deadline and the `0` caveat;
  - plaintext dropped without a reply;
  - the cap-refusal consequence;
  - the Kratos URL with `gen-cert`;
  - the chart recipe (`config:` env plus `tls:` mount).

  *Proof (AC12):* `cargo test` passes, including `tests/store_contract_docs.rs`
  and the guide's doc checks. A reviewer can tick every B9 point against
  the section.
- [ ] **T15: off is byte-identical, and every gate passes.**
  *Proof (AC11, AC13):* run every command in `docs/house-rules.md` § Gates,
  `--locked`, on the final tree:
  - fmt and clippy;
  - `cargo test --workspace`, including the unmodified
    `plain_listener_is_byte_identical_and_refuses_starttls`;
  - `cargo build --bin swarmail`;
  - the evals fmt, clippy and tests with `SWARMAIL_BIN` set to the
    candidate's binary;
  - `brokkr compile --bundle .brokkr/recipes/self`;
  - `scripts/coverage-gate.sh` (exact 100%) and its `selftest`;
  - `scripts/publish-guard-test.sh` and `scripts/seal-branch-test.sh`;
  - `cargo deny check licenses`.

  `git diff main -- Cargo.lock evals/Cargo.lock deny.toml .github scripts .brokkr charts Dockerfile`
  must be empty.
