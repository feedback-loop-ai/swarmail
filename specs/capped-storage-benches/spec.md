# Spec — epic #13 wave 9 (final child), #12: benchmark capped storage, stop over-allocating on counts and limited views

Epic #13 wave 9 · issue
[feedback-loop-ai/swarmail#12](https://github.com/feedback-loop-ai/swarmail/issues/12) · parent
epic [#13](https://github.com/feedback-loop-ai/swarmail#13).

Synthesis of the two contrarian design positions
(`.forge/design/positions/simplicity.md`, `.forge/design/positions/robustness.md` — both
`pass`). Both seats measured the same tree this session and agree on the four mechanical
shapes; they conflict on five points of *method and scope*, ruled below — not averaged.
**No decision document is drafted**: nothing here touches a frozen guarantee, a wire
contract, or a ruling (the constitution check in
[plan.md](plan.md) carries the per-guarantee verdict and the reasoning).

## The problem, as the tree holds it (all measured this run)

Five findings, each anchored and measured (probes run-local, release profile, real store):

1. **Retention shifts on every capped insert.** `src/store.rs:319` —
   `list.drain(..overflow)` on a `Vec<Arc<Email>>`: a front-drain of a Vec is a memmove of
   every remaining element, per insert, at the cap. Isolated primitive: **14–19 µs** per
   capped insert vs **0.005–0.011 µs** for a deque `pop_front` (~3000×). Full path:
   `probe/insert@cap100k-steady` = **13.5 µs/mail** — **~8.5× slower than the published
   1.58 µs** (`README.md:378`), and the entire gap is the shift (781 KiB memmove/insert at
   cap 100 000 ≈ 67 GB/s — textbook memmove bandwidth).
2. **`count` builds a full `Vec<Arc<Email>>` to return a length.** `src/store.rs:390-392`
   delegates to `list()` — one `Arc` clone per match plus one allocation, discarded for a
   `usize`. Measured: **1.12 ms** at the default cap.
3. **The thread view lists the whole inbox before taking 200.** `src/threads.rs:246-253`
   clones every `Arc` in the inbox, *then* `.take(VIEW_CAP)` (200). Measured:
   **1.20 ms** per view — and `src/feed.rs` renders a full view **per accepted mail, per
   SSE subscriber**, so at cap the UI watching a test starves the test's 250s.
4. **REST listing deep-clones whole `Email`s, raw bytes included, then drops them.**
   `src/api.rs:262-267` maps `(*e).clone()` over the matches; `raw` is `#[serde(skip)]`
   (`src/model.rs:52-53`), so every cloned byte is provably dead work — at the 50 MiB wire
   cap a default page transiently allocates up to ~5 GiB that serialization never reads.
5. **The published ingest bench measures the wrong configuration.**
   `benches/ingest.rs:39` uses `Store::new(0)` — unlimited — so the README's 1.58 µs/mail
   says nothing about steady-state cap eviction, *and* the store grows without bound
   during a sample (millions of mails ≈ hundreds of MB of RSS per 5 s criterion sample,
   which both destabilizes the number and OOMs small runners).

All three query paths hold the `DashMap` shard read guard while over-allocating;
`insert` needs the write guard on the same shard. At the default cap (`src/config.rs:46`,
`default_value_t = 100_000` — the default *is* the hit case), every `/count` poll, every
`/assert`, and every feed frame costs the accept path up to ~1.2 ms of writer starvation.
The over-allocation is an accept-latency lever aimed at decision 0001, not a perf nit.

## The synthesis — adopted, refused, and ruled

**Adopted (both seats agree, measurement justifies each):**

- **(a) Steady-state capped benchmarks** in the existing `ingest` bench (`Cargo.toml:68-70`
  unchanged — one target, one group, new ids): pre-fill `Store::new(100_000)` to exactly
  the cap **outside the timed region**, then measure inserts that each trigger exactly one
  eviction — no unbounded growth during a sample. Plus capped view and count benches.
  The uncapped `ingest/store/insert` stays **unchanged as the control**: the delta
  uncapped ↔ capped *is* the measured eviction cost that decision 0003 cites.
- **(b) Retention eviction without the O(n) shift**: the per-inbox container becomes
  `std::collections::VecDeque<Arc<Email>>`; `push_back` + a general `pop_front` loop under
  the *same* shard write guard. No hand-rolled ring, no `unsafe`, no head index — std's
  deque *is* the framing's "ring/deque index". The loop stays **general** (not a single
  `pop_front`): restore does not enforce the cap, so the first post-restart insert may owe
  `overflow > 1` — today's `drain(..overflow)` trims the excess honestly and the deque
  loop must keep doing exactly that (silently pruning on restore would delete mail a prior
  process answered 250 for — a 0001 violation by another route).
- **(b) De-cloned `count`**: same signature, same exact integer (decision 0010's exactness
  untouched), zero allocation — scan the inbox list and count matches. The scan itself is
  inherent: a count must inspect *matching* mail; only the clones and the allocation die.
- **(b) Bounded thread view**: the view bounds **before** listing via one new ~5-line
  store method, `list_bounded(inbox, filter, cap)` — newest-first, filter applied **before**
  the bound, so the result is exactly today's `list(...).take(VIEW_CAP)` window walked only
  as far as the bound needs. Wire-identical for the REST threads surface and the SSE feed.
- **(c) REST listing stops cloning raw bytes**: `EmailList.emails` becomes
  `Vec<Arc<Email>>` and the `(*e).clone()` map is deleted. `Arc<Email>` serializes exactly
  as `Email` under the `rc` feature the crate enables *for exactly this purpose*
  (`Cargo.toml:39-41` says so verbatim) — the wire is byte-identical, proven by pin test.

**Refused (both seats, ratified):** no maintained per-inbox counters or count caches (the
first bug is a lying count — the exact failure 0010's exact-count asserts exist to catch);
no ASCII-only case folding in the filter (changes which mails a `subject=` filter matches
for non-ASCII senders — a behavior change under the byte-compat search contracts; hoist
the needle, never the haystack, and only with a measured need); no byte cap as a drive-by
(a contract change to 0003 that belongs to the operator); no `EmailSummary` DTO; no new
dependency; no second bench binary, no bench reorganization, no knobs; no `coverage(off)`;
no scope creep into `wait_for`, `list_all`/the Mailpit shim, or `AwaitResponse`'s
per-email clone (bounded by `want`, default 1) — the framing's list is closed.

**Ruled conflicts (the losing side's cost is named):**

1. **Steady state: asserted, not assumed** — *ruled for robustness*. Prefill outside the
   timed region gives the shape; the bench additionally asserts, **after** sampling, that
   the sampled inbox length is exactly the cap and that the `emails_dropped` delta equals
   the `emails_inserted` delta — panicking otherwise. Both counters are read *outside* the
   timed region, so the canary costs the measurement nothing; a rewrite that ever evicts
   outside the guard fails the bench instead of a customer (the burst-atomicity canary of
   robustness §2.3). Simplicity's "by construction" is kept as the *shape*; robustness's
   "cannot lie" is kept as the *check*. Cost if we are wrong: ~10 lines of bench code.
2. **No O(1) fast path for the trivial filter** — *ruled for simplicity*. The probe
   measured `count` with `Filter::default()` (the `/await`, `/assert` and burst shape):
   post-de-clone that scan is tens of µs, allocation-free — against a 5 s await timeout
   and fleet poll rates, noise. The fast path's failure mode is a **lying count**: if
   `Filter` grows a field and the triviality check is not updated, `count` returns `len()`
   for a non-trivial filter — silently and wrongly, the exact hazard 0010 polices.
   *Cost of losing*: `count` stays O(inbox) — tens of µs instead of ~0; the starvation
   lever still shrinks ~40×, and the residual is published by the committed bench. If the
   number ever matters, a later change lands the fast path as `*filter ==
   Filter::default()` (a `PartialEq` against default, which added fields cannot silently
   break) — with its measured justification attached.
3. **Needle hoisting (per-query lowered filter needles) is deferred, with its evidence
   base committed** — *ruled against robustness's drive-by, for its measurement*. The
   probe measured only the trivial-filter regime; the filtered regime (where
   `Filter::matches` lowers the needle `to.to_lowercase()` **per email**, `src/store.rs:46,
   57, 71`) was never measured. The wave therefore commits one more bench id —
   `count@cap` with a **non-matching needle filter**, the pure-waste case — and the hoist
   itself stays out of scope: it is not in the framing's fix list, and it reshapes the
   public `Filter` surface without a measured need. *Cost of losing*: filtered counts at
   cap keep paying one needle allocation per mail (hundreds of µs at 100k) until a
   follow-up lands the hoist with this bench's number in hand; the PR body reports the
   residual honestly.
4. **No persisted-variant bench; label instead** — *ruled for simplicity, adopting
   robustness's own fallback*. A steady-state bench over a temp-file `Store::open` needs a
   100k-commit prefill (minutes of machine-dependent fsync in CI) and measures the SQLite
   commit, not the eviction this wave is about. The committed bench's doc comment and the
   README row label the memory-only regime **"no `--data-file`"**. *Cost of losing*: the
   headline number describes the memory store; the label says so, and 0001's persisted-250
   cost remains documented behavior rather than a bench claim.
5. **No body-size-regime bench for fix (c)** — *ruled out*. Fix (c) needs no measurement:
   `raw` is `#[serde(skip)]`, so the clone is dead work *by construction*, and its proof is
   the serialized-bytes pin test plus the existing HTTP tests. A ~1 MiB-body page bench is
   the evidence base for a body-size-regime wave, not for a construction argument.
   *Cost of losing*: fix (c)'s PR-body entry carries the construction argument and the
   wire-identity proof instead of a µs figure.
6. **`list_bounded` takes a filter** — *ruled for simplicity's shape over robustness's
   unfiltered-only `list_newest`*. The feared misuse — bounding a filtered query *before*
   filtering — is structurally impossible in this shape: the filter is applied in the
   iterator chain *before* `take`, so the method returns the newest `cap` **matches** for
   any filter, which is exactly today's `list(...).take(n)` semantics. The doc comment
   pins the order and a unit test pins the boundary case. *Cost of losing*: none — no
   caller needs an unfiltered-only shape today, and the general one cannot be misused.
7. **`EmailList` serializes Arcs rather than gaining a clone-without-raw helper** — *ruled
   for simplicity*. The helper *adds* a partial-clone code path (new lines under the exact
   100/100/100 gate, and a "clone that isn't a clone" trap); the Arc form *deletes* work
   with a pattern the house already endorses in writing. *Cost of losing*: the public Rust
   type of `EmailList.emails` changes (`Vec<Email>` → `Vec<Arc<Email>>`) — wire
   byte-identical (frozen contracts are behavioral, not type-theoretical), proven by the
   pin test and the existing HTTP tests; the crate's public shape already exposes
   `Arc<Email>` via `threads::Thread`.

## What this wave does not touch

`wait_for`'s list-then-truncate (its `Timeout { matched }` contract needs the full count —
both seats defer it; it keeps the 0002 law), `list_all` and the Mailpit shim
(byte-compatible surface, not named by the framing), `AwaitResponse`'s per-email clone,
`get_message`/`seed_inbox` single-mail clones (named in the PR body as follow-up), the
persist path, the webhook dispatcher, chaos. `Filter`'s matching semantics are untouched —
Unicode folding stays exactly as it is.

## Acceptance Criteria

- **AC1 — Steady-state capped ingest bench.** `cargo bench` publishes, in the existing
  `ingest` group, a bench that pre-fills a `Store::new(100_000)` to exactly the cap
  outside the timed region and measures inserts that each trigger exactly one eviction.
  After sampling it asserts the sampled inbox length is exactly 100 000 **and** the
  `emails_dropped` delta equals the `emails_inserted` delta; a violation panics with a
  message naming the drift. The cap is a hardcoded `100_000` whose comment ties it to
  `src/config.rs:46` (pinned by `tests/config_cli.rs:58`, so the two cannot drift
  silently). The bench target remains the single `ingest` binary; no `probe` group ships.
- **AC2 — Control retained.** The uncapped `ingest/store/insert` bench is unchanged
  (still `Store::new(0)`), as the control against which the capped delta reads; its doc
  comment notes the store grows during its (short) sampling window.
- **AC3 — Capped query benches.** The group also publishes the capped thread view
  (`threads::view` at the prefilled cap) and the capped count path twice: with the default
  filter (the `/await`, `/assert` and burst shape) and with a non-matching needle filter
  (the pure-waste regime where per-email needle lowering lives).
- **AC4 — `count` is clone-free and exact.** `Store::count` keeps its signature and
  returns the same integer for every store state and filter (decision 0010 exactness)
  without cloning any `Arc` and without materializing a `Vec`. Proven by the existing
  count tests (both `Some`/`None` arms) plus a new unit test asserting
  `count(inbox, non-trivial filter) == list(inbox, non-trivial filter).len()` on a seeded
  inbox.
- **AC5 — The view bounds before listing.** `threads::view` covers exactly the newest
  `VIEW_CAP` mails with the oldest outside the window (existing tests green), via a new
  store method that applies the filter before the bound; a unit test pins the
  filter-then-take boundary (the newest `cap` *matches*, not the newest mails filtered
  down), the cap-respected newest-kept case, and the missing-inbox → empty case. The
  threads REST surface and the SSE feed are wire-identical.
- **AC6 — Eviction is shift-free and law-identical.** The per-inbox container is a
  `VecDeque<Arc<Email>>`; eviction is a general `pop_front` loop under the same shard
  write guard as the push, so the cap is never observed exceeded and `emails_dropped`
  counts every eviction exactly; eviction removes only front (oldest) entries and never a
  delivery in flight. All existing cap/burst/persistence tests stay green, plus a new pin
  for the restore-over-cap catch-up: a data file written under a higher cap, reopened
  under a lower one, trimmed exactly to the lower cap by the first insert with
  `emails_dropped` counting the excess.
- **AC7 — REST listing clones no raw bytes.** `EmailList.emails: Vec<Arc<Email>>`, the
  `(*e).clone()` map deleted, `total` computed from the same match set; a unit pin test
  asserts the serialized page is byte-identical to the `Vec<Email>` shape and contains no
  `raw` key; all existing HTTP tests stay green.
- **AC8 — Gates and floor.** `cargo fmt --all -- --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`, `brokkr compile --bundle .`,
  `scripts/coverage-gate.sh` (exact 100/100/100 — every new production line ships with the
  test that executes it; no `coverage(off)`) and `cargo deny check licenses` all pass.
  `Cargo.toml` gains no dependency (`VecDeque` is std; criterion is already the harness).
- **AC9 — The published number is honest.** The README benchmark table carries the
  steady-state capped ingest figure (criterion, this machine) beside the retained uncapped
  figure, labeled with its regime — memory-only, "no `--data-file`" — and the PR body
  carries the full before/after criterion table, including the corrected row that
  production-default ingest was published at 1.58 µs while actually costing ~13.5 µs at
  cap. If any fix shows no measurable win on the committed bench, it is skipped *with the
  number reported* (none is expected: the margins are 8×–3000×).
- **AC10 — Contracts unchanged.** Counts, listing and thread shapes stay byte-compatible;
  the Mailpit shim is untouched; guarantees 0001 (accept == stored, persist before 250),
  0002 (subscribe before check) and 0003 (oldest-only, never mid-delivery) hold with no
  new decision document — the wave changes 0003's cost, never its law.
