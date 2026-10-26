# Ordered tasks — deliverability report contract

All unchecked items below are implementation work, not claims of completed
behavior. Test names are required witnesses to add/extend in the listed owner;
existing preservation suites are named where no new test is needed. Keep
inline tests measured unless 0020's concrete cold-path rule requires a harness.
The 2026-10-26 synthesis in spec.md resolves the current panel; these tasks
implement that contract, with no gate on the withdrawn MCP prerequisite.
T1 establishes the adapter boundary; T7 wires it; T10 proves both transports.

- [x] **T1 — Pin the current-main MCP contract (AC7).** Use main's existing
  2025-03-26 text_result/error_result and input admission owners. No dependency
  on 0079/0080/0082 or #152/#153 artifacts. **Proof:** T10's HTTP/stdio witnesses
  pin the current report shape and unchanged initialization. Record report
  structuredContent/outputSchema as a later, small follow-up when #153 lands,
  not as an unchecked requirement of this delivery.
- [x] **T2 — Implement typed contract and canonicalizer (AC1/AC2).** Add
  src/deliverability.rs and its module declaration; exact severity/location
  shapes, complete ordering/deduplication and size limits. **Proof:** inline
  `report_contract_variants_and_order` pins every severity, header absent/repeated
  coordinate, root/nested part path, link offset, tie-break and duplicate;
  `report_limits_refuse_without_truncation` exercises exact and over-limit
  finding/encoded-byte boundaries using bounded synthetic contributions.
- [x] **T3 — Implement only the artificial demo (AC1/AC2).** Bounded raw header
  scanner, fixed rule id/hint/version, immutable demo_enabled config. **Proof:**
  inline `demo_marker_uses_first_raw_header` covers absence, all three tokens,
  repeated and folded fields, empty/unknown/case-mismatched values, invalid name,
  orphan/terminated continuation, LF/CRLF/EOF and non-UTF-8; plus
  `demo_header_budget_is_exact` and `report_ignores_arrival_metadata` pin full
  header limits, disabled behavior and byte-identical results across instances.
- [x] **T4 — Add the bounded Store-owned cache (AC2/AC4).** Weak byte-object
  identity, value config/version keys, 128 slots, pinned-aware oldest eviction;
  never add Email cache fields or mutation/restore invalidation callbacks.
  **Proof:** owner tests `cache_identity_covers_bytes_config_rules`,
  `cache_rejects_clone_reinsert_and_recovered_owner`,
  `cache_eviction_never_retrieves_removed_mail` and
  `cache_weak_entries_do_not_retain_raw_mail` cover cold/warm use, independent
  version/config changes, same-id replacement, Arc ownership escape, all slot
  boundaries, delete/clear/prune and allowed pre-removal snapshot completion.
- [x] **T5 — Bound offload and failure ownership (AC5).** Serialize same-key
  misses, reserve four permits before submission, move permit/pin/owned guard
  into worker; no cached failures or custom panic conversion. **Proof:**
  owner-local barrier tests `coalesced_miss_survives_caller_cancel`,
  `report_capacity_refuses_and_recovers`, `report_panic_releases_owned_capacity`
  and `report_failure_retries_without_clean_success`. Test distinct keys and
  stores, caller cancellation before/after submission, all-pinned cache refusal,
  worker failure/unwind and warm reuse. Add a cold-path harness only if needed.
- [x] **T6 — Prove lazy execution and storage preservation (AC3/AC4/AC5).** Use
  a per-service test-only evaluation counter/gate; do not add runtime probes to
  SMTP. **Proof:** owner-local real-server
  `report_is_lazy_and_ingest_fields_unchanged` proves zero evaluations through
  SMTP DATA 250, seed and persistent restore, immediate exact /raw reads, then
  first evaluation and reuse; `report_workers_do_not_hold_store_guards` holds
  four workers while another real SMTP transaction commits and is queryable.
  `report_restart_recomputes_same_bytes` in tests/persistence.rs compares stored
  fields/rows before and after report/restart. Run existing burst, p2, rate,
  persistence and chaos/commit-refusal tests unchanged as preservation proofs.
- [x] **T7 — Expose REST and legacy MCP reports (AC6/AC7).** Add the report GET,
  closed empty query domain, one required-id tool and fixed errors. **Proof:**
  tests/api.rs `report_rest_schema_and_refusals` and tests/mcp.rs
  `report_legacy_mcp_matches_rest` use real servers to compare reports, validate
  required/unknown/null/wrong-type arguments, missing/deleted/pruned ids and
  existing guard precedence. Owner seams prove invalid requests evaluate zero
  reports and saturation/failure returns the documented 503/isError shapes.
  tests/stdio.rs `report_legacy_stdio_matches_http` checks the real bridge.
- [x] **T8 — Add shared assert/wait qualification (AC9).** Freeze inputs, qualify
  owned snapshots outside guards, keep the original unqualified path and one
  subscribe/scan loop. **Proof:** tests/api.rs
  `report_threshold_exact_count_and_min_one` and tests/mcp.rs
  `report_wait_thresholds_and_validation` cover all nine severity/threshold
  combinations, empty reports, mixed clean/dirty inboxes, exact zero/mismatch,
  empty-inbox wait, count=0, newest-first output, filter-before-page and retained
  totals/caps; prove option refusal on unrelated list/count surfaces.
- [x] **T9 — Prove wait interleavings and truthful deadlines (AC5/AC9).** Extend
  existing owner-local seams; never synchronize by sleeping. **Proof:**
  `report_wait_subscribes_before_analysis`,
  `report_wait_observes_delivery_during_analysis`,
  `report_wait_resubscribes_after_clear`, `report_wait_recovers_lag` and
  `report_wait_deadline_and_shutdown` deterministically cover initial and Closed
  subscription windows, completion without another delivery, pinned-config
  stability, cancellation during a slot wait/worker, exact zero-timeout policy,
  complete-scan timeout and incomplete-scan unavailable. Cover warm, empty, cold
  and busy zero-timeout scans and prove that zero timeout submits no cold work. Real REST/MCP shutdown
  tests pin existing error shapes. `report_failed_scan_is_not_exact_or_clean`
  proves an assert never reports a partial successful count or subscribes for
  future mail.
- [x] **T10 — Prove current-main MCP report results (AC7).** Use the existing
  2025-03-26 result helpers and tool descriptor. **Proof:**
  `report_mcp_text_contract` in tests/mcp.rs and `report_stdio_text_contract` in
  tests/stdio.rs prove REST parity for empty/nonempty findings, one JSON text
  block, isError=false on success, existing operational errors, -32602 invalid
  inputs, closed required-id admission and unchanged initialization behavior.
  Assert the report has no structuredContent and its descriptor no outputSchema.
  Document the small follow-up when #153 lands; no negotiator is added here.
- [x] **T11 — Render the message report (AC8).** Existing server-rendered page,
  same evaluator, escaped textual locations/hints, explicit empty/failure state;
  no inbox-list warming. **Proof:** tests/ui.rs `report_ui_states_and_escaping`
  checks real pages against REST content and original fields; owner fixture
  rendering injects adversarial strings for every field and confirms escaping,
  no active link construction, message 200 with failed section and missing 404.
- [x] **T12 — Prove and document the #253 seam (AC10).** Static collector plus
  immutable typed effective-input key; no DNS/resolver/plugin framework.
  **Proof:** owner test `report_provider_snapshot_is_deterministic` adds a
  fixture rule with MIME/link locations, verifies canonical merging,
  serialization and threshold reuse, and changes its explicit input identity
  to prove no stale reuse. Production ships only demo.marker.v1.
- [x] **T13 — Update living contract documentation (AC6/AC10).** OpenAPI,
  llms.txt and README; verify the existing proposed 0085 index row; document snapshot/failure bounds,
  synthetic rule, empty-report meaning and exact-count predicate example.
  **Proof:** `report_served_docs_match_contract` fetches actual OpenAPI/llms from
  a real server and checks independent expected keys, variants, enums, statuses
  and examples; existing `every_decision_file_has_exactly_one_resolving_index_row`
  and `every_index_row_matches_its_records_status` pass without accepting 0085.
- [x] **T14 — Commit and verify the candidate (AC10).** Run focused witnesses,
  all house gates offline, engine/shim hashes, exact coverage and selftest,
  publish/seal proofs, licenses, native-tls absence, design selftest and delivery
  proof. **Proof:** fresh per-command logs/exits/hashes bound to committed tree;
  exact zero missed lines/branches/functions; no inherited receipt substituted.
  Record CI/operator-owned proofs separately and use the candidate's unchanged
  routing/review. No push, merge, tag, publication, issue closure or acceptance.


Implementation witness placement: the wire witnesses for T7/T8/T10/T11/T13
are grouped in `tests/deliverability.rs`, with stdio in `tests/stdio.rs`.
Owner tests combine related ordering/lifetime properties under descriptive
`report_*` names; the spec's implementation record names their homes. T14
is proved by the fresh committed-candidate receipt linked below.


All fourteen local checks passed on implementation commit `e8828fe`, including
exact coverage at **15,520/15,520 lines, 876/876 branches and 1,656/1,656 functions**
and its independent selftest. The [implementation gate receipt](../../docs/delivery/issue252-implementation-gates.json)
binds commands, exits and log hashes to that committed candidate. This final
receipt/checklist commit changes no production, dependency, gate or bundle bytes.
CI/operator-owned proofs remain as listed in spec.md. 0085 remains proposed;
report structuredContent/outputSchema follows when #153 lands.
