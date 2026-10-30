# 0100 — Everything brokkr owns lives in `.brokkr/`

- **Status:** accepted (operator, 2026-10-29 — "yes, accepted", given in
  chat).
- **Date:** 2026-10-29 (operator direction given 2026-10-29: "can we
  organize everything under .brokkr?"; default recipe and state timing
  chosen the same day)
- **Scope:** where swarmail keeps brokkr's configuration and runtime state.
  No recipe, office, route, limit, gate or decision status changes; every
  recipe compiles to the same seats it compiled to before.
- **Adopts:** brokkr's accepted decision 0083 (an adopter's brokkr lives in
  one folder) with its shelf amendment (#696) and the personal-shelf
  composition of #697, all present in the engine pinned when this was
  written (`731189f`, binary `ab93555…`) and in its successor pin
  (`a9893b4`, binary `4873928…`).

## Context

Swarmail has been a brokkr *recipe library*: `bundles/` (aliased as
`recipes`), `agents/`, `adapters/`, `dialects/` and `realms.json` at the
repository root, runtime state in gitignored `.forge/`, and a stale,
gitignored `.brokkr/` holding leftovers from before 2026-10-23. The
library layout predates brokkr's folder: 0083 moves everything brokkr owns
into `.brokkr/`, tracks the configuration, and ignores only
`.brokkr/state/`. The engine's shelf `.brokkr/recipes/<name>/` keeps
`extends` working between recipes on it, which swarmail's composed lanes
(`self` → `codex` → `codex-careful` → `claude-careful`,
`mixed-opus-terra`) need.

The layout was proved on a throwaway worktree with the pinned engine: all
six recipes compile from the shelf through their `extends` chains, the
folder bundle compiles, and `brokkr doctor` reports the folder `ok`. The
engine refuses the folder while a root `.forge/` stands beside it.

## Rulings

1. **The layout.** Tracked: `.brokkr/bundle.json`, `.brokkr/realms.json`
   (realm path `..`, journal `state/forge.db`), `.brokkr/recipes/<name>/`
   for `self`, `codex`, `codex-careful`, `codex-fast`, `mixed-opus-terra`
   and `claude-careful`, `.brokkr/agents/`, `.brokkr/adapters/`,
   `.brokkr/dialects/`, and `.brokkr/.gitignore`, whose one line is
   `state/`. Nothing brokkr-owned stands at the root: no `bundles/`,
   `recipes`, `agents/`, `adapters/`, `dialects/`, `realms.json` or
   `.forge/`. The root `scripts/` shims are the project's and stay; they
   dispatch to `.brokkr/recipes/self/scripts/` and `shims.sha256` pins them.
2. **The default recipe is `claude-careful`.** `.brokkr/bundle.json` is
   `{"name": "swarmail", "extends": "claude-careful"}` and adds nothing, so a
   run with no `--recipe` delivers on the zero-Codex lane of accepted
   [0099](0099-claude-careful-recipe.md). Every other recipe still runs by
   name.
3. **Runtime state is `.brokkr/state/`.** The journal, results, ledgers
   (`ship-seat.sh` writes `.brokkr/state/ledger/<run>.md`), task framing
   (`.brokkr/state/tasks/`), the staged engine (`.brokkr/state/bin/brokkr`)
   and run worktrees stand there. The 0057 journal `.forge/forge.db` moves
   there whole, once no run holds it open; it is never copied, merged or
   aliased while open. Earlier journals stay historical evidence.
4. **Order.** The tracked move lands first. The local checkout adopts it
   only together with the state move, after the claude-careful runs queued
   on 2026-10-29 drain, because the engine refuses the folder while
   `.forge/` stands beside it. Live and queued runs keep resolving from their
   frozen bundle worktrees and are unaffected.
5. **Historical records keep their paths.** Accepted decisions, specs,
   delivery receipts and task notes name `bundles/…`, `agents/…` and
   `.forge/…` as they were; this record is the map from those paths to the
   folder.

## Consequences

- A recipe change still ships as a tracked change. Personal recipes on
  `$XDG_CONFIG_HOME/brokkr/recipes/` may extend shelf recipes but compile
  against the folder's agents and adapters; whether routing experiments may
  live there is not ruled here.
- Candidate branches cut before this change carry the library layout and
  compile as a library; once they merge this change, any engine-written
  `.forge/` in their worktree must be cleared before a run uses the folder.

## Dated addendum — 2026-10-29: the ship ledger

Ruling 3's ledger path could not be written: the ship seat's box mounts the
repository's state folder read-only, so every review-passed run stopped at
ship with `mkdir: Read-only file system` and was closed by the operator
with `brokkr ledger --repo`. `ship-seat.sh` now writes the ledger beside the
engine's result files for the run, `<results dir>/<run>.ledger.md`, the one
directory every ship attempt can write and read back. `brokkr ledger --repo`
still renders `.brokkr/state/ledger/<run>.md` for the operator.

## Dated addendum — 2026-10-30: the ship ledger, again

The 2026-10-29 addendum put the ledger in the results directory. In the
seat box that directory is read-only like the rest of the state folder;
only the engine's result file is writable, so run
`issue-153-operator-continuation--323fdc1b` parked at ship on it. The ledger
now lives in the worktree's own git directory
(`git rev-parse --git-path brokkr-ledger/<run>.md`), which the box mounts
read-write and which no tracked-tree check sees. The seat's notes stage in
the box's private temporary directory.

## Dated correction — 2026-10-30: why ship could not write

Both ledger addenda above name the wrong cause. The engine binds the
journal's directory read-only for the ship seat when the journal lies
outside the worktree (brokkr `engine/boxed.rs`, `runtime_hands`). The
operator's journal is `.forge/forge.db` in the main checkout, and the
candidate worktrees lived under `.forge/landing/`, so at ship the whole
candidate became read-only, the engine's result file included. A candidate
outside the journal's directory ships normally. Candidates now live outside
`.forge/`. The ledger stays in the git directory, which is harmless.
