# Ordered tasks: portable SIGTERM delivery proof

These tasks belong to the future implementation and remain unchecked at design.
Each proof must name the actual candidate, platform, command and exit. Gate logs
and panel positions stay ignored; the design commit contains only spec.md,
plan.md and tasks.md. Independent verify/review and macOS CI remain separate.

- [ ] T1 — Confirm authority and scope (AC1–AC3, AC5–AC7). Proof: verify the packet's SHA256SUMS, record HEAD/platform/status, and compare protected files to `90eb2a0`; only the named binary fixture and the three design documents may change. The full workspace suite in T6 proves existing tests still execute.
- [ ] T2 — Replace the unconnected reservation inline (AC4/AC7). Proof: the exact focused command in plan.md selects one passing test using a reuse-disabled explicitly bound connected client and retained accepted peer; real refusal is exactly `ConnectionRefused` under `HELPER_DEADLINE`. Source review verifies local-address selection, bounded connect/accept and no keeper, half-close or ownership gap.
- [ ] T3 — Add the final executable ownership checks (AC4). Proof: the same focused test requires competing binds with reuse false and true each to fail with `AddrInUse`, before explicit final pair release. Verify no contender listens and both streams remain owned through the prior assertions.
- [ ] T4 — Preserve shutdown and diagnostic observations (AC4). Proof: the same focused test observes the stalled request and exhaustion before SIGTERM, successful EOF/reap, one final message with 0/0/0/1/1 totals, one substantive cause warning and absence of all three secrets. Inspect the diff for unchanged existing assertions, URLs, budgets and deadlines; no added ignore or relaxed failure.
- [ ] T5 — Exercise the complete binary suite and inspect scope (AC1–AC7). Proof: `cargo test --locked --test binary` exits zero; the focused invocation selected exactly one test. Compare other binary regions and all protected files against `90eb2a0`; no production, dependency, gate, routing or decision changes.
- [ ] T6 — Execute all local house proofs on final repair (AC8). Proof: every charter gate exits zero, including digest-verified candidate-local engine compile without sweep warnings, exact zero-missed coverage with completed exit, coverage selftest, publish/seal proofs and licenses. Verify shims, run pinned design selftest/delivery proof and strict native-tls absence per plan.md. Save current candidate-bound logs; historical/design passes do not satisfy this task.
- [ ] T7 — Commit and hand off the scoped repair (AC7/AC8). Proof: unsigned implementation commit, clean intended worktree and receipts for T1–T6. Preserve current recipe review/security hold. Record macOS CI and other external proof owners without claiming execution; no push, merge, tag, publication, issue closure or decision acceptance.

## Dated clarification — 2026-10-25, earlier 23.5 proof (historical)

Before portability re-entry, the owning tasks recorded completion of the
behavioural snapshot repair at `90eb2a0`: the identical focused scrape test
passed, rejected a compiling split-lock mutant at two real acquisitions versus
one, and passed after exact restoration. Its recorded coverage was
13,871/13,871 lines, 782/782 branches and 1,512/1,512 functions, with local
gates and native-tls absence passed. Receipts were recorded under
`.forge/epic192/issue191/implementation-235/`. This preserves the earlier
completion record, not a new audit of those receipts or proof of the macOS
fixture. The original checked task list remains in Git history. It is not
reopened; T1–T7 above govern only the new portability repair.
