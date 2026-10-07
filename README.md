# Swarmail 🦀📮

> **The fastest AI-native SMTP mail mock.** A single Rust binary that swallows email
> faster than your tests can produce it — losslessly — and hands it to agents, test
> suites and humans through REST, MCP and a web UI.

**Status: under active construction (v0.1).** This README describes the target of the
current build-out; check the [roadmap](#roadmap) for what has landed.

## Why

Every existing dev mail server fails agents one way or another:

| | MailSlurper | MailCrab | Mailpit | MailDev | **Swarmail (goal)** |
|---|---|---|---|---|---|
| Burst handling | SMTP session leaks | **documented message loss** >100/s | 200–300/s | Node, untested | **10,000+/s, zero loss — benchmarked in CI** |
| Wait-until-arrival | ❌ poll | ❌ poll | ❌ poll | ❌ poll | ✅ `await`/`wait_for_email` |
| Per-test inboxes | ❌ | ❌ | ❌ | ❌ | ✅ free parallel isolation |
| MCP server | ❌ | ❌ | ❌ | ✅ (Node) | ✅ HTTP + stdio, native |
| Chaos on SMTP | ❌ | ❌ | error codes only | ❌ | ✅ full failure simulation |
| Link/code extraction | ❌ | ❌ | ❌ | ❌ | ✅ first-class |

## Quickstart

```bash
docker run -p 1025:1025 -p 8025:8025 ghcr.io/feedback-loop-ai/swarmail
# or
cargo install swarmail && swarmail serve
```

Point Kratos, your server, or your tests at `localhost:1025` and watch
`http://localhost:8025`.

## Roadmap

- [ ] **P1 — the fast path**: concurrent SMTP, in-memory per-inbox store, REST API
      (`list/get/delete/clear/await`), OpenAPI, health/metrics
- [ ] **P2 — the agent layer**: MCP (HTTP + stdio), webhooks with retry, chaos admin,
      link/code extraction, OpenAPI 3.1 + `llms.txt`
- [ ] **P3 — the polish**: embedded UI, seed API, per-test inbox strategies,
      STARTTLS, 5k-mail guarantee test in CI, criterion benches with published numbers

## License

Dual licensed under either of:

- **Apache License, Version 2.0** — [LICENSE-APACHE](./LICENSE-APACHE)
- **MIT license** — [LICENSE-MIT](./LICENSE-MIT)

at your option (SPDX: `MIT OR Apache-2.0`) — the same convention as the Rust
ecosystem itself.
