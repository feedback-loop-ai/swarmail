#!/usr/bin/env bash
# Exact coverage gate — literal 100% of lines, branches and functions.
#
# The contract (copied from brokkr-rewrite's scripts/coverage-exact.sh):
#
#   1. Counts come from the canonical LCOV records of the report this gate
#      itself produces — never from a display column or a rounded
#      percentage:
#        - a LINE (DA:) record is covered when its hit field > 0;
#        - a BRANCH (BRDA:) record is covered when its taken field is a
#          positive integer. The never-instrumented '-' records are brokkr's
#          exclusion, kept verbatim: they count, and can never be covered,
#          so a report carrying one refuses by equality.
#        - a FUNCTION is covered when any positive compiled instance exists.
#        Rust crate hashes and generic call sites mint many symbols for one
#        source function; a function's stable identity is file + start line,
#        and any compiled instance of it with FNDA > 0 covers it.
#   2. The gate passes iff all three counts are > 0 AND covered == count in
#      every dimension — literal integer equality. There is no percentage
#      knob: the contract is exact 100/100/100. The floor may rise, never
#      fall; COVERAGE_FLOOR is accepted only at its final value, 100, and
#      any other value is refused.
#   3. Hardening: every run exports a unique CARGO_LLVM_COV_TARGET_DIR, so
#      no stale instrumented executable or profile from an earlier source
#      graph can join the merge; the full lcov report and a machine-readable
#      summary are preserved under target/coverage/ before the verdict, so a
#      refusal leaves burn-down evidence; test-harness source (tests.rs,
#      *_tests.rs, tests/) leaking into the production report refuses; and
#      any coverage(off) attribute exclusion in src/ refuses — production
#      code cannot shrink the denominator.
#   4. The measurement is nightly: cargo +<nightly> llvm-cov --workspace
#      --locked --branch, where <nightly> is an installed toolchain that
#      actually carries llvm-tools. An unqualified `+nightly` is refused by
#      construction: rustup resolves it to the latest channel build and will
#      try to auto-install it — impossible inside a hermetic seat, and a
#      silent toolchain drift everywhere else. Without a nightly there are
#      no BRDA records and the branch dimension would not exist.
set -euo pipefail

# Exact 100/100/100 has no percentage knob. COVERAGE_FLOOR survives only as a
# hardened value: anything other than 100 is refused, so no environment can
# lower the bar below covered == count.
if [ -n "${COVERAGE_FLOOR:-}" ] && [ "$COVERAGE_FLOOR" != "100" ]; then
  printf '%s\n' "coverage gate: COVERAGE_FLOOR=$COVERAGE_FLOOR refused — the contract is exact 100% of lines, branches and functions; the floor may rise, never fall" >&2
  exit 1
fi

command -v jq >/dev/null 2>&1 || {
  printf '%s\n' 'coverage refusal: jq is required for exact integer verification' >&2
  exit 1
}

cd "$(dirname "$0")/.."

# Attribute-based source exclusions are forbidden everywhere under src/ so
# production code cannot silently shrink the denominator (house-rules). The
# check must run, not guess: a git failure refuses, and GIT_CONFIG_COUNT=0
# sidesteps harness sessions that export a broken GIT_CONFIG_KEY_0 (repo-local
# config still applies).
git_grep_status=0
attribute_hits="$(GIT_CONFIG_COUNT=0 git grep -n 'coverage(off)' -- 'src/*.rs' 'src/**/*.rs')" || git_grep_status=$?
if [ "$git_grep_status" -gt 1 ]; then
  printf '%s\n' "coverage refusal: the coverage(off) check could not run (git grep exit $git_grep_status)" >&2
  exit 1
fi
if [ -n "$attribute_hits" ]; then
  printf '%s\n' "$attribute_hits"
  printf '%s\n' 'coverage refusal: attribute-based source exclusions are forbidden' >&2
  exit 1
fi

# Resolve the measurement toolchain: an installed nightly whose llvm-tools
# component is present. Nothing is auto-installed — a hermetic seat cannot
# install toolchains, and the gate's verdict must come from what is on disk.
coverage_toolchain=""
while IFS= read -r tc; do
  tc="${tc%% *}"
  case "$tc" in nightly-*) ;; *) continue ;; esac
  if rustup component list --installed --toolchain "$tc" 2>/dev/null | grep -q '^llvm-tools'; then
    coverage_toolchain="$tc"
    break
  fi
done < <(rustup toolchain list 2>/dev/null)
if [ -z "$coverage_toolchain" ]; then
  printf '%s\n' 'coverage refusal: no installed nightly toolchain carries llvm-tools — install one with rustup toolchain install nightly-<date> --component llvm-tools-preview; the gate never auto-installs' >&2
  exit 1
fi
if ! cargo +"$coverage_toolchain" llvm-cov --version >/dev/null 2>&1; then
  printf '%s\n' "coverage refusal: cargo +$coverage_toolchain llvm-cov is unavailable — cargo-llvm-cov must be installed (cargo install cargo-llvm-cov)" >&2
  exit 1
fi

forge_coverage_dir="$(mktemp -d "${TMPDIR:-/tmp}/swarmail-coverage.XXXXXX")"
trap 'rm -rf "$forge_coverage_dir"' EXIT
mkdir -p target/coverage

# A unique target directory is stronger than cleaning shared coverage state:
# no stale instrumented executable can participate in this candidate's merge.
export CARGO_LLVM_COV_TARGET_DIR="$forge_coverage_dir/target"

# A report is candidate-bound only when no instrumented executable or profile
# from an earlier source graph can participate in the merge.
cargo +"$coverage_toolchain" llvm-cov clean --workspace >/dev/null 2>&1

if ! cargo +"$coverage_toolchain" llvm-cov \
  --workspace \
  --locked \
  --branch \
  --lcov \
  --output-path "$forge_coverage_dir/lcov.info" \
  >"$forge_coverage_dir/run.log" 2>&1; then
  tail -n 40 "$forge_coverage_dir/run.log" >&2 || true
  cp "$forge_coverage_dir/run.log" target/coverage/run.log 2>/dev/null || true
  printf '%s\n' 'coverage refusal: the instrumented run failed — its log is preserved at target/coverage/run.log' >&2
  exit 1
fi

# Preserve the complete report before evaluating the contract. A red exact
# gate must still leave operators enough evidence to see and burn down every
# missing region instead of returning only an opaque non-zero exit.
cp "$forge_coverage_dir/lcov.info" target/coverage/lcov.info

# Test harnesses live in cargo-llvm-cov's conventional `tests.rs`,
# `*_tests.rs`, and `tests/` paths. (Inline #[cfg(test)] modules inside src/
# are the realm's unit-test shape: they are part of the report and enforced,
# not harness source.)
if awk '
  /^SF:/ {
    path = substr($0, 5);
    if (path ~ /(^|\/)(tests\.rs|[^\/]+_tests\.rs|tests\/)/) { print path; found = 1; exit }
  }
  END { exit found ? 1 : 0 }
' target/coverage/lcov.info; then :; else
  printf '%s\n' 'coverage refusal: test harness source leaked into the production report (record above)' >&2
  exit 1
fi

# Decision 0011 (proposed): harness names are reserved under src/. A file
# named tests.rs / *_tests.rs / *-tests.rs is dropped from the report by
# cargo-llvm-cov's default ignore regex, and the leak-check above fires only
# on paths PRESENT in the report — so production code parked in a
# harness-named file would be invisible to both while cargo build succeeds
# and the exact verdict silently lies (the denominator shrinks by whatever
# was hidden). Every harness-named file under src/ must therefore be
# declared by its parent module under #[cfg(test)]; anything else refuses,
# fail-closed, the same species as the coverage(off) grep above: a false
# positive is loud and names the file; the failure direction is refuse,
# never silently mis-count. This is a refusal, not a recount — no counting
# stage is added and none may be.
harness_refusals=""
while IFS= read -r harness; do
  stem="$(basename "$harness" .rs)"
  dir="$(dirname "$harness")"
  if [ "$dir" = "src" ]; then
    # A top-level harness file is declared by the crate root.
    parents=(src/lib.rs src/main.rs)
  else
    # src/store/persist_tests.rs is declared by src/store.rs (or the
    # conventional src/store/mod.rs fallback).
    parents=("${dir}.rs" "${dir}/mod.rs")
  fi
  declared=""
  for parent in "${parents[@]}"; do
    if [ -f "$parent" ] && awk -v stem="$stem" '
      BEGIN {
        escaped = stem;
        gsub(/[^A-Za-z0-9_]/, "\\\\&", escaped);
        re = "^[[:space:]]*mod[[:space:]]+" escaped "[[:space:]]*;$";
        re_attr = "^[[:space:]]*#\\[[^]]*\\][[:space:]]+mod[[:space:]]+" escaped "[[:space:]]*;$";
      }
      { lines[NR] = $0 }
      $0 ~ re || $0 ~ re_attr { mods[NR] = $0 }
      END {
        # Any declaration of `mod <stem>;` counts; the attribute stack
        # directly above it (over blank and comment lines) must carry
        # #[cfg(test)] — and no `mod` keyword may appear between, which the
        # non-attribute break enforces.
        for (m in mods) {
          if (mods[m] ~ /#\[cfg\(test\)\]/) exit 0;
          for (i = m - 1; i >= 1; i--) {
            up = lines[i];
            if (up ~ /^[[:space:]]*$/) continue;
            if (up ~ /^[[:space:]]*\/\//) continue;
            if (up ~ /^[[:space:]]*#/) {
              if (up ~ /#\[cfg\(test\)\]/) exit 0;
              continue;
            }
            break;
          }
        }
        exit 1;
      }
    ' "$parent"; then declared="$parent"; break; fi
  done
  if [ -z "$declared" ]; then
    harness_refusals="$harness_refusals $harness"
  fi
done < <(find src -type f \( -name 'tests.rs' -o -name '*_tests.rs' -o -name '*-tests.rs' \) | sort)
if [ -n "$harness_refusals" ]; then
  printf '%s\n' "coverage refusal: harness-named file(s) under src/ not declared by their parent module under #[cfg(test)]:${harness_refusals} — decision 0011 (docs/decisions/0011-concurrency-tests-and-the-coverage-gate.md)" >&2
  exit 1
fi

read -r line_count line_covered branch_count branch_covered function_count function_covered < <(
  awk '
    /^DA:/ {
      record = substr($0, 4);
      split(record, line_fields, ",");
      line_count += 1;
      if (line_fields[2] + 0 > 0) line_covered += 1;
    }
    /^BRDA:/ {
      record = substr($0, 6);
      split(record, branch_fields, ",");
      branch_count += 1;
      if (branch_fields[4] != "-" && branch_fields[4] + 0 > 0) branch_covered += 1;
    }
    /^SF:/ { source_file = substr($0, 4); }
    /^FN:/ {
      record = substr($0, 4);
      split(record, function_fields, ",");
      name = record;
      sub(/^[^,]*,/, "", name);
      function_start[source_file SUBSEP name] = function_fields[1];
    }
    /^FNDA:/ {
      record = substr($0, 6);
      split(record, function_fields, ",");
      name = record;
      sub(/^[^,]*,/, "", name);
      function_hits[source_file SUBSEP name] += function_fields[1] + 0;
    }
    END {
      # Rust crate hashes and generic call-site types create multiple symbols
      # for one source-defined function. Its stable identity is file + start
      # line; any positive compiled instance covers that source function.
      for (symbol in function_start) {
        split(symbol, parts, SUBSEP);
        source_function = parts[1] SUBSEP function_start[symbol];
        functions[source_function] = 1;
        if (function_hits[symbol] > 0) function_is_covered[source_function] = 1;
      }
      for (source_function in functions) {
        function_count += 1;
        if (function_is_covered[source_function]) function_covered += 1;
      }
      print line_count + 0, line_covered + 0,
            branch_count + 0, branch_covered + 0,
            function_count + 0, function_covered + 0;
    }
  ' target/coverage/lcov.info
)

jq -n \
  --argjson line_count "$line_count" \
  --argjson line_covered "$line_covered" \
  --argjson branch_count "$branch_count" \
  --argjson branch_covered "$branch_covered" \
  --argjson function_count "$function_count" \
  --argjson function_covered "$function_covered" \
  '{
    lines: {count: $line_count, covered: $line_covered},
    branches: {count: $branch_count, covered: $branch_covered},
    functions: {count: $function_count, covered: $function_covered}
  }' >target/coverage/coverage-summary.json

if (( line_count == 0 || line_covered != line_count ||
      branch_count == 0 || branch_covered != branch_count ||
      function_count == 0 || function_covered != function_count )); then
  jq . target/coverage/coverage-summary.json >&2
  printf '%s\n' 'coverage refusal: literal nonzero 100% source-line/branch/function equality not met — evidence: target/coverage/lcov.info, target/coverage/coverage-summary.json' >&2
  exit 1
fi

jq . target/coverage/coverage-summary.json
printf '%s\n' 'coverage gate: pass — exact 100/100/100 (lines/branches/functions)'
