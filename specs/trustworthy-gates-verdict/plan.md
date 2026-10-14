# Plan — run 6c: re-certify the delivered tree at its final HEAD, judge it, ship it

## Technical context

One Rust crate (`src/`), real-protocol integration suites under `tests/` (0004: no mocks),
the exact coverage gate (`scripts/coverage-gate.sh`, literal 100/100/100 with per-run
evidence under `target/coverage/run.XXXXXX`), and the hermetic verify seat
`bundles/self/scripts/verify-seat.sh` — an exec seat whose gate set is fixed and ordered
(the nine gates of AC-6c.1), budget 10800 s with max_attempts 2
(`bundles/self/bundle.json`). The workspace brokkr is `.forge/bin/brokkr` (its stderr
`REVIEW-RESIDUAL-OK` note is benign); the pinned toolchain record is
`bundles/self/tooling.json`. Git in harness sessions here runs only with
`GIT_CONFIG_COUNT` unset — the workspace exports it with no matching key and git refuses
to parse until it is gone. No gpg-agent is reachable from a seat (read-only `~/.gnupg`,
no secret key on disk): commits land unsigned with the fact recorded in the message body,
as the 6b design commit `996574d` did — the operator re-signs. Review is the protected
phase: four members per `bundles/self/bundle.json` (correctness, security, guarantees,
tests), one independent model route each (decision 0013), security-hold hard stop. Ship
is an exec seat (1800 s, max_attempts 2) that signs on a clean worktree and no drift.

Tree of record: `7c2c446` = the ten batch commits (`463f9d2`…`584b23d`) + the 6b spec
trio and its two repairs (`996574d`, `54e9315`, `8877543`) + the operator's root fix.
Counts of record: **5827/320/729** (the operator's `run.BUoDkq` on the fixed tree);
5826/320/729 are the batch tree's counts and belong to the captured concurrent pair —
stale as an expectation (spec R3). The chief's own fast gates this session, at `7c2c446`
with a clean worktree: fmt clean, clippy exit 0, `cargo deny check licenses` ok,
`brokkr compile --bundle bundles/self` exit 0, `coverage-gate.sh selftest` pass, and
`cargo test --test binary --locked` 9 passed in 5.13 s. The full workspace suite and the
remaining hermetic gates belong to the verify seat and are claimed by no one else.

## Constitution check

- **Frozen guarantees (0001/0002/0003/0005)** — untouched: the run's only `src/` change
  is a comment (R2), and it states what the code already does. The suites that prove the
  guarantees run unchanged inside the verify seat; the review panel re-argues them from
  the delivered code (the robustness seat's ground read is the design-side input; the
  guarantees member votes on the tree).
- **0020 disciplines** — no new production lines, so no new test obligation; the repaired
  comment moves no measured line (doc comments are not DA records — R3's detector proves
  it at 5827). Any panel-forced edit obeys the disciplines in full: chaos at the poles
  only, no sleep as synchronization, no wall-clock validity assertions, per-test fresh
  stores/inboxes, seams `#[cfg(test)]`-only on the owning module.
- **0004** — no mocks anywhere; the seat's proofs are hermetic by construction (the
  publish guard's own local server, the selftest's generated fixtures, never shipped).
- **0006 / deny** — no new dependency, no allowlist touch; `Cargo.lock` and `deny.toml`
  untouched.
- **0018 §6 / frozen contexts** — no CI edit at all this run; the eight required context
  names must remain byte-identical (AC-6c.6 greps them).
- **Decisions** — none drafted. The repair is doc-only and decision 0026 is already
  accepted; this spec declares no decision, and the deterministic design check requires
  none.
- **Coverage floor** — exact 100/100/100, missed == 0, no exclusions;
  `scripts/coverage-gate.sh` is never edited this run.

## Project structure

Files touched by this run — one existing file plus this spec trio (no other new
committed file):

- `specs/trustworthy-gates-verdict/spec.md|plan.md|tasks.md` — this spec, committed by
  the design step.
- `src/smtp.rs` — one comment: the `JoinSet` drain parenthetical in `serve`, rewritten
  to the stage-precise shutdown policy (R2 / AC-6c.3).

Run-local (never committed): `.forge/results/*` seat records,
`.forge/design/positions/*` panel evidence, `.forge/tmp/*` captured output and — only if
a seat needs an operator edit — `.forge/run6c-operator-edits.md`. `target/coverage/run.*`
keeps accumulating as gate evidence (reaped by `cargo clean`; CI's `upload-artifact` is
the durable copy).

## Approach

**Repair first, so the proven tree is the shipped tree.** The implement step lands R2 as
one named commit — the drain parenthetical in `src/smtp.rs` restated stage-precisely
(AC-6c.3's wording is the contract) — then fmt, clippy, the gates that read the touched
path (in particular the lifecycle suite, whose in-flight-commit test already asserts the
wire truth) and the workspace suite. Nothing else in `src/` moves.

**Verify — the parked work, at the final HEAD.** Run
`bash bundles/self/scripts/verify-seat.sh` to exit 0. The nine gates run in the script's
order; nothing is skipped, retried, or parallelized away. The exact-gate step at the
final HEAD is the detector demanded by R3: its summary must read 5827/320/729. The
seat's captured output must show `tests/binary.rs` complete (AC-6c.2). If the deadline
fires, the attempt fails honestly; one retry is allowed by the bundle; a second kill is
reported to the operator — a seat cannot edit `bundles/`, and a shortened gate set is a
false green.

**Review — judge the delivered batch.** Each member judges against the children's
acceptance criteria (issues #50–#53, located by shape from baseline `ac8c326`), the
parent spec's R1–R15 and ACs, and accepted decision 0026. The correctness member's read
includes the deltas named in R6: the labeled gauge registries and drop-guards
(`src/api.rs`), the park wiring (`src/mcp.rs`/`src/smtp.rs`), the pre-bind SIGINT
watcher (`src/main.rs`), the ANSI-conditional subscriber. Cheap objective checks any
member may cite: the operator's `run.BUoDkq` summary (5827/320/729) beside the seat's
own, the captured concurrent pair (`run.1DTHyN` + `run.Tg4n8z`),
`git log --oneline ac8c326..HEAD`, and the two positions' audits. A defect named by the
panel lands per the spec's discipline (own commit, test, gates that read the touched
paths; the full seat again if `src/` changed). Verdicts are typed per member and
aggregated by the bundle; security-hold stops the run.

**Ship — clean and drift-free.** `git status --porcelain` empty; the run's commits are
exactly the spec trio plus the repair on top of `7c2c446` (plus any panel-forced
repair); grep the eight frozen context names; confirm `agents/`, `adapters/`,
`bundles/` are untouched by seats and `git log` shows no issue-close or PR by a seat.
The ship seat signs the tree verify proved — the final HEAD, not the tree of record;
drift is measured between verify's tree and ship's.

## Verification order (the run's own)

1. The repair lands (one named commit).
2. The full hermetic verify seat runs to exit 0 — the parked work, completed once at the
   final HEAD; its exact-gate summary is the counts detector (5827/320/729) and its
   captured output is AC-6c.2's evidence.
3. Review votes on the delivered batch against the children's ACs, the parent spec and
   accepted decision 0026 (the protected phase; typed verdicts).
4. Any panel-named defect loops through implement → the gates that read the touched
   paths (the full seat if `src/` changed) → review again.
5. Ship checks the clean worktree, the frozen context names, and the commit shape; only
   then does it sign.

## Risks

- **The 3 h verify budget is still tight** (the ~49 min hermetic suite + the
  instrumented gate run, which rebuilds in its own target dir, + the selftest, the
  publish-guard proof, deny and the bundle compile) → the seat's order is fixed and
  nothing is skipped; if the deadline fires the attempt fails honestly and max_attempts
  2 allows one retry; a second kill is reported to the operator (the budget is operator
  property). Never economize the gate set into a false green.
- **The seat's counts drift from 5827/320/729** → the delta was not comment-only as
  claimed, or the seat's environment resolved a different source graph: stop, name it,
  re-prove. Mitigation is the R3 detector itself; the stale 5826 must not be written
  into any expectation — a spec pinned to it would red on a legitimate delta.
- **A serve-spawning test hangs again** → the emitter fix is environment-independent,
  but the contract is still the startup line's format: any failure is named at the
  exact line (`parse_addrs`), reported red, and repaired at the root — never with a
  `NO_COLOR` ritual in the harness or a sleep (R4).
- **The verify seat's environment lacks a nightly toolchain carrying llvm-tools** → the
  gate refuses loudly naming the toolchain gap (seen in this workspace with a bare
  PATH); the refusal is the honest direction. Never auto-install, never fake the
  verdict.
- **Review locates findings by the issues' stale line numbers** (baseline `ac8c326`,
  main has moved) → the spec mandates locate-by-shape; a finding anchored to a moved
  line is re-anchored before it is judged.
- **A defect is named late (after the seat's green)** → it lands as its own minimal
  named commit with its test; the gates that read the touched paths re-run; a `src/`
  change invalidates the seat's green, so the full seat re-runs before ship. Budgets
  exist for exactly this (implement 7200 s × 3 attempts, verify 10800 s × 2 attempts).
- **Scope creep — the recorded residuals (global webhook cap, attacker-shaped aggregate
  memory, unbounded idle peers) tempt an implement step** → R1's refusal list; review
  refuses out-of-spec churn even when the change would be good, because a completion
  run's job is the verdict, not new features.
- **The captured exactness evidence is reaped before review reads it** (`target/` is
  gitignored) → nothing in this spec depends on the batch-tree pair surviving: it is
  mechanism-proof, not counts-proof; the counts proof at the final tree is the seat's
  summary beside the operator's `run.BUoDkq` (R3). Re-staging is refused unless the
  counts drifted.
- **The commit lands unsigned** (no gpg-agent from a seat) → recorded honestly in the
  commit body and the seat result; the operator re-signs, as with `996574d`. Never
  claim a signature that is not there.
- **The repair is challenged as unlicensed delta** → the spec rules it (R2, panel-named,
  verified in tree by the chief); the commit message cites the ruling; the
  deterministic design check passes on the declared-no-decision spec because nothing
  semantic changes.
