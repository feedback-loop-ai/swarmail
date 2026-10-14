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
#      graph can join the merge; the full lcov report, the run log and a
#      machine-readable summary are preserved in a per-run evidence
#      directory target/coverage/run.XXXXXX — deliberately NOT removed by
#      the EXIT trap (which cleans only the instrumented build tree), so a
#      refusal leaves burn-down evidence and consecutive runs never
#      overwrite each other; test-harness source (tests.rs, *_tests.rs,
#      tests/) leaking into the production report refuses; production
#      source parked under src/**/{tests,examples,benches}/ refuses (it is
#      excluded from the report by construction, so the denominator would
#      shrink silently); and any coverage(off) attribute exclusion in src/
#      refuses — production code cannot shrink the denominator.
#   4. The measurement is nightly: cargo +<nightly> llvm-cov --workspace
#      --locked --branch, where <nightly> is an installed toolchain that
#      actually carries llvm-tools. An unqualified `+nightly` is refused by
#      construction: rustup resolves it to the latest channel build and will
#      try to auto-install it — impossible inside a hermetic seat, and a
#      silent toolchain drift everywhere else. Without a nightly there are
#      no BRDA records and the branch dimension would not exist.
#   5. `coverage-gate.sh selftest` is part of the contract: it rebuilds
#      every refusal arm of every check against mktemp fixtures and must
#      pass before the gate's verdict can be trusted. It is fail-closed —
#      an arm that passes when it must refuse, or fails when it must pass,
#      exits non-zero.
set -euo pipefail

# Exact 100/100/100 has no percentage knob. COVERAGE_FLOOR survives only as a
# hardened value: anything other than 100 is refused, so no environment can
# lower the bar below covered == count.
if [ -n "${COVERAGE_FLOOR:-}" ] && [ "$COVERAGE_FLOOR" != "100" ]; then
  printf '%s\n' "coverage gate: COVERAGE_FLOOR=$COVERAGE_FLOOR refused — the contract is exact 100% of lines, branches and functions; the floor may rise, never fall" >&2
  exit 1
fi

cd "$(dirname "$0")/.."

refuse() {
  printf '%s\n' "$1" >&2
  exit 1
}

# --- static source checks ---------------------------------------------------
#
# Pure functions over a source directory (or a report), so the selftest can
# rebuild every refusal arm against a fixture tree. They run before any
# toolchain work: a refusal must not need a nightly build to be heard.

check_no_coverage_off() {
  # Attribute-based source exclusions are forbidden everywhere under src/ so
  # production code cannot silently shrink the denominator (house-rules). A
  # plain recursive grep: the check must run, not guess — a scan failure
  # refuses, and no VCS plumbing (git config workarounds, index state)
  # belongs inside a contract check.
  local hits status=0
  hits="$(grep -R -n --include='*.rs' 'coverage(off)' "$1")" || status=$?
  if [ "$status" -gt 1 ]; then
    refuse "coverage refusal: the coverage(off) check could not run (grep exit $status)"
  fi
  if [ -n "$hits" ]; then
    printf '%s\n' "$hits" >&2
    refuse 'coverage refusal: attribute-based source exclusions are forbidden'
  fi
}

check_no_hidden_harness_dirs() {
  # Production source parked inside src/**/tests/, examples/ or benches/ is
  # dropped from the report by cargo-llvm-cov's default ignore regex and is
  # invisible to the harness-name check (which sees files, not directories):
  # the build succeeds while the exact verdict silently lies. Refuse,
  # fail-closed, naming every file.
  local hits
  hits="$(find "$1" -type f -name '*.rs' \( -path '*/tests/*' -o -path '*/examples/*' -o -path '*/benches/*' \))"
  if [ -n "$hits" ]; then
    printf '%s\n' "$hits" >&2
    refuse 'coverage refusal: production source under a harness directory (src/**/{tests,examples,benches}/) is excluded from the report by construction — move it to a measured path'
  fi
}

check_no_harness_leak() {
  # Test harnesses live in cargo-llvm-cov's conventional `tests.rs`,
  # `*_tests.rs`, and `tests/` paths. (Inline #[cfg(test)] modules inside src/
  # are the realm's unit-test shape: they are part of the report and enforced,
  # not harness source.) A report carrying one means the measurement counted
  # a harness as production source.
  if awk '
    /^SF:/ {
      path = substr($0, 5);
      if (path ~ /(^|\/)(tests\.rs|[^\/]+_tests\.rs|tests\/)/) { print path > "/dev/stderr"; found = 1; exit }
    }
    END { exit found ? 1 : 0 }
  ' "$1"; then :; else
    refuse 'coverage refusal: test harness source leaked into the production report (record above)'
  fi
}

check_harness_declarations() {
  # Decision 0020 (docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md):
  # harness names are reserved under src/. A file named tests.rs / *_tests.rs
  # / *-tests.rs is dropped from the report by cargo-llvm-cov's default
  # ignore regex, and the leak-check fires only on paths PRESENT in the
  # report — so production code parked in a harness-named file would be
  # invisible to both while cargo build succeeds and the exact verdict
  # silently lies (the denominator shrinks by whatever was hidden). Every
  # harness-named file under src/ must therefore be declared by its parent
  # module under #[cfg(test)] — EVERY declaration of it, in EVERY declaring
  # parent (a first-match break would let one guarded declaration launder an
  # unguarded one), and no declaration may carry #[path = "..."], which
  # would move the reserved name somewhere the ignore regex cannot see. The
  # guard is the literal attribute `#[cfg(test)]` on its own line in the
  # attribute stack directly above the declaration (blank and comment lines
  # may intervene). Anything else refuses, fail-closed, the same species as
  # the coverage(off) grep: a false positive is loud and names the file; the
  # failure direction is refuse, never silently mis-count. This is a
  # refusal, not a recount — no counting stage is added and none may be.
  local src="$1" harness_refusals=""
  while IFS= read -r harness; do
    local stem dir; stem="$(basename "$harness" .rs)"; dir="$(dirname "$harness")"
    local parents=()
    if [ "$dir" = "$src" ]; then
      # A top-level harness file is declared by the crate root.
      parents=("$src/lib.rs" "$src/main.rs")
    else
      # src/store/persist_tests.rs is declared by src/store.rs (or the
      # conventional src/store/mod.rs fallback).
      parents=("$dir.rs" "$dir/mod.rs")
    fi
    local any_decl=0 any_bad=0 details=""
    local parent rc detail
    for parent in "${parents[@]}"; do
      [ -f "$parent" ] || continue
      rc=0
      detail="$(awk -v stem="$stem" -v harness="$harness" -v parent="$parent" '
        BEGIN {
          escaped = stem;
          gsub(/[^A-Za-z0-9_]/, "\\\\&", escaped);
        }
        {
          lines[NR] = $0;
          if ($0 ~ "^[[:space:]]*mod[[:space:]]+" escaped "[[:space:]]*;") mods[++nmods] = NR;
        }
        END {
          bad = 0;
          if (nmods == 0) exit 3;  # this parent does not declare the harness
          for (k = 1; k <= nmods; k++) {
            m = mods[k]; guarded = 0;
            # Walk the attribute stack above the declaration: blank and
            # comment lines may intervene; an attribute line must be the
            # literal guard, and a #[path = "..."] is a refusal in itself.
            for (i = m - 1; i >= 1; i--) {
              t = lines[i];
              gsub(/^[[:space:]]+|[[:space:]]+$/, "", t);
              if (t == "" || t ~ /^\/\// || t ~ /^\/\*/ || t ~ /^\*/ ) continue;
              if (t ~ /^#/) {
                if (t ~ /^#\[[[:space:]]*path[[:space:]]*=/) {
                  print harness ": declared via #[path] in " parent; bad = 1;
                } else if (t == "#[cfg(test)]") {
                  guarded = 1;
                }
                continue;
              }
              break;
            }
            if (!guarded) {
              print harness ": declared without #[cfg(test)] in " parent; bad = 1;
            }
          }
          exit bad ? 1 : 0;
        }
      ' "$parent")" || rc=$?
      if [ "$rc" -eq 3 ]; then continue; fi
      any_decl=1
      if [ "$rc" -ne 0 ]; then
        any_bad=1
        details="${details}${detail}
"
      fi
    done
    if [ "$any_bad" -ne 0 ]; then
      harness_refusals="${harness_refusals}${details}"
    elif [ "$any_decl" -eq 0 ]; then
      harness_refusals="${harness_refusals}${harness}: not declared by any parent module under #[cfg(test)]
"
    fi
  done < <(find "$src" -type f \( -name 'tests.rs' -o -name '*_tests.rs' -o -name '*-tests.rs' \) | sort)
  if [ -n "$harness_refusals" ]; then
    printf '%s' "$harness_refusals" >&2
    refuse 'coverage refusal: harness-named file(s) under src/ must be declared by their parent module under #[cfg(test)] (decision 0020: docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md)'
  fi
}

# --- canonical counting -----------------------------------------------------

count_dimensions() {
  # $1: the lcov report. Prints six integers: line_count line_covered
  # branch_count branch_covered function_count function_covered.
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
  ' "$1"
}

# --- selftest ---------------------------------------------------------------

selftest() {
  local root
  root="$(mktemp -d "${TMPDIR:-/tmp}/swarmail-coverage-selftest.XXXXXX")"
  local failures=0

  expect_pass() {
    local label="$1"; shift
    local out status=0
    out="$("$@" 2>&1)" || status=$?
    if [ "$status" -ne 0 ]; then
      printf 'coverage gate: selftest failure — %s: expected pass, got exit %s:\n%s\n' "$label" "$status" "$out" >&2
      failures=$((failures + 1))
    fi
  }
  expect_refuse() {
    local label="$1"; shift
    local out status=0
    out="$("$@" 2>&1)" || status=$?
    if [ "$status" -eq 0 ]; then
      printf 'coverage gate: selftest failure — %s: expected refusal, got success:\n%s\n' "$label" "$out" >&2
      failures=$((failures + 1))
    fi
  }

  # 1. The floor has no percentage knob: anything but 100 refuses before any
  #    toolchain work (a re-entry with a bad floor refuses at the top).
  expect_refuse 'COVERAGE_FLOOR=99 is refused' env COVERAGE_FLOOR=99 bash "$0" selftest

  # 2. coverage(off): the attribute refuses; clean source passes.
  local fx="$root/co-off"; mkdir -p "$fx/src"
  printf '#[coverage(off)]\nfn hot() {}\n' > "$fx/src/lib.rs"
  expect_refuse 'coverage(off) in src is refused' check_no_coverage_off "$fx/src"
  fx="$root/co-clean"; mkdir -p "$fx/src"
  printf 'fn hot() {}\n' > "$fx/src/lib.rs"
  expect_pass 'clean src passes the coverage(off) check' check_no_coverage_off "$fx/src"

  # 3. Harness source inside the report refuses; a production-only report
  #    passes.
  printf 'SF:/repo/src/tests/leaked.rs\nDA:1,1\nend_of_record\n' > "$root/leak.info"
  expect_refuse 'harness source in the report is refused' check_no_harness_leak "$root/leak.info"
  printf 'SF:/repo/src/api.rs\nDA:1,1\nend_of_record\n' > "$root/clean.info"
  expect_pass 'a production-only report passes the leak check' check_no_harness_leak "$root/clean.info"

  # 4. Production source parked in a harness directory refuses.
  fx="$root/hidden"; mkdir -p "$fx/src/tests"
  printf 'fn prod() {}\n' > "$fx/src/tests/prod.rs"
  expect_refuse 'production source under src/tests/ is refused' check_no_hidden_harness_dirs "$fx/src"
  fx="$root/visible"; mkdir -p "$fx/src"
  printf 'fn prod() {}\n' > "$fx/src/prod.rs"
  expect_pass 'production source on a measured path passes' check_no_hidden_harness_dirs "$fx/src"

  # 5. Harness declarations (decision 0020). Each arm is a fresh fixture
  #    tree; the failure direction is refuse.
  fx="$root/decl"; mkdir -p "$fx/src/store"
  printf 'fn persisted() {}\n' > "$fx/src/store/persist_tests.rs"

  printf 'mod persist_tests;\n' > "$fx/src/store/mod.rs"
  expect_refuse 'an unguarded harness declaration is refused' check_harness_declarations "$fx/src"

  printf '#[cfg(test)]\nmod persist_tests;\n' > "$fx/src/store/mod.rs"
  expect_pass 'a guarded harness declaration passes' check_harness_declarations "$fx/src"

  printf '// the persistence unit tests\n#[cfg(test)]\n\n/* measured, not excluded */\nmod persist_tests;\n' > "$fx/src/store/mod.rs"
  expect_pass 'blank and comment lines may intervene in the guard stack' check_harness_declarations "$fx/src"

  printf '#[cfg(test)] mod persist_tests;\n' > "$fx/src/store/mod.rs"
  expect_refuse 'a same-line attribute is not a stack guard' check_harness_declarations "$fx/src"

  printf '#[cfg(test)]\n#[path = "elsewhere.rs"]\nmod persist_tests;\n' > "$fx/src/store/mod.rs"
  expect_refuse 'a #[path] harness declaration is refused' check_harness_declarations "$fx/src"

  printf 'fn persisted() {}\n' > "$fx/src/store/persist_tests.rs"
  printf 'fn other() {}\n' > "$fx/src/store.rs"
  printf '#[cfg(test)]\nmod persist_tests;\n' > "$fx/src/store.rs"
  printf 'mod persist_tests;\n' > "$fx/src/store/mod.rs"
  expect_refuse 'one guarded declaration cannot launder an unguarded one' check_harness_declarations "$fx/src"

  printf 'fn other() {}\n' > "$fx/src/store.rs"
  rm "$fx/src/store/mod.rs"
  expect_refuse 'a harness file with no declaration at all is refused' check_harness_declarations "$fx/src"

  # 6. Canonical counting: DA hits, BRDA taken (the '-' record counts and can
  #    never be covered), and the file + start-line function identity across
  #    generic instantiations. Expected: lines 3/2, branches 2/1,
  #    functions 3/2.
  {
    printf 'SF:/repo/src/a.rs\n'
    printf 'FN:10,_ZN1a4hotE\nFNDA:3,_ZN1a4hotE\n'
    printf 'FN:20,_ZN1a4coldE\nFNDA:0,_ZN1a4coldE\n'
    printf 'BRDA:10,0,0,5\nBRDA:12,0,1,-\n'
    printf 'DA:10,3\nDA:12,0\nend_of_record\n'
    printf 'SF:/repo/src/a.rs\n'
    printf 'FN:10,_ZN1a4hot17hE\nFNDA:0,_ZN1a4hot17hE\n'
    printf 'end_of_record\n'
    printf 'SF:/repo/src/b.rs\n'
    printf 'FN:5,_ZN1b1gE\nFNDA:2,_ZN1b1gE\n'
    printf 'DA:5,2\nend_of_record\n'
  } > "$root/synth.info"
  local got
  got="$(count_dimensions "$root/synth.info")"
  if [ "$got" != "3 2 2 1 3 2" ]; then
    printf 'coverage gate: selftest failure — counting: got "%s", expected "3 2 2 1 3 2"\n' "$got" >&2
    failures=$((failures + 1))
  fi

  if [ "$failures" -ne 0 ]; then
    printf 'coverage gate: selftest failed with %s arm(s)\n' "$failures" >&2
    exit 1
  fi
  printf 'coverage gate: selftest pass — every refusal arm refuses, every pass arm passes\n'
}

case "${1:-}" in
  selftest) selftest; exit 0 ;;
  '') : ;;
  *) refuse "coverage gate: unknown argument '$1' (no argument runs the gate; 'selftest' runs the refusal-arm fixtures)" ;;
esac

command -v jq >/dev/null 2>&1 || {
  printf '%s\n' 'coverage refusal: jq is required for exact integer verification' >&2
  exit 1
}

# The static refusals are heard before a single toolchain byte is built.
check_no_coverage_off src
check_no_hidden_harness_dirs src
check_harness_declarations src

# Resolve the measurement toolchain: the `nightly` channel alias when it
# carries llvm-tools — exactly what CI's rust-toolchain@nightly resolves to —
# else an installed dated nightly that does. Nothing is auto-installed: a
# hermetic seat cannot install toolchains, and the gate's verdict must come
# from what is on disk.
probe_llvm_tools() {
  rustup component list --installed --toolchain "$1" 2>/dev/null | grep -q '^llvm-tools'
}
coverage_toolchain=""
if probe_llvm_tools nightly; then
  coverage_toolchain="nightly"
else
  while IFS= read -r tc; do
    tc="${tc%% *}"
    case "$tc" in nightly-*) ;; *) continue ;; esac
    if probe_llvm_tools "$tc"; then
      coverage_toolchain="$tc"
      break
    fi
  done < <(rustup toolchain list 2>/dev/null)
fi
if [ -z "$coverage_toolchain" ]; then
  refuse 'coverage refusal: no installed nightly toolchain carries llvm-tools — install one with rustup toolchain install nightly --component llvm-tools-preview; the gate never auto-installs'
fi
if ! cargo +"$coverage_toolchain" llvm-cov --version >/dev/null 2>&1; then
  refuse "coverage refusal: cargo +$coverage_toolchain llvm-cov is unavailable — cargo-llvm-cov must be installed (cargo install cargo-llvm-cov)"
fi

forge_coverage_dir="$(mktemp -d "${TMPDIR:-/tmp}/swarmail-coverage.XXXXXX")"
trap 'rm -rf "$forge_coverage_dir"' EXIT

# Per-run evidence: a fresh directory per invocation, preserved after the run
# (the EXIT trap cleans only the instrumented build tree). Consecutive runs
# never overwrite each other's reports, and a refusal leaves burn-down
# evidence behind.
mkdir -p target/coverage
run_dir="$(mktemp -d target/coverage/run.XXXXXX)"

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
  --output-path "$run_dir/lcov.info" \
  >"$run_dir/run.log" 2>&1; then
  tail -n 40 "$run_dir/run.log" >&2 || true
  refuse "coverage refusal: the instrumented run failed — its log is preserved at $run_dir/run.log"
fi

check_no_harness_leak "$run_dir/lcov.info"

read -r line_count line_covered branch_count branch_covered function_count function_covered < <(
  count_dimensions "$run_dir/lcov.info"
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
  }' >"$run_dir/coverage-summary.json"

if (( line_count == 0 || line_covered != line_count ||
      branch_count == 0 || branch_covered != branch_count ||
      function_count == 0 || function_covered != function_count )); then
  jq . "$run_dir/coverage-summary.json" >&2
  refuse "coverage refusal: literal nonzero 100% source-line/branch/function equality not met — evidence: $run_dir/lcov.info, $run_dir/coverage-summary.json"
fi

jq . "$run_dir/coverage-summary.json"
printf 'coverage gate: pass — exact 100/100/100 (lines/branches/functions); evidence: %s\n' "$run_dir"
