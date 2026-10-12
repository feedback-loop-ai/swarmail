# Spec — total shutdown: every owned task tracked on stop, webhook fan-out bounded and loss-observable

Epic [feedback-loop-ai/swarmail#13](https://github.com/feedback-loop-ai/swarmail#13),
issues [#7](https://github.com/feedback-loop-ai/swarmail/issues/7) (shutdown is total)
and [#11](https://github.com/feedback-loop-ai/swarmail/issues/11) (webhook delivery is
bounded, lag-observable, best-effort with observable loss). The panel's positions:
[`.forge/design/positions/simplicity.md`](../../../.forge/design/positions/simplicity.md)
and
[`.forge/design/positions/robustness.md`](../../../.forge/design/positions/robustness.md)
(both `pass`; run-local evidence, gitignored). Adjacent rulings referenced below:
[0010-watcher-channel-lifetime.md](../../docs/decisions/0010-watcher-channel-lifetime.md)
(`proposed`, same subsystem) and
[0011-concurrency-tests-and-the-coverage-gate.md](../../docs/decisions/0011-concurrency-tests-and-the-coverage-gate.md)
(`proposed`, test placement).

## The problem

`RunningServer.tasks` holds exactly the three accept loops (`src/lib.rs:40`); `stop()`
awaits those three and returns. Everything else the process spawned is an orphan. The
webhook dispatcher is spawned before the shutdown channel exists (`lib.rs:67` vs `:95`)
and is in nobody's tracker; every delivery is spawned per (mail, target)
(`src/webhook.rs:189`) and is likewise unowned; every SMTP/POP3 session is spawned
inside its accept loop (`src/smtp.rs:91`, `src/pop3.rs:39`). An SSE feed is an untracked
HTTP response that graceful shutdown waits on forever: with one feed open, `stop()` never
returns. Meanwhile the fan-out downstream of insert is unbounded — a burst spawns one
POST per target per mail, no cap, no queue — and its losses are invisible: a firehose
`Lagged` is a warn line (`webhook.rs:159`), an exhausted retry budget is a warn line
(`webhook.rs:215`), and nothing anywhere counts. Issue 11's acceptance criterion — an
exact count of undelivered notifications — is unauditable today, not merely unmet.

## Ruling — how the two positions were reconciled

Both positions agree on the load-bearing frame: **the accept path is a moat.** Everything
this feature adds lives strictly downstream of the commit; every bound is `try`-shaped
(drop-and-count, never wait-and-delay); every cancellation is owned by its spawner;
`stop()` must be as honest as `250`. Four points genuinely conflict; this spec rules
rather than averages.

**Adopted from both (the agreements, restated as refusals).** No second queue — the
firehose (`Store::global_tx`, a bounded broadcast ring, `store.rs:188`) is the queue, and
bounding happens strictly downstream of `recv()`; a queue-full event is a drop-and-count,
never a delayed 250 (0001). No durable outbox — the store is the truth; loss is made
observable, not recoverable. No session goodbye (no 421 write) — the closed socket is the
signal; a polite write manufactures a lie the client can act on. No configurable cap — a
documented `const` like `CA_CLIENT_CAP`. No new dependency, no new module, no new chaos
surface between commit and 250 (that leg is fail-safe by construction; injecting there
manufactures stored-but-unacked duplicates on purpose). Cancellation is **structural
ownership**: a `JoinSet` per owner — `RunningServer` owns the three serve tasks plus the
dispatcher, each serve loop owns its sessions, the dispatcher owns its deliveries — never
a snapshot-and-chase abort list, which races the accept window it claims to close.

**Ruled against simplicity: one exhaustive per-instance ledger, not one static counter.**
Simplicity's single `static WEBHOOK_DROPS: AtomicU64` (one `swarmail_webhook_dropped_total`
row, "dropped at the bound") is refused on two grounds. First, a process-global static is
the tree's own cautionary precedent, not its endorsement: the CA-cache counters are
statics, and `delivery_tests` already needs a `CACHE_TESTS` mutex to keep parallel tests
from cross-contaminating exact asserts — a static webhook counter re-creates that mutex
in a realm where 0004 puts every test on its own inbox and every test binary runs many
`RunningServer`s side by side, making AC5's exact assert a cross-contamination race.
Second, one counter is not exhaustive: a burst larger than the firehose ring (8192) loses
mail through `Lagged`, which a dropped counter never sees — AC5's exact count would be
unauditable by construction. Adopted instead: robustness's ledger — per-instance counters
on `Webhooks` (`delivered`, `dropped`, `lagged`, `exhausted`, `aborted`; `AppState`
already carries the `Arc`, the dispatcher already receives it), surfaced as five
`swarmail_webhook_*_total` rows on `/metrics`. Every death a notification can die has a
name; the ledger sums to the notifications presented. Costs, named: ~25 lines and one
small struct over simplicity's static; from robustness's side, the ledger is
cumulative-since-start (a restart is a new ledger — loss before the crash is visible only
in the old process's metrics) and an oversized mail is POSTed whole (a counted failure,
never a silently trimmed one). Simplicity's doc point survives verbatim in spirit: the
dropped row means "refused at the bound" and nothing else — shutdown loss gets its own
row, never mixed into another's.

**Ruled against robustness: the JoinSet is the queue; the permit semaphore is refused.**
The bound is `deliveries.len() < WEBHOOK_DELIVERY_CAP` over one per-dispatcher `JoinSet`,
harvested each pass — not an `Arc<Semaphore>` acquired with `try_acquire`. The intake
flag says it plainly: *"the criteria above bind observable behavior, not the mechanism"*,
and one structure is at once the bound, the cancellation unit and the harvest point; a
semaphore is a second structure that must itself be owned and whose permits duplicate the
set's length. What robustness loses: nothing observable — the bound is identical
(in-flight ≤ cap, try-shaped, drop-and-count), the congestion drop stays per-delivery
(one stalled target costs only its own mail; the others keep delivering), and the
per-URL-permit-map refusal it itself made stands. What it keeps: the dispatcher still
owns its children and aborts them at stop.

**Adopted from robustness (over simplicity's silence): two load-bearing orderings.**
(1) **The dispatcher subscribes to the firehose before the first listener binds.** Today
this is true (`lib.rs:67` precedes the binds at `:87-93`) and simplicity's own suggested
edit — moving `spawn_dispatcher` below the watch creation *as written* — would break it
(the channel is created after the binds). Ruled: the watch creation moves **up** to the
store's birthplace; the dispatcher spawns immediately after it, before TLS and before any
bind. This is decision 0002's discipline one layer down, frozen here: a consumer created
after a bind can miss the webhook for a mail accepted in between. (2) **The payload is
materialized once per mail and shared** (`Arc<str>`), not rebuilt per (mail, target)
inside the fan-out loop (`webhook.rs:184-188`). Simplicity is right that AC7 needs no
*code* — the payload is already captured at enqueue and never re-read from the store —
and robustness is right that the per-target `to_string()` multiplies a 50 MB mail by the
target count. Building once is the simplification, and the owned `Arc` is what makes
criterion 7's red line (no store re-read, no truncation) true by construction rather than
by discipline.

**Ruled on the feed: cooperative end, not global abort.** The SSE stream ends by folding
the existing shutdown watch into `feed_events`' unfold state (one select arm → `None`);
the response completes, axum's graceful shutdown finishes, and the tracked HTTP serve
task returns — the only untracked HTTP task is one hyper owns, and no per-connection
handle exists short of replacing `axum::serve`. Robustness's tolerated alternative
(aborting the HTTP serve task at stop) is declined: it kills benign in-flight requests
with the guilty. The named fallback, should the cooperative end prove insufficient, is a
`stop()`-wide deadline-and-abort — second choice, not first. A frame may race the closing
stream; the browser's `EventSource` reconnect re-subscribes before its first scan (0002)
and re-snapshots — exactly the recovery 0010 §5 documents. The `Closed` arm
(`feed.rs:85`) remains the only non-shutdown way a feed ends; the two arms coexist in one
select and touch none of 0010's machinery.

**The precise `stop()` promise (robustness's wording, adopted as the contract).**
`spawn_blocking` commits are uncancellable: a mail whose commit began before its session
was aborted can land after `stop()` returns. The client never saw the 250 — the reply is
written only after the commit outcome is known (`smtp.rs:461`) — so the outcome is a
duplicate on retry, never a loss, and never a rollback of an unacked session. A quiesce
fence (stop() awaiting in-flight commits) is refused: it adds tracking code to the accept
path's store leg to serve a promise the wire cannot observe. The test discipline this
buys is mandatory: exact post-stop counts assert only on inboxes the aborted session
never touched. The dispatcher's select is **biased to the firehose**: a buffered mail is
always dispatched or dropped (and counted) before the shutdown arm can win — no mail is
silently skipped at stop; the cost is a shutdown that waits out the firehose backlog
(microseconds per mail, bounded by the 8192 ring).

**Refused outright (both positions agree, and the spec binds them):** a durable outbox; a
second queue; a quiesce fence; a session goodbye; a configurable cap; payload truncation
or store re-reads at delivery time; per-URL fairness maps (a stalled target holds its
share of the bound for at most `POST_TIMEOUT` + the retry budget, ~17 s worst case); a
new chaos surface for webhook targets.

## Why no decision document

The intake ruled this feature non-semantic
(`semantic_change_to_frozen_guarantee: false`, `decision_document_required: false`), and
the ruling holds under the panel's scrutiny. Nothing touches accept == stored: insert and
the commit→250 leg are unedited, and the bound lives strictly downstream of `recv()`.
Nothing touches subscribe-before-check: the dispatcher's subscription ordering is
*tightened* (frozen before-bind), not weakened, and the feed still subscribes before its
first scan. Nothing touches oldest-only pruning: the enqueued payload is an owned
`Arc<str>`; pruning never touches a delivery in flight. The contract text criterion 8
asks for documents a posture that already exists — the firehose is a bounded ring whose
`Lagged` is already loss — and makes it bounded and observable. A semantic change (a
quiesce fence, a durable outbox, a second queue reachable from the accept path) would
carry a `proposed` decision document; this spec refuses all three, so none is drafted.

## Acceptance Criteria

Each criterion names the observable and the test that proves it; each must be able to go
red. All tests run against real servers over the real protocol (no mocks, 0004); timing
discipline per 0011 rule 5 (real deadlines, no sleep-as-sync, per-test inboxes).

1. **SHUTDOWN-COMPLETES-WITH-FEED-OPEN** — `tests/lifecycle.rs` gains a test that opens
   an SSE feed (the `common::SseReader` raw-GET pattern), then calls `srv.stop()` under a
   real `tokio::time::timeout` deadline and asserts stop() returned, the feed socket now
   reads EOF, and both ports refuse connections. Red today: stop() only closes listeners;
   the in-flight SSE response never completes, graceful shutdown never finishes, and the
   deadline fires.
2. **NOTHING-UNTRACKED** — every server-owned spawned task — the HTTP/SSE serve task,
   SMTP sessions, POP3 sessions, the webhook dispatch loop and its per-delivery tasks —
   is registered in a tracker owned by `RunningServer` (or its owning serve loop) and
   aborted during shutdown. `tests/lifecycle.rs` gains a test pointing a webhook at a
   stalled sink (accepts, never replies) with a delivery in flight, then asserting
   `stop()` completes within the deadline. Red today the task is orphaned; red after the
   change if the dispatcher drained instead of aborting (the stalled delivery's own
   budget is ~17 s and the deadline fires).
3. **WIRE-SESSIONS-DIE-CLEANLY** — shutdown with an SMTP session mid-transaction (e.g.
   mid-DATA) and a POP3 session mid-transaction completes, `stop()` returns under the
   deadline, and both ports refuse new connections; the aborted sessions left no partial
   mail in the store (exact stored count on inboxes the sessions never acked).
4. **CONCURRENCY-BOUND-HELD** — with a real local sink that counts its own concurrent
   requests and delays each reply, a burst larger than the cap observes a maximum
   concurrency of exactly `WEBHOOK_DELIVERY_CAP` and never above it
   (`tests/webhook.rs`).
5. **LOSS-OBSERVABLE** — after a forced storm of N mails against one held-stalled target,
   `swarmail_webhook_dropped_total` on `/metrics` reads exactly `N −
   WEBHOOK_DELIVERY_CAP` — no more, no less (under-count = a refusal lost, over-count = a
   refused delivery counted twice) — and the ledger's other rows name the remaining
   deaths: `delivered`, `lagged`, `exhausted`, `aborted`. Red while the rows are absent
   or disagree with the wire by even one.
6. **HOOKS-NEVER-BLOCK-ACCEPTANCE** (0001) — with the sink stalled and the bound full,
   SMTP `DATA` still returns `250` for every mail of the storm and each mail is queryable
   immediately (exact 250 count + exact stored count; `tests/webhook.rs` in the
   `tests/rate.rs` posture). Red if the bound ever touches the accept path.
7. **QUEUED-DELIVERIES-SURVIVE-PRUNING** (0003) — the payload is captured at enqueue
   (owned `Arc<str>`, never re-read from the store): with a sink that captures the
   request then delays its reply, sending one mail and then filling its inbox past the
   configured cap (oldest-only eviction, 0003) leaves the delivery's received body
   byte-identical to what was enqueued. Red if a delivery ever re-read the store.
8. **CONTRACT-DOCUMENTED** — best-effort with observable loss, **no durable outbox**, the
   cap value (`swarmail::webhook::WEBHOOK_DELIVERY_CAP` = 32) and the five metric names
   are written where webhook behavior is documented (the README Webhooks section); the
   doc's dropped-row name is the exact string criterion 5 asserts.

Coverage posture (0011): the gate stays exact 100/100/100; every new production line is
warm-driven on every run (the driver map lives in
[plan.md](plan.md) § Coverage); no `coverage(off)`; no new harness file is expected — no
new line is cold-path-only. The recorded ingest bench criterion must not regress: the
bound sits post-insert, on the fan-out side of the moat.
