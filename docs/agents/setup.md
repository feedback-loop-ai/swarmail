# Wire an agent to swarmail

Copy-paste setup for Claude Code, Codex, Cursor and dsh, a skill, and one
`AGENTS.md` block that teaches the clear → act → wait → extract loop, how the
application's mail reaches the inbox you read, and the traps. Every JSON, TOML
and YAML snippet below is written verbatim into a fresh project by
[`tests/agent_setup_docs.rs`](../../tests/agent_setup_docs.rs) and driven
against the real `swarmail` binary on the transport it declares; the section
[What CI proves](#what-ci-proves) says exactly how far that goes.

## 1. Start the server

<!-- not-executed: needs a network pull of the published image -->
```sh
docker run --rm --stop-timeout 30 -p 1025:1025 -p 1110:1110 -p 8025:8025 ghcr.io/feedback-loop-ai/swarmail:main
```

<!-- not-executed: builds from a checkout, which CI already built -->
```sh
cargo install --locked --path .
```

The server the snippets below talk to is started by this command, which the
suite runs with its argv unchanged:

```sh
swarmail serve
```

It listens for SMTP on `127.0.0.1:1025`, POP3 on `127.0.0.1:1110` and HTTP on
`127.0.0.1:8025`. The MCP endpoint is `POST /mcp` on the HTTP port, so every
URL below is `http://127.0.0.1:8025/mcp`. Nothing is stored on disk unless you
configure a data file (`SWARMAIL_DATA_FILE`), and without one a restart loses
every mail.

### Where mail lands

The inbox an agent reads is named by how the application's mail arrives, so
decide the route before you trigger anything. Name inboxes with lowercase
letters, digits and `-` (`[a-z0-9-]`): such a name is legal on both routes.

- **SMTP AUTH, on every server.** The AUTH username names the inbox, any
  password is accepted, and a session that sends no AUTH delivers to the inbox
  `default`
  ([decision 0004](../decisions/0004-per-test-inboxes-via-smtp-auth.md)).
  Point the application under test at `smtp://<inbox>:x@127.0.0.1:1025`. If
  your task or environment already hands you an inbox, use that one.
- **A recipient routed by domain, on a server started with a route domain.**
  Start the server with `--route-domain <domain>` or
  `SWARMAIL_ROUTE_DOMAIN=<domain>`; a recipient `<local>@<label>.<domain>` is
  then delivered to the inbox `<label>`, whatever the AUTH username (a routed
  recipient outranks AUTH), and a label under the domain that is not 1 to 63
  characters of `[a-z0-9-]` is refused with `550 5.1.3`. `swarmail_create_inbox`
  mints such an address
  ([decision 0112](../decisions/0112-recipient-inbox-routing.md), *proposed*;
  the binary in this repository implements it).

The `swarmail serve` command above has no route domain, so there
`swarmail_create_inbox` answers an error that names `--route-domain`. That is
why the block makes the routed clause conditional and teaches the AUTH form
first.

## 2. Claude Code

Claude Code reads `.mcp.json` from the project root. Use the HTTP transport
when the server is already running:

```json
{
  "mcpServers": {
    "swarmail": {
      "type": "http",
      "url": "http://127.0.0.1:8025/mcp"
    }
  }
}
```

Or let Claude Code launch the stdio bridge. `swarmail mcp` speaks JSON-RPC on
stdin and stdout, logs only to stderr, and forwards every line to
`SWARMAIL_URL` (default `http://127.0.0.1:8025`), so the server must still be
running:

```json
{
  "mcpServers": {
    "swarmail": {
      "command": "swarmail",
      "args": ["mcp"]
    }
  }
}
```

The skill for Claude Code is [section 6](#6-the-skill).

## 3. Codex

Codex reads MCP servers from a TOML table. The keys below are the ones the
repository's eval wrappers set.
**Unverified:** the file path. Codex keeps user configuration in
`~/.codex/config.toml`, but that path is not sourced in this repository and CI
never loads it.

```toml
[mcp_servers.swarmail]
url = "http://127.0.0.1:8025/mcp"
startup_timeout_sec = 20
tool_timeout_sec = 60
```

`tool_timeout_sec = 60` matches the longest wait the server allows
(`timeout_ms` of 60000).

## 4. Cursor

**Unverified:** Cursor's file name and shape. The snippet below follows the
`mcpServers` convention Claude Code uses, saved as `.cursor/mcp.json` in the
project; no Cursor binary or documentation is available to this repository,
so CI proves only that the JSON parses and that its URL answers.

```json
{
  "mcpServers": {
    "swarmail": {
      "url": "http://127.0.0.1:8025/mcp"
    }
  }
}
```

## 5. dsh

dsh takes a patch list on the command line (`dsh --patch swarmail.yml …`).
This row inserts the swarmail MCP client; it is the row the eval wrappers
build, and its support for the HTTP transport is schema-evidenced, not
live-proven. `toolCallTimeoutMs` matches the server's 60000 ms wait maximum.

```yaml
- {"insert": [{"id": "mcp-swarmail", "name": "@deepseek-ai/dsh-mcp-client", "config": {"serverName": "swarmail", "transport": "streamable-http", "url": "http://127.0.0.1:8025/mcp", "failOnStartupError": true, "toolCallTimeoutMs": 60000}}]}
```

## 6. The skill

The skill is the block below with a small header, kept byte-identical to
[`docs/agents/swarmail/SKILL.md`](swarmail/SKILL.md); a test compares them.
**Unverified:** the install path and the frontmatter. Claude Code skills are
folders holding a `SKILL.md` with `name` and `description` frontmatter; copy
the folder to `.claude/skills/swarmail/` in your project. Neither the path nor
the frontmatter is sourced in this repository, and CI never loads the skill.

## 7. The AGENTS.md block

Paste this into your project's `AGENTS.md`. It is the single `markdown` fence
of this guide.

```markdown
## Reading mail with swarmail

Use the swarmail MCP tools whenever a flow mails you something you must read:
a sign-up message, a one-time code, a magic link or a reset link.

If your task or environment names your swarmail inbox, use that name and pass
it as `"inbox"` to every tool. Otherwise choose a name of lowercase letters,
digits and `-`, and point the application's SMTP at
`smtp://<inbox>:x@127.0.0.1:1025`: the SMTP AUTH username names the inbox and
any password is accepted. Mail sent without AUTH lands in the inbox
`default`, not in yours. On a server started with `--route-domain`, you may
instead call `swarmail_create_inbox` and give the flow the address it returns.

**If the flow has already mailed you, do not clear: wait, or read, straight
away.** The wait returns mail that is already in your inbox.

Otherwise work in four steps, in this order: clear, act, wait, extract.

1. **Clear.** Call `swarmail_clear_inbox` with your inbox. Clear before you
   act, never between act and extract: the clear deletes the mail you are
   waiting for.
2. **Act.** Trigger the flow exactly once, with the application pointed at
   your inbox as above. To seed a message instead of sending one, call
   `swarmail_seed_email`. A second trigger sends a second mail and makes the
   first code stale.
3. **Wait.** Call `swarmail_wait_for_email` with your inbox and a
   `timeout_ms` of at most 60000. For one code or one link, use
   `swarmail_wait_for_code` or `swarmail_wait_for_link` instead. The wait
   registers before it scans, so wait after act and never poll-then-decide.
4. **Extract.** Take the email id from the wait, then call
   `swarmail_extract_codes` or `swarmail_extract_links` on that id. Codes and
   links are extracted at ingest, so extract reads stored results and a
   cleared or pruned mail is gone from them too.

A `250` from SMTP, or a successful `swarmail_seed_email`, means the mail is
queryable now. Never re-send after it.

### Failure classes

- `wrong_message`: you read a mail meant for someone else. Keep to your own
  inbox and match the recipient and subject before you extract.
- `stale_code`: you used a code from an older mail. Trigger once, and take
  the code from the newest mail.
- `wrong_link`: you opened a look-alike link. Wait with
  `swarmail_wait_for_link` and select the link by its `anchor_text`, `host`
  and `path`, not by its position in a list.
- `wait_timeout`: the wait ended with nothing. Read the timed-out answer's
  `near_misses` (mail that arrived but did not match, listed when the inbox
  holds any), then look at the inbox before you re-trigger; a second trigger
  makes the first code stale.
- `gave_up`: you stopped before the task was done. Wait again, or report why
  no mail can arrive.
- `tool_misuse`: you called a tool with the wrong arguments, or in the wrong
  order. Read the tool's schema and the steps above.
```

## 8. The tools

The server answers these tools on `tools/list`; a test compares this
table with the live list, so a tool added or removed without a row here fails
CI. The block above names only the loop's tools.

| Tool | What it does |
|---|---|
| `swarmail_clear_inbox` | Delete every mail in one inbox: the loop's first step. |
| `swarmail_clear_all` | Delete every mail in every inbox. |
| `swarmail_create_inbox` | Mint a private inbox name and the address that routes to it; needs `--route-domain`. |
| `swarmail_seed_email` | Inject a synthetic mail into an inbox without SMTP. |
| `swarmail_wait_for_email` | Block until mail matching a filter arrives; push-based, `timeout_ms` from 0 to 60000. |
| `swarmail_wait_for_code` | Wait for mail carrying a code and answer the newest one. |
| `swarmail_wait_for_link` | Wait for mail whose link matches the `anchor_text`, `host` and `path` selectors you give. |
| `swarmail_extract_codes` | The verification codes of one mail, read from the results stored at ingest. |
| `swarmail_extract_links` | The plain URLs of one mail, read from the results stored at ingest. |
| `swarmail_get_email` | Fetch one mail in full by id. |
| `swarmail_get_latest_email` | Fetch the most recent mail of an inbox. |
| `swarmail_search_emails` | Search an inbox by recipient, sender, subject or arrival time. |
| `swarmail_delete_email` | Delete one mail by id. |
| `swarmail_list_attachments` | List the attachments of one mail, recorded at ingest. |
| `swarmail_get_attachment` | Fetch one attachment of a mail by its index in `swarmail_list_attachments`. |
| `swarmail_get_deliverability_report` | A report on one mail's stored bytes. |
| `swarmail_set_chaos` | Enable deliberate SMTP failure for resilience tests. |
| `swarmail_clear_chaos` | Disable all chaos rules. |

## 9. The guarantees the loop relies on

- A `250` (or a successful seed) means the mail is queryable now by the
  running server; when a data file is configured the mail is on disk before
  that reply; without one, a restart loses mail. Never re-send after a `250`
  ([decision 0001](../decisions/0001-synchronous-insert-lossless.md)).
- The wait registers before it scans, so a mail that arrives between the two
  can never be missed. Wait after act, and never poll-then-decide
  ([decision 0002](../decisions/0002-awaits-subscribe-before-checking.md)).
- Codes and links are extracted at ingest, so extract reads stored results
  and a cleared or pruned mail is gone from them too
  ([decision 0005](../decisions/0005-extraction-at-ingest.md)).
- Unless the flow has already mailed you, clear before you act, never between
  act and extract: the clear deletes the mail you are waiting for. If the flow
  has already mailed you, do not clear: the wait returns mail that is already
  stored, and a clear would delete it with no way back.

## What CI proves

For each JSON, TOML and YAML snippet above, CI proves two things: the bytes
parse, and the server answers on the transport the snippet declares. The suite
writes the bytes into a fresh temporary project, reads them back, checks them
against the binary's own defaults, then starts the real `swarmail` binary and
runs `initialize`, `tools/list`, clear, seed, wait and extract with a seeded
code. Those five round trips prove each snippet's transport and nothing about
where an application's mail lands, because they seed through
`swarmail_seed_email` with an explicit inbox. The executed bytes differ from
the shipped bytes in one place: the HTTP port is replaced by an ephemeral one
(and the stdio and start commands get their address from the environment),
because tests cannot own port 8025.

One more test drives the loop and the route the block teaches over real SMTP:
it takes the `smtp://` URL from the block, sends mail to the server with the
URL's user as the AUTH name, and reads it with `swarmail_wait_for_email` and
`swarmail_extract_codes`. It also covers the already-mailed case (mail
delivered before any clear is found, and a clear deletes it) and two
wrong-route controls (mail sent without AUTH lands in `default`; mail sent with
another AUTH name lands in that inbox). A last test starts the same command
with `SWARMAIL_ROUTE_DOMAIN` set and shows that `swarmail_create_inbox` mints
an address that routes, and that on the default server it is an error naming
`--route-domain`.

CI does not prove that Claude Code, Codex, Cursor or dsh load these files.
That is an operator-owned receipt. The labels above marked **unverified** are
the parts of this guide that no source in the repository backs: Cursor's file
name and shape, the Codex config path, the skill's install path and its
frontmatter. The TOML and YAML readers in the suite are subset validators for
the snippets written here, not parsers.

A fresh project wired from this guide has not been run against a real model:
the shipped eval wrappers run private configurations and refuse a project
`.mcp.json` or `AGENTS.md`, so a trial there measures the wrapper's wiring and
not this guide. That measurement is open.
