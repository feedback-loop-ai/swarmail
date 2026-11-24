# Baseline CLI wrappers

The three entrypoints launch fixed recipes for Claude Code **2.1.329**, Codex
**0.166.0**, and dsh **0.2.2-rc.1** (accepted 0137's 2026-11-23 amendment; the
recipes were written against 2.1.310, 0.164.0 and 0.1.5-rc.1, whose selectors
now decode in published rows only). They use the operator's
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
needs an operator-provided explicit launcher. A launcher is a version-1
convenience only: a paid (version-2) selection must pin the CLI's own native
binary, as below. There is no installer, PATH
inheritance, version discovery, arbitrary argv/env override, API-key substitution
for Claude/Codex, refresh subsystem or fallback model. In a version-1 selection
profile versions describe the recipe, not a binary attestation; the paid
version-2 selection below attests them.

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

## Paid selection (version 2, proposed 0137)

`swarmail-evals baseline` spends only against the closed **version-2**
selection, which adds each family's pinned CLI contract to the version-1
locators (inert values):

```json
{"version":2,"families":{
 "claude":{"executable":"/opt/claude/2.1.329/claude","credential_file":"/private/claude-oauth.json","profile":"claude_2_1_329","cli_version":"2.1.329","cli_sha256":"<sha256 of that file>","auth_shape":"claude_oauth_v1"},
 "codex":{"executable":"/opt/codex/0.166.0/codex","credential_file":"/private/auth.json","profile":"codex_0_166_0","cli_version":"0.166.0","cli_sha256":"<sha256>","auth_shape":"codex_chatgpt_v1"},
 "dsh":{"executable":"/opt/dsh/lib/node_modules/@deepseek-ai/dsh/dist/cli.js","credential_file":"/private/dsh-routes.json","profile":"dsh_0_2_2_rc_1","cli_version":"0.2.2-rc.1","cli_sha256":"<pin identity>","auth_shape":"dsh_routes_attested_v1",
  "interpreted":{"runtime":"/opt/node/22.11.0/bin/node","runtime_version":"v22.11.0","runtime_sha256":"<sha256>","package_root":"/opt/dsh/lib/node_modules/@deepseek-ai/dsh","entry":"dist/cli.js","files":{"<relative path>":"<sha256>", "...":"..."}}}}}
```

A family the product does not spend may be absent; any other field refuses,
except the optional `nested` member that pins the brokkr driver for
`--via-brokkr` (route (b), below). The selection's `profile` stays the direct
one even then, and the trial declares the matching `*_via_brokkr` profile.
The file rules above apply to the selection and every credential. Before
anything spends, `baseline` checks every family the product names and refuses
by `contract <family>: <cause>`:

* `contract_missing` (absent family), `profile_mismatch` (wrong family or a
  `*_via_brokkr` selector), `version_unsupported` (`cli_version` is not the
  profile's exact version), `digest_invalid`, `auth_shape_unsupported`;
* `executable_unsafe`: the executable must be a regular file itself, not a
  symlink or forwarding path, executable, owned by the operator or root and
  writable by no one else, under safe ancestors. Pin the real binary, not a
  launcher whose downstream bytes cannot be measured;
* `executable_not_native`: the pinned file must open as a native executable
  image (ELF, or Mach-O thin or universal). An interpreter script (`#!`),
  such as an npm `codex` launcher, forwards to bytes its digest never covers,
  so it cannot establish CLI identity: pin the native binary it runs. A
  *native* launcher that execs another binary is not detectable and passes;
  the row then names the launcher's bytes, not the CLI's, so pinning the
  real binary is the operator's attestation. A CLI shipped only as an
  interpreted package has nothing to pin and cannot run paid;
* `digest_mismatch`: the measured sha256 differs from `cli_sha256`;
* `version_mismatch` / `version_probe_failed` / `version_probe_timeout`: a
  fixed `EXECUTABLE --version` with a cleared environment, an empty private
  HOME with private TMPDIR and XDG directories (decision 0075 §1), no stdin, 4096 bytes of stdout and a 10 s deadline (its process group
  is killed afterwards) must answer exactly `2.1.329 (Claude Code)`,
  `codex-cli 0.166.0` or `0.2.2-rc.1`. The banner is never printed or kept.
  Only these versions qualify (accepted 0137, 2026-11-23 amendment);
* `profile_superseded`: the selection names `claude_2_1_310`,
  `codex_0_164_0` or `dsh_0_1_5_rc_1`, which left the approved set.

**dsh is an interpreted package** (accepted 0137's 2026-11-23 amendment).
Its selection carries an `interpreted` pin. `executable` is the pin's entry
file and `cli_sha256` is the pin's identity: the SHA-256 of
`swarmail-interpreted-pin/1\n`, `runtime <sha256> <version>\n`,
`entry <path>\n`, then one `<sha256>  <path>\n` line per package entry in
byte order of path. Do not compute it by hand; run
`swarmail-evals pin-interpreted --runtime NODE --runtime-version vX.Y.Z
--package-root ROOT --entry REL`. It prints the three members to paste into
the selection, reads no credential and refuses with `pin refused: <reason>`.
The manifest covers every entry under the root, hidden files included. A
symbolic link is recorded by its in-root target and never followed. Before
spend and again at exec, the harness checks the following, in order:

* `interpreted_pin_missing` (dsh without a pin) and
  `interpreted_pin_unexpected` (a pin on Claude or Codex);
* `interpreted_pin_invalid` (a relative runtime or root, a bad digest, an
  unsafe manifest path) and `entry_invalid` (an entry the manifest does not
  list);
* `executable_not_entry`: `executable` is not the pin's entry, for example
  a shim such as `~/.volta/bin/dsh`;
* `digest_mismatch`: `cli_sha256` is not the pin's identity;
* `runtime_executable_unsafe`, `runtime_executable_not_native`,
  `runtime_digest_mismatch`: the runtime is held to the native-binary rules
  above, so a shim or symlink in its place refuses;
* `package_file_changed`, `package_file_extra`, `package_file_missing`:
  the measured package against the manifest;
* `package_symlink_escapes`, `package_symlink_invalid`,
  `package_special_file`, `package_unsafe` (writable by group or others, or
  owned by someone else), `package_name_invalid`, `package_too_large`,
  `package_root_invalid`: refused while walking;
* `runtime_version_mismatch`, then `version_mismatch`: `RUNTIME --version`
  must answer the pinned line, then `RUNTIME ENTRY --version` must answer
  `0.2.2-rc.1`. Both run under the bounded probe above.

The launch then runs `RUNTIME ENTRY <dsh argv>`, with no shim in between.
At exec, the wrapper's refusals carry the `wrapper_cli_` prefix.

The wrapper itself rechecks the pin immediately before exec
(`wrapper_cli_digest_mismatch`, `wrapper_contract_missing`) and refuses an
expired login (`private_credential_expired`). Under a paid baseline the
trial appends `--paid-launch SHA256:VALID_THROUGH` to the wrapper's argv: the
run's manifest digest and the instant the chunk was admitted for. Neither
secret, they never enter the agent's environment, which stays the accepted
inputs of 0075 §1. A selection re-pinned since the manifest refuses
(`wrapper_contract_drift`), and so does a login valid for less than that
(`private_credential_margin`), and a malformed binding refuses
(`wrapper_paid_binding_invalid`). Any such refusal is reported on the launch
descriptor (fd 3, inherited only with `--paid-launch` and closed on exec), so the slot stays unscored rather than counting as a model
failure. Only the verified profile, version and sha256 enter rows, as
`provenance.agent.cli`; the path stays private. Keep pinned installations
unchanged for the whole run. Before a chunk starts, a binary replaced under
the old pin refuses as `contract <family>: digest_mismatch`, and a selection
re-pinned to a new binary refuses as `contract_drift`; a complete run's
resume needs neither.

Credential validity is read the way each auth shape carries it, and malformed
or absent evidence refuses by `credential <family|route>: <cause>`:

* Claude (`claude_oauth_v1`, the Claude schema above): `expiresAt`, epoch
  milliseconds.
* Codex (`codex_chatgpt_v1`, the Codex schema above): `last_refresh` must
  parse, but it is copy time and no validity evidence. The expiry is the
  integer `exp` claim of the selected access token's base64url JSON payload
  (`credential_expiry_unknown` otherwise). The id token's `exp` is not read:
  the CLI does not need a current id token, and a short-lived one would keep
  every Codex login inside the margin. The token and its claims are never
  printed, kept or put in argv.
* dsh (`dsh_routes_attested_v1`): each route keeps its key with an
  operator-attested UTC valid-through instant, epoch seconds:
  `{"DASHSCOPE_API_KEY":{"key":"<route key>","valid_through":1767225600},"SPARK_API_KEY":{"key":"<route key>","valid_through":1767225600}}`.
  The two-key version-1 file has no validity evidence and refuses here. On
  rotation, replace the whole file atomically with a conservative horizon you
  can justify for that key; the assertion says nothing about revocation.

Version-1 selections keep working for direct `agent-*` callers and never
admit a paid baseline.

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
selectors are `claude_2_1_329`, `codex_0_166_0`, `dsh_0_2_2_rc_1` (AgentSpec
`wrapper_profile`, or trial `--wrapper-profile`), plus the three
`*_via_brokkr` selectors below. The superseded `claude_2_1_310`,
`codex_0_164_0` and `dsh_0_1_5_rc_1` (and their `*_via_brokkr` forms) still
decode so published rows keep their bytes; a paid selection refuses them. A selector is an explicit
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
  --wrapper-profile claude_2_1_329 --deadline-ms 600000 -- \
  "$E" agent-claude --model claude-opus-5-5 --config /private/wrappers.json
```

Then use the same baseline command without --dry-run and a fresh output path.
The command first proves all fresh controls and retained bindings, checks the
credential-free selection, then admits the six-model product. Paid baseline,
publication, failure-taxonomy reprioritization and sealing are operator work.
No paid or networked agent is run by this delivery. Root and standalone test
receipts, including carried failures, must be assessed separately from smoke.

## Launching through a brokkr driver (`--via-brokkr`, route (b) of 0119's 2026-11-23 addendum)

`agent-claude|agent-codex|agent-dsh ... --via-brokkr /abs/brokkr` launches the
family CLI through the pinned brokkr v0.15.2 driver, with `swarmail-evals`
acting as the driver's engine (proposed 0138, `specs/nested-driver-usage/`).
The outer `exec` adapter drops a nested driver's stdout (brokkr#1289), so the
wrapper reads the driver's protocol itself.

**The pin.** `--via-brokkr` needs a version-2 selection whose family carries a
`nested` member, checked before any credential is read:

```json
"nested": {
  "driver": "/abs/brokkr-v0.15.2/brokkr",
  "driver_sha256": "61cba4ee438eb0bd0dbc223f7d394219238aeecc061250e36281a4989e455062",
  "driver_version": "0.15.2",
  "platform": "linux-x86_64",
  "archive_sha256": "ecce9b5acea9e07eb5b9f978b87272aa87273add34f1f215cdc400aeb2f52eb3",
  "effort": "medium",
  "codex_sandbox": "danger-full-access"
}
```

`driver_sha256` is the `brokkr` executable extracted from the release asset
`brokkr-linux-x86_64.tar.gz`, whose `SHA256SUMS` line is `archive_sha256`.
This pin is separate from the delivery engine's
(`.brokkr/recipes/self/tooling.json`). `effort` is required for claude and
codex (`low|medium|high|xhigh|max`) and refused for dsh. `codex_sandbox`
(`read-only|workspace-write|danger-full-access`) is required for codex and
refused elsewhere. Refusals, each before any private write or spawn:
`driver_invalid` (not an absolute path to an executable), `driver_pin_missing`
(no `nested` member, or a version-1 selection), `driver_selection_mismatch`
(the flag names another path), `driver_version_unsupported`,
`driver_platform_unsupported`, `driver_archive_unrecognized`,
`driver_digest_invalid`, `driver_effort_invalid`, `driver_sandbox_invalid`,
`driver_executable_unsafe`, `driver_executable_not_native` and
`driver_digest_mismatch` (the bytes), and `driver_version_mismatch` (the
bounded `--version` banner is not `brokkr 0.15.2`). The CLI's own pin
(native binary, or dsh's interpreted pin) is rechecked exactly as for a direct
launch.

**The exchange.** The wrapper runs `brokkr driver <family> -- <flags>`. Only
the family's flags follow the separator:

| family | flags after `--` | how the driver finds the CLI |
|---|---|---|
| claude | `--permission-mode acceptEdits --model M --effort E` | `BROKKR_CLAUDE_BIN` = the pinned binary, its directory first on `PATH` |
| codex | `--model M --effort E --sandbox CLASS` (prompt on stdin) | `codex` on `PATH`, whose first entry is the pinned binary's directory; the pinned file must be named `codex` (`driver_cli_unresolvable`) |
| dsh | `--model route/model` | `BROKKR_DSH_BIN` = the pinned entry file |

dsh's driver runs `BROKKR_DSH_BIN` as one program, so the pinned entry must
start the pinned runtime itself: it must be executable and its `#!` line must
be the runtime's absolute path, or `/usr/bin/env node` with the runtime named
`node` (its directory is first on `PATH`). Anything else refuses
`driver_entry_not_executable` or `driver_entry_shebang_unsupported`. No script
of ours stands between the runtime and the entry.

The wrapper sends `hello` (with `engine_version`), then `start` with exactly the
eight input keys (`feature` = the goal, `phase`/`seat` = `trial`, `role_path` =
a fixed charter in a fresh `.swarmail-seat-<attempt>/` under the work
directory, `workdir`, `result_path` in the same directory,
`allowed_results` = `done`, `stuck`, `context` = the object
`{"task": <task id>}`), then `shutdown` once and closes the driver's stdin.
On every ending (a result, a protocol error, EOF) the driver's stdin is
closed and the driver gets `DRIVER_EXIT_GRACE` (3 s) to exit; one that
ignores `shutdown` or keeps its stdout open is killed then, and the typed
receipt stands. It reads `capabilities`,
`accepted`, `checkpoint`* and one `result` with the bounded strict reader of
`evals/src/driver.rs`: 64 KiB per frame, 4 MiB and 4096 frames per exchange,
depth 16, duplicate keys and repeated `msg_id`s refused, both identities on
every post-start frame, a checkpoint's facts in its `data` object. Every
error is a fixed code, never peer bytes. The driver's stderr is discarded.

**Endings.** `accepted` is the launch witness. The wrapper writes `ready` on
descriptor 3 only when it arrives. After the exchange it writes one receipt
line, `nested {"disposition", "attempt_id", "family", "facts"}`. Descriptor 3
is close-on-exec before the driver spawns, so neither the driver nor the CLI
can write it.

| what the driver did | wrapper | the trial |
|---|---|---|
| nothing (silent exit 0, or no `capabilities`) | `unstarted`, `driver_launched_nothing` | `harness_error` (`agent_unstarted`), never `gave_up` |
| a `failed` result before `accepted` | `unstarted`, `driver_refused_start` | `harness_error` |
| accepted, then EOF without a result | `ready` + receipt `driver_result_missing` | `harness_error` with its usage |
| accepted, then `failed` | `ready` + receipt `driver_session_failed` | `harness_error` with its usage |
| accepted, then `succeeded` without an object naming `done`/`stuck` | `ready` + receipt `driver_result_invalid` | `harness_error` with its usage |
| accepted, then a protocol violation | `ready` + receipt naming the violation | `harness_error` with its usage |
| accepted, then `succeeded` naming `done` or `stuck` | `ready` + receipt `completed` | graded as usual |
| killed at the trial deadline | `ready` only | graded as usual; that session's usage `receipt_missing` (no receipt is written on termination) |

The runner gives every nested trial the launch pipe (`--launch-report`, and
`--paid-launch` as before on a paid slot), one per agent. Both switches are
the runner's: a caller's agent argv that carries either, as its own word or
as `--switch=value`, refuses `agent_argv_reserved_flag` before anything
starts, whether it comes after `trial --`, from a matrix agents file or in a
request handed to `validate_request`.

**Fail closed (the operator's 2026-11-24 ruling; the panel's holds on
a73ac17a).** Every paid (version-2) launch, direct or nested, runs only on a
verified binding, checked before any credential is read or anything spawns:

- **the host baseline's grant.** `--paid-launch` is a request, never a
  permission: its identity is a pure function of the selection, so anyone
  can compute it. The baseline coordinator, under its product lock, gives
  each paid slot's worker one private `SOCK_SEQPACKET` endpoint per expected
  agent (descriptors 4 and up; the worker hands agent `i` its own at fd 4).
  The wrapper sends its binding and launches only on the coordinator's
  `granted`, and both sides read the kernel's `SCM_CREDENTIALS`: the reply
  must come from an ancestor of the wrapper running this very executable,
  the request from a descendant of the coordinator's own worker running it.
  Each endpoint answers once; a forged identity, a caller-made socket, a
  replay or a coordinator that is gone refuses (`paid_grant_channel_missing`,
  `paid_grant_unauthenticated`, `paid_grant_refused`,
  `paid_grant_channel_closed`). A worker given a paid context without its
  endpoints refuses before any server starts. The endpoint is
  close-on-exec in every process, so no CLI or driver inherits it. The peer
  evidence is Linux's; elsewhere a paid binding refuses
  (`paid_grant_unsupported`).
- **the recipe route.** No binding, and this process's own mount table
  (`/proc/self/mountinfo`) shows the four DNS inputs and the selection, the
  CLI program (dsh's package root), the driver and the credential file under
  read-only mounts with no writable mount stacked inside. A missing source,
  the systemd resolver directory included, refuses `recipe_bind_missing`.
  Claude's and codex's auth are bound read-only there, which no session has
  been shown to run under (the 2026-11-23 smoke needed `~/.claude`,
  `~/.claude.json` and `~/.codex` writable), so a paid claude or codex launch
  on the recipe route refuses `paid_unbound: recipe_auth_unruled` until the
  operator rules; dsh proves the mount table.

On both routes a nested driver must be the release executable
(`driver_release_unrecognized` otherwise). Anything else refuses
`paid_unbound: recipe_bind_writable` (or `_missing`, `_unverifiable`,
`recipe_auth_unruled`), so a bare `trial` or wrapper with a paid selection
never starts the CLI or the driver. `matrix --brokkr` decodes every agents-file
entry before dispatch, from the selection its wrapper would read (never the
profile alone): any paid entry, including one declared by argv alone behind a
free first slot, refuses `brokkr_paid_unbound` before a slot unless
`.brokkr/recipes/eval/bundle.json` declares every F5 bind below.

**Release pin and test equipment.** A release build of `swarmail-evals`
trusts the published driver digest alone, whatever the environment says. A
debug build (the profile the test suites build) also trusts the offline
stand-in built beside it (`paid-cli-standin`), has the hidden
`grant-standin` subcommand that serves the coordinator's endpoint code to the
command it launches, and takes a shorter exit grace from
`EVAL_TEST_DRIVER_EXIT_GRACE_MS`. Such a build spends nothing but synthetic
credentials: a paid launch whose credential file holds no `"SYNTHETIC-` value
refuses `paid_seam_build_refused`, on every route and with the genuine pin. `swarmail-evals trial` accepts
`--via-brokkr` only beside an amended `*_via_brokkr` profile. The superseded
ones (`claude_2_1_310_via_brokkr`, `codex_0_164_0_via_brokkr`,
`dsh_0_1_5_rc_1_via_brokkr`) still refuse `driver_contract_unknown`. The flag
without a via-brokkr profile, or such a profile without the flag, refuses
`via_brokkr_profile_mismatch`.

**The paid baseline.** `swarmail-evals baseline --via-brokkr /abs/brokkr ...`
is a separate product. Each paid agent takes its amended `*_via_brokkr`
profile and the flag, so the printed `product_id` the operator authorizes
covers the route and the driver path. The controls are unchanged. A nested
family's contract identity (`cli.sha256` in the manifest and on every row) is
a digest over the CLI's pin, the driver's pin, `effort` and `codex_sandbox`.
Resume rechecks it live, and each launch's paid binding must equal it, so a
changed driver, effort or sandbox class refuses (`wrapper_contract_drift`)
rather than running under the recorded identity. A relative driver path
refuses `baseline_driver_locator_invalid`.

**Usage.** The receipt's six facts come only from the family's session-final
checkpoint:

| family | session-final step | per-turn steps never counted |
|---|---|---|
| claude | `claude-code-session-finished` | `seat-turn` |
| codex | `codex-session-finished` | `turn-completed` |
| dsh | `deepseek-harness-session-finished` | `seat-turn` |

The six facts are `input_tokens`, `output_tokens`, `cache_read_tokens`,
`cache_write_tokens`, `num_turns` and `total_cost_usd`. Each is a value or
one reason: `no_session_final`, `field_absent`, `field_malformed`,
`final_conflicting` or (runner-side) `receipt_missing`. An identical repeated
final counts once, and an explicit `0` is kept. The `result` object is
populated from the agent's own file, so it carries no accounting authority,
even where it repeats the totals. The runner settles one session per expected
agent, in assignment order (a lost receipt is `receipt_missing`, an agent
that never launched `agent_unstarted`, an attempt a sibling already owns or
another family's receipt is unattributable), and publishes them as the trial
leaf's usage companion `usage.json`, bound in the leaf's `metadata.json`
(proposed 0138; `evals/README.md`). The schema-2 row carries no usage field.
That is the join (0075 §4, harness-authored).

**Private projections (F5).** The driver gets a cleared environment: the
runtime map (`HOME`, `TMPDIR`, XDG directories, `PATH` with the pinned CLI's
directory first, `LANG`/`LC_ALL`, `PWD`) plus each family's keys. Everything
lands in the trial's fresh private `HOME` (directories 0700, files 0600):

- claude: `.claude/.credentials.json`, `.claude.json` (the MCP servers, user
  scope) and `.claude/settings.json` (the tool allowance);
- codex: `.codex/auth.json` and `.codex/config.toml` (MCP servers, the fixture
  helper, `web_search = "disabled"`, the work directory untrusted), with
  `CODEX_HOME`;
- dsh: `.dsh/profiles/headless/cordis.patch.yml` (the selected route and the
  MCP servers; the driver composes the model and session rows itself), with
  `DSH_HOME`, `DSH_TELEMETRY_DISABLED=1` and only the selected route's key
  (`SPARK_API_KEY` or `DASHSCOPE_API_KEY`), passed by name.

On the host route trials are unconfined (accepted 0076, the direct-wrapper
posture 0137 accepts), so the DNS inputs (`/etc/resolv.conf`,
`/run/systemd/resolve`, `/etc/nsswitch.conf`, `/etc/hosts`) are the host's own;
the opt-in release proof shows the CLI can read them. On the recipe route they
are read-only binds (the paid read set below), and a write through them is
refused by the mount, not by file permissions.
dsh's overlay location comes from the release's own wording ("the headless
profile's `cordis.patch.yml`"). No dsh in this box confirms it; the
operator's dsh session does.

**Codex's sandbox class.** A typed field of the selection, admitted only as
`read-only` or `workspace-write` (the operator's 2026-11-24 ruling).
`danger-full-access` refuses `driver_sandbox_unconfined` before anything is
read or spawned: no enclosing boundary is verified that could stand in for
codex's own. The class moves the contract identity, so resume and the launch
binding refuse a changed one. The via-brokkr codex descriptor
(`ToolSurface::declared`) names this rule instead of the direct launch's
"read-only sandbox". The smoke's `workspace-write` lost the result inside a
brokkr seat box, which is not this environment; `workspace-write` is the
proposed class for the operator's session. The stand-in proofs only show the
seat writing its result inside the work directory under the class the driver
was given. Real codex enforcement is unproven until that session writes its
result.

**Proofs.** `evals/tests/wrappers_launch/nested.rs` drives the real wrapper
against a forge-driver/v1 stand-in (`fake_driver.py` behind the native
forwarder, so it passes the pin), for all three families: the eight keys, the
flags, the projections, the usage, every ending above, the pin refusals, and a
nested agent through `swarmail-evals trial` whose row carries its usage.
A three-agent `trial --task inbox-isolation` under the coordinator's grant
endpoints (`grant-standin`) commits all three sessions' companion and is read
back by `report --usage-from`; without the endpoints the same worker refuses.
The stand-in driver is admitted only by debug builds, as above.
`real_driver.rs` runs the staged release itself over inert stand-in CLIs on
the host route. It is ignored unless `EVAL_BROKKR_DRIVER` names the extracted
v0.15.2 executable (its digest is asserted):

```bash
cd evals && EVAL_BROKKR_DRIVER=/abs/brokkr-v0.15.2/brokkr SWARMAIL_BIN=... \
  cargo test --locked --test wrappers real_driver -- --ignored
```

It caught the `driver` subcommand the stand-in had accepted without. It
shows claude's and codex's session-final tokens, dsh's pinned runtime given
the pinned entry first, the result written inside the work directory, and
the launch pipe unreachable from the CLI. `mount_proof.rs` is the recipe
binds' proof: in a bubblewrap namespace with the binds, a write to an
owner-writable source through each read-only target fails with `EROFS`, the
private result directory takes a write, the sources are unchanged after
teardown, and the namespace's mount table passes the wrapper's verifier while
the host's does not. It needs an unprivileged user namespace, which a boxed
seat lacks, so it is a host proof (ignored by default):
`cd evals && cargo test --locked --test wrappers mount_proof -- --ignored`.

The four seams the earlier plan named (`trial.sh`'s `env -i`, the runner's
fresh agent environment, and the old assembly's cleared environment and
`/dev/null` output) need no crossing under route (b). The usage never leaves
the wrapper except on its own launch pipe. The tests of seams 1 and 2
(`recipe.rs::the_trial_script_strips_the_matrix_environment`,
`recipe.rs::the_runner_rebuilds_the_agents_environment`) stand unchanged, and
the old `through_driver` assembly is gone.

**What stays the operator's.** One authenticated session per family through
this route (accepted 0137, acceptance criterion 4) is the only paid step. Each
must record non-null `input_tokens` and `output_tokens`. Nothing in delivery
runs one.

### The paid read set (brokkr fact F5)

A paid trial under the recipe reads the operator's private selection file,
the credential file it names, the driver and CLI executables with their
install trees, and needs the network. The operator approved
`.brokkr/recipes/eval/bundle.json` by name on 2026-11-24 for these binds
alone. The `trial` seat declares, besides `network: true`:

| path | mode | why |
|---|---|---|
| `/etc/resolv.conf`, `/run/systemd/resolve`, `/etc/nsswitch.conf`, `/etc/hosts` | `ro` | DNS inputs |
| `~/.config/swarmail-evals` | `ro` | the operator's version-2 selection |
| `~/.local/share/brokkr-0.15.2` | `ro` | the extracted release driver |
| `~/.local/share/claude` | `ro` | Claude Code's native install tree |
| `~/.volta` | `ro` | codex-cli behind Volta, and the pinned Node runtime |
| `~/.claude`, `~/.codex` | `ro` | Claude's and Codex's source auth and config |
| `~/.local/bin` | `ro` | Claude Code's launcher |
| `~/.claude.json` | `ro` | Claude Code's user configuration |
| `~/.dsh/spark.env` | `ro` | dsh's Spark route environment |
| `~/.dsh/profiles` | `overlay` | dsh's profiles: readable, writes stay in the seat |

Keep the selection, the driver, the CLIs and dsh's package under these trees:
the wrapper refuses any of them outside a read-only mount. dsh's route key
lives in the selection's credential file. `matrix --brokkr` refuses a paid
agent unless the bundle declares every entry with its mode
(`evals/src/binds.rs`), and a paid dsh wrapper re-proves the effect in its own
mount table; a paid claude or codex wrapper refuses there
(`recipe_auth_unruled`). The 2026-11-23 smoke needed `~/.claude`,
`~/.claude.json` and `~/.codex` read-write; the operator approved read-only
binds only, so they stay `ro`, and **a paid claude or codex session on the
recipe route under read-only auth is unproven and needs an operator ruling
before it runs.** All fourteen sources exist on the operator's host as of
2026-11-24 (the dispatcher's host proofs); a source missing inside a seat
refuses with a typed reason (`recipe_bind_missing`), never silently drops.
Whether brokkr applies these binds to a recipe seat, and the release driver
launching inside one, is not yet observed: the host proofs cover the host
route's real driver and the mount primitive only.
