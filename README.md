# Swarmail 🦀📮

> **The fastest AI-native SMTP mail mock.** One Rust binary that swallows email
> faster than your tests can produce it — **losslessly** — and hands it to agents,
> test suites and humans through REST, MCP and a web UI.

[![CI](https://github.com/feedback-loop-ai/swarmail/actions/workflows/ci.yml/badge.svg)](https://github.com/feedback-loop-ai/swarmail/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/feedback-loop-ai/swarmail)](https://github.com/feedback-loop-ai/swarmail/releases)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)
[![Rust](https://img.shields.io/badge/rust-stable-orange)](https://www.rust-lang.org)
[![clippy · -D warnings](https://img.shields.io/badge/clippy%20%C2%B7%20--D%20warnings-orange)](.github/workflows/ci.yml)
[![deps · permissive-only](https://img.shields.io/badge/deps%20%C2%B7%20permissive--only-brightgreen)](deny.toml)
[![coverage · 100% lines](https://img.shields.io/badge/coverage%20%C2%B7%20100%25%20lines-green)](scripts/coverage-gate.sh)

**v0.1.0** · [Releases](https://github.com/feedback-loop-ai/swarmail/releases) ·
machine-readable surfaces: [`/openapi.json`](http://localhost:8025/openapi.json) ·
[`/llms.txt`](http://localhost:8025/llms.txt) · `/metrics` · `/healthz`

**Engineering discipline** (the brokkr-school, adapted): frozen guarantees
with [decision records](docs/decisions/), a [realm charter](docs/house-rules.md),
[contribution gates](CONTRIBUTING.md), a never-falling
[coverage floor](scripts/coverage-gate.sh) and a
[permissive-only dependency tree](deny.toml). Agents start at
[AGENTS.md](AGENTS.md).

## Why

Every existing dev mail server fails agentic and bursty test runs one way or
another. The field's actual record:

| | MailSlurper | MailCrab | Mailpit | MailDev | **Swarmail** |
|---|---|---|---|---|---|
| Burst handling | SMTP session leaks | **documented message loss** >100/s | 200–300/s | Node, untested | **lossless — 5000/5000 exact over 100 concurrent fresh sessions, asserted in CI** |
| Ingest path | — | — | — | — | **~630k mails/s/core** (criterion median: 1.58 µs/mail parse+extract+insert) |
| Wait-until-arrival | ❌ poll | ❌ poll | ❌ poll | ❌ poll | ✅ `await` long-poll + MCP `wait_for_email` |
| Per-test inboxes | ❌ | ❌ | ❌ | ❌ | ✅ AUTH username = inbox (free parallel isolation) |
| MCP server | ❌ | ❌ | ❌ | ✅ (Node) | ✅ HTTP + stdio, native, 12 tools |
| Link/code extraction | ❌ | ❌ | ❌ | ❌ | ✅ first-class (`links`, `codes` on every email) |
| Chaos on SMTP | ❌ | ❌ | error codes only | ❌ | ✅ connect/mail_from/rcpt/data — probability, error line, delay |
| Webhooks | ❌ | ❌ | 1/s, no retry | ❌ | ✅ queued, retried (100ms→1.6s), inbox-filtered, http + https (TLS, reqwest/rustls) |
| Footprint | Go+DB | 7.8 MB | ~10 MB | Node | **static binary, scratch container ≈ binary size** |

*Methodology: the 5000-mail figure is `tests/rate.rs` (release, fresh SMTP
session per mail — the Ory Kratos courier pattern); the µs/mail figure is
`cargo bench` on the pure ingest path. Both run in CI.*

## Quickstart

```bash
docker run -p 1025:1025 -p 1110:1110 -p 8025:8025 ghcr.io/feedback-loop-ai/swarmail:v0.1.0
# or
cargo install swarmail && swarmail serve
# or from source
cargo run -- serve
```

Point any SMTP client (Ory Kratos, Nodemailer, curl) at `localhost:1025` and open
`http://localhost:8025`. No config, no auth required — if a client authenticates,
**the username names the inbox** (`proj-42`, `test-run-7`, …), giving every
parallel test or agent its own inbox for free.

Environment knobs: `SWARMAIL_SMTP_LISTEN` (default `1025`),
`SWARMAIL_POP3_LISTEN` (default `1110`), `SWARMAIL_HTTP_LISTEN` (default `8025`),
`SWARMAIL_MAX_PER_INBOX`
(default `100000`, `0` = unlimited), `SWARMAIL_URL` (for the MCP stdio bridge),
`SWARMAIL_TLS_CERT`/`SWARMAIL_TLS_KEY` (STARTTLS on the SMTP port),
`SWARMAIL_TLS_HANDSHAKE_TIMEOUT_MS` (default `10000` — a STARTTLS handshake
that stalls past it is dropped; the plaintext session has no idle timeout
and keeps none).

### STARTTLS

```bash
swarmail gen-cert --domain localhost --out ./certs   # cert.pem + key.pem
swarmail serve --tls-cert ./certs/cert.pem --tls-key ./certs/key.pem
```

The plaintext listener stays byte-identical without a cert; with one, EHLO
advertises `STARTTLS` and the session upgrades to real TLS (rustls/ring — the
same provider story as the webhooks). After the handshake the session
restarts per RFC 3207 §4.2: everything the client sent before it is
discarded, and every further command is TLS-only. A self-signed pair is the
intended dev/test bootstrap; operators can hand it a real cert — the PEM pair
is parsed before anything binds, so a malformed one refuses to serve rather
than sitting half-alive.

The upgrade itself is bounded: a client that connects, greets and then goes
silent mid-handshake — no ClientHello, ever — is dropped after
`--tls-handshake-timeout-ms` (default `10000`, i.e. 10 s) instead of pinning
its session task, and the listener keeps serving. A real handshake is
milliseconds; the deadline only exists so a stalled one cannot be held open
forever. The pre-TLS plaintext session has no idle timeout and gains none —
only the upgrade wait is bounded.

### Persistence across restarts

By default the store is in memory — fast, and gone when the process exits.
Start with a data file and every insert, delete and clear is committed to
SQLite (bundled, WAL, fsync-per-accept) and the full state — inboxes,
messages with raw bytes, the `emails_inserted`/`emails_dropped` counters —
is restored before the server answers anything:

```bash
swarmail serve --data-file /var/lib/swarmail/mail.db   # or SWARMAIL_DATA_FILE=…
```

The ingest path stays synchronous: an SMTP `250` means the mail is queryable
**and** on disk (decision 0001). Restored mail does not re-fire webhooks or
waiters on restart — it is already-delivered state, not a new delivery.

## The agent loop: clear → act → assert

```bash
# 1. start clean
curl -X DELETE localhost:8025/api/v1/inboxes/test-run/messages

# 2. trigger the flow (signup, reset, invite, …)

# 3. block until the mail arrives — push-based, no sleep-polling
curl "localhost:8025/api/v1/inboxes/test-run/await?to=user@x.io&count=1&timeout_ms=5000"

# 4. the response already carries subject/text/html, plus every URL and
#    likely OTP code — no regexing HTML:
#    { "matched": 1, "emails": [ { "subject": …, "links": […], "codes": ["424242"] } ] }
```

Fixtures without SMTP: `POST /api/v1/inboxes/test-run/seed` (runs the full
parse + extraction pipeline).

## POP3 — RFC 1939 on the same store

Swarmail also answers plain POP3 (default `0.0.0.0:1110`, `SWARMAIL_POP3_LISTEN`).
The maildrop **is** a swarmail inbox: `USER proj-42` names an existing inbox
exactly like SMTP AUTH does, so a test that delivers over SMTP reads the same
mail back over POP3 — including mail that arrived before the POP3 session
opened.

Supported commands: USER, PASS, STAT, LIST [n], UIDL [n], RETR n, DELE n,
NOOP, RSET, TOP n k, CAPA, QUIT. UIDLs are the swarmail message ids; octet
counts are the stored raw sizes (RETR normalizes bare-LF lines to CRLF in
transit, as RFC 1939 requires). DELE marks a message during the session and
the deletion is applied on QUIT — the RFC's update stage; RSET unmarks, and a
connection that drops without QUIT leaves the maildrop untouched. Plaintext
only: no APOP, no STLS, no TLS wrapper — swarmail's POP3 is a test-harness
surface behind the same trust boundary as the SMTP listener.

## MailHog / Mailpit API shims

The two shapes test suites actually hit, answered from the same store — point
an existing MailHog/Mailpit client at swarmail's HTTP port and it works:

| Endpoint | Behavior |
|---|---|
| `GET /api/v1/messages` | Mailpit's summary envelope (`total`/`unread`/`count`/`messages_count`/… /`messages`), newest first; `?inbox=` scopes to one swarmail inbox, `?limit=`/`?start=` page |
| `GET /api/v1/message/{id}` | Mailpit's full message shape (`Text`, `HTML`, `ReturnPath`, …) |
| `GET /api/v1/message/{id}/plain`, `GET /api/v1/messages/{id}/plain` | the extracted text part as `text/plain; charset=utf-8` |
| `DELETE /api/v1/messages` | Mailpit's `{"ids": [...]}` delete — or a wipe when the body is absent/empty/unparseable, like upstream's decoders; replies `{"removed": n}` |
| `DELETE /api/v1/delete-all` | MailHog's wipe alias |
| `GET /api/v1/search` | Mailpit search: `kind=from\|to\|subject\|containing`, `?query=` required, `400 {"error": …}` otherwise |
| `GET /api/v2/messages` | MailHog v2 `{total, count, start, items}` with `data.Message` shapes — `Content.{Headers,Body,Size,MIME}`, `Raw.{From,To,Data,Helo}` |
| `GET /api/v2/search` | MailHog search: `kind=from\|to\|containing`; a bare `400` (no body, as upstream answers) on an unknown kind or empty query |
| `GET /api/v1/messages/{id}/download`, `GET /api/v1/message/{id}/raw` | the full RFC 5322 source as `message/rfc822`, `attachment; filename="<id>.eml"` |

Mapping notes: Mailpit's `Username` is the swarmail inbox (swarmail routes by
inbox and has no read state — `Read` is always `false` and the unread counts
mirror `total`); Mailpit's `Return-Path` and MailHog's `Raw.From` carry the
header From, the closest recorded reverse path; header maps are synthesized
from fields extracted at ingest (decision 0005), never re-parsed at query
time. Mailpit's `mail.Address` renders with lowercase keys and drops empty
names; MailHog's `Path` splits at the domain (both sides empty when a mail
has no From at all); `MIME` is `null` when the mail has no body.

**Deliberate omissions** — swarmail has no counterpart and a shim would lie:
attachments and inline images (the fields exist, the arrays are empty), read/
unread state, tags (always `[]`), the send API (mail arrives by SMTP),
`/api/v1/messages/{id}/mime/part/...` and attachment downloads, Mailpit's
`to:`-style query prefixes (swarmail takes `kind=` instead), and the two web
UIs. swarmail's own `/api/v1/messages/{id}` stays native — the Mailpit-style
single-message route is the singular `/api/v1/message/{id}`.

## Threads and the live inbox view

`GET /api/v1/inboxes/{inbox}/threads` groups the inbox into conversations —
References/In-Reply-To/Message-ID chains when they resolve, the normalized
subject (Re:/Fwd: markers stripped, case-folded) as the fallback — each with a
stable `key` served at `…/{inbox}/threads/{key}`, conversation oldest first.
`GET /api/v1/inboxes/{inbox}/feed` is a server-sent-events stream of the full
thread view: one snapshot, then one event per accepted mail, driven by the same
per-inbox watcher that powers `await` (subscribe-before-scan, so nothing is
missed while connecting). The human UI at `/ui/inbox/{inbox}` and
`/ui/inbox/{inbox}/thread/{key}` is a thin shell over these endpoints and
updates live — no manual refresh.

## MCP (Claude Code, Codex, Cursor, …)

```jsonc
// .mcp.json — HTTP transport
{ "mcpServers": { "swarmail": { "type": "http", "url": "http://localhost:8025/mcp" } } }
// or stdio subprocess:
{ "mcpServers": { "swarmail": { "command": "swarmail", "args": ["mcp"] } } }
```

12 tools: `swarmail_search_emails`, `swarmail_get_email`,
`swarmail_get_latest_email`, `swarmail_delete_email`, `swarmail_clear_inbox`,
`swarmail_clear_all`, **`swarmail_wait_for_email`** (blocks until arrival),
`swarmail_extract_links`, `swarmail_extract_codes`, `swarmail_seed_email`,
`swarmail_set_chaos`, `swarmail_clear_chaos`. Agents close the loop on signup,
reset and magic-link flows without a human touching a browser tab.

## Surfaces

| Surface | Where |
|---|---|
| REST API v1 | `/api/v1/inboxes`, `…/{inbox}/messages` (GET/DELETE), `…/count`, `…/await`, `…/assert`, `…/threads[/{key}]`, `…/feed`, `…/seed`, `/api/v1/messages/{id}[/raw]` |
| Admin | `PUT/DELETE /api/v1/chaos`, `PUT/GET/DELETE /api/v1/webhooks` |
| MCP | `POST /mcp` (Streamable HTTP JSON-RPC) · `swarmail mcp` (stdio bridge) |
| Machine docs | `/openapi.json` (3.1) · `/llms.txt` · `/metrics` (Prometheus) · `/healthz` |
| Human UI | `/`, `/ui/inbox/{name}`, `/ui/inbox/{name}/thread/{key}`, `/ui/message/{id}` — zero frontend deps; the inbox and thread views update live from the SSE feed |
| SMTP | `:1025` — EHLO, AUTH PLAIN/LOGIN (accept-any; username = inbox), PIPELINING, 8BITMIME, SMTPUTF8, SIZE, 50 MiB cap; optional STARTTLS (rustls/ring) with a 10 s handshake deadline (`--tls-handshake-timeout-ms`) |
| POP3 | `:1110` — RFC 1939 core + TOP/CAPA; the maildrop is an inbox (see *POP3*) |
| MailHog / Mailpit shims | `/api/v1/messages` (GET/DELETE), `/api/v1/message/{id}[/plain\|/raw\|/headers]`, `/api/v1/messages/{id}[/plain\|/download]`, `/api/v1/search` · `/api/v2/search`, `/api/v1/delete-all` — see *MailHog / Mailpit API shims* |

## Chaos — test your failure paths

```bash
curl -X PUT localhost:8025/api/v1/chaos -H 'content-type: application/json' -d \
  '{"data": {"probability": 30, "error": "451 4.3.0 tempfail", "delay_ms": 250}}'
# ... run your retry-logic tests ... then:
curl -X DELETE localhost:8025/api/v1/chaos
```

## Webhooks

```bash
curl -X PUT localhost:8025/api/v1/webhooks -H 'content-type: application/json' -d \
  '[{"url": "http://127.0.0.1:9999/hooks", "inbox": null, "secret": "s3cret"}]'
```
Every accepted email is POSTed as `{"event":"received","email":{…}}` with the
secret in `X-Swarmail-Secret` — queued, retried (100 ms → 1.6 s), and never
blocking the SMTP path. Delivered via reqwest + rustls: `http://` targets
stay plain, `https://` targets are TLS with certificate verification on —
hand a self-signed test server its own CA through the target's optional
`ca_pem` (PEM of a root to trust on top of the built-in root store). Clients
pinned to a `ca_pem` are cached per PEM under a fixed bound of 32
(`swarmail::webhook::CA_CLIENT_CAP`): the least-recently-used build is
evicted and that root's next delivery simply rebuilds its client — the
built-in root store's client sits outside the cache and is built once.
`swarmail_webhook_ca_cache_{entries,builds_total,evictions_total}` on
`/metrics` lets you watch the bound hold.

## Guarantees

- **Accept == stored.** SMTP `DATA` is inserted synchronously; a `250` reply
  means the message is queryable. There is no async gap where mail can vanish —
  this is the failure mode of MailCrab under load and MailSlurper's session
  leaks.
- **Exact-count assertions are real.** CI asserts `5000/5000` across 100
  concurrent fresh sessions (see *Testing*).
- **Pruning is by age/volume, never mid-delivery.** Per-inbox cap (default
  100k) evicts the *oldest* mail only.

## Benchmarks

| Measurement | Result | How |
|---|---|---|
| Parse + extract | **1.28 µs/mail** (~780k/s/core) | `cargo bench` · criterion median |
| + store insert | **1.58 µs/mail** (~630k/s/core) | `cargo bench` · criterion median |
| End-to-end SMTP ingest | **5000/5000 exact**, zero loss | `cargo test --release --test rate -- --ignored --nocapture` (100 concurrent fresh sessions; sandboxed runner, session-latency-bound — scales with cores) |

Reproduce with `cargo bench` (results land in `target/criterion`).

## Testing & coverage

**Every integration test runs against real servers speaking the real
protocol** — no mocks in the loop:

| Suite | What it proves |
|---|---|
| `tests/burst.rs` (6) | 1000 mails / 50 conns exact-count, per-inbox isolation, cap eviction, chaos rejection + recovery |
| `tests/p2.rs` (4) | MCP end-to-end (initialize → seed → search → clear), seed extraction, webhooks with secret + inbox filter |
| `tests/rate.rs` (1) | **the 5k guarantee** — 5000/5000 exact over 100 fresh sessions + throughput report |
| `tests/pop3.rs` (17) | real POP3 over TCP: auth (incl. unknown maildrop), stat/list/uidl/retr round-trips, DELE→QUIT deletes, RSET, TOP, dot-stuffing, oversized lines, persistence restart |
| `tests/compat.rs` (10) | MailHog/Mailpit shapes over real HTTP: envelopes, scoping, every search kind, plain/raw/download, selective + wipe deletes, bodyless and From-less mail |
| unit (90) | extraction, filters, chaos gating, model plumbing, POP3 line protocol, compat shapes |

```bash
cargo test                                                    # the suite (seconds)
cargo test --release --test rate -- --ignored --nocapture    # 5k guarantee + rate
cargo bench                                                   # ingest path → target/criterion
bash scripts/coverage-gate.sh                                 # the floor gate
```

**Line coverage: 100%** — every line of production code, verified by
`cargo llvm-cov` (4064/4064) and enforced by `scripts/coverage-gate.sh`: the
gate is **exact** (missed lines == 0, not a rounded 99.95→100) and **the floor
may rise, never fall** (the brokkr rule). `#[coverage(off)]` is forbidden, so
production code cannot shrink the denominator. The suite that carries it:

| Suite | What it proves |
|---|---|
| `tests/smoke.rs` | startup, round trip, per-test inboxes |
| `tests/smtp_edges.rs` | every protocol verb, refusal, AUTH form, DATA limit, dot-unstuffing, group addresses, chaos on connect/MAIL FROM/DATA |
| `tests/rate.rs` | the 5k losslessness guarantee + burst |
| `tests/api.rs` | every REST route, happy + error branches |
| `tests/mcp.rs` | every tool, JSON-RPC protocol errors, `isError` results |
| `tests/ui.rs` | every page, escaped (injection-proof) rendering |
| `tests/webhook.rs` | delivery with secret, pathless target, the 4-attempt retry budget, https over a real TLS server (handshake-failure + refused-connection paths) |
| `tests/stdio.rs` | the stdio JSON-RPC bridge over in-process duplex pipes |
| `tests/starttls.rs` | STARTTLS: byte-identical plaintext listener, real rustls upgrade, RFC 3207 restart (pipelined plaintext discarded), refused certs, stalled handshake closed by the deadline, malformed PEM refusal |
| `tests/lifecycle.rs` | graceful stop: serve futures return, ports release |
| `tests/binary.rs` | the shipped binary: SIGINT → clean exit, mcp EOF → 0 |
| `tests/pop3.rs` | the POP3 surface: every verb happy + refused, multi-line replies, deletion-on-QUIT |
| `tests/compat.rs` | the shims: Mailpit + MailHog envelopes, searches, deletes, 404/400 branches |

## AI-native delivery (brokkr)

This repo develops itself under [brokkr](https://github.com/feedback-loop-ai/brokkr),
the deterministic delivery engine — same constitution pattern as
[`xbox-inference`](https://github.com/feedback-loop-ai/xbox-inference):

- **Model seats:** `glm-flash` → **`spark-glm/GLM-5.3-Flash-EXL3`** via the `dsh`
  driver, local route, `SPARK_API_KEY` — spark only, no other provider route.
  The reviewer/judge is `glm-flash` too.
- **Hermetic gates:** `verify` and `ship` are exec seats (`scripts/verify-seat.sh`,
  `scripts/ship-seat.sh`) — no model in the loop where a shell suffices.
- **Phase machine:** `intake → implement → verify → review → ship`; the review
  gate is constitutionally protected (`protected_phase: review`), security-hold
  is a hard stop.
- **Journal:** `.forge/forge.db` (append-only, WAL, gitignored).

```bash
brokkr doctor                          # verify tools + workspace
brokkr compile --bundle .              # verify the pinned bundle digest
brokkr run                             # start a run (needs SPARK_API_KEY)
```

The realm charter is [`docs/house-rules.md`](docs/house-rules.md): the frozen
guarantees, the gates, the dependency policy. Seat charters live in
`agents/charters/`; the decisions the seats read are under
[`docs/decisions/`](docs/decisions/).

## Development

```bash
cargo run -- serve            # dev loop (SMTP :1025, HTTP :8025)
cargo test && cargo clippy --all-targets -- -D warnings
docker build -t swarmail .    # scratch image ≈ binary size
```

- Rust stable · Tokio · Axum 0.8 · mail-parser · DashMap · zero frontend deps
- The store is synchronous by design: `Store::insert` returning == the mail is queryable
- CI: fmt + clippy `-D warnings` + test on every push; coverage summary + ghcr image on `main`

## Publishing

The crate ships to [crates.io](https://crates.io/crates/swarmail) from the
repo root. The packaged file set is curated in `Cargo.toml`
([`include`](https://doc.rust-lang.org/cargo/reference/manifest.html#the-exclude-and-include-fields)):
the registry gets the sources, the embedded static assets, the real-protocol
test suites, the criterion bench, both license texts (`LICENSE-MIT`,
`LICENSE-APACHE`) and the README — not the delivery constitution, the brokkr
seats or the CI workflow.

```bash
cargo login <token>          # an operator's crates.io token — never committed
cargo publish --dry-run --locked   # optional: prove it before the act
cargo publish --locked       # the act itself
```

`--locked` publishes exactly the dependency graph `Cargo.lock` pins — the same
one every gate in this repo runs against — and the dry run builds the
packaged sources standalone before anything is uploaded.

**Version policy:** a version bump is the operator's release act — bump
`version` in `Cargo.toml`, refresh `Cargo.lock` (`cargo check` keeps it in
lockstep, so `--locked` stays publishable), append
`docs/releases/vX.Y.Z.md`, and sign the tag (see *Releases* in
[CONTRIBUTING](CONTRIBUTING.md)); the lockfile and the release notes move
together, and no package is ever hand-crafted. Historical release notes
are frozen: append a new `docs/releases/vX.Y.Z.md`, never edit a shipped
one.

## Roadmap

- [x] STARTTLS + self-signed cert generation
- [x] HTTPS webhook targets (reqwest + rustls)
- [x] SQLite persistence (`--data-file` / `SWARMAIL_DATA_FILE`)
- [x] POP3 server; MailHog/Mailpit API compat shims
- [ ] crates.io publish — the package is publish-ready (see *Publishing*);
  the operator runs `cargo publish --locked` with their token
- [x] UI: live-updating inbox view, message threads — grouped by
  References/In-Reply-To chains with normalized-subject fallback
  (`…/{inbox}/threads`), pushed live over SSE (`…/{inbox}/feed`)

## License

Dual licensed under either of:

- **Apache License, Version 2.0** — [LICENSE-APACHE](./LICENSE-APACHE)
- **MIT license** — [LICENSE-MIT](./LICENSE-MIT)

at your option (SPDX: `MIT OR Apache-2.0`) — the same convention as the Rust
ecosystem itself.
