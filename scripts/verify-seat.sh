#!/usr/bin/env bash
set -u
prompt_file="${1:-}"
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
run "cargo fmt --all -- --check"  'cargo fmt --all -- --check'
run "cargo clippy -D warnings"    'cargo clippy --workspace --all-targets --locked -- -D warnings'
run "cargo test"                  'cargo test --workspace --locked'
run "brokkr compile --bundle ."   'brokkr compile --bundle .'
run "coverage gate (exact 100% lines+branches+functions)"  'bash scripts/coverage-gate.sh'
run "cargo deny licenses"         'cargo deny check licenses'
# Webhook TLS (reqwest + rustls) must be the only HTTP/TLS story: native-tls
# may never re-enter the tree (e.g. via a reqwest default-feature regression).
run "no native-tls in the tree"   '! cargo tree -i native-tls >/dev/null 2>&1'
printf 'all gates passed (fmt, clippy, test, bundle, exact 100/100/100 coverage, deny licenses, no native-tls) with network denied' > "$notes"
write_result pass
