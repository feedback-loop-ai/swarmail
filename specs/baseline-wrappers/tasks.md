# Baseline wrappers tasks

Ordered implementation tasks; unchecked items are not implementation receipts.
Each item names its proof. All mutations stay under evals and its docs after
this design commit. Missing contract evidence is explicit, not a passing test.

- [ ] T1 — Document available CLI versions, literal invocation, MCP registration,
  extra-source refusal and selective auth contracts in evals/wrappers.md. Name
  unresolved dsh MCP, Codex ambient sources and auth layouts exactly as in the
  spec. Do not implement unsupported flags or count refusing stubs as usable.
  Proof: source-to-configuration review table plus offline parser/config probes
  for each evidenced field; unknown versions/configurations refuse with zero
  provider launches. AC1/2 remain outstanding until their required evidence exists.
- [ ] T2 — Add the exact six-entry agents-baseline.json and shared BaselinePlan
  derived from tasks::catalogue(), with 1/5 repetitions, concurrency 1 and
  600000 ms. Resolve the eval executable and private locator without publishing
  private argv. Proof: parse_agents exact-set/mapping tests and plan order/count
  tests for 32 control rows and 480 paid slots (AC1, AC5, AC8).
- [ ] T3 — Implement shared wrapper input/private-config handling and the three
  explicit subcommands using only T1's substantiated CLI interfaces. Preserve
  goal bytes and selected route credentials. Proof: tests/wrappers.rs executable
  probes inspect complete argv/env/generated config; table-driven missing EVAL,
  malformed endpoint/header/deadline, goal-file mismatch, wrong model, missing/
  unsafe config and unsupported-version cases launch no CLI (AC1, AC2).
- [ ] T4 — Add the minimal stdio fixture HTTP helper and register it beside
  Swarmail MCP in each supported CLI configuration. Proof: real loopback
  initialize/tools/list/tools/call tests see one tool, fixed identity on GET /
  and GET/POST actions, bounded bodies/time, no proxies/redirect request, and
  safe rejection of invalid methods/paths/headers. Preserve fixture trap data
  and confirm helper child has no provider credentials (AC2, AC4).
- [ ] T5 — Prove private state/output/lifecycle for the wrappers and helper.
  Proof: canaries on success/setup failure/noisy child failure/interruption stay
  out of argv/public declarations/diagnostics/rows/reports/retained artifacts;
  descriptor/private-mode/collision assertions precede payload; acknowledged
  deadline/signal/cleanup probes observe descendants drained/reaped and owned
  private state removed. Preserve existing runner cleanup tests (AC3, AC5).
- [ ] T6 — Factor only the matrix sink and cancellation context needed by the
  in-process baseline coordinator. Keep early errors, full refusal suffixes and
  MatrixEvidence ownership. Proof: existing runner/report/provenance/matrix
  tests plus real signal-at-transition and active-cleanup handshake tests show
  no paid admission after cancellation and no premature return (AC5, AC7).
- [ ] T7 — Implement fresh all-task oracle/null controls and exact admission.
  Proof: tests/baseline.rs corruption table covers missing/duplicate/extra/
  malformed/foreign rows, wrong outcomes/classes/declarations/limits, infra
  rows, bad exits and interruption with zero paid launches; complete expected
  exit-1 controls admit. Run the real tests/proof.rs suite (AC6, AC9).
- [ ] T8 — Implement paid execution, exclusive streamed retention, accounting
  and reporting with 2 > 1 > 0 status precedence. Proof: deterministic test-only
  row/worker probes exercise the entire 480-slot plan without a provider,
  verify controls -> paid -> report order, preserve failures/errors/refused
  suffixes byte-for-byte, and report after exit 1/2. Missing coordinates,
  malformed final lines, collision/write/report failures keep original evidence
  and exit 2. Existing strict reader/report assertions remain unchanged (AC7).
- [ ] T9 — Implement dry-run from the same plan, update README/run-plan setup,
  commands/deadline/status/ownership, and repair eval-owned authority links using
  tracked evidence. Proof: actual baseline --dry-run prints complete ordered
  plan and paths with no config reads/output creation/process launches/services;
  docs/CLI examples agree and boundary link assertions are not weakened. Name
  any remaining outside-evals broken link and #243 owner (AC8, AC10).
- [ ] T10 — Run standalone eval fmt, locked clippy, full tests and direct proof;
  run every local house gate with the digest-verified candidate-local engine,
  exact coverage/selftest, publish/seal proofs, license/native-tls absence and
  pinned design/delivery selftests. Proof: per-command current-candidate logs
  and exit statuses, explicit failures/ownership and no paid/networked agents.
  Root green is not eval green. Commit authorized implementation unsigned;
  never push or claim feature completion while T1 or a required proof remains
  outstanding (AC9, AC10).
