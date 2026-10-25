> Historical #195 artifact restored on 2026-10-25. The current port and its
> proof obligations are governed by [webhook-ca-port-lows](../webhook-ca-port-lows/spec.md).
> Earlier stops and gate receipts below are dated history. 0067/0068 are
> accepted; 0069 was accepted for #220 on 2026-10-25 and is not implemented here.
> The low shared-blocking-pool availability residual is tracked by #220.

# Ordered completion tasks — webhook CA safety

The earlier implementation exists; these checkboxes track fresh completion
against [spec.md](spec.md) AC1–AC10 and the
[plan matrix](plan.md#clause-to-assertion-and-command-matrix). Old checked tasks
are historical implementation claims, not proof of these gaps. Every task is
paired with its proof; W/H/U/A abbreviations expand to commands in the matrix.
No new decision or 0069 implementation is authorized.

- [x] **T1 — Make privacy proof failures safe (AC1,2,8).** Remove response/raw-PEM/secret operands from assertion diagnostics and HTTP/JSON failure helpers before negative controls. **Test:** focused privacy/direct-install tests pass; deliberately failing assertion checks emit fixed case identifiers and no key-bearing values in captured output.
- [x] **T2 — Complete canonical and fail-closed wire cases (AC1,2).** Add repeated/noncanonically wrapped roots, unrelated blocks/prose; check exact canonical DER/order/duplicates, four fields/nulls, redaction and privately compared raw installation. Retain every unprojectable direct-input case and absent/null distinction plus synchronous projection. **Test:** W(private_keys_in_ca_pem_never_appear_in_get_or_put_responses), W(direct_webhook_install_projects_unloadable_ca_as_empty_on_get), U(a_redacted_target_keeps_only_validated_public_ca_certificates), admission/later-root tests in the matrix.
- [x] **T3 — Prove real API ownership and neutrality (AC3,5).** Exercise actual GET/PUT at validation/projection holds, old-state/progress, one successful swap/echo and observed cancellation of the handler owner. Add only necessary exercised cfg(test) observations. Drive real API calls in next-eviction neutrality proof. **Test:** extended H(webhook_api_preparation_is_off_worker_and_swaps_once), new H(cancelled_put_handler_cannot_install_after_preparation), H(api_preparation_preserves_lru_order_and_next_eviction), H(api_preparation_join_failures_are_value_free_and_leave_put_untouched).
- [x] **T4 — Prove exact delivery publication/accounting over protocols (AC4).** Add isolated SMTP/TLS/metrics phases for duplicate misses, full-cache distinct misses, touch/hit, eviction/rebuild and raw-text identity. Retain helper winner/bijection assertions; fix runtime only if these reveal accepted-contract failure. **Test:** new H(protocol_ca_cache_publication_and_accounting) with exact +2/0, +2/+2, +0/+0, +1/+1 build/eviction deltas as applicable, plus H(concurrent_cache_misses_recheck_publication_and_preserve_lru_bijection).
- [x] **T5 — Complete failure/cancellation evidence (AC5,9).** Pin fixed errors, retry/one terminal outcome and failed-build non-counting; observe successful CPU count after cancelled waiter before no-late-effects assertions. Preserve stop order/Store panic behavior. **Test:** existing join-failure, panic, cancelled-delivery and real shutdown tests named in the matrix; full binary/p2 regressions.
- [x] **T6 — Retain causal placement and finite pool evidence (AC6,7).** Record actual capacities/deadlines for held-A progress and >=33-root/eight-SMTP-session fan-out; preserve GET/delivery contention and cancellation experiments as #220 evidence. **Test:** all placement, fan-out and four pool-capacity tests in the matrix, immediate query after every tested 250 and watchdog-not-used assertions.
- [x] **T7 — Align living documentation and served contract (AC7,9).** Replace stale proposed-0062 references with accepted 0067/0068 where owned; retain canonical/null/raw-input, counter, distinct GET/PUT 500 and finite queue/build-time descriptions. Do not edit accepted records, routing or 0069. **Test:** A(openapi_webhook_get_documents_awaited_preparation_failure), live API failure proof, complete API contract suite and relative-link/status audit.
- [x] **T8 — Capture private discriminating evidence (AC8).** Collect independent local raw-echo, on-worker, under-mutex and GET-500-removal negative/restored-positive receipts only after T1; restore each before normal checks. **Test:** each focused regression fails for its intended fixed diagnostic, then passes on restored source; diff confirms no negative-control changes remain. Record commands/source identity/exit codes without fixture data.
- [ ] **T9 — Finish all gates and bind the candidate (AC1–9).** Run targeted suites, every authoritative house gate with digest-checked `.forge/bin/brokkr`, exact coverage/selftest, pinned shim/design/delivery proofs and fail-closed native-tls absence. Commit implementation unsigned before hermetic verification. **Test:** zero-exit receipts, zero missed lines/branches/functions, clean final diff and actual pinned verify result bound to committed candidate. CI-only checks remain explicitly CI-owned.
- [ ] **T10 — Obtain current protected review (AC10).** Hand off full disclosure-fix scope and clause receipts through unchanged accepted-0058 full-risk routing; retain #220 residual ownership and all security facts without self-clearance. **Test:** five separate current correctness/security/tests/guarantees/independent results on the verified candidate and policy-controlled disposition; no old-panel substitution. No push, merge, tag, publication, issue closure or decision acceptance.

Implementation evidence is mapped in plan.md's clause matrix and final handoff.
T1–T8 are proved locally; T9 includes downstream hermetic verification and T10
is protected review, so they remain unchecked until their owning seats run.
The implementer records local gates and commits its work; it does not fabricate
those downstream results or launch another delivery run. #220 owns the low
shared-pool residual; it does not hold #195 merely by existing.
