# Reconcile the required merge checks

Created: 2026-10-24. Status: draft for scoped documentation implementation.
Issue: [#205](https://github.com/feedback-loop-ai/swarmail/issues/205), epic #192.
Run: `continue-epic-192-issue-205-usin-17f8493f`.

## Feature and rationale

Repair the sole merge-blocker inventory in
[house rules — Gates](../../docs/house-rules.md#gates--part-of-every-change)
using the supplied, hash-recorded host evidence. Reconcile this directory's
three artifacts with what the repository can deliver. The two requested
checks are already present in the captured protection. Replaying the earlier
settings operation would add risk without repairing the remaining gap.

The actual host observer can falsely report all-CI-pass when a required check
is missing. Its persistent owner and consumer remain unidentified. This
repository deliverable therefore consists of an accurate inventory and an
explicit external handoff; it does not complete watcher enforcement or issue
#205. Both independent live refusal experiments remain required and PENDING.

This is engine/gate work under accepted
[0057](../../docs/decisions/0057-codex-delivery-recipes.md) and
[0058](../../docs/decisions/0058-codex-review-routing.md), even though the
permitted tracked changes are Markdown. It restores documentation of existing
obligations under [0007](../../docs/decisions/0007-hermetic-gates-single-model.md),
[0045](../../docs/decisions/0045-delivery-repairs.md) and
[0050](../../docs/decisions/0050-delivery-gates.md). No frozen guarantee,
public mail contract or accepted ruling changes; no new ADR is declared.
The conditional external watcher requirements below describe unfinished issue
work, not a newly deployed repository interface or accepted policy amendment.
Any future scope that contradicts an accepted ruling requires a proposed
amendment and operator acceptance before conflicting implementation.

## Evidence and ownership

The packet is `.forge/epic192/issue205/host-evidence/`, captured at
`2026-10-24T18:06:37.978922+00:00` for `feedback-loop-ai/swarmail`, branch
`main`, SHA `dc1822a45e4d2ae8326b44baf22818187a8320e9`.
Chief recomputed all nine file lengths and SHA-256 values in
`host-evidence-manifest.json`; all match. The manifest's SHA-256 is
`3210d84bb16eeebc60e81a2c1ade594500ad3728656563c167e7f144c745d92f`.
`current-main-protection.json` has nine unique context/app tuples, all bound
to GitHub Actions app `15368`; its contexts and checks agree. `strict=true`
and `enforce_admins.enabled=true`. `rulesets.json` and `effective-rules.json`
are explicitly empty arrays. The issue receipt says OPEN. These are dated
host observations, not live API reads by chief or perpetual drift guarantees;
hashes establish byte consistency, not independent remote authenticity.

The earlier `.forge/epic192/issue205/action-receipt.json`, recorded at
`2026-10-24 17:59:03 UTC`, reports the narrow PATCH preserving seven checks
and adding the two requested checks, followed by read-back preserving unrelated
fields. It references `protection-before.json` and `protection-after.json`.
That receipt is separate from the nine-file manifest. It is supplied historical
evidence, not an operation or permission/coordination proof invented by this
seat. No API mutation is authorized in this continuation.

The actual observed source is
`/home/valentin/source/feedback-loop-ai/swarmail/.forge/epic192/ci-watch.py`.
Its verbatim packet copy is
`.forge/epic192/issue205/host-evidence/host-ci-watch.py`, SHA-256
`6c9e88dcd3af91116e808c2ecfd49a7e62f3ef2ec2c9a84d4b03d9dc687967b1`.
It is an ignored host helper, untracked in this checkout. The saved bounded
Swarmail search found no tracked watcher/caller. The Brokkr search at
`54d5a14e8401bb0feb3fce02b9183396a8adfe9f` found protection snapshot/tests,
not a watcher/caller. Bounded searches do not establish global absence.

This helper polls `gh pr view` and never merges. It is a plausible referent
for the issue's delivery watcher, but its identity as that admission path,
persistent source owner, launcher, consumers and activation mechanism are
UNRESOLVED. No authorized persistent owner in this repository is evidenced.
A separate driver is unidentified, not presumed to be a Brokkr component.
Do not edit the evidence copy, create a second generic watcher, or claim that
tracked documentation updates or deploys the ignored helper.

Its nonempty observed-rollup predicate accepts COMPLETED checks with SUCCESS,
SKIPPED or NEUTRAL, without required-name/app completeness or candidate-head
binding. Chief evaluated the AST-extracted predicate offline: omitting
`brokkr bundle (pinned)` or `docker smoke` separately from otherwise successful
fixtures still returns true. The helper prints the observed head but does not
validate it, ignores fetched PR state/mergeability, and times out after 180
iterations by printing and falling through to exit zero. Subprocess reads are
unbounded, so the nominal thirty minutes is not a hard deadline. Each poll
overwrites the same per-PR receipt. This establishes defects in the supplied
source, not live server refusal, consumer behavior or a deployed repair.

## Dated scope clarification — 2026-10-24

The earlier clarification at `05d1a01:specs/required-merge-checks/spec.md`
superseded `epic-192-issue-205-continuation--a4c15323` with a settings-only
operation. The broader design remains at `bb8b040:specs/required-merge-checks/`.
That dated history is preserved; the following host-evidence clarification
supersedes its current execution instructions, including the exclusion of
watcher discovery and the requirement to obtain API access before any repair.

## Dated host-evidence clarification — 2026-10-24

The supplied packet resolves the raw issue/settings/source access gaps enough
to proceed with the authorized documentation work. It does not resolve watcher
ownership or behavioral proof. Prior failure results `5fd46e63`, `fce52e40`,
`5ee4ef54` and `da9ae7e8`, and escalation run
`continue-epic-192-issue-205-from-14fccc86`, remain historical evidence; this
continuation neither erases their failures nor claims their work succeeded.

| Issue or historical obligation | Current disposition |
|---|---|
| Require the two added checks on main | Present/read back in supplied dated host evidence; no repeat write and no continuous remote-state claim |
| Inventory agrees with required checks | Repository implementation still required in Gates; this design commits only spec/plan/tasks |
| Old AC3–AC4: actual watcher owner, repair, review and activation | External, UNRESOLVED; exact helper known, intended-driver identity/owner/launcher/consumer unproved |
| Old AC5: genuine Brokkr-only red refusal | **PENDING** independent live proof, with other blockers excluded and attributable server refusal |
| Old AC6: genuine Docker-only red refusal | **PENDING** separate live proof with the same attribution standard |
| Old AC7: durable probe receipts and cleanup | Unproved; this run creates no probes and does not clean up others' resources |
| Old AC8: root integration, CI and complete issue evidence | Outstanding; local documentation and captured settings do not discharge it |
| Raw packet archival handoff | Operator-accessible retention receipt outstanding; ignored worktree files alone are not durable archival evidence |
| Candidate gates and five-role review | Outstanding downstream; design authoring is neither verification nor review success |

No API mutation, push, PR/ref creation, merge, bypass, release or issue closure
is authorized here. Keep #205 open. A future authorized fresh observation of
drift requires reconciliation under that authority; it does not reactivate the
old PATCH/retry plan. Fresh API access is not a prerequisite for describing this
explicitly dated observation.

## Council synthesis and rulings

Both current positions were read with the intake framing. They substantially
agree; there is no genuine conflict about current implementation scope. The
chief rules their different emphasis as follows:

| Choice | Adopted ruling, rejected alternative and cost |
|---|---|
| Owned scope | Adopt simplicity's four existing Markdown files across design/implementation and robustness's full obligation map. Reject replaying settings writes or inventing a watcher owner. Cost: the external defect and issue remain open. |
| Inventory | Adopt both positions' one exact list in Gates, bound to app 15368, with dated strict/admin observation. Reject a second maintained list in specs or executable registry. Cost: a retained run-local comparison must be repeated when the observation changes. |
| Watcher handoff | Adopt robustness's identity, candidate-head, timeout and activation concerns as external acceptance requirements. Adopt simplicity's refusal of speculative infrastructure. Reject treating a standalone new predicate as repair of the real consumer. Cost: conservative eventual readiness can delay delivery, and owner discovery remains necessary. |
| Proof | Adopt simplicity's small run-local comparison with robustness's omission, duplicate, producer and malformed-evidence controls. Reject a permanent Markdown parser or mock service solely for this repair. Cost: explicit evidence retention instead of a new maintained test framework. |
| Publication scope | Adopt both positions' separation of PR checks from tag rate/chart publication and main-or-tag image publication. Reject calling all publication tag-only. Cost: retain precise scope in the existing inventory prose. |
| Delivery bar | Adopt both positions' complete house gates and five separate 0058 reviews. Reject a cheap lane for a small diff or claiming configuration equals refusal proof. Cost: full verification and unresolved live experiments can delay closure. |

## Requirements

Gates shall contain exactly the captured nine required context names, verbatim,
with a common binding to Actions app `15368`, dated strict/admin observation
and evidence provenance. Preserve every existing command and CI-only proof.
Keep this as the sole maintained gate inventory; these spec artifacts reference
it rather than enumerate another complete set. `release-rate-5k` (display name
`release rate 5k (tags only)`) is a tag proof, not a PR blocker. Publication jobs
are not PR blockers: chart publication is tag-only, image publication also runs
on main pushes. No job names, workflows or accepted decisions change.

An external owner must first establish the actual watcher and consumer, source
of truth, authorized persistent source location and activation procedure.
Only then can a separately scoped repair be designed. Required readiness
must positively establish every required name/app tuple for an explicit repo,
PR and candidate head, from authoritative current observations. Missing/wrong
apps or names, unknown/partial/malformed observations, queued/in-progress,
non-success (including skipped/neutral), stale heads and ambiguous reruns
cannot satisfy a required check. Extra unrelated/tag-only rollup entries need
not equal the required set; preserve the existing observed-failure refusal
unless the owner's design explicitly resolves that behavior.

The real owner must bind current run identities, recheck the expected head and
open PR/strict-base eligibility at readiness, use bounded reads and a deadline,
and make timeout/read exhaustion non-ready/nonzero. Restart revalidates current
truth; concurrent invocation receipts must not overwrite each other's proof.
A final read is not atomic merge authorization: any eventual consumer remains
bound to the same expected head and server enforcement. SUCCESS-only is a
stricter watcher requirement than GitHub's acceptance of skipped/neutral; do
not misdescribe it as current helper or server behavior. Tests must exercise
the actual owned helper and consumer, with separate activation evidence.
These are unresolved external obligations, not coding permission in this run.

## Acceptance Criteria

- **AC1 — Evidence fidelity.** Reproducible validation checks all nine manifest
  lengths/hashes, nine unique tuples with matching contexts/checks, app 15368,
  strict/admin true and explicit empty rules arrays. Missing/malformed data
  fails validation. Record capture/revisions and distinguish host observations
  from seat actions, live truth and the separate historical action receipt.
- **AC2 — One accurate inventory.** Gates' exact required context/app set equals
  the validated packet with no duplicates, omissions or extras. Retained
  comparison controls reject each new check omitted separately, an old check
  omitted, duplicate/extra context, changed app and a tag-only addition. Diff
  review proves existing commands/CI responsibilities and publication scopes
  intact. This is documentation proof, not live enforcement proof.
- **AC3 — Honest watcher disposition.** Spec/plan/tasks retain the exact source
  path/hash and bounded search revisions, identify unknown owner/consumer and
  activation, and claim no deployed repair. The source predicate reproduction
  accepts each missing-check fixture, documenting the defect. Eventual external
  completion requires the actual consumer to refuse missing identities,
  pending/non-success/stale or ambiguous observations, timeout and read errors,
  plus real activation proof; these remain unsatisfied here.
- **AC4 — No erased issue criteria.** Current tasks contain no repeat PATCH or
  capability-acquisition dependency for the documentation. Historical entries,
  archival handoff, watcher/cleanup/integration obligations and both independent
  live refusals remain visible. Each live proof must link PR/run/head and
  attributable server refusal for its genuine red check with other blockers
  excluded. Configuration, offline fixtures, generic BLOCKED or green CI do
  not satisfy it. No forbidden remote action occurs; #205 stays open.
- **AC5 — Required delivery proof.** The implementation candidate passes the
  entire authoritative house inventory and pinned hermetic verification, with
  CI-only obligations separately identified, and obtains distinct Astra
  correctness, official DeepSeek Flash security, Sonnet 5.5 tests, Astra
  guarantees and Fable 5.1 independent results under accepted 0058. No
  engine-lane fallback or DeepSeek Pro; worst verdict, protected review and
  hard security hold remain. Failed/unexecuted gates and external work stay
  outstanding. This draft claims no implementation or whole-issue completion.
