# 0024 — The boundary refuses to start a commit once stopping: one shutdown policy for mutations

- **Status:** proposed (awaiting operator acceptance)
- **Date:** 2026-10-14
- **Amends:** [0018](0018-shutdown-observed-waits-and-explicit-bounds.md)
  (accepted) §1's last sentence and §2's mutation mechanics **in place**, by
  the 0019 precedent: an accepted decision is pointed at, never rewritten —
  0018 carries the dated `Amended by` marker, and this document carries the
  amendment. (0018's own dates are corrected to their true 2026-10-13
  landing by this run's docs pass — a factual repair, not an amendment.)
- **Preserves:** [0001](0001-synchronous-insert-lossless.md) — the refusal
  precedes any acceptance: nothing was promised, so nothing is torn;
  [0002](0002-awaits-subscribe-before-checking.md) — the check reads the
  watch with the established `borrow_and_update` discipline;
  [0003](0003-oldest-only-pruning.md) — a refused insert evicts nothing;
  [0012](0012-commit-failure-refusal-and-the-mutation-boundary.md) §1/§2 —
  one public variant (`rusqlite::Error`), no new error type, the fixed
  refusal text on every surface, the SQLite detail a log concern;
  [0014](0014-one-ordered-mutation-boundary.md) §1/§2/§3 — the boundary's
  span, lock order and no-await rule are untouched (the check is a
  synchronous, lock-free read; no new serialization layer, no reordering);
  [0015](0015-boundary-failure-contract.md) §2 — the busy bound stays the
  dependency default, tripwired, never overridden; [0016](0016-lag-invariant.md)
  §4 — the tripwire stands.

## Context

Issue #32's reproduction: a server on a disposable SQLite data file, a
second connection holding `BEGIN IMMEDIATE`, three concurrent `POST
/api/v1/inboxes/{inbox}/seed` requests, then SIGINT with the lock held
through exit. Observed: shutdown took 15.386s; the requests refused `503`
at 5.169s, 10.355s and 15.556s. The arithmetic is the store mutex: the
mutations serialize on the boundary (`src/store.rs`, the `mutation` std
`Mutex`), each queued operation reaches the commit and burns its **own**
SQLite busy wait against the held lock — N queued = N × 5s. HTTP graceful
shutdown waits for the handlers; `RunningServer::stop()` joins the serve
task. ADR 0018's consequence sentence — *"a burning busy timeout does not
hold the shutdown — at most one commit lands or refuses after the fact"* —
is currently **false for queued HTTP mutations**: it holds for the SMTP
posture (sessions are aborted, sockets die) but not for a handler parked
inside axum's graceful drain. Slice 1 must make that sentence true, not
delete it.

The trap that decides the shape: the obvious small fix — wrapping a
handler's `insert_async(...).await` in `tokio::select!` — *detaches* the
already-spawned `spawn_blocking` task, and tokio's runtime drop **waits
for detached blocking tasks before `main` returns** (tokio 1.53.2,
`runtime/runtime.rs`: "The `Drop` implementation waits forever for
this"). The 15.386s does not disappear; it moves from axum's graceful
wait into the runtime drop at process exit — green on a `stop()` test,
red on the real SIGINT leg. The release therefore cannot live at an
adapter: it must happen before any blocking task exists.

The observed body — the fixed refusal text of 0012 §2 — was honest about
the *outcome* (nothing stored; the restart copy agrees) and silent about
the *cause* (a serialized busy wait per queued request). This decision
fixes the cause; the wire text stays.

## Decision

1. **One flag: the store reads the watch it already has.** `Store` gains
   `stopping: std::sync::OnceLock<tokio::sync::watch::Receiver<bool>>` and
   a one-shot `observe_shutdown` setter, wired once by `run_on` after the
   shutdown watch is born and before any listener binds. No constructor
   signature changes — the 46+ `Store::new` call sites keep their shape; a
   store never wired (every unit test, every embedding) keeps today's
   behavior exactly. The read is `borrow_and_update()` — synchronous,
   lock-free, no await ever enters the boundary (0014 §1). 0018 §1's
   *"The store alone does not observe shutdown"* is amended for the
   mutation half: the store is a **reader** of the same watch — one source
   of "stopping", never a second channel — while waits keep racing it at
   their adapters exactly as 0018 §1 fixed them. The policy is the
   boundary's, not the mirror's: a memory-only store refuses too (a
   stopping server acknowledges nothing, whatever it persists to).
2. **One check, at the chokepoint.** `record()` — the single commit
   chokepoint all four mutations pass through — refuses to **start** a
   commit once the stop has fired. The refusal is a constructed
   `SQLITE_ABORT` ("aborted prior to completion" — truthful: nothing was
   written); its only consumers are the existing `?` propagation and the
   existing fixed-text mappers, which are `Err(_)` and stay that way. The
   cause is a `tracing::info!` at the one owner of the mirror (`what`,
   "refused: shutting down; nothing applied"), never a peer-visible
   string. The refusal applies nothing: no mail, no inbox, no counter
   move, no eviction, no watcher or webhook fan-out (0014 §3's list,
   verbatim) — memory and data file agree.
3. **Three clauses, one policy.** *Admit*: a mutation reaching the
   boundary after the stop refuses in microseconds. *Cancel*: every
   queued lock-waiter is released from its wait **without one SQLite
   call** — each acquires the mutex in turn, hits the check, and refuses;
   a queued request burns a scheduling instant, not its own busy timeout.
   *Finish*: a commit that has already started is past the check; it runs
   to completion — SQLite transactions are atomic and the boundary is
   never torn. **The budget arithmetic**: the boundary serializes, so in
   every interleaving at most one mutation holds it when the stop fires
   and at most one check-passing commit starts afterwards — the process
   burns at most **one busy bound** (≤ 5s, 0015 §2, tripwired) after
   shutdown begins, plus ε per queued release. Inside 0018 §2's 10s
   budget with margin. The pre-existing residual — an aborted session's
   detached blocking task (0018 §2's SMTP posture) and the
   check-passed-then-flag-flipped race — is the same *at most one commit
   lands or refuses after the fact*, under the same bound.
4. **What an interrupted client may observe.** A client whose request was
   in flight when the stop began observes one of: *neither reply nor
   effect* (refused before the commit — the restart copy never holds it);
   *effect without reply* (the commit had already started — it lands
   after the socket died; the mail **is in the store** and the client saw
   EOF or a refusal, not acceptance); or, on a hard kill, it learns the
   outcome only from the restarted server. The effect may be durable even
   though the client saw no reply — and equally may not. Per request the
   outcome is **refused-or-committed, never partial**. Abandoned work is
   never claimed to have "not happened": a refusal says *not stored*, a
   landed commit says *stored, no reply delivered*, and retry is lossless
   but may duplicate — at-least-once is the named price of the client's
   move, the same posture 0018 §2 records for waits.
5. **The wire does not fork.** A shutdown refusal answers the fixed
   refusal text of 0012 §2 on every surface — the text means *nothing
   stored; retry*, which is true in both families (busy and stopping).
   `shutting down` stays the waits' answer (0018 §1). The cause
   distinction is a log and ADR concern, like every other SQLite detail.
   Refused with a price: mapping the shutdown refusal to per-surface
   `shutting down` texts would add an error-inspection arm at eight
   surfaces plus their tests, to distinguish causes whose observable
   window is nearly empty — graceful shutdown refuses new connections and
   the abort arm kills sessions, so a post-stop mutation arrives only on
   a connection that is already dying.
6. **Refused designs, with the price that kills each.** *Adapter-level
   `select!`* (race the handler against `shutdown_signal`): detaches the
   spawn_blocking task and moves the burn into the runtime drop — the
   measured trap; green on `stop()`, red on SIGINT. *Grant before
   offload* (a cancellable capacity-1 gate acquired in async land before
   `spawn_blocking`): a second serialization layer wrapped around the
   boundary — new lock choreography (`grant` outer → `mutation` inner, a
   release-under-cancellation divergence when an aborted session leaves a
   detached task holding the mutex) at the hottest code in the crate,
   eight+ surface edits and their tests — for a budget bound the
   chokepoint check already delivers; its one real benefit, queued
   mutations parking in async land instead of on blocking-pool threads,
   is **orthogonal to shutdown** (the pressure exists without any stop)
   and owes its own decision if ever wanted. *Busy-timeout overrides*
   (0015 §2), *shutdown deadlines or abort-on-timer* (0018 §2), *bounded
   writer queues* (0012 §7 / 0014 §7), *a constructor-signature change*
   (46+ mechanical edits for a feature only the server needs), *a
   caller-side re-check loop* (the boundary is the one place every
   mutation passes; a second check would be a second policy).

## Consequences

- The issue's scenario completes in ≈ one busy bound + ε: the first
  mutation finishes (or refuses) its burn, every queued mutation refuses
  in microseconds, the handlers resolve, the graceful drain completes,
  and `stop()` — and the process — exit inside the documented budget.
- 0018 §2's consequence sentence is now true for HTTP mutations, not
  only for the SMTP posture.
- The fixed `503 {"error":"storage commit failed"}` body survives, and
  stays honest: the restart copy of a refused set is empty, and a landed
  commit is complete. The log names the cause; the ADR documents it.
- New production code: one field, one setter, one check, one wiring line.
  `src/persist.rs` ends at zero diff. No new type, no new dependency, no
  new wire string, no new `src/` file.
- Every new line is a deterministic branch: the unwired arm is the
  existing suite's behavior, the wired-not-stopping arm is every server
  test's traffic, the stopping arm is the new inline refusal probe —
  inline, measured, per 0011 rule 1; no cold path, no harness relocation,
  no `coverage(off)`.
- **Accepted costs, named:** queued mutations still park blocking-pool
  threads until the flag flips (pre-existing, orthogonal to shutdown —
  the grant design's benefit, refused above, is the designed next step if
  it ever matters); a hostile client still milks one spawn per post-stop
  mutation (microseconds); the wire cannot say *why* a refusal happened
  (the log does); `docs/decisions/README.md`'s 0018 row still says
  `proposed` where the file says `accepted` — status lines are the
  operator's, and the drift is left visible, not silently fixed.

## Landing tests

- Inline (`src/store.rs` `#[cfg(test)]`, 0011 rule 1): a wired store whose
  flag is set refuses `insert` with the constructed error and applies
  nothing — no inbox minted, no counter moved, no fan-out; the same store
  before the flag inserts normally (the false arm), and the existing
  suite pins the unwired arm.
- The `stop()` leg (`tests/lifecycle.rs`): real server, real data file, a
  second real connection holding `BEGIN IMMEDIATE`, three concurrent seed
  requests in flight to different inboxes, `stop()` under them —
  all three answer `503` with the fixed body, `stop()` completes inside
  `STOP_BUDGET` (upper bound only), and a fresh read of the file shows
  the refused set stored nothing.
- The process leg (`tests/binary.rs`): the real binary on a disposable
  data file, the test's own connection holding `BEGIN IMMEDIATE` through
  exit, three concurrent seeds, real SIGINT — exit observed within the
  budget (upper bound, sequence-independent), exit status success, and
  the restarted copy is exactly the committed set — here empty, every
  commit having refused against the held lock — never a partial row.
