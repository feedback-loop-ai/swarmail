# Spec — epic #13 wave 3 completion: the watcher release lands, and the suite can prove it

Epic #13, wave 3 (continuation) · issues
[feedback-loop-ai/swarmail#3](https://github.com/feedback-loop-ai/swarmail/issues/3) and
[#14](https://github.com/feedback-loop-ai/swarmail/issues/14) · parent epic
[#13](https://github.com/feedback-loop-ai/swarmail#13).

Companion to the ratified wave spec
[`specs/watcher-atomicity-release/spec.md`](../watcher-atomicity-release/spec.md) — that spec
owns the *design* (wake ring, atomic registration, release-with-the-mail, closure-aware wait);
this spec owns the *completion* of its parked implementation. Decision
[0021](../../docs/decisions/0021-watcher-channel-lifetime.md) (status `proposed`, the operator
accepts) already records the public-contract deltas; **no new decision document is drafted
here** — the completion touches no accepted ruling, adds no production surface, and its only
new test assertion is test-only code. The two measured defects routed out of this wave are
recorded below in writing; the next free decision number is **0011**.

## The problem, as the tree holds it (all measured this run)

The watcher implementation exists in the tree as the parked WIP `fa93a61`
(`src/store.rs` +348/−18, `src/feed.rs` +31, `tests/burst.rs` +57) under `a88d601`. Verified
against `specs/watcher-atomicity-release/tasks.md`: tasks 1–5 and 7–9 are materially done
(`WATCHER_CAP = 16` + the atomic `entry().or_insert_with()` registration, unconditional
drain release, delete-empties release, the `clear_all` sweep + `watcher_count()`, the
`post_subscribe_gate` seam, the sleeps test, the feed comment + test, the real-protocol AC6
test). What remains is one defect with three faces, plus a measured hole the WIP did not
create and must not silently carry:

1. **The `Closed` arm is a live pre-fix spin** (`src/store.rs:538-543`). Instead of
   plan.md step 5's re-subscribe, the arm carries a mutation-testing leftover — a literal
   `// MUTATION B: no re-subscribe (pre-fix spin)` comment and a probe counter where the
   subscribe should be. Measured twice this run: **202,455** and **276,432** `Closed`-arm
   hits inside one 200 ms wait — a ~1.4 M/s spin per waiter on a released inbox. The
   tree's own `insert` doc comment already *claims* the re-subscribe happens; the
   documentation is ahead of the code.
2. **The probe reference breaks every non-test build.** `CLOSED_PROBE` is declared
   `#[cfg(test)]` (`src/store.rs:24-25`) but referenced ungated at `src/store.rs:540`:
   `cargo check` fails with `E0425`, so `cargo test --test burst` cannot link and no
   Rust-building gate can go green. The wave's own required suite is unrunnable until this
   lands. The WIP was committed with a gate-failing state; the completion must not repeat
   that — the arm fix and the probe gating land in the **same commit**.
3. **The gate misfires, so the suite cannot see the spin.** `post_subscribe_gate`'s contract
   is "runs after *every* `subscribe` inside `wait_for`", but the WIP fires it *inside* the
   arm without a subscribe. The misfire performs the test's fire-2 insert for free, so
   `a_waiter_on_a_released_channel_is_not_stranded` stays green under the mutation — the
   claimed redness mechanism (no re-subscribe → gate never fires again → `Timeout`) is
   disconnected.
4. **The ratified spin probe is measured not to bite.** The sleeps test passed while the
   spin was live: the 5 ms ticker advanced **33 of ~40 ticks** under 202K–276K arm hits —
   tokio's cooperative budget yields the spinner periodically, so the collapse the spec
   predicted (≈ 0 ticks) does not happen; a ~17 % dip is the worst case. The ratified
   spec's "structural" claim and its probabilistic-loss hedge are both now *measured
   facts*; per task 10's rule the ratified spec and decision 0021 are not edited — the
   contradiction returns through design, and **this spec is that return**.
5. **Two same-family races stand one door down** (measured, pre-existing, not introduced by
   the WIP — routed, see below): `delete` racing `clear` panics on its `.expect`
   (`src/store.rs:405`) at **247 / 30,000** hammered rounds, and `insert` racing `clear`
   strands an accepted mail outside its inbox (`index.insert` at `src/store.rs:330` runs
   after the inboxes shard guard drops) at **3,952 / 30,000 (~13 %)** — a hole in
   guarantee 0001's listable sense: `get(id)` → `Some`, `list(inbox)` → 0.

## The committed design — a completion, not a redesign

One policy, stated once: **the parked WIP is the ratified design; the completion restores
its one unfinished line, deletes everything in the file that exists only to observe the
bug except what the suite needs to *prove* the bug stays dead, and refuses to widen the
surface.** No restart, no re-derivation of settled tasks, no new files, no new
dependencies, no new vocabulary.

### (1) The `Closed` arm is restored — re-subscribe first, gate second

The arm becomes exactly plan.md step 5's shape, plus the probe write that turns it into
teeth (below), all probe lines `#[cfg(test)]`:

```rust
Ok(Err(RecvError::Closed)) => {
    // The inbox's mail was removed and its channel released:
    // re-apply decision 0002 per iteration — subscribe, then scan.
    rx = self.subscribe(inbox);
    #[cfg(test)]
    self.run_post_subscribe_gate();
    #[cfg(test)]
    CLOSED_PROBE.fetch_add(1, Ordering::Relaxed);
}
```

`rx` is already `let mut`. This is decision 0002 re-applied per iteration, which is what
makes release safe (no miss, no spin) and what restores `not_stranded`'s redness
mechanism *with no new test*: a reintroduced no-re-subscribe means the gate never fires
again, the fire-2 insert never happens, and the wait parks to `Timeout` with
`fires == 1`. The gate fires only where a `subscribe` happened — the wave-1
`pre_mirror_gate` placement discipline verbatim; the misfiring third call site dies with
the mutation.

### (2) The probe becomes teeth, not a leftover

- **Kept, gated to test builds:** the `CLOSED_PROBE` counter — the only deterministic
  instrument for the property this wave exists to ship ("waiters sleep, not spin").
  Compiled out of the shipping binary like the seam; its two executed lines are
  self-covering at the exact 100 % gate.
- **Deleted:** `PROBE_THREAD`, the `eprintln!("PROBE …")` diagnostic block
  (`src/store.rs:884, 897-904`), and the `MUTATION B` comment. A probe that *reports* is
  deleted with the bug; a probe that *asserts* stays.
- **New teeth, one assertion in the existing sleeps test** — a **delta bound**, immune to
  parallel test binaries' hits (the counter is process-global):

  ```rust
  let hits_before = CLOSED_PROBE.load(Ordering::Relaxed);
  // … the wait …
  let hits = CLOSED_PROBE.load(Ordering::Relaxed) - hits_before;
  assert!(
      hits <= 4,
      "a single release must be answered with one Closed arm hit, not {hits}"
  );
  ```

  A parked waiter's `Closed` arm fires **once per release** (measured post-fix: 1); a
  spinning one fires hundreds of thousands of times (measured under the mutation:
  202,455–276,432). The bound is deterministic — the gate orders the single release —
  needs no new production surface, no wall-clock reading, and no CPU-percentage claim,
  and it is red on exactly the regression the wave names. `AtomicU64`/`Ordering` stay
  imported (`Totals` uses them).

### (3) The ticker canary ships unchanged — and is honestly demoted

`a_waiter_on_a_released_channel_sleeps_until_its_deadline` keeps its ratified assertions
(`Timeout { matched: 0 }` at the 200 ms deadline; ticker ≥ 10 of ~40) with **no
re-tuning** — that is new tuning vocabulary with its own flake risk. But its role is now
recorded truthfully: a **canary**, not teeth — measured 33-of-40 under a live spin. The
deterministic delta bound above is the regression. The recorded fallback (drop to the
outcome-level `not_stranded` assertion if the ticker proves flaky in CI) stands.

### (4) What is already right and is not touched

`src/feed.rs` (the corrected `Closed`-arm comment + `a_cleared_inbox_ends_the_feed_after_the_snapshot`)
and `tests/burst.rs`
(`await_survives_a_release_and_finds_later_mail`) are **verify-only**: their parked state is
correct and their tests prove it; the only reason the burst test fails today is the lib
build break. No edits are expected in either file; if a gate exposes one, fix exactly what
is exposed. Decision 0021 and `specs/watcher-atomicity-release/spec.md` are untouched
(task 10 is verify-only); the ten checkboxes in `specs/watcher-atomicity-release/tasks.md`
are ticked by implement **after** the gates prove them, never before.

## The ruling — the panel's two positions, adopted / rejected / reconciled

Both seats verified the same tree and agree on the skeleton: restore the arm (re-subscribe
*before* the gate), delete the ungated-reference build break, refuse the `/metrics` gauge,
the idle-TTL sweeper, subscriber-count release machinery, the `watch`/`Notify` swap, any
wall-clock CPU-percentage assertion, the compile-only fence (`#[cfg(test)]` on the
`fetch_add` line while keeping the mutation), and any edit to 0021 or the ratified spec.
They split on the probe's fate and on scope. Ruled, not averaged:

| Conflict | Ruling | Cost paid by the losing side |
|---|---|---|
| **The probe counter**: simplicity deletes *all* probe scaffolding ("a probe kept is a permanent 100 %-denominator obligation"); robustness keeps it and adds the deterministic arm-hit bound | **Robustness wins the counter; simplicity wins the deletion of everything else.** The counter stays, gated, *as the bound's instrument* — the only deterministic teeth for the wave's core property, free (already written), and self-covering. `PROBE_THREAD`, the `eprintln!`, and the mutation comment go. Simplicity's own reason ("a probe that exists to observe a bug is deleted with the bug") does not apply to a probe that *asserts the invariant*: the build break was the *ungated reference*, which dies either way; the 100 % obligation is two lines the two existing gate tests execute. | Simplicity's side carries ~2 permanent `#[cfg(test)]` lines in the coverage denominator. Robustness's side loses the probe as a *diagnostic*: nothing prints in CI; the assert message is the only report. |
| **The ticker probe's status**: the ratified spec calls its teeth "structural"; both seats measured that claim false (33 of ~40 ticks advanced under a 202K-iteration spin) | **Keep the ticker exactly as ratified — as a canary, not teeth — and record the demotion here** rather than editing the ratified spec (task 10 is verify-only; this document is the design-phase return of that contradiction). No margin re-tuning. | The honest loss recorded at ratification is now a measured fact: the no-spin property's *guarantee* rests on the restored code shape plus the delta bound, not on the ticker. |
| **`not_stranded` cannot distinguish a real re-subscribe from a misfired gate** (robustness F2) | **Restoring the task-6 order repairs the mechanism, no new test** (simplicity's point, adopted): under the restored arm, a no-re-subscribe regression means `fires == 1` → `Timeout` → red. The seam's contract stays one reviewed line — fire only after `subscribe()` returns. | Robustness's demand for a *stronger* seam is refused; the protection remains review discipline on one line, not machinery. |
| **Fold in the `delete`-races-`clear` panic and the `insert`-races-`clear` stranding (F4/F5)**: robustness argues it is the wave's own thesis applied one door down, inside the declared `src/store.rs` surface; simplicity refuses anything beyond the watcher contract | **Routed out of this wave, in writing — not refused, not deferred by oversight** (see the next section). The deciding asymmetry: the fix changes the store's critical-section discipline and `delete`'s public failure mode across four protocol surfaces, which the house rules say carries its own 0001-adjacent decision document (0020) and its own review — a rider on a deadline-bounded completion (the implement seat has already missed one 3,600 s deadline; the budget is now 7,200 s) couples two risk surfaces and re-rolls the wave. This is robustness's own recorded fallback sentence, adopted as the ruling. | Robustness's side pays in time: the measured hole persists one more change — 247/30,000 delete-panics and ~13 % stranded inserts under clear-race load — recorded with numbers and shape so the next intake cannot miss it. Simplicity's side pays in debt: the "delete the window, not fence it" thesis now has a documented exception until 0020 lands. |
| **Hostile DATA bodies** (non-UTF-8 line kills the session with no SMTP reply while EHLO advertises `8BITMIME`/`SMTPUTF8`; unbounded line growth) | **Adopt robustness's P2 as routing**: recorded with measurements and shape, its own change (byte-oriented `read_until` loop, bounded buffer, `552` after the terminator, `502` for undecodable command lines, real-protocol proof in `tests/smtp_edges.rs`). | The protocol lie persists this wave: a hostile peer burns one connection per attempt at zero cost to itself. Outside the declared three-file surface (`src/smtp.rs` + a new test file), so it cannot ride this wave without breaking the framing's own contract. |

Adopted from simplicity: no restart (nine of ten tasks are materially in the tree);
no new test *files*; feed.rs/burst.rs verify-only; checkbox ticking only after the gates;
one commit; no spec/decision edits; every refusal above.

Adopted from robustness: the arm-restoration order as *the* fix; the build-break diagnosis
and the same-commit rule; the deterministic delta bound; the routing discipline (deferrals
recorded with measurements, shape, and their own decision requirement).

Rejected from both: simplicity's wholesale probe deletion (it deletes the only honest
teeth); robustness's P1 fold-in (right disease, wrong vehicle).

## Routed out of this wave, in writing

These are **decisions of this spec**, recorded so no later seat mistakes them for oversights.

- **0001-adjacent concurrency repair (robustness F4/F5) → its own change with decision 0020
  (next free number).** Two statements, as robustness specified: `delete`'s tail becomes
  fallible — if the inbox's list is absent the drain already removed the mail, so return
  `true` (the mirror's `record_delete` is idempotent) and drop the `.expect`
  (`src/store.rs:405`); `insert` holds the inboxes shard guard across `index.insert`
  (`src/store.rs:330` moves inside the entry block — precedent: the pruning loop already
  touches `index` under that guard; no AB-BA hazard exists — no path holds an *index* guard
  across an *inboxes* access). Regression tests follow the settlement-seam pattern
  (`pre_delete_gate` between the `index.remove` and the `get_mut`; the insert seam placed
  after the entry block, never inside it — a gate calling `clear()` under the shard guard
  deadlocks, the trap plan.md's risk table already names). Evidence carried forward:
  **247/30,000** panic rounds; **3,952/30,000 (~13 %)** stranded ghosts; reachable from
  POP3 `QUIT`, MCP delete, compat bulk delete, REST `DELETE /api/v1/messages/{id}`. The
  shipped suite does not trip it today because each test file binds its own server.
- **Hostile DATA bodies → their own change** (shape above; evidence: a real-TCP session
  sending `b"\xff\xfe\x80 not utf8\r\n"` after `DATA` gets EOF with no reply; a single
  unterminated line grows `line` unboundedly — `over_limit` bounds `raw`, not `line`).
- **The `/metrics` gauge** (`swarmail_watchers`): stays de-scoped per the ratified spec's
  recorded follow-up shape; `watcher_count()` remains the test-observable count.
- **The chaos `post_accept` hook** (robustness's crash-window test surface): its own
  test-surface change, not this wave.

## Acceptance Criteria

- **AC1 — The `Closed` arm re-subscribes before the gate fires; no released waiter is
  stranded.** `a_waiter_on_a_released_channel_is_not_stranded` returns `Found` with the
  gate's `fires == 2`; under a reintroduced no-re-subscribe the same test is red
  (`fires == 1` → `Timeout`), because the fire-2 insert lives in the re-subscribe gate.
- **AC2 — The crate builds for every target.** `cargo check` exits 0 and
  `cargo test --no-run` links every integration target including `tests/burst.rs` —
  `E0425: cannot find value CLOSED_PROBE` is gone. The arm restoration and the probe
  gating land in the same commit.
- **AC3 — The no-spin property has deterministic teeth.**
  `a_waiter_on_a_released_channel_sleeps_until_its_deadline` asserts the `Closed`-arm
  **delta** over its 200 ms window is ≤ 4 (measured post-completion: 1; measured under
  the mutation: 202,455–276,432). The counter and the assertion are `#[cfg(test)]`;
  production builds contain neither.
- **AC4 — The ratified canary is unchanged.** The same test still asserts
  `Timeout { matched: 0 }` and the 5 ms ticker advanced ≥ 10 of ~40; no margin re-tuning;
  no wall-clock CPU-percentage assertion anywhere; `PROBE_THREAD`, the `eprintln!`, and
  the `MUTATION B` comment no longer exist.
- **AC5 — The settled watcher material is green and untouched.**
  `concurrent_first_subscribers_all_attach_and_all_see_the_insert`,
  `a_cleared_inbox_releases_its_watcher_channel`,
  `a_delete_that_empties_an_inbox_releases_its_watcher_channel`,
  `clear_all_releases_channels_of_awaited_never_mailed_inboxes`,
  `wait_for_covers_the_lagging_watcher` (assertion frozen),
  `second_subscriber_on_the_same_inbox_gets_the_live_channel`, and
  `a_cleared_inbox_ends_the_feed_after_the_snapshot` pass with no edits to their bodies.
- **AC6 — The real-protocol guard runs against a real server.**
  `cargo test --test burst await_survives_a_release` passes: an honest `408/matched: 0`
  await, an API clear, an SMTP insert, and a second await that finds the mail — no mocks.
- **AC7 — All six house gates green; the record closed.** `cargo fmt --all -- --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`,
  `brokkr compile --bundle .`, `bash scripts/coverage-gate.sh` (exact 100/100/100,
  missed == 0, no `coverage(off)`), `cargo deny check licenses`; the ten checkboxes in
  `specs/watcher-atomicity-release/tasks.md` ticked *after* the gates; the completion
  lands as one commit in the repository's style on top of `a88d601`.
- **AC8 — The routed deferrals are on the record.** This spec's routing section carries
  the measured rates (247/30,000; 3,952/30,000 ≈ 13 %), the two-statement fix shape, the
  0020 requirement, and the hostile-DATA evidence, so the next intake inherits evidence,
  not folklore.

## Out of scope (deliberate, recorded)

Everything in the routing section above, plus the ratified spec's standing refusals
unchanged and binding: one `entry()` registration path (no get-fast-path), one wait loop
shared by HTTP and MCP, no TTL sweeper, no subscriber-count release machinery, no new
files or dependencies, no `coverage(off)`, `insert` comment-only (already done in the
WIP), the fan-out ordering untouched (0001), `wait_for_covers_the_lagging_watcher`'s
assertion frozen (AC5). The completion does not re-open any settled conflict row.

## Verification quickstart

```bash
cargo check                                      # E0425 gone
cargo test --no-run                              # every integration target links
cargo test --lib store::wait_tests -- --nocapture
cargo test --lib feed::
cargo test --test burst await_survives_a_release -- --nocapture   # real HTTP + SMTP
bash scripts/coverage-gate.sh                    # exact 100/100/100
```
