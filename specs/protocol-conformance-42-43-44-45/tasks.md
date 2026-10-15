# Tasks — protocol conformance #42/#43/#44/#45

Ordered. Each task is paired with the real-protocol test that proves it
(real servers via `tests/common`, never mocks). Implement executes 1-10
in order; task 10 is verify-only. The exact 100/100/100 gate is the
arbiter: if an arm measures uncovered, the arm is dead or under-tested
and the fix is a test — `coverage(off)` is forbidden. Decision 0020's
disciplines bind every edit: no sleeps as synchronization, chaos at the
poles only (probability 0 or 100), per-test fresh stores/inboxes, no
wall-clock assertions.

- [ ] **1. Terminate DATA on `.` CRLF only** — in `src/smtp.rs:838`, drop
  `|| bytes == b".\n"`; reword the allowance comment's "`.` CRLF or `.`
  LF" clause (`:806-811`) — the floor stays `.max(3)` because the CRLF
  terminator is three bytes. A bare-LF `.` line is payload, passed
  verbatim by the `..`-only unstuffing (`:842-846`).
  *Test:* new leg in `tests/smtp_edges.rs` —
  `a_bare_lf_dot_line_is_payload_not_a_terminator`: authenticate, MAIL,
  RCPT, send `foo\n.\nbar\r\n.\r\n` as data; assert exactly one stored
  mail whose raw contains `.\n` (via `GET /api/v1/messages`), the reply
  is one `250`, and nothing after the `.\n` was parsed as a command.
  Red on current code (two mails, or a smuggled command). Proves AC2.
- [ ] **2. Consume the envelope at the terminator** — in
  `src/smtp.rs:878-879`, replace the read with
  `mail_from.take()` / `std::mem::take(&mut rcpts)` taken immediately
  after the data loop breaks (before the `over_limit`, chaos and insert
  branches), feeding `build_email` directly. No outcome path and no
  abort path runs cleanup — the invariant the spec pins. **Keep the
  take before `build_email`; do not leave a `clone()` beside it.**
  *Test:* legs in `tests/smtp_edges.rs`: (a)
  `a_completed_transaction_leaves_nothing` — MAIL/RCPT/DATA → `250`,
  then `RCPT` answers `503 5.5.1 Error: need MAIL command` and `DATA`
  answers `503 5.5.1 Error: need RCPT command`, store count unchanged;
  (b) the pipelining chain MAIL/RCPT/DATA → MAIL (probability-100
  chaos at `mail_from`) → DATA → `.` — the second DATA answers `503`,
  the lone `.` is answered as a command, and **no empty mail is
  stored**; (c) a DATA refused `552` (over-cap) or `451` (chaos or
  commit-refused via `refusing_trigger`) likewise leaves `DATA`
  answering `503`. Proves AC1 and AC4 (with task 3's leg).
- [ ] **3. Refuse out-of-sequence commands** — in `src/smtp.rs`: after
  the chaos check in the `MAIL` arm, `if mail_from.is_some()` →
  `503 5.5.1 Error: nested MAIL command` (state unchanged); in the
  `DATA` arm (`:787-795`) check `mail_from.is_none()` →
  `503 5.5.1 Error: need MAIL command` *before* the `rcpts.is_empty()`
  → `need RCPT` check (which leaves the transaction open); in the
  `AUTH` arm, after the `accept_any_auth` 502 gate,
  `if mail_from.is_some()` → `503 5.5.1`. In the `MAIL` chaos branch
  (`:736-745`) clear `mail_from`/`rcpts` before the error — a refused
  attempt ends the transaction.
  *Test:* legs in `tests/smtp_edges.rs`:
  `a_nested_mail_is_refused_and_leaves_the_transaction_open` (nested
  MAIL → `503`, then DATA still stores);
  `auth_during_a_transaction_is_refused` (AUTH after MAIL →
  `503`, then DATA stores in the original inbox);
  `a_chaos_refused_mail_consumes_the_envelope` (probability-100 rule at
  `mail_from` mid-transaction → refused MAIL, then DATA → `503`, store
  count unchanged); `data_without_mail_answers_need_mail` (DATA first →
  `503 5.5.1 Error: need MAIL command`, exact line). Proves AC3, AC4,
  AC5.
- [ ] **4. Honor the AUTH LOGIN initial response; refuse credentials that
  name nothing** — in `src/smtp.rs`: `AUTH LOGIN <b64>` decodes the
  initial response as the username (strict: `b64_decode` →
  `String::from_utf8` — no `from_utf8_lossy`, no silent `Some`) and
  prompts only `334 UGFzc3dvcmQ6`; a bare `*` at any prompt or in the
  initial response (both mechanisms), undecodable base64, and an
  empty-after-trim username answer `501 5.5.2` and change nothing —
  the inbox is assigned only on success. The password is inspected only
  for a bare `*`. PLAIN keeps `auth_plain_inbox`'s shape but maps its
  `None` to `501 5.5.2` instead of a silent `235`. Zero new types.
  *Test:* rewrite the three pinned silent-`235` tests in
  `tests/smtp_edges.rs` (`:94-95` area,
  `auth_login_with_undecodable_username_stays_default`,
  `auth_plain_with_empty_authcid_stays_default`) to pin `501 5.5.2`;
  add: `auth_login_initial_response_names_the_inbox` (smtplib shape —
  `AUTH LOGIN <b64(user)>` → `334 UGFzc3dvcmQ6` → password → `235`,
  deliver, find the mail in the named inbox via HTTP — the issue's own
  scenario); `a_star_cancels_auth` (at username prompt, at password
  prompt, at the PLAIN prompt → `501`, inbox unchanged);
  `a_non_utf8_username_is_refused` (`501`, inbox unchanged);
  `an_undecodable_initial_response_is_refused`. Proves AC7, AC8, AC9,
  AC10.
- [ ] **5. Derive POP3 sizes from the writer** — in `src/pop3.rs`: add
  `fn wire_len(raw: &[u8]) -> u64` beside `write_stuffed` (`:631`) —
  stuff into a scratch buffer, return its length (one reused buffer is
  fine); build the PASS snapshot's `MaildropMsg::size` with it
  (`:315-323`); restructure `RETR` (`:410-416`) to stuff into a `body`
  buffer and answer `+OK {body.len()} octets`. No mirrored walk of the
  rules anywhere; `Email.size` untouched.
  *Test:* the inline unit corpus (branch-stable, 0020 rule 1) —
  `wire_len` over `.lead\r\nmid\n.\r\r\nno-newline`,
  `plain\r\nlines\r\n` and an empty raw; wire legs in
  `tests/pop3.rs`: bare-LF mail (STAT, LIST arg, LIST multi-line, RETR
  header), mixed/dot-stuffed mail, and
  `the_advertised_octets_equal_the_transmitted_bytes` pinning
  `advertised == c.data_bytes().await.len()` on one message. Proves
  AC11 (with 7) and AC12 (with 6).
- [ ] **6. Rewire the test expectations to the wire** — in
  `tests/pop3.rs`: `octets()` (`:18-20`) derives its count from the
  wire transformation applied to `raw_of(...)` (CRLF-termination +
  dot-stuffing, `.\r\n` excluded) instead of `raw_of(...).len()`;
  rewrite the pinned stored-size test (`:200-224`, comment included)
  and every other size expectation. `tests/common/mod.rs` stays at zero
  diff unless a leg genuinely needs a helper.
  *Test:* the rewritten
  `retr_stuffs_leading_dots_and_normalizes_line_endings` —
  `+OK {expected} octets` with `expected` derived from the
  wire, and the full POP3 suite green. Proves AC12.
- [ ] **7. Mark committed deletes gone; keep the snapshot honest** — in
  `src/pop3.rs`: `MaildropMsg` gains `gone: bool` (`:171-175`, built
  `false` at PASS `:315-323`); the `QUIT` loop (`:489-518`) moves each
  successful number out of `deleted` and sets its slot's `gone`,
  breaking on the first failure with the suffix unmarked;
  `resolve`/`counts`/`live_numbers` (`:554-595`) treat `gone` as
  "no such message"/skipped; `RSET` (`:479-488`) is unchanged code —
  now honest; the `QUIT` comment is rewritten to 0015 §1's words
  (committed prefix gone, failing suffix unmarked, nothing resurrects).
  *Test:* inline unit tests (branch-stable): `counts`/`resolve` with a
  gone slot; wire legs extending the partial-`QUIT` leg —
  `RETR` on the committed number → `-ERR no such message`, on the
  suffix → `-ERR message already deleted`, `RSET`'s summary agrees with
  what `RETR` serves, retry `QUIT` signs off. Proves AC15 (with 8) and
  AC17.
- [ ] **8. Close the POP3 session edges** — in `src/pop3.rs:295-303`:
  both `-ERR` branches of `USER` clear the pending name. Update
  `README.md`: the octet rule sentence (`:314-315`), the
  snapshot-not-lock sentence and the AUTH refusal sentence; module docs
  state the 0028 postures.
  *Test:* wire legs: `pass_after_a_failed_user_is_refused` (USER
  invalid → PASS → `-ERR USER required first`, not a login into the
  previous mailbox — deliver into the previous inbox first to prove
  nothing leaked); `mail_removed_elsewhere_is_reported_honestly`
  (clear an inbox over HTTP mid-session: `LIST`/`STAT` still name the
  snapshot, `RETR`/`TOP` → `-ERR no such message`). Proves AC14 and
  AC16.
- [ ] **9. Run the regression nets** — no code; the proof that nothing
  else moved.
  *Test:* `tests/starttls.rs`, `tests/p2.rs`, `tests/burst.rs`,
  `tests/rate.rs`, `tests/smtp_edges.rs` and `tests/pop3.rs` pass
  unmodified except where tasks 1-8 rewrote pins; the binary tests'
  `smtp=` startup-line contract stays plain when piped. Proves AC6.
- [ ] **10. Run the gates** — `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo test --workspace --locked`,
  `brokkr compile --bundle bundles/self`,
  `bash scripts/coverage-gate.sh`, `cargo deny check licenses`. The
  diff must show the zero-diff demand holding (`src/store.rs`,
  `src/persist.rs`, `src/model.rs`, `src/extract.rs`, `src/api.rs`,
  `src/mcp.rs`, `src/chaos.rs`, `src/threads.rs`, `src/compat.rs`,
  `src/webhook.rs`, `src/tls.rs`, `src/config.rs`, `src/feed.rs`,
  `src/stdio.rs`, `src/ui.rs`, `src/lib.rs`, `src/main.rs` untouched).
  *Test:* the gates themselves — exact 100/100/100, missed == 0.
  Proves AC18.
