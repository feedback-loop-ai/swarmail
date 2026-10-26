# Implementation prerequisite: blocked

Date: 2026-10-26. Result: blocked; routing implementation remains outstanding.

The implementer independently reproduced the compatibility prerequisite in
proposed [0081](../../docs/decisions/0081-glm-flash-seats.md), following the
[plan](plan.md) and task 1 of [tasks](tasks.md). The operator's GLM model
selection is settled. The missing capability is admitted dsh workspace-hands
support compatible with the pinned engine, or an explicit operator revision
of the conflicting hands/engine constraints. Neither is present in this
workspace. This finding does not require a live Spark probe.

## Reproduction and observations

Both supplied issue-0 evidence files passed their SHA256SUMS verification.
The candidate-local engine's SHA256 was
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`,
matching the task and tooling record. The tested candidate was
`224d49025f06d0a6f45eeeb2dea760acecc11940`.

Copy `agents/`, `adapters/`, `bundles/`, `docs/`, `dialects/` and
`realms.json` into a disposable directory within the worktree, and create its
`recipes -> bundles` symlink. Change only `models` to `["glm-flash"]` and
`efforts` to `{}` in the copies of `codex-intake`, `codex-careful-intake` and
`codex-review-tests`. Retain every other field. From that directory, invoke
the absolute candidate-local engine path with `agents show <office>` for
each office and `compile --bundle bundles/<recipe>` for all four recipes.
These commands resolve configuration without provider dispatch.

| Office | Command exit | Selected resolution | Resolved effort |
| --- | --- | --- | --- |
| `codex-intake` | 0 | dsh / glm-flash / blocked | `{}`; not applicable |
| `codex-careful-intake` | 0 | dsh / glm-flash / blocked | `{}`; not applicable |
| `codex-review-tests` | 0 | dsh / glm-flash / ok | `{}`; not applicable |

Both intake resolutions report:

> the provider declares hands unsupported, so the agent's hands cannot be put in the box and the agent would run with the harness's own tools

The `agents show` process exit of zero therefore does not prove a usable
candidate. `compile` exits 1 for both `codex` and `codex-careful` with that
same intake refusal. `self` and `codex-fast` compile with exit 0 and zero
unruled policy valuations. Fast has no intake, so its success does not resolve
the prerequisite. No provider, Spark host, canonical journal, CI, GitHub or
sibling worktree was accessed for this reproduction.

## Committed scope and remaining work

This attempt indexes the existing proposed 0081 and records this independent
reproduction. No routing edit is applied to the candidate: the design requires
task 1 to succeed before dependent routing/proof changes. Current source,
proof pins and route summaries therefore continue to describe the existing
routes. All routing tasks and final feature acceptance remain outstanding.
A tests-review-only change would leave the requested intake behavior unmet.

Deleting intake hands, claiming unsupported adapter capabilities, adding a
paid fallback, or changing the engine pin would exceed the supplied authority.
Resume dependent implementation only after separately authorized compatible
support or an explicit operator revision, with reconciled design and fresh
resolution/compile proof. Proposed 0081 remains proposed; this report accepts
no decision. Passing gates on the preserved routes cannot prove the requested
GLM routing. Fresh local gate receipts and full/focused dispatcher outcomes
are retained separately in run-local evidence; none replaces that prerequisite.
