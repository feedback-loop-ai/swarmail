#!/usr/bin/env bash
# The one entry point of the fair mail comparison (issue #163, proposed
# decision 0091; method in docs/evals/README.md). Thin preflight and
# dispatch only: the bench driver owns every server it launches, and no
# mode here starts a paid agent.
#
#   compare-mail.sh local    --output DIR [--protocol P] [--sources S]
#                            [--mailpit BIN] [--mailhog BIN]      (smoke, never evidence)
#   compare-mail.sh collect  --output DIR --mailpit BIN --mailhog BIN
#                            [--protocol P] [--sources S]   (the frozen protocol, then admission)
#   compare-mail.sh controls --output DIR [PREREQUISITES]
#   compare-mail.sh evidence --output DIR --final DIR [PREREQUISITES]
#   compare-mail.sh measure  --output DIR --config FILE [PREREQUISITES] [--dry-run]
#
# PREREQUISITES, checked in this order, the first missing one named in the
# refusal: --baseline, --contracts (final MCP contracts), --controls-dir,
# --receipts (#148/#201 coordination), --cost, --identity, --product.
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
contracts='' controls_dir='' cost='' identity='' product=''
while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run) dry_run=1; shift; continue ;;
    --protocol|--sources|--output|--mailpit|--mailhog|--baseline|--final|--receipts|--config|--contracts|--controls-dir|--cost|--identity|--product)
      [ $# -ge 2 ] || blocked arguments "$1 needs a value"
      flag="${1#--}"
      flag="${flag//-/_}"
      printf -v "$flag" '%s' "$2"
      shift 2 ;;
    *) blocked arguments "unknown argument: $1" ;;
  esac
done

case "$mode" in
  local|collect|controls|evidence|measure) ;;
  *) usage; blocked arguments "unknown mode: $mode" ;;
esac
[ -n "$output" ] || blocked arguments "--output DIR is required"
[ ! -e "$output" ] || blocked prerequisites "--output exists; runs are never overwritten or resumed"
[ -f "$protocol" ] || blocked prerequisites "protocol $protocol is missing"
[ -f "$sources" ] || blocked prerequisites "source lock $sources is missing"

# T10's prerequisites in their fixed order. Each is a path the operator
# supplies; a missing, absent or empty one is refused by name before any
# later one is looked at. This script only stats the paths, never reads
# them: the genuine contents (the historical #151 baseline, the final
# contracts, free controls, #148/#201 receipts, cost basis, identity and
# the full product) are produced and verified by the eval owners, so a
# supplied path is necessary and never sufficient.
PREREQUISITES=(baseline final-contracts controls coordination-receipts cost-basis identity full-product)

prerequisite_path() {
  case "$1" in
    baseline) printf '%s' "$baseline" ;;
    final-contracts) printf '%s' "$contracts" ;;
    controls) printf '%s' "$controls_dir" ;;
    coordination-receipts) printf '%s' "$receipts" ;;
    cost-basis) printf '%s' "$cost" ;;
    identity) printf '%s' "$identity" ;;
    full-product) printf '%s' "$product" ;;
  esac
}

prerequisite_flag() {
  case "$1" in
    baseline) echo --baseline ;;
    final-contracts) echo --contracts ;;
    controls) echo --controls-dir ;;
    coordination-receipts) echo --receipts ;;
    cost-basis) echo --cost ;;
    identity) echo --identity ;;
    full-product) echo --product ;;
  esac
}

prerequisite_chain() {
  local name path
  for name in "${PREREQUISITES[@]}"; do
    path=$(prerequisite_path "$name")
    [ -n "$path" ] || blocked "prerequisite-$name" "$(prerequisite_flag "$name") is not supplied"
    if [ -d "$path" ]; then
      continue
    fi
    [ -s "$path" ] || blocked "prerequisite-$name" "$(prerequisite_flag "$name") $path does not exist or is empty"
  done
}

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
  local|collect)
    if [ "$mode" = collect ]; then
      [ -n "$mailpit" ] && [ -n "$mailhog" ] \
        || blocked competitor-binaries "collect measures all three declared targets; pass --mailpit and --mailhog with their pinned executables"
    fi
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
    if [ "$mode" = collect ]; then
      # Collection never starts on a protocol the driver would refuse.
      "$bench" check --protocol "$protocol" --sources "$sources" \
        || blocked collection-ready "the protocol and lock are not ready for collection (see above)"
      run_mode=collect
    else
      "$bench" check --protocol "$protocol" --sources "$sources" || true
      run_mode=smoke
    fi
    commit=$(git rev-parse HEAD 2>/dev/null || true)
    dirty=false
    [ -z "$(git status --porcelain 2>/dev/null)" ] || dirty=true
    args=(run --mode "$run_mode" --protocol "$protocol" --sources "$sources"
      --output "$output" --scratch "$output.scratch"
      --run-id "$mode-$(date -u +%Y%m%dT%H%M%SZ)-$$"
      --bin "swarmail=target/release/swarmail")
    [ -z "$mailpit" ] || args+=(--bin "mailpit=$mailpit")
    [ -z "$mailhog" ] || args+=(--bin "mailhog=$mailhog")
    [ -z "$commit" ] || args+=(--commit "$commit" --dirty "$dirty")
    "$bench" "${args[@]}"
    status=$?
    [ "$status" -le 2 ] || status=2
    if [ "$mode" = collect ] && [ "$status" -le 1 ]; then
      # A collected run counts only after admission against its protocol.
      "$bench" admit "$output" "$protocol" || status=2
    fi
    [ "$status" -ne 2 ] || printf 'compare-mail: blocked at phase "%s-run": see the driver output above\n' "$mode" >&2
    exit "$status"
    ;;
  controls)
    prerequisite_chain
    eval_prerequisite
    ;;
  evidence)
    [ -n "$final" ] || blocked arguments "--final is required in evidence mode"
    [ -e "$final" ] || blocked prerequisites "--final $final does not exist"
    prerequisite_chain
    eval_prerequisite
    ;;
  measure)
    # Operator-only: delegates the final product to the eval owners after
    # separately granted authority. Neither a dry run nor a refusal opens
    # the wrapper configuration or launches a trial; the prerequisites are
    # only stat-ed, in their fixed order.
    [ -n "$config" ] || blocked arguments "--config is required in measure mode"
    [ -e "$config" ] || blocked prerequisites "--config does not exist"
    if [ "$dry_run" = 1 ]; then
      printf 'compare-mail measure (dry run): prerequisite order: %s. Each is refused by name when missing; the delegated final Swarmail/Mailpit product starts only after all hold. Nothing was opened or launched.\n' "${PREREQUISITES[*]}"
    fi
    prerequisite_chain
    eval_prerequisite
    ;;
esac
