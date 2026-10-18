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
#   3. Hardening: the instrumented build resolves to a fresh, unique
#      directory under target/coverage/ (gitignored, disk-backed) by
#      default — never under $TMPDIR, whose tmpfs quota turned a disk-full
#      build into a mid-build LLVM IO failure (issue #101) — so no stale
#      instrumented executable or profile from an earlier source graph can
#      join the merge; an explicit CARGO_LLVM_COV_TARGET_DIR is honoured
#      verbatim, and the EXIT trap removes only a directory the gate itself
#      created. A free-space preflight refuses before the build, naming the
#      path, the filesystem and the escape variable. The full lcov report,
#      the run log and a machine-readable summary are preserved in a
#      per-run evidence directory target/coverage/run.XXXXXX — deliberately
#      NOT removed by the EXIT trap (which cleans only the instrumented
#      build tree), so a refusal leaves burn-down evidence and consecutive
#      runs never overwrite each other; test-harness source (tests.rs,
#      *_tests.rs, tests/) measured into the production report refuses —
#      unless the file is in the production build's own file set (decision
#      0035: the refusal is by membership; the build's dep-info is the file
#      set); production source parked under src/**/{tests,examples,benches}/
#      refuses (it is excluded from the report by construction, so the
#      denominator would shrink silently); a coverage(off) exclusion that
#      survives the compiler's canonical expansion refuses (decision 0035);
#      and any literal coverage(off) spelling in src/ refuses pre-build —
#      production code cannot shrink the denominator.
#   4. The measurement is nightly: cargo +<nightly> llvm-cov --workspace
#      --locked --branch, where <nightly> is an installed toolchain that
#      actually carries llvm-tools. An unqualified `+nightly` is refused by
#      construction: rustup resolves it to the latest channel build and will
#      try to auto-install it — impossible inside a hermetic seat, and a
#      silent toolchain drift everywhere else. Without a nightly there are
#      no BRDA records and the branch dimension would not exist.
#   5. `coverage-gate.sh selftest` is part of the contract: it rebuilds
#      every refusal arm of every check against fixture trees under
#      target/coverage/selftest.XXXXXX (removed by the EXIT trap) and must
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
  # are the realm's unit-test shape: part of the report and enforced, not
  # harness source.) A harness-named SF record refuses — unless its file is IN
  # the production build's file set: measured-as-production is the desired end
  # state, a test harness measured as production is the original disease
  # (decision 0035's amendment of 0020 §3 — membership, never presence).
  local prod="$2" root="$3/"
  if awk -v prod="$prod" -v root="$root" '
    FILENAME == prod { set[$0] = 1; next }
    /^SF:/ {
      path = substr($0, 4);
      if (path ~ /(^|\/)(tests\.rs|[^\/]+_tests\.rs|tests\/)/) {
        rel = path;
        if (index(path, root) == 1) rel = substr(path, length(root) + 1);
        if (!(rel in set)) { print path > "/dev/stderr"; found = 1; exit }
      }
    }
    END { exit found ? 1 : 0 }
  ' "$prod" "$1"; then :; else
    refuse 'coverage refusal: test harness source measured as production while absent from the production build file set (record above; decision 0035: docs/decisions/0035-coverage-gate-refuses-by-effect.md)'
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

# --- the by-effect refusals (decision 0035) ----------------------------------
#
# Queries over artifacts the gate's own run already produced — the build's
# dep-info and the compiler's canonical expansion. Nothing here reads Rust
# source text: extraction at ingest, queries over the extracted record.

FN_SCAN_BIN=""

ensure_fn_scan() {
  # Build the parser helper once into the shared gitignored target dir; its
  # absence refuses — the gate never parses Rust with bash.
  [ -n "$FN_SCAN_BIN" ] && return 0
  # A cached machine builds offline; a fresh one bootstraps its registry
  # with the same pinned lockfile (a helper build is tooling bootstrap,
  # not the measured measurement — the workspace itself stays --offline).
  cargo build --manifest-path tools/coverage-fn-scan/Cargo.toml \
    --target-dir target/coverage/fn-scan-target --offline --locked >/dev/null 2>&1 ||
    cargo build --manifest-path tools/coverage-fn-scan/Cargo.toml \
      --target-dir target/coverage/fn-scan-target --locked >/dev/null 2>&1 || {
    refuse 'coverage refusal: the coverage-fn-scan helper could not be built (cargo build --manifest-path tools/coverage-fn-scan/Cargo.toml --offline --locked)'
  }
  FN_SCAN_BIN="$PWD/target/coverage/fn-scan-target/debug/coverage-fn-scan"
}

validate_report() {
  # Absence checks need a living report: a missing, empty or SF-less export
  # refuses before any check consumes it (a truncated merge would vacuously
  # pass the leak check, which fires only on records present).
  if [ ! -s "$1" ] || ! grep -q '^SF:' "$1"; then
    refuse "coverage refusal: the report $1 is missing, empty or carries no SF: records — every by-effect check reads the report this run produced, and an absent one refuses before any check consumes it"
  fi
}

check_report_covers_production_files() {
  # $1: report, $2: production set (sorted, workspace-relative), $3: root. A
  # harness-name exclusion may only drop files NOT in the production build's
  # file set: every fn-bearing file the build testifies to must have an SF:
  # record — measured or refused, never silently absent (decision 0035).
  local sfset="$1.sfset" missing
  LC_ALL=C awk -v root="$3/" '/^SF:/ { p = substr($0, 4); if (index(p, root) == 1) p = substr(p, length(root) + 1); print p }' "$1" | LC_ALL=C sort >"$sfset"
  missing="$(LC_ALL=C comm -23 "$2" "$sfset")" || refuse "coverage refusal: the production file set and the report's SF records could not be joined"
  if [ -n "$missing" ]; then
    printf '%s\n' "$missing" >&2
    refuse 'coverage refusal: production file(s) the build compiled are absent from the report (records above) — a harness-name exclusion may only drop files outside the production build file set (decision 0035: docs/decisions/0035-coverage-gate-refuses-by-effect.md)'
  fi
}

check_no_coverage_exclusions() {
  # $@: canonical expansion dumps. An exclusion that survives the compiler's
  # own output is a function compiled but uninstrumented; the helper's token
  # walk carves only #[automatically_derived] impl bodies (decision 0035).
  local expansion
  for expansion in "$@"; do
    [ -s "$expansion" ] || refuse "coverage refusal: the expansion $expansion is missing or empty — the expansion must resolve the same cfg set as the instrumented build"
    "$FN_SCAN_BIN" attrs "$expansion" || refuse "coverage refusal: a coverage exclusion survived into the canonical expansion (refusal above; decision 0035: docs/decisions/0035-coverage-gate-refuses-by-effect.md)"
  done
}

# --- instrumented build location ---------------------------------------------

# The instrumented build reserves ≈ 4 GiB (a full workspace build with
# instrumentation plus the profile data); the floor adds slack for the
# report, the merge and the evidence directory: 6 GiB, stated once.
MIN_BUILD_FREE_KIB=$((6 * 1024 * 1024))

check_free_space() {
  # $1: the directory whose filesystem must hold the instrumented build.
  # $2: the minimum free KiB. One df, fail-closed: the refusal names the
  # path, the filesystem and the escape variable — "the instrumented run
  # failed" (a mid-build LLVM IO failure, hours in) is exactly the refusal
  # this preflight exists to prevent (issue #101).
  local dir="$1" min_kib="$2" df_line avail_kib fs mount
  if ! df_line="$(df -Pk "$dir" 2>/dev/null)"; then
    refuse "coverage refusal: the free-space preflight could not stat $dir — the instrumented build reserves ${min_kib} KiB; point CARGO_LLVM_COV_TARGET_DIR at a writable path with room"
  fi
  avail_kib="$(printf '%s\n' "$df_line" | awk 'NR == 2 { print $4 }')"
  fs="$(printf '%s\n' "$df_line" | awk 'NR == 2 { print $1 }')"
  mount="$(printf '%s\n' "$df_line" | awk 'NR == 2 { print $6 }')"
  case "$avail_kib" in
    ''|*[!0-9]*)
      refuse "coverage refusal: the free-space preflight could not read the free space of $dir (filesystem ${fs:-?}) — the instrumented build reserves ${min_kib} KiB; point CARGO_LLVM_COV_TARGET_DIR at a writable path with room"
      ;;
  esac
  if (( avail_kib < min_kib )); then
    refuse "coverage refusal: $dir has $avail_kib KiB free on filesystem $fs (mounted at $mount); the instrumented build reserves $min_kib KiB — free space there, or point CARGO_LLVM_COV_TARGET_DIR at a larger filesystem"
  fi
}

resolve_build_dir() {
  # The instrumented build directory, two arms (issue #101):
  #   explicit — CARGO_LLVM_COV_TARGET_DIR is honoured verbatim: the caller
  #     owns the directory and its cleanup, and the gate exports the value
  #     it was given, unmodified;
  #   default  — a fresh mktemp directory under the gitignored, disk-backed
  #     target/coverage/, never $TMPDIR: an instrumented build is ≈ 4 GiB
  #     and a tmpfs quota failure surfaces as a mid-build LLVM IO error, not
  #     a refusal.
  # Sets RESOLVED_BUILD_DIR and BUILD_DIR_OWNED (1 only when the gate
  # created the directory — only an owned directory is trap-removed).
  if [ -n "${CARGO_LLVM_COV_TARGET_DIR:-}" ]; then
    BUILD_DIR_OWNED=0
    mkdir -p "$CARGO_LLVM_COV_TARGET_DIR"
    RESOLVED_BUILD_DIR="$CARGO_LLVM_COV_TARGET_DIR"
  else
    BUILD_DIR_OWNED=1
    mkdir -p target/coverage
    RESOLVED_BUILD_DIR="$(mktemp -d "$PWD/target/coverage/build.XXXXXX")"
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

# Resolve the measurement toolchain: the `nightly` channel alias when it
# carries llvm-tools — exactly what CI's rust-toolchain@nightly resolves to —
# else an installed dated nightly that does. Nothing is auto-installed: a
# hermetic seat cannot install toolchains, and the gate's verdict must come
# from what is on disk. Sets coverage_toolchain; shared with the selftest's
# real-build arms, which must build with the same nightly the gate measures.
probe_llvm_tools() {
  rustup component list --installed --toolchain "$1" 2>/dev/null | grep -q '^llvm-tools'
}

resolve_coverage_toolchain() {
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
}

# --- selftest ---------------------------------------------------------------

selftest() {
  # $1: the fixture root — created and trap-removed by the caller, under the
  # disk-backed target/coverage/, never $TMPDIR (issue #101).
  local root="$1"
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
  expect_refuse_naming() {
    # A refusal whose message must name $2 — the preflight's refusal is only
    # useful when it says where, and what the escape is.
    local label="$1" needle="$2"; shift 2
    local out status=0
    out="$("$@" 2>&1)" || status=$?
    if [ "$status" -eq 0 ]; then
      printf 'coverage gate: selftest failure — %s: expected refusal, got success:\n%s\n' "$label" "$out" >&2
      failures=$((failures + 1))
    elif [[ "$out" != *"$needle"* ]]; then
      printf 'coverage gate: selftest failure — %s: the refusal does not name "%s":\n%s\n' "$label" "$needle" "$out" >&2
      failures=$((failures + 1))
    fi
  }
  expect_eq() {
    local label="$1" got="$2" want="$3"
    if [ "$got" != "$want" ]; then
      printf 'coverage gate: selftest failure — %s: got "%s", want "%s"\n' "$label" "$got" "$want" >&2
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

  # 3. Harness source inside the report refuses — by membership (decision
  #    0035): the fake /repo paths are outside every production set, so the
  #    refusal fires; a production-only report passes.
  : > "$root/empty-set.txt"
  printf 'SF:/repo/src/tests/leaked.rs\nDA:1,1\nend_of_record\n' > "$root/leak.info"
  expect_refuse 'harness source in the report is refused' check_no_harness_leak "$root/leak.info" "$root/empty-set.txt" "$PWD"
  printf 'SF:/repo/src/api.rs\nDA:1,1\nend_of_record\n' > "$root/clean.info"
  expect_pass 'a production-only report passes the leak check' check_no_harness_leak "$root/clean.info" "$root/empty-set.txt" "$PWD"

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
  expect_eq 'canonical counting' "$got" '3 2 2 1 3 2'

  # 7. Build-dir resolution (issue #101): an explicit CARGO_LLVM_COV_TARGET_DIR
  #    verbatim and not owned (the trap never removes the caller's
  #    directory); the default a fresh, owned directory under the
  #    disk-backed target/coverage/ — never $TMPDIR.
  local resolved
  resolved="$(CARGO_LLVM_COV_TARGET_DIR="$root/explicit" resolve_build_dir; printf '%s %s' "$BUILD_DIR_OWNED" "$RESOLVED_BUILD_DIR")"
  expect_eq 'an explicit CARGO_LLVM_COV_TARGET_DIR is honoured verbatim and not owned' "$resolved" "0 $root/explicit"
  expect_eq 'the explicit build dir is created' "$( [ -d "$root/explicit" ] && echo yes || echo no )" yes
  mkdir -p "$root/tmpdir"
  resolved="$(unset CARGO_LLVM_COV_TARGET_DIR; TMPDIR="$root/tmpdir" resolve_build_dir; printf '%s %s' "$BUILD_DIR_OWNED" "$RESOLVED_BUILD_DIR")"
  case "$resolved" in
    "1 $PWD/target/coverage/build."*)
      resolved="${resolved#1 }"
      expect_eq 'the default build dir is created' "$( [ -d "$resolved" ] && echo yes || echo no )" yes
      rm -rf "$resolved"
      ;;
    *) expect_eq 'the default build dir is owned and under target/coverage/, never TMPDIR' "$resolved" "1 $PWD/target/coverage/build.XXXXXX" ;;
  esac

  # 8. The free-space preflight refuses before the build: a floor larger
  #    than the filesystem refuses naming the path, the filesystem and the
  #    escape variable; an unstatable path refuses naming the escape; a
  #    floor the filesystem holds passes.
  local fsname
  fsname="$(df -Pk "$root" | awk 'NR == 2 { print $1 }')"
  expect_refuse_naming 'a short filesystem refusal names the path' "$root has" check_free_space "$root" 999999999999
  expect_refuse_naming 'a short filesystem refusal names the filesystem' "filesystem $fsname" check_free_space "$root" 999999999999
  expect_refuse_naming 'a short filesystem refusal names the escape variable' 'CARGO_LLVM_COV_TARGET_DIR' check_free_space "$root" 999999999999
  expect_refuse_naming 'an unstatable path refuses naming the escape variable' 'CARGO_LLVM_COV_TARGET_DIR' check_free_space "$root/absent/dir" 1
  expect_pass 'a preflight floor of 1 KiB passes' check_free_space "$root" 1

  # 9. The by-effect refusals (decision 0035), proven from REAL cargo builds —
  #    never from text fixtures. The bypass fixture carries every harness-named
  #    load shape of #101's A, B, C, D, G and every exclusion spelling of E; its
  #    green twin is the same minus the bypasses, plus a declarations-only file,
  #    a derive(PartialEq, Eq) struct and a const fn — one green fixture proves
  #    the fn probe, the carve-out and const-fn instrumentation at once.
  fx_cmd() { # $1: dir, $2: log name, $3: label, $4: stdout file, rest: command.
    local dir="$1" log="$2" label="$3" out="$4"; shift 4
    (cd "$dir" && "$@") >"$out" 2>"$root/$log" || {
      printf 'coverage gate: selftest failure — %s: the real build failed:\n' "$label" >&2
      tail -n 30 "$root/$log" >&2
      exit 1
    }
  }
  resolve_coverage_toolchain; ensure_fn_scan
  local fxb="$root/fx-bypass" fxg="$root/fx-green" fxt="$root/fx-bypass-target" fxgt="$root/fx-green-target"
  mkdir -p "$fxb/src" "$fxg/src"
  printf '[package]\nname = "fxbypass"\nversion = "0.1.0"\nedition = "2021"\n' >"$fxb/Cargo.toml"
  printf '%b' "#![feature(coverage_attribute)]\n#![allow(dead_code)]\\npub fn visible() -> i32 { 42 }\\n#[coverage(off)]\\nfn direct_hidden() {}\\nmacro_rules! hide {\\n    (\$n:ident) => { #[coverage(off)] fn \$n() {} };\\n}\\nhide!(macro_hidden);\\n#[cfg_attr(all(), coverage(/**/ off ))]\\nfn attr_hidden() {}\\n#[cfg_attr(coverage, coverage( off ))]\\nfn cfg_gated_hidden() {}\\npub fn inner_hidden() { #![coverage( off )] }\\n#[automatically_derived]\\n#[coverage(/**/ off )]\\nfn derived_shadow_hidden() {}\\nmod tail { #[automatically_derived] pub struct Tail; }\\ninclude!(\"inc_tests.rs\");\\ninclude!(\"macros_tests.rs\");\\n#[path = \"path_tests.rs\"]\\npub mod path_loaded;\\n#[cfg(not(test))]\\npub(crate) mod cfg_tests;\\n/* #[cfg(test)] */\\nmod guard_tests;\\npub mod\\n    split_tests;\\n/* #[cfg(test)] mod split_tests; */\\n#[cfg(test)]\\nmod t { #[test] fn calls_visible() { assert_eq!(super::visible(), 42); } }\\n" >"$fxb/src/lib.rs"
  for shape in inc path cfg guard split; do printf 'pub fn %s_hidden() {}\n' "$shape" >"$fxb/src/${shape}_tests.rs"; done
  printf 'macro_rules! unused_gen { () => {}; }\npub const HANDLER: u8 = 7;\n' >"$fxb/src/macros_tests.rs"
  printf '[package]\nname = "fxgreen"\nversion = "0.1.0"\nedition = "2021"\n' >"$fxg/Cargo.toml"
  printf '%b' "pub fn visible() -> i32 { 42 }\\npub const fn meaning() -> i32 { 42 }\\n#[derive(Debug, PartialEq, Eq)]\\npub struct Compared { pub a: u8, pub b: u8 }\\nconst _: &str = include_str!(\"data.txt\");\\n#[cfg(test)]\\nmod t {\\n    #[test]\\n    fn proves() {\\n        assert_eq!(super::visible(), 42);\\n        assert_eq!(super::meaning(), 42);\\n        assert_eq!(super::Compared { a: 1, b: 2 }, super::Compared { a: 1, b: 2 });\\n    }\\n}\\n" >"$fxg/src/lib.rs"
  printf '//! Declarations only — nothing any dimension could count.\npub struct Bare { pub x: u8 }\n' >"$fxg/src/model.rs"
  printf 'hello swarmail\n' >"$fxg/src/data.txt"
  for fx in "$fxb" "$fxg"; do (cd "$fx" && cargo +"$coverage_toolchain" generate-lockfile --offline >/dev/null 2>&1); done
  for fx in "fxbypass $fxb $fxt bypass" "fxgreen $fxg $fxgt green"; do
    set -- $fx
    fx_cmd "$2" "$4-cov.log" "the $1 report" "$root/$4-cov.out" env CARGO_LLVM_COV_TARGET_DIR="$3" cargo +"$coverage_toolchain" llvm-cov --lcov --offline --locked --output-path "$root/$4.info"
    # A lib-only fixture's instrumented run builds no rlib (no bin consumes
    # one), so the harvest reads a plain cargo build's dep-info of the same
    # source graph — a real cargo depfile either way.
    fx_cmd "$2" "$4-lib.log" "the $1 lib build" "$root/$4-lib.out" cargo +"$coverage_toolchain" build --lib --offline --locked --target-dir "$3"
    fx_cmd "$root" "$4-harvest.log" "the $1 harvest" "$root/$4-prod.txt" "$FN_SCAN_BIN" files --workspace "$2" --build-dir "$3" --package "$1" --capture "$root/$4-depinfo"
    fx_cmd "$2" "$4-exp.log" "the $1 expansion" "$root/$4-exp.rs" cargo +"$coverage_toolchain" rustc --lib --offline --locked --target-dir "$3" -- -Zunpretty=expanded --cfg coverage --cfg coverage_nightly
  done
  for arm in 'src/inc_tests.rs:#101 D' 'src/path_tests.rs:#101 A' 'src/cfg_tests.rs:#101 B' 'src/guard_tests.rs:#101 C' 'src/split_tests.rs:#101 G' 'src/macros_tests.rs:the macro/const inventory'; do
    expect_refuse_naming "stage 1 refuses the ${arm#*:} harness load" "${arm%%:*}" check_report_covers_production_files "$root/bypass.info" "$root/bypass-prod.txt" "$fxb"
  done
  for arm in 'direct_hidden:direct' 'macro_hidden:macro-emitted' 'attr_hidden:cfg_attr-resolved' 'cfg_gated_hidden:coverage-cfg-gated' 'derived_shadow_hidden:derive-shadowed'; do
    expect_refuse_naming "stage 2 refuses the ${arm#*:} exclusion" "${arm%%:*}" check_no_coverage_exclusions "$root/bypass-exp.rs"
  done
  expect_refuse 'the inner-attribute exclusion refuses (the fn name is not adjacent to an inner attr)' check_no_coverage_exclusions "$root/bypass-exp.rs"
  expect_pass 'stage 1 passes the green twin (declarations-only file, asset, derive, const fn)' check_report_covers_production_files "$root/green.info" "$root/green-prod.txt" "$fxg"
  expect_pass 'stage 2 passes the green twin (the minted exclusion carved, the const fn silent)' check_no_coverage_exclusions "$root/green-exp.rs"

  # 10. Fast arms: validate-before-consume, the leak check's membership axis,
  #     a hostile dep-info entry, a dep-info-less dir, an empty expansion.
  : >"$root/empty.info"
  expect_refuse_naming 'an empty report refuses naming the artifact' "$root/empty.info" validate_report "$root/empty.info"
  printf 'DA:1,1\nend_of_record\n' >"$root/sfless.info"
  expect_refuse_naming 'an SF-less report refuses naming the artifact' 'no SF:' validate_report "$root/sfless.info"
  expect_pass 'a real report passes validation' validate_report "$root/bypass.info"
  printf 'SF:%s/src/inc_tests.rs\nDA:1,1\nend_of_record\n' "$fxb" >"$root/measured.info"
  printf 'src/inc_tests.rs\n' >"$root/prod-set.txt"
  expect_pass 'a harness-named SF record inside the production set passes' \
    check_no_harness_leak "$root/measured.info" "$root/prod-set.txt" "$fxb"
  printf 'SF:%s/src/leaked_tests.rs\nDA:1,1\nend_of_record\n' "$fxb" >"$root/unmeasured.info"
  expect_refuse 'a harness-named SF record outside the production set refuses' \
    check_no_harness_leak "$root/unmeasured.info" "$root/prod-set.txt" "$fxb"
  mkdir -p "$root/hostile" "$root/empty-target"
  sed 's|src/lib\.rs|src/vanished.rs|g' "$root/bypass-depinfo/"*.d >"$root/hostile/lib.d"
  expect_refuse_naming 'a hostile dep-info entry refuses naming it' 'src/vanished.rs' \
    "$FN_SCAN_BIN" files --workspace "$fxb" --build-dir "$root/hostile" --package fxbypass
  expect_refuse_naming 'a harvest over a dep-info-less dir refuses naming the path' 'empty-target' \
    "$FN_SCAN_BIN" files --workspace "$fxb" --build-dir "$root/empty-target" --package fxbypass
  : >"$root/empty-exp.rs"
  expect_refuse_naming 'an empty expansion refuses naming the artifact' "$root/empty-exp.rs" \
    check_no_coverage_exclusions "$root/empty-exp.rs"

  if [ "$failures" -ne 0 ]; then
    printf 'coverage gate: selftest failed with %s arm(s)\n' "$failures" >&2
    exit 1
  fi
  printf 'coverage gate: selftest pass — every refusal arm refuses, every pass arm passes\n'
}

case "${1:-}" in
  selftest)
    mkdir -p target/coverage
    selftest_root="$(mktemp -d "$PWD/target/coverage/selftest.XXXXXX")"
    trap 'rm -rf "$selftest_root"' EXIT
    selftest "$selftest_root"
    exit 0
    ;;
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

# The instrumented build directory is resolved, owned and preflighted before
# any toolchain work: the disk-full refusal must not need a nightly build to
# be heard (issue #101).
resolve_build_dir
forge_target_dir="$RESOLVED_BUILD_DIR"
if [ "$BUILD_DIR_OWNED" -eq 1 ]; then
  # The EXIT trap cleans only a directory the gate itself created; a caller's
  # explicit CARGO_LLVM_COV_TARGET_DIR is the caller's to keep.
  trap 'rm -rf "$forge_target_dir"' EXIT
fi
check_free_space "$forge_target_dir" "$MIN_BUILD_FREE_KIB"
# A unique target directory is stronger than cleaning shared coverage state:
# no stale instrumented executable can participate in this candidate's merge.
export CARGO_LLVM_COV_TARGET_DIR="$forge_target_dir"

resolve_coverage_toolchain

# Per-run evidence: a fresh directory per invocation, preserved after the run
# (the EXIT trap cleans only the instrumented build tree). Consecutive runs
# never overwrite each other's reports, and a refusal leaves burn-down
# evidence behind.
mkdir -p target/coverage
run_dir="$(mktemp -d target/coverage/run.XXXXXX)"

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

# Validate before consume: every new check is an absence check, so a truncated
# export refuses before any of them reads it (decision 0035's ordering).
validate_report "$run_dir/lcov.info"

# The production units' file set, harvested and captured from the gate's own
# instrumented build dir (decision 0035: extraction at ingest). Captured
# BEFORE the expansion pass, so its dep-info can never clobber the evidence; a
# missing dep-info refuses, naming the path and the shared-directory contract
# — the same window the staleness sentence governs.
ensure_fn_scan
unit_json="$(cargo metadata --no-deps --format-version 1 --offline --locked 2>/dev/null)" ||
  refuse 'coverage refusal: cargo metadata could not be read — the harvest needs the workspace units'

mapfile -t bin_args < <(jq -r '.packages[0].targets[] | select(.kind | index("bin")) | "--bin-root=" + .src_path' <<<"$unit_json")
if ! "$FN_SCAN_BIN" files --workspace "$PWD" --build-dir "$forge_target_dir" \
  --package "$(jq -r '.packages[0].name' <<<"$unit_json")" "${bin_args[@]}" \
  --capture "$run_dir/dep-info" >"$run_dir/production-files.txt" 2>"$run_dir/harvest.log"; then
  tail -n 20 "$run_dir/harvest.log" >&2 || true
  refuse "coverage refusal: the production file harvest failed — its log is preserved at $run_dir/harvest.log (a missing dep-info names the path and the shared-directory contract)"
fi

# The canonical expansion of both production units, into the preserved run
# dir, with the measurement's toolchain and target dir: a cfg mismatch refuses
# loudly (an empty dump is the loudest mismatch there is).
expand_unit() {
  # The cfg set matches the measurement's: cargo-llvm-cov enables cfg(coverage)
  # and cfg(coverage_nightly) for the instrumented build (its own run log says
  # so), and an expansion without them cannot see a
  # `#[cfg_attr(coverage, coverage(off))]` — the attribute resolves to nothing
  # and the exclusion sails past the scan. The review's HIGH on the cfg set.
  cargo +"$coverage_toolchain" rustc $1 --locked --offline \
    --target-dir "$forge_target_dir" -- -Zunpretty=expanded --cfg coverage --cfg coverage_nightly >"$2" 2>"$2.log" || {
    tail -n 20 "$2.log" >&2 || true
    refuse "coverage refusal: the expansion of the $1 unit failed — its log is preserved at $2.log"
  }
  [ -s "$2" ] || refuse "coverage refusal: the expansion $2 is empty — the expansion must resolve the same cfg set as the instrumented build"
}
expansion_files=("$run_dir/expanded-lib.rs")
expand_unit --lib "$run_dir/expanded-lib.rs"
mapfile -t bin_names < <(jq -r '.packages[0].targets[] | select(.kind | index("bin")) | .name' <<<"$unit_json")
for bin_name in "${bin_names[@]}"; do
  expansion_files+=("$run_dir/expanded-bin-$bin_name.rs")
  expand_unit "--bin $bin_name" "$run_dir/expanded-bin-$bin_name.rs"
done

check_no_harness_leak "$run_dir/lcov.info" "$run_dir/production-files.txt" "$PWD"
check_report_covers_production_files "$run_dir/lcov.info" "$run_dir/production-files.txt" "$PWD"
check_no_coverage_exclusions "${expansion_files[@]}"

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
