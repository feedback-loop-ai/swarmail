# Ordered tasks — agent eval harness

All tasks remain implementation work. Every checkbox names the evidence
required to close it; [spec.md](spec.md) is the acceptance authority.

- [ ] **T1 / AC1:** Create the standalone unpublished evals crate and offline
  lock with exact existing dependencies. **Proof:** metadata shows its own
  workspace; a full lock name/version/source subset test passes; root
  package listing excludes evals and restricted-path diffs stay empty.
- [ ] **T2 / AC3, AC8:** Define the 16 typed catalogue entries, closed failure
  enum, template validation, state/result schema and pure checks.
  **Proof:** table tests cover every ID/tier/target/idle class, duplicate and
  unknown values, incomplete templates, malformed snapshots and the entire
  scoring precedence including idle and expected reset replay.
- [ ] **T3 / AC2:** Implement fixture identity/flow ownership, public and
  bearer-admin listeners, redacted per-flow reads and freeze/snapshot.
  **Proof:** real HTTP tests show read-only idempotence, missing/wrong bearer
  refusal, no solution secrets in responses, no arbitrary success writes,
  identity isolation and consistent frozen state.
- [ ] **T4 / AC2, AC7:** Implement bounded SMTP/AUTH and MIME for all eight
  flows, OTP generations/expiry, reset single-use and actual HTTP mutations.
  **Proof:** live Swarmail accepts every flow via DATA 250 and exposes mail
  through MCP; multiline/CRLF/dot-stuffing/encoded subject/HTML/ICS cases
  pass; expired/superseded OTP, bad identity/token and reset replay controls
  refuse. Clock-injected expiry tests use no waiting for wall time.
- [ ] **T5 / AC7, AC11:** Implement ICS invitation download/accept and HTTP
  support challenge/reply. **Proof:** generated MIME contains the exact ICS
  served only through the emailed capability; wrong UID/time/challenge and
  empty reply fail, correct actions pass; docs name the #159/#160 limits.
- [ ] **T6 / AC7:** Provision all traps with deterministic causal barriers.
  **Proof:** live tests observe stale plus current OTP, three link roles,
  sibling/Unicode decoys, decoded entity destination, exactly 40 digest
  links, first MCP wait timeout before slow release, and real 451 followed
  by fixture retry DATA 250 before agent launch; chaos/isolation mix refuses.
- [ ] **T7 / AC9:** Implement root-binary resolution, trial ownership, port
  discovery, protocol readiness, child environment/argv and log draining.
  **Proof:** real child tests check exact argv and identical prompt/env,
  fresh ports/state, ambient configuration isolation, no per-trial Cargo
  build, startup/spawn failure and bounded noisy-child operation.
- [ ] **T8 / AC8, AC9:** Implement deadlines, server monitoring, signals,
  process-group cleanup, atomic snapshots/results and one stdout record.
  **Proof:** real subprocess tests exercise a nonterminating external agent,
  descendant surviving its leader, SIGINT/SIGTERM, catchable panic, service
  death, malformed state and cleanup error; verify reaped children/closed
  listeners and exactly one honest result/status on each handled path.
- [ ] **T9 / AC4:** Implement per-task oracle and null child subcommands with
  current MCP initialize/text-result parsing and app HTTP only.
  **Proof:** `cargo test --locked --test proof` from evals runs all 16 oracle
  cases through the real CLI, all pass, with no mailbox REST/seed/private
  state path; protocol errors cannot masquerade as successful tool data.
- [ ] **T10 / AC5, AC8:** Establish the null and deliberate wrong-action
  controls before trusting measurements. **Proof:** the same `proof` target
  runs all 16 null cases, all fail with their declared idle class; real
  wrong-message/stale-code/wrong-link/tool-misuse children produce their
  classes, deadline produces budget, and harness faults produce harness_error.
- [ ] **T11 / AC6:** Add the three-child isolation coordinator and concurrent
  trial proof. **Proof:** two overlapping isolation trials both pass with
  two servers and six agents, each server's three distinct AUTH inboxes;
  foreign-token control fails, and all raw child statuses are preserved.
- [ ] **T12 / AC9, AC10:** Add reproducible `check`, matrix and report CLI.
  **Proof:** retained snapshot yields the same business verdict, invalid
  state refuses, matrix emits product cardinality despite trial errors,
  report math/quantiles/counts match fixed fixtures, and empty/all-error/
  malformed input cases obey the schema and exit rules.
- [ ] **T13 / AC11:** Document commands, routes, task goals, agent env/argv,
  all result fields, SWARMAIL_BIN discovery, scope limits and interim proof
  ownership. **Proof:** execute README oracle/null/matrix/report/check
  examples and compare them to CLI help/catalogue; never run a paid model
  as part of this batch.
- [ ] **T14 / AC1, AC12:** Run evals fmt/clippy/full tests and dependency/
  license/native-tls checks; run unchanged root gates in the specified order
  with write-format before each check, exact coverage and its selftest,
  publish guard and licenses. **Proof:** preserve command/exit receipts;
  implement runs evals and reviewers reproduce it; verify owns pinned
  bundle compilation. A missing or failed gate remains explicitly outstanding.
- [ ] **T15 / AC12:** Review scope and sign implementation on the existing
  branch, citing #148, #149, #150. **Proof:** restricted-path diff is empty,
  dependency allowlist unchanged, commit signature verifies, tree clean;
  no push, PR, merge or issue closure. Record any Swarmail defect with its
  reproduction rather than working around it or claiming completion.
