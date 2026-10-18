# Ordered tasks — REST and webhook validation

Resume confirmation, 2026-10-18. Follow [spec.md](spec.md), [plan.md](plan.md)
and proposed [0048](../../docs/decisions/0048-rest-webhook-validation.md).
Design `e1e142a` and implementation `2079455` already exist. The original
implementation tasks are superseded by this resume checklist, not reopened:
unchecked boxes below mean pending confirmation or delivery evidence, not
missing production code. Preserve both commits; repair only a demonstrated
issue-acceptance gap in a new signed commit.

- [ ] **R1 — Commit the design confirmation (#122/#134/#135).** Keep this
  feature slug and 0048's proposed status; commit exactly the three existing
  spec files, signed, on `epic120-rest-webhook`.
  **Proof:** artifact validation (nonempty files, Acceptance Criteria, Risks,
  checkboxes, valid relative decision link and proposed status), exact
  three-path commit diff, good signature, and both original commits still
  signed ancestors. No runtime or lifecycle-test diff from `2079455`.

- [ ] **R2 — Confirm #122's existing proof against AC1–AC3.** Inspect the
  single complete builder and validation-before-swap ordering; do not rebuild
  the feature or warm the cache during validation.
  **Proof:** `webhook_ca_put_rejects_every_invalid_certificate` in
  `tests/webhook.rs` includes complete installed-state preservation and
  subsequent authenticated delivery, as well as trailing-invalid roots;
  `webhook_second_ca_root_delivers_over_verified_https` proves later-root
  trust. `webhook_validation_does_not_dial` and inline
  `every_root_is_loaded_through_the_delivery_build` /
  `validation_never_touches_the_client_cache` pin no dial and cache isolation.
  Reconcile the original red/green map in `2079455`; fresh executions belong
  to R5. Keep existing directness, retry, credential and cache-bound proofs.

- [ ] **R3 — Confirm #134/#135's existing input proofs against AC4–AC8/AC10.**
  Keep local data-error 400s, empty view queries, inert assert limit and the
  documented compat exception. Do not replace the value-free readers with
  default serde errors that quote wrong-shaped values.
  **Proof:** `seed_unknown_key_refuses_without_storing`,
  `native_count_assert_and_views_refuse_ignored_keys`,
  `seed_and_webhook_data_errors_are_local_400s`,
  `null_strings_refuse_on_rest_like_mcp`, and
  `the_strictness_claims_match_the_wire` in `tests/api.rs`;
  `webhook_unknown_and_null_fields_preserve_configuration` and the existing
  `a_webhook_inbox_filter_answers_to_the_inbox_rule` in `tests/webhook.rs`.
  Inline `a_refused_feed_query_never_subscribes`,
  `the_target_shape_is_closed_and_refuses_by_kind_never_by_value` and served
  OpenAPI parameter/body tests pin ordering, safe errors and contract shape.
  Preserve successful feed/thread/filter/exact-count tests. Blank inboxes
  are batch-1 preservation on `ffded10`, not a new regression to manufacture.

- [ ] **R4 — Confirm #135's diagnostics and proof limits against AC9.**
  Preserve the message position, five totals, sweep/join ordering and
  eight-cause traversal limit. Review application logging for copied 4xx
  bodies and transport sources for credential disclosure.
  **Proof:** real-binary
  `sigterm_with_a_delivery_in_flight_prints_the_final_counts`, inline
  `a_transport_error_carries_no_url` and
  `causes_are_appended_in_order_and_bounded`. The chief's source inspection
  found no application log copying seed/webhook 4xx envelopes. DNS/TLS
  non-disclosure is not separately execution-pinned; review records its
  assessment instead of claiming a universal test proof. No byte-size bound
  follows from the source-depth limit.

- [ ] **R5 — Obtain fresh verify receipts against AC11–AC12.** Run the
  plan's ordered full inventory with `cargo fmt --all` before every check:
  fmt check, clippy, workspace tests, named lifecycle suite, exact coverage,
  coverage selftest, publish-guard test, licenses, fail-closed native-tls
  absence and tooling-pinned bundle compilation in verify. No TMPDIR override.
  **Proof:** recorded exit statuses and missed == 0 for lines, branches and
  functions; current host load before/after tests and a passing
  `an_in_flight_commit_neither_holds_stop_nor_half_lands` in the unchanged
  `tests/lifecycle.rs`. Avoid competing heavy builds. Retain failures and
  investigate recurrence; do not edit test timing or erase a failed receipt
  with retries. Historical quiet-host passes do not discharge this task.
  Record sandbox refusals honestly; cargo refusal requires immediate blocker
  reporting and `.brokkr/state/epic120-rest-webhook.patch`.

- [ ] **R6 — Review the confirmed feature; repair only on evidence.** Hand
  the protected review phase the acceptance map and fresh receipts, including
  accepted synchronous PUT CPU cost, in-memory configuration, root-loadability
  scope, compat tolerance and non-disclosure proof limits.
  **Proof:** review findings explicitly assess AC1–AC12 and frozen guarantees.
  Any required repair has an issue-named failing regression, a new signed
  commit and refreshed affected/full gates; no speculative implementation
  work, dependency widening, accepted-ruling rewrite or lifecycle timing edit.

- [ ] **R7 — Leave the authorized local ship handoff.** Only after verify
  and review pass, hand off the existing branch with any approved signed
  repair commits on top. Do not push, open a PR, merge or close an issue.
  **Proof:** clean tracked tree on `epic120-rest-webhook`, good signatures,
  original commit ancestry, proposed 0048 and unchanged lifecycle file.
  Report no feature-completion claim while a required receipt or review is
  outstanding; the operator's assistant owns remote landing.
