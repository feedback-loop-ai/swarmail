# Plan: Fast triage on Opus medium

## Technical context

One Rust crate; changes are an existing JSON office overlay, Markdown and
existing Rust/Python delivery tests. Reuse the existing Claude adapter and
model key. No production Rust, dependency, migration or new provider. Only a
spec template exists under `.specify/templates`; this plan supplies the
technical context, constitution check, structure, research, data model,
contracts and quickstart required by the spec-kit dialect.

## Constitution check

Accepted [0057](../../docs/decisions/0057-codex-delivery-recipes.md) receives
only the directed dated append; accepted
[0058](../../docs/decisions/0058-codex-review-routing.md) remains unchanged.
No proposed decision is needed or declared: the operator explicitly directed
this existing-record amendment. No frozen guarantee or 0005 behavior changes.
Retain exact coverage, all gates, protected review and hard security hold.

The chief commits exactly the three design files, leaving existing intake and
positions untouched. Implementation commits all authorized work before return,
per the current task and the spec's 2026-10-25 clarification. Commit unsigned,
never push. Later verification/review binds to that candidate; design proof
alone does not authorize delivery or certify the future implementation.

## Research and evidence

Use current [intake](intake.md), both panel positions and the checksum-verified
operator packet; the historical patch is reference only. Baseline is
`d7e615d5e94ac28aa84d5a825c95abf1cd436997`. Main already has correct Luna/xhigh
expectations. The previous design's duplicate-index/stale-test findings do not
apply. The run-local intake directory is absent, but tracked intake matches
the run. Preserve existing dated clarification and explicitly supersede its
obsolete timing/scope text, as spec.md does.

The candidate-local engine digest must equal
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f` and the
unchanged tooling record. `recipes show` exposes office/provider/chain;
`agents show codex-triage` exposes composed models, efforts, resolution chain,
hands and limits. Concrete model identity comes from the existing adapter
mapping; do not invent a concrete-model or effort field in recipe summaries.
A resolution status of ok with presence unknown is configuration evidence,
not a live-provider compatibility or quality result.

## Project structure and files touched

| File | Implementation change |
| --- | --- |
| `agents/codex-triage.json` | Replace singleton models and complete effort map; optionally make provider-specific explanatory hands text accurate without changing hands |
| `agents/charters/codex-triage.md` | Opus 5.5/medium/no fallback and five-role full-risk wording; retain all classification and result instructions |
| `agents/README.md` | Current fast triage/implementation split |
| `bundles/README.md` | Both current Sol-triage statements and existing model-table documentation |
| `docs/house-rules.md` | Name Opus/medium/no fallback alongside Luna/xhigh; preserve the gate inventory |
| `docs/decisions/0057-codex-delivery-recipes.md` | Append dated operator clarification; retain historical prefix and accepted status |
| `tests/delivery_recipes.rs` | Exact model/effort/adapter, preserved controls and intake/triage absence assertions |
| `tests/codex_review_routing.py` | Extend existing resolved matrix, exact triage negative controls and baseline preservation checks; keep dispatcher fixtures unchanged |

This phase changes only `specs/fast-opus-triage/{spec,plan,tasks}.md`.
No adapter, bundle selector, engine, policy, proof-script, CI, index or
application file change is needed. Reuse existing helpers rather than creating
an abstraction or test suite. Baseline comparisons may read unchanged files.

## Implementation approach

1. Record baseline commit/tree, engine digest, four recipe manifests and
   relevant composed offices in fresh ignored evidence. Use literal expected
   tuples, independent of candidate configuration. A baseline comparison
   proves preservation, not that an old defect has been fixed.
2. Change only triage models/efforts and bounded documentation. Append 0057's
   operator clarification with directive date 2026-10-24, explicitly superseding
   its earlier fast-triage sentence; retain Luna and 0058 amendments.
3. Extend the Rust triage guard to whole-object equality and the unchanged
   adapter concrete mapping/medium support. Add Standard/Careful absent triage
   and exact intake office/effort checks; keep Luna and review tests intact.
4. Extend Python's existing resolved matrix to include fast triage, preserve
   Standard/Careful intake and absent triage, and fast absent intake. Resolve
   office effort through `agents show`; combine it with recipe selection and
   adapter mapping into a small explicit record. Check whole chains, not
   membership. Preserve review equality and implementation checks, add missing
   effort assertions where needed, and compare existing self baseline routes
   rather than skipping them. Keep shared design/proof ownership guarded by
   existing Rust tests and unchanged-file comparison.
5. Mutate missing triage, provider, alias, concrete mapping, effort and extra
   fallback independently through the same equality comparison. The real
   actual record must pass; every mutated record must fail. Do not alter any
   dispatcher fixture or cross-attempt assertion. No alternative evaluator,
   live model call or provider retry mechanism is needed.
6. Commit the implementation; run focused proofs and every house gate. Save
   separate route-only and full routing-proof results. If the latter reproduces
   the known defect, report it unchanged, without re-pin, skip or weakened
   expectation. Actual delivery review remains with the phase engine.

## Data model and contracts

Existing objects remain recipe, strategy, office, ordered model chain, effort
map, adapter and result. Only the fast triage selection changes to `[opus]`
and `{"opus":"medium"}`. No data schema, public API or migration. Preserve
three attempts/3600-second inherited limits, workspace hands/masks, result
vocabulary, classification predicates and bounded escalation. Opus dispatch
failure uses existing failure policy with no alternate model. No new semantics
are claimed for the engine's known attempt-association defect.

## Quickstart and validation

After the chief's exact three-file commit:

```bash
bash bundles/self/scripts/design-check.sh check .
git diff-tree --no-commit-id --name-only -r HEAD
git status --short -- specs/fast-opus-triage
```

For implementation, use the candidate-local engine and fresh output paths:

```bash
sha256sum .forge/bin/brokkr
cargo test --locked --test delivery_recipes
.forge/bin/brokkr compile --bundle bundles/self
.forge/bin/brokkr compile --bundle bundles/codex
.forge/bin/brokkr compile --bundle bundles/codex-careful
.forge/bin/brokkr compile --bundle bundles/codex-fast
python3 tests/codex_review_routing.py --engine "$PWD/.forge/bin/brokkr" --out .forge/evidence/opus-triage-routing-ATTEMPT
```

Run the same matrix independently for a separate, explicitly route-only verdict:

```bash
python3 - <<'ROUTES'
from pathlib import Path
import runpy
proof = runpy.run_path('tests/codex_review_routing.py')
out = Path('.forge/evidence/opus-triage-matrix-ATTEMPT')
out.mkdir(parents=True, exist_ok=False)
proof['resolved_matrix'](str(Path('.forge/bin/brokkr').resolve()), out.resolve())
ROUTES
```

Replace ATTEMPT with a fresh identifier. The full proof still runs unchanged;
route-only success cannot substitute for its verdict. Four compiles must have
no policy-sweep warnings. Do not use the older patch's engine changes.

The authoritative inventory is [house rules](../../docs/house-rules.md#gates--part-of-every-change).
Run every gate with locked Cargo commands, exact coverage and selftest,
publish/seal proofs and licenses. Validate shims against the pinned manifest;
execute pinned bundle copies as specified by `verify-seat.sh`. Include its
design-check/delivery-proof selftests and fail-closed native-TLS absence proof.
Use offline cached tooling, install nothing, and preserve failures with exact
missing inputs or decisive diagnostics. The full suite supplies real-server
runtime proofs; no extra protocol test is justified by a routing-only edit.

Store commands, exits, full logs, commit/tree identity and engine digest in
ignored run-local evidence. A rerun on a new candidate must have a new evidence
path. CI-only checks and actual five-role review retain the owners stated in
spec.md; no local platform/network acceptance condition is invented.

## Risks

| Risk | Mitigation and accepted cost |
| --- | --- |
| Opus outage, latency, cost or quality differs from Sol | Existing bounded failure; no fallback or unrequested comparative trial; local checks prove identity only |
| Minimal source test misses composed fallback/effort | Actual recipe and office resolution, full-chain equality and independent negative controls |
| Current charter still instructs four reviewers | Correct nearby counts to accepted 0058 while preserving all risk predicates |
| Stale patch regresses Luna or imports engine fixes | Re-derive on this baseline; review exact path diff, engine/pin identity and unchanged controls |
| Historical Sol statement confuses readers | Explicit dated supersession in 0057; current summaries agree; preserve provenance |
| Preserved self or intake routes drift unnoticed | Exact baseline comparisons, absent-triage assertions and existing Rust inheritance guards |
| Cross-attempt defect is mislabeled green or becomes new scope | Keep full proof unchanged, retain red evidence, separate route-only verdict; no engine repair |
| A fixture is reported as five-role provider review | Delivery engine owns distinct actual review results on the verified candidate |
| Gate unavailable or pre-existing failure recurs | Record exact refusal; do not waive, invent success or repair unrelated systems |
| Stale ship-only timing prevents committed handoff | Apply current explicit commit instruction and spec's dated reconciliation; no external actions |
