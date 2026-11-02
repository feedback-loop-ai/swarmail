# Intake framing: #148 rework after CI refused the eval recipe

Run: issue-148-rework-after-ci-refuse-daac5d68. Branch repair/issue148-eval-recipe
(HEAD ae7cb83). Local main (39a04d0) and origin/main (3ed961e) are both ancestors of HEAD.
Engine: brokkr 0.12.0, binary sha256 29dc3309... (.brokkr/recipes/self/tooling.json).

## Goal

`.brokkr/recipes/eval` compiles under the pinned engine, keeps 0116's intent (one
trial seat, one exec scoring gate, journal joins by trial_id), and every gate that
compiles recipes covers every directory under `.brokkr/recipes/`.

## Findings (checked by reading)

- `eval/bundle.json` has `policy: policy.json` and no `extends`. `eval/policy.json`
  has phases trial, score, done, stop. Only `self/bundle.json` declares
  `protected_phase: "review"`. The other recipes extend `self` and inherit review.
- The refusal was reproduced by the dispatcher. Brokkr is not installed on this
  host, so this seat did not reproduce it.
- Spec fact F4 (`specs/eval-brokkr-recipe/spec.md:212`) assumed a two-seat recipe
  with its own policy and no review phase compiles. The pinned engine refutes it.
- Fixed recipe lists to replace:
  - `.github/workflows/ci.yml:399`: `for recipe in self codex codex-careful codex-fast mixed-opus-terra claude-careful eval`
  - `.brokkr/recipes/self/scripts/verify-seat.sh:125`: compiles only `self`. 0116 §7 R2 says it stays unchanged; item 2 supersedes that and must be recorded.
  - `docs/house-rules.md:62`: names only `self`.
- The seat name `score` is read by the evals crate: `evals/src/journal.rs:699`,
  `evals/src/main.rs:946` and `:5454`. Renaming the seat changes evals.
- `tests/delivery_recipes.rs:109-128` pins that recipes may not replace the
  protected policy or the exec gates. Check whether it reaches eval before editing.
- Drafted brokkr issues: `specs/eval-brokkr-recipe/implementation.md:112` (five
  drafts). AC12 is `spec.md:647`; filing is operator-owned.

## Decision to record: 0116 R4 (dated 2026-11-02, status stays proposed)

Shape to try first, "the scoring gate is the protected review":
- `bundle.json`: `protected_phase: "review"`.
- Seat `review`: class gate, exec driver, results pass, fail, unscorable, max_attempts 1, 600 s.
- Policy phases: trial, review, done, stop. Rules: TRIAL-RAN -> review;
  TRIAL-HARNESS-ERROR -> stop; REVIEW-PASS and REVIEW-FAIL -> done; REVIEW-UNSCORABLE -> stop.
- No second scorer run. Evals reads the renamed seat.

Fallbacks, in order, if the engine rejects the first shape:
1. A minimal deterministic review seat (exec, results clean, residual) that checks
   `score.json` against the row without replaying the business check.
2. Move eval out of `.brokkr/recipes/` (AC11) and file brokkr issue 6.

The design phase must establish which shape the pinned engine accepts. The
implement seat verifies it with the pinned artifact.

Brokkr issue 6 (new, drafted for the PR, not filed): a recipe with its own policy
and no review phase is refused by brokkr 0.12.0 with "policy has no 'review'
phase; the protected review gate is non-removable (extension model, layer 1)".
Ask whether an exec-only protected review phase is lawful for a non-delivery
recipe, and what vocabulary that review must use.

## Acceptance criteria (checkable)

- AC1. The pinned artifact's `brokkr compile --bundle .brokkr/recipes/eval` exits 0 and
  its log has no 'lets the run go on without reading' line. Red today.
- AC2. eval's policy has exactly one protected review phase, one trial seat (class
  work, exec, max_attempts 1, results ran and harness_error) and one exec gate.
  Transitions match the list above. A test in `tests/` reads the files and is red on
  the current tree.
- AC3. No change to any other recipe's bundle.json or policy.json, nor to
  `self/bundle.json` or `self/policy.json`. `self/scripts/verify-seat.sh` changes only
  its compile loop, confirmed by diff.
- AC4. verify-seat.sh, the ci.yml bundle loop and the house-rules gate block all
  compile every directory under `.brokkr/recipes/` that holds a bundle.json, by glob,
  with no fixed name list. A refusal fails the gate.
- AC5. A new `tests/` file goes red on the pre-change tree, where verify-seat compiles
  only self and ci.yml names a list. It goes green after the change. The implement
  seat records the red run before the change.
- AC6. 0116 gains a dated R4 section: the chosen shape, F4 falsified, and §7 R2's
  "verify seat not changed" superseded. Status stays `proposed`. The index row in
  `docs/decisions/README.md` keeps its status.
- AC7. `implementation.md` gains brokkr issue 6, verbatim, stated as drafted and not
  filed. No seat files issues.
- AC8. If the seat name changes, `evals/src/journal.rs` and `evals/src/main.rs` are
  updated and `cd evals && SWARMAIL_BIN=<candidate> cargo test --locked` is green.
- AC9. Gates, all `--locked`: `cargo fmt --all -- --check`;
  `cargo clippy --workspace --all-targets --locked -- -D warnings`;
  `cargo test --workspace --locked`; `cargo build --locked --bin swarmail`;
  `cd evals && cargo fmt --all -- --check`; `cd evals && cargo clippy --all-targets --locked -- -D warnings`;
  `cd evals && SWARMAIL_BIN=<candidate> cargo test --locked`;
  `brokkr compile --bundle` for every directory under `.brokkr/recipes/` (pinned artifact;
  reported as not run if absent);
  `bash scripts/coverage-gate.sh` (exact 100%) and `bash scripts/coverage-gate.sh selftest`;
  `bash scripts/publish-guard-test.sh`; `bash scripts/seal-branch-test.sh`;
  `cargo deny check licenses`.
- AC10. Unsigned commit. No push, merge, publish, decision acceptance or new dependency
  (no Cargo.toml or Cargo.lock change).
- AC11. If no shape passes AC1 under the pinned engine, eval moves out of
  `.brokkr/recipes/` with the reason recorded in R4. AC4 still passes. The result is
  reported as not complete, with brokkr issue 6 named.

## Frozen guarantees

None of 0001-0003 is near this work: no `src/` change is expected. The protected
review gate is touched only by adding eval's own review. Self's review is unchanged.

## Gate note

No load proof is required. If the implement seat runs any proof under load, it is
bounded to nproc/4 busy processes for at most 10 minutes, and the load is reported.

## Open facts (only the engine or operator can settle these)

- Whether the pinned engine accepts an exec-only protected review (AC1 decides).
- Whether the implement seat has a brokkr binary. Unknown from here; the verify seat has it.
