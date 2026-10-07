#!/usr/bin/env bash
# Coverage floor gate — brokkr-style honesty, swarmail-sized.
#
# The floor may rise; it may never fall. Raising it is an ordinary PR.
# Lowering it is refused here, in CI, and by review. Attribute-based
# exclusions (#[coverage(off)]) are forbidden so production code cannot
# shrink the denominator.
set -euo pipefail

FLOOR="${COVERAGE_FLOOR:-59.9}"

command -v cargo-llvm-cov >/dev/null 2>&1 || {
  printf '%s\n' 'coverage gate: cargo-llvm-cov not on PATH (cargo install cargo-llvm-cov)' >&2
  exit 1
}

out="$(mktemp "${TMPDIR:-/tmp}/swarmail-cov.XXXXXX")"
trap 'rm -f "$out"' EXIT

cargo llvm-cov --summary-only --output-path "$out" >/dev/null 2>&1

# The TOTAL row's Lines / Missed Lines columns (8th and 9th fields in
# llvm-cov's fixed summary layout) — compute the percentage from them
# rather than trusting a display column.
total="$(awk '/^TOTAL/ {printf "%.1f", ($8-$9)/$8*100; exit}' "$out")"

if [ -z "$total" ]; then
  printf '%s\n' 'coverage gate: could not parse the TOTAL line — refusing to guess' >&2
  exit 1
fi

printf 'coverage: %s%% lines (floor %s%%)\n' "$total" "$FLOOR"

awk -v t="$total" -v f="$FLOOR" 'BEGIN { exit !(t+0 >= f+0) }' || {
  printf '%s\n' "coverage gate REFUSED: ${total}% is below the ${FLOOR}% floor." >&2
  printf '%s\n' 'The floor may rise, never fall. Add tests or fix the regression.' >&2
  exit 1
}

printf '%s\n' 'coverage gate: pass'
