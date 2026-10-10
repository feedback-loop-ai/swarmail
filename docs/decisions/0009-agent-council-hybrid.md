# 0009 — The agent council: a hybrid delivery machine on the speckit dialect

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

Adopt brokkr's pattern, sized to this repository, **on the speckit
dialect**:

1. **The realm declares `dialect: speckit`.** The design phase is a
   real dialect phase: the engine injects the dialect's design
   instructions into the seat's council, the spec artifacts follow the
   speckit shape, and the dialect's own
   `.specify/scripts/bash/check-prerequisites.sh` is the tool of
   record. The workspace SpecKit engine is the source for the scaffold:
   `.specify/{templates,scripts,memory}` carry its spec template, its
   prerequisites checker and its constitution. `specify` 0.8.7 is the
   pinned version and is already on this machine.
2. **A `design` phase before implement**, whose seat is a **sequence**:
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
     `proposed`. It runs beside the dialect's own validator, not
     instead of it: the house checks what spec-kit does not.
3. **Review splits into a panel** (`aggregate: review-panel`, worst
   verdict wins): `review-correctness` — guarantees, acceptance
   criteria, test realism — and `review-security` — the threat model.
   Both are strictly read-only (`fixes_applied: false`); only
   `review-security` can return `security-hold`. The seat's result
   vocabulary and the protected-phase rules are unchanged.
4. **Munin** joins as the standing overseer — reads a fleet dossier,
   writes proposals for the operator, rules nothing — available to
   `brokkr agents show muninn` and future fleet runs, wired outside the
   phase machine exactly as brokkr's is.
5. **Adapters gain the dual-runtime option**: `claude.json` and
   `codex.json` join `dsh.json`, so any seat can be re-pointed at a
   trusted CLI runtime without touching the charters. The default model
   chain stays glm-flash via dsh.
6. **The realm map ships in-repo** (`realms.json`, self-referential
   `path: "."`, journal `.forge/forge.db`, house `docs/house-rules.md`)
   beside an in-repo `dialects/` — the mcp-scaleway pattern. The hermetic
   compile gate must resolve in a fresh clone with no workspace around
   it, so the map, the dialect library and the `.specify/` requires are
   committed. The workspace map (`../realms.json`) declares the same
   dialect so a workspace-level run resolves identically.

## Deliberate omissions (and why)

- **No `worktrees` phase.** Single repository, no cross-repo fan-out;
  a worktree phase would be ceremony without a consumer.
- **No `regression` phase.** The panel is strictly read-only, so the
  tree the verify gate proved cannot drift during review; the gates
  re-run in CI on every push regardless. If review ever gains write
  authority, regression comes back — that is the trigger.
- **No `specify`-generated specs in the deterministic check.** The
  chief commits the three markdown artifacts directly; the dialect's
  checker validates prerequisites and `design-check.sh` validates
  structure. If a run wants `specify` to author the spec, that is an
  additional step in the sequence, not a change to this contract.

## Consequences

- The phase machine gains one phase and three rules (`DESIGN-OK` →
  implement, `DESIGN-RETRY` → retry once, `DESIGN-FAIL-TWICE` — a hard
  stop — and `INTAKE-OK` now targets `design`).
- Design costs two panel models + one chief per framed feature, on a
  cheap model chain — the price of never implementing an unframed or
  unargued feature.
- Fresh clones carry the full realm: the map, the dialect library and
  the SpecKit scaffold are ordinary committed text.
- The charters stay ordinary text: edit them, commit them, and the
  gates stay green. Only `policy.json`'s table and `bundle.json`'s seat
  wiring are load-bearing; `brokkr compile --bundle .` refuses a
  malformed machine.
