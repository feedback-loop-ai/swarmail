# Alkemio Workspace Constitution

Principles that govern cross-repo (vertical) feature work at this workspace.

## 1. Single source of truth per feature

A vertical feature has exactly one `spec.md` — at `specs/NNN-feature-name/spec.md`
on this workspace repo. Per-repo `tasks/<repo>.md` files describe slices, never
duplicate the spec.

## 2. Per-repo independence is preserved

Sibling repos remain independent: their own CI, own branching, own PRs, own
merge cadence. The workspace coordinates intent, not git state. We do **not**
use submodules or subtrees.

## 3. Repo boundaries are real

If a feature *can* be delivered as one PR in one repo, it is **not** a vertical
feature: it needs no cross-repo plan, no rollout ordering and no fan-out. Its
spec still lives here (Principle 7), but with a single affected repo. The cost
of cross-repo coordination is real; only pay it when the change genuinely
couples ≥ 2 repos.

## 4. Rollout ordering is explicit

When PR ordering matters (most often: backend ships behind a flag → ops flips
the flag → client uses it), `plan.md` MUST record the order. PRs that depend
on others MUST link to them.

## 5. SHA pinning is post-merge, not pre-merge

`repos.yaml` inside each spec records merged SHAs **after** the relevant PRs
merge. We don't pre-pin (which would force submodule-style discipline). This
gives us reproducibility after the fact without operational drag during.

## 6. The workspace is a coordination layer, not a code dependency

No sibling repo should `import` from the workspace. Anything the workspace
produces (specs, scripts, docs) is for humans and AI, not for build pipelines.

## 7. Every spec lives at the workspace; the code stays in its repo

Every SpecKit spec — single-repo or cross-repo — is authored at
`agents-hq/specs/NNN-*`. Sibling repos' own `.specify/` engines and `specs/`
directories are deprecated: kept as record, never extended. The only path that
writes no spec is `/bugfix`, whose record is the PR itself. Implementation
always stays inside the affected repo's `feat/NNN-*` worktree, and repo-local
conventions stay in that repo's CLAUDE.md. (Amended 2026-09-23 to match the
workspace CLAUDE.md, which had superseded the earlier "component-internal work
stays in its repo" wording.)

## 8. Slash commands are reused, not reinvented

The `speckit-*` skills work at workspace level the same way they work in each
sibling. We extend the templates (per-repo sections) rather than forking the
commands.

## 9. Concurrent features are isolated via git worktrees

Every vertical feature's per-repo implementation MUST happen in a dedicated git
worktree — `worktrees/NNN-feature-name/<repo>/` on a `feat/NNN-feature-name`
branch (via `.scripts/feature-worktree.sh`). Never implement a feature directly
in a sibling's primary clone. This is what lets multiple vertical features be
delivered concurrently without cross-contaminating working trees or branches: a
glance at a sibling's worktrees and branch list tells you exactly which features
are in flight in that repo. Worktrees are disposable — torn down once the
feature's PRs merge and `repos.yaml` is pinned (see Principle 5).

## 10. Cross-cutting architectural decisions are recorded in the registry

Decisions that span ≥ 2 repos — API contracts, shared data shapes, auth models,
rollout/coupling strategies, technology choices with workspace-wide blast
radius — MUST be captured as an Architecture Decision Record in the workspace
registry at `docs/decisions/` (one `NNNN-title.md` per decision: context,
decision, status, consequences). A `plan.md` that makes such a choice links to
its ADR rather than restating the rationale. Decisions internal to a single repo
stay in that repo's own ADRs (see Principle 7); the workspace registry is only
for cross-cutting concerns.

## 11. Every spec is tied to a backlog story

Each spec MUST map to exactly one story on the delivery board
([`projects/50`](https://github.com/orgs/alkem-io/projects/50)). The mapping is
recorded in the spec (its story URL/number) and is bi-directional — no
orphan specs, no orphan stories. If no story exists for the work, one is created
(reverse flow, `/board-sync`); if the spec is generated from a story, it
back-links to it (forward flow).

Before creating a new story, the backlog MUST be searched for similar or
overlapping stories to avoid duplicates. If a matching story is found, link to
it instead of opening a new one; only create when the search comes back empty.

This principle is enforced by `/speckit-story-sync`, which runs as the mandatory
`after_specify` hook (search → link-or-create → record the mapping in `spec.md`
and `.specify/feature.json`) and can also be invoked manually to backfill an
existing spec.

## 12. Hardening requires independent premise-disproof

A reported edge case is not, by itself, a requirement. Before any hardening is
specified or implemented, the coordinating agent MUST independently trace a
real producer through the currently supported topology to a reachable product
or operational effect. Public and untrusted boundaries count as real producers;
mock-only state, unused types, fixtures, and withdrawn deployment modes do not.

The review MUST first determine whether the edge is created by a wrong,
duplicated, speculative, generic, or overcomplicated owning path. Remedies are
considered in this mandatory order:

1. delete the unnecessary path or mechanism;
2. simplify ownership and data flow;
3. restore proven behaviour;
4. only then add the smallest local guard for a real case that remains.

Hardening MUST NOT be used to preserve machinery that should be removed. A
discriminating failing test or real workflow, plus the owning-boundary RCA, MUST
exist before production code changes. Reports from delegated agents are inputs
to this review, never substitutes for it. When the producer or remaining need
cannot be proven, the default decision is to delete, simplify, or make no change.
