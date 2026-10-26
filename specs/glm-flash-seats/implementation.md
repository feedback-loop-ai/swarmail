# Implementation — tests-only GLM Flash review

Date: 2026-10-26. Implements the revised operator direction recorded in
proposed [0081](../../docs/decisions/0081-glm-flash-seats.md); no decision is accepted.

Only `agents/codex-review-tests.json` changes executable routing: singleton
`glm-flash`, explicit empty efforts. The existing dsh adapter maps it to local
`spark-glm/GLM-5.3-Flash-EXL3`; reasoning effort is not applicable. The shared
overlay supplies Standard, Careful and both fast full panels. Intake stays
Sol medium/high with its existing workspace hands. Other seats, charters,
permissions, policies, engine, adapter and application code stay unchanged.

The Rust source proof pins the Spark route and empty effort map. The Python
resolved proof compares admitted provider/model/local/status/effort and member
identity, rejecting old Sonnet, wrong provider/model/route, blocked resolution,
fallback, office or compiled effort, and missing/replaced members through the
same comparator. Complete panel checks and existing dispatch controls remain.
A run-local comparison against d55a89d checks unchanged source and resolved
fields; recipe descriptions are the only bundle changes.

Final verification receipts are run-local and identify the committed candidate
and pinned engine. House gates and focused resolution are recorded separately
from the full dispatcher script: its pre-existing `1-cross-attempt` assertion
must remain red if the pinned engine completes instead of refusing. A focused
pass does not claim dispatcher-wide correlation enforcement. No live provider
is called. CI and operator proof ownership remains as specified in the
[spec](spec.md); no additional local network or platform criteria apply.

## Superseded attempt — 2026-10-26

The blocked report below preserves the earlier intake migration findings.
The operator's later **“GLM-5.3-Flash seats (revised)”** ruling supersedes its
scope and its statement that tests-only delivery is insufficient. The intake
refusal remains valid evidence of the pinned engine's hands boundary; it is
no longer a prerequisite for this tests-only change. Read the report below
as a historical account of that attempt, not current implementation status.

---

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
