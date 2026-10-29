#!/usr/bin/env bash
# seal-branch-test.sh — the keyless proof of seal-branch.sh (decision 0056),
# run by CI's quality job and the verify seat. It drives the pinned tool
# beside this file against real scratch repositories with real git and real
# gpg: unsigned commits and tags, an isolated HOME and git config, a private
# empty GNUPGHOME and no inherited agent sockets, so no host agent and no key
# is ever reached. It proves:
#
#   --check   the exact report on a graph with a side branch and a merge,
#             with nothing changed — not even with --push beside it, which
#             leaves a local bare origin's branch where it was;
#   exit 3    sealing with no key set, and with a configured or passed key
#             gpg cannot reach, with the branch, index and worktree untouched;
#   exit 1    a dirty worktree or index (its bytes preserved), a detached
#             HEAD, the base branch itself, missing operands, an unknown
#             argument, an invalid base, unrelated histories and a missing gpg;
#   exit 0    "nothing to seal" for a branch already at its base, with no key
#             and no gpg needed.
#
# Then every guard and report is broken in a disposable copy of the tool and
# the matching case must fail: a proof that cannot fail is not a proof.
# Successful signing is the operator's evidence, never this proof's claim.
set -euo pipefail
pin_dir="$(CDPATH='' cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tool="$pin_dir/seal-branch.sh"
[ -f "$tool" ] || { echo "seal-branch-test: pinned tool $tool is missing" >&2; exit 1; }
for prerequisite in git gpg timeout; do
  command -v "$prerequisite" >/dev/null 2>&1 || { echo "seal-branch-test: $prerequisite is not installed; refusing rather than skipping" >&2; exit 1; }
done
work="$(mktemp -d "${TMPDIR:-/tmp}/seal-branch-test.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# Isolation: nothing the host configured reaches git or gpg.
while IFS= read -r variable; do
  case "$variable" in GIT_* | GPG_* | SSH_AUTH_SOCK | SSH_AGENT_PID) unset "$variable" ;; esac
done < <(compgen -e)
export HOME="$work/home" XDG_CONFIG_HOME="$work/home/.config" GNUPGHOME="$work/gnupg"
export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_COUNT=0 GIT_TERMINAL_PROMPT=0
mkdir -p "$HOME" "$XDG_CONFIG_HOME"
mkdir -m 700 "$GNUPGHOME"
# Even the gpg git itself would call never starts an agent or keyboxd here.
printf 'no-autostart\n' >"$GNUPGHOME/gpg.conf"

failures=0
fail() { [ "${quiet:-0}" = 1 ] || printf 'seal-branch-test: FAIL — %s\n' "$1" >&2; failures=$((failures + 1)); }
# seal <repo> <args...>: runs the tool; leaves $status, $work/out, $work/err.
seal() {
  local dir="$1"; shift
  status=0
  bash "$tool" -C "$dir" "$@" >"$work/out" 2>"$work/err" || status=$?
}
expect() { # <status> <stream> <fixed text> <case>
  [ "$status" = "$1" ] || { fail "$4: exit $status, expected $1 ($(cat "$work/err"))"; return 0; }
  grep -qF -- "$3" "$work/$2" || fail "$4: $2 lacks '$3' (got: $(cat "$work/$2"))"
}
# Everything sealing could disturb: HEAD, every ref, the index bytes, the
# worktree bytes (tracked and untracked) and the bare origin's refs.
snapshot() {
  {
    git -C "$1" symbolic-ref -q HEAD || git -C "$1" rev-parse HEAD
    git -C "$1" for-each-ref --format='%(refname) %(objectname)'
    sha256sum "$1/.git/index"
    (cd "$1" && find . -path ./.git -prune -o -type f -print | LC_ALL=C sort | xargs sha256sum)
    git -C "$work/origin.git" for-each-ref --format='origin %(refname) %(objectname)'
  } >"$2"
}
unchanged() { # <repo> <case>
  snapshot "$1" "$work/after"
  cmp -s "$work/before" "$work/after" || fail "$2: the repository changed: $(diff "$work/before" "$work/after" | head -n 5)"
}
commit() { # <repo> <file> <text>
  printf '%s\n' "$3" >>"$1/$2"
  git -C "$1" add "$2"
  git -C "$1" commit -q -m "Add $3"
}

# The fixture: main with two commits on a local bare origin; feature with a
# side branch merged back (four commits over the base, one a merge); feature
# pushed to origin before its last commit; empty at the base.
git init -q --bare -b main "$work/origin.git"
repo="$work/repo"
git init -q -b main "$repo"
git -C "$repo" config user.name 'Seal Test'
git -C "$repo" config user.email 'seal@test.invalid'
git -C "$repo" config commit.gpgsign false
git -C "$repo" config tag.gpgsign false
git -C "$repo" config core.hooksPath /dev/null
commit "$repo" base.txt one
commit "$repo" base.txt two
git -C "$repo" remote add origin "$work/origin.git"
git -C "$repo" push -q origin main
git -C "$repo" fetch -q origin
git -C "$repo" remote set-head origin main >/dev/null
git -C "$repo" checkout -q -b feature
commit "$repo" feature.txt alpha
git -C "$repo" push -q origin feature
git -C "$repo" checkout -q -b side
commit "$repo" side.txt beta
git -C "$repo" checkout -q feature
commit "$repo" feature.txt gamma
git -C "$repo" merge -q --no-ff -m 'Merge side' side
git -C "$repo" branch -q -D side
git -C "$repo" branch -q empty origin/main
merge_base="$(git -C "$repo" merge-base feature origin/main)"
printf 'untracked\n' >"$repo/notes.txt"

case_check() {
  snapshot "$repo" "$work/before"
  seal "$repo" --check
  expect 0 out "branch=feature base=origin/main merge-base=${merge_base:0:10} commits=4 unsigned=4 signing=unavailable" '--check'
  expect 0 out 'committer identity and date may change' '--check'
  unchanged "$repo" '--check'
  seal "$repo" --check --push --key 0000000000000000000000000000000000000000
  expect 0 out 'commits=4 unsigned=4 signing=unavailable' '--check --push'
  unchanged "$repo" '--check --push'
}
case_no_key() {
  snapshot "$repo" "$work/before"
  seal "$repo"
  expect 3 err 'signing unavailable' 'no key set'
  unchanged "$repo" 'no key set'
}
case_missing_key() {
  snapshot "$repo" "$work/before"
  git -C "$repo" config user.signingkey 0000000000000000000000000000000000000000
  seal "$repo" --push
  expect 3 err 'signing unavailable' 'configured key gpg cannot reach'
  git -C "$repo" config --unset user.signingkey
  seal "$repo" --key 'missing key <nobody@test.invalid>'
  expect 3 err 'signing unavailable' 'passed key gpg cannot reach'
  unchanged "$repo" 'unreachable key'
}
case_dirty() {
  printf 'edit\n' >>"$repo/base.txt"
  snapshot "$repo" "$work/before"
  seal "$repo" --key 0000000000000000000000000000000000000000
  expect 1 err 'uncommitted changes' 'dirty worktree'
  unchanged "$repo" 'dirty worktree'
  git -C "$repo" add base.txt
  snapshot "$repo" "$work/before"
  seal "$repo" --check
  expect 1 err 'uncommitted changes' 'dirty index'
  unchanged "$repo" 'dirty index'
  git -C "$repo" reset -q --hard
}
case_detached() {
  git -C "$repo" checkout -q --detach feature
  snapshot "$repo" "$work/before"
  seal "$repo" --check
  expect 1 err 'HEAD is detached' 'detached HEAD'
  unchanged "$repo" 'detached HEAD'
  git -C "$repo" checkout -q feature
}
case_empty() {
  git -C "$repo" checkout -q empty
  snapshot "$repo" "$work/before"
  seal "$repo"
  expect 0 out 'empty has no commits over origin/main: nothing to seal' 'branch at its base'
  seal "$repo" --check
  expect 0 out 'nothing to seal' 'branch at its base, --check'
  PATH="$work/no-gpg" seal "$repo"
  expect 0 out 'nothing to seal' 'branch at its base, no gpg'
  unchanged "$repo" 'branch at its base'
  git -C "$repo" checkout -q feature
}
case_refusals() {
  snapshot "$repo" "$work/before"
  seal "$repo" --base
  expect 1 err '--base needs a value' 'missing --base operand'
  seal "$repo" --key ''
  expect 1 err '--key needs a value' 'empty --key operand'
  status=0; bash "$tool" -C >"$work/out" 2>"$work/err" || status=$?
  expect 1 err '-C needs a value' 'missing -C operand'
  seal "$repo" --sign
  expect 1 err 'unknown argument --sign' 'unknown argument'
  seal "$repo" --base origin/nowhere --check
  expect 1 err 'base origin/nowhere does not name a commit' 'invalid base'
  seal "$repo" --base "$(git -C "$repo" commit-tree -m orphan "$(git -C "$repo" mktree </dev/null)")"
  expect 1 err 'no merge-base' 'unrelated base'
  PATH="$work/no-gpg" seal "$repo" --check
  expect 1 err 'gpg is not installed' 'missing gpg'
  seal "$repo" --help
  expect 0 out 'Usage: seal-branch.sh' '--help'
  unchanged "$repo" 'refusals'
  git -C "$repo" checkout -q main
  commit "$repo" base.txt unpushed
  snapshot "$repo" "$work/before"
  seal "$repo" --base main --check
  expect 1 err 'main is the base branch itself' 'the base branch'
  seal "$repo" --base origin/main --check
  expect 1 err 'main is the base branch itself' 'the base branch on its remote'
  unchanged "$repo" 'the base branch'
  git -C "$repo" reset -q --hard origin/main
  git -C "$repo" checkout -q feature
}

# A PATH with everything the tool needs except gpg.
mkdir "$work/no-gpg"
for utility in bash git sed tail wc grep cat cmp mkdir mktemp rm env timeout; do
  ln -s "$(command -v "$utility")" "$work/no-gpg/$utility"
done

cases=(case_check case_no_key case_missing_key case_dirty case_detached case_empty case_refusals)
for case in "${cases[@]}"; do "$case"; done
[ "$failures" -eq 0 ] || { echo "seal-branch-test: $failures assertion(s) failed" >&2; exit 1; }

# The proof must be able to fail: each mutant breaks one guard or report in
# a disposable copy of the tool, and the case that owns it must catch it.
# Each entry: <case>@@<exact text in the tool>@@<replacement>; the first
# occurrence is replaced.
mutants=(
  'case_check@@git rev-list --count "$mb..$old"@@git rev-list --first-parent --count "$mb..$old"'
  'case_check@@grep -cvx G@@grep -cx G'
  'case_check@@  exit 0
fi

new=@@fi

new='
  'case_no_key@@    exit 3@@    exit 1'
  'case_missing_key@@signing=unavailable
if@@signing=available
if'
  'case_dirty@@git diff --quiet && git diff --cached --quiet || die "the tree has uncommitted changes"@@true'
  'case_detached@@die "HEAD is detached; check out the PR branch"@@branch="$(git rev-parse --abbrev-ref HEAD)"'
  'case_empty@@  echo "seal-branch: $branch has no commits over $base: nothing to seal"@@  :'
  'case_refusals@@  "refs/heads/$branch" |@@  "refs/heads/never-a-branch" |'
  'case_refusals@@command -v "$gpg_program" >/dev/null 2>&1 || die@@true ||'
)
text="$(cat "$tool"; printf x)"; text="${text%x}"
mutant="$work/mutant.sh"
for spec in "${mutants[@]}"; do
  owner="${spec%%@@*}"; rest="${spec#*@@}"
  from="${rest%%@@*}"; to="${rest#*@@}"
  case "$text" in *"$from"*) ;; *) echo "seal-branch-test: mutant for $owner no longer matches the tool: $from" >&2; exit 1 ;; esac
  printf '%s' "${text/"$from"/"$to"}" >"$mutant"
  survived="$(
    tool="$mutant" failures=0 quiet=1
    "$owner" >/dev/null 2>&1 || true
    printf '%s' "$failures"
  )"
  git -C "$repo" checkout -q -f feature
  [ "$survived" -gt 0 ] || { echo "seal-branch-test: mutant survived — $owner did not catch: $from" >&2; failures=$((failures + 1)); }
done
[ "$failures" -eq 0 ] || { echo "seal-branch-test: $failures mutant(s) survived" >&2; exit 1; }
echo "seal-branch-test: ${#cases[@]} cases and ${#mutants[@]} mutants passed (keyless; signing is the operator's evidence)"
