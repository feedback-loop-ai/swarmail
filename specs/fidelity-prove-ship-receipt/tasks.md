# Tasks — the prove-ship receipt at `791f83d`

Ordered for this pass and the phases that follow. Every task pairs with
the check or test that proves it; a task that cannot be proved is not a
task. This pass runs no production code; the proving tests are the ones
the repaired tree already carries, plus deterministic inspections of the
committed record. Run at the repository root with `.brokkr/state/env.sh`
sourced.

## This pass — the receipt, the record truth, the spine

- [ ] 1. Verify the tree first-hand before minting anything: the suite
  count, the pin, the shape claims.
  **Proved by:** `cargo test --workspace --locked` — **470 passed, 0
  failed, 1 ignored** (`tests/rate.rs`'s opt-in smoke);
  `cargo test --lib extract inline_adjacent_codes_behave_as_the_renderer_renders`
  — green, an output assertion with no duration;
  `grep -n 'REF_MAX' src/extract.rs` (one named local `const`),
  `grep -rn 'html_to_text' src/ --include=*.rs` (exactly three call
  sites: `src/compat.rs:560`, `:801`, `src/extract.rs:155`),
  `grep '^percent-encoding' Cargo.toml` (spec.md AC-T.1–T.3).
- [ ] 2. Verify the two-servers hole and the 0001 proof-family bill it
  prices: no `locking_mode` anywhere in the store, and the second-
  connection machinery the EXCLUSIVE alternative would break.
  **Proved by:** `grep -n 'locking_mode' src/persist.rs` (empty);
  `grep -n 'hold_write_lock' tests/common/mod.rs tests/lifecycle.rs`
  (`BEGIN IMMEDIATE` at `tests/common/mod.rs:990`, used at
  `tests/lifecycle.rs:447,568`); the vandal at
  `src/persist.rs::load_surfaces_a_corrupt_row_type` (`:341`) (spec.md
  Ruling 3, AC-S.1).
- [ ] 3. Verify the corrected cert sentence: `server_config()` validates
  structure, not time, and no doc claims otherwise.
  **Proved by:** `grep -n 'not_after\|expire' src/tls.rs README.md`
  (no startup time check, no doc sentence claiming one) (spec.md
  Ruling 4, AC-S.3).
- [ ] 4. Apply the amendment set to the owning artifacts — the count
  469→470 in the contract's AC-G.1 and tasks.md task 1 and the
  record-honesty AC-1 (proof clause kept truthful), the pin named in
  AC-39.4 of both the design of record and the contract, the Base tree
  to `791f83d`, and the two null-surface clauses (AC-39.6 design of
  record, AC-39.5 contract) — exactly and only those spots.
  **Proved by:** `grep -n '470' specs/fidelity-closures-prove-ship/spec.md
  specs/fidelity-closures-prove-ship/tasks.md
  specs/fidelity-record-honesty/spec.md`,
  `grep -n inline_adjacent_codes_behave_as_the_renderer_renders
  specs/fidelity-closures/spec.md specs/fidelity-closures-prove-ship/spec.md`,
  `grep -n '791f83d' specs/fidelity-closures-prove-ship/spec.md`,
  `grep -n 'unwrap_or_default' specs/fidelity-closures/spec.md
  specs/fidelity-closures-prove-ship/spec.md` — and
  `git diff --stat` shows no other change to those files (spec.md
  AC-R.1–R.4).
- [ ] 5. File the decision: `docs/decisions/0033-one-server-per-data-file.md`,
  status `proposed`, referenced from the spec, index untouched.
  **Proved by:** the file's status line reads `proposed (awaiting
  operator)`; the spec links it by relative path; `git show --stat HEAD`
  shows no `docs/decisions/README.md` change (spec.md AC-S.1, AC-V.3).
- [ ] 6. Run the gates the pass can touch and record the outputs:
  `cargo fmt --all -- --check` and `brokkr compile --bundle bundles/self`
  exit 0. Clippy and `cargo deny check licenses` are unaffected by a
  markdown-only pass and stand as recorded at `791f83d`; the coverage
  gate is not re-run (contract red line 10 — TMPDIR is absolute, never
  twice without need).
  **Proved by:** the two recorded outputs (plan.md Constitution check).
- [ ] 7. Commit the synthesis and the amendments as one commit in the
  repo's style; never push; leave `.brokkr/state/` uncommitted.
  **Proved by:** `git show --stat HEAD` — exactly
  `specs/fidelity-prove-ship-receipt/{spec,plan,tasks}.md`,
  `docs/decisions/0033-one-server-per-data-file.md`, and the amendment
  edits to the four owning spots — no `src/`, no `Cargo.toml`, no
  `deny.toml`, no README change (spec.md AC-V.3); `git status
  --porcelain -- specs/fidelity-prove-ship-receipt` empty for the
  deterministic check.

## Hand-off — the phases that remain (bound by the amended contract)

- [ ] 8. Verify proves the amended contract's demonstrations — the gates
  (AC-G.1–G.4 with **470**), the per-issue proof sites (AC-39…57), the
  frozen three (AC-T.4) — and records the outputs; the only changed
  expectations are the count and the pin's name.
  **Proved by:** the contract's tasks 1–14, executed as written.
- [ ] 9. Review rules line- and sentence-level against the contract's
  red lines 1–14 plus this pass's 15–16 (the hostile-body invariant
  pinned structurally, the bench family its proof home; cert time
  validity client-visible, no doc claims startup validation); a
  security-hold is a hard stop.
  **Proved by:** the review verdicts in the run journal (contract
  AC-R.2); any repair follows the contract's plan protocol — owning
  file, same suite, 0020, one `proposed` decision only if genuinely
  semantic.
- [ ] 10. Ship closes hermetically; 0031, 0032 and **0033** leave review
  `proposed` with evidence attached — 0033 explicitly flagged as
  unimplemented, its landing (refusal + test + sentence in one commit)
  the operator's act.
  **Proved by:** the ship seat's exec of the recorded evidence
  (contract task 13) and the journal's result chain ending clean.
