# Spec — total shutdown: every owned task tracked on stop, webhook fan-out bounded and loss-observable

Epic [feedback-loop-ai/swarmail#13](https://github.com/feedback-loop-ai/swarmail#13),
issues [#7](https://github.com/feedback-loop-ai/swarmail/issues/7) (shutdown is total)
and [#11](https://github.com/feedback-loop-ai/swarmail/issues/11) (webhook delivery is
bounded, lag-observable, best-effort with observable loss). This is the design sequence's
second round on the same frame: the first-round spec (commit `613747d`) was parked in
implement, the panel re-argued against the committed text
([`.forge/design/positions/simplicity.md`](../../../.forge/design/positions/simplicity.md)
and
[`.forge/design/positions/robustness.md`](../../../.forge/design/positions/robustness.md),
both `pass`), and this spec is the reconciled ruling. It supersedes the first round's
letter in exactly two places, named and costed below. Adjacent rulings referenced:
[0021-watcher-channel-lifetime.md](../../docs/decisions/0021-watcher-channel-lifetime.md)
(`proposed`, same subsystem) and
[0020-concurrency-tests-and-the-coverage-gate.md](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md)
(`proposed`, test placement); exact-count discipline per
[0010-exact-count-asserts-and-the-shared-seed.md](../../docs/decisions/0010-exact-count-asserts-and-the-shared-seed.md).

## The problem

`RunningServer.tasks` holds exactly the three serve tasks (`src/lib.rs:40`); `stop()`
awaits those three and returns. Everything else the process spawned is an orphan: the
webhook dispatcher, spawned at `lib.rs:67` — before the shutdown channel exists at `:95`
— and in nobody's tracker; every delivery spawned per (mail, target)
(`src/webhook.rs:189`); every SMTP/POP3 session spawned inside its accept loop
(`src/smtp.rs:91`, `src/pop3.rs:46`). An SSE feed is an untracked HTTP response that
graceful shutdown waits on forever: with one feed open, `stop()` never returns. Meanwhile
the fan-out downstream of insert is unbounded — one POST per target per mail, no cap, no
queue — and its losses are invisible: a firehose `Lagged` is a warn line
(`webhook.rs:159`), an exhausted retry budget is a warn line (`webhook.rs:215`), and
nothing anywhere counts. Issue 11's acceptance criterion — an exact count of undelivered
notifications — is unauditable today, not merely unmet.

This round's grounding adds two facts the first round got wrong or missed. First, the
loss the ledger must name is not hypothetical: a stalled sink burns a **22.1 s** budget
(four 5 s `POST_TIMEOUT`s plus 100/400/1600 ms backoff, `webhook.rs:204-216`), not the
~17 s the first round wrote — it is the expected worst case, not a tail. Second, a
hostile burst of large mails backed up in the 8192-ring makes the first-round design
serialize the entire backlog before honoring stop — the "microseconds per mail" cost
sentence held only for small mails.

## Ruling — how the two positions were reconciled

Both positions hold the load-bearing frame, and this spec holds it with them: **the
accept path is a moat.** Everything this feature adds lives strictly downstream of the
commit; every bound is `try`-shaped (drop-and-count, never wait-and-delay); every
cancellation is owned by its spawner; `stop()` must be as honest as `250`. Four points
genuinely conflict; this spec rules rather than averages, and names what each losing
side costs.

**Adopted from both: structural ownership — a `JoinSet` per owner.** `RunningServer`
owns the three serve tasks plus the dispatcher (`JoinSet` replaces the `Vec`, the
dispatcher's handle now returned and pushed in); each serve loop owns its sessions
(`abort_all()` + drain on the existing shutdown arm); the dispatcher owns its
deliveries. Dropping a `JoinSet` aborts its children, so ownership composes by
construction. Robustness formally withdrew its `Arc<Semaphore>` this round — a permit
acquired before a `tokio::spawn` can leak permanently if the spawn path panics or is
refactored around it, capacity silently lost for the process's life with no counter to
name it; `JoinSet::len()` cannot leak, it *is* the set, owned by the one task that
spawns into and harvests from it. A snapshot-and-chase abort list is refused: it races
the accept window it claims to close.

**Ruled against simplicity: the ledger is per-instance and exhaustive, not two
statics.** Simplicity re-advocated two `static AtomicU64`s (a `dropped` and a
`lag_skipped` row). Refused, on the same two grounds the first round ruled and this
round's code re-confirmed. A process-global static is this tree's cautionary precedent,
not its endorsement: `delivery_tests` already pays for the CA-cache statics with a
`CACHE_TESTS` mutex (`webhook.rs:462`) to keep parallel tests from cross-contaminating
exact asserts — and 0004 puts every test on its own inbox, with many `RunningServer`s
alive side by side, so a static webhook counter makes criterion 5's exact assert a
cross-contamination race by construction. And two counters are not exhaustive: a burst
larger than the firehose ring (8192, `store.rs:188`) dies through `Lagged`, which no
dropped counter sees — the exact count would be unauditable with the feature shipped.
Adopted: robustness's ledger — five per-instance counters on `Webhooks` (`delivered`,
`dropped`, `lagged`, `exhausted`, `aborted`), surfaced as five `swarmail_webhook_*_total`
rows on `/metrics`. Every death a notification can die has a name. **What simplicity
loses:** two rows become five, ~25 lines, and one small `Copy` snapshot type in the
`CaCacheStats` shape — the single casualty of its "zero new types" claim, since the
counters themselves are five plain fields on the existing `Webhooks` struct (no new sync
type, no new module). **What survives of simplicity verbatim:** the dropped row means
"refused at the bound" and nothing else — shutdown loss gets its own row, never mixed
into another's, so the exact-count asserts stay reproducible. `lagged`'s unit is mails,
not notifications (the ring evicted them before the dispatcher saw them; their fan-out
is unknowable by construction) — documented as such.

**Ruled for simplicity's instinct, against its edit as written: the watch moves up, not
the dispatcher down.** Simplicity would "move the `spawn_dispatcher` call below the
watch-channel creation" — as written that breaks subscribe-before-bind, because the
channel is created at `lib.rs:95`, *after* the binds at `:87-93`: a consumer created
after a bind can miss the webhook for a mail accepted in between, and the HTTP insert
API accepts mail too, so the window needs no SMTP. Ruled: the watch creation moves **up**
to the store's birthplace; the dispatcher spawns immediately after it, still before TLS
parsing and before any bind. This is decision 0002's discipline one layer down, frozen
here. Simplicity's underlying demand — the dispatcher must observe the watch — is
adopted; its edit would have manufactured the very miss the ordering exists to prevent.

**Ruled against robustness: the startup-failure "dispatcher leak" is refused — its
premise is false in this tree.** Robustness's fourth sharpening claims a failed startup
pins the `Arc<Store>` (and its sqlite handle) in a parked dispatcher task. The code says
otherwise: `spawn_dispatcher` consumes the store only to `subscribe_all()`
(`webhook.rs:199-202`) and spawns `dispatch_loop(rx, webhooks)` — no `Arc<Store>`
crosses into the task, so the parked task pins nothing but the receiver. The failure
path already self-resolves twice over: the store dropping on `run_on`'s error legs
closes the firehose (`Wake::Stop`, `webhook.rs:193`), and — once this spec lands — the
watch sender dropping resolves the dispatcher's shutdown arm. **What robustness loses:**
its sharpening; a detached dispatcher persists only until that drop-cascade,
microseconds after the `Err`. The one-statement `abort()` on the error legs is available
hygiene, but it would be a new error-leg line needing its own test to prove a
non-problem — refused on the coverage currency this realm rations.

**Ruled for robustness (amending this spec's own first round): `stop()` must not
serialize the backlog.** The first round ruled the dispatcher's select biased to the
firehose so "a buffered mail is always dispatched or dropped (and counted) before the
shutdown arm can win", costing "microseconds per mail". That cost sentence was true only
for small mails: the biased arm keeps building payloads for every buffered mail before
honoring shutdown, and a max-size mail's `json!` serialization is tens of milliseconds —
a hostile burst of large mails backed up in the 8192-ring turns `stop()` into a
minutes-long serialization campaign, firing criterion 2's deadline by design drift
rather than by bug. Superseded: **the `Mail` arm's first statement is the shutdown
check** (one atomic load, `has_changed()`); once stop is signaled, buffered mails are
dropped and counted `aborted`, never serialized. The stop arm drains what remains of the
ring with `try_recv` and counts each mail's matching targets — the exact sweep
robustness named but "did not require"; the ledger invariant the first round committed
("the ledger sums to the notifications presented") now requires it. The first round's
promise survives in its honest form: **nothing is silently skipped — every abandonment
has a name** — and the spawn-then-abort it replaces was a phantom attempt anyway (the
set drops on return; the process is exiting). **What is lost:** a mail in the final
window receives no delivery attempt. **What is gained:** `stop()` latency decoupled from
backlog × payload size. The honest residual, retained: a commit landing after the sweep
is uncounted — a window of microseconds between the sweep and process exit.

**Corrected from robustness: the budget is 22.1 s, and criterion 2's deadline sits
below it.** Four attempts × 5 s `POST_TIMEOUT` plus 100/400/1600 ms backoff = **22.1 s**;
the stalled-sink case (accept, never reply) burns all four timeouts, so it is the
expected worst case, not a tail. The first round's "~17 s" understates by a quarter and
every cost sentence is corrected. But the position's own deadline direction is inverted:
it writes "AC2's deadline must sit above 22.1 s or a drain masquerades as an abort" — a
deadline *above* the drain path is exactly what lets the drain masquerade. Ruled: the
deadline sits **strictly between** the abort path (milliseconds) and the drain path
(22.1 s); the tasks fix it at 10 s, so a drain fires it and an abort never comes near
it. Robustness's reasoning — the red line must be able to distinguish abort from drain —
is adopted entire; only its arithmetic direction was wrong.

**Adopted from robustness, uncontested: the cap is checked before the payload is
built.** As first written, the payload materializes before the bound check — a 50 MB
mail's `Arc<str>` is allocated even when the delivery is certain to be refused. Reordered
(check `deliveries.len() < WEBHOOK_DELIVERY_CAP` before the `json!` build): the drop is
free. Under a hostile large-mail burst at the cap that is real allocator churn on the
dispatcher for zero deliveries — the sort of thing that only shows up as a mysterious
RSS plateau under chaos.

**Ruled on the feed: cooperative end, not global abort** (both positions converge, the
first round's ruling stands). The SSE stream ends by folding the shutdown watch into
`feed_events`' unfold state (one select arm → `None`); the response completes, axum's
graceful shutdown finishes, and the tracked HTTP serve task returns — the only untracked
HTTP task is one hyper owns, and no per-connection handle exists short of replacing
`axum::serve`. Robustness's tolerated alternative (a global deadline-and-abort of the
HTTP serve task) stays the named fallback — second choice, never first: it kills benign
in-flight requests with the guilty. A frame may race the closing stream; the browser's
`EventSource` reconnect re-subscribes before its first scan (0002) and re-snapshots —
exactly the recovery 0021 §5 already documents. The `Closed` arm (`feed.rs:85`) remains
the only non-shutdown way a feed ends; the two arms coexist in one select and touch none
of 0021's machinery.

**The precise `stop()` promise** (first round's wording, standing, plus the counting
amendment above). `spawn_blocking` commits are uncancellable: a mail whose commit began
before its session was aborted can land after `stop()` returns. The client never saw the
250 — the reply is written only after the commit outcome is known (`smtp.rs:461-462`) —
so the outcome is a duplicate on retry, never a loss, and never a rollback of an unacked
session. A quiesce fence is refused. The test discipline this buys is mandatory: exact
post-stop counts assert only on inboxes the aborted session never touched (0010). The
ledger's rows are independent `Relaxed` observations: exact sums hold at quiescence —
which is exactly when tests assert — not under a live `/metrics` reader.

**A residual neither position named: the long-poll.** `/api/v1/inboxes/{inbox}/await`
(`api.rs:71`) holds its connection for up to the client's own `timeout_ms` (default
5 s, **uncapped** — `api.rs:156-158`). An open long-poll at `stop()` blocks axum's
graceful shutdown for exactly that client-chosen deadline: bounded by the client, not by
the server. The feed was unbounded and is fixed cooperatively; the long-poll is named
and left alone — capping `timeout_ms` would change a public contract, and waking
long-polls at stop would thread the watch through every handler, the dozens-of-branches
cost both positions refused. The named fallback (a `stop()`-wide deadline-and-abort)
covers it should a hostile `timeout_ms` ever matter in practice.

**Refused outright** (both positions agree, and the spec binds them): a durable outbox;
a second queue; a quiesce fence; a session goodbye; a configurable cap; payload
truncation or store re-reads at delivery time (any future re-read reinstates the 0003
race criterion 7 pins — refused on sight, the tripwire is the criterion itself);
pre-serializing payloads on the accept path (a 0001 violation in costume); per-URL
fairness maps (a stalled target holds its slot for at most 22.1 s, named by the
ledger); a new chaos surface for webhook targets (a fired DATA rule fails the stage —
the mail is never stored, so no webhook is owed; chaos between commit and 250 would
manufacture stored-but-unacked mail on purpose); statics for the ledger; `coverage(off)`;
a harness file for a warm path (0020).

## Why no decision document

The intake ruled this feature non-semantic
(`semantic_change_to_frozen_guarantee: false`, `decision_document_required: false`),
and the ruling holds under the second round's scrutiny. Nothing touches accept ==
stored: `Store::insert`, `insert_async` and the commit→250 leg are unedited, and the
bound lives strictly downstream of `recv()` (the firehose send is non-blocking,
`store.rs:379`; the cap and every counter sit after it). Nothing touches
subscribe-before-check: the dispatcher's subscription ordering is *tightened* (frozen
before-bind), not weakened, and the feed still subscribes before its first scan.
Nothing touches oldest-only pruning: the enqueued payload is an owned `Arc<str>`;
pruning never touches a delivery in flight. The public contract this feature adds — the
cap value and the five metric row names — is unchanged from the first round; the one
ruling superseded here lives in the first-round spec itself, not in `docs/decisions/`,
and its supersession is recorded in this file and in append-only git history. The
contract text documents a posture the firehose already had (a bounded ring whose
`Lagged` is already loss), made bounded and observable; the stop-window amendment makes
loss *more* observable, never less. Should the operator want the stop-window contract
memorialized as a decision, that is an ordinary request afterward — this spec does not
declare one, so none is drafted.

## Acceptance Criteria

Each criterion names the observable and the test that proves it; each must be able to go
red. All tests run against real servers over the real protocol (no mocks, 0004); timing
discipline per 0020 rule 5 (real deadlines, no sleep-as-sync, per-test inboxes, chaos at
the poles only).

1. **SHUTDOWN-COMPLETES-WITH-FEED-OPEN** — `tests/lifecycle.rs` gains a test that opens
   an SSE feed (the `common::SseReader` raw-GET pattern), then calls `srv.stop()` under
   a real `tokio::time::timeout` deadline and asserts stop() returned, the feed socket
   now reads EOF, and both SMTP and HTTP ports refuse new connections. Red today:
   stop() never finishes while the SSE response is open, and the deadline fires.
2. **NOTHING-UNTRACKED** — every server-owned spawned task — the HTTP/SSE serve task,
   SMTP sessions, POP3 sessions, the webhook dispatch loop and its per-delivery tasks —
   is registered in a tracker owned by `RunningServer` (or its owning serve loop) and
   cancelled during shutdown. `tests/lifecycle.rs` gains a test pointing a webhook at a
   stalled sink (accepts, never replies) with a delivery in flight, then asserting
   within a 10 s deadline — strictly between the abort path (milliseconds) and the
   corrected 22.1 s drain path — that `stop()` returned *and* the sink's connection
   ended (the delivery was aborted, not drained: a drain holds the socket ≥ 22.1 s and
   fires the deadline). Red today: the delivery is an orphan and its connection stays
   open past the deadline; red after the change if the dispatcher drained instead of
   aborting.
3. **WIRE-SESSIONS-DIE-CLEANLY** — shutdown with an SMTP session mid-transaction (e.g.
   mid-DATA) and a POP3 session mid-transaction completes, `stop()` returns under the
   deadline, both ports refuse new connections, each session's socket reads EOF within
   the deadline, and the aborted sessions left no partial mail in the store (exact
   stored count on inboxes the sessions never acked). Red today: the sessions are
   orphans and their sockets stay open past the deadline.
4. **CONCURRENCY-BOUND-HELD** — with a real local sink that counts its own concurrent
   requests and delays each reply, a burst larger than the cap observes a maximum
   concurrency of exactly `WEBHOOK_DELIVERY_CAP` (32) and never above it
   (`tests/webhook.rs`).
5. **LOSS-OBSERVABLE** — after a forced storm of N mails against one held-stalled
   target, `swarmail_webhook_dropped_total` on `/metrics` reads exactly `N − 32` — no
   more, no less (under-count = a refusal lost, over-count = a refused delivery counted
   twice) — while every SMTP send of the storm answered `250`. The ledger's other rows
   name the remaining deaths: `delivered`, `lagged`, `exhausted`, and `aborted`.
   `aborted`'s stop-arm counting is proven deterministically by the inline `loop_tests`
   (stop signaled with a buffered mail: the mail is never serialized, its matching
   targets are counted); the wire tests prove the deadline half. Red while the rows are
   absent or disagree with the wire by even one.
6. **HOOKS-NEVER-BLOCK-ACCEPTANCE** (0001) — with the sink stalled and the bound full,
   SMTP `DATA` still returns `250` for every mail of the storm and each mail is
   queryable immediately (exact 250 count + exact stored count; `tests/webhook.rs` in
   the `tests/rate.rs` posture). Red if the bound ever touches the accept path.
7. **QUEUED-DELIVERIES-SURVIVE-PRUNING** (0003) — the payload is captured at enqueue
   (owned `Arc<str>`, never re-read from the store): with a sink that captures the
   request then delays its reply, sending one mail and then filling its inbox past the
   configured cap (oldest-only eviction, 0003) leaves the delivery's received body
   byte-identical to what was enqueued. Red if a delivery ever re-read the store — and
   any future change that does so reinstates the race this criterion pins.
8. **CONTRACT-DOCUMENTED** — best-effort with observable loss, **no durable outbox**,
   the cap value (`swarmail::webhook::WEBHOOK_DELIVERY_CAP` = 32) and the five metric
   names are written where webhook behavior is documented (the README Webhooks
   section), with the cumulative-since-start caveat and the 22.1 s stalled-slot bound;
   the doc's dropped-row name is the exact string criterion 5 asserts.

Coverage posture (0020): the gate stays exact 100/100/100; every new production line is
warm-driven on every run (the driver map lives in [plan.md](plan.md) § Coverage); no
`coverage(off)`; no new harness file is expected — no new line is cold-path-only (the
stop-arm counting, the cap-check false arm and the startup-failure self-resolution are
all deterministically drivable on every run). The recorded ingest bench criterion must
not regress: the bound sits post-insert, on the fan-out side of the moat.
