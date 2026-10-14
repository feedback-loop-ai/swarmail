#!/usr/bin/env bash
set -u
# The hermetic seat has no network, and the workspace graph is fully cached
# (cargo fetch --locked ran on the host before this seat fires). Making the
# refusal explicit keeps a cache miss a one-line "no matching package" instead
# of three DNS retries per crate — and keeps the gate's verdict deterministic.
export CARGO_NET_OFFLINE=true
prompt_file="${1:-}"
brokkr="${2:-brokkr}"
[ -f "$prompt_file" ] || { echo "verify-seat: prompt file missing" >&2; exit 2; }
result_path=""
while IFS= read -r line; do
  line="${line#"${line%%[![:space:]]*}"}"; line="${line%"${line##*[![:space:]]}"}"
  case "$line" in /*.json) result_path="$line" ;; esac
done < "$prompt_file"
[ -n "$result_path" ] || { echo "verify-seat: result path missing" >&2; exit 2; }
mkdir -p "$(dirname "$result_path")"
output="$(dirname "$result_path")/verify-output.$$"
notes="$(dirname "$result_path")/verify-notes.$$"
trap 'rm -f "$output" "$notes"' EXIT
write_result() { awk -v result="$1" '
  function json(text,out,i,byte,c){for(i=1;i<=length(text);i++){c=substr(text,i,1);if(c=="\\")out=out "\\\\";else if(c=="\"")out=out "\\\"";else{for(byte=1;byte<32&&c!=sprintf("%c",byte);byte++){}out=out (byte<32?sprintf("\\u%04x",byte):c)}}return out}
  BEGIN{printf "{\"result\": \"%s\", \"notes\": \"",result}{if(NR>1)printf "\\n";printf "%s",json($0)}END{print "\"}"}' "$notes" > "$result_path"; }
run() {
  label="$1"; command="$2"
  if ! bash -lc "$command" > "$output" 2>&1 </dev/null; then
    printf '%s failed; decisive output follows verbatim:\n' "$label" > "$notes"
    grep -E '(error|Error|ERROR|fail|FAIL|not found|offline)' "$output" | tail -n 20 >> "$notes" || true
    [ "$(wc -l < "$notes")" -gt 1 ] || tail -n 20 "$output" >> "$notes"
    write_result fail; exit 0
  fi
}
# The realm charter's six gates, in house-rules order. Every one must pass;
# the coverage gate is EXACT — literal 100% of lines, branches and functions
# (covered == count in all three dimensions), never a rounded percentage. The
# nightly switch lives inside the gate, so the seat command stays the same.
# --locked everywhere: a stale Cargo.lock is a red gate, never a silent
# resolve — and the hermetic seat has no network to resolve with anyway.
#
# The Helm chart contract (scripts/chart-verify.sh) is CI-only, by design:
# it renders both value profiles with helm and asserts the deployment
# boundary against the working tree — the hermetic seat proves the rust
# story (and the publish guard's own proof, below), the chart job in CI
# proves the chart.
#
# The publish guard's proof rides this seat now that "publish guard" is a
# required check name in CI (decision 0018 §6): the guard's test is
# hermetic by construction — a real HTTP server bound to 127.0.0.1 with
# the registry seam injected, no external network.
#
# The gate's own contract too: `coverage-gate.sh selftest` rebuilds the
# refusal arms against a synthetic source tree — a gate that cannot fail
# is not a gate.
run "cargo fmt --all -- --check"  'cargo fmt --all -- --check'
run "cargo clippy -D warnings"    'cargo clippy --workspace --all-targets --locked -- -D warnings'
run "cargo test"                  'cargo test --workspace --locked'
run "brokkr compile --bundle bundles/self"   "\"$brokkr\" compile --bundle bundles/self"
run "coverage gate (exact 100% lines+branches+functions)"  'bash scripts/coverage-gate.sh'
run "coverage gate selftest"      'bash scripts/coverage-gate.sh selftest'
run "publish guard proof"         'bash scripts/publish-guard-test.sh'
run "cargo deny licenses"         'cargo deny check licenses'
# Webhook TLS (reqwest + rustls) must be the only HTTP/TLS story: native-tls
# may never re-enter the tree (e.g. via a reqwest default-feature regression).
# Fail closed: the absence is proved by the probe's own "did not match any
# packages" report — the no-match exit code varies by cargo (1 on some,
# 101 on current) — so exit 0 (present) refuses, a failure that does not
# name the absence refuses, and only the named absence passes. The old
# `! cargo tree` passed on every failure including a broken cargo, which
# is the opposite of a proof.
run "no native-tls in the tree (fail closed)"  'out=$(cargo tree -i native-tls 2>&1); code=$?; if [ "$code" -eq 0 ]; then echo "REFUSAL: native-tls is in the dependency tree:" >&2; echo "$out" >&2; exit 1; elif printf "%s" "$out" | grep -q "did not match any packages"; then exit 0; else echo "REFUSAL: cargo tree failed (exit $code) and did not report absence; absence cannot be proved:" >&2; echo "$out" >&2; exit 1; fi'
printf 'all gates passed (fmt, clippy, test, bundle, exact 100/100/100 coverage + selftest, publish-guard proof, deny licenses, no native-tls fail-closed) with network denied' > "$notes"
write_result pass
