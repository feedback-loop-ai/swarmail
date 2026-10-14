# Spec — epic #13: one Store seam, two contracts (commit-failure refusal + the mutation boundary)

Epic [feedback-loop-ai/swarmail#13](https://github.com/feedback-loop-ai/swarmail#13),
contracts **#1 + #2** (`/tmp/epic13/feature-boundary.txt`, branch
`feat/epic13-01-02-boundary`). Intake framing:
[`.forge/intake/epic13-boundary-framing.md`](../../../.forge/intake/epic13-boundary-framing.md).
The panel's positions —
[`.forge/design/positions/simplicity.md`](../../../.forge/design/positions/simplicity.md) and
[`.forge/design/positions/robustness.md`](../../../.forge/design/positions/robustness.md)
(both `pass`) — are run-local evidence, gitignored.

Decision: **0012** —
[docs/decisions/0012-commit-failure-refusal-and-the-mutation-boundary.md](../../docs/decisions/0012-commit-failure-refusal-and-the-mutation-boundary.md)
(status `proposed` — acceptance is the operator's). The intake framing argued
no decision document was required; this spec disagrees on one axis — the
change re-shapes two public contracts (`Store::insert`'s signature and the
acceptance-path reply vocabulary) — and records the ruling anyway.

## The problem — a `250` over a failed commit, and a two-map mutation that is not one unit

Two lies and one race, all in `src/store.rs`:

1. **The acceptance lie.** `record` (`src/store.rs:243-253`) swallows the
   persist error with `"memory store remains authoritative"`, and the SMTP
   DATA path (`src/smtp.rs:461-462`) writes `250 2.0.0 OK: stored as {id}`
   unconditionally after `insert_async` resolves. A client completing
   `DATA` against a failing data file — full volume, read-only remount,
   `SQLITE_BUSY` — is acknowledged for a mail the restart copy will never
   hold. The memory store is truthful; the reply is not. The same lie is
   laundered further: the refused mail still fires the per-inbox watcher
   and the global webhook feed, so consumers act on a mail the client was
   never told was accepted.
2. **The race.** `insert` (`src/store.rs:326-380`) pushes to the per-inbox
   deque, prunes, and inserts into the global ID index as separate steps
   over two `DashMap`s; `delete` (`:455`), `clear` (`:478`) and
   `clear_all` (`:498`) likewise span both maps. Concurrent mutations can
   observe `count(inbox)` disagreeing with the set of ids `get(id)`
   retrieves. The two concrete reds an 8-thread barrier probe hits today:
   the **ghost** — an insert whose `index.insert` (`:358`) lands after a
   clear's `drain_inbox` (`:103-119`) leaves an index entry with no deque
   entry, so `count == 0` while `get(id)` is `Some`; and the **panic** —
   `delete`'s `expect("indexed mail always has its inbox list")` (`:464`)
   fires when a clear drains the inbox between delete's index remove and
   its deque retain.

## Ruling — how the two positions were reconciled

The seats converge on the load-bearing core and diverge on four additions;
this spec rules rather than averages.

**Adopted from both (convergent, stated once):** a failed commit refuses
and never acknowledges; the reply is the fixed line `451 4.3.0 storage
commit failed` — distinct from chaos's fixed `451 4.3.0 chaos`, internals
to the log, never to the peer; memory truth stands (no rollback); fan-out
gated on commit success is an explicit criterion, not an accident of where
an early return lands; the bench takes the `Result`; no latch, no
write-behind, no disk-first reordering (each refused for the reasons both
seats give).

**Adopted from simplicity — the refusal seam is `insert` only** (ruling
against robustness rider 1, "a fallible seam for all four mirror ops").
The framing's AC 1.1 names `insert`/`insert_async` and AC 1.5's
"no caller may silently re-swallow" governs *insert's* callers;
`delete`/`clear`/`clear_all` are not insert's callers. Widening drags
refusal shapes into REST `DELETE`, the compat shims and POP3 `DELE`
session semantics — a protocol change with zero framed criteria, each new
line owing its own leg under the exact 100/100/100 gate — and robustness's
own costs section concedes its rider does not close the resurrection it
cites (the disk row stays behind on refusal either way). **Cost of the
losing side:** a failed delete commit still answers `200`, and a restart
can resurrect mail the live store shows deleted until the disk recovers
and a retry lands. That hazard stays *named debt* — recorded in decision
0012 §5 and already noted in the tree at
`tests/persistence.rs:401-407` — not a silent gap: a follow-up epic owns
it if the operator wants it.

**Adopted from simplicity — the forcing mechanism is a second SQLite
connection holding the write lock** (ruling against robustness's
`PRAGMA max_page_count` pin). `max_page_count` is per-connection and not
persisted in the file, so it cannot be pre-set from outside;
`Store.persist` is private; `fail_writes` is `#[cfg(test)]` and does not
compile into the integration test target at all — their mechanism needs a
new test-only handle through the production API, which is the vocabulary
"a real failing data file" was framed to avoid. The write-lock mechanism
needs nothing from the server: `rusqlite` is in `[dependencies]`
(`tests/common/mod.rs` already uses main-dependency crates), the test owns
the data-file path, and a held `BEGIN IMMEDIATE` makes the server's real
commit fail with a real `SQLITE_BUSY` on the real file. **Cost of the
losing side:** each refused attempt stalls one bounded 5 s busy-timeout
(`rusqlite-0.32.1/src/inner_connection.rs:119`) — ~15 s across the three
refusal legs, honest wall time instead of a seam that fakes the failure.

**Adopted from simplicity — no `persist_failures_total` counter and no
poison-as-refusal** (ruling against robustness riders 5 and 6.3). Under
contract #1 the detector *is* the refusal — 451/503 on the wire, the log
carries `what` + the error; a counter is a new metric, a new production
line and a new test for no framed criterion. The house's poison vocabulary
is panic-and-propagate (`persist.rs` guards with
`.expect("persist state poisoned")`); a typed refusal for a poisoned
boundary is new semantics plus a new cold path. **Cost of the losing
sides:** an operator tailing metrics (not logs) sees nothing; and a panic
inside the boundary fail-stops the process rather than answering
subsequent mutations with a refusal — the honest posture
(`persist.rs` already does exactly this to its own callers), but it is a
harder failure than robustness's typed refusal.

**Adopted from robustness — the barrier probe spans inboxes** (ruling
against simplicity's "not an obligation"): the probe's roles run across
two inboxes with `clear_all` racing inserts on both. Not as a demanded
separate criterion, but as the probe's shape — it is a few lines inside
the one test, and the cross-inbox wipe is the exact multi-shard shape the
boundary exists for (`clear_all` drains every inbox and removes the
global index wholesale; a per-inbox design would pass the one-inbox probe
and still race). Simplicity concedes the few lines; the ruling takes them.

**Adopted from simplicity — the ground-truth correction:** the framing's
"restore path (`src/threads.rs:542`)" and "feed ingest
(`src/feed.rs:123,146`)" are inline **test** call sites, not production
paths. The production restore is `Store::open`, which deliberately
bypasses `insert` (no watchers, no webhooks, no re-prune) and is untouched
by the signature change. The production callers of the changed seam are
exactly three — `smtp.rs:461`, `api.rs:454`, `mcp.rs:202` — plus two
benches; the ~50 remaining call sites are inline tests, a mechanical
`let _ =` ripple forced by `Result`'s `#[must_use]`.

## The design — two contracts

### Contract #1 — refuse, don't lie

- `record` (`src/store.rs:243-253`) changes from `()` to
  `rusqlite::Result<()>`. It keeps its single `tracing::error!` on
  failure — the log stays the one non-silent witness at the one owner of
  the mirror.
- `insert` (`:326`) calls it with `?` and returns
  `Result<Arc<Email>, rusqlite::Error>` — no new error type
  (`rusqlite::Error` is already public via `Store::open`; insert has
  exactly one failure mode). The lifetime counters still count the
  refused mail (memory truth). The `?` sits **before** the watcher send
  (`:375-377`) and the `global_tx` send (`:378-379`): a refused mail
  fires no "accepted" event.
- `insert_async` (`:554`) inherits the `Result` through the existing
  generic `offload` (`:536`) — panic symmetry untouched, decision 0023's
  proven commit→reply ordering extended, not redone.
- SMTP DATA (`src/smtp.rs:461-462`): one `match` on the awaited result →
  `250 2.0.0 OK: stored as {id}` on `Ok`, **`451 4.3.0 storage commit
  failed`** on `Err`. RFC 5321 §4.2.2 "requested action aborted: local
  error in processing" — the canonical storage-failure reply, and the
  exact shape the file already speaks (the chaos stage's default is
  `451 4.3.0 chaos`, `src/smtp.rs:448`).
- REST seed (`src/api.rs:454`): map the `Err` to
  `ApiError(StatusCode::SERVICE_UNAVAILABLE, "storage commit failed")` —
  the `ApiError(StatusCode, String)` tuple already exists (`src/api.rs:36`);
  503 mirrors the 451's retry semantics. MCP seed (`src/mcp.rs:202`):
  `error_result("storage commit failed")`, exactly as the refused-fixture
  path one screen up already does (`src/mcp.rs:197`). No new error schema
  anywhere; the SQLite detail is in the log.
- `delete`/`clear`/`clear_all` keep their swallow as `let _ =
  self.record(...)` — still logged inside `record`, never silent, never
  re-shaped in this epic (decision 0012 §5).

### Contract #2 — one lock over the memory phase

- One field: `mutation: std::sync::Mutex<()>`. The memory-mutation phase
  of `insert` (push + prune + pruned-id index removals + global index
  insert + the counter), `delete` (index remove + retain + watcher
  release) and `clear`/`clear_all` (the `drain_inbox` loop) holds it.
- It is **released before `record` and before the `pre_mirror_gate`**,
  whose closures re-enter the store — a boundary spanning the gate would
  deadlock the existing clear-race tests
  (`src/store/persist_tests.rs:113,140`) by construction. Commits stay
  outside the lock: they are already serialized by `persist::State`'s own
  mutex (`src/persist.rs:14,122`), so holding it across them would add
  nothing but latency. No nesting order to remember: boundary ⊥ persist
  mutex ⊥ DashMap shard locks — none acquired inside another.
- Readers (`count`, `get`, `list`, `inboxes`, `wait_for`, `subscribe`)
  are untouched. The invariant is a rest-state invariant — "after any
  mutation completes" — so mutations being atomic units is sufficient;
  the barrier probe defines the sampling. `parking_lot` would be a new
  dependency for a critical section containing no I/O; refused.
- Pruning stays oldest-only (0003): push and prune still share the shard
  write guard, `pop_front` still evicts, `emails_dropped` still counts
  every eviction exactly, and the boundary changes nothing about *which*
  mail is evicted.
- Poison is fail-stop: `.expect("store mutation lock poisoned")` — the
  house's existing vocabulary (`src/persist.rs:100,122`).

### What this feature deliberately does not do

No memory rollback on refusal; no widening of the `Result` to
`delete`/`clear`/`clear_all`; no degraded-mode latch, retry queue or
write-behind (0023 §3 extends to the failure side); no disk-first
reordering; no per-inbox boundary lock (it cannot span `clear_all`'s
cross-inbox wipe and nests with `subscribe()`'s entry guard); no merge of
the two `DashMap`s; no reader-side locking; no `insert_checked` beside an
infallible `insert`; no new forcing seam (`fail_writes` stays exactly
where 0020 puts it); no commit-stage chaos event; no new metric. Each cut
carries its cost in decision 0012 §"Why" or in the ruling above.

## Frozen guarantees — posture

| Guarantee | Verdict |
|---|---|
| **0001** accept == stored | **Strengthened.** A `250` now additionally implies a known-good commit; refusal replaces the false acknowledge. The memory half (queryable after `insert`) is unchanged; the disk leg's ordering (0010) is untouched. |
| **0002** await subscribes before it checks | Untouched. The ghost never fires a watcher channel; a scanning awaiter finds it via the store-is-truth rescan — the same shape as every other stored-but-unnotified mail. `wait_for` is not edited. |
| **0003** pruning is oldest-only | Preserved. The eviction choice is byte-identical; `emails_dropped` still counts every eviction exactly (AC 8). |
| **0023** off-worker commits (*proposed*) | Built upon. `PRAGMA synchronous = FULL` and the reply-after-outcome ordering are not touched; the `Err` flows through the same awaited seam; panic symmetry stays. |
| Exact 100/100/100 gate | Held. Every new production line (the surfaced error path, the refusal arms, the boundary) ships with the test that executes it; `coverage(off)` stays forbidden; the denominator never shrinks. |

## Acceptance Criteria

- **AC1.** `Store::insert` and `insert_async` return
  `Result<Arc<Email>, rusqlite::Error>`; a mirror-commit failure surfaces
  as `Err` instead of being swallowed in `record`.
  *Red check:* the rewritten `a_failing_data_file_never_blocks_the_store`
  (`src/store/persist_tests.rs:88`) asserts the `Err` and fails against
  the current tree (which returns the mail and logs); it is green after.
- **AC2.** The SMTP DATA path writes `250 2.0.0 OK: stored as {id}` only
  after the commit outcome is known and only on `Ok`; on `Err` it writes
  exactly `451 4.3.0 storage commit failed` and no line containing `250`.
  *Proved by:* a wire leg in `tests/persistence.rs` — real protocol,
  real server, no mocks — where the held write lock forces the failure and
  the client completing `DATA` observes the refusal.
- **AC3.** The REST seed answers `503` with a body naming the refusal, and
  the MCP seed answers an `isError: true` result — neither re-swallows the
  surfaced error. *Proved by:* wire legs in `tests/persistence.rs` and
  `tests/mcp.rs` under the same forced failure.
- **AC4.** Fan-out is part of acceptance: a commit-failed mail fires no
  per-inbox watcher and no global/webhook broadcast. *Proved by:* the
  rewritten degraded test installs `subscribe("box")` and
  `subscribe_all()` before the refused insert and asserts both stay empty
  (`try_recv`) — red today, green after.
- **AC5.** Memory truth stands under refusal: the refused mail stays
  queryable (`get(id).is_some()`, `count` includes it), the lifetime
  counters count it, and there is no rollback, tombstone or latch.
  *Proved by:* the memory-truth assertions the degraded test keeps.
- **AC6.** `delete`/`clear`/`clear_all` keep their logged swallow and keep
  working under failing writes. *Proved by:* the degraded test's existing
  delete/clear/clear_all assertions, unchanged in meaning.
- **AC7.** After any completed `insert`/`delete`/`clear`/`clear_all`,
  `count(inbox)` equals the number of ids retrievable via `get(id)` whose
  inbox is that inbox — for every inbox. *Proved by:* the 8-thread barrier
  probe in `src/store.rs` (inline per 0020 rule 1): roles across two
  inboxes plus a `clear_all` spanning both, barrier-sampled per-inbox
  invariants, no panics, no stuck threads. The probe is **red** on the
  current tree (the ghost and the `:464` expect) and green after.
- **AC8.** Pruning stays oldest-only: the eviction choice is untouched and
  `emails_dropped` counts every eviction exactly. *Proved by:* the
  cap-steady canary in `benches/ingest.rs` (`insert@cap-steady` asserts
  `len == cap` and `dropped == inserted`) plus the existing pruning tests.
- **AC9.** The forcing mechanism is a second real SQLite connection
  holding the write lock on the real data file; **no production API,
  no `#[cfg(test)]` seam and no new dependency exist to serve the tests**.
  *Proved by:* `git diff main --stat` shows no new `src/` items, and
  `cargo deny check licenses` stays clean (zero new deps).
- **AC10.** All six gates are green on the final tree (`cargo fmt --all --
  --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
  `brokkr compile --bundle .`, `bash scripts/coverage-gate.sh` at exact
  100/100/100, `cargo deny check licenses`), and `cargo bench --bench
  ingest` compiles and runs with the `Result`-shaped `insert`.

## Verify gate

```bash
cargo test                                        # full suite, real servers
cargo test a_failing_data_file_never_blocks_the_store   # rewritten degraded test
cargo test --test persistence                     # wire refusal legs
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
brokkr compile --bundle .
bash scripts/coverage-gate.sh                     # exact 100/100/100, missed == 0
cargo deny check licenses
cargo bench --bench ingest                        # bench compiles + runs post-signature-change
```
