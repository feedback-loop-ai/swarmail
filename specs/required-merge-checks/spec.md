# Align the required merge checks

Created: 2026-10-24. Revised: 2026-10-24.
Status: synthesized design for the bounded documentation repair; implementation
and protected verification/review follow this design.
Issue: #205, epic #192.
Run: `epic-192-issue-205-require-the-d-47e37eec`.

## Feature and rationale

Make the sole [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
state exactly which CI checks are required before merging to main. Align the
relevant contributor/delivery prose and reconcile these inherited artifacts
with the operator's settled scope. The live setting already includes Docker
smoke and pinned brokkr; the remaining defect is documentation that does not
identify the merge-blocking set and design instructions that demand obsolete work.

The supplied issue history records PR #168 merging after failed Docker smoke
runs. Executing a job and requiring its result are different obligations.
Docker smoke proves deployment persistence across stop/kill and container
packaging properties; pinned brokkr compilation checks the delivery constitution.
Both are required alongside the existing checks. Neither substitutes for the
other or for the complete local gates.

This is engine/gate work under accepted
[0057](../../docs/decisions/0057-codex-delivery-recipes.md) and
[0058](../../docs/decisions/0058-codex-review-routing.md).
It records existing obligations under
[0007](../../docs/decisions/0007-hermetic-gates-single-model.md),
[0045](../../docs/decisions/0045-delivery-repairs.md) and
[0050](../../docs/decisions/0050-delivery-gates.md).
No frozen guarantee, public application contract or accepted ruling changes;
no new decision is declared. The operator ruling is implemented, not reopened
or extended to accept another proposal.

## Dated evidence receipt — 2026-10-24

The operator's assistant read main branch protection through the GitHub API
on 2026-10-24. The supplied task reports these nine required check contexts:

- `format, clippy, publish guard`
- `test (ubuntu-latest)`
- `test (macos-latest)`
- `coverage`
- `deny`
- `RustSec dependency audit`
- `helm chart`
- `brokkr bundle (pinned)`
- `docker smoke`

The same supplied observation records strict on, enforce_admins on, linear
history required, and main CI green at `dc1822a`. This list is a dated receipt
and the fixed comparison input for this repair; the maintained inventory
belongs only in house rules. This seat has not read GitHub or CI. The
observation does not prove current-candidate gates, perpetual enforcement,
or a live red-check merge-refusal experiment.

Chief verified both local packet files against their directory's SHA256SUMS:

| Input | SHA-256 |
|---|---|
| `.forge/epic192/issue205/README.md` | `8fbea3f3d17b528fd4cd78c5aea744097cbc51fef2bc639cd1ac1e6b35e845aa` |
| `.forge/epic192/issue205/operator-rulings.md` | `ce354e19307d268f9e9ab1e8f36c1897fd3d56c8049cbf65ab1f8fed1dac61fa` |

The README supplies the original issue and every prior run outcome. Ruling
#205 settles that the two checks are already required and the remaining work
is documentation/inventory alignment. The task supplies the full list, linear
history and green-main observation. Checksums establish local byte consistency,
not an independent remote observation. The candidate-local `.forge/bin/brokkr`
was hashed and matches
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
Binary identity alone is not a compile or verify pass.

The older `host-evidence/`, `host-evidence-manifest.json`,
`action-receipt.json`, helper copy and historical gate logs are absent from
this workspace's packet. Earlier app-15368 bindings, empty-ruleset assertions,
helper analyses and archive claims are not newly verified facts. Their absence
is recorded, not made a prerequisite: this task and the verified ruling suffice
for the bounded repair. Do not reconstruct those files or invent their contents.

Local inspection confirms the workflow already produces the nine names,
including both test matrix expansions. `brokkr-compile` and `docker-smoke` are
job identifiers; their displayed contexts have spaces as in the receipt.
The current Gates section lacks this required list. CONTRIBUTING incorrectly
places native-tls absence among CI-only proofs, although the inventory assigns
it to CI and verify. `tests/delivery_recipes.rs` already asserts Luna/xhigh,
with an adjacent regression preserving that choice; no selector repair remains.

## Preserved dated clarifications — historical, superseded

These entries preserve the chronology of the earlier design. They describe
prior scopes and claims only; the current requirements below supersede them.
The earlier artifacts remain in Git history, including the imported design
commits whose original references were bb8b040, 05d1a01, 8c97f91 and e1eed3b.
The inherited candidate design before this synthesis is `a9068b1`.

- **Scope clarification, 2026-10-24:** the earlier 05d1a01 design narrowed the
  broader bb8b040 design to a settings operation. Earlier attempts stopped on
  unavailable remote protection evidence and an unidentified watcher. Those
  failures remain historical; they are not requirements to obtain API access now.
- **Host-evidence clarification, 2026-10-24:** a later design described a
  captured host packet/helper and treated watcher ownership, live refusal
  experiments, archival retention and root integration as unresolved obligations.
  It claimed packet validation and missing-check predicate reproduction.
  Those raw inputs are unavailable here; no such reproduction or app/ruleset
  validation is claimed in this run.
- **Implementation attempt disposition, 2026-10-24:** the inherited design
  recorded a Sol-versus-Luna assertion failure in workspace tests and coverage,
  along with some passing local proofs. The verified README also records the
  final earlier attempt stopping with an unavailable pinned engine and no five
  review results. These are attributed historical outcomes, not current gate
  results; the missing logs cannot settle the earlier accounts independently.
- **Return clarification, 2026-10-24:** the previous design authorized a narrow
  Luna/xhigh assertion repair and retained external prerequisites. The current
  base already includes the assertion correction. Its previously checked T2
  cannot certify the current inventory, which is absent. Earlier task completion
  and gate claims therefore do not carry forward to this candidate.

## Current scope clarification — 2026-10-24

The current operator ruling and task supersede every obsolete executable
settings, watcher, selector, packet-archive and live-probe instruction above.
Implementation changes house rules and the necessary contributor wording,
then records actual progress in these three artifacts. Live throwaway-PR
merge-refusal proofs are operator-owned and out of scope, not pending local
acceptance tasks. Watcher discovery, ownership, repair and activation are
outside this delivery. There is no new watcher contract or assertion that a
watcher was repaired. No external owner or source location is invented.

CI owns real Docker, Helm, network audit, macOS, port-80 and release proofs as
specified by the house inventory. Namespace proofs are not part of this
feature. Operator owns live settings and remote experiments. This seat's
acceptance criteria require only workspace-provable documentation and local
delivery evidence. No push, merge, tag, publication, issue closure, remote
settings change, journal access or decision acceptance is authorized.

## Council synthesis and rulings

Chief read both current positions and the current intake framing. They agree
on the bounded scope; there is no substantive panel conflict to manufacture.
The tension is how much proof a small prose edit needs. The rulings are:

| Question | Adopt, reject and cost |
|---|---|
| File scope | Adopt simplicity's existing-file documentation repair and robustness's complete reconciliation of current instructions. Reject the inherited selector repair, unowned watcher work and additional receipt/contract files. Cost: no deployed observer improvement or raw historical archive reconstruction. |
| Inventory authority | Adopt both positions' single maintained list in Gates and robustness's exact displayed-name/matrix distinction. Reject a JSON/YAML registry, new jobs or renamed checks. Cost: future drift still needs explicit maintenance. |
| Proof strength | Rule for robustness's uniqueness-before-equality comparison and separate negative controls, within simplicity's disposable run-local approach. Reject mere visual addition of two names and a permanent parser. Cost: a small reproducible comparison now, with no automatic future drift detector. |
| Evidence | Adopt robustness's dated attribution and simplicity's removal of unsupported raw-receipt prerequisites. Reject app-binding/effective-rules claims unsupported by the current packet. Cost: a deliberately narrower evidence claim; offline comparison cannot prove server refusal. |
| History and ownership | Adopt both positions' preservation of dated history with explicit supersession. Reject retaining external proofs as unchecked local work or erasing earlier failures. Cost: operator-owned experiments remain outside this deliverable, without blocking documentation. |
| Runtime reasoning | Adopt robustness's explanation that Docker deployment proof and pinned constitution proof cover distinct boundaries. Treat its runtime failure analysis as preservation rationale, not new application requirements or tests. Cost: this change makes no new crash/webhook/TLS guarantees. |
| Delivery bar | Adopt both positions' full local gates and existing five-result engine review. Reject a cheaper route for a small Markdown diff. Cost: full proof execution despite the small implementation. |

## Requirements

House rules shall contain exactly the nine displayed context names from the
dated receipt, once each in a clearly bounded required-on-main list, and
attribute the setting to the 2026-10-24 operator observation. Both new checks
are merge blockers. Preserve all existing gate commands, exact coverage and
CI responsibilities. Preserve the distinction between required PR contexts,
tag-only rate proof/chart publication and image publication on main/tags.

Relevant CI/delivery prose shall point to that inventory. Correct
CONTRIBUTING's native-tls ownership wording and identify the link as the place
to find main's required checks; do not duplicate the registry. No Rust, test,
workflow, script, bundle, tooling-pin, policy, dependency or accepted-decision
changes are needed. Pinned proof execution and checked shim identities remain.
Synchronous accept==stored, subscribe-before-check, oldest-only pruning,
ingest extraction and permissive dependencies remain untouched.

## Acceptance Criteria

- **AC1 — Exact documented inventory.** Gates names exactly the nine receipt
  contexts, with unique entries and exact spelling, including both OS names.
  A run-local comparison checks against the independent receipt and actual
  workflow producers/matrix expansion. Separate scratch controls reject omitted
  brokkr, omitted Docker, an omitted existing check, duplicate, misspelling,
  added tag-rate context and added publisher context.
- **AC2 — Honest dated evidence.** This spec records the date, observer, full
  observed set, strict/admin/linear-history settings and green `dc1822a` as
  supplied evidence. Packet checksums pass. No remote observation by the seat,
  current-candidate verification, app binding or ruleset claim is inferred.
- **AC3 — Consistent bounded scope.** Spec, plan and tasks agree on the current
  documentation repair. Superseded dated clarifications remain historical;
  live refusal experiments are explicitly operator-owned/out of scope.
  Watcher, selector repair, remote access and absent historical archival inputs
  are not current prerequisites. Implementation status follows this candidate's
  receipts, never inherited checked tasks.
- **AC4 — Ownership and contract preservation.** Diff review confirms one
  maintained inventory, corrected CONTRIBUTING ownership, intact local gate
  commands and CI/publication scopes, and no changes outside the authorized
  Markdown files. Existing Luna/xhigh tests and recipe routing remain intact.
  CI/operator evidence is assigned to those owners, not new local criteria.
- **AC5 — Candidate-bound local delivery proof.** The eventual implementation
  passes every required local house gate with the digest-matched candidate-local
  engine, including exact coverage and its selftest. Logs/exits identify the
  candidate. Pinned verify and the existing protected review follow through the
  engine; design positions and chief preflights do not substitute for them.
  Failures remain explicit and cannot be reported as a verified implementation.
  Commits are unsigned and no unauthorized remote action occurs.

## Implementation receipt — 2026-10-24

The implementation updates the sole status-context inventory in house rules
and points CONTRIBUTING to it, with corrected CI/verify ownership wording. The
spec, plan and task list retain this bounded documentation scope. No workflow,
source, test, gate, bundle, policy or dependency file is changed. The dated
operator observation above remains the only live-settings evidence; this seat
has not read GitHub or CI. The live throwaway-PR refusal experiments remain
operator-owned and out of scope. The run-local inventory comparator and its
negative-control output are under
.forge/epic192/issue205/implement-47e37eec/. Candidate gate receipts and the
implementation commit are recorded with the implementation tasks. Protected
verification and review remain with the following engine phases.
