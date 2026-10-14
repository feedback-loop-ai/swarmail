# 0020 — Concurrency regression tests and the exact coverage gate: the harness moves, the bar does not

- **Status:** proposed (awaiting operator acceptance)
- **Date:** 2026-10-12
- **Supersedes:** none
- **Provenance:** originally filed as `0011-concurrency-tests-and-the-coverage-gate`;
  renumbered 0011 → 0020 at issue #32's docs pass — it was the second 0011
  to land, so the shim-delete contract kept 0011.
- **Companion:** epic [#13](https://github.com/feedback-loop-ai/swarmail#13) settlement wave,
  issue [#18](https://github.com/feedback-loop-ai/swarmail/issues/18); spec at
  [`specs/concurrency-tests-coverage-gate/spec.md`](../../specs/concurrency-tests-coverage-gate/spec.md);
  unblocks issues #2 and #3 (their regression tests are *not* written by that wave — this
  ruling is the ground they stand on).
- **Numbering:** 0011 was the next free number on `main` at filing; two 0010 records already exist
  (`0010-exact-count-asserts-and-the-shared-seed`, accepted, and
  `0021-watcher-channel-lifetime` — filed as `0010-watcher-channel-lifetime`,
  proposed on the wave-3 branch). Per the
  operator-renumbers-at-acceptance protocol recorded in 0010-exact-count-asserts, the
  operator resolves any collision at acceptance.

## Context

Issue #18 is a collision between two instruments the realm refuses to bend, and it must be
settled **before** the #2 and #3 regression tests are written:

- The exact coverage gate counts inline `#[cfg(test)]` modules in `src/` **as production
  report** (`scripts/coverage-gate.sh:103-106`): every line and branch of an inline test is
  part of the 100/100/100 denominator, so every branch an inline test carries must be taken.
- Deterministic interleaving tests need crate-private seams — `Store::set_pre_mirror_gate`
  is `#[cfg(test)] pub(crate)` (`src/store.rs:243-244`) — which the issue read as "so they
  must live inline".
- Stress/race regression tests are naturally conditional ("retry until the window is hit");
  their failure branches go uncovered once the bug is fixed, so written inline they would
  red the exact gate on a green-code day, forever.
- Integration tests under `tests/` are already outside the report, so public-API stress can
  live there.

The exact 100/100/100 floor does not move: lines, branches and functions stay exact
(missed == 0, not a rounded 99.95), and `coverage(off)` stays forbidden. The only open
question is **which files constitute the measured report** — and what keeps that answer
honest.

Both panel seats measured the same load-bearing fact, independently (one from the tool's
source, one empirically in this tree): **cargo-llvm-cov 0.9.1's default ignore regex
excludes from the report any `src/` file whose basename is `tests.rs` or `<name>_tests.rs`
(and any path with a `tests/` segment)** — while such a file still compiles into the lib
test target, still runs under `cargo test`, and its *execution of production lines* is
still counted as covered. The exclusion is name-based; the file's own lines drop out of the
denominator; the production lines it exercises do not. Preserved evidence:
`target/coverage/lcov.info` carries exactly 18 `SF:` records, all `src/*.rs` — zero for
`tests/`.

## Ruling

Three questions, three single answers — the settlement does not hedge.

**1. Where each kind of test lives — the axis is *cold-path presence*, not seam access.**

- **Inline `#[cfg(test)]` in `src/<module>.rs` is the default and stays measured.** A test
  whose every branch and line is taken on every run belongs inline — *including*
  seam-driven deterministic interleavings. The exact gate polices inline test bodies as a
  vacuity detector (a refactor that stops an arm being exercised goes red even when the
  assertion still passes), and it refuses dead seams: if nothing installed a gate, the
  `if let Some(gate) = gate` arms (`src/store.rs:260-262, 297-299`) would go uncovered.
  *Refused here:* the simplicity seat's rule that "a deterministic interleaving test that
  needs a crate-private seam lives in `src/<module>/<name>_tests.rs`". Taken literally it
  sweeps branch-stable seam tests out of the measured denominator for no gain — the harness
  must be a refuge for the cold path, never a landfill for the inconvenient. Cost of the
  refusal: seam-adjacent tests stay subject to the exact gate's refusal on any cold line
  they carry, which is the constitution working, not a problem.
- **A test that needs a *cold failure path* — a line that runs only while the bug is
  present or the schedule betrays (a retry budget exhausted, a "window never hit"
  diagnostic, a sleep-as-sync residual) — is unwritable inline** (measured: the natural
  race-regression shape placed inline refuses the gate — lines 4294/4293, branches
  264/263 — while the same test passes from a harness file with counts identical to the
  clean migration). Its home is **`src/<module>/<name>_tests.rs`, declared
  `#[cfg(test)] mod <name>_tests;` in its parent**: it sees `pub(crate)`, it runs in
  `cargo test`, its own lines are excluded from the report by its harness name, and it
  still covers the production lines it executes. The *name is the contract* — keep the
  underscore form `<name>_tests.rs`; the tool also excludes hyphenated basenames but the
  realm's convention does not.
- **Public-API, wire-shaped races live in `tests/*.rs`** — real protocol against real
  servers, no mocks (0004): burst, rate, restart, hostile bytes, TLS expiry. A `tests/`
  test must not need a crate-private seam; if it does, it is a rule-2 test (the wire is not
  where those races live — `pub(crate)` is invisible outside the crate).
- **Writing disciplines that keep a regression test honest, wherever it lives:** chaos at
  the poles only (probability 0 or 100 — a mid-band rule is unreproducible by construction);
  no wall-clock validity assertions (generate the fixture per run, or drive the branch
  behind a generated expired-cert/seam fixture); no sleep as synchronization (a sleep a
  test depends on is a race wearing a timeout — use a seam or a real deadline); no shared
  global mutable state without a seam (fresh `Store::new` inline; per-test inboxes on the
  wire). Assert invariants that hold under every interleaving inline; assert windows only
  through seams.
- **The worked proof — one module migrates and stays: `persist_tests`.** The inline
  `mod persist_tests` (`src/store.rs:1077-1237`) moves to `src/store/persist_tests.rs`,
  declared `#[cfg(test)] mod persist_tests;` in its parent — a pure move: the module path
  `store::persist_tests` is identical before and after, so `use super::*`, `pub(crate)`
  access and every visibility rule compile unchanged; the module keeps its name (renaming
  to `tests` is refused — the basename pattern already matches, and the recorded
  before/after evidence depends on the pure move). It stays re-homed as the **standing
  reference harness beside its owner**: the file that owns the pre-mirror-gate seam-window
  family and the `db_path` helper, beside which #2's conditional descendants land. It is
  the single deliberate re-homing; it does **not** license sweeping other inline modules
  out — an inline module migrates only when a concrete test demands it (a cold path, or
  seam-family adjacency). Cost, named: the module's 106 lines and 12 functions of
  branch-stable assertion code permanently leave the measured denominator; their policing
  drops from coverage-refused to functional (the asserts still fire and `cargo test` still
  runs them).

**2. How crate-private seams are exposed — exactly as today: `#[cfg(test)]` on the owning
module, `pub(crate)` where a sibling harness file needs them.** `set_pre_mirror_gate`,
`set_post_subscribe_gate`, `watcher_count` and `CLOSED_PROBE` keep their attributes
(`src/store.rs:27-28, 243-244, 269-270, 276-277`) and are compiled out of shipping
binaries like the rest of the harness. **A feature flag is refused**: it doubles every
gate's build matrix, mints a configuration no gate tests by construction, and risks
feature-unification pulling test hooks into artifacts that must not have them. Every seam
keeps at least one test that installs it — and the dead-arm refusal is
*placement-independent*, because coverage follows execution: the measured proof kept
branches at 262/262 **with the only `set_pre_mirror_gate` installers living in the moved
harness file** — harness execution of production lines still feeds the report.

**3. What the gate counts — the tool's report, plus one new structural refusal.**

- The measured report is whatever cargo-llvm-cov 0.9.1 produces: production `src/**` plus
  inline `#[cfg(test)]` modules; harness-named paths and `tests/` directories are excluded
  by the tool's default ignore regex, and the script's leak-check
  (`coverage-gate.sh:103-116`) independently refuses any harness path that leaks **into**
  the report. The contract is untouched: literal integer `covered == count` in all three
  dimensions (`:181-187`), `COVERAGE_FLOOR` accepted only at 100 (`:39-42`),
  `coverage(off)` refused under `src/` (`:56-66`), unique instrumented target dir per run.
  **No counting stage is added and none may be**: the gate refuses, it never re-counts.
- **Adopted from the robustness seat — one structural tripwire the simplicity seat
  declined:** every harness-named file under `src/` must be declared by its parent module
  under `#[cfg(test)]`; anything else refuses. The leak-check fires on *presence*; it
  cannot see the opposite hole, and the settlement opens it: a file named
  `src/anything/tests.rs` (or `*_tests.rs`) is invisible to the report *and* to the
  leak-check, so production code parked there — declared `mod tests;` without the gate —
  would silently shrink the measured denominator while `cargo build` succeeds and the
  100/100/100 verdict lies. That is a lowering no summary JSON would show, i.e. exactly
  what house rules refuse; the tripwire is the same species of refusal the script already
  ships for `coverage(off)` (a git grep that fails closed). A false positive is loud and
  names the file; the correct failure direction is *refuse*, never *silently mis-count*.
- *Ruled against the simplicity seat's "the gate script lands with a byte-identical diff,
  i.e. no diff":* its own principle decides the conflict. Simplicity's risk ledger requires
  that the failure mode be loud and that nothing silently mis-count — but without the
  tripwire, the hide-out direction is *silent* by construction, and the wave that legalizes
  harness-named files under `src/` is the same wave that first makes the hole reachable.
  Cost of the adoption: ~15 lines of refusal logic in the most security-sensitive file in
  the repo, plus one new convention (harness names are reserved under `src/`). Cost of the
  declined alternative, named: a silent way to make 100/100/100 lie ships inside the very
  commit that proves harness names are invisible. The four existing hardening blocks land
  byte-identical — the tripwire adds a refusal, it touches no count.
- *Reconciliation with the intake's C5 ("a script change lands IFF the worked proof
  requires it"):* the worked proof did not need a script change for the migration to pass —
  no harness reclassification exists and none is created. The tripwire is required by the
  *same measured fact* the proof established, in the opposite direction: the proof
  demonstrated that a harness-named file is invisible to the report; invisibility is
  precisely the property production code would abuse. C5's substantive intent — the
  exact-equality check, the floor refusal and the `coverage(off)` grep stay byte-identical,
  and no measurement stage is added — holds verbatim.

## The worked proof

- Baseline (pristine tree): **4392/4392 lines · 262/262 branches · 580/580 functions** —
  preserved in `target/coverage/coverage-summary.json`; the report counts inline test code
  (106 `DA:` records and 12 function identities belong to `mod persist_tests`).
- After the pure move to `src/store/persist_tests.rs`: all six gates green; **4286/4286 ·
  262/262 · 568/568**; **zero `SF:` records** for the harness file; the report is still
  exactly the 18 `src/*.rs` files. Denominator delta == the moved file's own footprint
  (−106 lines, ±0 branches, −12 functions); `covered == count` holds in all three
  dimensions before and after; no production line moved into the harness file.
- The load-bearing asymmetry, proven maximally: `set_pre_mirror_gate`'s only installers are
  the two tests the move relocated (`src/store.rs:1194, 1221`), and the branches still came
  out 262/262 — the gate-installing harness file kept `run_pre_mirror_gate`'s arm covered.
- Refusal probes, all reverted before the final gate run: `COVERAGE_FLOOR=99` → exit 1
  ("the contract is exact 100% … the floor may rise, never fall"); a `#[coverage(off)]`
  item under `src/` → exit 1 before any build; a cold diagnostic path inline → exact-gate
  refusal; the same cold path in the harness file → pass with identical counts.

## Consequences

- The production denominator shrinks **only** by reclassified test lines — 106/12 today,
  and further only when a concrete cold-path or seam-adjacency demand migrates a module.
  The floor does not move: it is still exact 100/100/100 *of what is measured*, and
  `covered == count` remains literal integer equality.
- `docs/house-rules.md` gains the placement rule set under **Gates** (inline default; the
  cold-path harness beside the owner; `tests/` for the wire; `#[cfg(test)]`-only seams;
  the writing disciplines; reserved harness names), keeping verbatim that the floor is
  exact 100/100/100 and that `coverage(off)` is forbidden.
- `docs/decisions/README.md` gains the 0020 row (status **proposed**, filed
  as 0011) — and indexes the two committed 0010 records (exact-count, and
  watcher-lifetime now 0021) with their recorded statuses, which the index
  currently omits.
- The exclusion depends on cargo-llvm-cov's upstream default regex. Accepted residual:
  `Cargo.lock` + `--locked` pin the graph, and both failure directions are tripped —
  harness leaking *in* refuses via the leak-check (the drift alarm if upstream changes);
  production hiding *out* refuses via the structural tripwire.
- Issues #2 and #3 are unblocked to write their regression tests in their natural shape —
  conditional retry budgets with cold diagnostics, in harness files beside the seams they
  force — without reding the exact gate on a green-code day.

## Non-goals

- No semantic change to 0001 (synchronous insert), 0002 (subscribe-before-check) or 0003
  (oldest-only pruning): `Store::insert`, the await path and pruning are untouched, and
  `set_pre_mirror_gate` stays `#[cfg(test)]`-only — absent from non-test builds.
- No `coverage(off)`, no feature-flagged seams, no test-support crate, no workspace split,
  no new dependency (the allowlist is untouched), no mass migration of inline modules, no
  renames.
- **The #2 and #3 regression tests are not written by this settlement** — it only lays the
  ground they stand on.
