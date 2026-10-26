# GLM Flash intake and tests-review seats

Created: 2026-10-26. Status: draft; implementation has a measured compatibility blocker.

Decision: 0081

Proposed amendment: [0081 — GLM Flash seats](../../docs/decisions/0081-glm-flash-seats.md).
Authorities: accepted [0058](../../docs/decisions/0058-codex-review-routing.md),
[0057](../../docs/decisions/0057-codex-delivery-recipes.md), and the operator's
2026-10-26 “GLM-5.3-Flash seats” ruling quoted in 0081. Only the operator
accepts decisions. The current directive authorizes these two seat types;
it grants no unrelated amendment or permission relaxation.

## Feature and rationale

Use the existing local, no-token-cost `glm-flash` route for the two Codex
intake offices and the shared tests reviewer. Preserve their identities,
charters, permissions, limits and required results. This reduces paid model
use at the operator's direction while retaining independent review lenses
and deterministic delivery gates. Offline checks establish configuration
and engine behavior, not equal reviewer judgment or Spark availability.

| Office | Current route | Required route | Required effort |
| --- | --- | --- | --- |
| `codex-intake` | Sol / medium | `glm-flash` only | not applicable |
| `codex-careful-intake` | Sol / high | `glm-flash` only | not applicable |
| `codex-review-tests` | Sonnet / high | `glm-flash` only | not applicable |

Each required route means provider `dsh`, concrete model
`spark-glm/GLM-5.3-Flash-EXL3`, adapter route class `local`, and an empty
resolved effort map. Encode `efforts: {}`, never an effort called `none`.
There is no paid fallback. Keep the adapter's actual capability declarations.

## Composition that must remain

| Recipe/lane | Intake or triage | Required review members |
| --- | --- | --- |
| Standard `codex` | existing `codex-intake` | Fable correctness, official Flash security, GLM tests (3) |
| `codex-careful` | existing `codex-careful-intake` | Astra correctness, official Flash security, GLM tests, Astra guarantees, independent Fable (5) |
| Fast design/engine | Opus medium triage; no intake | same five distinct roles as Careful |
| Fast chore | Opus medium triage; no intake | one combined official Flash reviewer |
| Fast feature | Opus medium triage; no intake | Qwen Max correctness, official Flash security (2) |

Fast feature alone retains Qwen-to-Sonnet fallback on authorized dispatch
failure before a valid review result. No other role or effort changes:
Standard implementer Sol/high, Careful Sol/xhigh, fast Luna/xhigh, the shared
Sol/Astra design sequence and legacy self routes remain. Astra correctness
and guarantees retain separate offices/results; independent Fable remains
required. DeepSeek Pro stays forbidden. Size triage, selectors, bounded
returns, protected review, security hold and candidate binding remain.

## Panel synthesis and ruling

Adopt simplicity's three-overlay design, existing proof files, inherited
charters and rejection of new route registries, wrappers, recipes, fallback
frameworks, dependencies and application changes. The cost of that choice is
explicit per-office assertions and repeated resolved comparisons rather
than a reusable routing abstraction. No abstraction is justified by three
existing offices.

Adopt robustness's exact provider/concrete/local/effort checks, negative
controls and unchanged-role comparisons. Reject a string-only substitution
or a broad “GLM is allowed” test: each would miss inherited effort or an
unauthorized route change. The small additional assertion cost is warranted.
Resolve the effort ambiguity in robustness's favor: use explicit empty
maps in all three overlays. A disposable probe of the pinned engine shows
that `{}` replaces the base tests reviewer's Fable/high metadata; omission
is unnecessary ambiguity. Preserve the base office and adapter unchanged.

Reject both positions' assumption that these three changes suffice to
compile the two intake recipes while retaining workspace hands. Chief
research found a concrete conflict, described below. Preserve the hands
and the engine's refusal; do not delete permissions, claim unsupported
capabilities, or re-pin the engine to make a green result. The losing
assumption costs immediate implementation readiness and delays both intake
moves until a separately authorized compatibility resolution exists.

Adopt robustness's application-failure discussion as preserved reviewer
responsibility, not new scope: synchronous stored acceptance, watcher
subscription ordering, oldest-only pruning, extraction at ingest, bounded
webhook behavior and real-protocol deterministic tests keep their existing
contracts. Reject new runtime hardening, performance benchmarks and provider
probes. The accepted cost is no new availability or model-quality evidence.

## Measured compatibility prerequisite

The supplied evidence README and operator rulings both passed their recorded
SHA256 checks. The admitted candidate-local engine is
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
On 2026-10-26, the chief copied the candidate's agents, adapters, bundles,
realm map, dialect and docs into a disposable local fixture, changed only
these three offices to singleton GLM and empty effort maps, and used that
engine without provider I/O. Both intake offices resolved with status
`blocked`; both Standard and Careful compilations exited 1:

> the provider declares hands unsupported, so the agent's hands cannot be put in the box and the agent would run with the harness's own tools

Fast compiled with the GLM tests reviewer and `effort: null`, consistent
with its absence of intake. The tests office resolved `ok` with `{}` effort.
An `agents show` process exit of zero is not successful resolution: its
per-candidate status must be checked. The fixture's initial missing-dialect
setup error was corrected by copying the candidate realm/dialect; the
reported hands refusal is from the fully configured fixture.

The missing prerequisite is admitted dsh workspace-hands support compatible
with this pinned engine, or an explicit operator revision of the conflicting
constraints. Neither exists in the supplied workspace. This design does not
authorize either change. Under current constraints there is no proven
implementable all-three-office patch. Implementers must preserve this
blocker and must not claim final acceptance from tests-review-only success.
The operator's model selection is settled; the compatibility conflict does
not reopen that selection or justify a paid fallback.

## Acceptance Criteria

These are final implementation criteria, not claims made by this design commit.

1. **AC1 — Exact intake resolution.** Source and pinned-engine resolution for
   both named intake offices use exactly `[glm-flash]`, dsh, the concrete
   Spark model and local classification. Every selected candidate resolves
   `ok`; Standard/Careful retain their existing office names. Both compile
   with their original hands. The measured prerequisite above currently
   prevents satisfying this criterion; blocked resolution never passes.
2. **AC2 — Exact tests resolution.** The tests office has that same singleton
   route in Standard, Careful and fast design/engine. Fast chore/feature have
   no added tests member. Role counts and separate member/result identities
   remain exactly 3, 5, 5, 1 and 2 as listed above.
3. **AC3 — No Spark effort.** All three source overlays contain empty effort
   maps, their resolved maps are empty and compiled selected Spark candidates
   have no effective effort (`null` in the manifest). Reject explicit or
   inherited selected Spark effort. Documentation records “not applicable.”
4. **AC4 — Unchanged boundaries.** Compare to baseline `a037dc9`: all three
   offices' non-routing fields and inherited charters/limits are identical.
   Intake keeps workspace hands, network declaration, Cargo credential masks
   and read-only rustup bind. Review remains gate-class/read-only with no
   added hands/tools and `fixes_applied: false`. Keep dsh's unsupported native
   tool-permission/MCP declarations and the candidate-change refusal; do not
   claim equivalent native sandbox enforcement.
5. **AC5 — Preserved matrix and contracts.** Every other resolved office,
   chain, effort, role, selector and protected driver/hands/boundary remains
   as in the composition above and baseline. Retain the sole Qwen dispatch
   fallback, Opus medium triage without fallback, no fast intake, independent
   result identities, safety-fact refusals and all legacy self routes.
6. **AC6 — Exact regression proofs.** Update the existing Rust and Python
   routing proofs and any other executable old-route pins. Do not allow GLM
   generically. The same comparator accepting actual resolved routes must
   reject an old paid alias, wrong provider, wrong concrete model, non-local
   route, non-ok resolution, extra fallback and explicit Spark effort.
   Preserve current triage, role-count, fallback, safety-fact, candidate-fence
   and evaluator controls. Source-only assertions cannot satisfy AC1–AC3.
7. **AC7 — Decision and documentation.** Index proposed 0081 with matching
   status, quote the dated operator ruling, and make current route summaries
   agree with AC1–AC5, including effort not applicable. Keep accepted bodies
   intact; any pointer to 0081 is a dated addendum. No tracked link points
   into ignored evidence. Keep legitimate Sonnet fallback references.
8. **AC8 — Local proof and truthful limits.** On committed implementation,
   all four bundles compile with the admitted engine and no policy-sweep
   warnings; focused resolution, Rust routing tests and all local house gates
   pass, including exact coverage and its selftest. Run the full dispatcher
   proof separately and retain its actual outcome and unexecuted controls.
   Intake reported a baseline `1-cross-attempt` failure (completed instead
   of refusal); reproduce/compare it without weakening the assertion,
   repairing the engine or calling the full proof green. Independently run
   its existing `resolved_matrix` function. A known baseline failure is not
   a gate waiver or dispatcher-wide safety proof.

CI owns macOS, privileged port-80, chart/history, Docker, network RustSec
and tag-only proofs per the house inventory. The operator owns Spark host
availability, credentials, live settings, any compatibility authorization
and branch sealing. These are ownership notes, not new local acceptance
criteria. No host/network probe, namespace proof, publication or acceptance
of any decision is authorized.
