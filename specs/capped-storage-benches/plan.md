# Plan — epic #13 wave 9 (#12): benchmark capped storage, stop over-allocating on counts and limited views

Companion to [`spec.md`](spec.md) (WHAT/WHY). How the four mechanical shapes land in the
four files the wave already owns, what the benches must promise before their numbers are
published, and the risks with mitigations.

## Technical context

Tree clean at `26d491c` on `feat/epic13-12-benchmarks`; every anchor read from the working
tree this session.

- `src/store.rs` — the container and the count body.
  - `:313-321` — `insert`'s prune block: `or_default()` → `push` → `drain(..overflow)` on a
    `Vec<Arc<Email>>`. The push becomes `push_back`, the drain becomes a general
    `pop_front` loop; the index removals, the `emails_dropped` fetch_add and the persist
    mirror stay inside the same shard write guard, byte-for-byte in *order*.
  - `:358-368` — `list` (`iter().rev().filter().cloned()`): **unchanged code** —
    `iter().rev()` on a deque with front = oldest yields the same newest-first stream.
  - `:390-392` — `count` delegates to `list().len()`; rewritten to a clone-free scan with
    the same signature (`match self.inboxes.get(inbox) { Some(list) =>
    list.iter().filter(|e| filter.matches(e)).count(), None => 0 }`).
  - `:102-119` — `drain_inbox`'s signature (`&DashMap<String, Vec<Arc<Email>>>`) widens to
    `VecDeque`; its body (`remove`, iterate) already works on a deque.
  - `:199-215` — `Store::open` restore: `.push(...)` becomes `.push_back(...)` — insertion
    order, front = oldest, so restart order is unchanged and restore still does **not**
    prune (0003 puts eviction in `insert`; the general loop makes the first post-restart
    insert pay the catch-up honestly).
  - `:377-385, 401-404, 438-448, 465-468` — `list_all` (`Extend`, `sort_by`), `delete`
    (`retain`, stable on `VecDeque` since 1.61; toolchain is 1.98), `clear`/`clear_all`
    (via `drain_inbox`), `inboxes()` (`len`): all work on a deque unchanged.
  - New method beside `list` (~5 lines + doc): `pub fn list_bounded(&self, inbox: &str,
    filter: &Filter, cap: usize) -> Vec<Arc<Email>>` — `iter().rev().filter(..).take(cap)
    .cloned().collect()`; the doc comment states the load-bearing order (filter **before**
    bound: the newest `cap` *matches*).
  - `Filter` (`:38-84`) and `wait_for` (`:511-554`) are **not edited** (rulings 2 and the
    closed list).
- `src/threads.rs` — `view` (`:246-253`) calls `list_bounded(inbox, &Filter::default(),
  VIEW_CAP)` instead of `list(...).take(VIEW_CAP)`; `group_threads` and everything
  downstream are type-identical. `VIEW_CAP = 200` (`:25`) stays a constant.
- `src/api.rs` — `EmailList.emails: Vec<Email>` → `Vec<Arc<Email>>` (`:175-179`);
  `list_messages` (`:253-269`) deletes the `.map(|e| (*e).clone())`, takes `total` off the
  same match Vec before the move, and collects the page of `Arc`s. `AwaitResponse`
  (`:181-186`) is untouched. `EmailList` is `Serialize`-only and referenced nowhere
  outside `api.rs`.
- `benches/ingest.rs` — the single `ingest` target (`Cargo.toml:68-70`, `harness = false`)
  gains: a `prefill(cap)` helper; `store/insert@cap-steady` (cap `100_000`, comment tied
  to `src/config.rs:46`, prefill outside `b.iter`, timed inserts from `i = cap` onward,
  post-sample steady-state asserts); `threads/view@cap`; `store/count@cap` (default
  filter); `store/count@cap-filtered` (a needle filter matching nothing — the pure-waste
  regime). `Throughput::Elements(1)` throughout; the uncapped `store/insert` and
  `build_email/parse+extract` benches stay untouched. The run-local `probe` group that
  appeared during the panel session is **not committed** (verified absent from the tree;
  `.forge/tmp/` holds the run-local copies, gitignored).
- `README.md` — the benchmark table (`:374-380`): the `+ store insert` row is replaced by
  the steady-state capped figure, labeled memory-only ("no `--data-file`"), with the
  uncapped figure retained as the control row.
- Tests: `src/store.rs`'s existing `#[cfg(test)]` modules (the cap lives only at unit
  level — `tests/common/mod.rs:20` sets `max_per_inbox: 0`, per 0003's exact-counts
  consequence, so the integration suites run uncapped), `src/api.rs` (the serialization
  pin), plus verify-only runs of `tests/burst.rs`, `tests/p2.rs`, `tests/rate.rs`,
  `tests/api.rs`, `tests/persistence.rs`.
- Toolchain facts: `commit.gpgsign` resolves **false** in this worktree (brokkr's config
  overlay) and every recent commit is unsigned — commit like the rest of the history; the
  coverage measurement is `cargo +nightly llvm-cov --workspace --branch` and inline
  `#[cfg(test)]` modules in `src/` are counted in the denominator (benches are not part of
  the production report); `GIT_CONFIG_COUNT=0` sidesteps the harness's broken
  `GIT_CONFIG_KEY_0` for any in-session git use.

## Constitution check — frozen guarantees

| Guarantee | Verdict |
|---|---|
| 0001 accept == stored | **Untouched in law.** `insert` stays synchronous: push + prune under the same shard write guard, `record_insert` still committed before any fan-out and before the 250; the deque introduces no async gap. The bench measures the real path and must not be used to argue persist off the 250 path. Restore still never prunes. |
| 0002 await subscribes before it checks | **Untouched** — `wait_for` is not edited (deferred by ruling; both seats rank it first to defer). |
| 0003 pruning is oldest-only | **Preserved mechanically.** A deque can only evict from the front, and the front is the oldest: there is no code path that touches a newer mail, so "never a delivery in flight" becomes a property of the shape. Overflow > 1 catch-up preserved; `emails_dropped` stays exact; cap-vs-burst atomicity kept (push + prune under one guard) and now asserted by the bench's steady-state canary. Cost changes; the law does not. |
| 0005 extraction at ingest | Untouched. |
| 0010 exact counts | **Preserved.** The de-cloned `count` answers the same integer for the same store state; no cache, no maintained counter, no approximation. The refused O(1) fast path would have introduced the lying-count hazard; refused. |
| Coverage floor (exact 100/100/100) | Every new production line ships with the test that executes it (see tasks); touched branches keep their settlement tests; no `coverage(off)` anywhere; benches live outside `src/` and are not in the production report. |
| Public contract | No wire delta anywhere: the threads view, the SSE feed, and the listing page serialize to the same bytes (pin tests + existing HTTP suites). `EmailList.emails` changes public *Rust* type only (`Vec<Email>` → `Vec<Arc<Email>>`) — frozen contracts are behavioral, not type-theoretical, and `serde(rc)` is the house's own written pattern for exactly this. |
| Dependency policy | No new dependency: `std::collections::VecDeque`, `criterion` already in the tree. `deny.toml` untouched. |

**No decision document.** The wave touches no frozen guarantee, no wire contract and no
ruling: 0001's law, 0002's law and 0003's law are all in force exactly as written; the
deques, the scan and the bound change their *cost*, not their *meaning*. A spec that
drafted a decision here would manufacture paper for a mechanical change — the simplicity
seat's sentence, ratified: the constitution check above is where the chief says so. The
next free decision number in this tree is **0011** (note for the operator, not for this
wave: 0010 is doubled — `exact-count-asserts-and-the-shared-seed` and
`watcher-channel-lifetime`, both `proposed` — and renumbering one of them remains an
acceptance-time task).

## Project structure

No new files, no new modules, no new DTOs, no knobs. Four existing files absorb the wave
(`src/store.rs`, `src/threads.rs`, `src/api.rs`, `benches/ingest.rs`) plus the one-line
README row. The new vocabulary is exactly one method (`list_bounded`, ~5 lines, documented)
and four bench ids; everything else is a type-level diff or a deleted clone.

## The measurement contract (what a number must promise before it is published)

1. **The shipped configuration, not the flattering one** — the capped benches run at the
   default cap; the uncapped bench is retained and labeled as the control.
2. **Steady state is asserted, not assumed** — post-sample: sampled inbox length == cap,
   and `emails_dropped` delta == `emails_inserted` delta, read from `Totals`/`inboxes()`
   *outside* the timed region so the canary cannot perturb the measurement. A broken
   eviction panics the bench instead of publishing drift as throughput.
3. **The regime is named in the result** — ids carry `@cap`; the README row and the bench
   doc comment say memory-only, "no `--data-file`"; the filtered count bench names the
   needle regime.
4. **Stable names** — all ids live in the existing `ingest` group so criterion baselines
   stay name-keyed; the panel's run-local `probe` group never ships.
5. **Before/after on the committed bench** — the implementer runs the committed bench
   *before* the fixes (the `before` column is the probe table reproduced: ~13.5 µs insert,
   ~1.20 ms view, ~1.12 ms count at cap) and *after* (expected: insert returns to ≈ the
   uncapped 1.6 µs; view and count to single-digit/tens of µs). Numbers go in the PR body;
   the escape hatch stands — a fix with no measurable win is skipped *with its number
   reported*.

## Quickstart

```bash
cargo bench                                    # the committed benches; before/after tables
cargo test                                     # unit + integration suites (real servers)
cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings
brokkr compile --bundle .
bash scripts/coverage-gate.sh                  # exact 100/100/100
cargo deny check licenses
```

## Risks

- **The deque swap touches the hottest, most constitutionally loaded function in the
  crate.** *Mitigation:* the diff is type-level; the behavior tests already pin the
  observable contract (`insert_prunes_oldest_beyond_cap` at cap 2,
  `cap_pruning_is_mirrored_to_the_data_file`, `totals_counters_are_readable`,
  `tests/burst.rs`'s exact counts under churn); the one genuinely new branch shape
  (restore-over-cap catch-up) gets its own pin; and the bench's steady-state assert is the
  standing canary — if a future edit ever evicts outside the guard, the bench fails, not a
  customer. If the committed bench disagrees with the probe's prediction (13.5 → ~1.6–2.2
  µs), the number wins and the swap is reported on honestly.
- **`EmailList.emails` changes a public Rust type.** *Mitigation:* the wire is
  byte-identical — `Arc<Email>` serializes exactly as `Email` under the `rc` feature the
  crate enables for exactly this purpose — proven by a new serialization pin test and the
  existing HTTP suites; the crate's public shape already exposes `Arc<Email>` via
  `threads::Thread`. Reviewed as the type-level diff it is.
- **The exact 100/100/100 gate.** *Mitigation:* each new production line is enumerated in
  tasks.md with the test that executes it — `list_bounded`'s three unit tests, the count
  equivalence pin, the restore-over-cap pin, the serialization pin; touched branches keep
  their existing settlement tests. Benches are excluded from the production report by
  construction (not under `src/`), so the denominator only grows by tested lines.
- **The bench asserts could mislead if read as behaviour tests.** *Mitigation:* the
  steady-state check is a canary, documented as such in the bench: it proves the
  measurement's honesty (eviction kept pace; no growth during a sample), not the fix. It
  passes on the pre-fix tree too — pre-fix eviction is slow, not broken — which is exactly
  what makes the `before` column trustworthy.
- **The published number gets worse before it gets honest** (~2 µs expected at cap, not
  1.58). *Mitigation:* that is the point of the wave — the README publishes the number the
  default deployment actually experiences, with the regime label, and the PR body carries
  the full correction history including the 1.58-vs-13.5 row.
- **Bench prefill cost** (~0.2–1.5 s per capped bench in release, three prefills).
  *Mitigation:* measured by the probe; negligible against CI; the prefill runs once per
  bench outside the timed region.
- **If a fix shows no measurable win on the committed bench.** *Mitigation:* the framing's
  escape hatch, pre-committed: skip it and report the number. The probe's margins
  (8×–3000×) make this unlikely for all four shapes; honesty is a deliverable either way.
