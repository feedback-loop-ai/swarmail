# Tasks — the boundary stands, its failure contract is written down

Ordered; each task names the test that proves it. No task touches a
production behavior line — the only `src/` change in the whole run is a
comment. A red criterion at any step reopens the council (the intake
framing's rule); it is never fixed silently under this spec.

- [ ] 1. Add the selective refusing trigger to the injection vocabulary: one
  new kind in `tests/common/mod.rs` (`refusing_trigger`'s exhaustive match)
  that installs a `BEFORE DELETE … WHEN OLD.id = ?` `RAISE(ABORT)` trigger
  refusing exactly one id and allowing the rest, with the same `Drop`
  cleanup as the existing kinds. *Proved by:* the new leg in task 2 reaching
  its mid-loop refusal (an all-refusing trigger cannot produce it — the
  first delete would fail and hide the prefix).
- [ ] 2. Write the POP3 partial-prefix wire leg in `tests/pop3.rs`
  (`a_failed_quit_prefix_commits_and_the_retry_signs_off`): deliver two
  mails, learn the target id over REST, install the selective trigger,
  `DELE` both, `QUIT` → `-ERR storage commit failed`; the session stays in
  TRANSACTION (`LIST` still names the refused mail); the allowed id is gone
  live **and** after a fresh `Store::open` (prefix committed); the refused
  id is present on both trees; drop the trigger, retry `QUIT` in the same
  session → `+OK` sign-off; the restart copy holds exactly the committed
  set. One sequential session — no sleeps, no wall-clock assertions (0011
  rule 5). *Proved by:* the leg itself — red on `5b09843` (which swallowed
  and answered `+OK`), green on HEAD (AC3).
- [ ] 3. Correct the `src/pop3.rs` `QUIT` comment to the partial-prefix
  truth: the update applies marks one at a time, a mid-loop failure leaves
  the committed prefix committed and the failing suffix unmarked, the
  session stays in TRANSACTION, the retry converges — replacing "the update
  is abandoned whole: the marks are unapplied". Comment-only edit; zero
  behavior lines. *Proved by:* task 2's leg plus
  `a_failed_quit_answers_err_and_keeps_the_marks` staying green, and
  `bash scripts/coverage-gate.sh` still exact 100/100/100 (AC5, AC6).
- [ ] 4. Amend `docs/decisions/0012-commit-failure-refusal-and-the-mutation-boundary.md`
  in place: §5's `QUIT` clause states the at-least-once partial-prefix truth
  with an "(Amended by [0015](0015-boundary-failure-contract.md) …)"
  marker; §7 gains "restart is the recovery — the restart copy is always
  exactly the committed set" under the same marker; §Why's busy-handler
  bullet gains the refusal-contract sentence (the ~5 s bound is rusqlite's
  default at open — `sqlite3_busy_timeout(db, 5000)` — never overridden in
  this crate, and deliberately not wall-clock-asserted). Status stays
  `proposed`; no other section changes. *Proved by:* review reading the
  diff against spec AC5 — today's "its marks unapplied" claim is the red
  state.
- [ ] 5. Land `docs/decisions/0015-boundary-failure-contract.md` (committed
  with this spec, status `proposed`) and add its index row to
  `docs/decisions/README.md`. *Proved by:* the row exists, the file's
  status line reads `proposed`, and no accepted decision is edited (AC5).
- [ ] 6. Run the whole gate block and report it verbatim:
  `cargo fmt --all -- --check`; `cargo clippy --all-targets -- -D warnings`;
  `cargo test` (real servers, no mocks — includes AC1's
  `sixteen_threads_cannot_leave_memory_and_disk_in_two_orders`, AC2/AC3/AC4's
  `persistence`/`compat`/`mcp`/`pop3` legs and the `persist_tests` harness);
  `brokkr compile --bundle bundles/self`;
  `GIT_CONFIG_COUNT=0 bash scripts/coverage-gate.sh` (exact
  100% lines/branches/functions, missed == 0); `cargo deny check licenses`.
  *Proved by:* the gate outputs themselves (AC1–AC4, AC6) — with a
  production diff of exactly zero behavior lines.
