# Ordered tasks — deliverability report contract

All unchecked items below are implementation work, not claims of completed
behavior. Test names are required witnesses to add/extend in the listed owner;
existing preservation suites are named where no new test is needed. Keep
inline tests measured unless 0020's concrete cold-path rule requires a harness.

- [ ] **T1 — Record the integration prerequisite (AC7).** Receive locally the
  #152 negotiation implementation/tests and accepted 0079/0080 plus #153 shared
  output/schema implementation/tests and accepted 0082. Compare their declared
  behavior before integrating; do not fetch or invent it. **Proof:** the supplied
  negotiation/result regression suites pass locally, and
  `report_mcp_revision_matrix` later pins each revision. T2–T9 may proceed while
  this is unresolved; T10 and feature completion may not.
- [ ] **T2 — Implement typed contract and canonicalizer (AC1/AC2).** Add
  src/deliverability.rs and its module declaration; exact severity/location
  shapes, complete ordering/deduplication and size limits. **Proof:** inline
  `report_contract_variants_and_order` pins every severity, header absent/repeated
  coordinate, root/nested part path, link offset, tie-break and duplicate;
  `report_limits_refuse_without_truncation` exercises exact and over-limit
  finding/encoded-byte boundaries using bounded synthetic contributions.
- [ ] **T3 — Implement only the artificial demo (AC1/AC2).** Bounded raw header
  scanner, fixed rule id/hint/version, immutable demo_enabled config. **Proof:**
  inline `demo_marker_uses_first_raw_header` covers absence, all three tokens,
  repeated and folded fields, empty/unknown/case-mismatched values, invalid name,
  orphan/terminated continuation, LF/CRLF/EOF and non-UTF-8; plus
  `demo_header_budget_is_exact` and `report_ignores_arrival_metadata` pin full
  header limits, disabled behavior and byte-identical results across instances.
- [ ] **T4 — Add the bounded Store-owned cache (AC2/AC4).** Weak byte-object
  identity, value config/version keys, 128 slots, pinned-aware oldest eviction;
  never add Email cache fields or mutation/restore invalidation callbacks.
  **Proof:** owner tests `cache_identity_covers_bytes_config_rules`,
  `cache_rejects_clone_reinsert_and_recovered_owner`,
  `cache_eviction_never_retrieves_removed_mail` and
  `cache_weak_entries_do_not_retain_raw_mail` cover cold/warm use, independent
  version/config changes, same-id replacement, Arc ownership escape, all slot
  boundaries, delete/clear/prune and allowed pre-removal snapshot completion.
- [ ] **T5 — Bound offload and failure ownership (AC5).** Serialize same-key
  misses, reserve four permits before submission, move permit/pin/owned guard
  into worker; no cached failures or custom panic conversion. **Proof:**
  owner-local barrier tests `coalesced_miss_survives_caller_cancel`,
  `report_capacity_refuses_and_recovers`, `report_panic_releases_owned_capacity`
  and `report_failure_retries_without_clean_success`. Test distinct keys and
  stores, caller cancellation before/after submission, all-pinned cache refusal,
  worker failure/unwind and warm reuse. Add a cold-path harness only if needed.
- [ ] **T6 — Prove lazy execution and storage preservation (AC3/AC4/AC5).** Use
  a per-service test-only evaluation counter/gate; do not add runtime probes to
  SMTP. **Proof:** owner-local real-server
  `report_is_lazy_and_ingest_fields_unchanged` proves zero evaluations through
  SMTP DATA 250, seed and persistent restore, immediate exact /raw reads, then
  first evaluation and reuse; `report_workers_do_not_hold_store_guards` holds
  four workers while another real SMTP transaction commits and is queryable.
  `report_restart_recomputes_same_bytes` in tests/persistence.rs compares stored
  fields/rows before and after report/restart. Run existing burst, p2, rate,
  persistence and chaos/commit-refusal tests unchanged as preservation proofs.
- [ ] **T7 — Expose REST and legacy MCP reports (AC6/AC7).** Add the report GET,
  closed empty query domain, one required-id tool and fixed errors. **Proof:**
  tests/api.rs `report_rest_schema_and_refusals` and tests/mcp.rs
  `report_legacy_mcp_matches_rest` use real servers to compare reports, validate
  required/unknown/null/wrong-type arguments, missing/deleted/pruned ids and
  existing guard precedence. Owner seams prove invalid requests evaluate zero
  reports and saturation/failure returns the documented 503/isError shapes.
  tests/stdio.rs `report_legacy_stdio_matches_http` checks the real bridge.
- [ ] **T8 — Add shared assert/wait qualification (AC9).** Freeze inputs, qualify
  owned snapshots outside guards, keep the original unqualified path and one
  subscribe/scan loop. **Proof:** tests/api.rs
  `report_threshold_exact_count_and_min_one` and tests/mcp.rs
  `report_wait_thresholds_and_validation` cover all nine severity/threshold
  combinations, empty reports, mixed clean/dirty inboxes, exact zero/mismatch,
  empty-inbox wait, count=0, newest-first output, filter-before-page and retained
  totals/caps; prove option refusal on unrelated list/count surfaces.
- [ ] **T9 — Prove wait interleavings and truthful deadlines (AC5/AC9).** Extend
  existing owner-local seams; never synchronize by sleeping. **Proof:**
  `report_wait_subscribes_before_analysis`,
  `report_wait_observes_delivery_during_analysis`,
  `report_wait_resubscribes_after_clear`, `report_wait_recovers_lag` and
  `report_wait_deadline_and_shutdown` deterministically cover initial and Closed
  subscription windows, completion without another delivery, pinned-config
  stability, cancellation during a slot wait/worker, exact zero-timeout policy,
  complete-scan timeout and incomplete-scan unavailable. Real REST/MCP shutdown
  tests pin existing error shapes. `report_failed_scan_is_not_exact_or_clean`
  proves an assert never reports a partial successful count.
- [ ] **T10 — Integrate newer MCP through #153 (AC7; requires T1).** Register the
  report with the supplied shared schema/result owner, keep legacy text and
  current negotiation/deviation policy. **Proof:** `report_mcp_revision_matrix`
  in tests/mcp.rs and `report_stdio_revision_matrix` in tests/stdio.rs cover
  negotiated 2025-03-26/2025-06-18/2025-11-25, tools/list schema versus actual
  result, empty and nonempty findings, malformed inputs and operational errors.
  Existing #152/#153 regression suites must remain green; no new negotiator.
- [ ] **T11 — Render the message report (AC8).** Existing server-rendered page,
  same evaluator, escaped textual locations/hints, explicit empty/failure state;
  no inbox-list warming. **Proof:** tests/ui.rs `report_ui_states_and_escaping`
  checks real pages against REST content and original fields; owner fixture
  rendering injects adversarial strings for every field and confirms escaping,
  no active link construction, message 200 with failed section and missing 404.
- [ ] **T12 — Prove and document the #253 seam (AC10).** Static collector plus
  immutable typed effective-input key; no DNS/resolver/plugin framework.
  **Proof:** owner test `report_provider_snapshot_is_deterministic` adds a
  fixture rule with MIME/link locations, verifies canonical merging,
  serialization and threshold reuse, and changes its explicit input identity
  to prove no stale reuse. Production ships only demo.marker.v1.
- [ ] **T13 — Update living contract documentation (AC6/AC10).** OpenAPI,
  llms.txt, README and proposed 0085 index row; document snapshot/failure bounds,
  synthetic rule, empty-report meaning and exact-count predicate example.
  **Proof:** `report_served_docs_match_contract` fetches actual OpenAPI/llms from
  a real server and checks independent expected keys, variants, enums, statuses
  and examples; existing `every_decision_file_has_exactly_one_resolving_index_row`
  and `every_index_row_matches_its_records_status` pass without accepting 0085.
- [ ] **T14 — Commit and verify the candidate (AC10).** Run focused witnesses,
  all house gates offline, engine/shim hashes, exact coverage and selftest,
  publish/seal proofs, licenses, native-tls absence, design selftest and delivery
  proof. **Proof:** fresh per-command logs/exits/hashes bound to committed tree;
  exact zero missed lines/branches/functions; no inherited receipt substituted.
  Record CI/operator-owned proofs separately and use the candidate's unchanged
  routing/review. No push, merge, tag, publication, issue closure or acceptance.
