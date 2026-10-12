# Feature Specification: Commits off async workers — `250` still means durable

**Feature Branch**: `off-worker-commits`
**Created**: 2026-10-11
**Status**: Draft (design phase — committed spec)
**Backlog Story**: feedback-loop-ai/swarmail#13 wave 4, closes #10 (coordinated with #1 + #2)
**Input**: The review's finding — with `--data-file`, the synchronous SQLite mirror commit runs
inside async worker threads, blocking the tokio runtime. The repair must use `spawn_blocking` (or
an equivalent dedicated writer path) so async workers are never blocked by disk, WITHOUT moving
the commit after the acceptance reply and WITHOUT fire-and-forget write-behind.

## The problem

`src/smtp.rs:455` calls `store.insert(email)` inside the SMTP session task (one `tokio::spawn`
per connection, `src/smtp.rs:91`). `Store::insert` (`src/store.rs:229`) mutates memory, prunes
oldest-only, then mirrors synchronously — `self.record("insert", …)` (`src/store.rs:251`) →
`Persist::record_insert` (`src/persist.rs:112`), a WAL transaction under
`PRAGMA synchronous = FULL` (`src/persist.rs:84`) — and `src/smtp.rs:456` writes
`250 2.0.0 OK: stored as {id}` on the next line. The fsync is correct (decision 0001's disk leg)
but it executes **on an async worker**: while one commit runs, every other task parked on that
worker — banners, `NOOP`s, long-poll `await`s, other sessions — stalls behind an unbounded
external wait. The same shape exists at all 12 production mutation call sites (`smtp.rs:455`;
`api.rs:349/367/459`; `mcp.rs:126/134/136/185`; `pop3.rs:357`; `compat.rs:254/268/269`). Nothing
is offloaded today (`grep -rn "spawn_blocking|block_in_place" src/` returns nothing), and
`src/persist.rs:12-13` documents the false claim that the commit "cannot stall the runtime".

This is a **placement defect, not a throughput defect**: commits already serialize on the persist
mutex and already ride the accept path. The defect is blast radius — the wait on the *outside
world's speed* (a disk that can be slow, hung, dying) is paid by threads that owe other sessions
work. The repair's measure is: **when the disk is slow, hung, or failing, exactly the sessions
waiting on commits pay — and nothing else does.**

## The feature

Every mutation (`insert`, `delete`, `clear`, `clear_all`) invoked from an async handler runs
**whole** on tokio's blocking pool via a single awaited seam — `tokio::task::spawn_blocking`
behind one private `offload` helper plus four thin `*_async` wrappers on `Store` — and the
mutation's result is known before the caller replies. Concretely:

- The four synchronous mutation methods stay **unchanged and single-owned**: memory insert,
  oldest-only prune (0003), the pre-mirror gate window, the mirror commit, and the fan-out move
  as one unit onto a blocking-pool thread. No memory-half/commit-half split.
- All 12 production call sites switch to the awaited wrappers. `src/smtp.rs:456`'s `250` write
  stays exactly where it is — immediately **after** the awaited mutation resolves.
- `src/persist.rs` gains no code: the persist `Mutex` remains the writer queue, tokio's blocking
  pool remains the writer threads, and `PRAGMA synchronous = FULL` (`src/persist.rs:84`) is
  untouchable. Only the module doc's false sentence is corrected.
- No fire-and-forget write-behind, no queue, no background flush, no eventual-consistency window:
  the mutation still runs synchronously to completion on the acceptance critical path — it merely
  stops *occupying an async worker* to do it. Decision 0001's substance (memory insert and disk
  commit both **before** the `250`) is preserved byte for byte.

## Why

- **Decision 0001 (accept == stored) must not weaken.** An SMTP `250` still means the mail is
  queryable **and** committed to disk. The guarantee's letter also survives: `Store::insert`
  remains synchronous; async handlers call new wrappers that complete the synchronous mutation
  before resolving. What changes is thread identity, and thread identity is not the guarantee —
  **ordering** (commit-before-acceptance, failure-before-acceptance) is. This interpretation is
  the ruling recorded in the decision document below.
- **The runtime is shared infrastructure.** A slow disk today pins a worker that long-poll
  watchers (0002's subscribers), healthy sessions, and the API need. The repair makes the disk
  wait cost *parked tasks*, not *blocked threads*.
- **Panic isolation and shutdown come free.** A panicking mutation unwinds one blocking task and
  re-raises in the one awaiting caller — exactly today's inline unwind, but isolated per commit.
  A dedicated writer *thread* would have the worst version: a panicked writer loop is a permanent
  wedge (every future `DATA` hangs), and a rendezvous actor needs a shutdown drain or
  `RunningServer::stop` hangs. `spawn_blocking` has neither problem.
- **The wave-2 constraints ride along unchanged.** Injected disk failure still surfaces before
  the `250` (`record()` logs and keeps memory authoritative *inside* the awaited mutation, so the
  observation is strictly before the reply byte), and the clear/insert race guard is untouched —
  the gate closure and drain-ids mirror stay exactly where they are.

## Ruling — adoption, rejection, reconciliation

The two positions converge on the spine: the whole mutation behind four awaited
`spawn_blocking` wrappers, reply after resolve, 12 call-site switches, one doc correction, no new
dependency, no new `src/` file, no mode branch. I commit that spine. Where they diverge, I rule:

1. **Stall proof — structural, not hardware-margined (adopt robustness A2, refine it).**
   The deterministic red test for "no async worker stall" is a structural pin (current-thread
   runtime + pre-mirror gate + tick assertions), *not* a real-server latency bound. A
   margin-tight real-server red flakes on tmpfs CI; a flaky red is worse than no red. Hardware
   numbers belong in the recorded measurement (criterion 5), where the honesty clause rules.
   *Refinement over both seats*: the pin's escape hatch must be a **watchdog that releases the
   gate on a deadline**, not a `tokio::time::timeout` — tokio timers cannot fire while the old
   inline design spins the wedged runtime thread, so a timeout alone would hang, not go red. With
   the watchdog release, the "ticks kept flowing while the gate was held" assertion fails fast
   and deterministically in every wrong direction (inline → wedged ticks; `block_in_place` →
   red panic on current-thread runtimes; fire-and-forget → resolved-while-held).
   *Cost of the losing side (simplicity's canary-latency real-server red)*: no protocol-level
   test is red against the defect on every machine. Mitigated: the structural pin is strictly
   stronger deterministically, the real-server write-heavy leg still asserts correctness under
   concurrency, and the bench carries the honest numbers.

2. **Hard-kill restart leg (adopt robustness A1).** The existing durability net structurally
   cannot see an ack-before-fsync / commit-after-`250` / write-behind / softened-pragma
   regression: `tests/persistence.rs` and `tests/binary.rs` restart only after *graceful* stops,
   and SQLite checkpoints the WAL at clean exit. A SIGKILL mid-burst restart test is the only
   mechanical net for 0001's disk leg — red against every one of those regressions, green against
   both the old and the new code, with no timing margins (only observed `250` ids are asserted,
   so where the kill lands only changes how many mails are checked). It is test-only: no
   production denominator, and `libc` is already a dev-dependency (`tests/binary.rs` precedent).
   *Cost*: one more integration test in the suite. Accepted.

3. **Where the injected-failure proof lives (reconcile both).** Three pins, three roles, all
   `#[cfg(test)]` in-crate because the seams (`Persist::fail_writes` is `pub(crate)`,
   `src/persist.rs:190`; the `pre_mirror_gate` setter, `src/store.rs:200`) are only visible
   there — and no production injection surface (an env-gated failure knob in the binary is a
   durability-breaking branch forever): the **ordering pin** (await does not resolve while the
   gate holds the commit; red against fire-and-forget and reply-before-commit), the
   **panic-symmetry pin** (a panicking mutation re-raises in the awaiting caller — which also
   deterministically covers the `offload` helper's `Err` arm at the exact 100/100/100 branch
   gate), and the **real-protocol leg** (a real listener + real `TcpStream` speaking SMTP with
   `fail_writes()` set: the `250` still arrives — log-and-accept preserved — and the row is
   absent on reopen, the wave-2 degraded restart copy). A1's SIGKILL leg is the protocol-level
   net for durability itself.

4. **The measurement (adopt simplicity's shape, robustness's honesty).** One bench comparing both
   paths in one process against the same real data file, metric = max overshoot of a 1 ms ticker
   while a write-heavy ingest loop runs, sync `insert` (the finding, isolated) vs awaited
   `insert_async`. The stall discriminator runs on a **current-thread runtime**, documented:
   on a multi-thread runtime the old path blocks one worker among many, so a ticker only stalls
   when it lands on that worker — precisely the nondeterminism the ruling in (1) refuses. The
   real-server probe's numbers are recorded alongside. **No batching, group commit, pragma or WAL
   tuning may be introduced to prettify the number** — that is write-behind by another name. If
   the measurement shows no clear win, the numbers are reported as measured and the operator
   rules; the quality defect stands regardless.

5. **Decision document — I overrule the simplicity seat here.** Simplicity rules "no decision
   document"; I commit one. The change re-places the mechanism that *is* 0001's disk leg and adds
   public contract surface (four `pub async fn` on the public `Store`), and the house rule is
   that the decisions directory is the authority on why the code is shaped the way it is —
   future readers will ask why `insert_async` exists beside `insert`, and the spec alone is not
   that authority. Status `proposed`; acceptance is the operator's. It records the interpretation
   ruling (the disk leg is ordering, not thread identity) and pins the two lines that must not
   move. *Cost of the losing side*: one more proposed record for the operator to read and accept
   — trivial against the cost of an unrecorded ruling on a frozen guarantee.

6. **Everything else both seats cut, I confirm cut**: a dedicated writer thread/actor (a permanent
   wedge on panic + a drain to get wrong, re-buying the mutex and the blocking pool);
   `tokio::task::block_in_place` (panics on current-thread runtimes, frees no worker); rewriting
   `Store` as async (churns the founding API and breaks the wave-2 gate closure, which is a sync
   `Fn()` inside the drain→mirror window); a memory-half/commit-half split (re-engineers exactly
   the window the clear-race tests pin); a mode branch ("offload only when persisting" — a
   permanent dual path to save a µs hop); a new failure protocol (4xx on disk failure — changes
   0001's log-and-accept semantics, its own decision); a `Persist` trait for testability (traits
   are how an exact-100% budget dies); a commit-latency histogram in `/metrics` (covered surface
   bought with gate budget; the tracing error path, A1 and the bench are the observability).

## Named windows and accepted costs

Named so review does not mistake them for regressions introduced by this change:

- **Commit → `250` widens by one task wakeup.** Today commit and reply are back-to-back on one
  thread; after the move there is a wake-and-resume between them (microseconds). A kill in that
  window leaves a stored, committed mail the client never saw — client retries, gets a duplicate
  id. The window exists today (crash before `write_line`); the reverse — a `250` without a
  commit — is impossible in both designs.
- **A distressed disk becomes SMTP latency, not worker stall.** Under a slow disk the await
  resolves late; sessions pile up as parked tasks and ingest latency converges to commit latency.
  That is the design working: unbounded waiting is the honest posture, the failure is visible as
  latency, not as a lie. The only alternative (bounded queue, `451` under pressure) is new
  accept-path semantics — decision-document territory, not asked for.
- **The memory half and fan-out also run on the blocking thread.** Same code, thread-safe
  ingredients; per-session order is preserved by the await; cross-session ordering was never
  guaranteed. Pre-existing race gets a name, not a fix: `record_insert` assigns `seq` under the
  persist mutex at commit time (`src/persist.rs:120-121`) while the memory push happened earlier,
  so two racing inserts to one inbox can swap restart order — today's behaviour across async
  workers already; the move changes its texture, not its kind.
- **Pre-existing named debt stays debt**: the mirror-image resurface hazard (push-before-drain,
  commit-after-delete, documented at `tests/persistence.rs:389-395`) — a single-writer FIFO actor
  would narrow it but would couple memory visibility to commit rate (mail N+1 queryable only
  after commit N — the exact coupling this change removes); the real fix (epoch/generation) is
  tracked separately. Poisoned-mutex semantics are unchanged (one mid-commit panic makes later
  commits fail via the existing `expect`s; memory stays authoritative; the session dies before any
  reply).
- **`build_email` (parse + extract) stays on the worker** (`src/smtp.rs:452-453`): bounded CPU per
  mail, not an unbounded external wait. Moving it too is scope creep with no guarantee gain.
- **Startup restore still blocks once** (`src/lib.rs:56-64`), before any listener binds — a
  blocked startup never answers from a half-restored store; keeping it synchronous is deliberate.
- **The µs-scale blocking-pool hop when no data file is set** is accepted: one unconditional path,
  no mode branch; `tests/rate.rs` (exact 5000/5000 over 100 sessions) has orders of magnitude of
  headroom; the founding ingest bench measures the untouched sync store.

## Acceptance Criteria

Each criterion is red-able or mechanically checkable; the framing's six, sharpened by the
rulings above.

1. **No async worker stall.** A structural pin (current-thread `#[tokio::test]`, pre-mirror gate
   holding the mutation in flight on the blocking pool, watchdog releasing it) proves the runtime
   thread keeps ticking while a mirror commit is held in flight, that the mutation does not
   resolve while held, and that it resolves after release. Red against an inline
   (worker-blocking) implementation, red against `block_in_place`, red against fire-and-forget;
   green only when workers are genuinely free. The pin rides the new wrapper vocabulary, so it
   cannot literally execute against today's tree — the property it pins is precisely the one the
   current inline shape violates (the gate's spin would occupy the runtime thread and wedge every
   tick), and it is red in every wrong direction a future edit could take.
2. **Commit-before-acceptance holds — injected failure still surfaces before `250`.** With
   `Persist::fail_writes()` set and a real listener + real SMTP client: the `250` reply is
   written only after the commit outcome is known — the mutation's future does not resolve while
   a gate holds the commit; after release, memory stays authoritative (log-and-accept), the mail
   is queryable over the wire, and the row is absent on reopen. The reply moves only if the
   commit moves after the reply or becomes fire-and-forget, which this pin makes red.
3. **Durability unchanged.** `tests/persistence.rs` stays green — mail answered `250` is restored
   verbatim after restart; prune/delete/clear mirrors intact; the clear/insert race guard
   (`a_clear_racing_inserts_never_loses_a_250_answered_mail_on_restart`) untouched and green.
4. **Hard-kill leg (A1).** The real binary with `--data-file`, bursted real SMTP mail with every
   observed-`250` id recorded, `SIGKILL` mid-burst, respawn on the same file: **every recorded id
   is in the restart copy.** Red against commit-after-`250`, write-behind, and
   `synchronous = NORMAL`; green against old and new code alike.
5. **All production entry surfaces.** No async caller (SMTP, REST, MCP, POP3, compat shims)
   executes a mirror commit on an async worker: all 12 call sites go through the one awaited
   seam, proven by the existing real-server suites (smtp/burst/rate/p2/api/mcp/pop3/compat/
   persistence) executing every wrapper, plus the structural pin on the shared path.
6. **Measurement recorded.** A repeatable bench (`benches/writer_stall.rs`) reports max ticker
   overshoot for sync-`insert` vs awaited-`insert_async` against the same real data file, and the
   real-server probe numbers are recorded with it — in a docs note or the PR body — verbatim, no
   tuning, no rounding. If no clear win is measurable, that is reported honestly and the operator
   rules.
7. **Gates green.** `cargo fmt --all -- --check`; `cargo clippy --all-targets -- -D warnings`;
   `cargo test` (full suite, real servers, no mocks); `brokkr compile --bundle .`;
   `cargo deny check licenses` (no new dependencies — the tree is unchanged); and
   `scripts/coverage-gate.sh` at literal missed == 0 in all three dimensions — every new
   production line executed by a test, the `offload` helper's `Err` arm included (via the
   panic-symmetry pin), and no `coverage(off)` anywhere.
8. **The two lines that must not move.** `src/smtp.rs:456` — the `250 2.0.0 OK: stored as {id}`
   write stays **after** the awaited mutation resolves. `src/persist.rs:84` —
   `PRAGMA synchronous = FULL` stays. Any softening of either is a decision document the operator
   signs, not an edit.

## Decision Documents

This change re-places the mechanism of frozen guarantee 0001's disk leg and adds public contract
surface, so it carries a decision record — **Decision: 0010**,
[docs/decisions/0010-off-worker-commits.md](../../docs/decisions/0010-off-worker-commits.md),
status `proposed`, acceptance the operator's. It records the interpretation ruling (the disk leg
is ordering and atomicity, not thread identity), the wrapper contract, and the two lines that
must not move. No other decision is touched: 0002 (await subscribes before checking) and 0003
(oldest-only pruning) are adjacent but receive no semantic change — the awaited mirror does not
reorder subscribe-before-check, and pruning stays inside the moved mutation under the same locks.

## Out of scope

Startup restore's one-shot blocking open; `build_email`'s placement; the resurface hazard's epoch
fix; poisoned-mutex recovery semantics; any new failure protocol; any pragma, batching or
group-commit tuning; a commit-latency metric endpoint; and the dedicated-writer actor in any
form. Each is named above with the reason it stays out.
