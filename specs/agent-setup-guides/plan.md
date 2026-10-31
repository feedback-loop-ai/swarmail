# Plan — agent setup guides

## Technical context

Documentation plus one root integration suite in Rust 2024 using std,
tokio, serde_json and the crate's public `swarmail::config` CLI types —
all already in the locked tree. The suite spawns the shipped binary
(`env!("CARGO_BIN_EXE_swarmail")`) and speaks real HTTP and NDJSON to it,
so it is a decision 0020 rule-3 test: public API, wire shape, no
crate-private seam, outside the coverage denominator by location.
`.specify/templates/plan-template.md` is absent; this plan carries the
requested sections directly.

## Constitution check

0001–0005 are described, never touched. The guide's 0001 sentence is the
verified one (the mirror commit precedes the reply; on disk when a data
file is configured). No `src/` change, so the 100% exact floor is
unaffected. No dependency, lockfile, `deny.toml`, workflow or bundle
change. Commits unsigned (0056). No decision proposed. Evals stays out of
CI (#201).

## Research and resolved choices

- HTTP MCP is non-streaming JSON-RPC (`src/mcp.rs` header): one POST, one
  JSON body, no SSE branch. An `Origin`-less loopback client is allowed.
- Defaults: `SWARMAIL_HTTP_LISTEN` `127.0.0.1:8025`, `SWARMAIL_URL`
  `http://127.0.0.1:8025` (`src/config.rs`); the suite obtains them by
  `Cli::try_parse_from` and never restates them.
- `127.0.0.1:0` on each `SWARMAIL_*_LISTEN` binds an ephemeral port and the
  server prints the bound address (`tests/binary.rs` relies on it).
- `toml` is absent from both lockfiles: subset readers in `tests/`.
- `FailureClass` lives at `evals/src/state.rs:20` with eight variants; the
  six agent-caused ones are the block's vocabulary.
- Snippet sources: `evals/src/wrappers.rs` (Claude `mcp.json`, Codex
  `mcp_servers.*` keys, dsh insert row), README "Wire it up". Cursor, the
  Codex file path, the skill path and frontmatter: unsourced, labelled.
- Wait tools cap `timeout_ms` at 60000; the dsh row's `toolCallTimeoutMs`
  matches it.

## Project structure and touched files

| Path | Change |
|---|---|
| `docs/agents/setup.md` | new: the guide (R1) |
| `docs/agents/swarmail/SKILL.md` | new: the skill (R2) |
| `tests/agent_setup_docs.rs` | new: the suite (R4) |
| `README.md` | "Wire it up" block → sentence + link; suite table row (R5) |
| `docs/tasks/issue-161-agent-setup-guides.md` | commit the intake framing |

Nothing else. `tests/common/mod.rs` may gain a shared helper only if an
existing one is moved there, never copied.

## Data model and contracts

```text
Fence  { info: Info, marker: Option<String>, body: String, line: usize }
Info   = Json | Toml | Yaml | Markdown | Sh | Text
Wiring = Http { url: String } | Stdio { command: String, args: Vec<String> }
```

- `extract(doc: &str) -> Result<Vec<Fence>, Refusal>` — the one scanner.
  Refusals (each a red-control case): CR byte; non-column-0, `~~~` or
  four-plus-backtick fence; unterminated fence; unknown info string;
  marker with empty reason; marker not followed by a fence; marker on an
  executed language; unmarked `text`; unmarked `sh` that is not the start
  command; zero or more than one `markdown` fence.
- `reduce(fence) -> Result<Wiring, Refusal>` — JSON `mcpServers.swarmail`;
  TOML subset `[mcp_servers.swarmail]` + `key = <json>`; YAML subset
  `- <json>` rows with an `insert` array holding the swarmail row. Every
  other shape refuses.
- `assert_defaults(wiring, cli_defaults)` — URL equals the parsed
  `SWARMAIL_URL` default plus `/mcp`; stdio equals `swarmail` + `["mcp"]`.
- `substitute_once(snippet, from, to) -> Result<String, Refusal>` — the
  count must be exactly one.
- `round_trip(transport) -> Result<Receipt, Failure>` — the seven-call loop
  with a per-exchange `tokio::time::timeout` and an outer deadline.
- Prose pins: `doc()`/`states()` as in `tests/listener_contract_docs.rs`.
- Enum read: slice `evals/src/state.rs` between `pub enum FailureClass {`
  and its closing brace, map `CamelCase` to `snake_case`.

## Implementation approach

1. Write the guide and skill to the scanner convention, with the exact
   sentences the pins require, then write the suite against them; iterate
   until the suite is green on the committed bytes.
2. Server lifecycle: one `serve` child per test (`kill_on_drop(true)` plus
   explicit kill-and-reap in a guard that also runs on panic), stderr
   drained on a task into a bounded buffer and printed on failure, stdout
   read line by line until all three addresses are known. Per-test inbox
   and a temporary directory under `std::env::temp_dir()` named from the
   test name, pid and a counter. No `std::env::set_var`.
3. HTTP client: raw `TcpStream` HTTP/1.1 POST with `content-length`, parse
   status line, headers and body; one function. Stdio client: spawn the
   snippet's argv with `SWARMAIL_URL` in `Command::env`, write one NDJSON
   line, read lines until the matching `id`.
4. Red controls: `#[test]` functions over mutated in-memory strings for
   every refusal arm, plus default-equality controls for port, host, path,
   command and argument.
5. README edit last; run the evals tracked-link guard so the new links
   resolve in a clean checkout.

## Quickstart and verification

```bash
cargo test --locked --test agent_setup_docs
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
brokkr compile --bundle .brokkr/recipes/self
bash scripts/coverage-gate.sh && bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh && bash scripts/seal-branch-test.sh
cargo deny check licenses
cargo fmt --manifest-path evals/Cargo.toml -- --check
cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --offline --locked
cargo build --offline --locked --bin swarmail
cargo build --offline --locked --manifest-path evals/Cargo.toml
evals/target/debug/swarmail-evals trial --task signup --agent oracle   # then otp, magic-login; then --agent null
```

## Risks

| Risk | Mitigation |
|---|---|
| Executed bytes differ from shipped bytes (one token or the environment). | Default-equality assertion on the literal before substitution; checked single-occurrence replacement; stated in the suite's doc comment and the guide's "what CI proves". |
| Port stolen between the server's bind and the test's dial. | The server binds `:0` itself and reports the port; there is no pre-bind window. A dial failure is a hard failure naming the port, never a retry loop. |
| Leaked child holds ports or wedges the job. | Guard kills and reaps on drop and on panic; every exchange and every test has a deadline; stderr is printed on failure. |
| Scanner vacuity (a fence silently skipped). | Fail closed on every unrecognized shape; a red control per arm; exactly one `markdown` fence required; the executed-fence count asserted against five wiring snippets plus the start command. |
| Subset readers drift from real TOML/YAML. | They validate snippets we author, say so in the guide, and refuse everything outside the subset; the sourced snippets need nothing more. |
| `FailureClass` reformatted so the text read breaks. | The read fails loudly naming the file; the fix is one line; no cross-crate dependency. |
| Unverified client formats mislead a reader. | Visible "unverified" labels pinned by the test; the report repeats them; AC-8 is the stated boundary. |
| The issue's own criterion (one real model) stays unmet. | Reported as open with the reason (wrappers refuse docs wiring); never recorded as passed. |
| Parallel tests in one binary collide on inboxes or directories. | Per-test inbox and directory names; each test owns its server. |
| macOS CI differences. | std-only process handling, no signals, no unix-only APIs; the binary name is the same on both. |
| New Markdown breaks the evals tracked-link guard. | All links in-repo and relative; the evals suite runs before reporting. |
