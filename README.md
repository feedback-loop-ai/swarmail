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

Environment knobs: `SWARMAIL_SMTP_LISTEN` (default `1025`),
`SWARMAIL_POP3_LISTEN` (`1110`), `SWARMAIL_HTTP_LISTEN` (`8025`),
`SWARMAIL_MAX_PER_INBOX` (`100000`, `0` = unlimited), `SWARMAIL_URL` (the
MCP stdio bridge's upstream), `SWARMAIL_TLS_CERT`/`SWARMAIL_TLS_KEY`,
`SWARMAIL_TLS_HANDSHAKE_TIMEOUT_MS` (`10000`), `SWARMAIL_DATA_FILE`.

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
POP3 are plain TCP). The contract worth knowing before you scale it:

| Turn on | What it changes |
|---|---|
| `persistence.enabled` | SQLite on an RWO PVC (`/data/swarmail.db`), **strategy → Recreate** — the store is single-replica by design ([0001](docs/decisions/0001-synchronous-insert-lossless.md)); scaling would silently fork the mailboxes |
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

**STARTTLS** is real: with `--tls-cert`/`--tls-key` (or the env pairs), EHLO
advertises `STARTTLS`, the session upgrades with rustls/ring, and the
session restarts per RFC 3207 §4.2 — plaintext pipelining before the
handshake is discarded, every further command is TLS-only. A stalled
handshake is dropped after `--tls-handshake-timeout-ms` (default 10 s); the
plaintext session has no idle timeout and gains none. Without a cert the
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
| `GET /api/v1/inboxes/{inbox}/await` | **block until arrival** — `to`, `count` (minimum one, even `count=0`), `timeout_ms`, filtered by from/subject/contains |
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
`swarmail_set_chaos`, `swarmail_clear_chaos`. Wire it up:

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
swarmail ids; octet counts are the stored raw sizes (RETR normalizes
bare-LF to CRLF in transit, per the RFC). DELE marks in-session and applies
on QUIT — the update stage; RSET unmarks; a dropped connection leaves the
maildrop untouched. Plaintext only: no APOP, no STLS — a test-harness
surface behind the same trust boundary as the SMTP listener.

### MailHog / Mailpit shims

Point an existing client at the HTTP port and it works — the two shapes
test suites actually hit, answered from one store:

| Endpoint | Behavior |
|---|---|
| `GET /api/v1/messages` | Mailpit's summary envelope, newest first; `?inbox=` scopes, `?limit=`/`?start=` page |
| `GET /api/v1/message/{id}` | Mailpit's full message shape (`Text`, `HTML`, `ReturnPath`, …) |
| `GET /api/v1/message/{id}/plain`, `GET /api/v1/messages/{id}/plain` | the extracted text part as `text/plain; charset=utf-8` |
| `DELETE /api/v1/messages` | Mailpit's `{"ids": [...]}` delete — the named ids and nothing else; every other body (absent, empty, unparseable, `{"ids": []}`) is a `400` that deletes nothing |
| `DELETE /api/v1/delete-all` | MailHog's wipe alias |
| `GET /api/v1/search` | Mailpit search: `kind=from\|to\|subject\|containing`, `?query=` required |
| `GET /api/v2/messages` | MailHog v2 `{total, count, start, items}` with `data.Message` shapes |
| `GET /api/v2/search` | MailHog search: `kind=from\|to\|containing`; a bare `400` on an unknown kind |
| `GET /api/v1/messages/{id}/download`, `GET /api/v1/message/{id}/raw` | the RFC 5322 source as `message/rfc822` |

**Deliberate omissions** — swarmail has no counterpart and a shim would
lie: attachments and inline images (the fields exist, the arrays are
empty), read/unread state, tags (always `[]`), the send API (mail arrives
by SMTP), Mailpit's `mime/part` routes, and the `to:`-style query prefixes
(swarmail takes `kind=`). swarmail's own single-message route is
`/api/v1/messages/{id}`; the Mailpit-style singular `/api/v1/message/{id}`
is the shim.

### Threads and the live UI

`GET /api/v1/inboxes/{inbox}/threads` groups the inbox into conversations —
References/In-Reply-To/Message-ID chains when they resolve, the normalized
subject (Re:/Fwd: stripped, case-folded) as the fallback — each with a
stable `key`: the hash of the conversation's *first* mail's anchor (its
`Message-ID`, else its `In-Reply-To`/first `References` entry, else the
normalized subject, else its own id). A reply — even one whose message-id
sorts before the root's — can never move the URL a client was already
given.

Mail stored before this derivation is re-keyed on the next request —
threads are recomputed per request, so there is no data migration and
nothing to run; a key also moves when its conversation's oldest mail
leaves the newest-200 view window, is deleted or cap-pruned, or when two
mails share a receive millisecond and the id tie-break re-orders them.
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
it. MCP: `swarmail_set_chaos` / `swarmail_clear_chaos`.

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
functions** (today 4162/4162, 252/252, 558/558), refuses
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
| `tests/smoke.rs` | startup, round trip, per-test inboxes |
| `tests/smtp_edges.rs` | every protocol verb, refusal, AUTH form, DATA limit, dot-unstuffing, group addresses, chaos on connect/MAIL FROM/RCPT/DATA |
| `tests/rate.rs` | the 5k losslessness guarantee + burst |
| `tests/burst.rs` | 1000 mails / 50 conns exact-count, per-inbox isolation, cap eviction, chaos rejection + recovery |
| `tests/api.rs` | every REST route, happy + error branches |
| `tests/config_cli.rs` | the CLI/env config surface |
| `tests/mcp.rs` | every tool, JSON-RPC protocol errors, `isError` results |
| `tests/ui.rs` | every page, escaped (injection-proof) rendering |
| `tests/webhook.rs` | delivery with secret, pathless target, the retry budget, https over a real TLS server, the CA client cache bound |
| `tests/stdio.rs` | the stdio JSON-RPC bridge over in-process duplex pipes |
| `tests/starttls.rs` | STARTTLS: byte-identical plaintext listener, real rustls upgrade, RFC 3207 restart, refused certs, stalled handshakes, malformed PEM refusal, gen-cert file handling |
| `tests/lifecycle.rs` | graceful stop: serve futures return, ports release |
| `tests/binary.rs` | the shipped binary: SIGINT → clean exit, mcp EOF → 0 |
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
