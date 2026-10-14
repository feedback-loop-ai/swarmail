# Tasks — epic #13 wave 3 completion (the parked WIP `fa93a61` under `a88d601`)

Ordered; each task names the test that proves it. Nothing here re-derives a settled
task: the WIP already carries tasks 1–5 and 7–9 of
[`specs/watcher-atomicity-release/tasks.md`](../watcher-atomicity-release/tasks.md)
materially — this list is the remainder, exactly. `src/feed.rs` and `tests/burst.rs`
are verify-only (task 5); no new files, no new dependencies.

- [ ] 1. Restore the `Closed` arm of `wait_for`'s recv match (`src/store.rs:536-543`) to
       decision 0002's per-iteration ordering: delete the `// MUTATION B: no re-subscribe
       (pre-fix spin)` comment and the misfiring `run_post_subscribe_gate()` call; write
       `rx = self.subscribe(inbox);` **first**, then `#[cfg(test)]
       self.run_post_subscribe_gate();` — the gate fires only where a subscribe happened
       (the wave-1 placement discipline). `rx` is already `let mut`.
       — *Proved by: `a_waiter_on_a_released_channel_is_not_stranded` returns
       `Found` with the gate's `fires == 2` (AC1). Redness restored with no new test:
       with the re-subscribe removed the gate never fires again → `fires == 1` →
       `Timeout`. The release/sweep/lag tests (AC5) prove no collateral change.*

- [ ] 2. Gate the probe: keep `static CLOSED_PROBE` (`src/store.rs:24-25`, `#[cfg(test)]`)
       and wrap its arm write in `#[cfg(test)]` (`#[cfg(test)]
       CLOSED_PROBE.fetch_add(1, Ordering::Relaxed);` after the gate call); delete the
       `PROBE_THREAD` `thread_local!` (`:27-30`) and both uses (`:884`, `:901`) and the
       `eprintln!("PROBE …")` block (`:897-903`). `AtomicU64`/`Ordering` stay imported
       (`Totals` uses them). The arm fix and this gating land in the same commit.
       — *Proved by: `cargo check` exits 0 and `cargo test --no-run` links every
       integration target including `tests/burst.rs` — `E0425: cannot find value
       CLOSED_PROBE` (src/store.rs:540) is gone (AC2); `bash scripts/coverage-gate.sh`
       keeps missed == 0 (the counter's two executed lines are covered by tasks 1/3's
       tests) and the shipping binary contains neither counter nor write.*

- [ ] 3. Add the deterministic no-spin teeth to
       `a_waiter_on_a_released_channel_sleeps_until_its_deadline`: snapshot
       `CLOSED_PROBE` before the wait, and after the existing `Timeout { matched: 0 }`
       and ticker asserts add the delta assert `hits <= 4` (delta, not absolute — the
       counter is process-global and the harness runs tests in parallel). Do **not**
       re-tune the ticker margin.
       — *Proved by: the test itself (AC3/AC4) — post-completion the delta is exactly 1
       (one gate-ordered release, one arm hit) while the ticker still advances ≥ 10 of
       ~40; under a reintroduced no-re-subscribe the delta is 202,455–276,432 (measured
       this run), so the bound is red on exactly the regression the wave names.*

- [ ] 4. Run the six gates in `specs/watcher-atomicity-release/tasks.md`'s order —
       `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
       `cargo test` (real servers, no mocks), `brokkr compile --bundle .`,
       `bash scripts/coverage-gate.sh` (exact 100/100/100, missed == 0, no
       `coverage(off)`), `cargo deny check licenses` — and fix **only what they expose**.
       — *Proved by: the six clean outputs (AC7); anything an exposure requires beyond
       `src/store.rs` and the checkbox ticks goes back through design, not a silent
       edit (the gate doing its job).*

- [ ] 5. Verify the verify-only surface: `src/feed.rs` (the corrected `Closed`-arm
       comment, `a_cleared_inbox_ends_the_feed_after_the_snapshot`) and
       `tests/burst.rs` (`await_survives_a_release_and_finds_later_mail`) are materially
       correct in the WIP — assert it by running their proofs, and edit them only if a
       gate from task 4 exposed a defect.
       — *Proved by: `cargo test --lib feed::` (the stream ends on the clear: next poll
       is `None`) and `cargo test --test burst await_survives_a_release` against the
       real server — honest `408/matched: 0`, API clear, SMTP insert, second await
       finds the mail (AC5 feed half, AC6).*

- [ ] 6. Tick the ten checkboxes in `specs/watcher-atomicity-release/tasks.md` — after
       the gates, never before; do not edit the ratified `spec.md` or decision 0021
       (task 10 there is verify-only: the decision exists, is `proposed`, and is
       referenced from its spec).
       — *Proved by: the ticked file diff and the green suite behind each tick (AC7);
       `docs/decisions/0021-watcher-channel-lifetime.md` unchanged with status
       `proposed`.*

- [ ] 7. Land the completion as **one** commit on top of `HEAD`, repository style
       (imperative summary, a body that says why — plan.md §5 has the suggested
       message): `src/store.rs`, the ticked
       `specs/watcher-atomicity-release/tasks.md`, and nothing else. Never push.
       — *Proved by: `git show --stat HEAD` — exactly `src/store.rs` +
       `specs/watcher-atomicity-release/tasks.md` (feed.rs/burst.rs appear only if a
       gate forced them, and the body says so in one sentence); `commit.gpgsign`
       resolves false in this worktree and all recent commits are unsigned, so commit
       like the rest of the history (AC7).*
