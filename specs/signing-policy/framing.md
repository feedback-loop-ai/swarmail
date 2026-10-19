# Framing — Swarmail signing policy: commit unsigned, seal the branch before merge

Intake seat, run `swarmail-signing-policy-commit-u-bd5eed0f` (2026-10-20).
Feature evidence, verbatim: `.brokkr/state/signing-policy-feature.txt`.
Operator's proven tool: `.brokkr/state/seal-branch.reference.sh` (proven on a
scratch repo with a merge commit and on lanetally PR #230).

## Goal (one sentence)

Seats and the operator's assistant commit unsigned; the operator seals every
PR branch once, before merge, by re-signing its commits in place on the same
merge-base with `scripts/seal-branch.sh` — and that policy is recorded in the
house rules, the charters and an accepted decision, with a keyless test in
the normal gates proving the tool's refusal and check paths.

## Where this sits relative to the frozen guarantees

**None of 0001–0003.** No `src/` change is in scope: synchronous insert,
subscribe-before-check and oldest-only pruning are untouched, and the exact
100/100/100 coverage floor is unaffected (no new production lines, so the
denominator must not move). The adjacent accepted rulings the work touches:

- **0045 §9** (*Signing statements distinguish authorship and landing*) says
  branch author commits use the operator key and that a disclosed inability
  to sign "is an exception requiring the applicable operator instruction".
  This change **is** that operator instruction (ruled 2026-10-20, in chat);
  decision 0056 must say so explicitly and scope the supersession to mid-run
  commits — landing signatures (GitHub squash merges, signed release tags,
  0019) are unchanged.
- **0050 §1** — gate scripts live pinned in `bundles/self/scripts/`; the seal
  proof joins them the way `publish-guard-test.sh` did (repo shim + pinned
  copy + a `run` line in the pinned `verify-seat.sh` + the house-rules gate
  list). The seal **tool** itself stays an operator tool: nothing in CI, the
  bundle, or any seat executes its signing path.
- **0034** — the run's own result records stay as they are; this change does
  not touch the result contract.

## Acceptance criteria (each can go red)

**AC1 — `scripts/seal-branch.sh` exists with the reference tool's contract.**
Adapted from `.brokkr/state/seal-branch.reference.sh` to this repository's
shell style (`set -euo pipefail`, `die()` refusals on stderr, `seal-branch:`
prefix, as in `scripts/publish-guard*.sh`). Flags: `-C <repo>`, `--base
<ref>`, `--key <keyid>`, `--push` (force-with-lease on the old head),
`--check` (report, change nothing). Exits: `0` sealed or nothing to seal,
`3` signing unavailable, `1` any other refusal. On any failed check the
branch is restored to its pre-run head. Sealing amends every commit in place
on the unchanged merge-base (amend, not squash, never a rebase onto a newer
main) and verifies per commit that tree, message and author are unchanged
and every commit reports `%G?` = `G`. *Red check:* the AC2 cases below.

**AC2 — A keyless test runs in the normal gates.** `scripts/seal-branch-test.sh`
(thin entry) over a pinned `bundles/self/scripts/seal-branch-test.sh` (the
`publish-guard-test.sh` pattern: pinned algorithm, candidate's `scripts/`
never executed by the seat) with a `run` line in the pinned
`verify-seat.sh` and one line in the house-rules gate list. The test builds
a scratch repository with `commit.gpgsign=false`, `tag.gpgsign=false` and a
fresh empty `GNUPGHOME` (no host agent reachable — probe with
`--no-autostart`, as the reference does), and proves:

  1. `--check` prints its report line (`branch=… base=… merge-base=…
     commits=… unsigned=… signing=…`) and exits `0` changing nothing (HEAD
     sha identical before and after);
  2. sealing with no reachable key exits `3` and leaves the branch
     untouched (HEAD sha identical);
  3. a dirty tree refuses with exit `1`;
  4. a detached HEAD refuses with exit `1`;
  5. a branch already at the base exits `0` with "nothing to seal".

No network, no key, no installed packages — the verify seat's offline
conditions. *Red check:* `bash scripts/seal-branch-test.sh` exits `0` and
each numbered case fails if its guard is broken (mutate the tool, watch the
case go red). The signing path itself is proven by the operator's evidence,
never in CI.

**AC3 — `docs/house-rules.md` § "Commits and history" carries the new rule.**
Replace "Commits are GPG-signed with the operator's key (`commit.gpgsign` is
set repo-locally). … the signed author commits remain on the PR branch."
(lines 137–141) with: author commits are unsigned until sealed; the operator
seals every PR branch before merge with `scripts/seal-branch.sh`; seal
before updating a branch from main so GitHub's merge commit stays
GitHub-signed; never rebase when sealing. Keep on record the three kinds of
signature already there — author-commit GPG signatures, GitHub-signed squash
merges, signed release tags (0019) — adding **sealed branch authorship** as
the operator's signature added at the end, and keep the historical
exceptions as recorded. The imperative-style/body-that-says-why sentence
stays. *Red check:* grep the section for the new rule's presence and the old
standing sentence's absence.

**AC4 — No charter tells a seat to sign or to stop when signing fails.**
`agents/charters/implementer.md:25` ("Commits are signed …") becomes: commit
unsigned, never push, do not stop for signing. This is the only seat-facing
signing instruction in the repo (verified by intake: `grep -iE 'gpg|sign'`
over `agents/`, `AGENTS.md`, `CONTRIBUTING.md`, `bundles/self/` — AGENTS.md
only points at the house rules; `CONTRIBUTING.md:45` covers signed release
tags and stays; no charter JSON mentions signing). *Red check:* the grep
returns no seat instructed to sign or to halt on signing failure.

**AC5 — Decision 0056, status Accepted.** `docs/decisions/0056-*.md`, next
free number after 0055, recorded **Accepted** with the attribution
"(operator, 2026-10-20 — the operator ruled this change for swarmail in
chat)". It records: the ruling (nothing signs mid-run; this clone pins
`commit.gpgsign=false` repo-locally; six epic #120 runs stopped
IMPL-BLOCKED with finished work); the amend-in-place mechanism (re-sign
every commit of the PR branch on the same merge-base; amend not squash;
never rebase onto a newer main; check tree/message/author per commit;
leased force-push); why content-keyed gates still vouch for a re-signed
head (brokkr compares per-file patch ids from the merge-base — the
mechanism lives in the engine; the reference tool's header and the feature
text state it, record it as stated); and the alternatives weighed (squash —
collapses author commits under GitHub's signature; a server-held seat key —
refused because anything on the box could then sign as the operator). It
must cite 0045 §9 and scope the supersession per "Where this sits" above.
*Red check:* the file exists with `Accepted`, the operator attribution, and
those five elements.

**AC6 — Final tree and standing rules.** Branch `signing-policy` cut from
current `main` after `git fetch origin`, checked out, clean tree, commits
unsigned (that is the point of the change). The full house inventory is
green on it — `cargo fmt --all` then `--check`;
`cargo clippy --workspace --all-targets --locked -- -D warnings`;
`cargo test --workspace --locked`; `brokkr compile --bundle bundles/self`;
`bash scripts/coverage-gate.sh` and `… selftest`;
`bash scripts/publish-guard-test.sh`; `cargo deny check licenses` — plus
`bash scripts/seal-branch-test.sh`. No push, no PR, no merge, no issue
writes. *Red check:* `git status` clean on `signing-policy`; gate receipts.

## Verify gate (what verify must run)

The full house inventory on the final tree (AC6 list), and decisively for
this change: **`bash scripts/seal-branch-test.sh`** (the new proof, wired
into the pinned verify seat so it runs there too), `cargo test --workspace
--locked` (proves `src/` untouched), and `brokkr compile --bundle
bundles/self` (proves the constitution still compiles after the wiring
edit). `cargo fmt --all` (write) precedes `-- --check` per the standing
rules.

## Toolchain facts the next seat will hit

- **The harness git env is broken until repaired in-shell.** The session
  exports `GIT_CONFIG_COUNT=1` and `GIT_CONFIG_VALUE_0=false` but no
  `GIT_CONFIG_KEY_0`, so *every* git command dies with `missing config key
  GIT_CONFIG_KEY_0`. Precedent: `specs/total-shutdown/plan.md` ("drive git
  with `GIT_CONFIG_COUNT=0`") and
  `specs/fidelity-closures-prove-ship/plan.md` ("complete the env as
  `GIT_CONFIG_KEY_0=commit.gpgsign`"). Either fix per shell; never assume
  the repo is unreachable.
- `commit.gpgsign` resolves `false` (repo-local per the standing rule; the
  env overlay also pins false). Recent history is unsigned. Commit
  unsigned — do not flip any config the operator set.
- Tree at framing time: `main` at `1d45e57`, clean, tracking `origin/main`,
  no divergence.
- Intake cut no branch and wrote no tracked file beyond this framing; the
  first writing seat cuts `signing-policy` from origin/main.

## Open points for the design council

1. Wiring the seal proof into the pinned constitution mid-run: the
   publish-guard precedent did exactly this (a verify-seat.sh diff plus the
   pinned copy); confirm the shims list/`shims.sha256` treatment for a
   fifth entry and keep the diff minimal.
2. How the pinned test reaches the candidate's `scripts/seal-branch.sh`
   (the `delivery-proof-test.sh` fixture copies repo scripts into its green
   tree — same shape works).
3. Exact wording of the "three kinds of signature" record in AC3 and its
   interaction with the retained squash-merge sentence.
4. Whether `agents/README.md` or `bundles/self/` prose needs the one-line
   pointer to 0056 — keep it to what a grep proves necessary.
