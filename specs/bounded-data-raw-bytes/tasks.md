# Tasks — bounded DATA path, real 8BITMIME, byte-exact `/raw`

Ordered. Each task is paired with the real-protocol test that proves it
(real servers via `tests/common`, never mocks). Implement executes 1–13 in
order; task 13 is verify-only. The exact 100/100/100 gate is the arbiter:
if an arm measures uncovered, the arm is dead or under-tested and the fix
is a test — `coverage(off)` is forbidden.

- [ ] **1. Add the byte-exact test helpers** — in `tests/common/mod.rs`:
  `SmtpConn::data_bytes(&[u8])` (raw DATA payload + `.` terminator, final
  reply returned) and `http_get_bytes(addr, path) -> (u16, String, Vec<u8>)`
  (raw TCP, status, header block, body split on the first `\r\n\r\n`).
  *Proved by:* every later task uses them; `cargo test` stays green with
  the existing suites (helpers are additive, `#![allow(dead_code)]` already
  covers per-binary compilation).
- [ ] **2. The bounded byte reader + DATA arm rewiring** — in
  `src/smtp.rs`: one free reader beside the DATA arm (`Complete(Vec<u8>)` /
  `OverCap` / `Eof`, `fill_buf`/`consume`, wire-line bound
  `cfg.max_message_size + 1`, drain-to-terminator on over-cap, bytes never
  through `String`); the DATA arm accumulates bytes, keeps the existing
  `over_limit` check as the sole refusal authority, and preserves today's
  EOF/vanish semantics (nothing stored). *Proved by:* new
  `over_cap_line_is_drained_and_the_session_stays_in_sync` (small cap,
  unterminated over-cap line dribbled in small writes → `552`, count 0,
  `/healthz` answers, same connection then completes a small mail with
  `250`) **and** the existing `oversized_data_gets_552_and_is_not_stored`
  and peer-vanish tests stay green.
- [ ] **3. Byte terminator + byte unstuffing** — terminator check on
  `b".\r\n"` / `b".\n"`, `..` → drop one leading byte (predicate unchanged,
  RFC receiver-rule rewrite rejected per decision 0022). *Proved by:*
  existing `dot_stuffing_is_unstuffed_and_peer_vanish_is_silent` plus the
  new at-cap stuffed-line test (task 5) and the binary round-trip (task 4),
  which carry a `..`-stuffed line and a literal `.`-starting line.
- [ ] **4. Binary round-trip through the API (contracts #16a + #16b)** —
  new `binary_data_round_trips_byte_identical_through_the_api` in
  `tests/smtp_edges.rs`: DATA a body with `0xFF 0xFE 0x00`, a `NUL`, a bare
  `CR`, a stuffed line and a literal-dot line → `250` → `GET
  /api/v1/messages/{id}/raw`: status 200, `Content-Type: message/rfc822`,
  body `assert_eq!` byte-identical, `size` == body length. *Proved by:* the
  test itself; it is the contract.
- [ ] **5. The `+1` pin: at-cap stuffed line and advertised == enforced** —
  new `exactly_at_cap_stuffed_line_is_accepted_pins_the_cap_slack` in
  `tests/smtp_edges.rs` (cap 64): `EHLO` advertises `250-SIZE 64`; one
  stuffed wire line of 65 bytes → `250`; stored payload is the exact 64
  unstuffed bytes via `/raw`; `size` == 64. *Proved by:* the test itself —
  it goes red if the `+1` slack is dropped (spurious `552` at the cap).
- [ ] **6. `/raw` delegates to the exact-bytes owner (contract #16b)** —
  `src/api.rs::get_raw` → `compat::raw_response(&e)`; `src/compat.rs`
  gains one visibility word (`pub(crate)`). *Proved by:* all existing
  `/raw` assertions in `tests/api.rs`, `tests/persistence.rs`,
  `tests/smtp_edges.rs` (UTF-8 bodies byte-identical) plus task 4's header
  and byte asserts.
- [ ] **7. Binary round-trip over STARTTLS** — new
  `binary_data_round_trips_over_starttls` in `tests/starttls.rs`: the task-4
  body over an upgraded session, `/raw` identical. *Proved by:* the test
  itself (the reader is transport-agnostic on `BufReader<Conn>`; this pins
  it).
- [ ] **8. Binary body survives the restart round-trip** — new
  `binary_body_survives_the_restart_round_trip` in `tests/persistence.rs`:
  binary DATA → data file → drop → respawn → `/raw` byte-identical.
  *Proved by:* the test itself (the disk is byte-honest too).
- [ ] **9. Chaos over hostile bytes** — extend
  `chaos_on_connect_mailfrom_and_data_with_and_without_error` in
  `tests/smtp_edges.rs` with one binary-body `Data` case: error rule → not
  stored (count 0), delay-only → stored with bytes intact. *Proved by:* the
  extended test (ordering after the reader, before the insert, is intact
  over hostile bytes).
- [ ] **10. `8BITMIME` pinned on the EHLO surface** — add the
  `250-8BITMIME` assertion to `ehlo_advertises_the_size_limit` in
  `tests/smtp_edges.rs`. *Proved by:* the assertion (the advertisement is
  contract surface; task 4 makes it mean something).
- [ ] **11. Machine-readable + human docs say what the wire now says** —
  `src/static/openapi.json:245` description → `message/rfc822`; README
  SMTP-surfaces sentence gains the cap semantics clause (over-cap → `552`,
  nothing stored, DATA never buffers past the cap, bytes verbatim through
  `/raw`) and the `max_per_inbox × max_message_size` envelope note.
  *Proved by:* task 4's header assert (docs match wire) + full suite green
  (`/openapi.json` is served and only its `openapi` version is pinned by
  `tests/api.rs:601-604`).
- [ ] **12. No-dependency + decision-completeness check** — confirm no
  `Cargo.toml` change (`cargo deny check licenses` clean), and that
  decision 0022 (`proposed`) is committed alongside the spec.
  *Proved by:* `cargo deny check licenses` and the committed tree.
- [ ] **13. All six gates clean** — `cargo fmt --all -- --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`,
  `scripts/coverage-gate.sh` (exact 100/100/100),
  `cargo deny check licenses`, `brokkr compile --bundle .`.
  *Proved by:* the gates themselves; any red is a defect in tasks 1–12,
  not a knob.
