#!/usr/bin/env bash
# seal-branch.sh — re-sign every commit of a PR branch in place, after the
# work is done (decision 0056). An operator tool: nothing in CI or any seat
# runs its signing path; seal-branch-test.sh proves only its keyless arms.
#
# Seats and the operator's assistant commit unsigned; the operator's
# signature is added once, at the end, when the forwarded gpg agent is
# reachable. Each commit in merge-base..HEAD is re-created, parent first,
# from its own tree, exact message bytes, author record and ordered parents
# (mapped to their sealed copies), and signed. No rebase, no patch replay,
# no merge recomputation, and the merge-base never moves, so content-keyed
# gates (brokkr's per-file patch map from the merge-base) still vouch for
# the sealed head. Committer identity and date change, and so do the IDs.
#
# The branch ref moves only after every sealed commit has been compared
# field by field with its original and reads %G? = G, and then only by a
# compare-and-swap against the captured old head: a failed check leaves the
# branch, the index and the worktree exactly as they were. The tool never
# fetches — fetch first, and the base is frozen at the OID it resolves to.
#
# Usage: seal-branch.sh [-C repo] [--base ref] [--key keyid] [--push] [--check]
#   -C       the repository to seal (default: .)
#   --base   the upstream to seal from (default: origin/HEAD, else origin/main)
#   --key    signing key (default: git config user.signingkey)
#   --push   force-push the sealed branch to origin with a lease on the old head
#   --check  report whether the branch needs sealing and whether signing
#            works; change nothing (never pushes)
# Exit: 0 sealed (or nothing to seal, or a --check report), 3 signing
# unavailable, 1 any other refusal.
set -euo pipefail

die() { printf 'seal-branch: %s\n' "$*" >&2; exit 1; }
usage() { sed -n '/^# Usage:/,/^set -euo/p' "${BASH_SOURCE[0]}" | sed -e '$d' -e 's/^# \{0,1\}//'; }

repo=. base='' key='' push=0 check=0
while [ $# -gt 0 ]; do
  case "$1" in
    -C | --base | --key)
      [ $# -ge 2 ] && [ -n "$2" ] || die "$1 needs a value"
      case "$1" in -C) repo="$2" ;; --base) base="$2" ;; --key) key="$2" ;; esac
      shift 2 ;;
    --push) push=1; shift ;;
    --check) check=1; shift ;;
    -h | --help) usage; exit 0 ;;
    *) die "unknown argument $1 (see --help)" ;;
  esac
done

command -v git >/dev/null 2>&1 || die "git is not installed"
# The public shim runs from the repository root; SEAL_BRANCH_FROM is where
# its caller stood, so -C and the default '.' mean what the operator typed.
cd "${SEAL_BRANCH_FROM:-.}" && cd "$repo" 2>/dev/null || die "cannot enter $repo"
git rev-parse --git-dir >/dev/null 2>&1 || die "$repo is not a git repository"
# Read the object graph as written: no replace refs, no grafts, no shallow
# history, and no optional index refresh (--check changes nothing).
export GIT_NO_REPLACE_OBJECTS=1 GIT_OPTIONAL_LOCKS=0
[ "$(git rev-parse --is-shallow-repository)" = false ] || die "the repository is shallow; fetch the full history first"
[ ! -e "$(git rev-parse --git-path info/grafts)" ] || die "the repository has grafts; seal a graft-free clone"
for state in MERGE_HEAD CHERRY_PICK_HEAD REVERT_HEAD BISECT_LOG rebase-merge rebase-apply sequencer; do
  [ ! -e "$(git rev-parse --git-path "$state")" ] || die "an unfinished git operation is in progress ($state); finish or abort it first"
done

branch="$(git symbolic-ref -q --short HEAD)" || die "HEAD is detached; check out the PR branch"
old="$(git rev-parse --verify -q "refs/heads/$branch^{commit}")" || die "$branch has no commits"
git diff --quiet && git diff --cached --quiet || die "the tree has uncommitted changes"

if [ -z "$base" ]; then
  base="$(git symbolic-ref -q --short refs/remotes/origin/HEAD || echo origin/main)"
fi
base_oid="$(git rev-parse --verify -q "$base^{commit}")" || die "base $base does not name a commit (fetch first?)"
base_ref="$(git rev-parse --symbolic-full-name "$base" 2>/dev/null || true)"
case "$base_ref" in
  "refs/heads/$branch" | "refs/remotes/"*"/$branch") die "$branch is the base branch itself; seal a PR branch, never $base" ;;
esac
mbs="$(git merge-base --all "$old" "$base_oid")" || die "no merge-base between $branch and $base"
[ "$(printf '%s\n' "$mbs" | wc -l)" -eq 1 ] || die "$branch and $base have more than one merge-base; refusing to pick one"
mb="$mbs"
count="$(git rev-list --count "$mb..$old")"
if [ "$count" -eq 0 ]; then
  echo "seal-branch: $branch has no commits over $base: nothing to seal"
  exit 0
fi

gpg_program="$(git config gpg.program || echo gpg)"
[ "$(git config gpg.format || echo openpgp)" = openpgp ] || die "gpg.format is not openpgp; only gpg signatures are sealed"
command -v "$gpg_program" >/dev/null 2>&1 || die "$gpg_program is not installed"
command -v timeout >/dev/null 2>&1 || die "timeout is not installed"
key="${key:-$(git config user.signingkey || true)}"
unsigned="$(git log --format=%G? "$mb..$old" | grep -cvx G || true)"
# A bounded probe that never starts an agent: a missing key, a locked key or
# an unreachable forwarded agent all read as unavailable.
signing=unavailable
if [ -n "$key" ] && printf probe | timeout 30 env -u GPG_TTY "$gpg_program" --no-autostart --batch --no-tty \
  --clearsign -u "$key" >/dev/null 2>&1; then
  signing=available
fi

if [ "$check" = 1 ]; then
  echo "branch=$branch base=$base merge-base=${mb:0:10} commits=$count unsigned=$unsigned signing=$signing"
  echo "seal-branch: sealing keeps every tree, message, author and parent; committer identity and date may change"
  exit 0
fi

new="$old"
if [ "$unsigned" -eq 0 ]; then
  echo "seal-branch: all $count commits on $branch are already signed"
else
  [ "$signing" = available ] || {
    echo "seal-branch: signing unavailable for '${key:-<no key: pass --key or set user.signingkey>}' (forwarded agent unreachable, key missing or locked); $branch is untouched" >&2
    exit 3
  }
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  trap 'exit 130' INT TERM
  mkdir "$tmp/map"
  mapped() { if [ -f "$tmp/map/$1" ]; then cat "$tmp/map/$1"; else printf '%s' "$1"; fi; }
  # The header fields a sealed commit must keep — every header but the
  # committer and the old signature — with parents mapped to sealed copies.
  kept() {
    local line skip=0
    while IFS= read -r line; do
      [ -n "$line" ] || break
      case "$line" in
        ' '*) [ "$skip" = 1 ] || printf '%s\n' "$line" ;;
        'committer '* | 'gpgsig '* | 'gpgsig-sha256 '*) skip=1 ;;
        'parent '*) skip=0; printf 'parent %s\n' "$(mapped "${line#parent }")" ;;
        *) skip=0; printf '%s\n' "$line" ;;
      esac
    done <"$1"
  }
  for c in $(git rev-list --reverse --topo-order "$mb..$old"); do
    git cat-file commit "$c" >"$tmp/$c.raw"
    tree='' author='' encoding='' parents=() last=''
    while IFS= read -r line; do
      [ -n "$line" ] || break
      case "$line" in
        'tree '*) tree="${line#tree }" ;;
        'parent '*) parents+=(-p "$(mapped "${line#parent }")") ;;
        'author '*) author="${line#author }" ;;
        'encoding '*) encoding="${line#encoding }" ;;
        'committer '* | 'gpgsig '* | 'gpgsig-sha256 '*) ;;
        ' '*) case "$last" in gpgsig | gpgsig-sha256) ;; *) die "commit ${c:0:10} has a header seal-branch cannot keep (${last:-continuation}); $branch is untouched" ;; esac; continue ;;
        *) die "commit ${c:0:10} has a header seal-branch cannot keep (${line%% *}); $branch is untouched" ;;
      esac
      last="${line%% *}"
    done <"$tmp/$c.raw"
    # The message is every byte after the first empty line, kept verbatim.
    tail -c +"$(($(sed '/^$/q' "$tmp/$c.raw" | wc -c) + 1))" "$tmp/$c.raw" >"$tmp/$c.msg"
    # author: "Name <email> seconds zone"
    zone="${author##* }"; rest="${author% *}"; seconds="${rest##* }"; ident="${rest% *}"
    name="${ident%% <*}"; email="${ident#* <}"; email="${email%>}"
    enc=(); [ -z "$encoding" ] || enc=(-c "i18n.commitEncoding=$encoding")
    # ${a[@]+...}: an empty array under set -u, on bash 3.2 (macOS) too.
    sealed="$(GIT_AUTHOR_NAME="$name" GIT_AUTHOR_EMAIL="$email" GIT_AUTHOR_DATE="@$seconds $zone" \
      timeout 120 env -u GPG_TTY git ${enc[@]+"${enc[@]}"} commit-tree --gpg-sign="$key" \
      ${parents[@]+"${parents[@]}"} "$tree" <"$tmp/$c.msg")" ||
      die "signing ${c:0:10} failed; $branch is untouched"
    git cat-file commit "$sealed" >"$tmp/$sealed.raw"
    kept "$tmp/$c.raw" >"$tmp/$c.kept"
    kept "$tmp/$sealed.raw" >"$tmp/$sealed.kept"
    tail -c +"$(($(sed '/^$/q' "$tmp/$sealed.raw" | wc -c) + 1))" "$tmp/$sealed.raw" >"$tmp/$sealed.msg"
    cmp -s "$tmp/$c.kept" "$tmp/$sealed.kept" ||
      die "the sealed copy of ${c:0:10} changed its tree, author, encoding or parents; $branch is untouched"
    cmp -s "$tmp/$c.msg" "$tmp/$sealed.msg" || die "the sealed copy of ${c:0:10} changed its message; $branch is untouched"
    printf '%s' "$sealed" >"$tmp/map/$c"
  done
  new="$(mapped "$old")"
  # The whole range, checked again from the sealed head before the ref moves.
  [ "$(git rev-list --count "$mb..$new")" -eq "$count" ] || die "the sealed range lost or gained commits; $branch is untouched"
  [ "$(git merge-base --all "$new" "$base_oid")" = "$mb" ] || die "the merge-base moved; $branch is untouched"
  [ "$(git rev-parse "$new^{tree}")" = "$(git rev-parse "$old^{tree}")" ] || die "the head tree changed; $branch is untouched"
  left="$(git log --format='%G? %h' "$mb..$new" | grep -v '^G ' || true)"
  [ -z "$left" ] || die "sealed commits do not read signed good: $left; $branch is untouched"
  [ "$(git symbolic-ref -q --short HEAD)" = "$branch" ] || die "HEAD moved off $branch while sealing; $branch is untouched"
  git diff --quiet && git diff --cached --quiet || die "the tree changed while sealing; $branch is untouched"
  git update-ref -m "seal-branch: re-sign $count commits" "refs/heads/$branch" "$new" "$old" ||
    die "$branch moved while sealing; it is untouched by this run"
  echo "seal-branch: sealed $count commits on $branch: ${old:0:10} -> ${new:0:10} (trees, messages, authors and parents unchanged; merge-base ${mb:0:10})"
fi

if [ "$push" = 1 ]; then
  git push -q --force-with-lease="refs/heads/$branch:$old" origin "refs/heads/$branch:refs/heads/$branch" ||
    die "push refused (the remote branch moved, or the lease failed); the local seal at ${new:0:10} is kept for inspection"
  echo "seal-branch: pushed $branch"
fi
