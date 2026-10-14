# Tasks — epic #13 settlement wave: the coverage gate and where concurrency tests live

Ordered; each task names the test that proves it. Scope guard: the #2 and #3 regression
tests are **not** written here (no test function beyond the migrated module's existing
seven); the diff touches only decision 0020, the decisions README rows,
`docs/house-rules.md`, `scripts/coverage-gate.sh` (one refusal block) and the one migrated
module with its parent's declaration swap. Decision
[0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md) is the
ruling; this list is its landing sequence.

- [ ] 1. Record the baseline on the pristine tree: run `bash scripts/coverage-gate.sh`
       and copy `target/coverage/coverage-summary.json` aside as the audit anchor
       (expected 4392/4392 lines · 262/262 branches · 580/580 functions — the preserved
       run agrees). Nothing is edited yet.
       — *Proved by: the gate exits 0 printing exact 100/100/100, and the recorded JSON
       matches the preserved baseline; `grep -c '^SF:' target/coverage/lcov.info` is 18,
       all `src/*.rs` (AC4's `before`).*

- [ ] 2. Migrate the one module, as a pure move: `mkdir -p src/store`; move the body of
       `mod persist_tests` (`src/store.rs:1078-1237` — `use super::*;`, `db_path`, and the
       seven tests, nothing else) verbatim into `src/store/persist_tests.rs`, de-indented
       by `cargo fmt --all`; replace the inline module in `src/store.rs` with
       `#[cfg(test)]` + `mod persist_tests;` (attribute on the declaration). No rename, no
       edit to the moved code, no production item in the new file; the
       `#[cfg(test)] pub(crate) fn email` fixture and both seams stay where they are.
       — *Proved by: `cargo test --lib store::persist_tests` runs the same seven tests
       (module path `store::persist_tests` unchanged — same names, same outcomes), and the
       diff shows the body moved with no content change beyond the de-indent (AC2).*

- [ ] 3. Add the structural tripwire to `scripts/coverage-gate.sh` — one refusal block
       after the leak-check: every `src/**/*.rs` named `tests.rs`/`*_tests.rs`/
       `*-tests.rs` must have a parent module file (`src/<dir>.rs` or
       `src/<dir>/mod.rs`) that declares it with `#[cfg(test)]` governing the declaration
       (one-line `#[cfg(test)] mod <stem>;`, or `#[cfg(test)]` first in the attribute
       stack directly above `mod <stem>;`); missing parent, missing declaration or an
       ungated declaration exits 1 naming the file and citing decision 0020. The four
       hardening blocks (`:39-42` floor, `:56-66` `coverage(off)`, `:103-116` leak-check,
       `:181-187` exact equality) stay byte-identical; no counting stage is added.
       — *Proved by: the gate exits 0 on the migrated tree (the worked example passes);
       `git diff main -- scripts/coverage-gate.sh` shows exactly the one block; and both
       negative probes of task 4 refuse (AC6).*

- [ ] 4. Prove the refusals, reverting every probe before the final gate run:
       (a) `COVERAGE_FLOOR=99 bash scripts/coverage-gate.sh` → exit 1; (b) a temporarily
       injected uncovered branch in `src/` → exit 1; (c) a temporary `#[coverage(off)]`
       item under `src/` → exit 1 (before any build); (d) a temporary
       `src/store/orphan_tests.rs` declared nowhere → exit 1; (e) the migrated declaration
       temporarily ungated (`mod persist_tests;`) → exit 1 while `cargo build` still
       succeeds — the silent hole the tripwire exists to close.
       — *Proved by: all five probes exit 1 with the documented refusal text, and
       `git status` is clean of probe residue before the next gate run (AC5, AC6).*

- [ ] 5. Run the denominator audit: post-migration `coverage-summary.json` shows lines
       4286, branches 262, functions 568 with `covered == count` in all three dimensions;
       delta == the moved file's footprint (−106 lines = its recorded `DA:` count, ±0
       branches, −12 functions); `grep '^SF:' target/coverage/lcov.info |
       grep 'tests\.rs'` is empty; the report is still exactly the 18 `src/*.rs` files.
       A different delta means a production line moved — stop and investigate, never tune.
       — *Proved by: the recorded before/after JSON pair and the empty grep (AC3, AC4);
       `run_pre_mirror_gate`'s arm still covered with the gate's only installers now in
       the harness file (the dead-seam refusal follows execution).*

- [ ] 6. Write the placement rules into `docs/house-rules.md` under **Gates**, after the
       `coverage(off)` paragraph: decision 0020's six rules (inline default and its
       vacuity policing; the cold-path harness `src/<module>/<name>_tests.rs` declared
       `#[cfg(test)] mod <name>_tests;` beside its owner, a refuge for cold paths never a
       landfill; `tests/*.rs` for public-API wire-shaped races with no crate-private
       seams; seams `#[cfg(test)]`-only, never features, each with an installing test;
       chaos at poles only, no wall-clock assertions, no sleep-as-sync, no shared global
       state without a seam; harness names reserved under `src/`, enforced by the gate).
       The exact-floor and `coverage(off)` sentences stay verbatim.
       — *Proved by: `git diff main -- docs/house-rules.md` shows only the appended
       paragraph, the floor and `coverage(off)` sentences unchanged, and no sentence
       contradicting decision 0020 (AC7).*

- [ ] 7. Index the decisions: add the 0020 row (filed as 0011) to `docs/decisions/README.md` with status
       **proposed (awaiting operator)** in the 0008/0009 shape, and index the two
       committed 0010 records with their recorded statuses
       (`0010-exact-count-asserts-and-the-shared-seed` — accepted;
       `0021-watcher-channel-lifetime` — proposed). No other churn.
       — *Proved by: the rendered table lists 0009, both 0010s and the filed-as-0011 row in order with
       statuses matching each file's `- **Status:**` line (AC8).*

- [ ] 8. Leave all six gates green on the final tree and commit exactly the diff surface:
       `cargo fmt --all -- --check`; `cargo clippy --all-targets -- -D warnings`;
       `cargo test`; `brokkr compile --bundle .`; `bash scripts/coverage-gate.sh`;
       `cargo deny check licenses` — then a single commit of `src/store.rs`,
       `src/store/persist_tests.rs`, `scripts/coverage-gate.sh`, `docs/house-rules.md`,
       `docs/decisions/README.md` (decision 0020 and the spec artifacts are already
       committed by the chief). Commit unsigned, like the rest of this worktree's history;
       never push. Confirm the scope guard: no #2/#3 test bodies anywhere in the diff.
       — *Proved by: all six commands exit 0 (AC10); `git show --stat` lists exactly the
       six files; `git grep -n 'fn ' -- src/store/persist_tests.rs` shows the same seven
       test functions the module had inline, no more (AC9); `set_pre_mirror_gate` remains
       `#[cfg(test)] pub(crate)` at its declaration and absent from
       `cargo build` artifacts (AC10).*
