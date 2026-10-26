# Issue #151 part 1 — trusted wrappers and baseline command (intake)

Run: epic-164-issue-151-part-1-the-ag-df845d53
Phase: intake; framing only, no implementation or model execution.

## Goal

Deliver three trusted CLI wrappers, the approved six-model agent declarations,
and one control-gated baseline command under evals/ so the operator can run
and retain the approved sequential baseline.

## Authority and evidence

The issue-151 evidence packet and operator rulings both passed their supplied
SHA256SUMS: README.md be74941231e61014f3f4caa0803713901a876d28a91f514b2e29febde76d5df9;
operator-rulings.md 03a4460d368cc77d7c2c927cd40aff6bab7568fbee5a5ca5b8369f5643c54534.
The packet records no prior issue-151 outcomes. The 2026-10-26 ruling approves
all current v2 tasks, the six exact models below, five paid trials per cell,
concurrency 1, and free controls before paid work. Delivery seats must never
run a paid/networked agent. This part does not itself satisfy issue #151's
operator-owned baseline publication and epic reprioritization criteria.

Read authorities: docs/house-rules.md, decision index, accepted 0055 and 0071;
compatible existing behavior described by proposed 0063, 0073, 0075 and 0076.
No proposal is accepted or accepted contract amended here. Preserve current
bundle routing and protected review. The pinned candidate engine has SHA-256
ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f.

Spot-checked owners: evals/src/runner.rs RuntimeDirs::apply (env_clear),
resolve_agent_command, validate_request, run_trial and existing cleanup owner;
evals/src/main.rs Matrix, assigned_cell, matrix_with_options and run_child_assigned;
evals/src/report.rs AgentSpec, parse_agents and report; evals/src/tasks.rs
catalogue, idle_class, DEFAULT_DEADLINE_MS (60000), MAX_DEADLINE_MS (3600000).
The literal argv/eight-input contract is in evals/README.md; the current
procedure in evals/run-plan.md covers only signup/otp and needs full coverage.
evals/tests/proof.rs already proves all 16 oracle/null cases and sequential
admission; wrapper and orchestration regressions are the new proof surface.
Relevant history: 422dd9b (#199 sequential trials), c13a507 (#200 provenance),
aec4b46 (#198 traps), f2efbb7 (#225 cleanup probe). Intake base: f49d122.

## Scope and frozen guarantees

Implementation changes only evals/ and documentation beneath it, including
new wrapper/config/command files and offline tests as needed. Root production,
root manifests/locks, gates, allowlists, bundle routing, scoring and protocol
semantics remain outside scope. 0001 accept==stored, 0002 subscribe-before-check
and 0003 oldest-only pruning are adjacent only through the existing real-server
proofs; this work changes none of them. 0004 real SMTP/MCP/HTTP proof and 0005
ingest extraction also remain intact. No semantic decision is missing for
this compatible wrapper/orchestration work. CLI syntax/configuration choices
belong to design and must be grounded in locally available evidence; identify
any missing CLI contract precisely rather than ship guessed invocations.

## Acceptance criteria

1. Exactly one trusted wrapper for each of Claude Code, Codex and dsh is
   provided under evals/. Each accepts an explicit literal model argument,
   passes EVAL_GOAL byte-for-byte to the CLI, configures its MCP client for
   EVAL_MCP_URL, and exposes exactly one fixture HTTP tool for EVAL_APP_URL.
   Real loopback tool tests assert X-Eval-Identity on every fixture request.
   CLI/config inspection asserts the intended MCP endpoint and HTTP tool,
   with no inherited client configuration adding extra tool access. Preserve
   the documented unconfined execution limit; do not claim sandboxing.
2. Wrappers work from the harness's cleared environment and fresh private
   HOME, TMPDIR and XDG directories with the existing eight EVAL_* inputs.
   They preserve literal argv/model/goal values (including spaces and shell
   metacharacters), do not evaluate a shell command string, and do not rely
   on inherited caller credentials, HOME, proxy or startup configuration.
   Missing/invalid required inputs and unsupported models refuse before a
   CLI is launched. Probe tests assert actual argv and the complete child
   environment/configuration, including rejection cases.
3. Credentials enter only through an explicitly selected wrapper's existing
   private configuration. No credential value enters argv, tracked files,
   agent labels/model/config declarations, reports or retained evidence.
   Offline tests use synthetic canaries in private configuration and emitted
   child errors and assert their absence from those public sinks, for success
   and refusal paths. Private per-trial CLI files/log handling is documented
   and tested; setup/failure diagnostics do not echo private config or secrets.
4. Adopt a 600000 ms (10 minute) per-trial agent deadline for this baseline,
   explicitly passed by the command and documented in run-plan.md. This gives
   real models room for startup and several MCP/HTTP turns beyond the current
   60 second fixture default; it is an operational choice, not measured model
   performance. It is within the existing one-hour maximum. Tests assert the
   effective deadline reaches every wrapper and expiry cannot leave an agent
   running beyond the harness's existing owned cleanup lifecycle. Preserve
   the existing cleanup owner and budgets; do not add a shorter competing
   parent kill timer. Document that sequential total runtime may be long.
5. A parseable agents file declares exactly these six models with distinct
   credential-free labels, literal wrapper argv/model arguments and explicit
   model provenance: Claude claude-opus-5-5 and claude-sonnet-5-5;
   Codex gpt-6.1-sol and gpt-6-luna; dsh dashscope/glm-5.3 and
   spark-glm/GLM-5.3-Flash-EXL3. No model substitution or automatic fallback.
   Offline tests check the exact set and wrapper mappings through parse_agents.
6. One documented baseline command first runs the free oracle/null matrix over
   every v2 catalogue task (currently 16 tasks, one control repetition each).
   It validates a complete, unique set of control rows: every oracle passes,
   every null fails with that task's declared idle_class, and none is an
   infrastructure error. Expected null failures give matrix exit 1; this alone
   must neither abort valid controls nor authorize paid continuation. Missing,
   duplicate, malformed or extra rows, wrong classes/outcomes, and an unexpected
   command exit all refuse before any paid agent is invoked. Fault-injection
   tests establish each refusal and a zero paid-launch count.
7. Only after valid controls, that command runs every current v2 task against
   all six models, five trials per cell, concurrency 1, using the selected
   deadline, then generates the report. Currently this means 32 control rows
   and 480 requested paid rows (16 x 6 x 5), including the multi-agent isolation
   task; slots are not a claim about agent session count. Preserve every emitted
   row, including failures, harness errors and unstarted refusals. No retries,
   filtering, overwrite or conversion of infrastructure errors to successes.
   Tests prove the order controls -> validation -> paid matrix -> report and
   that a paid matrix nonzero status still preserves its rows and permits a
   diagnostic report where input is valid, while the overall command fails
   for infrastructure/refusal errors. Ordinary measured business failures
   remain visible and are not represented as a control failure.
8. A dry-run mode prints the complete ordered plan, exact model/task/trial
   cardinalities, concurrency, deadline, output/retention locations and report
   invocation without launching wrappers, trials or contacting services.
   Offline execution probes assert zero launches and exact plan contents.
   The operator instructions name prerequisite CLI/private-config setup,
   the single baseline command and dry-run form, output retention and status
   interpretation, and explicitly assign the paid execution to the operator.
9. Existing v2 trap scoring, provenance, private artifact publication, literal
   argv, complete row accounting and sequential-admission tests remain green.
   Tests exercising real harness acceptance use real local SMTP/MCP/HTTP;
   offline probe executables test CLI assembly and secret handling without
   substituting for those protocol proofs. No paid or external-network agent
   is run during delivery. No tokens, cost or transcripts are fabricated.

## Verification commands and ownership

Use CARGO_NET_OFFLINE=true. Run the entire house inventory against the committed
candidate with the verified candidate-local engine. Verify shim digests first;
execute the pinned bundle scripts rather than treating public dispatch shims
as proof owners (0050). Record each command and outcome; failures are not waived.

```bash
sha256sum .forge/bin/brokkr
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
```

The coverage gate must prove exact zero missed lines, branches and functions;
its selftest is mandatory. Also run the fail-closed native-tls absence command
from bundles/self/scripts/verify-seat.sh: only cargo tree's explicit
"did not match any packages" diagnostic proves absence, never a generic error.
Root gates do not exercise the standalone eval crate, so run independently:

```bash
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --locked
cargo test --manifest-path evals/Cargo.toml --locked --test proof
```

The full eval suite must include the new wrapper, canary, controls/refusal and
dry-run tests; design must choose and document their exact command if any
non-Cargo test entrypoint is needed. The actual new baseline dry-run command
must run as a separate recorded proof after implementation. No benchmark is
needed: this phase delivers execution setup, not performance measurements.

CI owns the charter's macOS root tests, chart, Docker, RustSec/network and
port-80 proofs; #201 owns macOS eval execution. #221 owns namespace isolation.
The operator owns live CLI/provider interoperability, paid baseline execution,
publication of baseline-<date>.md with success/token/taxonomy evidence, epic
reprioritization, live settings, sealing and decisions. #152/#153 PRs remain
unmerged until the baseline exists; #154-#160 wait for reprioritization.
These ownership facts do not become unprovable local acceptance criteria.
No remote reads, journal reads, sibling-worktree reads, push, merge, tag,
publication, issue closure or decision acceptance is authorized.

## Intake outcome

Resolved framing: the requested compatible Phase 0 delivery is actionable.
This document is not an implementation, a paid baseline or a verification
verdict. Remaining feature implementation and its proofs belong to later seats.

## Observed intake verification (2026-10-26)

All house gates above passed offline, including exact 100/100/100 coverage,
coverage selftest, pinned compile (0 unruled policy valuations), publish guard,
keyless seal, licenses and fail-closed native-tls absence. Engine/shim checks,
design selftest and delivery tamper proof also passed. Standalone eval fmt,
locked clippy and the direct tests/proof.rs oracle/null/sequential control
suite passed.

The full standalone eval test command exited 101 at
boundary::sequential_contracts_and_decision_links_are_honest (tests/boundary.rs:85).
Its exact failure is evals/README.md's broken relative link to
../.forge/epic192/issue199/operator-rulings.md. That target is absent from this
workspace, and the same link exists at the unmodified intake base. The full
run passed 49 library and 18 binary tests before stopping in boundary.rs;
subsequent integration suites were not reached by that invocation. The direct
proof suite above ran separately afterward and passed. This is not a green
full-eval receipt and must not be represented as one.

The operator's #243 ruling assigns tracked authority links a tracked home;
this intake does not invent the missing packet, weaken the link test or
implement that separate documentation delivery. Design/implementation must
carry the observed failure forward and resolve its ownership before claiming
whole-feature verification. Detailed command outcomes and logs are retained
as ignored run-local evidence under .forge/intake-proof/.
