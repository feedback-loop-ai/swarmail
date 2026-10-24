# Ordered tasks: metric-budget-witnesses

## Receipt basis

The checked statuses below record evidence from the previously completed
issue #178 implementation, not fresh proof from this documentation follow-up.
Triage's read-only inspection of the canonical journal for
`epic-192-continuation-for-issue--315fe237` records a verify pass (seq 9),
the four-lens residual review (seq 505/506), and shipped close-out bound to
`f0e2a4d` (seq 522; terminal seq 524). The final correctness review records
T1–T10 as supported and T11 as pending, and points to implementation and
mutation receipts in run `epic-192-next-178-luna-xhigh-del-5b30174f`,
effect `aa865b12`. Those receipts are tied to source
`f0e2a4de567da4a91f5aee736c7d9202ff034310`.

This follow-up's preflight confirms that the candidate HEAD is that source,
its merge base is `origin/main` at `45d68df37044de3360439b34c39687975dd59b84`,
and `.forge/bin/brokkr` matches the digest in `bundles/self/tooling.json`.
The canonical journal is not mounted in this boxed checkout, so these are
inherited receipt references summarized by triage; this document does not
claim new journal events or a new review. The full house gates are still
rerun on this follow-up's final documentation diff. Nothing here claims
issue #178 or epic #192 is complete.

- [x] T1 — Verify the candidate-local Brokkr digest, current ancestry, candidate-owned target, and registration-label exclusivity. **Proof:** the implementation receipts and final correctness review above support the completed preflight and label audit. This follow-up confirmed the candidate-local digest and ancestry. The current candidate is based on `45d68df` plus pin commit `238372d`; the older `9b15fda`/`450bef65` lineage is historical.
- [x] T2 — Give REST nocap and the malformed-timeout setup measured liveness controls and strict later observations (AC1–AC2). **Proof:** `the_wait_refuses_over_the_cap_registers_no_waiter_and_accepts_the_boundary` and `await_supplied_malformed_timeouts_are_parse_refusals_and_zero_is_a_real_deadline` pass with +1 setup and +0 refusals while existing response/boundary assertions remain; covered by the implementation receipts and final review above.
- [x] T3 — Give MCP nowaiter/capbox the same protocol-local controls; fix `next177_establish_registration` and observed408 baselines (AC1–AC2). **Proof:** `the_wait_registers_no_waiter_when_staging_refuses`, `the_staging_refuses_over_the_cap_and_honors_the_boundary`, `wait_results_carry_the_post_wait_total_in_both_arms` and every local registration-helper caller pass; retain found/readiness/active-cleanup assertions; covered by the implementation receipts and final review above.
- [x] T4 — Add immediate independent singleton storage identities (AC3). **Proof:** `failing_target_is_retried_four_times_not_more`, `untrusted_certificate_fails_the_handshake_and_is_retried` and `non_2xx_over_tls_is_retried_four_times_not_more` read the correct recipient/subject/text and exact one mail before exhaustion; HTTP filtered mail is independently queryable in its other inbox; four-attempt assertions still pass; covered by the implementation receipts and final review above.
- [x] T5 — Index the accepted batch fixtures and read each complete set immediately after its burst (AC3). **Proof:** `the_delivery_bound_holds_exactly_and_the_excess_is_counted` returns all 50 identities, and `a_stalled_target_cannot_starve_a_healthy_one` returns all cap+8 identities with unique ids, while their exact drop/progress/wire assertions still pass. Existing stalled/refused/redirect storage witnesses remain green; covered by the implementation receipts and final review above.
- [x] T6 — Remove count-channel tuple plumbing and `hits()`, update every caller/comment, and retain terminal observations (AC5). **Proof:** `a_window_closing_before_any_attempt_observes_zero` observes capture cancellation before zero for all three sink types; HTTP/TLS retry tests observe new exhaustion before four; complete webhook suite preserves capture, trust, drop-first, credentials and redirect no-leak behavior; covered by the implementation receipts and final review above.
- [x] T7 — Compare equivalent URL spellings and a distinct target in the inline key test (AC4). **Proof:** `cargo test --locked --lib sink_keys_normalize_and_fall_back` passes equality/inequality and retained Url/Raw checks; exact coverage executes all new measured assertions; covered by the implementation receipts and final review above.
- [x] T8 — Demonstrate counter and storage falsifiability (AC6). **Proof:** missing/renamed registration instrumentation fails the intended liveness assertion separately in REST nocap and MCP nowaiter/capbox; omitting one accepted fixture fails the intended singleton and batch readback assertions. Save red command/source receipts, restore each mutation and rerun those same tests green; covered by the implementation mutation receipts and final review above.
- [x] T9 — Demonstrate normalization and retry falsifiability (AC6). **Proof:** raw valid-URL keys and constant valid keys each fail the intended key assertion; a fifth attempt fails HTTP, untrusted TLS and trusted TLS budget counts after exhaustion. Preserve listener lifetime, restore each mutation and rerun green; compile failure or unrelated timeout is not the required red; covered by the implementation mutation receipts and final review above.
- [x] T10 — Verify the complete restored candidate (AC6). **Proof:** full api/mcp/webhook suites and every gate in docs/house-rules.md pass with locked builds, candidate-owned target, exact coverage plus selftest, publish/seal proofs, licenses and fail-closed native-tls absence; pinned compile and actual hermetic verification bind to the final source; supported by the implementation receipts and final correctness review above. The follow-up reruns all local house gates after these documentation edits.
- [ ] T11 — Complete the remaining protected delivery route and record external evidence boundaries (AC6). **Proof:** this documentation follow-up requires one independent official DeepSeek Flash review under 0057; never DeepSeek Pro or a fallback. External CI evidence remains pending for the chart contract/history/bump selftest, real-Docker persistence/context/volume smoke, network RustSec audit, and real port-80 bridge exchange. Record tag-only release-rate-5k proof only if publication applies; publication is unauthorized here. Keep this open until the pending review and CI evidence are recorded; do not claim #178 or epic #192 completion.
