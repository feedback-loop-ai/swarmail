# Wire an agent to swarmail

Copy-paste setup for Claude Code, Codex, Cursor and dsh, a skill, and one
`AGENTS.md` block that teaches the clear → act → wait → extract loop and its
traps. Every JSON, TOML and YAML snippet below is written verbatim into a
fresh project by [`tests/agent_setup_docs.rs`](../../tests/agent_setup_docs.rs)
and driven against the real `swarmail` binary on the transport it declares;
the section [What CI proves](#what-ci-proves) says exactly how far that goes.

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
a sign-up message, a one-time code, a magic link or a reset link. Work in
four steps, always in this order: clear, act, wait, extract.

1. **Clear.** Pick an inbox name that is yours alone, then call
   `swarmail_clear_inbox` with it. Clear before you act, never between act
   and extract: the clear deletes the mail you are waiting for.
2. **Act.** Trigger the flow exactly once, with a recipient in your inbox. To
   seed a message instead of sending one, call `swarmail_seed_email`. A
   second trigger sends a second mail and makes the first code stale.
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
  the code from the newest mail after your clear.
- `wrong_link`: you opened a look-alike link. Choose the link by its anchor
  text, host and path, not by its position in the list.
- `wait_timeout`: the wait ended with nothing. Report it, then look at the
  inbox before you re-trigger; a second trigger makes the first code stale.
- `gave_up`: you stopped before the task was done. Wait again, or report why
  no mail can arrive.
- `tool_misuse`: you called a tool with the wrong arguments, or in the wrong
  order. Read the tool's schema and the four steps above.
```

## 8. The guarantees the loop relies on

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
- Clear before you act, never between act and extract: the clear deletes the
  mail you are waiting for.

## What CI proves

CI proves two things about each snippet above: the bytes parse, and the
server answers on the transport the snippet declares. For every JSON, TOML
and YAML snippet the suite writes the bytes into a fresh temporary project,
reads them back, checks them against the binary's own defaults, then starts
the real `swarmail` binary and runs `initialize`, `tools/list`, clear, seed,
wait and extract with a seeded code. The executed bytes differ from the shipped
bytes in one place: the HTTP port is replaced by an ephemeral one (and the
stdio and start commands get their address from the environment), because
tests cannot own port 8025.

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
