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

## When review sends the run back

When your input carries `context.returned_from` with phase `review`, the
review panel found the design itself defective (`spec_defect: true`) and
returned the run to design (decision 0102, addendum 2026-10-31).
`context.returned_from.result` holds the panel's verdict: its findings,
severities and notes. The positions above were argued against those
findings. Revise the design; do not start another:

- Revise the same `specs/<feature-slug>/` in a new commit on top of the
  branch. Never amend, rebase, rename or delete the earlier design commit
  or its files: git history keeps the spec the code was first built to.
- Answer each finding in the artifact that owns it. Correct spec.md, then
  bring plan.md and tasks.md into line with it, and give each finding the
  implementer must act on its own task and proving test. That includes
  every security finding and every ordinary finding above low the panel
  reported beside the defect: the implementer reads your revised tasks, not
  the review's notes. Say in plan.md which finding each change answers.
- A criterion that conflicts with an accepted decision is corrected toward
  the decision; cite the decision as the authority.
- Never weaken an acceptance criterion merely to make the review pass. Drop
  or loosen one only when the finding shows the criterion itself is wrong,
  and say why in plan.md with the evidence. When the code misses a sound
  criterion, keep the criterion and add the task that fixes the code.
- When the finding is that the design relies on a ruling nobody has made,
  such as a dependency the operator has not approved, design without it
  where you can. Otherwise draft the proposed record as above; you cannot
  accept it, and the run parks for the operator if review still finds the
  same defect.

The same check rules the revision: your new commit must be HEAD and touch
exactly one `specs/<slug>/`. Design returns share decision 0102's five
review returns; the sixth review that still finds a spec defect parks for
the operator.

Result: `{"result": "drafted", "notes": "<the committed paths>"}`. Your
result vocabulary is exactly `drafted`: the seat-level vocabulary
(`designed`/`fail`) applies only to the sequence's final validation
step, never to you.
