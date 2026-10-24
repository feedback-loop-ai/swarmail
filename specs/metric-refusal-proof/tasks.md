# Ordered tasks: metric refusal proof

The owning runtime tasks remain in [decoded-inbox-metrics](../decoded-inbox-metrics/tasks.md).
This sequence implements those outstanding proofs; completion here must link
to the same dated evidence rather than invent a second receipt. All tasks are
pending at design time, including independent review.

- [x] T1 — Reconcile current runtime instructions with this continuation; preserve accepted 0065, reservations, previous repair and dated receipts. Test: diff against `62e6496` shows decisions/index and historical recovery/recording artifacts unchanged; relative links and `cargo test --locked --test store_contract_docs` pass, with direct manual 0065-to-0043 review and its finite-audit limitation recorded (AC6).
- [x] T2 — Capture and pin real HTTP control, invalid-UTF-8 and mixed-invalid refusals before runtime edits; retain absence of raw/lossy/control await registration. Test: exact status/raw-body assertions pass with the original helper, then with the final helper (AC1; owning T2/T9).
- [x] T3 — Audit exclusive names and cumulative headroom in both affected executables; establish failure-safe bounded fixture ownership and independent parser CR/NUL fixtures, extracting only shared parser support if needed. Test: executable-wide registration upper bound proves room or isolated-process execution proves isolation; parser accepts exact controls and rejects retained malformed fixtures; normal parallel API/library runs pass without reset/serialization (AC5).
- [x] T4 — Add the two concurrent encoded-await wire regression and full seed/join/cleanup lifecycle. Test: with both active rows observed, old helper fails the explicit decoded HTTP-row assertion in one full parsed scrape; final helper passes all six value-1 rows, encoded-alias absence, matched==1 responses, live-row removal and cumulative retention (AC3; owning T3/T5).
- [x] T5 — Add the request-owned cfg(test) post-handler hold and owner-local real-HTTP refusal regression, placing only concrete cold diagnostics in a declared harness. Test: reached/release deterministically exposes the actual refused response's live decoded CR/NUL request row; full scrape has one exact value-1 row, no encoded alias/fixture injection, no await registrations, identical joined refusal and subsequent row removal. Exercise hold absence, installed release and any measured cancellation/drop arms; old helper fails the label assertion (AC4–AC5; owning T9).
- [x] T6 — Implement exactly-once strict decoding in the existing selected-segment helper and update its comment/inline matrix. Test: every AC2 case passes, including whole-raw mixed-invalid fallback and slash/double-encoding; both T4/T5 regressions turn green. Restore every mutant and retain exact old-helper red/restored-green commands/outcomes (owning T4).
- [x] T7 — Finish selector/collision guidance and owning evidence/task reconciliation, retaining historical receipts. Test: README selectors equal asserted decoded strings, no obsolete current raw-key/source-prohibition instruction remains, links/diff checks pass, and each completed checkbox maps to its entire executed proof; request activity is explicitly not admission/storage evidence (AC6; owning T6).
- [x] T8 — Run targeted API/library/document regressions, then every fresh local house-rule gate with pinned tooling/shim integrity and fail-closed native-tls absence. Test: commands exit zero, coverage reports zero missed lines/branches/functions and its selftest passes; publish/seal proofs and all remaining inventory gates pass; logs identify the actual candidate and CI-owned obligations remain pending with CI (AC7; owning T7/T8).
- [ ] T9 — Obtain the engine's full protected independent review of the final candidate and witnessed refused-control residual. Test: actual four-lens correctness/security/guarantees/tests results bind to that candidate, required rework and subsequent verification are resolved, and the security disposition satisfies the protected policy. Dispatch, old reviews or this drafted spec cannot check this task; no push, merge or issue close (AC7).


## Implementation receipt — 2026-10-24

T1–T8 and the owning runtime tasks T2–T9 are complete as recorded in the
decoded-inbox plan's continuation receipt. The final candidate preserves 0065
and its reservations. T9 above remains open: the engine must obtain the full
protected four-lens review and actual security disposition; no prior review or
test result is substituted for that phase.
