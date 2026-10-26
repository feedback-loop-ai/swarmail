# Intake — Codex tests review on GLM-5.3-Flash

Run: `route-the-codex-recipes-tests-re-4add6118`
Date: 2026-10-26. Phase: intake only. Result: resolved framing.
Baseline: `d55a89de70f4a4e6cc598d3726eb2fa55bc2df76`.

## Goal

Route only the existing read-only `codex-review-tests` member in the codex, codex-careful and codex-fast recipes to dsh GLM-5.3-Flash, preserving every other seat's model, effort, permissions and routing behavior.

## Authority and evidence

Verified `.forge/epic192/issue0/README.md` and `operator-rulings.md` against that directory's SHA256SUMS before framing. README SHA256: `4dff0b60b60a41c282518f3440032f2900c6a447471d1379341f1a5bf389e44a`; rulings SHA256: `b68560f6782247377e75e90698f95f33feef390cdbdb71ddd8c9a3cc0fbdfd9e`. README has no prior-run entries; the task and local commits supply the previous outcome. The later ruling controls the README's older intake-plus-tests request:

> The pinned engine refuses dsh for workspace-writing seats, so intake stays on its current models. Only the tests review member (`codex-review-tests`, read-only) of the codex, codex-careful and codex-fast recipes moves to `glm-flash`. Every other seat keeps its model.

Read house rules, the decision index, accepted 0057/0058 and proposed 0081. Spot-checked office JSON, `adapters/dsh.json`, recipe member references, `resolved_matrix` in `tests/codex_review_routing.py`, reviewer assertions in `tests/delivery_recipes.rs` and the pinned verify driver. Local history records original framing 68aeab9, design 224d490 and IMPL-BLOCKED report d55a89d. Existing intake offices are Sol medium/high; tests review is Sonnet high. dsh already maps `glm-flash` to `spark-glm/GLM-5.3-Flash-EXL3`, local, with effort not applicable.

The earlier design and implementation report describe the superseded request. Their reproduced refusal remains evidence: dsh declares hands unsupported, so the engine cannot box intake's workspace hands and refuses those recipes. The revised operator ruling resolves that prerequisite by preserving intake, without deleting hands or changing the adapter/engine. Proposed 0081 exists and must be rewritten, not accepted. The ruling is authority for this narrow amendment; no further decision acceptance is inferred. No missing external input prevents framing.

## Scope and guarantees

Implementation targets `agents/codex-review-tests.json`, `tests/delivery_recipes.rs`, `tests/codex_review_routing.py`, current routing summaries in bundles/docs/agents where stale, and proposed 0081/index. Reconcile `specs/glm-flash-seats/{spec,plan,tasks}.md` with this framing during design; the old intake criteria are superseded. Preserve accepted decision bodies; use dated pointers if required. Keep tracked provenance self-contained, without tracked links into ignored evidence.

Frozen guarantees 0001–0003 are not directly touched: synchronous accept/store, subscribe-before-check, and oldest-only pruning retain their contracts. No application runtime, extraction-at-ingest, dependency/license, engine pin, adapter capability, shell gate, protected review or security-hold change belongs here. No benchmark or performance claim is required.

## Checkable acceptance criteria

1. Source and pinned-engine resolution select exactly `["glm-flash"]` for `codex-review-tests`, provider `dsh`, concrete model `spark-glm/GLM-5.3-Flash-EXL3`, local route, with resolution status `ok`. Assert status explicitly: `agents show` exiting zero alone does not establish an admitted route. No fallback exists on this office.
2. Spark receives no reasoning effort. The office's explicit empty effort map overrides inherited Sonnet effort; resolved office/route evidence proves no selected or inherited effort reaches this route. Documentation records effort as **not applicable**, with no effort value named `none` and no adapter relaxation.
3. This exact office resolves as Standard's tests member, Careful's tests member, and both fast design/engine tests members. Standard retains three results; Careful and fast design/engine retain five distinct results. Fast chore/feature gain no tests member and retain one/two results. Tests review retains the `review-tests` charter, gate/read-only classification, limits, permissions, separate result identity and candidate-change refusal, with no write hands added.
4. Compare the complete source/resolved matrix against d55a89d: every other seat's model, chain, effort, charter, permissions and recipe topology remains identical. Specifically intake remains Sol medium/high with its existing workspace hands; fast still has no intake and keeps Opus medium triage. Preserve Codex implementers, shared design, self routes, Fable correctness, official Flash security, Astra correctness/guarantees, independent Fable, fast-feature Qwen-to-Sonnet pre-result fallback, and combined chore review. No new fallback or DeepSeek Pro is admitted.
5. Both routing proof files pin the new tests identity, provider, concrete/local route, admitted status and no-effort semantics without weakening unchanged-role assertions. Negative controls make the same route assertions reject the old Sonnet route, wrong provider/concrete route, blocked status, added fallback and explicit Spark effort. Existing panel, fallback, safety-fact, candidate-fence and dispatcher controls remain intact.
6. The digest-matched `.forge/bin/brokkr` compiles self, codex, codex-careful and codex-fast with no policy-sweep warnings. Resolved evidence proves the changed tests seat uses dsh in every affected recipe. No engine repair/re-pin or intake substitution is used to make this pass.
7. Rewrite 0081, retaining **proposed** status, to quote/date the revised ruling, amend only 0058's tests assignment, and record the earlier intake refusal and its workspace-hands reason. Its index and current routing descriptions agree with tests-only scope and effort not applicable. Accepted 0057/0058 history is preserved; no decision is accepted by a seat. Historical refusal evidence is clearly distinguished from the revised scope.
8. All local house/pinned gates below pass on committed work, including exact zero-missed line/branch/function coverage and its selftest. Run focused resolution and full dispatcher proofs separately. Preserve/report the known baseline full-script cross-attempt failure (engine completed instead of refusal) and any unexecuted later cases; it is not green evidence, a route regression, or permission to weaken controls or fix the engine in this task. Commit all phase work unsigned; no push, merge, tag, publication or issue closure.

## Verify commands

Offline; install nothing. Use current candidate bundles and the candidate-local engine only. Public gate names are the house inventory's scripts commands; after verifying their fixed shim digests, the verify seat executes the equivalent pinned copies below (0050).

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
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
python3 tests/codex_review_routing.py --engine .forge/bin/brokkr --out .forge/proofs/glm-tests-full-final
```

Run the fail-closed native-tls absence proof too:

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

Independently run the existing focused function, retaining the full script's outcome alongside it; use fresh proof directories each time:

```bash
python3 - <<'PYPROOF'
from pathlib import Path
import runpy
proof = runpy.run_path('tests/codex_review_routing.py')
out = Path('.forge/proofs/glm-tests-resolved-final')
out.mkdir(parents=True, exist_ok=False)
proof['resolved_matrix'](Path('.forge/bin/brokkr').resolve(), out.resolve())
PYPROOF
```

Only disposable controlled proof journals may be created by the existing test; never read the canonical journal. CI owns macOS, privileged port-80, Helm/chart history, Docker smoke, network-backed RustSec and tag-only release proofs in the house inventory. The operator owns live Spark availability (reported working from the host), credentials, live settings and sealing. Do not contact Spark, GitHub, CI or sibling worktrees; no namespaces/network acceptance criterion is added.

This framing is the intake deliverable; implementation and final feature verification remain outstanding. Record run-local mirrors for inherited design discovery, commit framing unsigned, and let the engine choose the next phase.

## Intake baseline verification receipt

Executed on committed framing `6ab2f6d7b3c27569d6766a85b7e4a0ff84e2f65d` through the workspace tool, offline with the digest-matched candidate-local engine. All house gates passed: fmt, locked workspace clippy/tests, self compile, exact 100/100/100 coverage, coverage selftest, publish guard, keyless seal proof and licenses. Shim integrity, design selftest, delivery tamper-boundary proof and fail-closed native-tls absence also passed. All four current bundles compiled with zero unruled policy valuations and no policy-sweep warnings. These are baseline proofs on unchanged routing, not proof that GLM tests routing is implemented.

The independent focused `resolved_matrix` proof passed on existing routes. The full Python routing proof exited 1 at `1-cross-attempt`: `AssertionError: ('1-cross-attempt', 'completed')` (`main` line 294, `check` line 276). This matches the previously recorded engine correlation failure; subsequent cases did not execute. The control remains unchanged. No engine repair, pin change or green dispatcher claim is made.

Complete per-gate logs and status are retained in the run-local `.forge/proofs/glm-tests-intake-4add6118/` directory, with disposable full/focused proof evidence. The workspace transport timed out after 300 seconds while the batch continued; the final status file records every exit code and `done: true`. This receipt adds documentation only. Feature implementation, revised design/0081, final routing verification and protected review remain outstanding.
