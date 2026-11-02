# Repo layout and the decision culture

[← README](../../README.md) · [Guide index](README.md)

| Path | What it is |
|---|---|
| `src/smtp.rs`, `src/pop3.rs` | the two wire protocols, hand-rolled on tokio |
| `src/store.rs`, `src/persist.rs` | the synchronous store and the SQLite restart copy |
| `src/extract.rs`, `src/threads.rs` | ingest-time extraction and conversation grouping |
| `src/api.rs`, `src/compat.rs` | the native REST API and the MailHog/Mailpit/MailSlurper shims |
| `src/mcp.rs`, `src/stdio.rs` | the MCP HTTP surface and the stdio bridge |
| `src/chaos.rs`, `src/webhook.rs`, `src/tls.rs` | the failure injector, the dispatcher, STARTTLS |
| `src/static/` | the embedded UI, `/openapi.json`, `/llms.txt` |
| [`docs/decisions/`](../decisions/) | the constitution: numbered rulings, indexed |
| [`docs/house-rules.md`](../house-rules.md) | the realm charter — frozen guarantees, gates, dependency policy |
| [`assets/logo.svg`](../../assets/logo.svg) | the brand mark, and `social-preview.png`, the repository's link card |
| [`scripts/coverage-gate.sh`](../../scripts/coverage-gate.sh) | the exact-coverage gate: literal 100% line/branch/function, or refusal |
| [`clients/`](../../clients/) | the Playwright and pytest fixtures — thin REST clients, versioned with the server, not yet published |
| [`charts/swarmail/`](../../charts/swarmail/) | the Helm chart — verified offline by [`scripts/chart-verify.sh`](../../scripts/chart-verify.sh), published as OCI |

## The decision culture

Every semantic change is a numbered ruling in
[`docs/decisions/`](../decisions/), kept in full and cited by number in
the code that enforces it — the [guarantees](overview.md#the-guarantees) are exactly those
citations. An implementer may write a decision, but only with status
`proposed`; acceptance is the operator's, recorded in the file. A ruling is
never edited into a different meaning — a new number supersedes it and says
so. That is why the README, the error messages and the tests can all cite
"decision 0001" and mean the same paragraph.

The realm charter is [`docs/house-rules.md`](../house-rules.md); agents
start at [`AGENTS.md`](../../AGENTS.md).
