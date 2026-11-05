# The swarmail guide

[← README](../../README.md)

The detail behind the [README](../../README.md), one page per topic. The
frozen guarantees and the rulings behind them are in
[`docs/decisions/`](../decisions/README.md); the realm charter is
[`docs/house-rules.md`](../house-rules.md).

## Start here

| Page | What it covers |
|---|---|
| [Overview](overview.md) | the name, what swarmail does, the five guarantees in full |
| [Quickstart](quickstart.md) | Docker and source installs, persistence in Docker, the real transcript, one connection per inbox, environment knobs, logging, stopping |
| [Network posture](network-posture.md) | loopback binds, Host and Origin checks, forwarded ports, `/metrics`, the no-auth boundary, bounded pages |

## Surfaces

| Page | What it covers |
|---|---|
| [SMTP](smtp.md) | the session boundaries, UTF-8 commands, listener health, idle bounds, the stop, STARTTLS |
| [REST API](rest-api.md) | the routes, extracted fields, body caps, filters and closed queries, the inbox-name rule |
| [Client fixtures](client-fixtures.md) | the Playwright fixture and the pytest plugin |
| [MCP](mcp.md) | revisions, batches, compact reads and `fields`, the tool list, refusals, agent setup, the stdio bridge |
| [Focused waits](focused-waits.md) | `swarmail_wait_for_code`, `swarmail_wait_for_link` and near misses on a timeout |
| [Attachments](attachments.md) | `swarmail_list_attachments`, `swarmail_get_attachment`, calendar invites, the caps |
| [POP3](pop3.md) | the maildrop snapshot, octet counts, DELE on QUIT |
| [MailHog / Mailpit / MailSlurper shims](compat-shims.md) | the routes, deliberate omissions, known differences |
| [Threads and the live UI](threads-and-ui.md) | conversation keys, the SSE feed, the UI |
| [Webhooks](webhooks.md) | targets and validation, delivery bounds, CA work, the ledger counters |
| [Chaos](chaos.md) | per-stage failure rules and their exact behavior |
| [Persistence](persistence.md) | the data file, one server per file, memory, timestamps |
| [Recipient routing](recipient-routing.md) | `--route-domain` and `swarmail_create_inbox` |
| [Deliverability reports](deliverability.md) | the report shape, the bulk-sender checklist, the HTML rules, the per-link `link.*` rules, the bounds |

## Operating and contributing

| Page | What it covers |
|---|---|
| [Kubernetes (Helm)](kubernetes.md) | the install, the release order, every chart value |
| [Benchmarks](benchmarks.md) | ingest microbenchmarks, the 5000/5000 proof, what they do not measure |
| [Testing & coverage](testing.md) | the CI jobs, the exact coverage gate, every suite, delivery lanes |
| [Repo layout and decision culture](repo-layout.md) | where things live, how rulings are made |
| [Publishing](publishing.md) | crates.io status, the version policy, release notes |
| [Roadmap](roadmap.md) | what is done and what is left |
