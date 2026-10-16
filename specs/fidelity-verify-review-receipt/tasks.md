# Tasks — the record's completion at `47fa0e2`

Ordered for this pass and the phases that follow. Every task pairs with
the check or test that proves it; a task that cannot be proved is not a
task. This pass runs no production code; the proving tests are the ones
the tree already carries, plus deterministic inspections of the committed
record. Run at the repository root with `.brokkr/state/env.sh` sourced.

## This pass — the amendment set

- [ ] 1. Verify the base first-hand before editing anything: HEAD
  `47fa0e2`, clean tree, the recorded count.
  **Proved by:** `git status --porcelain` (empty) and `git log --oneline -1`
  (`47fa0e2`); `cargo test --workspace --locked` — **470 passed, 0
  failed, 1 ignored** (the opt-in `tests/rate.rs` smoke) (spec.md V1).
- [ ] 2. Apply **A1** — the receipt of record's cap-race bullet states
  accept-and-evict (0003) and drops the `552`-tail claim.
  **Proved by:** the bullet at the cap-race anchor in
  `specs/fidelity-prove-ship-receipt/spec.md` reads "accepts and evicts
  oldest … the store never refuses on count"; `grep -n "552"
  specs/fidelity-prove-ship-receipt/spec.md` shows only the DATA
  allowance and the commit-refusal mentions (spec.md AC-A.1).
- [ ] 3. Apply **A2** — the hostile-DATA bullet names the message-byte
  allowance (`552 5.3.4` at the terminator) and `MAX_COMMAND_LINE = 4096`
  as the command-line cap answered `500 5.5.2`.
  **Proved by:** the bullet at the hostile-DATA anchor;
  `grep -n "MAX_COMMAND_LINE\|5.5.2 Line too long\|5.3.4" src/smtp.rs`
  matches the sentence (spec.md AC-A.2).
- [ ] 4. Apply **A3** — the receipt's #56 pin clause says "derive per
  view request (0018 §5)" with the fixed precedence and the recorded
  full-folding limitation.
  **Proved by:** the clause text; `grep -rn "fold at ingest" specs/ src/
  docs/` is empty across the tree (spec.md AC-A.3).
- [ ] 5. Apply **A4** — tighten the count discipline: the envelope leg in
  `tests/compat.rs` asserts `total == 1` and `items.len() == 1`, and the
  design of record's AC-55.2 is aligned to `total == 1`.
  **Proved by:** `cargo test --test compat
  an_envelope_only_recipient_surfaces_in_every_compat_shape` green on the
  tightened assertions; the criterion text at
  `specs/fidelity-closures/spec.md` (AC-55.2) (spec.md AC-A.4).
- [ ] 6. Apply **A5** — pin the guard clause in
  `the_live_shell_guards_paints_with_a_token`:
  `script.contains("seen !== paintSeq")`.
  **Proved by:** `cargo test --test ui
  the_live_shell_guards_paints_with_a_token` green; red-able by deleting
  the `seen !== paintSeq` early return at `src/static/ui.js:81`
  (spec.md AC-A.5).
- [ ] 7. Confirm the completion diff is exactly the amendment set before
  committing: the two test files, the two spec files — nothing else, and
  no `src/` file.
  **Proved by:** `git diff --stat` names exactly
  `specs/fidelity-prove-ship-receipt/spec.md`,
  `specs/fidelity-closures/spec.md`, `tests/compat.rs`, `tests/ui.rs`
  (spec.md AC-A.6).

## The gates — all six, in the standing form

- [ ] 8. `cargo fmt --all -- --check` and `cargo clippy --workspace
  --all-targets -- -D warnings` — clean on the edited tree.
  **Proved by:** both commands exit 0; the two test edits introduce no
  formatting or lint divergence.
- [ ] 9. `cargo test --workspace --locked` — **470 passed, 0 failed,
  1 ignored**; a different total needs explanation, not silent
  acceptance (V2).
  **Proved by:** the run's summary line; the count is unchanged because
  both edits are assertions inside existing tests.
- [ ] 10. Re-prove the per-issue criteria on the committed tree (V3,
  real servers, no mocks).
  **Proved by:** `cargo test --test api` and `cargo test --lib extract`
  (#39); `cargo test --test persistence --test pop3 --lib persist`
  (#54); `cargo test --test compat` (#55); `cargo test --test ui --lib
  threads` (#56) — all green.
- [ ] 11. `brokkr compile --bundle bundles/self` — the constitution
  compiles against the amended record.
  **Proved by:** exit 0.
- [ ] 12. `TMPDIR=/home/valentin/scratch/cov-tmp bash
  scripts/coverage-gate.sh` — exact 100/100/100, missed == 0. The
  `TMPDIR` is mandatory: never run the disk-bound gate on `/tmp` tmpfs.
  **Proved by:** the gate's own verdict; a red here is an environment
  problem to diagnose, not a code problem (plan.md, Risks).
- [ ] 13. `cargo deny check licenses` — the permissive-only tree stands.
  **Proved by:** exit 0; no dependency was added.
- [ ] 14. Frozen-guarantee inspection (V4): no await in the
  insert/persist path; eviction still oldest-only; 0002 untouched.
  **Proved by:** code inspection of the persist path (`src/store.rs`,
  `src/persist.rs` — `insert` synchronous, `pop_front` eviction,
  `wait_for` subscribe-before-scan) plus the suites in task 10 passing.

## The phases that remain

- [ ] 15. Review rules on a record whose sentences match the code:
  findings are repaired only from a red criterion or a review finding;
  the recorded residuals (`html_to_text` on the async workers, the 0033
  hole, expired-supplied-cert visibility) are accepted with named
  follow-ups, not new work. **Proved by:** review's own verdict; the
  residuals' record sentences (spec.md §Named residuals).
- [ ] 16. Ship: the batch plus this receipt's completion commit, history
  append-only on `main`, never pushed from a seat.
  **Proved by:** the phase machine's ship step; `git log` shows the
  completion commit directly on `47fa0e2` with the amendment set as its
  whole diff (AC-A.6).
