# 0056 — Commit unsigned and seal the branch before merge

- **Status:** accepted (operator, 2026-10-20 — the operator ruled this
  change for swarmail in chat).
- **Date:** 2026-10-20
- **Supersedes in scope:** the mid-run signing requirement — house rules
  "Commits and history" as it stood, the implementer charter's "Commits are
  signed", and proposed [0045](0045-delivery-repairs.md) §9's branch-author
  sentence. 0045's other rules and its proposed status are untouched, as is
  proposed 0050.
- **Preserves:** append-only `main`, signed release tags, the pinned proof
  ownership boundary (proposed 0050 §1) and decisions 0001–0005 (no `src/`
  change).
- **Design:** [spec](../../specs/signing-policy/spec.md),
  [plan](../../specs/signing-policy/plan.md),
  [tasks](../../specs/signing-policy/tasks.md). The design commit drafted
  this record as *proposed* to keep the chief's implementation choices
  distinct from the operator's ruling; the ruling itself was already given,
  and this revision records it as accepted.

## Context

GPG signing at every commit depended on a forwarded gpg agent being
reachable from the seat. It dropped repeatedly — no agent in the session,
an unattended pinentry cancelling after two minutes, an expired cache — and
six runs of epic #120 stopped at IMPL-BLOCKED with their work finished. The
signature was gating delivery on the availability of a socket, not on
anything about the work.

## Ruling

1. **Nothing signs mid-run.** Seats and the operator's assistant commit
   unsigned (this clone sets `commit.gpgsign=false` repo-locally), never
   push, and never stop or report `blocked` because signing is unavailable.
2. **The operator seals every PR branch once, before merge,** by re-signing
   every commit of the branch in place on the same merge-base: amend, not
   squash, and never a rebase onto a newer `main`. Each commit's tree,
   message and author must be unchanged, checked, and the branch is
   published with a leased force-push.
3. **Seal before updating a branch from `main`,** so the merge commit
   GitHub creates for that update keeps GitHub's signature and is never
   re-signed as the operator's.
4. The signatures on record keep distinct meanings: **sealed branch
   authorship** (the operator's signature added at the end; author commits
   signed at commit time before this ruling carry the same meaning),
   **GitHub landing** (squash merges and GitHub-created merge commits) and
   **signed release tags**. Historical exceptions stay as recorded; none
   is a standing permission.

## Mechanism — amend in place

`scripts/seal-branch.sh` is the operator's tool, a fixed shim over the
pinned `bundles/self/scripts/seal-branch.sh`, adapted from the operator's
reference (`.brokkr/state/seal-branch.reference.sh`) and keeping its
contract: `-C`, `--base`, `--key`, `--push`, `--check`; exit 0 sealed or
nothing to seal, 3 signing unavailable, 1 any other refusal.

- **Amend, object by object.** The reference amended each commit with
  `git rebase --force-rebase --exec 'commit --amend -S'` onto the
  merge-base. The adaptation does the same amendment without the rebase
  machinery: walking `merge-base..HEAD` parent first, it re-creates each
  commit with `git commit-tree -S` from its own tree, its exact message
  bytes, its author name, email, date and zone, its encoding header, and
  its ordered parents mapped to their sealed copies (parents outside the
  range keep their IDs). No patch is replayed and no merge is recomputed,
  so a merge commit cannot be re-resolved and no rebase state can be left
  behind. Only the committer and the signature change, and with them the
  commit IDs. A header it cannot carry over (`mergetag`, anything
  unknown) refuses.
- **Check before the ref moves.** Every sealed commit is compared with its
  original — all kept header fields byte for byte, parents through the
  map, message bytes — and then the whole range again from the sealed
  head: same count, same head tree, merge-base unchanged against the base
  OID captured at the start, every commit `%G?` = `G`. Only then does
  `git update-ref` move the branch, compare-and-swap against the captured
  old head. A failed check therefore "restores" the branch by never having
  moved it: no reset, no checkout, the index and worktree untouched.
- **Refusals before anything.** Detached HEAD, a dirty index or worktree,
  an unfinished merge/rebase/cherry-pick, shallow history, grafts, the base
  branch itself, an invalid base, zero or several merge-bases. The tool
  never fetches: the operator fetches first and the base is frozen at the
  OID it resolves to. Signing availability is a bounded probe that never
  starts an agent; a missing, locked or unreachable key is exit 3 before
  any object is written, and `--check` reports it with exit 0.
- **Push last.** `--push` force-pushes with
  `--force-with-lease=refs/heads/<branch>:<old head>`; a moved remote
  refuses with exit 1, keeps the verified local seal and is never retried.
  `--check --push` never pushes.

`bundles/self/scripts/seal-branch-test.sh` (shim `scripts/seal-branch-test.sh`)
proves the keyless arms in CI's quality job and the verify seat: scratch
repositories with unsigned commits and tags, isolated git config, a fresh
`GNUPGHOME` with `no-autostart`, no inherited agent sockets. It covers
`--check`'s exact report over a merge graph, exit 3 with no key and with a
configured or passed key gpg cannot reach (branch, refs, index and worktree
bytes unchanged), refusal on a dirty tree, a dirty index and a detached
HEAD, "nothing to seal" at the base, and the other refusals; then it
breaks each guard and report in a disposable copy of the tool and requires
the owning case to fail. No key is provisioned anywhere in the gates.

## Why content-keyed gates still vouch for a sealed head

brokkr's delivered-by-brokkr check is keyed on content, not commit IDs: it
compares per-file patch IDs from the merge-base. Sealing keeps the
merge-base and every tree, so every file's patch from the merge-base is
the same; new signatures and committer lines change commit IDs only. A
re-signed head is therefore still the head the run delivered. Run-local
records keyed on a commit SHA (the ledger's "Repository head") name the
pre-seal head — one more reason sealing comes after the final run. This
is the engine behaviour the operator described; this repository does not
test the engine.

## Evidence

- The reference tool is proven by the operator on a scratch repository
  with a merge commit and on lanetally PR #230. That evidence is for the
  reference's rebase path, not this adaptation's object path.
- The implement seat exercised the adaptation's signing path once with a
  throwaway key in a scratch `GNUPGHOME` (not the operator's key, not in
  any gate): six commits including a merge, an ISO-8859-1 encoded message,
  a message without a final newline and a `+0530` non-ASCII author were
  sealed with all fields and parent counts unchanged and every commit
  reading `G`; a second run reported "already signed"; `--push` against a
  stale remote was refused by the lease, and against a matching remote
  pushed. The operator's first real seal is the authoritative proof.

## Alternatives weighed

- **Keep signing every commit, or supervise the forwarded agent.** Keeps
  delivery hostage to the socket that stopped six finished runs.
- **Squash to one signed commit.** Collapses the author commits the review
  read, breaks the per-commit tree/message/author check, and a GitHub
  squash signature attests landing, not the operator's authorship.
- **A server-held seat key.** Refused by the operator: anything on the box
  could then sign as the operator. No convenience is worth moving that
  authority onto the build host.
- **The reference's rebase verbatim.** Directly covered by its evidence,
  but even onto the same merge-base it replays merges and can leave rebase
  state to abort and reset; the object path keeps the same contract with
  nothing to unwind, at the cost of evidence of its own.

## Consequences

Intermediate history on PR branches is unsigned by design until the
operator seals it. Sealing is one operator command per PR, after the final
run and before merge. The tool lives in the pinned bundle, so a change to
it is a bundle change and is proved the way the other gates are.

## Addendum (2026-10-31)

Operator ruling, in chat: delivery becomes fully automated. Once the bot of
[0109](0109-automated-delivery-bot.md) is live, it seals and merges under its
own key in place of ruling 2's operator seal. Until then ruling 2 stands. The
refusal of a server-held key that signs as the operator stands; the bot's key
does not sign as the operator. Rulings 1, 3 and 4 and the operator's release
tag signature are unchanged.

## Addendum (2026-11-01): no seal; merge on green

Operator ruling, in chat, after weighing the delivery bot of
[0109](0109-automated-delivery-bot.md): ruling 2 is withdrawn. The operator
no longer seals PR branches, and the 2026-10-31 addendum's bot does not
replace it.

- **Why:** `main` requires its CI checks but not signatures or a review, and
  the operator's GitHub token is reachable by everything running as the
  operator's user. The seal was therefore a convention the pipeline honoured,
  not a control GitHub enforced, and it stopped finished work for hours
  whenever the forwarded agent was unreachable. A bot key would not have
  changed that while the token stays.
- **The rule now:** the operator's assistant merges a delivery run's PR by
  squash once every house gate passes on the final tree (after `main` is
  merged in) and every required CI check is green. Branch commits stay
  unsigned; the squash commit on `main` carries GitHub's landing signature.
- **Held for the operator, never merged on green:** a PR that changes a
  decision's status to accepted; adds a package to `Cargo.lock` or
  `evals/Cargo.lock` or changes `deny.toml`; or changes `.github/`,
  `scripts/`, `.brokkr/` (recipes, policies, charters, adapters) or the
  toolchain configuration. The operator may approve a held PR by name.
- **Unchanged:** rulings 1, 3 and 4 (author commits unsigned, no rebase onto a
  newer `main`, the signature meanings on record), signed release tags, and
  the operator's sole acceptance of decisions. `scripts/seal-branch.sh` and
  its proof stay available; sealing is optional.

A real trust boundary (required signatures, a bot as sole merger, and agents
holding a token that cannot merge) remains possible later as a separate
ruling.
