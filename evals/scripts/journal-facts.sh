#!/usr/bin/env bash
# journal-facts.sh — turn the pinned brokkr's journal export into journal facts
# v1 (decision 0119), the closed shape `swarmail-evals report --journal` reads.
#
#   journal-facts.sh BROKKR DB OUT --run RUN [--run RUN]... TRIAL_ID...
#
#   BROKKR     absolute path to the pinned brokkr executable
#   DB         the journal (.brokkr/state/forge.db)
#   OUT        the facts file to create (never overwritten)
#   RUN        a run whose journal to read (`brokkr export --run`)
#   TRIAL_ID   the trial ids of the matrix being reported
#
# EXPORT SHAPE (brokkr 0.12.0 to 0.15.2, observed 2026-11-23). For each RUN the
# script runs `brokkr export --db DB --run RUN --out SCRATCH` in a private
# scratch directory and reads SCRATCH/RUN.ndjson: one event object per line,
# with `type`, `run_id`, top-level `attempt_id` on attempt events and a
# `payload`. A seat is named once, on `effect/requested.payload.seat`; its
# attempts are `effect/started`, `effect/checkpointed` (the fact in
# `payload.checkpoint`) and `effect/succeeded` (the seat's typed result in
# `payload.result`, whose `inputs.trial_id` names the trial). Events of
# different effects interleave; nothing is matched by order or time, only by
# (run, effect, attempt).
#
# OWNERSHIP. A trial is owned by the one succeeded attempt of a `trial` seat
# whose result names it. Only that attempt's checkpoints count: a retried or
# failed attempt of the same effect never adds to it. A trial id claimed by
# two attempts (one run or several) is `journal_ambiguous`; a trial no export
# carries is absent from OUT (the report types it `journal_missing`).
#
# FACTS. Every figure comes from the journal; nothing is estimated. All five
# facts per trial are typed {status: present, value} or {status: unavailable,
# reason}; `0` never stands in for an unavailable number:
#   * tokens: the attempt's session-final checkpoint (a step ending in
#     `-session-finished` that carries counts) gives input_tokens,
#     output_tokens and cache_read_tokens (`cached`); repeated identical finals
#     count once, differing ones are `journal_ambiguous`; per-turn rows
#     (`seat-turn`, `turn-completed`) never count;
#   * turns: that checkpoint's num_turns;
#   * deadline_killed: false for a succeeded owner (brokkr reports a deadline
#     kill as an indeterminate attempt, never a success);
#   * wall_time_ms and tool_calls: the export carries no bounded figure for
#     them, so they are `journal_has_no_accounting`.
#
# FAILURES. The script fails hard and typed: it exits 2 naming the class on
# stderr and writes no OUT when `brokkr export` fails or writes no
# RUN.ndjson (export_failed), a line is not JSON (export_not_json), the
# version is unreadable (export_version_unreadable) or an event does not have
# the shape above (export_shape_mismatch). OUT is published whole, or not at
# all.
#
# JOURNAL_EXPORT_DIR=DIR reads previously saved RUN.ndjson files instead of
# running `brokkr export`; their SHA-256s are then the recorded query.
set -u

die() { echo "journal-facts: ${2:+$2: }$1" >&2; exit 2; }

[ "$#" -ge 6 ] || die "usage: journal-facts.sh BROKKR DB OUT --run RUN [--run RUN]... TRIAL_ID..."
brokkr="$1"; db="$2"; out="$3"; shift 3
runs=()
while [ "${1:-}" = --run ]; do
  [ "$#" -ge 2 ] || die "--run needs a run id"
  runs+=("$2"); shift 2
done
[ "${#runs[@]}" -ge 1 ] || die "at least one --run RUN is required"
[ "$#" -ge 1 ] || die "at least one TRIAL_ID is required"
command -v jq >/dev/null 2>&1 || die "jq is unavailable"
command -v sha256sum >/dev/null 2>&1 || command -v shasum >/dev/null 2>&1 || die "no sha256 tool"
sha() { if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi; }
case "$brokkr" in /*) ;; *) die "BROKKR must be an absolute path" ;; esac
[ -f "$brokkr" ] && [ -x "$brokkr" ] || die "BROKKR is not an executable file"
case "$out" in /*) ;; *) die "OUT must be an absolute path" ;; esac
[ ! -e "$out" ] || die "OUT exists; it is never overwritten"
for run in "${runs[@]}"; do
  [[ "$run" =~ ^[A-Za-z0-9._-]{1,128}$ ]] || die "a run id is not a plain identifier"
done
for id in "$@"; do
  [[ "$id" =~ ^[A-Za-z0-9._-]{1,128}$ ]] || die "a trial id is not a plain identifier"
done

umask 077
work="$(mktemp -d "${TMPDIR:-/tmp}/journal-facts.XXXXXX")" || die "no scratch directory"
trap 'rm -rf "$work"' EXIT
mkdir "$work/export" || die "no scratch directory"

query_parts=()
for run in "${runs[@]}"; do
  if [ -n "${JOURNAL_EXPORT_DIR:-}" ]; then
    [ -f "$JOURNAL_EXPORT_DIR/$run.ndjson" ] || die "no saved export for a run" export_failed
    cp "$JOURNAL_EXPORT_DIR/$run.ndjson" "$work/export/$run.ndjson" || die "cannot read a saved export" export_failed
    query_parts+=("$run sha256:$(sha "$work/export/$run.ndjson")")
  else
    [ -f "$db" ] || die "DB is not a file"
    "$brokkr" export --db "$db" --run "$run" --out "$work/export" > /dev/null 2> "$work/export.err" ||
      die "brokkr export failed" export_failed
    [ -f "$work/export/$run.ndjson" ] || die "brokkr export wrote no RUN.ndjson" export_failed
    query_parts+=("$run")
  fi
  jq -e -s '.' "$work/export/$run.ndjson" > "$work/export/$run.json" 2>/dev/null ||
    die "the export is not JSON lines" export_not_json
done
if [ -n "${JOURNAL_EXPORT_DIR:-}" ]; then
  query="saved export ${query_parts[*]}"
else
  query="brokkr export --db $(basename "$db") --run ${query_parts[*]}"
fi

version="$("$brokkr" --version 2>/dev/null | head -n 1 | sed 's/^brokkr //')"
[ -n "$version" ] || die "cannot read the brokkr version" export_version_unreadable
digest="$(sha "$brokkr")"

# SHAPE: every event an object with a string type; effect events carry an
# object payload naming their effect, attempt events their attempt.
for run in "${runs[@]}"; do
  jq -e '
    length > 0 and all(.[];
      type == "object" and (.type | type == "string")
      and (if (.type | startswith("effect/")) then
             (.payload | type == "object") and (.payload.effect_id | type == "string")
             and (if .type == "effect/requested" then (.payload.seat | type == "string")
                  elif .type == "effect/checkpointed" then
                    (.attempt_id | type == "string") and (.payload.checkpoint | type == "object")
                  elif .type == "effect/succeeded" then (.attempt_id | type == "string")
                  else true end)
           else true end))
  ' "$work/export/$run.json" >/dev/null 2>&1 || die "the export does not have the shape this script reads" export_shape_mismatch
done

# EXTRACT: per run, the owners (succeeded trial-seat attempts naming a trial)
# and each owner's session-final checkpoints.
read -r -d '' OWNERS <<'JQ'
. as $events
| ([ $events[] | select(.type == "effect/requested") | {key: .payload.effect_id, value: .payload.seat} ]
   | from_entries) as $seat
| [ $events[]
    | select(.type == "effect/succeeded" and $seat[.payload.effect_id] == "trial")
    | select(.payload.result | type == "object")
    | select(.payload.result.inputs | type == "object")
    | select(.payload.result.inputs.trial_id | type == "string")
    | select(.payload.result.inputs.trial_id as $t | $ids | index($t))
    | {trial: .payload.result.inputs.trial_id, run: $run, effect: .payload.effect_id, attempt: .attempt_id} ] as $owners
| [ $owners[] | . as $o
    | $o + {finals: [ $events[]
        | select(.type == "effect/checkpointed" and .payload.effect_id == $o.effect and .attempt_id == $o.attempt)
        | .payload.checkpoint
        | select((.step | type) == "string" and (.step | endswith("-session-finished")))
        | select(has("input_tokens") or has("output_tokens") or has("num_turns")) ] } ]
JQ

read -r -d '' EXTRACT <<'JQ'
def num: if type == "number" and . >= 0 and . == floor then . else null end;
def unavailable($reason): {status: "unavailable", reason: $reason};
def present: {status: "present", value: .};
def none: unavailable("journal_has_no_accounting");
def all_unavailable($id; $run; $reason):
  {trial_id: $id, run_id: $run, seat: "trial", tokens: unavailable($reason),
   turns: unavailable($reason), wall_time_ms: unavailable($reason),
   deadline_killed: unavailable($reason), tool_calls: unavailable($reason)};
# One field over the owner's distinct finals: present when they agree.
def field($finals; $k):
  ([ $finals[] | .[$k] ] | unique) as $v
  | if ($v | length) == 1 and ($v[0] | num) != null then {ok: $v[0]}
    elif ($v | length) > 1 then {ambiguous: true}
    else {} end;
def entry($o):
  ($o.finals | unique) as $finals
  | (field($finals; "input_tokens")) as $in
  | (field($finals; "output_tokens")) as $outp
  | (field($finals; "cache_read_tokens")) as $cached
  | (field($finals; "num_turns")) as $turns
  | {trial_id: $o.trial, run_id: $o.run, seat: "trial",
     tokens: (if $in.ambiguous or $outp.ambiguous or $cached.ambiguous then unavailable("journal_ambiguous")
              elif ($in | has("ok")) and ($outp | has("ok")) and ($cached | has("ok"))
              then {input: $in.ok, output: $outp.ok, cached: $cached.ok} | present
              else none end),
     turns: (if $turns.ambiguous then unavailable("journal_ambiguous")
             elif ($turns | has("ok")) then $turns.ok | present else none end),
     wall_time_ms: none,
     deadline_killed: (false | present),
     tool_calls: none};
$ids[] as $id
| [ $owners[] | select(.trial == $id) ] as $found
| if ($found | length) == 0 then empty
  elif ($found | length) > 1 then all_unavailable($id; "ambiguous"; "journal_ambiguous")
  else entry($found[0]) end
JQ

printf '%s\n' "$@" | jq -R . | jq -s . > "$work/ids.json" || die "cannot read the trial ids"
: > "$work/owners"
for run in "${runs[@]}"; do
  jq -c --argjson ids "$(cat "$work/ids.json")" --arg run "$run" "$OWNERS | .[]" \
    "$work/export/$run.json" >> "$work/owners" 2>/dev/null ||
    die "the export does not match the shape this script reads" export_shape_mismatch
done
jq -s . "$work/owners" > "$work/owners.json" || die "cannot assemble the owners"
jq -n --argjson ids "$(cat "$work/ids.json")" --slurpfile owners "$work/owners.json" \
  "\$owners[0] as \$owners | $EXTRACT" > "$work/entries" 2>/dev/null ||
  die "the export does not match the shape this script reads" export_shape_mismatch
jq -s --arg version "$version" --arg digest "$digest" --arg query "$query" \
  '{schema_version: 1,
    source: {brokkr_version: $version, binary_sha256: $digest, query: $query},
    trials: .}' "$work/entries" > "$work/facts.json" || die "cannot assemble the facts"
# Exclusive, whole publication: staged beside OUT, then linked to its name,
# which fails when OUT appeared meanwhile.
stage="$(mktemp "$out.XXXXXX")" || die "OUT could not be created"
cat "$work/facts.json" > "$stage" && ln "$stage" "$out" 2>/dev/null
linked=$?
rm -f "$stage"
[ "$linked" -eq 0 ] || die "OUT could not be created"
echo "journal-facts: wrote $(jq '.trials | length' "$out") of $# trials"
