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
test_command='cargo test --workspace'
lint_command='cargo clippy --workspace --all-targets -- -D warnings'
if [ -z "$test_command" ] || [ -z "$lint_command" ]; then
  printf 'no stack was recognized; fill in scripts/verify-seat.sh before running this gate' > "$notes"
  write_result fail; exit 0
fi
run "$test_command" "$test_command"
run "$lint_command" "$lint_command"
printf '%s and %s passed with network denied' "$test_command" "$lint_command" > "$notes"
write_result pass
