#!/usr/bin/env bash
set -u
# The hermetic seat has no network, and the workspace graph is fully cached
# (cargo fetch --locked ran on the host before this seat fires). Making the
# refusal explicit keeps a cache miss a one-line "no matching package" instead
# of three DNS retries per crate — and keeps the gate's verdict deterministic.
export CARGO_NET_OFFLINE=true
prompt_file="${1:-}"
brokkr="${2:-brokkr}"
# Proof ownership is transitive (decision 0050 §1): this script, the gates it
# runs, their scanner and the tooling record are the pinned bundle's bytes,
# resolved beside this file (/runtime/bundle/scripts in the exec box). The
# working directory is the candidate — the material assessed, never a source
# of proof code. A missing pinned resource refuses before any gate runs.
pin_dir="$(CDPATH='' cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
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
artifact_log="$(dirname "$result_path")/verify-artifacts.$$"
metadata_file="$(dirname "$result_path")/verify-metadata.$$"
bin_file="$(dirname "$result_path")/verify-swarmail-bin.$$"
trap 'rm -f "$output" "$notes" "$artifact_log" "$metadata_file" "$bin_file" "$result_path.tmp.$$"' EXIT
: > "$notes"
# Valid JSON with exactly result/inputs/notes, rendered by jq (no lossy
# escaping) into a sibling temporary file and renamed into place, so a
# reader never sees a partial or stale result. Without jq the seat refuses
# with a fixed-text failure rather than hand-escape.
write_result() {
  local tmp="$result_path.tmp.$$"
  if command -v jq >/dev/null 2>&1 &&
    jq -n --arg result "$1" --rawfile notes "$notes" '{result: $result, inputs: {}, notes: $notes}' > "$tmp"; then
    mv -f "$tmp" "$result_path"
  else
    printf '%s\n' '{"result": "fail", "inputs": {}, "notes": "verify-seat: jq could not render the verdict; refusing"}' > "$tmp" &&
      mv -f "$tmp" "$result_path"
  fi
}
fail_now() { printf '%s\n' "$1" >> "$notes"; write_result fail; exit 0; }
# Static call closure: coverage-gate.sh owns the scanner manifest, lock and
# source; design-check.sh calls both speckit helpers; delivery-proof-test.sh
# exercises the pinned proof boundary; publish-guard-test.sh uses the guard;
# seal-branch-test.sh uses the seal tool. shims.sha256 and ../tooling.json are
# consumed directly by this seat for wrapper and engine identity checks.
for resource in coverage-gate.sh publish-guard.sh publish-guard-test.sh design-check.sh delivery-proof-test.sh \
  seal-branch.sh seal-branch-test.sh speckit/check-prerequisites.sh speckit/common.sh coverage-fn-scan/Cargo.toml \
  coverage-fn-scan/Cargo.lock coverage-fn-scan/src/main.rs shims.sha256 ../tooling.json; do
  [ -f "$pin_dir/$resource" ] || fail_now "verify-seat: pinned resource $pin_dir/$resource is missing; refusing before any gate runs"
done

# 1. The public wrappers are fixed shims (scripts/coverage-gate.sh and its
#    siblings). They are never executed here, but a candidate that edits,
#    empties or deletes one refuses: its bytes must match the pinned
#    digests. The refusal is recorded and the gates still run, so a tampered
#    wrapper beside an uncovered line yields both diagnostics.
integrity_failed=0
if ! integrity="$(sha256sum --check --strict "$pin_dir/shims.sha256" 2>&1)"; then
  integrity_failed=1
  {
    printf 'wrapper integrity failed; the public entry points differ from the pinned shims (decision 0050 §1):\n'
    printf '%s\n' "$integrity" | grep -v ': OK$'
  } >> "$notes"
fi

# 2. Compare the launched executable's SHA-256 with the operator-recorded
#    binary_sha256 beside this pinned script (a string or a list). Version
#    equality cannot establish byte identity. This checks the recorded digest,
#    not source provenance or reproducibility of the base-plus-patches build.
#    Setup provisions the artifact before this offline seat runs.
recorded_sha="$(jq -r '.brokkr.sha // empty' "$pin_dir/../tooling.json" 2>/dev/null)"
launched="$(command -v "$brokkr" 2>/dev/null)" || fail_now "verify-seat: the launched brokkr '$brokkr' cannot be resolved; its identity is unprovable"
launched_digest="$(sha256sum "$launched" 2>/dev/null | cut -d' ' -f1)"
recorded_digests="$(jq -r '.brokkr.binary_sha256 | if type == "array" then .[] elif type == "string" then . else empty end' "$pin_dir/../tooling.json" 2>/dev/null)"
if [ -z "$recorded_digests" ]; then
  fail_now "verify-seat: tooling.json records no binary_sha256 for brokkr ${recorded_sha:-?}; the launched $launched (sha256 $launched_digest) cannot be checked against the recorded digest"
fi
printf '%s\n' "$recorded_digests" | grep -qx "$launched_digest" ||
  fail_now "verify-seat: the launched brokkr $launched (sha256 $launched_digest) does not match the recorded binary_sha256 ($recorded_digests) for source base ${recorded_sha:-?}"

launched_q="$(printf '%q' "$launched")"

run() {
  label="$1"; command="$2"
  if ! bash -c "$command" > "$output" 2>&1 </dev/null; then
    printf '%s failed; decisive output follows verbatim:\n' "$label" >> "$notes"
    grep -E '(error|Error|ERROR|fail|FAIL|refus|not found|offline)' "$output" | tail -n 20 >> "$notes" || true
    [ "$(wc -l < "$notes")" -gt 1 ] || tail -n 20 "$output" >> "$notes"
    write_result fail; exit 0
  fi
}
pin_q="$(printf '%q' "$pin_dir")"
# The realm charter's gates, in house-rules order. Every one must pass; the
# coverage gate is EXACT — literal 100% of lines, branches and functions
# (covered == count in all three dimensions), never a rounded percentage.
# --locked everywhere: a stale Cargo.lock is a red gate, never a silent
# resolve — and the hermetic seat has no network to resolve with anyway.
#
# The Helm chart contract (scripts/chart-verify.sh) is CI-only, by design.
# The publish guard's proof rides this seat (decision 0018 §6): hermetic by
# construction, a real HTTP server bound to 127.0.0.1. The gate's own
# contracts too: the coverage gate's and the design validator's selftests
# rebuild their refusal arms — a gate that cannot fail is not a gate. Every
# proof below runs from $pin_dir against the candidate in the working
# directory; nothing under the candidate's scripts/ or tools/ executes.
run "cargo fmt --all -- --check"  'cargo fmt --all -- --check'
run "cargo clippy -D warnings"    'cargo clippy --workspace --all-targets --locked -- -D warnings'
run_root_test_and_capture() {
  if (set -o pipefail; cargo test --workspace --locked --message-format=json-render-diagnostics 2>&1 | tee "$artifact_log" | awk 'substr($0, 1, 1) != "{"') > "$output" 2>&1; then
    :
  else
    status=$?
    printf 'cargo test --workspace --locked failed; captured output follows verbatim:\n' >> "$notes"
    cat "$artifact_log" >> "$notes"
    write_result fail; exit 0
  fi
  if ! cargo metadata --locked --format-version 1 --no-deps > "$metadata_file" 2> "$output"; then
    printf 'cargo metadata could not identify the root package; output follows verbatim:\n' >> "$notes"
    cat "$output" >> "$notes"
    write_result fail; exit 0
  fi
  if ! python3 - "$artifact_log" "$metadata_file" "$(pwd -P)/Cargo.toml" > "$bin_file" 2> "$output" <<'SELECTOR'
import json
import os
import sys

artifact_path, metadata_path, manifest = sys.argv[1:]
manifest = os.path.realpath(manifest)
with open(metadata_path, encoding="utf-8") as handle:
    metadata = json.load(handle)
packages = [p for p in metadata["packages"] if os.path.realpath(p["manifest_path"]) == manifest]
if len(packages) != 1:
    raise SystemExit(f"expected one canonical root package for {manifest}, found {len(packages)}")
package_id = packages[0]["id"]
executables = set()
with open(artifact_path, encoding="utf-8", errors="replace") as handle:
    for line in handle:
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        target = message.get("target", {})
        kind = target.get("kind", [])
        profile = message.get("profile", {})
        executable = message.get("executable")
        if (message.get("reason") == "compiler-artifact"
                and message.get("package_id") == package_id
                and target.get("name") == "swarmail"
                and isinstance(kind, list) and "bin" in kind
                and profile.get("test") is False
                and isinstance(executable, str)):
            if not os.path.isabs(executable):
                raise SystemExit(f"Cargo reported a non-absolute Swarmail executable: {executable!r}")
            path = os.path.realpath(executable)
            if not os.path.isfile(path) or not os.access(path, os.X_OK):
                raise SystemExit(f"Cargo reported a missing or non-executable Swarmail artifact: {path}")
            executables.add(path)
if len(executables) != 1:
    raise SystemExit(f"expected one distinct root swarmail executable, found {sorted(executables)!r}")
print(next(iter(executables)))
SELECTOR
  then
    printf 'root test artifact selection refused; output follows verbatim:\n' >> "$notes"
    cat "$output" >> "$notes"
    write_result fail; exit 0
  fi
  candidate_bin="$(cat "$bin_file")"
  if [ -z "$candidate_bin" ] || [ ! -f "$candidate_bin" ] || [ ! -x "$candidate_bin" ]; then
    fail_now "root test artifact selection returned no executable Swarmail path"
  fi
  printf 'candidate root-test artifact: %s\n' "$candidate_bin" >> "$notes"
}
run_root_test_and_capture
run "eval cargo fmt --manifest-path evals/Cargo.toml --all -- --check" \
  'cargo fmt --manifest-path evals/Cargo.toml --all -- --check'
run "eval cargo clippy --all-targets --locked -- -D warnings" \
  'cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings'
printf -v candidate_bin_q '%q' "$candidate_bin"
run "eval cargo test --locked with root test artifact" \
  "SWARMAIL_BIN=$candidate_bin_q cargo test --manifest-path evals/Cargo.toml --locked"
run "brokkr compile --bundle bundles/self"   "$launched_q compile --bundle bundles/self"
# A compile that exits 0 but sweeps a rule as reading past a hard rule's facts
# (#125's residual warning) is not a clean constitution: refuse it by name.
if grep -q 'lets the run go on without reading' "$output"; then
  { printf 'brokkr compile --bundle bundles/self emitted a policy-sweep warning:\n'; grep 'lets the run go on without reading' "$output"; } >> "$notes"
  write_result fail; exit 0
fi
run "coverage gate (exact 100% lines+branches+functions)"  "bash $pin_q/coverage-gate.sh"
run "coverage gate selftest"      "bash $pin_q/coverage-gate.sh selftest"
run "design check selftest"       "bash $pin_q/design-check.sh selftest"
run "delivery tamper-boundary proof"  "bash $pin_q/delivery-proof-test.sh"
run "publish guard proof"         "bash $pin_q/publish-guard-test.sh"
# The operator's seal tool, proved keyless (decision 0056): its report and
# refusals against scratch repositories with an empty GNUPGHOME. Its signing
# path is the operator's evidence and never runs here.
run "seal-branch keyless proof"   "bash $pin_q/seal-branch-test.sh"
run "cargo deny licenses"         'cargo deny check licenses'
# Webhook TLS (reqwest + rustls) must be the only HTTP/TLS story: native-tls
# may never re-enter the tree. Fail closed: only the probe's own "did not
# match any packages" report passes.
run "no native-tls in the tree (fail closed)"  'out=$(cargo tree -i native-tls 2>&1); code=$?; if [ "$code" -eq 0 ]; then echo "REFUSAL: native-tls is in the dependency tree:" >&2; echo "$out" >&2; exit 1; elif printf "%s" "$out" | grep -q "did not match any packages"; then exit 0; else echo "REFUSAL: cargo tree failed (exit $code) and did not report absence; absence cannot be proved:" >&2; echo "$out" >&2; exit 1; fi'
if [ "$integrity_failed" -ne 0 ]; then
  printf 'every gate passed, but wrapper integrity failed (above): not verified\n' >> "$notes"
  write_result fail; exit 0
fi
printf 'all gates passed from the pinned bundle (root fmt/clippy/test, eval fmt/clippy/full suite against %s, bundle, exact 100/100/100 coverage + selftest, design-check selftest, delivery tamper-boundary proof, publish-guard proof, seal-branch keyless proof, deny licenses, no native-tls fail-closed) with network denied; brokkr %s\n' "$candidate_bin" "$launched_digest" > "$notes"
write_result pass
