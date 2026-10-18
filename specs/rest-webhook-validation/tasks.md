# Ordered tasks — REST and webhook validation

Implement [spec.md](spec.md) under [plan.md](plan.md) and proposed
[0048](../../docs/decisions/0048-rest-webhook-validation.md). All tasks below
remain open at design handoff. Proposed test names may change, but the signed
implementation commit must map #122, #134 and #135 to the actual proving tests.

- [ ] **T1 — Establish signed design delivery.** Fetch current main, verify
  `2a325c9` ancestry and earlier batches, create/check out
  `epic120-rest-webhook`, and commit exactly these three spec files and 0048
  using the configured GPG key. Do not bypass read-only Git or disable signing.
  **Proof:** nonempty artifacts, exact Acceptance Criteria/Risks headings,
  checkbox tasks, valid relative decision link with proposed status;
  `git verify-commit HEAD`, exact four-path commit diff and expected branch.

- [ ] **T2 — Record behavioral controls before repair (#122/#134/#135).**
  Add the focused wire regressions below and run against the original review
  behavior plus current main; identify already-landed blank-inbox repairs.
  **Proof:** run-local issue/test/expected/observed red receipts on `2a325c9`
  and `ffded10` where defective; unchanged preservation tests recorded green,
  with no baseline control mutations retained in the working tree (AC11).

- [ ] **T3 — Validate every CA with the delivery builder (#122 first).**
  Replace first-only parsing with uncached complete build, reused by PUT
  validation and delivery misses. Preserve cache and boundary policy.
  **Proof:** `webhook_ca_put_rejects_every_invalid_certificate` covers absent
  section, empty certificate, malformed PEM/base64, invalid DER, later invalid
  DER and later malformed block on HTTP/HTTPS; named 400s (AC1).
  `webhook_ca_refusal_preserves_complete_configuration` covers valid target
  followed by invalid target, all stored fields and subsequent old-target
  authenticated delivery (AC2). Existing valid-first failures go red on both
  bases; already-refused malformed input remains a preservation pin.

- [ ] **T4 — Prove complete trust and isolation (#122).** Retain generated-CA
  verified HTTPS; add a server trusted solely by the second root. Prove PUT
  performs no outbound request or delivery-cache mutation.
  **Proof:** `webhook_second_ca_root_delivers_over_verified_https`,
  `webhook_validation_does_not_dial_or_change_cache` with positive controls,
  isolated cache entries/builds/evictions/LRU victim checks, and existing
  directness, proxy/redirect, retry and cap tests (AC3). No global resets.

- [ ] **T5 — Close body shapes and scope the 400 amendment (#134/#135).**
  Close SeedRequest/WebhookTarget, reject supplied optional webhook nulls with
  the existing deserializer, and map only the two routes' typed JSON data
  rejections to native 400s before effects. Preserve nullable redacted output.
  **Proof:** `seed_unknown_key_refuses_without_storing`,
  `webhook_unknown_and_null_fields_preserve_configuration` (each optional
  null tested independently), `seed_and_webhook_data_errors_are_local_400s`
  for wrong types/missing fields/top-level shapes and unchanged syntax/media/
  size/chaos responses, plus `webhook_echo_shape_stays_redacted_and_is_not_input`.
  Include credential-bearing wrong-type inputs and all no-effect assertions
  (AC2/AC4/AC5/AC8). Retain existing
  `a_webhook_inbox_filter_answers_to_the_inbox_rule` as batch-1 preservation.

- [ ] **T6 — Close native query shapes (#134).** Introduce independent flat
  CountQuery/AssertQuery and one empty view query, retaining assert limit and
  all valid filter/count semantics. Parse before reading or subscribing.
  **Proof:** `native_count_assert_and_views_refuse_ignored_keys` covers each
  route/key pairing with named 400s, including supported-elsewhere filters on
  views; `count_and_assert_keep_their_filter_and_exact_count_contracts` covers
  default/zero/above-250 counts, inert limits, bad numeric types and 200/409;
  retain `the_assert_never_refuses_on_the_page_cap` (AC6).

- [ ] **T7 — Prove feed ordering (#134).** Keep successful subscribe-before-
  scan and ensure a refused query cannot construct a feed subscription.
  **Proof:** `refused_feed_query_never_subscribes` observes the per-instance
  subscription entry with an accepted positive control; real HTTP refusal
  plus no-query initial SSE and subsequent SMTP delivery prove both paths.
  Retain existing no-query thread/single-thread and feed tests (AC7). Follow
  0020 placement; do not use awaits metric counts as feed observations.

- [ ] **T8 — Repair stop and exhaustion diagnostics (#135, shared #122).**
  Add bounded source-chain formatting after URL removal; use it for TLS
  builder and delivery failures. Put the stop constant in message position.
  **Proof:** inline no-source/nested/eight-source-bound tests;
  `dispatcher_stop_has_one_message_and_five_totals` against a real binary;
  `webhook_exhaustion_names_cause_without_credentials` pins a substantive
  real transport cause, preserved prefix, no userinfo/secret, unchanged four
  attempts and exhausted accounting (AC9). Preserve constant line mapping.

- [ ] **T9 — Publish exact contracts with executable pins (#134/#135/#122).**
  Update OpenAPI request/response shapes, llms strictness scope, CA semantics
  and README examples; index 0048 as proposed without rewriting accepted text.
  **Proof:** `rest_validation_contracts_match_served_schemas_and_requests`
  checks independent allowed-key sets and actual wire behavior, local
  400/422 distinction, nullable output versus non-null request fields, and
  compat list/search tolerance alongside strict latest (AC10). Relative
  links and decision status remain valid; no blanket strictness claim remains.

- [ ] **T10 — Complete regression and gate receipts (all issues).** Run the
  targeted API/webhook/binary suites and the plan's ordered full gates,
  formatting in write mode before every check. Keep tests/code at exact
  coverage without exclusions, new dependencies or allowlist changes.
  **Proof:** green fmt, clippy, workspace tests, coverage 100/100/100 with
  missed counts zero and no TMPDIR override, coverage selftest, publish-guard
  test, licenses and fail-closed native-tls absence; verify seat records
  tooling-pinned bundle compilation honestly (AC11). Cargo refusal requires
  immediate blocker reporting and `.brokkr/state/epic120-rest-webhook.patch`.

- [ ] **T11 — Deliver signed implementation without publishing.** Commit on
  the requested branch with #122/#134/#135 in the subject and the actual
  issue-to-test red/green map in the body. Keep run-local evidence uncommitted.
  **Proof:** valid GPG signature, reviewed diff preserving earlier protections,
  clean working tree on `epic120-rest-webhook`; no push, PR, merge or issue
  closure. Do not claim completion while a required gate, signature or
  acceptance proof remains unavailable.
