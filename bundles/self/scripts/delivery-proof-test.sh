#!/usr/bin/env bash
# delivery-proof-test.sh — #123's tamper boundary, proved by execution
# (decision 0050 §§1, 5). Builds real candidate fixtures — a
# dependency-free crate with a lib, a bin and an integration test that runs
# the bin — and runs the PINNED gate and the PINNED shim digests against
# them, exactly as verify-seat.sh does (working directory = candidate):
#
#   red   an uncovered production line, and the candidate's public
#         scripts/coverage-gate.sh replaced by `exit 0`: the tampered shim
#         alone would pass (the bypass #123 reproduced), but the pinned gate
#         refuses by coverage and the integrity check refuses the shim —
#         both diagnostics, neither preempting the other;
#   green the same crate with the line covered and the shims byte-identical:
#         the pinned gate passes and the integrity check passes;
#   more  a missing shim, a no-op `exit 1` shim and a modified shim each
#         refuse integrity by name.
#
# Run from the candidate repository root; fixtures live under its
# gitignored target/coverage/ and are removed on exit.
set -euo pipefail
pin_dir="$(CDPATH='' cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[ -f Cargo.toml ] || { echo "delivery-proof-test: run from the candidate repository root" >&2; exit 1; }
repo="$PWD"

eval_dispatch() {
  local work evidence_dir candidate engine verify target_dir candidate_head
  mkdir -p "$repo/target/coverage"
  work="$(mktemp -d "$repo/target/coverage/eval-dispatch-work.XXXXXX")"
  trap 'rm -rf "$work"' EXIT
  evidence_dir="$repo/target/coverage/eval-dispatch-evidence"
  mkdir -p "$evidence_dir"
  engine="$repo/.forge/bin/brokkr"
  verify="$pin_dir/verify-seat.sh"
  [ -x "$engine" ] || { echo "eval-dispatch: candidate-local brokkr is missing or not executable" >&2; return 1; }
  [ -f "$verify" ] || { echo "eval-dispatch: pinned verify-seat.sh is missing" >&2; return 1; }
  local expected_engine actual_engine
  expected_engine="$(jq -r '.brokkr.binary_sha256 | if type == "array" then .[0] else . end' "$repo/bundles/self/tooling.json")"
  actual_engine="$(sha256sum "$engine" | cut -d' ' -f1)"
  [ "$actual_engine" = "$expected_engine" ] || {
    echo "eval-dispatch: candidate-local brokkr digest does not match tooling.json" >&2
    return 1
  }

  candidate="$work/candidate"
  git clone --quiet --local --no-hardlinks "$repo" "$candidate"
  git diff --binary HEAD | git -C "$candidate" apply --binary
  mkdir -p "$candidate/.forge/bin" "$candidate/.forge/results"
  cp "$engine" "$candidate/.forge/bin/brokkr"
  candidate_head="$(git -C "$candidate" rev-parse HEAD)"
  target_dir="$candidate/target/custom target"
  cp "$candidate/evals/src/runner.rs" "$work/runner.rs"
  cp "$candidate/evals/tests/runner.rs" "$work/evals-runner.rs"

  verify_run() {
    local name="$1" expected="$2" result_path prompt log status diff_sha command ci_log ci_meta selected_bin
    result_path="$candidate/.forge/results/eval-dispatch-$name.json"
    prompt="$candidate/.forge/results/eval-dispatch-$name.prompt"
    log="$evidence_dir/eval-dispatch-$name.log"
    printf '%s\n' "$result_path" > "$prompt"
    rm -f "$result_path"
    printf -v command 'CARGO_TARGET_DIR=%q SWARMAIL_BIN=/bin/false bash %q %q %q' "$target_dir" "$verify" "$prompt" "$engine"
    diff_sha="$(git -C "$candidate" diff --binary HEAD | sha256sum | cut -d' ' -f1)"
    if (cd "$candidate" && unset SWARMAIL_EVAL_DISPATCH_MUTANT && CARGO_TARGET_DIR="$target_dir" SWARMAIL_BIN=/bin/false bash "$verify" "$prompt" "$engine") > "$log" 2>&1; then
      status=0
    else
      status=$?
    fi
    {
      printf 'candidate_head=%s\n' "$candidate_head"
      printf 'candidate_diff_sha256=%s\n' "$diff_sha"
      printf 'command=%s\n' "$command"
      printf 'process_exit=%s\n' "$status"
      printf 'expected_gate=%s\n' "$expected"
    } > "$evidence_dir/eval-dispatch-$name.meta"
    [ -f "$result_path" ] && cp "$result_path" "$evidence_dir/eval-dispatch-$name.json"
    if [ "$status" -ne 0 ] || [ ! -f "$result_path" ]; then
      echo "eval-dispatch: $name did not produce a verifier result (exit $status)" >&2
      tail -n 40 "$log" >&2 || true
      return 1
    fi
    python3 - "$result_path" "$expected" "$target_dir" <<'CHECK'
import json
import sys

result_path, expected, target_dir = sys.argv[1:]
with open(result_path, encoding="utf-8") as handle:
    result = json.load(handle)
if expected == "pass":
    if result.get("result") != "pass" or target_dir not in result.get("notes", ""):
        raise SystemExit(f"clean dispatch did not pass with the custom-target artifact: {result}")
else:
    notes = result.get("notes", "")
    if result.get("result") != "fail" or expected not in notes:
        raise SystemExit(f"dispatch did not refuse at {expected}: {result}")
print(f"{result['result']}: {expected}")
CHECK
    if [ "$expected" != pass ]; then
      ci_log="$evidence_dir/eval-dispatch-$name-ci.log"
      ci_meta="$evidence_dir/eval-dispatch-$name-ci.meta"
      case "$name" in
        fmt)
          command='cargo fmt --manifest-path evals/Cargo.toml --all -- --check'
          if (cd "$candidate" && cargo fmt --manifest-path evals/Cargo.toml --all -- --check) > "$ci_log" 2>&1; then status=0; else status=$?; fi
          marker='Diff in'
          ;;
        clippy)
          command='cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings'
          if (cd "$candidate" && CARGO_TARGET_DIR="$target_dir" cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings) > "$ci_log" 2>&1; then status=0; else status=$?; fi
          marker='needless_return'
          ;;
        tests)
          selected_bin="$(python3 - "$result_path" <<'BIN'
import json
import sys
with open(sys.argv[1], encoding="utf-8") as handle:
    notes = json.load(handle)["notes"]
for line in notes.splitlines():
    prefix = "candidate root-test artifact: "
    if line.startswith(prefix):
        print(line[len(prefix):])
        break
else:
    raise SystemExit("verify result did not retain the selected root-test artifact")
BIN
          )"
          command="SWARMAIL_BIN=<artifact-derived> cargo test --manifest-path evals/Cargo.toml --locked"
          if (cd "$candidate" && env -u SWARMAIL_EVAL_DISPATCH_MUTANT CARGO_TARGET_DIR="$target_dir" SWARMAIL_BIN="$selected_bin" cargo test --manifest-path evals/Cargo.toml --locked) > "$ci_log" 2>&1; then status=0; else status=$?; fi
          marker='injected_eval_test_failure'
          ;;
        *)
          echo "eval-dispatch: unexpected mutant $name" >&2
          return 1
          ;;
      esac
      {
        printf 'candidate_head=%s\n' "$candidate_head"
        printf 'candidate_diff_sha256=%s\n' "$diff_sha"
        printf 'command=%s\n' "$command"
        printf 'process_exit=%s\n' "$status"
      } > "$ci_meta"
      if [ "$status" -eq 0 ] || ! grep -q "$marker" "$ci_log"; then
        echo "eval-dispatch: CI-equivalent $name command did not fail for its mutant" >&2
        tail -n 40 "$ci_log" >&2 || true
        return 1
      fi
    fi
  }

  python3 - "$candidate/evals/src/runner.rs" <<'MUTATE_FMT'
from pathlib import Path
import sys
path = Path(sys.argv[1])
text = path.read_text()
old = "pub const RESULT_SCHEMA: u32 = 1;"
new = "pub  const RESULT_SCHEMA:u32=1;"
if old not in text:
    raise SystemExit("cannot install formatting mutant")
path.write_text(text.replace(old, new, 1))
MUTATE_FMT
  verify_run fmt 'eval cargo fmt --manifest-path evals/Cargo.toml --all -- --check'
  cp "$work/runner.rs" "$candidate/evals/src/runner.rs"

  cat >> "$candidate/evals/tests/runner.rs" <<'CLIPPY_MUTANT'

#[test]
#[deny(clippy::needless_return)]
fn injected_eval_clippy_warning() {
    return;
}
CLIPPY_MUTANT
  verify_run clippy 'eval cargo clippy --all-targets --locked -- -D warnings'
  cp "$work/evals-runner.rs" "$candidate/evals/tests/runner.rs"

  cat >> "$candidate/evals/tests/runner.rs" <<'TEST_MUTANT'

#[test]
fn injected_eval_test_failure() {
    assert!(std::env::var_os("SWARMAIL_EVAL_DISPATCH_MUTANT").is_some());
}
TEST_MUTANT
  verify_run tests 'eval cargo test --locked with root test artifact'
  cp "$work/evals-runner.rs" "$candidate/evals/tests/runner.rs"

  verify_run clean pass
  printf 'eval-dispatch: pass — actual pinned verify refused eval fmt, clippy and test mutants and passed a restored candidate using %s; evidence: %s\n' "$target_dir" "$evidence_dir"
}

case "${1:-}" in
  eval-dispatch)
    eval_dispatch
    exit $?
    ;;
  "")
    ;;
  *)
    echo "delivery-proof-test: unknown mode $1" >&2
    exit 2
    ;;
esac

mkdir -p target/coverage
work="$(mktemp -d "$repo/target/coverage/delivery-proof.XXXXXX")"
trap 'rm -rf "$work"' EXIT
failures=0
fail() { printf 'delivery-proof-test: FAIL — %s\n' "$1" >&2; failures=$((failures + 1)); }

candidate() { # $1: dir, $2: "covered" or "uncovered"
  local dir="$1"
  mkdir -p "$dir/src" "$dir/tests" "$dir/scripts" "$dir/.specify/scripts/bash"
  printf '[package]\nname = "fxcandidate"\nversion = "0.1.0"\nedition = "2021"\n\n[workspace]\n' >"$dir/Cargo.toml"
  {
    # The gate needs a nonzero count in every dimension: one branch, taken
    # both ways by the unit test.
    printf 'pub fn answer() -> i32 {\n    42\n}\npub fn sign(x: i32) -> i32 {\n    if x > 0 { 1 } else { 0 }\n}\n'
    [ "$2" = uncovered ] && printf 'pub fn uncovered(x: i32) -> i32 {\n    x + 1\n}\n'
    printf '#[cfg(test)]\nmod t {\n    #[test]\n    fn answers() {\n        assert_eq!(super::answer(), 42);\n        assert_eq!((super::sign(3), super::sign(-3)), (1, 0));\n    }\n}\n'
  } >"$dir/src/lib.rs"
  printf 'fn main() {\n    println!("{}", fxcandidate::answer());\n}\n' >"$dir/src/main.rs"
  printf '#[test]\nfn runs_the_bin() {\n    let out = std::process::Command::new(env!("CARGO_BIN_EXE_fxcandidate")).output().unwrap();\n    assert_eq!(String::from_utf8(out.stdout).unwrap(), "42\\n");\n}\n' >"$dir/tests/run.rs"
  # The public shims, byte-identical to the candidate repository's own.
  local shim
  while read -r _ shim; do
    mkdir -p "$dir/$(dirname "$shim")"
    cp "$repo/$shim" "$dir/$shim"
  done <"$pin_dir/shims.sha256"
  (cd "$dir" && cargo generate-lockfile --offline >/dev/null 2>&1)
}
run_in() { # $1: dir, $2: output file, rest: command — prints the exit status
  local dir="$1" out="$2" status=0; shift 2
  (cd "$dir" && "$@") >"$out" 2>&1 || status=$?
  printf '%s' "$status"
}
integrity() { sha256sum --check --strict "$pin_dir/shims.sha256"; }

# red: uncovered line + `exit 0` public gate.
red="$work/red"; candidate "$red" uncovered
printf '#!/usr/bin/env bash\nexit 0\n' >"$red/scripts/coverage-gate.sh"
[ "$(run_in "$red" "$work/red-shim.out" bash scripts/coverage-gate.sh)" = 0 ] ||
  fail 'the tampered `exit 0` shim should itself pass — the bypass this proof guards against'
status="$(run_in "$red" "$work/red-integrity.out" integrity)"
if [ "$status" = 0 ] || ! grep -q 'scripts/coverage-gate.sh: FAILED' "$work/red-integrity.out"; then
  fail "the integrity check did not refuse the tampered shim by name (exit $status): $(cat "$work/red-integrity.out")"
fi
status="$(run_in "$red" "$work/red-gate.out" bash "$pin_dir/coverage-gate.sh")"
if [ "$status" = 0 ] || ! grep -q 'literal nonzero 100% source-line/branch/function equality not met' "$work/red-gate.out"; then
  fail "the pinned gate did not refuse the uncovered line by coverage (exit $status): $(tail -n 20 "$work/red-gate.out")"
fi

# green: covered, clean shims.
green="$work/green"; candidate "$green" covered
status="$(run_in "$green" "$work/green-integrity.out" integrity)"
[ "$status" = 0 ] || fail "clean shims failed integrity (exit $status): $(cat "$work/green-integrity.out")"
status="$(run_in "$green" "$work/green-gate.out" bash "$pin_dir/coverage-gate.sh")"
[ "$status" = 0 ] || fail "the pinned gate refused the covered twin (exit $status): $(tail -n 20 "$work/green-gate.out")"

# more: missing, no-op and modified shims refuse integrity by name.
rm "$green/scripts/publish-guard-test.sh"
grep -q 'publish-guard-test.sh' <<<"$(cd "$green" && integrity 2>&1 || true)" && ! (cd "$green" && integrity >/dev/null 2>&1) ||
  fail 'a missing shim did not refuse integrity by name'
cp "$repo/scripts/publish-guard-test.sh" "$green/scripts/publish-guard-test.sh"
printf '#!/usr/bin/env bash\nexit 1\n' >"$green/scripts/design-check.sh"
grep -q 'scripts/design-check.sh: FAILED' <<<"$(cd "$green" && integrity 2>&1 || true)" ||
  fail 'a no-op shim did not refuse integrity by name'
cp "$repo/scripts/design-check.sh" "$green/scripts/design-check.sh"
printf '# appended\n' >>"$green/.specify/scripts/bash/check-prerequisites.sh"
grep -q 'check-prerequisites.sh: FAILED' <<<"$(cd "$green" && integrity 2>&1 || true)" ||
  fail 'a modified shim did not refuse integrity by name'

if [ "$failures" -ne 0 ]; then
  printf 'delivery-proof-test: %s arm(s) failed\n' "$failures" >&2
  exit 1
fi
printf 'delivery-proof-test: pass — an `exit 0` public gate cannot hide an uncovered line from the pinned gate, and every shim substitution refuses integrity\n'
