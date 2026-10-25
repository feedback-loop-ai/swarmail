# Implementation plan — CA review repairs

Implements [spec.md](spec.md) and its five acceptance criteria.

## Technical context

One Rust crate; existing Tokio async test mutex, oneshot replies, std worker
channels and reservation destructors. No dependency or public API change.
`.specify/templates/plan-template.md` is absent (also recorded by the predecessor
plan); its required sections are supplied here. Research, data model, contracts
and quickstart are inline so this design needs exactly three committed files.

## Constitution check

Accepted 0069 authorizes the existing outcome; proposed 0078 is not accepted or
amended. This is implementation conformance, not a semantic amendment. Preserve
0001–0005, 0023/0030 offload and Store panic symmetry, 0041 shutdown order and
0067 no-builder-drain. Preserve routing in the candidate bundles and protected
review/security hold. Use real HTTP/SMTP, existing keyed seams and decision
0020 test placement; no production coverage exclusions or new test feature.

## Research and resolved approach

`execute_envelope` currently combines `reply.is_closed()` and
`scope.live().is_none()`, drops input/reservation and silently closes the sender.
`Reservation::run` correctly treats genuine receive failure as Worker, so fix
its cause at entry rather than remapping all receive failures. Existing API
live-scope checks and delivery error mapping already handle Unavailable.

`tests/webhook.rs` owns a private CA_CONTRACTS mutex, while four other binaries
admit without one. Common code is compiled separately into each integration
binary, exactly matching the process-global executor's contention domain.
Even empty GET and PUT without custom roots admit. DELETE and early refusal
may not admit, but their successful setup/observations do.

## Project structure

| Existing path | Planned implementation change / proof |
| --- | --- |
| `tests/common/mod.rs` | Define the shared async CA_CONTRACTS fixture and explain its per-process lifetime. |
| `tests/webhook.rs` | Import that fixture, remove the duplicate static, preserve all guards and intended concurrency. |
| `tests/api.rs` | Guard success-dependent admitting tests, including mixed query/body/DELETE contracts. |
| `tests/lifecycle.rs`, `tests/p2.rs`, `tests/persistence.rs` | Guard admitting configuration/delivery fixtures through cleanup and owned results. |
| `src/webhook/ca_executor.rs` | Explicit charged Unavailable for a live receiver whose scope is closed at entry. |
| `src/webhook/ca_executor/executor_tests.rs` | Extend existing entry/disposal/retained-reply tests to prove outcome, nonexecution, charge and refund. |
| `src/api/webhook_handler_tests.rs`, `src/webhook/ca_construction_tests.rs` | Reuse caller, panic, cancellation and persistent SMTP isolation proofs; extend only a missing assertion needed by AC3. |
| `docs/decisions/README.md` | Delete only the blank line before 0078. |

Starting fixture inventory: api's `query_free_native_operations_refuse_by_name`,
`unknown_query_config_deletes_preserve_complete_configuration`,
`webhook_targets_get_set_and_cleared`, `seed_and_webhook_data_errors_are_local_400s`;
lifecycle's `a_stalled_sink_cannot_hold_stop_hostage` and
`a_fanout_in_flight_at_the_drain_is_counted_and_never_delivered`;
p2's `webhooks_fire_with_secret_and_retry_targets`; persistence's
`a_refused_insert_is_never_exposed_as_accepted_mail`. Audit all route/setup
callers, not just these names. Record exempt sites with a pre-admission or
non-execution reason. Do not automatically lock common HTTP/start helpers:
nested locks deadlock and per-request locks release ownership too early.

## Data model and ownership

No new types or persisted data. Preserve Envelope -> Charged -> disposal as the
single reservation chain:

| Entry state | Job action | Reply and reservation |
| --- | --- | --- |
| Receiver already cancelled | Drop input without executing | Drop reservation after input; no reply needed. |
| Live receiver, closed scope | Drop input without executing | Send Charged(Unavailable) carrying the original reservation. |
| Live receiver, live scope | Existing caught job execution | Existing Charged result / Worker on panic. |
| Receiver cancels during send | No extra execution | Failed send owns and drops Charged, refunding once. |

Use the ordinary reply send for both live-receiver branches where practical.
Never refund then reconstruct a reservation. Do not hold the scope lock across
construction. A close after the entry check does not hard-cancel admitted work;
existing live-owner checks guard installation/publication/send.

Acquire the async fixture at the test boundary before relevant server activity;
retain it through CA results and cleanup. Where work is abandoned, observe
existing completion/refund before releasing the guard. Do not add a production
drain or use a blocking mutex across await.

## Contracts and deterministic proof

Keep exact native unavailable/error strings, status mappings, request guard and
extractor precedence, refused-PUT state and delivery attempt/terminal owners.
Reuse `cancelled_and_closed_unentered_envelopes_are_disposed`, separating its
cases as needed: enqueue into its controlled channel, close the scope, enter
worker, receive and retain the refusal, check nonexecution and usage, dispose,
check zero and reserve under a fresh scope. Waiting for idle before consuming
this reply is forbidden: the test itself owns the charge. Use an existing
keyed execution observation to distinguish skipped work from a fabricated
error after execution. Existing private harness covers cold failure paths;
no new module/seam unless a concrete missing observation demands an exercised,
owner-local cfg(test) seam under 0020.

Run actual caller tests and preserve existing Worker panic/channel tests.
Record final test names. The new closed-entry assertion must reject the old
sender-drop behavior; no broad receive-error remapping is an acceptable repair.

## Quickstart and verification

Use offline installed tooling and the candidate-local engine; do not fetch or
install. Preserve the intake's full gate command inventory and candidate-bound
receipts, checking shims then running pinned bundle scripts. The charter remains
the sole gate authority. Execute all gates even if an earlier one fails; retain
each exit code. Compilation must have no policy-sweep warning. Final hermetic
verification is the existing recipe's verify seat.

Focused commands:

```bash
export CARGO_NET_OFFLINE=true
cargo test --workspace --locked --lib webhook::ca_executor::
cargo test --workspace --locked --lib webhook::ca_construction_tests::
cargo test --workspace --locked --lib api::webhook_handler_tests::
cargo test --locked --test webhook
```

For AC1, build all four binaries first, then run the following five rounds;
Cargo's build lock must not be mistaken for test concurrency. Record each
process's exit and each suite's final counts in an ignored receipt alongside
its log. Verify overlapping execution from timestamps; prebuilt binaries may
be invoked directly if Cargo launch serialization prevents that overlap.

```bash
cargo test --locked --test api --test lifecycle --test p2 --test persistence --no-run
mkdir -p target/issue220-contention
for round in 1 2 3 4 5; do
  pids=()
  for suite in api lifecycle p2 persistence; do
    cargo test --locked --test "$suite" -- --test-threads=32 \
      >"target/issue220-contention/${suite}-${round}.log" 2>&1 &
    pids+=("$!")
  done
  failed=0
  for pid in "${pids[@]}"; do wait "$pid" || failed=1; done
  test "$failed" -eq 0 || exit 1
done
```

A single passing baseline or serial suite is not proof of isolation. Pair load
results with the static inventory and deterministic worker-entry ownership proof.
The full current gate inventory and additional pinned design/delivery proofs are
specified in [intake](../bounded-ca-executor/intake-low-repairs.md#required-verification).
No throughput/latency benchmark or remote claim is required.

## Risks

- Missing indirect setup leaves flaky admission. Audit every caller and record
  exemptions; run the complete four binaries under intra-process contention.
- Guard releases while abandoned work owns capacity. Retain full fixture scope
  and explicit existing refund/completion observation; stop is not a drain.
- Nested locking stalls tests. Acquire once at fixture entry; helpers remain
  lock-free and dedicated overload requests run inside the enclosing fixture.
- Early refund or double ownership breaks W/J/B. Move the original reservation,
  assert retained charge and exact refund, and retain failed-send disposal proof.
- Broad RecvError remapping hides worker failures. Change only the known closed-
  scope branch and preserve Worker assertions and fixed 500 contracts.
- An empty job does not prove nonexecution. Use a keyed execution observation;
  assert its absence and exercise its installation so the proof cannot be vacuous.
- Stress can pass by chance and scoped locking costs time. Static coverage and
  deterministic tests carry the invariant; five recorded rounds corroborate it.
- Extra scope or new policy would require authority. Limit changes to the listed
  repairs, leave 0078 proposed, and leave CI/operator-owned proofs with them.
