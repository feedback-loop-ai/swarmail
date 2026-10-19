# 0021 — Watcher channel lifetime: a wake ring released with its inbox's mail

- **Status:** accepted (operator, 2026-10-16 — "accept all")
- **Date:** 2026-10-11
- **Supersedes:** none
- **Provenance:** originally filed as `0010-watcher-channel-lifetime`;
  renumbered 0010 → 0021 at issue #32's docs pass — it landed after
  `0010-exact-count-asserts-and-the-shared-seed`, which kept 0010.
- **Companion:** epic #13 wave 3 — issues [#3](https://github.com/feedback-loop-ai/swarmail/issues/3)
  and [#14](https://github.com/feedback-loop-ai/swarmail/issues/14); spec at
  [`specs/watcher-atomicity-release/spec.md`](../../specs/watcher-atomicity-release/spec.md)

## Context

The review of epic 13 wave 3 measured three defects in one subsystem:

1. `Store::subscribe` (`src/store.rs:374-384`) is a get-then-insert with no
   single boundary — concurrent first subscribers of a fresh inbox can each
   create a channel; the displaced receiver hangs off a sender the map no
   longer holds. Eight threads produced seven receivers that missed a
   subsequent insert. This is a hole in decision 0002 as practiced: its
   subscribe-before-scan ordering only ever held for the *winning* first
   subscriber.
2. A `broadcast::channel(4096)` sender is inserted into `watchers` on first
   subscription and never removed — every inbox ever awaited keeps ~128 KiB
   for the life of the process (issue #14: 5,000 awaited fresh inboxes,
   ~641 MB). Under decision 0004, inboxes are per-test and effectively
   unbounded in count, so a shared dev server grows with *tests served*,
   not with mail held.
3. `wait_for` answers a closed receiver (`src/store.rs:427`) as just
   another wakeup: instant `Err` → rescan → instant `Err` → spin until the
   deadline — the review measured 1.3–1.5 cores for waiters that should
   sleep. This is latent today (channels are immortal) and becomes the
   main path the moment a release policy ships.

None of this is covered by an existing decision: 0002 rules on *ordering*,
not channel lifetime; 0001 on acceptance; 0003 on pruning.

## Decision

1. **A watcher channel is a wake token, not a mailbox.** Both per-inbox
   consumers (`wait_for`, the SSE feed) discard the payload and re-read
   the store — the store is the truth (0002's own consequence: every
   wakeup re-scans). The ring is therefore sized for waking, not for
   buffering: `WATCHER_CAP = 16`. Lag is repaired by the rescan that
   already exists and is already pinned by
   `wait_for_covers_the_lagging_watcher`; `send` stays non-blocking, so
   the 250 path (0001) is untouched at any capacity.
2. **Registration is atomic.** `Store::subscribe` is a single
   `DashMap::entry().or_insert_with()` path: the shard write lock is held
   across channel creation, so concurrent first subscribers serialize on
   one boundary and every receiver attaches to the same live channel.
   There is no second path left to race — the get-then-insert window is
   deleted, not fenced, and no get-fast-path is added to dodge the
   per-subscribe allocation.
3. **The lifetime rule: a watcher channel is created by the first
   `subscribe` to an inbox and released when the inbox's mail is removed**
   — by `drain_inbox` (shared by `clear` and `clear_all`), by a `delete`
   that empties the inbox's list, and by the `clear_all` sweep
   (`watchers.clear()`), which also releases channels of
   awaited-but-never-mailed inboxes — the population issue #14's probe
   creates and no per-inbox drain hook can reach. `insert` never creates a
   channel: a notify with no watcher entry is a no-op, as today. Channel
   lifetime is tied to the inbox's mail, not to subscriber count (tokio
   has no last-receiver drop hook; guard/weak-map machinery was refused as
   a new bug class) and not to a timer (no idle TTL sweeper — a
   clock-driven removal would make a parked waiter's promptness depend on
   a background sweep).
4. **Decision 0002 is re-applied per iteration.** `wait_for` answers
   `RecvError::Closed` with exactly one re-subscribe and falls through to
   the loop's next scan — subscribe → scan, again. This is what makes
   release safe: a waiter whose channel dies wakes, rescans,
   re-subscribes, and sleeps; it never spins and never misses mail,
   because any insert landing between the release and the re-subscribe is
   found by the post-subscribe scan. Reviewers' explicit
   subscribe-before-check check now covers the `Closed` arm. Sleeping
   until the deadline on `Closed` and returning `Timeout` immediately were
   refused: the first turns "mail at t+5 ms" into a wait that returns at
   the deadline; the second abandons a waiter whose inbox the next insert
   re-creates.
5. **The SSE feed ends when its inbox's mail is removed.**
   `feed_events`'s existing `RecvError::Closed => None` arm becomes
   reachable and is the honest signal of a destructive teardown; the
   browser's `EventSource` reconnect re-establishes 0002 at the wire
   boundary (`feed_stream` subscribes before the first view scan) and
   re-snapshots the post-drain view. Internal re-subscription inside the
   feed was refused: it would hide drains from clients and make closure
   untestable.

## Consequences

- **The public contract changes in one observable way:** an SSE stream on
  an inbox whose mail is removed now ends instead of staying open forever.
  Reconnect yields a correct, empty snapshot; no mail is lost (the store
  is the truth and the snapshot is the view).
- **0002 is strengthened, not amended:** its ordering now holds for every
  first subscriber under every schedule (the race is deleted), and per
  iteration for waiters that observe a release. Duplicate wakeups remain
  allowed; missing one remains impossible.
- **Memory is bounded and test-pinned:** per-inbox residue drops from
  ~128 KiB to ~0.5 KiB (issue #14's probe: ~641 MB → ~2.5 MB), and to
  zero for any inbox whose mail is ever removed. Residue is not zero for
  awaited-but-never-cleared inboxes — bounded, documented, and pinned by
  regression tests via a `#[cfg(test)]` count accessor; the production
  `swarmail_watchers` gauge is a deliberate follow-up, not bundled here.
- **Restore semantics unchanged:** restart copies never re-fire watchers
  (`src/store.rs:159-166`), and a wait does not survive the process
  (0008: single replica, `Recreate`) — clients re-request against the
  restored store.
- **Refused here, on the record:** the `tokio::sync::watch`/`Notify` swap
  (rewrites the public signature, the feed loop and three test modules to
  delete a lag path that is already proven safe), subscriber-count
  release (no drop hook; refcount/guard machinery), and idle-TTL
  eviction (nondeterministic promptness). If a later wave wants the token
  swap, it arrives with its own review.

## Later-decision markers — 2026-10-19 (issue #144)

Appended 2026-10-19 by the issue #144 provenance audit, after the operator's
2026-10-19 ruling accepting 0034–0050 and 0055. The text above stays as the
accepted historical ruling; each marker names a later record with that
record's own verb and scope, and nothing above is rewritten by it.

- *(Amended by [0038](0038-store-memory-and-timestamps.md) (accepted
  2026-10-19): the Consequences' bounded-memory/residue claim. A distinct
  awaited name can leave a channel after its last receiver leaves, so
  watcher residue is unbounded in aggregate across distinct names (about
  780 B per name observed on one build, not a ceiling); reusing a name
  reuses its channel. The lifecycle and atomic registration above are
  preserved (0038 §1).)*
