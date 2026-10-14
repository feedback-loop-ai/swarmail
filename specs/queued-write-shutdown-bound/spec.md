# The queued-write shutdown bound: one policy at the boundary, decisions renumbered by landing order, contract notes at the lines

Issue: #32 (slices 1, 5, 6 · design run `close-issue-32-https-github-com--ce358800`)
Decision: **0024** — [the boundary refuses to start a commit once stopping](../../docs/decisions/0024-shutdown-refusal-at-the-boundary.md) (status *proposed*, awaiting the operator).

Slices 3 and 4 (CI timing + gating) are already delivered on main (PR #33,
3e0a996); slice 2 (shared REST/MCP validation) is a later run and is **not**
taken here. Line references below are from the reviewed commit 66fab48;
`main` has since moved only in `tests/compat.rs`, so `src/` shapes still
hold — everything here locates by shape, not by line.

The 15.386s reproduction is three serialized busy waits wearing one
shutdown: each queued HTTP mutation reaches the commit and burns its own
5s SQLite busy wait against a held `BEGIN IMMEDIATE`, serialized by the
boundary mutex (`src/store.rs` — `mutation` std `Mutex`; `record()` at
`:258` is the single commit chokepoint all four mutations pass through;
`offload` at `:612` runs each whole mutation on the blocking pool), while
axum's graceful shutdown waits for the handlers (`src/api.rs:84-86`) and
`RunningServer::stop()` joins the serve task (`src/lib.rs:51-55`). ADR
0018's sentence — *"at most one commit lands or refuses after the fact"* —
is false for exactly these requests. The release cannot live at an
adapter: a `select!` around `insert_async(...).await` detaches the already-
spawned blocking task, and tokio's runtime drop waits for detached tasks
before `main` returns — the 15s moves from the graceful wait to process
exit, green on `stop()`, red on real SIGINT.

The two positions agreed on more than they disagreed: the budget bound is
one busy timeout, not one per queued request; a started commit runs to
completion; both test legs (library `stop()` and a real SIGINT process
exit) are mandatory; the wire's fixed refusal text survives; and slice 6
is comment-only. They split on **where the release lives**, on the wire
text for shutdown refusals, and on the slice-6 404-delete arm. Ruled
below, with the losing side's price named.

## Rulings

Each ruling names the position that wins, what the losing position costs,
and the test that proves it. Position files:
`.forge/design/positions/{simplicity,robustness}.md`.

**R1 — The release lives where the queue lives: the boundary observes the
stop watch and refuses at the chokepoint** *(simplicity wins the
structure; robustness wins the diagnosis, the oracles and the honesty
sentences)*.

`Store` gains a `OnceLock<watch::Receiver<bool>>` wired once by `run_on`
after the watch is born and before any listener binds; `record()` — the
one function every mutation's commit flows through (`:393`, `:519`,
`:567`, `:587`) — refuses to start a commit once the stop has fired. The
check is a synchronous, lock-free `borrow_and_update()`: no await enters
the boundary (0014 §1), no constructor signature changes (46+ call sites
stay), `persist.rs` ends at zero diff. The budget arithmetic is airtight
because the boundary serializes: at most one mutation holds it when the
stop fires, so the process burns at most **one** busy bound after
shutdown begins, plus microseconds per queued release — inside 0018 §2's
10s budget in every interleaving. New mutations refuse fast; queued
lock-waiters are released without one SQLite call; a commit past the
check is atomic and finishes.

*Cost of the losing position (robustness's grant-before-offload)*: a
cancellable capacity-1 gate acquired in async land before `spawn_blocking`
wraps a **second serialization layer around the boundary** — new lock
choreography (`grant` outer → `mutation` inner) whose release conditions
diverge under cancellation exactly when an aborted session leaves a
detached task holding the mutex, plus eight+ per-surface admission edits
and their tests, at the hottest code in the crate — to buy a budget bound
the chokepoint check already delivers. Its one genuine benefit, queued
mutations parking in async land instead of on blocking-pool threads, is
orthogonal to shutdown (the pressure exists with no stop in sight) and is
recorded in 0024 §6 as the designed next step if it ever matters. The
per-surface admission checks are refused outright: the chokepoint check
*is* the admission gate, reached microseconds after mutex acquisition,
and a post-stop mutation's only cost is one `spawn_blocking` that refuses
immediately — there is no unbounded post-stop arrival stream, because
graceful shutdown refuses new connections and the abort arm kills
sessions.

**R2 — The wire does not fork** *(simplicity wins; robustness's
per-surface `shutting down` refused)*.

A shutdown refusal answers the fixed refusal text (0012 §2) on every
surface — the text means *nothing stored; retry*, which is true for busy
refusals and for stopping refusals alike. `shutting down` stays the
waits' answer (0018 §1). The cause lives in the log (`tracing::info!` at
the boundary) and in 0024 §4. *Cost of the losing side*: mapping the
shutdown refusal to per-surface texts needs an error-inspection arm at
eight surfaces plus their tests, to distinguish causes whose observable
window is nearly empty — after the stop, new connections are refused and
sessions are aborted, so a post-stop mutation arrives only on a
connection that is already dying. If the operator ever wants the
distinction, it is purchasable later as its own decision (it forks the
0012 §2 fixed-text vocabulary by cause).

**R3 — Slice 5: the earliest-*landed* file keeps the number**
*(simplicity's measured chronology wins, verified against the git record
this session)*.

Three files are numbered 0010 and three are numbered 0011. The rule from
the task — earliest-merged keeps its number — is applied to **committer
dates** (when a commit landed on main), because "merged" means landed:
`00fb027` (0010-watcher-channel-lifetime) was *authored* 06:02:12 but
*landed* 16:28:38, the one straddle between the two orders, and it was
not on main until it landed. Keepers:
`0010-exact-count-asserts-and-the-shared-seed` (landed 12:18:48) and
`0011-shim-delete-contract` (landed 16:08:44). Movers, renumbered to the
next free numbers **in landing order**:
`0011-concurrency-tests-and-the-coverage-gate` (16:10:17) → **0020**,
`0010-watcher-channel-lifetime` (16:28:38) → **0021**,
`0011-bounded-data-raw-bytes` (16:32:53) → **0022**,
`0010-off-worker-commits` (23:38:49) → **0023**. The slice-1 decision is
**0024**. Note the surprise the simplicity position flagged: the file
`docs/house-rules.md` calls "decision 0011" is the *mover* — the citation
sweep for it is the widest (house rules, decisions 0012/0014/0015/0016,
the `0011 rule N` comments across `src/`), and that is exactly the
mechanical work this slice asks for. 0018's dates are corrected to
2026-10-13 (the file says 2026-10-15; be50788 landed 2026-10-13 21:07).
Status lines of 0012/0014/0015/0016 — and every other decision — are not
touched. Every citation is repointed **by subject** (watcher-lifetime →
0021, off-worker commits → 0023, exact-count/shared-seed → 0010 stays,
concurrency-tests → 0020, shim-delete → 0011 stays, bounded-data → 0022),
never by blind `sed`: a citation that names a rule (`0011 rule 5`) always
means the concurrency-tests decision, a citation about channel release or
the wait loop means watcher-lifetime, a citation about the blocking pool
or the seed constructor is classified by its sentence.

*Cost of the losing reading* (author-date order): `0010-watcher-channel-
lifetime` would keep 0010 and `0010-exact-count-asserts` would move,
inverting the sweep for a dozen sites over a commit that existed for ten
hours *before it was on main*. The task's word "merged" rules it.

**R4 — Slice 6: documentation over behavior change** *(simplicity wins on
the 404 delete; both positions converge on POP3 QUIT and the fan-out
comment; one extra stale comment joins the sweep)*.

- `Store::delete` commits a write transaction even for an id the store
  never held (the mirror delete is the guard against a lingering mirror
  row). **Document the deliberate cost at the boundary** — one comment
  sentence naming the price (one bounded commit per 404, 0015 §2) and the
  guard it buys. *Cost of the losing arm* (robustness's
  `changes() > 0` skip): the guard's derivation runs through 0016's ghost
  and the post-poison zombie (the disk row of a memory-lost mail is the
  restart truth — deleting it would launder a zombie's memory loss), the
  change edits the commit semantics inside the boundary (0014 §2: *delete
  commits its mirror delete*) and so owes its own decision, and the
  proving test must observe a **non**-commit at the real layer — an
  observation the tree has no instrument for. A micro-optimization with
  that bill is refused; the comment names the price so the choice stays
  visible.
- POP3 `QUIT` on a failed update answers `-ERR` and keeps the session in
  TRANSACTION (`src/pop3.rs`, the Transaction arm): the handler comment
  states the deviation **as deliberate** — RFC 1939's `QUIT` closes the
  session, Mailpit keeps it, and keeping it lets the client retry or
  `RSET` on the same connection. No behavior change.
- `insert`'s fan-out comment says the sends run "outside the boundary
  tail" while the inbox shard's `entry()` write guard is in fact kept
  alive through them (`src/store.rs`, the insert apply phase). The guard
  stays — the sends are non-blocking, so it costs nothing — and the
  comment tells the whole truth: the *commit* is outside the boundary's
  tail, the shard guard is held across the sends *because* they are
  non-blocking, and a hung webhook target can never hold the boundary.
- One ride-along found while grounding: `src/api.rs`'s seed-handler match
  arm still says *"Memory truth stands: the refused mail is still
  queryable"* — the pre-0014 ghost language, dead since 0014 §3 (*"A
  refused insert is invisible, everywhere"*) and contradicted by the arm
  this run's refusal now flows through. The comment is corrected to the
  refusal contract: nothing applied, nothing queryable; during shutdown
  the same fixed text covers the refusal family (0024).

**R5 — Oracles and test placement** *(robustness wins the legs, 0011 rule
1 wins the placement)*.

Both legs are mandatory and both use the real layer: the `stop()` leg in
`tests/lifecycle.rs` (real server, real data file, a second real
connection holding `BEGIN IMMEDIATE` via `common::hold_write_lock`, three
concurrent seeds, `stop()` under them) and the process leg in
`tests/binary.rs` (the real binary, the test's own connection holding the
lock through exit, three concurrent seeds, **real SIGINT** via
`libc::kill`, exit observed in real time) — a returning library method
does not by itself prove blocking tasks have stopped, so the process leg
is the one that rules. Placement is inline for the refusal probe (0011
rule 1: deterministic branches — flag set/unset — no cold path, no
schedule betrayal, measured in the production report); no harness
relocation is warranted because no line here runs only while a bug is
present. Timing asserts are **upper bounds only** (`< STOP_BUDGET`),
sequence-independent, with a sequencing pause (300ms) that is
demonstrably inside the 5s burn — the R11 precedent's discipline, never a
sleep-as-sync.

## Acceptance Criteria

1. **One shutdown policy bounds the backlog.** After shutdown begins, in
   every interleaving, the process burns at most one SQLite busy bound
   (≤ 5s) — never one per queued request. Queued lock-waiting mutations
   are released without a single SQLite call; new mutations refuse fast;
   a commit that has already started runs to completion; the boundary is
   never torn. Proven by the inline refusal probe (flag set → `Err`,
   nothing applied — no inbox minted, no counter moved, no fan-out; flag
   unset → normal), the `stop()` leg (AC2) and the SIGINT leg (AC3).
2. **The `stop()` leg holds.** Real server on a real data file; a second
   real connection holds `BEGIN IMMEDIATE`; three concurrent seed
   requests to different inboxes are in flight; `stop()` completes within
   `STOP_BUDGET` (10s, asserted as an upper bound); all three requests
   answer `503` with the fixed body `{"error":"storage commit failed"}`;
   a fresh read of the data file shows the refused set stored nothing.
3. **The process leg holds — the one that rules.** The real `swarmail
   serve` binary on a disposable data file; the test's own connection
   holds `BEGIN IMMEDIATE` through exit; three concurrent seeds are
   submitted; **real SIGINT** is delivered; process exit is observed
   within the budget (upper bound only, sequence-independent) with exit
   status success; and the restarted copy is exactly the committed set —
   with the lock held throughout, empty (every commit refused) — never a
   partial row.
4. **Commit-before-acceptance and atomicity are preserved, and the
   interrupted-client contract is written down.** `Store::insert` stays
   synchronous (0001); the single ordered mutation boundary and its lock
   order stand unchanged (0014); `src/persist.rs` ends at zero diff
   (0015 §2's bound untouched, tripwire green). 0024 §4 carries the
   verbatim observability sentences: neither-reply-nor-effect, effect-
   without-reply, learn-from-the-restart; the effect may be durable even
   though the client saw no reply — and equally may not; refused-or-
   committed per request, never partial; abandoned work is never claimed
   to have "not happened"; retry is lossless and may duplicate.
5. **The documentation moves with the behavior.** ADR 0018 is amended in
   place with a dated marker (0019's pattern) so its §1 sentence and §2
   mutation mechanics state the new truth; 0018's dates read 2026-10-13;
   the README's lifecycle table and any budget claims, `llms.txt` and
   `openapi.json` agree with the enforced policy. The status lines of
   0012/0014/0015/0016 are untouched.
6. **The numbering collision is resolved, exactly.** Exactly one decision
   file per number; `0010-exact-count-asserts-and-the-shared-seed` and
   `0011-shim-delete-contract` keep their numbers (earliest landed);
   `0011-concurrency-tests-and-the-coverage-gate` → 0020,
   `0010-watcher-channel-lifetime` → 0021, `0011-bounded-data-raw-bytes`
   → 0022, `0010-off-worker-commits` → 0023, each with a one-line
   provenance note; every code comment, doc reference and relative link
   (including the historical `specs/` records) points at exactly one
   decision; `docs/decisions/README.md`'s index matches; nothing else is
   renamed.
7. **Slice 6's notes sit at the lines, changing zero behavior.** The
   404-delete commit cost is documented at the boundary; POP3 `QUIT`'s
   TRANSACTION-retention is documented as the deliberate deviation; the
   fan-out comment tells the whole truth about the shard guard; the stale
   seed-handler "memory truth stands" line states 0014 §3's contract.
   Comment-only edits: the coverage denominator is untouched by them.
8. **The gates are clean at the run's HEAD.** `cargo fmt --all --check`,
   `cargo clippy --all-targets -- -D warnings`,
   `cargo test --workspace --locked`, `brokkr compile --bundle
   bundles/self`, `scripts/coverage-gate.sh` (the floor may rise, never
   fall) and `cargo deny check licenses` all pass; no new dependency, no
   new `src/` file, no bundle edit; the modular monolith and the single
   ordered mutation boundary stand.
