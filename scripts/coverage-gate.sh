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
#   3. Hardening: the instrumented build resolves under target/coverage/
#      (gitignored, disk-backed) by default — never under $TMPDIR, whose
#      tmpfs quota turned a disk-full build into a mid-build LLVM IO
#      failure (issue #101) — and an explicit CARGO_LLVM_COV_TARGET_DIR is
#      honoured verbatim; the EXIT trap removes only a directory the gate
#      itself created. A free-space preflight refuses before the build,
#      naming the path, the filesystem and the escape variable. The full
#      lcov report, the run log and a machine-readable summary are
#      preserved in a per-run evidence directory target/coverage/run.XXXXXX
#      — deliberately NOT removed by the EXIT trap (which cleans only the
#      instrumented build tree), so a refusal leaves burn-down evidence and
#      consecutive runs never overwrite each other; test-harness source
#      (tests.rs, *_tests.rs, tests/) leaking into the production report
#      refuses; production source parked under src/**/{tests,examples,
#      benches}/ refuses (it is excluded from the report by construction,
#      so the denominator would shrink silently); any coverage(off)
#      attribute exclusion in src/ refuses — matched token-wise, after
#      comment stripping — and every non-harness src/*.rs file must appear
#      in the report as an SF: record: a file absent from the report was
#      never measured, and that absence is the check that ends the arms
#      race a cleverer regex cannot win (issue #101).
#   4. The measurement is nightly: cargo +<nightly> llvm-cov --workspace
#      --locked --branch, where <nightly> is an installed toolchain that
#      actually carries llvm-tools. An unqualified `+nightly` is refused by
#      construction: rustup resolves it to the latest channel build and will
#      try to auto-install it — impossible inside a hermetic seat, and a
#      silent toolchain drift everywhere else. Without a nightly there are
#      no BRDA records and the branch dimension would not exist.
#   5. `coverage-gate.sh selftest` is part of the contract: it rebuilds
#      every refusal arm of every check against fixture trees under
#      target/coverage/selftest.XXXXXX (removed by the trap) and must pass
#      before the gate's verdict can be trusted. It is fail-closed — an arm
#      that passes when it must refuse, or fails when it must pass, exits
#      non-zero.
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

strip_rust_comments() {
  # One shared comment-stripping pass over a Rust file (issue #101): line
  # comments and nesting-aware block comments become nothing, line
  # structure preserved (the declaration walk reads lines; a newline is
  # always emitted). Strings, raw strings, char literals and lifetimes are
  # modelled by a char-level scanner whose raw-open decision is a proof,
  # not a guess (run 12b, issues #101/#102): the raw prefix is the closed
  # set r/br/cr, bounded before the whole prefix at an identifier boundary,
  # and the # run is counted exactly — no cap, no sliding window, no
  # undecidable residue to resolve toward erasure. Emission is streamed (no
  # per-line accumulator): a hostile multi-megabyte line costs linear work,
  # not quadratic. The one tolerated divergence is over-emission — a
  # comment stays visible, which is loud and bound by the SF: cross-check;
  # erasing live code is the one direction this function may not take.
  awk '
    function isident(ch) { return ch ~ /[A-Za-z0-9_]/ }
    function emit(s)      { printf "%s", s }
    {
      line = $0; n = length(line); i = 1;
      p1 = ""; p2 = ""; p3 = ""; run = 0;
      while (i <= n) {
        c = substr(line, i, 1); c2 = substr(line, i, 2);
        if (depth > 0) {
          # Comment interior: emitted as nothing.
          if (c2 == "*/") { depth--; i += 2; p1 = p2 = p3 = ""; run = 0 }
          else if (c2 == "/*") { depth++; i += 2; p1 = p2 = p3 = ""; run = 0 }
          else i++;
        } else if (instr) {
          if (shash > 0) {
            # A raw string ends at `"` followed by exactly shash `#`s.
            if (c == "\"") {
              j = i + 1; cnt = 0;
              while (substr(line, j, 1) == "#") { cnt++; j++ }
              if (cnt == shash) { instr = 0; emit(substr(line, i, j - i)); i = j; p1 = p2 = p3 = ""; run = 0; continue }
            }
            emit(c); i++;
          } else {
            # A raw string has no escapes; a plain string does.
            if (!sraw && c == "\\") { emit(substr(line, i, 2)); i += 2; continue }
            if (c == "\"") { instr = 0 }
            emit(c); i++; p1 = p2 = p3 = ""; run = 0;
          }
        } else if (inchar) {
          if (c == "\\") { emit(substr(line, i, 2)); i += 2; continue }
          if (c == "\047") { inchar = 0 }
          emit(c); i++;
        } else {
          if (c2 == "/*") { depth++; i += 2; p1 = p2 = p3 = ""; run = 0; continue }
          if (c2 == "//") { break }
          if (c == "\"") {
            # Raw-string opener proof at the quote: the # run ending here is
            # `run` and the character before it is p1 — the raw prefix is the
            # closed set r, br, cr, each at an identifier boundary (the
            # character before the whole prefix is no identifier char). The
            # lookback is exact for any run length, so the decision is a
            # proof, not a guess: there is no unresolved case to resolve
            # toward erasure. The only tolerated divergence is over-emission
            # (a comment stays visible — loud, and the SF: cross-check still
            # binds); erasing live code is the one direction this function
            # may not take.
            if (p1 == "r" && ((!isident(p2) && p2 != "b" && p2 != "c") || ((p2 == "b" || p2 == "c") && !isident(p3)))) { instr = 1; shash = run; sraw = 1 }
            else { instr = 1; shash = 0; sraw = 0 }
            emit(c); i++; p1 = p2 = p3 = ""; run = 0;
          } else if (c == "\047") {
            ch1 = substr(line, i + 1, 1); ch2 = substr(line, i + 2, 1);
            if (ch1 == "\\" || ch2 == "\047") { inchar = 1 }
            # else a lifetime: emit the quote and move on.
            emit(c); i++; p1 = p2 = p3 = ""; run = 0;
          } else {
            emit(c); i++;
            if (c == "#") { run++ }
            else { p3 = p2; p2 = p1; p1 = c; run = 0 }
          }
        }
      }
      # Line structure is preserved: exactly one newline per input line.
      print "";
      # String state persists across newlines for every literal kind: an
      # ordinary Rust string legally spans lines, and resetting its state
      # at the newline let a `/*` on the next line open a comment that
      # erased live code (the reviewer HIGH). An unterminated ordinary
      # string cannot occur in a tree that compiled, so persistence cannot
      # swallow live code of any input the gate actually sees; raw strings
      # and comments already persisted.
    }
  ' "$1"
}

rs_boundary_tokens() {
  # The comment-stripped file with every whitespace run replaced by a single
  # non-identifier byte (0x01) — a token stream that PRESERVES token
  # boundaries. Deleting whitespace outright (rs_tokens) merges `return
  # include` + a newline + `!(...)` into `returninclude!(`, and an
  # identifier-boundary guard then sees no `include` token at all (the run
  # 12c reviewer HIGH). The load-shape scan reads this stream; the
  # coverage(off) match keeps the collapsed rs_tokens.
  strip_rust_comments "$1" | tr -s '[:space:]' '\001'
}

rs_tokens() {
  # The comment-stripped file with all whitespace removed — the token stream
  # the coverage(off) match runs over. `#[coverage(/**/off)]` and
  # `#[coverage( off )]` collapse to `#[coverage(off)]` like the plain
  # spelling (issue #101, fixture E).
  strip_rust_comments "$1" | tr -d '[:space:]'
}

check_no_coverage_off() {
  # Attribute-based source exclusions are forbidden everywhere under src/ so
  # production code cannot silently shrink the denominator (house-rules).
  # The match is token-wise (issue #101): every .rs file is comment-stripped
  # and whitespace-collapsed before the comparison, so a comment hidden
  # inside the attribute's arguments and whitespace-padded spellings refuse
  # like the plain one. A scan failure refuses — the check must run, not
  # guess — and no VCS plumbing belongs inside a contract check.
  local dir="$1" found="" file
  if ! [ -d "$dir" ]; then
    refuse "coverage refusal: the coverage(off) check could not run — $dir is not a directory"
  fi
  while IFS= read -r -d '' file; do
    local stream
    if ! stream="$(rs_tokens "$file")"; then
      refuse "coverage refusal: the coverage(off) check could not read $file"
    fi
    if [ "${stream#*coverage(off)}" != "$stream" ]; then
      printf '%s\n' "$file" >&2
      found=1
    fi
  done < <(find "$dir" -type f -name '*.rs' -print0)
  if [ -n "$found" ]; then
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

check_report_covers_src() {
  # The check that ends the arms race (issue #101): every non-harness .rs
  # file under src/ must appear as an SF: record in the report. A laundering
  # shape that evades the declaration walk — a #[path] into a harness-named
  # file from a parent that is never read, an include!, a spelling the line
  # regexes have not met yet — leaves the harness-named file absent from the
  # report, and absence refuses. (Harness-named files are excluded here by
  # construction: they are dropped from the report by cargo-llvm-cov and
  # their declarations are the declaration walk's business.) The comparison
  # is a full-suffix match on the report's path — `/<src dir name>/<relative
  # path>$` — so a sibling name cannot satisfy it by substring.
  #
  # One absence is legitimate: a definition-only file — structs, enums,
  # consts, derive-generated impls — carries no `fn` item once comments are
  # stripped, so llvm-cov emits no SF record for it (there is no function to
  # report) and it contributes zero lines, branches and functions to every
  # denominator. Nothing is hidden, the floor is untouched. Anything that
  # could execute carries a `fn` item in its comment-stripped source and
  # stays a refusal — the file is named, so the first `fn` added to a
  # definition-only file fails loudly here, at the gate, not silently in
  # review. The probe is word-boundary'd over the comment-stripped text
  # (whitespace preserved — `pub fn` must read as two tokens), and strings
  # are not modelled: a literal "fn" can only over-refuse, loudly, never
  # over-pass — the same direction as the strip itself.
  local report="$1" src="$2" base missing="" rel
  base="$(basename "$src")"
  while IFS= read -r -d '' file; do
    rel="${file#"$src"/}"
    if ! awk -v want="/${base}/${rel}" '
      /^SF:/ {
        path = substr($0, 5);
        if (length(path) >= length(want) &&
            substr(path, length(path) - length(want) + 1) == want) { found = 1; exit }
      }
      END { exit found ? 0 : 1 }
    ' "$report"; then
      if strip_rust_comments "$file" | grep -qE '(^|[^A-Za-z0-9_])fn([^A-Za-z0-9_]|$)'; then
        missing="${missing}  ${rel}
"
      else
        printf 'coverage note: %s measures nothing (no fn item in its source) — legitimately absent from the report\n' "$rel"
      fi
    fi
  done < <(find "$src" -type f -name '*.rs' ! -name 'tests.rs' ! -name '*_tests.rs' ! -name '*-tests.rs' -print0)
  if [ -n "$missing" ]; then
    printf 'coverage refusal: production source absent from the report (never measured):\n%s' "$missing" >&2
    exit 1
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
  # unguarded one).
  #
  # Hardened (issue #101, fixtures A–G): the parent is comment-stripped
  # first (a guard inside a block comment is no guard); a declaration is any
  # `mod <stem>;` in any visibility (`pub`, `pub(...)`) including the `r#`
  # spelling; any `#[path = "..."]` whose target is harness-named refuses
  # wherever it sits in the parent (it loads the reserved name through a
  # module the walk does not see); an `include!` of a harness-named file
  # refuses likewise; a `#[path]`-carrying declaration of `<stem>` does not
  # count as a declaration of the file (it loads somewhere else — the file
  # is then uncompiled, and the no-declaration refusal fires); and a
  # non-`test` cfg on the declaration's attribute stack refuses by name.
  # Further hardened (run 12c): the #[path]/include! load-shape scan runs
  # over the whitespace-collapsed token stream (rs_tokens), not the
  # line-preserved stream, because every load-shape whose tokens are split
  # across lines — a stacked `#[allow(...)] #[path = ...]`, `include` at
  # end of line, a `#` on one line and `[path = ...]` on the next, a
  # `#[path =` whose string opens on a later line, a target continued with
  # a backslash-newline — is invisible to a line-based scan and launders
  # the reserved name (all probed red at the reviewed base). In the token
  # stream every occurrence is contiguous and is checked; an argument that
  # is not a plain `"..."` literal cannot be resolved by this walk and
  # refuses, fail-closed, the include! arm's established closed-form rule.
  # The guard is the literal attribute `#[cfg(test)]` on its own line in the
  # attribute stack directly above the declaration (blank lines may
  # intervene; comments are already gone). Anything else refuses,
  # fail-closed, the same species as the coverage(off) check: a false
  # positive is loud and names the file; the failure direction is refuse,
  # never silently mis-count. This is a refusal, not a recount — no counting
  # stage is added and none may be.
  local src="$1" harness_refusals=""
  while IFS= read -r harness; do
    local stem dir; stem="$(basename "$harness" .rs)"; dir="$(dirname "$harness")"
    # Every .rs file under src/ is a candidate declaring parent (the
    # review's HIGH on the conventional-parents-only walk: a second module
    # can `#[path = "store/persist_tests.rs"] pub mod production_alias;`
    # the harness through a parent the walk never read). The per-parent
    # scan below refuses any #[path]/include! that names a harness file
    # wherever it sits, and exits 3 for a candidate that does not declare
    # the harness at all — so widening the candidate set costs only scan
    # time and closes the bypass; the conventional parents are kept first
    # so refusals name them first.
    local parents=() cand
    if [ -f "$src/lib.rs" ]; then parents+=("$src/lib.rs"); fi
    if [ -f "$src/main.rs" ]; then parents+=("$src/main.rs"); fi
    if [ "$dir" != "$src" ] && [ -f "$dir.rs" ]; then parents+=("$dir.rs"); fi
    if [ "$dir" != "$src" ] && [ -f "$dir/mod.rs" ]; then parents+=("$dir/mod.rs"); fi
    while IFS= read -r -d '' cand; do
      parents+=("$cand")
    done < <(find "$src" -type f -name '*.rs' ! -name 'tests.rs' ! -name '*_tests.rs' ! -name '*-tests.rs' -print0 | sort -z)
    local any_decl=0 any_bad=0 details=""
    local parent rc detail
    for parent in "${parents[@]}"; do
      [ -f "$parent" ] || continue
      rc=0
      # Load-shape scan over the boundary-preserving token stream: every
      # effective path attribute and every include! in the parent, wherever
      # it sits and however its tokens are laid out, is checked here. The
      # path arm matches an identifier-boundary `path` followed by `=` and a
      # string — `#[path = ...]`, `#[r#path = ...]` and
      # `#[cfg_attr(all(), path = ...)]` all load a file in production and
      # all carry that token shape (the run 12c reviewer HIGH on the
      # `#[`-anchored scan); a path whose tokens are split across lines is
      # visible for the same reason. Whitespace must NOT be deleted: `return
      # include` + newline + `!(...)` collapses to `returninclude!(` in a
      # deleted stream and the identifier-boundary guard would see no
      # `include` token (the reviewer HIGH on token boundaries) — the
      # boundary stream keeps them distinct. An argument that is not a plain
      # string literal, an escape-carrying target, and a harness-named
      # target each refuse, fail-closed, naming the parent.
      detail="$(rs_boundary_tokens "$parent" | awk -v harness="$harness" -v parent="$parent" '
        {
          rest = $0;
          # An effective path attribute is `path = "..."` closed by `]` — the
          # close being ANY run of `)` and 0x01 separators before the bracket,
          # so a `cfg_attr` wrapper at ANY nesting depth is recognized (a
          # doubly-nested close `))]` matched neither of the two previous
          # disjuncts and laundered). The class allows zero closers, so the
          # bare-`]` case is subsumed, not lost; an assignment (`let path =
          # expr;`, `let path = dir.path().join("persist.db");`) whose
          # post-quote tail is not such a run keeps passing. A `path = "`
          # candidate whose target carries a string escape refuses: the
          # escape could spell the reserved name and the walk does not
          # decode it.
          while (match(rest, /(^|[^A-Za-z0-9_])path[\001]*=[\001]*"/)) {
            after = RSTART + RLENGTH;
            q = index(substr(rest, after), "\"");
            if (q == 0) { rest = substr(rest, after); continue; }
            target = substr(rest, after, q - 1);
            tail = substr(rest, after + q);
            if (tail !~ /^[\001)]*\]/) {
              # Not attribute-shaped: an ordinary assignment keeps passing.
              rest = substr(rest, after + q - 1);
              continue;
            }
            if (target ~ /\\/) {
              print harness ": a path target with a string escape cannot be resolved by this walk in " parent " — spell the path literally";
              scan_bad = 1;
            } else {
              nseg = split(target, seg, "/"); base = seg[nseg];
              if (base ~ /(^|[_-])tests\.rs$/) {
                print harness ": harness-named file loaded via a path attribute ("" target "") in " parent;
                scan_bad = 1;
              }
            }
            rest = substr(rest, after + q - 1);
          }
          rest = $0;
          while (match(rest, /(^|[^A-Za-z0-9_])include[\001]*![\001]*[({[]/)) {
            rest = substr(rest, RSTART + RLENGTH);
            if (rest !~ /^"/) {
              print harness ": an include! that is not a plain string literal cannot be resolved by this walk in " parent " — load the file with #[path] or a literal include!";
              scan_bad = 1; continue;
            }
            rest = substr(rest, 2);
            q = index(rest, "\"");
            if (q == 0) { scan_bad = 1; break; }
            target = substr(rest, 1, q - 1);
            if (target ~ /\\/) {
              print harness ": an include! target with a string escape cannot be resolved by this walk in " parent " — spell the path literally";
              scan_bad = 1;
            } else {
              nseg = split(target, seg, "/"); base = seg[nseg];
              if (base ~ /(^|[_-])tests\.rs$/) {
                print harness ": harness-named file included via include! in " parent;
                scan_bad = 1;
              }
            }
            rest = substr(rest, q);
          }
        }
        END { if (scan_bad) exit 1 }
      ')" || rc=$?
      if [ "$rc" -eq 1 ]; then
        any_bad=1
        details="${details}${detail}
"
      elif [ "$rc" -ne 0 ]; then
        refuse "coverage refusal: the harness declaration walk could not scan $parent"
      fi
      rc=0
      detail="$(strip_rust_comments "$parent" | awk -v stem="$stem" -v harness="$harness" -v parent="$parent" '
        BEGIN {
          escaped = stem;
          gsub(/[^A-Za-z0-9_]/, "\\\\&", escaped);
          decl = "^[[:space:]]*(pub(\\([^)]*\\))?[[:space:]]+)?mod[[:space:]]+(r#)?" escaped "[[:space:]]*;";
        }
        {
          lines[NR] = $0;
          if ($0 ~ /(^|[^A-Za-z0-9_])include[[:space:]]*![[:space:]]*[({[]/) {
            # An include! whose argument is not a plain string literal
            # cannot be resolved by this walk (concat!, env!, any macro
            # spelling): while a harness-named file exists under src/ it
            # cannot be proven innocent of loading one, so it refuses,
            # fail-closed, naming the parent (run 12b residual —
            # `include!(concat!(...))` laundered a harness-named file past
            # the literal-only arm). A literal argument is checked against
            # the harness-name regex as before.
            # Every macro delimiter and spacing is recognized —
            # `include! { ... }`, `include![ ... ]`, `include ! ( ... )`
            # load a file exactly like `include!( ... )` (the reviewer
            # HIGH on the parenthesis-only walk).
            if (($0 !~ /(^|[^A-Za-z0-9_])include[[:space:]]*![[:space:]]*[({[]/) || ($0 !~ /(^|[^A-Za-z0-9_])include[[:space:]]*![[:space:]]*[({[][[:space:]]*"/)) {
              print harness ": an include! that is not a plain string literal cannot be resolved by this walk in " parent " — load the file with #[path] or a literal include!";
              scan_bad = 1;
            } else {
              rest = $0;
              sub(/^.*include[[:space:]]*![[:space:]]*[({[][[:space:]]*"/, "", rest);
              q = index(rest, "\"");
              if (q > 0) {
                target = substr(rest, 1, q - 1);
                if (target ~ /\\/) {
                  print harness ": an include! target with a string escape cannot be resolved by this walk in " parent " — spell the path literally";
                  scan_bad = 1;
                } else {
                nseg = split(target, seg, "/"); base = seg[nseg];
                if (base ~ /(^|[_-])tests\.rs$/) {
                  print harness ": harness-named file included via include! in " parent;
                  scan_bad = 1;
                }
                }
              }
            }
          }
          if ($0 !~ decl) next;
          # Walk the attribute stack above the declaration: blank lines may
          # intervene; the first non-attribute line ends the stack. Comments
          # are already stripped, so a commented-out guard is no guard.
          guarded = 0; path_decl = 0; badcfg = "";
          for (i = NR - 1; i >= 1; i--) {
            t = lines[i];
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", t);
            if (t == "") continue;
            if (t !~ /^#/) break;
            if (t ~ /^#\[[[:space:]]*path[[:space:]]*=/) { path_decl = 1; }
            else if (t == "#[cfg(test)]") { guarded = 1; }
            else if (t ~ /^#\[[[:space:]]*cfg/) { badcfg = t; }
          }
          if (path_decl) next;  # loads a different file — not a declaration of this one
          if (badcfg != "") {
            print harness ": declared under a non-test cfg (" badcfg ") in " parent;
            scan_bad = 1; next;
          }
          nmods++; mods[nmods] = NR; guards[nmods] = guarded;
        }
        END {
          if (scan_bad) exit 1;
          if (nmods == 0) exit 3;  # this parent does not declare the harness
          for (k = 1; k <= nmods; k++) {
            if (!guards[k]) {
              print harness ": declared without #[cfg(test)] in " parent; bad = 1;
            }
          }
          exit bad ? 1 : 0;
        }
      ')" || rc=$?
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
  if [ -z "$avail_kib" ] || (( avail_kib < min_kib )); then
    printf 'coverage refusal: %s has %s KiB free on filesystem %s (mounted at %s); the instrumented build reserves %s KiB — free space there, or point CARGO_LLVM_COV_TARGET_DIR at a larger filesystem\n' \
      "$dir" "${avail_kib:-?}" "${fs:-?}" "${mount:-?}" "$min_kib" >&2
    exit 1
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
    RESOLVED_BUILD_DIR="$(mktemp -d target/coverage/build.XXXXXX)"
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

  # 2. coverage(off): the attribute refuses — plain, comment-hidden and
  #    whitespace-padded spellings alike (issue #101, fixture E); clean
  #    source passes.
  local fx="$root/co-off"; mkdir -p "$fx/src"
  printf '#[coverage(off)]\nfn hot() {}\n' > "$fx/src/lib.rs"
  expect_refuse 'coverage(off) in src is refused' check_no_coverage_off "$fx/src"
  fx="$root/co-clean"; mkdir -p "$fx/src"
  printf 'fn hot() {}\n' > "$fx/src/lib.rs"
  expect_pass 'clean src passes the coverage(off) check' check_no_coverage_off "$fx/src"
  fx="$root/co-token-comment"; mkdir -p "$fx/src"
  printf '#[coverage(/**/off)]\nfn hot() {}\n' > "$fx/src/lib.rs"
  expect_refuse 'a comment-hidden coverage(off) is refused (fixture E)' check_no_coverage_off "$fx/src"
  fx="$root/co-token-space"; mkdir -p "$fx/src"
  printf '#[coverage( off )]\nfn hot() {}\n' > "$fx/src/lib.rs"
  expect_refuse 'a whitespace-padded coverage(off) is refused (fixture E)' check_no_coverage_off "$fx/src"

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

  # 5. Harness declarations (decision 0020, hardened by #101). Each arm is a
  #    fresh fixture tree; the failure direction is refuse.
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
  expect_refuse 'a #[path]-carrying declaration does not declare the harness file' check_harness_declarations "$fx/src"

  printf 'fn persisted() {}\n' > "$fx/src/store/persist_tests.rs"
  printf 'fn other() {}\n' > "$fx/src/store.rs"
  printf '#[cfg(test)]\nmod persist_tests;\n' > "$fx/src/store.rs"
  printf 'mod persist_tests;\n' > "$fx/src/store/mod.rs"
  expect_refuse 'one guarded declaration cannot launder an unguarded one' check_harness_declarations "$fx/src"

  printf 'fn other() {}\n' > "$fx/src/store.rs"
  rm "$fx/src/store/mod.rs"
  expect_refuse 'a harness file with no declaration at all is refused' check_harness_declarations "$fx/src"

  # 5a. The #101 bypass fixtures, each of which passed the old line regexes.
  printf '#[cfg(test)]\nmod persist_tests;\n#[path = "store/persist_tests.rs"]\npub mod helpers;\n' > "$fx/src/store/mod.rs"
  expect_refuse 'a harness-named #[path] target anywhere in the parent is refused (fixture A)' check_harness_declarations "$fx/src"

  printf '#[cfg(test)]\nmod persist_tests;\n#[cfg(not(test))]\npub(crate) mod persist_tests;\n' > "$fx/src/store/mod.rs"
  expect_refuse 'a non-test cfg on the declaration refuses (fixture B)' check_harness_declarations "$fx/src"

  printf '/*\n#[cfg(test)]\n*/\nmod persist_tests;\n' > "$fx/src/store/mod.rs"
  expect_refuse 'a guard inside a block comment is no guard (fixture C)' check_harness_declarations "$fx/src"

  printf '#[cfg(test)]\nmod persist_tests;\ninclude!("store/persist_tests.rs");\n' > "$fx/src/store/mod.rs"
  expect_refuse 'an include! of a harness-named file refuses (fixture D)' check_harness_declarations "$fx/src"

  printf 'pub mod\npersist_tests;\n/*\n#[cfg(test)]\nmod persist_tests;\n*/\n' > "$fx/src/store/mod.rs"
  expect_refuse 'a split visibility spelling with a commented guard refuses (fixture G)' check_harness_declarations "$fx/src"

  printf '#[cfg(test)]\npub mod persist_tests;\n' > "$fx/src/store/mod.rs"
  expect_pass 'a guarded pub declaration passes' check_harness_declarations "$fx/src"

  printf '#[cfg(test)]\nmod r#persist_tests;\n' > "$fx/src/store/mod.rs"
  expect_pass 'a guarded r# declaration passes' check_harness_declarations "$fx/src"

  # 5b. The stripper's erasure shapes (run 12b, issues #101/#102). Each
  #     fixture hides a forbidden #[coverage(off)] item behind a shape that
  #     erased live code from the shared token stream at the base stripper;
  #     the attribute must stay visible to check_no_coverage_off, which then
  #     refuses. An erased attribute would let the file pass — the base
  #     stripper's red on these arms is recorded in the run's landing notes.
  fx="$root/co-raw-cr"; mkdir -p "$fx/src"
  printf 'static S: &str = cr#"abc\n/* "#;\n#[coverage(off)]\npub fn hidden() {}\n' > "$fx/src/lib.rs"
  expect_refuse 'a cr#" raw string keeps the forbidden attribute visible (R1)' check_no_coverage_off "$fx/src"

  fx="$root/co-raw-9hash"; mkdir -p "$fx/src"
  printf 'const S: &str = r#########"x"/*"y"#########;\n#[coverage(off)]\nmod hidden {}\n' > "$fx/src/lib.rs"
  expect_refuse 'a 9-hash raw string keeps the forbidden attribute visible (R2)' check_no_coverage_off "$fx/src"

  fx="$root/co-raw-col0"; mkdir -p "$fx/src"
  printf 'static S: &str =\nr#"say "/*" hi"#;\n#[coverage(off)]\nfn hidden() {}\n' > "$fx/src/lib.rs"
  expect_refuse 'a column-0 raw string keeps the forbidden attribute visible (I)' check_no_coverage_off "$fx/src"

  fx="$root/co-raw-multiline"; mkdir -p "$fx/src"
  printf 'static S: &str = r"say\n/* hi";\n#[coverage(off)]\nfn hidden() {}\n' > "$fx/src/lib.rs"
  expect_refuse 'a multi-line r"…" raw string keeps the forbidden attribute visible (R4)' check_no_coverage_off "$fx/src"

  #     The keep direction: shapes that must pass through the stripper
  #     verbatim — the raw string is emitted, the code after it survives.
  fx="$root/raw-col0-keep"; mkdir -p "$fx/src"
  printf 'static S: &str =\nr#"say "/*" hi"#;\nfn visible() {}\n' > "$fx/src/keep.rs"
  expect_eq 'a column-0 raw string is emitted verbatim (I, keep direction)' "$(strip_rust_comments "$fx/src/keep.rs")" "$(cat "$fx/src/keep.rs")"

  fx="$root/raw-8hash-keep"; mkdir -p "$fx/src"
  printf 'const S: &str = r########"x"/*"y"########;\n#[coverage(off)]\nmod hidden {}\n' > "$fx/src/keep.rs"
  expect_eq 'the 8-hash boundary is emitted verbatim (keep direction)' "$(strip_rust_comments "$fx/src/keep.rs")" "$(cat "$fx/src/keep.rs")"
  expect_refuse 'the 8-hash boundary keeps the forbidden attribute visible' check_no_coverage_off "$fx/src"

  fx="$root/raw-outr-keep"; mkdir -p "$fx/src"
  printf 'static S: &str = outer"/*"x";\n#[coverage(off)]\nfn hot() {}\n' > "$fx/src/keep.rs"
  expect_eq 'an identifier ending in r is a plain string, emitted verbatim (keep direction)' "$(strip_rust_comments "$fx/src/keep.rs")" "$(cat "$fx/src/keep.rs")"
  expect_refuse 'the plain-string control keeps the forbidden attribute visible' check_no_coverage_off "$fx/src"

  # 5c. The run 12b residual on the declaration walk: an include! whose
  #     argument is not a plain string literal (concat!, env!, any macro
  #     spelling) cannot be resolved, and while a harness-named file exists
  #     under src/ it cannot be proven innocent of loading one — it refuses,
  #     fail-closed. At the base gate both laundering shapes below PASSED
  #     the walk (recorded red in the landing notes); a literal include! of
  #     an ordinary file must keep passing.
  fx="$root/co-inc-env"; mkdir -p "$fx/src/store"
  printf 'pub fn hidden_production() {}\n' > "$fx/src/store/persist_tests.rs"
  printf '#[cfg(test)]\nmod persist_tests;\ninclude!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/store/persist_tests.rs"));\npub fn production() { hidden_production(); }\n' > "$fx/src/lib.rs"
  expect_refuse 'an unresolved include! (concat! + env!) refuses (laundering residual)' check_harness_declarations "$fx/src"

  fx="$root/co-inc-lit"; mkdir -p "$fx/src/store"
  printf 'pub fn hidden_production() {}\n' > "$fx/src/store/persist_tests.rs"
  printf '#[cfg(test)]\nmod persist_tests;\ninclude!(concat!("src/store/", "persist_tests.rs"));\npub fn production() { hidden_production(); }\n' > "$fx/src/lib.rs"
  expect_refuse 'an all-literal concat! include! of a harness-named file refuses' check_harness_declarations "$fx/src"

  fx="$root/co-inc-plain"; mkdir -p "$fx/src/store"
  printf 'pub fn ordinary() {}\n' > "$fx/src/store/ordinary.rs"
  printf '#[cfg(test)]\nmod persist_tests;\ninclude!("store/ordinary.rs");\n' > "$fx/src/lib.rs"
  expect_pass 'a literal include! of a non-harness file passes' check_harness_declarations "$fx/src"

  # 5h. The run 12c reviewer residuals, each its own arm: an ordinary
  #     string that spans lines must not let the next line open a comment
  #     that erases the forbidden attribute; every include! macro
  #     delimiter and spacing launders exactly like the parenthesis
  #     spelling; and a string escape in a target (\x74 spells t) dodges
  #     the literal comparison, so an escape-carrying target refuses.
  fx="$root/co-multiline-string"; mkdir -p "$fx/src/store"
  printf 'pub fn ordinary() {}\n' > "$fx/src/store/ordinary.rs"
  printf 'const S: &str = "start\n/*";\n#[coverage(off)]\npub fn hidden() {}\nconst T: &str = "*/";\n' > "$fx/src/lib.rs"
  expect_refuse 'a multiline string must not let the next line open an erasing comment (reviewer residual)' check_no_coverage_off "$fx/src"

  for spelling in 'include! { "store/persist_tests.rs" }' 'include!["store/persist_tests.rs"]' 'include ! ( "store/persist_tests.rs" )'; do
    fx="$root/co-inc-spelling"; mkdir -p "$fx/src/store"
    printf 'pub fn hidden_production() {}\n' > "$fx/src/store/persist_tests.rs"
    printf '#[cfg(test)]\nmod persist_tests;\n%s;\n' "$spelling" > "$fx/src/lib.rs"
    expect_refuse 'a harness-named include! in an alternate macro spelling refuses (reviewer residual)' check_harness_declarations "$fx/src"
  done

  fx="$root/co-escape-target"; mkdir -p "$fx/src/store"
  printf 'pub fn hidden_production() {}\n' > "$fx/src/store/persist_tests.rs"
  printf '#[cfg(test)]\nmod persist_tests;\ninclude!("store/persist_\\x74ests.rs");\n' > "$fx/src/lib.rs"
  expect_refuse 'an escape-carrying include! target refuses (reviewer residual)' check_harness_declarations "$fx/src"

  fx="$root/co-escape-path"; mkdir -p "$fx/src/store"
  printf 'pub fn hidden_production() {}\n' > "$fx/src/store/persist_tests.rs"
  printf '#[cfg(test)]\nmod persist_tests;\n#[path = "store/persist_\\x74ests.rs"]\npub mod alias;\n' > "$fx/src/lib.rs"
  expect_refuse 'an escape-carrying #[path] target refuses (reviewer residual)' check_harness_declarations "$fx/src"

  # 5j. The run 12c reviewer residuals on the scan's input model: an
  #     effective path attribute in a wrapper (cfg_attr, raw-identifier
  #     name) and a token boundary a whitespace-DELETING stream destroys.
  #     Each fixture carries the declared harness so the check is
  #     non-vacuous, and each is red at fec409f.
  fx="$root/co-cfg-attr-path"; mkdir -p "$fx/src/store"
  printf 'pub fn hidden_production() {}\n' > "$fx/src/store/persist_tests.rs"
  printf 'pub fn ordinary() {}\n' > "$fx/src/alias.rs"
  printf '#[cfg(test)]\nmod persist_tests;\n#[cfg_attr(not(test), path = "store/persist_tests.rs")]\npub mod alias;\n' > "$fx/src/lib.rs"
  expect_refuse 'a cfg_attr-wrapped path attribute to a harness-named file refuses (reviewer residual)' check_harness_declarations "$fx/src"

  fx="$root/co-raw-attr-path"; mkdir -p "$fx/src/store"
  printf 'pub fn hidden_production() {}\n' > "$fx/src/store/persist_tests.rs"
  printf 'pub fn ordinary() {}\n' > "$fx/src/alias.rs"
  printf '#[cfg(test)]\nmod persist_tests;\n#[r#path = "store/persist_tests.rs"]\npub mod alias;\n' > "$fx/src/lib.rs"
  expect_refuse 'a raw-identifier path attribute to a harness-named file refuses (reviewer residual)' check_harness_declarations "$fx/src"

  fx="$root/co-split-include"; mkdir -p "$fx/src/store"
  printf 'pub fn hidden_production() {}\n' > "$fx/src/store/persist_tests.rs"
  printf '#[cfg(test)]\nmod persist_tests;\npub fn caller() {\n    return include\n        !("store/persist_tests.rs");\n}\n' > "$fx/src/lib.rs"
  expect_refuse 'an include whose tokens a whitespace-deleting stream merges refuses (reviewer residual)' check_harness_declarations "$fx/src"

  # 5i. The run 12c design-sequence residuals: a load-shape whose tokens the
  #     line-based walk cannot resolve launders the reserved name. Each arm
  #     is red at the reviewed base (bcc7510) — the shape passed the walk —
  #     and refuses once the load-shape scan runs over the token stream.
  fx="$root/co-stacked-path"; mkdir -p "$fx/src/store"
  printf 'pub fn hidden_production() {}\n' > "$fx/src/store/persist_tests.rs"
  printf '#[cfg(test)]\nmod persist_tests;\n' > "$fx/src/store/mod.rs"
  printf '#[allow(dead_code)] #[path = "store/persist_tests.rs"] pub mod alias;\n' > "$fx/src/lib.rs"
  expect_refuse 'a stacked #[path] attribute hides the harness-named target (design residual)' check_harness_declarations "$fx/src"

  fx="$root/co-stacked-clean"; mkdir -p "$fx/src/store"
  printf 'pub fn ordinary() {}\n' > "$fx/src/store/ordinary.rs"
  printf 'pub fn hidden_production() {}\n' > "$fx/src/store/persist_tests.rs"
  printf '#[cfg(test)]\nmod persist_tests;\n#[allow(dead_code)] #[path = "store/ordinary.rs"] pub mod alias;\n' > "$fx/src/lib.rs"
  expect_pass 'a stacked #[path] to a non-harness file passes (keep direction)' check_harness_declarations "$fx/src"

  fx="$root/co-inc-line-split"; mkdir -p "$fx/src/store"
  printf 'pub fn hidden_production() {}\n' > "$fx/src/store/persist_tests.rs"
  printf '#[cfg(test)]\nmod persist_tests;\n' > "$fx/src/store/mod.rs"
  printf 'pub fn x() {}\ninclude\n!("store/persist_tests.rs");\n' > "$fx/src/lib.rs"
  expect_refuse 'an include! with its tokens split across lines refuses (design residual)' check_harness_declarations "$fx/src"

  fx="$root/co-hash-split"; mkdir -p "$fx/src/store"
  printf 'pub fn hidden_production() {}\n' > "$fx/src/store/persist_tests.rs"
  printf '#[cfg(test)]\nmod persist_tests;\n' > "$fx/src/store/mod.rs"
  printf 'pub fn x() {}\n#\n[path = "store/persist_tests.rs"]\npub mod alias;\n' > "$fx/src/lib.rs"
  expect_refuse 'a # and [path] split across lines refuses (design residual)' check_harness_declarations "$fx/src"

  fx="$root/co-split-open"; mkdir -p "$fx/src/store"
  printf 'pub fn hidden_production() {}\n' > "$fx/src/store/persist_tests.rs"
  printf '#[cfg(test)]\nmod persist_tests;\n' > "$fx/src/store/mod.rs"
  printf 'pub fn x() {}\n#[path =\n"store/persist_tests.rs"] pub mod alias;\n' > "$fx/src/lib.rs"
  expect_refuse 'a #[path] whose string opens on a later line refuses (design residual)' check_harness_declarations "$fx/src"

  fx="$root/co-continuation-target"; mkdir -p "$fx/src/store"
  printf 'pub fn hidden_production() {}\n' > "$fx/src/store/persist_tests.rs"
  printf '#[cfg(test)]\nmod persist_tests;\n' > "$fx/src/store/mod.rs"
  printf 'pub fn x() {}\n#[path = "store/persist_\\\ntests.rs"] pub mod alias;\n' > "$fx/src/lib.rs"
  expect_refuse 'a backslash-newline-continued #[path] target refuses (design residual)' check_harness_declarations "$fx/src"

  # 5k. The run 12d residual: a cfg_attr nested inside a cfg_attr closes
  #     with `))]` — the two-disjunct close test matched neither disjunct,
  #     so the shape fell through as an ordinary assignment and the reserved
  #     name laundered (red at 05ed2b8; rustc honors the nesting, so the
  #     shape is real). The close test accepts any run of `)` and
  #     separators before `]`, so nesting at any depth is recognized and
  #     the escape rule reaches the newly recognized shapes. Each fixture
  #     carries the declared harness so the check is non-vacuous.
  fx="$root/co-nested-attr-harness"; mkdir -p "$fx/src/store"
  printf 'pub fn hidden_production() {}\n' > "$fx/src/store/persist_tests.rs"
  printf 'pub fn ordinary() {}\n' > "$fx/src/alias.rs"
  printf '#[cfg(test)]\nmod persist_tests;\n#[cfg_attr(all(), cfg_attr(all(), path = "store/persist_tests.rs"))]\npub mod alias;\n' > "$fx/src/lib.rs"
  expect_refuse 'a doubly-nested cfg_attr path attribute to a harness-named file refuses (run 12d residual)' check_harness_declarations "$fx/src"

  fx="$root/co-nested-attr-escape"; mkdir -p "$fx/src/store"
  printf 'pub fn hidden_production() {}\n' > "$fx/src/store/persist_tests.rs"
  printf 'pub fn ordinary() {}\n' > "$fx/src/alias.rs"
  printf '#[cfg(test)]\nmod persist_tests;\n#[cfg_attr(all(), cfg_attr(all(), path = "store/persist_\\x74ests.rs"))]\npub mod alias;\n' > "$fx/src/lib.rs"
  expect_refuse 'a doubly-nested cfg_attr path attribute with an escape-carrying target refuses (run 12d residual)' check_harness_declarations "$fx/src"

  fx="$root/co-nested-attr-clean"; mkdir -p "$fx/src/store"
  printf 'pub fn ordinary() {}\n' > "$fx/src/store/ordinary.rs"
  printf 'pub fn hidden_production() {}\n' > "$fx/src/store/persist_tests.rs"
  printf 'pub fn ordinary() {}\n' > "$fx/src/alias.rs"
  printf '#[cfg(test)]\nmod persist_tests;\n#[cfg_attr(all(), cfg_attr(all(), path = "store/ordinary.rs"))]\npub mod alias;\n' > "$fx/src/lib.rs"
  expect_pass 'a doubly-nested cfg_attr path attribute to a non-harness file passes (keep direction)' check_harness_declarations "$fx/src"

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

  # 7. Build-dir resolution (issue #101): the explicit value verbatim, the
  #    default under the disk-backed target/coverage/ — never $TMPDIR.
  local resolved
  resolved="$(CARGO_LLVM_COV_TARGET_DIR="$root/explicit" resolve_build_dir; printf '%s' "$RESOLVED_BUILD_DIR")"
  expect_eq 'an explicit CARGO_LLVM_COV_TARGET_DIR is honoured verbatim' "$resolved" "$root/explicit"
  expect_eq 'the explicit build dir is created' "$( [ -d "$root/explicit" ] && echo yes || echo no )" yes
  resolved="$(unset CARGO_LLVM_COV_TARGET_DIR; resolve_build_dir; printf '%s' "$RESOLVED_BUILD_DIR")"
  case "$resolved" in
    target/coverage/build.*)
      expect_eq 'the default build dir is created' "$( [ -d "$resolved" ] && echo yes || echo no )" yes
      rm -rf "$resolved"
      ;;
    *) expect_eq 'the default build dir resolves under target/coverage/' "$resolved" 'target/coverage/build.XXXXXX' ;;
  esac

  # 8. The free-space preflight: an absurd floor refuses naming the path,
  #    the filesystem and the escape variable; a real one passes.
  expect_refuse 'a preflight floor larger than the filesystem refuses' check_free_space "$root" 999999999999
  expect_pass 'a preflight floor of 1 KiB passes' check_free_space "$root" 1

  # 9. Report completeness (issue #101): every non-harness src file must
  #    appear as an SF: record — absence refuses.
  fx="$root/sfcov"; mkdir -p "$fx/src"
  printf 'fn a() {}\n' > "$fx/src/a.rs"
  printf 'fn b() {}\n' > "$fx/src/b.rs"
  printf 'SF:/repo/src/a.rs\nDA:1,1\nend_of_record\n' > "$root/partial.info"
  expect_refuse 'a src file absent from the report refuses' check_report_covers_src "$root/partial.info" "$fx/src"
  printf 'SF:/repo/src/a.rs\nDA:1,1\nend_of_record\nSF:/repo/src/b.rs\nDA:1,0\nend_of_record\n' > "$root/full.info"
  expect_pass 'a complete report passes the SF cross-check' check_report_covers_src "$root/full.info" "$fx/src"

  # 9b. A definition-only file cannot measure: no `fn` in its comment-
  #     stripped token stream, so llvm-cov emits no SF record and it adds
  #     zero to every denominator — its absence passes. A file that could
  #     execute (or a `fn` hidden only in a comment) keeps the strict
  #     direction: the first real `fn` added to such a file refuses loudly.
  fx="$root/defonly"; mkdir -p "$fx/src"
  printf 'fn a() {}\n' > "$fx/src/a.rs"
  printf '//! Core data model.\n\n/// A captured email.\n#[derive(Debug, Clone)]\npub struct Email {\n    pub id: String,\n}\n' > "$fx/src/model.rs"
  printf 'SF:/repo/src/a.rs\nDA:1,1\nend_of_record\n' > "$root/defonly.info"
  expect_pass 'a definition-only src file is legitimately absent from the report' check_report_covers_src "$root/defonly.info" "$fx/src"
  printf '//! Core data model.\n// fn hidden_in_a_comment() {}\npub struct Email {\n    pub id: String,\n}\n' > "$fx/src/model.rs"
  expect_pass 'a fn inside a comment does not make a file measurable' check_report_covers_src "$root/defonly.info" "$fx/src"
  printf '//! Core data model.\n\n/// A captured email.\npub struct Email {\n    pub id: String,\n}\n\npub fn touch() {}\n' > "$fx/src/model.rs"
  expect_refuse 'a fn-bearing src file absent from the report refuses' check_report_covers_src "$root/defonly.info" "$fx/src"

  if [ "$failures" -ne 0 ]; then
    printf 'coverage gate: selftest failed with %s arm(s)\n' "$failures" >&2
    exit 1
  fi
  printf 'coverage gate: selftest pass — every refusal arm refuses, every pass arm passes\n'
}

case "${1:-}" in
  selftest)
    mkdir -p target/coverage
    selftest_root="$(mktemp -d target/coverage/selftest.XXXXXX)"
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
export CARGO_LLVM_COV_TARGET_DIR="$forge_target_dir"

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

# Per-run evidence: a fresh directory per invocation, preserved after the run
# (the EXIT trap cleans only the instrumented build tree). Consecutive runs
# never overwrite each other's reports, and a refusal leaves burn-down
# evidence behind.
mkdir -p target/coverage
run_dir="$(mktemp -d target/coverage/run.XXXXXX)"

# A unique target directory is stronger than cleaning shared coverage state:
# no stale instrumented executable can participate in this candidate's merge.
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
check_report_covers_src "$run_dir/lcov.info" src

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
