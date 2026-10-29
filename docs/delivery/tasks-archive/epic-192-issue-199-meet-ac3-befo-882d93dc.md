# Intake — close #199 AC3 drain-retention residual

Run: `epic-192-issue-199-meet-ac3-befo-882d93dc`
Phase: intake only; resolved means recorded framing, not implementation acceptance.

## Goal

Make failed cleanup retain every outstanding output drain and its admission/private-state lifetime until actual completion on a surviving runtime, and prove #199 AC3 with a regression that fails on the current port.

## Authority and bounded evidence

Read the issue199 README and operator rulings first, then verified all eight entries from `.forge/epic192/issue199/SHA256SUMS` from that directory: both authority files, feature, port, three patches and review all match. An initial checksum invocation from the repository root used the wrong relative base; the directory-relative verification passed every entry. Read house rules, the decision index, accepted 0055 including its dated #199 amendment, proposed 0076 and 0073, accepted 0020 and 0058, the full `review-82604c8b.md`, the prior intake framing and current AC3/R3. No external source, journal or sibling worktree was accessed; no required framing input is missing.

The operator chose sequential refusal, fresh HOME and honest documentation, with namespaces owned by #221 and macOS eval proof by #201. That ruling is authority without accepting 0076. Keep 0076 proposed and all other decisions/statuses intact. Current HEAD is `d973b0eed2605524f3b44eb21f6388593e3e1982`, the reviewed port; tracked checkout was clean. Runner history includes port commit d925b1d and current-main #200/#225/#198 integration. Historical gate receipts are evidence of those candidates only.

Spot-check confirms `OwnedGroup::ensure_cleanup` caches the cleanup verdict, then retains its group until cessation/reaping and awaits `group.drains` on failure. `cleanup_once` takes that entire vector before bounded joins and returns on the first timeout, dropping/detaching the timed-out handle and all remaining handles. The captured-helper stdout and Cargo stdout closures in `spawn_owned` capture neither admission nor private runtime, while ordinary drains do. Thus empty group membership, reaped child and closed ordinary stderr can leave live stdout work outside the claimed lifetime. This is the review's source finding, not an intake live reproduction.

The current failed-observation/forced-expiry regression observes a surviving owned process but does not assert contender refusal on the failed-cleanup path. Existing acknowledged-Pending cleanup and finalize holds cover different ordering windows. The candidate-local `.forge/bin/brokkr` digest matches `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`. Inspected the pinned verify commands and candidate recipe definitions; use those routes unchanged, with the engine owning phase selection and protected review.

## Scope and guarantees

Expected implementation owner is `evals/src/runner.rs`, with narrowly necessary cleanup regression support in the existing eval test harness. Design chooses retention of handles or equivalent ownership that actually keeps all outstanding drains/lifetimes. Do not add a second supervisor, change cleanup budgets or verdict caching, or expand the task into general admission, isolation, documentation or test cleanup.

This work is outside frozen 0001 synchronous accept/store, 0002 subscribe-before-check and 0003 oldest-only pruning. All remain untouched, as do 0004/0005. It restores existing 0055 lifecycle ownership and #199 R3/AC3 under the supplied ruling; it requests no new semantic exception or decision acceptance. Keep scoring, provenance, concurrency rows, matrix accounting, privacy, dependencies, allowlist, root source, gate scripts and recipe routing unchanged.

## Checkable acceptance criteria

1. **Every pending drain stays owned after failure.** When the bounded output wait fails, the timed-out handle and every later unjoined handle remain owned through actual completion. With a surviving runtime, the direct child reaped, no running group members and ordinary stderr finished, a still-pending captured-helper stdout or Cargo stdout drain keeps admission held and its private directory present. Reporting a cleanup error cannot drop, detach without retained lifetime, or abort those pending drains to manufacture completion. The same invariant holds for remaining handles after the first timeout.
2. **Causal red/green helper and Cargo proof.** Add deterministic regression cases exercising both actual stdout paths, with acknowledged pending work and the actual drain-timeout branch. While pending after cleanup failure, assert `Admission::acquire()` returns exactly `concurrent_trials_refused` and the private-state path remains present. Release the controlled pending work, acknowledge drain completion, drop remaining legitimate owners, then prove later admission succeeds and private state is disposed. Control lifetime deliberately so a retained caller/Groups/Proc reference or an unrelated stderr drain cannot make a broken implementation pass. Keep the runtime alive during these observations. The new test(s) must compile and fail against d973b0e at the lifetime/admission assertion, then pass with the fix; a tooling, scheduling, schema or compilation failure is not the red proof. Record exact names, commands and observations. No sleep-as-synchronization, hostile poacher implementation or external namespace proof is required.
3. **Close the same-path LOW finding.** Extend `cleanup_observation_error_and_forced_expiry_fail_with_live_owned_processes`, or an equivalent causal surviving-runtime probe, so both observation failure and forced expiry assert admission refusal while acknowledged live owned work remains after the failed verdict. Cleanup/release then permits admission. A temporary early-release negative control must fail that assertion and be restored before final gates. Retain existing actual-entry/Pending/release handshake, identity-aware teardown and checked surviving-process assertions.
4. **Preserve established failure behavior.** Cleanup failure remains a cached harness error with the existing finite diagnostic/category; eventual drain completion must not convert it to success or restart cleanup deadlines. Keep one cleanup owner, direct-child reaping, fixed TERM/KILL budgets, cancellation and dropped-library-waiter lifetime, finalize fence, and matrix stop-admission/full-suffix behavior. Existing cleanup, sequential, matrix, trap, privacy and provenance tests pass without weakened assertions. Any new seam is test-only and exercised; follow 0020 placement disciplines. Evals remains standalone outside root coverage; root green alone is not eval proof.
5. **Candidate-bound completion.** Commit the authorized implementation and regressions unsigned before final verification. All local house commands and standalone eval checks below pass for that candidate, including exact root coverage with zero missed lines/branches/functions and its selftest. Record fresh red/green evidence and candidate identity; retain unchanged proposal statuses, dependency/routing files and unrelated code in the final diff. A previous run's green receipt does not satisfy this criterion.

## Review disposition within this scope

The correctness MEDIUM drain timeout/ownership defect and tests-panel LOW missing failed-cleanup admission assertion are in scope (AC1–3). The correctness INFO about restricted gate reruns is addressed by fresh candidate-bound local verification, without claiming old sandbox attempts were green. The independent INFO about worker harness-error category consistency is on the cleanup reporting boundary: preserve and rerun `unproved_worker_cleanup_stops_matrix_with_full_refusal_suffix` under AC4; it reports no additional defect to fix.

The independent LOW packet-link boundary test and uncoordinated retained-child cancellation probe are different paths and are not authorized by this follow-up. Likewise unrelated INFO about admission error-arm coverage, path-vs-openat checks, unused barrier helpers and generic harness-error tests remains recorded in the supplied review; do not broaden scope to resolve it. If any required gate exposes one as a failure, record the exact failed command/evidence and surface the scope conflict instead of weakening a gate or quietly changing unrelated code. No review finding is silently declared fixed.

## Verification commands and owners

Intake verifies packet/engine integrity and records commands; implementation and hermetic verify execute the gates after the candidate exists. Work offline with provisioned tools; install nothing. Use candidate-local engine and the current bundle's pinned proof chain. Public house commands below are traceability names: hermetic verify validates `bundles/self/scripts/shims.sha256` and executes the corresponding pinned scripts directly, never unverified public wrappers.

```bash
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum -c -
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
bash scripts/seal-branch-test.sh
cargo deny check licenses
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
```

Pinned script equivalents for the four public gate invocations above are `bash bundles/self/scripts/coverage-gate.sh`, its `selftest`, `bash bundles/self/scripts/publish-guard-test.sh`, and `bash bundles/self/scripts/seal-branch-test.sh`. Keep the pinned fail-closed dependency absence proof:

```bash
out=$(cargo tree -i native-tls 2>&1); code=$?
if [ "$code" -eq 0 ]; then
  echo 'REFUSAL: native-tls is present' >&2; exit 1
elif printf '%s' "$out" | grep -q 'did not match any packages'; then
  :
else
  printf '%s\n' "$out" >&2; exit 1
fi
```

Standalone eval gates (default Cargo test threading) and focused regression commands:

```bash
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --workspace --locked
cargo test --manifest-path evals/Cargo.toml --locked cleanup_observation_error_and_forced_expiry_fail_with_live_owned_processes
cargo test --manifest-path evals/Cargo.toml --locked runner_cleanup_await_pending_and_bypass_controls_cover_signals_and_faults
cargo test --manifest-path evals/Cargo.toml --locked cleanup_handshake_
cargo test --manifest-path evals/Cargo.toml --locked sequential_admission_library_and_cleanup_lifetime
cargo test --manifest-path evals/Cargo.toml --locked unproved_worker_cleanup_stops_matrix_with_full_refusal_suffix
cargo test --manifest-path evals/Cargo.toml --locked --test proof
cargo test --manifest-path evals/Cargo.toml --locked --test runner
cargo test --manifest-path evals/Cargo.toml --locked --test report
```

Design/implementation must add the exact new helper/Cargo drain regression filters to its evidence; the full standalone command above executes every new test regardless of its name. No-match filters are not proof. No performance claim requires `cargo bench`.

CI owns the house charter's privileged port-80, Helm/chart/history, Docker, network RustSec and release-rate proofs; #201 owns macOS eval proof. The operator owns live GitHub settings, sealing, cleanup after hard death, and external updates. These are ownership records, not new local acceptance conditions. No push, merge, tag, publication, issue closure, decision acceptance, network access or namespace setup is authorized.

## Intake handoff

No implementation or full gate run occurred in intake. Record and commit this framing; return resolved only for actionable intake. Implementation, fresh local verification and protected review remain outstanding in their designated phases. The engine alone decides the next phase.
