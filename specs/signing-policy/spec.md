# Feature specification — unsigned work, operator branch sealing

Date: 2026-10-20. Feature branch: `signing-policy`. Phase: design.

Decision: 0056 — [signing policy](../../docs/decisions/0056-signing-policy.md).

## Purpose and authority

Six epic #120 runs finished their implementation but stopped at IMPL-BLOCKED
because the forwarded GPG agent disappeared. Signing belongs to the operator
after delivery, not to the seats producing the work. Seats and the operator's
assistant commit unsigned, never push, and do not stop for signing. The
operator seals every PR branch before merge, preserving its content and
history shape while adding the operator's signature to its commits.

The operator explicitly ruled this policy for swarmail on 2026-10-20. The
chief's semantic decision artifact is **proposed**, as required by the design
sequence; it records that already-given authority without claiming operator
acceptance of the chief's additional implementation choices. Implementation
records the requested **accepted (operator, 2026-10-20)** policy decision
under the existing authorization. No additional permission to commit unsigned
is needed. Earlier proposed decisions 0045 and 0050 remain proposed.

Evidence read: both run-local positions, the reference sealing script, and
`specs/signing-policy/framing.md` (the intake location in this checkout;
`.brokkr/state/tasks/` is absent). The operator reports the reference proven
on a scratch merge history and lanetally PR #230. That is evidence for the
reference, not a claim that this adaptation has already passed signing tests.

## Chief's rulings on the panel

| Dispute | Ruling and cost of rejecting the other position |
|---|---|
| Who signs | Adopt both panels: one operator owns the end-of-work seal. No agent recovery service, seat key, signing retry, or extra delivery phase. Intermediate commits intentionally carry no signature. |
| Reference rebase versus literal “never rebase” | Adopt simplicity's parent-first object amendment. Reject robustness's retention of `git rebase --force-rebase`: even on the same base it invokes rebase and can replay merges. Re-create commits from their existing trees and exact metadata, mapping parents, then atomically move the branch. This costs a small object map and loses direct applicability of the reference's successful rebase-path evidence. No patch replay, squash, or base movement is allowed. |
| Fetch inside the tool | Adopt simplicity: remove implicit fetch. The operator fetches before invocation; the tool captures the resolved base object and merge-base once. Reject robustness's fetched-ref requirement inside the tool: its cost is that stale local refs are the operator's responsibility. `--check` can then be offline and repository-read-only. |
| Proof ownership | Adopt simplicity's canonical tool and test under `bundles/self/scripts/`, with fixed public shims. Reject robustness's unpinned candidate tool copied into scratch: relocation does not establish provenance. Cost: changes to the operator tool participate in bundle pinning. Merely storing it in the bundle never authorizes a seat to sign. |
| History checks and cleanup | Adopt robustness's preservation and fail-closed intent, but reject keeping the reference's checks verbatim. Its `shape()` discards parent identity/count, and shell command substitution cannot carry NULs. Compare exact object fields through files and an explicit ordered parent map. Keep one cleanup owner; avoid reset/checkout/rebase-abort by leaving HEAD and the worktree untouched until verification succeeds. |
| Key and tool failures | Adopt robustness's explicit prerequisites, bounded signing operations, and quoted argv. Adopt simplicity's absent-key semantics: absent or unusable key is signing unavailable (3 on a seal attempt, 0 with an unavailable report on `--check`), not invocation error 1. Do not hide this distinction behind a dummy-key-only test. |
| Test isolation | Adopt robustness's complete Git/GPG environment isolation and merge fixture. Reject its local-origin workaround because the tool no longer fetches. A local bare remote is useful only to prove that `--check --push` cannot update it. |

## Scope and user scenarios

P1: a seat can commit completed work while the operator's agent is absent.
P1: the operator can inspect a branch offline, then seal it when signing is
available, with no change to author content or base.
P1: CI and verify can prove check/refusal behavior without a private key.

No Rust production changes, mail-protocol changes, dependency additions,
coverage exclusions, engine changes, key provisioning, automatic merge,
push from a seat, PR creation, or issue writes are in scope. Decisions
0001–0005 and append-only `main` remain intact.

## Acceptance Criteria

1. **Unsigned work and recorded policy.** House rules and every active seat
   instruction say commit unsigned, never push, and never stop for signing.
   Keep the imperative-message/why-body rule. The operator seals after the
   last run, before merge and before updating the branch from main, so a
   subsequent GitHub-created merge commit retains GitHub's signature. No
   sealing operation invokes rebase. Distinguish historical author-commit
   signatures, sealed branch authorship added by the operator at the end,
   GitHub landing signatures, and signed release tags. Preserve recorded
   historical exceptions; do not invent exceptions or rewrite old records.
   Final decision 0056 records accepted operator authority dated 2026-10-20,
   narrowly supersedes the mid-run signing requirement in 0045 §9, and
   explains the content-keyed proof and rejected alternatives.

2. **CLI compatibility.** Executable `scripts/seal-branch.sh` provides
   `-C <repo>`, `--base <ref>`, `--key <keyid>`, `--push`, `--check`, and help.
   Default base is `refs/remotes/origin/HEAD`, falling back to `origin/main`;
   default key is `user.signingkey`. Missing operands, invalid refs, missing
   required executables, unsupported repository state, detached HEAD, and
   dirty tracked index/worktree refuse with 1 and `seal-branch:` diagnostics.
   Untracked files are not touched. Refuse unresolved Git operations and
   sealing the base branch itself. No fetch occurs inside the tool.

3. **Check and no-op behavior.** For a nonempty range, `--check` reports
   `branch=… base=… merge-base=… commits=… unsigned=… signing=available|unavailable`
   and the fact that committer identity/date may change during sealing.
   Counts cover all reachable commits in the range, including side branches
   and merge commits; unsigned means signature status other than `%G?=G`.
   Check exits 0 even without a configured/reachable key and never changes
   refs, index, tracked/untracked files, or pushes, including `--check --push`.
   An empty range exits 0 with “nothing to seal”, without requiring a key or
   signing tools. All-good signatures are an idempotent no-op; this retains
   the reference's `%G?=G` trust policy, not a new fingerprint identity test.

4. **Safe seal and exits.** When sealing is needed, missing/locked/unreachable
   key or an unsuccessful bounded availability probe returns 3 before branch
   mutation. Missing `gpg`/`timeout` is 1, not 3. A failure after signing starts
   returns 1. Re-create every commit in the range in parent-first order and
   sign it; preserve each tree, exact message bytes, full author identity and
   date including timezone, and ordered parent edges via an old-to-new map.
   Parents outside the range remain the same OIDs. Committer identity/date
   and signature may change, and therefore commit IDs change. Before updating
   the branch verify count, every mapped field, every `%G?=G`, and unchanged
   merge-base against the captured base OID. Any unsupported commit encoding
   or metadata that cannot be preserved refuses explicitly before publication.
   A failed check leaves the original branch and checkout intact. Update the
   ref with comparison against the captured old head, never overwrite a
   concurrent update. No reset, checkout, patch replay, or merge recomputation.

5. **Optional operator push.** Push only after successful verification or an
   already-signed no-op, using `--force-with-lease=refs/heads/<branch>:<old>`
   and an explicit source/destination refspec to `origin`. Empty-range and
   check modes never push. A rejected push returns 1 and leaves the verified
   local seal available for inspection; it is not a failed content check.
   Never retry a lease failure automatically. This feature's delivery does
   not invoke the push or successful signing paths.

6. **Real keyless proof.** A pinned shell test and fixed shim run in the normal
   CI quality and hermetic verify gates. Scratch Git repositories use unsigned
   commits/tags, isolated HOME/XDG/Git config, private empty GNUPGHOME, no
   inherited agent socket variables, and GPG `--no-autostart`. No key creation,
   host agent, network, mocks, sleeps, or skip-on-missing-tools. Prove exact
   check fields/counts on a graph containing a merge; exit 3 for both unset key
   and configured-but-missing key; dirty tracked tree and detached HEAD exit
   1; empty range exits 0. Compare branch ref, symbolic HEAD, index/tree and
   fixture bytes before/after each path, including preserving dirty contents.
   Also prove missing operands/invalid base refuse and `--check --push` leaves
   a local bare remote unchanged. Deliberately broken guards/reports must
   make the corresponding proof fail. Signing success is operator evidence,
   never a CI claim; the prior evidence does not prove interruption handling
   or this new object's signing implementation.

7. **Pinned execution.** Verify runs the pinned test and canonical tool beside
   itself, never the candidate's `scripts/` implementation. Both public shims
   have pinned digests. Missing resources or changed shims refuse. Wire the
   new proof into verify's resource checks/run list and CI's existing quality
   job; list it in the house gate inventory. Keep existing tamper proofs and
   toolchain identity checks effective. Active runs must dispatch a bundle
   containing the new proof before claiming it was executed.

8. **Delivery evidence.** Final implementation is on `signing-policy`, cut
   from current main after a successful fetch, clean and committed unsigned.
   Run the complete house gates plus the seal proof, preserving exact
   100/100/100 coverage and the fail-closed native-tls absence check. No push,
   PR, merge or issue mutation. A blocked/unrun command remains outstanding.
   The chief commits only spec, plan, tasks and the proposed decision;
   implementation owns runtime scripts, policy documentation and final gates.
