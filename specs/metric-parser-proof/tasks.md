# Tasks: metric parser proof

Ordered downstream tasks under [spec.md](spec.md) and [plan.md](plan.md).

## Operator clarification — 2026-10-24

Replace former upstream-parser discovery/execution tasks with the in-repo
proof below. No upstream tooling prerequisite remains. All boxes are open:
this design neither implements the new matrix nor supplies protected review.
Preserve historical receipts and true/low residual facts. The implementation
receipt belongs in the dated owning decoded-inbox plan; reference it here.

- [ ] T1 — Extend independent strict-reader fixtures in `tests/api.rs` (AC1–AC3). **Test:** the existing reader-control test accepts exact escape/control/C1/bidi bytes, distinguishes backslash-n from LF and rejects illegal escapes, raw LF, unterminated quotes, missing delimiters and trailing malformed syntax. Expected strings are independent of production encoding/decoding.
- [ ] T2 — Add the finite sequential real-request matrix to `src/api/refusal_tests.rs`, reusing the hold and locally asserting complete byte-preserving HTTP framing (AC1–AC4). **Test:** `cargo test --locked --lib api::refusal_tests` observes each response-produced held row, independently specified exact exposition bytes, full strict parse, value 1 and byte-exact identity; all individual C1/bidi and mixed fixtures execute.
- [ ] T3 — Prove attribution, admission, cleanup and parallel safety (AC2/AC4/AC6). **Test:** before/held/released observations show no encoded alias or extra fixture-bearing row, unchanged 400/body/no-await behavior for controls, successful admitted count responses, joined completion and request-row removal. Preserve hold drop/cancellation tests, audit per-executable cumulative headroom below 4096, and pass normal parallel library/API suites without resets or sleeps.
- [ ] T4 — Demonstrate the new proof is sensitive to wrong bytes (AC3). **Test:** one temporary wrong expected escape/identity or extra attributed sample fails an explicit assertion in the targeted test, followed by restored-green; record command, exit and diagnostic. Restore all mutations before candidate verification; timeout/build failure does not qualify.
- [ ] T5 — Retain original issue and accepted-contract proofs (AC5/AC6). **Test:** locked API/library tests prove two parked space/non-ASCII awaits with six exact same-scrape rows, alias absence, seeded successful joins, live cleanup/cumulative retention, whole-raw invalid/mixed UTF-8 fallback and unchanged extractor refusals; existing plus/slash/double-encoding fixtures remain green. Review unchanged production/frozen boundaries.
- [ ] T6 — Record the implementation receipt and reconcile current dependent guidance with the operator replacement (AC7). **Test:** dated owning receipt identifies candidate, independent matrix, headroom audit and assertion-red/restored-green evidence; relative links and `cargo test --locked --test store_contract_docs` pass. Preserve old receipts and finite pointer-audit limitation; no current upstream-tool prerequisite remains.
- [ ] T7 — Commit the implementation candidate unsigned and run every local gate (AC7). **Test:** final-candidate commands exit zero for fmt, locked all-target Clippy/workspace tests, digest-bound candidate-local bundle compile, exact zero-missed coverage plus selftest, publish/seal proofs, licenses, shim integrity and fail-closed native-tls absence. Run targeted suites and delivery-recipes regression; preserve actual failures and CI-owned distinctions. No dependency, accepted-decision, gate, floor or routing diff.
- [ ] T8 — Hand the verified candidate to the existing protected route. **Test:** the engine obtains five distinct candidate-bound results (Astra correctness, official Flash security, Sonnet tests, Astra guarantees, independent Fable) with policy-admissible disposition. Author tests/design cannot clear true/low or unknown facts. No push, merge, tag, publication, issue closure or decision acceptance.
