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
| Webhooks | ❌ | ❌ | 1/s, no retry | ❌ | ✅ queued, retried (100ms→1.6s), inbox-filtered |
| Footprint | Go+DB | 7.8 MB | ~10 MB | Node | **static binary, scratch container ≈ binary size** |

*Methodology: the 5000-mail figure is `tests/rate.rs` (release, fresh SMTP
session per mail — the Ory Kratos courier pattern); the µs/mail figure is
`cargo bench` on the pure ingest path. Both run in CI.*

## Quickstart

```bash
docker run -p 1025:1025 -p 8025:8025 ghcr.io/feedback-loop-ai/swarmail:v0.1.0
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
`SWARMAIL_HTTP_LISTEN` (default `8025`), `SWARMAIL_MAX_PER_INBOX`
(default `100000`, `0` = unlimited), `SWARMAIL_URL` (for the MCP stdio bridge).

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
| REST API v1 | `/api/v1/inboxes`, `…/{inbox}/messages` (GET/DELETE), `…/count`, `…/await`, `…/assert`, `…/seed`, `/api/v1/messages/{id}[/raw]` |
| Admin | `PUT/DELETE /api/v1/chaos`, `PUT/GET/DELETE /api/v1/webhooks` |
| MCP | `POST /mcp` (Streamable HTTP JSON-RPC) · `swarmail mcp` (stdio bridge) |
| Machine docs | `/openapi.json` (3.1) · `/llms.txt` · `/metrics` (Prometheus) · `/healthz` |
| Human UI | `/`, `/ui/inbox/{name}`, `/ui/message/{id}` — zero frontend deps, server-rendered |
| SMTP | `:1025` — EHLO, AUTH PLAIN/LOGIN (accept-any; username = inbox), PIPELINING, 8BITMIME, SMTPUTF8, SIZE, 50 MiB cap |

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
blocking the SMTP path. (v0.1: `http://` targets.)

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

**17 integration + unit tests**, every one against real servers speaking the
real protocol — no mocks in the loop:

| Suite | What it proves |
|---|---|
| `tests/burst.rs` (6) | 1000 mails / 50 conns exact-count, per-inbox isolation, cap eviction, chaos rejection + recovery |
| `tests/p2.rs` (4) | MCP end-to-end (initialize → seed → search → clear), seed extraction, webhooks with secret + inbox filter |
| `tests/rate.rs` (1) | **the 5k guarantee** — 5000/5000 exact over 100 fresh sessions + throughput report |
| unit (7) | extraction, filters, chaos gating, model plumbing |

```bash
cargo test                                                    # the suite (seconds)
cargo test --release --test rate -- --ignored --nocapture    # 5k guarantee + rate
cargo bench                                                   # ingest path → target/criterion
bash scripts/coverage-gate.sh                                 # the floor gate
```

**Line coverage: 100%** — every line of production code, verified by
`cargo llvm-cov` (1712/1712) and enforced by `scripts/coverage-gate.sh`: the
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
| `tests/webhook.rs` | delivery with secret, pathless target, the 4-attempt retry budget |
| `tests/stdio.rs` | the stdio JSON-RPC bridge over in-process duplex pipes |
| `tests/lifecycle.rs` | graceful stop: serve futures return, ports release |
| `tests/binary.rs` | the shipped binary: SIGINT → clean exit, mcp EOF → 0 |

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

## Roadmap

- [ ] STARTTLS + self-signed cert generation
- [ ] HTTPS webhook targets (reqwest + rustls)
- [x] SQLite persistence (`--data-file` / `SWARMAIL_DATA_FILE`)
- [ ] POP3 server; MailHog/Mailpit API compat shims
- [ ] crates.io publish
- [ ] UI: live-updating inbox view, message threads

## License

Dual licensed under either of:

- **Apache License, Version 2.0** — [LICENSE-APACHE](./LICENSE-APACHE)
- **MIT license** — [LICENSE-MIT](./LICENSE-MIT)

at your option (SPDX: `MIT OR Apache-2.0`) — the same convention as the Rust
ecosystem itself.
