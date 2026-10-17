<p align="center"><img src="assets/logo.svg" width="132" alt="Swarmail — an open envelope under the feedback-loop pulse: two resting dots and one live one"></p>

# Swarmail

[![CI](https://github.com/feedback-loop-ai/swarmail/actions/workflows/ci.yml/badge.svg)](https://github.com/feedback-loop-ai/swarmail/actions/workflows/ci.yml)
[![release](https://img.shields.io/github/v/release/feedback-loop-ai/swarmail?label=release&color=blue)](https://github.com/feedback-loop-ai/swarmail/releases/latest)
[![license: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)
[![coverage · literal 100%](https://img.shields.io/badge/coverage%20%C2%B7%20literal%20100%25-brightgreen)](scripts/coverage-gate.sh)
[![deps · permissive-only](https://img.shields.io/badge/deps%20%C2%B7%20permissive--only-brightgreen)](deny.toml)
[![platforms](https://img.shields.io/badge/linux%20%C2%B7%20macos-blue)](.github/workflows/ci.yml)
[![rust](https://img.shields.io/badge/rust-stable-orange)](Cargo.toml)

**Email infrastructure for tests and agents.** One Rust binary: a lossless
SMTP mock with per-test inboxes, chaos engineering, push-based waiting,
native MCP for AI agents, and a live web UI — swallowing mail faster than
your tests can produce it.

*The swarm never drops a letter.*

> **Accept is the truth.** An SMTP `250` means the mail is queryable — and,
> with a data file, already on disk. There is no async gap where mail can
> vanish, because there is no async gap.

[The landing page](docs/index.html) · machine-readable surfaces:
[`/openapi.json`](http://localhost:8025/openapi.json) ·
[`/llms.txt`](http://localhost:8025/llms.txt) · `/metrics` · `/healthz`

## Contents

- [The name](#the-name)
- [What it does](#what-it-does)
- [The guarantees](#the-guarantees)
- [60-second quickstart](#60-second-quickstart)
- [Kubernetes (Helm)](#kubernetes-helm)
- [The surfaces](#the-surfaces)
  - [SMTP — accept is the truth](#smtp--accept-is-the-truth)
  - [The REST API — await, don't poll](#the-rest-api--await-dont-poll)
  - [MCP — agents drive the mail](#mcp--agents-drive-the-mail)
  - [POP3 — the same store, RFC 1939](#pop3--the-same-store-rfc-1939)
  - [MailHog / Mailpit shims](#mailhog--mailpit-shims)
  - [Threads and the live UI](#threads-and-the-live-ui)
  - [Webhooks](#webhooks)
  - [Chaos — break SMTP on purpose](#chaos--break-smtp-on-purpose)
  - [Persistence — the restart copy](#persistence--the-restart-copy)
- [Benchmarks](#benchmarks)
- [Repo layout](#repo-layout)
- [The decision culture](#the-decision-culture)
- [Testing & coverage](#testing--coverage)
- [Roadmap](#roadmap)
- [License](#license)

## The name

A **swarm** is a thousand independent agents with no coordinator and no
single point of loss. That is exactly what a test fleet looks like: fifty
parallel CI jobs, three AI agents, one flaky retry loop — every one of them
convinced it is the only mail user in the world. Swarmail's whole contract
is that each of them gets its own inbox (the AUTH username *is* the inbox)
and that the swarm, as a whole, never drops a letter.

The name is also the promise: mail *in* swarm quantities — 5000 mails over
100 concurrent fresh sessions, exact — and mail *out* to whoever waits for
it: a test's `await`, an agent's `swarmail_wait_for_email`, a webhook sink,
or a human watching the inbox update live.

**The binary is `swarmail`.** No services, no database to babysit, no Node
runtime — a static binary and a scratch container about its size.

## What it does

You point any SMTP client at it — Ory Kratos's courier, Nodemailer, a bare
`smtplib` — and mail lands in a queryable store with everything the
assertion will need already extracted: the links, the OTP-like codes, the
thread headers. Tests wait with `await` (push-based, subscribes before it
checks); agents drive the same store over MCP; existing MailHog/Mailpit
clients work through the shims; a human watches the inbox update itself.

```
docker run -p 1025:1025 -p 1110:1110 -p 8025:8025 ghcr.io/feedback-loop-ai/swarmail:v0.1.0
swarmail serve                 # or: cargo run -- serve — same ports on localhost
swarmail mcp                   # the stdio MCP bridge
swarmail gen-cert --domain localhost --out ./certs
```

Twelve MCP tools, one REST API, two wire protocols in (SMTP, POP3), one
compatibility layer (MailHog/Mailpit), one observability story
(`/metrics`, `/healthz`, `/openapi.json`, `/llms.txt`) — and a web UI with
zero frontend dependencies.

**The binds are loopback by default, and the loopback plane refuses
cross-origin pages and rebound hosts.** All three listeners default to
`127.0.0.1` — a mail mock serves the machine it runs on — and the image
and the chart state `0.0.0.0` explicitly, because a container that
trusted its defaults would never publish a port. While the HTTP listener
is loopback-bound, two headers are checked before the router. **Host**:
a request whose `Host` is not a loopback spelling of the bound HTTP port
is answered `403` — this is the DNS-rebinding defense, because a rebound
page sends *same-origin* requests (and same-origin GET/HEADs carry no
`Origin` at all) while its `Host` line still spells the attacker's
hostname. **Origin**: a present `Origin` that is not a loopback spelling
is answered `403` — the one header a browser sets honestly, so a
cross-site drive is named and refused. Every method and every route is
covered, `/mcp` and the UI included; requests without an `Origin` — curl,
agents, the stdio bridge, every test harness — pass untouched (their
`Host` is the loopback spelling they dialed), and an operator who binds
`0.0.0.0` states the trust themselves (the guard is inert, not the
posture weaker). There is no auth on any surface, by design: the loopback
posture is the boundary, and a server with secrets would need auth —
this one holds test mail, and its knobs are local (`set-chaos`
included). Webhooks dial out only to addresses an operator explicitly
configures — **and that is the whole boundary**: an arbitrary or
internal address is accepted at registration and dialed from this
process, so on a network with sensitive internal services the operator
should spell loopback or otherwise-sanctioned targets (no allowlist, no
auth on the registration API — the posture is the trust statement, and
the SSRF exposure of an internal-target registration is recorded in
decision 0030 §5 rather than papered over). The delivery budget is
`WEBHOOK_DELIVERY_CAP` (32) **per normalized target URL** — many
configured targets multiply the ceiling, and there is no global
backstop across targets; nothing ever dials an address learned from
mail content.

**The pages are bounded.** Every result page — the REST list, `await`,
the MailHog/Mailpit shims, and every MCP search/get-latest/wait — is
capped at **250 items** (`limit`/`count` above the cap is refused with a
400 on REST and the shims, a `-32602` error over MCP: named, never
clamped). `await`'s exact-count `assert` keeps its unclamped semantics —
its `limit` is parsed for parity with `list` but inert, because the
assert answer is a count, not a page. SMTP request parsing and the REST
list/await/compat page copies run off the async workers on the blocking
pool, so a 50 MiB mail cannot stall the accept loop or the await wakes
it gates; the MCP `swarmail_seed_email` seed build and the SSE
thread-view serialization still run inline (the seed is the test
fixture's own path, and the thread view is a per-request read render —
neither is on the ingest path the stall guards).

## The guarantees

These are the frozen guarantees ([decision records](docs/decisions/README.md)
bind them; review refuses a change that weakens one, no matter how green
the tests are):

1. **Accept == stored** ([0001](docs/decisions/0001-synchronous-insert-lossless.md)).
   `Store::insert` is synchronous; an SMTP `250` means the mail is queryable,
   and with `--data-file` it is committed with fsync-per-accept before the
   reply leaves.
2. **`await` subscribes before it checks** ([0002](docs/decisions/0002-awaits-subscribe-before-checking.md)).
   Nothing is missed while connecting; no sleep-polling exists anywhere in
   the product.
3. **Pruning is oldest-only, never mid-delivery**
   ([0003](docs/decisions/0003-oldest-only-pruning.md)). The per-inbox cap
   (default 100k, `0` = unlimited) evicts the oldest mail only.
4. **The AUTH username is the inbox**
   ([0004](docs/decisions/0004-per-test-inboxes-via-smtp-auth.md)). Free
   parallel isolation for every test and agent; no config to route it.
5. **Extraction happens at ingest, not query time**
   ([0005](docs/decisions/0005-extraction-at-ingest.md)). Links, codes and
   thread headers are parsed once, when the mail is accepted; queries
   re-parse nothing.

## 60-second quickstart

One binary, three ports, no config. **Docker:**

```
docker run -p 1025:1025 -p 1110:1110 -p 8025:8025 ghcr.io/feedback-loop-ai/swarmail:v0.1.0
```

**From source:** `cargo run -- serve` (Rust stable; everything else is vendored).

**Then loop.** This transcript is real — a stock `smtplib` delivering, the
REST surface answering:

```
$ curl -X POST localhost:8025/api/v1/inboxes/quickstart/seed \
    -H 'content-type: application/json' \
    -d '{"from":"noreply@x.io","to":"ada@x.io","subject":"Verify your account",
         "text":"Welcome! Verify at https://x.io/verify?token=abc123 — code 424242 is your fallback."}'
{"id":"01a1261e-…","inbox":"quickstart", …,
 "links":["https://x.io/verify?token=abc123"], "codes":["424242"]}      # extraction ran

$ python3 - <<'EOF'                                    # a real client, really delivered
import smtplib
s = smtplib.SMTP('localhost', 1025, timeout=10)
s.sendmail('kratos@x.io', ['ada@x.io'],
           "From: kratos@x.io\r\nTo: ada@x.io\r\nSubject: Reset your password\r\n\r\n"
           "Reset: https://x.io/reset?token=xyz789 or use 998877.")
s.quit()
EOF

$ curl "localhost:8025/api/v1/inboxes/quickstart/await?to=ada@x.io&count=1&timeout_ms=5000"
{
  "matched": 1,
  "total": 1,
  "emails": [ { "subject": "Reset your password",
                "links": ["https://x.io/reset?token=xyz789"],
                "codes": ["998877"] } ]                 # no regex over HTML — fields
}
```

`await` blocks until the mail arrives (or the timeout answers `matched: 0`)
— push-based, no polling. Fixtures without SMTP: `…/seed` runs the full
parse + extraction pipeline. Per-test isolation needs no config: the AUTH
username names the inbox.

Environment knobs: `SWARMAIL_SMTP_LISTEN` (default `127.0.0.1:1025`),
`SWARMAIL_POP3_LISTEN` (`127.0.0.1:1110`), `SWARMAIL_HTTP_LISTEN`
(`127.0.0.1:8025` — the image and the chart set `0.0.0.0`, see the
posture above), `SWARMAIL_MAX_PER_INBOX` (`100000`, `0` = unlimited),
`SWARMAIL_URL` (the MCP stdio bridge's upstream, default
`http://127.0.0.1:8025`),
`SWARMAIL_TLS_CERT`/`SWARMAIL_TLS_KEY`,
`SWARMAIL_TLS_HANDSHAKE_TIMEOUT_MS` (`10000`), `SWARMAIL_DATA_FILE`.
Logging: `RUST_LOG` (env-filter syntax — `serve` logs to stdout, every
other subcommand to stderr so its stdout stays protocol-clean; default
`swarmail=info,tower_http=off`).

## Kubernetes (Helm)

A shared namespace mailbox, an ephemeral per-run instance, an agent-sidecar —
the chart packages the same binary with no configuration language of its own
([decision 0008](docs/decisions/0008-kubernetes-deployment-contract.md),
*proposed*):

```
helm install swarmail oci://ghcr.io/feedback-loop-ai/charts/swarmail \
  --version 0.1.0 --namespace swarmail --create-namespace
helm test swarmail -n swarmail        # the /healthz hook, from a curl pod
```

The chart adds nothing the binary does not read — configuration is exactly
the env contract, passed through `config: {SWARMAIL_MAX_PER_INBOX: "0"}`
or `envExtra`. One Service carries all three ports, so one DNS name serves
every client; the optional Ingress exposes only the HTTP surface (SMTP and
POP3 are plain TCP). The contract worth knowing before you scale it — you
cannot: the chart refuses a `replicaCount` above 1 at render time, naming
the contract ([0008 §4](docs/decisions/0008-kubernetes-deployment-contract.md)):

| Turn on | What it changes |
|---|---|
| `replicaCount` | must stay 1 — a second pod would fork the mailboxes or mount the same RWO claim twice; anything above 1 fails `helm template` |
| `persistence.enabled` | SQLite on an RWO PVC (`/data/swarmail.db`) — the strategy is **Recreate at every setting** ([0008 §5](docs/decisions/0008-kubernetes-deployment-contract.md)): a rolling surge would run two pods behind one Service, and the store is single-replica by design ([0001](docs/decisions/0001-synchronous-insert-lossless.md)) |
| `ingress.enabled` | the UI/API/MCP surface on your ingress class; SMTP/POP3 stay cluster-internal |
| `serviceMonitor.enabled` | a prometheus-operator ServiceMonitor on `/metrics` — **fails closed with a named refusal** if the CRDs are absent |
| `networkPolicy.enabled` | same-namespace ingress on the three ports only |
| `image.tag` / `image.pullPolicy` | defaults to the chart's `appVersion` and `IfNotPresent` |

`helm template` output is valid plain manifests — Kustomize and raw-YAML
teams are first-class, not gated behind the chart. The chart is verified
offline by `scripts/chart-verify.sh` (lint + both profiles + contract
assertions), which is also the CI `chart` job.

## The surfaces

### SMTP — accept is the truth

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
([0034](docs/decisions/0034-data-terminator-boundary.md), *proposed*).
DATA ends only on CRLF `.` CRLF — a `.` CRLF line behind a bare-LF line is
payload, stored with its dot, and so is a `.` bare-LF line; a terminator
straight after the `354` is the empty message. An over-cap message drains
to that real boundary before its one `552`. A sender whose lines end in bare
LF only must still close with a CRLF-ended line before the dot, or the
session idles out with nothing stored. MAIL needs `FROM:<path>` and RCPT
needs `TO:<path>` — keyword case-insensitive, whitespace after the colon
allowed, angle brackets required; a bracketless, inverted or glued-on path
is `501`, `MAIL FROM:<>` is the null sender and `RCPT TO:<>` is refused.
Paths are otherwise opaque: local names (`<alice>`) and UTF-8 addresses are
kept, no `@` or DNS check is made. MAIL accepts `SIZE=<digits>` (a
declaration above the cap, however many digits, is `552` at MAIL; the real
bytes are still counted at DATA), `BODY=7BIT|8BITMIME` and bare `SMTPUTF8`;
a malformed or repeated one is `501`, any other MAIL parameter is `555`, and
RCPT takes no parameters (`555`). Refused MAIL and RCPT commands leave the
open transaction exactly as it was — they are checked before chaos rolls —
and HELO/EHLO reset the transaction like RSET (HELO answers one line). PLAIN
must be exactly `authzid NUL authcid NUL password`; only the authcid names
the inbox, the authzid is ignored and any password — empty included — is
accepted. One AUTH succeeds per session (`503` after that, even as
`default`), surviving RSET and greetings; only a successful STARTTLS starts
afresh, clearing identity and envelope. A bare `AUTH` or surplus tokens is
`501`, an unknown mechanism `504`. Deliberately lenient: MAIL before any
greeting is accepted, and so are greetings with no hostname.

**The listener never dies on a bad accept.** A transient `accept()` error
backs the loop off (1 s) and clears on the next success, and `/healthz`
says so honestly: `200 {"status":"ok"}` when every listener accepts, `503
{"status":"degraded","degraded":["smtp", ...]}` naming the surfaces that
cannot, and `503 {"status":"stopping"}` once the stop has fired — that
state wins over everything else. Sessions are bounded where the wire
admits them: more than 256 concurrent SMTP or POP3 sessions is refused
before any greeting (`421 4.7.0 Too many connections, closing transmission
channel` / `-ERR too many connections`), a freed slot is offered to the
next peer, more than 100 recipients on one mail is refused `452 4.5.3 Too
many recipients` before anything is appended, and an idle session is
answered `421 4.4.2 idle timeout, closing transmission channel` (POP3:
`-ERR idle timeout`) and closed — SMTP 5 min, POP3 10 min, never armed
across a commit (a mid-DATA expiry stores nothing, exactly as if the
peer had hung up).

**The stop is wired to both signals and has a documented exit vocabulary.**
`SIGINT` and `SIGTERM` take the same path: the first starts the graceful
drain (`shutting down` on stdout, the 5 s drain deadline raced against
graceful serve), and a second signal during the drain is the operator's
explicit "now" — a forced `exit(70)`, never a disposition accident. The
exit codes are **0** graceful, **70** forced; the image declares
`STOPSIGNAL SIGTERM` so `docker stop` rides the graceful path.

**STARTTLS** is real: with `--tls-cert`/`--tls-key` (or the env pairs), EHLO
advertises `STARTTLS`, the session upgrades with rustls/ring, and the
session restarts per RFC 3207 §4.2 — plaintext pipelining before the
handshake is discarded, every further command is TLS-only. A stalled
handshake is dropped after `--tls-handshake-timeout-ms` (default 10 s); the
plaintext session has no idle timeout and gains none. *(Amended by
decision 0027: the plaintext idle posture is now bounded — 5 min SMTP /
10 min POP3, as above; the STARTTLS handshake timeout itself is
unchanged.)* Without a cert the
plaintext listener is byte-identical to the one without the flag.

```
$ swarmail gen-cert --domain localhost --out ./certs    # cert.pem + key.pem, 0600
$ swarmail serve --tls-cert ./certs/cert.pem --tls-key ./certs/key.pem
```

### The REST API — await, don't poll

`:8025` — the native API. The verbs a test actually needs:

| Verb | What it does |
|---|---|
| `GET /api/v1/inboxes` | the live inbox list |
| `GET/DELETE /api/v1/inboxes/{inbox}/messages` | list newest-first; wipe the inbox |
| `GET /api/v1/inboxes/{inbox}/await` | **block until arrival** — `to`/`from`/`subject`/`since_ms` filters, `count` (minimum one, even `count=0`), `limit`/`count` ≤ 250 (refused, never clamped), `timeout_ms` (≤ 60000: refused with 400 above, never clamped; a stopping server answers 503 `shutting down` instead of parking) |
| `GET /api/v1/inboxes/{inbox}/assert` | exact-count assertion — `count=0` asserts an empty filter; never clamped — 200 or a named failure |
| `GET /api/v1/inboxes/{inbox}/threads[/{key}]` | conversations, oldest first, stable keys |
| `GET /api/v1/inboxes/{inbox}/feed` | SSE: one snapshot, then one event per accepted mail |
| `POST /api/v1/inboxes/{inbox}/seed` | a fixture without SMTP — full parse + extraction; `html` preserved verbatim (supersedes `text`), CR/LF/NUL envelope fields refused |
| `GET /api/v1/messages/{id}[/raw]` | one mail; the RFC 5322 source as `message/rfc822` |

Every mail carries `text`, `html`, `links`, `codes` — extracted at ingest.
`GET /api/v1/inboxes/{inbox}/count` is the exact-count read;
`/openapi.json` serves the whole contract (3.1).

### MCP — agents drive the mail

```
$ curl -s -X POST localhost:8025/mcp -H 'content-type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{…}}'
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-03-26",
  "serverInfo":{"name":"swarmail","version":"0.1.0"}, …}}

$ curl -s -X POST localhost:8025/mcp -H 'content-type: application/json' \
    -d '{"jsonrpc":"2.0","id":2,"method":"tools/call",
         "params":{"name":"swarmail_wait_for_email",
                   "arguments":{"inbox":"quickstart","to":"ada@x.io","count":1,"timeout_ms":2000}}}'
{"jsonrpc":"2.0","id":2,"result":{"isError":false,
  "content":[{"type":"text","text":"{\"emails\":[{…\"codes\":[\"424242\"]…}]}"}]}}
```

Twelve tools: `swarmail_search_emails`, `swarmail_get_email`,
`swarmail_get_latest_email`, `swarmail_delete_email`, `swarmail_clear_inbox`,
`swarmail_clear_all`, **`swarmail_wait_for_email`** (blocks until arrival),
`swarmail_extract_links`, `swarmail_extract_codes`, `swarmail_seed_email`,
`swarmail_set_chaos`, `swarmail_clear_chaos`. Every tool validates its supplied arguments at the
envelope: a wrong-typed or malformed value — the `arguments` container
included — refuses with a JSON-RPC `-32602` error naming the field and its
domain, before any effect; an unknown tool name is `-32601`; a JSON parse
failure is `-32700`; a notifications-shaped line (no `id`) stays silent.
Wire it up:

```jsonc
// .mcp.json — HTTP transport
{ "mcpServers": { "swarmail": { "type": "http", "url": "http://localhost:8025/mcp" } } }
// or stdio subprocess:
{ "mcpServers": { "swarmail": { "command": "swarmail", "args": ["mcp"] } } }
```

`swarmail mcp` speaks JSON-RPC over stdin/stdout and bridges to
`SWARMAIL_URL` (default `http://127.0.0.1:8025`). The HTTP surface is
Streamable JSON-RPC at `POST /mcp`.

### POP3 — the same store, RFC 1939

`:1110` — USER, PASS, STAT, LIST, UIDL, RETR, DELE, NOOP, RSET, TOP, CAPA,
QUIT. The maildrop **is** a swarmail inbox: `USER proj-42` names the inbox
exactly like SMTP AUTH does, so mail delivered over SMTP reads back over
POP3 — including mail that arrived before the session opened. UIDLs are the
swarmail ids; octet counts are the octets RETR transmits — the payload
CRLF-terminated and dot-stuffed, the terminating `.` not counted — derived
from the stored payload by running the same writer RETR uses, never stored
a second time. DELE marks in-session and applies
on QUIT — the update stage; RSET unmarks; a dropped connection leaves the
maildrop untouched. Plaintext only: no APOP, no STLS — a test-harness
surface behind the same trust boundary as the SMTP listener.

### MailHog / Mailpit shims

Point an existing client at the HTTP port and it works — the two shapes
test suites actually hit, answered from one store:

| Endpoint | Behavior |
|---|---|
| `GET /api/v1/messages` | Mailpit's summary envelope, newest first; `?inbox=` scopes, `?limit=`/`?start=` page |
| `GET /api/v1/message/{id}` | Mailpit's full message shape (`Text`, `HTML`, `ReturnPath`, …); `GET /api/v1/message/latest` is the same shape over the newest stored mail (404 only when the store is empty) |
| `GET /api/v1/message/{id}/plain`, `GET /api/v1/messages/{id}/plain` | the text part as `text/plain; charset=utf-8` — html-only mail is rendered to text at request time |
| `DELETE /api/v1/messages` | Mailpit's `{"ids": [...]}` delete (`IDs` accepted alike) — the named ids and nothing else; every other body (absent, empty, unparseable, `{"ids": []}`) is a `400` that deletes nothing |
| `DELETE /api/v1/delete-all` | MailHog's wipe alias |
| `GET /api/v1/search` | Mailpit search: `kind=from\|to\|subject\|containing`, `?query=` required |
| `GET /api/v2/messages` | MailHog v2 `{total, count, start, items}` with `data.Message` shapes |
| `GET /api/v2/search` | MailHog search: `kind=from\|to\|containing` present and one of the three, as upstream requires; a bare `400` on an unknown kind, an absent kind or an empty query; `?limit=`/`?start=` page |
| `GET /api/v1/messages/{id}/download`, `GET /api/v1/message/{id}/raw` | the RFC 5322 source as `message/rfc822` |

**Deliberate omissions** — swarmail has no counterpart and a shim would
lie: attachments and inline images (the fields exist, the arrays are
empty), read/unread state, tags (always `[]`), the send API (mail arrives
by SMTP), Mailpit's `mime/part` routes, and the `to:`-style query prefixes
(a Mailpit search whose query carries a `to:` prefix answers a named
`400` — the prefix grammar is not implemented; swarmail takes `kind=`).
swarmail's own single-message route is
`/api/v1/messages/{id}`; the Mailpit-style singular `/api/v1/message/{id}`
is the shim.

**Known differences** — the shapes are close but not byte-identical, and
the divergence is the honest part: a malformed `limit=`/`start=` answers
axum's plain-text `400` (not Mailpit's JSON error envelope — matching it
means a custom query extractor for one status-code shape);
`/api/v1/delete-all` is swarmail's alias, not an upstream route name; and
the Mailpit search envelope carries `messages_count`/`messages_unread_count`
where upstream Mailpit spells the latter `messages_unread` — the count
semantics match (mailbox `total`/`unread`, match-scoped `messages_*`), the
field name is swarmail's.

### Threads and the live UI

`GET /api/v1/inboxes/{inbox}/threads` groups the inbox into conversations —
References/In-Reply-To/Message-ID chains when they resolve, the normalized
subject (leading reply/forward markers stripped — `Re:`/`Fw:`/`Fwd:`/`Aw:`,
whitespace before the colon tolerated; a numbered `Re[2]:` is *not* a
marker and stays in the fallback key — and the
remainder Unicode-lowercased) as the fallback for the *chain-less* mails
only: a reply threads by its References even when the parent id was
never published, because absence does not authorize a subject merge. Each
conversation carries a
stable `key`: the hash of the conversation's *first* mail's anchor (its
`Message-ID`, else its `In-Reply-To`/first `References` entry, else the
normalized subject, else its own id). A reply — even one whose message-id
sorts before the root's — can never move the URL a client was already
given.

Mail stored before this derivation is re-keyed on the next request —
threads are recomputed per request, so there is no data migration and
nothing to run; a key also moves when its conversation's oldest mail
leaves the newest-200 view window, is deleted or cap-pruned, when two
mails share a receive millisecond and the id tie-break re-orders them, or
when a bridge mail merges two chains — the merged chains retire their old
keys, which answer a loud 404; re-list to find the surviving anchor's key.
`…/feed` is the SSE stream of the full thread view: one
snapshot, then one event per accepted mail, driven by the same per-inbox
watcher that powers `await`. The human UI at `/ui/inbox/{inbox}` and
`/ui/inbox/{inbox}/thread/{key}` is a thin shell over these endpoints and
updates live — no manual refresh, zero frontend dependencies.

### Webhooks

```
$ curl -X PUT localhost:8025/api/v1/webhooks -H 'content-type: application/json' -d \
    '[{"url": "http://127.0.0.1:9999/hooks", "inbox": null, "secret": "s3cret"}]'
```

Every accepted email is POSTed as `{"event":"received","email":{…}}` with
the secret in `X-Swarmail-Secret` — queued, retried (100 ms → 1.6 s, four
attempts), inbox-filtered, and never blocking the SMTP path. `http://`
targets stay plain; `https://` targets are TLS with verification on — hand
a self-signed sink its own CA through the target's optional `ca_pem`.
Clients pinned to a `ca_pem` are cached per PEM under a fixed bound of 32
(`swarmail::webhook::CA_CLIENT_CAP`); the LRU build is evicted and the next
delivery rebuilds it. `swarmail_webhook_ca_cache_{entries,builds_total,
evictions_total}` on `/metrics` lets you watch the bound hold.

A target is validated when it is PUT, not at delivery: the URL must parse
with an `http`/`https` scheme and a host to dial (`localhost:9000` is read
as *scheme* `localhost` and refused — the misconfiguration that would
otherwise surface only at delivery), the `ca_pem` must load through the
delivery TLS stack itself, and the `secret` must be a single-line header
value of at most 4096 octets. A refusal is a 400 naming the defect and
applies nothing. `GET /api/v1/webhooks` redacts the secret (PUT is the
only writer; an operator retains their own payload) and returns everything
else verbatim.

Delivery is best-effort with the loss made observable — there is no durable
outbox: a mail whose fan-out cannot fire is never re-fired after a restart.
At most 32 deliveries are in flight **per target**
(`swarmail::webhook::WEBHOOK_DELIVERY_CAP`, keyed by the target's
normalized URL): a stalled sink holds only its own target's slots, so it
cannot cause drops for a healthy one. A mail whose target's slots are all
taken has that fan-out refused at the bound, counted once in
`swarmail_webhook_dropped_total`, and never retried. A target that accepts
and never answers wedges its delivery for the dispatcher's whole budget —
about 22.1 s (four attempts under the 5 s POST timeout plus the backoff) —
so a stalled slot holds the bound for that long, then the delivery is
counted in `swarmail_webhook_exhausted_total`, and its final refusal is
logged at the default level: one `warn` naming the target URL and the last
delivery error (no per-attempt spam). The five ledger counters —
`swarmail_webhook_{delivered,dropped,lagged,exhausted,aborted}_total` — are
cumulative since process start (they reset on restart, they are not
persisted), and a shutdown counts its backlog of undelivered notifications
in `swarmail_webhook_aborted_total` instead of dropping them silently.

### Chaos — break SMTP on purpose

```
$ curl -X PUT localhost:8025/api/v1/chaos -H 'content-type: application/json' -d \
    '{"data": {"probability": 100, "error": "451 4.3.0 tempfail", "delay_ms": 250}}'

$ python3 -c "
import smtplib
s = smtplib.SMTP('localhost', 1025, timeout=10)
s.sendmail('a@x.io', ['b@x.io'], 'Subject: t\r\n\r\nbody')"
…
smtplib.SMTPDataError: (451, b'4.3.0 tempfail')              # the injected line, verbatim

$ curl -X DELETE localhost:8025/api/v1/chaos                  # recovery is a test too
```

Chaos is per-stage — `connect`, `mail_from`, `rcpt`, `data` — each with its
own probability, error line and delay. Set it, run the retry suite, clear
it. MCP: `swarmail_set_chaos` / `swarmail_clear_chaos`. Wire values are
never narrowed: `probability` must be an integer 0..=100 — present, a true
integer, within the ceiling — each stage refusing by name, and a
wrong-typed `delay_ms` is refused while an absent one honestly means "no
delay" (decision 0018). A supplied `error` must be a reply line: a single
line starting `4xx`/`5xx`, at most 510 octets (RFC 5321 §4.5.3.1.2's 512
less the CRLF `write_line` appends) — a `2xx` error is refused, because a
rule that would answer success while the mail is not stored is a lie the
wire cannot tell. A supplied `delay_ms` is an integer 0..=300000, the
stage-timer scale — a chaos stall is a wire condition, not an unbounded
hang. Refused values answer 400 with the rule's own sentence (the same
words on the MCP plane), and a rejected PUT applies nothing.

A fired rule always applies its delay, then always fails its stage — a
delay-only rule (probability set, no `error`) is a slow failure, never a
harmless slow-down (issue #17):

| Stage | Delay-only rule (no `error`) | With `error` |
|---|---|---|
| `connect` | wait, then close with no line at all — neither banner nor error | wait, then the given line replaces the banner |
| `mail_from` | wait, then `451 4.3.0 chaos` | wait, then the given line |
| `rcpt` | wait, then `451 4.3.0 chaos` | wait, then the given line |
| `data` | wait, then `451 4.3.0 chaos` — the mail is **not** stored | wait, then the given line — the mail is **not** stored |

So `error: null` means *the stage's default failure*, not *no failure* —
turn chaos off with `probability: 0` or by clearing the config. Two
orderings are pinned: a genuine over-limit `552 5.3.4` is answered before
the data chaos rule is consulted (resource failures precede injected
ones), and a data rule's delay applies after the body has been received
and before the reply — the transfer itself is never slowed.

### Persistence — the restart copy

By default the store is memory — fast, and gone when the process exits.
With a data file, every insert, delete and clear is committed to bundled
SQLite (WAL, fsync-per-accept) and the full state — inboxes, messages with
raw bytes, the `emails_inserted`/`emails_dropped` counters — is restored
before the server answers anything:

```
swarmail serve --data-file /var/lib/swarmail/mail.db   # or SWARMAIL_DATA_FILE=…
```

Restored mail does not re-fire webhooks or waiters — it is
already-delivered state, not a new delivery. Restart is exercised by
`tests/persistence.rs`: the exact state comes back, and a clear racing a
250-answered insert never loses the answered mail.

## Benchmarks

| Measurement | Result | How |
|---|---|---|
| Parse + extract | **1.28 µs/mail** (~780k/s/core) | `cargo bench` · criterion median |
| + store insert | **1.58 µs/mail** (~630k/s/core) | `cargo bench` · criterion median |
| + store insert at the default cap, steady state | **~2.1 µs/mail** (~470k/s/core) — memory-only, no `--data-file` | `cargo bench` · criterion median (`ingest/store/insert@cap-steady`) |
| End-to-end SMTP ingest | **5000/5000 exact**, zero loss | `cargo test --release --test rate -- --ignored --nocapture` (100 concurrent fresh sessions; session-latency-bound — scales with cores) |

Reproduce with `cargo bench` (results land in `target/criterion`). The
cap-steady row is the shipped default configuration
(`SWARMAIL_MAX_PER_INBOX` = 100 000): a store prefilled to exactly the cap,
every measured insert evicting exactly one oldest mail — the shape the
accept path runs in production. The uncapped rows above describe an
unbounded store, and before the deque eviction landed, the same cap-steady
bench measured ~13.7 µs/mail at this cap: the front-drain shift, not the
parse, was the cost. The capped query paths publish in the same run
(`ingest/threads/view@cap`, `ingest/store/count@cap`,
`ingest/store/count@cap-filtered`).

## Repo layout

| Path | What it is |
|---|---|
| `src/smtp.rs`, `src/pop3.rs` | the two wire protocols, hand-rolled on tokio |
| `src/store.rs`, `src/persist.rs` | the synchronous store and the SQLite restart copy |
| `src/extract.rs`, `src/threads.rs` | ingest-time extraction and conversation grouping |
| `src/api.rs`, `src/compat.rs` | the native REST API and the MailHog/Mailpit shims |
| `src/mcp.rs`, `src/stdio.rs` | the MCP HTTP surface and the stdio bridge |
| `src/chaos.rs`, `src/webhook.rs`, `src/tls.rs` | the failure injector, the dispatcher, STARTTLS |
| `src/static/` | the embedded UI, `/openapi.json`, `/llms.txt` |
| [`docs/decisions/`](docs/decisions/) | the constitution: numbered rulings, indexed |
| [`docs/house-rules.md`](docs/house-rules.md) | the realm charter — frozen guarantees, gates, dependency policy |
| [`assets/logo.svg`](assets/logo.svg) | the brand mark, and `social-preview.png`, the repository's link card |
| [`scripts/coverage-gate.sh`](scripts/coverage-gate.sh) | the exact-coverage gate: literal 100% line/branch/function, or refusal |
| [`charts/swarmail/`](charts/swarmail/) | the Helm chart — verified offline by [`scripts/chart-verify.sh`](scripts/chart-verify.sh), published as OCI |

## The decision culture

Every semantic change is a numbered ruling in
[`docs/decisions/`](docs/decisions/), kept in full and cited by number in
the code that enforces it — the guarantees above are exactly those
citations. An implementer may write a decision, but only with status
`proposed`; acceptance is the operator's, recorded in the file. A ruling is
never edited into a different meaning — a new number supersedes it and says
so. That is why the README, the error messages and the tests can all cite
"decision 0001" and mean the same paragraph.

The realm charter is [`docs/house-rules.md`](docs/house-rules.md); agents
start at [`AGENTS.md`](AGENTS.md).

## Testing & coverage

**Every integration test runs against real servers speaking the real
protocol** — no mocks in the loop. The bar for a human contribution is the
bar the machine is already held to; these are required jobs in
[`.github/workflows/ci.yml`](.github/workflows/ci.yml):

| Gate | Command |
|---|---|
| Canonical formatting | `cargo fmt --all -- --check` |
| Clippy, warnings as errors | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| Full suite, ubuntu + macOS | `cargo test --workspace --locked` |
| **Exact coverage** | `bash scripts/coverage-gate.sh` |
| Licenses | `cargo deny check licenses` |
| RustSec advisory audit | `rustsec/audit-check` (CI-only; the hermetic verify has no network) |

The coverage gate is not a percentage to trend upwards. It demands literal
integer equality — `covered == count` — over **lines, branches and
functions** (today 5497/5497, 316/316, 692/692), refuses
`#[coverage(off)]` outright so production code cannot shrink its own
denominator, and preserves the full report under `target/coverage/` when
red so the burn-down starts from evidence, not from a number. There is no
threshold to lower.

```bash
cargo test                                                    # the suite (seconds)
cargo test --release --test rate -- --ignored --nocapture    # the 5k guarantee
cargo bench                                                   # ingest path → target/criterion
bash scripts/coverage-gate.sh                                 # the floor gate
```

| Suite | What it proves |
|---|---|
| `tests/p2.rs` | the P2 end-to-end: MCP JSON-RPC, the seed API's parse+extraction pipeline, webhook delivery with secret and retry targets |
| `tests/smtp_edges.rs` | every protocol verb, refusal, AUTH form, DATA limit, dot-unstuffing, group addresses, chaos on connect/MAIL FROM/RCPT/DATA; the 0034 boundaries — mixed terminators, path and parameter matrices, greeting resets, validation before chaos, strict PLAIN and one AUTH per session |
| `tests/rate.rs` | the 5k losslessness guarantee + burst |
| `tests/burst.rs` | 1000 mails / 50 conns exact-count, per-inbox isolation, cap eviction, chaos rejection + recovery |
| `tests/api.rs` | every REST route, happy + error branches |
| `tests/config_cli.rs` | the CLI/env config surface |
| `tests/mcp.rs` | every tool, JSON-RPC protocol errors, `isError` results |
| `tests/ui.rs` | every page, escaped (injection-proof) rendering |
| `tests/webhook.rs` | delivery with secret, pathless target, the retry budget, https over a real TLS server, the CA client cache bound |
| `tests/stdio.rs` | the stdio JSON-RPC bridge over in-process duplex pipes |
| `tests/starttls.rs` | STARTTLS: byte-identical plaintext listener, real rustls upgrade, RFC 3207 restart (identity and envelope reset), refused certs, stalled handshakes, malformed PEM refusal, gen-cert file handling |
| `tests/lifecycle.rs` | graceful stop: serve futures return, ports release, parked awaits answer 503 inside the stop budget, an in-flight commit neither holds stop nor half-lands, queued seeds refuse once the stop has fired — at most one busy wait, never one per queued request; a stalled body and a partial head cannot hold stop hostage (the drain deadline ends the wait) and the partial head is answered `503 stopping` on the wire |
| `tests/binary.rs` | the shipped binary: SIGINT and SIGTERM → clean exit, SIGINT with seeds queued on a held write lock exits inside the budget and stores nothing, a second signal during the drain exits 70, a mail committed before SIGTERM is queryable after restart, mcp EOF → 0 |
| `tests/pop3.rs` | the POP3 surface: every verb happy + refused, multi-line replies, deletion-on-QUIT |
| `tests/compat.rs` | the shims: Mailpit + MailHog envelopes, searches, deletes, 404/400 branches |
| `tests/persistence.rs` | restart restores exact state; a racing clear never loses 250-answered mail |
| unit | extraction, filters, chaos gating, model plumbing, POP3 line protocol, compat shapes |

## Roadmap

- [x] STARTTLS + self-signed cert generation
- [x] HTTPS webhook targets (reqwest + rustls)
- [x] SQLite persistence (`--data-file` / `SWARMAIL_DATA_FILE`)
- [x] POP3 server; MailHog/Mailpit API compat shims
- [x] Exact 100/100/100 coverage contract (lines, branches, functions) in the gate and CI
- [x] MCP: HTTP + stdio, 12 tools
- [x] Kubernetes: the Helm chart (OCI-published, decision 0008 *proposed*)
- [ ] crates.io publish — the package is publish-ready (see *Publishing*);
  the operator runs `cargo publish --locked` with their token

## Publishing

The crate ships to [crates.io](https://crates.io/crates/swarmail) from the
repo root. The packaged file set is curated in `Cargo.toml` (`include`): the
registry gets the sources, the embedded static assets, the real-protocol
test suites, the criterion bench, both license texts and the README — not
the delivery constitution, the brokkr seats or the CI workflow.

```bash
cargo login <token>          # an operator's crates.io token — never committed
cargo publish --dry-run --locked   # optional: prove it before the act
cargo publish --locked       # the act itself
```

`--locked` publishes exactly the dependency graph `Cargo.lock` pins — the
same one every gate in this repo runs against.

**Version policy:** a version bump is the operator's release act — bump
`version` in `Cargo.toml`, refresh `Cargo.lock` (`cargo check` keeps it in
lockstep, so `--locked` stays publishable), append
`docs/releases/vX.Y.Z.md`, and sign the tag (see *Releases* in
[CONTRIBUTING](CONTRIBUTING.md)). Historical release notes are frozen:
append a new one, never edit a shipped one.

## License

Dual licensed under either of:

- **Apache License, Version 2.0** — [LICENSE-APACHE](./LICENSE-APACHE)
- **MIT license** — [LICENSE-MIT](./LICENSE-MIT)

at your option (SPDX: `MIT OR Apache-2.0`) — the same convention as the Rust
ecosystem itself. Unless you explicitly state otherwise, any contribution
intentionally submitted for inclusion in the work by you, as defined in the
Apache-2.0 license, shall be dual licensed as above, without any additional
terms or conditions.
