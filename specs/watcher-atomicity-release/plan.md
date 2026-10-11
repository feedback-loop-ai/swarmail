# Plan — watcher registration atomicity (#3) + watcher channel release (#14)

Companion to [`spec.md`](spec.md) (WHAT/WHY). Implementation approach, the
files it touches, and the risks with mitigations.

## Technical context

- `src/store.rs` — the only file with behaviour in this change.
  `Store::subscribe` (374–384) is a `DashMap` get-then-insert; `wait_for`
  (404–429) subscribes once and answers every `recv` outcome — including
  `Closed` (427) — as "re-scan"; `drain_inbox` (79–93) is the free function
  `clear`/`clear_all` share; `delete` (310–324) retains one id out of its
  inbox list; `insert` (260–264) notifies via `watchers.get(inbox)` only —
  it never creates a channel. The `pre_mirror_gate` seam (107–119, 198–219,
  329–330, 353–354) is the wave-1 settlement pattern to copy.
- `src/feed.rs` — `feed_events` maps `RecvError::Closed` → `None`
  (75–82) behind a comment that becomes false; `feed_stream` subscribes
  before the first scan (94, decision 0002 at the wire boundary).
- `src/mcp.rs:148` — the MCP wait path delegates to `Store::wait_for`, so
  the closure-aware loop covers it with no separate edit.
- `src/api.rs:179-210` — the `/metrics` rows: the deferred gauge's exact
  follow-up shape (`("gauge", "swarmail_watchers", …)` next to `ca_cache`).
- `tests/burst.rs` — has `await_waits_for_arrival`; the real-protocol
  release guard lands beside it. Real servers only, no mocks.
- Dependencies: none added. `tokio::sync::broadcast` (already a dep),
  `std::sync::Barrier` / `std::thread::scope` (std) cover everything.
- Coverage measurement: `cargo +nightly llvm-cov --workspace --locked
  --branch` — `#[cfg(test)]` items in `src/` are instrumented and counted,
  which is exactly why the `pre_mirror_gate` pattern is written
  self-covering; the new seam and accessor follow the same discipline.

## Constitution check — frozen guarantees

| Guarantee | Verdict |
|---|---|
| 0001 accept == stored | Untouched. `insert` stays synchronous; the fan-out stays after the map write and before the 250; `broadcast::send` is non-blocking at any capacity (a full ring lags, never blocks), so `WATCHER_CAP = 16` cannot delay an acceptance. |
| 0002 await subscribes before it checks | **Strengthened.** Today the ordering only holds for the *winning* first subscriber (the race was a hole in 0002); after (a) it holds for every subscriber under every schedule, and after (c) it is re-applied *per iteration* (re-subscribe → scan). Reviewers' explicit check now covers the `Closed` arm. |
| 0003 pruning is oldest-only | Untouched; no drain-order change. |
| 0004 per-test inboxes | Served: the isolation model stops growing with tests served. |
| 0005 extraction at ingest | Untouched. |

Public-contract deltas (SSE stream ends on drain; wait path answers
`Closed` with a re-subscribe; channel lifetime policy) are recorded in
[`docs/decisions/0010-watcher-channel-lifetime.md`](../../docs/decisions/0010-watcher-channel-lifetime.md),
status `proposed`.

## Project structure — the complete diff surface

```
src/store.rs            behaviour + seam + unit tests (see below)
src/feed.rs             one comment correction + one unit test
tests/burst.rs          one real-protocol test (HTTP await + SMTP insert + clear)
docs/decisions/0010-watcher-channel-lifetime.md   proposed (this change)
specs/watcher-atomicity-release/{spec,plan,tasks}.md
```

No new files under `src/`; no new test files; no touch on `tests/p2.rs`,
`tests/rate.rs`, or any other suite.

## Data model / contracts

- `watchers: DashMap<String, broadcast::Sender<Arc<Email>>>` — the only
  place channels live; now it has a removal policy. Field doc updated:
  created on first subscribe, released when the inbox's mail is gone.
- New const: `const WATCHER_CAP: usize = 16;` — the wake-ring size,
  documented as "a wake token, not a mailbox: both consumers discard the
  payload; lag is repaired by the rescan".
- Policy (verbatim, for review): **a watcher channel is created by the
  first `subscribe` to an inbox and released when the inbox's mail is
  removed — by `drain_inbox` (`clear`, `clear_all`) or by a `delete` that
  empties the inbox's list — plus the `clear_all` sweep; the next
  `subscribe` re-creates it. A waiter that observes the release
  re-subscribes before its next scan and sleeps.**
- Test-visible count: `#[cfg(test)] pub(crate) fn watcher_count(&self) ->
  usize { self.watchers.len() }` (the `set_pre_mirror_gate` precedent for
  test-only surface). The production gauge is a deliberate follow-up.

## Implementation steps (per file)

### `src/store.rs` — behaviour

1. **`WATCHER_CAP` + atomic registration.** Replace the get-then-insert
   match (374–384) with one path:

   ```rust
   const WATCHER_CAP: usize = 16;

   pub fn subscribe(&self, inbox: &str) -> broadcast::Receiver<Arc<Email>> {
       self.watchers
           .entry(inbox.to_string())
           .or_insert_with(|| broadcast::channel(WATCHER_CAP).0)
           .subscribe()
   }
   ```

   `or_insert_with` runs the constructor only when vacant, so the
   `second_subscriber_on_the_same_inbox_gets_the_live_channel` invariant is
   preserved; that test's "the Some(sender) branch" comment dies with the
   branch. One `String` allocation per subscribe — accepted, and the
   get-fast-path alternative is refused (a second path is the bug being
   deleted).

2. **Release on drain.** `drain_inbox` gains a `watchers` parameter and one
   unconditional `watchers.remove(inbox)` — the drain is where an inbox
   dies, so it is where its channel dies; `clear` and `clear_all` inherit
   by passing `&self.watchers`. A `clear` of a nonexistent inbox removes
   nothing (the no-op path already covered by
   `get_delete_and_clear_edge_cases`).

3. **Release on delete-empties.** In `delete`, after the `retain`, when
   `list.is_empty()` remove `self.watchers` entry for that inbox. Disjoint
   map borrows (inboxes guard held, watchers mutated) — no re-entrancy, no
   shard cycle.

4. **The `clear_all` sweep.** After the per-inbox drain loop:
   `self.watchers.clear()`. This releases channels for
   awaited-but-never-mailed inboxes — the population #14's 5,000-inbox
   probe creates, which no per-inbox drain hook can reach because those
   inboxes leave no entries to iterate. A subscriber racing the sweep sees
   `Closed` and takes the re-subscribe arm (bounded: one extra rescan +
   re-subscribe cycle per waiter per sweep).

5. **The closure-aware wait loop.** In `wait_for`, replace the discarded
   `let _ = timeout(remaining, rx.recv()).await;` with a match:

   ```rust
   match tokio::time::timeout(remaining, rx.recv()).await {
       Ok(Ok(_)) | Ok(Err(RecvError::Lagged(_))) => {}
       Ok(Err(RecvError::Closed)) => {
           // The inbox's mail was removed and its channel released:
           // re-apply decision 0002 per iteration — subscribe, then scan.
           rx = self.subscribe(inbox);
           #[cfg(test)]
           self.run_post_subscribe_gate();
       }
       Err(_elapsed) => {} // parked past the deadline; the next scan reports it
   }
   ```

   (`use tokio::sync::broadcast::error::RecvError;` added.) The loop's
   module doc (396–403) and the `watchers` field doc are updated: a closed
   channel re-subscribes, it no longer "falls through to the deadline
   re-check"; `insert`'s notify comment notes a missing entry means no
   subscribers yet (or released) and waiters re-subscribe.

### `src/store.rs` — the test-only seam (settlement pattern)

`post_subscribe_gate`, mirroring `pre_mirror_gate` exactly: a
`#[cfg(test)] Mutex<Option<Arc<dyn Fn() + Send + Sync>>>` field, a
`set_post_subscribe_gate` setter, a `run_post_subscribe_gate` runner, fired
after **every** `subscribe` inside `wait_for` (the initial one and each
re-subscribe — two call sites). It carries wave 1's unsettled
subscribe→scan window too, so the seam serves both waves. Compiled out of
the shipping binary; self-covering: the `Some` arm by the two gate-driven
tests, the `None` arm by every existing wait test.

### `src/store.rs` — unit tests (all in the existing `#[cfg(test)]` modules)

1. `concurrent_first_subscribers_all_attach_and_all_see_the_insert` —
   plain `#[test]`, `std::thread::scope`, `Barrier::new(8)`: eight threads
   subscribe one fresh inbox, join; one insert; every receiver
   `try_recv`s the mail. Seam-free by design (AC1).
2. `a_cleared_inbox_releases_its_watcher_channel` — subscribe → `clear` →
   `try_recv() == Err(TryRecvError::Closed)` (AC2-i). Fully deterministic:
   the map held the only sender.
3. `a_delete_that_empties_an_inbox_releases_its_watcher_channel` — two
   mails, delete one (channel still live), delete the last (`Closed`)
   (AC2-ii).
4. `clear_all_releases_channels_of_awaited_never_mailed_inboxes` — N fresh
   subscriptions, `watcher_count() == N`, `clear_all()`, `== 0` (AC2-iii),
   plus the nonexistent-inbox no-op already covered (AC2-iv).
5. `a_waiter_on_a_released_channel_is_not_stranded` — `#[tokio::test]`,
   gate: fire 1 → `clear("box")`; fire 2 → insert. `wait_for` returns
   `Found` (AC3). Red on no-release / no-resubscribe / sleep-through; the
   asserted state is unreachable without the controlled interleaving
   (vacuity rule).
6. `a_waiter_on_a_released_channel_sleeps_until_its_deadline` —
   `#[tokio::test(flavor = "current_thread")]`, a 5 ms ticker co-task
   (`AtomicU64` counter), gate fire 1 → clear, fire 2 → no-op; deadline
   200 ms. Assert `Timeout { matched: 0 }` **and** ticker ≥ 10 (AC4). A
   reintroduced spin starves the single worker and collapses the count.
7. `wait_for_covers_the_lagging_watcher` — unchanged in assertion; its
   comment now says "overflow the 16-slot wake ring" (AC5).

### `src/feed.rs`

- Rewrite the false comment above `Err(RecvError::Closed) => None`: the
  sender is released when the inbox's mail is removed (decision 0010), the
  stream ends, and the browser's `EventSource` reconnect re-establishes
  0002 (`feed_stream` subscribes before the first view scan) with the
  post-drain snapshot. No code change to the arm.
- New test `a_cleared_inbox_ends_the_feed_after_the_snapshot`:
  `feed_events` with a store-subscribed receiver → snapshot frame →
  `store.clear("box")` → next poll is `None` (AC4, feed half).
  Deterministic by injection (`feed_events` takes `rx`).

### `tests/burst.rs`

- `await_survives_a_release_and_finds_later_mail` — real server
  (`start_with`): (1) `await` on a fresh inbox with a short timeout →
  honest `Timeout { matched: 0 }` (creates the channel); (2) `clear` the
  inbox via the API (releases it); (3) insert via the real SMTP helper;
  (4) `await` again → finds the mail. Proves a released inbox keeps
  serving waiters over the wire (AC6), no mocks.

### Gates

All six run at the end (tasks.md). No `deny.toml` change (nothing new);
no `coverage-gate.sh` change (the floor stays exactly 100).

## Risks

| Risk | Mitigation |
|---|---|
| **The SSE feed now ends on drain** — a client on a cleared inbox sees its stream close. | Deliberate, spec'd, and recorded in proposed decision 0010. The reconnect re-establishes 0002 at the wire boundary and re-snapshots the (empty) view; keeping the feed alive would require the server to hold a sender clone — reintroducing the leak through the front door. Pinned by the new feed test. |
| **Lag arrives sooner (16 vs 4096 slots).** | Repaired by the rescan that already exists and is already pinned (`wait_for_covers_the_lagging_watcher`, unchanged assertion). `send` is non-blocking at any capacity → no 250-path delay (0001). |
| **The `clear_all` sweep drops a channel a racing subscriber just registered.** | The subscriber's `wait_for` sees `Closed`, re-subscribes, re-scans — 0002's per-iteration ordering is the mechanism that makes blunt sweeps safe; cost is one extra rescan + re-subscribe cycle per waiter per sweep. |
| **Entry allocates a `String` per subscribe and holds the shard write lock across channel creation.** | Once per long-poll/SSE connection and once per inbox respectively; microseconds against a 1.58 µs/mail ingest path. The fast-path alternative re-opens a second code path — the bug being deleted. |
| **Residue is not zero: awaited-but-never-cleared inboxes hold ~0.5 KiB each until a drain or sweep.** | Bounded (was: forever), documented in the spec, and pinned by the sweep test. Full reclamation needs last-receiver drop hooks (guard/weak-map machinery) — refused as a new bug class by both seats. |
| **The ticker probe could flake on a loaded runner.** | Gate-ordered setup (no sleep-and-hope), a lenient margin (≥ 10 of ~40 ticks), and a recorded fallback: if it proves flaky in CI, raise the margin or drop to the outcome-level `not_stranded` test — the structural no-spin invariant stays the reviewed property. Its red-on-today's-code is probabilistic under cooperative budgeting; its regression teeth (a spin starves its own worker) are structural. |
| **New seam + accessor lines enter the 100% denominator.** | Self-covering by construction (gate `Some` arm ← the two gate tests; `None` arm ← every existing wait test; accessor ← the sweep test), one-time, compiled out of production. An uncovered seam is the signal its exemplar rotted — the gate doing its job. |
| **`delete`-empties release churns in selective-delete loops.** | Each release is a map remove; a parked waiter pays one rescan + re-subscribe; a non-emptying delete is untouched. |
| **A crash between accept and commit / hostile bodies / hung webhooks.** | Out of this change's blast radius and answered upstream (0001's ordering untouched; extraction at ingest; `POST_TIMEOUT`-bounded dispatch). The wake is an optimization over the store-is-truth rescan and is proven by deterministic tests, not probabilistic injection. |

## Quickstart (reviewer)

```bash
cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings
cargo test                                  # real servers, no mocks
cargo test --lib store::wait_tests -- --nocapture
cargo test --lib feed::tests -- --nocapture
cargo test --test burst await_survives_a_release -- --nocapture
brokkr compile --bundle .
bash scripts/coverage-gate.sh
cargo deny check licenses
```
