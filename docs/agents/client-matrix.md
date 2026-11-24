# The MCP client matrix: checklist and receipts

Part of issue [#392](https://github.com/feedback-loop-ai/swarmail/issues/392)
(epic #164). The epic's compatibility exit criterion is a green recorded
result for Claude Code, Codex, the MCP Inspector and Claude Desktop.
Decisions [0079](../decisions/0079-mcp-version-negotiation.md) and
[0080](../decisions/0080-mcp-source-reconciliation.md) count only observations
made through the named client. A local driver, a fake client, a CI run or an
empty receipt directory proves nothing about any of the four.

This page is the fixed checklist and the receipt format. The wiring is not
repeated here: each route links to its section of the
[setup guide](setup.md). Recording the receipts is the operator's job.
Interactive clients need a person.

**Status, 2026-11-22: no receipts yet.** [`docs/evals/clients/`](../evals/clients/README.md)
holds no receipt. `tests/agent_setup_docs.rs` checks the shape of each
receipt there and prints `no receipts yet` while the directory is empty. It
never reports compatibility. **Update, 2026-11-24:** the MCP Inspector also
has a headless CLI route, recorded by `swarmail-evals client-matrix run`
([below](#headless-the-inspector-cli-through-the-runner)). It has been proven
only with an offline stand-in client, so there are still no receipts.

## Routes at a glance

| Client | Wiring | Transport | Who runs it |
|---|---|---|---|
| Claude Code (`claude-code`) | [setup § 2](setup.md#2-claude-code), the HTTP entry | `http` | a person, or headless (`claude -p`) on a host where the operator has signed the CLI in |
| Codex (`codex`) | [setup § 3](setup.md#3-codex) | `http` | a person, or headless (`codex exec`) on a host where the operator has signed the CLI in |
| MCP Inspector (`mcp-inspector`) | the URL from [setup § 1](setup.md#1-start-the-server) | `http` | a person through the Inspector's web UI, or headless through its CLI and the runner (§ 4) |
| Claude Desktop (`claude-desktop`) | [setup § 2](setup.md#2-claude-code), the stdio entry | `stdio` | needs a person: the route goes through the Desktop app |

The headless commands for Claude Code and Codex are given below. They are
**unverified**: this repository holds no source for either CLI's flags, and
no sandbox here runs an authenticated client. A headless run is recorded with
`"mode": "headless"`. It still needs a client the operator has signed in.

## 1. Prepare one observation set

Each client gets its own set: one fresh server, one client session and one
delivery. Do not reuse a server, a session or a stored mail across clients.

1. **Build main, clean.** In a checkout of `main` with a clean tree:

   ```sh
   git fetch origin && git switch --detach origin/main
   test -z "$(git status --porcelain)" && git rev-parse HEAD   # swarmail_commit, 40 hex digits
   cargo build --locked --release --bin swarmail
   BIN="$PWD/target/release/swarmail"
   sha256sum "$BIN"            # macOS: shasum -a 256 "$BIN"; executable_sha256, 64 hex digits
   ```

   Every later step launches exactly `$BIN`. A Desktop entry or a capture
   wrapper names this absolute path. Hash the file at the path the client
   actually launches.

2. **Start a fresh memory-only server** with the setup guide's start command,
   [setup § 1](setup.md#1-start-the-server), run as `$BIN`: no data file and
   no chaos rule.

   ```sh
   env -u SWARMAIL_DATA_FILE -u SWARMAIL_ROUTE_DOMAIN "$BIN" serve
   ```

   It must log SMTP on `127.0.0.1:1025` and HTTP on `127.0.0.1:8025`. If
   another process holds a port, stop that process. Do not move the ports:
   the wiring in the setup guide names these defaults. If the server stops or
   restarts during the set, abandon the set and start again. A restarted
   memory-only server has lost the mail.

3. **Start private observation before the client connects** (§ 3 below).

4. **Deliver the fixture.** Save this synthetic message as
   `matrix-fixture.eml`:

   ```text
   From: Matrix Sender <sender@matrix.test>
   To: operator@matrix.test
   Subject: Swarmail client matrix fixture
   MIME-Version: 1.0
   Content-Type: multipart/mixed; boundary="matrix-boundary"

   --matrix-boundary
   Content-Type: text/plain; charset=utf-8

   Your verification code is 482913.

   --matrix-boundary
   Content-Type: text/plain; name="matrix.txt"
   Content-Disposition: attachment; filename="matrix.txt"
   Content-Transfer-Encoding: base64

   aGVsbG8gbWF0cml4Cg==
   --matrix-boundary--
   ```

   Send it over SMTP without AUTH, so it lands in the inbox `default`
   ([setup, "Where mail lands"](setup.md#where-mail-lands)):

   ```sh
   curl -sv --crlf smtp://127.0.0.1:1025 --mail-from sender@matrix.test \
     --mail-rcpt operator@matrix.test --upload-file matrix-fixture.eml 2>&1 | grep '^< 250' | tail -1
   ```

   The last line is the final DATA reply, `250 2.0.0 OK: stored as <id>`.
   That `<id>` is the **stored ID**. Every step below must match it. A `250`
   means the mail is queryable now (decision 0001), so never re-send. Any
   other final reply ends the set.

The fixture's known values are:

| Value | Expected |
|---|---|
| inbox | `default` |
| subject | `Swarmail client matrix fixture` |
| code | `482913` |
| attachment | filename `matrix.txt`, content type `text/plain`, decoded size `13` |

## 2. The checklist: six steps, one session

All six steps run in the one client session that connected to the fresh
server. A step passes only when the client's own request and the server's
successful reply to it are both observed (§ 3). The client's summary or a
model's prose is not evidence. A cached tool list, a second session or a
separate `curl` handshake is not evidence either. Otherwise the step fails
with a short, specific reason, for example "client hid the raw reply" or
"text block only, no structuredContent". If `initialize` fails, every later
step fails too, with a reason that says initialization blocked it.

| Step key | Call | Pass when |
|---|---|---|
| `initialize` | the client's own `initialize` | The captured JSON-RPC reply is `"jsonrpc": "2.0"` with the request's `id` and a `result` whose `protocolVersion` is a nonblank string. That string is `negotiated_revision`. |
| `list_tools` | `tools/list` | The client sent it, and the descriptors returned include `swarmail_get_latest_email`, `swarmail_wait_for_code`, `swarmail_search_emails` and `swarmail_list_attachments`. |
| `read` | `swarmail_get_latest_email` with `{"inbox": "default", "count": 1}` | The one email returned has the stored ID and the fixture subject. |
| `wait_for_code` | `swarmail_wait_for_code` with `{"inbox": "default", "timeout_ms": 0}` | `status` is `selected`, `selected.code` is `482913` and `selected.email_id` is the stored ID. |
| `structured_content` | `swarmail_search_emails` with `{"inbox": "default"}` | `result.structuredContent` is an object whose `emails` holds the stored ID. A text block alone fails this step. Under `2025-03-26`, which has no structured output, it fails too. |
| `list_attachments` | `swarmail_list_attachments` with `{"id": "<stored ID>"}` | The reply's `id` is the stored ID, and one entry is `matrix.txt`, `text/plain`, `size` 13. |

`timeout_ms: 0` is enough because the mail is already stored when the wait
starts. The wait subscribes, then scans what is stored (decision 0002), so it
never sleeps.

When a client asks a model to make the calls, paste this prompt and replace
`<stored ID>` with the ID. The prompt names no expected code.

```text
Use only the swarmail MCP tools, one call each, in this order, and show me each raw result:
1. swarmail_get_latest_email with {"inbox": "default", "count": 1}
2. swarmail_wait_for_code with {"inbox": "default", "timeout_ms": 0}
3. swarmail_search_emails with {"inbox": "default"}
4. swarmail_list_attachments with {"id": "<stored ID>"}
Do not call any other tool and do not retry a call.
```

## 3. Observe the raw exchange privately

Start the capture before the client connects and keep it until the set ends.
Keep the capture in a private directory (`umask 077`). It holds the mail and
may hold machine names, so never commit it.

- **HTTP (Claude Code, Codex, Inspector).** Record loopback traffic on port
  8025. Use `lo` on Linux and `lo0` on macOS:

  ```sh
  umask 077; mkdir -p ~/matrix-capture
  sudo tcpdump -i lo -U -w ~/matrix-capture/http.pcap 'tcp port 8025'
  ```

  Afterwards list the requests with
  `tshark -r ~/matrix-capture/http.pcap -Y http.request -T fields -e tcp.stream -e http.file_data`.
  For each JSON-RPC request, follow its stream with
  `tshark -r ~/matrix-capture/http.pcap -q -z follow,tcp,ascii,<stream>`, or
  use Wireshark's Follow → HTTP Stream. Read the reassembled response on the
  same connection and match its `id` to the request's. The server answers
  JSON, not SSE, so each reply is one body.

- **stdio (Claude Desktop).** Point the client at a pass-through wrapper
  instead of `swarmail` itself. The wrapper copies the bridge's stdin and
  stdout to private files, leaves stdout carrying only protocol bytes, and
  passes stderr through to the client's log:

  ```sh
  #!/bin/sh
  # swarmail-mcp-capture: temporary; delete it after the set
  umask 077; d="$HOME/matrix-capture"; mkdir -p "$d"
  tee -a "$d/stdin.ndjson" | /absolute/path/to/target/release/swarmail mcp | tee -a "$d/stdout.ndjson"
  ```

  Make it executable and put its absolute path in the client's `command`.
  Each line is one JSON-RPC message. Match each request in `stdin.ndjson` to
  the reply in `stdout.ndjson` with the same `id`.

Before recording, copy only the `initialize` reply into the receipt. Redact
machine-specific values in place (a host name, a path, an account, a token)
as `"<redacted>"`. Keep `jsonrpc`, `id`, `protocolVersion` and every other
protocol member, extension members included. Then delete the capture
directory, and remove the wrapper and its config entry.

## 4. Per-client routes

### Claude Code

1. In a fresh project, write the HTTP `.mcp.json` from
   [setup § 2](setup.md#2-claude-code). Start the capture (§ 3, HTTP).
2. Record `claude --version` as `client_version`.
3. Start `claude` in the project and approve the project's `swarmail` server
   when asked. `/mcp` shows it as connected. That connection is the
   `initialize` and `tools/list` to read from the capture.
4. Paste the prompt from § 2. Approve each of the four tool calls.
5. Read each step's request and reply from the capture.

Headless (**unverified** flags), on a signed-in host:
`claude -p "<the prompt>" --allowedTools mcp__swarmail__swarmail_get_latest_email,mcp__swarmail__swarmail_wait_for_code,mcp__swarmail__swarmail_search_emails,mcp__swarmail__swarmail_list_attachments`.
The capture is read the same way.

### Codex

1. Add the table from [setup § 3](setup.md#3-codex). That section's caveat
   still applies: the config path is unverified. Start the capture (§ 3, HTTP).
2. Record `codex --version` as `client_version`.
3. Start a `codex` session. `/mcp` lists the `swarmail` server and its tools.
   The capture holds that session's `initialize` and `tools/list`.
4. Paste the prompt from § 2 and approve each call.
5. Read each step's request and reply from the capture.

Headless (**unverified** flags), on a signed-in host: `codex exec "<the prompt>"`,
with tool approval allowed for the swarmail server. The capture is read the
same way.

### MCP Inspector

1. Start the capture (§ 3, HTTP). The Inspector's proxy is the process that
   talks to port 8025, so the capture sees its requests.
2. Start an installed MCP Inspector and record its version, which is the
   installed package's version (for an npm install,
   `npm ls -g @modelcontextprotocol/inspector`).
3. Choose the Streamable HTTP transport with the URL
   `http://127.0.0.1:8025/mcp` from [setup § 1](setup.md#1-start-the-server),
   then **Connect**.
4. Open **Tools**, then **List Tools**.
5. For each call in § 2, select the tool, enter the arguments in its form and
   choose **Run Tool**. The response pane shows the reply. Check it against
   the capture.

The UI route needs a person and keeps the one-session rule of § 2.

#### Headless: the Inspector CLI through the runner

The Inspector's CLI mode runs one method per launch. Proposed decision
[0138](../decisions/0138-inspector-cli-evidence.md) lets one set span five
launches against one fresh server and one delivery, each launch with its
own `initialize`. A receipt made this way (`"mode": "headless"`) shows that
each method worked in a fresh session at the first launch's revision. It
does not show one long session; the UI route above does.

The runner, `swarmail-evals client-matrix run` in the
[`evals/`](../../evals/README.md) workspace, does steps 2–4 of § 1 itself.
It starts `$BIN` memory-only on ephemeral loopback ports, delivers this
page's fixture over SMTP once, and takes the stored ID from the final DATA
reply. For each launch it starts a loopback observer in front of `/mcp`, so
the capture of § 3 is not needed. It keeps only the first `initialize`
reply, with this host's name, the home directory, the user and the run's
paths redacted. It downloads nothing. It runs the argv given after `--` and
appends `--cli <observer URL> --transport http` and then one of these:

| Step | Appended arguments |
|---|---|
| `list_tools` | `--method tools/list` |
| `read` | `--method tools/call --tool-name swarmail_get_latest_email --tool-arg count=1 --tool-arg inbox=default` |
| `wait_for_code` | `--method tools/call --tool-name swarmail_wait_for_code --tool-arg inbox=default --tool-arg timeout_ms=0` |
| `structured_content` | `--method tools/call --tool-name swarmail_search_emails --tool-arg inbox=default` |
| `list_attachments` | `--method tools/call --tool-name swarmail_list_attachments --tool-arg id=<stored ID>` |

A step passes only when all of these hold for its launch. The launch's own
handshake succeeded at the first launch's revision. The observer saw the
client send exactly that request. Swarmail's reply with the same `id`
carries the expected values of § 2. The CLI exited 0 and printed a result
that carries them too. A clean exit without the request on the wire fails.
The first launch's `initialize` goes in the receipt. If it fails, the four
calls are recorded as failed, blocked by it.

The Inspector must already be installed. The runner never installs it, and
`npx` must not either. Record its version from the installed package, for
example `npm ls -g @modelcontextprotocol/inspector`. Then, with `$BIN` and
the commit from § 1 (`$MAIN_COMMIT`), run from `evals/`:

```sh
cargo run --locked --bin swarmail-evals -- client-matrix run \
  --binary "$BIN" --commit "$MAIN_COMMIT" --client-version "$INSPECTOR_VERSION" \
  --os "$OS_VERSION" --recorded-by operator --output-dir "$DRAFTS" \
  -- npx --offline --no-install "@modelcontextprotocol/inspector@$INSPECTOR_VERSION"
```

An installed executable's path works in place of the `npx` prefix. A
dispatcher records with `--recorded-by dispatcher`, on the operator's
approval. The exit status is 0 when all six steps pass and 1 when a red
draft was written; the failed steps and their reasons are printed. It is 2
for an error before any attributable observation, and then no draft is
written. The draft is `$DRAFTS/YYYY-MM-DD-mcp-inspector.json`. The runner
never overwrites an existing file. Review the draft, copy it into
[`docs/evals/clients/`](../evals/clients/README.md) and check it with
`cargo run --locked --bin swarmail-evals -- client-matrix validate <file>`.
A red draft is published as red, with its failing step.

**Status, 2026-11-24:** this route has run only against the offline
stand-in of `evals/tests/client_matrix.rs`, not a real Inspector. The
printed-result check expects the CLI to print the method's result as JSON.
If a real Inspector prints something else, its steps fail with "the client
printed no JSON result". That is a fault of this route to report, never a
pass.

### Claude Desktop

1. Write the capture wrapper (§ 3, stdio) with the absolute path of `$BIN`.
2. In Claude Desktop, open **Settings → Developer → Edit Config**. Add the
   stdio entry from [setup § 2](setup.md#2-claude-code), with `command`
   set to the wrapper's absolute path and `args` empty, because the wrapper
   already passes `mcp`.
3. Quit Claude Desktop fully and start it again. Record the version shown in
   **About Claude** as `client_version`.
4. In a new conversation, check that the swarmail tools are connected, then
   paste the prompt from § 2 and allow each call.
5. Read each step's request and reply from `stdin.ndjson` and `stdout.ndjson`.

This route needs a person. There is no headless form.

## 5. The receipt

Write one UTF-8 JSON file per client and date:
`docs/evals/clients/YYYY-MM-DD-<client>.json`, where `<client>` is
`claude-code`, `codex`, `mcp-inspector` or `claude-desktop`. Never overwrite
an existing receipt. A second run of the same client on the same day waits
for the next date. A failed run is recorded too: each step has its own
status, and the reason says why it failed.

| Field | Value |
|---|---|
| `date` | The day of the observation, `YYYY-MM-DD`. It must be a real date and match the file name. |
| `client` | `claude-code`, `codex`, `mcp-inspector` or `claude-desktop`. It must match the file name. |
| `client_version` | The exact installed version or build, nonblank. |
| `os` | The operating system and its version, nonblank. |
| `swarmail_commit` | The full commit of main that was built, 40 lowercase hex digits. |
| `executable_sha256` | The SHA-256 of the executable that was launched, 64 lowercase hex digits. |
| `transport` | `http` or `stdio`. |
| `negotiated_revision` | The `protocolVersion` string of the captured reply's `result`, or `null` when no such string was observed. The key is never omitted. |
| `initialize_response` | The whole captured JSON-RPC reply to `initialize`, redacted (§ 3), or `null` when no JSON reply was captured. An error reply is kept as captured. The key is never omitted. |
| `steps` | An object with exactly the six step keys of § 2. Each value is `{"status": "pass"}` or `{"status": "fail", "reason": "<short reason>"}`. |
| `recorded_by` | `operator` or `dispatcher`. |
| `mode` | `interactive` or `headless`. |

No other field is allowed, and no field may appear twice. The shape check
(`cargo test --locked --test agent_setup_docs client_matrix`) reads every
file in the directory except its `README.md`. The same checker runs on one
file or a directory with `swarmail-evals client-matrix validate`, and on
every draft the runner writes. It refuses a missing, extra or
duplicate field, a step outside the checklist, a wrong type or enum value,
non-hex or wrong-length hashes, an impossible or non-canonical date, a file
name that disagrees with `date` or `client`, and a file that is not a regular
`.json` receipt. It also refuses handshake evidence that contradicts itself:

- `negotiated_revision` differs from the reply's `result.protocolVersion`, or is
  not `null` when the reply has none;
- the reply is not a JSON-RPC 2.0 object with an `id` and exactly one of
  `result` and `error`;
- `initialize` passes without a success reply carrying a nonblank revision;
- `initialize` fails while a later step passes.

The check reports only that the files are well formed. It never says that a
client is compatible. That verdict is the operator's, read from four
receipts whose six steps all pass.

A synthetic example, **not evidence**, with made-up identities and the
file name `2026-11-25-claude-code.json`:

```json
{
  "date": "2026-11-25",
  "client": "claude-code",
  "client_version": "example 0.0.0 (synthetic)",
  "os": "example OS 1.0 (synthetic)",
  "swarmail_commit": "0123456789abcdef0123456789abcdef01234567",
  "executable_sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
  "transport": "http",
  "negotiated_revision": "2025-11-25",
  "initialize_response": {
    "jsonrpc": "2.0",
    "id": 0,
    "result": {
      "protocolVersion": "2025-11-25",
      "capabilities": {"tools": {}},
      "serverInfo": {"name": "swarmail", "version": "0.5.0"}
    }
  },
  "steps": {
    "initialize": {"status": "pass"},
    "list_tools": {"status": "pass"},
    "read": {"status": "pass"},
    "wait_for_code": {"status": "pass"},
    "structured_content": {"status": "fail", "reason": "example: the client showed the text block only"},
    "list_attachments": {"status": "pass"}
  },
  "recorded_by": "operator",
  "mode": "interactive"
}
```
