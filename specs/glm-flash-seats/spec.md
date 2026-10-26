# GLM Flash tests review

Created: 2026-10-26. Revised: 2026-10-26. Status: draft; tests-only implementation remains outstanding.

Decision: 0081

Proposed amendment: [0081 — GLM Flash tests review](../../docs/decisions/0081-glm-flash-seats.md).
Authorities: accepted [0057](../../docs/decisions/0057-codex-delivery-recipes.md),
[0058](../../docs/decisions/0058-codex-review-routing.md), and the operator's
2026-10-26 “GLM-5.3-Flash seats (revised)” ruling quoted in 0081.
This directive authorizes the narrow routing change; 0081 remains proposed
and no further acceptance is inferred.

## Feature and rationale

Route only the existing read-only `codex-review-tests` office to the local,
no-token-cost `glm-flash` model in Standard, Careful and fast design/engine
review. Retain every other seat's model, effort, charter and permissions.
The operator chose this substitution to reduce paid review use; configuration
proofs do not claim equal model judgment or live Spark availability.

The selected route is exactly provider `dsh`, alias `glm-flash`, concrete
model `spark-glm/GLM-5.3-Flash-EXL3`, route class `local`, without fallback.
Its office explicitly sets `efforts: {}` and its effective route effort is
absent (an omitted or null serialized effort field). Prose records **not applicable**;
there is no effort value named `none`.

## Dated clarification — revised scope, 2026-10-26

The first design attempted both intake offices and tests review. Its local
probe, independently reproduced by the previous implementation attempt,
found both dsh intake resolutions blocked and Standard/Careful compilation
refused because the provider cannot supply boxed workspace hands. `agents
show` exited zero even for blocked resolution. Fast, which has no intake,
could compile the read-only tests route. These remain historical findings.

The operator subsequently ruled that intake stays on its current models.
This revision replaces the old intake migration acceptance criteria and
compatibility prerequisite; it does not require fixing the adapter or engine.
The older [implementation report](implementation.md) describes the superseded
attempt, including its statement that tests-only delivery was insufficient.
That statement no longer governs. The current intake framing already records
the revised scope. Implementation will label the old report accordingly.

## Composition to preserve

| Recipe/lane | Intake or triage | Required review composition |
| --- | --- | --- |
| Standard `codex` | Sol / medium intake, existing hands | Fable correctness, official Flash security, GLM tests (3) |
| `codex-careful` | Sol / high intake, existing hands | Astra correctness, official Flash security, GLM tests, Astra guarantees, independent Fable (5) |
| Fast design and engine | Opus / medium triage, no intake | Same five distinct roles as Careful |
| Fast chore | Opus / medium triage, no intake | One combined official Flash reviewer |
| Fast feature | Opus / medium triage, no intake | Qwen Max correctness and official Flash security (2) |

Fast feature alone retains Qwen-to-Sonnet fallback for the authorized dispatch
failure before a valid review result. Standard Sol/high, Careful Sol/xhigh
and fast Luna/xhigh implementers, shared Sol/Astra design, and legacy self
routes remain. Astra correctness and guarantees keep separate offices and
results; independent Fable remains required. DeepSeek Pro remains forbidden.

## Panel synthesis and rulings

Both current positions agree on tests-only scope; there is no conflicting
model recommendation to average. Adopt simplicity's one-overlay change,
existing adapter, inherited charter and two existing proof files. Reject
new offices, copied charters, route registries, wrappers, dependencies and
engine repairs. Retaining paid intake costs the original intake savings;
the revised operator ruling settles that tradeoff.

Adopt robustness's stronger effective-route proof over the smallest
string-replacement test. A source alias alone cannot detect a blocked route,
inherited effort, wrong concrete provider route or added fallback. Require
exact comparisons and focused mutations through the same comparator. The
losing minimal-test alternative saves assertions but leaves those errors
undetected; we accept the extra assertions. Simplicity's suggested local
comparison wins over a persistent snapshot framework: baseline comparison
is run-local evidence and adds no new tracked routing owner.

Reconcile both positions by retaining explicit empty efforts only on the
shared tests overlay, preserving the base's unrelated Fable/GLM chain and
all other effort assertions. Keep complete panel/unchanged-route checks.
Adopt robustness's runtime failure discussion as preserved tests-review
responsibility, not new application work. Synchronous stored acceptance,
subscribe-before-check, oldest-only pruning and extraction at ingest remain;
webhook, chaos, certificate and protocol contracts retain their existing
proofs. No new runtime hardening, provider benchmark or live health probe
belongs here. The cost is no new quality or availability evidence, and a
Spark outage can stop review without a paid fallback.

## Acceptance Criteria

These criteria apply to final implementation, not to this design-only commit.

1. **AC1 — Exact admitted route.** `codex-review-tests` retains
   `extends: "review-tests"`, selects exactly `["glm-flash"]`, and resolves
   `ok` through dsh to `spark-glm/GLM-5.3-Flash-EXL3`, local, with no fallback.
   Check source, effective office and compiled manifest; CLI exit zero alone
   is insufficient. Source/resolved office efforts equal `{}` and effective
   selected effort is absent (omitted or null), with no explicit or inherited Spark effort.
2. **AC2 — Exact membership.** The same office is the tests member of
   Standard, Careful and both fast design/engine panels. Preserve counts
   3/5/5/5, separate results and all other identities. Fast chore/feature
   retain one/two results and gain no tests member.
3. **AC3 — Preserved routes and boundaries.** Compare full source/resolved
   routing with baseline `d55a89de70f4a4e6cc598d3726eb2fa55bc2df76`: only the
   tests office model/effort route changes. Every other model, chain, effort,
   charter, permission, limit, selector and protected driver/hands/boundary
   remains identical. Both intake offices retain Sol medium/high and their
   boxed workspace hands. Preserve gate/read-only tests classification,
   no added hands/tools, candidate-change refusal, distinct result identity,
   protected review, security hold and bounded returns. Keep dsh's truthful
   unsupported native tool-permission/MCP declarations; claim no native
   sandbox equivalence. Preserve the complete composition above.
4. **AC4 — Sensitive regression proofs.** Extend `tests/delivery_recipes.rs`
   and `tests/codex_review_routing.py` in place. Source assertions pin models,
   inheritance, exact empty efforts and existing Spark mapping/local/effortless
   metadata; effective-route assertions prove AC1–AC3. The same comparator
   accepting actual resolution rejects old Sonnet, wrong provider, wrong
   concrete model, non-local route, blocked status, extra fallback, explicit
   Spark effort and missing/replaced tests member. Keep unrelated-role effort,
   triage, panel, fallback, safety-fact, candidate-fence and dispatcher controls.
5. **AC5 — Pinned compatibility.** Verify engine SHA256
   `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f` at
   `.forge/bin/brokkr`. It compiles self, codex, codex-careful and codex-fast
   with no policy-sweep warnings; effective evidence shows dsh tests routing
   in all four affected panel placements. No engine/pin/adapter capability
   change or intake substitution is used to obtain success.
6. **AC6 — Coherent documentation.** Revised 0081 stays proposed, quotes
   and dates the revised ruling, amends only 0058's tests assignment and
   records the prior intake refusal and reason. Update its existing index
   row, current routing summaries and stale bundle descriptions. Label the
   earlier implementation report superseded without discarding its evidence.
   Preserve accepted decision bodies, legitimate Sonnet fallback references
   and dated provenance; any amendment pointer is a dated addendum. Record
   effort not applicable and create no tracked link into ignored evidence.
7. **AC7 — Verification and honest limits.** Commit implementation unsigned
   and run all local [house gates](../../docs/house-rules.md#gates--part-of-every-change),
   including locked checks, exact zero-missed lines/branches/functions and
   coverage selftest. Run shim integrity, fail-closed native-tls absence,
   design selftest and delivery proof. Focused routing and required local
   house gates must pass. Separately run the full Python dispatcher proof,
   preserving its actual result and any unexecuted later cases. The known
   baseline `1-cross-attempt` failure (completed instead of refusal) remains
   visible, never converted to green or suppressed. It is not a routing
   regression, a gate waiver or authority to repair/re-pin the engine here.

CI owns macOS, privileged port-80, chart/history, Docker, network RustSec
and tag-only proofs in the house inventory. The operator owns Spark host
availability, credentials, live settings and branch sealing. These are
ownership notes, not additional seat-local acceptance criteria. No live
Spark/provider probe, namespace proof, publication or decision acceptance.
