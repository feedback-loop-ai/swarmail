# Ordered tasks — chaos validation proof

Implementation tasks are intentionally unchecked. Each task pairs the work
with the executable proof that establishes it; AC references resolve to
[spec.md](spec.md). Read [plan.md](plan.md) for scope and evidence discipline.

- [ ] T1 — Reconfirm candidate ancestry and exact candidate-local Brokkr before
  engine use; retain triage's full lane (AC8). Proof: `git merge-base
  --is-ancestor` for both recipe/base commits, `sha256sum` of the plan's exact
  absolute binary matching the required digest, and a clean starting status.
- [ ] T2 — Run the existing affected tests and inspect real locked-response
  diagnostics before changing assertions (AC4–AC7). Proof: named API/MCP/TLS
  test receipts with source identity and observed errors; distinguish existing
  green runtime behavior from missing schema/assertion coverage.
- [ ] T3 — Publish closed chaos objects, required probability, bounded/defaulted
  delay, exact error grammar, nullable events and truthful 400/422 descriptions
  (AC1–AC3). Proof: extend and run
  `the_served_openapi_publishes_the_shared_domain`, pin owner constants and
  shape/description details; existing real served-OpenAPI test remains green.
  New schema pins must detect the old document's missing constraints.
- [ ] T4 — Complete REST and MCP probability cells with two-rule baselines and
  per-refusal snapshots (AC4). Proof:
  `chaos_wrong_typed_probability_keeps_the_extractor_envelope_and_the_rules`
  and `set_chaos_refuses_every_malformed_wire_value_by_name` show named negative
  and omission cells, exact envelopes/codes and unchanged full GET state after
  each failure; run existing legal poles/default/null-clear controls too.
- [ ] T5 — Replace weak serde-path checks with independently expected complete
  diagnostics for required `to`, all four optional seed strings and null
  `data.error` (AC6). Proof: `null_strings_refuse_on_rest_like_mcp` and
  `seed_and_webhook_data_errors_are_local_400s` assert exact decoded native
  errors/plain text and preserved no-effect/status controls. Where feasible,
  record a disposable generic-prefix-only negative control that fails these
  assertions; restore it and rerun green.
- [ ] T6 — Seed default-inbox mail once and check exact count and identity before
  and immediately after each invalid clear probe (AC7). Proof:
  `a_wrong_typed_argument_refuses_by_name` passes with nonempty baseline;
  a disposable clear-before-error control must fail where feasible. Preserve
  other malformed-tool legs and remove the obsolete terminal zero-count claim.
- [ ] T7 — Pin verbatim public CA GET after valid PUT, including newlines, and
  secret redaction (AC5). Proof: `https_target_is_delivered_over_tls` compares
  the actual decoded GET string with generated input and still observes real
  verified HTTPS delivery; existing URL-userinfo-redaction tests stay green.
- [ ] T8 — Run focused library/API/MCP/webhook tests on the final diff, review
  scope and retain honest baseline/control results (AC1–AC8). Proof: locked
  test receipts, `git diff --check`, no residual control mutations, no new
  dependency/runtime validation changes, and no unrelated tracked edits.
- [ ] T9 — Execute the complete house-rules local gate inventory and fail-closed
  native-tls proof using candidate-local build output and the digest-matched
  engine (AC8). Proof: every gate's exit status and source-bound output,
  including exact zero missed coverage and its selftest, publish/seal proofs
  and pinned-script/shim verification. Investigate failures; do not lower gates.
- [ ] T10 — Hand off to independent verify/review with remaining external CI
  obligations stated (AC8). Proof: trace every AC to its executed test receipt,
  verify recipe ancestry and unsigned commits, and report Helm/history/selftest,
  Docker, RustSec, port-80 and applicable tag-rate evidence as pending until
  genuine receipts exist. No push, merge, publish or #181 completion claim.
