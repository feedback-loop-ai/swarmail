# Implicit TLS (SMTPS) on the SMTP listener

Decision: 0116 — [Implicit TLS (SMTPS) as an opt-in mode of the SMTP listener](../../docs/decisions/0116-implicit-tls-smtp.md) (proposed)

- **Issue:** #313 (operator request 2026-11-02, "start now").
- **Route:** full delivery on `claude-careful` (accepted 0098/0099; review
  residual rework per accepted 0102 and its spec-defect addendum).
- **Base:** main 5645bd9.
- **Authorities:** accepted [0001](../../docs/decisions/0001-synchronous-insert-lossless.md),
  [0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md),
  [0003](../../docs/decisions/0003-oldest-only-pruning.md),
  [0004](../../docs/decisions/0004-per-test-inboxes-via-smtp-auth.md),
  [0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md),
  [0027](../../docs/decisions/0027-living-listeners-bounded-sessions.md),
  [0034](../../docs/decisions/0034-data-terminator-boundary.md),
  [0046](../../docs/decisions/0046-smtp-pop3-edges.md),
  [0112](../../docs/decisions/0112-recipient-inbox-routing.md).

## What

An opt-in **implicit-TLS mode** for swarmail's existing SMTP listener. With
`--smtp-implicit-tls` (env `SWARMAIL_SMTP_IMPLICIT_TLS`) and the existing
`--tls-cert`/`--tls-key` pair, the listener at `--smtp-listen` runs the
rustls handshake before it writes anything. The `220` greeting and every
reply after it travel inside TLS, as RFC 8314 §3.3 describes for
submission over implicit TLS. Without the flag, nothing changes.

## Why

Some senders are configured only for implicit TLS. The concrete consumer
is Ory Kratos' courier on Alkemio's test cluster:
`smtps://<user>:<pass>@<host>:1025/?skip_ssl_verify=true&legacy_ssl=true`.
`legacy_ssl=true` means implicit TLS, and MailSlurper's `smtps` image serves
it on its ordinary SMTP port. Swarmail offers only STARTTLS today, so such a
sender cannot switch to swarmail without also changing its own config. With
this mode, the sender changes its host and nothing else. `1025` is already
swarmail's SMTP port.

## The ruling between the panel's positions

The panel argued two positions:
`.brokkr/state/design/positions/simplicity.md` (a mode flag) and
`.brokkr/state/design/positions/robustness.md` (a separate `--smtps-listen`
listener). The chief rules as follows. Decision 0116 records the full
reasoning.

| Question | Ruling | Source | What the losing side costs |
|---|---|---|---|
| Separate listener or mode flag | **Mode flag** on the existing listener | simplicity | One process cannot serve STARTTLS and implicit TLS at once, and two processes means two stores. There is no `smtps` health name and no in-process cross-transport cap race. |
| Shape of the setting | `SmtpConfig::implicit_tls: bool`, refused fail-closed without a pair by **both** `run_on` (before binding) and `smtp::serve` (any caller), through one shared check | simplicity's field and robustness's fail-closed demand | The illegal state can still be written in a `Config`; it is refused at runtime rather than made uncompilable by robustness's `IncomingTls` enum. This avoids churning every `serve` call site. |
| Handshake placement | Inside the spawned session, after admission, under `tls_handshake_timeout` | both | — |
| Chaos `Connect` ordering in implicit mode | Handshake → chaos → greeting | both | — |
| Plaintext on the TLS port | Dropped after rustls refuses the ClientHello; **no SMTP reply** | both | — |
| Cap refusal in implicit mode | **Byte-identical to today**: the plaintext `421 4.7.0 …` that accepted 0027 §3 rules | the accepted ruling, over robustness | An implicit client at the cap sees a failed handshake, not a readable 421. Robustness's silent close would amend 0027 §3 and is left to the operator. |
| Health and session cap | Unchanged: one SMTP listener, one `smtp` health flag, one cap | simplicity | — |
| Test home | Extend `tests/starttls.rs`, which holds the TLS fixtures | simplicity | No new suite file. |
| Startup token | `smtp=<addr>` unchanged; no new token | simplicity | — |
| Proofs of 0002, 0005 and hostile bytes over TLS | **Adopted** from robustness, folded into the delivery and await tests | robustness | — |
| Cross-transport burst into one inbox (0003) | **Not adopted.** It cannot be expressed with one mode per process, and the store never sees the transport | simplicity | — |

## Behaviour

**B1. The setting.** `--smtp-implicit-tls` is a boolean flag. The env
variable `SWARMAIL_SMTP_IMPLICIT_TLS` accepts `true`/`1` (on) and
`false`/`0`/unset (off). The default is off. It maps to
`SmtpConfig::implicit_tls` (default `false`).

**B2. It requires the pair.** If the mode is on and no
`--tls-cert`/`--tls-key` pair is configured, `run_on` returns the error
`--smtp-implicit-tls requires --tls-cert and --tls-key` before any listener
binds. The binary exits non-zero with that message. If `smtp::serve` is
called with the mode on and no TLS config, it returns the same error
without accepting a connection. A malformed pair is refused exactly as
today.

**B3. TLS from the first byte.** Each admitted connection completes a
rustls handshake with the configured pair before the session writes any
byte. The handshake is bounded by `tls_handshake_timeout`
(`--tls-handshake-timeout-ms`, default 10 s). If the deadline elapses or
rustls returns an error, the socket is dropped. No SMTP byte is written,
and the listener keeps serving.

**B4. The session enters TLS-active.** After the handshake, the existing
session runs with STARTTLS state `Active`. Concretely:
- the `220 <hostname> …` greeting travels over TLS;
- the EHLO reply lists every extension it lists today *except*
  `STARTTLS`;
- `STARTTLS` is answered `554 5.5.1 TLS already active` and does not reset
  identity or envelope;
- AUTH PLAIN/LOGIN assign the inbox exactly as today (0004, 0028), and a
  second successful AUTH in the session is `503` (0034 §5);
- recipient routing under `--route-domain` (0112), MAIL/RCPT/DATA, the
  park, the parse at ingest (0005), the synchronous insert and the `250`
  after commit (0001) run unchanged;
- per-I/O idle bounds (0027/0036) and the close posture (0046 §1) apply
  unchanged.

**B5. Chaos `Connect` ordering.** In implicit mode, a `Connect` chaos rule
is checked after the handshake. An injected error line therefore reaches
the client as TLS application data and never as cleartext. In
STARTTLS-capable and plain modes, the ordering is unchanged.

**B6. Cap refusal unchanged.** Over `--max-sessions`, the accept loop
behaves byte-for-byte as accepted 0027 §3 rules, in every mode. It writes
`421 4.7.0 Too many connections, closing transmission channel` under the
shared deadline, before any session state, and closes. The guide documents
that an implicit client experiences this as a failed handshake and
retries.

**B7. Off means byte-identical.** Without the flag, the SMTP listener,
STARTTLS, POP3, HTTP, `/healthz`, the startup line and every existing test
behave exactly as on 5645bd9. The flag changes nothing else:
- no new listener, port, health surface or startup token;
- no chart, Dockerfile, `scripts/`, `.github/`, `evals/`, `Cargo.lock` or
  `deny.toml` change;
- no new dependency (`rustls`, `tokio-rustls`, `rustls-pemfile` and `rcgen`
  are already in the tree).

**B8. Wording.** Two kinds of text say "STARTTLS handshake" where the
deadline now bounds any SMTP TLS handshake: the help text of
`--tls-handshake-timeout-ms` and `--tls-cert`, and the doc comments of
`SmtpConfig::tls_handshake_timeout` and the module. They are corrected to
say "TLS handshake (STARTTLS or implicit)" and "STARTTLS, or implicit TLS
with `--smtp-implicit-tls`".

**B9. Documentation.** `docs/guide/smtp.md` gains an "Implicit TLS
(SMTPS)" subsection after STARTTLS. It covers:
- the flag and env, off by default;
- the required pair;
- that the setting is whole-listener (no STARTTLS offered; one mode per
  process);
- the shared handshake deadline, and the `0` caveat;
- that plaintext on the port is dropped without a reply;
- the cap-refusal consequence;
- the Kratos URL shape with `swarmail gen-cert`;
- the chart recipe: `config: {SWARMAIL_SMTP_IMPLICIT_TLS: "true"}` plus
  `tls.enabled`/`tls.secretName`.

`docs/decisions/README.md` gains 0116's index row, status `proposed`.

## Acceptance Criteria

Each criterion is proven over the real wire. The client is a real rustls
`TlsConnector` that trusts the per-run generated certificate. The server
is a real `run_on` server. There are no mocks (0004, 0020 rule 3).

- **AC1: delivery to the AUTH-named inbox, accept == stored.** A client
  starts TLS on connect (no STARTTLS) and reads `220` over TLS. It
  AUTHs as a per-test inbox and sends MAIL, RCPT and DATA. The DATA body
  holds a byte `0xFF`, a NUL and a dot-stuffed line. The client receives
  `250`. Immediately after the `250`, with no wait or retry, the HTTP API
  returns exactly one message in that inbox. The message's raw bytes equal
  the sent payload after dot-unstuffing (0001, 0004, 0034), and its
  extracted fields (subject, from, a link in the body) are present on the
  stored record (0005).
- **AC2: STARTTLS is not advertised and is refused inside implicit TLS.**
  The EHLO reply over implicit TLS lists the same extensions as a plain
  listener's EHLO, with no `STARTTLS` line. In the same session,
  `STARTTLS` answers `554 5.5.1 TLS already active` and a second `AUTH`
  answers `503`. A following delivery without re-AUTH still lands in the
  AUTH-named inbox, which proves identity was not reset.
- **AC3: recipient routing is unchanged (0112).** With `route_domain` set
  and AUTH as inbox `a`, a delivery over implicit TLS to
  `x@<label>.<domain>` is stored in inbox `<label>` and not in `a`.
- **AC4: await subscribes before it checks (0002).** A long-poll `await` on
  a fresh inbox is issued over HTTP before the delivery. The delivery then
  happens over implicit TLS. The await returns that message. No sleep is
  used as synchronization; the await's own deadline bounds the test.
- **AC5: a stalled handshake is dropped at the timeout.** With
  `tls_handshake_timeout` set short (e.g. 750 ms), a raw TCP client
  connects and sends nothing. A read well inside the deadline (deadline/5)
  is still pending. The connection then closes with EOF, having received
  zero bytes, well before 5× the deadline. Afterwards a fresh implicit-TLS
  client delivers successfully.
- **AC6: a plaintext client on the TLS port is refused.** With a long
  handshake timeout (≥ 10 s), a raw TCP client sends `EHLO x\r\n`. The
  connection closes before the deadline. The bytes read back contain no
  SMTP reply: they are either empty or a TLS alert record (first byte
  `0x15`), and they never contain `220`. A following implicit-TLS client
  delivers successfully.
- **AC7: Connect chaos travels inside TLS.** A `Connect` chaos rule with an
  error line, at probability 100 (0020 rule 5), is installed. An
  implicit-TLS client completes the handshake and reads the injected error
  line decrypted. A raw TCP client on the same port never receives the
  error text in cleartext.
- **AC8: the cap refusal is 0027's, unchanged.** With `max_sessions = 1`,
  one implicit-TLS session is held open. A second raw TCP connection reads
  exactly `421 4.7.0 Too many connections, closing transmission channel\r\n`
  and then EOF. A real TLS client in the same position fails its handshake
  and does not hang. After the held session QUITs, a new implicit-TLS
  client delivers. The cap test synchronizes on the wire with no sleep, as
  the existing cap tests do.
- **AC9: the mode is refused without a pair, fail-closed.**
  - `run_on` with `smtp.implicit_tls = true` and no pair returns `Err`
    whose message is exactly
    `--smtp-implicit-tls requires --tls-cert and --tls-key`, and no server
    starts.
  - The binary `swarmail serve --smtp-implicit-tls` without a pair exits
    non-zero with that message on stderr.
  - An inline unit test calls `smtp::serve` with `implicit_tls = true` and
    `tls = None`. It returns that error without accepting a connection.
- **AC10: the setting maps from CLI and env, off by default.**
  - `swarmail serve` alone yields `smtp.implicit_tls == false`.
  - `--smtp-implicit-tls` yields `true`.
  - `SWARMAIL_SMTP_IMPLICIT_TLS=true` yields `true`.
  - `SWARMAIL_SMTP_IMPLICIT_TLS=false` yields `false`.
- **AC11: off means byte-identical.** Every existing test, unmodified apart
  from the ServeArgs literal in `tests/config_cli.rs` gaining the new field,
  passes. That includes the whole of `tests/starttls.rs`, the plain-listener
  byte-identity test and the eval suite against the candidate binary.
- **AC12: documentation.**
  - `docs/guide/smtp.md` has an "Implicit TLS (SMTPS)" section covering
    every point of B9.
  - The `--tls-handshake-timeout-ms` and `--tls-cert` help text no longer
    limit the handshake to STARTTLS.
  - `docs/decisions/README.md` lists 0116 as `proposed`.
  - `cargo test` (including `tests/store_contract_docs.rs`) passes.
- **AC13: gates.** Every house gate in `docs/house-rules.md` § Gates passes
  `--locked` on the final tree:
  - the exact coverage gate (100% lines, branches and functions, no
    `coverage(off)`, no new harness file) and its selftest;
  - publish-guard and seal-branch proofs;
  - `cargo deny check licenses`;
  - the evals crate's fmt, clippy and tests against the candidate binary;
  - `brokkr compile`.

  `Cargo.lock` and `evals/Cargo.lock` gain no package.

## Out of scope

- A second SMTP listener, or STARTTLS and implicit TLS together in one
  process.
- Auto-detecting the protocol.
- A TLS mode enum.
- POP3S.
- Separate SMTPS certificate, version, cipher, SNI, ALPN or client-certificate
  settings.
- Certificate-expiry checks.
- Changing 0027 §3's cap refusal in implicit mode (an operator question;
  see 0116 §5).
