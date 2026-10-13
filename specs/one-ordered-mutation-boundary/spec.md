# Spec — issue #31, the P1 storage-consistency slice: one ordered mutation boundary (commit before visibility)

Issue [feedback-loop-ai/swarmail#31](https://github.com/feedback-loop-ai/swarmail#31)
(*"Story: Close persistence, lifecycle and deployment gaps from the architecture
reassessment"*, reviewed commit `5b09843`). This run delivers **only** the
storage-consistency slice; lifecycle, adapter offloading, the Helm chart, the
contract-agreement and the CI findings are a second run. Intake framing:
[`.forge/intake/issue31-p1-persistence-framing.md`](../../../.forge/intake/issue31-p1-persistence-framing.md).
The panel's positions —
[`.forge/design/positions/simplicity.md`](../../../.forge/design/positions/simplicity.md) and
[`.forge/design/positions/robustness.md`](../../../.forge/design/positions/robustness.md)
(both `pass`) — are run-local evidence, gitignored.

Decision: **0014** —
[docs/decisions/0014-one-ordered-mutation-boundary.md](../../docs/decisions/0014-one-ordered-mutation-boundary.md)
(status `proposed` — acceptance is the operator's). It amends decision
[0012](../../docs/decisions/0012-commit-failure-refusal-and-the-mutation-boundary.md)
(proposed) §4/§5/§6 **in place** — legitimate because 0012 still awaits the
operator — and the amendment itself is a task of this run (AC5), not committed
here.

## The problem — two orders wearing three hats

One bug, three P1 findings, all in `src/store.rs`:

1. **Memory and disk do not share one mutation order** (finding 1).
   `insert` (`src/store.rs:348-410`) releases the mutation lock at `:386`
   *before* taking the persistence lock at `:391`; `delete`/`clear`/
   `clear_all` release before their mirrors at `:514`/`:541`/`:574`. The
   persist mutex orders commits; the mutation lock orders memory phases;
   **nothing orders commit *k* against memory phase *k+1*** — two
   serializations are two orders. Concretely, with `max_per_inbox = 1` and
   16 barrier-released threads on public `Store::insert`: a later insert
   prunes an earlier id from memory and deletes its row, and the earlier
   transaction then persists an already-evicted message — live `Store::list`
   and a fresh `Store::open` on the same file disagree (reproduced three
   times, no injected storage failures). ADR0012 §6's claim that separate
   serialization "adds nothing across `record`" is the exact error.
2. **A refused insert is exposed as accepted mail** (finding 2). With cap = 1,
   one committed message and a rejecting `BEFORE INSERT` trigger on the real
   data file, the second send is refused (`451 4.3.0 storage commit failed`)
   — yet the refused message stays live and queryable (HTTP 200,
   `matched = 1` via `await`), because 0012 §4 rules "memory truth stands".
   The committed-first message was evicted from memory, and after restart
   only committed-first remains: the ghost is the *only* copy.
3. **A failed deletion reports success** (finding 3). With an equivalent
   `BEFORE DELETE` trigger, native `DELETE /api/v1/messages/{id}` answers
   `200 {"deleted":true}` (`src/api.rs:393`) while the live inbox empties
   (`let _ = self.record(...)` at `src/store.rs:514`) and the message
   resurrects after restart. The same swallow lives in `clear` (`:541`) and
   `clear_all` (`:574`).

## Ruling — how the positions were reconciled

The seats converge on the load-bearing core and diverge on four points; this
spec rules rather than averages.

**Adopted from both (convergent, stated once):** one store-wide ordered
boundary — the existing `mutation: std::sync::Mutex<()>` field with its span
widened to cover the *whole* mutation, the SQLite transaction included — and
**commit before visibility**: each mutation is *plan → commit → apply* inside
one critical section, so memory order, disk order and `seq` order are one
order. A refused insert never touches memory (no rollback exists to write);
failed deletes propagate with 0012 §2's fixed-text vocabulary;
`src/persist.rs` changes by **zero lines** (`record_insert`/
`record_delete`/`record_deletes` are already atomic and exact-id; the mirror
was never wrong — the sequencing around it was). Refused cuts, for the
reasons both seats give: no bounded writer (a queue between SMTP and the
store, with a full-queue policy and a shutdown-drain obligation — 0010 §3
forbids write-behind), no undo/rollback engine (three inverse paths over the
two-map seam, each infallible-by-fiat, versus a reorder that makes the ghost
unrepresentable), no per-inbox locks (they cannot span `clear_all` and
re-derive finding 1), no `DELETE … WHERE inbox = ?` (the per-id mirror stays —
exact ids are self-verifying), no new error type (`rusqlite::Error` is
already public), no new dependency, no degraded mode, no write-behind queue,
no `persist_failures_total` (the refusal is the detector), no reader-side
changes, no new commit-stage chaos event (injection stays real: triggers and
lock-holders on the real file).

**Ruling 1 — the `pre_mirror_gate` seam is kept and shrunk (simplicity),
not deleted (robustness).** Robustness would delete the seam; but the seam
is the *only* deterministic injection point that executes two surviving
production lines: `offload`'s `Err`/`resume_unwind` arm (the panic-symmetry
pin is its only cover, and the exact 100/100/100 gate refuses uncovered
lines) and the await-resolution ordering pin. The seam survives with its
window moved — it fires **pre-commit, inside the boundary**, under a
documented **no-re-entry contract** (its closures panic, block or observe;
they never call back into the store). Its two re-entry race tests
(`a_clear_racing_an_insert_cannot_erase_the_inserts_row`,
`a_clear_all_racing_an_insert_…`) are **deleted, not ported**: the
drain→mirror window they model no longer exists, their property is
structural, and it stays covered by the wire race leg
(`tests/persistence.rs:291`) plus the new C1 probe. **Cost of the losing
side (robustness):** its "rewrite to the poles" legs already exist as the
restore/mirror harness legs — nothing is lost by not writing them again.
**Cost of the surviving shape (over simplicity's stronger cut):** the seam
remains a test-only re-entry hazard, priced at one sentence of contract in
0014 and 0012 §6, and it is `#[cfg(test)]`-only — production never sees it.

**Ruling 2 — the ghost-era tests are rewritten, not preserved (both; the
intake framing demands it).** `a_failing_data_file_never_blocks_the_store`
(`src/store/persist_tests.rs:88`) asserts 0012 §4's ghost; the wire legs
`a_blocked_commit_answers_451_and_never_claims_250` and
`a_blocked_commit_refuses_the_rest_seed_with_503`
(`tests/persistence.rs:595,683`) assert "the refused mail is queryable while
live" — finding 2 verbatim. All three flip to the zero-trace refusal. That
flip *is* the criterion going red on the current tree and green after; it is
not a lost guarantee (0012 is proposed, and §4 is exactly the claim the
findings refute).

**Ruling 3 — persist.rs stays at zero lines; robustness's `seq` corollary
becomes a criterion, not a code change.** `next_seq` is assigned inside
`record_insert` under the persist mutex; once the boundary spans the commit,
seq order == boundary order == memory push order, so the restart copy's
per-inbox order provably equals the live order. That is asserted (AC1), not
machinery.

**Ruling 4 — counters are events of the commit (robustness's phrasing),
implemented by the plan's post-counters (simplicity's table).** The plan
computes post-counters (`inserted + 1`, `dropped + overflow`) and the
transaction writes them; the memory bump happens in apply, after `Ok`. A
refused insert's tentative eviction never happened and is never counted —
anywhere, live or restart. `emails_dropped` counts every eviction exactly
once, at its commit (0003's exactness, settled explicitly — C4); the
eviction *choice* is byte-identical: front ids, peeked in order.

**Ruling 5 — the "applicable adapter" set is enumerated, closing the gap
both positions named.** With `delete`/`clear`/`clear_all` becoming
`Result<bool/usize, rusqlite::Error>` (0012 §1's precedent: one public
variant, no new type), exactly **eight** production call sites map the
failure — SMTP has no delete surface and `src/ui.rs`/`src/stdio.rs` are
read-only or REST-bound, so they need nothing:

| Site | Surface | Failure mapping |
|---|---|---|
| `src/api.rs:393` | REST `DELETE /api/v1/messages/{id}` | `503 storage commit failed` (404 stays for absent-on-`Ok(false)`) |
| `src/api.rs:374` | REST `DELETE /api/v1/inboxes/{inbox}/messages` | `503 storage commit failed` |
| `src/api.rs:84` → `src/compat.rs:282` | Mailpit/Mailhog `delete_messages` | `503`; the one sync call moves to `delete_async` (one line, strictly 0010) |
| `src/compat.rs:256` | Mailpit delete-all | `503 storage commit failed` |
| `src/mcp.rs:126/135/139` | MCP delete / clear_inbox / clear_all | `error_result("storage commit failed")` → `isError: true` |
| `src/pop3.rs:362` | POP3 `QUIT` update state | `-ERR storage commit failed`; session stays in TRANSACTION, marks unapplied |

**Ruling 6 — POP3's shape is corrected against both positions.** Simplicity
had it right: `DELE` is only a mark (`src/pop3.rs:336-351`); the store
mutation happens at `QUIT` (`:362-374`). Robustness's "`DELE` answers
`-ERR`" is rejected as protocol-wrong — the failure surfaces one command
later, still on the wire, still never a success claim. After a failed
update the session remains in TRANSACTION with its marks (RFC 1939 §5);
the client may `RSET` or retry `QUIT` after recovery.

**Ruling 7 — the burst cost is measured, not assumed (robustness).**
`tests/burst.rs` stays a gate and the `benches/ingest.rs` steady-state
number is recorded in the 0012 amendment. The analysis says the marginal
cost is nanosecond-scale (commits were already globally serialized by the
persist mutex; only memory-phase/commit-tail overlap is new); the bench is
the proof.

## The design — one boundary, four mutations, commit first

Every persistent mutation runs its whole body inside one `mutation.lock()`
critical section:

| Mutation | Plan (fallible-free read) | Commit (the one fallible step) | Apply (infallible memory half) |
|---|---|---|---|
| `insert` | peek the inbox deque: `overflow = (len+1)−cap`, pruned ids = front ids; post-counters `inserted+1`, `dropped+overflow` | `record_insert(email, &pruned_ids, post_counters)` | `push_back`, `pop_front × overflow`, index insert/removals, counter bumps |
| `delete(id)` | — (the id is the plan) | `record_delete(id)?` | index remove, deque retain, watcher release if empty |
| `clear(inbox)` | snapshot the deque's ids | `record_deletes(&ids)?` | `drain_inbox` (releases the channel) |
| `clear_all()` | snapshot every inbox's ids | `record_deletes(&ids)?` | drain each inbox, `watchers.clear()` |

Properties, finding by finding: the commit order *is* the memory order *is*
the `seq` order (finding 1 is structurally impossible — a prune decision and
its row-write share one critical section); a refused insert never enters
memory, so nothing is queryable, nothing is evicted, no counter moves, no
watcher wakes, no webhook fires, and no phantom empty-inbox entry is minted
(the plan peeks with `get`; the `entry().or_default()` wart dies — finding 2
has no representation); a failed delete commits nothing and removes nothing,
so "success followed by resurrection" is unrepresentable (finding 3).
Fan-out stays **after** apply, inside the boundary's tail but outside the
commit: a hung webhook target can never hold the mutation lock (0012 §3
preserved). Lock order is fixed and one-directional — `mutation` (outer) →
`persist::State` (inner) — with no path back; the whole mutation still runs
on tokio's blocking pool (0010), so a std mutex is never held across an
`await`. Readers (`get`, `list`, `count`, `inboxes`, `wait_for`,
`subscribe`) stay lock-free and answer throughout, including during a
bounded 5 s `SQLITE_BUSY` stall.

**Crash-window vocabulary (named, accepted):** the only new window is
*commit succeeded, memory apply not yet run*; a process death there leaves
the row on disk with no `250` sent — the client retries, SMTP delivers the
duplicate. That is at-least-once, and strictly better than today's
memory-first window (live in memory, never on disk, process dead). The
restart copy is always exactly the committed set.

**Accepted costs:** cross-inbox serialization behind any commit (a slow or
failing disk delays every mutation, bounded by the busy handler — the price
of a shared order); head-of-line blocking across inboxes under one order
(a hostile inbox's burst can *delay* others, never corrupt or reorder them);
an apply-phase panic after a successful commit fail-stops via the poison
`expect` (invariant-break territory — the mirror image of today's
record-panic residual, both fail-stop); a refused insert burns its UUIDv7
id and nothing else; an inbox left over cap by a refused insert stays over
cap until the next accepted insert (0003 puts eviction in `insert`;
pruning on refusal would evict committed mail the boundary must protect).

## Frozen guarantees — posture

| Guarantee | Verdict |
|---|---|
| **0001** accept == stored | **Strengthened.** A `250` now implies the mail is live **and** committed before the reply; a failed commit answers `451` and stores nothing anywhere. |
| **0002** await subscribes before it checks | Untouched. `wait_for` is not edited; a refused mail never enters memory, fires no watcher, and the store-is-truth rescan finds nothing. |
| **0003** pruning is oldest-only | Preserved. The eviction choice is byte-identical (front ids, peeked in order); `emails_dropped` counts every eviction exactly once, at its commit. |
| **0010** (proposed) off-worker commits | Built upon. The whole mutation still runs on the blocking pool; `synchronous = FULL` and reply-after-outcome are untouched; panic symmetry stays. |
| **0012** (proposed) | Amended in place by this run's tasks (§4 ghost reversed, §5 swallow reversed, §6 two-orders claim replaced); 0014 is the new ruling. Both stay `proposed` for the operator. |
| Exact 100/100/100 gate | Held. Every new production line ships with the test that executes it; no `coverage(off)`; the denominator never shrinks; no new `src/` file. |

## Acceptance Criteria

- **AC1 — One ordered boundary (finding 1).** All persistent mutations
  execute plan → commit → apply inside one `mutation`-lock critical section
  that spans the SQLite transaction. *Proved by:* the 16-thread
  barrier-released probe (inline in `src/store.rs`, 0011 rule 1) over a real
  data file with `max_per_inbox = 1`: after the barrier, live
  `list`/`get`/`count` and a fresh `Store::open` on the same file agree on
  the same per-inbox ID sets, the same per-inbox ordering and the same
  counters; no inbox exceeds the cap after restart; `delete`/`clear`/
  `clear_all` racer roles keep the same agreement across rounds. The probe
  is **red** on the current tree (three recorded runs, per the issue) and
  green after; wire-level interleavings live in `tests/persistence.rs`.
- **AC2 — A refused insert is invisible (finding 2).** With cap = 1, one
  committed message and a rejecting `BEFORE INSERT` trigger on the real data
  file: the second send is refused on every ingest surface (SMTP exactly
  `451 4.3.0 storage commit failed`; REST seed `503`; MCP `isError: true`),
  and the refused message is queryable through **nothing** — `get`, `list`,
  `count`, the exact-count assert API, `await` (`matched = 0`, not 1), POP3
  or the UI. The committed-first message is not evicted (live **and**
  restart); accepted-mail counters do not increment; no webhook delivery and
  no watcher wake for the refused mail; no phantom empty-inbox entry appears
  in `inboxes()`. After the trigger is dropped (storage recovery), a retry
  answers `250` and lands normally. *Proved by:* wire legs in
  `tests/persistence.rs` (real trigger on the real file, no mocks) — **red**
  on the current tree, which answers the refusal and still serves the mail
  (HTTP 200, `matched = 1`).
- **AC3 — Failed deletions report failure (finding 3).** With an equivalent
  `BEFORE DELETE` trigger on the real data file: native
  `DELETE /api/v1/messages/{id}` returns `503` — never
  `200 {"deleted":true}` — and the clear, clear-all, MCP, compat-shim and
  POP3-`QUIT` surfaces map the same failure per the table above, never a
  success claim. Live state equals committed state on every refusal (no
  memory removal, no watcher release), and a restart shows no resurrection
  and no lost mail. After the trigger is dropped, the same operations
  succeed normally. *Proved by:* wire legs in `tests/persistence.rs`
  (+ `tests/mcp.rs`, `tests/compat.rs`, `tests/pop3.rs` where the harness
  lives) — **red** on the current tree, which answers `200` and resurrects.
- **AC4 — Counters stay exact (C4).** Counters are events of the commit: a
  refused insert increments nothing live or after restart; every eviction of
  an accepted insert is counted exactly once. *Proved by:*
  `counters_keep_counting_across_restarts` (`tests/persistence.rs:413`)
  extended with a refusal leg, and the probe's counter agreement.
- **AC5 — Documentation tells the truth (C5).** ADR0012 is amended in place:
  §4's ghost reversed, §5's logged swallow reversed, §6's "separate
  serialization is sufficient" replaced by the boundary-across-commit rule
  (with the no-re-entry seam contract and the fixed lock order); its status
  line remains `proposed`. `docs/decisions/README.md` lists 0012. The retired
  mirror-image debt note (`tests/persistence.rs:401-407`) is updated — the
  boundary closes it. The burst bench number is recorded. *Proved by:* the
  review/docs criterion — the amended §6 no longer contains the claim, the
  index row exists, the note no longer calls the race open debt.
- **AC6 — The gate discipline holds (C6).** All six gates green:
  `cargo fmt --all -- --check`; `cargo clippy --all-targets -- -D warnings`;
  `cargo test` (real servers, no mocks); `brokkr compile --bundle
  bundles/self`; `bash scripts/coverage-gate.sh` (exact 100% lines/
  branches/functions, missed == 0); `cargo deny check licenses` (zero new
  dependencies). Every new production line has an executing test; no new
  `src/` file exists to serve tests. *Proved by:* the gate runs themselves.

## Verify gate

```bash
GIT_CONFIG_COUNT=0 bash scripts/coverage-gate.sh   # exact 100/100/100 (AC6)
cargo test --lib                                    # the probe + harness failure paths (AC1, AC2)
cargo test --test persistence                       # wire legs: triggers, lock-holder, restarts (AC1-AC4)
cargo test --test mcp --test compat --test pop3     # adapter failure mappings (AC3)
cargo test counters_keep_counting_across_restarts   # AC4
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
brokkr compile --bundle bundles/self
cargo deny check licenses
cargo bench --bench ingest -- --quick               # AC6 + the AC5 bench number
```
