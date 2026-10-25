# Issue #220 LOW review repairs — intake

Run: `epic-192-issue-220-close-the-thr-f2af8f8f`.
Baseline: `0edd421d2db618d966264a375e7444edb6b4742d`.
Phase: intake; implementation and final verification remain downstream.

## Goal

Close exactly the three LOW findings in the bounded CA executor review: isolate successful-admission fixtures, return an explicit unavailable outcome for a scope closed before worker entry, and rejoin the proposed 0078 decision-index row.

## Authority and scope

Read the workspace house rules, decision index, frozen 0001–0003, test-isolation 0004, accepted 0069 and proposed 0078, the supplied issue/operator packet and full review. The packet passes `(cd .forge/epic192/issue220 && sha256sum -c SHA256SUMS)`, including README.md and operator-rulings.md. Its original issue/proposal wording is historical: the dated operator ruling accepts 0069 on 2026-10-25. 0078 remains proposed. The engine digest matches `bundles/self/tooling.json`: `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.

The full supplied review is `.forge/epic192/issue220/review-e3bd9c06.md`; its evidence and the task's named paths are the basis for this framing, not findings to re-audit. The packet README has an empty prior-runs section; the supplied review and task provide the predecessor outcomes (REVIEW-RESIDUAL-OK and the earlier parallel api failure). No journal, GitHub, CI or sibling checkout was consulted.

Spot-checks: `21d9b76` introduced process-owned bounded CA execution; HEAD `0edd421` repairs the native error schemas. `execute_envelope` currently combines cancelled-receiver and closed-scope disposal, dropping the sender; `Reservation::run` maps the resulting receive error to Worker. The existing `cancelled_and_closed_unentered_envelopes_are_disposed` verifies reclamation without asserting a closed-scope reply. `tests/webhook.rs` already uses a scoped `CA_CONTRACTS` mutex; equivalent admitting fixtures in api/lifecycle/p2/persistence need isolation. The index has a blank line immediately before 0078. The worktree was clean at intake.

This work sits nearest frozen **0001**: webhook preparation/refusal must never delay or qualify SMTP acceptance, and DATA 250 must still mean queryable stored mail. Preserve **0002** subscribe-before-check and **0003** oldest-only pruning. Store/SMTP placement, worker bounds, accounting tariff, process ownership, admission fairness, cache policy, retry/backoff and shutdown order are outside these three repairs. No new dependency, routing change, decision acceptance or INFO-finding repair is authorized.

## Checkable acceptance criteria

1. **Fixture isolation.** Inventory every test in `tests/api.rs`, `tests/lifecycle.rs`, `tests/p2.rs` and `tests/persistence.rs` that requires successful server-managed webhook GET/PUT preparation (including GET observations after DELETE and configurations used for delivery). Each holds the shared admission fixture for the admitting work and owned-result lifetime, or has a documented independence proof against the process-global policy. Scope the guard to these fixtures; keep unrelated tests parallel. Preserve dedicated overload/refusal tests and their assertions. No retries masking 503, production admission relaxation or whole-suite serialization. Run all four complete integration binaries with 32 test threads for five consecutive successful rounds, with competing test processes as specified below; record commands, exit codes and counts, not a claim based on an isolated pass.
2. **Closed scope outcome and ownership.** Deterministically enqueue an envelope, close its scope before worker entry and keep its waiter alive. Assert that the job body does not execute and the waiter receives the explicit `TargetPreparationError::Unavailable` outcome rather than channel closure/Worker. Assert job disposal and exactly-once slot/byte refund; if a charged reply retains the reservation, verify it stays charged until reply disposal. Keep the cancelled-receiver case and execution/panic Worker cases green. GET and PUT map Unavailable to native 503 with exactly `webhook CA preparation unavailable`; delivery retains that fixed attempt error and existing retry/terminal ownership. Execution/panic failures retain their distinct fixed 500/worker meanings. Prove the worker-entry case in the existing executor harness and the caller mappings with the existing or extended handler/construction tests. Use channels/barriers or owner-local test seams under 0020, with no sleep synchronization, mid-band chaos or payload-bearing diagnostics.
3. **Index and status.** The 0078 row is contiguous with the decision-index table (no intervening blank line), appears exactly once, and still says proposed. `docs/decisions/0078-ca-executor-ownership.md` retains its proposed status; accepted rulings are unchanged.
4. **Completion evidence.** Commit all candidate changes unsigned before deterministic verification. All house gates below pass for that committed candidate, including exact zero missed lines, branches and functions and the coverage selftest. Retain candidate-bound logs/receipts; a failing gate is unfinished downstream work. Protected review uses this candidate's existing recipe routing. No push, merge, tag, publication, issue closure or decision acceptance.

## Required verification

The verify seat is offline and installs nothing. Use the candidate-local engine after checking its exact digest. Public dispatch shims are checked against the pinned manifest; execute the pinned copies as the verify seat prescribes. These are required downstream commands, not passing results asserted by this intake document.

Focused regressions and parallel fixture load (four binaries are separate processes; admission is process-global within each binary, so `--test-threads=32` supplies the relevant intra-process contention; concurrent binaries add scheduling load):

```bash
export CARGO_NET_OFFLINE=true
cargo test --workspace --locked --lib webhook::ca_executor::
cargo test --workspace --locked --lib webhook::ca_construction_tests::
cargo test --workspace --locked --lib api::webhook_handler_tests::
cargo test --locked --test webhook
for round in 1 2 3 4 5; do
  pids=()
  for suite in api lifecycle p2 persistence; do
    cargo test --locked --test "$suite" -- --test-threads=32 >"target/issue220-${suite}-${round}.log" 2>&1 &
    pids+=("$!")
  done
  failed=0
  for pid in "${pids[@]}"; do wait "$pid" || failed=1; done
  test "$failed" -eq 0 || exit 1
done
```

Record the final test names for AC2 and their results. Ordinary-fixture tests must retain success assertions so missing isolation can fail; the closed-entry regression must fail against the prior sender-drop behavior. No benchmark is required: this repair makes no new throughput or latency claim.

Full house and pinned proof inventory, retaining each exit code even if an earlier command fails:

```bash
export CARGO_NET_OFFLINE=true
export PATH="$PWD/.forge/bin:$PATH"
echo 'ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f  .forge/bin/brokkr' | sha256sum -c -
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
out=$(cargo tree --locked -i native-tls 2>&1); code=$?
if [ "$code" -ne 0 ] && printf '%s' "$out" | rg -q 'did not match any packages'; then
  :
else
  printf '%s\n' "$out"; exit 1
fi
```

Bundle compilation must also emit no policy-sweep warning. The gate floor stays exact 100% across all three dimensions. Use `bundles/self/scripts/verify-seat.sh` through the existing engine route for the authoritative hermetic verdict; this intake's local attempts do not substitute for that seat.

CI/operator own macOS, live branch settings, Helm/chart baseline/history proofs, Docker smoke, network-backed RustSec, privileged port-80 exchanges, tag-only release proofs and eventual sealing. Namespace work is outside this task. None is an unprovable local acceptance condition. Intake is resolved once this framing is recorded; it does not claim the three code/test/documentation repairs have landed.
