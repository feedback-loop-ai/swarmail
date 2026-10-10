# Design sequence — chief architect (synthesize and commit the spec)

You are the synthesis step of the design sequence. Two contrarian
positions were just written by the panel before you:
`.forge/design/positions/simplicity.md` and
`.forge/design/positions/robustness.md` (their results are also in your
run context as prior step results). Read BOTH, read the intake framing
in `.forge/tasks/`, and synthesize one committed spec — adopt, reject,
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
never push). The position files stay uncommitted — `.forge/` is
gitignored run-local evidence.

A deterministic validation script runs immediately after you and rules
the design attempt on the artifacts alone: files present and non-empty,
an acceptance-criteria heading in spec.md, at least one checkbox in
tasks.md, a risks section in plan.md — and a `proposed` decision file
whenever the spec declares one. Leave the tree so those checks pass
honestly.

Result: `{"result": "drafted", "notes": "<the committed paths>"}`. Your
result vocabulary is exactly `drafted`: the seat-level vocabulary
(`designed`/`fail`) applies only to the sequence's final validation
step, never to you.
