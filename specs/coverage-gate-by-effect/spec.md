# Spec — the gate refuses declaration bypasses by effect: the build's own file set and the compiler's canonical expansion (issue #101, second half)

- **Run:** run 12g (chief architect, synthesizing the simplicity and
  robustness positions; run-local evidence:
  `.brokkr/state/design/positions/simplicity.md`,
  `.brokkr/state/design/positions/robustness.md`, both uncommitted) ·
  **Feature slug:** `coverage-gate-by-effect`
- **Issue:** [#101](https://github.com/feedback-loop-ai/swarmail/issues/101),
  second half — declaration bypasses: a harness-named file compiled into the
  production crate is dropped from the coverage report *by name*
  (cargo-llvm-cov's default ignore regex), and a `coverage(off)` in *any*
  spelling — including one emitted by a macro or resolved through a
  `cfg_attr` chain — removes a function's record while the function keeps
  running in production. Fixtures A–G each passed the gate. #50 asked for
  this to be closed structurally. (First half — the gate off the host's
  `/tmp` — landed in run 12f.)
- **Decisions:** **0035 minted, `proposed`**
  ([docs/decisions/0035-coverage-gate-refuses-by-effect.md](../../docs/decisions/0035-coverage-gate-refuses-by-effect.md))
  — it amends 0020 §3's sentence about the leak check's refusal direction
  and records the by-effect contract; acceptance is the operator's. 0020's
  test-placement rules, 0005's extraction-at-ingest transposition and every
  frozen guarantee are applied as written, not weakened.
- **Base:** `745d6ad`; `scripts/coverage-gate.sh` is 606 lines and contains
  no source lexer (runs 12–12e's recognizer died; 12f restored main's
  declaration checks and marked `stripper-prove-or-refuse`,
  `load-shape-token-stream` and `nested-attr-close-gate` superseded).
- **Shape:** two new refusals in the gate, both over artifacts the gate's
  own run already produces (its instrumented build's dep-info, its own
  expansion dump), plus one small parser helper outside the measured crate.
  No production code under `src/` changes; the frozen guarantees are
  untouched (the gate refuses more, it never counts differently); the
  counting contract (`count_dimensions`) stays byte-identical.

## The problem

A/B/C/D/G are one disease: **a harness-named file is compiled into the
production lib or bin while cargo-llvm-cov drops it from the report by
name.** The loading spelling — `#[path]`, unguarded `mod`, block-comment
guard, `include!`, a `pub mod` split across lines, a cfg-guard trick — is
irrelevant; the *effect* is identical, and the exact floor never sees lines
that run in production. E is the other disease: **a function is compiled but
excluded from instrumentation**, invisible to any source-side check when the
attribute arrives through a macro or a resolved `cfg_attr`.

Runs 12–12e answered with a Rust lexer in bash (482 → 1176 lines) and every
review found one more spelling, because a source scanner cannot prove the
absence of something macros, `cfg_attr` chains and `include!` can produce.
The operator ruled the design itself was the problem. The only exit is to
stop reading source and start reading *what the build says it did*.

## The facts this design stands on (all re-verified this session, against the real tree and the real toolchain)

1. **The lib's dep-info is the total file inventory of the production
   build.** The `.d` cargo writes for the lib unit lists every file rustc
   read into it, however loaded: for this tree, the 18 `src/*.rs` files
   (including `src/model.rs`) plus the three `include_str!` assets
   (`src/static/*`) — and *not* `src/main.rs` (the bin is its own unit) and
   *not* `src/store/persist_tests.rs` (0020-conformant, compiled only under
   `--cfg test`). The bin's `.d` lists `src/main.rs`.
2. **cargo-llvm-cov drops harness-named files from the report by name**
   (the panel's fixture: one `SF:` record for `lib.rs`; the `include!`ed and
   `#[path]`-loaded files compiled, running, absent).
3. **A literal "every dep-info `.rs` must have an `SF:` record" check
   refuses the real tree — this is the defect in both positions' stage 1.**
   The report's `SF:` set is 18 files: the 17 fn-bearing lib files plus
   `src/main.rs`. It does **not** carry `src/model.rs`, which is
   `pub mod model;` containing only structs and derives — zero functions —
   and llvm-cov emits no `SF:` record for a file with nothing countable.
   Neither position ran this set difference on the real tree; the chief did.
   The check must therefore demand records for the files that *declare
   functions*, not for every file the build read.
4. **Both units write `deps/swarmail-<hash>.d` — there is no
   `libswarmail-*.d`** on this toolchain, so the lib's dep-info cannot be
   picked by filename. It is picked structurally: the `.d` carries a rule
   whose target is `libswarmail-<hash>.rlib` (the lib) and the bin's carries
   a rule whose target is the extension-less `swarmail-<hash>` executable;
   the lib's *test harness* writes the same executable-shaped `.d` and is
   excluded because its prerequisite list contains the lib root
   (`src/lib.rs`). Each `.d` also contains one no-prerequisite rule per
   source file and `# env-dep:` comment lines; the parser handles the real
   format, not a simplification of it.
5. **An excluded function has no `FN:` record**; a coverage record's start
   line is the line of the `fn` item; const-only functions *are*
   instrumented (no false-positive class there).
6. **`-Zunpretty=expanded` canonicalizes every spelling into exactly one.**
   The expansion of a fixture carrying a direct attribute, a
   macro-emitted attribute and `#[cfg_attr(all(), coverage(/**/ off ))]`
   prints `#[coverage(off)]` for all three; comments do not survive the
   parser. The real tree's expansion (15,446 lines, ~33 s — one lib compile,
   no codegen) carries **exactly five** `coverage(off)` attributes, all on
   `fn assert_fields_are_eq` inside `#[automatically_derived] impl
   ::core::cmp::Eq for …` blocks — compiler-minted by `derive(Eq)`, and
   *deliberately* uninstrumented. There is no `no_coverage` anywhere. Any
   check that does not carve the minted impls out refuses main.
7. **The alternative inventories fail on contact.** rustdoc JSON points
   macro-generated fns at the call site while coverage records use the def
   site — a line join false-refuses every macro instantiation, a name join
   false-passes collisions. No `llvm-dwarfdump` ships in llvm-tools and
   rlibs carry no subprogram DWARF; `nm` set-differencing drowns in
   closures, generics and shims. Plain `syn` over the *unexpanded* source is
   blind to macro-generated attributes — and redundant with main's grep for
   every literal spelling.

## The ruling — synthesis of the panel (adopted, rejected and reconciled, explicitly)

**Adopted from robustness — the through-line.** The inventory is extracted
at ingest — build time — and the checks are queries over the extracted
record (0005 transposed). A check that re-derives its inventory from source
at check time is the design error the last four runs made. Concretely
adopted: the dep-info-vs-report set difference; `syn` over the *canonical
expansion* (never a regex) for functions; one helper bin outside the
measured crate; capture-before-trap (the checks read copies preserved in the
run dir); validate-before-consume (a truncated or empty report refuses
before any check reads it); a green twin per red arm; and "an unresolvable
dep-info entry refuses, never skips".

**Ruling 1 — the file-set refusal is over fn-bearing entries, not all
`.rs`.** Both positions wrote "every dep-info `.rs` entry must appear in the
report"; fact 3 makes that refuse main (`src/model.rs`). The producer of the
inventory parses each entry and demands a record only for files that
*declare at least one function* — by effect, not by a name list. A
declarations-only file has no lines any dimension could count; demanding a
record for it is demanding the report lie. Cost accepted: a file whose every
function sits behind an unresolved `cfg` is refused loudly (the tree is
lying to itself about needing it — robustness's own class 2).

**Ruling 2 — stage 2 is the attribute scan on the canonical expansion
(simplicity); the record-join at function granularity (robustness) is
rejected.** Three reasons. (a) The record-join refuses main *today*: the
five minted `assert_fields_are_eq` fns have no records (fact 6), and
robustness's design carries no carve-out. (b) Its extra refusal classes —
uninstantiated generics, cfg'd-out files, dead macro arms, functions the
backend eliminated — make "the real tree passes" a standing wager against
every future honest tree, and reshaping the tree to satisfy a gate exceeds
the commission, whose acceptance is fixture-refusal plus real-tree-passes.
(c) For the bypass class the commission names — `coverage(off)` in any
spelling, from any macro — the attribute scan is *exactly equivalent* to the
record-join, because the attribute is rustc's only per-function exclusion
mechanism on this toolchain (fact 6): refusing the attribute on the
compiler's own canonical output is refusing the missing record, computed
where the computation is sound. **Cost of the rejection, named:**
uninstantiated generics stay invisible to the gate — and to the exact floor,
which cannot see them either; if the operator ever wants that stricter
check, it is a decision document, not an edit.

**Ruling 3 — `check_no_harness_leak` is rescoped by membership; neither
"keep byte-identical" (simplicity) nor "delete" (robustness).** The
commission is explicit: *"a harness-name exclusion may only drop files that
are NOT in the production build's file set. A harness-named file loaded into
production is then measured, not excluded."* A name-based presence-refusal
contradicts that sentence — under a future cargo-llvm-cov that measures
harness-named files, the gate would refuse the measured state the commission
demands. The check becomes: a harness-named `SF:` record whose file is *not*
in the production build's fn-bearing set refuses (test harness measured as
production — the original disease); one whose file *is* in the set passes
(production measured — the desired end state). Today's tool drops such
records always, so the check is a tripwire against tool drift, now pointing
the commission's way. **This amends 0020 §3's sentence that the leak check
"fires on presence" — the amendment lands as decision 0035, `proposed`.**
Cost: main's check changes shape (it takes the production set as a second
argument) and its selftest arms gain the membership dimension.

**Ruling 4 — `check_no_coverage_off` (the grep) stays byte-identical.** It
is the only refusal that fires *before* the multi-GiB instrumented build
(fast feedback in the common case), it is 12f's landed contract, and the
expansion check is its *generalization*, not its replacement: the grep
refuses every literal spelling (including inside macro bodies) pre-build;
the expansion check adds every spelling the compiler can produce. Robustness
declined. Cost of keeping: a doc comment or string in `src/` containing the
literal token refuses — loud, named, rephrase-the-comment; that is the
landed species 0020 already endorses ("a false positive is loud and names
the file").

**Ruling 5 — both stages cover the lib *and* the bin.** The bin is measured
by the report (`SF:…/src/main.rs` is in every real report), its dep-info is
harvestable by the structural rule of fact 4, and a harness-named load into
`main.rs` is otherwise a live bypass. Simplicity's "bin residual" is closed;
robustness's lib-and-bin harvest is adopted with a selection rule derived
from fact 4 rather than from filenames alone.

**Ruling 6 — `check_harness_declarations` (0020 rule 6) and
`check_no_hidden_harness_dirs` stay byte-identical.** The tripwire guards a
decision-backed *placement* discipline; the hidden-dir check refuses
*parked* source — text dep-info never sees, because dep-info knows only what
was compiled. Robustness's "delete what the dep-info rule dominates" is
declined: the dep-info rule dominates neither. Cost: none (both are main's,
both are cheap, both fail closed).

## The design

**Unchanged, byte-identical:** the `COVERAGE_FLOOR` refusal,
`check_no_coverage_off`, `check_no_hidden_harness_dirs`,
`check_harness_declarations`, the free-space preflight, build-dir
resolution, toolchain resolution, `count_dimensions`, the jq summary and the
exact-equality verdict. `check_no_harness_leak` keeps its name and its
regex; only its verdict gains the membership condition (ruling 3).

**Added — refusal stage 1, files by effect
(`check_report_covers_production_files`).** After the instrumented run, the
helper harvests the production units' dep-info from the gate's *own* build
dir — the lib via its `.rlib` rule, the bin via its executable rule whose
prerequisites name the bin root and not the lib root (fact 4) — normalizes
every entry against the workspace root, refuses any entry that does not
resolve to an existing file under the workspace (never skips), and emits the
**fn-bearing** subset (ruling 1): entries that parse as Rust and declare at
least one function. Non-Rust entries (`include_str!` assets, fact 5) and
declarations-only files (`src/model.rs`, fact 3) are exempt, named as such.
The gate then refuses naming every emitted file with no `SF:` record in
`$run_dir/lcov.info` — a sorted `comm`. This closes A, B, C, D and G **by
construction**: each shape compiles the harness-named file into a production
unit, the unit's own dep-info testifies to it, and the report — which
dropped it by name — cannot carry it. cfg-guard tricks close themselves: the
build evaluated the cfg, and whatever it compiled is in the dep-info.

**Added — refusal stage 2, functions by effect
(`check_no_coverage_exclusions`).** The gate expands the production lib and
bin with the coverage toolchain it already resolved
(`cargo +<tc> rustc --lib|--bin swarmail --locked --offline --
-Zunpretty=expanded`, same target dir, same cfg set) into the run dir, and
the helper parses each expansion with `syn`, walking every item and refusing
to name any attribute `coverage(off)` (and its deprecated precursor
`no_coverage`) **outside `#[automatically_derived]` impl blocks** — which
covers the attribute on functions, modules and impls alike, at any nesting.
An empty or unparseable expansion refuses (a cfg mismatch refuses loudly,
never silently). This closes fixture E: direct spellings, comment-hidden
tokens, whitespace padding, `cfg_attr` resolutions and macro emissions all
canonicalize to the one spelling the expansion can contain (fact 6) — and
main's grep already refuses the literal ones pre-build, so the helper is the
generalization.

> **Implementation amendment (implement seat, against the real toolchain):**
> no syn parses the real tree's canonical dump — `std::pin::pin!`
> (`src/api.rs:521`) mints the experimental `super let` (decision 0035's
> third context fact; verified for syn 2.0.119 and 3.0.9). Stage 2 walks the
> dump's **token tree** (`proc-macro2`) instead of a syn parse; the fn probe
> keeps syn over source files, which parse. The carve-out and the
> exactly-one-spelling argument are unchanged.

**Added — the helper (`tools/coverage-fn-scan/`, ~200 lines).** A standalone
crate *outside the measured crate* (its own `[workspace]` table — the
standard xtask pattern; the measured manifest gains nothing) with
subcommands `files` (dep-info harvest + fn probe + normalization) and
`attrs` (expansion attribute scan). It depends on `syn` 2.0.119 (+ its
`proc-macro2`/`unicode-ident` tree) — already in the measured crate's
`Cargo.lock` via `serde_derive` and cached in the realm's cargo home, so
the hermetic build needs no network and licenses are already vetted by the
existing `cargo deny check licenses` over the same versions (MIT OR
Apache-2.0). Its own `Cargo.lock` is committed, pinning those versions.
One sentence of defense for the dependency: *a real parser, because the
last four runs prove bash cannot be one.*

**Ordering is part of the design (capture before trap, validate before
consume).** export → `llvm-cov clean --workspace` → instrumented build +
report → **validate the report** (exists, non-empty, carries `SF:` records —
else refuse with a named message, because every new check is an absence
check and a truncated export vacuously passes all of them) → **harvest and
capture** (copy the harvested `.d` files, the fn-bearing file list and both
expansions into the preserved `run.XXXXXX` dir; the checks read copies, so
evidence outlives the build dir the trap removes) → **expansions** (after
the merge, so their differing rustflags cannot poison the measured build's
fingerprints) → the two new checks and the rescoped leak check →
`count_dimensions` → verdict. With an explicit `CARGO_LLVM_COV_TARGET_DIR`
shared by two runs, a concurrent `clean` can remove this run's dep-info
between its build and its harvest; the harvest is the first step after the
run and a missing `.d` refuses, naming the path and the contract — the same
window the staleness sentence already governs, extended verbatim.

**Selftest.** `coverage-gate.sh selftest` keeps every existing arm unchanged
and adds: one zero-dependency bypass fixture crate and its green twin — the
same shapes minus the bypasses — each *really built* under the selftest root
(`cargo +<tc> build`, `cargo +<tc> llvm-cov`, `cargo +<tc> rustc
-Zunpretty=expanded`, all `--offline`); the red arms invoke the two new
checks against the bypass fixture's real `.d`, real `lcov.info` and real
expansion and must refuse naming the bypassed names; the green arms invoke
them against the twin and must pass — the twin carries a declarations-only
file (the `model.rs` shape), a `derive(PartialEq, Eq)` struct (the
carve-out) and a const fn, so one green fixture proves three exemptions.
Fast arms cover the rescoped leak check (harness-named `SF:` in and out of
the production set) and the hostile-entry refusal (a copy of the real
fixture `.d` with one entry pointed at a nonexistent path). The selftest
grows from milliseconds to roughly half a minute; it is part of the
contract and fail-closed as before.

## Acceptance Criteria

1. **Fixtures A–G refused by effect.** Each of #101's fixtures A, B, C, D
   and G (harness-named loads: `#[path]`, `include!`, cfg-guard, split
   `pub mod`) is refused by `check_report_covers_production_files`, and
   fixture E (excluded function, in every spelling) is refused by
   `check_no_coverage_exclusions` — each proven by a selftest arm built from
   a **real cargo build** of a fixture crate, never from text fixtures.
2. **A `macro_rules!` that emits `#[coverage(off)]` is refused** — the
   bypass fixture's expansion carries the macro-emitted attribute and the
   expansion check names its function.
3. **Every red arm has a green twin** — the same fixture minus the bypass —
   that must pass, so a check "fixed" into refusing everything cannot pass
   the suite.
4. **The real tree passes.** `bash scripts/coverage-gate.sh` (no `TMPDIR`
   override) exits 0 at exact 100/100/100 with the new checks silent —
   `src/model.rs` and the `include_str!` assets exempt, the five minted
   `assert_fields_are_eq` fns carved, `src/main.rs` covered via the bin
   harvest.
5. **The leak check points the commission's way:** a harness-named `SF:`
   record whose file is absent from the production set refuses; one whose
   file is present passes. Both arms in the selftest.
6. **No Rust lexing in bash.** No source text of Rust is parsed by
   `scripts/coverage-gate.sh`; the gate's net line count is at or below
   main's 606 plus the new checks' own weight (budget stated in
   `plan.md` — estimated 746, landed 788 with the reason recorded there);
   no resurrected stripper, raw-string reader or token-stream recognizer.
7. **The helper builds hermetically and stays outside the measured crate:**
   `cargo build --locked --offline` in `tools/coverage-fn-scan/` succeeds
   with the realm's cargo home and no network; the measured manifest and
   lock are untouched; syn 2.0.119 (fn probe) and proc-macro2 1.0.107
   (token walk) are already vetted by `cargo deny check licenses` over the
   same versions.
8. **Hostile dep-info refuses:** an entry that does not resolve to an
   existing file under the workspace refuses, naming it — it never skips.
9. **Absence is never a pass:** a missing, empty, truncated or unparseable
   report, a missing dep-info, or an empty/unparseable expansion each
   refuses with a message naming the artifact.
10. **All gates green:** `cargo fmt --all -- --check`, `cargo clippy
    --all-targets -- -D warnings`, `cargo test`, `brokkr compile --bundle
    bundles/self`, `bash scripts/coverage-gate.sh selftest`,
    `bash scripts/coverage-gate.sh`, `cargo deny check licenses`.
11. **Decision 0035 lands `proposed`** at
    `docs/decisions/0035-coverage-gate-refuses-by-effect.md`, amending 0020
    §3's leak-check sentence and recording the by-effect contract; 0020's
    file is not edited. Issues are closed by the operator, not by this
    batch.

## Residuals — named, accepted, each narrower than what main misses today

1. **The forged-impl residual.** A `macro_rules!` body can carry a *forged*
   `#[automatically_derived]` impl wrapping a comment-split
   `#[coverage(/**/off)]` fn; both the grep (comment-split) and stage 2
   (carved out) miss it. Closing it requires distinguishing compiler-minted
   impls from forged ones — derive-provenance analysis, the 1176-line lexer
   reborn in Rust. It requires deliberate laundering by someone who has read
   the gate; the next increment is a decision, not another recognizer.
2. **Test-cfg-only emissions.** The expansions resolve the production cfg
   set; a macro-emitted exclusion under `#[cfg(test)]`-only code is not
   scanned. Such code is not in the shipped crate, and the grep still
   catches literal spellings there.
3. **Uninstantiated generics and dead code stay invisible** (ruling 2's
   cost) — they are invisible to the exact floor too, and carry no bypass.
4. **A future non-attribute exclusion mechanism** (a rustc that drops
   functions from instrumentation by some other means) evades stage 2 — and
   every source-side check ever proposed; the leak check and review face it
   then.
5. **A `.rs` file `syn` cannot parse** (a future nightly syntax) refuses
   loudly, naming the file; the remedy is a helper lock bump.
6. **Expansion/build cfg coupling.** The expansion must resolve the same
   cfgs as the instrumented build: same toolchain, same target dir, and the
   manifest declares no features. If features or `--cfg`s are ever added to
   the instrumented build, the expansion step must follow them — stated at
   the call site; a mismatch refuses loudly (empty/unparseable expansion or
   an unexpected attribute), never silently.

## Decision

[docs/decisions/0035-coverage-gate-refuses-by-effect.md](../../docs/decisions/0035-coverage-gate-refuses-by-effect.md)
— status `proposed`, acceptance the operator's. It records the by-effect
contract (inventory at ingest; fn-bearing file set; the canonical-expansion
attribute scan; the rejected record-join) and amends 0020 §3's sentence on
the leak check's refusal direction. (Numbered 0035 because the delivery
dialect cites 0034 for result sealing; the repo's free numbers resume after
it.)
