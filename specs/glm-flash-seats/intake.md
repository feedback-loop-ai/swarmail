# Intake — GLM-5.3-Flash intake and tests-review seats

Run: `route-the-codex-recipes-intake-a-2001065d`
Date: 2026-10-26. Phase: intake only. Disposition: resolved framing.
Baseline: `a037dc9`. Implementation, design, final verification and protected review remain outstanding.

## Goal

Route the existing Codex recipe intake offices and tests-review office to local dsh GLM-5.3-Flash without changing their duties, permissions, other model assignments, recipe topology or proof requirements.

## Authority and bounded evidence

Read `.forge/epic192/issue0/README.md` and `operator-rulings.md` first and verified both against that directory's SHA256SUMS: both OK. The README lists no prior runs. The 2026-10-26 “GLM-5.3-Flash seats” ruling explicitly authorizes this narrow routing amendment and requires proposed decision 0081; no seat accepts it or any other decision. Its later dated instruction controls the earlier next-number ledger. No 0081 file currently exists. Nothing external is needed for this framing.

Read docs/house-rules.md, the decision index and accepted 0057/0058. Spot-checked the three offices, adapters/dsh.json, recipe composition, tests/delivery_recipes.rs and resolved_matrix in tests/codex_review_routing.py. Current intake overlays pin Sol medium/high, and the tests-review overlay pins Sonnet high. The Python proof pins those routes and efforts; the Rust proof also assumes all work offices use Sol/Astra and all reviewers have high effort. These assertions need narrow updates, not general relaxation. The dsh adapter already maps glm-flash to spark-glm/GLM-5.3-Flash-EXL3, marks spark-glm local and effortless, and says to pin no effort and record not applicable. Its tool_permissions and mcp are unsupported; do not invent native dsh confinement claims.

Local history: 6a56cfc introduced accepted 0058 routing; 45d68df established Luna/xhigh fast implementation; 22efe77 moved fast triage to Opus/medium with no fallback. codex-fast currently removes intake and starts triage: preserve this, rather than creating an intake or moving triage. It uses codex-review-tests only on design/engine panels. Candidate-local .forge/bin/brokkr hashes to ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f. No remote services, canonical journal or sibling worktrees were consulted.

## Scope and frozen guarantees

Affected configuration: agents/codex-intake.json, agents/codex-careful-intake.json, agents/codex-review-tests.json; routing proofs in tests/codex_review_routing.py and tests/delivery_recipes.rs and any additional existing pins found during design. Update current routing descriptions in bundles/, agents/README.md and docs/house-rules.md where necessary, plus the new proposed ADR/index and required delivery records. Preserve accepted ADR bodies; any historical cross-reference uses an explicit dated addendum. Record the supplied ruling with date and quotation/provenance without creating tracked links into ignored .forge/ evidence.

No runtime application code or dependency change is needed. Frozen guarantees 0001–0003 are not directly touched: synchronous accept/store, subscribe-before-check and oldest-only pruning remain unchanged. Extraction-at-ingest (0005), exact coverage, dependency/license policy, engine/tooling pin, legacy self routes, protected review, security hold and fail-closed safety facts retain their contracts. This is the operator-authorized narrow routing change, not grounds to block on re-litigating 0058. No benchmark or performance claim is required.

## Checkable acceptance criteria

1. Both source and pinned-engine resolution for codex-intake and codex-careful-intake select exactly [glm-flash], provider dsh, concrete model spark-glm/GLM-5.3-Flash-EXL3 and local route. codex and codex-careful resolve their existing intake office names to those assignments. No paid-model fallback remains on either intake office. codex-fast still has no intake and retains codex-triage on [opus], claude-opus-5-5, medium, with no fallback.
2. codex-review-tests selects exactly [glm-flash] with that same dsh/concrete/local identity. This resolves in Standard review, Careful review and fast design/engine review. No tests member is added to fast chore/feature: Standard retains 3 members; Careful and fast design/engine retain 5 separate offices/results; fast chore retains its single combined Flash reviewer and fast feature its 2 members.
3. The three changed offices pin no reasoning effort for Spark. Source and resolved-effort/adapter proofs reject any explicit effort for the selected route, including inherited/stale effort that would reach dispatch. Documentation/evidence explicitly records Spark effort as “not applicable”; do not encode an effort named none, rewrite the adapter to allow one, or retain the old Sol/Sonnet effort as the selected route's effort. Verify through the actual pinned engine's resolution, not source JSON alone.
4. Compare against baseline a037dc9: each changed office retains its inherited charter, limits, tool capabilities and hands/permissions. Intake workspace hands, network declaration, credential masks and read-only rustup bind remain identical. Tests review remains gate-class/read-only, inherits review-tests' charter, and gets no write-capable hands or extra tools. Preserve the adapter's unsupported native permissions/MCP declarations and the existing candidate-change refusal; read-only dsh conduct remains a charter obligation.
5. The complete resolved routing matrix preserves every other office and effort: Standard Fable correctness, official DeepSeek Flash security; Careful/fast full panels Astra correctness, Astra guarantees and independent Fable; fast feature Qwen Max with the sole authorized pre-result Sonnet dispatch fallback and Flash security; fast chore combined Flash; Sol high/xhigh and Luna xhigh implementers, shared design, self routes and Opus triage. No fallback is added elsewhere, DeepSeek Pro remains forbidden, and fast selectors, no-default refusal, bounded scope returns and all protected shell drivers/hands/boundaries remain unchanged.
6. Routing proofs pin these exact new identities/effort semantics and preserve the existing unchanged-role assertions. Update both named test files and every other executable pin of superseded intake/tests assignments. Include negative controls that make the same resolved-route assertions fail for an old paid model, wrong provider/concrete route, extra fallback or explicit Spark effort. Preserve existing role-count, triage, fallback, safety-fact, candidate fence and dispatcher/evaluator controls; do not weaken their expected failures or change the engine pin to manufacture green.
7. Add docs/decisions/0081-<descriptive-name>.md with Status proposed and an index row, naming/quoting the operator's 2026-10-26 “GLM-5.3-Flash seats” ruling and its two seat types (three office files). Describe the narrow amendment to accepted 0058 and superseded intake routing in 0057 without rewriting either accepted historical body or accepting 0081. Current routing documentation and bundle descriptions must agree with the implemented matrix, Spark effort not applicable, and preserved Opus triage/Codex implementers. No unrelated ruling is amended or accepted.
8. All four candidate bundles compile cleanly under the digest-matched candidate-local engine, and focused resolved-route proofs plus every local house/pinned gate below pass on committed work. Run the full Python dispatcher proof as well and report its outcome separately; compare any failure with the intake baseline below and preserve its assertion. A pre-existing unrelated engine failure does not authorize an engine repair/re-pin in this routing task and cannot be reported as green. No live Spark/provider availability claim is needed or permitted from these offline proofs.

## Verify commands and ownership

Use only candidate-local engine and current bundles. Work offline and install nothing. Check the public dispatch-shim digests, then run the pinned implementations against the candidate (0050), matching the verify seat; the public scripts/ commands in the house inventory name the same gates. All commands below are required final proofs, not a claim that intake implements the feature.

```bash
export CARGO_NET_OFFLINE=true
export PATH="$PWD/.forge/bin:$PATH"
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum -c -
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
.forge/bin/brokkr compile --bundle bundles/codex
.forge/bin/brokkr compile --bundle bundles/codex-careful
.forge/bin/brokkr compile --bundle bundles/codex-fast
cargo test --locked --test delivery_recipes
python3 tests/codex_review_routing.py --engine .forge/bin/brokkr --out .forge/proofs/glm-flash-seats-final
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
```

Use a fresh --out directory for each Python proof. The full script creates only isolated controlled proof journals; never inspect or mutate the canonical run journal. Compile policy-sweep warnings refuse. Exact coverage requires zero missed lines, branches and functions and passing selftest; no exclusions or floor reductions.

Run the verify seat's fail-closed native-tls absence proof as well:

```bash
out=$(cargo tree -i native-tls 2>&1); code=$?
if [ "$code" -eq 0 ]; then
  printf '%s\n' "$out"; exit 1
elif printf '%s' "$out" | rg -q 'did not match any packages'; then
  :
else
  printf '%s\n' "$out"; exit 1
fi
```

CI owns macOS and privileged real port-80 tests, chart/bump/history checks, Docker smoke, network-backed RustSec audit and tag-only release-rate-5k/publication proofs as assigned by the house inventory. The operator owns Spark host availability, credentials, live settings and eventual branch sealing. Do not reach the Spark host, GitHub, CI or sibling worktrees. These ownership notes are not extra local acceptance conditions. No namespaces, network service or live provider proof is added.

Record this framing in specs/glm-flash-seats/intake.md and identical run-local mirrors at .forge/tasks/route-the-codex-recipes-intake-a-2001065d.md and .brokkr/state/tasks/route-the-codex-recipes-intake-a-2001065d.md so inherited design charters can find it. Commit framing unsigned before returning; later seats commit their work before final verification. No push, merge, tag, publication, issue closure or decision acceptance. The engine alone selects the next phase.


## Intake baseline execution receipt

On committed framing candidate 68aeab9e57136f3da530384f6c6d5e833f1cd55d, all house gates passed: fmt, locked workspace clippy and tests, self compilation, exact 100/100/100 coverage and its selftest, publish-guard proof, keyless seal-branch proof and license check. Public shim digests, design-check selftest, delivery tamper-boundary proof and fail-closed native-tls absence also passed. All four recipe compilations exited 0 without policy-sweep warnings. Evidence is .forge/epic192/issue0/intake-gates/status.json and the per-gate logs there. These logs were generated through the workspace tool with CARGO_NET_OFFLINE=true and the admitted candidate-local engine. The transport timed out after 300 seconds while the batch continued; the subsequent complete status file, not the timeout, establishes these outcomes. The receipt-only follow-up changes no routing, application, test or gate code.

The supplemental full tests/codex_review_routing.py exited 1. Its resolved_matrix comparisons and negative controls passed on the existing routes, as did preceding Qwen fallback/safety cases. It then failed at main's 1-cross-attempt assertion (line 294, through check at line 276): the pinned engine returned completed where the existing control requires refusal. Later controls did not execute. This is a pre-existing baseline engine correlation defect, already identified in the earlier fast-triage delivery's evidence; it is not a GLM route regression, live provider observation or green dispatcher proof. The supplied operator ruling assigns the attempt_id correlation repair to a separate careful engine delivery before any re-pin. Retain the control and surface this exact failure in final evidence; do not repair the engine, change its pin, hide the red or broaden this task. House-gate green and resolved-route green do not establish dispatcher-wide green.

To prove the route-specific acceptance claims independently of that pre-existing dispatcher stop, run the existing resolved_matrix function directly on final committed routing, in addition to running/reporting the full script. This invokes the same whole-matrix assertions and negative controls, without editing or bypassing any assertion in the full proof:

```bash
python3 - <<'PYPROOF'
from pathlib import Path
import runpy
proof = runpy.run_path('tests/codex_review_routing.py')
out = Path('.forge/proofs/glm-flash-seats-resolved-final')
out.mkdir(parents=True, exist_ok=False)
proof['resolved_matrix'](Path('.forge/bin/brokkr').resolve(), out.resolve())
PYPROOF
```

Use a fresh output directory per execution and retain the complete full-script failure evidence alongside this focused result. No feature implementation was performed by intake, and final GLM route acceptance remains for later seats.
