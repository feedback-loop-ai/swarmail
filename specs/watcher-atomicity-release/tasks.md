# Tasks — watcher registration atomicity (#3) + watcher channel release (#14)

Ordered; each task names the test that proves it. All gates
(`cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test`, `brokkr compile --bundle .`, `bash scripts/coverage-gate.sh`,
`cargo deny check licenses`) run after task 10 and before reporting.

- [x] 1. Add `const WATCHER_CAP: usize = 16;` (documented as a wake ring:
       both consumers discard the payload; lag is repaired by the rescan)
       and replace `Store::subscribe`'s get-then-insert match
       (`src/store.rs:374-384`) with the single atomic
       `entry().or_insert_with()` path. Update the `watchers` field doc
       (creation + release policy) and drop the dead "the Some(sender)
       branch" comment in `second_subscriber_on_the_same_inbox_gets_the_live_channel`.
       — *Proved by: `second_subscriber_on_the_same_inbox_gets_the_live_channel`
       (unchanged assertion) and the new
       `concurrent_first_subscribers_all_attach_and_all_see_the_insert`
       (AC1): plain `#[test]`, `std::thread::scope`, `Barrier::new(8)` —
       eight threads subscribe one fresh inbox, join, one insert, every
       receiver `try_recv`s the mail, zero missed; green under every
       schedule post-fix, red on the pre-fix code whenever two threads
       overlap.*

- [x] 2. Release on drain: give `drain_inbox` a `watchers` parameter and one
       unconditional `watchers.remove(inbox)`; pass `&self.watchers` from
       `clear` and `clear_all`. Comment (per the panel ratification): the
       drain is where an inbox dies, so it is where its channel dies — an
       *empty* clear may still release a channel (awaited-but-never-mailed
       inbox), miss-free because the waiter answers `Closed` with a
       re-subscribe before its next scan (0002 per iteration); the
       nonexistent-inbox "no-op" (AC2-iv) means *removes no mail, returns
       0*, not channel immortality. Do not condition the remove on the
       drain having found mail.
       — *Proved by: `a_cleared_inbox_releases_its_watcher_channel`
       (AC2-i): subscribe → `clear` → `try_recv() == Err(TryRecvError::Closed)`;
       the nonexistent-inbox no-op path stays covered by
       `get_delete_and_clear_edge_cases` (AC2-iv).*

- [x] 3. Release on delete-empties: in `delete`, when the `retain` leaves
       the inbox list empty, remove that inbox's watcher channel.
       — *Proved by: `a_delete_that_empties_an_inbox_releases_its_watcher_channel`
       (AC2-ii): two mails; delete one → a held receiver still receives a
       pushed insert (channel alive); delete the last → `try_recv()` is
       `Closed`.*

- [x] 4. Add the `clear_all` sweep: after the per-inbox drain loop,
       `self.watchers.clear()` — releasing channels of
       awaited-but-never-mailed inboxes (issue #14's 5,000-fresh-inbox
       probe population, which leaves no entries to iterate).
       — *Proved by: `clear_all_releases_channels_of_awaited_never_mailed_inboxes`
       (AC2-iii): N subscriptions on fresh inboxes, `watcher_count() == N`,
       `clear_all()`, `watcher_count() == 0`, via the new
       `#[cfg(test)] pub(crate) fn watcher_count(&self) -> usize`
       accessor (`set_pre_mirror_gate` precedent).*

- [x] 5. Add the `post_subscribe_gate` seam (`pre_mirror_gate` shape, ~19
       `#[cfg(test)]` lines): field + `set_post_subscribe_gate` setter +
       `run_post_subscribe_gate` runner, fired after **every** `subscribe`
       inside `wait_for` (initial and each re-subscribe — two call sites).
       — *Proved by: the seam's `Some` arm is executed by tasks 6 and 7's
       gate-driven tests; the `None` arm by every existing wait test;
       `bash scripts/coverage-gate.sh` stays at exact 100/100/100
       (missed == 0).*

- [x] 6. Make the wait loop closure-aware: match on
       `tokio::time::timeout(remaining, rx.recv())` — `Ok(Ok(_))` and
       `Ok(Err(Lagged(_)))` fall through as today; `Ok(Err(Closed))`
       re-subscribes (then fires the gate) and falls to the next scan
       (decision 0002 per iteration); the elapsed arm falls to the next
       scan, which reports the `Timeout`. Update `wait_for`'s module doc
       and `insert`'s notify comment (a missing watcher entry now means
       "no subscribers yet, or released").
       — *Proved by:
       `a_waiter_on_a_released_channel_is_not_stranded` (AC3): gate fire 1
       → `clear("box")` (releases the waiter's channel), fire 2 → insert;
       `wait_for` returns `Found`. Red on all three regressions —
       no-release (the insert never happens: it lives in the re-subscribe
       gate → parks to `Timeout`), no-re-subscribe (gate never fires again
       → `Timeout`), sleep-through-on-`Closed` (`Timeout`) — and the
       asserted state is unreachable without the controlled interleaving
       (vacuity rule). The `Lagged` arm stays pinned by
       `wait_for_covers_the_lagging_watcher`; the elapsed arm by
       `wait_for_times_out_with_the_match_count`.*

- [x] 7. Prove the no-spin property: add
       `a_waiter_on_a_released_channel_sleeps_until_its_deadline` —
       `#[tokio::test(flavor = "current_thread")]`, a 5 ms co-scheduled
       ticker (`AtomicU64`), gate fire 1 → clear, fire 2 → no-op, 200 ms
       deadline; assert `Timeout { matched: 0 }` and ticker ≥ 10 of ~40.
       A reintroduced spin starves the single worker and collapses the
       count. No wall-clock CPU-percentage assertion anywhere.
       — *Proved by: the test itself (AC4, wait half); if it proves flaky
       in CI the recorded fallback is a larger margin or the outcome-level
       task-6 test (plan.md, Risks).*

- [x] 8. Correct `src/feed.rs`'s now-false `Closed`-arm comment (the sender
       is released when the inbox's mail is removed — decision 0021; the
       stream ends and the browser's `EventSource` reconnect re-establishes
       0002 with the post-drain snapshot). No code change to the arm.
       — *Proved by: `a_cleared_inbox_ends_the_feed_after_the_snapshot`
       (AC4, feed half): `feed_events` with a store-subscribed receiver →
       snapshot frame → `store.clear("box")` → next poll is `None`;
       deterministic by injection (`feed_events` takes `rx`).*

- [x] 9. Add the real-protocol guard in `tests/burst.rs`:
       `await_survives_a_release_and_finds_later_mail` — `await` on a fresh
       inbox (short timeout, honest `matched: 0`), `clear` the inbox via
       the API, insert via the real SMTP helper, `await` again → finds the
       mail. Real server, no mocks.
       — *Proved by: the test itself (AC6); run with
       `cargo test --test burst await_survives_a_release -- --nocapture`.*

- [x] 10. Verify the ruling is recorded (verify-only — it was committed with
        the spec at `126632a`):
        `docs/decisions/0021-watcher-channel-lifetime.md` (status
        `proposed`) exists and is referenced from `spec.md`. Do not
        re-edit the decision or the spec; any contradiction found during
        implement goes back through the design phase, not a silent edit.
        No edit to accepted decisions; the deferred `/metrics` gauge is
        recorded in the spec's out-of-scope section with its follow-up
        shape.
        — *Proved by: the decision file exists with `proposed` status and
        the spec references it by relative path; `brokkr compile --bundle .`
        and the other five gates stay green.*
