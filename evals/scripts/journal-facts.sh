#!/usr/bin/env bash
# journal-facts.sh — turn the pinned brokkr's journal export into journal facts
# v1 (decision 0116), the closed shape `swarmail-evals report --journal` reads.
#
#   journal-facts.sh BROKKR DB OUT TRIAL_ID...
#
#   BROKKR     absolute path to the pinned brokkr executable
#   DB         the journal (.brokkr/state/forge.db)
#   OUT        the facts file to create (never overwritten)
#   TRIAL_ID   the trial ids of the matrix being reported
#
# Every figure comes from the journal; nothing is estimated. A trial the export
# does not carry is simply absent from OUT (the report types it
# `journal_missing`); a trial claimed by two runs is `journal_ambiguous`; a run
# whose seat carries no accounting is `journal_has_no_accounting`. `0` never
# stands in for an unavailable fact.
#
# EXPORT SHAPE. brokkr was not reachable when this was written, so the one
# thing this script assumes about `brokkr export` is isolated in EXTRACT below:
# a JSON document (or JSON lines) holding `runs[]`, each with `run_id` and
# `seats[]`; a seat has `seat`, `result`, `inputs` and, when the journal
# accounted it, `usage` = {input_tokens, output_tokens, cached_tokens, turns,
# wall_time_ms, deadline_killed, tool_calls[]{tool, outcome, argument_bytes,
# argument_sha256, result_bytes, result_sha256}}. When the real export is
# observed, only EXTRACT changes; a mismatch makes every trial
# `journal_missing`, visibly, never a number.
#
# JOURNAL_EXPORT_FILE=PATH reads a previously saved export instead of running
# `brokkr export`; the file's SHA-256 is then the recorded query.
set -u

die() { echo "journal-facts: $1" >&2; exit 2; }

[ "$#" -ge 4 ] || die "usage: journal-facts.sh BROKKR DB OUT TRIAL_ID..."
brokkr="$1"; db="$2"; out="$3"; shift 3
command -v jq >/dev/null 2>&1 || die "jq is unavailable"
command -v sha256sum >/dev/null 2>&1 || command -v shasum >/dev/null 2>&1 || die "no sha256 tool"
sha() { if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi; }
case "$brokkr" in /*) ;; *) die "BROKKR must be an absolute path" ;; esac
[ -f "$brokkr" ] && [ -x "$brokkr" ] || die "BROKKR is not an executable file"
case "$out" in /*) ;; *) die "OUT must be an absolute path" ;; esac
[ ! -e "$out" ] || die "OUT exists; it is never overwritten"
for id in "$@"; do
  [[ "$id" =~ ^[A-Za-z0-9._-]{1,128}$ ]] || die "a trial id is not a plain identifier"
done

work="$(mktemp -d "${TMPDIR:-/tmp}/journal-facts.XXXXXX")" || die "no scratch directory"
trap 'rm -rf "$work"' EXIT

if [ -n "${JOURNAL_EXPORT_FILE:-}" ]; then
  [ -f "$JOURNAL_EXPORT_FILE" ] || die "JOURNAL_EXPORT_FILE is not a file"
  cp "$JOURNAL_EXPORT_FILE" "$work/export" || die "cannot read the export"
  query="saved export sha256:$(sha "$work/export")"
else
  [ -f "$db" ] || die "DB is not a file"
  "$brokkr" export --db "$db" > "$work/export" 2> "$work/export.err" || die "brokkr export failed"
  query="brokkr export --db $(basename "$db")"
fi
jq -e -s '.' "$work/export" > "$work/export.json" 2>/dev/null || die "the export is not JSON"

version="$("$brokkr" --version 2>/dev/null | head -n 1 | sed 's/^brokkr //')"
[ -n "$version" ] || die "cannot read the brokkr version"
digest="$(sha "$brokkr")"

# EXTRACT: the only code that knows the export's shape.
read -r -d '' EXTRACT <<'JQ'
def num: if type == "number" and . >= 0 and . == floor then . else null end;
def unavailable($reason): {status: "unavailable", reason: $reason};
def present: {status: "present", value: .};
def none: unavailable("journal_has_no_accounting");
def calls($c):
  ($c // []) as $list
  | if ($list | type) == "array"
       and ($list | all(.[];
            (.tool | type == "string" and length > 0)
            and (.outcome | IN("ok", "error", "invalid_argument"))
            and (.argument_bytes | num != null) and (.result_bytes | num != null)
            and (.argument_sha256 | type == "string" and test("^[0-9a-f]{64}$"))
            and (.result_sha256 | type == "string" and test("^[0-9a-f]{64}$"))))
    then [$list | to_entries[] | .value + {ordinal: (.key + 1)}
          | {ordinal, tool, outcome, argument_bytes, argument_sha256, result_bytes, result_sha256}]
    else null end;
def entry($id; $run; $u):
  if $u == null or ($u | type) != "object" then
    {trial_id: $id, run_id: $run, seat: "trial", tokens: none, turns: none,
     wall_time_ms: none, deadline_killed: false, tool_calls: []}
  else
    (calls($u.tool_calls)) as $c
    | if $c == null then
        {trial_id: $id, run_id: $run, seat: "trial", tokens: none, turns: none,
         wall_time_ms: none, deadline_killed: false, tool_calls: []}
      else
        {trial_id: $id, run_id: $run, seat: "trial",
         tokens: (if ($u.input_tokens | num) != null and ($u.output_tokens | num) != null
                       and ($u.cached_tokens | num) != null
                  then {input: $u.input_tokens, output: $u.output_tokens, cached: $u.cached_tokens} | present
                  else none end),
         turns: (if ($u.turns | num) != null then $u.turns | present else none end),
         wall_time_ms: (if ($u.wall_time_ms | num) != null then $u.wall_time_ms | present else none end),
         deadline_killed: ($u.deadline_killed == true),
         tool_calls: $c}
      end
  end;
[ .[] | (.runs // [])[] | . as $run
  | (.seats // [])[] | select(.seat == "trial" and (.inputs.trial_id | type == "string"))
  | {id: .inputs.trial_id, run: ($run.run_id | tostring), usage: .usage} ] as $seats
| $ids[] as $id
| [ $seats[] | select(.id == $id) ] as $found
| if ($found | length) == 0 then empty
  elif ($found | length) > 1 then
    {trial_id: $id, run_id: "ambiguous", seat: "trial",
     tokens: unavailable("journal_ambiguous"), turns: unavailable("journal_ambiguous"),
     wall_time_ms: unavailable("journal_ambiguous"), deadline_killed: false, tool_calls: []}
  else $found[0] as $one | entry($id; ($one.run | if test("^[A-Za-z0-9._-]{1,128}$") then . else "unsafe" end); $one.usage) end
JQ

printf '%s\n' "$@" | jq -R . | jq -s . > "$work/ids.json"
jq --argjson ids "$(cat "$work/ids.json")" "$EXTRACT" "$work/export.json" > "$work/entries" 2>/dev/null ||
  die "the export does not match the shape this script reads"
jq -s --arg version "$version" --arg digest "$digest" --arg query "$query" \
  '{schema_version: 1,
    source: {brokkr_version: $version, binary_sha256: $digest, query: $query},
    trials: .}' "$work/entries" > "$work/facts.json" || die "cannot assemble the facts"
( set -o noclobber; cat "$work/facts.json" > "$out" ) 2>/dev/null || die "OUT could not be created"
echo "journal-facts: wrote $(jq '.trials | length' "$out") of $# trials"
