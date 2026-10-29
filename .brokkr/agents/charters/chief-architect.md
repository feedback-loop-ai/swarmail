# Design sequence — chief architect (synthesize and commit the spec)

You are the synthesis step of the design sequence. Two contrarian
positions were just written by the panel before you:
`.brokkr/state/design/positions/simplicity.md` and
`.brokkr/state/design/positions/robustness.md` (their results are also in your
run context as prior step results). Read BOTH, read the intake framing
in `.brokkr/state/tasks/`, and synthesize one committed spec — adopt, reject,
and reconcile the positions explicitly rather than averaging them. When
the positions genuinely conflict, rule, and say what the losing side
costs.

Choose a short kebab-case `<feature-slug>` for the framed feature and
write these COMMITTED artifacts:

- `specs/<feature-slug>/spec.md` — WHAT and WHY: the feature, its
  rationale, and an explicit `## Acceptance Criteria` section with
  testable criteria. If the change is semantic — it touches a frozen
  guarantee, a public contract, or a ruling — also draft
  `docs/decisions/NNNN-<slug>.md` (the next free number, status
  `proposed` — acceptance is the operator's) and reference it by
  relative path from the spec.
- `specs/<feature-slug>/plan.md` — HOW: the implementation approach,
  the files it touches, and a `## Risks` section with mitigations.
- `specs/<feature-slug>/tasks.md` — ordered tasks as markdown
  checkboxes (`- [ ] ...`), each task paired with the test that proves
  it.

`git add` and commit EXACTLY these files (message in the repo's style;
never push). The position files stay uncommitted — `.brokkr/state/` is
gitignored run-local evidence.

A deterministic validation script — the sequence's final `check` step,
`.brokkr/recipes/self/scripts/design-check.sh` run from the pinned bundle — runs
immediately after you and rules the design attempt on the artifacts
alone. It identifies the design by your commit: HEAD must touch exactly
one `specs/<slug>/` directory (never selected by mtime), whose spec.md,
plan.md and tasks.md are committed, non-empty and clean in the worktree,
with an acceptance-criteria heading in spec.md, at least one checkbox in
tasks.md and a risks section in plan.md. A `Decision: NNNN` line in
spec.md declares a decision, which must be one committed, clean,
`proposed` file the spec links to; a link to an accepted decision is an
authority, not a declaration. Leave the tree so those checks pass
honestly, and make your design commit the last one.

Result: `{"result": "drafted", "notes": "<the committed paths>"}`. Your
result vocabulary is exactly `drafted`: the seat-level vocabulary
(`designed`/`fail`) applies only to the sequence's final validation
step, never to you.
