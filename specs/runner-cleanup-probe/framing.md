# Issue #225 intake: runner cleanup probe synchronization

Run: `epic-192-issue-225-make-the-runn-02c2231f` (intake only).

## Goal

Make the runner cleanup regression test independent of machine load by synchronizing on an explicit cleanup-hold signal while preserving its existing controls and production behavior.

## Authority and evidence

Read `docs/house-rules.md`, the decision index, accepted 0020 and 0055, and proposed 0073. The issue packet's `README.md` and `operator-rulings.md` both pass `sha256sum --check SHA256SUMS` from `.forge/epic192/issue225/`; there are no prior-run entries after the README's prior-runs heading. The operator's macOS ruling assigns evals macOS proof to #201's CI gating. No decision is accepted by this work.

The recorded failure is `timed out waiting for runner cleanup hold entry` under heavy machine load, followed by isolated and full-suite passes. Spot-check confirms `wait_for_cleanup_pending` in `evals/src/runner.rs` uses `eventually` to poll `entered-*` markers and the actual Pending marker; `eventually` asserts a 30-second wall-clock deadline and sleeps. `OwnedGroup::cleanup_once` owns the cfg(test) cleanup hold. The inline Linux-only `runner::test_probes` module drives real child processes. Its named regression covers SIGTERM, SIGINT, panic and server-death positive paths, actual cleanup-future Pending, plus SIGTERM/SIGINT bypass negative controls and identity-safe rescue. The test also currently uses timed child waits as evidence of pending/return; synchronization repairs must preserve the causal assertions without making elapsed time their oracle.

Relevant history: `8e61ab4` (#194/#216) introduced the cleanup fence and probes; `ef8a3dd` (#196/#222) subsequently changed runner privacy. Scope is the test and owner-local cfg(test) seam in `evals/src/runner.rs`, with focused proof evidence. Do not change root production sources, dependencies, accepted decisions, coverage policy or bundle routing.

## Checkable acceptance criteria

1. The named regression obtains an explicit signal from the cleanup hold through an owner-local `#[cfg(test)]` seam before asserting cleanup state. Neither a sleep, polling deadline nor elapsed duration establishes hold entry or the Pending/cleanup-order verdict. A bounded watchdog may only diagnose a hang. The deterministic regression remains inline; at least one test installs and executes every new seam. Do not hide the test in a coverage-excluded harness or introduce a feature flag (0020 rules 1, 4 and 5).
2. Preserve and execute all existing controls: SIGTERM and SIGINT, panic and server death; observation of the actual cleanup future returning Pending; no successful return with live owned work; release followed by observed cessation and direct-child reaping; both signal bypass controls rejecting the same completion predicate while owned work is live, followed by checked teardown. Preserve failure-safe ownership and teardown.
3. A deliberate delay before hold entry, controlled through the seam, still permits the unchanged causal assertions to pass once signaled and released. Record the seam configuration, exact command, successful exit and assertions exercised. Do not replace the former deadline with a larger validity deadline or depend on machine load to generate the interleaving.
4. A temporary mutant bypassing/removing the actual awaited cleanup fence is rejected by this regression's semantic completion assertions, with a nonzero test exit. Record the exact mutation, command and failing assertion. A compilation failure, watchdog expiry, or unrelated failing test is not mutant detection. Restore the candidate afterward and rerun the positive filter; the existing expected-bypass controls alone do not substitute for this mutant proof.
5. All executable changes are test-only and compile out of non-test builds. Review the diff and compare the non-test runner representation with the pre-change baseline to establish production behavior is byte-identical; retain the comparison method and result. No runtime semantics, schemas, cleanup budgets, dependencies, manifest/lock changes or delivery routes are authorized.
6. The standalone evals fmt, all-targets clippy and full tests pass with the commands below, and every required root/pinned verification gate passes on the committed implementation candidate. Record failures honestly; intake framing itself is not implementation or verification completion.

## Frozen guarantees and decisions

No direct proximity to 0001 (synchronous insert), 0002 (mail await subscribes before checking), or 0003 (oldest-only pruning): runner cleanup awaiting is a separate lifecycle concern. All remain untouched. This is compatible test-only enforcement of accepted 0020 and 0055 lifecycle ownership; it does not amend or accept proposed 0073. No semantic decision is needed for the bounded repair.

## Verification commands and evidence

Run offline; install nothing. From the repository root, prove the standalone crate independently (root coverage does not build or measure evals under accepted 0055):

```bash
export CARGO_NET_OFFLINE=true
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --workspace --locked
cargo test --manifest-path evals/Cargo.toml --locked --lib runner::test_probes::runner_cleanup_await_pending_and_bypass_controls_cover_signals_and_faults -- --exact --nocapture
```

Use that exact regression filter for the deliberately delayed seam case and the temporary mutant. Design/implementation must record how each is selected, the mutant patch, and its assertion-level failure; rerun the unmutated filter after restoring. Keep all existing real-process test controls; no mock replacement. No benchmark is required.

Verify the engine digest equals `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f` before using `.forge/bin/brokkr` (intake checked it). Preserve this candidate's `bundles/` recipe routing. The verify seat must check fixed public shim bytes, then execute the pinned gate copies, as `bundles/self/scripts/verify-seat.sh` specifies:

```bash
sha256sum .forge/bin/brokkr
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
```

Exact coverage means missed lines, branches and functions all equal zero, with no floor reduction or denominator escape; retain `target/coverage/` summary and lcov evidence. A compile policy-sweep warning (`lets the run go on without reading`) is a failed gate even with exit zero. The fail-closed native-tls absence proof is also mandatory:

```bash
out=$(cargo tree -i native-tls 2>&1); code=$?
if [ "$code" -eq 0 ]; then
  echo "$out" >&2; exit 1
elif printf '%s' "$out" | grep -q 'did not match any packages'; then
  :
else
  echo "$out" >&2; exit 1
fi
```

The house-rule public commands `bash scripts/coverage-gate.sh`, its `selftest`, `bash scripts/publish-guard-test.sh`, and `bash scripts/seal-branch-test.sh` dispatch to those pinned copies; the verify seat validates the shims and runs the copies directly.

CI owns chart verification/history/selftest, real Docker smoke, network RustSec audit, tag-only release-rate-5k, and the real port-80 proof. The operator owns signing/sealing, hosted settings and any publication; macOS evals evidence belongs to #201 CI. These are not local issue #225 acceptance criteria. No GitHub, journal, sibling worktree, network, namespace, push, merge, tag, publication, issue closure or decision acceptance is authorized.

## Delivery boundary

Intake records this framing and returns `resolved`; it does not implement or claim the repair is complete. Preserve the same text at `.forge/tasks/epic-192-issue-225-make-the-runn-02c2231f.md` and `.brokkr/state/tasks/epic-192-issue-225-make-the-runn-02c2231f.md` for inherited council readers; these are framing evidence, not journal records. The tracked copy is `specs/runner-cleanup-probe/framing.md`. Commit authorized framing unsigned, never push. Later seats must commit implementation before verification and satisfy all proof obligations above.

## Intake baseline observations

The framing-only candidate `3c661f8` passed every root/pinned gate listed above: engine identity, shim integrity, fmt, locked all-targets clippy, locked workspace tests, bundle compile without policy-sweep warnings, exact 100/100/100 coverage, coverage selftest, design-check selftest, delivery proof, publish-guard proof, seal keyless proof, licenses and fail-closed native-tls absence. Coverage evidence is `target/coverage/run.cNFNgq`. Evals fmt and locked all-targets clippy passed.

The first full evals run exited 101: `tests::matrix_stdout_backpressure_is_bounded_and_still_observes_signals` failed at `evals/src/main.rs:2312` with `NotFound` from `Result::unwrap`. Its library tests, including the issue #225 regression, all passed. A full retry using `cargo test --manifest-path evals/Cargo.toml --workspace --locked --no-fail-fast` exited 0, including that matrix test and all integration suites. Preserve the initial intermittent failure as baseline evidence; no out-of-scope repair was made and this issue does not assert it is resolved.

The initial isolated invocation mistakenly selected `--bin swarmail-evals` and ran zero tests; it is invalid proof. The corrected `--lib` command above ran exactly one regression and passed (33 filtered out). A later Linux targeted run must likewise show one executed test, never accept an exit-zero empty selection.

Machine-readable command/exit evidence is committed in `specs/runner-cleanup-probe/intake-verification.json`; full logs remain under `.forge/tasks/epic-192-issue-225-make-the-runn-02c2231f-intake-gates/`. Long workspace calls exceeded the client's response window, but the saved driver status reached `done`; the recorded verdicts come from completed command logs, not transport timeouts. These observations assess the unchanged implementation at intake and do not prove the requested delay/mutant repair. The final intake changes only framing and this receipt, so those source/gate observations still apply; implementation needs fresh verification on its own committed candidate.
