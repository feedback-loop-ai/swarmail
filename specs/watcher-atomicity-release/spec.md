# Spec — watcher registration atomicity (#3) + watcher channel release (#14)

Epic #13, wave 3 · issues [feedback-loop-ai/swarmail#3](https://github.com/feedback-loop-ai/swarmail/issues/3)
and [feedback-loop-ai/swarmail#14](https://github.com/feedback-loop-ai/swarmail/issues/14) —
one change, one registration code path, per the epic's pairing.

Decision: 0010 — [`docs/decisions/0010-watcher-channel-lifetime.md`](../../docs/decisions/0010-watcher-channel-lifetime.md)
(status `proposed`; the operator accepts). See
[Why a decision document](#why-a-decision-document-despite-the-panels-votes).

## The problem, as the review measured it

1. **Concurrent first subscriptions race channel creation**
   (`src/store.rs:374-384`). `Store::subscribe` is a get-then-insert with no
   single boundary: eight threads subscribing to one fresh inbox produced
   seven receivers attached to a sender that the last writer's insert
   *overwrote* — displaced, closed, and mute. The inbox map and the watcher
   map are updated without one boundary.
2. **Watcher channels are never released.** A `broadcast::channel(4096)`
   sender is inserted into `watchers` on first subscription and never
   removed: every inbox ever awaited keeps ~128 KiB for the life of the
   process. Issue #14's reproduction: 5,000 awaited fresh inboxes retained
   ~641,160 KiB (Rust 1.97.0, at 4bca86a7). Under decision 0004 (per-test
   inboxes are the isolation model) a shared dev server grows with *tests
   served*, not with mail held.
3. **A closed receiver makes `wait_for` burn a core.** The loop answers
   `Closed` (`src/store.rs:427`) as just another wakeup: instant `Err` →
   rescan → instant `Err` → spin, until the deadline. The review measured
   1.3–1.5 cores busy-spinning for waiters that should sleep. Today the spin
   is masked because channels are immortal; the moment (2) ships a release,
   every drained inbox hands its waiters a closed receiver — **(b) is unsafe
   to ship without (c)'s closure-aware loop, and (a) is unsafe to ship
   without (b) because the drain races the creation.** The wave's three
   deliverables are one design.

## The committed design

One policy, stated once: **a watcher channel is a wake token, not a
mailbox — it is created by the first `subscribe` to an inbox, atomically,
and released when the inbox's mail is gone; a waiter that observes the
release re-subscribes before its next scan (decision 0002, re-applied every
iteration) and sleeps.**

- **(a) Atomic registration.** `Store::subscribe` becomes a single
  `DashMap::entry().or_insert_with()` path: the shard write lock is held
  across channel creation, so concurrent first subscribers serialize on one
  boundary and every receiver attaches to the same live channel. The
  get-then-insert window is *deleted*, not fenced — there is no second path
  left to race. Capacity is a named constant `WATCHER_CAP = 16`.
- **(b) Release where the mail dies.** `drain_inbox` (shared by `clear` and
  `clear_all`) removes the inbox's channel; `delete` removes it when its
  retain empties the inbox's list; `clear_all` ends with a
  `watchers.clear()` sweep that also releases channels of
  awaited-but-never-mailed inboxes — the exact population #14's probe
  creates (5,000 awaits, no mail, nothing to iterate). `insert` never
  creates a channel; a notify with no entry is a no-op, exactly as today.
- **(c) The wait path sleeps.** `wait_for` answers `RecvError::Closed` with
  exactly one re-subscribe and falls through to the loop's next scan —
  decision 0002's subscribe → scan ordering re-applied per iteration. Every
  other wakeup (`Ok(_)`, `Lagged`, deadline) behaves as today. Structural
  no-spin invariant: every `recv` parks on a live channel unless the
  channel is closed, and a closed channel is answered by one re-subscribe
  per iteration — there is no schedule in which an iteration does neither.
- **The SSE feed ends when its inbox is drained.** `feed_events` already
  maps `Closed` → stream end (`src/feed.rs:81`); release makes that arm
  reachable. This is deliberate: a drain is a destructive teardown, the
  end-of-stream is its honest signal, and the browser's `EventSource`
  reconnect re-establishes 0002 at the wire boundary (`feed_stream`
  subscribes before the first view scan) with the post-drain snapshot.
  Internal re-subscription would hide drains from the client and make the
  closure untestable. The only `feed.rs` code change is the now-false
  comment above the arm.

Per-inbox residue after this change: ~0.5 KiB (a 16-slot wake ring) instead
of ~128 KiB — #14's 641 MB probe becomes ~2.5 MB — and ~0 for any inbox
whose mail is ever removed.

## Adopted from the positions — and where they conflicted, the ruling

Both seats agree on the skeleton: one `entry()` registration, release on
drain plus a `clear_all` sweep, one `Closed → re-subscribe` arm, the feed
ends on drain, and refusals of the `watch`/`Notify` swap, an idle-TTL
sweeper, subscriber-count release (refcount/guard machinery), and any new
dependency. The races between them were ruled, not averaged:

| Conflict | Ruling | Cost paid by the losing side |
|---|---|---|
| **Ring capacity**: simplicity keeps 4096 ("a knob with no demand"); robustness shrinks to 16 (the channel is a wake token; both consumers discard the payload) | **16.** #14's 641 MB probe awaits fresh inboxes and *never clears them* — drain-only release leaves that hole open, and a background sweeper is refused by both seats. The shrink is the only element that bounds residue for never-cleared inboxes. | Lag arrives sooner. It is repaired by the rescan the suite already proves (`wait_for_covers_the_lagging_watcher` still passes — it inserts 4,200 into a 16-slot ring and pins the invariant, not the capacity); `send` stays non-blocking, so the 250 path (0001) is untouched. |
| **`delete`-to-empty releases?** simplicity: no ("channel lifetime == inbox lifetime, not mail count"); robustness + intake criterion 2: yes | **Yes.** The intake framing is the wave's charter and names "per-mail delete draining an inbox" in the release contract; Mailpit-style selective delete is 0004's world. | The policy is one clause longer and `delete` gains one conditional. Simplicity's purer rule would leave residue on exactly the flow the charter names. |
| **A test seam**: simplicity adds `post_subscribe_gate`; robustness wants zero new seams | **The seam is added.** The closure-during-wait sequence needs the clear to land after the initial subscribe and the insert to land after the re-subscribe — no scheduler trick reaches that window (it holds no await point), so it is the wave-1 settlement pattern (`pre_mirror_gate` shape) verbatim. Robustness itself named this shape as its probe's fallback. | ~19 `#[cfg(test)]` lines compiled out of the shipping binary, self-covering at the exact gate (Some arm by the closure tests, None arm by every existing wait test). |
| **Busy-spin teeth**: simplicity refuses any CPU/timing assertion ("flake factory"); robustness offers a single-worker ticker starvation probe | **Both probes ship, no CPU meter.** The gate-driven *not-stranded* test (red on no-release, no-resubscribe, and sleep-through — the mail exists only if the re-subscribe happened) plus the ticker probe (a spin starves its own worker; a sleeping waiter lets the ticker advance). A wall-clock CPU-percentage assertion is refused. | The ticker's red-on-today's-code is probabilistic under cooperative budgeting (an honest loss recorded in the plan); its regression teeth — a reintroduced spin starves the ticker — are structural. |
| **`swarmail_watchers` gauge on `/metrics`**: robustness adds it; simplicity defers it | **Deferred.** The intake explicitly de-scopes it ("issue-level and NOT in this wave's stated deliverables; design may take it or leave it, but must make the count test-observable"). The count is test-observable via a `#[cfg(test)] watcher_count()` accessor. | A leak is not observable from outside the process until the follow-up lands; mitigated by the regression tests pinning the bound. The follow-up is deliberate and recorded: `pub fn watcher_count()` + one `("gauge", "swarmail_watchers", …)` tuple in `api.rs::metrics`, mirroring the `ca_cache` pattern. |

The intake's seam placement ("a hook at the watchers get/insert boundary in
`Store::subscribe`") is **revised by this spec**: after (a) there is no
get/insert boundary — preserving a hook inside it would mean keeping the bug
alive to test it. The race test is seam-free (a barrier of concurrent first
subscribers, green under every schedule post-fix); the settlement pattern's
seam moves to the window that genuinely survives the fix — the
subscribe/re-subscribe boundary in `wait_for`.

### Refused, with the sentence that refuses it

- **`tokio::sync::watch`/`Notify` per inbox** (#14's aside): rewrites the
  public `Store::subscribe` signature, the feed loop, and three test
  modules to delete a lag path that is already proven safe — and (b)+(c)
  still have to be written. The capacity change achieves the bound with
  zero churn.
- **Idle-inbox TTL sweeper**: a clock-driven removal makes a parked
  waiter's promptness depend on a background sweep, must itself be kept at
  literal 100% coverage, and attacks the one case that is negligible
  (an inbox that never clears still holds its *mail*, which dwarfs its
  channel). Issue #14 proposes no timer — its own mechanisms are
  release-when-no-subscribers and/or a bufferless channel.
- **Subscriber-count release** ("when the inbox has no subscribers"):
  tokio gives no drop hook on the last receiver; this means a guard wrapper
  with a self-referential weak map handle — a new bug class and a removal
  racing a concurrent subscriber, the exact hazard #14 says to avoid.
- **A get-fast-path in `subscribe`** (to dodge the per-subscribe
  allocation): a second code path is the bug being deleted.
- **Sleep-until-deadline on `Closed`** (turns "mail at t+5 ms" into a wait
  that returns at the deadline) and **return `Timeout` immediately on
  `Closed`** (abandons a waiter whose inbox the next insert re-creates):
  both break push semantics; re-subscribe is the only arm that preserves
  them, and it is one line.

## Why a decision document, despite the panel's votes

Both seats voted "no decision document". The chief overrides, narrowly: the
change settles a **public contract** — the SSE feed's stream now ends when
its inbox's mail is removed, and the wait path's answer to a closed channel
is now a re-subscribe rather than a fall-through. Neither weakens a frozen
guarantee (0001 untouched; 0002 strengthened — its ordering now holds under
concurrency and per iteration; 0003, 0004, 0005 untouched), and no wakeup
can be missed (the re-subscribe precedes the rescan; the store is the
truth). But 0002's letter says reviewers check the subscribe-before-scan
ordering explicitly, and the feed's lifetime is observable by clients — so
[`0010`](../../docs/decisions/0010-watcher-channel-lifetime.md) records the
policy with status `proposed` for the operator. This is belt-and-braces
paperwork in the exact form simplicity said was acceptable.

## Out of scope (deliberate, recorded)

- The `/metrics` gauge (follow-up shape above).
- Any change to the global webhook firehose channel (`broadcast(8192)`,
  one per process, value-consuming — constant residue by construction).
- Any touch on 0001's ordering: fan-out stays after the map write and
  before the 250; the wake cannot delay or fail an acceptance.
- Restored-mail watcher semantics (restart copies never re-fire watchers —
  `src/store.rs:159-166`), and wait survival across restarts (0008:
  single replica, `Recreate`).

## Acceptance Criteria

- **AC1 — Concurrent first subscribers all attach to one live channel.**
  A deterministic race test releases eight threads past a `Barrier`, each
  calling `Store::subscribe` on one fresh inbox; one insert follows; every
  receiver observes it via `try_recv` — zero missed. Post-fix the assertion
  holds under *every* schedule (the race is deleted, not controlled); on
  the pre-fix code it fails whenever two threads overlap (the review's
  7-of-8 measurement). No seam inside `subscribe`: the test needs no hook
  and no sleep-and-hope loop.
- **AC2 — Channels are released when their inbox's mail is gone.**
  (i) `clear(inbox)` → a receiver held across the clear observes
  `try_recv() == Err(Closed)`. (ii) A `delete` that empties an inbox's list
  releases the channel the same way. (iii) `clear_all` releases channels of
  *awaited-but-never-mailed* inboxes: N subscriptions on fresh inboxes
  (no mail), then `clear_all()`, then the test-visible count returns to
  baseline — issue #14's own criterion, made literal. (iv) A `clear` of a
  nonexistent inbox stays a no-op.
- **AC3 — A release can never strand a waiter or lose a wakeup.**
  Gate-driven closure-during-wait test: the gate fired after the initial
  subscribe clears the inbox (releasing the waiter's channel); the gate
  fired after the `Closed` arm's re-subscribe inserts the mail; the wait
  returns `Found`. Red on all three regressions — no release on clear
  (the insert never happens: it lives in the re-subscribe gate → parks to
  `Timeout`), no re-subscribe on `Closed` (gate never fires again →
  `Timeout`), sleep-through-on-`Closed` (`Timeout`) — and it proves the
  0002 per-iteration ordering: a mail inserted *after* the release is
  observed, because the re-subscribe precedes the rescan.
- **AC4 — Waiters sleep; they do not busy-spin.**
  A single-worker (`current_thread`) runtime runs a 5 ms ticker co-task and
  a waiter whose channel is released by the gate; the waiter returns
  `Timeout { matched: 0 }` at its 200 ms deadline **and** the ticker
  advanced (≥ 10 of ~40 ticks): a spinning loop would starve its own worker
  and collapse the count. The SSE feed is pinned the same way at the unit
  boundary: after its snapshot, `clear` of the inbox ends the stream
  (next poll is `None`) — no spin, no hang.
- **AC5 — The wake ring's invariants survive the shrink.**
  `WATCHER_CAP = 16` and `wait_for_covers_the_lagging_watcher` stays green
  unchanged in its assertion (4,200 inserts overflow a 16-slot ring; the
  rescan returns the mails) — lag repair is the documented behaviour, not
  headroom. Per-inbox residue is documented as ~0.5 KiB (issue #14's probe
  drops from ~641 MB to ~2.5 MB).
- **AC6 — End-to-end over the real protocol.**
  An integration test in `tests/burst.rs` drives the real server: an
  `await` on a fresh inbox, a clear of that inbox, an SMTP insert, and a
  second `await` that finds the mail — a released inbox keeps serving
  waiters, and the first await's timeout is honest (`matched: 0`).
- **AC7 — All six house gates green.**
  `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test` (real servers, no mocks), `brokkr compile --bundle .`,
  `bash scripts/coverage-gate.sh` (exact 100/100/100, missed == 0, no
  `coverage(off)`), `cargo deny check licenses`. Every new production line
  is executed by a test; no new dependencies.

## Verification quickstart

```bash
cargo test --lib store::            # race, release, sweep, stranded, no-spin
cargo test --lib feed::             # the feed ends on drain
cargo test --test burst await_survives_a_release   # real HTTP + SMTP
bash scripts/coverage-gate.sh       # exact 100/100/100
```
