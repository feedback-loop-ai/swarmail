#!/usr/bin/env bash
# A stand-in for `brokkr run --recipe eval --feature "swarmail-eval-request: PATH"`
# (decision 0116). It plays the engine's part and nothing else: it synthesizes
# the prompt file each seat receives, runs the recipe's real trial.sh and
# score.sh in policy order, and exits. The engine itself is not present in
# delivery; the operator's first engine run is its proof.
#
# EVAL_TEST_RECIPE_DIR   the recipe directory under test (required)
# EVAL_TEST_BROKKR_MODE  ok | no-row | foreign-row | bad-exit | bad-score | no-score
#                        | category-stderr | hang-after-start
# EVAL_TEST_BROKKR_LOG   if set, one line per invocation: the feature argument
# EVAL_TEST_BROKKR_READY if set, hang-after-start writes its pid here
set -u
[ "${1:-}" = run ] && [ "${2:-}" = --recipe ] && [ "${3:-}" = eval ] && [ "${4:-}" = --feature ] || exit 64
feature="${5:-}"
case "$feature" in
  "swarmail-eval-request: /"*.json) request="${feature#swarmail-eval-request: }" ;;
  *) exit 64 ;;
esac
[ -z "${EVAL_TEST_BROKKR_LOG:-}" ] || printf '%s\n' "$feature" >> "$EVAL_TEST_BROKKR_LOG"
mode="${EVAL_TEST_BROKKR_MODE:-ok}"
slot="$(dirname "$request")"
recipe="${EVAL_TEST_RECIPE_DIR:?}"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

seat() { # seat <name> <script>: prints the seat's result word
  # The prompt is rendered the way the engine renders it (observed 2026-11-02):
  # the feature text after "Feature: " and the result path on an indented line.
  local prompt="$work/$1.prompt" result="$work/$1.result.json"
  printf '# Seat %s\n\nFeature: swarmail-eval-request: %s\nPhase: %s\n\nWrite your result to exactly this file:\n\n    %s\n' \
    "$1" "$request" "$1" "$result" > "$prompt"
  bash "$recipe/scripts/$2" "$prompt" >&2 || return 1
  jq -r .result "$result"
}

case "$mode" in
  no-row) exit 3 ;;
  hang-after-start)
    [ -z "${EVAL_TEST_BROKKR_READY:-}" ] || printf '%s' "$$" > "$EVAL_TEST_BROKKR_READY"
    exec sleep 600
    ;;
esac

[ "$(seat trial trial.sh)" = ran ] || exit 1
case "$mode" in
  foreign-row)
    jq -c '.trial_id = "00000000000000000000000000000000"' "$slot/row.json" > "$work/row" &&
      cp "$work/row" "$slot/row.json"
    ;;
  bad-exit) printf '2\n' > "$slot/exit" ;;
  category-stderr) printf 'swarmail-evals: harness error: concurrent_trials_refused\n' >> "$slot/trial.stderr" ;;
esac
verdict="$(seat score score.sh)"
if [ "$mode" = no-score ]; then
  rm -f "$slot/score.json"
fi
if [ "$mode" = bad-score ]; then
  jq -c '.outcome = (if .outcome == "pass" then "fail" else "pass" end)' "$slot/score.json" > "$work/score" &&
    cp "$work/score" "$slot/score.json"
fi
[ "$verdict" = unscorable ] && exit 1
exit 0
