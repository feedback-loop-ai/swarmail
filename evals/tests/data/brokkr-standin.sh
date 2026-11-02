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
#                        | budget-over-pass | harness-error-over-pass | budget-over-unscorable
# EVAL_TEST_BROKKR_LOG   if set, one `swarmail-eval-request: PATH` line per
#                        invocation, then one `score=<result word>` line
#                        after the score seat has reported
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

# rewrite_row <outcome> <failure_class> <exit>: the way a runner's precedence
# legs would have left the slot (decision 0116, the agreement table). The
# deadline and infrastructure legs cannot be raced offline (0020 section 5),
# so the row, the slot's exit, the retained result.json and its digest in
# metadata.json are rewritten together, as provenance.rs's `rebind` does. No
# check is relaxed: the matrix still reads the whole chain it reads for any row.
rewrite_row() {
  local trials_root trial_id leaf
  trials_root="$(jq -r .trials_root "$request")"
  trial_id="$(jq -r .trial_id "$request")"
  leaf="$trials_root/$trial_id"
  jq -c --arg o "$1" --arg c "$2" '.outcome = $o | .failure_class = $c' "$slot/row.json" > "$work/row" &&
    cp "$work/row" "$slot/row.json" &&
    cp "$work/row" "$leaf/result.json" &&
    printf '%s\n' "$3" > "$slot/exit" &&
    jq -c --argjson len "$(wc -c < "$leaf/result.json" | tr -d '[:space:]')" \
      --arg sha "$(sha256sum "$leaf/result.json" | cut -d' ' -f1)" \
      '.result.length = $len | .result.sha256 = $sha' "$leaf/metadata.json" > "$work/meta" &&
    cp "$work/meta" "$leaf/metadata.json" || exit 70
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
  budget-over-pass) rewrite_row fail budget 1 ;;
  harness-error-over-pass) rewrite_row harness_error harness_error 2 ;;
  budget-over-unscorable)
    rewrite_row fail budget 1
    # The retained state is cut after it froze and its digest is left alone, so
    # the gate replays `unscorable` over a leaf the row says was validated.
    leaf="$(jq -r '.trials_root + "/" + .trial_id' "$request")"
    head -c 100 "$leaf/state.json" > "$work/state" && cp "$work/state" "$leaf/state.json"
    ;;
esac
verdict="$(seat score score.sh)"
[ -z "${EVAL_TEST_BROKKR_LOG:-}" ] || printf 'score=%s\n' "$verdict" >> "$EVAL_TEST_BROKKR_LOG"
if [ "$mode" = no-score ]; then
  rm -f "$slot/score.json"
fi
if [ "$mode" = bad-score ]; then
  jq -c '.business = (if .business == "pass" then "fail" else "pass" end)' "$slot/score.json" > "$work/score" &&
    cp "$work/score" "$slot/score.json"
fi
[ "$verdict" = unscorable ] && exit 1
exit 0
