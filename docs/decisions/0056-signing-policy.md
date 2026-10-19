# 0056 — Commit unsigned and seal the branch before merge

- **Status:** proposed — chief's design record; acceptance of additional design choices is the operator's.
- **Date:** 2026-10-20
- **Operator authority already supplied:** accepted policy ruling (operator,
  2026-10-20 — the operator ruled this change for swarmail in chat).
- **Supersedes in scope:** the mid-run signing requirement described in
  proposed [0045](0045-delivery-repairs.md) §9, not that record's other rules
  or its proposed status.
- **Preserves:** accepted [0019](0019-release-tag-integrity.md), append-only
  main, and the existing pinned proof ownership boundary.
- **Design:** [spec](../../specs/signing-policy/spec.md),
  [plan](../../specs/signing-policy/plan.md),
  [tasks](../../specs/signing-policy/tasks.md).

## Context and operator ruling

Forwarded GPG-agent availability repeatedly stopped delivery after the work
was finished: six epic #120 runs reached IMPL-BLOCKED for signing. On
2026-10-20 the operator ruled that nothing signs mid-run in swarmail. Seats
and the operator's assistant commit unsigned, never push, and do not stop
for signing. This clone uses repo-local `commit.gpgsign=false`.

The operator adds their signature once, after the final run and before merge,
by sealing every PR-branch commit on the same merge-base. The requested
accepted policy record is authorized by that ruling; implementation records
its status as **accepted (operator, 2026-10-20 — ruled in chat)**. The chief
retains proposed status here to satisfy the design sequence's semantic draft
contract and distinguish its implementation recommendations from operator
acceptance. This is not a request to re-authorize unsigned work.

## Proposed mechanism implementing the ruling

1. `scripts/seal-branch.sh` is the operator entry point, with `-C`, `--base`,
   `--key`, `--push` and `--check`. The canonical implementation and keyless
   proof live in the pinned bundle behind fixed, digest-checked shims. No
   seat or CI invokes successful signing. The operator fetches beforehand;
   the tool never fetches and freezes the selected base and merge-base OIDs.
2. Amend the commit graph in place by reconstructing objects from the
   original trees, exact messages, original authors and ordered mapped
   parents. Never squash, replay patches, recompute merges or invoke rebase,
   including rebase onto the same base. Parents outside the sealed range
   keep their OIDs. Committer identity/date and signatures may change.
   Verify every mapped commit and good signature, then atomically update
   the branch with the captured old OID as the expected value. Unsupported
   metadata refuses rather than changing silently. The old branch remains
   intact on any failed preservation check; no worktree reset is necessary.
3. Exit 0 for sealed/already-good/empty range and valid check reports;
   3 for unavailable signing on a seal attempt; 1 for other refusals.
   Check reports unavailable signing without failing and never pushes.
   Missing prerequisite executables are an error, not an unavailable agent.
   The existing `%G?=G` all-good policy is retained: it does not newly prove
   that a pre-existing signature belongs to a particular fingerprint.
4. Optional push leases explicitly on the old branch OID. A remote change
   refuses, with no automatic retry. Push failure leaves a verified local
   seal for the operator; it is not a failed content check. Main remains
   append-only. This operator-authorized PR-branch repair is the limited
   force-push exception; seats still never push.
5. Seal before updating the PR branch from main so a subsequent
   GitHub-created merge commit keeps GitHub's signature. Record the distinct
   meanings: historical author-commit GPG signatures; **sealed branch
   authorship**, the operator signature added at the end; GitHub landing
   signatures (including squash merges); and signed release tags. Preserve
   historical exceptions as recorded, without treating them as standing
   permission or inventing additional historical facts.

## Why delivery proofs survive a seal

The supplied operator framing and reference describe brokkr's
delivered-by-brokkr check as comparing per-file patch IDs from the
merge-base. Preserving the base and every tree preserves that content map;
new signatures and committer metadata change commit IDs, not those patches.
This records the engine mechanism supplied by the operator, not an engine
test performed by this repository. SHA-keyed run-local references may become
stale, which is another reason sealing follows the final run. Other systems
that require a particular commit SHA are not claimed to survive rewriting.

## Evidence and consequences

The operator's reference script was reported proven on a scratch repository
containing a merge and on lanetally PR #230. It invokes same-base rebase;
the literal no-rebase requirement takes precedence in this design. That
evidence cannot be relabeled as execution of the adapted object-construction
path or as proof of interruption recovery. Those remain operator-only
validation. Automated gates use real disposable Git histories and isolated,
empty GPG homes to prove report and refusal behavior without signing keys.

Unsigned intermediate history is intentional. Exact preservation costs an
old-to-new object map and byte comparisons. A failed pre-publication attempt
may leave unreachable Git objects, but not altered branch content. Catchable
interrupts clean temporary files; SIGKILL cleanup is not promised. Pinning
the canonical operator tool costs a bundle change when it changes, preserving
the existing proof ownership boundary rather than executing mutable scripts.

## Alternatives rejected

- **Sign every seat commit or supervise the forwarded agent:** retains the
  availability dependency that blocked six completed runs.
- **Squash to add one signature:** collapses author commits, fails the
  per-commit preservation requirement, and a GitHub squash signature attests
  landing rather than the operator's sealed branch authorship.
- **Server-held seat key:** refused by the operator because anything on the
  box could then sign as the operator. No convenience benefit justifies that
  authority transfer.
- **Keep the reference's rebase implementation verbatim:** preserves direct
  applicability of its reported tests but violates the literal no-rebase
  instruction and introduces replay/abort state. Object amendment costs new
  review and operator-only signing validation instead.
- **Unpinned operator implementation copied by a pinned test:** copying does
  not pin code. Keeping both canonical scripts in the bundle costs one extra
  shim and preserves transitive proof ownership.
