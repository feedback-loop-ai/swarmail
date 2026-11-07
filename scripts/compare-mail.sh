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
#   compare-mail.sh controls --output DIR --product P --mailpit BIN --mailhog BIN
#                            --mailslurper BIN                (free oracle and null rows)
#   compare-mail.sh evidence --output DIR --final DIR [PREREQUISITES]
#   compare-mail.sh measure  --output DIR --config FILE [PREREQUISITES]
#                            (--dry-run | --authorized-product ID
#                             --mailpit BIN --mailhog BIN --mailslurper BIN)
#
# `controls` needs only the product record and the three executables: it
# builds the candidate and the adapter driver, probes each target's MCP
# contract, runs the builtin oracle and null agents on the hard tier against
# each of the four targets (one worker, each target one named command under
# `timeout 600`, with uptime before and after), and writes the controls
# directory the verifier reads. It runs no paid agent.
#
# PREREQUISITES, checked in this order, the first missing one named in the
# refusal: --baseline, --contracts (final MCP contracts), --controls-dir,
# --receipts (#148/#201 coordination), --cost, --identity, --product. They
# are stat-ed in that order, then the eval crate is built (`eval-build`) and
# `swarmail-evals comparison-check` verifies their contents and bindings, and
# names the first that does not hold.
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
output='' mailpit='' mailhog='' mailslurper='' baseline='' final='' receipts='' config='' dry_run=0
contracts='' controls_dir='' cost='' identity='' product='' authorized_product=''
while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run) dry_run=1; shift; continue ;;
    --protocol|--sources|--output|--mailpit|--mailhog|--mailslurper|--authorized-product|--baseline|--final|--receipts|--config|--contracts|--controls-dir|--cost|--identity|--product)
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
# crate must build, and its verifier must find every prerequisite's contents
# genuine. Conflict syntax is not a build, and a private lock or a filtered
# suite is not the #148/#201 shared-host proof either. The verifier needs the
# controls of all four targets, which only a runner that served every
# competitor can have produced.
evals_bin=${COMPARE_MAIL_EVALS_BIN:-evals/target/debug/swarmail-evals}

build_evals() {
  local conflicted
  conflicted=$(grep -rlE '^(<<<<<<<|>>>>>>>)( |$)' evals/src evals/tests 2>/dev/null | sort | tr '\n' ' ')
  if [ -n "$conflicted" ]; then
    blocked eval-build "unresolved conflict syntax in $conflicted(#148/#201 prerequisite)"
  fi
  command -v cargo >/dev/null 2>&1 || blocked eval-build "cargo is not installed"
  if [ -z "${COMPARE_MAIL_EVALS_BIN:-}" ]; then
    (cd evals && cargo build --locked --offline >&2) \
      || blocked eval-build "the eval crate does not build (cargo build --locked --offline in evals/)"
  fi
  [ -x "$evals_bin" ] || blocked eval-build "the eval executable $evals_bin is missing"
}

# Run the verifier in one mode; its stdout is the transcript, and a refusal
# is relayed by the name of the prerequisite it names.
verify() {
  local vmode="$1" out err
  err=$(mktemp) || blocked prerequisites "cannot make a temporary file"
  if out=$("$evals_bin" comparison-check --mode "$vmode" --baseline "$baseline" \
      --contracts "$contracts" --controls-dir "$controls_dir" --receipts "$receipts" \
      --cost "$cost" --identity "$identity" --product "$product" --sources "$sources" \
      --repo . 2>"$err"); then
    rm -f "$err"
    printf '%s\n' "$out"
    return 0
  fi
  local msg
  msg=$(sed -n 's/^swarmail-evals: comparison refused: //p' "$err" | head -n1)
  rm -f "$err"
  case "$msg" in
    prerequisite-*) blocked "${msg%%:*}" "${msg#*: }" ;;
    *) blocked verifier "the comparison verifier failed: ${msg:-no reason given}" ;;
  esac
}

eval_prerequisite() {
  build_evals
  verify "$1"
}

# ---- controls: the free oracle and null rows (decision 0091 section 5) ----

controls_run() {
  [ -n "$product" ] || blocked arguments "--product is required in controls mode"
  [ -s "$product" ] || blocked prerequisites "--product $product does not exist or is empty"
  for t in mailpit mailhog mailslurper; do
    bin="${!t}"
    [ -n "$bin" ] || blocked competitor-binaries "controls measure all four targets; pass --$t with its pinned executable"
    [ -x "$bin" ] || blocked competitor-binaries "--$t $bin is not an executable file"
  done
  build_evals
  cargo build --locked --offline --bin swarmail >&2 || blocked build "the candidate binary does not build"
  local swarmail adapter
  swarmail=$PWD/target/debug/swarmail
  adapter=$(cargo bench --locked --offline --no-run --bench mail_compare --message-format=json 2>/dev/null \
    | grep -o '"executable":"[^"]*mail_compare[^"]*"' | tail -n1 | cut -d'"' -f4)
  [ -n "$adapter" ] && [ -x "$adapter" ] || blocked build "the mail_compare driver does not build"
  local scratch="$output.scratch" product_id seed fixture_time head
  mkdir -p "$scratch" || blocked prerequisites "cannot make $scratch"
  product_id=$(sed -n 's/^  "id": "\(.*\)",$/\1/p' "$product" | head -n1)
  [ -n "$product_id" ] || blocked prerequisites "--product has no id"
  # One master seed and one time for the whole run: a task and an attempt see
  # the same fixture on every target.
  seed=$(printf '%s' "$product_id" | sha256sum | cut -d' ' -f1)
  fixture_time=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  printf '[{"label":"oracle","argv":[]},{"label":"null","argv":[]}]\n' >"$scratch/agents.json"
  local t status common
  for t in swarmail mailpit mailhog mailslurper; do
    common=(--target "$t" --product "$product")
    if [ "$t" != swarmail ]; then
      common+=(--adapter "$adapter" --target-bin "$(realpath "${!t}")" --sources "$(realpath "$sources")")
    fi
    {
      echo "# $t: one worker, each command under timeout 600"
      echo "# uptime before: $(uptime)"
    } >"$scratch/timing-$t.txt"
    echo "# command: timeout 600 /usr/bin/time swarmail-evals comparison-contract ${common[*]}" >>"$scratch/timing-$t.txt"
    SWARMAIL_BIN="$swarmail" timeout 600 /usr/bin/time -f '# elapsed %e s, max rss %M KB' \
      "$evals_bin" comparison-contract "${common[@]}" --output "$scratch/contract-$t.json" \
      2>>"$scratch/timing-$t.txt" \
      || blocked "contract-$t" "the $t contract probe failed (see $scratch/timing-$t.txt)"
    echo "# command: timeout 600 /usr/bin/time swarmail-evals matrix ${common[*]} --tier hard --agents oracle,null --trials 5" >>"$scratch/timing-$t.txt"
    SWARMAIL_BIN="$swarmail" timeout 600 /usr/bin/time -f '# elapsed %e s, max rss %M KB' \
      "$evals_bin" matrix "${common[@]}" --tier hard --agents "$scratch/agents.json" --trials 5 \
      --seed "$seed" --fixture-time "$fixture_time" --artifacts-dir "$scratch/art-$t" \
      >"$scratch/rows-$t.jsonl" 2>>"$scratch/timing-$t.txt"
    status=$?
    echo "# uptime after: $(uptime)" >>"$scratch/timing-$t.txt"
    # The null agent fails by design, so exit 1 (a failed row) is expected;
    # only 0 and 1 are a finished matrix.
    [ "$status" -le 1 ] || blocked "controls-$t" "the $t matrix ended with status $status (see $scratch/timing-$t.txt)"
  done
  head=$(git rev-parse HEAD 2>/dev/null) || blocked prerequisites "the candidate is not a git checkout"
  local suite_cmd="cargo test --locked --offline (in evals/, SWARMAIL_BIN=target/debug/swarmail, RUST_TEST_THREADS=32)"
  local suite_status=pass
  (cd evals && SWARMAIL_BIN="$swarmail" RUST_TEST_THREADS=32 timeout 600 cargo test --locked --offline -j 32 >"$scratch/eval-suite.log" 2>&1) \
    || suite_status=fail
  "$evals_bin" comparison-index --scratch "$scratch" --output "$output" --product "$product" \
    --swarmail "$swarmail" --mailpit "$mailpit" --mailhog "$mailhog" --mailslurper "$mailslurper" \
    --adapter "$adapter" --head "$head" --eval-suite-command "$suite_cmd" --eval-suite-status "$suite_status" \
    || blocked controls-index "the controls index could not be written"
  rm -rf "$scratch"
  [ "$suite_status" = pass ] || blocked eval-suite "the locked eval suite did not pass; the receipts record $suite_status"
  echo "compare-mail controls: wrote $output (120 rows, 4 targets)"
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
    controls_run
    ;;
  evidence)
    [ -n "$final" ] || blocked arguments "--final is required in evidence mode"
    [ -e "$final" ] || blocked prerequisites "--final $final does not exist"
    prerequisite_chain
    eval_prerequisite evidence
    # The verified prerequisites hold: cite the final run's rows by (target,
    # agent, task). A run that wrote no comparison rows has nothing to cite.
    [ -s "$final/rows.jsonl" ] || blocked final-rows "--final $final holds no rows.jsonl to cite"
    "$evals_bin" comparison-cite --rows "$final/rows.jsonl" >"$output.citation.json" \
      || blocked final-rows "the final rows cannot be cited (see above)"
    echo "compare-mail evidence: wrote $output.citation.json"
    ;;
  measure)
    # Operator-only: delegates the final product to the eval owners after
    # separately granted authority. A dry run and a refusal never open the
    # wrapper configuration or launch a trial; the prerequisites are stat-ed in
    # their fixed order, and then verified.
    [ -n "$config" ] || blocked arguments "--config is required in measure mode"
    [ -e "$config" ] || blocked prerequisites "--config does not exist"
    prerequisite_chain
    if [ "$dry_run" = 1 ]; then
      eval_prerequisite dry-run
      exit 0
    fi
    eval_prerequisite measure
    product_id=$(sed -n 's/^  "id": "\(.*\)",$/\1/p' "$product" | head -n1)
    [ -n "$authorized_product" ] \
      || blocked authorization "--authorized-product ID is required: the paid run needs the operator's confirmation of product $product_id"
    [ "$authorized_product" = "$product_id" ] \
      || blocked authorization "--authorized-product $authorized_product is not the product $product_id"
    for t in mailpit mailhog mailslurper; do
      bin="${!t}"
      [ -n "$bin" ] && [ -x "$bin" ] || blocked competitor-binaries "measure runs all four targets; pass --$t with its pinned executable"
    done
    command -v cargo >/dev/null 2>&1 || blocked prerequisites "cargo is not installed"
    cargo build --locked --offline --bin swarmail >&2 || blocked build "the candidate binary does not build"
    swarmail=$PWD/target/debug/swarmail
    adapter=$(cargo bench --locked --offline --no-run --bench mail_compare --message-format=json 2>/dev/null \
      | grep -o '"executable":"[^"]*mail_compare[^"]*"' | tail -n1 | cut -d'"' -f4)
    [ -n "$adapter" ] && [ -x "$adapter" ] || blocked build "the mail_compare driver does not build"
    trials=$(sed -n 's/^  "trials": \([0-9]*\),$/\1/p' "$product" | head -n1)
    [ -n "$trials" ] || blocked prerequisites "--product has no trials"
    seed=$(printf '%s' "$product_id" | sha256sum | cut -d' ' -f1)
    fixture_time=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    mkdir -p "$output" || blocked prerequisites "cannot make $output"
    for t in swarmail mailpit mailhog mailslurper; do
      common=(--target "$t" --product "$product")
      if [ "$t" != swarmail ]; then
        common+=(--adapter "$adapter" --target-bin "$(realpath "${!t}")" --sources "$(realpath "$sources")")
      fi
      mkdir -p "$output/$t"
      SWARMAIL_BIN="$swarmail" "$evals_bin" matrix "${common[@]}" --tier hard --agents "$config" \
        --trials "$trials" --seed "$seed" --fixture-time "$fixture_time" \
        --artifacts-dir "$output/$t/artifacts" >"$output/$t/rows.jsonl"
      status=$?
      [ "$status" -le 1 ] || blocked "measure-$t" "the $t matrix ended with status $status; later targets did not run"
    done
    cat "$output/swarmail/rows.jsonl" "$output/mailpit/rows.jsonl" "$output/mailhog/rows.jsonl" "$output/mailslurper/rows.jsonl" >"$output/rows.jsonl"
    echo "compare-mail measure: wrote $output/rows.jsonl"
    ;;
esac
