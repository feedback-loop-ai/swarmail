# Plan — epic #13 wave 3 completion: the watcher release lands, and the suite can prove it

Companion to [`spec.md`](spec.md) (WHAT/WHY). How the parked WIP `fa93a61` (under `a88d601`)
is completed: the exact edit, the files it touches, and the risks with mitigations.

## Technical context

The tree is clean at `a88d601` on `feat/epic13-03-14-watcher-atomicity`; the WIP is its
parent `fa93a61`, so every anchor below is read from the working tree.

- `src/store.rs` — the only file with a behaviour change.
  - `:24-30` — the probe statics: `#[cfg(test)] static CLOSED_PROBE: AtomicU64` and the
    `#[cfg(test)] thread_local! PROBE_THREAD`. The counter stays (gated, instrumented by
    the sleeps test); the thread-local goes.
  - `:536-543` — the `Closed` arm of `wait_for`'s recv match: the `MUTATION B` comment,
    the **ungated** `CLOSED_PROBE.fetch_add` (`:540` — the `E0425`), and the misfiring
    `run_post_subscribe_gate()` with no subscribe.
  - `:884, 897-903` — the sleeps test's `PROBE_THREAD.with(...)` line and the
    `eprintln!("PROBE advanced=… closed_arm_hits=…")` diagnostic block: delete.
  - Already correct from the WIP and **not touched**: `WATCHER_CAP = 16` + the atomic
    `subscribe` (`:474-489`), `drain_inbox`'s unconditional `watchers.remove`
    (`:104-119`), `delete`'s empties-release (`:410-412`), the `clear_all` sweep
    (`:444-450`), the seam field/setter/runner (`:153-159, 275-306`), the race/release/
    sweep tests (`:645-746`), the two gate-driven wait tests (`:825-906`).
- `src/feed.rs` — verify-only. The corrected `Closed`-arm comment and
  `a_cleared_inbox_ends_the_feed_after_the_snapshot` are already in the WIP; no edit
  expected.
- `tests/burst.rs` — verify-only. `await_survives_a_release_and_finds_later_mail` is
  already written to the file's own helpers (`start`, `http_json`, `smtp_send` from
  `common`, the `408`/`matched: 0` convention of `await_times_out_cleanly`); it cannot
  link only because the lib does not compile. No edit expected.
- `specs/watcher-atomicity-release/tasks.md` — the ten checkboxes are ticked **after**
  the gates prove them.
- Toolchain facts the implementer will hit: `commit.gpgsign` resolves **false** in this
  worktree (brokkr's config overlay) and every recent commit is unsigned — commit like
  the rest of the history; the implement seat's budget is 7,200 s (bundle.json); the
  coverage measurement is `cargo +nightly llvm-cov` and `#[cfg(test)]` items in `src/`
  are counted in the denominator (which is why the probe must stay self-covering).

## Constitution check — frozen guarantees

| Guarantee | Verdict |
|---|---|
| 0001 accept == stored | Untouched. `insert` is not edited; the fan-out ordering stays after the map write and before the 250. The routed 0001 hole (insert-races-clear stranding) is *pre-existing* and recorded in spec.md's routing section — not widened by this completion. |
| 0002 await subscribes before it checks | **Strengthened to its letter.** The `Closed` arm now re-applies the ordering per iteration (subscribe → scan); the code catches up to the `insert` doc comment, which already promises this. |
| 0003 pruning is oldest-only | Untouched. |
| 0004 / 0005 | Untouched. |
| Coverage floor (exact 100/100/100) | The denominator **shrinks** (`PROBE_THREAD`, the `eprintln!`, the misfiring call site die); the kept probe lines and the restored `subscribe` line are executed by the two existing gate-driven tests. No `coverage(off)` anywhere. |
| Public contract | No new delta. The deltas already recorded in proposed 0010 (SSE ends on drain; wait path re-subscribes) become *real* instead of *documented*. |

## Project structure — the complete diff surface

```
src/store.rs            one restored arm + probe discipline + one delta assertion
specs/watcher-atomicity-release/tasks.md   ten checkbox ticks
specs/watcher-completion/{spec,plan,tasks}.md   this design (committed by the chief)
```

No new files, no new test files, no new dependencies, no `deny.toml` movement, no
`coverage-gate.sh` movement. `src/feed.rs` and `tests/burst.rs` are verify-only: if any
gate exposes a defect in them, fix exactly what is exposed and nothing else — otherwise
they are committed untouched.

## Implementation steps

### 1. `src/store.rs` — restore the `Closed` arm (the one behaviour line)

Replace the arm's body (`:538-543`) with:

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

The `MUTATION B` comment dies here. `rx` is already `let mut`; `RecvError` is imported.
`#[cfg(test)]` on a statement is the file's established pattern (the gate line above
already uses it) — the probe write must be gated exactly like the gate call so the
non-test build never sees it.

### 2. `src/store.rs` — probe discipline

- Delete the `PROBE_THREAD` `thread_local!` block (`:27-30`) and its two uses
  (`:884`, `:901`).
- Delete the `eprintln!` diagnostic block (`:897-904`).
- Keep `static CLOSED_PROBE` (`:24-25`) — it stays `#[cfg(test)]`; `AtomicU64`/`Ordering`
  remain imported (`Totals` uses them in every build).

### 3. `src/store.rs` — the deterministic delta bound

In `a_waiter_on_a_released_channel_sleeps_until_its_deadline`, snapshot before the wait
and assert after:

```rust
let hits_before = CLOSED_PROBE.load(Ordering::Relaxed);
let out = store
    .wait_for("box", &Filter::default(), 1, Duration::from_millis(200))
    .await;
// … existing Timeout/Found match and the ticker assert, unchanged …
let hits = CLOSED_PROBE.load(Ordering::Relaxed) - hits_before;
assert!(
    hits <= 4,
    "a single release must be answered with one Closed arm hit, not {hits}"
);
```

Delta (not absolute) because the counter is process-global and the lib harness runs tests
in parallel; ≤ 4 is loose against a sibling test's one hit and red at 2×10⁵. The ticker
assert (`advanced >= 10`) and the `Timeout { matched: 0 }` assert are **not** re-tuned.

### 4. The six gates, in `specs/watcher-atomicity-release/tasks.md`'s order

`cargo fmt --all -- --check` → `cargo clippy --all-targets -- -D warnings` →
`cargo test` (real servers, no mocks — the burst suite links for the first time) →
`brokkr compile --bundle .` → `bash scripts/coverage-gate.sh` (exact 100/100/100) →
`cargo deny check licenses`. Fix **only what the gates expose**; anything else that
surfaces goes back through design, not a silent edit.

### 5. Close the record, then commit

Tick the ten checkboxes in `specs/watcher-atomicity-release/tasks.md` (their proofs are
the green gates). One commit on top of `HEAD`, repository style, imperative mood, a body
that says why — suggested:

> **watchers: complete the release — the Closed arm re-subscribes**
>
> The parked attempt carried a live mutation in wait_for's Closed arm: the
> post-subscribe gate fired without the re-subscribe, so a released waiter
> burned its core on the pre-fix spin (202k–276k arm hits per 200 ms), and
> the ungated probe static broke every non-test build with E0425. Restore
> 0002's per-iteration ordering — subscribe, then scan — and turn the
> probe counter into the deterministic teeth the measured ticker cannot
> be: one Closed-arm hit per release, not a hundred thousand.

`src/feed.rs` and `tests/burst.rs` are expected to be absent from the diff (verify-only);
if a gate forced an edit there, the commit body says so in one sentence.

## Risks

| Risk | Mitigation |
|---|---|
| **The kept probe re-enters the 100 % denominator and rots uncovered.** | Two lines: the `fetch_add` (executed by both gate-driven tests — the arm is hit once in each) and the `load` (executed by the sleeps test). If either goes uncovered, the exact gate says so loudly — an uncovered seam is the signal its exemplar rotted, the gate doing its job. |
| **The delta bound flakes.** | It cannot in the shipped schedule: the gate orders exactly one release, so the post-fix delta is exactly 1. The bound is a delta, immune to a parallel test's hits; and it has 4× headroom over 1 but fails at 2×10⁵ — no tuning knob exists between. If a future test legitimately releases a channel twice within one wait, the bound is *supposed* to count it (≤ 4 still holds). |
| **The restored arm breaks `wait_for_covers_the_lagging_watcher` or the timeout tests.** | They never take the `Closed` arm (no release happens in them); the `Lagged` and elapsed arms are byte-identical to the WIP. Only release-path tests change behaviour — in the direction their assertions already demand. |
| **`not_stranded` turns out to pass under a *new* misfire** (someone fires the gate without a subscribe again). | The seam's contract is one reviewed line — fire only after `subscribe()` returns — and 0002's letter makes reviewers check the subscribe-before-scan ordering explicitly. The delta bound additionally caps the arm at one hit per release: a misfiring *spin* is still red. What no test can catch is a misfire that still inserts the mail once — that is review's job, and it is the protection the realm chose. |
| **The ticker canary flakes on a loaded runner** (its recorded honest loss). | Unchanged margin (≥ 10 of ~40), gate-ordered setup, and the recorded fallback: drop to the outcome-level `not_stranded` + the delta bound, which now carry the property deterministically. The fallback's cost is lower than at ratification time because the teeth no longer depend on the ticker. |
| **The burst AC6 test is flaky against a real server.** | It is gate-ordered and honest: the first await's `408`/`matched: 0` is asserted at a 100 ms timeout, the clear is a synchronous API call, and the final await has 5 s against a 1.58 µs ingest path. If it still flakes, that is a finding about the server, not about the test's shape — fix what is exposed, in this surface. |
| **A gate exposes an unexpected failure** (fmt on the arm, clippy on the assert message, coverage moving off exactly 100). | Fix only the exposure; do not re-derive settled tasks, add files, or touch feed.rs/burst.rs beyond the exposure. Anything that needs a *design* answer (a new seam, a signature change, a guarantee interaction) returns through design per the ratified spec's task-10 rule. |
| **The routed 0001 hole is mistaken for "fixed by this wave".** | spec.md's routing section and AC8 record the measured rates and the 0011 requirement; the completion's commit body does not claim it. Review checks the claim, not the wish. |
| **GPG**: the house rules describe signed commits, but this worktree's effective `commit.gpgsign` is **false** (brokkr's config overlay) and all eight recent commits are unsigned. | Commit like the rest of the history; do not flip config the operator set. If the operator re-signs history later, this commit is in the re-sign set like every other. |

## Quickstart (reviewer)

```bash
cargo check && cargo test --no-run               # AC2: builds everywhere
cargo test --lib store::wait_tests -- --nocapture # AC1/AC3/AC4: redness mechanism + teeth + canary
cargo test --lib feed::                           # AC5: the feed ends on drain
cargo test --test burst await_survives_a_release -- --nocapture  # AC6: real protocol
bash scripts/coverage-gate.sh && cargo deny check licenses && brokkr compile --bundle .
git log -1 --stat                                 # AC7: one commit, three-file surface + checkbox ticks
```
