# Ordered continuation tasks — eval gates

Candidate `26df727` already implements most of the inherited contract. These
boxes track continuation validation and repair, not an assertion that the
original implementation is absent. Mark a task done only with its named
proof; inherited handoff prose and this design check alone are insufficient.
Keep original receipts and dated clarification history. Use unsigned commits;
no push, merge, issue close or decision acceptance.

- [x] **T1 — Reconcile candidate and receipts (AC8–AC11).** Record HEAD/worktree state; map implemented owners and inherited timeout/success/dispatch receipts to acceptance criteria, candidate/diff and pin identity. **Test:** inspect actual logs/results and diagnostics, identify missing or invalidated proofs explicitly, and confirm concurrent worktree content was neither reset nor imported.
- [x] **T2 — Bound the retained-server regression (AC6–AC7).** Replace blocking FIFO rendezvous and unbounded child wait locally; start the progress deadline before rendezvous, preserve the 131072-byte output crossing with retained `Proc`, and perform bounded cleanup before assertions. **Test:** isolated stalled-drainer control emits the intended deadline failure and reaps children/completes tasks within the declared budget without outer termination; restored normal-drain control passes with final marker and log bound. Keep failure diagnostics and child cleanup evidence.
- [x] **T3 — Revalidate readiness, boundaries and harness truth (AC3–AC7).** Run existing runner framing/receiver/EOF/error/agent/descendant controls, lock tuple controls, real fixture MCP shape and private decoder refusals; fix only uncovered or failing requirements. **Test:** exact/oversized/forged-suffix and retained-server cases pass; foreign tuple controls refuse; tolerated extras remain valid; full eval suite using the successful root-test artifact yields 16 oracle passes, 16 declared null failures and two overlapping isolation passes, with lifecycle/scoring precedence intact.
- [x] **T4 — Prove gate execution and binary provenance (AC1–AC2, AC8).** Preserve current CI/pinned verify selectors and explicit wiring; rerun missing or invalidated outer eval-dispatch and selection controls using isolated copies. Repair existing recording/wiring only for proven gaps. **Test:** actual pinned fmt/clippy/test mutants and CI-equivalent checks fail at intended gates, restored dispatch passes, failure JSON is checked even with process exit zero, custom target with spaces/decoy succeeds, and failed-root/missing/ambiguous/wrong-package/wrong-target/harness-only/relative/non-executable artifacts refuse. Record candidate/diff, pin, command, exit/result and decisive logs; preserve shim/engine refusals.
- [x] **T5 — Check authority, inventory and fixture guidance (AC9–AC11).** Reconcile any remaining current contradictions in both older specs/dependent plans/tasks, README and eval manifest; preserve accepted 0055's historical body and genuine limits. **Test:** focused text/link review finds current accepted-0055 and wired-gate guidance, sole inventory is accurate, fixture source exists and CLI exposes `fixture`, dated history remains, and unavailable external #149 wording has an explicit canonical-path operator handoff without an external-edit claim.
- [x] **T6 — Validate the final candidate (AC1–AC11).** Run the full eval suite with the successful root-test artifact, every house gate and required pinned selftest/tamper/identity/dispatch proof; compile affected inherited recipes with the recorded engine. **Test:** passing final-candidate receipts cover the complete linked house inventory, exact coverage and selftest, publish/seal proofs, license/native-TLS boundary and delivery proofs. Missing/failed/environment-blocked evidence remains outstanding; local passes do not claim CI-only proofs.
- [x] **T7 — Prepare protected review handoff (AC8–AC11).** Record scoped changes, criterion-to-proof evidence, remaining external #149 follow-up and any unresolved facts; update task status from evidence and append unsigned implementation commits with rationale. **Test:** diff preserves root contracts/dependency/coverage boundaries and concurrent work, all required gates are accounted for, no new acceptance or forbidden remote mutation occurred, and the full engine route retains protected review/security hold. Only the engine and subsequent seats rule their own outcomes.


## Implementation handoff — 2026-10-24

Implemented in unsigned commits 613c51d (bounded retained-server proof)
and 4668c96 (delivery proof isolation). The candidate is clean at
4668c96; no concurrent worktree was reset or imported.

- **T1:** Reconciled the inherited timeout and later success receipts against
  the source tree. The original eval-gates wiring, docs, one-shot readiness,
  lock/MCP controls and full suite were present; the only code gap was the
  unbounded retained-server regression control. The final verify receipt is
  bound to the same tracked tree as 4668c96.
- **T2:** The stalled and restored controls passed independently under a
  60-second outer timeout. The stalled control reports the five-second FIFO
  progress deadline, terminates/reaps the group, and completes both drain
  tasks under the ten-second cleanup budget. The restored control crosses
  the 131072-byte write, drains 16 MiB more output, reaches the final marker
  and retains no more than the 64-KiB log tail.
- **T3:** The final eval suite passed with the root-test artifact. The proof
  target records 16 oracle passes, 16 null failures in declared idle classes,
  two overlapping isolation passes, the real MCP text-block shape and the
  lock-subset controls. Existing scoring, lifecycle and child cleanup tests
  passed.
- **T4:** target/coverage/final-eval-dispatch/ contains the actual pinned
  verifier refusals and CI-equivalent fmt/clippy/test mutant failures, the
  restored full pinned verifier pass, and failed-root/missing/ambiguous/
  wrong-package/wrong-target/test-only/relative/non-executable artifact
  refusals. The clean dispatch used a target path containing spaces and
  overwrote /bin/false with the selected root-test binary. The clone was
  based on 613c51d with diff SHA-256
  674605f6edb2da43ec71cc9bf1c1965743117991d13b11e4fd1efdd383a9efea,
  exactly the tree committed as 4668c96. The integrated proof's
  single-command response exceeded the workspace's 300-second limit; its
  component outcomes were checked from their result JSON and decisive logs.
- **T5:** The older specs and README cite operator-accepted 0055 and the
  wired eval gates. README points to evals/src/fixture.rs and
  swarmail-evals fixture. The exact erroneous path in external #149 was
  not present in task materials; the canonical-path correction remains an
  operator follow-up. No external issue edit or decision acceptance occurred.
- **T6:** Root and eval formatting, locked Clippy and tests passed. Pinned
  verify passed offline with the canonical custom-target binary, exact
  100/100/100 coverage and selftest, bundle compile, delivery tamper proof,
  publish proof, seal proof, license check and fail-closed native-TLS check.
  CI-only chart, Docker, network advisory, release-tag and privileged-port
  proofs remain CI-owned and are not claimed locally.
- **T7:** This implementation and its receipts are ready for the protected
  review phase. Security hold and the four-reviewer policy remain in force.
  No push, merge or issue closure occurred.

Detailed local logs and JSON receipts are under the ignored
target/coverage/implementation-gates/, target/coverage/final-eval-dispatch/
and target/coverage/eval-dispatch-work.vFvNR1/ directories.
