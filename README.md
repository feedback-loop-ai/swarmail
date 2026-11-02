<p align="center"><img src="assets/logo.svg" width="132" alt="Swarmail — an open envelope under the feedback-loop pulse: two resting dots and one live one"></p>

# Swarmail

**Most mail mocks are built for a human looking at a web UI. Swarmail is built for a program waiting on mail.**

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

- [Why swarmail](#why-swarmail)
- [60-second quickstart](#60-second-quickstart)
- [What's inside](#whats-inside)
- [For AI agents](#for-ai-agents)
- [The guarantees](#the-guarantees)
- [Kubernetes (Helm)](#kubernetes-helm)
- [Contributing and records](#contributing-and-records)
- [License](#license)

Everything below is the short version. The detail lives in
[the guide](docs/guide/README.md), one page per topic.

## Why swarmail

- **Zero-loss capture.** An SMTP `250` means the mail is stored and
  queryable. 5000 mails over 100 concurrent fresh sessions arrive exact
  ([benchmarks](docs/guide/benchmarks.md)).
- **Await, don't poll.** `await` blocks until the mail arrives. It
  subscribes before it checks, and the links and codes are already
  extracted.
- **Built for AI agents.** Agents drive the same store over MCP, on HTTP or
  stdio: wait for a mail, take the newest code or the one link, seed a
  fixture, break SMTP on purpose.
- **Drop-in for MailHog and Mailpit.** Existing clients work through the
  [shims](docs/guide/compat-shims.md) on the HTTP port.

Eighteen MCP tools, one REST API, two wire protocols in (SMTP, POP3), one
compatibility layer (MailHog/Mailpit), one observability story
(`/metrics`, `/healthz`, `/openapi.json`, `/llms.txt`) — and a web UI with
zero frontend dependencies. More in the [overview](docs/guide/overview.md).

## 60-second quickstart

One binary, three ports, no config:

```
docker run --rm --stop-timeout 30 -p 1025:1025 -p 1110:1110 -p 8025:8025 ghcr.io/feedback-loop-ai/swarmail:main
```

From source: `cargo run -- serve`, or `cargo install --locked --path .` from
a checkout — the crate is not published on crates.io.

Point any SMTP client at `localhost:1025`. The AUTH username names the
inbox, so every test gets its own. Then wait for the mail and read the
extracted fields:

```
$ curl "localhost:8025/api/v1/inboxes/quickstart/await?to=ada@x.io&count=1&timeout_ms=5000"
{"matched": 1, "total": 1,
 "emails": [ { "subject": "Reset your password",
               "links": ["https://x.io/reset?token=xyz789"], "codes": ["998877"] } ]}
```

The [full quickstart](docs/guide/quickstart.md) has the real `smtplib`
transcript, persistence in Docker, every environment knob, one connection
per inbox, and how to stop the server gracefully.

## What's inside

| Surface | In one line | Detail |
|---|---|---|
| SMTP `:1025` | AUTH username = inbox, STARTTLS, a 50 MiB cap, exact DATA boundaries | [smtp.md](docs/guide/smtp.md) |
| REST API `:8025` | `await`, exact-count `assert`, threads, SSE feed, seed fixtures | [rest-api.md](docs/guide/rest-api.md) |
| Client fixtures | Playwright and pytest: `waitForCode()`, `wait_for_link()` | [client-fixtures.md](docs/guide/client-fixtures.md) |
| MCP | eighteen tools over HTTP `POST /mcp` and the stdio bridge | [mcp.md](docs/guide/mcp.md) |
| Focused waits | the newest code, or the one matching link, as a decision | [focused-waits.md](docs/guide/focused-waits.md) |
| Attachments | list them, fetch one, read a calendar invite | [attachments.md](docs/guide/attachments.md) |
| POP3 `:1110` | the same store over RFC 1939 | [pop3.md](docs/guide/pop3.md) |
| MailHog / Mailpit shims | the routes existing test suites hit, with known differences | [compat-shims.md](docs/guide/compat-shims.md) |
| Threads and live UI | stable conversation keys, a UI that updates itself | [threads-and-ui.md](docs/guide/threads-and-ui.md) |
| Webhooks | every accepted mail POSTed to your sink, with retries | [webhooks.md](docs/guide/webhooks.md) |
| Chaos | per-stage SMTP failures and delays, on purpose | [chaos.md](docs/guide/chaos.md) |
| Persistence | an SQLite restart copy, one server per data file | [persistence.md](docs/guide/persistence.md) |
| Recipient routing | one sending identity, a private inbox per agent | [recipient-routing.md](docs/guide/recipient-routing.md) |
| Deliverability reports | per-mail findings: the bulk-sender checklist and HTML client rules | [deliverability.md](docs/guide/deliverability.md) |
| Network posture and limits | loopback by default, Origin checks, `/metrics`, bounded pages | [network-posture.md](docs/guide/network-posture.md) |
| Benchmarks | ingest microbenchmarks and the 5000/5000 proof | [benchmarks.md](docs/guide/benchmarks.md) |

## For AI agents

Wire Claude Code, Codex, Cursor or dsh with the copy-paste
[agent setup guide](docs/agents/setup.md); the tools are on the [MCP page](docs/guide/mcp.md).
The wiring snippets in `docs/agents/setup.md` are written into a fresh project
and driven against the real binary by `tests/agent_setup_docs.rs` in CI;
install commands are marked not-executed.

## The guarantees

Frozen: review refuses a change that weakens one, no matter how green the
tests are ([details](docs/guide/overview.md#the-guarantees)).

1. **Accept == stored** ([0001](docs/decisions/0001-synchronous-insert-lossless.md)).
2. **`await` subscribes before it checks** ([0002](docs/decisions/0002-awaits-subscribe-before-checking.md)).
3. **Pruning is oldest-only, never mid-delivery** ([0003](docs/decisions/0003-oldest-only-pruning.md)).
4. **The AUTH username is the inbox** ([0004](docs/decisions/0004-per-test-inboxes-via-smtp-auth.md)).
5. **Extraction happens at ingest, not query time** ([0005](docs/decisions/0005-extraction-at-ingest.md)).

## Kubernetes (Helm)

One replica, no configuration language of its own
([decision 0008](docs/decisions/0008-kubernetes-deployment-contract.md),
*accepted*). Install from a checkout:

```
git clone https://github.com/feedback-loop-ai/swarmail && cd swarmail
helm install swarmail ./charts/swarmail --set image.tag=main --set image.pullPolicy=Always \
  --namespace swarmail --create-namespace
helm test swarmail -n swarmail        # the /healthz hook, from a curl pod
```

Values, release order and the scaling contract: [kubernetes.md](docs/guide/kubernetes.md).

## Contributing and records

- [CONTRIBUTING](CONTRIBUTING.md), and [`AGENTS.md`](AGENTS.md) for agents.
- The gates: [`docs/house-rules.md` § Gates](docs/house-rules.md#gates--part-of-every-change);
  tests and coverage: [testing.md](docs/guide/testing.md).
- [Decisions](docs/decisions/README.md) — why the code is shaped the way it is
  ([the decision culture](docs/guide/repo-layout.md#the-decision-culture)).
- [Release notes](docs/releases/), [publishing](docs/guide/publishing.md),
  [roadmap](docs/guide/roadmap.md), [repo layout](docs/guide/repo-layout.md).

## License

Dual licensed under either of:

- **Apache License, Version 2.0** — [LICENSE-APACHE](./LICENSE-APACHE)
- **MIT license** — [LICENSE-MIT](./LICENSE-MIT)

at your option (SPDX: `MIT OR Apache-2.0`) — the same convention as the Rust
ecosystem itself. Unless you explicitly state otherwise, any contribution
intentionally submitted for inclusion in the work by you, as defined in the
Apache-2.0 license, shall be dual licensed as above, without any additional
terms or conditions.

The WHATWG HTML entities data and derived table are licensed under CC-BY-4.0; attribution and transformation details are in [NOTICE](./NOTICE).
