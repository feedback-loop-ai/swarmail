#!/usr/bin/env bash
# The one entry point of the fair mail comparison (issue #163, proposed
# decision 0091; method in docs/evals/README.md). Thin preflight and
# dispatch only: the bench driver owns every server it launches, and no
# mode here starts a paid agent.
#
#   compare-mail.sh local    --output DIR [--protocol P] [--sources S]
#                            [--mailpit BIN] [--mailhog BIN]
#   compare-mail.sh controls --output DIR [--protocol P] [--sources S]
#   compare-mail.sh evidence --output DIR --baseline DIR --final DIR
#                            --receipts FILE [--protocol P] [--sources S]
#   compare-mail.sh measure  --output DIR --baseline DIR --config FILE
#                            --receipts FILE [--dry-run]
#
# Exit 0: the mode completed its declared checks. Exit 1: a sealed run
# whose workload failed somewhere (the report says where). Exit 2: a
# prerequisite, operational, interrupt or cleanup failure; the message
# names the blocked phase. A blocked mode never downgrades to another.
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit 2

usage() {
  awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' "${BASH_SOURCE[0]}" >&2
}

blocked() {
  printf 'compare-mail: blocked at phase "%s": %s\n' "$1" "$2" >&2
  exit 2
}

mode="${1:-}"
if [ -z "$mode" ]; then
  usage
  exit 2
fi
shift

protocol=docs/evals/protocol.json
sources=docs/evals/sources.lock.json
output='' mailpit='' mailhog='' baseline='' final='' receipts='' config='' dry_run=0
while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run) dry_run=1; shift; continue ;;
    --protocol|--sources|--output|--mailpit|--mailhog|--baseline|--final|--receipts|--config)
      [ $# -ge 2 ] || blocked arguments "$1 needs a value"
      flag="${1#--}"
      printf -v "$flag" '%s' "$2"
      shift 2 ;;
    *) blocked arguments "unknown argument: $1" ;;
  esac
done

case "$mode" in
  local|controls|evidence|measure) ;;
  *) usage; blocked arguments "unknown mode: $mode" ;;
esac
[ -n "$output" ] || blocked arguments "--output DIR is required"
[ ! -e "$output" ] || blocked prerequisites "--output exists; runs are never overwritten or resumed"
[ -f "$protocol" ] || blocked prerequisites "protocol $protocol is missing"
[ -f "$sources" ] || blocked prerequisites "source lock $sources is missing"

# The eval prerequisite every integrated mode shares: the standalone eval
# crate must build. Conflict syntax is not a build, and a private lock or a
# filtered suite is not the #148/#201 shared-host proof either.
eval_prerequisite() {
  local conflicted
  conflicted=$(grep -rlE '^(<<<<<<<|>>>>>>>)( |$)' evals/src evals/tests 2>/dev/null | sort | tr '\n' ' ')
  if [ -n "$conflicted" ]; then
    blocked eval-build "unresolved conflict syntax in $conflicted(#148/#201 prerequisite)"
  fi
  blocked comparison-integration "the eval crate has no Swarmail/Mailpit comparison target or envelope yet (tasks T7-T9)"
}

case "$mode" in
  local)
    for t in mailpit mailhog; do
      if grep -q "\"$t\"" "$protocol"; then
        bin="${!t}"
        [ -n "$bin" ] || blocked competitor-binaries "the protocol declares $t; pass --$t with its pinned executable"
        [ -x "$bin" ] || blocked competitor-binaries "--$t $bin is not an executable file"
      fi
    done
    command -v cargo >/dev/null 2>&1 || blocked prerequisites "cargo is not installed"
    cargo build --release --locked --offline --bin swarmail >&2 \
      || blocked build "the candidate binary does not build"
    bench=$(cargo bench --locked --offline --no-run --bench mail_compare --message-format=json 2>/dev/null \
      | grep -o '"executable":"[^"]*mail_compare[^"]*"' | tail -n1 | cut -d'"' -f4)
    [ -n "$bench" ] && [ -x "$bench" ] || blocked build "the mail_compare driver does not build"
    "$bench" check --protocol "$protocol" --sources "$sources" || true
    commit=$(git rev-parse HEAD 2>/dev/null || true)
    dirty=false
    [ -z "$(git status --porcelain 2>/dev/null)" ] || dirty=true
    args=(run --mode smoke --protocol "$protocol" --sources "$sources"
      --output "$output" --scratch "$output.scratch"
      --run-id "local-$(date -u +%Y%m%dT%H%M%SZ)-$$"
      --bin "swarmail=target/release/swarmail")
    [ -z "$mailpit" ] || args+=(--bin "mailpit=$mailpit")
    [ -z "$mailhog" ] || args+=(--bin "mailhog=$mailhog")
    [ -z "$commit" ] || args+=(--commit "$commit" --dirty "$dirty")
    "$bench" "${args[@]}"
    status=$?
    [ "$status" -le 2 ] || status=2
    [ "$status" -ne 2 ] || printf 'compare-mail: blocked at phase "local-run": see the driver output above\n' >&2
    exit "$status"
    ;;
  controls)
    eval_prerequisite
    ;;
  evidence)
    for need in baseline final receipts; do
      [ -n "${!need}" ] || blocked arguments "--$need is required in evidence mode"
      [ -e "${!need}" ] || blocked prerequisites "--$need ${!need} does not exist"
    done
    eval_prerequisite
    ;;
  measure)
    # Operator-only: delegates the final product to the eval owners after
    # separately granted authority. Neither a dry run nor a refusal opens
    # the wrapper configuration or launches a trial.
    for need in baseline config receipts; do
      [ -n "${!need}" ] || blocked arguments "--$need is required in measure mode"
      [ -e "${!need}" ] || blocked prerequisites "--$need does not exist"
    done
    if [ "$dry_run" = 1 ]; then
      printf 'compare-mail measure (dry run): would validate the genuine baseline, contracts and gate receipts, run fresh free controls, then delegate the final Swarmail/Mailpit product to the eval owners. Nothing was opened or launched.\n'
    fi
    eval_prerequisite
    ;;
esac
