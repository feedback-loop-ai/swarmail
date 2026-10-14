# Plan — the boundary stands, its failure contract is written down

Implementation plan for
[specs/persistence-failure-contract/spec.md](spec.md). Prove-and-close run:
the mechanism (`3b158cd`, decision 0014 proposed) is already at HEAD and no
criterion is red, so the work is one test-only instrument, two comment/doc
corrections, the 0012 in-place amendments and the full gate block. **No
production line changes.**

## Technical context

- Single crate, modular monolith. Memory state is two `DashMap`s (per-inbox
  deques + global id index) in `src/store.rs`; the mirror is `src/persist.rs`
  (SQLite, WAL + `synchronous = FULL`) — **frozen at zero changed lines** by
  0014 §5, and untouched here.
- The one ordered mutation boundary is the `mutation: std::sync::Mutex<()>`
  span covering plan → commit → apply (`src/store.rs:363`, `:514`, `:547`,
  `:577`); the whole mutation runs on tokio's blocking pool (0023); readers
  stay lock-free.
- Refusal vocabulary is 0012 §2's fixed text (`451 4.3.0 storage commit
  failed` / `503` / `isError` / `-ERR`); no new wire string exists or is
  added.
- The busy bound is **rusqlite's default** — `sqlite3_busy_timeout(db, 5000)`
  set by the dependency at open (rusqlite 0.32.1 `src/inner_connection.rs`);
  this crate sets no busy handler. The tests' second connections set their
  own 10 s (`tests/common/mod.rs:727,783`), which only bounds *test* waits.
- Test injection is real: `refusing_trigger(db, kind)` opens a second
  connection on the real data file and installs a `RAISE(ABORT)` trigger
  (`refuse_insert` / `refuse_delete`); `RefusingTrigger::drop` removes it.
  Kinds are exhaustive (`panic!` on unknown) — this plan adds one.

## Constitution check

| Decision | Posture |
|---|---|
| 0001 accept == stored | Preserved; 0015 documents the refusal contract's timing and mid-batch shape — the converse arm that keeps `250` honest. |
| 0002 await subscribes first | Untouched; no `wait_for` edit. |
| 0003 oldest-only pruning | Untouched; no eviction path edit; refusal residue reaffirmed as 0014 states it. |
| 0004 real servers, no mocks | The new leg speaks real POP3 against a real server on a real data file; the trigger is real SQL on that file. |
| 0006/0007 (dual license, hermetic gates) | No new dependency; `Cargo.toml`/`Cargo.lock` untouched; `cargo deny check licenses` stays clean. |
| 0023 (proposed) | Built upon — test-code only; nothing reopens the off-worker shape. |
| 0020 (proposed) | The new leg is wire-shaped → `tests/pop3.rs` (rule 3). No cold path is introduced, so no harness move; no sleep, no wall-clock validity assertion (rule 5); no seam added (rule 4). |
| 0012 (proposed) | Amended **in place** (§5, §7) with 0015 markers — legitimate while 0012 awaits the operator (0014's precedent). |
| 0014 (proposed) | Untouched. The mechanism, the seam and the refusal list stand. |
| 0013 (accepted) | Review stays the four-model panel; correctness + security seats read this spec. |

## Project structure — files this run touches

| File | Change | Production lines |
|---|---|---|
| `docs/decisions/0015-boundary-failure-contract.md` | **New**, status `proposed` — committed with this spec by the design seat. | — |
| `docs/decisions/0012-commit-failure-refusal-and-the-mutation-boundary.md` | §5 and §7 amended in place with "(Amended by [0015](0015-boundary-failure-contract.md) …)" markers; §Why's busy-handler bullet gains the contract sentence. | — |
| `docs/decisions/README.md` | Index row for 0015 (proposed). | — |
| `src/pop3.rs` | `QUIT` comment only: the partial-prefix truth replaces "abandoned whole: the marks are unapplied" (the sentence that already admits the prefix stays). Zero behavior lines. | 0 |
| `tests/common/mod.rs` | One new trigger kind: a **selective** refusing delete (refuse one id, allow the rest), parameterized by id — test code only. | 0 (test file) |
| `tests/pop3.rs` | One new wire leg driving `QUIT` across a mid-loop refusal (AC3's prefix instrument). | 0 (test file) |
| `specs/persistence-failure-contract/{spec,plan,tasks}.md` | This spec set, committed by the design seat. | — |

Nothing else: no `src/` file added or removed, no type, no dependency, no
wire string, no `persist.rs` line, no seam. The coverage denominator cannot
move (comment-only `src/` delta; new tests live under `tests/`, outside the
report).

## Approach

1. **The instrument first** (so the doc change lands with its red/green
   witness): extend `refusing_trigger` with a selective delete kind — SQL of
   the shape `CREATE TRIGGER refuse_delete_except BEFORE DELETE ON emails
   WHEN OLD.id = ?1 BEGIN SELECT RAISE(ABORT, 'refused'); END;` bound to the
   id — then write the POP3 leg: deliver two mails (different inboxes or
   cap ≥ 2), learn the target id over REST, install the selective trigger,
   `DELE` both, `QUIT` → assert `-ERR storage commit failed`, session still
   in TRANSACTION (`LIST` still names the refused mail), the allowed id gone
   (prefix committed) on the live tree **and** after a fresh open, the
   refused id present on both; drop the trigger (its `Drop` uninstalls),
   retry `QUIT` in the same session → `+OK` sign-off. Sequential on one
   connection — no sleeps, no wall-clock assertions, no barrier.
2. **Correct the comment** in `src/pop3.rs`'s `QUIT` arm to the
   partial-prefix truth, matching the new leg and 0015 §1.
3. **Amend 0012 in place**: §5's `QUIT` clause and §7 (poison → restart is
   the recovery), each with an "(Amended by 0015 …)" marker; extend §Why's
   busy-handler bullet with the refusal-contract sentence (the ≤5 s bound is
   the dependency's default, never overridden here, deliberately not
   wall-clock-asserted).
4. **Index 0015** in `docs/decisions/README.md`.
5. **Run the full gate block** (below). Everything must be green with **no
   production diff** — if a gate exposes a red that a production change
   would be needed to fix, that red reopens the council (the framing's rule),
   it is never fixed silently under this spec.

## Contracts

No public contract changes. The documented contract — the one 0015 records —
is: insert refuses with zero trace; delete/clear/clear-all refuse and cannot
resurrect; POP3 `QUIT` is at-least-once with a committed prefix; a stalled
disk delays a refusal by at most ~5 s per statement before `SQLITE_BUSY`
surfaces as the fixed-text refusal. All eight adapter sites keep 0012 §2's
existing fixed text.

## Verify gate (quickstart)

```bash
GIT_CONFIG_COUNT=0 cargo test --test pop3 a_failed_quit_prefix_commits_and_the_retry_signs_off   # the new leg (AC3)
cargo test --test pop3 a_failed_quit_answers_err_and_keeps_the_marks                             # existing QUIT leg (AC3)
cargo test --lib sixteen_threads_cannot_leave_memory_and_disk_in_two_orders                      # AC1
cargo test --test persistence --test compat --test mcp                                           # AC2–AC4
cargo test                                                                                       # everything
cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings
brokkr compile --bundle bundles/self
GIT_CONFIG_COUNT=0 bash scripts/coverage-gate.sh                                                 # exact 100/100/100
cargo deny check licenses
```

(`GIT_CONFIG_COUNT=0` works around the harness's malformed
`GIT_CONFIG_KEY_0`; see the intake framing's environment note.)

## Risks

- **Risk: amending 0012 §5 is misread as weakening the delete-refusal
  guarantee.** *Mitigation:* the amendment keeps the `-ERR`-with-TRANSACTION
  contract and adds only the mid-batch truth (prefix committed, suffix
  unmarked, retry converges); AC3's existing legs stay green unchanged, and
  the new leg pins the truth from the wire. Review reads the diff against
  AC5's red state — today's self-contradictory text.
- **Risk: the selective-trigger leg is flaky.** *Mitigation:* the trigger is
  deterministic SQL on the real file; the leg is a single sequential POP3
  session — no timing, no sleeps, no barriers (0020 rule 5). The only
  cross-process element is the second connection, already the house pattern
  (`refusing_trigger`), whose own 10 s busy timeout bounds only test-side
  waits.
- **Risk: the leg's prefix assertion races the trigger's installation.**
  *Mitigation:* install the trigger after delivery and before `DELE`, over
  the second connection's own completed `execute_batch` — the same
  happens-before every existing trigger leg relies on; the trigger names the
  exact id learned from the API, so no ordering of unrelated inserts can
  dodge it.
- **Risk: doc drift among 0012/0014/0015.** *Mitigation:* 0014 is untouched;
  0012 carries dated 0015 markers in place; the README row lists 0015; the
  review gate reads all three against the failure-contract table in the
  spec.
- **Risk: the comment-only `src/pop3.rs` edit trips the coverage gate's
  reserved-name or denominator checks.** *Mitigation:* comments produce no
  executable lines; no file is added or renamed under `src/`; the gate's
  exact-100 check counts lines, not prose. If the gate disagrees, that is a
  red criterion → council, not a workaround.
- **Risk: scope creep during implement/verify** (atomic QUIT "while we're
  here", an open-guard, an exit-on-poison). *Mitigation:* the spec's refusal
  list is explicit and each refusal names its owner (adapter-consistency
  run, deployment run, lifecycle run). Any red the gates expose reopens the
  council; anything green stays untouched.

## Out of scope (the second run, unchanged)

Lifecycle, adapter offloading (incl. atomic `QUIT` and the compat bulk
delete), the Helm chart, the contract-agreement, the CI findings — plus the
owned items 0015 names: the advisory second-writer open guard, exit
semantics on poison, and any change to the busy bound.
