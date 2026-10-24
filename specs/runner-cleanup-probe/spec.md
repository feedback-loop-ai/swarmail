# Runner cleanup probe synchronization

- **Status:** Draft design, 2026-10-25; implementation and its proofs remain tasks.
- **Feature:** Epic #192, issue #225.
- **Framing:** [intake](framing.md), including its baseline failures and verification receipt.

## Feature and rationale

The inline Linux regression
`runner::test_probes::runner_cleanup_await_pending_and_bypass_controls_cover_signals_and_faults`
in `evals/src/runner.rs` must establish cleanup ordering through explicit,
retained signals from the real cleanup hold and awaited cleanup future.
Machine scheduling must not decide whether entry or Pending was observed.
The reported loaded-machine failure used a deadline to discover marker
files; the same regression also treats 50 ms without exit and a two-second
bypass exit as evidence. All three timing oracles need replacement.

The observer must distinguish a cleanup owner running independently from
the runner actually awaiting that owner. Merely announcing hold entry or
sampling a future that is subsequently discarded would repair the symptom
while allowing the original missing-await defect to pass.

A finite watchdog can still diagnose a hung harness. Scheduling delay can
consume its hang budget; this design does not promise progress under
infinite starvation. Watchdog expiry proves neither correct waiting nor
mutant detection.

## Scope and authority

Executable changes belong only to owner-local `#[cfg(test)]` code and the
existing inline tests in `evals/src/runner.rs`. Preserve real child processes,
runner ownership, process identity checks, release, cessation and direct-child
reaping. Production runner behavior remains byte-identical after test code
is compiled out. No dependencies, manifests, locks, schemas, cleanup budgets,
root production sources, coverage policy or recipe routing change.

This enforces accepted [0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md)
and [0055](../../docs/decisions/0055-agent-eval-harness.md), without changing
a public contract or ruling. No new semantic decision is required.
[0073](../../docs/decisions/0073-matrix-cancellation.md) explains the existing
cleanup fence but remains proposed; this work neither amends nor accepts it.
The frozen guarantees 0001–0003 and ingest extraction 0005 stay untouched.

The supplied `.forge/epic192/issue225/README.md` and `operator-rulings.md`
were verified against their directory's `SHA256SUMS`. Both matched. The
README contains no prior-run entries. The operator's rulings are authority;
macOS evals evidence belongs to #201 CI. Local checks require no GitHub,
journal, sibling worktree, namespace, network or hosted settings access.

## Panel synthesis and rulings

Both run-local positions were read: `.brokkr/state/design/positions/simplicity.md`
and `.brokkr/state/design/positions/robustness.md`. They agree on the core
repair; their emphases are reconciled as follows, without inventing a dispute.

| Question | Ruling and cost |
| --- | --- |
| Scope and mechanism | Adopt simplicity's one-file, private, per-probe handshake using existing Unix IPC facilities and existing `Probe` ownership. Adopt robustness's retained events and explicit endpoint lifetime. Reject a reusable supervisor, protocol framework, event journal or second process registry. The cost is private test protocol code with deliberately limited reuse. |
| Strength of the oracle | Adopt robustness's requirement that Pending comes from the real awaited continuation, plus simplicity's explicit actual-return observation. Reject a hold-entry-only repair, preliminary-Pending-only oracle, larger deadlines and sampled non-exit. The smaller alternative loses: it costs missed-await sensitivity and can turn the required mutant into a hang. The chosen design costs additional causal states and a return observation gate. |
| Completion observation | Adopt simplicity's return acknowledgement gate after actual `run_trial` return, before runtime destruction. Reconcile robustness's prohibition on substitute fences by publishing return before gating and testing the same owned-work completion predicate in positive and bypass paths. This delays test-process exit, so actual runner return and process reaping must be tracked separately. |
| Delay proof | Adopt the acknowledged pre-entry permission gate. Choose robustness's full control matrix in ordinary and delayed modes, including both signal bypass controls. The cheaper signal-only proof loses coverage of fault-specific ordering; the chosen matrix costs additional real-process test runtime. |
| Unrelated timing helpers | Adopt simplicity's bounded scope; retain marker compatibility for existing budget, leader-first and runtime-lifetime probes. Reject wholesale migration of `eventually` or unrelated readiness/teardown tests. The cost is leaving their separate timing patterns outside this repair. |
| Coverage and platforms | Keep the regression inline and every new seam installed by a test, as both positions require. Accepted 0055 keeps evals outside root coverage; root 100% is not evidence of evals coverage. No exclusion, feature flag or coverage-boundary change is authorized. Platform and remote proof stays with its existing CI/operator owner. |

## Acceptance Criteria

1. **Causal synchronization.** The named regression uses an explicit
   cleanup-hold entry signal and the real cleanup future's Pending result
   from its awaited continuation. Hold entry is announced only after a
   retained release latch is armed. Sleep, marker polling, elapsed time,
   absence of an event, and sampled child non-exit do not establish these
   states. A separate watchdog only reports a harness hang and starts rescue.
2. **Preserved controls.** Execute positive SIGTERM, SIGINT, panic and
   server-death cases, plus both signal bypass cases. Positive cases observe
   held cleanup, actual awaited Pending and live recorded owned work, then
   explicit release, runner return, cessation and direct-child reaping;
   `Probe::finish` succeeds without rescue. Bypass cases observe actual return
   with live work, reject the same completion predicate, then release and
   perform checked identity-safe rescue. Keep actual preliminary Pending
   observation in bypass controls as distinct from an awaited continuation.
3. **Deliberate delay.** The same named regression runs the full matrix
   with and without delayed entry. In delayed mode an owner acknowledges a
   pre-entry checkpoint and cannot enter until the parent grants permission.
   After granting permission, the same causal assertions pass. Delay is an
   enforced ordering, with no elapsed minimum, sleep or load generator.
   Record each case/mode and checkpoint/permission evidence.
4. **Mutant sensitivity.** A temporary change that bypasses/removes the
   actual awaited cleanup fence in a positive case makes the exact named
   regression execute one test and exit nonzero at an ordering/completion
   assertion. Keep child creation and owner observability intact. Record
   the precise patch, command, exit and assertion. Compilation failure,
   timeout, EOF alone, unrelated test failure or the expected-bypass cases
   passing is insufficient. Restore the candidate and record the same
   one-test filter passing afterward.
5. **Lifecycle safety.** Events received before a wait are retained;
   independent owners may arrive in any order. A release applies to all
   current and later holders in that probe. EOF, malformed events and
   unexpected return fail visibly. Failure teardown releases/closes every
   new gate before existing bounded, identity-safe rescue. Test endpoints
   are private per probe, installed before triggering cleanup, and do not
   change the parent environment or leak inherited descriptors to descendants.
6. **Test-only boundary.** Keep tests inline without exclusions, with each
   new seam installed and exercised. Review the executable diff and compare
   baseline/candidate canonical non-test runner expansion bytes using the
   same compiler and configuration. Retain method, inputs and equality
   result. An executable hash affected by build metadata or regex-stripped
   source alone is not this proof.
7. **Local verification.** On the committed implementation candidate, the
   standalone evals fmt, locked all-targets clippy (`-D warnings`), full tests
   and exact named filter pass. Run every house gate, exact coverage and its
   selftest included, using the digest-checked `.forge/bin/brokkr`, fixed
   shims and current bundle routing; retain command/exit evidence. Root
   coverage must have zero missed lines, branches and functions. Preserve
   the intake's intermittent matrix-stdout failure as baseline evidence;
   this design does not claim to repair it.

## Proof ownership

This chief commits the design only; implementation must supply the delay,
mutant, production-equivalence and fresh candidate verification evidence.
The sequence's deterministic check owns the design verdict. CI retains
chart/history/selftest, real Docker, network advisory audit, release-rate-5k,
port-80 and macOS obligations as assigned in the framing and house rules.
The operator retains signing/sealing, hosted settings, publication and any
decision acceptance. None becomes an unprovable local acceptance criterion.
