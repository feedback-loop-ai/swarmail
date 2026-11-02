#!/usr/bin/env bash
# trial.sh — the eval recipe's `trial` seat (decision 0116, interim path).
#
# An opaque exec driver: it runs `swarmail-evals trial` from the request the
# matrix wrote before it spawned `brokkr run --recipe eval`. That trial starts
# a fresh Swarmail and a fresh fixture as its own children, runs the agent
# (for a paid trial the real `brokkr driver claude|codex|dsh`, through the
# wrapper's --via-brokkr), saves the app's state, and stops both servers.
# Nothing here is confined; every row records the trial as `unconfined`.
#
#   trial.sh <prompt_file>
#
# The prompt carries the seat's result path (its last trimmed line that is an
# absolute `/...json` path) and the request (its last trimmed line holding
# `swarmail-eval-request: /abs/request.json`, bare or after `Feature: `). The script writes the trial's single row to <slot>/row.json, the
# trial's exit code to <slot>/exit and a bounded copy of its diagnostics to
# <slot>/trial.stderr, then reports:
#
#   {"result":"ran","inputs":{"trial_id":"..."},"notes":"..."}   exit 0, 1 or 2
#                                                                with one row
#   {"result":"harness_error","inputs":{},"notes":"..."}         anything else
#
# It never reads or prints the row's content, and no path, secret or child
# byte reaches a result's notes.
set -u

prompt_file="${1:-}"
[ -f "$prompt_file" ] || { echo "trial: prompt file missing" >&2; exit 2; }

result_path=""
request=""
# The engine renders the feature as "Feature: <text>" and the result path as an
# indented line of its own (observed 2026-11-02), so each line is trimmed of
# surrounding whitespace and a trailing CR first, as verify-seat.sh does, and
# the request token is matched anywhere on the trimmed line.
while IFS= read -r line || [ -n "$line" ]; do
  line="${line%$'\r'}"
  line="${line#"${line%%[![:space:]]*}"}"
  line="${line%"${line##*[![:space:]]}"}"
  case "$line" in
    /*.json) result_path="$line" ;;
    *swarmail-eval-request:\ /*.json) request="${line#*swarmail-eval-request: }" ;;
  esac
done < "$prompt_file"
[ -n "$result_path" ] || { echo "trial: result path missing" >&2; exit 2; }
mkdir -p "$(dirname "$result_path")" || exit 2

# report <result> <trial_id|""> <notes>: a sibling temporary, then a rename.
report() {
  local tmp="$result_path.tmp.$$"
  if command -v jq >/dev/null 2>&1; then
    jq -n --arg result "$1" --arg id "$2" --arg notes "$3" \
      '{result: $result, inputs: (if $id == "" then {} else {trial_id: $id} end), notes: $notes}' \
      > "$tmp" || { rm -f "$tmp"; exit 2; }
  else
    printf '%s\n' '{"result":"harness_error","inputs":{},"notes":"jq is unavailable; the trial was not run"}' > "$tmp" || exit 2
  fi
  mv -f "$tmp" "$result_path" || { rm -f "$tmp"; exit 2; }
}
refuse() { report harness_error "" "$1"; exit 0; }

command -v jq >/dev/null 2>&1 || refuse "jq is unavailable; the trial was not run"
[ -n "$request" ] || refuse "the prompt carries no request line"
[ -f "$request" ] && [ ! -L "$request" ] || refuse "the request is not a regular file"
size="$(wc -c < "$request" 2>/dev/null | tr -d '[:space:]')"
case "$size" in ''|*[!0-9]*) refuse "the request size is unreadable" ;; esac
[ "$size" -le 65536 ] || refuse "the request is larger than 64 KiB"
jq -e '
  type == "object" and .schema_version == 1
  and (.trial_id | type == "string" and test("^[A-Za-z0-9._-]{1,128}$"))
  and (.slot | type == "string" and startswith("/") and (index("\u0000") | not))
  and (.trials_root | type == "string" and startswith("/") and (index("\u0000") | not))
  and (.evals_bin | type == "string" and startswith("/") and (index("\u0000") | not))
  and (.swarmail_bin | type == "string" and startswith("/") and (index("\u0000") | not))
  and (.args | type == "array" and all(.[]; type == "string" and (index("\u0000") | not)))
' "$request" >/dev/null 2>&1 || refuse "the request is malformed"

trial_id="$(jq -r '.trial_id' "$request")"
slot="$(jq -r '.slot' "$request")"
evals_bin="$(jq -r '.evals_bin' "$request")"
swarmail_bin="$(jq -r '.swarmail_bin' "$request")"
args=()
while IFS= read -r -d '' arg; do args+=("$arg"); done < <(jq -j '.args[] + "\u0000"' "$request")

[ -d "$slot" ] && [ ! -L "$slot" ] || refuse "the slot directory is unavailable"
[ -f "$evals_bin" ] && [ -x "$evals_bin" ] || refuse "the evals executable is unavailable"
[ -f "$swarmail_bin" ] && [ -x "$swarmail_bin" ] || refuse "the swarmail executable is unavailable"

# Exclusive create: a second run of one slot must never overwrite its row.
( set -o noclobber; : > "$slot/row.json" ) 2>/dev/null || refuse "the slot already holds a row"
rm -f "$slot/exit" "$slot/trial.stderr"

env -i PATH=/usr/bin:/bin LANG=C LC_ALL=C SWARMAIL_BIN="$swarmail_bin" \
  "$evals_bin" trial ${args[@]+"${args[@]}"} 2>&1 >"$slot/row.json" </dev/null |
  { head -c 65536 > "$slot/trial.stderr"; cat > /dev/null; }
code="${PIPESTATUS[0]}"
printf '%s\n' "$code" > "$slot/exit" || refuse "the exit code could not be recorded"

case "$code" in 0|1|2) ;; *) refuse "the trial ended outside the 0, 1, 2 contract (exit $code)" ;; esac
lines="$(wc -l < "$slot/row.json" | tr -d '[:space:]')"
bytes="$(wc -c < "$slot/row.json" | tr -d '[:space:]')"
if [ "$lines" != 1 ] || [ "${bytes:-0}" -le 1 ]; then
  refuse "the trial left no single-line row (exit $code)"
fi
report ran "$trial_id" "trial ran; exit $code; unconfined; the row is in the slot"
exit 0
