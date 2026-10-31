# Baseline CLI wrappers

The three entrypoints launch fixed recipes for Claude Code **2.1.310**, Codex
**0.164.0**, and dsh **0.1.5-rc.1** (MCP client rc.3). They use the operator's
selected existing logins. Delivery uses local stand-ins only: authenticated
six-model smoke, exact-version tool inventories and HTTP MCP interoperability,
especially the **live dsh HTTP MCP row**, remain the operator's first smoke
proof. No provider run, tokens, cost, transcript or baseline result is claimed.

The supplied 2026-10-26 operator ruling replaces the former missing-contract
refusal. Its checksum-verified packet and evidence distinctions are recorded in
[continuation intake](issue151-continuation-intake.md). The design is
[CLI wrapper launch](../specs/cli-wrapper-launch/spec.md); proposed
[0083](../docs/decisions/0083-cli-wrapper-launch.md) remains proposed. No decision
is accepted here. The dated earlier intake receipts remain historical.

## Private selection

Each entrypoint takes `--model MODEL --config ABSOLUTE_PRIVATE_FILE`.
The closed version-1 selection has exactly these fields (inert paths):

```json
{"version":1,"families":{"claude":{"executable":"/opt/claude","credential_file":"/private/claude-oauth.json"},"codex":{"executable":"/opt/codex","credential_file":"/private/auth.json"},"dsh":{"executable":"/opt/dsh","credential_file":"/private/dsh-keys.json"}}}
```

Choose actual absolute executable paths or trusted launchers that work with
`PATH=/usr/bin:/bin`, `LANG=LC_ALL=C`. A Volta shim requiring ambient VOLTA_HOME
needs an operator-provided explicit launcher. There is no installer, PATH
inheritance, version discovery, arbitrary argv/env override, API-key substitution
for Claude/Codex, refresh subsystem or fallback model. Profile versions describe
the recipe, not a binary attestation.

MCP configuration requires UTF-8 runtime paths and an ASCII identity header;
unsupported representations refuse before launch. Entry-point selection
locators use native process paths; the baseline JSON handoff requires UTF-8.

The selection and each selected credential must be regular, single-link,
effective-uid-owned files with no group/other permissions (normally **0600**),
at most **65536 bytes**. Parent traversal uses no-follow directory descriptors;
ancestors must be root/current-uid owned without group/world write, except the
root-owned sticky `/tmp` followed by private directories. Symlinks, unsafe
parents, nonregular files, oversized reads, duplicates, unknown fields and
malformed records refuse without echoing values or paths. Only the selected
family's credential file is opened. Never copy a login directory or put secrets
in argv, labels, public config IDs, tracked examples, rows or reports.

Selected file schemas:

* Claude: exactly `{"claudeAiOauth":{"accessToken":"<selected OAuth access token>","expiresAt":1999999999999,"scopes":["user:inference"]}}`.
  Select a minimal record from the existing login. Expiry is a nonnegative
  JavaScript-safe integer; the wrapper does not infer current token validity.
  It creates only `$HOME/.claude/.credentials.json`, with these three fields.
  Do not supply an API key or refresh token. `--bare` is deliberately absent
  because the captured CLI excludes OAuth under that switch.
* Codex: exactly `{"auth_mode":"chatgpt","OPENAI_API_KEY":null,"tokens":{"id_token":"<selected id token>","access_token":"<selected access token>","refresh_token":"<existing refresh token>","account_id":"<selected account>"},"last_refresh":"2026-10-26T00:00:00Z"}`.
  The source is the selected ChatGPT login auth.json. The wrapper projects only
  id/access/account values into `$HOME/.codex/auth.json`; its refresh token is
  the inert literal `unavailable`, and last_refresh is copy time. It never
  forwards the genuine refresh token or updates the source. Renew expired
  credentials through the operator login.
* dsh: exactly `{"DASHSCOPE_API_KEY":"<selected route key>","SPARK_API_KEY":"<selected route key>"}`.
  Only the selected route's value enters its named CLI environment variable.
  No .env file is sourced and no DEEPSEEK_API_KEY is forwarded.

All generated credential/config/session files are below the trial's private
HOME: files are exclusively created **0600 before payload**, directories **0700**.
The runner retains them through owned cleanup and then disposes them. No source
credential is modified. Raw CLI stdout/stderr go directly to `/dev/null`, on
success or failure; there is no raw debug or redaction mode. CLI-created private
session files are disposed with HOME. Crash-proof erasure is not promised.

## Recipes, tools and limits

| Family / exact models | Applied recipe | Remaining surface / evidence limits |
|---|---|---|
| Claude: `claude-opus-5-5`, `claude-sonnet-5-5` | `-p --output-format stream-json --verbose --model MODEL --no-session-persistence --permission-mode dontAsk --permission-prompts none --tools "" --strict-mcp-config --mcp-config PRIVATE --allowedTools EXPLICIT_NAMES --restricted --disable-slash-commands`; DISABLE_AUTOUPDATER=1 | Empty builtins and strict MCP measured on nearby 2.1.287; exact-version permission behavior and server-delivered account policy remain smoke. No whole-server allow matcher is assumed. |
| Codex: `gpt-6.1-sol`, `gpt-6-luna` | `exec --json --model MODEL -C PRIVATE_CWD --skip-git-repo-check --ephemeral --ignore-user-config --ignore-rules --sandbox read-only`, per-key TOML overrides and final `-`; web_search disabled; untrusted project override | Native reading/edit tooling, code-mode MCP access and ChatGPT account connectors remain. Project-trust override effect and connector removal are unmeasured. No speculative feature-disable matrix. Read-only is not a native-tool allowlist. |
| dsh: `dashscope/glm-5.3`, `spark-glm/GLM-5.3-Flash-EXL3` | Fresh DSH_HOME, `--profile headless --patch PRIVATE_SEAT_YML GOAL`, DSH_TELEMETRY_DISABLED=1; route/model/session rows and two MCP insert rows in overlay | Native shell/file/web/subagent tools remain; removal by disabled rows and plugin composition are unmeasured. No native-tool row disable qualifies as measured in the supplied packet, so none is asserted. HTTP MCP is schema-evidenced, not live-proven. |

Goals are unchanged stdin bytes for Claude/Codex. dsh receives one literal
argument, never a shell string or invented --model flag; leading `-`, NUL or
more than 65535 UTF-8 bytes refuses. No effort override, substitution or retry.
All eight EVAL inputs remain required, including prompt-file byte equality,
numeric loopback URLs, a valid identity header and the existing deadline bounds.

Every child has a cleared environment, fresh HOME/TMPDIR/XDG and private cwd.
Ancestry through `/` refuses `.claude`, `.codex`, `.agents`, `.dsh`, `.mcp.json`,
`AGENTS.md`, `AGENTS.local.md`, `CLAUDE.md`, `CLAUDE.local.md` and `.env`.
Fresh HOME refuses preexisting entries. Claude refuses any `/etc/claude-code`;
Codex refuses any `/etc/codex`, including dangling symlinks or indeterminate
inspection. No system-root override exists in production. No dsh /etc source
was established by the supplied package evidence.

Each recipe configures Swarmail HTTP at EVAL_MCP_URL with X-Eval-Identity and
the stdio fixture helper. Claude's explicit names are guarded against the real
Swarmail tools/list. Codex uses per-key overrides because whole tables merge.
dsh restates only its selected provider: the captured DashScope HTTPS endpoint
or `http://spark:8888/v1`, exact model, and the captured spark compaction policy.
Overlay JSON flow nodes are YAML 1.2, with control characters escaped.

The internal `fixture-http-isolated` entrypoint clears the environment again
and execs `fixture-http` with only fixed runtime and validated EVAL inputs.
Provider credentials cannot reach the helper environment even if a CLI inherits
them for MCP children. The helper exposes exactly `eval_http`, supplies identity
on every GET/POST, forbids arbitrary origins/headers, disables proxies/redirects,
bounds inputs/responses and applies its request budget. This is independent of
Swarmail MCP. Unix exec keeps CLI/helper descendants in the runner's group;
the existing runner alone owns the deadline, TERM/KILL/reap/drain and admission.

The helper accepts and discards MCP `params._meta` on initialize, ping,
`tools/list`, `tools/call` and initialized notifications for 2025-03-26,
2025-06-18 and 2025-11-25. It never echoes or forwards metadata or emits
progress notifications. Tool params remain closed except for this reserved
field; `arguments._meta` remains an invalid tool argument.

GET requires an omitted `body`. Any supplied JSON value, including `""` or
`null`, returns a tool result with `isError: true` and the fixed text
`http_get_body_not_supported`, without an HTTP request. POST retains JSON
serialization, including the distinction between omission and explicit null.
Other helper failures retain the generic `fixture_request_refused` tool result.

## Rows, report and operator handoff

Every current row requires canonical `provenance.agent.tool_surface`, including
setup failures and unstarted slots. Wrapper profiles have revision 1 and basis
`declared_configuration`, restrictions and conservative residuals. The finite
selectors are `claude_2_1_310`, `codex_0_164_0`, `dsh_0_1_5_rc_1` (AgentSpec
`wrapper_profile`, or trial `--wrapper-profile`). A selector is an explicit
operator declaration, never inferred from a label, executable or CLI output.
Generic external commands remain `external_unknown`; builtins are
`builtin_not_applicable`; invalid requests are `invalid_request`. Missing,
noncanonical or model-conflicting surfaces refuse. Historical rows need their
historical binary. Catalogue, snapshot and result business versions remain 2.

Reports show distinct declared surfaces per group and explicitly state the
per-family asymmetry. Scoring, attempts, success rates, Wilson intervals and
quantiles are unchanged. Every trial remains **unconfined**; same-uid hostile
file access and session escape are outside this contract. Namespace isolation
belongs to #221 and macOS eval proof to #201.

Build and run the actual dry-run using [run-plan.md](run-plan.md). It reads no
selection/credential, creates no output and launches nothing. Before the full
baseline the operator performs one authenticated trial with **each of the six
models**, its corresponding profile, selected config and 600000 ms deadline.
For example, after setting absolute E and SWARMAIL_BIN as in the run plan:

```bash
$E trial --task signup --agent smoke --agent-model claude-opus-5-5 \
  --wrapper-profile claude_2_1_310 --deadline-ms 600000 -- \
  "$E" agent-claude --model claude-opus-5-5 --config /private/wrappers.json
```

Then use the same baseline command without --dry-run and a fresh output path.
The command first proves all fresh controls and retained bindings, checks the
credential-free selection, then admits the six-model product. Paid baseline,
publication, failure-taxonomy reprioritization and sealing are operator work.
No paid or networked agent is run by this delivery. Root and standalone test
receipts, including carried failures, must be assessed separately from smoke.
