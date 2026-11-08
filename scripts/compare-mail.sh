#!/usr/bin/env bash
# The one entry point of the fair mail comparison (issue #163, proposed
# decision 0091; method in docs/evals/README.md). Thin preflight and
# dispatch only: the bench driver owns every server it launches, and no
# mode here starts a paid agent.
#
#   compare-mail.sh local    --output DIR [--protocol P] [--sources S]
#                            [--documents DIR] [--bin NAME=PATH]... [--runtime NAME=DIR]...
#                            [--mailpit BIN] [--mailhog BIN]      (smoke, never evidence)
#   compare-mail.sh collect  --output DIR --bin NAME=PATH... [--runtime NAME=DIR]...
#                            [--protocol P] [--sources S]   (the frozen protocol, then admission)
#
# NAME is a competitor the protocol declares (proposed decision 0130): the
# required binaries are derived from the protocol, never from a fixed list.
# --mailpit BIN and --mailhog BIN remain aliases of --bin mailpit=BIN and
# --bin mailhog=BIN. A target whose lock pin needs a runtime (GreenMail's JRE, the
# Alkemio row's musl loader root)
# also takes --runtime NAME=DIR; the swarmail candidate is built here. A
# historical lock (docs/evals/history/) is verified against the captured
# documents given by --documents (docs/evals), not the directory it sits in.
#   compare-mail.sh controls --output DIR --product P --bin NAME=PATH...
#                            (free oracle and null rows of the agent suite)
#   compare-mail.sh evidence --output DIR --final DIR [PREREQUISITES]
#   compare-mail.sh measure  --output DIR --config FILE [PREREQUISITES]
#                            (--dry-run | --authorized-product ID --bin NAME=PATH...)
#
# The agent modes (`controls`, `evidence`, `measure`) take main's grammar and
# need a `--bin` for each competitor lock entry the product record names:
# `mailpit`, `mailhog` and `mailslurper-source-build` (the MailSlurper built
# from source, accepted decision 0130; the agent target `mailslurper` runs
# that row, never the official release, which does not start).
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
output='' documents='' mailpit='' mailhog='' baseline='' final='' receipts='' config='' dry_run=0
contracts='' controls_dir='' cost='' identity='' product='' authorized_product=''
# --bin NAME=PATH and --runtime NAME=DIR repeat; they are kept as plain
# "NAME=VALUE" words (no associative arrays: macOS ships bash 3.2).
bin_args=() runtime_args=()
while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run) dry_run=1; shift; continue ;;
    --bin|--runtime)
      [ $# -ge 2 ] || blocked arguments "$1 needs a value"
      case "$2" in
        [a-z0-9]*=?*) ;;
        *) blocked arguments "$1 takes NAME=VALUE, not $2" ;;
      esac
      if [ "$1" = --bin ]; then bin_args+=("$2"); else runtime_args+=("$2"); fi
      shift 2 ;;
    --protocol|--sources|--documents|--output|--mailpit|--mailhog|--baseline|--final|--receipts|--config|--contracts|--controls-dir|--cost|--identity|--product|--authorized-product)
      [ $# -ge 2 ] || blocked arguments "$1 needs a value"
      flag="${1#--}"
      flag="${flag//-/_}"
      printf -v "$flag" '%s' "$2"
      shift 2 ;;
    *) blocked arguments "unknown argument: $1" ;;
  esac
done
[ -z "$mailpit" ] || bin_args+=("mailpit=$mailpit")
[ -z "$mailhog" ] || bin_args+=("mailhog=$mailhog")

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
# COMPARE_MAIL_EVALS_BIN, COMPARE_MAIL_SWARMAIL_BIN and COMPARE_MAIL_ADAPTER_BIN
# name prebuilt executables instead of building them. They exist so that the
# wrapper's own logic can be proved without a build; they change what is run,
# never what is checked, and the verifier still reads the same bytes.

# Resolve the candidate's swarmail and the adapter driver, building them
# unless a prebuilt one was named.
resolve_binaries() {
  swarmail=${COMPARE_MAIL_SWARMAIL_BIN:-}
  adapter=${COMPARE_MAIL_ADAPTER_BIN:-}
  if [ -z "$swarmail" ]; then
    cargo build --locked --offline --bin swarmail >&2 || blocked build "the candidate binary does not build"
    swarmail=$PWD/target/debug/swarmail
  fi
  if [ -z "$adapter" ]; then
    adapter=$(cargo bench --locked --offline --no-run --bench mail_compare --message-format=json 2>/dev/null \
      | grep -o '"executable":"[^"]*mail_compare[^"]*"' | tail -n1 | cut -d'"' -f4)
  fi
  [ -n "$adapter" ] && [ -x "$adapter" ] || blocked build "the mail_compare driver does not build"
  # Measure a frozen copy, named by its own digest: a later build of the same
  # target path (a test run relinks target/debug/swarmail) can then never
  # change the bytes the identity record names.
  local dir=${COMPARE_MAIL_BIN_DIR:-target/compare-mail}
  mkdir -p "$dir" || blocked prerequisites "cannot make $dir"
  swarmail=$(freeze_binary "$swarmail" "$dir") || blocked build "cannot freeze the candidate binary"
  adapter=$(freeze_binary "$adapter" "$dir") || blocked build "cannot freeze the adapter driver"
}

# Copy an executable to DIR/<name>-<16 hex of its sha256> (once) and print the
# absolute path of the copy.
freeze_binary() {
  local src="$1" dir="$2" sum dest
  sum=$(sha256sum "$src" | cut -c1-16) || return 1
  dest="$dir/$(basename "$src")-$sum"
  if [ ! -x "$dest" ]; then
    cp "$src" "$dest.tmp" && chmod 755 "$dest.tmp" && mv "$dest.tmp" "$dest" || return 1
  fi
  realpath "$dest"
}

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

# ---- the agent suite's binaries (proposed decision 0129, under accepted 0130) ----

# The lock entry whose executable an agent-suite target runs. The agent target
# `mailslurper` names the service API the adapter drives; its executable is
# the lock's `mailslurper-source-build` row. The swarmail candidate has none.
lock_entry_of() {
  case "$1" in
    mailpit) printf mailpit ;;
    mailhog) printf mailhog ;;
    mailslurper) printf mailslurper-source-build ;;
  esac
}

# The path given by --bin for an agent-suite target's lock entry.
agent_bin() {
  bin_for "$(lock_entry_of "$1")"
}

# The competitor lock entries the product record names, one per line.
product_entries() {
  grep -o '"lock_entry": "[a-z0-9-]*"' "$product" | cut -d'"' -f4
}

# Every competitor lock entry of the product needs its own --bin NAME=PATH,
# and a --bin for any other name is refused: the required set comes from the
# product record, never from a fixed list here.
agent_binaries() {
  local entries e bin w name
  entries=$(product_entries)
  [ -n "$entries" ] || blocked prerequisites "--product $product names no competitor lock_entry"
  for e in $entries; do
    bin=$(bin_for "$e")
    [ -n "$bin" ] || blocked competitor-binaries "the product record needs --bin $e=PATH with its pinned executable"
    [ -x "$bin" ] || blocked competitor-binaries "--bin $e=$bin is not an executable file"
  done
  for w in ${bin_args[@]+"${bin_args[@]}"}; do
    name="${w%%=*}"
    case $'\n'"$entries"$'\n' in
      *$'\n'"$name"$'\n'*) ;;
      *) blocked competitor-binaries "--bin $name names a lock entry the product record does not use (it uses: $(printf '%s' "$entries" | tr '\n' ' '))" ;;
    esac
  done
}

# ---- controls: the free oracle and null rows (decision 0091 section 5) ----

controls_run() {
  if [ -z "$product" ]; then
    # No product record, no input: main's refusal stands (accepted 0130, the
    # first prerequisite of the chain), with a hint on what this mode needs.
    echo "compare-mail: controls needs --product P and a --bin for each competitor lock entry of it" >&2
    prerequisite_chain
    blocked arguments "--product is required in controls mode"
  fi
  [ -s "$product" ] || blocked prerequisites "--product $product does not exist or is empty"
  agent_binaries
  build_evals
  local swarmail adapter
  resolve_binaries
  # The harness embeds the commit it was built at, so every commit rebuilds
  # it: the controls run, and the identity names, a frozen copy of it too.
  evals_bin=$(freeze_binary "$evals_bin" "${COMPARE_MAIL_BIN_DIR:-target/compare-mail}") \
    || blocked eval-build "cannot freeze the eval executable"
  local scratch product_id seed fixture_time head
  # Absolute: the eval suite runs from evals/, and must find its log there too.
  # Portable (BSD realpath has no -m): make the directory, then resolve it.
  mkdir -p "$output.scratch" || blocked prerequisites "cannot make $output.scratch"
  scratch=$(cd "$output.scratch" && pwd -P) || blocked prerequisites "cannot resolve $output.scratch"
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
      common+=(--adapter "$adapter" --target-bin "$(realpath "$(agent_bin "$t")")" --sources "$(realpath "$sources")")
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
    echo "# command: timeout 600 /usr/bin/time swarmail-evals matrix ${common[*]} --tier hard --agents <the builtin oracle and null> --trials 5" >>"$scratch/timing-$t.txt"
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
  # The suite is a gate, not a measurement: it takes about twenty minutes on
  # this host at 32 threads, so it has its own bound, apart from the ten
  # minutes each target's controls get.
  local suite_cmd="cargo test --locked --offline -j 32 (in evals/, SWARMAIL_BIN=$swarmail, RUST_TEST_THREADS=32, timeout 3000)"
  local suite_status=pass
  if [ -n "${COMPARE_MAIL_EVAL_SUITE:-}" ]; then
    # A test seam: the wrapper's own logic is proved without the real suite.
    suite_cmd="COMPARE_MAIL_EVAL_SUITE=$COMPARE_MAIL_EVAL_SUITE (a stand-in, never a receipt)"
    "$COMPARE_MAIL_EVAL_SUITE" >"$scratch/eval-suite.log" 2>&1 || suite_status=fail
  else
    (cd evals && SWARMAIL_BIN="$swarmail" RUST_TEST_THREADS=32 timeout 3000 cargo test --locked --offline -j 32 >"$scratch/eval-suite.log" 2>&1) \
      || suite_status=fail
  fi
  "$evals_bin" comparison-index --scratch "$scratch" --output "$output" --product "$product" \
    --swarmail "$swarmail" --mailpit "$(agent_bin mailpit)" --mailhog "$(agent_bin mailhog)" --mailslurper "$(agent_bin mailslurper)" \
    --adapter "$adapter" --head "$head" --eval-suite-command "$suite_cmd" --eval-suite-status "$suite_status" \
    || blocked controls-index "the controls index could not be written"
  rm -rf "$scratch"
  [ "$suite_status" = pass ] || blocked eval-suite "the locked eval suite did not pass; the receipts record $suite_status"
  echo "compare-mail controls: wrote $output (120 rows, 4 targets)"
}

# The competitors a protocol declares: its top-level targets list, without the
# candidate (this script builds it) and without a target the protocol
# publishes as not starting (its `does_not_start` list: the driver never
# launches one, so no binary is asked for it). A preflight read of the file so
# a missing binary refuses before any build; the driver's own `targets`
# answer, taken after the build, must agree with it.
declared_competitors() {
  local skip
  skip=$(sed -n '/^  "does_not_start": \[/,/^  \]/p' "$1" | grep -o '"target": "[a-z0-9-]*"' | cut -d'"' -f4)
  sed -n '/^  "targets": \[/,/^  \]/p' "$1" | grep -o '"[a-z0-9-]*"' | tr -d '"' \
    | grep -v -e '^targets$' -e '^swarmail$' \
    | while IFS= read -r name; do
        case $'\n'"$skip"$'\n' in
          *$'\n'"$name"$'\n'*) ;;
          *) printf '%s\n' "$name" ;;
        esac
      done
}

# The directory given for one target by --runtime (the last wins), or empty.
runtime_for() {
  local want="$1" w found=''
  for w in ${runtime_args[@]+"${runtime_args[@]}"}; do
    [ "${w%%=*}" != "$want" ] || found="${w#*=}"
  done
  printf '%s' "$found"
}

# The path given for one target by --bin (the last wins), or empty.
bin_for() {
  local want="$1" w found=''
  for w in ${bin_args[@]+"${bin_args[@]}"}; do
    [ "${w%%=*}" != "$want" ] || found="${w#*=}"
  done
  printf '%s' "$found"
}

case "$mode" in
  local|collect)
    declared=$(declared_competitors "$protocol")
    # A smoke of the candidate alone declares no competitor; collection
    # measures competitors, so a protocol with none cannot collect.
    if [ "$mode" = collect ] && [ -z "$declared" ]; then
      blocked competitor-binaries "collect measures competitors, and the protocol $protocol declares none"
    fi
    for w in ${bin_args[@]+"${bin_args[@]}"}; do
      name="${w%%=*}"
      case $'\n'"$declared"$'\n' in
        *$'\n'"$name"$'\n'*) ;;
        *) blocked competitor-binaries "--bin $name names a target the protocol does not declare (it declares: $(printf '%s' "$declared" | tr '\n' ' '))" ;;
      esac
    done
    for w in ${runtime_args[@]+"${runtime_args[@]}"}; do
      name="${w%%=*}"
      case $'\n'"$declared"$'\n' in
        *$'\n'"$name"$'\n'*) ;;
        *) blocked competitor-binaries "--runtime $name names a target the protocol does not declare" ;;
      esac
    done
    for t in $declared; do
      bin=$(bin_for "$t")
      [ -n "$bin" ] || blocked competitor-binaries "the protocol declares $t; pass --bin $t=PATH with its pinned executable"
      if [ -n "$(runtime_for "$t")" ]; then
        # A pinned runtime (GreenMail's JRE, the Alkemio row's musl loader)
        # runs this file; it is read, never exec'd, so a jar is no executable.
        [ -f "$bin" ] || blocked competitor-binaries "--bin $t=$bin is not a file"
      else
        [ -x "$bin" ] || blocked competitor-binaries "--bin $t=$bin is not an executable file"
      fi
    done
    doc_args=()
    [ -z "$documents" ] || doc_args=(--documents "$documents")
    command -v cargo >/dev/null 2>&1 || blocked prerequisites "cargo is not installed"
    cargo build --release --locked --offline --bin swarmail >&2 \
      || blocked build "the candidate binary does not build"
    bench=$(cargo bench --locked --offline --no-run --bench mail_compare --message-format=json 2>/dev/null \
      | grep -o '"executable":"[^"]*mail_compare[^"]*"' | tail -n1 | cut -d'"' -f4)
    [ -n "$bench" ] && [ -x "$bench" ] || blocked build "the mail_compare driver does not build"
    if [ "$mode" = collect ]; then
      # Collection never starts on a protocol the driver would refuse.
      "$bench" check --protocol "$protocol" --sources "$sources" ${doc_args[@]+"${doc_args[@]}"} \
        || blocked collection-ready "the protocol and lock are not ready for collection (see above)"
      run_mode=collect
    else
      "$bench" check --protocol "$protocol" --sources "$sources" ${doc_args[@]+"${doc_args[@]}"} || true
      run_mode=smoke
    fi
    # The driver's own list is authoritative; the preflight must agree.
    from_driver=$("$bench" targets --protocol "$protocol") \
      || blocked collection-ready "the driver refuses the protocol $protocol"
    [ "$from_driver" = "$declared" ] \
      || blocked competitor-binaries "the driver declares a different competitor list than the preflight read"
    commit=$(git rev-parse HEAD 2>/dev/null || true)
    dirty=false
    [ -z "$(git status --porcelain 2>/dev/null)" ] || dirty=true
    args=(run --mode "$run_mode" --protocol "$protocol" --sources "$sources" ${doc_args[@]+"${doc_args[@]}"}
      --output "$output" --scratch "$output.scratch"
      --run-id "$mode-$(date -u +%Y%m%dT%H%M%SZ)-$$"
      --bin "swarmail=target/release/swarmail")
    for t in $declared; do args+=(--bin "$t=$(bin_for "$t")"); done
    for w in ${runtime_args[@]+"${runtime_args[@]}"}; do args+=(--runtime "$w"); done
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
    agent_binaries
    command -v cargo >/dev/null 2>&1 || blocked prerequisites "cargo is not installed"
    resolve_binaries
    trials=$(sed -n 's/^  "trials": \([0-9]*\),$/\1/p' "$product" | head -n1)
    [ -n "$trials" ] || blocked prerequisites "--product has no trials"
    seed=$(printf '%s' "$product_id" | sha256sum | cut -d' ' -f1)
    fixture_time=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    mkdir -p "$output" || blocked prerequisites "cannot make $output"
    for t in swarmail mailpit mailhog mailslurper; do
      common=(--target "$t" --product "$product")
      if [ "$t" != swarmail ]; then
        common+=(--adapter "$adapter" --target-bin "$(realpath "$(agent_bin "$t")")" --sources "$(realpath "$sources")")
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
