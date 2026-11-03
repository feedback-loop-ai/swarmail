# 0116 — Implicit TLS (SMTPS) as an opt-in mode of the SMTP listener

- **Status:** accepted (operator, 2026-11-02 — "Accept 0116, 0117, 0118",
  given in chat to the dispatcher).
- **Date:** 2026-11-02
- **Scope:** issue #313 (operator request 2026-11-02): a sender configured
  for implicit TLS (`smtps://…?legacy_ssl=true`, Ory Kratos' courier on
  Alkemio's test cluster) can switch from MailSlurper to swarmail by
  changing its host only.
- **Specification:** [implicit TLS on the SMTP listener](../../specs/implicit-tls-smtp/spec.md).
- **Amends:** none. The ruling is additive. An unconfigured server is
  byte-identical to today, and with the mode on every accepted ruling holds
  as written, including 0027 §3's cap refusal, which stays byte-identical
  (see §5).
- **Preserves:** [0001](0001-synchronous-insert-lossless.md) (the `250`
  still follows the synchronous commit; the transport changes only where
  the socket becomes TLS), [0002](0002-awaits-subscribe-before-checking.md)
  (the park under the session's inbox and the waiters' order are
  untouched), [0003](0003-oldest-only-pruning.md) (one store, one
  oldest-only eviction), [0004](0004-per-test-inboxes-via-smtp-auth.md)
  (AUTH names the inbox, exactly as today), 0005 (extraction at ingest on
  the same decrypted octets), [0112](0112-recipient-inbox-routing.md)
  (routing is settled at `RCPT TO`, transport-blind), 0027/0036 (the
  per-listener cap and per-I/O idle bounds), 0028 (refusal codes of AUTH),
  0034 §5 (one AUTH per TLS epoch, and an implicit session is one epoch),
  0046 §1 (close posture: `close_notify` on a good end, drop on a failed
  one), 0030 (loopback default bind).
- **Number:** drafted as 0116, the next free number on main 5645bd9
  (0115 is the highest there). An unmerged branch (#148) also holds a
  `0116-*` record, and #314 may take a number first. Whichever lands
  second renumbers to the next free number at integration; no existing
  record is renumbered.

## Context

Swarmail speaks plain SMTP and STARTTLS (`--tls-cert`/`--tls-key`,
rustls/ring, `--tls-handshake-timeout-ms`). Implicit TLS is different: the
TLS handshake happens before the SMTP greeting (RFC 8314 §3.3). Kratos'
`legacy_ssl=true` asks for exactly that, on port 1025, and MailSlurper's
`smtps` image serves it on its ordinary SMTP port. A STARTTLS server cannot
answer such a client: the client sends a ClientHello where the server
expects to write `220`, and the session dies.

The design panel argued two positions (run
`issue-313-an-implicit-tls-smtps--237b16b1`). *Simplicity* argued for a mode
flag on the existing listener. *Robustness* argued for a separate
`--smtps-listen` listener with its own health surface and session cap. This
record is the chief's ruling between them.

## Decision

1. **A mode flag, not a second listener.** `--smtp-implicit-tls` (env
   `SWARMAIL_SMTP_IMPLICIT_TLS`), carried as `SmtpConfig::implicit_tls`,
   default off. When on, the SMTP listener at `--smtp-listen` speaks TLS
   from the first byte. Reasons:
   - The consumer dials `:1025`, swarmail's SMTP port. With the flag, only
     the host changes, which is what the issue asks for.
   - A second listener brings a new `Config` field (two dozen struct
     literals), a third accept loop and shutdown arm, a third `/healthz`
     surface, and a new ruling on whether 0027's cap is shared. It also
     needs a new port in the Dockerfile, chart Service and
     `scripts/chart-verify.sh`, which puts the PR in 0056's operator-held
     set. No consumer needs any of that.
   - The flag regresses no plaintext consumer. It is per process and off by
     default. Every test, the eval harness and the Docker smoke start their
     own servers without it.
2. **Same certificate, same deadline.** The mode reuses the
   `--tls-cert`/`--tls-key` pair. The pair is parsed once, before anything
   binds, exactly as today. It also reuses `--tls-handshake-timeout-ms`,
   which now bounds any SMTP TLS handshake, implicit or STARTTLS. There is
   no new certificate, version, cipher, SNI or ALPN setting.
3. **Fail closed without a pair.** The mode without a certificate pair is
   refused. `run_on` refuses it before any listener binds, with the error
   `--smtp-implicit-tls requires --tls-cert and --tls-key`. `smtp::serve`
   refuses the same combination with the same error for any in-process
   caller. A port that was asked to be TLS never serves plaintext.
4. **The session is today's session, entered TLS-active.** The handshake
   runs inside the spawned session task, after admission and under the
   handshake deadline. It never runs in the accept loop. On success the
   session starts with the TLS state *active*:
   - the `220` greeting goes out over TLS;
   - EHLO does not advertise `STARTTLS`;
   - `STARTTLS` gets `554 5.5.1 TLS already active` and changes nothing;
   - AUTH, `RCPT` routing, DATA, the park, the parse, the insert and the
     `250` run the existing code with no transport branch.

   On a handshake error or deadline, the socket is dropped and the session
   writes no SMTP byte. A plaintext client on the implicit port gets no SMTP
   reply. At most it gets the TLS alert rustls emits, and then the close.
   A `Connect` chaos rule is checked after the handshake, so an injected
   error line reaches the client as a TLS record and is never sent in
   cleartext.
5. **The cap refusal stays as 0027 §3 rules it.** Over the cap, the accept
   loop writes the plaintext `421 4.7.0 Too many connections, closing
   transmission channel` under the shared deadline, before any session
   state exists, and closes. That holds in implicit mode too. An implicit
   client reads those bytes as a failed handshake and retries, which is
   0027's contract: one honest connection refused, never a lost mail. The
   string is fixed and public, so nothing leaks.

   Two alternatives were considered and not taken:
   - Closing silently would change an accepted ruling's wire behaviour.
   - Completing a handshake to refuse inside TLS would put TLS work, and a
     stall, in the accept loop that the cap exists to protect.

   If the operator wants a silent close in implicit mode, that is a narrow
   amendment of 0027 §3 and belongs in its own record.
6. **Off by default, documented, exactly covered.** Without the flag, the
   listener and every other plane are byte-identical. The guide documents
   the mode in `docs/guide/smtp.md`. Every new production line is executed
   by a test under the exact coverage gate, with no `coverage(off)` and no
   new harness file.

## Consequences

- One process serves either STARTTLS or implicit TLS on its SMTP listener,
  not both. A deployment that needs both runs two processes, and those are
  two stores: mail does not cross between them. If a real consumer needs
  both modes over one store, a later additive decision can add a listener;
  this flag does not block that.
- An implicit-TLS client at the session cap sees a handshake failure
  instead of a readable `421`. It is still refused, closed and free to
  retry.
- A stalled or trickling handshake holds one capped slot for at most the
  deadline. That is the same exposure STARTTLS has today.
- `--tls-handshake-timeout-ms 0` with the mode on makes every connection
  fail before the greeting. It is accepted at startup, as it is today for
  STARTTLS, and the guide says so.
- The server never checks its own certificate's expiry. Tests generate
  their pair per run. Kratos' `skip_ssl_verify=true` relaxes only the
  client.
- No dependency, `Cargo.lock`, chart, Dockerfile, `scripts/`, `.github/` or
  `evals/` change. The chart's existing `config:` env map and `tls:` mount
  are the whole deployment story.

## Alternatives rejected

- **A separate `--smtps-listen` listener** (robustness position). It is the
  stronger shape when one process must serve both modes, and it allows a
  cross-transport cap test over one store. Its cost is item 1's list, for a
  need nobody has stated. The ruling costs that position the in-process
  "both at once", the per-listener health name `smtps`, and the
  cross-transport burst proof. 0003 is already proven transport-blind by
  the existing suites, because the store never sees a transport.
- **A `--smtp-tls-mode starttls|implicit` enum.** A bool covers the two
  states that exist.
- **Auto-detecting TLS by peeking the first byte.** SMTP is server-first.
  A plaintext client sends nothing, so detection needs a new
  wait-for-first-byte deadline in front of every greeting. It would also
  accept plaintext on a port configured for TLS.
- **Closing silently at the cap in implicit mode** (robustness position).
  It departs from accepted 0027 §3 (§5 above). It is left to the operator
  as a possible narrow amendment.
- **POP3S and implicit TLS for POP3.** POP3 has no TLS today, and nothing
  asks for it.

## Dated acceptance note — 2026-11-02

The operator accepted this record on 2026-11-02 ("Accept 0116, 0117,
0118"). It amends nothing, as the Amends line states, so no accepted
record gains a marker. The **Number** line is historical: this record
landed first, as `f549b14` (#317), #314 took 0118, and on main `58ad25b`
no other record holds 0116. The code merged while this record was
proposed, as compatible work under accepted 0045 §8.
