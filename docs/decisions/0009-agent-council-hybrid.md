# 0009 — The agent council: a hybrid delivery machine

- **Status:** proposed (awaiting operator acceptance)
- **Date:** 2026-10-10
- **Supersedes:** none

## Context

Swarmail's delivery constitution is the starter trio: intake,
implementer, reviewer — three model seats on glm-flash, with hermetic
exec gates for verify and ship. It works, but single-model review has a
structural weakness: the same seat that would notice an overbuilt design
is the one that already owns the implementation, and one reviewer passes
judgment on correctness and security in a single pass. Brokkr's own
delivery machine — the one that built this repo — resolved this with a
**hybrid**: model seats propose and argue, deterministic drivers rule,
and no seat both does and judges the same thing.

## Decision

Adopt brokkr's pattern, sized to this repository:

1. **An `architecture` phase before implement** — brokkr's own word,
   deliberately outside the dialect phases so the hermetic compile gate
   needs no in-repo dialect — whose seat is a **sequence**:
   - a *positions panel* — `position-simplicity` and
     `position-robustness` argue in parallel (`unanimous-pass`), writing
     run-local positions under `.forge/design/positions/` (gitignored
     evidence);
   - a *chief* — `chief-architect` — reads both positions and the
     framing, reconciles them explicitly (adopt, reject, name the
     trade), and commits `specs/<slug>/spec.md`, `plan.md`,
     `tasks.md`;
   - a *deterministic check* — `scripts/design-check.sh` — rules the
     attempt on the artifacts alone: files present and non-empty, an
     acceptance-criteria heading in the spec, at least one checkbox
     task, a risks section in the plan, and — when the spec declares a
     semantic change — a drafted `docs/decisions/NNNN-*.md` with status
     `proposed`.
2. **Review splits into a panel** (`aggregate: review-panel`, worst
   verdict wins): `review-correctness` — guarantees, acceptance
   criteria, test realism — and `review-security` — the threat model.
   Both are strictly read-only (`fixes_applied: false`); only
   `review-security` can return `security-hold`. The seat's result
   vocabulary and the protected-phase rules are unchanged.
3. **Munin** joins as the standing overseer — reads a fleet dossier,
   writes proposals for the operator, rules nothing — available to
   `brokkr agents show muninn` and future fleet runs, wired outside the
   phase machine exactly as brokkr's is.
4. **Adapters gain the dual-runtime option**: `claude.json` and
   `codex.json` join `dsh.json`, so any seat can be re-pointed at a
   trusted CLI runtime without touching the charters. The default model
   chain stays glm-flash via dsh.

## Deliberate omissions (and why)

- **No `worktrees` phase.** Single repository, no cross-repo fan-out;
  a worktree phase would be ceremony without a consumer.
- **No `regression` phase.** The panel is strictly read-only, so the
  tree the verify gate proved cannot drift during review; the gates
  re-run in CI on every push regardless. If review ever gains write
  authority, regression comes back — that is the trigger.
- **No spec-kit/openspec tooling.** The chief commits the three
  markdown artifacts directly; `design-check.sh` validates structure,
  not tooling.

## Consequences

- The phase machine gains one phase and three rules
  (`ARCH-OK` → implement, `ARCH-RETRY` → retry once,
  `ARCH-FAIL-TWICE` — a hard stop — and `INTAKE-OK` now targets
  `architecture`).
- Design costs two panel models + one chief per framed feature, on a
  cheap model chain — the price of never implementing an unframed or
  unargued feature.
- The charters stay ordinary text: edit them, commit them, and the
  gates stay green. Only `policy.json`'s table and `bundle.json`'s seat
  wiring are load-bearing; `brokkr compile --bundle .` refuses a
  malformed machine.
