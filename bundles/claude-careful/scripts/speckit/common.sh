#!/usr/bin/env bash
# Minimal speckit support for check-prerequisites.sh, pinned beside it in the
# bundle (decision 0050 §1). The feature directory is never rediscovered from
# a branch name or an mtime: the caller names it in SPECIFY_FEATURE_DIR
# (design-check.sh passes the one directory the chief's HEAD commit touched),
# and an absent or unusable value refuses.

get_feature_paths() {
  local repo_root branch has_git=false feature_dir
  if repo_root="$(git rev-parse --show-toplevel 2>/dev/null)"; then
    has_git=true
    branch="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo HEAD)"
  else
    repo_root="$PWD"
    branch=""
  fi
  if [ -z "${SPECIFY_FEATURE_DIR:-}" ]; then
    echo "ERROR: SPECIFY_FEATURE_DIR is not set; the feature directory is named by the caller, never guessed" >&2
    echo "exit 1"
    return 0
  fi
  case "$SPECIFY_FEATURE_DIR" in
    /*) feature_dir="$SPECIFY_FEATURE_DIR" ;;
    *) feature_dir="$repo_root/$SPECIFY_FEATURE_DIR" ;;
  esac
  printf 'REPO_ROOT=%q\n' "$repo_root"
  printf 'CURRENT_BRANCH=%q\n' "$branch"
  printf 'HAS_GIT=%q\n' "$has_git"
  printf 'FEATURE_DIR=%q\n' "$feature_dir"
  printf 'FEATURE_SPEC=%q\n' "$feature_dir/spec.md"
  printf 'IMPL_PLAN=%q\n' "$feature_dir/plan.md"
  printf 'TASKS=%q\n' "$feature_dir/tasks.md"
  printf 'RESEARCH=%q\n' "$feature_dir/research.md"
  printf 'DATA_MODEL=%q\n' "$feature_dir/data-model.md"
  printf 'QUICKSTART=%q\n' "$feature_dir/quickstart.md"
  printf 'CONTRACTS_DIR=%q\n' "$feature_dir/contracts"
}

# The branch is not the design's identity (HEAD's commit is), so any branch
# passes; the function survives for the upstream script's call shape.
check_feature_branch() { return 0; }

check_file() { if [ -f "$1" ]; then echo "  ✓ $2"; else echo "  ✗ $2"; fi; }
check_dir() { if [ -d "$1" ] && [ -n "$(ls -A "$1" 2>/dev/null)" ]; then echo "  ✓ $2"; else echo "  ✗ $2"; fi; }
