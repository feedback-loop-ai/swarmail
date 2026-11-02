# SMTP — accept is the truth

[← README](../../README.md) · [Guide index](README.md)

`:1025` — EHLO, AUTH PLAIN/LOGIN (accept-any; **username = inbox**),
PIPELINING, 8BITMIME, SMTPUTF8, SIZE, a 50 MiB cap. Every mail is inserted
synchronously on `DATA` completion — the `250` *is* the durability
contract, not an eventual promise. 8BITMIME is true: a DATA body is stored
as the exact octets it arrived as, and `/api/v1/messages/{id}/raw` serves
those same bytes back as `message/rfc822`. A message over the cap is
refused whole — `552 5.3.4 Message too big`, nothing stored, nothing
truncated — with the cap binding memory where the bytes arrive, not just
the accumulator; a protocol line over 4096 bytes — POP3's own line cap —
is drained and answered `500 5.5.2 Line too long`, and the session
continues.

**The session boundaries are exact**
([0034](../decisions/0034-data-terminator-boundary.md), *accepted*).
DATA ends only on CRLF `.` CRLF — a `.` CRLF line behind a bare-LF line is
payload, stored with its dot, and so is a `.` bare-LF line; a terminator
straight after the `354` is the empty message. An over-cap message drains
to that real boundary before its one `552`. A sender whose lines end in bare
LF only must still close with a CRLF-ended line before the dot, or the
session idles out with nothing stored. MAIL needs `FROM:<path>` and RCPT
needs `TO:<path>` — keyword case-insensitive, whitespace after the colon
allowed, angle brackets required; a bracketless, inverted or glued-on path
is `501`, `MAIL FROM:<>` is the null sender and `RCPT TO:<>` is refused.
A path that only looks like one is `501` too
([0046](../decisions/0046-smtp-pop3-edges.md), *accepted*): blank after
trimming whitespace (`< >`, `<\t>`, an ideographic space), or carrying any
control character — only the literal `<>` is null. Paths are otherwise
opaque and kept verbatim: local names (`<alice>`) and UTF-8 addresses are
kept, no `@` or DNS check is made. Two RFC 5321 forms are deliberately not
parsed: a quoted local part containing `>` (`<"a>b"@x.io>`) ends at its
first `>` and is refused, and a source route (`<@hop1,@hop2:bob@y.io>`) is
not stripped — the whole route is stored as the address. MAIL accepts `SIZE=<digits>` (a
declaration above the cap, however many digits, is `552` at MAIL; the real
bytes are still counted at DATA), `BODY=7BIT|8BITMIME` and bare `SMTPUTF8`;
a malformed or repeated one is `501`, any other MAIL parameter is `555`, and
RCPT takes no parameters (`555`). Refused MAIL and RCPT commands leave the
open transaction exactly as it was — they are checked before chaos rolls —
and HELO/EHLO reset the transaction like RSET (HELO answers one line). PLAIN
must be exactly `authzid NUL authcid NUL password`; only the authcid names
the inbox, the authzid is ignored and any password — empty included — is
accepted. The username (PLAIN's authcid, LOGIN's username) is the inbox
name verbatim, padding included, and a blank one or one carrying a control
character — a PLAIN-shaped `\0user\0password` sent to LOGIN included — is
`501 5.5.2` with nothing assigned. One AUTH succeeds per session (`503` after that, even as
`default`), surviving RSET and greetings; only a successful STARTTLS starts
afresh, clearing identity and envelope. A bare `AUTH` or surplus tokens is
`501`, an unknown mechanism `504`. Deliberately lenient: MAIL before any
greeting is accepted, and so are greetings with no hostname. A second AUTH
is the trap for clients that reuse one connection across inboxes — see
*One SMTP connection serves one inbox* in the [quickstart](quickstart.md).

**Commands are UTF-8; DATA is octets.** A command line — or an AUTH
continuation line — whose raw bytes are not valid UTF-8 ends the session
with **no reply**: the connection simply closes, and an open transaction is
dropped with it (a base64 AUTH value that *decodes* to invalid UTF-8 is a
`501`, as above). Non-ASCII mail content belongs inside DATA, which is
stored byte for byte; a non-ASCII address in MAIL/RCPT must be UTF-8.

**The listener never dies on a bad accept.** A transient `accept()` error
flags the listener degraded and backs the loop off (1 s); the flag clears
when the next accept is observed waiting or succeeds — an idle recovered
listener goes green without a peer, while back-to-back failures stay red
across the retry (the backoff clock alone never clears it). Waiting is an
observation, not a promise that a later accept succeeds. `/healthz` says
so: `200 {"status":"ok"}` when no listener is flagged, `503
{"status":"degraded","degraded":["smtp", ...]}` naming the flagged
surfaces, and `503 {"status":"stopping"}` for a request that reaches the
handler after the stop has fired — that state wins over everything else.
Sessions are bounded where the wire admits them: more than 256 concurrent
SMTP or POP3 sessions (the default of `--max-sessions`) is refused before any greeting (`421 4.7.0 Too many
connections, closing transmission channel` / `-ERR too many connections`);
completed sessions are reaped right after each accept, before the cap is
checked, so a slot freed while the listener waited is offered to the very
next peer. More than 100 recipients on one mail is refused `452 4.5.3 Too
many recipients` before anything is appended, and an idle session is
answered `421 4.4.2 idle timeout, closing transmission channel` (POP3:
`-ERR idle timeout`) and closed — SMTP 5 min, POP3 10 min, each its own
compiled default, never armed across a commit (a mid-DATA expiry stores
nothing, exactly as if the peer had hung up). A session that fails — a reply
write that expires included — drops its connection at once, without a
graceful close that could wait on the same stalled peer; a session that ended
well gets one more idle window for its close (TLS `close_notify`), so a
STARTTLS peer that stops reading frees its slot at the deadline. Every
accepted SMTP and POP3 socket sets `TCP_NODELAY` before its first byte, so a
pipelined group's replies are not held for the client's delayed ACK.

**The idle bounds are per read and per write, not a session lifetime**
(decision 0036, accepted). One incomplete read or write is cut off at the
bound, but a peer that completes a command or a DATA line inside each
window earns another window, indefinitely: there is no session lifetime,
no DATA lifetime and no minimum transfer rate. At the defaults,
complete-line tricklers can occupy all 256 SMTP slots indefinitely, and
each DATA connection can retain a payload buffer approaching 50 MiB —
about 12.5 GiB across 256 connections before allocation, parsing and
other overhead (an exposure estimate, not an RSS ceiling); an over-limit
DATA drain has no total time bound either while complete lines keep
arriving. POP3 command trickles can hold its 256 slots the same way. A POP3
reply has no whole-reply deadline either: its bound applies to each socket
write, and every write that moves bytes starts a fresh window, so a reader
whose socket accepts some bytes of each next write within its window
receives a 50 MiB RETR in full, and keeps its slot and that reply's buffer
for as long as it takes — at the defaults, 256 such replies are about
12.5 GiB of raw-message-scale exposure, not an RSS ceiling; a write that
moves nothing for a whole window cuts the reader off (decision 0046,
accepted).
Progress means bytes the local socket accepted, not bytes the client's
application consumed, so a reader that stopped is seen only once its
socket buffers fill. Command reads keep their whole-command bound and the
cap's refusal stays one bounded write. The
cap and the idle bounds are no availability guarantee against a
deliberately slow client.

**The stop is wired to both signals and has a documented exit vocabulary.**
`SIGINT` and `SIGTERM` take the same path: the first starts the graceful
drain (`shutting down` on stdout, the 10 s drain deadline raced against
graceful serve). HTTP graceful shutdown closes admission: a new probe
after that gets connection refusal, not a `stopping` body; an already
accepted connection gets `503 {"status":"stopping"}` only if its request
reaches the handler after the stop, and may close without a response. A
second signal during the drain is the operator's
explicit "now" — a forced `exit(70)`, never a disposition accident. The
exit codes are **0** graceful, **70** forced; the image declares
`STOPSIGNAL SIGTERM` so `docker stop` rides the graceful path.

**STARTTLS** is real: with `--tls-cert`/`--tls-key` (or the env pairs), EHLO
advertises `STARTTLS`, the session upgrades with rustls/ring, and the
session restarts per RFC 3207 §4.2 — plaintext pipelining before the
handshake is discarded, every further command is TLS-only. A stalled
handshake is dropped after `--tls-handshake-timeout-ms` (default 10 s).
The value is not range-checked: `0` (or `SWARMAIL_TLS_HANDSHAKE_TIMEOUT_MS=0`)
is accepted at startup, and then every STARTTLS handshake fails — the
client gets `220 2.0.0 Ready to start TLS` and the connection is dropped
before any handshake can finish (with `--smtp-implicit-tls`, below, every
connection fails before the greeting), so use a positive value. Plaintext
sessions answer to the per-I/O idle bounds above (5 min SMTP,
10 min POP3, decisions 0027 and 0036). Without a cert the
plaintext listener is byte-identical to the one without the flag.

```
$ swarmail gen-cert --domain localhost --out ./certs    # cert.pem + key.pem, 0600
$ swarmail serve --tls-cert ./certs/cert.pem --tls-key ./certs/key.pem
```

### Implicit TLS (SMTPS)

Some senders are configured for TLS *before* the greeting (RFC 8314 §3.3):
Ory Kratos' courier with `smtps://…?legacy_ssl=true`, MailSlurper's `smtps`
image. They cannot talk to a STARTTLS server. `--smtp-implicit-tls` (env
`SWARMAIL_SMTP_IMPLICIT_TLS=true`, decision 0116) switches the SMTP listener
to that mode. It is **off by default**; without it the listener is
byte-identical to the one above.

- **It needs the certificate.** It reuses `--tls-cert`/`--tls-key` and
  refuses to start without the pair —
  `--smtp-implicit-tls requires --tls-cert and --tls-key`, before anything
  binds. A port asked to be TLS never serves plaintext.
- **The whole listener changes, and only the transport.** The SMTP listener
  (`--smtp-listen`, port 1025 by default) runs the TLS handshake on connect
  and the `220` greeting follows inside TLS. `STARTTLS` is not advertised
  and is answered `554 5.5.1 TLS already active`. AUTH-named inboxes,
  `--route-domain` routing, `await`, extraction and the `250`-means-stored
  guarantee behave exactly as over plaintext. There is one mode per
  process: STARTTLS *or* implicit TLS, never both on the one listener. A
  deployment that needs both runs two servers, and those are two stores.
- **Same handshake deadline.** `--tls-handshake-timeout-ms` (default 10 s)
  bounds this handshake too: a client that connects and goes silent is
  dropped at the deadline with no byte written. As above, `0` is accepted
  but makes every connection fail before the greeting.
- **Plaintext on the port gets no SMTP reply.** A client that sends `EHLO`
  instead of a ClientHello is closed when the handshake fails — at most it
  receives a TLS alert record, never a `220`.
- **At the session cap** (`--max-sessions`) the refusal is the unchanged,
  plaintext `421 4.7.0 Too many connections, closing transmission channel`
  (decision 0027 §3). An implicit-TLS client reads it as a failed handshake
  and retries; the mail was refused, never lost.

The certificate is the one `gen-cert` mints; Kratos' `skip_ssl_verify=true`
relaxes only the client, so a self-signed pair works for it.

```
$ swarmail gen-cert --domain localhost --out ./certs
$ swarmail serve --smtp-implicit-tls --tls-cert ./certs/cert.pem --tls-key ./certs/key.pem
# Kratos courier: smtps://user:pass@<host>:1025/?skip_ssl_verify=true&legacy_ssl=true
```

On the Helm chart, mount the pair with `tls.enabled: true` and
`tls.secretName`, and set the flag through the env map:
`config: {SWARMAIL_SMTP_IMPLICIT_TLS: "true"}`.
