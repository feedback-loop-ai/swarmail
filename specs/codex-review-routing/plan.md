# Plan: Codex review routing

## Technical context

This is configuration and delivery-contract work in one Rust repository.
JSON bundles compose existing agent offices; adapters resolve model aliases;
the recorded brokkr engine owns dispatch, aggregation, policy and script origin.
Rust integration tests under `tests/` guard the delivery definitions. No new
production code, crate, dependency, storage entity or public mail API is needed.

Use the recorded composite in `bundles/self/tooling.json`, not a binary selected
only by version: base `597f4868e3fb971b62e6a368a2d8aa23271f4531`, its three ordered
patches, and binary digest `e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`.
Validate against the actual record again before execution. No incidental pin
change is authorized. The repository contains only a spec template, not a plan
template; this plan supplies the sections required by the speckit design dialect.

## Constitution check

[0058](../../docs/decisions/0058-codex-review-routing.md) proposes only the
operator-directed amendment to [0057](../../docs/decisions/0057-codex-delivery-recipes.md).
It remains proposed; implementation must retain the distinction between the
explicit current directive and operator acceptance of a new record. Any change
beyond that exact directive which contradicts an accepted ruling waits for the
operator under 0045 §8. This design grants no new semantic authority.

Preserve protected review, security hold, every shell gate and its pinned origin,
unknown-fact refusal, bounded returns and candidate binding (0007/0009/0013,
0045/0050). Keep 0001–0005, coverage 100/100/100 exact, licenses and 0020 test
placement. Commit unsigned under 0056; never push, merge or close issues.

The chief commit contains exactly spec.md, plan.md, tasks.md and proposed 0058.
Consequently, the decision-index row and 0057 pointer are implementation tasks.
The existing `store_contract_docs` index-completeness test will reject the
intermediate unindexed proposal; do not call that an implementation-ready or
all-gates-green tree. The final design validator checks the four committed
artifacts; implementation must add the index row before its own full gate run.

## Research and evidence

Read the intake and both positions for run
`update-the-codex-delivery-recipe-c79c3f2d`; the synthesis rulings are in spec.md.
The inspected existing definitions establish:

- Standard has four roles; Careful inherits them; fast feature shares Standard
  correctness/security; fast design/engine repeat the four-role panel.
- Existing aliases already name Sonnet 5.5, Fable 5.1, Astra, Qwen Max and Flash.
  DashScope's adapter requires omitted effort. No new provider is needed.
- The correctness charter covers the full diff, criteria, guarantees and proof,
  and excludes security-hold. It is suitable for independent Fable by inheritance.
- `recipes` is a relative symlink to `bundles`; there is no separate recipe copy.
  Existing discovery/CI enumeration remains valid because no recipe is added.
- `tests/delivery_recipes.rs` currently asserts four roles, equality of full
  panels and Careful inheritance; those assertions must be replaced.
- The position authors found no tracked older fallback acceptance. Cite the
  present directive, never reconstruct a historical receipt.

Outstanding research belongs first in implementation: locate the actual pinned
engine's supported dispatcher/aggregator/evaluator proof interface and determine
its dispatch-failure classification. Execute controlled failures through that
interface, or a harness linked to that exact engine core. Do not write a second
policy evaluator or infer dispatch semantics from JSON. If no faithful seam
exists, report the exact unavailable proof and bounded engine correction needed;
this plan does not preauthorize a pin or gate rewrite. Live compatibility is a
separate evidence obligation; ordinary regression tests must not launch paid
provider work or claim controlled outcomes are live measurements.

## Project structure and touched files

| Path | Intended implementation change |
| --- | --- |
| `bundles/codex/bundle.json` | Three-role Standard review and accurate description; retain seat contract metadata. |
| `bundles/codex-careful/bundle.json` | Add explicit review override with all five roles; keep intake and Sol xhigh work overrides. |
| `bundles/codex-fast/bundle.json` | Dedicated Qwen feature correctness; explicit five-role design/engine panels; accurate description. |
| `agents/codex-review-{correctness,security,tests,guarantees}.json` | Single-model Fable, Flash, Sonnet, Astra respectively, with supported high effort. |
| `agents/codex-review-astra-correctness.json` | New overlay of `review-correctness`, single Astra/high; distinct from guarantees. |
| `agents/codex-review-feature-correctness.json` | New overlay of `review-correctness`, Qwen then Sonnet, only Sonnet/high effort. |
| `agents/codex-review-independent.json` | Tiny inheritance alias of `codex-review-correctness`; one independent Fable member, no copied charter. |
| `agents/charters/review-{correctness,security,guarantees,tests}.md` | Only necessary companion/count wording corrections for variable panel sizes; preserve lenses, read-only and typed result obligations. |
| `tests/delivery_recipes.rs` | Exact per-lane matrix, routes, effort, office/charter identity, fallback scope and negative controls. |
| `tests/delivery_constitution.rs` | Preserve protection pins; extend only where new panel shapes need proof. |
| `docs/decisions/README.md`, `docs/decisions/0057-codex-delivery-recipes.md` | Index 0058 with actual status; append dated, exact amendment pointer to 0057 without rewriting historical body. |
| `bundles/README.md`, `agents/README.md`, `docs/house-rules.md` | Update current delivery summaries for matrix, mapping and sole fallback exception. |

Existing adapters, fast policy, self bundle, public shims, pinned scripts,
`.github/workflows/ci.yml`, application sources and manifests are preservation
inputs, not planned edits. If a real engine harness needs an additional test
file, keep it test-only and narrowly named for this proof; no production engine
copy, application dependency or gate rewrite. The exact dispatch harness entry
point must come from supported recorded-engine interfaces, not an invented CLI.

## Implementation approach

1. Capture resolved bundles/offices, policy, script origins, tooling identity and
   protected contracts as baseline run evidence. Resolve all four recipes and
   each fast case using the real engine. Prove the fallback boundary before
   relying on its model-chain representation.
2. Introduce the three isolated offices above. Set only the directed model/effort
   overlays. Explicitly replace inherited model arrays, so `glm-flash` does not
   leak in from legacy charters. Fable independent inherits correctness's full
   read-only contract; it never extends the combined `reviewer` charter.
3. Compose Standard's three roles and the three explicit five-role declarations.
   Keep review results/inputs/attempts/timeouts unchanged. Keep both Astra member
   identities and result destinations; never deduplicate by model. Leave chore
   unchanged and feature with two roles. Make no strategy/policy changes.
4. Replace stale assertions with AC1–AC9 proofs; use static assertions for file
   contracts and actual engine resolution/execution for behavior. Reuse existing
   safety/candidate-binding proof machinery. Add controlled outcomes at the
   provider boundary, not mocks of mail-server protocol behavior.
5. Update the exact scoped decision pointer/index and current summaries. Preserve
   accepted historical bodies, legacy routing, canonical journal/isolation rules
   and all prior gate inventory text.
6. Execute targeted tests, engine proofs, compilation and the full house gates.
   Separate unavailable or failing checks from successful checks. Leave no
   claimed-complete implementation with outstanding acceptance criteria.

## Data model and contracts

No application data migration or public schema changes. Existing engine entities
remain: recipe, strategy selector, office, role charter, ordered model chain,
review result and verified candidate identity. Role keys are exactly those in
the spec matrix; `independent` adds a member, not a new result vocabulary.

The result contract stays `result`, `inputs`, `notes`. Existing review vocabulary
and input keys are unchanged. Every invocation gets its own run/attempt/member
identity and result path. Fallback retains one correctness role: the engine
records attempted models, not two alternative verdicts to choose between.
Missing/invalid safety or scope facts remain unknown, never false by default.
No result from another candidate, run or member can satisfy the required result.

## Quickstart and verification

After implementing, run:

```bash
cargo test --locked --test delivery_recipes --test delivery_constitution --test store_contract_docs
```

Compile `self`, `codex`, `codex-careful` and `codex-fast` using the executable
whose hash matches the recorded tooling. Run controlled dispatcher and policy
cases from AC5/AC7–AC9 through that same engine, retaining red/green evidence
under ignored run state. Unknown selectors must refuse; the feature fallback
must be isolated.

Then execute the complete, single canonical inventory in
[house rules](../../docs/house-rules.md#gates--part-of-every-change), preserving
its locked Cargo commands, coverage/selftest, publish/seal proofs, license check,
native-tls refusal and CI-only proof ownership. Also retain actual pinned design,
verify and candidate-binding execution proofs under 0050; passing source-level
assertions is not a substitute. No new unconditional paid-provider test is added.
For this design alone, validate its final HEAD with
`bash bundles/self/scripts/design-check.sh check .`; the sequence's pinned final
check remains the authority. Keep the design commit last.

## Risks

| Risk | Mitigation / accepted cost |
| --- | --- |
| Three five-role declarations drift or inherit Standard's smaller panel | Assert exact resolved equality across Careful and both full-risk cases; accept small JSON repetition rather than add a recipe or generator. |
| Shared correctness changes erase Qwen from feature | Dedicated feature office with exact chain; negative control for inheriting Standard. |
| Fallback masks adverse findings or malformed facts | Execute real dispatcher outcomes; restrict exception to dispatch failure; fail closed and expose unsupported engine behavior. |
| Security accidentally routes to a different endpoint or loses hold | Resolve provider and model, preserve security charter and execute hold/unknown fact cases on each panel shape. |
| Two Astra roles are counted as one or share a result | Separate office/charter/result identities; negative missing/merged-member controls. |
| Independent Fable becomes cosmetic or gets excessive authority | Tiny alias inherits full correctness lens, separately dispatched; residual/missing-result tests; never inherit combined reviewer. |
| Five roles still share model blind spots | Retain independent Fable and honest role-vs-model language; accept five dispatches and remaining correlation. |
| Gate origin or candidate binding changes through inheritance | Before/after compiled comparison plus pinned execution and stale/wrong-candidate refusal proofs. |
| New proposed decision is mistaken for accepted law | Preserve proposed status, date/source and historical bodies; operator alone accepts; index update is an explicit downstream task. |
| Full validation unavailable in boxed design environment | Report actual availability/results; implementation and verify still owe every gate and engine proof, with no inferred success. |
