# Tasks — record honesty at the repaired tree

Ordered for this pass and the phases that follow. Every task pairs with
the check or test that proves it; a task that cannot be proved is not a
task. This pass runs no production code; the proving tests are the ones
the repaired tree already carries, plus deterministic inspections of the
committed record. Run at the repository root with `.brokkr/state/env.sh`
sourced.

## This pass — repair the record

- [ ] 1. Amend the contract of record in its own files: AC-G.1 and
  tasks.md task 1 to **469 tests**, AC-39.2's proof list to name
  `src/extract.rs::an_ampersand_run_decodes_in_bounded_work`, and the
  Base line to record `e245fae` as the tree the remaining phases prove.
  **Proved by:** `grep -n '469' specs/fidelity-closures-prove-ship/spec.md
  specs/fidelity-closures-prove-ship/tasks.md` (AC-G.1 and task 1),
  `grep -n an_ampersand_run_decodes_in_bounded_work
  specs/fidelity-closures-prove-ship/spec.md` (AC-39.2), and
  `grep -n e245fae specs/fidelity-closures-prove-ship/spec.md` (Base) —
  exactly the amendment set, nothing else (spec.md AC-5).
- [ ] 2. Verify the pin the HIGH's repair added is real, inline, and an
  output assertion. **Proved by:** `cargo test --lib extract
  an_ampersand_run_decodes_in_bounded_work` — green; the test body
  asserts `extract_links` output on a 500,000-`&` URL with `assert_eq!`,
  no duration (spec.md AC-2).
- [ ] 3. Verify the suite count the contract now claims.
  **Proved by:** `cargo test --workspace --locked` — **469 passed, 0
  failed**, the single ignored test being `tests/rate.rs`'s opt-in
  throughput smoke (spec.md AC-1).
- [ ] 4. Verify the repair's shape is unchanged and minimal.
  **Proved by:** `git show e245fae --stat` — exactly `src/extract.rs`
  (24+/1−) and `src/static/openapi.json` (1+/1−); `grep -n 'REF_MAX'
  src/extract.rs` shows one named local `const` with its reason comment;
  the 503 description at
  `/paths//api/v1/messages/delete/responses/503` names 0014's
  per-mutation refusal (spec.md AC-3, AC-4, AC-6).
- [ ] 5. Commit the synthesis and the amendments as one commit in the
  repo's style; never push; leave `.brokkr/state/` uncommitted.
  **Proved by:** `git show --stat HEAD` — exactly
  `specs/fidelity-record-honesty/{spec,plan,tasks}.md` plus the two
  amended contract files, no `src/`, no `Cargo.toml`, no `deny.toml`, no
  decision document (spec.md AC-7); `git status --porcelain --
  specs/fidelity-record-honesty` empty for the deterministic check.

## Hand-off — the phases that remain (bound by the contract, unchanged)

- [ ] 6. Verify proves the amended contract's demonstrations — the gates
  (AC-G.1–G.4 with 469), the per-issue proof sites (AC-39…57), the frozen
  three (AC-R.3) — and records the outputs; the coverage gate runs only
  with `TMPDIR=/operator-scratch/cov-tmp`, never on tmpfs, never
  twice without need.
  **Proved by:** the contract's tasks 1–14, executed as written; the only
  changed expectation is the 469 count.
- [ ] 7. Review rules line- and sentence-level against the contract's red
  lines 1–11 plus this pass's 12–14 (`REF_MAX` stays a named local; the
  pin stays an inline output assertion; no parse timer; restore stays
  before bind and outside `insert`; the prose stays truthful to the
  accepted decisions); a security-hold is a hard stop.
  **Proved by:** the review verdicts in the run journal (contract
  AC-R.2); any repair follows the contract's plan protocol — owning file,
  same suite, 0020, one `proposed` decision only if genuinely semantic.
- [ ] 8. Ship closes hermetically; 0031 and 0032 leave review `proposed`
  with evidence attached, acceptance remaining the operator's act.
  **Proved by:** the ship seat's exec of the recorded evidence
  (contract task 13) and the journal's result chain ending clean.
