# Plan — run 3: the boundary stands, its invariant is stated, its busy bound is loud

Technical context, constitution check, project structure and the risks of
the change specified in
[spec.md](spec.md) and ruled in
[0016](../../docs/decisions/0016-lag-invariant.md) (proposed). The adopting
run touches **no behavior line**: its entire `src/` delta is two tests in
`src/persist.rs`'s existing inline `#[cfg(test)] mod tests`.

## Technical context (measured at HEAD `19cc49d`, not remembered)

- **The boundary is landed.** One `mutation: std::sync::Mutex<()>` spans
  plan → commit → apply for all four persistent mutations — `insert`
  (`src/store.rs:363`), `delete` (`:514`), `clear` (`:547`), `clear_all`
  (`:577`) — commit before visibility, refusal with zero trace
  (`record`, `store.rs:258-270`), fan-out only after apply. Readers stay
  lock-free. `git diff 5b09843..HEAD -- src/persist.rs Cargo.toml
  Cargo.lock` is empty.
- **The failure contract is landed.** Eleven refusal call sites reuse 0012
  §2's fixed text (`api.rs:385/:414/:520`, `smtp.rs:624`, `compat.rs:263/
  :304`, `mcp.rs:132/:139/:144/:214`, `pop3.rs:393`); the POP3 partial-
  prefix truth is in 0012 §5 as amended; poison → restart is 0015 §3.
- **The busy bound's true location.** There is no `busy_timeout` anywhere in
  `src/` — the ~5 s bound is rusqlite's open-time default
  (`sqlite3_busy_timeout(db, 5000)`, rusqlite 0.32.1
  `src/inner_connection.rs:119`, cited by 0012 §Why). The only overrides in
  the tree are test helpers on their *own* connections
  (`tests/common/mod.rs:727/:810`), which prove the contract rather than
  change it. The tripwire must read the **server's own connection**:
  `Persist.state.conn` is private to `persist.rs`, and the inline `mod
  tests` is its child module, so the test reaches it directly — no new
  visibility, no seam, no harness file.
- **Instruments already present.** The C1 probe
  (`store::bounded_tests::sixteen_threads_cannot_leave_memory_and_disk_in_two_orders`)
  and the ten real-server wire legs in `tests/persistence.rs`; both re-run
  green during this design (0.59 s and 5.9 s respectively). The existing
  `db_path(tag)` helper in persist.rs's test module provides the unique
  temp data file the tripwire needs.

## Constitution check

| Item | Verdict |
|---|---|
| 0001 accept == stored | Unchanged, still strengthened; the lag invariant is the stated reason. |
| 0002 await subscribes first | Untouched — no `wait_for`/`subscribe` edit. |
| 0003 oldest-only pruning | Untouched — no eviction path edit. |
| 0023 (proposed) off-worker commits | Built upon — the tripwire runs inline under `cargo test`, never inside an offload. |
| 0020 (proposed) test placement | Followed — the tripwire is branch-stable inline (rule 1), a fact not a timing (rule 5), and the `#[should_panic]` leg is deterministic with no sleeps. |
| 0012/0014/0015 (proposed) | Amended **in place** with markers naming 0016; both still await the operator, so the amendment is legitimate (0014's precedent). No accepted decision is edited. |
| Semantic change? | **Yes — rulings are amended** (0014 gains §8; 0015 §2 is extended), hence 0016. |
| Dependency policy | No dependency added; `Cargo.deny` has nothing new to judge. |
| Coverage gate | Exact 100/100/100 preserved: the tripwire's lines execute on every run; no harness file, no `coverage(off)`. |

## Project structure — every file the adopting run touches

| Path | Change |
|---|---|
| `docs/decisions/0016-lag-invariant.md` | **Committed with this spec** — status `proposed`. |
| `docs/decisions/0014-one-ordered-mutation-boundary.md` | Add §8, the lag invariant, with an *(Added by 0016 …)* marker. Status stays `proposed`. |
| `docs/decisions/0015-boundary-failure-contract.md` | Extend §2 with the ENOSPC claim, the size-cap refusal and the tripwire clause under an *(Amended by 0016 …)* marker. Status stays `proposed`. |
| `docs/decisions/README.md` | Add the 0016 row. |
| `src/persist.rs` | **Test module only**: a shared `assert_dependency_busy_default(&Connection)` helper, the green leg `the_opened_connection_keeps_the_dependency_busy_default`, and the `#[should_panic]` leg `the_busy_default_tripwire_fires_when_the_bound_is_overridden`. |
| `specs/p1-persistence-closure/` | This spec set. |

Nothing else — no new `src/` file, no type, no dependency, no wire string,
no counter, no behavior line, no `tests/` addition (the wire legs already
prove the slice's criteria; a new leg would be an unowned test).

## Contracts and data model

- **The lag invariant** (0016 §2 → 0014 §8) is the design's load-bearing
  property: insert keeps memory ⊆ disk at every instant; delete/clear/
  clear-all keep memory ⊇ disk; the ghost and the resurrection are the two
  directions of *leading* and are unrepresentable; the one tear is poison's,
  healed by restart (0015 §3).
- **The tripwire contract**: `assert_dependency_busy_default` asserts
  `PRAGMA busy_timeout` == 5000 — the dependency default, cited to rusqlite
  0.32.1 `inner_connection.rs:119`. The green leg runs it on a real
  `Persist::open` connection; the can-fire leg sets
  `PRAGMA busy_timeout = 30000` on a throwaway `Persist`'s own connection
  and asserts the helper panics. Both legs execute every line of the helper
  on every run — the helper's failing branch is *used*, not dead, which is
  what keeps the exact gate honest.
- **The refusal contract** is referenced, never restated: 0012 §2 (fixed
  text), 0015 §1 (POP3 prefix), 0015 §2 (timing). Restating invites drift.

## Quickstart

```bash
export PATH="$HOME/.cargo/bin:$PATH"
GIT_CONFIG_COUNT=0 cargo test --lib sixteen_threads_cannot_leave_memory_and_disk_in_two_orders  # AC1: no red criterion
GIT_CONFIG_COUNT=0 cargo test --lib busy_default                # AC3: the tripwire, both legs
cargo test                                                      # AC5: real servers, no mocks
GIT_CONFIG_COUNT=0 bash scripts/coverage-gate.sh                # AC4: exact 100/100/100
```

`GIT_CONFIG_COUNT=0` neutralizes the harness's broken `GIT_CONFIG_*` env
(its `GIT_CONFIG_KEY_0` is missing) — the same workaround run 2's verify
gate records. A red on the C1 probe or any wire leg **stops the run**: the
confirmation's premise is gone, and the council reopens.

## Risks

- **The tripwire couples the suite to rusqlite's default (5000).** A
  dependency bump that legitimately changes the default fails the green leg.
  *Mitigation:* that failure **is** the instrument — it fires 0015 §2's
  decision-document obligation at the moment of the change, and the helper's
  comment cites the exact dependency line so the fixer reads the ruling
  first. If the operator accepts a new default, the decision document and
  the constant move together.
- **`#[should_panic]` legs can mask a wrong panic.** The can-fire leg would
  pass on any panic, not only the tripwire's. *Mitigation:* the helper is
  the only thing the leg calls, so the panic can only be the assertion; the
  message is asserted (`#[should_panic(expected = …)]`-style message match on
  the tripwire's own text) so a stray panic fails the leg.
- **Inline test lines join the measured denominator.** 0020 rule 1 makes
  persist.rs's inline module part of the production report — an untaken line
  fails the exact gate. *Mitigation:* the helper is straight-line plus one
  assert, and *both* of its branches are executed on every run (green leg =
  passing branch, can-fire leg = panicking branch); the coverage gate itself
  is the proof.
- **Doc drift between the spec and the decisions.** The invariant is stated
  in 0016 and lands in 0014; a copy here could diverge. *Mitigation:* the
  spec carries the ruling and a one-line summary only, and points at 0016
  §2 / 0014 §8 as the single authority; the amendments are the same run's
  tasks, landed together.
- **Confirmation theater — the run adds vocabulary while claiming to add
  none.** *Mitigation:* the standing refusal list (spec) is a review check;
  the file table above is exhaustive; AC4 proves the `src/` diff is exactly
  two tests; `git diff --stat` review refuses anything else.
- **The premise (nothing is red) rots while the run works.** *Mitigation:*
  task 1 certifies the premise first — C1 probe + the slice's wire legs —
  and any red stops the run and reopens the council instead of being fixed
  silently under a confirmation spec.
- **Serialization cost re-litigated by a future load number.** The
  mutation-phase serialization is priced (nanoseconds against an
  fsync-bound tail); if `benches/ingest.rs` or `tests/burst.rs` ever convict
  it, the answer is batching inside one transaction — never a second order.
  *Mitigation:* the canary is named in the spec; a floor drop is a red
  criterion, not noise.
