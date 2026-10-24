# Ordered tasks: record the metric amendment

Implementation obligations for [spec.md](spec.md), not completed design work.
The baseline and allowed paths are in [plan.md](plan.md). Each task remains
unchecked until its whole paired proof passes.

- [x] T1 — Capture baseline clauses, historical bodies and old validation receipt; confirm the supplied acceptance/reservations and bounded path inventory. Test: `git show 59a37f2fadaf2e0faafbdf581ae91db4d72e8bbc:<path>` supplies each comparison fixture; current status/diff confirms no unrelated edits, and the proof checklist covers AC1–AC6.
- [x] T2 — Rename only the decoded-inbox amendment to 0065; correct title/status/numbering and add dated operator-acceptance provenance, original proposed status and historical-language clarification. Test: old file absent, one 0065 file with exact status/date/reservations, verbatim equality of all five ruling clauses and preserved alternatives/proof text; manual provenance review (AC1, AC2).
- [x] T3 — Add exactly one resolving accepted 0065 index row without altering other rows. Test: `cargo test --locked --test store_contract_docs` passes inventory, status agreement and existing mutation controls; inspect row date/scope and absence of an old amendment row (AC3).
- [x] T4 — Insert the dated accepted-0065 marker immediately after 0043 clause 6 and scoped supersession links at HTTP-plane spec Scope boundaries and plan AC3. Test: removing just the new marker blocks yields byte-for-byte baseline bodies for all three files; relative targets resolve and review confirms strict once-only/fallback scope, preserved 0047 markers and unchanged refusals (AC4).
- [x] T5 — Repair the amendment's own spec/plan/tasks current identity and links; add dated acceptance clarifications wherever historical prerequisites would otherwise govern. Test: affected-link resolution and classified old-number search pass; old validation receipt remains byte-for-byte identical; all runtime and mixed unproved tasks stay unchecked and no implementation/test success is asserted (AC5).
- [x] T6 — Review the complete documentation diff and exact historical comparisons. Test: `git diff --check` passes; path inventory matches the eight logical documents plus required design artifacts only; reserved-lane decisions, other proposals, unrelated 0017 reference, source/tests/dependencies/gates/recipes are unchanged; review all five clauses and marker scope separately from automated tests (AC1–AC6).
- [x] T7 — Run the targeted documentation suite and every current house-rule gate using pinned tooling, exact coverage and its selftest, publish/seal proofs and fail-closed native-tls absence. Test: each required local command exits successfully with zero missed coverage; retain fresh logs/receipt in this design, preserve historical receipts and leave CI-only obligations explicitly with CI (AC7).
- [ ] T8 — Submit the bounded change to the engine-owned independent four-seat review: Qwen Max correctness, Claude security, official DeepSeek Flash guarantees and GLM 5.3 tests, with no Pro/fallback. Test: actual journal review evidence covers provenance, numbering, semantics, historical preservation and gate proofs; protected review/security hold remain enforced. Commit unsigned and perform no push, merge, issue close or further acceptance (AC6, AC7).


## Recovery route clarification — 2026-10-24

The earlier T8 sentence describes the original acceptance-recording route.
For this recovery's T8, follow the task-selected four review lenses: Qwen Max
correctness, Claude security, official DeepSeek Flash guarantees (never Pro),
and GLM 5.3 tests with Haiku allowed only as a dispatch-failure fallback.
That fallback cannot replace a substantive verdict. Keep T8 open until actual
independent results and security disposition are recorded; dispatch alone is
not completion.
