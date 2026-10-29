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
