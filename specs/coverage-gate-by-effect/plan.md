# Plan — the gate refuses by effect: harvest, expansion, helper (run 12g, feature `coverage-gate-by-effect`)

Companion to [spec.md](spec.md); the ruling and acceptance live there. This
document is the implementation approach: exact artifacts, exact commands,
exact line budgets, and the risks with their mitigations.

## Technical context

- Base `745d6ad`. `scripts/coverage-gate.sh` (606 lines) runs: static
  refusals → build-dir resolution + preflight → toolchain resolution →
  `llvm-cov clean --workspace` → `cargo +<tc> llvm-cov --workspace --locked
  --branch --lcov --output-path <run>/lcov.info` → `check_no_harness_leak`
  → `count_dimensions` → jq summary → exact-equality verdict. The default
  build dir is a fresh `target/coverage/build.XXXXXX` (trap-removed only if
  gate-owned); per-run evidence lives in `target/coverage/run.XXXXXX`
  (preserved).
- Toolchain: nightly with `llvm-tools` (resolved by `probe_llvm_tools`;
  `nightly-2026-09-05` qualifies on this host). `cargo-llvm-cov` 0.9.1.
  Host awk is mawk; the gate adds no awk beyond what main has.
- Single crate, no `[features]`, no path dependencies; `src/main.rs` has no
  `#[cfg(test)]` modules; the bench targets live under `benches/` and are
  excluded from the report by the tool's default regex (status quo, out of
  scope).

## Constitution check

- **Frozen guarantees (0001/0002/0003):** untouched — no `src/` file
  changes; the gate refuses more and counts nothing differently.
- **Extraction at ingest (0005):** transposed — both inventories are
  extracted at build time (dep-info written by the build; the expansion
  dumped from the compiler) and the checks are queries over the extracted
  record. Nothing re-derives an inventory from source at check time.
- **Decision 0020:** rule 6's tripwire stays byte-identical; no test harness
  moves; the helper is a tool, not a harness file, and lives outside `src/`.
  §3's leak-check sentence is amended by decision 0035 (`proposed`), not
  edited.
- **Decisions 0004/0007 (no mocks, hermetic):** the selftest arms are real
  cargo builds of zero-dependency fixture crates, `--offline`, under
  `target/coverage/` (never `$TMPDIR`); the verify seat's network denial
  breaks nothing new — `syn` 2.0.119 is cached in the realm's cargo home.
- **Coverage floor:** 100% exact, unchanged; `count_dimensions`
  byte-identical; no `coverage(off)` anywhere (the fixtures live in
  generated selftest trees under `target/coverage/`, gitignored, and are
  removed by the trap).

## Project structure — files touched

| Path | Change |
|---|---|
| `scripts/coverage-gate.sh` | two new refusals, report/expansion validation, harvest+capture, rescoped `check_no_harness_leak`, real-build selftest arms. No Rust lexing. Budget: ≤ 606 + 140 = **746 lines**. *(Implementation amendment: landed at **788** — 606 + 182. The overage is the real-build arms' own weight the commission demands — a lib+report+harvest+expansion sequence per fixture plus the fast arms — and the validator's fail-closed messages; no lexer returned, `count_dimensions` is byte-identical, and nothing under `src/` changed. The estimate was written before the arm shapes were known.)* |
| `tools/coverage-fn-scan/Cargo.toml` | new; own `[workspace]`; `syn = { version = "2.0.119", features = ["full"] }` (the fn probe) and `proc-macro2 = { version = "1.0.107", features = ["span-locations"] }` (the token walk); `edition = "2021"`. |
| `tools/coverage-fn-scan/Cargo.lock` | new; committed; pins syn 2.0.119 + proc-macro2 + unicode-ident (all cached, all MIT OR Apache-2.0). |
| `tools/coverage-fn-scan/src/main.rs` | new; ~200 lines; subcommands `files` and `attrs`; refuses (nonzero + stderr message) or passes (exit 0). |
| `docs/decisions/0035-coverage-gate-refuses-by-effect.md` | new; status `proposed`. |
| `specs/coverage-gate-by-effect/{spec,plan,tasks}.md` | this batch. |

Nothing under `src/`, `tests/`, `bundles/`, `deny.toml` or the root
manifest changes. The three superseded recognizer specs keep their 12f
status lines.

## The dep-info harvest rule (derived from fact 4 of the spec, verified against a real build)

A `.d` in the build dir's `deps/` carries several rules; the parser keeps
only the two production shapes and only their prerequisite lists:

1. **The lib:** the rule whose target is `libswarmail-<hash>.rlib`. Its
   prerequisites are the lib's file set (18 `.rs` + 3 `include_str!`
   assets). The lib's *test harness* never writes an `.rlib` rule, so a
   `--cfg test` unit cannot be mistaken for the plain lib. Multiple
   `.rlib`-carrying `.d`s cannot occur for one package in one build dir
   (`cargo llvm-cov clean --workspace` precedes the build).
2. **The bins:** rules whose target is an extension-less
   `deps/swarmail-<hash>` executable — but only those whose prerequisites
   contain a bin root (`src/main.rs`) and not the lib root (`src/lib.rs`).
   The lib's test harness writes the same executable-shaped `.d` and is
   excluded exactly by that content difference; the bin's own test harness
   (if ever built) shares the bin's file set and is harmless to harvest.
   Unit roots come from `cargo metadata --no-deps` via jq (one call, no new
   env-var vocabulary), not from hardcoding.
3. **Prerequisite hygiene:** honor `\`-continuations; unescape GNU-make
   escapes (`\ `, `\#`); skip blank lines, `#` comments and `env-dep:`
   lines; ignore the per-file no-prerequisite rules. Every entry is
   normalized against the workspace root; an entry that is not an existing
   regular file under the workspace **refuses, naming it** (a newline in a
   filename is unrepresentable in a depfile and resolves to nothing — the
   hostile-path rule).
4. **The fn probe:** each resolved `.rs` entry (and any non-`.rs` entry that
   parses as Rust — this catches `include!` of code through a non-`.rs`
   name) is parsed with `syn`; entries declaring at least one function are
   emitted to the production set; declarations-only files (`src/model.rs`)
   and unparsable data (`src/static/*`) are exempt. A `.rs` entry that
   fails to parse refuses loudly (rustc parsed it; syn must too — remedy:
   helper lock bump).

The emitted set is written to `$run_dir/production-files.txt` (captured
evidence). The set check is a sorted `comm -23` of that file against the
`SF:` paths of `$run_dir/lcov.info` normalized by stripping the workspace
prefix; any remainder refuses, naming the files and the disease.

## The expansion attribute scan

- Commands, run with the gate's resolved toolchain and target dir, after
  the merge:
  `cargo +<tc> rustc --lib --locked --offline -- -Zunpretty=expanded >
  $run_dir/expanded-lib.rs` and the same for `--bin swarmail` →
  `expanded-bin.rs`. Each is one compile, no codegen (~30 s for the lib on
  this host); a missing or empty output refuses.
- The helper parses each expansion with `syn`, walks every item (modules,
  impls, traits, nested fns) and refuses naming any `coverage(off)` or
  `no_coverage` attribute **outside `#[automatically_derived]` impl
  blocks** — the carve-out is by the attribute's presence on the impl, so
  all five minted `assert_fields_are_eq` fns pass and any user-written
  exclusion refuses, whatever item it sits on.
- The expansion/build cfg coupling is stated at the call site: same
  toolchain, same target dir, no features in the manifest; a mismatch
  refuses loudly.

## Ordering and evidence

```
export CARGO_LLVM_COV_TARGET_DIR   # unchanged, before clean (staleness contract)
cargo +tc llvm-cov clean --workspace
cargo +tc llvm-cov … --output-path $run/lcov.info
validate_report        # exists, non-empty, has SF: — else refuse naming it
harvest + capture      # helper `files`; copy the .d files and the list into $run
expansions             # helper `attrs` inputs, written into $run
check_no_harness_leak <report> <production-files>   # rescoped (ruling 3)
check_report_covers_production_files <report> <production-files>
check_no_coverage_exclusions <expansions…>          # helper `attrs`
count_dimensions → jq summary → exact-equality verdict
```

The checks read only the captured copies; the EXIT trap removes only a
gate-owned build dir. With an explicit `CARGO_LLVM_COV_TARGET_DIR` shared
by two runs, a concurrent `clean` can remove this run's dep-info before the
harvest — the harvest is the first step after the run and a missing `.d`
refuses, naming the path and the shared-directory contract, next to the
staleness sentence.

## Selftest arms (added; every existing arm unchanged)

- **Real-build red arms:** one zero-dependency bypass fixture crate
  (plain fn; direct `#[coverage(off)]` fn; macro-emitted `#[coverage(off)]`
  fn; `#[cfg_attr(all(), coverage(/**/ off ))]` fn; `include!`ed
  `inc_tests.rs`; `#[path]`-loaded `path_tests.rs`) built and reported
  under the selftest root: `cargo +<tc> build --lib` (its real `.d`),
  `cargo +<tc> llvm-cov --lcov` (its real report), `cargo +<tc> rustc
  --lib -- -Zunpretty=expanded` (its real expansion). Stage 1 must refuse
  naming `inc_tests.rs`/`path_tests.rs`; stage 2 must refuse naming the
  macro-emitted and `cfg_attr` functions. **Green twin:** the same shapes
  minus the bypasses, plus a declarations-only file, a
  `derive(PartialEq, Eq)` struct and a const fn — stage 1 and stage 2 must
  pass (one green fixture proves the fn probe, the carve-out and const-fn
  instrumentation at once).
- **Fast arms:** the rescoped leak check (harness-named `SF:` inside the
  production set → pass; outside → refuse); the hostile-entry refusal (the
  real fixture `.d` with one entry rewritten to a nonexistent path →
  refuse naming it); validate-before-consume (an empty report refuses
  naming it).
- The helper is built once per invocation into the shared gitignored
  `target/coverage/fn-scan-target/` (cargo-fingerprinted; never part of the
  measured crate; concurrent runs serialize on cargo's lock).
- Toolchain resolution is factored so the selftest can resolve the same
  nightly the gate resolves (the real-build arms need it).

## Gates

All seven house gates, run as written; `bash
scripts/coverage-gate.sh` with no `TMPDIR` override; the selftest run
twice (it is now ~30 s, and its own fixtures must leave no `/tmp` litter —
the 12f no-litter check applies).

## Risks

| Risk | Mitigation |
|---|---|
| **The gate's line budget grows past the acceptance** (real-build arms are verbose). | Budget stated: ≤ 746. The arms share one fixture pair; fixture generation uses `printf` heredocs, not a generator script; if the arms overflow the budget, the fallback is stage 1 + the existing grep (documented in the spec's ruling 2 cost), reported honestly rather than reached by deleting main's checks. |
| **The expansion pass rebuilds the lib in the build dir and its `.d` clobbers the harvested one.** | Harvest and copy the `.d` files *before* the expansion pass (capture ordering above); the expansion's own `.d` is equivalent anyway (same file set), so even a clobber is harmless — the ordering makes it impossible. |
| **A concurrent run sharing an explicit `CARGO_LLVM_COV_TARGET_DIR` removes this run's dep-info.** | Capture first; a missing `.d` refuses, naming the path and the contract (the pre-existing "caller owns uniqueness" sentence extended verbatim). |
| **syn cannot parse a future nightly form** in a source file or the expansion. | Fail closed: refuse naming the file; remedy is a helper lock bump (syn is already in the measured lock's version family). |
| **cargo-llvm-cov changes its default ignore regex** and stops dropping harness-named files. | Then bypass files appear in the report *measured* — stage 1 passes (by design, the commission's end state) and the rescoped leak check stays silent for production files, refusing only harness-named test-side leaks. The gate's behavior tracks the tool instead of contradicting it. |
| **The selftest's real builds slow the contract** (~30 s) or break hermetically. | Zero-dependency fixtures, `--offline`, `--locked`; the selftest root is trap-removed; the helper target dir is shared and cached. If the seat lacks the toolchain, the real-build arms refuse with a named message (fail-closed, as the toolchain refusal already does). |
| **A reviewer reads stage 2 as a spelling recognizer again.** | It runs on the compiler's canonical output, where the spelling space is exactly one (spec fact 6); the spec carries the equivalence argument and the forged-impl residual. No source text is parsed by bash. |
| **Decision 0035 rejected by the operator.** | The batch lands with it `proposed`; the leak check's rescoping is inert on today's tool (harness-named records cannot appear), so rejecting the decision requires only re-scoping that one check — the by-effect refusals stand on their own. |
