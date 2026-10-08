#!/usr/bin/env bash
# Coverage floor gate — brokkr-style honesty, swarmail-sized.
#
# The floor may rise; it may never fall. Raising it is an ordinary PR.
# Lowering it is refused here, in CI, and by review. Attribute-based
# exclusions (#[coverage(off)]) are forbidden so production code cannot
# shrink the denominator.
#
# The measurement is exact: line-by-line counts from the lcov report
# (DA: records), never a display column or a rounded percentage. At a 100
# floor this is airtight by arithmetic — (lines-missed)/lines >= 1.0 over
# integers holds only when missed == 0.
#
# If a stale local run reports phantom misses, clear the merge pool first:
#   cargo llvm-cov clean --workspace
set -euo pipefail

FLOOR="${COVERAGE_FLOOR:-100}"

command -v cargo-llvm-cov >/dev/null 2>&1 || {
  printf '%s\n' 'coverage gate: cargo-llvm-cov not on PATH (cargo install cargo-llvm-cov)' >&2
  exit 1
}

out="$(mktemp "${TMPDIR:-/tmp}/swarmail-cov.XXXXXX")"
trap 'rm -f "$out"' EXIT

cargo llvm-cov --lcov --output-path "$out" >/dev/null 2>&1

lines="$(grep -c '^DA:' "$out" || true)"
missed="$(awk -F, '/^DA:/ { if ($2 + 0 == 0) n++ } END { print n + 0 }' "$out")"

if [ -z "$lines" ] || [ "$lines" -eq 0 ]; then
  printf '%s\n' 'coverage gate: lcov report has no line records — refusing to guess' >&2
  exit 1
fi

total="$(awk -v l="$lines" -v m="$missed" 'BEGIN { printf "%.1f", (l-m)/l*100 }')"
printf 'coverage: %s lines, %s missed (%s%%) — floor %s%%\n' "$lines" "$missed" "$total" "$FLOOR"

awk -v l="$lines" -v m="$missed" -v f="$FLOOR" 'BEGIN { exit !((l-m)/l*100 >= f+0) }' || {
  printf '%s\n' "coverage gate REFUSED: $missed missed lines — ${total}% is below the ${FLOOR}% floor." >&2
  printf '%s\n' 'The floor may rise, never fall. Add tests or fix the regression.' >&2
  exit 1
}

printf '%s\n' 'coverage gate: pass'
