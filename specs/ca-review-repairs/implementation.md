# CA review repairs — implementation evidence

2026-10-25. This implements the three LOW findings in the supplied issue #220
review. The checksummed evidence packet and operator rulings verified, as did
the candidate-local engine digest `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
Accepted 0069 supplies authority; 0078 remains proposed and its record is
byte-identical. No accepted ruling, dependency, routing or capacity policy changed.

## Fixture audit

`tests/common/mod.rs` now owns CA_CONTRACTS. Common code is compiled separately
into each integration binary, matching the process-global executor's domain.
`tests/webhook.rs` imports that definition and retains its existing guards and
intentional concurrent requests. The following fixtures acquire it before server
activity and retain it through their complete responses and teardown:

| Binary | Guarded fixtures |
| --- | --- |
| api | `query_free_native_operations_refuse_by_name`, `unknown_query_config_deletes_preserve_complete_configuration`, `webhook_targets_get_set_and_cleared`, `seed_and_webhook_data_errors_are_local_400s` |
| lifecycle | `a_stalled_sink_cannot_hold_stop_hostage`, `a_fanout_in_flight_at_the_drain_is_counted_and_never_delivered` |
| p2 | `webhooks_fire_with_secret_and_retry_targets` |
| persistence | `a_refused_insert_is_never_exposed_as_accepted_mail` |

These cover successful setup, empty GETs, validation, public projection and GET
observations after DELETE. The four binaries contain no other webhook setup or
admitting calls. None of these fixtures abandons CA work: GET/PUT preparation
finishes before its response; their delivery targets use built-in roots, except
api's custom-root configuration test, which does not send mail to those targets.
The full fixture lifetime therefore includes every admitted job and owned reply.
Existing webhook tests retain their cleanup and overload assertions.

The remaining api references are independent: `query_free_native_contracts_match_served_docs`,
`openapi_webhook_get_documents_awaited_preparation_failure`,
`openapi_webhook_ca_admission_statuses` and their schema helpers inspect the
served OpenAPI document; `webhook_docs_state_bounded_ca_limits` inspects source
and documentation. `each_published_status_is_answered_on_its_operation` sends
webhook bodies refused by extraction (415/413) before preparation.
`every_published_operation_lists_and_answers_the_guard_403` exercises dynamic
operation paths with an untrusted Host, refused before admission. AppState
construction with default Webhooks in `zero_timeout_finishes_on_the_first_router_poll`
serves an await route, not webhook preparation. Other protocol fixtures remain
parallel. No success assertion was weakened and no retry was added.

## Closed-entry ownership and caller contracts

`execute_envelope` separates receiver cancellation from known scope closure.
Cancellation still disposes input before refund. A live receiver with a closed
scope receives Charged(Unavailable), carrying the original reservation after
input disposal. Live execution retains caught Worker failures and the common
reply send; receive failures are not remapped.

`closed_unentered_envelope_replies_unavailable_and_retains_charge` enqueues into
a controlled worker channel, closes before entry and retains the receiver. It
asserts explicit Unavailable, original slot/bytes through receipt, exact refund
on disposal and reuse of the entire byte budget under a fresh scope. The existing
keyed API panic remains armed, proving the skipped job never executed; executing
the same job under a live scope consumes the key and yields Worker. This uses
CACHE_TESTS and no new seam, sleep or production exclusion. The regression fails
against the prior implementation because the charge prematurely becomes zero;
`closed-entry-red.log` retains that intended failure.

Focused results (all commands exited 0):

- `webhook::ca_executor::`: 10 passed, including the new closed-entry proof,
  `cancelled_unentered_envelope_is_disposed`, `ca_closed_handoff_refunds_once`,
  `completed_unreceived_reply_keeps_its_reservation` and
  `ca_envelope_cancellation_transfers_once`.
- `webhook::ca_construction_tests::`: 21 passed. Strengthened
  `closed_scope_discards_put_and_cache_results_and_refuses_sends` checks exact
  native GET/PUT 503 bodies, unchanged installed PUT state and the fixed delivery
  refusal. `api_preparation_panic_is_keyed_and_one_shot` retains both fixed 500s;
  `builder_panic_is_value_free_and_does_not_poison_cache` and
  `delivery_builder_join_failure_retries_to_one_successful_outcome` retain genuine
  Worker behavior. `ca_pressure_delivery_recovers_or_exhausts_once` retains
  ordinary retry/terminal ownership. `all_ca_workers_held_smtp_persists_before_release`
  and `cancelled_get_retains_charge_without_blocking_smtp` retain real HTTP/SMTP
  acceptance, persistence and queryability before release.
- `api::webhook_handler_tests::`: command succeeds with zero matching test
  functions; this module supplies real-handler cancellation seams exercised by
  the construction suite above.
- `tests/webhook.rs`: 31 passed, one existing ignored test.

## Contention and committed-candidate proof

All four complete integration binaries were built first and launched directly
with `--test-threads=32`, avoiding Cargo's build lock. Five consecutive rounds
passed, with all four processes overlapping in every round: api 79, lifecycle 12,
p2 4 and persistence 12, or 535 passing tests in 20 process runs. Success
assertions remain unchanged. This finite stress evidence accompanies the static
fixture audit, not a fairness or latency claim.

Run-local evidence is under `.forge/issue220-low-repairs/implement-gates/`:
`focused-load-receipt.json` contains commands, timestamps, counts and individual
exits; `load-overlap.json` checks four-way overlap; `focused-source-sha256.json`
binds those runs to source bytes. The red log records the baseline failure.
The final `receipt.json`, written after this implementation commit, owns every
house gate's actual verdict, log digest, candidate HEAD/tree and engine identity.
The gate runner checks shims, uses the pinned bundle scripts, executes all gates
even after a failure, and records the exact coverage output and selftest. This
file does not substitute for that receipt or the recipe's hermetic verify seat.

The index contains exactly one proposed 0078 row directly after 0077. Only the
separating blank line was removed. Production Store/SMTP, subscribe-before-check,
oldest-only pruning and ingest extraction are untouched. Protected verification
and review stay on the existing route. CI/operator retain macOS, live settings,
Helm/chart, Docker, network RustSec, privileged port-80, tag-only release proofs
and sealing. No push, merge, tag, publication, issue closure or acceptance occurred.
