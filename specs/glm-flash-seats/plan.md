# Plan: GLM Flash tests review

## Technical context

One Rust crate; this change uses existing JSON office inheritance and dsh
routing, Rust source assertions and Python resolution through the pinned
CLI. No application source, dependency, data migration or new API is needed.
The repository has a spec template but no plan template; this document
provides the dialect's technical context, constitution check, project
structure, research, data model, contracts, quickstart and risks.

## Constitution check

The operator's revised 2026-10-26 direction authorizes only the tests routing
amendment. 0081 remains proposed; do not infer any further acceptance.
Accepted 0057/0058 govern everything else. Preserve frozen 0001–0003,
extraction-at-ingest 0005, deterministic real-protocol tests under 0004/0020,
exact coverage, dependency policy, protected review, security hold and
candidate binding. Preserve adapter capability truth and workspace hands.
Use pinned proofs per 0050; unsigned commits per 0056, never push.

The chief commit touches exactly the existing spec.md, plan.md, tasks.md and
proposed 0081. The decision already has an index row, so no new numbering or
inventory entry is needed. Its old row description and current route prose
are implementation updates; until then route prose describes unchanged
routing. The old implementation report is historical under this spec's dated
clarification. Final implementation must remove those stale current claims.

## Research and design choices

Both current panel positions and both task framings were read. Required
README and revised rulings checksums match; the README has no prior-run
entries and still describes the superseded request. The revised ruling
controls. Engine digest matches the required value. Baseline for preserved
routing is `d55a89de70f4a4e6cc598d3726eb2fa55bc2df76`; intake subsequently
committed the revised framing and verification receipt without routing edits.

`agents/codex-review-tests.json` currently selects Sonnet/high and extends
`review-tests`. Its parent carries Fable/GLM with Fable effort. Explicit
`efforts: {}` avoids inherited metadata; preserve the overlay rather than
selecting the parent's fallback chain. The existing dsh adapter already maps
Spark, marks it local and effortless, and needs no change. All four panel
placements already name this office, so topology needs no edit. The prior
blocked intake fixture is historical evidence, not a prerequisite to
reproduce or repair for this revised task. Both panels support this cut.

Choose robustness's exact effective-route assertions and negative controls
inside simplicity's two existing proof files. Source-only replacement is
insufficient; a new generic routing framework is unnecessary. Keep baseline
comparison as fresh run-local evidence. A chief disposable fixture with only the tests overlay changed compiled all
four recipes without policy warnings and resolved this office ok on dsh with
empty efforts. The manifest omits effort: treat omission and JSON null as
absence, rejecting every explicit value. This validates design feasibility,
not committed implementation or provider dispatch. The known full dispatcher failure
must remain distinct from focused route success and from house-gate results.

## Project structure and touched files

| Path | Implementation responsibility |
| --- | --- |
| `agents/codex-review-tests.json` | Only executable routing edit: singleton glm-flash and explicit empty efforts; preserve extends and all other fields. |
| `tests/delivery_recipes.rs` | Exact office-specific model/effort assertions, Spark mapping/local/effortless metadata; unchanged charters, hands and panels. |
| `tests/codex_review_routing.py` | Extend resolved_matrix for the actual tests office in all four placements, exact status/provider/concrete/local/effort tuple and mutation controls. Preserve full dispatcher assertions. |
| `agents/README.md`, `bundles/README.md`, `docs/house-rules.md` | Reconcile current tests-review descriptions and effort; keep intake and gate inventory intact. |
| `bundles/codex/bundle.json`, `bundles/codex-careful/bundle.json`, `bundles/codex-fast/bundle.json` | Stale description text only; preserve composition, selectors and policy. |
| `docs/decisions/0081-glm-flash-seats.md` | Chief's revised tests-only proposed record; never accept. |
| `docs/decisions/README.md` | Update existing 0081 row's scope/citation, preserving proposed status. |
| `specs/glm-flash-seats/implementation.md` | Add dated supersession notice for the old blocked attempt, preserving findings and provenance. |
| `docs/decisions/0057-codex-delivery-recipes.md`, `docs/decisions/0058-codex-review-routing.md` | Only dated amendment pointers if needed; preserve accepted bodies. |

Other executable old tests-route pins found by scoped search may be updated
in place; do not globally replace Sonnet. Preservation inputs include both
intake offices, every other office/base/charter, dsh adapter, bundle policies,
self, tooling/engine, gate scripts/shims, Cargo files and all `src/`.

## Implementation approach

1. Verify authority and engine digests; record starting revision. Capture
   full source/resolved routing and charter hashes against baseline. Use a
   disposable export of tracked baseline files if needed, never a sibling
   worktree or canonical journal. Search for current old-route pins.
2. Change the one overlay. Resolve its effective office and all recipes,
   assert `ok`, singleton dsh/Spark/local route, empty office effort and null
   compiled effort. Compile all four bundles before trusting the substitution.
3. Adapt the Rust reviewer loop to expect an exact empty map only for tests;
   retain exact high-effort maps elsewhere. Pin the existing dsh metadata.
   Extend Python with one local route comparator for selected manifest and
   effective office values, retaining exact full-matrix membership checks.
4. Feed copied mutated observations through that same comparator, one fault
   at a time: old Sonnet, provider, concrete model, local class, blocked
   status, fallback, effort, missing/replaced member. Every mutant must fail.
   This proves assertion sensitivity, not live provider execution.
5. Compare all non-target routes/fields and protected boundaries against
   baseline. Preserve distinct results, triage selectors, official Flash,
   independent Fable and sole pre-result Qwen-to-Sonnet fallback. No health
   probe, retry change or Spark fallback is introduced.
6. Reconcile current docs/index and label prior blocked evidence superseded.
   Commit implementation before final proof. No tracked provenance links
   into ignored evidence; the revised ruling is quoted in 0081.
7. Run focused resolution and the full dispatcher separately, all compile
   checks and local gates; record candidate/engine identities, exits and
   fresh output paths. Report red and unexecuted controls explicitly. No
   baseline failure authorizes weakening a control or changing the engine.

## Data model and contracts

No schema change. The entities remain office identity, inherited charter,
model chain, efforts map, adapter route and compiled candidate tuple.
Empty office effort is `{}`; compiled absence is an omitted or null effort
field, normalized to null by the comparison; human wording is
“not applicable.” Route class local is metadata, not measured reachability.
Tests review remains gate-class/read-only with separate required result,
existing limits and candidate-change refusal. Existing result/inputs/notes
and safety/scope/fixes facts keep their vocabulary and validation. Preserve
missing/invalid-result refusal and bounded failure behavior. The recorded
cross-attempt proof gap prevents a claim of dispatcher-wide enforcement.

## Quickstart and verification

Use only `.forge/bin/brokkr`, whose SHA256 must be
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
Work offline, install nothing; set `CARGO_NET_OFFLINE=true` and prepend
`$PWD/.forge/bin` to PATH. Verify `bundles/self/scripts/shims.sha256` with
`sha256sum --check --strict` before executing pinned equivalents of the
house gates. Use candidate bundles as they stand.

```bash
cargo test --locked --test delivery_recipes --test delivery_constitution --test store_contract_docs
.forge/bin/brokkr agents show codex-review-tests
.forge/bin/brokkr compile --bundle bundles/self
.forge/bin/brokkr compile --bundle bundles/codex
.forge/bin/brokkr compile --bundle bundles/codex-careful
.forge/bin/brokkr compile --bundle bundles/codex-fast
python3 tests/codex_review_routing.py --engine .forge/bin/brokkr --out .forge/proofs/glm-tests-full-UNIQUE
```

Independently invoke the existing focused proof; replace UNIQUE with a fresh
suffix on every execution and retain full-script outcomes alongside it:

```bash
python3 - <<'PROOF'
from pathlib import Path
import runpy
proof = runpy.run_path('tests/codex_review_routing.py')
out = Path('.forge/proofs/glm-tests-resolved-UNIQUE')
out.mkdir(parents=True, exist_ok=False)
proof['resolved_matrix'](Path('.forge/bin/brokkr').resolve(), out.resolve())
PROOF
```

Run the entire canonical [house inventory](../../docs/house-rules.md#gates--part-of-every-change):
fmt, locked clippy/workspace tests, self compilation, exact coverage and its
selftest, publish guard, keyless seal proof and licenses. Use pinned scripts
under `bundles/self/scripts/` after shim verification. Also run design-check
selftest and delivery-proof-test. For native-tls absence, run `cargo tree -i
native-tls` and accept only nonzero exit with “did not match any packages”;
success or any other error refuses. Retain compile warnings as failures.

The baseline full Python script stops at `1-cross-attempt`, engine completed
instead of refusal; later cases do not run. Reproduce/report this separately
without calling the script green. The operator assigned correlation repair
to separate engine work before any re-pin. House/focused green is narrower
than full dispatcher green. Disposable controlled proof journals are allowed;
canonical journal, GitHub, CI, host endpoint and sibling worktrees are not.

For the chief's final HEAD run `bash bundles/self/scripts/design-check.sh
check .`; keep the four-file design commit last and clean. Run every requested
local gate on that design commit too, recording outcomes only in ignored
run evidence and the result receipt. These gates exercise unchanged routing;
implementation and final route acceptance remain later-phase obligations.
CI/operator ownership is as stated in the spec, with no new local conditions.

## Risks

| Risk | Mitigation and accepted cost |
| --- | --- |
| Old intake blocker accidentally persists as a requirement | Explicit dated scope replacement in spec/0081; preserve paid Sol intake and its hands, label old report during implementation. |
| Inherited effort or parent's fallback leaks into Spark | Keep overlay, set explicit empty map, assert effective office and compiled candidate, reject effort/fallback mutants. |
| Alias or CLI exit hides wrong/blocked route | Compare exact provider/concrete/local/status tuple and exercise negative controls. |
| Shared edit silently affects other seats or panel shape | Full source/resolved baseline comparison, exact memberships and unchanged charters/permissions/efforts. |
| Extra proof machinery creates another routing owner | Extend two existing suites, use one local comparator and run-local baseline evidence only. |
| dsh read-only charter mistaken for native confinement | Preserve unsupported adapter declarations and candidate fence; no enforcement-equivalence claim or added tools. |
| Spark outage or review-quality variation | Existing limits, independent roles and shell proofs remain; no paid fallback, quality-equivalence claim or seat network probe. |
| Baseline correlation defect hidden by focused green | Preserve failing assertion, record full/focused exits separately and list unexecuted cases; no engine repair in this task. |
| Four-file design restriction leaves stale summary prose | Spec declares phase boundary; ordered implementation documentation task updates index, summaries and old report before final acceptance. |
| Proposed amendment mistaken for broad acceptance | Quote precise revised operator direction; retain proposed status and historical accepted bodies. |
| Evidence collisions | Fresh output directories with candidate and engine identities; never overwrite prior receipts. |
