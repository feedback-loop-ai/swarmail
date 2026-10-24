# Returned residual repair plan — matrix cancellation

Implements [spec.md](spec.md). Existing proposed
[0062](../../docs/decisions/0062-matrix-cancellation.md) remains unchanged.
This chief commits only spec.md, plan.md and tasks.md in this directory.

## Technical context

Baseline `20c20b0d2a37e87d1e7ebc0a9281573155888a74` includes residual
implementation e95f3d9 and accepted 0058. Root Rust integration test
`tests/delivery_recipes.rs` reads committed recipe/office/adapter JSON using
its existing helpers. Standalone Rust 2024 `evals/` owns Tokio process/signal/I/O
and real protocol tests; root coverage does not certify it (accepted 0055).
No dependency, feature, manifest or lock change. No plan template exists in
this checkout; the required dialect sections are supplied here directly.

## Constitution check

Preserve frozen 0001–0003, AUTH/ingest 0004–0005, 0055 schema/lifecycle/report
contracts, 0020 placement and every house gate. Accepted 0057 already settles
Luna/xhigh; 0058 settles the five-result review route. Existing proposed 0062
is not accepted; compatible work may proceed under 0045 §8. This repair adds
no semantic ruling or ADR. A contradiction requires the operator; a frozen
guarantee cannot weaken. No gate, policy, model, tooling pin, accepted-decision,
license or coverage-denominator change. Commit unsigned under 0056; never
push, merge or close the issue. Keep engine classification and return counts.

## Research and evidence

Both returned positions and updated intake identify the same concrete blocker:
all four implement selectors choose codex-fast-implementer (Luna/xhigh), while
the shared test expects Sol/high. Its adjacent dedicated test pins the accepted
route. Correct the stale oracle and assert office identity in the existing loop;
reusing helper office() avoids inventing an inheritance/route framework.

The retained 3e202f8a correctness/tests members report medium proof gaps, and
security/guarantees were clean on the predecessor. Exact seq-714 aggregate
binding is unavailable; do not invent it. The ddf345e5 return records current
root-test/coverage failure. Existing T1–T8 and implementation map
`.forge/implement-194/t1-map-d9c1d0eb.md` record delivered residual proofs;
source presence and old execution receipts are not fresh verification.
Preserve dated clarification entries and failed receipts. Inspect current
proofs before adding anything; only a reproduced residual-specific failure
justifies further runtime work at its existing owner.

The forced-expiry seam currently exits before the ordinary clock loop: it
proves propagation/reporting, not that loop or unkillable-task cessation.
The single sequential writer normally awaits acknowledgement before another
enqueue; synthetic Full-state controls are distinct from real stdout pressure.
Review these limits explicitly instead of adding producers or a new supervisor.

Fresh chief execution at initial draft 0b522d9 also reproduced AC9's
`runner_probe_failure_teardown_handles_owner_and_leader_exit` timeout waiting
for cleanup-hold entry, both in the locked suite and in isolation; a later
full `--no-fail-fast` run passed. See the spec's dated evidence and AC12.
This is an observed proof failure within the recorded teardown residual, not
permission to redesign production. Source inspection suggests investigating
whether bypass return can drop the JoinSet before cleanup tasks acknowledge
hold entry; that is a hypothesis, not an established root cause. Preserve
failed logs and determine the actual event order before the smallest local fix.

## Project structure and touched files

| Path | Work or role |
| --- | --- |
| specs/matrix-cancellation/{spec,plan,tasks}.md | Chief's exact committed scope; reconcile returned blocker and preserve history. |
| tests/delivery_recipes.rs | Implementer changes the existing four-case loop: exact office identity, singleton Luna and xhigh, with class diagnostics. Retain all adjacent assertions. |
| bundles/codex-fast/bundle.json; agents/codex-fast-implementer.json; adapters/codex.json | Read-only accepted contract inputs; inspect all four callers and resolved provider/model. |
| evals/src/main.rs; evals/src/runner.rs | Preserve implemented pending/spawn/commit controls, waits, budgets, error probes and explicit guard completion; edit only after recorded in-scope failure. |
| evals/tests/report.rs; evals/tests/runner.rs; evals/tests/common/mod.rs | Retain real CLI/lifecycle helpers, output and cleanup regressions; no speculative relocation. |
| evals/src/report.rs; TrialResult/refused_row; evals/tests/proof.rs | Unchanged consumers and oracle/null/isolation proof. |

No other spec directory is part of the chief commit. Root src/, recipes,
adapters, policies, accepted decisions, gate scripts, tooling pins, manifests,
locks and journals are not repair targets. README changes need an actual
in-scope divergence. Any necessary seam remains owner-local cfg(test), with
an installer and ordinary dev/release isolation proof; binary and library
unit configurations are separate. No coverage exclusions or test framework.

## Data model and contracts

No new data model or public interface. Preserve schema-1 TrialResult, immutable
queued bytes, raw observed statuses, full ordered product and both report
formulas. Attempts include unstarted slots. The active interrupted row follows
cleanup, with measured lifetime and observed statuses; no usable row means a
fresh-ID unknown-status fallback. Unstarted rows have fresh IDs, zero elapsed
time and no launch. Handled TERM/INT exit 2; ordinary precedence stays 2/1/0.

Preserve subscriptions before owned spawn, sticky scheduling stop, cancellation
winning pending commitment, retained concurrently issued children, one forwarded
TERM and one cleanup owner per group. Close/join admission before cleanup;
retain direct waits and clean groups concurrently under fixed five-second TERM
plus five-second cessation/reap bounds. Repeated signals do not renew deadlines;
matrix has no shorter competing kill timer. Unknown observation, direct-wait
failure or expiry remains diagnosed failure, never successful cessation.

Keep 64 KiB capture, five-second output acknowledgement and post-exit EOF bounds,
real Linux identity observation and conservative other-Unix behavior. Group
leader exit, kill success, EOF, cessation and direct reaping are different facts.
Complete JSONL requires progressing stdout; cleanup and output completion remain
separate. No universal wall-time or hostile-process confinement promise.

## Implementation approach

1. Record actual candidate revision/tree and engine hash. Reproduce the named
   shared-selector failure before correction; retain its exit and diagnostic.
   Compare all four callers with accepted 0057/0058, not merely the stale test.
2. In that loop, assert implement[class]["agent"] == "codex-fast-implementer",
   then resolve office() and assert models == ["luna"] and efforts.luna ==
   "xhigh". Keep class diagnostics for all three assertions. Expected values
   are literal accepted requirements, never copied from the config under test.
   Retain exact case/default and reviewer-map checks, dedicated adapter/model
   and no-fallback regression, and existing route/provider/inheritance proofs.
3. Run both targeted model tests and full delivery_recipes. Assertions must
   catch wrong office even when its model strings coincide, Sol/high, wrong
   provider/model/effort, fallback, missing case/default or reviewer-map drift.
   Structural tests complement engine compilation/route proofs, not replace them.
4. First resolve AC12 in the existing runner test/control owner: capture actual
   Pending, cleanup task/hold entry, bypass return and leader exit causally.
   If task issuance/bypass scheduling is responsible, add only the necessary
   cfg(test) acknowledgement before deliberately allowing the broken return;
   retain the actual-future Pending observation and failed cleanup predicate.
   Do not extend deadlines, insert sleep-as-sync, weaken survivor/finish checks,
   disable the regression or assume a runtime defect from a harness timeout.
   Prove the ordering deterministically, then run the isolated regression and
   full suite. Reproduce existing residual proof families below with readiness,
   actual awaited-future Pending and release events with real processes; no
   sleep-as-sync or mocks. Positive teardown fails on rescue/survivors/unreaped
   direct children; bypass controls first establish product failure, then rescue.
   Revalidate test-owned identities before signalling. Preserve failure evidence.
5. Run standalone evals checks, then all root gates and pinned proof checks.
   If a concrete residual-specific failure emerges, repair only its owning
   boundary and rerun affected proof. Broader discoveries return through the
   engine's bounded policy. Do not repeat completed proof implementation.
6. Reconcile task evidence only when earned. Present the actual verified
   candidate for all five separate accepted 0058 results. Do not close T9/T10
   from historical success or chief structural checks; only the engine rules
   advancement. Chief's design commit remains HEAD at handoff.

## Test map and quickstart

At root, before correction run the first command and retain its expected
Sol/Luna assertion failure. After correction run all three:

```bash
cargo test --locked --test delivery_recipes fast_strategy_selectors_have_no_default_and_keep_one_two_or_five_reviewers
cargo test --locked --test delivery_recipes fast_implementation_uses_luna_at_xhigh_without_provider_fallback
cargo test --locked --test delivery_recipes
```

From evals/, retain and reproduce these named controls (cargo test --locked
accepts each function name as a filter); the complete suite includes them:

| Criteria | Existing proof |
| --- | --- |
| AC1/3 | matrix_sigterm_and_sigint_wait_for_the_active_trials_owned_cleanup; real report ingestion and ordinary aggregation tests |
| AC2 | matrix_completion_fence_is_observed_and_its_negative_control_exits_early; runner_cleanup_await_pending_and_bypass_controls_cover_signals_and_faults |
| AC4 | pending_matrix_decisions_observe_queued_signals_and_retain_spawned_children; mixed_signals_during_matrix_cleanup_do_not_bypass_the_retained_wait; mixed_signals_after_cleanup_budget_creation_keep_one_owner_and_fixed_budgets |
| AC5 | cleanup_observation_error_and_forced_expiry_fail_with_live_owned_processes; resistant-descendant/leader-first tests; explicitly retain the expiry proof limit |
| AC6 | matrix_spawn_failure_keeps_all_rows_and_unknown_agent_statuses; matrix_signal_cancels_and_reaps_the_fallback_cargo_group; setup/bootstrap and once-only resolution controls |
| AC7 | matrix_stdout_backpressure_is_bounded_and_still_observes_signals; matrix_post_exit_inherited_pipe_is_bounded_and_holder_is_reaped_by_the_test; a_broken_matrix_stdout_stops_after_the_owned_trial_finishes |
| AC8/9/12 | runner_probe_failure_teardown_handles_owner_and_leader_exit; runner_guard_finish_rejection_fails_in_isolated_process_and_rescues; matrix_probe_readiness_unwind_releases_and_reaps_owned_processes; normal dev/release control isolation |

Check repeated-signal proof causally after budget creation: each signal receipt
is acknowledged, owner identity and the retained future are unchanged, and
actual event order supports fixed deadlines. Constants printed as 5000/10000
alone cannot establish no renewal. Keep live work and both bypass falsifiers.
Preserve guard timing: 30-second post-release ordinary wait, then 15-second TERM
allowance and five-second forced observation/reap; no shorter test kill clock.

```bash
# From evals/
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked --test report
cargo test --locked --test runner
cargo test --locked
```

For AC12, from evals/ use the exact isolated reproduction before and after
repair, keeping its stderr and exit; a green retry alone cannot close it:

```bash
cargo test --locked --lib runner::test_probes::runner_probe_failure_teardown_handles_owner_and_leader_exit -- --exact --nocapture
```

Full evals includes proof.rs oracle/null/isolation, interruption/deadline/panic/
server-death/normal cleanup, validators and report formulas. Repeat ordinary
release-build seam-isolation checks with the current full control inventory;
record commands and exits in run-local evidence, not inferred from cfg(test).

For AC10 use the single authoritative [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change).
Hash the candidate-local absolute .forge/bin/brokkr against tooling.json before
compile. Verify pinned shim integrity and fail-closed native-tls absence; execute
pinned coverage plus selftest, publish/keyless-seal, design selftest and delivery
tamper-boundary proofs. Hermetic verify installs nothing and executes pinned
implementations. CI retains its chart/history/selftest, Docker, advisory,
tag-rate and port-80 duties. No local receipt claims those CI results.

Five separate current-candidate reviews: Astra correctness, official DeepSeek
Flash security, Sonnet 5.5 tests, Astra guarantees and independent Fable 5.1.
Preserve protected review, worst verdict, hard security hold and return limits.
No DeepSeek Pro, unauthorized fallback, review downgrade or stale receipt.

## Risks

| Risk | Mitigation and accepted cost |
| --- | --- |
| Green obtained by weakening selector assertions | Literal office/model/effort requirements; preserve case/default, provider and reviewer-map checks. |
| Test mismatch becomes unauthorized policy rollback | Accepted 0057/0058 are authority; recipe, model and adapter stay unchanged. |
| Earlier scope or checked tasks treated as current proof | Keep dated entries and failed return, retain missing seq binding, require revision/command/exit evidence. |
| Repeating completed work expands runtime scope | Verify current controls first; runtime changes require a reproduced recorded residual failure. |
| Signals coalesce or tests miss actual awaited boundaries | Per-signal acknowledgements, post-budget actual Pending and matched bypass falsifiers. |
| Fixed constants or injected expiry overclaim time proof | Trace owner/future/event causality; distinguish seam propagation from real clock and kernel limits. |
| Synthetic fullness substitutes for stdout/EOF proof | Preserve distinct real unread-pipe and inherited-descriptor tests and reachability explanation. |
| Teardown hold acknowledgement races bypass return | Retain failed isolated/full-suite receipts; prove event order with owner-local test acknowledgements before repair and keep the negative predicate effective. |
| Test rescue masks unsafe completion | Explicit checked finish, nonzero forced-failure probes, identity-safe bounded unwind rescue. |
| Model-only patch downgrades lifecycle review | Retain engine lane and all five 0058 results despite latency. |
| Output/cleanup promise exceeds available evidence | Keep progressing-stdout condition, O(product) suffix, conservative unknown failures and escape limits. |
| Root gates mistaken for eval proof | Execute standalone evals suite, fmt/clippy and dev/release isolation in addition to every root gate. |
