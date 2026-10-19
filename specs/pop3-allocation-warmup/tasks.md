# Ordered tasks — POP3 allocation warm-up

Implementation tasks follow the committed [spec](spec.md) and [plan](plan.md).
Unchecked items are obligations, not claimed results. Receipts belong under
gitignored `.brokkr/state/epic120/` and identify the tested candidate.

- [x] T1 — Prepare the actual control barrier with one completed disarmed
  round on both participants; retain the two measured rounds and megabyte
  worker allocation (AC1). **Test:** run the exact named counter test and
  confirm its unchanged full 1000-byte tally and disarmed-afterward check.
- [x] T2 — Warm only the fixture Store's empty snapshot gate before its
  first snapshot; document the per-instance setup and absence of production
  rehearsal (AC2, AC4). **Test:** run the exact named snapshot test with its
  eleven-request pin, largest-request bound and complete order/size oracle;
  inspect the diff for unchanged seam location and assertions.
- [x] T3 — Add the local OnceLock cold/warm proof using real `observe` and
  observable retained buffers (AC3). **Test:** cold tally is exactly
  `{1,64,64,64,64}`; warm scope including the explicit 1000-byte allocation
  is exactly `{1,1000,1000,1000,1000}` in Tally field order, on Linux.
- [x] T4 — Execute the entire allocation harness in normal parallel and
  single-thread modes (AC4). **Test:** all listed tests execute and pass,
  including exact RETR construction, zero-allocation sizing, real-session
  bytes/memory and panic disarming; unchanged pins are visible in the diff.
- [x] T5 — Disable request accounting temporarily, save patch and receipt,
  then restore (AC5). **Test:** the exact original counter test executes
  once and fails its tally assertion; the restored same command passes.
- [x] T6 — Add an observable per-call allocation to real `payload_len`,
  save patch and receipts, then restore (AC5). **Test:** the exact original
  snapshot test fails its eleven-request pin and the sizing test fails
  ZERO; both restored commands pass. Compile failures do not count.
- [x] T7 — Delete only the new warm fixture's pre-arm initialization,
  save patch and receipt, then restore (AC3, AC5). **Test:** the mechanism
  test fails at `{2,1064,1000,1064,1064}` versus the warm pin and passes
  again after restoration. Identify the existing second-RETR-buffer mutant
  receipt, or rerun and restore that mutant to prove the RETR test fails.
- [x] T8 — Run the final restored-tree house inventory from plan.md (AC6).
  **Test:** formatting write then check, locked workspace clippy/tests,
  coverage with zero missed lines/branches/functions, coverage selftest,
  publish-guard proof, license check and pinned brokkr compile all pass;
  pinned verify also checks shims and absence of native-tls. Record any
  unavailable execution as outstanding, never successful.
- [ ] T9 — Append signed implementation commits and leave the checked-out
  branch clean (AC7). **Test:** `git verify-commit` succeeds for each new
  commit, `6d843d2` remains an ancestor, staged/committed paths match the
  authorized scope, and `git status --porcelain` is empty. No push or
  remote mutation is performed; signing failure blocks delivery.
- [ ] T10 — Collect the existing `test (macos-latest)` result through the
  operator's assistant after it pushes the candidate (AC1, AC2, AC7).
  **Test:** that candidate's CI job passes both original exact tests and
  the complete harness. Until the receipt exists, record macOS as pending;
  Linux preparation controls do not satisfy this platform obligation.

## Implementation receipts (Linux, 2026-10-19)

Under `.brokkr/state/epic120/macfix/`. All four mutants were applied one at
a time in a detached worktree carrying the candidate, then restored, and
the restored worktree's diff matched the candidate byte for byte. Each
exact test ran once and failed on its named assertion (exit 101):

| Mutant | Test | Failure |
|---|---|---|
| `m1` request count disabled | counter control | `requests: 0` vs 1 |
| `m2` 8-byte allocation in `payload_len` | snapshot; sizing | 19 vs 11; `{8,64,8,0,8}` vs ZERO |
| `m3` warm preparation deleted | lazy-object proof | `{2,1064,1000,1064,1064}` vs `{1,1000,…}` |
| `m4` `raw.to_vec()` in `retr_reply` | RETR tally; real session | `{18,…,peak 2424853}` vs 17; peak bound |

The harness passes in parallel and `--test-threads=1` modes. T9 is done by
the commits that carry this file, and T10 stays pending until the
operator's assistant pushes the candidate to CI.
