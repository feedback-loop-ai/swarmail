# Agent setup guides and skills, exercised in CI — issue #161

## Feature and rationale

A stranger with a fresh project must be able to point Claude Code, Codex,
Cursor or dsh at a running swarmail by copying bytes out of one guide, drop
one `AGENTS.md` block into their project that teaches the clear → act →
wait → extract loop and its traps, and trust that the bytes they copied are
the bytes CI ran. Today the only wiring text is an unexercised `jsonc` block
in the README (with `localhost`, where the server binds `127.0.0.1`) and a
copy button on the landing page; nothing proves either, and nothing teaches
the loop.

This change is documentation plus a root integration suite. It adds no
production line, no dependency, no CI workflow edit and no decision. Part of
epic #164; priority P2. The intake framing is
`docs/tasks/issue-161-agent-setup-guides.md` (committed by the implementer
with the delivery, see tasks). Authority for the behaviour the guides
describe: accepted decisions 0001, 0002, 0005 (frozen guarantees), 0012
(the mirror commit precedes the reply), 0029 (stdio bridge, stderr-only
logging), 0039/0044 (bridge line semantics), 0047 (bridge URL), and the
eval harness decisions 0055, 0071 and 0076 as read-only context. No
decision is proposed: the guides describe rulings and change none.

## Panel synthesis and rulings

Both run-local positions were read (`.brokkr/state/design/positions/
simplicity.md` and `robustness.md`; neither is a tracked citation). They
agree on scope — one guide, a skill, one root suite, zero production change,
no dependency, no CI edit, AC-3 open — and disagree on what "exercised"
must mean. Rulings:

1. **Layout: simplicity's.** One guide `docs/agents/setup.md`, one skill
   `docs/agents/swarmail/SKILL.md`, one suite `tests/agent_setup_docs.rs`,
   a README edit. No per-client files, no `docs/agents/` index, no change to
   this repository's own `AGENTS.md` or `.claude/`. Rejected: any helper in
   `src/` — it would put lines on the measured path for test convenience.

2. **Fence scanner: simplicity's narrow subset with robustness's
   fail-closed semantics.** A fence is a line that is exactly three
   backticks at column 0 plus an info string; everything else fence-shaped
   (`~~~`, indented fences, four or more backticks) is red, an unterminated
   fence is red, an unknown info string is red, CR anywhere in a guide file
   is red. The allowed info strings are `json`, `toml`, `yaml` (driven as
   wiring), `markdown` (the AGENTS.md block, exactly one per guide), `sh`
   and `text`. No CommonMark parser is regrown (#248 is the precedent) and
   no regex crate is added. Cost accepted: contributors author the guide to
   this convention; the scanner says why when they do not.

3. **The `not-executed` marker lives on the line immediately above its
   fence, not inside it.** Robustness asked for the marker inside the fence;
   inside a JSON or TOML fence an HTML comment corrupts the very bytes the
   reader copies, so it loses. Rules: `<!-- not-executed: <reason> -->` with
   a non-empty reason; a marker not immediately followed by a fence is red;
   a marker on a `json`, `toml`, `yaml` or `markdown` fence is red (wiring
   is always executed — a marker is not an exercise, as robustness says); an
   unmarked `sh` fence must be the start command the suite runs; an unmarked
   `text` fence is red.

4. **Port: ephemeral loopback with one checked substitution, plus a
   default-equality assertion.** Both positions reject binding 8025 (up to
   eight runs share a host under 0057). The suite writes each wiring snippet
   verbatim into a fresh temporary project, reads it back, reduces it to
   `Http(url)` or `Stdio(command, args)`, and asserts the literal against
   the binary's own defaults obtained by parsing `swarmail::config::Cli`
   from `["swarmail", "serve"]` and `["swarmail", "mcp"]` — never a second
   copy of `8025` in test code. Only then is the authority `127.0.0.1:8025`
   replaced by the ephemeral one, in a replacement that fails unless the
   token occurs exactly once. The stdio snippet is executed with its argv
   unchanged and `SWARMAIL_URL` supplied through `Command::env` — the
   documented knob, never `set_var`. The start command `swarmail serve`
   runs with its argv unchanged and the three `SWARMAIL_*_LISTEN` knobs set
   to `127.0.0.1:0`; the bound ports are read from the server's own startup
   lines as `tests/binary.rs` already does. Cost accepted and stated in the
   suite's doc comment: executed bytes differ from shipped bytes by one
   asserted token (HTTP) or by environment (stdio, serve).

5. **TOML and YAML: no parser, a validating subset reader.** `toml` is in
   neither lockfile and AC-9 bars adding one; both positions converge on a
   purpose-built reader in `tests/`. TOML: `[table]` headers and
   `key = <JSON value>` lines, every value read with `serde_json`; YAML:
   `- <JSON flow node>` rows, the grammar `evals/src/wrappers.rs` already
   emits. Blank lines and `#` comments are allowed; any other line is red.
   This validates snippets we author; it is not a parser and the guide says
   so.

6. **Wire client: plain JSON.** `src/mcp.rs` documents the HTTP transport as
   non-streaming JSON-RPC at `POST /mcp`, so robustness's SSE branch is
   unnecessary and is rejected; the client POSTs `content-type:
   application/json` and reads one JSON body. The stdio client speaks NDJSON
   to the child as `tests/binary.rs`'s `McpChild` does. Each exchange has a
   deadline; each child is killed and reaped on every exit path, stderr
   drained and printed on failure (robustness D3, adopted).

7. **Red controls run in-process over the same functions, not against a
   second server.** Robustness wanted the port control to run the whole
   harness against a mutated document; a failing live dial costs a timeout
   per control and proves only that a wrong port does not answer. The
   controls feed mutated strings to the extraction, reduction, default-
   equality, tool-name and prose-pin functions and assert refusal. Cost:
   the controls prove the checks, and the green live dial on the unmutated
   bytes proves the transport.

8. **Seed: one benign seed through the documented tool.** Robustness's
   hostile SMTP DATA seed is rejected: the guide's loop seeds with
   `swarmail_seed_email`, not DATA, so dot-stuffing and bare CRLF never
   reach this suite; SMTP hostility is `tests/burst.rs` and the SMTP unit
   tests' job. The seed carries one six-digit code in `text`; the extract
   step must return exactly it.

9. **The 0001 sentence is the verified one.** The implementer need not
   guess: `Store::insert` documents that the mirror commit completes before
   the caller replies (0001, 0012, 0023). The guide states: a `250` (or a
   successful seed) means the mail is queryable now by the running server;
   when a data file is configured the mail is on disk before that reply;
   without one, a restart loses mail. Never re-send after a `250`. This is
   robustness's demand with the true wording.

10. **Loop teaching.** Adopt simplicity's named trap: clear before you act,
    never between act and extract — the clear deletes the mail being
    waited for. Adopt robustness's 0002 and 0005 sentences: the wait
    registers before it scans, so wait after act and never poll-then-decide;
    codes and links are extracted at ingest, so `extract` reads stored
    results and a cleared or pruned mail is gone from them too. Reject
    robustness's cursor-resume teaching and webhook pinning: the guide is
    silent on webhooks, and after a wait timeout it teaches "report
    `wait_timeout`, then look before you re-trigger", because a second
    trigger makes the first code `stale_code`.

11. **AC-6 source is `evals/src/state.rs`, read as text.** Simplicity is
    right that 0071 names three classes, not six; the closed enum is
    `FailureClass` (eight variants). The suite reads the enum body from the
    file at runtime, converts variants to snake_case, and asserts the six
    agent-caused names the block carries are members and that the block
    names no class outside the enum. The root suite does not depend on the
    evals crate.

12. **Unverified formats are labelled, not asserted.** No client binary is
    on PATH and web fetch is off. Claude Code `.mcp.json` (HTTP and stdio)
    is sourced from `evals/src/wrappers.rs` and the README; Codex
    `[mcp_servers.swarmail]` keys from `wrappers.rs`; dsh's insert row from
    `wrappers.rs` and `evals/wrappers.md` ("schema-evidenced, not
    live-proven"). The Codex config file path, Cursor's `.cursor/mcp.json`,
    the skill install path and the skill frontmatter are unsourced in this
    realm and carry a visible "unverified" label. CI proves parse and
    transport only (AC-8). Both positions agree.

13. **No TLS snippet, no evals in CI, no scripted docs-following agent.**
    Both positions agree; a TLS fixture would expire under a test, evals CI
    is #201's, and a config-reading agent would duplicate the oracle to
    prove what the round trip already proves.

14. **Out of scope, recorded as residual.** `docs/index.html`'s copy-button
    `.mcp.json` with `localhost` is the landing page, not a guide file, and
    is left as is. AC-3 (one real model wired from the docs) cannot be
    closed here: the shipped wrappers run private configs and refuse any
    `.mcp.json` or `AGENTS.md` in the ancestry, so even a paid trial would
    measure the wrapper's wiring, not the docs. The issue's own acceptance
    criterion therefore stays open and is reported as not measured.

## Requirements

- R1. `docs/agents/setup.md` carries, in this order: starting the server
  (`swarmail serve` as the one executed `sh` fence; install commands marked
  not-executed), one section per client — Claude Code `.mcp.json` HTTP,
  Claude Code `.mcp.json` stdio, Codex `config.toml`, Cursor
  `.cursor/mcp.json`, dsh `--patch` overlay — the skill, the `AGENTS.md`
  block as the guide's single `markdown` fence, the guarantees the loop
  relies on, and a "what CI proves" section that states AC-8's boundary and
  the unverified labels. Every URL is `http://127.0.0.1:8025/mcp`; the stdio
  snippet is `{"command": "swarmail", "args": ["mcp"]}` with no `env`. The
  dsh row keeps `toolCallTimeoutMs: 60000`, the wait tools' maximum.
- R2. `docs/agents/swarmail/SKILL.md` is YAML frontmatter (`name:
  swarmail`, one-line `description:`) followed by a body byte-identical to
  the guide's `markdown` fence.
- R3. The `AGENTS.md` block names the four steps in order — clear, act,
  wait, extract — and exactly the six agent-caused classes `wrong_message`,
  `stale_code`, `wrong_link`, `wait_timeout`, `gave_up`, `tool_misuse`,
  each with the trap that produces it; names only tools that exist; and
  teaches the clear-before-act, wait-after-act and look-before-re-trigger
  rules.
- R4. `tests/agent_setup_docs.rs` extracts every fence of both guide files
  through one function, drives every wiring fence through `initialize`
  (asserting `protocolVersion`), `notifications/initialized`, `tools/list`
  (non-empty), `swarmail_clear_inbox`, `swarmail_seed_email`,
  `swarmail_wait_for_email` and `swarmail_extract_codes` against the real
  binary on ephemeral loopback ports, per-test inbox and temporary
  directory, with deadlines and child reaping; and pins the prose per
  rulings 9–11 in the style of `tests/listener_contract_docs.rs`.
- R5. README: the `jsonc` block under "Wire it up" is replaced by one
  sentence and a link to the guide; the suite table gains the new row. No
  other README claim changes.
- R6. `Cargo.toml`, `Cargo.lock`, `deny.toml`, `evals/Cargo.lock`,
  `.github/workflows/*`, `src/**` and the pinned bundle are byte-unchanged.

## Acceptance Criteria

- AC-1 Oracle baseline (seat-run evidence, not docs proof). Every Straight-
  tier task in `evals/src/tasks.rs` passes `swarmail-evals trial --agent
  oracle` offline, exit 0; `--agent null` fails each. The report says this
  proves the tasks on this tree and nothing about the docs.
- AC-2 Wired from the docs. For each of the five wiring fences, the suite
  writes the bytes verbatim into a fresh temporary project, reads them back,
  asserts them against the parsed CLI defaults, substitutes exactly one
  token (HTTP) or supplies `SWARMAIL_URL` by `Command::env` (stdio), and
  completes the R4 round trip with the seeded code extracted. Red control: a
  snippet with a changed port, host, path, command or argument is refused
  by the default-equality check.
- AC-3 One real model — open, not measured. No paid trial is run. The
  report records why the shipped wrappers cannot measure docs wiring and
  that the criterion is not passed.
- AC-4 Snippet coverage, fail closed. Every fence in both guide files is
  executed, is the executed start command, or carries a valid marker on the
  line above. Red controls: unmarked `text` fence; marker with empty
  reason; marker followed by a blank line; marker on a `json` fence; `~~~`
  fence; indented fence; unterminated fence; unknown info string; CR byte.
- AC-5 Tool-name drift. Every `swarmail_[a-z_]+` token in both guide files
  is a name in the live `tools/list`. Red control: a misspelt name is
  refused.
- AC-6 Loop block. The `markdown` fence names the four steps in order and
  each of the six classes; each named class is a snake_case member of
  `FailureClass` read from `evals/src/state.rs`; no other class token
  outside the enum appears. The SKILL.md body equals the fence byte for
  byte. Red controls: a removed step, a removed class, an invented class.
- AC-7 Guarantee text. Whitespace-normalized pins for the 0001 sentence of
  ruling 9, the 0002 wait-registers-first sentence, the 0005 ingest
  sentence and the clear-before-act rule, each with a red control that
  removes it.
- AC-8 Client acceptance. The guide and the report state that CI proves
  parse and transport only; Cursor, the Codex file path, the skill path and
  frontmatter are labelled unverified in the guide text, and the test pins
  the presence of those labels.
- AC-9 Gates. Every gate in the intake framing's verify list is green and
  locked, including the evals crate's fmt, clippy and offline tests (the
  tracked-link guard reads the new Markdown). Coverage stays 100% exact.
  R6 holds.
