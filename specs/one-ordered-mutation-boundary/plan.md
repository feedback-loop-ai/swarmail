# Plan — issue #31 P1 slice: one ordered mutation boundary (commit before visibility)

Companion to [`spec.md`](spec.md) (WHAT/WHY) and decision
[`0014`](../../docs/decisions/0014-one-ordered-mutation-boundary.md)
(status `proposed`); decision
[`0012`](../../docs/decisions/0012-commit-failure-refusal-and-the-mutation-boundary.md)
is amended in place by task 8. How it lands: the exact edits, the files they
touch, the tests that prove them, and the risks with mitigations.

## Technical context

Anchors read from the working tree at HEAD `a994cee` and re-verifiable in
seconds:

- `src/store.rs` — the spine. `drain_inbox` (`:103-119`); the `mutation`
  field (`:148`, today "held only across the memory phase … never across
  `record`"); the `pre_mirror_gate` seam (`:154-161`, `:269-288`);
  `record` (`:253-265`); `insert` (`:348-410`: memory phase `:352-386`
  with `entry().or_default()` at `:358`, `record` at `:391-399`, fan-out
  `:405-408`); `delete` (`:484-516`, swallow `:514`); `clear`
  (`:519-543`, gate `:528-529`, swallow `:541`); `clear_all`
  (`:546-576`, gate `:568-569`, swallow `:574`); `offload` (`:589-598`);
  `insert_async`/`delete_async`/`clear_async`/`clear_all_async`
  (`:609-629`); the inline probe + fixture mod; the harness
  `src/store/persist_tests.rs` (`a_failing_data_file_never_blocks_the_store`
  at `:88-124` — asserts 0012 §4's ghost; the two gate race tests at
  `:162-209`; the panic-symmetry and await-ordering pins at `:217-259`).
- `src/persist.rs` — the mirror, changing by zero lines. `record_insert`
  (`:115-162`: one transaction — row, exactly-id pruned deletes
  (`DELETE … WHERE id = ?1`), post-counters upsert; `next_seq` assigned
  under the state mutex), `record_delete` (`:164-170`),
  `record_deletes` (`:181-188`), `fail_writes` (`:192-196`,
  `#[cfg(test)]`), the state mutex (`:77`).
- Adapter call sites (the eight): `src/api.rs:374-379` (`clear_inbox`,
  returns `Json`), `:393-401` (`delete_message`, `Json`-only today),
  `:493-506` (seed → `503 storage commit failed`, the 0012 pattern to
  copy), `:36-45` (`ApiError` + `IntoResponse`); `src/compat.rs:256`
  (delete-all → `clear_all_async`), `:282` (`delete_messages` — the one
  **sync** `store.delete` call, in an async handler); `src/mcp.rs:126-141`
  (delete/clear/clear_all) and `:56-58` (`error_result`); `src/pop3.rs`
  `DELE` `:336-351` (mark only), `RSET` `:353`, `QUIT` `:362-374` (applies
  the marks via `delete_async`, then signs off unconditionally).
- `src/smtp.rs` DATA stage `:618-626` — already the 0012 shape (`Ok` →
  `250`, `Err` → `451 4.3.0 storage commit failed`); **untouched** by this
  run. `src/ui.rs`, `src/stdio.rs`, `src/feed.rs` — read-only or REST-bound
  (the `feed.rs`/`threads.rs` `insert` sites are `#[cfg(test)]`); untouched.
- `tests/persistence.rs` — `data_file`/`clean_up`/`serve_data_file`
  (`:13-37`); the clear-race wire leg `:291-410` and its **retired debt
  note** `:401-407` ("an epoch/generation scheme would be needed" — the
  boundary closes it); `counters_keep_counting_across_restarts` `:413`;
  the write-heavy leg `:466`; the blocked-commit legs `:595-654` and
  `:683-727`, which today assert the ghost ("the refused mail is queryable
  while live") and flip. `tests/common/mod.rs` — `hold_write_lock`
  (`:725-732`) plus real-protocol helpers; `tests/webhook.rs` has
  `destination_sink` for the no-webhook leg; `tests/{mcp,compat,pop3}.rs`
  host their adapters' harnesses.
- `benches/ingest.rs` — `insert` only (already `Result`-shaped); benches
  owe no `delete`/`clear` ripple. `scripts/coverage-gate.sh` — exact
  100/100/100; run it with `GIT_CONFIG_COUNT=0` (the documented sidestep
  for the broken harness `GIT_CONFIG_KEY_0`). Commits: `commit.gpgsign`
  resolves **true** from `~/.gitconfig` and recent history is signed
  (`a994cee` verifies) — sign as usual.

## Constitution check — frozen guarantees

| Guarantee | Verdict |
|---|---|
| 0001 accept == stored | **Strengthened**: the `250` is written only after a commit that both landed and applied; the refusal stores nothing anywhere. |
| 0002 await subscribes before it checks | Untouched: `wait_for` is not edited; a refused mail never enters memory, fires no watcher, and the rescan finds nothing. |
| 0003 pruning is oldest-only | Preserved: front ids peeked in order; `pop_front` still evicts; `emails_dropped` counts each eviction exactly once at its commit; no eviction touches a delivery in flight. |
| 0004 per-test inboxes | Untouched (wire legs use per-test inboxes and AUTH usernames). |
| 0010 (proposed) off-worker commits | Built upon: the whole mutation still runs on the blocking pool; `synchronous = FULL` and reply-after-outcome untouched; panic symmetry stays via the moved seam. |
| 0012 (proposed) | Amended in place (§4/§5/§6) — its §1-§3 refusal machinery is what this run's refusal paths *use*; §7's fail-stop stands. |
| Exact 100/100/100 gate | Held: every new production line gets an executing test; no `coverage(off)`; no new `src/` file; the denominator never shrinks. |

## Project structure — files touched

| File | Change |
|---|---|
| `src/store.rs` | The boundary: widen `mutation`'s span; reorder `insert` to plan → commit → apply; commit-first `delete`/`clear`/`clear_all`; `Result` signatures; the `?`s; the gate moved pre-commit; the 16-thread probe added inline. |
| `src/api.rs` | `clear_inbox`, `delete_message` map `Err` → `503 storage commit failed` (become `ApiResult`). |
| `src/compat.rs` | `delete_messages` moves to `delete_async` and maps `Err` → `503`; delete-all maps `Err` → `503`. |
| `src/mcp.rs` | delete/clear/clear_all map `Err` → `error_result("storage commit failed")`. |
| `src/pop3.rs` | `QUIT` maps the first `Err` → `-ERR storage commit failed`, stays in TRANSACTION, marks unapplied; no sign-off. |
| `src/persist.rs` | **Zero lines.** |
| `src/smtp.rs`, `src/ui.rs`, `src/stdio.rs` | **Zero lines.** |
| `src/store/persist_tests.rs` | Rewrite the degraded test to the zero-trace refusal; delete the two gate race tests; port the two pins to the pre-commit gate. |
| `tests/persistence.rs` | Rewrite the two blocked-commit legs to absence; add trigger-based C2/C3 legs, the no-webhook leg, the counter-refusal leg; retire the `:401-407` debt note. |
| `tests/{mcp,compat,pop3}.rs` | Failure-mapping legs per adapter. |
| `tests/common/mod.rs` | A trigger helper (`refusing_trigger(db, kind)`) wrapping real `CREATE TRIGGER … RAISE(ABORT)`. |
| `docs/decisions/0012-…md` | Amended in place (§4/§5/§6 + the bench number). |
| `docs/decisions/README.md` | The missing 0012 row (and the missing 0011 rows, flagged to review). |

## The edits, precisely

1. **`insert`** — inside one `let _guard = self.mutation.lock()…`:
   `let list = self.inboxes.get(&inbox)`; `overflow = if cap > 0 {
   (list.len() + 1).saturating_sub(cap) } else { 0 }`; pruned ids = the
   front `overflow` of the deque **by iterator** (peek, never pop; also
   never `entry().or_default()` — no phantom inbox); if `persist` is
   `Some`, `record_insert(&email, &pruned_ids, inserted+1, dropped+overflow)?`;
   then apply: `push_back`, `pop_front × overflow` (removing those ids from
   the index), `index.insert(id)`, `fetch_add` ×2; drop the guard; then
   watcher send + `global_tx.send` (outside the commit, as today). `Err`
   returns before any memory touch.
2. **`delete`** — `record_delete(id)?` first, then the existing memory
   removal (index remove → deque retain → watcher release); return
   `Ok(removed.is_some())`. An absent id still mirrors (a no-op `DELETE`
   that cleans any orphan row) and stays `Ok(false)`.
3. **`clear` / `clear_all`** — snapshot the ids (clone id strings under the
   lock), `record_deletes(&ids)?`, then `drain_inbox` / per-inbox drains +
   `watchers.clear()`; return `Ok(count)`. The gate fires between snapshot
   and commit — inside the boundary, under the no-re-entry contract.
4. **Signatures** — `delete`/`delete_async` → `Result<bool, rusqlite::Error>`;
   `clear`/`clear_async`/`clear_all`/`clear_all_async` →
   `Result<usize, rusqlite::Error>`; the eight call sites map per the spec's
   table; ~30 inline-test call sites take mechanical `.unwrap()`s (their
   `Ok` paths are unchanged). `record`'s log line's "memory store remains
   authoritative" wording flips to the truth: refused, nothing stored,
   committed state preserved.
5. **Coverage** — every new arm has an executing test: refusal arms via
   `fail_writes()` (harness) and triggers/lock-holder (wire); apply paths
   via the existing restore legs + the probe; the POP3 `-ERR` arm, MCP
   `isError` arms, REST/compat `503` arms via their adapters' legs. The
   moved gate stays covered by the two ported pins.

## Test placement (decision 0011)

- The 16-thread barrier probe and the delete/clear/clear-all interleaving
  rounds are **inline** in `src/store.rs` — branch-stable deterministic
  interleavings (barrier → join → assert), warm-path, 0011 rule 1.
- The refusal/failure paths that exist only while storage betrays are
  **harness** tests in `src/store/persist_tests.rs` (0011 rule 2) —
  `fail_writes()` makes every commit fail instantly; no sleeps, no
  wall-clock validity assertions, fresh `Store::open` per test, no shared
  global state.
- The wire-shaped races and adapter contracts live in `tests/` — real
  triggers on the real data file, a second connection holding the write
  lock, real restarts; per-test inboxes; no mocks (0004).
- The seam stays `#[cfg(test)]`-only on `Store`, `pub(crate)`; the two pins
  remain its executors (0011 rule 4's "every seam keeps at least one test
  that installs it").

## Risks

- **Deadlock by re-entry inside the boundary.** *Mitigation:* the boundary
  contains no callbacks and no `await`; lock order is one-directional
  (`mutation` → `persist::State`, no path back); the gate's no-re-entry
  contract is documented in 0014 §6 and 0012 §6, and the ported pins prove
  the pre-commit timing works (a gate that blocked *and* re-entered would
  hang the pins' own deadlines — `join_with_deadline` turns that into a
  failure, not a hang).
- **The C2/C3 wire legs deadlock or stall the server under a trigger.**
  *Mitigation:* `RAISE(ABORT)` fails the statement instantly — no busy
  wait; legs use their own inboxes; the trigger is dropped before the
  recovery leg; `serve_data_file`'s NOOP liveness bounds any wedged leg.
- **A `BEFORE DELETE` trigger also fails capped inserts** (the prune leg
  lives in the same transaction). *Mitigation:* C3 legs run with
  `max_per_inbox = 0` (or an inbox under cap) so no prune is attempted;
  documented in the leg comments; the C2/C1 legs keep cap = 1 with an
  `INSERT`-only trigger.
- **The probe is flaky (red on a fixed tree, green on the bug).**
  *Mitigation:* the barrier pins the interleaving window; cap = 1 makes
  every insert a prune; the red is recorded three times before the reorder
  lands (the issue's own bar); the wire race leg is the long-window
  fallback. If the probe proves unreproducible after honest effort, stop
  and investigate — never tune the assertion (the store-boundary probe's
  standing rule).
- **Regression in the 0012 surfaces** (SMTP `451`, REST/MCP seed refusal)
  while their callers' signatures ripple. *Mitigation:* SMTP and the seed
  mappings are byte-untouched; the existing blocked-commit legs keep
  running and are only *extended* (absence assertions added), never
  weakened.
- **Throughput regression from the wider span.** *Mitigation:* commits were
  already globally serialized by the persist mutex; the span adds only the
  memory-phase/commit-tail overlap. `tests/burst.rs` stays a gate;
  `benches/ingest.rs --quick` runs in the verify gate; the number is
  recorded in the 0012 amendment (AC5).
- **Scope creep into the second run** (lifecycle, offloading, chart,
  contract-agreement, CI). *Mitigation:* the task list carries the scope
  guard; review refuses anything outside the storage-consistency slice.

## Data model & contracts

- Storage: unchanged schema (`emails` with `seq`, `meta` counters);
  unchanged mirror transaction shape (row + exactly-id pruned deletes +
  counter upsert, all-or-nothing). The boundary is sequencing, not schema.
- Wire contract deltas: `DELETE /api/v1/messages/{id}` and the inbox
  clear/clear-all routes gain `503 {"error":"storage commit failed"}` on
  mirror failure (bodies keep 0012 §2's fixed text; `404`/counts keep
  their `Ok` meanings); MCP gains `isError: true` results with the same
  text; compat shims gain `503`; POP3 `QUIT` gains `-ERR storage commit
  failed` with the session left in TRANSACTION. No new SMTP string — DATA
  already speaks `451 4.3.0 storage commit failed`.
- Rust contract deltas: `Store::{delete,clear,clear_all}` and their `_async`
  wrappers return `Result<…, rusqlite::Error>`; `insert`'s shape is
  unchanged from 0012.

## Quickstart (reviewer)

```bash
GIT_CONFIG_COUNT=0 bash scripts/coverage-gate.sh                 # exact 100/100/100 (AC6)
cargo test --lib                                                  # probe + harness (AC1, AC2)
cargo test --test persistence                                     # wire legs (AC1-AC4)
cargo test --test mcp --test compat --test pop3                   # adapter mappings (AC3)
cargo test counters_keep_counting_across_restarts                 # AC4
cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings
brokkr compile --bundle bundles/self && cargo deny check licenses
cargo bench --bench ingest -- --quick                             # AC5 number + AC6
git show --stat                                                   # AC6: no new src/ file, no new dep
```
