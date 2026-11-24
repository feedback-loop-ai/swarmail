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

## Inspector CLI runner and shared checker — 2026-11-24

Part of #392 (epic #164). Run `issue-392-record-the-mcp-client--c566e8ab`,
implement seat. The sections above stay as recorded; their "Not built" item
is built here under proposed [0138](../../docs/decisions/0138-inspector-cli-evidence.md).

### What landed

- **One checker.** The receipt types and checks moved to
  `evals/src/client_matrix/receipt.rs`. The root `agent_setup_docs` target
  compiles that same file by path for the checked-in directory, the
  handbook's example and the docs checks. The mutation cases moved to
  `evals/src/client_matrix/receipt_tests.rs`, plus one mistyping case per
  required field, step, status and conditional reason, and a typed-draft
  round trip. The checker's rules are unchanged.
- **The runner.** `swarmail-evals client-matrix run` and
  `client-matrix validate` (`evals/src/client_matrix/mod.rs`). The runner
  starts the given binary memory-only on ephemeral loopback ports through
  the new `runner::start_memory_swarmail`, which shares `launch_swarmail`
  with the trial's start. It delivers the fixture once with
  `mail::send_stored`, unauthenticated, and keys every step on the stored ID
  of the final DATA reply. It runs the client argv five times. Each launch
  is behind its own `capture::observe` forwarder: the trial forwarder plus a
  bounded per-launch exchange log (64 exchanges, 8 MiB). Each launch is
  bounded by `runner::run_captured`, which is now shared with the git
  helper. That function reaps the process group before reading stdout, so
  a descendant holding the pipe cannot stall a launch. The draft is
  redacted (host name, home, user, run paths), validated against its final
  name and installed through `artifacts::publish` with no clobber. The exit
  status is 0 green, 1 red draft, 2 error with no draft.
- **The judge.** `evals/src/client_matrix/evidence.rs` is pure. It pairs
  requests and replies by id within one exchange, refuses an id reused
  within a launch, requires exactly one handshake per launch at the first
  launch's revision, and requires exactly the expected request with the
  checklist's arguments. It checks the HTTP status, the JSON-RPC envelope
  and `isError`. It applies each step's predicate to Swarmail's reply and
  again to the result the client printed.
- **The fixture** is `receipt::FIXTURE`. A root docs test checks that the
  handbook's fenced message equals it. The evals identity proofs rebuild
  evals from a copy of its own tree, so the runner cannot read the handbook
  at compile time.
- **Proof.** `evals/tests/client_matrix.rs` (Linux; the non-Linux ignore
  attribute is on the test) drives the public command with
  `inspector-cli-standin` (`evals/tests/standin/inspector_cli.rs`, a new
  test-only `[[bin]]`). The stand-in takes the Inspector's CLI grammar and
  speaks real MCP to `SWARMAIL_BIN`. The test covers these cases:
  - a six-pass draft with the measured digest and the server's own
    `initialize` reply;
  - a refused second run that keeps the existing draft byte for byte;
  - a red draft for an omitted search call after a clean exit, with the
    other five steps passing and a lingering `sleep` descendant reaped;
  - March negotiation failing `structured_content` alone;
  - oversized output;
  - a spent budget and an unusable output path, each an error with no
    draft;
  - no matrix server left alive at the end.
- **Docs.** `docs/agents/client-matrix.md` has the headless Inspector route:
  the invocation, the five appended argument sets, what a pass requires and
  the narrower evidence under 0138. The UI route is unchanged. The receipt
  README, `docs/guide/mcp.md`, the negotiation implementation record,
  `docs/evals/README.md` and a new `evals/README.md` section carry the dated
  2026-11-24 status. Every receipt is still owed. `docs/decisions/README.md`
  gained 0138's missing index row (`store_contract_docs` refused the design
  commit without it). Decisions 0079 and 0080 are not edited.

### Not claimed

No receipt is committed: `docs/evals/clients/` holds only its README. No
client is claimed green. The runner has run only against the stand-in. Its
printed-result check assumes the Inspector CLI prints the method's result as
JSON. A real Inspector that prints otherwise fails with "the client printed
no JSON result". The dispatcher should confirm this on the host's first
real Inspector run before recording a receipt. Claude Code and Codex stay on
the approval-gated routes, and Claude Desktop needs a person.

### Test cost (host measurements, warm build, load average 6–10)

- `cargo test --locked --test client_matrix` in `evals/`: 3.39–3.40 s test
  time over three runs (7.6–7.8 s wall including cargo's freshness check).
  It runs seven real runner invocations.
- New lib unit tests (`client_matrix::*` and the two observer tests): 0.02 s.
- Root `agent_setup_docs client_matrix`: 7 tests, 0.03 s. The 13 mutation
  cases it held moved to evals, where they cost under 0.01 s.
- Added CI test time: about 3.5 s. Build cost, measured separately: about
  4.0 s to rebuild the new test target and 3.4 s for the stand-in binary,
  each after a touch.

### Held paths

None touched: no change under `scripts/`, `.github/`, `.brokkr/`, either
lock, `deny.toml` or the toolchain. `evals/Cargo.toml` gained a test-only
`[[bin]]` target and no dependency. No production code under the root
`src/` changed.

### Local gates — 2026-11-24, on 5fc1e927

All were run in the foreground on the shared host.

- `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets
  --locked -- -D warnings`: clean.
- `cargo test --workspace --locked`: no failures (4 m 7 s). The first run
  found 0138's missing index row, which is fixed above.
- `cargo build --locked --bin swarmail`: built.
- The evals fmt and clippy: clean. The evals `cargo test --locked
  --no-fail-fast` against that binary: no failures (24 m 33 s).
- `bash scripts/coverage-gate.sh`: pass, exact 100/100/100. Its `selftest`:
  pass.
- `bash scripts/publish-guard-test.sh`, `bash scripts/seal-branch-test.sh`
  and `cargo deny check licenses`: pass.
- `brokkr compile` over `.brokkr/recipes/*/`: **not run**. This sandbox has
  no brokkr binary, and the seat installs nothing. No file under `.brokkr/`
  changed. The verify seat and CI run this gate with the pinned engine.

CI owns macOS (the portable unit tests compile and run there; the
integration test is ignored there), the native-tls absence proof, Helm,
Docker smoke, RustSec and the port-80 bridge exchange.

### Owed by the operator

These four receipts are still owed: Claude Code, Codex, MCP Inspector and
Claude Desktop. The dispatcher may record the Inspector receipt on the host
with this runner after merge. Claude Code and Codex are paid sessions that
need the operator's approval. Desktop needs a person. Epic #164's
compatibility exit criterion is not met.

