# Plan — Require the complete merge-check set

## Technical context

Implement [spec.md](spec.md) for issue #205. Swarmail is one Rust crate, but
this repair concerns existing GitHub settings, Markdown and the delivery
watcher's owning code. Its language, repository and harness are unresolved;
do not assume it is Brokkr or an inbox watcher. No new production dependency
or mail schema is needed. Workspace-only seats prepare local work and hand
external operations to the already-authorized operator/driver.

No plan template is present under `.specify/`; this plan follows the sections
required by `dialects/speckit/design.md`, as existing repository plans do.

## Constitution check

Preserve synchronous insert/accept, subscribe-before-scan, oldest-only pruning
and ingest extraction (0001–0005). Preserve all gates, exact coverage and
selftest, pinned proof ownership, protected review and security hold. Apply
0056's unsigned local commits/no push and 0057's full engine lane and isolated
run discipline. No accepted ruling changes, so no ADR is declared. Proposed
0051's release work is outside scope. If discovery proves a necessary semantic
amendment, return to design rather than silently changing that assessment.

This design commits exactly the three artifacts in this directory. It does
not claim full implementation gates or external proof. Implementation and
root integration must satisfy the single house inventory before success.

## Project structure and files touched

| Path/system | Intended work |
|---|---|
| `specs/required-merge-checks/{spec,plan,tasks}.md` | Chief's three-file design commit; later reconcile discovery and evidence pointers before final verification |
| `docs/house-rules.md` | Add the authoritative exact merge-blocker inventory within Gates, preserving the existing command inventory and CI distinctions |
| Actual watcher owner source/configuration and behavioral test paths | Locate first, then record concrete paths here; patch existing parser/wait/caller and use the owner's existing harness; external review is mandatory if external |
| `feedback-loop-ai/swarmail` main protection | Authorized narrow update adding the two contexts; preserve existing identities and other fields |
| `.github/workflows/ci.yml` | Existing producer authority to inspect; no durable job rename, new workflow or publisher change planned |
| `tests/delivery_constitution.rs`, `tests/delivery_recipes.rs` | Preserve/run existing delivery checks; extend only a demonstrated in-scope local regression, never use string checks as live proof |
| `.forge/epic192/issue-205/` | Ignored evidence, operation ledger, raw receipts and checksum manifest; durable copy/link required before temporary refs disappear |
| Two uniquely named remote probe branches/PRs | Operator/driver creates isolated temporary job failures, retains refusals, closes/deletes owned resources |

No `src/`, Cargo inputs, allowlist, release files, pinned gate implementations
or bundle tooling changes are planned. A newly discovered necessary local
surface must be justified against #205 before adding it to this table.

## Research and discovery

Read both `.brokkr/state/design/positions/{simplicity,robustness}.md`, both
intake framings, house rules, decision index and the applicable accepted
0001/0002/0003/0005/0007/0045/0050/0056/0057 records. Local CI confirms names
`brokkr bundle (pinned)` and `docker smoke`; it also contains expanded test
names and tag-only jobs, so enumerating YAML job keys is not a live inventory.
`ci_runs_the_chart_history_check_and_the_docker_smoke` checks workflow wiring
only. Bounded repository searches found no actual delivery watcher or
`gh pr checks --required` implementation outside evidence/specs. That does
not establish absence in an external owner.

Implementation starts by tracing the operator's actual delivery launch command
and configuration to its owner/source/revision. Capture the effective required
set and producer identities from authoritative API/check data. Record the
owner test command, candidate association semantics, rerun handling, paging,
existing expected-set mechanism, deadline and caller in this plan before
editing that owner. If unavailable, preserve the exact unresolved boundary;
do not invent a local substitute or report AC3 complete.

## Implementation approach

1. **Capture and reserve.** Reserve the issue-scoped settings writer through
   the existing operator/driver coordination. Capture main protection,
   applicable rules/rulesets, actor permission and genuine producer identities.
   Retain raw data and compute `R0 union A`, preserving identities. Prepare a
   reviewable narrow request; no broad repository-settings template. Record
   intended action and unique probe ref names before side effects.
2. **Repair the actual watcher.** Use its existing explicit required-set
   mechanism, or retain its live reader with a reviewed baseline guard. One
   comparator enforces equality with effective requirements and completeness
   of current results; `--required` cannot determine what may be omitted.
   Keep compatible existing polling/backoff/deadline/error behavior, making
   requests bounded if necessary. Parse all pages and both result types when
   the baseline uses them. Resolve current producer/run/attempt and candidate
   association; ambiguous evidence refuses. Re-read policy/head/base before
   ready, reset on changes/restart, and pass expected head through the real
   caller's merge operation. Add owner tests and obtain owner review before
   activation; record the exact launched revision.
3. **Apply and verify protection.** Under the reservation, re-read immediately
   before the narrow update. A change requires recomputation and review of
   the resulting scope. Do not assume endpoint compare-and-swap support.
   Read back effective settings and compare all relevant old fields and
   tuples. Failures stop admission; reconcile from current state without
   restoring an old snapshot. Never disable strict or admin enforcement.
4. **Align documentation and activation.** Write the complete live names and
   applicable app constraints in the existing Gates section. Compare normalized
   `D`, reviewed watcher `E` and effective `R`, then exercise the actual launch
   path. Normalization may sort/deduplicate equivalent records but cannot fold
   case, trim names, strip app constraints or confuse job keys with contexts.
   Changes to live policy invalidate readiness until reconciled.
5. **Run two isolated probes.** For each run-unique PR, introduce an unconditional
   failing step only within the intended existing job on that disposable
   branch. Retain job name/producer and other workflow behavior; choose a step
   placement that leaves wiring checks and all other requirements green.
   Validate the actual check outcomes rather than assuming isolation. No
   fault switch or probe workflow lands on the delivery branch. Confirm PR
   events and branch pushes cannot publish, no tag is created and auto-merge
   is off. Fulfil actual reviews/signatures, conflict and strict-base conditions
   before attributing a refusal. Run the watcher against each candidate too.
6. **Retain server refusal safely.** The operator/driver establishes the exact
   supported proof request and its non-merge preconditions before execution.
   GitHub merge endpoints have no assumed dry run. Pin expected head, verify
   the legitimate terminal failure and effective protection immediately before
   the request, and prevent concurrent reruns/edits during the probe. No bypass
   or successful test merge is allowed. If deterministic refusal cannot be
   established, keep AC5/AC6 open rather than send a speculative request.
   A generic refusal is valid only with surrounding authoritative evidence
   excluding every other blocker and tying it to the one required red job.
7. **Retain, clean and integrate.** Archive exact receipts/checksums outside
   disposable refs with an operator-retained durable link. Close each PR,
   delete only recorded owned refs, confirm unmerged closure and absence,
   and re-read protection. Reconcile interrupted actions from exact remote
   names before retrying; retain unknown outcomes until resolved. Complete
   local gates and actual verify/review/ship, then operator-owned root
   integration and CI. Record final revisions, owner activation and final
   settings. Do not close #205 on a lane-local green result.

## Data model and contracts

This is an evidence model, not a new database or cross-repository schema:

| Record | Minimum identity and content |
|---|---|
| Policy observation | Repository/branch, timestamp, raw protection/effective rules, strict/admin state, exact context/app tuples, source response identities |
| Candidate observation | PR number/URL, head/base SHAs, synthetic merge association if used, current run/attempt/status identities and conclusions, policy observation |
| Watcher evidence | Owner/path, reviewed change, launched revision/caller, expected set, tests, readiness/refusal and candidate identity |
| Operation ledger | Run identity, intended action, unique ref/PR identity, request/response receipt paths, outcome or unresolved status, cleanup status |
| Completion handoff | Durable receipt location and checksums, reviewed owner revision, integrated root SHA, full gate/CI receipts and final settings |

These records contain no credentials. Raw responses and a concise index are
sufficient; do not build a permanent proof subsystem. Save intent before an
external write; a lost response is unknown, not permission to repeat blindly.

## Validation and quickstart

First resolve owner and live inventory through the authorized driver; then run
the recorded owner harness against the real parser/wait implementation. Use a
complete passing control and the AC4 adversarial cases. Deterministic API
observations prove watcher logic, not GitHub enforcement; runtime tests retain
the real-protocol/no-mocks rule. Demonstrate the old omission path fails the
regression. Reuse existing pagination/retry tests where they cover the owner.

Local targeted commands after implementation:

```bash
cargo test --locked --test delivery_constitution
cargo test --locked --test delivery_recipes
bash bundles/self/scripts/design-check.sh check .
```

The design check above applies while chief's design commit is HEAD; later
implementation commits do not have to impersonate a new chief commit. The
sequence's final check executes the pinned bundle copy and owns advancement.

Run the complete [house inventory](../../docs/house-rules.md#gates--part-of-every-change)
on the final implementation candidate using its digest-matched provisioned
engine and pinned proofs. This includes locked commands, exact coverage and
its selftest, publish/seal proofs, permissive licenses and fail-closed native-tls
absence. Preserve the documented CI-only responsibilities and tag distinctions;
no tag/release is required or authorized for #205. Verify installs nothing.
Owner tests/review, both live negative proofs and required root CI are separate
obligations. Final tracked evidence changes precede verification; later raw
receipts remain ignored and durably retained without dirtying verified code.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Concurrent protection edit loses requirements | Existing coordination reservation, fresh comparison and narrow update; no stale rollback; accept scheduling delay and no claim of atomic GitHub transactions |
| Unknown owner or unused replacement | Trace real launch path, record source/revision, owner review and activation; keep AC3 open until proved |
| Drift among expected/live/documented sets | Equality comparison with reviewed baseline; drift blocks until reconciled; no silent shrinking |
| Wrong app, truncated pages or stale green | Exact identities, complete enumeration, authoritative reruns and candidate association; ambiguity can cause a conservative refusal |
| Head/base/policy changes after reads | Final re-read, reset observation, expected-head merge and server strict enforcement; readiness is not a lasting authorization |
| Legitimate synthetic checks rejected | Prove head/base/test-merge linkage in owner tests and actual caller observations |
| Probe fails two checks or another prerequisite | Inspect complete candidate evidence; discard ambiguous proof and redesign only the disposable branch |
| Probe accidentally merges or publishes | Genuine deterministic red job, stable expected head, no concurrent retry, strict protection, no bypass/auto-merge/tags, inspect event conditions; unresolved safety keeps proof open |
| Crash loses remote resources or receipts | Run-unique intent ledger, reconcile unknown outcomes, durable receipts before cleanup, verify closed/unmerged and refs absent |
| Additional software for two experiments | Existing operator procedure and owner harness; no permanent reconciler/proof runner or dependency |
| Local green mistaken for issue completion | AC8 binds owner review, all house gates and root CI to final revisions; no closure with missing evidence |
