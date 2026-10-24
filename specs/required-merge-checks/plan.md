# Plan — Align the required merge checks

## Technical context

This single Rust crate already has the CI producers and accepted 0058 routing.
The change is Markdown alignment with a supplied operator observation.
Workspace capability suffices. No plan template is present under `.specify/`,
`bundles/` or `dialects/`; this plan follows the sections required by
`dialects/speckit/design.md`. Research, data model, contracts and quickstart
are inline; no auxiliary tracked design documents are needed.

## Constitution check

Preserve decisions 0001–0005, exact 100% line/branch/function coverage, pinned
proof execution and fail-closed native-tls/shim checks. No dependency or ruling
changes; no proposed ADR is needed. Accepted 0056 requires unsigned commits,
with no seat push. Accepted 0057/0058 retain the engine lane, protected review,
worst verdict and security hold. The engine owns five separate review results:
Astra correctness, official DeepSeek Flash security, Sonnet 5.5 tests, Astra
guarantees and independent Fable 5.1. No fallback or DeepSeek Pro; chief does
not dispatch or impersonate downstream reviewers.

Chief commits exactly this directory's spec.md, plan.md and tasks.md. The
sequence's pinned deterministic check owns the design verdict. Implementation
then changes the documentation and records real progress. Chief's local gate
runs exercise the design candidate and do not certify the later implementation.

## Project structure and files touched

| Path | Role |
|---|---|
| `specs/required-merge-checks/spec.md` | Requirements, synthesis, dated evidence and superseded history |
| `specs/required-merge-checks/plan.md` | Implementation/proof approach and risk disposition |
| `specs/required-merge-checks/tasks.md` | Ordered work paired with proofs; no inherited completion claims |
| `docs/house-rules.md` Gates | Implementation adds the sole required-on-main list and scopes its ownership |
| `CONTRIBUTING.md` Gates | Implementation corrects native-tls ownership and links main-required checks to the sole inventory |
| `.github/workflows/ci.yml`, `tests/delivery_recipes.rs`, `bundles/` | Read-only producers, already-correct selector regression and existing proof/routing authority |
| `.forge/epic192/issue205/{README.md,operator-rulings.md,SHA256SUMS}` | Read-only supplied evidence; never rewrite checksums or inputs |
| `.forge/epic192/issue205/chief-47e37eec/` and a distinct implementation evidence directory | Ignored local comparison/gate logs, exit statuses and candidate identities |

Across design and implementation the tracked scope is five existing Markdown
files. Positions stay ignored. No watcher, runtime, workflow, test, gate,
recipe, tooling or dependency edits are planned. Publishing the pinned binary
is the separate #206 ruling, not work added here.

## Research and resolved questions

Read both `.brokkr/state/design/positions/` arguments, the framing
`.brokkr/state/tasks/epic-192-issue-205-require-the-d-47e37eec.md`, the verified
packet, decision index and relevant accepted rulings. The spec is the durable
record of their reconciliation. Local producer inspection confirms exact
names; `coverage` uses its job ID as its default display name. The test job
expands ubuntu-latest and macos-latest. Publisher/rate jobs are distinct.

CONTRIBUTING's CI-only native-tls description conflicts with house rules and
the verifier. Correct it narrowly. The existing Luna/xhigh regression already
passes the literal source inspection; no repair task or redundant test is
warranted. Historical host packet/helper claims cannot be revalidated because
the files are absent. No additional discovery is required for this scope.

## Implementation approach

1. Recheck the supplied packet hashes and engine identity locally, and record
   the candidate revision plus source-document hashes with the run-local proof.
   Use the current task's dated receipt as expected data; do not infer names
   solely from the workflow being checked.
2. Add a clearly bounded required-on-main list in Gates with exact displayed
   names and dated attribution. Keep command inventory and all proof ownership.
   Explain PR versus tag/main publication scope without changing job semantics.
3. Correct CONTRIBUTING's native-tls ownership and point readers to Gates for
   required-main checks. Search relevant maintained CI/delivery prose for
   contradictions; retain the five-file bound and no duplicate registry.
4. Use a disposable comparator to extract the required list, assert length
   nine and uniqueness before equality, and compare it to the receipt and
   workflow names/matrix. Read job-level names rather than step names. Require
   exact strings, not substring matches; fail if boundaries or producers cannot
   be identified. Use existing facilities without adding dependencies.
5. Prove that comparator with isolated scratch inputs: omit brokkr, omit
   Docker, omit an existing check, duplicate one, misspell one, add the tag-rate
   context, and add a publisher context. Baseline passes and every control
   refuses. Mutations must reach the actual comparison, not only a checksum
   check. Keep supplied inputs unchanged and retain source/output locally.
6. Review all three artifacts together for current scope, dated attribution
   and honest status. Preserve historical clarifications as superseded; record
   new receipts only for actual work. Operator-owned live experiments and
   watcher work stay outside the local task list.
7. Run all house gates on the candidate with `.forge/bin/brokkr`, including
   exact coverage/selftest, plus the verifier's integrity/proof checks. Retain
   command, candidate identity, exit status and output. Commit the implementation
   unsigned before handoff; engine verify/review must bind to that committed
   candidate. Do not waive or weaken a failing gate or reuse an earlier pass.

## Data model and contracts

No application data model or API changes. The documentation contract is the
nine unique exact check names in one maintained inventory. The dated spec
receipt is historical expected data, not a second evolving policy registry.

Run-local evidence needs the input digests, candidate revision/diff identity,
actual extracted names, baseline and each negative-control outcome, and gate
commands/exits/log paths. These are ordinary proof files, not a new receipt
schema or service. Do not put credentials or invented remote receipts in them.
The result file is the existing engine contract; no journal access is needed.

Local name equality proves documentation consistency. The operator owns live
settings and throwaway-PR refusal experiments, which are out of scope. CI owns
Docker, Helm/history/selftest, RustSec/network, OS-specific/port-80 and
release proofs under house rules. No local macOS, namespace or remote proof
criterion is introduced. An offline pass does not establish live refusal.

## Validation and quickstart

Verify the inputs from the workspace:

```bash
(cd .forge/epic192/issue205 && sha256sum -c SHA256SUMS)
sha256sum .forge/bin/brokkr
```

Compare the binary digest with the spec's supplied pin and the local tooling
record. After chief's three-file commit, run the local structural preflight:

```bash
bash bundles/self/scripts/design-check.sh check .
```

The dispatched pinned check remains authoritative. HEAD must touch only this
spec directory, the three artifacts must be committed/nonempty/clean, and the
required headings and checkbox tasks must exist. Chief returns `drafted`.

Implementation's focused proof is the small comparison and independent
negative controls described above, followed by scope/evidence diff review.
No new Rust tests are justified for reversible prose edits; the existing
workspace tests include the delivery routing regressions.

Execute the complete current
[house inventory](../../docs/house-rules.md#gates--part-of-every-change), using
`.forge/bin/brokkr compile --bundle bundles/self` for its pinned engine command.
This link remains the command authority; do not create another maintained gate
inventory here. Keep `--locked`, exact coverage/selftest, publish/seal proofs
and permissive licenses. Also preserve fail-closed native-tls absence, checked
shim identities, design-check selftest and delivery tamper-boundary proof.
Hermetic verify executes dispatched pinned scripts, installs nothing and has
no network. Chief's local invocations are preflights, not a fabricated verify
seat result. Record failures without expanding scope or claiming success.

The engine subsequently gathers five separate 0058 review results. Do not
make their availability in chief a prerequisite for drafting. Implementation,
verification and review evidence must describe their own candidate; the green
main observation and earlier attempted runs do not certify it.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Omission, duplicate or typo weakens the stated merge contract | Exact strings, nine entries, uniqueness before equality and separate omission/duplicate/typo controls |
| Step/job identifiers or tag publishers become required PR names | Inspect actual job producers/matrix; reject tag-rate and publisher additions independently |
| Small edit leaves contradictory inherited prerequisites | Reconcile spec/plan/tasks together; replace executable instructions and retain dated history as superseded |
| Dated operator evidence is mistaken for live enforcement | Explicit observer/date/provenance and ownership; accept no continuous drift detection or local server-refusal proof |
| Absent old packet tempts reconstructed app/ruleset/helper claims | Name missing inputs and remove unsupported operative claims; current verified ruling is sufficient |
| Earlier checked tasks or green main masquerade as current proof | Reset implementation tasks; bind each new proof to candidate and distinguish design from implementation |
| Corrected selector is changed again or review routing is weakened | Markdown-only diff, existing routing tests, digest-matched engine and unchanged five-result engine route |
| Documentation change is used to waive expensive gates | Execute complete local inventory and pinned verification; keep CI/operator responsibilities explicit |
| Disposable comparator does not prevent future drift | Retain reproducible run-local source/output and one maintained inventory; accept manual future upkeep instead of a new policy subsystem |
| Local gate environment cannot execute a required proof | Record exact command/failure and missing prerequisite; preserve the design deliverable without a false verification claim |
