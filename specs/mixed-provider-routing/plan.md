# Plan: mixed-provider routing verification

## Technical context

Single Rust crate, existing JSON recipes/offices/adapters, Rust configuration
and CI tests, Python effective-resolution and controlled-dispatch proof.
Use candidate `c2f76e26989bbd5eb7aabcf28f57e4ef8073d136` and its parent as
provenance anchors. No production Rust, dependency, lockfile, API or storage
migration is required. The repository has a spec template but no plan
template; this plan supplies the dialect's required sections inline.

## Constitution check

The operator's present instruction authorizes the additional future route;
[proposed 0090](../../docs/decisions/0090-mixed-provider-routing.md) records
that authority without accepting itself, 0081 or any other decision. Preserve
0057/0058 for existing recipes, inherited Spark tests routing, frozen
0001–0003, 0005, exact coverage, protected review and hard holds. Historical
0017 does not authorize a broader review write boundary for this run.
Use pinned scripts and checked shims under 0050, unsigned commits under
0056, and the candidate-local pinned engine. Never push or merge. A design
commit creates a new HEAD: later proofs/reviews must identify that descendant
and any implementation changes, rather than reuse c2f76e2 receipts as final.

## Research and chosen approach

Read both complete positions, matching current feature/intake evidence,
decision index, 0057/0058/0081, historical 0017 and the complete candidate
diff. Legacy intake framing is absent and remains a recorded gap. Keep the
small recipe overlay and require stronger observations within existing proof
owners. The source test currently permits broader overrides than the framed
contract, Python enumerates only the older four recipes and assumes Codex
implementation outside self, and CI negative tests default to careful before
the new final entry. These are concrete, bounded proof gaps.

Both positions agree on exact routes and no fallback; the principal ruling
is to require effective and real delivery evidence despite its cost. No new
engine, adapter, retry registry or health probe is justified. The inherited
full dispatcher failure must remain visible, even if all route checks pass.

## Project structure and files

| Paths | Responsibility |
| --- | --- |
| `specs/mixed-provider-routing/{spec,plan,tasks}.md`, `docs/decisions/0090-mixed-provider-routing.md` | This four-file chief commit only. |
| `bundles/mixed-opus-terra/bundle.json`, `agents/mixed-terra-intake.json`, `agents/mixed-opus-implementer.json`, `adapters/codex.json` | Existing candidate under test; make only demonstrated contract corrections. |
| `tests/delivery_recipes.rs` | Exact two-seat override/key set, terra/terra61 preservation, singleton efforts, work hands and inherited limits/charters; existing review policy assertions. |
| `tests/codex_review_routing.py` | Add mixed to effective matrix, explicit Terra/Opus work expectation, precise mixed/careful comparison and negative controls; preserve full dispatcher assertions. |
| `tests/delivery_constitution.rs` | Extend existing CI fixture with mixed failure/stdout/stderr warning cases and omission refusal; rename stale test label. |
| `.github/workflows/ci.yml` | Verify existing pinned five-recipe compile loop; correction only if executable proof exposes a defect. No new CI job. |
| `docs/decisions/README.md`, `bundles/mixed-opus-terra/README.md` | Implementation adds proposed 0090 index row and links scope/evidence limits where needed. No accepted-record edits. |
| `.forge/proofs/`, `.brokkr/state/design/evidence/` | Ignored, fresh candidate-bound logs and receipts; never stage positions or results. |

Source `src/`, Cargo files, existing recipe work assignments, review offices,
charters, adapters' capability declarations, engine/tooling pin and proof
scripts are preservation inputs, not planned edit targets. Do not copy
policy, gates or review declarations into the new bundle.

## Implementation sequence

1. Capture candidate/baseline, directive provenance, clean worktree status,
   engine digest/version and shim integrity. Use absolute candidate-local
   engine path throughout. Number the decision after the largest existing
   record (0089), leaving historical gaps alone.
2. Extend source tests and the existing effective comparator. Resolve all
   seven offices with `agents show` and recipe manifests with `recipes show`;
   keep raw output before projecting exact assertions. Pair manifest model
   pins with adapter mappings and effective office tools/hands/efforts/limits.
   An exit-zero office with non-ok resolution fails; presence unknown does
   not establish availability. Normalize only omitted/null selected Spark
   effort, and expected bundle identity/inventory and two work selections.
3. Compare mixed to careful: unchanged review records and capability sites,
   shared design, hands, boundary and drivers; source inheritance proves
   policy ownership where manifests do not expose policy/argv. Explicitly
   document any unrepresented field rather than silently dropping it.
   Check the 14400 office / 7200 seat timeout layers separately. Do not
   assert five independent providers or box enforcement from missing entries.
4. Feed one-fault copied observations to the same accepting comparator:
   wrong ID/provider/local or official route, effort/status, fallback, alias,
   extra override, missing/merged office and changed hands/boundary. Reuse
   the existing CI fixture's `FAIL_RECIPE` to reach mixed itself, and retain
   the exact successful call trace so omission fails. Preserve earlier
   digest/version and refusal tests.
5. Add the proposed decision index row and reconcile the recipe README only
   as necessary. Commit permitted implementation changes unsigned before
   final verification. No engine repair/repin is authorized if a proof fails.
6. Run focused resolution and full dispatcher proof in distinct fresh output
   directories. Run all five compilations and the complete house inventory;
   preserve command, candidate, engine digest, exit, logs and unexecuted
   cases. Report inherited failures separately without waiving them. The
   final verify seat remains the authority for advancement.
7. Let the existing protected review sequence obtain the five real verdicts.
   Use existing work/provider evidence only if it identifies the claimed
   invocation and boundary; this design seat does not dispatch substitute
   trials. If requested execution or result delivery cannot occur within
   the existing boundary, park with evidence. Never author another seat's
   result, relax tools or resume by silently changing the model. Reverify
   changes before collecting review bound to the new candidate.

## Data model

No new runtime schema. Evidence relates a candidate commit and engine digest
to a source recipe, effective office, manifest selection and adapter model
pin. A reviewer identity includes role, charter, provider/model, invocation,
attempt and consumed result; the two Astra roles cannot share one receipt.
Work evidence also records the actual boxed tool path. Availability remains
unknown until evidenced. Run logs retain prior failures; evidence is not
rewritten to make a later run appear to have occurred earlier.

## Contracts

Work seats retain boxed workspace writes, credential-masked Cargo overlay,
read-only Rustup and network setting. Review remains gate/read-only with
only the existing own-result delivery allowance. DSH unsupported native
permissions/MCP metadata is a limitation, not native confinement evidence.
Result envelopes remain exactly `result`, `inputs`, `notes`; typed facts
belong inside inputs. Scope/fixes, residual severity/security facts and
candidate/attempt attribution keep their current validation and vocabulary.
Only the design chief emits `drafted`; the deterministic check owns design
advancement, and verify/review/ship retain their own contracts.

## Quickstart and validation

Run offline and install nothing. Check `.forge/bin/brokkr` SHA-256 equals
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f` and
version equals the tooling record before any engine execution. Prepend the
candidate-local `.forge/bin` to PATH for tools that resolve brokkr themselves;
pass the absolute engine path to boxed drivers. Verify public shim integrity:

```bash
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo test --locked --test delivery_recipes --test delivery_constitution
.forge/bin/brokkr recipes show mixed-opus-terra
.forge/bin/brokkr recipes show codex-careful
.forge/bin/brokkr agents show mixed-terra-intake
.forge/bin/brokkr agents show mixed-opus-implementer
```

Resolve the five review office names from the spec too. For each of the five
recipes run `.forge/bin/brokkr compile --bundle bundles/<recipe>`, record
output/exit and refuse `lets the run go on without reading` warnings.
Invoke `resolved_matrix` from `tests/codex_review_routing.py` separately with
fresh evidence output after engine admission, then execute the full script:

```bash
python3 tests/codex_review_routing.py --engine .forge/bin/brokkr --out .forge/proofs/mixed-full-UNIQUE
```

Replace UNIQUE with a fresh run identifier; never overwrite evidence. The
controlled dispatcher does not prove live providers. No new standalone eval
is needed; any optional standalone eval first requires same-user coordination
and explicit non-overlap evidence, otherwise defer it.

The canonical inventory remains
[house rules](../../docs/house-rules.md#gates--part-of-every-change).
Run fmt, locked workspace clippy/tests, self compile, exact coverage and
coverage selftest, publish guard, keyless seal proof and license checks.
Execute scripts from `bundles/self/scripts/` after verifying shims; add the
required design-check selftest and delivery-proof-test. Use the fail-closed
native-tls check from pinned verify-seat: only a nonzero `cargo tree --locked
-i native-tls` reporting `did not match any packages` proves absence.
Do not infer absence from another cargo error. Keep all gate outcomes distinct
from compile, focused comparator and live-review receipts. The chief runs
available local gates against its committed draft and retains outcomes only
in ignored evidence; any failed gate remains outstanding downstream.

After the chief commit, run `bash bundles/self/scripts/design-check.sh check
.`. HEAD must touch exactly one spec directory and only the four requested
artifacts; keep them clean and this commit last. The engine's subsequent
pinned check owns the phase verdict. Final verification reruns required gates
on any implementation descendant. CI-owned cross-platform/privileged-port,
chart/history, Docker, advisory and tag-only proofs require their own receipts.

## Risks

| Risk | Mitigation and accepted cost |
| --- | --- |
| Aliases or compile hide wrong runtime selection | Exact concrete pins plus effective offices and negative controls; live identity claims need attributable receipts. |
| Careful inheritance drifts or work hands diverge | Exact comparison and two-seat override tests; accept coupled maintenance and two small hands declarations. |
| Reused or merged verdicts conceal missing reviewers | Five distinct roles/invocations/results and candidate/attempt binding; preserve residuals and refusal proofs. |
| Requested provider cannot deliver under read-only review | Park without fallback or wider writes; no 0017 environment repair in this scope. |
| dsh charter mistaken for enforced native confinement | Preserve capability declarations and report actual boundary evidence; do not claim enforcement equivalence. |
| Correlated Astra roles and Claude work/review provider | State correlation explicitly; retain separate Fable model/result and all other required lenses/gates. |
| Existing dispatcher cross-attempt defect | Preserve full failing proof and not-reached cases separately from focused green; engine repair requires separate work. |
| CI final recipe escapes warning/failure handling | Mixed-specific fixture failures and exact five-call omission check. |
| Design commit makes old receipts stale | Record c2f76e2 as route provenance, bind final verification/review to the descendant actually inspected. |
| Missing legacy framing or four-file restriction obscures provenance | Record matching current intake evidence, add proposed decision now and index downstream; never fabricate history. |
| Heavy gates or shared eval resources are unavailable | Keep failed/unexecuted gates explicit; no installs, paid trials or overlapping standalone evals. |
