# Ordered tasks — agent eval harness

Active amendment, 2026-10-25: accepted [0071](../../docs/decisions/0071-eval-trap-scoring.md)
advances catalogue/snapshot/result to v2 and gives seven named traps trial-wide
precedence. Current implementation/proof tasks live in
[eval-trap-scoring/tasks.md](../eval-trap-scoring/tasks.md). V1 replay uses the
old source/binary; old/mixed artifacts refuse and scores are never migrated or
pooled. Preserve #194/#196 assertions. Decision 0056 now requires unsigned
seat commits followed by operator sealing; run A's T15 signature receipt below
is historical and is not a current signing instruction.

Implemented in run A. Every checkbox names the evidence that closes it,
now under `evals/tests/` (`proof`, `fixture`, `runner`, `scoring`,
`report`, `boundary`); [spec.md](spec.md) is the acceptance authority. The
pinned `brokkr compile --bundle bundles/self` remains the verify seat's.

- [x] **T1 / AC1:** Create the standalone unpublished evals crate and offline
  lock with exact existing dependencies. **Proof:** metadata shows its own
  workspace; a full lock name/version/source subset test passes; root
  package listing excludes evals and restricted-path diffs stay empty.
- [x] **T2 / AC3, AC8:** Define the 16 typed catalogue entries, closed failure
  enum, template validation, state/result schema and pure checks.
  **Proof:** table tests cover every ID/tier/target/idle class, duplicate and
  unknown values, incomplete templates, malformed snapshots and the entire
  scoring precedence including idle and expected reset replay.
- [x] **T3 / AC2:** Implement fixture identity/flow ownership, public and
  bearer-admin listeners, redacted per-flow reads and freeze/snapshot.
  **Proof:** real HTTP tests show read-only idempotence, missing/wrong bearer
  refusal, no solution secrets in responses, no arbitrary success writes,
  identity isolation and consistent frozen state.
- [x] **T4 / AC2, AC7:** Implement bounded SMTP/AUTH and MIME for all eight
  flows, OTP generations/expiry, reset single-use and actual HTTP mutations.
  **Proof:** live Swarmail accepts every flow via DATA 250 and exposes mail
  through MCP; multiline/CRLF/dot-stuffing/encoded subject/HTML/ICS cases
  pass; expired/superseded OTP, bad identity/token and reset replay controls
  refuse. Clock-injected expiry tests use no waiting for wall time.
- [x] **T5 / AC7, AC11:** Implement ICS invitation download/accept and HTTP
  support challenge/reply. **Proof:** generated MIME contains the exact ICS
  served only through the emailed capability; wrong UID/time/challenge and
  empty reply fail, correct actions pass; docs name the #159/#160 limits.
- [x] **T6 / AC7:** Provision all traps with deterministic causal barriers.
  **Proof:** live tests observe stale plus current OTP, three link roles,
  sibling/Unicode decoys, decoded entity destination, exactly 40 digest
  links, first MCP wait timeout before slow release, and real 451 followed
  by fixture retry DATA 250 before agent launch; chaos/isolation mix refuses.
- [x] **T7 / AC9:** Implement root-binary resolution, trial ownership, port
  discovery, protocol readiness, child environment/argv and log draining.
  **Proof:** real child tests check exact argv and identical prompt/env,
  fresh ports/state, ambient configuration isolation, no per-trial Cargo
  build, startup/spawn failure and bounded noisy-child operation.
- [x] **T8 / AC8, AC9:** Implement deadlines, server monitoring, signals,
  process-group cleanup, atomic snapshots/results and one stdout record.
  **Proof:** real subprocess tests exercise a nonterminating external agent,
  descendant surviving its leader, SIGINT/SIGTERM, catchable panic, service
  death, malformed state and cleanup error; verify reaped children/closed
  listeners and exactly one honest result/status on each handled path.
- [x] **T9 / AC4:** Implement per-task oracle and null child subcommands with
  current MCP initialize/text-result parsing and app HTTP only.
  **Proof:** `cargo test --locked --test proof` from evals runs all 16 oracle
  cases through the real CLI, all pass, with no mailbox REST/seed/private
  state path; protocol errors cannot masquerade as successful tool data.
- [x] **T10 / AC5, AC8:** Establish the null and deliberate wrong-action
  controls before trusting measurements. **Proof:** the same `proof` target
  runs all 16 null cases, all fail with their declared idle class; real
  wrong-message/stale-code/wrong-link/tool-misuse children produce their
  classes, deadline produces budget, and harness faults produce harness_error.
- [x] **T11 / AC6 (updated 2026-10-25 under #199):** Preserve the three-child
  inbox task and foreign-token failure. Replace the historical cooperative
  overlap proof with an idle-victim/refused-competitor control and subsequent
  oracle success; all raw child statuses remain honest. Historical overlap
  receipts are not isolation evidence; baseline trials run sequentially.
- [x] **T12 / AC9, AC10:** Add reproducible `check`, matrix and report CLI.
  **Proof:** retained snapshot yields the same business verdict, invalid
  state refuses, matrix emits product cardinality despite trial errors,
  report math/quantiles/counts match fixed fixtures, and empty/all-error/
  malformed input cases obey the schema and exit rules.
- [x] **T13 / AC11:** Document commands, routes, task goals, agent env/argv,
  all result fields, SWARMAIL_BIN discovery, scope limits and interim proof
  ownership. **Proof:** execute README oracle/null/matrix/report/check
  examples and compare them to CLI help/catalogue; never run a paid model
  as part of this batch.
- [x] **T14 / AC1, AC12:** Run evals fmt/clippy/full tests and dependency/
  license/native-tls checks; run unchanged root gates in the specified order
  with write-format before each check, exact coverage and its selftest,
  publish guard and licenses. **Proof:** preserve command/exit receipts;
  implement runs evals and reviewers reproduce it; verify owns pinned
  bundle compilation. A missing or failed gate remains explicitly outstanding.
- [x] **T15 / AC12:** Review scope and sign implementation on the existing
  branch, citing #148, #149, #150. **Proof:** restricted-path diff is empty,
  dependency allowlist unchanged, commit signature verifies, tree clean;
  no push, PR, merge or issue closure. Record any Swarmail defect with its
  reproduction rather than working around it or claiming completion.
