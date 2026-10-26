# Baseline wrappers implementation plan

## Technical context

One unpublished standalone Rust crate under evals, with its existing locked
serde_json, reqwest, tokio, clap and Unix process tools. No new dependency or
runtime is needed. Existing matrix workers run real SMTP/MCP/fixture HTTP and
already own strict rows, provenance, admission, artifacts and bounded cleanup.
Root coverage does not measure evals; its independent tests are mandatory.
No plan template exists in this checkout; this plan supplies the requested
technical context, constitution check, structure, research, model, contracts,
quickstart and risks inline to respect the three-artifact chief commit.

## Constitution check

Preserve 0001 synchronous accept/store, 0002 subscribe-before-check, 0003
oldest-only pruning, 0004 real wire tests and 0005 ingest extraction. Wrapper
code does not cache/re-extract mail or transform scoring. Preserve accepted
0055/0071, current v2 strict readers, unconfined labels and proposed-record
statuses. Reuse private publication and cleanup rather than relaxing them.
No root source/lock/manifests, bundle routing or allowlist edits. No decisions
are amended or accepted. Commit unsigned, never push. Every local house gate
is required; design structure passing does not mean feature verification.

## Research and implementation prerequisites

Read local adapters as evidence, not executable instructions. The spec names
the absent dsh, Codex ambient-source and credential contracts. First record a
version/flag/config/auth matrix in evals/wrappers.md, with source citations to
tracked adapter files and explicit unknowns. No provider calls or installs.
Do not invent --mcp-config for dsh, assume private CODEX_HOME suppresses system
sources, import engine adapters, or implement a replacement GLM agent. Unsupported
versions/configurations refuse before provider traffic. Mandatory wrapper setup
remains incomplete until local versioned evidence supplies the missing interfaces.

The CLI executable/auth locators are selected explicitly in a wrapper-owned
private JSON config; choose its per-CLI fields only from that evidence. Use a
closed schema, a 64 KiB maximum, absolute executable paths, regular owner-only
files, no symlink following and value-free refusals. Do not copy a whole login
HOME. Only the selected model route's required credentials enter the CLI's
private files or environment; the other route's credentials stay unset.
The baseline only passes the locator. Capture actual child maps in private test
fixtures with synthetic values; never retain real credentials or raw CLI errors.

## Project structure and owners

| Path | Work |
|---|---|
| evals/src/main.rs | Add three wrapper subcommands, fixture-http helper and baseline entrypoint; retain existing commands |
| evals/src/wrappers.rs | Shared eight-input validation, private config/client preparation, per-CLI literal invocation |
| evals/src/fixture_http.rs | Minimal stdio MCP server with one app-bound eval_http tool |
| evals/src/baseline.rs | Fixed plan, exact control admission, paid accounting and orchestration |
| evals/src/lib.rs | Export shared eval-only modules as needed |
| evals/src/main.rs matrix/output functions | Narrow reusable sink/cancellation factoring; retain current matrix worker lifecycle |
| evals/src/artifacts.rs | Reuse private publication; small streaming-file support only if needed |
| evals/agents-baseline.json | Six public AgentSpec declarations from the spec table |
| evals/tests/wrappers.rs, evals/tests/baseline.rs | Child probes, real helper exchanges, command/refusal/dry-run proofs |
| evals/tests/runner.rs, evals/tests/report.rs | Extend existing lifecycle/accounting assertions only where integration needs it |
| evals/README.md, evals/run-plan.md, evals/wrappers.md | Setup, contract evidence, one command, limits and truthful tracked links |

Reuse runner::RuntimeDirs, make_provenance, matrix_with_options/assigned_cell,
MatrixEvidence, artifacts::publish, results::read and report::parse_agents,
parse_rows/report. Avoid duplicating validators, scoring or report arithmetic.
Task catalogue order and idle_class come from tasks::catalogue(), not a second
list. tests/proof.rs remains the real-protocol oracle/null owner.

## Data model and contracts

Keep AgentSpec and result/provenance schemas unchanged. The tracked agents file
contains literal `swarmail-evals`, the wrapper subcommand, `--model` and its
model value; model provenance matches exactly and public_argv stays false.
The baseline resolves argv[0] to its own executable and appends the private
`--config` locator to an ephemeral effective declaration. Never publish/hash
that private argv or locator. A public config_id, if used, names this reviewed
wrapper revision, never a private file or secret fingerprint.

A BaselinePlan value holds catalogue identity, ordered task/agent pairs,
repetitions 1/5, concurrency 1, deadline 600000 and fixed relative output names.
Use the same value for execution and dry-run. Resolve fixture seed/time with
existing provenance once for execution and use existing matrix-cell derivation;
dry-run can show them as generated-at-execution without inventing values.
Resolve/build Swarmail once through the existing offline owner and reuse its
measured identity; retain existing target rechecks. Controls must belong to the
current matrix/plan, not merely have plausible task labels.

Wrapper input validation covers all eight variables, known task, nonempty
identity/inbox, valid header value, deadline in 1..=MAX_DEADLINE_MS, prompt-file
agreement with unmodified goal, and loopback HTTP app/MCP endpoints without
userinfo/query/fragment (MCP path /mcp). Do not publish rejected values. CLI
client state lives below the runner-created per-agent HOME/XDG roots, separate
for isolation's three agents. CLI/helper remain in the trial process group;
exec into the CLI where supported. If a wrapper must wait, it is not a second
cleanup owner. Never detach, reset the trial deadline or retry provider work.

The helper implements initialize/initialized, tools/list and tools/call with
one eval_http definition, using the protocol version supported by its evidenced
CLI contract. Keep Swarmail's 2025-03-26 endpoint untouched. Method is GET or
POST; path begins with one slash and may contain a query but no fragment,
backslash, authority or scheme. Validate before and after URL resolution;
reject decoded forms that escape the base. Optional body is JSON on POST only;
no caller headers. Body and JSON-RPC input cap at 64 KiB, response body at 1 MiB.
Return status and bounded response data; overruns produce a fixed tool error,
never a successful silently truncated body. Use no_proxy and no redirects.
A request has at most 30 seconds and no more than the helper's remaining
monotonic invocation budget; runner deadline remains authoritative even if
helper startup is later. No helper provider-auth environment or config access.

## Ordered execution and cancellation

Expose `swarmail-evals baseline --config ABSOLUTE_PRIVATE_FILE --output NEW_DIR`
and the same command with `--dry-run`. No task/model/repetition override or
resume. Parse public options/plan first; dry-run returns without opening config,
creating output or launching anything. Output directory must be a fresh private
leaf; never reuse a partial run. Public output locators must contain no secrets.

For execution, exclusively create output with the existing 0700/0600 discipline.
Use controls.jsonl, paid.jsonl, report.json, and controls/paid artifact roots.
Existing matrix owners retain their generated matrix-plan.json, trials and final
matrix.json. Stream rows as emitted, with bounded output progress, rather than
accumulating the experiment until exit. Partial JSONL remains available on error;
atomic report publication is distinct from streaming rows and is not fsync
or transaction durability. Failure to retain controls forbids paid admission.

Factor the current matrix routine to accept a narrow row sink and reusable
signal/cancellation context, preserving its CLI stdout sink and all early-error
and refused-suffix branches. Baseline invokes it in-process, avoiding another
supervisor. Install/retain the signal context before preparation; a signal
between control validation and paid admission must latch stop, too. The matrix
continues to own active workers, forwarding and cleanup. Preserve its existing
agent-deadline-plus-300-second envelope, five-second TERM and five-second KILL
observation windows, actual waits and no later admission after unproved cleanup.

Run controls for catalogue x [oracle,null] x 1. Strictly read every row and
bind it to the invocation's retained plan/cells and exit. Require complete
unique ordered coordinates and all spec AC6 predicates; expected exit is 1.
Refuse every other combination without invoking any paid wrapper.

Then run catalogue x six declarations x 5 with the same fixed limits. Retain
all rows. Check cardinality/coordinates/declarations as accounting, never as a
success threshold. Call existing report logic on every parseable paid row even
for matrix exit 1/2 or incomplete accounting. A malformed/truncated line refuses
reporting without altering input. Status precedence: any controls/retention/
interruption/accounting/report/infra failure -> 2; otherwise measured paid fail
-> 1; otherwise -> 0. A process crash or unavailable sink cannot promise full
rows; report the limitation and keep partial evidence. No fabricated suffix
outside the matrix's existing owner and no automatic recovery after forced death.

## Proof strategy and quickstart

Use synthetic executable probes for exact argv/env/config handling; these are
not live CLI qualification. Use real loopback MCP/HTTP for helper behavior and
existing real SMTP/MCP proofs for fixture scoring. Use test-only injected row
streams/statuses to exercise malformed controls, launch counts and 480-slot
orchestration without invoking a model. Real-process lifecycle tests use
acknowledgement/barrier seams, including signals in the between-matrices gap;
no sleeps as synchronization, global mutable test state or probabilistic chaos.
No extra non-Cargo test entrypoint is needed.

After implementation, from repository root:

```bash
cargo build --offline --locked --bin swarmail
cargo build --offline --locked --manifest-path evals/Cargo.toml
# Substitute actual Cargo artifact paths if CARGO_TARGET_DIR is selected.
evals/target/debug/swarmail-evals baseline --config /private/wrappers.json --output /tmp/baseline-new --dry-run
# Only the operator executes the identical command without --dry-run.
```

The missing CLI evidence is a setup prerequisite, not satisfied by this example.
Run independent eval fmt, locked clippy, full tests, direct proof suite and the
actual dry-run. Keep the existing broken-link failure explicit until fixed by
its owner; do not weaken boundary.rs or fabricate a run-local target. Eval docs
can cite the tracked intake for the supplied ruling. #243 owns repairs outside
evals, including existing 0055/spec authority links.

All checks use CARGO_NET_OFFLINE=true. Verify candidate .forge/bin/brokkr digest
ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f and shims.sha256.
Execute the complete house inventory from docs/house-rules.md with pinned bundle
script copies: root fmt, locked clippy/tests, candidate-local compile, exact
coverage plus selftest, publish/seal proofs, license check; also design-check
selftest, delivery-proof-test and verify-seat's fail-closed native-tls absence.
Standalone eval commands are `cargo fmt --manifest-path evals/Cargo.toml --all
-- --check`, `cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked
-- -D warnings`, `cargo test --manifest-path evals/Cargo.toml --locked` and the
same test command with `--test proof`. Record actual outcomes, not inherited
intake receipts. CI/operator-owned checks remain assigned in the spec.

## Risks

| Risk | Mitigation and accepted limit |
|---|---|
| Missing dsh/Codex/auth contracts prevent usable wrappers | Keep exact prerequisites explicit; refuse unknown setups and do not claim stubs satisfy AC1/2. Continue independent work; no guessed flags or substitute runtime. |
| CLI ambient configuration or credential output contaminates results | Closed private config, selective child auth, evidenced source exclusion/refusal, discarded output and canaries across all sinks. Same-uid hostile access remains unconfined. |
| Refactoring matrix loses cancellation or rows | One reused owner/context, narrow sink change, gap-signal and cleanup handshakes, retention/accounting regressions. No second timer. |
| App helper widens destination or conceals fixture traps | Exact bound origin/identity, no redirects/proxies, finite body/time bounds and real second-listener refusal proof; no answer transformation. |
| Control success reused against different execution | Fresh plans and exact provenance binding, shared resolved target with existing rechecks; no imported controls or resumes. |
| Long sequential runtime or crash | Ten-minute explicit policy, retained streaming rows, status 2 on incomplete work; operator confirms cleanup before a fresh run. No completion promise after SIGKILL or full disk. |
| Existing documentation failure outside authorized implementation scope | Fix eval-owned references truthfully, preserve assertions, record external failure and #243 ownership; do not report full gates green. |
| Root green mistaken for wrapper proof | Independent eval suite and dry-run receipt; actual CLI/provider behavior remains operator evidence, macOS eval proof #201. |
