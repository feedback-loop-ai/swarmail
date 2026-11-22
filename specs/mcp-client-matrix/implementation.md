# Implementation ledger — MCP client matrix

Part of #392 (epic #164). Date: 2026-11-22. Run
`issue-392-record-the-mcp-client--a601d5d7`, implement seat.

## What landed

- [`docs/agents/client-matrix.md`](../../docs/agents/client-matrix.md): the
  shared fixture (a synthetic multipart message, delivered over loopback SMTP
  to `default`, its stored ID taken from the final DATA reply), the six steps
  with the arguments and expected values, private HTTP and stdio capture, and
  routes for Claude Code, Codex, MCP Inspector and Claude Desktop. Each route
  links to `docs/agents/setup.md` §§ 1–3 for wiring instead of restating it.
  Claude Code (`claude -p`) and Codex (`codex exec`) are marked headless-capable
  on a host where the operator has signed the CLI in. Those flags are marked
  unverified. The Inspector and Desktop routes need a person.
- The receipt format (12 fields, six step outcomes) in the same page, and
  [`docs/evals/clients/README.md`](../../docs/evals/clients/README.md) with the
  naming rule. No receipt is checked in.
- One typed reader, `tests/agent_setup_docs/client_matrix.rs`, a module of the
  existing `agent_setup_docs` target. It decodes the bytes into strict serde
  types (`deny_unknown_fields`, required nullable keys, duplicate keys refused
  by the derived decoder) and also refuses the array form serde would otherwise
  accept for a struct. It checks the date and basename, the hex lengths, the
  enums, blank strings, reason-on-pass and fail-without-reason, the JSON-RPC
  shape of the captured reply, the agreement between the recorded revision and
  `result.protocolVersion`, and that no step passes after a failed
  `initialize`. It prints `no receipts yet` for an empty directory and never
  states a compatibility verdict.
- Links and the dated 2026-11-22 zero-receipt status in `docs/agents/setup.md`,
  `docs/guide/mcp.md`, `specs/mcp-version-negotiation/implementation.md`
  (appended; earlier sections unchanged) and `docs/evals/README.md`.

The fixture's expected values (stored ID echoed by every call, code `482913`,
`matrix.txt` / `text/plain` / 13 bytes, `structuredContent.emails` on
2025-11-25) were checked once by hand against this branch's debug binary on
ephemeral loopback ports with `curl`. That run is not a client receipt.

## Not built

The headless recorder (task item 4). The design council chose manual
collection: a recorder needs an authorized client, handshake capture and
process cleanup, and a fake binary would prove only the launcher. No
`swarmail-evals` subcommand was added.

## Test cost

13 new tests, all file-only (temporary directories, no subprocess, socket or
wait). `cargo test --locked --test agent_setup_docs client_matrix` on a warm
build: test time 0.02 s, 0.24–0.29 s wall over three runs. Added CI time is
under one second plus the module's compile time inside the existing target.

## Owed by the operator

Four receipts, one each for Claude Code, Codex, MCP Inspector and Claude
Desktop, recorded through the client on a fresh main build per
`docs/agents/client-matrix.md`. Until then epic #164's compatibility exit
criterion is not met, and nothing here claims it.

## Held paths

None touched: no change under `scripts/`, `.github/`, `.brokkr/`, either lock,
`deny.toml` or the toolchain. No production code under `src/` changed.

## Local gates — 2026-11-22, on 13c30b61

Run in the foreground, on a shared host at load average 20–25:

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean.
- `cargo test --workspace --locked`: no failures (4 m 20 s).
- `cargo build --locked --bin swarmail`: built. The evals fmt and clippy were clean, and the evals `cargo test --locked` against that binary had no failures (24 m under load).
- `bash scripts/coverage-gate.sh`: pass, exact 100/100/100. Its `selftest`: pass.
- `bash scripts/publish-guard-test.sh`, `bash scripts/seal-branch-test.sh`, `cargo deny check licenses`: pass.
- `brokkr compile` over `.brokkr/recipes/*/`: **not run**. This sandbox has no brokkr binary, and the seat installs nothing. No file under `.brokkr/` changed, so the recipes are main's bytes. The verify seat and CI run this gate with the pinned engine.

CI owns macOS, the native-tls absence proof, Helm, Docker smoke, RustSec and the port-80 bridge exchange.
