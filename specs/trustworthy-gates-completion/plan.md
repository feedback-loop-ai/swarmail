# Plan — run 6b: verify, review and ship the trustworthy-gates batch

## Technical context

One Rust crate (`src/`), real-protocol integration suites under `tests/`
(0004: no mocks), the exact coverage gate (`scripts/coverage-gate.sh`,
literal 100/100/100 with per-run evidence under
`target/coverage/run.XXXXXX`), and the hermetic verify seat
`bundles/self/scripts/verify-seat.sh` — an exec seat whose gate set is
fixed and ordered: fmt; clippy `--workspace --all-targets --locked -- -D
warnings`; `cargo test --workspace --locked`; `brokkr compile --bundle
bundles/self`; the exact gate; the gate's `selftest`;
`scripts/publish-guard-test.sh`; `cargo deny check licenses`; the
fail-closed native-tls probe. Its budget is 10800 s with max_attempts 2
(`bundles/self/bundle.json`). The workspace brokkr is
`.forge/bin/brokkr` (its stderr `REVIEW-RESIDUAL-OK` note is benign);
the pinned toolchain record is `bundles/self/tooling.json` (SHA
`67a1886…`, version 0.12.0). Git in harness sessions here runs only with
`GIT_CONFIG_COUNT` unset — the workspace exports it with no matching key
and git refuses to parse until it is gone (the gate dropped `git grep`
in this batch, so the gate itself no longer cares).

Delivered tree of record: `a9efa40` = the nine batch commits
(`463f9d2`…`584b23d`) + operator `b9c2184` (bundle edits, verify budget)
+ `a9efa40` (decision 0026 accepted). Coverage counts of record:
5826/320/729 (lines/branches/functions) — every captured gate run at
the delivered tree reads them. Review is the protected phase: four members per
`bundles/self/bundle.json` (correctness, security, guarantees, tests),
one independent model route each (decision 0013), security-hold hard
stop. Ship is an exec seat that signs on a clean worktree and no drift.

## Constitution check

- **Frozen guarantees (0001/0002/0003/0005)** — untouched: the run's
  only `src/` change is a doc comment. The suites that prove the
  guarantees run unchanged inside the verify seat; the review panel
  re-argues them from the delivered code (robustness's ground read is
  the design-side input; the guarantees member votes on the tree).
- **0020 disciplines** — no new production lines, so no new test
  obligation; the repaired comment moves no measured line (doc comments
  are not DA records) and the README row is outside the report
  entirely. Any panel-forced edit obeys the disciplines in full: chaos
  at the poles only, no sleep as synchronization, no wall-clock
  validity assertions, per-test fresh stores/inboxes, seams
  `#[cfg(test)]`-only on the owning module.
- **0004** — no mocks anywhere; the seat's proofs are hermetic by
  construction (the publish guard's own local server, the selftest's
  generated fixtures, never shipped).
- **0006 / deny** — no new dependency, no allowlist touch; `Cargo.lock`
  untouched (the `tokio` `process` dev-feature already landed with the
  batch and is not reopened).
- **0018 §6 / frozen contexts** — no CI edit at all this run; the eight
  required context names must remain byte-identical (AC-6b.7 greps
  them).
- **Decisions** — none drafted. The repairs are doc-only and decision
  0026 is accepted; the spec declares no decision, and the deterministic
  design check requires none.
- **Coverage floor** — exact 100/100/100, missed == 0, no exclusions;
  `scripts/coverage-gate.sh` is never edited this run.

## Project structure

Files touched by this run — two existing files plus this spec trio (no
other new committed file):

- `specs/trustworthy-gates-completion/spec.md|plan.md|tasks.md` — this
  spec, committed by the design step.
- `src/chaos.rs` — one doc comment: the `ChaosEvent::Data` variant,
  rewritten to state the delivered DATA policy (R2).
- `docs/decisions/README.md` — one index row for accepted decision 0026
  (R3).

Run-local (never committed): `.forge/results/*` seat records,
`.forge/tmp/*` captured evidence, and — only if a seat needs an operator
edit — `.forge/run6b-operator-edits.md`. `target/coverage/run.*` keeps
accumulating as gate evidence (reaped by `cargo clean`; CI's
`upload-artifact` is the durable copy — R2 of the parent spec).

## Approach

**Repairs first, so the proven tree is the shipped tree.** The
implement step lands R2 and R3 as two named commits (different
provenances — one panel-named, one chief-ruled — so two messages):

1. `src/chaos.rs`, `ChaosEvent::Data` doc → the truth the wire already
   speaks: *a fired DATA rule waits, then fails — an explicit `error`
   line, else `451 4.3.0 chaos`; the mail is never stored, delay-only or
   not.* Match the file's own voice (`ChaosRule`'s `error` doc, the
   issue #17 citation in `src/smtp.rs`'s DATA arm).
2. `docs/decisions/README.md` → one row between the established columns
   (link, title, accepted status), placed after 0025, in the format the
   other 25 rows use.

Then `cargo fmt --all -- --check`, clippy, and the suite — green, and
the counts must still be 5826/320/729.

**Verify — the parked work, under the raised budget.** Run
`bash bundles/self/scripts/verify-seat.sh` to exit 0. The nine gates run
in the script's order; nothing is skipped, retried, or parallelized
away. The exact-gate step at the final HEAD is the detector demanded by
R5: its summary must read 5826/320/729. If the deadline fires, the
attempt is a fail; one retry is allowed by the bundle; a second kill is
reported blocked to the operator — a seat cannot edit `bundles/`, and a
shortened gate set is a false green.

**Review — judge the delivered batch.** Each member judges against the
children's acceptance criteria (issues #50–#53), the parent spec's
R1–R15 and ACs, and accepted decision 0026, locating by shape. The
correctness member's read includes the gauge plumbing (R7). Cheap
objective checks any member may cite: the captured exactness evidence
(`target/coverage/run.1DTHyN` + `run.Tg4n8z`, 0.33 s apart, both
5826/320/729; `run.toMECD`; `run.YDofH1`), `git log --oneline
ac8c326..a9efa40` for the delivered shape, and the two positions'
audits. A defect named by the panel lands per the spec's discipline
(own commit, test, gates that read the touched paths; the full seat
again if `src/` changed). Verdicts are typed per member and aggregated
by the bundle; security-hold stops the run.

**Ship — clean and drift-free.** `git status --porcelain` empty; the
run's commits are exactly the spec trio plus the repairs on top of
`a9efa40` (plus any panel-forced repair); grep the eight frozen context
names; confirm `agents/`, `adapters/`, `bundles/` are untouched by
seats and `git log` shows no issue-close or PR by a seat.

## Verification order (the run's own)

1. The two doc repairs land (each with its own named commit).
2. The full hermetic verify seat runs to exit 0 — the parked work,
   completed once, at the final HEAD; its exact-gate summary is the
   counts detector (5826/320/729).
3. Review votes on the delivered tree against the children's ACs, the
   parent spec and 0026 (the protected phase; typed verdicts).
4. Any panel-named defect loops through implement → the gates that read
   the touched paths (the full seat if `src/` changed) → review again.
5. Ship checks the clean worktree, the frozen context names, and the
   commit shape; only then does it sign.

## Risks

- **The 3 h verify budget is still tight** (suite ~49 min + the
  instrumented gate run, which rebuilds in its own target dir, + the
  selftest, publish-guard proof, deny and bundle compile) → the seat's
  order is fixed and nothing is skipped; if the deadline fires the
  attempt fails honestly and max_attempts 2 allows one retry; a second
  kill is reported blocked to the operator (the budget is operator
  property). Never economize the gate set into a false green.
- **The counts drift from 5826/320/729 at the final HEAD** → the delta
  was not comment/doc-only as claimed: stop, name it, and re-prove
  (re-stage the exactness trio at the new HEAD). Mitigation is the R5
  detector itself.
- **The verify seat's environment lacks a nightly toolchain carrying
  llvm-tools** → the gate refuses loudly naming the toolchain gap (seen
  in this workspace with a bare PATH); the refusal is the honest
  direction. Mitigation: the operator's environment provides the
  toolchain the house gates already ran under; never auto-install, never
  fake the verdict.
- **Review locates findings by the issues' stale line numbers**
  (baseline `ac8c326`, main has moved) → the spec mandates locate-by-
  shape and lists the shapes; a finding anchored to a moved line is
  re-anchored before it is judged.
- **A defect is named late (after the seat's green)** → it lands as its
  own minimal named commit with its test; the gates that read the
  touched paths re-run; a `src/` change invalidates the seat's green,
  so the full seat re-runs before ship. Budgets exist for exactly this
  (implement 7200 s × 3 attempts, verify 10800 s × 2 attempts).
- **Scope creep — the recorded losses (global webhook cap, aggregate
  memory, idle peers) tempt an implement step** → R1's refusal list;
  review refuses out-of-spec churn even when the change would be good,
  because a completion run's job is the verdict, not new features.
- **The captured exactness evidence is reaped before review reads it**
  (`target/` is gitignored) → AC-6b.2 says so: re-stage a concurrent
  pair at the final HEAD and cite the fresh evidence; the proof is the
  property, not the particular directories.
- **The 0026 row's wording drifts from the decision's own status line**
  → the row quotes the decision's title and its accepted status and
  links the file; review reads the two side by side; a mismatch is a
  named defect.
- **The repairs are challenged as unlicensed delta** → the spec rules
  them (R2 panel-named, R3 chief-ruled); the commit messages cite the
  rulings; the deterministic design check passes on the declared-no-
  decision spec because nothing semantic changes.
