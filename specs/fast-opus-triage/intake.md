# Intake — Opus medium for fast triage on main

Date: 2026-10-25
Run: `apply-the-operator-directed-fast-ff1692a8`
Baseline: `e24138e5844f1da6a2fae632d5553f2d5c2e1715`
Result: resolved — framing recorded; implementation, verification and review remain.

## Goal

Change only codex-fast triage to existing Claude Opus 5.5 (`opus`) at medium effort with no fallback, record the operator-directed dated clarification to accepted 0057, and prove preservation of all other routes on this candidate.

## Authority and bounded evidence

Read `.forge/epic192/opus-triage/README.md` and `operator-rulings.md` first;
all four packet files pass `sha256sum -c SHA256SUMS` from that directory.
The directive is the operator's accepted narrow amendment to 0057, not a
request for another numbered decision or any new acceptance by a seat.
Read the house rules, decision index, accepted 0057 and 0058, current routing
configuration, relevant test functions and local history. No GitHub, CI,
journal or sibling-worktree inspection is needed or authorized.

The workspace includes the earlier design at `specs/fast-opus-triage/`.
Its references to the stale baseline, missing intake, duplicate index rows,
Sol/high implementation assertions and a required engine cross-attempt fix
are historical evidence, not instructions for this run. Design must reconcile
its artifacts with this framing and the current operator directive before
implementation. Re-derive the reference patch; do not apply it wholesale.

Spot checks: `agents/codex-triage.json` still selects `[sol]` / medium;
`adapters/claude.json` already maps `opus` to `claude-opus-5-5` and supports
medium. `fast_triage_routes_only_bounded_bugs_directly_to_implementation`
still pins Sol. `resolved_matrix` in `tests/codex_review_routing.py` currently
compares review routes and implementation model chains but omits triage.
Standard and Careful have intake, with no triage office: preserve that absence.
Their intake offices are Sol/medium and Sol/high respectively. Main already
pins Luna/xhigh fast implementation in the Rust tests and uses Luna in the
Python proof; do not restore or re-fix stale Sol/high assertions.

History: `45d68df` supplies Luna/xhigh and candidate-local engine preflight;
`6a56cfc` lands accepted 0058 and the known cross-attempt proof control;
`e24138e` supplies the earlier design artifacts. The local engine's SHA-256
matches the required value:
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
No provider dispatch or delivery gate has been executed during intake.

## Acceptance criteria

1. Resolved codex-fast triage is office `codex-triage`, provider `claude`,
   chain exactly `[opus]`, concrete model `claude-opus-5-5`, effort medium.
   Its source declares exactly `models: ["opus"]` and
   `efforts: {"opus": "medium"}`. No fallback or obsolete Sol effort remains.
   Reuse the existing adapter mapping without editing adapter identity.
2. Except for that model/effort substitution and accurate documentation,
   triage retains baseline inheritance, hands, masks, limits, charter rules,
   five classification results, selectors, no-default refusal and bounded
   returns. Standard/Careful still have no triage office and retain their
   original intake offices and efforts. Compare these against the baseline.
3. Append a dated operator clarification (directive dated 2026-10-24) to
   `docs/decisions/0057-codex-delivery-recipes.md`, explicitly superseding
   its earlier fast-triage Sol/medium statement. Preserve the accepted status
   and historical body, Luna/xhigh clarification and accepted 0058 authority.
   Current `docs/house-rules.md`, `bundles/README.md`, `agents/README.md` and
   `agents/charters/codex-triage.md` consistently state the new triage route;
   any nearby role-count correction must match 0058 without changing rules.
   No new decision number, decision-index edit, proposal, renumbering or
   acceptance of another decision occurs; the packet's number ledger stands.
4. Exact route assertions preserve fast Luna/xhigh implementation in all
   four strategies; Standard Sol/medium intake and Sol/high implementation;
   Careful Sol/high intake and Sol/xhigh implementation; shared Sol/high
   simplicity and Astra/high robustness/chief. Legacy self retains GLM
   intake/implementation, its existing review chains, shared design and proof
   ownership. All configuration outside the specified triage substitution
   remains equal to the baseline.
5. Preserve accepted 0058 exactly: Standard Fable correctness, official Flash
   security, Sonnet tests; Careful and fast design/engine each require five
   separate Astra correctness, Flash security, Sonnet tests, Astra guarantees
   and independent Fable results. Fast chore retains one combined Flash;
   fast feature retains Qwen Max correctness and Flash security, with only
   its existing pre-session Qwen-to-Sonnet dispatch fallback. Existing
   efforts, protected review, security hold and inherited policy remain.
   DeepSeek Pro remains forbidden. No engine, tooling pin, patch stack,
   lockfile, dependency, gate script or unrelated CI changes occur.
6. Extend `tests/delivery_recipes.rs` and the resolved-route matrix in
   `tests/codex_review_routing.py` to assert criteria 1, 2, 4 and 5,
   including fast triage and preserved Standard/Careful intake and triage
   absence. Wrong triage provider, alias/concrete model, effort or an extra
   fallback must fail the same equality checks used for the real route.
   Keep existing review controls. In particular, leave the cross-attempt
   fixture, assertion and engine behavior exactly as on main: the known
   engine defect is out of scope, not a requirement to fix or weaken it.
7. Verify the committed implementation candidate with every house gate and
   the additional pinned verifier proofs below, offline with no installs.
   Coverage remains exact 100% lines, branches and functions (missed == 0),
   with its selftest. Record command exits, logs, candidate commit/tree and
   engine digest. Failures remain failures; no skip or expected failure is
   reported as a pass. Record the full routing proof's known cross-attempt
   failure separately from the route-matrix verdict, without expanding scope.
8. The delivery workflow obtains the actual accepted-0058 five-role review
   on the verified candidate, with distinct per-role invocations/results and
   candidate identity; controlled Python fixtures do not constitute that
   review. Intake does not invoke reviewers or claim feature completion.
   Changed candidate trees require renewed applicable verification/review.
9. Commit the requested work unsigned, with no push, merge, tag, publication,
   issue closure or new decision acceptance. This phase commits only its
   framing under the explicit instruction to commit work before returning;
   it makes no implementation change and does not select the next phase.

## Frozen guarantees and scope limits

No application code intersects 0001 (synchronous accept == stored), 0002
(subscribe before check) or 0003 (oldest-only pruning). All remain frozen;
no semantic change to them or ingest-time extraction is requested. No new
benchmark or targeted protocol test is justified by this routing-only work;
the full workspace test suite retains existing real-server protocol coverage.

The engine re-pin from the old worktree, attempt-correlation repairs and
unrelated index cleanup are excluded. The full Python proof can still fail
at its existing cross-attempt control; report that pre-existing defect
honestly. It is not an intake block or evidence that the routing change is
wrong. Do not claim all delivery proofs are clean while it remains red.

CI owns chart contracts/history/selftest, Docker smoke, network RustSec,
tag-only rate proof and privileged port-80 exchange under the house inventory.
Live settings, macOS or namespace proofs are not added to this task. The
operator owns sealing and any later publication. None is a local acceptance
criterion for this seat.

## Verify commands

Use only the candidate-local `.forge/bin/brokkr`, after comparing its digest
with the value above and `bundles/self/tooling.json`. Validate the public
shims against `bundles/self/scripts/shims.sha256` using the pinned verifier's
existing procedure, and execute pinned copies rather than candidate shims.
These are the house inventory commands with the pinned script paths:

```bash
sha256sum .forge/bin/brokkr
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
"$PWD/.forge/bin/brokkr" compile --bundle bundles/self
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
```

Also run the pinned verifier's required additional proofs:

```bash
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
out=$(cargo tree -i native-tls 2>&1); code=$?
if [ "$code" -eq 0 ]; then
  printf '%s\n' "$out" >&2
  exit 1
elif printf '%s' "$out" | grep -q 'did not match any packages'; then
  :
else
  printf '%s\n' "$out" >&2
  exit 1
fi
```

Focused routing commands supplement the full workspace suite:

```bash
cargo test --locked --test delivery_recipes
"$PWD/.forge/bin/brokkr" compile --bundle bundles/codex
"$PWD/.forge/bin/brokkr" compile --bundle bundles/codex-careful
"$PWD/.forge/bin/brokkr" compile --bundle bundles/codex-fast
"$PWD/.forge/bin/brokkr" recipes show self
"$PWD/.forge/bin/brokkr" recipes show codex
"$PWD/.forge/bin/brokkr" recipes show codex-careful
"$PWD/.forge/bin/brokkr" recipes show codex-fast
python3 tests/codex_review_routing.py --engine "$PWD/.forge/bin/brokkr" --out .forge/evidence/opus-triage-routing-ATTEMPT
```

Use fresh attempt-specific output paths and preserve prior logs. All four
compiles must be warning-free. To retain a separate route-only verdict
without changing or bypassing the existing cross-attempt assertion, invoke
the same matrix function independently (fresh output directory):

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

This invocation proves only resolved routes and their negative controls.
The full proof command still runs and retains its unmodified verdict;
no engine repair, re-pin, skip or green claim follows from the separate check.
