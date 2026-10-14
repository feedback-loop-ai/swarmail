# Tasks — the queued-write shutdown bound

Ordered. Every task names the test that proves it; no task is done until
its proving test runs green against the change and red against its
absence (or, for docs tasks, until every citation resolves to exactly one
decision and the gates are green). The gates
(`fmt --check`, `clippy -D warnings`, `test --workspace --locked`,
`brokkr compile`, `coverage-gate.sh`, `deny check licenses`) are the
final task's exit condition and re-run at HEAD before reporting.

- [ ] **Slice 5 — renumber the collision.** `git mv`
  `docs/decisions/0011-concurrency-tests-and-the-coverage-gate.md` →
  `0020-…`, `0010-watcher-channel-lifetime.md` → `0021-…`,
  `0011-bounded-data-raw-bytes.md` → `0022-…`,
  `0010-off-worker-commits.md` → `0023-…`; add the one-line provenance
  note under each moved file's header; update
  `docs/decisions/README.md`'s index (renumbered rows + 0024, status
  texts verbatim). Nothing else renamed.
  *Proving test*: `ls docs/decisions/ | grep -E '^00(10|11|2[0-4])'`
  shows exactly one file per number; `git status` shows four renames and
  no other path change.
- [ ] **Slice 5 — repoint every citation by subject.** Sweep
  `decision 0010|0011`, `0010-`/`0011-` links and `0011 rule N` across
  `src/` comments, `tests/`, `benches/`, `docs/` (house rules, all
  decision cross-references), `specs/` records, `README.md` and
  `src/static/`: watcher-lifetime → 0021, off-worker commits → 0023,
  exact-count/shared-seed → 0010 (stays), concurrency-tests/harness/
  `rule N` → 0020, shim-delete → 0011 (stays), bounded-data → 0022 —
  read each citation's sentence, never blind `sed`. Fix 0018's dates to
  2026-10-13 (Status line + Date field); leave every decision's status
  line untouched. *Proving test*:
  `grep -rn "decision 0010\|decision 0011\|0011 rule\|0010-watcher\|0010-off-worker\|0011-concurrency\|0011-bounded" src/ tests/ docs/ specs/ README.md`
  returns only citations that resolve to the surviving files of the right
  subject, and `grep -c "2026-10-15" docs/decisions/0018-*` is 0; the
  doc-test-free build stays green (`cargo test --workspace --locked`).
- [ ] **Slice 1 — the flag and the setter in `src/store.rs`.** Add
  `stopping: std::sync::OnceLock<tokio::sync::watch::Receiver<bool>>` to
  the struct (initialized in `new`, which `open` calls) and the one-shot
  `pub(crate) fn observe_shutdown` documented as wired once by `run_on`
  before any listener binds. *Proving test*: the crate compiles with
  zero constructor-signature churn — `grep -c "Store::new("` across
  `src/ tests/ benches/` is unchanged at 46+ sites, and the new field's
  doc comment is exercised by the wiring task below.
- [ ] **Slice 1 — the check at the chokepoint in `record()`.** First
  thing in `record()`: if wired and stopping, `tracing::info!(what,
  "refused: shutting down; nothing applied")` and return the constructed
  `rusqlite::Error::SqliteFailure(ffi::Error::new(ffi::SQLITE_ABORT),
  Some("shutting down".into()))`. No mapper, signature, or apply-phase
  line changes; `persist.rs` stays at zero diff.
  *Proving test* (inline, 0011 rule 1): a `Store::new` wired via
  `observe_shutdown` refuses `insert` with that exact code and applies
  nothing — `count == 0`, no inbox minted (`inboxes()` empty), no counter
  moved, `watcher_count()` unmoved; the same store before the flag set
  inserts `Ok` (the false arm); the unwired arm is every existing unit
  test, unchanged.
- [ ] **Slice 1 — wire it in `src/lib.rs`.** One line after the watch is
  born (`run_on`, before the dispatcher spawn and before any listener
  binds): `store.observe_shutdown(shutdown_rx.clone());` with the 0024
  comment. *Proving test*: the `stop()` leg below (the flag must be set
  for it to refuse) and every existing lifecycle test staying green
  (wired-not-stopping traffic is unaffected).
- [ ] **Slice 1 — the `stop()` leg in `tests/lifecycle.rs`.**
  `queued_mutations_do_not_burn_one_busy_timeout_each`: real data file
  (`start_with`), `common::hold_write_lock(&path)`, three concurrent
  POSTs to `/api/v1/inboxes/{s1,s2,s3}/seed` spawned as tasks, 300ms
  sequencing pause, `tokio::time::timeout(STOP_BUDGET, srv.stop())`, the
  elapsed asserted `< STOP_BUDGET` (upper bound only), all three
  requests answered `503 {"error":"storage commit failed"}`, then a fresh
  `rusqlite` read of the file asserts zero `emails` rows.
  *Proving test*: the leg itself — red on the pre-change tree (15s
  serialized burns exceed the budget timeout) and green after; run
  against the real server, no mocks.
- [ ] **Slice 1 — the process leg in `tests/binary.rs`.**
  `sigint_exits_within_the_budget_with_a_held_write_lock`: spawn the real
  `swarmail serve` on a disposable data file, `wait_http_up`, hold
  `BEGIN IMMEDIATE` on that path **from the test**, submit three
  concurrent seeds, 300ms pause, `libc::kill(child.id(), SIGINT)`,
  assert exit status success and elapsed-from-kill `< 10s` (upper bound,
  sequence-independent), collect the three `503`s (written during the
  graceful drain), drop the lock, reopen the file and assert the restart
  copy is exactly the committed set — empty — never a partial row.
  *Proving test*: the leg itself; a returning `stop()` cannot fake it —
  the assertion is on the real process's exit time, observed via
  `try_wait` polling like the existing `sigint_and_wait`.
- [ ] **Slice 1 — the lifecycle documentation moves with the behavior.**
  0018 gains the dated `Amended by: 0024` marker (0019's pattern) and §2
  states the mutation mechanics: new mutations refuse fast once stopping,
  queued lock-waiters are released without a busy timeout, a started
  commit runs to completion — the process burns at most one busy bound
  after shutdown begins. 0024 §4's observability sentences are the
  contract of record. README's lifecycle row, `llms.txt` and
  `openapi.json` budget/refusal wording aligned.
  *Proving test*: `grep -n "2026-10-13" docs/decisions/0018-*` shows both
  fields; the README row and 0018 §2 name the queued-mutation bound; no
  doc claims abandoned work "didn't happen"
  (`grep -rn "didn't happen\|never happened" docs/ README.md` is empty);
  gates green.
- [ ] **Slice 6 — the 404-delete cost documented at the boundary.** In
  `src/store.rs` `delete`'s mirror comment: the no-op DELETE is committed
  deliberately — the guard against a lingering mirror row — and its cost
  is one bounded commit per 404 (0015 §2); the `changes() > 0` skip is
  the refused arm (0014 §2's commit contract + the zombie derivation),
  named so the choice stays visible. *Proving test*: comment-only — the
  existing delete tests (`Ok(false)` for an absent id, no resurrection
  after restart) stay green and the coverage denominator is unchanged
  (`scripts/coverage-gate.sh`).
- [ ] **Slice 6 — POP3 QUIT's deviation documented at the handler.** In
  the `src/pop3.rs` Transaction-QUIT arm comment: on a failed update the
  session **stays** in TRANSACTION deliberately — RFC 1939 says `QUIT`
  closes, Mailpit keeps the session so the client can retry or `RSET` on
  the same connection; the at-least-once prefix truth (0015 §1) stands
  beside it. *Proving test*: comment-only — the existing POP3 legs
  (`tests/pop3.rs` QUIT-failure → `-ERR`, session usable) stay green.
- [ ] **Slice 6 — the fan-out comment tells the whole truth.** In
  `src/store.rs` insert's apply phase: the shard `entry()` guard is held
  across the watcher and global sends *because* the sends are
  non-blocking and cost ~nothing; "outside the boundary tail" refers to
  the commit; a hung webhook target can never hold the boundary. The
  guard itself is kept (dropping it early would reorder dispatch relative
  to the memory order a watcher observes). *Proving test*: comment-only
  — the existing watcher/fan-out tests and `coverage-gate.sh` green.
- [ ] **Slice 6 — the stale seed-handler comment corrected.** In
  `src/api.rs`'s seed match arm, replace "Memory truth stands: the
  refused mail is still queryable" (dead since 0014 §3) with the refusal
  contract: nothing applied, nothing queryable, no fan-out — and during
  shutdown the same fixed text covers the refusal family (0024).
  *Proving test*: comment-only — the seed 503 legs
  (`tests/lifecycle.rs`/`tests/api.rs`) stay green verbatim; the new
  `stop()` leg's body assertion pins the text.
- [ ] **The gate sweep at HEAD.** `cargo fmt --all -- --check`;
  `cargo clippy --all-targets -- -D warnings`; `cargo test --workspace
  --locked`; `brokkr compile --bundle bundles/self`;
  `bash scripts/coverage-gate.sh` (compare the floor: it may rise, never
  fall — the check's three arms are each executed by a named test);
  `cargo deny check licenses`. *Proving test*: all six commands exit 0
  at the run's HEAD; no new dependency in `Cargo.toml`/`deny.toml`; no
  new `src/` file; `git status` clean of stray artifacts (`.forge/` is
  gitignored run evidence and stays uncommitted).
