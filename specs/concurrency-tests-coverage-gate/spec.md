# Spec — epic #13 settlement wave: concurrency regression tests and the exact coverage gate

Epic [feedback-loop-ai/swarmail#13](https://github.com/feedback-loop-ai/swarmail#13),
settlement wave — issue [#18](https://github.com/feedback-loop-ai/swarmail/issues/18).
Unblocks issues #2 and #3: their regression tests are **not** written here — this wave
settles the ground they stand on, before they exist.

Decision: 0020 —
[docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md)
(status `proposed` — acceptance is the operator's). The panel's positions:
[`.forge/design/positions/simplicity.md`](../../../.forge/design/positions/simplicity.md) and
[`.forge/design/positions/robustness.md`](../../../.forge/design/positions/robustness.md)
(both `pass`; run-local evidence, gitignored).

## The problem — two instruments the realm refuses to bend, on a collision course

The exact coverage gate counts inline `#[cfg(test)]` modules in `src/` **as production
report**: every branch an inline test carries must be taken on every run, or the literal
`covered == count` verdict refuses. Deterministic interleaving tests need crate-private
seams (`Store::set_pre_mirror_gate`, `#[cfg(test)] pub(crate)` at `src/store.rs:243-244`),
which the issue read as forcing them inline; stress/race regression tests are naturally
conditional ("retry until the window is hit"), and their failure branches go uncovered
once the bug is fixed — so written inline they would red the exact gate on a green-code
day, forever. Meanwhile `tests/` is already outside the report. The settlement must decide
— not hedge — where each kind of test lives, how seams are exposed, and which files the
gate counts, and prove it on one migrated module. The 100/100/100 floor does not move;
`coverage(off)` stays forbidden; only *which files constitute the measured report* is open.

Both seats measured the same load-bearing fact two independent ways (cargo-llvm-cov
0.9.1's source, and an executed migration in this tree): a `src/` file named
`tests.rs`/`<name>_tests.rs` is excluded from the report **by the tool's default ignore
regex**, yet still runs under `cargo test`, still sees `pub(crate)`, and its *execution of
production lines is still counted* — preserved evidence: `target/coverage/lcov.info` holds
exactly 18 `SF:` records, all `src/*.rs`.

## Ruling — how the two positions were reconciled

The positions agree on the mechanism (the name-based exclusion, measured by both) and
diverge on three things; this spec rules rather than averages.

**Adopted from both: the three single answers.** (1) *The placement axis is cold-path
presence, not seam access*: inline `#[cfg(test)]` stays the default and the vacuity
detector's jurisdiction — even for seam-driven deterministic interleavings; a test that
needs a **cold failure path** (a line that runs only while the bug is present or the
schedule betrays — retry budgets, "window never hit" diagnostics) is unwritable inline
(measured: the natural race shape refuses the gate from `src/store.rs` — 4294/4293 lines,
264/263 branches — and passes unchanged from a harness file) and lives in
`src/<module>/<name>_tests.rs`, declared `#[cfg(test)] mod <name>_tests;` beside its
owner; public-API wire-shaped races live in `tests/*.rs` (real servers, no mocks, 0004).
(2) *Seams stay `#[cfg(test)]` on the owning module* — `pub(crate)` where a sibling
harness file needs them; a feature flag is refused (build-matrix doubling, an untested
configuration by construction). (3) *The gate keeps counting the tool's report*: exact
`covered == count` in all three dimensions, floor only at 100, `coverage(off)` refused,
and **no counting stage is added** — the gate refuses, it never re-counts.

**Ruled against simplicity: the gate script gains one structural refusal.** Simplicity
lands a byte-identical script diff and relies on the tool default plus the leak-check.
Refused, by simplicity's own principle: the leak-check fires on harness paths *present* in
the report and is structurally blind to the opposite hole — production code parked in a
harness-named file is excluded by the tool *and* unseen by the leak-check, so 100/100/100
silently lies while `cargo build` succeeds. Before this wave no harness-named file exists
under `src/`; the wave that first legalizes the pattern must first close the hole. The new
check is a refusal (fail-closed, the same species as the script's existing `coverage(off)`
grep), never a reclassification; the four existing hardening blocks land byte-identical.
Cost of the adoption: ~15 lines of refusal logic and one reserved-name convention. Cost of
the declined alternative, named: a silent way to make the verdict lie ships inside the
very commit that proves harness names are invisible.

**Ruled against robustness: the migrated module is `persist_tests`, not a thread-spawning
slice.** Robustness's worked-proof protocol (baseline recorded, delta attributed to the
moved file's exact footprint, arms proven still covered) is adopted verbatim; its
candidate is refused. The migration of `persist_tests`
(`src/store.rs:1077-1237` → `src/store/persist_tests.rs`, declared
`#[cfg(test)] mod persist_tests;` — a pure move, module path `store::persist_tests`
identical, name kept) is the *maximal-signal* proof: the module hosts the **only**
`set_pre_mirror_gate` installers (`:1194`, `:1221`), so after the move the measured
262/262 branches demonstrate the load-bearing asymmetry directly — a harness file kept the
gate arm covered. Migrating thread-spawning tests instead would prove reachability of
nothing #2/#3 need, and its delta was never measured. Cost of the refusal: the moved
module's own 106 lines / 12 functions leave the denominator permanently — accepted and
named (their policing drops from coverage-refused to functional: asserts still fire,
`cargo test` still runs them).

**Reconciled: the vacuity detector survives the move, placement-independently.**
Robustness's fear — moving seam-window tests blinds the detector "exactly where it earns
its keep" — is answered by measurement, not assertion: the dead-seam refusal follows
*execution*, not file location, because harness execution of production lines still feeds
the report (the 262/262 above). What a harness file loses is only the *policing of its own
body* — which is why the settlement pins the direction: determinism first, the harness as
refuge for the cold path, never a landfill for the inconvenient, and no mass migration
(simplicity's cut list, adopted: an inline module moves only when a concrete test demands
it).

**Reconciled: `persist_tests` stays re-homed as the standing reference harness.** The
proof must be reproducible by the verifier, so the migration is not reverted after
proving; the file becomes the home beside the owner where #2's conditional descendants
land (it already owns the pre-mirror-gate window tests and the `db_path` helper). This is
the single deliberate exception to "inline is the default", grandfathered with its cost
named in decision 0020 — a future reader must be able to see it was chosen, not drift.

**Adopted from robustness, into house rules: the writing disciplines.** Chaos at the poles
only (0 or 100); no wall-clock validity assertions (generate fixtures per run); no sleep as
synchronization (a sleep a test depends on is a race wearing a timeout); no shared global
mutable state without a seam (fresh `Store::new` inline, per-test inboxes on the wire).
These are the rules that keep a regression test honest in *any* home, and they are what
#2/#3 will be written against.

## What changes

- `docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md` — the settlement, status
  `proposed` (this wave's semantic ruling; the operator accepts).
- `scripts/coverage-gate.sh` — **one** new refusal block: every harness-named file under
  `src/` (`tests.rs`, `*_tests.rs`, `*-tests.rs`) must be declared by its parent module
  under `#[cfg(test)]`, else the gate refuses naming the file. The floor refusal
  (`:39-42`), the `coverage(off)` grep (`:56-66`), the leak-check (`:103-116`) and the
  exact-equality verdict (`:181-187`) land byte-identical.
- `src/store.rs` + `src/store/persist_tests.rs` — the one migrated module: the inline
  `mod persist_tests` body moves verbatim (de-indented by `cargo fmt --all`); the parent
  keeps `#[cfg(test)]` and gains `mod persist_tests;`. No production item moves; the
  `#[cfg(test)] pub(crate) fn email` fixture and the seams stay where they are.
- `docs/house-rules.md` — the regression-test placement rule set under **Gates**,
  mirroring decision 0020, with the exact-floor and `coverage(off)` sentences preserved
  verbatim.
- `docs/decisions/README.md` — the 0020 row (filed as 0011; status **proposed**), plus indexing the two
  committed 0010 records with their recorded statuses (the index currently stops at 0009
  and contradicts the directory).

## What does not change

- The exact 100/100/100 floor: lines, branches and functions, literal `covered == count`,
  missed == 0; `COVERAGE_FLOOR` accepted only at 100; `coverage(off)` forbidden.
- The seams: `#[cfg(test)]`-only, compiled out of shipping binaries; no feature flag, no
  new exposure mechanism.
- The frozen guarantees: `Store::insert`, the await subscribe-before-check path and pruning
  are untouched (0001/0002/0003) — `set_pre_mirror_gate` stays absent from non-test builds.
- No new dependency, no test-support crate, no workspace split, no renames, and **no #2/#3
  regression tests** — any test function beyond the migrated module's existing seven is
  out of scope.

## Acceptance Criteria

- **AC1.** `docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md` exists with
  `- **Status:** proposed` and rules all three questions with single named answers — no
  "may", "either" or open option on placement, seam exposure or the counted file set —
  and names the cost of each refused position.
- **AC2.** Exactly **one** inline `#[cfg(test)]` module migrates: `persist_tests` →
  `src/store/persist_tests.rs`, declared `#[cfg(test)] mod persist_tests;` in
  `src/store.rs` (the attribute on the declaration, exactly as the tripwire requires).
  The file contains only the moved test code; the diff is a pure move (module path
  `store::persist_tests` unchanged, `cargo fmt --all` de-indent). Zero or two migrations
  refuse; a production item in the harness file refuses.
- **AC3.** `bash scripts/coverage-gate.sh` exits 0 after the migration, and the migrated
  file has no `SF:` record in `target/coverage/lcov.info`
  (`grep '^SF:' target/coverage/lcov.info | grep 'tests\.rs'` is empty).
- **AC4.** The denominator delta equals the moved file's own footprint exactly — the
  recorded run: lines 4392→4286 (−106), branches 262→262 (±0), functions 580→568 (−12);
  `covered == count` holds in all three dimensions before and after, and no production
  line moved into the harness path. The numbers are re-measured at implementation on the
  same pristine tree and must reproduce.
- **AC5.** Exactness probes all refuse and are reverted before the final gate run:
  `COVERAGE_FLOOR=99` exits 1; a temporarily injected uncovered branch in `src/` exits 1;
  a temporary `#[coverage(off)]` under `src/` exits 1.
- **AC6.** The gate script's only diff is the one structural refusal: a temporary
  harness-named file under `src/` that is undeclared, or declared without `#[cfg(test)]`,
  makes the gate exit 1 naming the file (probe reverted); the properly declared migrated
  module passes; the four hardening blocks are byte-identical to `main`'s.
- **AC7.** `docs/house-rules.md` gains the placement rule set under **Gates** (inline
  default; the cold-path harness beside the owner; `tests/` for the wire;
  `#[cfg(test)]`-only seams; the writing disciplines; reserved harness names), while the
  exact-100/100/100 and `coverage(off)` sentences stay verbatim and nothing in the section
  contradicts decision 0020.
- **AC8.** `docs/decisions/README.md` carries the 0020 row (filed as 0011) with status **proposed** (same
  shape as the 0008/0009 proposed rows) and indexes the two committed 0010 records with
  their recorded statuses (`0010-exact-count-asserts` accepted,
  `0021-watcher-channel-lifetime` proposed).
- **AC9.** Scope guard: the #2 and #3 regression tests are **not** written — no test
  function is added beyond the migrated module's existing seven — and the diff touches
  nothing but decision 0020, the decisions README row, `docs/house-rules.md`,
  `scripts/coverage-gate.sh` (per AC6) and the one migrated module with its parent's
  declaration swap.
- **AC10.** All six gates are green on the final tree (`cargo fmt --all -- --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`, `brokkr compile --bundle .`,
  `bash scripts/coverage-gate.sh`, `cargo deny check licenses`), with `Store::insert`, the
  await subscribe-before-check path, pruning and `set_pre_mirror_gate`'s
  `#[cfg(test)] pub(crate)` declaration byte-identical — the seam absent from non-test
  builds.
