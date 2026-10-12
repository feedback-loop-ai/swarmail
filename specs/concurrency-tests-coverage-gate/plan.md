# Plan — epic #13 settlement wave: concurrency regression tests and the exact coverage gate

Companion to [`spec.md`](spec.md) (WHAT/WHY) and decision
[`0011`](../../docs/decisions/0011-concurrency-tests-and-the-coverage-gate.md) (status
`proposed`). How the settlement lands: the exact edit, the files it touches, and the risks
with mitigations. The #2/#3 regression tests are **not** written here.

## Technical context

The tree is clean on `feat/epic13-18-settlement`; every anchor below is read from the
working tree and re-verifiable in seconds.

- `scripts/coverage-gate.sh` — the exact gate. `:39-42` floor refusal (COVERAGE_FLOOR only
  100); `:56-66` `coverage(off)` git grep (run with `GIT_CONFIG_COUNT=0`, the documented
  sidestep for a broken harness `GIT_CONFIG_KEY_0`); `:79-96` the measurement
  (`cargo +nightly llvm-cov --workspace --locked --branch`, unique
  `CARGO_LLVM_COV_TARGET_DIR`); `:103-116` the leak-check (refuses `tests.rs`,
  `*_tests.rs`, `tests/` paths present among `SF:` records); `:118-187` the exact-count
  awk + literal `covered == count` verdict; `:189` the summary JSON.
- `src/store.rs` — the seam host. `#[cfg(test)] static CLOSED_PROBE` (`:27-28`);
  `#[cfg(test)] pub(crate) fn set_pre_mirror_gate` (`:243-244`) and
  `set_post_subscribe_gate` (`:276-277`); `watcher_count` (`:269-270`); the runner arms
  `run_pre_mirror_gate` (`:254-263`) / `run_post_subscribe_gate` (`:291-300`) whose
  `if let Some(gate)` lines are the dead-seam tripwire. Inline test mods: `tests`
  (`:585`), `wait_tests` (`:766`), `filter_tests` (`:941`), `persist_tests`
  (`:1077-1237`, with the shared `#[cfg(test)] pub(crate) fn email` fixture at `:557-583`
  — **not** inside a test mod, untouched). The only `set_pre_mirror_gate` callers are
  `:1194` and `:1221` (inside `persist_tests`); `set_post_subscribe_gate` callers
  (`:857`, `:897`) live in inline `tests` and stay inline — they are branch-stable.
- `target/coverage/` — the preserved baseline: `coverage-summary.json`
  **4392/4392 lines · 262/262 branches · 580/580 functions**; `lcov.info` with exactly 18
  `SF:` records, all `src/*.rs`. cargo-llvm-cov 0.9.1 is installed; nightly is required by
  `--branch`.
- `docs/house-rules.md` — **Gates** section ends at the `coverage(off)` paragraph
  (`:44-49`); the placement rules slot in after it, verbatim sentences preserved.
- `docs/decisions/` — `0010-exact-count-asserts-and-the-shared-seed.md` (accepted) and
  `0010-watcher-channel-lifetime.md` (proposed) are committed but absent from
  `README.md`'s index, which stops at 0009; the 0011 row lands with both repaired.
- Worktree facts: `commit.gpgsign` resolves **false** (brokkr's config overlay; recent
  history is unsigned) — commit like the rest of history; the harness environment exports
  a broken `GIT_CONFIG_COUNT=1` without `GIT_CONFIG_KEY_0`, so drive `git` with
  `GIT_CONFIG_COUNT=0` (the gate script's own sidestep) or unset the variable.

## Constitution check — frozen guarantees

| Guarantee | Verdict |
|---|---|
| 0001 accept == stored | Untouched. `Store::insert`, `src/persist.rs` and the SMTP accept path are not edited; the moved tests continue to prove them. |
| 0002 await subscribes before it checks | Untouched. `wait_for` is not edited; the `wait_tests`/`tests` modules stay inline and measured (they are branch-stable), and the post-subscribe seam keeps its inline installers. |
| 0003 pruning is oldest-only | Untouched. |
| 0004/0005 | Untouched. `tests/` keeps speaking the real protocol; extraction stays at ingest. |
| Coverage floor (exact 100/100/100) | **Not weakened — hardened.** The floor stays literal `covered == count`; the denominator shrinks only by the moved file's own test-code footprint (delta-attributed); `coverage(off)` stays refused; and the new tripwire closes the one silent-shrink path the harness name opens. |
| 0007 hermetic gates, single model | Untouched. No new capability, no new model route, no new build axis; the gate keeps its unique target dir. |
| 0009 the agent council | This spec is the design phase's product under 0009's dialect; the deterministic design check rules the artifacts. |
| 0010-exact-count-asserts (test discipline) | Extended, not weakened: the shared-seed/exact-count disciplines are orthogonal; the two 0010 records finally appear in the index. |

## Project structure — the complete diff surface

```
docs/decisions/0011-concurrency-tests-and-the-coverage-gate.md   the ruling (committed by the chief)
docs/decisions/README.md                                         0011 row + the two 0010 rows
docs/house-rules.md                                              placement rules under Gates
scripts/coverage-gate.sh                                         ONE structural refusal block
src/store.rs                                                     inline mod persist_tests -> declaration
src/store/persist_tests.rs                                       the moved module (new file)
specs/concurrency-tests-coverage-gate/{spec,plan,tasks}.md       this design (committed by the chief)
```

No new dependency, no `Cargo.toml`/`deny.toml` movement, no other `src/` file touched, no
new test function. `tests/` is untouched (its exclusion from the report is already real:
zero `SF:` records outside `src/`).

## Implementation steps

### 1. Baseline first (the audit's anchor)

Run `bash scripts/coverage-gate.sh` on the pristine tree and record
`target/coverage/coverage-summary.json` (expected: 4392/4392 · 262/262 · 580/580). Every
later number is attributed against this.

### 2. The migration — a pure move

1. `mkdir -p src/store`
2. Move the **body** of `mod persist_tests` (`src/store.rs:1078-1237`: `use super::*;`
   plus the seven tests and the `db_path` helper) to `src/store/persist_tests.rs`,
   de-indented one level by `cargo fmt --all`. Nothing else changes — `use super::*;`
   resolves to `store::` exactly as before, so `pub(crate)` items (`email`,
   `set_pre_mirror_gate`) and `persist.fail_writes()` compile unchanged.
3. Replace the inline module in `src/store.rs` with exactly:

   ```rust
   #[cfg(test)]
   mod persist_tests;
   ```

   (the `#[cfg(test)]` attribute already at `:1077` stays on the declaration — the
   tripwire's worked example).

### 3. The structural tripwire — one refusal block in `scripts/coverage-gate.sh`

Insert immediately after the leak-check (after `:116`), before the count extraction. It
must refuse — never re-count — and name the offending file:

- Enumerate `src/**/*.rs` whose basename is `tests.rs`, `*_tests.rs` or `*-tests.rs`
  (the tool's full basename set; house rules pin the underscore spelling).
- For each, locate the parent module file: the conventional sibling `src/<dir>.rs` (for
  `src/store/persist_tests.rs` → `src/store.rs`) or `src/<dir>/mod.rs`. No parent file →
  refuse (an orphaned harness file is a defect, fail-closed).
- The parent must declare the module with `#[cfg(test)]` governing the declaration: either
  `#[cfg(test)] mod <stem>;` on one line, or `#[cfg(test)]` as the first attribute of the
  attribute sequence directly above `mod <stem>;` (walk up over attribute-only lines; no
  `mod` keyword may appear between). Anything else → refuse.
- Refusal text names the file and cites decision 0011, in the style of the existing
  refusals; exit 1.

The four hardening blocks (`:39-42`, `:56-66`, `:103-116`, `:181-187`) land
**byte-identical**; the diff adds exactly one `#`-commented block plus its logic, after
the leak-check.

### 4. The probes — prove both directions, then revert everything

1. **Exactness still refuses:** `COVERAGE_FLOOR=99 bash scripts/coverage-gate.sh` → exit
   1; a temporarily injected uncovered branch in `src/` → exit 1; a temporary
   `#[coverage(off)]` under `src/` → exit 1 (before any build). Revert each.
2. **The tripwire refuses both broken shapes:** temporarily add
   `src/store/orphan_tests.rs` with no declaration anywhere → exit 1; temporarily change
   the migrated declaration to ungated `mod persist_tests;` → exit 1 (and `cargo build`
   still succeeding is exactly the hole the tripwire closes). Revert each; re-run the
   gate green.

### 5. The denominator audit (decision 0011's evidence)

Post-migration `target/coverage/coverage-summary.json` must show lines **4286**, branches
**262**, functions **568**, `covered == count` in all three dimensions; delta == the moved
file's footprint (−106 lines, ±0 branches, −12 functions — the module's recorded `DA:`
and function-identity counts); `grep '^SF:' target/coverage/lcov.info | grep 'tests\.rs'`
empty; the report still exactly the 18 `src/*.rs` files. Any other delta means a
production line moved into the harness — refuse the migration, not the gate.

### 6. `docs/house-rules.md` — the placement rules

Append under **Gates**, after the `coverage(off)` paragraph (`:44-49`, byte-identical), a
paragraph **"Regression-test placement (decision 0011)"** carrying the six rules: inline
default (measured, vacuity-policed, seam-driven or not); the cold-path harness
`src/<module>/<name>_tests.rs` declared `#[cfg(test)] mod <name>_tests;` beside its owner
(runs in `cargo test`, excluded from the report, still covers the production lines it
executes; a refuge for cold paths, never a landfill — seam access alone never forces a
move); `tests/*.rs` for public-API wire-shaped races (no mocks, 0004; no crate-private
seams there); seams stay `#[cfg(test)]` on the owning module, never features, each with a
test that installs it (dead-arm refusal follows execution, not file location); the writing
disciplines (chaos at poles only, no wall-clock assertions, no sleep-as-sync, no shared
global state without a seam); harness names reserved under `src/` (the gate refuses an
undeclared or ungated one).

### 7. `docs/decisions/README.md` — the index rows

One row for 0011 (`proposed (awaiting operator)`, the 0008/0009 shape) between 0009 and
the 0010 rows, plus the two committed 0010 records with their recorded statuses
(`0010-exact-count-asserts-and-the-shared-seed` — accepted; `0010-watcher-channel-lifetime`
— proposed). No other index churn.

### 8. The six gates, then one commit

`cargo fmt --all -- --check` · `cargo clippy --all-targets -- -D warnings` ·
`cargo test` · `brokkr compile --bundle .` · `bash scripts/coverage-gate.sh` ·
`cargo deny check licenses` — all green on the final tree, then a single commit of exactly
the diff surface above (spec artifacts already committed by the chief; unsigned, like the
rest of this worktree's history). **No #2/#3 test bodies.**

## Risks

| Risk | Mitigation |
|---|---|
| **The tripwire false-positives on a legitimate declaration** (extra attributes stacked on the `mod`). | The matcher accepts `#[cfg(test)]` plus an attribute-only stack directly above the declaration; the migrated module is the worked example that must pass. A false positive is loud and names the file — the fail-closed direction; the fix is to declare the harness properly. |
| **The tripwire misses an exotic alias** (`#[path = "..."]` pointing at a harness-named file, or a `mod` declared from a non-conventional parent). | Accepted residual, named in 0011: it requires deliberate misconduct, `dead_code` and review still apply, and the structural check is deliberately a grep (no heuristics that could silently mis-count). The conventional-parent rule keeps the check exact; exotic layouts refuse (no parent found → refuse). |
| **cargo-llvm-cov changes its upstream default ignore regex** and a harness file re-enters the report. | Two tripwires fire loudly: the leak-check refuses any harness path among `SF:` records, and the harness's own cold paths would fail exact equality. `Cargo.lock` + `--locked` pin the graph; the leak-check is the drift alarm. No silent path exists in either direction. |
| **The denominator shrink is misread as gate weakening.** | Delta attribution is the proof: `lines.count` moves by exactly the moved file's `DA:` records (−106), branches ±0, functions −12; `covered == count` holds throughout; no production line changes files. The summary before/after is preserved under `target/coverage/`. |
| **The migration is mistaken for a license to sweep inline tests out** (the "harness as landfill" failure). | Decision 0011 names `persist_tests` the single deliberate re-homing; house rules state inline as the default and the two triggers (cold path, seam adjacency); review checks every move against them. The vacuity detector's surviving teeth are measured: branches stayed 262/262 with the only gate installers in the harness file. |
| **The moved module rots unrun** (its file is invisible to the coverage report). | `cargo test` runs it — same module path `store::persist_tests`, same suite; the gate's instrumented run executes the whole workspace. Nothing escapes the six gates; it stops being *double*-counted. |
| **The `set_post_subscribe_gate` installers stay inline while the pre-mirror ones move** — an asymmetry a reviewer may flag. | It is the ruling applied, not an oversight: the post-subscribe tests (`:857`, `:897`) are branch-stable and stay measured (0002's jurisdiction); the pre-mirror module is the worked proof and #2's future home. Both facts are stated in 0011 and the spec. |
| **A gate exposes an unexpected failure** (fmt on the de-indent, clippy on the moved body, a count off by one). | Fix exactly the exposure; do not re-derive the settlement. A count mismatch is a stop-and-investigate (a production line moved) — not a knob. |
| **GPG**: house rules describe signed commits, but this worktree's effective `commit.gpgsign` is **false** and all recent commits are unsigned. | Commit like the rest of the history; do not flip config the operator set. If the operator re-signs, this commit is in the re-sign set like every other. |

## Quickstart (reviewer)

```bash
bash scripts/coverage-gate.sh                    # AC3/AC4: exact pass, 4286/262/568 recorded
grep -c '^SF:' target/coverage/lcov.info         # AC3: 18, none matching tests\.rs
git diff main -- scripts/coverage-gate.sh        # AC6: one refusal block added, hardening byte-identical
grep -n 'cfg(test)' src/store.rs | grep persist  # AC2: the declaration is gated
COVERAGE_FLOOR=99 bash scripts/coverage-gate.sh  # AC5: refuses; revert nothing (env-only probe)
cargo test --lib store::persist_tests            # AC10: the moved suite runs
cargo deny check licenses && brokkr compile --bundle .
git log -1 --stat                                # AC9: the scoped diff surface
```
