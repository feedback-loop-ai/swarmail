# Swarmail 🦀📮

> **The fastest AI-native SMTP mail mock.** One Rust binary that swallows email
> faster than your tests can produce it — **losslessly** — and hands it to agents,
> test suites and humans through REST, MCP and a web UI.

**v0.1.0** · MIT · [feedback-loop-ai/swarmail](https://github.com/feedback-loop-ai/swarmail)

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
docker run -p 1025:1025 -p 8025:8025 ghcr.io/feedback-loop-ai/swarmail:main
# or
cargo install swarmail && swarmail serve
```

Point any SMTP client (Ory Kratos, Nodemailer, curl) at `localhost:1025` and open
`http://localhost:8025`. No config, no auth required — if a client authenticates,
**the username names the inbox** (`proj-42`, `test-run-7`, …), giving every
parallel test or agent its own inbox for free.

## The agent loop: clear → act → assert

```bash
# 1. start clean
curl -X DELETE localhost:8025/api/v1/inboxes/test-run/messages

# 2. trigger the flow (signup, reset, invite, …)

# 3. block until the mail arrives — push-based, no sleep-polling
curl "localhost:8025/api/v1/inboxes/test-run/await?to=user@x.io&count=1&timeout_ms=5000"

# 4. the response already carries subject/text/html, plus every URL and
#    likely OTP code — no regexing HTML:
#    { "matched": 1, "emails": [ { "subject": ..., "links": [...], "codes": ["424242"] } ] }
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

Machine-readable surfaces: `/openapi.json` (OpenAPI 3.1), `/llms.txt`, `/metrics`
(Prometheus), `/healthz`.

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
secret in `X-Swarmail-Secret` — queued, retried, and never blocking the SMTP
path. (v0.1: `http://` targets.)

## Guarantees

- **Accept == stored.** SMTP `DATA` is inserted synchronously; a `250` reply
  means the message is queryable. There is no async gap where mail can vanish —
  this is the failure mode of MailCrab under load and MailSlurper's session
  leaks.
- **Exact-count assertions are real.** CI asserts `5000/5000` across 100
  concurrent fresh sessions (`cargo test --release --test rate -- --ignored
  --nocapture`).
- **Pruning is by age/volume, never mid-delivery.** Per-inbox cap (default
  100k) evicts the *oldest* mail only.

## Development

```bash
cargo test                 # 17 tests: unit + e2e (real servers, real protocol)
cargo bench                # ingest path numbers → target/criterion
cargo run -- serve         # dev loop
docker build -t swarmail . # scratch image ≈ binary size
```

## Roadmap

- [x] P1 — fast path: concurrent SMTP, per-inbox store, REST, await/assert, chaos
- [x] P2 — agent layer: MCP (HTTP+stdio), seed, webhooks, OpenAPI, llms.txt
- [x] P3 — embedded UI, 5k guarantee, benches, scratch image
- [ ] STARTTLS + self-signed cert generation (`ENABLE_TLS_AUTH` parity with MailCrab)
- [ ] HTTPS webhook targets (reqwest + rustls)
- [ ] SQLite persistence (`--data-file`)
- [ ] POP3 server; MailHog/Mailpit API compat shims

## License

MIT — see [LICENSE](./LICENSE).
