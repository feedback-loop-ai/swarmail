# Plan — epic #13: one Store seam, two contracts (commit-failure refusal + the mutation boundary)

Companion to [`spec.md`](spec.md) (WHAT/WHY) and decision
[`0012`](../../docs/decisions/0012-commit-failure-refusal-and-the-mutation-boundary.md)
(status `proposed`). How the two contracts land: the exact edits, the files
they touch, the tests that prove them, and the risks with mitigations.

## Technical context

Anchors read from the working tree on `feat/epic13-01-02-boundary` and
re-verifiable in seconds:

- `src/store.rs` — the whole feature's spine. `drain_inbox` (`:103-119`);
  `record` (`:243-253`, the swallow); the `pre_mirror_gate` seam
  (`:255-267`, `#[cfg(test)] pub(crate)`); `insert` (`:326`, `index.insert`
  `:358`, counter `:359`, `record` `:362-368`, watcher send `:375-377`,
  `global_tx` send `:378-379`); `delete` (`:455`, the
  `expect("indexed mail always has its inbox list")` at `:464`,
  `record` `:473`); `clear` (`:478`, gate + `record` `:493`);
  `clear_all` (`:498`, `record` `:521`); `offload` (`:536`);
  `insert_async` (`:554`). Inline test mods + the
  `#[cfg(test)] pub(crate) fn email` fixture; the harness module
  `src/store/persist_tests.rs` (`mod` declared beside its owner).
- `src/persist.rs` — the mirror. WAL + `PRAGMA synchronous = FULL`
  (`:84-87`); `State`'s `Mutex` (`:77`, guarded accessors `:100,122`);
  `record_insert` (one transaction, all-or-nothing);
  `fail_writes` (`:192-196`, `#[cfg(test)] pub(crate)` — PRAGMA
  query_only, fails instantly). Its poison vocabulary:
  `.expect("persist state poisoned")`.
- `src/smtp.rs` — the DATA path (`:405-463`): chaos DATA strictly
  pre-mutation (`:439-452`, default line `451 4.3.0 chaos` at `:448`);
  `insert_async` awaited then an unconditional `250` (`:461-462`).
- `src/api.rs:454` (seed → `insert_async`), `src/api.rs:36-45`
  (`ApiError(StatusCode, String)` + `IntoResponse`), `src/mcp.rs:202`
  (seed) and `src/mcp.rs:56-58` (`error_result`, already used for a
  refused fixture at `:197`).
- `tests/persistence.rs` — the wire suite for the data file:
  `data_file`/`clean_up`/`serve_data_file` helpers (`:13-37`),
  the mirror-image debt note (`:401-407`), heavy write legs
  (`:540-586`). `tests/common/mod.rs` — real-protocol helpers
  (`SmtpConn::data` asserts the `354` first, then returns the final
  reply, so a `451` final line flows straight through it) and uses
  main-dependency crates (`tokio_rustls`), so `rusqlite` in
  `[dependencies]` is available to test targets.
- `benches/ingest.rs` (`:36`, `:78`, `:117` — `insert` call sites; the
  `insert@cap-steady` canary at `:120-130`) and `benches/writer_stall.rs`
  (`:124`, `:166`, `:182`).
- `scripts/coverage-gate.sh` — exact 100/100/100; run it with
  `GIT_CONFIG_COUNT=0` (the documented sidestep for the broken harness
  `GIT_CONFIG_KEY_0`). Worktree facts: `commit.gpgsign` resolves **false**
  (recent history is unsigned); commit like the rest of this worktree's
  history.

## Constitution check — frozen guarantees

| Guarantee | Verdict |
|---|---|
| 0001 accept == stored | **Strengthened**: the `250` is written only after a known-good commit; the refusal replaces the false acknowledge. The memory half is unchanged. |
| 0002 await subscribes before it checks | Untouched. `wait_for` is not edited; the ghost fires no channel and is found by the store-is-truth rescan. |
| 0003 pruning is oldest-only | Preserved. Push and prune still share the shard write guard; `pop_front` still evicts; `emails_dropped` counts exactly (AC 8). |
| 0004/0005 | Untouched. Per-test inboxes still via SMTP AUTH; extraction still at ingest (`build_email` runs before `insert` either way). |
| 0010 (proposed) | Built upon: `PRAGMA synchronous = FULL` and the reply-after-outcome ordering untouched; `Err` flows through the same `offload`; panic symmetry stays. |
| Exact 100/100/100 | Held. Every new production line has an executing test; no `coverage(off)`; the denominator never shrinks; no new `src/` item exists to serve tests (AC 9). |

## Project structure — files touched

| File | Change |
|---|---|
| `src/store.rs` | `record` → `Result`; `insert` → `Result` with `?` before fan-out; one `Mutex<()>` boundary over the four memory phases; the 8-thread probe (inline). |
| `src/smtp.rs` | The DATA reply `match`: `250` on `Ok`, `451 4.3.0 storage commit failed` on `Err`. |
| `src/api.rs` | Seed maps the `Err` to `ApiError(SERVICE_UNAVAILABLE, "storage commit failed")`. |
| `src/mcp.rs` | Seed maps the `Err` to `error_result("storage commit failed")`. |
| `src/store/persist_tests.rs` | Rewrite `a_failing_data_file_never_blocks_the_store`; `let _ =` ripples on the other inserts. |
| `tests/common/mod.rs` | One forcing helper: open a second connection on a data file and hold the write lock, returning a guard. |
| `tests/persistence.rs` | Three refusal legs (SMTP DATA, REST seed, retry-250) + `let _ =` ripples. |
| `tests/mcp.rs` | One refusal leg (MCP seed). |
| `tests/` (other suites) | Only the mechanical `let _ =` ripple where a test drives `insert` directly (none do — the suites speak the wire). |
| `benches/ingest.rs`, `benches/writer_stall.rs` | `insert` takes the `Result`; `.expect(...)` per site. |
| Inline `#[cfg(test)]` mods in `src/` | The `let _ =` ripple (~50 sites), mechanical. |

Zero new files, zero new types (`rusqlite::Error` reused; one `Mutex<()>`
field), zero new dependencies (`parking_lot` refused — the critical
section holds no I/O). No `tests.rs`/`*_tests.rs` file is added.

## Contract #1 — the exact edits

1. `record`: `fn record(&self, what: &'static str, write: ...) ->
   rusqlite::Result<()>` — keep the `tracing::error!`, then
   `Ok(())`/return the `Err`. Its doc comment changes from "logged and
   swallowed" to "logged and returned" (insert propagates it;
   `delete`/`clear`/`clear_all` discard it, still non-silent).
2. `insert` (`src/store.rs:326`): `pub fn insert(&self, email: Email) ->
   Result<Arc<Email>, rusqlite::Error>`; the `record("insert", ...)`
   becomes `self.record("insert", ...)?` — the memory phase (push, prune,
   index insert, counters) is **before** it and unchanged, the fan-out
   (`:375-379`) is **after** it, so a refused mail fires neither channel.
3. `insert_async` (`:554`): the return type becomes
   `Result<Arc<Email>, rusqlite::Error>`; `offload` is already generic
   (`T: Send + 'static` — `rusqlite::Error` is).
4. `delete` (`:473`), `clear` (`:493`), `clear_all` (`:521`):
   `let _ = self.record(...)` — the only diff at those three lines.
5. SMTP DATA (`src/smtp.rs:461-462`):
   `match store.clone().insert_async(email).await { Ok(_) => write_line(..., &format!("250 2.0.0 OK: stored as {id}")).await?, Err(_) => write_line(..., "451 4.3.0 storage commit failed").await? }`
   — the reply still follows the outcome (0010), one arm per outcome, and
   the `id` is only named on acceptance.
6. REST seed (`src/api.rs:454`):
   `let stored = state.store.clone().insert_async(email).await.map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "storage commit failed".to_string()))?;`
   then `Ok(Json((*stored).clone()))`.
7. MCP seed (`src/mcp.rs:202-205`): map the `Err` to
   `Ok(error_result("storage commit failed"))` — mirroring the refused
   fixture arm at `:197`.

## Contract #2 — the exact edits

- Field: `mutation: std::sync::Mutex<()>` on `Store` (documented: the
  memory phase of the four mutations is one unit; never held across the
  commit, an `await`, or a gate callback; readers stay lock-free).
- `insert`: acquire before the entry-guard block, release after
  `index.insert` + the counter; the `?` on `record` and the fan-out stay
  outside.
- `delete` (`:455-473`): acquire before the index remove, release after
  the retain + watcher release; `let _ = self.record(...)` outside.
- `clear` (`:478-493`) / `clear_all` (`:498-521`): acquire before the
  drain(s), release **before** `run_pre_mirror_gate()` and `record` — the
  gate's closures re-enter the store, so a boundary spanning the gate
  deadlocks `a_clear_racing_an_insert_cannot_erase_the_inserts_row`
  (`src/store/persist_tests.rs:113`) by construction. Those two existing
  tests are the guard: they still pass unmodified.
- Guard acquisition: `.expect("store mutation lock poisoned")` — the
  house's existing poison vocabulary. The locked regions contain only
  DashMap ops and an `AtomicU64` increment, so poisoning means an
  invariant already broke; fail-stop is honest.
- The probe (inline `#[cfg(test)]` in `src/store.rs`, per 0011 rule 1 —
  it drives existing warm branches only): N rounds, each round a fresh
  `Store::new(0)`; eight threads with barrier-sampled roles — three
  inserts on `A`, one delete on `A`, one clear on `A`, two inserts on
  `B`, one `clear_all` — joined each round. At each round's end and at
  in-round barriers: for every inbox, `count(inbox) == |retrievable ids|
  `, no thread lost an accepted id, no panic propagated through `join`
  (which is the `:464` expect red), no stuck threads (the test's own
  join). ~50 rounds keep the window sampling dense and the runtime in
  milliseconds. Red today: the ghost (`count == 0` while `get(id)` is
  `Some`) and the `expect` panic both fire under hammering; green after
  the boundary under every schedule.

## Test placement (decision 0011)

- **Rewritten degraded test → `src/store/persist_tests.rs`** (the
  existing harness file): the refusal branch is a cold failure path — it
  runs only while writes fail — so it is unwritable inline; its home is
  the harness beside its owner, and the production lines it executes
  (the `?` early return, the fan-out skip) still feed the exact report.
  The rewrite: install `fail_writes()`; subscribe both channels first;
  assert `insert(...).is_err()` (red on today's tree); assert the
  memory-truth block (count/get/counters) unchanged; assert both
  channels stayed empty (`try_recv` — AC 4); keep the existing
  delete/clear/clear_all lines (AC 6).
- **Wire legs → `tests/persistence.rs` + `tests/mcp.rs`**: public-API,
  wire-shaped — real protocol against real servers, no mocks (0004).
- **The probe → inline** (branch-stable deterministic; no cold path).
- The forcing helper → `tests/common/mod.rs` (already `#![allow(dead_code)]`
  per binary). No new test file anywhere.

## The wire forcing mechanism (AC 9)

One helper in `tests/common/mod.rs`:

```rust
pub struct WriteLock { _conn: rusqlite::Connection }   // guard: drop rolls back
pub fn hold_write_lock(db: &Path) -> WriteLock {
    let conn = rusqlite::Connection::open(db).unwrap();
    conn.execute_batch("BEGIN IMMEDIATE").unwrap();     // EXCLUSIVE ≡ IMMEDIATE in WAL
    WriteLock { _conn: conn }
}
```

With the lock held, the server's real commit on the real data file fails
with a real `SQLITE_BUSY` after its own 5 s busy handler
(`rusqlite-0.32.1/src/inner_connection.rs:119`) — no mock, no injected
state, no new production API, no privileges. `PRAGMA query_only` from
outside is per-connection (unreachable); chmod read-only binds at
`open()` (an open fd keeps writing); `RLIMIT_FSIZE` raises `SIGXFSZ` and
kills the process under test; a filled tmpfs needs mount privileges — all
rejected. `benches/writer_stall.rs` proves the write-lock shape already
serializes this way under load.

Three legs over one held lock (one window, ~15 s of honest stall):

1. `tests/persistence.rs` — SMTP: `SmtpConn::data(...)` → the final reply
   starts with `451` and contains no `250`; then drop the guard and retry
   the same DATA → `250 2.0.0 OK: stored as …` (the retry leg is the
   argument for `451` over a permanent 5xx).
2. `tests/persistence.rs` — REST seed: `POST /api/v1/<inbox>/seed` under
   the lock → `503` with `{"error":"storage commit failed"}`; the refused
   mail is still queryable (memory truth), and the restart copy does not
   contain it (the refused id is absent after `srv.stop()` + reopen —
   asserting the lie is gone, not papered over).
3. `tests/mcp.rs` — MCP seed: `tools/call swarmail_seed_email` under the
   lock → `result.isError == true`.

The MCP/REST legs reuse the same server and the same guard; each leg
asserts the memory-truth side (`GET /api/v1/inboxes/<inbox>/messages`
still lists the refused mail) so the spec's "refused, not lost" posture is
on the wire too.

## Risks

- **The wire legs add ~15–20 s of wall time** (three 5 s busy-timeout
  stalls) to `cargo test` and to the coverage gate. *Mitigation:* one
  lock window serves all three legs (the stalls overlap nothing — they
  are sequential by construction); the suite already carries multi-second
  legs (`tests/persistence.rs` races 4 s windows and sleeps 1.5 s); the
  stall is bounded by `sqlite3_busy_timeout`, never unbounded; and no
  sleep-as-sync enters any test (the stall is the *server's* busy handler,
  a real deadline, not a test's `sleep`).
- **A flaky probe** (a red that depends on the schedule). *Mitigation:*
  the probe asserts an invariant that holds under *every* schedule once
  the boundary lands, so it is green-deterministic; the red side is made
  dense by ~50 rounds per run, and the task list requires a recorded red
  run against the unfixed store before the boundary lands — if a red run
  cannot be produced, stop and investigate, never tune the assertion.
- **A deadlock regression from the boundary** (the gate closures
  re-enter the store). *Mitigation:* the release-before-gate ordering is
  a load-bearing rule (decision 0012 §6), and the two existing
  clear-race tests exercise the gate on every `cargo test` run — a
  boundary spanning the gate hangs them immediately; the probe joins all
  eight threads, so a stuck boundary fails the test, not the CI timeout.
- **The `?` placement accidentally lands after the fan-out**, laundering
  the lie through the webhook port. *Mitigation:* AC 4's `try_recv`
  assertions make a late `?` red in the degraded test; the fan-out block
  is not touched at all — the `?` is a one-token change at `:362`.
- **The exact coverage gate refuses a new arm.** *Mitigation:* the arm
  map is closed: `record`'s return (degraded test), insert's `?` +
  fan-out skip (degraded test), SMTP `Ok`/`Err` arms (every SMTP test /
  the SMTP refusal leg), REST `map_err` (REST refusal leg), MCP
  `error_result` (MCP refusal leg), the boundary lock (probe + whole
  suite), the bench `expect` (the bench run). No arm exists without its
  executing test; the gate run itself is the check.
- **`rusqlite::Error` may not satisfy a bound `offload` needs**
  (`Send + 'static`). *Mitigation:* it implements `std::error::Error` and
  is `Send + Sync`; if a bound surprises, the fallback is a
  `Box<dyn Error + Send + Sync>` alias — but zero-new-types is the
  committed shape, so this is a compile-time check in task 1, not a
  design fork.
- **Scope creep at implementation time** (the two named debts —
  refused-delete resurrection, mirror-ordering/epoch — are one
  persuasive sentence away from "while we're here"). *Mitigation:*
  decision 0012 §5 and the tree's own note (`tests/persistence.rs:401-407`)
  name both as follow-up work; the tasks carry a scope guard like the
  0011 settlement did.

## Quickstart (reviewer)

```bash
GIT_CONFIG_COUNT=0 bash scripts/coverage-gate.sh   # exact 100/100/100 (AC10)
cargo test --lib a_failing_data_file_never_blocks_the_store  # AC1/AC4/AC5/AC6
cargo test --test persistence refusal              # AC2/AC3/AC9 wire legs
cargo test --test mcp seed                         # AC3 MCP leg
cargo test --lib store::tests::the_mutation_boundary_probe  # AC7 (name per landing)
cargo bench --bench ingest -- --quick              # AC8 canary + AC10 bench compiles
cargo clippy --all-targets -- -D warnings && cargo fmt --all -- --check
brokkr compile --bundle . && cargo deny check licenses
git show --stat                                    # AC9: no new src/ items, no new deps
```
