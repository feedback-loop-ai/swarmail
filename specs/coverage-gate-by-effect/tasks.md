# Tasks — run 12g, feature `coverage-gate-by-effect`

Ordered; each task names the test that proves it. Every task keeps the
seven house gates green; the gate-specific proofs run through
`bash scripts/coverage-gate.sh selftest` (fail-closed) and the full gate.

- [ ] **1. Land the helper `tools/coverage-fn-scan/`** — its own
      `[workspace]` table, `syn = { version = "2.0.119", features = ["full"]
      }`, a committed `Cargo.lock` pinning syn 2.0.119 + proc-macro2 +
      unicode-ident, and `src/main.rs` with the two subcommands: `files`
      (dep-info parse → normalization → hostile-entry refusal → fn probe →
      emit the fn-bearing production set) and `attrs` (expansion parse →
      refuse `coverage(off)`/`no_coverage` outside
      `#[automatically_derived]` impl blocks, refuse empty/unparseable
      input).
      *Test:* `cargo build --locked --offline` succeeds with the realm's
      cargo home and no network; unit-style invocations on the real tree's
      artifacts emit exactly the 18 fn-bearing files (17 lib + `main.rs`)
      and exit 0 on the real expansion (the five minted fns carved); a
      hostile `.d` (one entry rewritten to a nonexistent path) exits
      nonzero naming it.
- [ ] **2. Add `validate_report` to the gate** — the report must exist, be
      non-empty and carry `SF:` records before any check consumes it.
      *Test:* a selftest arm feeds an empty file and a `SF:`-less file to
      the validator and expects a refusal naming the artifact; a real
      `lcov.info` passes.
- [ ] **3. Add `check_report_covers_production_files`** — harvest the
      production units' dep-info from the gate's own build dir (lib via its
      `.rlib` rule, bins via extension-less executable rules whose
      prerequisites name the bin root and not the lib root; unit roots from
      `cargo metadata --no-deps`), capture the harvested `.d` files and the
      emitted list into the run dir, then refuse naming every emitted file
      absent from the report's normalized `SF:` set (sorted `comm -23`).
      *Test:* the selftest's real-build red arm (bypass fixture) refuses
      naming `inc_tests.rs`/`path_tests.rs`; its green twin (declarations-only
      file + derive(PartialEq, Eq) + const fn, no bypasses) passes.
- [ ] **4. Add `check_no_coverage_exclusions`** — expand the production lib
      and bin with the gate's resolved toolchain into the run dir
      (`cargo +<tc> rustc --lib|--bin swarmail --locked --offline --
      -Zunpretty=expanded`, after the merge), capture the dumps, and refuse
      on the helper's `attrs` verdict.
      *Test:* the selftest's real-build red arm refuses naming the
      macro-emitted and the `cfg_attr(all(), coverage(/**/ off ))`
      functions; the green twin passes (the minted `assert_fields_are_eq`
      carved, the const fn silent); an empty expansion output refuses
      naming the artifact.
- [ ] **5. Rescope `check_no_harness_leak` by membership** (ruling 3) — keep
      the name and the harness regex; add the production-set argument; a
      harness-named `SF:` record whose file is absent from the set refuses,
      one whose file is present passes. Decision 0035 records the amendment.
      *Test:* selftest arms — `SF:…/src/tests/leaked.rs` with a production
      set lacking it refuses (names the record); `SF:…/src/inc_tests.rs`
      with `src/inc_tests.rs` in the production set passes.
- [ ] **6. Wire the ordering**: validate → harvest+capture → expansions →
      the three checks → `count_dimensions` → verdict; the checks read only
      captured copies; the EXIT trap still removes only a gate-owned build
      dir; the harvest runs before the expansion pass so the expansion's
      own `.d` cannot clobber the captured one.
      *Test:* the full gate on the real tree passes at exact
      100/100/100 with the new checks silent (acceptance 4); a
      missing-dep-info refusal arm (harvest pointed at an empty dir)
      refuses naming the path and the shared-directory contract.
- [ ] **7. Grow the selftest with the real-build arms** — generate the
      bypass fixture and its green twin under the selftest root (zero
      dependencies), build + report + expand each with `--offline --locked`,
      invoke the new check functions against the real artifacts, and factor
      toolchain resolution so the arms resolve the same nightly the gate
      does. Keep every existing arm unchanged; the helper builds into the
      shared `target/coverage/fn-scan-target/`.
      *Test:* `bash scripts/coverage-gate.sh selftest` passes with the new
      red arms refusing and the green twins passing (acceptance 1–3), twice
      in a row (no `/tmp` litter, selftest root trap-removed).
- [ ] **8. Land decision 0035** at
      `docs/decisions/0035-coverage-gate-refuses-by-effect.md`, status
      `proposed`: the by-effect contract (inventory at ingest, fn-bearing
      file set, the canonical-expansion attribute scan, the rejected
      record-join) and the amendment of 0020 §3's leak-check sentence;
      `docs/decisions/0020-…` itself is not edited.
      *Test:* the file exists, is non-empty, carries `status: proposed`,
      and the spec references it by relative path; the validator's
      decision check passes.
- [ ] **9. Run the full contract** — `cargo fmt --all -- --check`,
      `cargo clippy --all-targets -- -D warnings`, `cargo test`,
      `brokkr compile --bundle bundles/self`, `bash
      scripts/coverage-gate.sh selftest`, `bash scripts/coverage-gate.sh`
      (no `TMPDIR` override), `cargo deny check licenses`; confirm the
      gate's net line count is ≤ 746 and contains no Rust lexing; confirm
      the measured manifest and `Cargo.lock` are untouched
      (`git diff --stat` shows only the gate, the helper, the decision and
      the spec files).
      *Test:* every gate exits 0; `wc -l scripts/coverage-gate.sh` ≤ 746;
      the no-`/tmp`-litter count (12f's check) shows no growth across a
      gate run plus a selftest run.
- [ ] **10. Commit exactly the batch** — GPG-signed, imperative message,
      body says why: `specs/coverage-gate-by-effect/{spec,plan,tasks}.md`,
      `docs/decisions/0035-coverage-gate-refuses-by-effect.md` and the
      implementation of tasks 1–7 as it lands. The position files stay
      uncommitted (`.brokkr/` is gitignored). If signing fails, stop and
      report; do not commit unsigned.
      *Test:* `git log --show-signature -1` shows the batch commit; `git
      status --short` shows no stray staged files.
