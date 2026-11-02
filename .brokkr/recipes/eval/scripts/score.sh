#!/usr/bin/env bash
# score.sh — the eval recipe's `review` gate (decision 0119, R4): the scoring gate
# that is the recipe's protected review. The file keeps its name; nothing in it
# reads the seat or phase name.
#
# Re-applies the task's deterministic check to the app's retained state: it
# runs `swarmail-evals check --artifacts-dir <trials_root>/<trial_id>`, which
# validates the leaf the trial retained (metadata, result, state and
# diagnostics, bound by digest) and re-scores state.json. The agent's account
# is not an input and this gate reads nothing the agent could have written
# after the trial froze the app.
#
#   score.sh <prompt_file>
#
# Writes <slot>/score.json = {"schema_version":1,"trial_id":...,
# "business":"pass|fail|unscorable","check_exit":N}: the business replay of the
# retained state and nothing else. It then reads <slot>/row.json for its two
# class fields only (outcome, failure_class) and reports the trial's class by
# the agreement table (spec, "Revision 3"; accepted 0055 section 2 and 0071
# section 2 order the row's precedence: infrastructure, then a live deadline as
# fail/budget, then the business check):
#
#   row                      agreeing replay        the seat reports
#   pass                     pass                   pass
#   fail, class not budget   fail                   fail
#   fail/budget              pass or fail           fail
#   harness_error            pass, fail, unscorable unscorable
#   anything else (a contradiction, or an unreadable row)  unscorable
#
# The row never enters the business verdict; it chooses only the result word,
# so the journal's run outcome and the committed row do not disagree. The
# matrix mirrors this table in Rust (gate_refusal), which stays the authority
# that commits or refuses a slot. No path, secret or child byte reaches a
# result's notes.
set -u

prompt_file="${1:-}"
[ -f "$prompt_file" ] || { echo "score: prompt file missing" >&2; exit 2; }

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
[ -n "$result_path" ] || { echo "score: result path missing" >&2; exit 2; }
mkdir -p "$(dirname "$result_path")" || exit 2

report() {
  local tmp="$result_path.tmp.$$"
  if command -v jq >/dev/null 2>&1; then
    jq -n --arg result "$1" --arg notes "$2" \
      '{result: $result, inputs: {}, notes: $notes}' > "$tmp" || { rm -f "$tmp"; exit 2; }
  else
    printf '%s\n' '{"result":"unscorable","inputs":{},"notes":"jq is unavailable; nothing was scored"}' > "$tmp" || exit 2
  fi
  mv -f "$tmp" "$result_path" || { rm -f "$tmp"; exit 2; }
}
refuse() { report unscorable "$1"; exit 0; }

command -v jq >/dev/null 2>&1 || refuse "jq is unavailable; nothing was scored"
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
' "$request" >/dev/null 2>&1 || refuse "the request is malformed"

trial_id="$(jq -r '.trial_id' "$request")"
slot="$(jq -r '.slot' "$request")"
trials_root="$(jq -r '.trials_root' "$request")"
evals_bin="$(jq -r '.evals_bin' "$request")"
[ -d "$slot" ] && [ ! -L "$slot" ] || refuse "the slot directory is unavailable"
[ -f "$evals_bin" ] && [ -x "$evals_bin" ] || refuse "the evals executable is unavailable"

env -i PATH=/usr/bin:/bin LANG=C LC_ALL=C \
  "$evals_bin" check --artifacts-dir "$trials_root/$trial_id" >/dev/null 2>&1 </dev/null
code=$?
case "$code" in
  0) business=pass ;;
  1) business=fail ;;
  *) business=unscorable ;;
esac

tmp="$slot/score.json.tmp.$$"
jq -n -c --arg id "$trial_id" --arg business "$business" --argjson code "$code" \
  '{schema_version: 1, trial_id: $id, business: $business, check_exit: $code}' > "$tmp" &&
  mv -f "$tmp" "$slot/score.json" || { rm -f "$tmp"; refuse "the verdict could not be recorded"; }

# The row's two class fields, read strictly: a regular file, not a symlink, at
# most 64 KiB, exactly one line. Anything else leaves the seat unscorable with
# the verdict above still written.
row="$slot/row.json"
[ -f "$row" ] && [ ! -L "$row" ] || { report unscorable "the slot's row is not a regular file"; exit 0; }
row_size="$(wc -c < "$row" 2>/dev/null | tr -d '[:space:]')"
case "$row_size" in ''|*[!0-9]*) report unscorable "the slot's row size is unreadable"; exit 0 ;; esac
[ "$row_size" -le 65536 ] || { report unscorable "the slot's row is larger than 64 KiB"; exit 0; }
[ "$(grep -c '' "$row")" -eq 1 ] ||
  { report unscorable "the slot's row is not exactly one line"; exit 0; }
jq -e '(.outcome | IN("pass","fail","harness_error")) and (.failure_class | type | IN("string","null"))' \
  "$row" >/dev/null 2>&1 || { report unscorable "the slot's row carries no usable outcome"; exit 0; }
row_outcome="$(jq -r '.outcome' "$row")"
row_class="$(jq -r '.failure_class // ""' "$row")"

case "$row_outcome:$row_class:$business" in
  pass::pass) report pass "the row is a pass and the retained state replays pass" ;;
  harness_error:*:*) report unscorable "the row is a harness error; the retained state replays $business" ;;
  fail:budget:pass | fail:budget:fail) report fail "the row is a budget failure over a validated state; a failed trial is data" ;;
  fail:budget:*) report unscorable "the row is a budget failure but the retained state replays $business" ;;
  fail:*:fail) report fail "the row is a business failure and the retained state replays fail; a failed trial is data" ;;
  *) report unscorable "the row is $row_outcome and the retained state replays $business" ;;
esac
exit 0
