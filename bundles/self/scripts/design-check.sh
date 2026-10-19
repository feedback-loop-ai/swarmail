#!/usr/bin/env bash
# design-check.sh — the deterministic validation that ends the design
# sequence (positions -> chief -> check; decision 0050 §3). No network, no
# model: structural checks over the design the chief's HEAD commit carries.
# This file runs from the pinned bundle (/runtime/bundle/scripts in the exec
# box); its prerequisite helpers are resolved beside it, never from the
# candidate workspace, which is only the material assessed.
#
#   design-check.sh <prompt_file>   the exec step: writes {"result":
#                                   "designed"|"fail", "inputs": {}, "notes"}
#                                   to the prompt's result path
#   design-check.sh check [dir]     the same verdict on stdout (exit 0 only
#                                   for "designed") — the selftest's entry
#   design-check.sh selftest        every refusal arm against Git fixtures
#
# Identity is HEAD, never mtime or branch history: exactly one specs/<slug>/
# directory must be touched by the HEAD commit, its spec/plan/tasks must be
# committed there, nonempty, structurally complete and identical to a clean
# worktree, every explicitly declared `Decision: NNNN` must be one tracked,
# clean, proposed record the spec links to, and the pinned speckit
# prerequisite check must pass for that directory.
set -u

pin_dir="$(CDPATH='' cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

findings=""
add_finding() {
  if [ -z "$findings" ]; then findings="$1"; else findings="$findings; $1"; fi
}

# validate <workspace>: sets verdict_result/verdict_notes.
validate() {
  local ws="$1" top slug="" path count=0 f blob dec decl_files dec_file status
  findings=""
  verdict_result=fail
  if ! top="$(git -C "$ws" rev-parse --show-toplevel 2>/dev/null)"; then
    add_finding "$ws is not a Git work tree (git rev-parse failed); the design is identified by HEAD"
    verdict_notes="$findings"; return
  fi
  if ! git -C "$top" rev-parse --verify -q HEAD >/dev/null; then
    add_finding "HEAD does not resolve to a commit"
    verdict_notes="$findings"; return
  fi
  # The slugs HEAD's own commit touches, NUL-delimited so no path can split.
  local slugs=() seen=" " s
  local changed
  if ! changed="$(git -C "$top" diff-tree --no-commit-id --root -r -z --name-only HEAD 2>/dev/null | tr '\0' '\n')"; then
    add_finding "git diff-tree HEAD failed"
    verdict_notes="$findings"; return
  fi
  while IFS= read -r path; do
    case "$path" in
      specs/*/*)
        s="${path#specs/}"; s="${s%%/*}"
        case "$seen" in *" $s "*) ;; *) slugs+=("$s"); seen="$seen$s " ;; esac
        ;;
    esac
  done <<<"$changed"
  count="${#slugs[@]}"
  if [ "$count" -eq 0 ]; then
    add_finding "the HEAD commit touches no specs/<slug>/ directory — the chief's design commit must be HEAD"
    verdict_notes="$findings"; return
  fi
  if [ "$count" -gt 1 ]; then
    add_finding "the HEAD commit touches ${count} spec directories (${slugs[*]}); exactly one design is validated"
    verdict_notes="$findings"; return
  fi
  slug="${slugs[0]}"
  if ! [[ "$slug" =~ ^[a-z0-9]+(-[a-z0-9]+)*$ ]]; then
    add_finding "spec directory '$slug' is not a kebab-case slug"
    verdict_notes="$findings"; return
  fi
  local dir="specs/$slug"
  for f in spec.md plan.md tasks.md; do
    if ! git -C "$top" cat-file -e "HEAD:$dir/$f" 2>/dev/null; then
      add_finding "$dir/$f is not committed at HEAD"
    elif [ "$(git -C "$top" cat-file -s "HEAD:$dir/$f")" -eq 0 ]; then
      add_finding "$dir/$f is empty at HEAD"
    fi
  done
  status="$(git -C "$top" status --porcelain --untracked-files=all -- "$dir" 2>&1)" || add_finding "git status failed for $dir"
  [ -z "$status" ] || add_finding "$dir differs from HEAD in the worktree (dirty or untracked: $(printf '%s' "$status" | tr '\n' ' '))"
  blob() { git -C "$top" show "HEAD:$dir/$1" 2>/dev/null; }
  if git -C "$top" cat-file -e "HEAD:$dir/spec.md" 2>/dev/null; then
    blob spec.md | grep -Eiq '^#{1,6}[[:space:]].*acceptance' ||
      add_finding "$dir/spec.md has no acceptance section heading"
  fi
  if git -C "$top" cat-file -e "HEAD:$dir/tasks.md" 2>/dev/null; then
    blob tasks.md | grep -Eq '^[[:space:]]*[-*][[:space:]]\[([ xX])\]' ||
      add_finding "$dir/tasks.md has no checkbox task item"
  fi
  if git -C "$top" cat-file -e "HEAD:$dir/plan.md" 2>/dev/null; then
    blob plan.md | grep -Eiq '^#{1,6}[[:space:]].*risk' ||
      add_finding "$dir/plan.md has no risks section heading"
  fi
  # Explicit declarations only: a line opening `Decision: NNNN`. A link to an
  # accepted authority elsewhere in the prose is a reference, not a declaration.
  if git -C "$top" cat-file -e "HEAD:$dir/spec.md" 2>/dev/null; then
    for dec in $(blob spec.md | grep -Eio '^[[:space:]]*\**decision\**[[:space:]]*:[[:space:]]*[0-9]{4}' | grep -Eo '[0-9]{4}' | sort -u); do
      decl_files="$(git -C "$top" ls-tree --name-only HEAD -- docs/decisions/ | grep -E "^docs/decisions/$dec-[^/]+\.md$")"
      if [ -z "$decl_files" ]; then
        add_finding "spec declares decision $dec but no docs/decisions/$dec-*.md is committed at HEAD"
        continue
      fi
      if [ "$(printf '%s\n' "$decl_files" | wc -l)" -ne 1 ]; then
        add_finding "decision $dec is ambiguous: $(printf '%s' "$decl_files" | tr '\n' ' ')"
        continue
      fi
      dec_file="$decl_files"
      [ -z "$(git -C "$top" status --porcelain -- "$dec_file" 2>&1)" ] ||
        add_finding "declared decision $dec_file differs from HEAD in the worktree"
      git -C "$top" show "HEAD:$dec_file" | grep -Eiq '^-[[:space:]]*\*\*Status:\*\*[[:space:]]*proposed' ||
        add_finding "declared decision $dec_file is not status proposed (a new declaration is proposed; cite an accepted one as authority instead)"
      blob spec.md | grep -Fq "docs/decisions/$(basename "$dec_file")" ||
        add_finding "spec declares decision $dec but does not link $(basename "$dec_file")"
    done
  fi
  # The pinned dialect prerequisites, for exactly this directory.
  local prereq="$pin_dir/speckit/check-prerequisites.sh" out
  if [ ! -f "$prereq" ] || [ ! -f "$pin_dir/speckit/common.sh" ]; then
    add_finding "the pinned speckit prerequisite helper is missing ($prereq)"
  elif ! out="$(cd "$top" && SPECIFY_FEATURE_DIR="$dir" bash "$prereq" --json --require-tasks 2>&1)"; then
    add_finding "the pinned speckit prerequisite check failed: $(printf '%s' "$out" | tr '\n' ' ')"
  fi
  if [ -z "$findings" ]; then
    verdict_result=designed
    verdict_notes="design '$slug' at $(git -C "$top" rev-parse --short HEAD) passes every check"
  else
    verdict_notes="$findings"
  fi
}

write_result() { # $1 result, $2 notes, $3 path — valid JSON, atomically renamed.
  local tmp="$3.tmp.$$"
  if command -v jq >/dev/null 2>&1 &&
    jq -n --arg r "$1" --arg n "$2" '{result: $r, inputs: {}, notes: $n}' >"$tmp" 2>/dev/null; then
    mv -f "$tmp" "$3"
  else
    rm -f "$tmp"
    printf '%s\n' '{"result": "fail", "inputs": {}, "notes": "design-check: jq could not render the verdict; refusing"}' >"$tmp" && mv -f "$tmp" "$3"
  fi
}

selftest() {
  local root failures=0 out
  root="$(mktemp -d "${TMPDIR:-/tmp}/design-check-selftest.XXXXXX")" || exit 1
  selftest_root="$root"
  trap 'rm -rf "$selftest_root"' EXIT
  g() { git -C "$repo" -c user.name=t -c user.email=t@t -c commit.gpgsign=false "$@" >/dev/null 2>&1; }
  fresh() { # a repo whose HEAD commits one valid design, plus an older valid one
    repo="$root/$1"; mkdir -p "$repo"; git init -q "$repo"
    mkdir -p "$repo/specs/old-valid" "$repo/specs/new-design" "$repo/docs/decisions"
    printf '# Old\n## Acceptance\n' >"$repo/specs/old-valid/spec.md"
    printf '# Plan\n## Risks\n' >"$repo/specs/old-valid/plan.md"
    printf -- '- [ ] T1\n' >"$repo/specs/old-valid/tasks.md"
    g add -A; g commit -m old
    printf '# New\nDecision: 0099 — [x](../../docs/decisions/0099-x.md)\n## Acceptance Criteria\n' >"$repo/specs/new-design/spec.md"
    printf '# Plan\n## Risks\n' >"$repo/specs/new-design/plan.md"
    printf -- '- [ ] T1\n' >"$repo/specs/new-design/tasks.md"
    printf '# 0099\n\n- **Status:** proposed\n' >"$repo/docs/decisions/0099-x.md"
    g add -A; g commit -m design
  }
  expect() { # $1 label, $2 designed|fail, $3 needle (for fail)
    local status=0
    out="$(bash "$pin_dir/design-check.sh" check "$repo" 2>&1)" || status=$?
    if [ "$2" = designed ] && [ "$status" -ne 0 ]; then
      printf 'design-check selftest failure — %s: expected designed, got:\n%s\n' "$1" "$out" >&2; failures=$((failures + 1))
    elif [ "$2" = fail ] && { [ "$status" -eq 0 ] || [[ "$out" != *"$3"* ]]; }; then
      printf 'design-check selftest failure — %s: expected a refusal naming "%s", got (exit %s):\n%s\n' "$1" "$3" "$status" "$out" >&2; failures=$((failures + 1))
    fi
  }
  fresh ok; expect 'a valid committed design passes' designed
  git -C "$repo" worktree add -q "$root/linked" HEAD >/dev/null 2>&1
  repo="$root/linked"; expect 'a linked worktree of the valid design passes' designed
  fresh risks; printf '# Plan\n' >"$repo/specs/new-design/plan.md"; g commit -am 'no risks'
  touch -d '2030-01-01' "$repo/specs/old-valid" "$repo/specs/old-valid/plan.md"
  expect 'a missing-risks draft refuses beside a newer-mtime valid spec' fail 'no risks section'
  fresh accept; printf '# New\n' >"$repo/specs/new-design/spec.md"; g commit -am x
  expect 'a spec without acceptance refuses' fail 'no acceptance section'
  fresh tasks; printf 'T1\n' >"$repo/specs/new-design/tasks.md"; g commit -am x
  expect 'tasks without a checkbox refuse' fail 'no checkbox task'
  fresh empty; : >"$repo/specs/new-design/tasks.md"; g commit -am x
  expect 'an empty artifact refuses' fail 'tasks.md is empty'
  fresh missing; g rm -q specs/new-design/plan.md; g commit -m x
  expect 'a missing artifact refuses' fail 'plan.md is not committed'
  fresh two; printf -- '- [x] T2\n' >>"$repo/specs/old-valid/tasks.md"; printf -- '- [x] T2\n' >>"$repo/specs/new-design/tasks.md"; g commit -am x
  expect 'two touched spec dirs refuse' fail 'touches 2 spec directories'
  fresh unrelated; printf 'x\n' >"$repo/README"; g add README; g commit -m unrelated
  expect 'an unrelated HEAD commit refuses' fail 'touches no specs'
  fresh dirty; printf 'more\n' >>"$repo/specs/new-design/plan.md"
  expect 'a dirty artifact refuses' fail 'differs from HEAD'
  fresh untracked; printf 'x\n' >"$repo/specs/new-design/notes.md"
  expect 'an untracked artifact refuses' fail 'differs from HEAD'
  fresh decdirty; printf 'edit\n' >>"$repo/docs/decisions/0099-x.md"
  expect 'a dirty declared decision refuses' fail 'differs from HEAD in the worktree'
  fresh decmissing; g rm -q docs/decisions/0099-x.md; printf -- '- [x] T9\n' >>"$repo/specs/new-design/tasks.md"; g add -A; g commit -m x
  expect 'a missing declared decision refuses' fail 'no docs/decisions/0099-*.md'
  fresh decaccepted; printf '# 0099\n\n- **Status:** accepted\n' >"$repo/docs/decisions/0099-x.md"; printf -- '- [x] T9\n' >>"$repo/specs/new-design/tasks.md"; g commit -am x
  expect 'an accepted record cannot be a new declaration' fail 'not status proposed'
  fresh declink; printf '# New\nDecision: 0099\n## Acceptance\n' >"$repo/specs/new-design/spec.md"; g commit -am x
  expect 'a declaration without its link refuses' fail 'does not link 0099-x.md'
  fresh authority; printf '# New\nSee [0007](../../docs/decisions/0007-y.md), accepted.\n## Acceptance\n' >"$repo/specs/new-design/spec.md"; g commit -am x
  expect 'an authority link is not a declaration' designed
  fresh slug; mkdir -p "$repo/specs/Bad_Slug"; cp "$repo/specs/new-design/"* "$repo/specs/Bad_Slug/"; g add -A; g commit -m x
  expect 'a non-kebab slug refuses' fail 'not a kebab-case slug'
  repo="$root/nogit"; mkdir -p "$repo"
  expect 'a non-Git directory refuses' fail 'not a Git work tree'
  # The prerequisite helper is part of the pinned chain: a copy of this
  # validator without it refuses, and one whose helper fails refuses.
  local pinless="$root/pinless"; mkdir -p "$pinless"; cp "$pin_dir/design-check.sh" "$pinless/"
  fresh prereq; out="$(bash "$pinless/design-check.sh" check "$repo" 2>&1)" &&
    { printf 'design-check selftest failure — a missing pinned helper passed:\n%s\n' "$out" >&2; failures=$((failures + 1)); }
  [[ "$out" == *"prerequisite helper is missing"* ]] ||
    { printf 'design-check selftest failure — the missing-helper refusal is unnamed:\n%s\n' "$out" >&2; failures=$((failures + 1)); }
  mkdir -p "$pinless/speckit"; printf 'exit 1\n' >"$pinless/speckit/check-prerequisites.sh"; : >"$pinless/speckit/common.sh"
  out="$(bash "$pinless/design-check.sh" check "$repo" 2>&1)" &&
    { printf 'design-check selftest failure — a failing pinned helper passed:\n%s\n' "$out" >&2; failures=$((failures + 1)); }
  # The exec entry: a real prompt file, a valid JSON result with exactly
  # result/inputs/notes, renamed into place.
  fresh exec; printf 'Result contract\n\n    %s/result.json\n' "$root" >"$root/prompt.md"
  (cd "$repo" && bash "$pin_dir/design-check.sh" "$root/prompt.md") >/dev/null 2>&1
  out="$(jq -c 'keys' "$root/result.json" 2>&1) $(jq -r .result "$root/result.json" 2>&1)"
  [ "$out" = '["inputs","notes","result"] designed' ] ||
    { printf 'design-check selftest failure — the exec result is %s\n' "$out" >&2; failures=$((failures + 1)); }
  if [ "$failures" -ne 0 ]; then
    printf 'design-check: selftest failed with %s arm(s)\n' "$failures" >&2
    exit 1
  fi
  printf 'design-check: selftest pass — every refusal arm refuses, the valid design passes\n'
}

case "${1:-}" in
  selftest) selftest; exit 0 ;;
  check)
    validate "${2:-.}"
    printf '%s: %s\n' "$verdict_result" "$verdict_notes"
    [ "$verdict_result" = designed ]; exit
    ;;
esac

prompt_file="${1:-}"
if [ -z "$prompt_file" ] || [ ! -f "$prompt_file" ]; then
  echo "design-check: prompt file argument missing or unreadable" >&2
  exit 2
fi
# The result contract renders the path as an indented line of its own: an
# absolute path ending in .json. Take the last match.
result_path=""
while IFS= read -r line; do
  trimmed="${line#"${line%%[![:space:]]*}"}"
  trimmed="${trimmed%"${trimmed##*[![:space:]]}"}"
  case "$trimmed" in /*.json) result_path="$trimmed" ;; esac
done <"$prompt_file"
if [ -z "$result_path" ]; then
  echo "design-check: no result_path line found in the prompt file" >&2
  exit 2
fi
mkdir -p "$(dirname "$result_path")" 2>/dev/null || true
validate .
write_result "$verdict_result" "$verdict_notes" "$result_path"
exit 0
