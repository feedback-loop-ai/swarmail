# Intake framing — issue #199 sequential eval trials

Run: `epic-192-issue-199-stop-claiming-7a82969b` · 2026-10-25 · intake only.

## Goal

Enforce one active eval trial at a time, reuse fresh private HOME ownership, record concurrency on every row, and replace cross-trial isolation claims with the operator-directed sequential baseline and its exact same-uid limitation.

## Authority and bounded observations

Both `.forge/epic192/issue199/README.md` and `operator-rulings.md` pass `(cd .forge/epic192/issue199 && sha256sum -c SHA256SUMS)`. The operator's 2026-10-24 #199 ruling chooses refusal of concurrency; namespace isolation belongs to #221. Prior runs are historical evidence, not current verification. No journal, sibling worktree, GitHub or CI was accessed.

Read house rules, the decision index, accepted 0055 and 0058, and proposed 0073/0075. Base HEAD is `58adf0b`; tracked worktree was clean. Local history includes #194 at `8e61ab4`, #196 at `ef8a3dd`, and original eval harness #148/#149/#150 at `2004bb9`.

Spot-checks: `runner::run_trial` admits independent calls without a shared trial admission fence; `main::matrix` already awaits one worker at a time. `TrialResult`, `refused_row`, normal/error constructors and matrix child-row validation currently lack concurrency metadata. `RuntimeDirs::create/apply` already creates private HOME/TMPDIR/XDG directories and clears child environments; `run_body` gives each agent its own runtime directories below fresh trial scratch. Retain this #196 implementation and add across-trial freshness proof. `evals/tests/proof.rs::per_task` deliberately starts trials concurrently, and `two_concurrent_isolation_trials_both_pass` asserts overlap; these proofs must change when admission changes. The three simultaneous agents inside one `inbox-isolation` trial are distinct from concurrent trials.

The engine `.forge/bin/brokkr` hashes to `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`. Intake read the pinned verify inventory and existing candidate bundle routing; do not edit routing.

## Decision and frozen-guarantee boundary

No change sits directly in frozen guarantees 0001 (synchronous accept/store), 0002 (subscribe before check), or 0003 (oldest-only pruning). No root server, extraction, transport, dependency, license, or coverage-policy change is authorized.

0055 §6 and its proof paragraph explicitly describe two concurrent trials and require overlapping isolation proof. Replacing those requirements needs a narrow decision record: design must create `docs/decisions/0076-sequential-eval-trials.md`, leave Status proposed, and index it as proposed. Cite the supplied operator direction as authority for sequential behavior; do not infer acceptance of 0076, 0073, 0075, namespace isolation, or anything further. Preserve historical 0055 text; use a dated addendum pointing to the ruling and scoped amendment, and update live eval specification/plan claims. The operator already selected the issue's otherwise branch; design does not revisit the isolation-versus-sequential choice. No new operator decision is missing for intake to frame that work.

## Scope and missing local document

Primary owners are `evals/src/runner.rs` (admission and row metadata), `evals/src/main.rs` (matrix admission/row validation and named refusal), and their tests. Touch `evals/src/report.rs` only as required to consume the revised row contract. Docs are `evals/README.md`, `specs/agent-eval-harness/{spec,plan,tasks}.md`, the 0055 addendum, and proposed 0076/index. Keep edits local because #200 concurrently owns runner/main; do not import or redesign its work.

No #151 run plan or exact issue-#148 AC4 text exists in the searched local Markdown beyond the supplied evidence's quoted AC4 claim. The evidence packet is sufficient authority for that claim. Add a clearly identified local #151 sequential baseline run plan (for example `evals/run-plan.md`) and link it from README/spec; do not invent an upstream edit or request unavailable remote evidence. The current spec's AC4 is the positive control, not issue #148's AC4: preserve that positive-control requirement while correcting the cross-trial claims and overlap AC6. No model measurement is required.

## Checkable acceptance criteria

1. **Single active trial.** Real CLI-process and library-path tests attempt a second trial while the first is acknowledged active. It cannot launch trial servers or agents concurrently; attempted overlap gets a stable named refusal and non-success status rather than silently becoming sequential. Matrix never has two active trials, including separately launched trial/matrix invocations under the same uid on this host. Concurrency requests greater than one receive a named refusal, never silent downgrade. Design must identify the admission scope and explicit concurrency-request interface. Hold ownership through child cessation, reaping and private-state cleanup, including normal completion, cancellation and errors. Prove a later trial can start after release. This is cooperative runner admission, not confinement of arbitrary same-uid processes.
2. **Fresh HOME across trials.** Run two sequential real trials with a helper that records its generated HOME and writes a sentinel there. Paths differ, neither equals caller HOME, and the second observes no first-trial sentinel. Confirm private directory ownership persists through cleanup and caller environment remains closed. Reuse #196's existing RuntimeDirs behavior unless this test exposes a gap; do not introduce another environment policy.
3. **Every row records concurrency.** Choose and document an explicit numeric concurrency field with supported admitted value 1. Inspect emitted JSON for pass, fail, active harness error, never-started refusal, cancellation suffix and retained result.json; all rows carry the field. Define the refused-row meaning honestly when no trial ran and distinguish requested concurrency from supported execution when relevant. Matrix validation/report parsing tests reject unsupported or contradictory concurrency instead of laundering it into a baseline row. Preserve full product cardinality, honest statuses, no retries, unconfined labeling and existing report formulas; align schema handling and fixtures explicitly with proposed 0076 and the parallel #200 boundary.
4. **No cross-trial isolation claim.** README, issue-#148 AC4's local contract in the eval spec/0055 addendum, and a linked #151 run plan explicitly state concurrent trials are not isolated, baseline trials run sequentially with fresh HOME, and every result records concurrency. Each states the exact limitation: a same-uid agent can read a concurrent trial's prompt from shared TMPDIR, use that trial's MCP/app/identity inputs to complete its task, and make its do-nothing victim pass. Private paths and runner serialization are not namespace isolation; #221 owns actual isolation. A documentation-contract check fails if the required statements or links disappear or live overlap-proof claims return.
5. **Negative control.** Prefer an `evals/tests/` control establishing that a do-nothing victim fails under the sequential runner while a second trial is refused before an agent can act. Use acknowledged readiness/admission ordering, not sleeps as synchronization. The control must fail if overlapping admission is restored. If no such control is delivered, explicitly record that choice and retain criterion 4's exact case in all required limitation text; the original issue permits this fallback. No reusable cross-trial attack helper is needed to establish refusal. Keep all-oracle-pass and all-null-declared-idle-failure proofs; replace the obsolete two-overlapping-trials proof and serialize test dispatch where it would otherwise contend with the production admission contract. Preserve the three-agent within-trial inbox/foreign-token proof.
6. **Inherited guarantees stay proven.** Evals' complete suite passes, including #194 cancellation/cleanup-before-return and refusal suffix tests, #196 whole-map/privacy/artifact tests, and real SMTP/HTTP/MCP scoring. Preserve pre-spawn signal registration, first-signal admission stop, owned fallback Cargo cleanup, cleanup fence and unknown/expiry error honesty. Deterministic test-only seams obey 0020. Do not reduce assertion strength or claim hostile-process confinement. Root gates remain unchanged and all pass.
7. **Decision/docs are reviewable.** Proposed 0076 and its index row name only this scoped change to 0055; accepted text receives an attributed dated addendum, not silent rewriting or self-acceptance. The local #151 plan names sequential invocation, fresh HOME, row concurrency and the exact poacher limitation. All links resolve. No namespace, network, macOS, live GitHub setting, remote issue update or model-baseline result is a local completion criterion.
8. **Candidate-bound proof.** Commit the implementation unsigned before final verify; all house and independent eval commands below pass for the reviewed candidate through the existing pinned engine/bundle proof chain. Exact root coverage has zero missed lines, branches and functions, with a passing coverage selftest. Protected review and security-hold remain the engine's responsibility using this candidate's existing recipe, without route edits or substitute historical receipts. Intake resolution alone is not implementation completion.

## Verification commands and owners

Verify runs offline and installs nothing. The public script names below are the house inventory; hermetic verify checks `bundles/self/scripts/shims.sha256` and executes the corresponding pinned bundle copies against the candidate, per 0050. Use the candidate-local engine with the digest above, not a PATH substitute. Retain the pinned verify seat's design selftest and delivery tamper proof as well.

```bash
sha256sum .forge/bin/brokkr
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
bash scripts/seal-branch-test.sh
cargo deny check licenses
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
```

Fail-closed TLS-tree proof (the pinned verify-seat probe; an arbitrary failure is not proof of absence):

```bash
out=$(cargo tree -i native-tls 2>&1); code=$?
if [ "$code" -eq 0 ]; then
  echo 'REFUSAL: native-tls is present' >&2; exit 1
elif printf '%s' "$out" | grep -q 'did not match any packages'; then
  :
else
  printf '%s\n' "$out" >&2; exit 1
fi
```

Evals is a separate workspace; root tests/coverage do not execute or measure it:

```bash
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --workspace --locked
cargo test --manifest-path evals/Cargo.toml --locked --test runner
cargo test --manifest-path evals/Cargo.toml --locked --test report
cargo test --manifest-path evals/Cargo.toml --locked --test proof
```

Design supplies exact new admission/HOME/concurrency/control test filters and documentation-check commands, traceable to criteria 1–7. Full eval tests remain mandatory after focused passes. No performance claim or benchmark change requires cargo bench.

CI owns chart/history/selftest, real Docker smoke, network advisory audit, tag-only release-rate proof and privileged port-80 exchange. The operator's supplied ruling assigns macOS eval evidence/CI integration to #201; Linux execution cannot establish macOS behavior. The operator owns eventual namespace decision acceptance under #221 and sealing/merge. None requires intake to fetch remote evidence or operate those systems. No push, merge, tag, publication, issue closure or decision acceptance is authorized.

## Intake outcome

Resolved: the verified operator packet supports actionable sequential-refusal framing with falsifiable criteria and complete verification commands. This artifact implements no runner behavior, accepts no decision and claims no feature/gate completion. The engine chooses subsequent phases from the result contract.
