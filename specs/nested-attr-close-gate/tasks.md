# Tasks — nested-attr-close-gate

Ordered; the run's implement seat executes them in sequence. Each task names
the test that proves it. Base: `05ed2b8`, tree pristine
(`git status --short` empty before any gate run). Test commands assume
`export PATH="$HOME/.cargo/bin:$PATH"`.

- [ ] **T1 — Add the three §5k arms with the repair NOT yet applied.** In
  `scripts/coverage-gate.sh` `selftest()`, after the last §5i arm (before the
  `# 6. Canonical counting` block), add `co-nested-attr-harness` (declared
  harness + hidden `store/persist_tests.rs` +
  `#[cfg_attr(all(), cfg_attr(all(), path = "store/persist_tests.rs"))] pub
  mod alias;` — refuse), `co-nested-attr-escape` (same nesting, target
  `store/persist_\x74ests.rs` — refuse), and `co-nested-attr-clean` (same
  nesting, target `store/ordinary.rs`, ordinary file present, hidden harness
  file still in the tree, honest reservation — keep), each mirroring the §5j
  fixture shape (`src/alias.rs`, `src/store/persist_tests.rs`).
  **Proved by:** `bash scripts/coverage-gate.sh selftest` — exactly the two
  refuse arms red (`got "pass", want "refuse"`: the shapes launder at the
  unrepaired walk), the keep arm green, every pre-existing arm unchanged.
  Capture this output verbatim for the landing notes (the red proof).
- [ ] **T2 — Apply the one-line repair and true the comment.** Replace the
  two-disjunct close test at :391 with `if (tail !~ /^[\001)]*\]/) {`; amend
  the comment above it (:379–384) to state the accepted close language (any
  run of `)` and separators before `]`, any `cfg_attr` nesting depth; zero
  closers allowed, so the bare-`]` case is subsumed; assignments whose
  post-quote tail is not such a run keep passing). Touch nothing else in the
  walk — the `rest` resumption points and the `include!` arm stay verbatim.
  **Proved by:** `bash scripts/coverage-gate.sh selftest` — `selftest pass`,
  every arm green including the three new ones; the framing's keep pins
  (`let path = "x";`, `let path = dir.path().join("persist.db");`) green via
  the battery's existing arms.
- [ ] **T3 — Fold the subordinate reader-trap fix.** Rename the second
  `co-split-include` fixture use (:886) to `co-inc-line-split` so the two §5i
  fixtures no longer share a name. **Proved by:** a clean re-run of
  `bash scripts/coverage-gate.sh selftest` after the rename — arm count and
  outcomes identical to T2's.
- [ ] **T4 — Prove the six house gates at the repaired tree, from pristine
  status.** `cargo fmt --all -- --check`; `cargo clippy --all-targets -- -D
  warnings`; `cargo test`; `brokkr compile --bundle bundles/self`;
  `TMPDIR=/home/valentin/scratch/cov-tmp bash scripts/coverage-gate.sh`;
  `cargo deny check licenses`. **Proved by:** all six exit 0; the coverage
  gate reports **exact 100/100/100, missed == 0** with the same counts as the
  base run (no Rust changed — the measured denominator must be identical);
  `git status --short` clean immediately before the runs.
- [ ] **T5 — Record the run and land.** Landing notes carry: the T1 red-proof
  output; the green-probe outcomes with their stated results —
  `coverage(off)` via `cfg_attr` **refuses** (deleted-stream asymmetry), a
  literal 0x01 plus attr-shaped text in an **ordinary** string **passes
  silently** (escaped quote blocks the match), the same shape in a **raw**
  string **refuses loudly**, a `concat!` path RHS is refused **by rustc**
  ("malformed `path` attribute input"), `)))]` and marker-glued
  `) \001 )]` tails are closed by the same class (mawk-confirmed); the
  pristine-tree statement; the unsigned-commit disclosure. Land the batch on
  the run's landing branch per the usual path; commit message cites #101 and
  #102; close no issue; push nothing. **Proved by:** the landing notes exist
  with the red proof and every probe outcome above; the landing commit's diff
  contains only `scripts/coverage-gate.sh` (T1–T3's regions); the spec's
  Acceptance Criteria 1–8 each map to a recorded result.
