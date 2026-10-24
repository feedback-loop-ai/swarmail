# Plan: Fast triage on Opus medium

## Technical context

One Rust crate; the implementation surface is JSON office configuration,
Markdown contracts and existing Rust/Python delivery tests. Brokkr composes
bundles and resolves aliases through adapters. Reuse Claude's existing `opus`
mapping and medium effort support. No production code, dependency, migration,
new provider or new script is needed. The repository supplies only a spec
template, not a plan template; these sections implement its design dialect.

## Constitution check

Accepted [0057](../../docs/decisions/0057-codex-delivery-recipes.md) receives
only the operator-authorized dated addendum. Accepted
[0058](../../docs/decisions/0058-codex-review-routing.md) stays authoritative.
No proposed decision is declared. Preserve frozen 0001–0003 and ingest-time
extraction under 0005, every protected gate, exact coverage, 0020 test
placement, candidate binding and security hold. No decision index edit or
reserved-number allocation is authorized.

The current chief instruction explicitly requires the design commit identified
by [0050](../../docs/decisions/0050-delivery-gates.md). Apply that authorization
only to this directory's spec.md, plan.md and tasks.md; commit unsigned with an
imperative subject and rationale body, and keep that commit last for the final
check. The spec's dated clarification records this ruling over the prior draft's
uncommitted-artifact approach. Implementation commits remain ship-only. The
existing untracked intake remains outside this spec directory and outside that
commit. The dated 0057 append belongs to implementation, not the design commit.
No new decision declaration or proposed record is appropriate for an amendment
the operator explicitly accepted and directed into the existing 0057 record.

## Research and evidence

Both positions and the matching intake were read; synthesis decisions are in
spec.md. Inspection confirms the triage office currently has `[sol]`/medium,
the Claude adapter already supports `opus -> claude-opus-5-5`, and the charter
still names Sol and four reviewers. Fast implementation is already Luna/xhigh;
its broad Rust selector test and Python all-implementers-Sol assertion are stale.
Python resolves self but currently skips its comparison. The `recipes` symlink
points to `bundles`, so edit canonical documentation once.

Use the candidate-local `.forge/bin/brokkr` and compare its digest to the actual
`bundles/self/tooling.json` record before any engine proof. Do not copy an old
spec's engine hash or use a sibling binary. Recipe manifest agent summaries
expose provider/chain, not every effort or concrete model field. Read those
from composed office and adapter definitions at the appropriate layer; do not
invent manifest properties or infer provider from an office name.

## Project structure and files touched

| Path | Planned change |
| --- | --- |
| `agents/codex-triage.json` | Singleton opus/medium; remove Sol effort; correct provider-specific hands explanation if needed, preserving the hands object |
| `agents/charters/codex-triage.md` | Opus 5.5/medium without fallback; five-role design/engine descriptions; preserve predicates and evidence/result instructions |
| `docs/decisions/0057-codex-delivery-recipes.md` | Append dated accepted clarification, no historical rewrite |
| `bundles/README.md` | Correct both Sol-triage claims; add existing Opus mapping to current table; retain Luna and accepted 0058 routes |
| `agents/README.md` | State triage/implementation split and correct nearby outdated 0058 status wording |
| `tests/delivery_recipes.rs` | Exact office/adapter/effort and preservation assertions; correct stale fast selector expectation |
| `tests/codex_review_routing.py` | Complete resolved work/review matrix, self equality and meaningful negative controls using existing helpers |
| `specs/fast-opus-triage/{spec,plan,tasks}.md` | This design and paired proof obligations |

Read `docs/house-rules.md` and root README for current contradictions; edit only
if a concrete statement conflicts with this amendment, documenting why. No
blanket text replacement. Keep 0058, the decision index, historical specs,
adapters, policies, engine pin and proof scripts unchanged. Tests may read
unchanged baseline files to establish preservation.

## Implementation approach

1. Capture current recipe manifests and unchanged route definitions as run-local
   evidence with baseline commit, engine digest and attempt identity. Expected
   routes in tests must be explicit; comparing the candidate to itself proves
   nothing. Add specific legacy-self chain/effort expectations.
2. Make the office substitution and focused charter/documentation corrections.
   Append the 0057 addendum last in that record, explicitly superseding its
   earlier Sol/medium sentence while preserving the Luna clarification.
3. Extend existing Rust structural tests. Assert complete model/effort objects,
   adapter resolution, retained sandbox and classification/selector constraints.
   Keep all fast implementation cases Luna/xhigh and existing review counts.
4. Extend the Python resolved matrix to include triage, intake, implementation,
   shared design, legacy self and preserved proof ownership. Combine manifest
   provider/chain evidence with office/adapter identity and effort evidence.
   Resolve each fallback alias individually: feature correctness's manifest
   primary provider is dsh but its Sonnet fallback is Claude. Compare whole
   role sets, not counts or substring membership.
5. Exercise the same equality predicate on valid triage and mutations of
   provider, model, effort and added fallback. Retain reviewer mutations and
   engine-backed adverse-result, fallback-boundary and foreign-attempt probes.
   Static equality establishes singleton selection; existing actual engine
   probes establish dispatch/evaluator semantics. Do not write a second policy
   evaluator or relabel controlled participants as provider review.
6. Run focused proofs, all four compiles and the complete house verification.
   Follow with five real 0058 reviews on the identical candidate. Record failures
   precisely; return scope/engine problems through the existing mechanism.
   Do not broaden this amendment into a runtime, engine or gate repair.

## Data model and contracts

No mail schema, storage or public API change. Existing entities are recipe,
strategy, office, ordered model chain, effort map, adapter, result and candidate
identity. The only configuration value change is triage's model/effort mapping;
there is no secondary choice. All existing classification results, attempts,
timeouts, scope returns and result fields (`result`, `inputs`, `notes`) remain.
Unknown strategy or malformed facts still refuses advancement. Preserve
separate run/attempt/member result identities and the verifier's candidate fence.

## Quickstart and validation

For design structure, use the pinned final checker. A local diagnostic is:

```bash
bash bundles/self/scripts/design-check.sh check .
```

Run this after the actual three-file design commit, then verify its exact path
set and this directory's clean worktree. Do not weaken the check or use a
throwaway commit as evidence. The engine's pinned final check owns advancement.

After implementation, run the existing focused suite and compile each recipe:

```bash
cargo test --locked --test delivery_recipes
.forge/bin/brokkr compile --bundle bundles/self
.forge/bin/brokkr compile --bundle bundles/codex
.forge/bin/brokkr compile --bundle bundles/codex-careful
.forge/bin/brokkr compile --bundle bundles/codex-fast
python3 tests/codex_review_routing.py --engine "$PWD/.forge/bin/brokkr" --out .forge/evidence/opus-triage-routing-ATTEMPT
```

Replace ATTEMPT with a fresh attempt identifier; retain earlier evidence.
Use `recipes show` for self, codex, codex-careful and codex-fast through the same
engine. No compile or policy-sweep warnings are accepted.

The single mandatory gate inventory is
[house rules](../../docs/house-rules.md#gates--part-of-every-change), not a reduced
copy here. Execute all of it with `--locked`, exact coverage and selftest,
publish/seal proofs and licenses, plus the verifier's fail-closed native-TLS
absence proof and design/delivery proof selftests. Validate public shim hashes
against pinned expectations and execute the pinned copies through the existing
verifier. The full suite includes real-server runtime tests; no new protocol
mock, sleep synchronization or coverage exclusion is justified. CI-owned chart,
Docker, RustSec, release-rate and privileged-port proofs remain explicitly CI-owned.

Record candidate commit/tree, relevant dirty-file content identity before ship,
engine digest, command exit statuses and logs in fresh ignored evidence. Final
verification/review binds to the exact candidate using the existing workflow;
if that workflow cannot attest an uncommitted candidate under the ship-only
restriction, surface the conflict rather than invent a substitute fence.
Request five actual provider reviews through the delivery engine, preserving
separate Astra correctness/guarantees results. Any tree change requires renewed
applicable proofs and review. No live-review claim follows from Python fixtures.

## Risks

| Risk | Mitigation and accepted cost |
| --- | --- |
| Design authorization is mistaken for permission to commit implementation early | Limit the explicit chief commit to the three design files; retain ship-only implementation commits and independently run the unchanged design check |
| Opus unavailable or incompatible at dispatch | Existing bounded refusal with no alternate model; accept reduced availability and report missing live evidence |
| Human instructions still name Sol or four reviewers | Correct current charter and route summaries together; preserve classification predicates and history |
| A membership/count test admits wrong routes | Whole tuple and role-set equality with mutations through the same predicate |
| Preserved implementation regresses to satisfy stale tests | Fix tests to accepted Luna/xhigh and compare every strategy plus Standard/Careful routes |
| Self is silently changed or ignored | Explicit legacy chains, effort/provider expectations, shared design and proof ownership comparisons |
| Fallback aliases hide wrong provider or effort | Resolve each alias, retain official Flash destination and effort-free DashScope, refuse Pro |
| Availability fallback hides a bad verdict | Preserve actual dispatcher refusal probes and scope/security controls; no triage fallback |
| Existing coverage or attempt-association failures recur | Retain failing evidence and stop delivery; no expectation weakening or unsolicited engine/runtime repair |
| Controlled proofs mistaken for real review | Separate five actual invocations/results and candidate identities from fixture evidence |
| Numbering or acceptance history drifts | Append only to accepted 0057; unchanged index/other decisions checked by diff |
| Narrow patch encourages skipped runtime gates | Full house inventory remains mandatory; frozen guarantees and ingest extraction unchanged |
