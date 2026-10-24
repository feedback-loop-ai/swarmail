# Plan — Reconcile the required merge checks

## Technical context

Implement the bounded [spec](spec.md) in the existing single Rust crate.
The deliverable is Markdown plus run-local evidence; no runtime code, new
service, schema, dependency or GitHub operation is needed. The seat has only
workspace capability. The host packet supplies the current framing's dated
observation, so lack of live API/sibling-tree access no longer blocks this
repair. No plan template exists under `.specify/`; follow the requested
sections from `dialects/speckit/design.md` in this established plan.

## Constitution check

Preserve synchronous accept==stored, subscribe-before-check, oldest-only
pruning and ingest extraction (0001–0005), dependency policy, exact coverage,
pinned proof ownership and unknown-fact refusal. No accepted ruling changes
and no ADR is declared. Preserve unsigned commits and never push (0056).
Engine/gate classification retains full design and five distinct 0058 review
results, protected review, worst verdict and hard security hold.

Chief commits exactly this directory's spec.md, plan.md and tasks.md; the
implementation seat edits Gates and records actual progress. The deterministic
pinned design check owns advancement. Structural design validation proves
neither implementation, live configuration nor issue completion. Full candidate
gates and review remain downstream, with failures reported honestly.

## Project structure and files touched

| Path | Role and allowed change |
|---|---|
| `specs/required-merge-checks/{spec,plan,tasks}.md` | Reconcile current scope, dated history, evidence and pending issue criteria; no duplicate inventory |
| `docs/house-rules.md` Gates | Implementation: insert the single exact nine-context list, common app binding, dated provenance and precise PR/publication distinction |
| `.forge/epic192/issue205/host-evidence/` | Read-only input packet; do not edit the copied helper or receipts |
| `.forge/epic192/issue205/action-receipt.json` and referenced receipts | Read-only historical settings operation evidence, separate from the packet manifest |
| `.forge/epic192/issue205/design-proof/` and later run-specific evidence directory | Ignored comparison/reproduction logs and evidence index; no fabricated journal |
| `.github/workflows/ci.yml`, existing delivery tests and bundle tooling/proofs | Read producer/scope definitions and run existing checks; no edits |
| Host helper at the absolute path in spec.md | External unresolved owner/consumer/activation; no claimed deployment from this tree |

No new tracked files or edits to Rust, Cargo, dependencies, tests, workflows,
gates/shims, recipes, adapters, tooling or accepted decisions are needed.
Positions remain ignored run-local evidence. No settings mutation, remote
probe/cleanup or generic watcher is part of implementation.

## Research

Read both current panel positions and
`.brokkr/state/tasks/continue-epic-192-issue-205-usin-17f8493f.md`, the decision
index and relevant accepted rulings. The spec records packet provenance,
source hash, bounded source searches, earlier failure history and rulings.
Chief independently verified the nine manifest entries and settings shapes,
and reproduced both missing-check successes using the supplied source's
AST-extracted predicate without running `gh` or executing the helper.

The saved bounded searches establish no tracked owner; do not turn that into
a claim about every host file. The observer is real but its relationship to
the issue's driver is unresolved. The existing CI names match the packet;
`brokkr-compile` and `docker-smoke` already produce the two requested checks.
The existing delivery constitution test verifies wiring, not server refusal.
`release-rate-5k` is tag-only; Docker publication also runs on main and chart
publication is tag-only. No new technical research file or permanent checker
is justified for these four existing Markdown surfaces.

## Implementation approach

1. Validate the supplied packet without network access. Retain a small script
   and outputs in a new run-specific evidence directory, preserving inputs.
   Check every manifest entry, exact tuple uniqueness/context agreement,
   strict/admin booleans, explicit empty rules arrays and captured target/head.
   Retain the manifest hash; distinguish the separate historical PATCH receipt.
   A failed validation means unknown evidence, never a guessed empty policy.
2. Edit only Gates to list the nine exact captured names and their common
   app 15368 binding, with the capture date and provenance. Preserve its command
   inventory, CI-only responsibilities and proposed 0051 reference/status.
   Clarify tag rate/chart publication and main-or-tag image publication without
   making publisher jobs merge blockers. Do not copy the full list elsewhere.
3. Compare the documented tuples against the validated packet. Use a small
   run-local extractor scoped to the explicit new list/common app sentence;
   require exactly nine unique entries before set comparison. Retain its
   source, inputs and output. Exercise the negative controls below against
   the comparison itself. No committed prose parser or second executable
   policy registry is warranted.
4. Reconcile implementation status in these three artifacts against actual
   receipts. Preserve dated clarification history and all unsatisfied issue
   criteria. Do not restore executable mutation/retry instructions or mark
   watcher work complete. Retain exact host path/hash, ownership unknowns,
   predicate defects and pending activation/live-proof/archival obligations.
5. Run the entire house inventory and pinned hermetic verification on the
   candidate and obtain the five distinct 0058 results through the engine.
   Retain logs and candidate identity; unsigned commits only, never push.
   If a gate fails or cannot run, record it and follow the delivery policy;
   documentation scope is no waiver. Revalidate affected proof after fixes.
6. Hand off evidence durably through the existing operator process, recording
   an accessible archival receipt or explicitly leaving retention pending.
   Link raw hashes, candidate/gate/review evidence and the external obligations.
   Do not claim issue closure or current remote truth beyond the dated packet.

## Data model and contracts

No production data model changes. Run-local records have these roles:

| Record | Required information |
|---|---|
| Packet verification | Capture time, repo/branch/head, manifest digest and each file's size/digest result; settings/rules shape checks |
| Inventory comparison | Candidate identity, documented names/common app binding, captured tuples, uniqueness/equality result and negative-control outcomes |
| Predicate reproduction | Source digest, AST-extracted expression, separate omission fixtures and observed true results; clearly offline |
| Handoff | Exact external path, unresolved source/launcher/consumer/activation, archival status, candidate/gate/review receipts and each pending issue criterion |

Do not store credentials, invent permission/coordination evidence or mutate raw
receipts. Hash checks are byte/provenance checks, not proof of remote authenticity.
The external contract in spec.md is a prerequisite for future owner work,
not a new tracked API. If genuine authorized ownership is later established,
return to design for the smallest change to that actual owner and consumer.

## Validation and quickstart

For this design, while chief's three-file commit is HEAD:

```bash
bash bundles/self/scripts/design-check.sh check .
```

This is a local structural preflight; the sequence's dispatched pinned check
owns the design verdict. It must select `required-merge-checks` from HEAD,
find committed nonempty clean artifacts and their required headings/checkboxes.

For implementation, first run the retained packet verification, then compare
Gates against the captured tuples. Passing input is the exact unique set.
Controls must reject omission of Brokkr, omission of Docker, omission of an
original context, an extra context, duplicate entries, a wrong/missing app
and a tag-only addition. Evidence-shape controls reject contexts/checks
mismatch, strict/admin false or missing, malformed/missing rules and modified
file bytes; missing data cannot default to empty. Review the diff for intact
commands, one inventory, precise CI/publication scopes, and no contradictory
current instructions or completion claims. These are offline documentation
checks, not the independent live refusal experiments.

Existing `delivery_constitution` and `delivery_recipes` tests can be evidenced
within `cargo test --workspace --locked`; do not rerun them solely to duplicate
logs. Execute every gate in the authoritative
[house inventory](../../docs/house-rules.md#gates--part-of-every-change),
including its selftests and proofs, fail-closed native-tls absence and pinned
shim/proof identities with the recorded digest-matched engine. Keep exact
coverage and locked operations. Offline verify installs nothing. CI-only
checks and tag-specific publication proofs retain their distinct scopes.

The engine must collect separate Astra correctness, official DeepSeek Flash
security, Sonnet 5.5 tests, Astra guarantees and Fable 5.1 independent results.
No engine-lane fallback, inferred success or DeepSeek Pro. Chief does not
invoke that downstream review panel or count design positions as reviews.

External follow-up must prove the actual helper/consumer against every required
name omitted, wrong/missing app, pending/non-success, stale head, ambiguous
rerun, malformed/partial/read failure, timeout, restart and concurrent receipt
isolation, plus the all-required-success case and deployment. Independent
Brokkr-red and Docker-red live server refusals require PR/run/head evidence
and exclusion of other blockers. Both remain PENDING; no local fixture or
generic BLOCKED response substitutes, and this run cannot create the probes.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Snapshot ages or is mistaken for a live read | Preserve timestamp/hashes and qualify observation; future drift requires fresh authorized evidence, not stale PATCH replay |
| Unknown helper owner leads to a parallel system | Record actual path and bounded search scope; defer repair/activation until owner and consumer are established |
| Host helper still reports false readiness | Keep defect and external tests explicit; accept unfinished issue work, never certify that helper as safe |
| Missing/duplicate/app-substituted inventory entries | Unique tuple equality plus negative controls; keep one maintained list |
| Tag-only checks accidentally block PRs | Inspect actual job conditions and preserve publisher/rate scope distinctions |
| Documentation is mistaken for deployed enforcement | Separate repository acceptance, external activation and independent live refusal proofs |
| Run-local packet disappears | Durable spec provenance plus operator archival receipt; leave archival pending until a location is evidenced |
| Old instructions trigger forbidden writes | Replace executable mutation/probe steps with dated history and explicit no-mutation scope |
| Small diff bypasses full proof | Complete house inventory, pinned verify and five separate reviews; unexecuted proof remains open |
| Whole issue is closed after partial delivery | Obligation map and unchecked external tasks retain watcher, live refusals, cleanup and integration gaps |
