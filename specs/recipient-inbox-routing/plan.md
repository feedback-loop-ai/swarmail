# Implementation plan — recipient inbox routing

Implements [spec.md](spec.md) under proposed
[0106](../../docs/decisions/0106-recipient-inbox-routing.md). This is a
design commit: it implements nothing and attests no gate. Merge of the
implementation waits for the operator's acceptance of 0106 (house rules,
"code under a proposed decision").

## Technical context

One Rust crate; Tokio, Axum, clap, serde_json and the `uuid` crate (v7
feature, already locked) are the only tools used. No new dependency, no
feature flag, no new workspace member. The eval harness is the standalone
`evals/` crate with its own lock, proven by its own suite. The named
`.specify/templates/plan-template.md` is absent in this checkout; this file
supplies technical context, constitution check, structure, research, data
model, contracts, quickstart and risks.

## Constitution check

- 0001: routing is decided at `RCPT`, read once at `DATA` before the parse
  is offloaded; the insert stays synchronous and the `250` stays after the
  commit outcome. One mail is one record.
- 0002: the in-flight row is parked under the routed inbox; `subscribe`
  creates the channel lazily, so a waiter on a minted, never-mailed name is
  registered before it scans. No new await enters the insert.
- 0003: pruning is per inbox, oldest-only, unchanged; three agents are three
  caps.
- 0004 is amended for configured servers only, 0055 §6 for the isolation
  task only; both wait on the operator. Default-off keeps unset servers
  byte-identical, which the existing 0004 tests prove unchanged.
- 0020 test placement: branch-stable tests inline and measured
  (`src/smtp.rs`, `src/mcp.rs`, `src/config.rs`); the wire-shaped
  isolation and refusal tests in `tests/inbox_routing.rs`; no sibling
  harness file is expected — every routing branch is reachable on a
  deterministic schedule. Chaos only at 0 or 100. No sleep as
  synchronization: the 0002 proof uses the existing parse-submission seam
  or the wait tool's registration against a send issued afterwards.
- Coverage: every new production line ships with a test; the label
  alphabet is a strict subset of `validate_inbox`, so routing does not call
  it (an `Err` arm that cannot run would fail the exact gate) and a unit
  test proves the subset instead.
- Dependencies: none added; `deny` unchanged. Shims untouched.

## Project structure

| File | Change |
| --- | --- |
| `src/config.rs` | `ServeArgs.route_domain: Option<String>` (`--route-domain`, `SWARMAIL_ROUTE_DOMAIN`, a clap `value_parser` that validates and lowercases); copied into `SmtpConfig`. |
| `src/smtp.rs` | `SmtpConfig.route_domain`; `Envelope.routed: Option<String>`; `routed_inbox(domain, rcpt) -> Result<Option<String>, &'static str>`; the two new `RCPT` refusal lines; the `DATA` arm's inbox expression; `RSET`/take already clear the envelope. |
| `src/api.rs` | `AppState.route_domain: Option<String>` from `cfg.smtp.route_domain`; passed into `McpContext` at the `/mcp` handler. |
| `src/mcp.rs` | `McpContext.route_domain`; the `swarmail_create_inbox` dispatch arm; descriptor appended after `focused::descriptors()`; `OUTPUT_SCHEMAS` entry; `read_only` match arm; the 15 → 16 assertions. |
| `src/mcp/focused.rs`, `src/mcp/batch_tests.rs` | `route_domain: None` in `McpContext` test literals. |
| wherever `run_on` builds `AppState` | thread the route domain. |
| `src/static/llms.txt`, `src/static/openapi.json`, `README.md` | the sixteen-tool wording, the flag, the rule. |
| `docs/decisions/README.md` | the 0106 index row. |
| `tests/inbox_routing.rs` | new: the wire suite (AC1, AC4, AC5, AC6 in part). |
| `tests/config_cli.rs` | the flag's startup validation (AC7). |
| `tests/mcp.rs`, `tests/p2.rs`, `tests/mcp_revisions/focused.rs` | 15 → 16; focused indices `13 + i` stay. |
| `evals/src/runner.rs` | `--route-domain swarmail.test` in the serve argv; `Identity.smtp_identity`; the `identities()` branch; the provision payload. |
| `evals/src/fixture.rs` | `ProvisionIdentity.smtp_identity` (required), stored on the account and used as the AUTH username in `deliver`. |
| `evals/src/mail.rs` | header sentence: the AUTH username is the configured sending identity; a routed recipient selects the inbox (0106). |
| `evals/src/wrappers.rs` | `swarmail_create_inbox` in `SWARMAIL_TOOLS`. |
| `evals/README.md`, `evals/tests/proof.rs` | the isolation sentence; the `fixture-app` emptiness assertion. |

## Research

- `src/smtp.rs` `RCPT` arm: syntax, then the `MAX_RCPTS` ceiling (`452
  4.5.3 Too many recipients`), then chaos, then `open.rcpts.push`. Routing
  admission sits between the ceiling and chaos, and `open.routed` is set in
  the same statement group as the push, so a chaos-failed `RCPT` leaves the
  route untouched (0034 §3).
- `DATA` arm: `let Envelope { from, rcpts } = envelope.take()` then
  `let inbox = identity.as_deref().unwrap_or("default")`, `park_delivery`,
  then `build_email` on the blocking pool. The destructure gains `routed`
  and the expression becomes
  `routed.as_deref().or(identity.as_deref()).unwrap_or("default")`. Nothing
  below it changes.
- `validate_inbox` (`src/model.rs`) refuses blank and control characters
  only; `[a-z0-9-]{1,63}` is a strict subset.
- `tool_descriptors` (`src/mcp.rs`) clones `TOOLS`, adds `outputSchema` from
  `OUTPUT_SCHEMAS[name]` and the annotation by a `read_only` name match;
  `TOOLS` is the thirteen literals plus `focused::descriptors()` appended.
  The new descriptor is appended after that call. The March catalogue
  fixtures pin the first thirteen entries and their bytes; appended entries
  are outside them, as the focused pair already is.
- `McpContext` is constructed at `src/api.rs` (the `/mcp` handler) and in
  four test literals. The stdio bridge (`swarmail mcp`) forwards to HTTP.
- `evals/src/runner.rs` spawns `serve` with fixed argv; `identities()`
  builds `Identity { identity, inbox }`; the provision payload posts
  `identities` to `/admin/provision`, deserialized by `ProvisionBody` with
  `deny_unknown_fields`. `fixture.rs` `deliver` calls
  `mail::send(self.smtp, &out.inbox, …)`; `Outbound.inbox` is the AUTH
  username today and will carry `smtp_identity` instead (renamed to say
  what it is).
- The chart renders `.Values.config` as `SWARMAIL_*` environment, so
  `SWARMAIL_ROUTE_DOMAIN` needs no chart change and no chart bump.

## Data model

- `SmtpConfig.route_domain: Option<String>` — lowercased, validated.
- `Envelope.routed: Option<String>` — the transaction's routed inbox, set
  on the first successful routed append, cleared with the envelope.
- `McpContext.route_domain: Option<String>` / `AppState.route_domain`.
- `ProvisionIdentity { identity, inbox, smtp_identity }` in the fixture;
  `Identity { identity, inbox, smtp_identity }` in the runner.
- No store, persistence or `Email` change.

## Contracts

- SMTP replies, fixed text: `550 5.1.3 Recipient names no inbox under the
  route domain`; `452 4.5.3 One routed inbox per transaction`.
- CLI: `--route-domain <DOMAIN>` / `SWARMAIL_ROUTE_DOMAIN`; refusal names the
  flag and the reason (empty, bad label, too long, stray dot).
- MCP: `swarmail_create_inbox`, closed empty `inputSchema`, `outputSchema`
  `{inbox: string, address: string}` both required, no extra properties;
  `isError` text naming `--route-domain` when unset. Appended last; 16
  tools.
- Fixture admin: `/admin/provision` entries require `smtp_identity`.

## Quickstart

```bash
swarmail serve --route-domain swarmail.test
# mint
curl -s localhost:8025/mcp -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"swarmail_create_inbox","arguments":{}}}'
# → {"inbox":"018f…","address":"agent@018f….swarmail.test"}
# the app sends with its one AUTH identity to that address;
# the agent reads inbox 018f… and nobody else's mail is there
```

## Risks

| Risk | Mitigation |
| --- | --- |
| Routing computed after the park reopens the 0002 window per inbox. | The inbox expression at `DATA` is the single source for `park_delivery`, `build_email` and the in-flight row; AC5 observes the row's name during a parked delivery and a wait registered before the send. |
| A derived inbox no read surface can name (`250` lies). | The admitted alphabet is a strict subset of `validate_inbox`; a unit test enumerates the alphabet through `validate_inbox`. |
| Silent re-merge when a recipient under the domain is malformed or the domain is rewritten by the sender. | Malformed-under-domain is a `550`, never a fallback (AC4). Domain rewriting is out of the server's reach; 0106 names it and the eval proof asserts `fixture-app` stays empty. |
| A CC to two agents loses one copy. | `452` per conflicting recipient with the transaction kept open: RFC 5321 senders deliver to the accepted set and retry the rest; AC4 proves the retry. A sender treating 452 as fatal is named as the accepted cost in 0106. |
| March catalogue bytes drift by appending a sixteenth tool. | Append after the focused pair; the thirteen pinned entries and the focused indices (`13 + i`) are unchanged; the structured and focused fixtures keep passing without regeneration. |
| Minted-name collision after restart. | UUID v7 (timestamp plus 74 random bits) from the crate already in the tree; no process counter, no store state. |
| The exact coverage gate refuses a cold arm. | Every branch is deterministic on the wire or inline: each refusal shape, the unset-domain `isError`, the flag parser's refusals, the `RSET` clear. No sibling harness is expected; if a concrete cold path appears, 0020 §2 names its home. |
| The eval change alters the metric's comparability. | Only the isolation task's wire shape changes; the other fifteen tasks' recipients are `@example.test`, for which the flag is inert. The isolation goal text is unchanged. |
| The isolation inbox label falls outside the alphabet. | `eval-<short>-<name>` is lowercased and checked against the label rule in the runner's own test; the trial id's tail is alphanumeric. |
| Agent success rate cannot be measured here. | Reported as unmeasured; oracle and null controls are the measured evidence (AC10). No paid trials. |
| Operator never accepts 0106. | The flag defaults off and the tool refuses without it; the branch is reviewable and shippable only on acceptance, which the delivery states rather than assumes. |
