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
#      join the merge; a nonempty explicit CARGO_LLVM_COV_TARGET_DIR must
#      reserve a new leaf atomically and is retained for the caller. The EXIT
#      trap removes only the fresh default build directory. A free-space
#      preflight refuses before the build, naming the
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
#      and any literal coverage(off) spelling, including raw identifiers,
#      in src/ refuses pre-build — production code cannot shrink the
#      denominator.
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

# The gate and its scanner are the pinned bundle's bytes (decision 0050 §1):
# PIN_DIR is this script's own directory (/runtime/bundle/scripts in the
# verify box), and the candidate under measurement is the working directory
# — a repository root, which the caller (the verify seat, or the fixed public
# shim scripts/coverage-gate.sh) selects. Nothing under the candidate's
# scripts/ or tools/ is executed.
PIN_DIR="$(CDPATH='' cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SELF="$PIN_DIR/$(basename "${BASH_SOURCE[0]}")"
if [ ! -f Cargo.toml ]; then
  printf '%s\n' "coverage refusal: $PWD carries no Cargo.toml — run the gate from the candidate repository root" >&2
  exit 1
fi

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
  # production code cannot silently shrink the denominator. This conservative
  # recursive grep recognizes raw identifiers as well as ordinary spelling;
  # a scan failure refuses, and no VCS plumbing belongs inside this check.
  # The file is read whole (-z), so split attributes still match. Inline
  # #[cfg(test)] code is under src/ and is held to the same rule.
  local hits status=0
  hits="$(grep -R -l -z -E --include='*.rs' '(r#)?coverage[^A-Za-z0-9_(]*\([^A-Za-z0-9_)]*(r#)?off' "$1" | tr '\0' '\n')" || status=$?
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
  # invisible to the harness-name check. Refuse any incomplete inventory
  # before interpreting even one path.
  local hits status=0
  hits="$(find "$1" -type f -name '*.rs' \( -path '*/tests/*' -o -path '*/examples/*' -o -path '*/benches/*' \))" || {
    status=$?
    refuse "coverage refusal: hidden-directory inventory failed (find exit $status)"
  }
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
  local src="$1" harness_refusals="" harnesses status=0
  harnesses="$(find "$src" -type f \( -name 'tests.rs' -o -name '*_tests.rs' -o -name '*-tests.rs' \) | sort)" || {
    status=$?
    refuse "coverage refusal: harness declaration inventory failed (find | sort exit $status)"
  }
  if [ -n "$harnesses" ]; then
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
  done <<< "$harnesses"
  fi
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
  # The source, manifest and lock are the pinned ones; the build output lands
  # in the candidate's gitignored scratch space (the bundle mount is
  # read-only), and cargo's fingerprints rebuild it whenever the pinned
  # source differs from what built the cached binary.
  local manifest="$PIN_DIR/coverage-fn-scan/Cargo.toml"
  [ -f "$manifest" ] && [ -f "$PIN_DIR/coverage-fn-scan/Cargo.lock" ] ||
    refuse "coverage refusal: the pinned coverage-fn-scan source is missing ($manifest)"
  cargo build --manifest-path "$manifest" \
    --target-dir target/coverage/fn-scan-target --offline --locked >/dev/null 2>&1 ||
    cargo build --manifest-path "$manifest" \
      --target-dir target/coverage/fn-scan-target --locked >/dev/null 2>&1 || {
    refuse "coverage refusal: the coverage-fn-scan helper could not be built (cargo build --manifest-path $manifest --offline --locked)"
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

check_package_inventory() {
  # $1: full `cargo metadata` JSON (resolve included), $2: the canonical root
  # manifest path. Prints the root package's name. The root is the one
  # package whose manifest_path IS the root Cargo.toml — never metadata
  # order, which a member named `aaa` reorders (#140). The harvest and the
  # report cover exactly one application package, so any other workspace
  # member and any other local (source-less: path) package anywhere in the
  # resolved graph refuses by name: its production code would be compiled
  # and shipped but appear in no artifact the gate reads (decision 0050 §2).
  # A standalone tool crate with its own [workspace] (the pinned
  # coverage-fn-scan) is not in this graph at all.
  local verdict
  verdict="$(jq -r --arg m "$2" '
    [.packages[] | select(.manifest_path == $m)] as $roots
    | if ($roots | length) != 1 then
        "REFUSE \($roots | length) packages carry the root manifest \($m)"
      else
        $roots[0] as $r
        | (.workspace_members - [$r.id]) as $others
        | ([.packages[] | select(.id as $id | $others | index($id)) | "REFUSE workspace member \(.name) at \(.manifest_path) is not the measured package"]
           + [.packages[] | select(.id != $r.id and .source == null) | "REFUSE local package \(.name) at \(.manifest_path) is outside the harvest"]
           + ["ROOT \($r.name)"])[]
      end' "$1" 2>&1)" || refuse "coverage refusal: the package inventory could not be read from cargo metadata: $verdict"
  if printf '%s\n' "$verdict" | grep -q '^REFUSE'; then
    printf '%s\n' "$verdict" | sed -n 's/^REFUSE /  /p' >&2
    refuse 'coverage refusal: the measured graph holds production packages the gate does not harvest (above) — one application package is measured; extend the gate before adding another (decision 0050 §2)'
  fi
  printf '%s\n' "$verdict" | sed -n 's/^ROOT //p'
}

check_expansions_equivalent() {
  # $1: the measured expansion, $2: another configuration's, $3: its label,
  # $4: "release" for the release-profile comparison. Equivalent when
  # byte-identical. The release comparison alone also admits a line that is
  # exactly `if true {` becoming `if false {` at the same indentation — the
  # residue of cfg!(debug_assertions) inside debug_assert!, where the
  # guarded code is present (and measured) in both. Any other changed
  # boolean (a `cfg!(coverage)` value, a coverage-cfg `if`) and any item, arm
  # or body present in only one configuration refuses: code that exists only
  # where the gate does not measure is outside the denominator (#140,
  # decision 0050 §2). No other normalization.
  local d status=0
  [ -s "$1" ] || refuse "coverage refusal: the expansion $1 is missing or empty"
  [ -s "$2" ] || refuse "coverage refusal: the $3 expansion $2 is missing or empty"
  d="$(diff "$1" "$2")" || status=$?
  [ "$status" -eq 0 ] && return 0
  [ "$status" -eq 1 ] || refuse "coverage refusal: the $3 expansion comparison could not run (diff exit $status)"
  if [ "${4:-}" != release ] || ! printf '%s\n' "$d" | awk '
    function flush() { if (n != m) bad = 1; n = 0; m = 0 }
    /^[0-9]/ { flush(); if ($0 !~ /^[0-9]+(,[0-9]+)?c[0-9]+(,[0-9]+)?$/) bad = 1; next }
    /^< / { old[++n] = substr($0, 3); next }
    /^---$/ { next }
    /^> / {
      m++; o = old[m]; new = substr($0, 3)
      if (o !~ /^[ \t]*if true \{$/) bad = 1
      sub(/if true \{$/, "if false {", o); if (o != new) bad = 1
      next
    }
    { bad = 1 }
    END { flush(); exit bad }'; then
    printf '%s\n' "$d" | head -n 40 >&2
    refuse "coverage refusal: the $3 expansion differs from the measured one (diff above: $1 vs $2) — code that exists only in an unmeasured configuration is outside the denominator (decision 0050 §2)"
  fi
}

check_test_unit_expansion() {
  # $1: the library's test-unit expansion. It must actually be a cfg(test)
  # expansion (rustc's harness marks every #[test] with rustc_test_marker), or
  # the scan proves nothing; then it is scanned like a production unit —
  # inline #[cfg(test)] code stays measured (decision 0035).
  [ -s "$1" ] || refuse "coverage refusal: the test-unit expansion $1 is missing or empty"
  grep -q 'rustc_test_marker' "$1" ||
    refuse "coverage refusal: the test-unit expansion $1 carries no test harness — cfg(test) was not enabled, so the scan would prove nothing"
  check_no_coverage_exclusions "$1"
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
  # A nonempty explicit path is the exact target, but it must be a new leaf.
  # Parent creation is allowed; one atomic mkdir reserves the leaf. The
  # explicit reservation remains caller-owned on success and failure.
  # The default remains a fresh mktemp directory under disk-backed
  # target/coverage/, never $TMPDIR. BUILD_DIR_OWNED means the EXIT cleanup
  # may remove the directory, not merely that this call created it.
  local requested parent parent_probe status=0
  if [ -n "${CARGO_LLVM_COV_TARGET_DIR:-}" ]; then
    requested="$CARGO_LLVM_COV_TARGET_DIR"
    parent_probe="$requested"
    while [ "$parent_probe" != "/" ] && [[ "$parent_probe" == */ ]]; do
      parent_probe="${parent_probe%/}"
    done
    if [[ "$parent_probe" == */* ]]; then
      parent="${parent_probe%/*}"
      [ -n "$parent" ] || parent="/"
    else
      parent="."
    fi
    mkdir -p -- "$parent" || {
      status=$?
      refuse "coverage refusal: explicit target parent could not be created ($parent; mkdir exit $status)"
    }
    mkdir -- "$requested" || {
      status=$?
      refuse "coverage refusal: explicit target must be a fresh, unused leaf ($requested; mkdir exit $status) — choose a never-created target path"
    }
    BUILD_DIR_OWNED=0
    RESOLVED_BUILD_DIR="$requested"
  else
    BUILD_DIR_OWNED=1
    mkdir -p -- target/coverage || {
      status=$?
      refuse "coverage refusal: default target parent could not be created (target/coverage; mkdir exit $status)"
    }
    RESOLVED_BUILD_DIR="$(mktemp -d "$PWD/target/coverage/build.XXXXXX")" || {
      status=$?
      refuse "coverage refusal: default instrumented target could not be allocated (mktemp exit $status)"
    }
  fi
}

cleanup_build_dir() {
  # Explicit target reservations and their evidence belong to the caller.
  # Only the generated default build tree is automatically removed.
  local dir="$1" owned="$2"
  [ "$owned" -eq 1 ] || return 0
  rm -rf -- "$dir"
}

run_llvm_cov_clean() {
  # The clean is a prerequisite even when this function is called from a
  # conditional context where errexit is disabled. Preserve its diagnostics.
  local log="$1" status=0
  if cargo +"$coverage_toolchain" llvm-cov clean --workspace >"$log" 2>&1; then
    return 0
  else
    status=$?
  fi
  tail -n 40 "$log" >&2 || true
  refuse "coverage refusal: llvm-cov clean failed (exit $status) — diagnostics are at $log"
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
  local observed_refusal_status=-1
  expect_refuse_naming() {
    # Every refusal arm names its reason, not merely a nonzero status.
    local label="$1" needle="$2"; shift 2
    local out status=0
    out="$("$@" 2>&1)" || status=$?
    observed_refusal_status="$status"
    if [ "$status" -eq 0 ]; then
      printf 'coverage gate: selftest failure — %s: expected refusal, got success:\n%s\n' "$label" "$out" >&2
      failures=$((failures + 1))
    elif [[ "$out" != *"$needle"* ]]; then
      printf 'coverage gate: selftest failure — %s: refusal did not contain expected reason "%s":\n%s\n' "$label" "$needle" "$out" >&2
      failures=$((failures + 1))
    fi
  }
  expect_refuse_naming_context() {
    # Require both the refusal category and its fixture-specific discriminator
    # from the same captured invocation.
    local label="$1" reason="$2" context="$3"; shift 3
    local out status=0
    out="$("$@" 2>&1)" || status=$?
    observed_refusal_status="$status"
    if [ "$status" -eq 0 ]; then
      printf 'coverage gate: selftest failure — %s: expected refusal, got success:\n%s\n' "$label" "$out" >&2
      failures=$((failures + 1))
    elif [[ "$out" != *"$reason"* || "$out" != *"$context"* ]]; then
      printf 'coverage gate: selftest failure — %s: refusal must contain "%s" and "%s":\n%s\n' "$label" "$reason" "$context" "$out" >&2
      failures=$((failures + 1))
    fi
  }
  expect_refuse_naming_without() {
    local label="$1" needle="$2" forbidden="$3"; shift 3
    local out status=0
    out="$("$@" 2>&1)" || status=$?
    observed_refusal_status="$status"
    if [ "$status" -eq 0 ]; then
      printf 'coverage gate: selftest failure — %s: expected refusal, got success:\n%s\n' "$label" "$out" >&2
      failures=$((failures + 1))
    elif [[ "$out" != *"$needle"* ]]; then
      printf 'coverage gate: selftest failure — %s: refusal did not contain expected reason "%s":\n%s\n' "$label" "$needle" "$out" >&2
      failures=$((failures + 1))
    elif [[ "$out" == *"$forbidden"* ]]; then
      printf 'coverage gate: selftest failure — %s: the failed inventory consumed partial row "%s":\n%s\n' "$label" "$forbidden" "$out" >&2
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

  # Exercise the real assertion helper in a child-local failure counter. The
  # expected refusal passes, an unrelated exit 127 fails, and expected text
  # paired with success also fails; none may mutate the parent's count.
  refusal_helper_control() {
    local failures=0 observed_refusal_status=-1
    case "$1" in
      intended)
        expect_refuse_naming 'helper control: intended refusal' 'coverage refusal: intended control reason' \
          bash -c 'printf "%s\n" "coverage refusal: intended control reason" >&2; exit 42'
        ;;
      unrelated)
        expect_refuse_naming 'helper control: unrelated exit' 'coverage refusal: intended control reason' \
          bash -c 'printf "%s\n" "unrelated command failure" >&2; exit 127'
        ;;
      success)
        expect_refuse_naming 'helper control: text with success' 'coverage refusal: intended control reason' \
          bash -c 'printf "%s\n" "coverage refusal: intended control reason" >&2; exit 0'
        ;;
      *) return 2 ;;
    esac
    printf '%s %s\n' "$failures" "$observed_refusal_status"
  }
  local helper_parent_failures="$failures" helper_control
  helper_control="$(refusal_helper_control intended 2>/dev/null)"
  expect_eq 'the naming helper accepts its intended refusal and preserves status' "$helper_control" '0 42'
  helper_control="$(refusal_helper_control unrelated 2>/dev/null)"
  expect_eq 'the naming helper rejects unrelated exit 127 in child state' "$helper_control" '1 127'
  helper_control="$(refusal_helper_control success 2>/dev/null)"
  expect_eq 'the naming helper rejects expected text with status zero in child state' "$helper_control" '1 0'
  expect_eq 'refusal helper controls leave the parent failure count intact' "$failures" "$helper_parent_failures"

  # 1. The floor has no percentage knob: anything but 100 refuses before any
  #    toolchain work (a re-entry with a bad floor refuses at the top).
  expect_refuse_naming 'COVERAGE_FLOOR=99 is refused' 'COVERAGE_FLOOR=99 refused' env COVERAGE_FLOOR=99 bash "$SELF" selftest

  # 2. coverage(off): the attribute refuses; clean source passes.
  local fx="$root/co-off"; mkdir -p "$fx/src"
  printf '#[coverage(off)]\nfn hot() {}\n' > "$fx/src/lib.rs"
  expect_refuse_naming_context 'coverage(off) in src is refused' 'attribute-based source exclusions are forbidden' "$fx/src/lib.rs" check_no_coverage_off "$fx/src"
  fx="$root/co-raw-off"; mkdir -p "$fx/src"
  printf '#[coverage(r#off)]\nfn raw_hot() {}\n' > "$fx/src/lib.rs"
  expect_refuse_naming 'the source grep refuses raw-off' 'attribute-based source exclusions are forbidden' check_no_coverage_off "$fx/src"
  fx="$root/co-raw-name"; mkdir -p "$fx/src"
  printf '#[r#coverage(off)]\nfn raw_named_hot() {}\n' > "$fx/src/lib.rs"
  expect_refuse_naming 'the source grep refuses a raw coverage name' 'attribute-based source exclusions are forbidden' check_no_coverage_off "$fx/src"

  fx="$root/co-clean"; mkdir -p "$fx/src"
  printf 'fn hot() {}\n' > "$fx/src/lib.rs"
  expect_pass 'clean src passes the coverage(off) check' check_no_coverage_off "$fx/src"

  # 3. Harness source inside the report refuses — by membership (decision
  #    0035): the fake /repo paths are outside every production set, so the
  #    refusal fires; a production-only report passes.
  : > "$root/empty-set.txt"
  printf 'SF:/repo/src/tests/leaked.rs\nDA:1,1\nend_of_record\n' > "$root/leak.info"
  expect_refuse_naming_context 'harness source in the report is refused' 'test harness source measured as production while absent from the production build file set' '/repo/src/tests/leaked.rs' check_no_harness_leak "$root/leak.info" "$root/empty-set.txt" "$PWD"
  printf 'SF:/repo/src/api.rs\nDA:1,1\nend_of_record\n' > "$root/clean.info"
  expect_pass 'a production-only report passes the leak check' check_no_harness_leak "$root/clean.info" "$root/empty-set.txt" "$PWD"

  # Failed inventories are named, preserve stderr and never consume partial rows.
  fx="$root/decl-empty"; mkdir -p "$fx/src"
  expect_pass 'an empty harness inventory passes' check_harness_declarations "$fx/src"

  fx="$root/find-hidden"; mkdir -p "$fx/src/tests"
  printf 'fn row_from_partial_find() {}\n' > "$fx/src/tests/partial.rs"
  injected_find_status=42
  injected_find_output=""
  find() {
    [ -z "$injected_find_output" ] || printf '%s\n' "$injected_find_output"
    printf 'injected find failure\n' >&2
    return "$injected_find_status"
  }
  expect_refuse_naming 'hidden-directory find failure has a named refusal' 'hidden-directory inventory failed' check_no_hidden_harness_dirs "$fx/src"
  expect_refuse_naming_context 'hidden-directory find diagnostics are preserved' 'hidden-directory inventory failed' 'injected find failure' check_no_hidden_harness_dirs "$fx/src"
  injected_find_output="$fx/src/tests/partial.rs"
  expect_refuse_naming_without 'partial hidden-directory output is not consumed' 'hidden-directory inventory failed' "$fx/src/tests/partial.rs" check_no_hidden_harness_dirs "$fx/src"
  conditional_hidden_inventory() {
    if check_no_hidden_harness_dirs "$1"; then : > "$2"; fi
  }
  rm -f "$root/hidden-marker"
  expect_refuse_naming 'conditional hidden-directory find failure still refuses' 'hidden-directory inventory failed' conditional_hidden_inventory "$fx/src" "$root/hidden-marker"
  expect_eq 'conditional hidden-directory refusal has no post-check marker' "$( [ -e "$root/hidden-marker" ] && echo present || echo absent )" absent

  fx="$root/find-harness"; mkdir -p "$fx/src/store"
  printf 'fn row_from_partial_find() {}\n' > "$fx/src/store/partial_tests.rs"
  injected_find_output=""
  expect_refuse_naming 'harness find failure has a named refusal' 'harness declaration inventory failed' check_harness_declarations "$fx/src"
  expect_refuse_naming_context 'harness find diagnostics are preserved' 'harness declaration inventory failed' 'injected find failure' check_harness_declarations "$fx/src"
  injected_find_output="$fx/src/store/partial_tests.rs"
  expect_refuse_naming_without 'partial harness output is not interpreted' 'harness declaration inventory failed' "$fx/src/store/partial_tests.rs" check_harness_declarations "$fx/src"
  conditional_harness_inventory() {
    if check_harness_declarations "$1"; then : > "$2"; fi
  }
  rm -f "$root/harness-marker"
  expect_refuse_naming 'conditional harness find failure still refuses' 'harness declaration inventory failed' conditional_harness_inventory "$fx/src" "$root/harness-marker"
  expect_eq 'conditional harness refusal has no post-check marker' "$( [ -e "$root/harness-marker" ] && echo present || echo absent )" absent

  unset -f find
  injected_find_output=""
  injected_find_status=0
  find() {
    printf '%s\n' "$root/partial_tests.rs"
    return 0
  }
  sort() {
    cat >/dev/null
    printf 'injected sort failure\n' >&2
    return 43
  }
  expect_refuse_naming 'harness sort failure has a named refusal' 'harness declaration inventory failed' check_harness_declarations "$fx/src"
  expect_refuse_naming_context 'harness sort diagnostics are preserved' 'harness declaration inventory failed' 'injected sort failure' check_harness_declarations "$fx/src"
  unset -f find sort

  # 4. Production source parked in a harness directory refuses.
  fx="$root/hidden"; mkdir -p "$fx/src/tests"
  printf 'fn prod() {}\n' > "$fx/src/tests/prod.rs"
  expect_refuse_naming_context 'production source under src/tests/ is refused' 'production source under a harness directory' "$fx/src/tests/prod.rs" check_no_hidden_harness_dirs "$fx/src"
  fx="$root/visible"; mkdir -p "$fx/src"
  printf 'fn prod() {}\n' > "$fx/src/prod.rs"
  expect_pass 'production source on a measured path passes' check_no_hidden_harness_dirs "$fx/src"

  # 5. Harness declarations (decision 0020). Each arm is a fresh fixture
  #    tree; the failure direction is refuse.
  fx="$root/decl"; mkdir -p "$fx/src/store"
  printf 'fn persisted() {}\n' > "$fx/src/store/persist_tests.rs"

  printf 'mod persist_tests;\n' > "$fx/src/store/mod.rs"
  expect_refuse_naming_context 'an unguarded harness declaration is refused' 'harness-named file(s) under src/ must be declared by their parent module under #[cfg(test)]' 'persist_tests.rs: declared without #[cfg(test)]' check_harness_declarations "$fx/src"

  printf '#[cfg(test)]\nmod persist_tests;\n' > "$fx/src/store/mod.rs"
  expect_pass 'a guarded harness declaration passes' check_harness_declarations "$fx/src"

  printf '// the persistence unit tests\n#[cfg(test)]\n\n/* measured, not excluded */\nmod persist_tests;\n' > "$fx/src/store/mod.rs"
  expect_pass 'blank and comment lines may intervene in the guard stack' check_harness_declarations "$fx/src"

  printf '#[cfg(test)] mod persist_tests;\n' > "$fx/src/store/mod.rs"
  expect_refuse_naming_context 'a same-line attribute is not a stack guard' 'harness-named file(s) under src/ must be declared by their parent module under #[cfg(test)]' 'persist_tests.rs: not declared by any parent module' check_harness_declarations "$fx/src"

  printf '#[cfg(test)]\n#[path = "elsewhere.rs"]\nmod persist_tests;\n' > "$fx/src/store/mod.rs"
  expect_refuse_naming_context 'a #[path] harness declaration is refused' 'harness-named file(s) under src/ must be declared by their parent module under #[cfg(test)]' 'declared via #[path]' check_harness_declarations "$fx/src"

  printf 'fn persisted() {}\n' > "$fx/src/store/persist_tests.rs"
  printf 'fn other() {}\n' > "$fx/src/store.rs"
  printf '#[cfg(test)]\nmod persist_tests;\n' > "$fx/src/store.rs"
  printf 'mod persist_tests;\n' > "$fx/src/store/mod.rs"
  expect_refuse_naming_context 'one guarded declaration cannot launder an unguarded one' 'harness-named file(s) under src/ must be declared by their parent module under #[cfg(test)]' 'persist_tests.rs: declared without #[cfg(test)]' check_harness_declarations "$fx/src"

  printf 'fn other() {}\n' > "$fx/src/store.rs"
  rm "$fx/src/store/mod.rs"
  expect_refuse_naming_context 'a harness file with no declaration at all is refused' 'harness-named file(s) under src/ must be declared by their parent module under #[cfg(test)]' 'persist_tests.rs: not declared by any parent module' check_harness_declarations "$fx/src"

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

  # 7. Build-target reservation: explicit targets require a fresh leaf,
  #    survive cleanup, and arbitrate cooperating callers with atomic mkdir.
  local resolved explicit target_statuses
  local explicit="$root/explicit parent/target with spaces"
  resolved="$(CARGO_LLVM_COV_TARGET_DIR="$explicit" resolve_build_dir; printf '%s %s' "$BUILD_DIR_OWNED" "$RESOLVED_BUILD_DIR")"
  expect_eq 'a fresh explicit target is reserved at the exact path and caller-owned' "$resolved" "0 $explicit"
  expect_eq 'explicit parent directories are created' "$( [ -d "$root/explicit parent" ] && echo yes || echo no )" yes
  local trailing_explicit="$root/trailing/leaf/"
  resolved="$(CARGO_LLVM_COV_TARGET_DIR="$trailing_explicit" resolve_build_dir; printf '%s %s' "$BUILD_DIR_OWNED" "$RESOLVED_BUILD_DIR")"
  expect_eq 'a fresh explicit target with a trailing slash is reserved exactly' "$resolved" "0 $trailing_explicit"
  cleanup_build_dir "$trailing_explicit" 0

  cleanup_build_dir "$explicit" 0
  expect_eq 'explicit-target cleanup leaves the reservation intact' "$( [ -d "$explicit" ] && echo yes || echo no )" yes

  resolve_explicit_target() { CARGO_LLVM_COV_TARGET_DIR="$1" resolve_build_dir; }
  mkdir -p "$root/existing-empty"
  expect_refuse_naming 'an existing empty explicit target refuses' 'fresh, unused leaf' resolve_explicit_target "$root/existing-empty"
  mkdir -p "$root/existing-dirty"
  printf 'keep me\n' > "$root/existing-dirty/sentinel"
  expect_refuse_naming 'an existing dirty explicit target refuses' 'fresh, unused leaf' resolve_explicit_target "$root/existing-dirty"
  expect_eq 'a dirty explicit target is left unchanged' "$(cat "$root/existing-dirty/sentinel")" 'keep me'
  printf 'keep me\n' > "$root/existing-file"
  expect_refuse_naming 'a regular-file explicit target refuses' 'fresh, unused leaf' resolve_explicit_target "$root/existing-file"
  mkdir -p "$root/symlink-referent"
  printf 'preserve referent\n' > "$root/symlink-referent/sentinel"
  ln -s "$root/symlink-referent" "$root/existing-symlink"
  expect_refuse_naming 'a symlink explicit target refuses' 'fresh, unused leaf' resolve_explicit_target "$root/existing-symlink"
  expect_eq 'a symlink referent is preserved' "$(cat "$root/symlink-referent/sentinel")" 'preserve referent'
  ln -s "$root/no-such-target" "$root/dangling-symlink"
  expect_refuse_naming 'a dangling symlink explicit target refuses' 'fresh, unused leaf' resolve_explicit_target "$root/dangling-symlink"
  expect_eq 'a dangling symlink remains present' "$( [ -L "$root/dangling-symlink" ] && echo yes || echo no )" yes

  printf 'parent blocker\n' > "$root/blocked-parent"
  expect_refuse_naming 'an uncreatable explicit parent has a named refusal' 'explicit target parent could not be created' resolve_explicit_target "$root/blocked-parent/child"

  mkdir() {
    if [ "$#" -eq 2 ] && [ "$1" = -- ] && [ "$2" = "$root/forced-leaf-failure" ]; then
      printf 'injected leaf mkdir failure\n' >&2
      return 42
    fi
    command mkdir "$@"
  }
  expect_refuse_naming 'an explicit leaf creation error has a named refusal' 'fresh, unused leaf' resolve_explicit_target "$root/forced-leaf-failure"
  unset -f mkdir

  local contended="$root/concurrent-target"
  local start_one="$root/start-one" start_two="$root/start-two"
  mkfifo "$start_one" "$start_two"
  (
    IFS= read -r _ < "$start_one"
    if (
      CARGO_LLVM_COV_TARGET_DIR="$contended" resolve_build_dir
    ) >"$root/one.out" 2>&1; then
      printf '0\n' > "$root/one.status"
    else
      printf '%s\n' "$?" > "$root/one.status"
    fi
  ) &
  local one_pid=$!
  (
    IFS= read -r _ < "$start_two"
    if (
      CARGO_LLVM_COV_TARGET_DIR="$contended" resolve_build_dir
    ) >"$root/two.out" 2>&1; then
      printf '0\n' > "$root/two.status"
    else
      printf '%s\n' "$?" > "$root/two.status"
    fi
  ) &
  local two_pid=$!
  exec 3>"$start_one"
  exec 4>"$start_two"
  printf 'go\n' >&3
  printf 'go\n' >&4
  exec 3>&-
  exec 4>&-
  wait "$one_pid"
  wait "$two_pid"
  target_statuses="$(printf '%s\n' "$(cat "$root/one.status")" "$(cat "$root/two.status")" | sort | tr '\n' ' ')"
  expect_eq 'two simultaneous explicit reservations have one winner' "$target_statuses" '0 1 '
  expect_pass 'the concurrent loser receives the named reservation refusal' bash -c 'grep -q "fresh, unused leaf" "$1" "$2"' _ "$root/one.out" "$root/two.out"
  expect_refuse_naming 'a retry of a retained explicit target refuses' 'fresh, unused leaf' resolve_explicit_target "$contended"

  mkdir -p "$root/tmpdir"
  resolved="$(unset CARGO_LLVM_COV_TARGET_DIR; TMPDIR="$root/tmpdir" resolve_build_dir; printf '%s %s' "$BUILD_DIR_OWNED" "$RESOLVED_BUILD_DIR")"
  case "$resolved" in
    "1 $PWD/target/coverage/build."*)
      resolved="${resolved#1 }"
      expect_eq 'the default build dir is created' "$( [ -d "$resolved" ] && echo yes || echo no )" yes
      cleanup_build_dir "$resolved" 1
      expect_eq 'default cleanup removes its generated build dir' "$( [ -e "$resolved" ] && echo yes || echo no )" no
      ;;
    *) expect_eq 'the default build dir is owned and under target/coverage/, never TMPDIR' "$resolved" "1 $PWD/target/coverage/build.XXXXXX" ;;
  esac
  resolved="$(CARGO_LLVM_COV_TARGET_DIR="" TMPDIR="$root/tmpdir" resolve_build_dir; printf '%s %s' "$BUILD_DIR_OWNED" "$RESOLVED_BUILD_DIR")"
  case "$resolved" in
    "1 $PWD/target/coverage/build."*)
      resolved="${resolved#1 }"
      cleanup_build_dir "$resolved" 1
      ;;
    *) expect_eq 'empty explicit selection uses a fresh default target' "$resolved" "1 $PWD/target/coverage/build.XXXXXX" ;;
  esac

  # 8. The free-space preflight refuses before the build: a floor larger
  #    than the filesystem refuses naming the path, the filesystem and the
  #    escape variable; an unstatable path refuses naming the escape; a
  #    floor the filesystem holds passes.
  local fsname
  fsname="$(df -Pk "$root" | awk 'NR == 2 { print $1 }')"
  expect_refuse_naming_context 'a short filesystem refusal names the path' 'KiB free on filesystem' "$root has" check_free_space "$root" 999999999999
  expect_refuse_naming_context 'a short filesystem refusal names the filesystem' 'KiB free on filesystem' "filesystem $fsname" check_free_space "$root" 999999999999
  expect_refuse_naming_context 'a short filesystem refusal names the escape variable' 'KiB free on filesystem' 'CARGO_LLVM_COV_TARGET_DIR' check_free_space "$root" 999999999999
  expect_refuse_naming_context 'an unstatable path names the stat refusal and escape variable' 'free-space preflight could not stat' 'CARGO_LLVM_COV_TARGET_DIR' check_free_space "$root/absent/dir" 1
  mkdir -p "$root/non-numeric-df"
  non_numeric_free_space() {
    local fixture_path="$1" min_kib="$2"
    df() {
      if [ "$#" -ne 2 ] || [ "$1" != -Pk ] || [ "$2" != "$fixture_path" ]; then
        printf 'unexpected df invocation\n' >&2
        return 97
      fi
      printf 'Filesystem 1024-blocks Used Available Capacity Mounted on\nfixture-fs 100000 1000 unavailable 1%% %s\n' "$fixture_path"
    }
    check_free_space "$fixture_path" "$min_kib"
  }
  expect_refuse_naming_context 'successful df with non-numeric availability refuses by parse reason' 'could not read the free space of' "$root/non-numeric-df" non_numeric_free_space "$root/non-numeric-df" 1
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

  # The production clean boundary must refuse in normal and conditional
  # callers, preserve diagnostics, and prevent every later-stage marker.
  local clean_mode=fail
  cargo() {
    case "$clean_mode" in
      fail)
        printf 'injected llvm-cov clean failure\n' >&2
        return 42
        ;;
      pass)
        printf 'injected llvm-cov clean success\n'
        return 0
        ;;
      *)
        printf 'unexpected clean mode\n' >&2
        return 44
        ;;
    esac
  }
  expect_refuse_naming 'llvm-cov clean failure has a named refusal' 'llvm-cov clean failed' run_llvm_cov_clean "$root/clean-failure.log"
  expect_refuse_naming_context 'llvm-cov clean failure preserves its diagnostic' 'llvm-cov clean failed' 'injected llvm-cov clean failure' run_llvm_cov_clean "$root/clean-failure.log"
  conditional_clean() {
    if run_llvm_cov_clean "$1"; then : > "$2"; fi
  }
  rm -f "$root/clean-marker"
  expect_refuse_naming 'conditional llvm-cov clean failure still refuses' 'llvm-cov clean failed' conditional_clean "$root/conditional-clean-failure.log" "$root/clean-marker"
  expect_eq 'failed clean has no post-clean measurement marker' "$( [ -e "$root/clean-marker" ] && echo present || echo absent )" absent
  expect_refuse_naming 'clean log creation failure has a named refusal' 'llvm-cov clean failed' run_llvm_cov_clean "$root/missing-parent/clean.log"
  clean_mode=pass
  rm -f "$root/clean-marker"
  expect_pass 'successful clean reaches the next production stage' conditional_clean "$root/clean-success.log" "$root/clean-marker"
  expect_eq 'successful clean writes the post-clean marker' "$( [ -e "$root/clean-marker" ] && echo present || echo absent )" present
  unset -f cargo

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
    expect_refuse_naming_context "stage 1 refuses the ${arm#*:} harness load" 'production file(s) the build compiled are absent from the report' "${arm%%:*}" check_report_covers_production_files "$root/bypass.info" "$root/bypass-prod.txt" "$fxb"
  done
  for arm in 'direct_hidden:direct' 'macro_hidden:macro-emitted' 'attr_hidden:cfg_attr-resolved' 'cfg_gated_hidden:coverage-cfg-gated' 'derived_shadow_hidden:derive-shadowed'; do
    expect_refuse_naming_context "stage 2 refuses the ${arm#*:} exclusion" 'coverage exclusion survived into the canonical expansion' "on fn ${arm%%:*}" check_no_coverage_exclusions "$root/bypass-exp.rs"
  done
  # The isolated inner-only fixture below owns this assertion; the aggregate dump has other offenders.

  expect_pass 'stage 1 passes the green twin (declarations-only file, asset, derive, const fn)' check_report_covers_production_files "$root/green.info" "$root/green-prod.txt" "$fxg"
  # Before scanner success can stand as proof of the derive carve-out, inspect
  # the real stage-2 dump for the compiler-minted method exclusion inside its
  # automatically_derived Eq impl. The association is local to that impl.
  if ! awk '
    function brace_delta(line, copy, opens, closes) {
      copy = line; opens = gsub(/\{/, "{", copy)
      copy = line; closes = gsub(/\}/, "}", copy)
      return opens - closes
    }
    {
      if ($0 ~ /^[[:space:]]*#\[automatically_derived\][[:space:]]*$/) {
        auto_pending = 1
        next
      }
      if (auto_pending && $0 ~ /^[[:space:]]*#\[/) next
      if (auto_pending && $0 ~ /^[[:space:]]*impl .*::core::cmp::Eq/) {
        in_eq = 1
        auto_pending = 0
      } else if (auto_pending && $0 !~ /^[[:space:]]*$/) {
        auto_pending = 0
      }
      if (in_eq) {
        if ($0 ~ /#\[coverage\(off\)\]/) coverage_pending = 1
        if ($0 ~ /fn assert_fields_are_eq[[:space:]]*\(/ && coverage_pending) method_is_covered = 1
        else if (coverage_pending && $0 !~ /^[[:space:]]*$/ && $0 !~ /^[[:space:]]*#\[/) coverage_pending = 0
        depth += brace_delta($0)
        if (depth == 0) in_eq = 0
      }
    }
    END { exit !(method_is_covered) }
  ' "$root/green-exp.rs"; then
    printf 'coverage gate: selftest failure — green stage-2 dump lacks coverage(off) on assert_fields_are_eq inside its automatically_derived Eq impl; expansion context:\n' >&2
    rg -n -C 5 'automatically_derived|assert_fields_are_eq|coverage\(off\)' "$root/green-exp.rs" >&2 || true
    failures=$((failures + 1))
  fi
  expect_pass 'stage 2 passes the green twin (the minted exclusion carved, the const fn silent)' check_no_coverage_exclusions "$root/green-exp.rs"

  # 10. Fast arms: validate-before-consume, the leak check's membership axis,
  #     a hostile dep-info entry, a dep-info-less dir, an empty expansion.
  : >"$root/empty.info"
  expect_refuse_naming_context 'an empty report refuses naming the artifact' 'missing, empty or carries no SF: records' "$root/empty.info" validate_report "$root/empty.info"
  printf 'DA:1,1\nend_of_record\n' >"$root/sfless.info"
  expect_refuse_naming 'an SF-less report refuses naming the artifact' 'no SF:' validate_report "$root/sfless.info"
  expect_pass 'a real report passes validation' validate_report "$root/bypass.info"
  printf 'SF:%s/src/inc_tests.rs\nDA:1,1\nend_of_record\n' "$fxb" >"$root/measured.info"
  printf 'src/inc_tests.rs\n' >"$root/prod-set.txt"
  expect_pass 'a harness-named SF record inside the production set passes' \
    check_no_harness_leak "$root/measured.info" "$root/prod-set.txt" "$fxb"
  printf 'SF:%s/src/leaked_tests.rs\nDA:1,1\nend_of_record\n' "$fxb" >"$root/unmeasured.info"
  expect_refuse_naming_context 'a harness-named SF record outside the production set refuses' \
    'test harness source measured as production while absent from the production build file set' 'leaked_tests.rs' \
    check_no_harness_leak "$root/unmeasured.info" "$root/prod-set.txt" "$fxb"
  mkdir -p "$root/hostile" "$root/empty-target"
  sed 's|src/lib\.rs|src/vanished.rs|g' "$root/bypass-depinfo/"*.d >"$root/hostile/lib.d"
  expect_refuse_naming_context 'a hostile dep-info entry refuses naming it' 'does not resolve to an existing regular file under' 'src/vanished.rs' \
    "$FN_SCAN_BIN" files --workspace "$fxb" --build-dir "$root/hostile" --package fxbypass
  expect_refuse_naming_context 'a harvest over a dep-info-less dir refuses naming the path' 'no dep-info (.d) file under' 'empty-target' \
    "$FN_SCAN_BIN" files --workspace "$fxb" --build-dir "$root/empty-target" --package fxbypass
  : >"$root/empty-exp.rs"
  expect_refuse_naming_context 'an empty expansion refuses naming the artifact' 'is missing or empty' "$root/empty-exp.rs" \
    check_no_coverage_exclusions "$root/empty-exp.rs"

  # 11. #140's remaining bypasses, each a REAL cargo fixture with its clean
  #     twin (decision 0050 §2). mkfx <name> <lib.rs lines...> writes a
  #     dependency-free crate; expand <dir> <out> <cargo args> [-- rustc args]
  #     dumps one unit's canonical expansion with the gate's nightly.
  mkfx() {
    local dir="$root/$1"; shift
    mkdir -p "$dir/src"
    printf '[package]\nname = "%s"\nversion = "0.1.0"\nedition = "2021"\n' "$(basename "$dir" | tr -c 'a-z0-9\n' '_')" >"$dir/Cargo.toml"
    printf '%s\n' "$@" >"$dir/src/lib.rs"
  }
  expand() {
    local dir="$1" out="$2"; shift 2
    fx_cmd "$dir" "$(basename "$out").log" "the $(basename "$dir") expansion" "$out" \
      cargo +"$coverage_toolchain" rustc --offline --target-dir "$dir/target" "$@"
  }
  # A real derived impl must finish before a later author exclusion. Otherwise
  # a broad derive-state carry can wrongly carve the independent offender.
  mkfx derive-order-red '#![feature(coverage_attribute)]' '#![allow(dead_code)]' \
    '#[derive(PartialEq, Eq)]' 'struct Derived { value: u8 }' \
    '#[coverage(off)]' 'fn post_derive_hidden() {}'
  mkfx derive-order-green '#![allow(dead_code)]' \
    '#[derive(PartialEq, Eq)]' 'struct Derived { value: u8 }' 'fn post_derive_clean() {}'
  for fixture in derive-order-red derive-order-green; do
    fx_cmd "$root/$fixture" "$fixture-lock.log" "the $fixture offline lock" "$root/$fixture-lock.out" \
      cargo +"$coverage_toolchain" generate-lockfile --offline
  done
  for fixture in derive-order-red derive-order-green; do
    fx_cmd "$root/$fixture" "$fixture-exp.log" "the $fixture canonical expansion" "$root/$fixture-exp.rs" \
      cargo +"$coverage_toolchain" rustc --lib --offline --locked --target-dir "$root/$fixture-target" -- \
        -Zunpretty=expanded --cfg coverage --cfg coverage_nightly
  done
  if ! awk '
    function brace_delta(line, copy, opens, closes) {
      copy = line; opens = gsub(/\{/, "{", copy)
      copy = line; closes = gsub(/\}/, "}", copy)
      return opens - closes
    }
    {
      if ($0 ~ /^[[:space:]]*#\[automatically_derived\][[:space:]]*$/) {
        auto_pending = 1
        next
      }
      if (auto_pending && $0 ~ /^[[:space:]]*#\[/) next
      if (auto_pending && $0 ~ /^[[:space:]]*impl .*PartialEq/) {
        in_derived_impl = 1
        derived_impl_seen = 1
        auto_pending = 0
      } else if (auto_pending && $0 !~ /^[[:space:]]*$/) {
        auto_pending = 0
      }
      if (in_derived_impl) {
        depth += brace_delta($0)
        if (depth == 0) {
          derived_impl_end = NR
          in_derived_impl = 0
        }
      }
      if ($0 ~ /fn post_derive_hidden[[:space:]]*\(/) offender_line = NR
    }
    END { exit !(derived_impl_seen && derived_impl_end && offender_line > derived_impl_end) }
  ' "$root/derive-order-red-exp.rs"; then
    printf 'coverage gate: selftest failure — derive-order red expansion does not show a closed compiler-generated PartialEq impl before post_derive_hidden; expansion context:\n' >&2
    rg -n -C 5 'automatically_derived|PartialEq|post_derive_hidden' "$root/derive-order-red-exp.rs" >&2 || true
    failures=$((failures + 1))
  fi
  expect_refuse_naming_context 'stage 2 catches an offender after the derived impl' 'coverage exclusion survived into the canonical expansion' 'on fn post_derive_hidden' \
    check_no_coverage_exclusions "$root/derive-order-red-exp.rs"
  expect_pass 'the same derive and post-derive function pass without an author exclusion' \
    check_no_coverage_exclusions "$root/derive-order-green-exp.rs"

  # Binary-only red/green fixtures exercise the independent bin dep-info path.
  # Integration tests spawn their built executable so the ordinary included
  # function contributes real report records.
  local fxbr="$root/fx-bin-red" fxbg="$root/fx-bin-green" fxbrt="$root/fx-bin-red-target" fxbgt="$root/fx-bin-green-target"
  for fixture in "$fxbr" "$fxbg"; do
    mkdir -p "$fixture/src" "$fixture/tests"
  done
  printf '[package]\nname = "fxbin_red"\nversion = "0.1.0"\nedition = "2021"\n[[bin]]\nname = "fxbin_red"\npath = "src/main.rs"\n' >"$fxbr/Cargo.toml"
  printf '#![feature(coverage_attribute)]\n#![allow(dead_code)]\ninclude!("main_tests.rs");\nfn main() { println!("{}", red_value()); }\n' >"$fxbr/src/main.rs"
  printf 'fn red_value() -> &'"'"'static str { "red" }\n#[coverage(off)]\npub fn bin_only_hidden() {}\n' >"$fxbr/src/main_tests.rs"
  printf '[package]\nname = "fxbin_green"\nversion = "0.1.0"\nedition = "2021"\n[[bin]]\nname = "fxbin_green"\npath = "src/main.rs"\n' >"$fxbg/Cargo.toml"
  printf '#![allow(dead_code)]\ninclude!("support.rs");\nfn main() { println!("{}", green_value()); }\n' >"$fxbg/src/main.rs"
  printf 'fn green_value() -> &'"'"'static str { "green" }\n' >"$fxbg/src/support.rs"
  printf '#[test]\nfn executes_the_binary() {\n    let output = std::process::Command::new(env!("CARGO_BIN_EXE_fxbin_red")).output().unwrap();\n    assert!(output.status.success());\n    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "red");\n}\n' >"$fxbr/tests/exec_bin.rs"
  printf '#[test]\nfn executes_the_binary() {\n    let output = std::process::Command::new(env!("CARGO_BIN_EXE_fxbin_green")).output().unwrap();\n    assert!(output.status.success());\n    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "green");\n}\n' >"$fxbg/tests/exec_bin.rs"
  for fx in "$fxbr" "$fxbg"; do
    fx_cmd "$fx" "$(basename "$fx")-lock.log" "the $(basename "$fx") offline lock" "$root/$(basename "$fx")-lock.out" \
      cargo +"$coverage_toolchain" generate-lockfile --offline
  done
  for spec in "fxbin_red:$fxbr:$fxbrt:red" "fxbin_green:$fxbg:$fxbgt:green"; do
    IFS=: read -r bin_name fixture target color <<<"$spec"
    fx_cmd "$fixture" "$bin_name-build.log" "the $bin_name executable build" "$root/$bin_name-build.out" \
      cargo +"$coverage_toolchain" build --bin "$bin_name" --offline --locked --target-dir "$target"
    fx_cmd "$fixture" "$bin_name-test.log" "the $bin_name executable integration test" "$root/$bin_name-test.out" \
      cargo +"$coverage_toolchain" test --test exec_bin --offline --locked --target-dir "$target"
    fx_cmd "$fixture" "$bin_name-cov.log" "the $bin_name coverage report" "$root/$bin_name-cov.out" \
      env CARGO_LLVM_COV_TARGET_DIR="$target" cargo +"$coverage_toolchain" llvm-cov --lcov --offline --locked --output-path "$root/$bin_name.info"
    fx_cmd "$root" "$bin_name-harvest.log" "the $bin_name bin harvest" "$root/$bin_name-prod.txt" \
      "$FN_SCAN_BIN" files --workspace "$fixture" --build-dir "$target" --package "$bin_name" \
      --bin-root=src/main.rs --capture "$root/$bin_name-depinfo"
    grep -Fxq 'src/main.rs' "$root/$bin_name-prod.txt" || {
      printf 'coverage gate: selftest failure — %s harvest omitted bin root src/main.rs\n' "$bin_name" >&2
      failures=$((failures + 1))
    }
    if [ "$color" = green ]; then
      grep -Fxq 'src/support.rs' "$root/$bin_name-prod.txt" || {
        printf 'coverage gate: selftest failure — green bin harvest omitted src/support.rs\n' >&2
        failures=$((failures + 1))
      }
      grep -Fq "$fixture/src/main.rs" "$root/$bin_name.info" || {
        printf 'coverage gate: selftest failure — green bin report omitted src/main.rs\n' >&2
        failures=$((failures + 1))
      }
      grep -Fq "$fixture/src/support.rs" "$root/$bin_name.info" || {
        printf 'coverage gate: selftest failure — green bin report omitted src/support.rs\n' >&2
        failures=$((failures + 1))
      }
    else
      grep -Fxq 'src/main_tests.rs' "$root/$bin_name-prod.txt" || {
        printf 'coverage gate: selftest failure — red bin harvest omitted src/main_tests.rs\n' >&2
        failures=$((failures + 1))
      }
    fi
    fx_cmd "$fixture" "$bin_name-exp.log" "the $bin_name bin expansion" "$root/$bin_name-exp.rs" \
      cargo +"$coverage_toolchain" rustc --bin "$bin_name" --offline --locked --target-dir "$target" -- \
        -Zunpretty=expanded --cfg coverage --cfg coverage_nightly
  done
  expect_refuse_naming_context 'binary stage 1 refuses the red harness-named include' \
    'production file(s) the build compiled are absent from the report' 'src/main_tests.rs' \
    check_report_covers_production_files "$root/fxbin_red.info" "$root/fxbin_red-prod.txt" "$fxbr"
  expect_refuse_naming_context 'binary stage 2 refuses the bin-only exclusion' \
    'coverage exclusion survived into the canonical expansion' 'on fn bin_only_hidden' \
    check_no_coverage_exclusions "$root/fxbin_red-exp.rs"
  expect_pass 'the green binary production set is covered by its report' \
    check_report_covers_production_files "$root/fxbin_green.info" "$root/fxbin_green-prod.txt" "$fxbg"
  expect_pass 'the green binary expansion has no surviving exclusion' \
    check_no_coverage_exclusions "$root/fxbin_green-exp.rs"

  # v1: a three-arm function only where coverage is off, a stub where it is on.
  mkfx v1-cfg-not-coverage '#[cfg(not(coverage))]' 'pub fn sign(x: i32) -> i32 { if x > 0 { 1 } else if x < 0 { -1 } else { 0 } }' \
    '#[cfg(coverage)]' 'pub fn sign(_x: i32) -> i32 { 0 }'
  expand "$root/v1-cfg-not-coverage" "$root/v1-measured.rs" --lib -- -Zunpretty=expanded --cfg coverage --cfg coverage_nightly
  expand "$root/v1-cfg-not-coverage" "$root/v1-plain.rs" --lib -- -Zunpretty=expanded
  expect_refuse_naming 'v1-cfg-not-coverage: code only without the coverage cfg refuses' 'no-coverage-cfg expansion differs' \
    check_expansions_equivalent "$root/v1-measured.rs" "$root/v1-plain.rs" 'v1 no-coverage-cfg'
  # v2: an item only the release profile compiles.
  mkfx v2-release-only 'pub fn shipped() -> i32 { 1 }' '#[cfg(not(debug_assertions))]' 'pub fn release_only(x: i32) -> i32 { if x > 0 { x } else { -x } }'
  expand "$root/v2-release-only" "$root/v2-measured.rs" --lib -- -Zunpretty=expanded --cfg coverage --cfg coverage_nightly
  expand "$root/v2-release-only" "$root/v2-release.rs" --lib --release -- -Zunpretty=expanded
  expect_refuse_naming 'v2-release-only: code only in the release profile refuses' 'release-profile expansion differs' \
    check_expansions_equivalent "$root/v2-measured.rs" "$root/v2-release.rs" 'v2 release-profile' release
  # The clean twin: a debug_assert! (present, measured, in both profiles) and
  # no cfg-dependent item pass both comparisons.
  mkfx v2-green 'pub fn checked(x: i32) -> i32 { debug_assert!(x >= 0, "nonnegative"); x }'
  expand "$root/v2-green" "$root/v2g-measured.rs" --lib -- -Zunpretty=expanded --cfg coverage --cfg coverage_nightly
  expand "$root/v2-green" "$root/v2g-plain.rs" --lib -- -Zunpretty=expanded
  expand "$root/v2-green" "$root/v2g-release.rs" --lib --release -- -Zunpretty=expanded
  expect_pass 'the clean twin passes the no-coverage-cfg comparison' check_expansions_equivalent "$root/v2g-measured.rs" "$root/v2g-plain.rs" 'v2-green no-coverage-cfg'
  expect_pass 'the clean twin (debug_assert!) passes the release comparison' check_expansions_equivalent "$root/v2g-measured.rs" "$root/v2g-release.rs" 'v2-green release-profile' release
  # v4: a configuration-dependent boolean that is no debug_assert! residue.
  # Both arms of the old comparator's global true->false rewrite admitted it:
  # the measured build returns true, the shipped one false, and every line
  # is covered. A cfg!(coverage) value refuses the no-coverage-cfg
  # comparison, a cfg!(debug_assertions) value the release one.
  mkfx v4-coverage-bool 'pub fn measured() -> bool { cfg!(coverage) }'
  expand "$root/v4-coverage-bool" "$root/v4-measured.rs" --lib -- -Zunpretty=expanded --cfg coverage --cfg coverage_nightly
  expand "$root/v4-coverage-bool" "$root/v4-plain.rs" --lib -- -Zunpretty=expanded
  expect_refuse_naming 'v4-coverage-bool: a cfg!(coverage) value refuses the no-coverage-cfg comparison' 'no-coverage-cfg expansion differs' \
    check_expansions_equivalent "$root/v4-measured.rs" "$root/v4-plain.rs" 'v4 no-coverage-cfg'
  mkfx v4b-debug-bool 'pub fn debug() -> bool { cfg!(debug_assertions) }'
  expand "$root/v4b-debug-bool" "$root/v4b-measured.rs" --lib -- -Zunpretty=expanded --cfg coverage --cfg coverage_nightly
  expand "$root/v4b-debug-bool" "$root/v4b-release.rs" --lib --release -- -Zunpretty=expanded
  expect_refuse_naming 'v4b-debug-bool: a cfg!(debug_assertions) value refuses the release comparison' 'release-profile expansion differs' \
    check_expansions_equivalent "$root/v4b-measured.rs" "$root/v4b-release.rs" 'v4b release-profile' release
  # v3/v3b: production logic in a path dependency (under benches/, under
  # crates/); v5b: a workspace member `aaa` sorting before the root, beside
  # #101 D's include!. Each refuses by the package inventory, naming it; a
  # standalone tool crate (its own [workspace], like the pinned scanner)
  # beside an ordinary root passes and the root is chosen by manifest path.
  inventory() { # $1: fixture dir — the inventory the gate itself computes.
    (cd "$1" && cargo +"$coverage_toolchain" generate-lockfile --offline >/dev/null 2>&1 &&
      cargo +"$coverage_toolchain" metadata --format-version 1 --offline --locked >"$1/metadata.json" 2>/dev/null) ||
      { printf 'coverage gate: selftest failure — the %s metadata could not be read\n' "$1" >&2; exit 1; }
    check_package_inventory "$1/metadata.json" "$(realpath "$1/Cargo.toml")"
  }
  local dep
  for dep in 'v3-dep-benches:benches/logic' 'v3b-dep-neutral:crates/neutral'; do
    fx="$root/${dep%%:*}"
    mkfx "${dep%%:*}" 'pub fn root() -> i32 { logic::hidden() }'
    printf '\n[dependencies]\nlogic = { path = "%s" }\n' "${dep#*:}" >>"$fx/Cargo.toml"
    mkdir -p "$fx/${dep#*:}/src"
    printf '[package]\nname = "logic"\nversion = "0.1.0"\nedition = "2021"\n' >"$fx/${dep#*:}/Cargo.toml"
    printf 'pub fn hidden() -> i32 { if 1 > 0 { 1 } else { 0 } }\n' >"$fx/${dep#*:}/src/lib.rs"
    expect_refuse_naming "${dep%%:*}: a local path dependency refuses naming it" 'local package logic' inventory "$fx"
  done
  mkfx v5b-ws-member-dep 'include!("inc_tests.rs");' 'pub fn visible() -> i32 { 1 }'
  printf 'pub fn inc_hidden() {}\n' >"$root/v5b-ws-member-dep/src/inc_tests.rs"
  printf '\n[workspace]\nmembers = ["aaa"]\n' >>"$root/v5b-ws-member-dep/Cargo.toml"
  mkdir -p "$root/v5b-ws-member-dep/aaa/src"
  printf '[package]\nname = "aaa"\nversion = "0.1.0"\nedition = "2021"\n' >"$root/v5b-ws-member-dep/aaa/Cargo.toml"
  printf 'pub fn a() {}\n' >"$root/v5b-ws-member-dep/aaa/src/lib.rs"
  expect_refuse_naming 'v5b-ws-member-dep: a workspace member sorting first refuses naming it' 'workspace member aaa' inventory "$root/v5b-ws-member-dep"
  mkfx inv-green 'pub fn visible() -> i32 { 1 }'
  mkdir -p "$root/inv-green/aaa-tool/src"
  printf '[package]\nname = "aaa-tool"\nversion = "0.1.0"\nedition = "2021"\n\n[workspace]\n' >"$root/inv-green/aaa-tool/Cargo.toml"
  printf 'fn main() {}\n' >"$root/inv-green/aaa-tool/src/main.rs"
  expect_eq 'a root beside a standalone tool crate is chosen by manifest path' "$(inventory "$root/inv-green" 2>&1)" inv_green
  # v6: coverage( off ) inside #[cfg(test)] — refused by the any-spelling
  # static check and, by effect, by the test unit's expansion scan; the
  # same test module without it passes (rustc's own harness markers carved).
  mkfx v6-test-helper-off '#![cfg_attr(test, feature(coverage_attribute))]' 'pub fn visible() -> i32 { 1 }' '#[cfg(test)]' 'mod t {' \
    '    #[coverage( off )]' '    fn helper() -> i32 { if super::visible() > 0 { 1 } else { 0 } }' \
    '    #[test]' '    fn proves() { assert_eq!(helper(), 1); }' '}'
  expect_refuse_naming_context 'v6-test-helper-off: the spaced spelling refuses statically' 'attribute-based source exclusions are forbidden' 'src/lib.rs' check_no_coverage_off "$root/v6-test-helper-off/src"
  expand "$root/v6-test-helper-off" "$root/v6-test.rs" --lib --profile test -- -Zunpretty=expanded --cfg coverage --cfg coverage_nightly
  expect_refuse_naming_context 'v6-test-helper-off: the test-unit expansion scan refuses, naming the fn' 'coverage exclusion survived into the canonical expansion' 'on fn helper' check_test_unit_expansion "$root/v6-test.rs"
  mkfx v6-green 'pub fn visible() -> i32 { 1 }' '#[cfg(test)]' 'mod t {' '    #[test]' '    fn proves() { assert_eq!(super::visible(), 1); }' '}'
  expand "$root/v6-green" "$root/v6g-test.rs" --lib --profile test -- -Zunpretty=expanded --cfg coverage --cfg coverage_nightly
  expect_pass 'a clean test module passes the test-unit scan (harness carved)' check_test_unit_expansion "$root/v6g-test.rs"
  expand "$root/v6-green" "$root/v6g-lib.rs" --lib -- -Zunpretty=expanded --cfg coverage --cfg coverage_nightly
  expect_refuse_naming 'a non-test expansion is not accepted as the test unit' 'carries no test harness' check_test_unit_expansion "$root/v6g-lib.rs"
  # The inner-attribute arm on its own fixture: no other offender, so
  # removing the scanner's `#!` handling turns this arm red; its inner-allow
  # twin passes. And a `pub fn` exclusion names its function.
  mkfx raw-exclusions '#![feature(coverage_attribute)]' \
    '#[coverage(r#off)]' 'pub fn raw_argument_hidden() {}' \
    '#[r#coverage(off)]' 'pub fn raw_name_hidden() {}'
  cargo generate-lockfile --offline --manifest-path "$root/raw-exclusions/Cargo.toml" >/dev/null 2>&1
  fx_cmd "$root/raw-exclusions" raw-exclusions-codegen.log 'raw-identifier exclusion codegen' "$root/raw-exclusions-codegen.out" \
    cargo +"$coverage_toolchain" rustc --lib --offline --locked --target-dir "$root/raw-exclusions-target" -- --emit=llvm-ir
  expand "$root/raw-exclusions" "$root/raw-exclusions.rs" --lib -- -Zunpretty=expanded --cfg coverage --cfg coverage_nightly
  expect_refuse_naming_context 'the canonical scanner refuses raw-off' 'coverage exclusion survived into the canonical expansion' 'on fn raw_argument_hidden' check_no_coverage_exclusions "$root/raw-exclusions.rs"
  expect_refuse_naming_context 'the canonical scanner refuses a raw coverage name' 'coverage exclusion survived into the canonical expansion' 'on fn raw_name_hidden' check_no_coverage_exclusions "$root/raw-exclusions.rs"
  mkfx raw-exclusions-green '#![allow(dead_code)]' 'pub fn r#off() -> &'"'"'static str { "r#coverage(r#off)" }'
  expand "$root/raw-exclusions-green" "$root/raw-exclusions-green.rs" --lib -- -Zunpretty=expanded --cfg coverage --cfg coverage_nightly
  expect_pass 'raw identifiers in ordinary names and string literals are not exclusions' check_no_coverage_exclusions "$root/raw-exclusions-green.rs"

  mkfx inner-only '#![feature(coverage_attribute)]' 'pub fn inner_only() { #![coverage(off)] }'
  expand "$root/inner-only" "$root/inner-only.rs" --lib -- -Zunpretty=expanded --cfg coverage --cfg coverage_nightly
  expect_refuse_naming_context 'the inner-attribute exclusion refuses on its own fixture' 'coverage exclusion survived into the canonical expansion' 'coverage(off)' check_no_coverage_exclusions "$root/inner-only.rs"
  mkfx inner-allow 'pub fn inner_allowed() { #![allow(unused)] }'
  expand "$root/inner-allow" "$root/inner-allow.rs" --lib -- -Zunpretty=expanded --cfg coverage --cfg coverage_nightly
  expect_pass 'its inner-allow twin passes' check_no_coverage_exclusions "$root/inner-allow.rs"
  mkfx pub-fn '#![feature(coverage_attribute)]' '#[coverage(off)]' '#[inline]' 'pub(crate) const fn named_pub() -> i32 { 1 }'
  expand "$root/pub-fn" "$root/pub-fn.rs" --lib -- -Zunpretty=expanded --cfg coverage --cfg coverage_nightly
  expect_refuse_naming_context 'a pub(crate) const fn exclusion names its function' 'coverage exclusion survived into the canonical expansion' 'on fn named_pub' check_no_coverage_exclusions "$root/pub-fn.rs"

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
# The cleanup helper is ownership-guarded: explicit reservations survive
# normal and failing exits; only the generated default build tree is removed.
trap 'cleanup_build_dir "$forge_target_dir" "$BUILD_DIR_OWNED"' EXIT
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
# from an earlier source graph can participate in the merge. The checked
# boundary refuses before measurement even when called conditionally.
run_llvm_cov_clean "$run_dir/clean.log"

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
cargo metadata --format-version 1 --offline --locked >"$run_dir/metadata.json" 2>"$run_dir/metadata.log" ||
  refuse "coverage refusal: cargo metadata could not be read — the harvest needs the workspace units (log: $run_dir/metadata.log)"
root_manifest="$(realpath Cargo.toml)"
root_package="$(check_package_inventory "$run_dir/metadata.json" "$root_manifest")" || exit 1
root_query='.packages[] | select(.manifest_path == $m)'

mapfile -t bin_args < <(jq -r --arg m "$root_manifest" "$root_query"' | .targets[] | select(.kind | index("bin")) | "--bin-root=" + .src_path' "$run_dir/metadata.json")
if ! "$FN_SCAN_BIN" files --workspace "$PWD" --build-dir "$forge_target_dir" \
  --package "$root_package" "${bin_args[@]}" \
  --capture "$run_dir/dep-info" >"$run_dir/production-files.txt" 2>"$run_dir/harvest.log"; then
  tail -n 20 "$run_dir/harvest.log" >&2 || true
  refuse "coverage refusal: the production file harvest failed — its log is preserved at $run_dir/harvest.log (a missing dep-info names the path and the shared-directory contract)"
fi

# The canonical expansion of both production units, into the preserved run
# dir, with the measurement's toolchain and target dir: a cfg mismatch refuses
# loudly (an empty dump is the loudest mismatch there is).
expand_unit() {
  # $1: the unit selector (and profile), $2: the dump, $3: "measured" or
  # "plain". The measured cfg set matches the instrumented build's:
  # cargo-llvm-cov enables cfg(coverage) and cfg(coverage_nightly) (its own
  # run log says so), and an expansion without them cannot see a
  # `#[cfg_attr(coverage, coverage(off))]`. The plain set is what ships: no
  # coverage cfg at all (#140, decision 0050 §2).
  local cfgs=()
  [ "$3" = measured ] && cfgs=(--cfg coverage --cfg coverage_nightly)
  cargo +"$coverage_toolchain" rustc $1 --package "$root_package" --locked --offline \
    --target-dir "$forge_target_dir" -- -Zunpretty=expanded "${cfgs[@]}" >"$2" 2>"$2.log" || {
    tail -n 20 "$2.log" >&2 || true
    refuse "coverage refusal: the expansion of the $1 unit failed — its log is preserved at $2.log"
  }
  [ -s "$2" ] || refuse "coverage refusal: the expansion $2 is empty — the expansion must resolve the same cfg set as the instrumented build"
}
unit_selectors=(--lib)
unit_names=(lib)
mapfile -t bin_names < <(jq -r --arg m "$root_manifest" "$root_query"' | .targets[] | select(.kind | index("bin")) | .name' "$run_dir/metadata.json")
for bin_name in "${bin_names[@]}"; do
  unit_selectors+=("--bin $bin_name")
  unit_names+=("bin-$bin_name")
done
expansion_files=()
for i in "${!unit_selectors[@]}"; do
  sel="${unit_selectors[$i]}" name="${unit_names[$i]}"
  expansion_files+=("$run_dir/expanded-$name.rs")
  expand_unit "$sel" "$run_dir/expanded-$name.rs" measured
  # The same unit without the coverage cfg pair, and as the shipped release
  # profile builds it (Dockerfile: cargo build --release): each must be the
  # measured source, or the difference is code the report never saw.
  expand_unit "$sel" "$run_dir/expanded-$name-nocov.rs" plain
  check_expansions_equivalent "$run_dir/expanded-$name.rs" "$run_dir/expanded-$name-nocov.rs" "$name no-coverage-cfg"
  expand_unit "$sel --release" "$run_dir/expanded-$name-release.rs" plain
  check_expansions_equivalent "$run_dir/expanded-$name.rs" "$run_dir/expanded-$name-release.rs" "$name release-profile" release
done
# The library's test unit: inline #[cfg(test)] code stays measured, so an
# exclusion there refuses like one in production (#140).
expand_unit "--lib --profile test" "$run_dir/expanded-lib-test.rs" measured

check_no_harness_leak "$run_dir/lcov.info" "$run_dir/production-files.txt" "$PWD"
check_report_covers_production_files "$run_dir/lcov.info" "$run_dir/production-files.txt" "$PWD"
check_no_coverage_exclusions "${expansion_files[@]}"
check_test_unit_expansion "$run_dir/expanded-lib-test.rs"

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
