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
#   compare-mail.sh collect-v4 --output DIR --bin NAME=PATH... [--runtime NAME=DIR]...
#                            [--protocol P --sources S --documents DIR] [--dry-run]
#                            (protocol 4: the whole cost run as bounded chunks, each in its own
#                            delegated systemd scope; Linux only; --dry-run prints the plan)
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
orig_args=("${@:2}")
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
  local|collect|collect-v4|controls|evidence|measure) ;;
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

# Every --bin and --runtime names a declared competitor, and every declared
# competitor has its --bin: a file the pinned runtime runs is read, never
# exec'd, so a jar is no executable. Uses $declared.
check_competitor_binaries() {
  local w name t bin
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
}

case "$mode" in
  local|collect)
    declared=$(declared_competitors "$protocol")
    # A smoke of the candidate alone declares no competitor; collection
    # measures competitors, so a protocol with none cannot collect.
    if [ "$mode" = collect ] && [ -z "$declared" ]; then
      blocked competitor-binaries "collect measures competitors, and the protocol $protocol declares none"
    fi
    check_competitor_binaries
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
  collect-v4)
    # Protocol 4 (accepted decision 0132 §11): the whole cost run as a fixed,
    # protocol-derived plan of bounded chunks. Each chunk is one driver in its
    # own delegated systemd scope, under `timeout --kill-after=30 560` and
    # `taskset` on at most nproc/4 CPUs (never more than 32); nothing here runs
    # longer than ten minutes. The plan is the driver's (`targets --plan v4`), never computed
    # here. Linux only: there is no unenforced fallback.
    [ "$(uname -s)" = Linux ] \
      || blocked platform "collect-v4 runs on Linux with delegated cgroup v2 only; $(uname -s) cannot enforce the budget"
    for tool in systemd-run systemctl taskset timeout; do
      command -v "$tool" >/dev/null 2>&1 || blocked prerequisites "collect-v4 needs $tool on PATH"
    done
    proto_version=$(sed -n 's/^ *"protocol_version": *\([0-9]*\).*/\1/p' "$protocol" | head -n1)
    [ "$proto_version" = 4 ] \
      || blocked protocol "collect-v4 needs a protocol_version 4 protocol, and $protocol is version ${proto_version:-unknown}; until the v4 freeze, pass the draft: --protocol docs/evals/draft/protocol.v4.json --sources docs/evals/draft/sources.lock.v5.json --documents docs/evals"
    ncpu=$(nproc 2>/dev/null) || ncpu=0
    case "$ncpu" in ''|*[!0-9]*) ncpu=0 ;; esac
    chunk_cpus=$((ncpu / 4))
    [ "$chunk_cpus" -le 32 ] || chunk_cpus=32
    [ "$chunk_cpus" -ge 3 ] \
      || blocked prerequisites "collect-v4 needs at least 3 CPUs in min(nproc/4, 32), and nproc $ncpu gives $chunk_cpus"
    # The chunks are pinned to CPUs 0-<n-1>, which are the operator's CPUs
    # only when the shell may use every CPU of the host: a narrowed shell
    # would be pinned outside the set it was given.
    all_cpus=$(nproc --all 2>/dev/null) || all_cpus=0
    [ "$ncpu" = "$all_cpus" ] \
      || blocked prerequisites "collect-v4 pins the chunks to CPUs 0-$((chunk_cpus - 1)) and needs a shell that may use all of the host's CPUs: nproc is $ncpu, nproc --all is $all_cpus"
    # Every process the mode starts after the preflight runs under this one
    # prefix (0132 §11.1: no process runs longer than 10 minutes or on more
    # than 32 CPUs). The 560 is `CHUNK_KILL_MS` of the driver's protocol.rs,
    # in seconds: the instant `timeout` sends SIGTERM, and SIGKILL follows 30
    # s later. Every `systemctl` call has its own `timeout` (1 s to ask, 3 s to
    # kill, 3 s to stop), so the ten minutes hold as a sum, for the worst case
    # of each kind of step:
    #   a build, `targets`, `check`, `footprint` or an admission: 560 + 30
    #   a chunk that failed: 560 + 30 + 1 (one query) + 3 (kill) + 3 (stop)
    #   a chunk that sealed: 560 + 20 x (1 + 0.5) (the polls) + 3 + 3
    bound=(timeout --kill-after=30 560 taskset -c "0-$((chunk_cpus - 1))")
    scope_polls=20 scope_poll_interval=0.5
    interrupted=0 step_pid='' step_starting='' step_status=0 owned='' owned_unit='' scope_clean=true
    interrupt_exit() {
      printf 'compare-mail: blocked at phase "interrupt": collect-v4 was interrupted; %s is incomplete and is never resumed\n' "$output" >&2
      exit 2
    }
    # A signal does two things only: it records the interrupt, and sends the
    # step that is running (its `timeout`, which passes the signal on) a
    # SIGTERM; the driver's own interrupt path tears down and exits 2,
    # unsealed. The handler ends the mode itself only when no step is running
    # or starting and no chunk is owned. Everything else is the main flow's:
    # it reads `interrupted` after every step, and a chunk is judged and
    # written to the ledger as a failed one before the mode exits 2.
    on_signal() {
      interrupted=1
      if [ -n "$step_pid" ]; then
        kill -TERM "$step_pid" 2>/dev/null
      elif [ -z "$step_starting" ] && [ -z "$owned" ]; then
        interrupt_exit
      fi
    }
    trap on_signal INT TERM HUP
    # The one owner of a running step: start the command under the bound, in
    # the background, and wait for it here, so that a trapped signal can act
    # while it runs. OUT, when not empty, receives its stdout; a step whose
    # output is read goes to a file, never through $(...), which would hold
    # the trap until it ended. The status is $step_status. No step starts
    # once an interrupt is recorded.
    run_step() {
      local out="$1"
      shift
      if [ "$interrupted" != 0 ]; then
        step_status=2
        return 0
      fi
      # From here until its pid is known the step is starting: a signal in
      # that window only records the interrupt (it must not end the mode and
      # orphan a step it has just forked, which could be an admission that
      # seals), and the check below delivers the SIGTERM.
      step_starting=1
      if [ -n "$out" ]; then
        "${bound[@]}" "$@" >"$out" &
      else
        "${bound[@]}" "$@" &
      fi
      step_pid=$!
      step_starting=''
      # A signal that came between the fork and the pid being known.
      [ "$interrupted" = 0 ] || kill -TERM "$step_pid" 2>/dev/null
      # `wait` returns early on a trapped signal: repeat until the step
      # itself has been reaped.
      while :; do
        wait "$step_pid"
        step_status=$?
        kill -0 "$step_pid" 2>/dev/null || break
      done
      step_pid=''
    }
    # A step that is not a chunk: an interrupt ends the mode at once, naming
    # the interrupt, before the caller reads the status.
    step() {
      run_step "$@"
      [ "$interrupted" = 0 ] || interrupt_exit
    }
    # What systemd says of a scope: the word `is-active` prints, or nothing.
    scope_state() {
      timeout 1 systemctl --user is-active "$1" 2>/dev/null | head -n1
    }
    # Only `inactive` and `failed` mean that a scope holds no process; any
    # other answer, none, a failed query or a cut one, is not known to be gone.
    scope_gone() {
      case "$1" in
        inactive|failed) return 0 ;;
      esac
      return 1
    }
    # SIGKILL, not systemctl's default SIGTERM: a target that ignores SIGTERM
    # would otherwise live until systemd's own stop timeout, past the sums.
    scope_end() {
      timeout 3 systemctl --user kill --signal=KILL "$1" >/dev/null 2>&1
      timeout 3 systemctl --user stop "$1" >/dev/null 2>&1
    }
    # A chunk that sealed: systemd learns that a scope is empty after the
    # driver has exited, so it is asked up to $scope_polls times, half a
    # second apart, and is clean at the first answer that says it is gone. An
    # interrupt stops the asking. A scope not gone at the last answer is
    # killed and stopped, and is unclean.
    judge_sealed() {
      local tries=0 state
      scope_clean=false
      while [ "$tries" -lt "$scope_polls" ]; do
        state=$(scope_state "$1")
        if scope_gone "$state"; then
          scope_clean=true
          return 0
        fi
        tries=$((tries + 1))
        [ "$interrupted" = 0 ] || break
        [ "$tries" -ge "$scope_polls" ] || sleep "$scope_poll_interval"
      done
      scope_end "$1"
    }
    # A chunk that failed: the run is already stopped, so the scope is asked
    # once and not waited for; unless it is gone it is killed and stopped.
    judge_failed() {
      if scope_gone "$(scope_state "$1")"; then
        scope_clean=true
      else
        scope_clean=false
        scope_end "$1"
      fi
    }
    commit=$(git rev-parse HEAD 2>/dev/null || true)
    if [ "$dry_run" != 1 ]; then
      [ -n "$commit" ] || blocked candidate "the candidate is not a committed git checkout; collect-v4 never measures an uncommitted tree"
      # A status git cannot read is not a clean tree.
      porcelain=$(git status --porcelain 2>/dev/null) \
        || blocked candidate "git status failed on the candidate tree; collect-v4 measures a committed, clean main only and cannot tell this one is clean"
      [ -z "$porcelain" ] \
        || blocked candidate "the candidate tree is dirty; collect-v4 measures a committed, clean main only"
      # On main (0132 §2.3): HEAD is in the history of main, the tip or an
      # earlier commit (a published run must stay repeatable after main has
      # moved on), by the local ref and, where it exists, the remote one.
      # Nothing is fetched. A pull-request branch is never in that history.
      # A ref exists when its loose file does, readable or not, or when git
      # lists it (a packed or reftable ref). Resolving it is not the test: a
      # loose ref that holds garbage does not resolve, and would read as absent.
      main_ref_exists() {
        local loose
        loose=$(git rev-parse --git-path "$1" 2>/dev/null) && [ -f "$loose" ] && return 0
        git for-each-ref --format='%(refname)' "$1" 2>/dev/null | grep -qxF "$1"
      }
      main_seen=0
      for main_ref in refs/heads/main refs/remotes/origin/main; do
        # An absent ref is skipped; one that exists and is not a readable
        # commit refuses, so it can never leave the other ref to decide.
        main_ref_exists "$main_ref" || continue
        git rev-parse --verify --quiet "$main_ref^{commit}" >/dev/null 2>&1 \
          || blocked candidate "$main_ref exists but is not a readable commit; collect-v4 cannot tell that the candidate is on main"
        main_seen=1
        git merge-base --is-ancestor HEAD "$main_ref" 2>/dev/null \
          || blocked candidate "the candidate $commit is not in the history of $main_ref; collect-v4 measures a commit of main only"
      done
      [ "$main_seen" = 1 ] \
        || blocked candidate "neither refs/heads/main nor refs/remotes/origin/main exists: collect-v4 cannot tell that the candidate is on main"
    fi
    declared=$(declared_competitors "$protocol")
    [ -n "$declared" ] || blocked competitor-binaries "collect-v4 measures competitors, and the protocol $protocol declares none"
    check_competitor_binaries
    doc_args=()
    [ -z "$documents" ] || doc_args=(--documents "$documents")
    command -v cargo >/dev/null 2>&1 || [ -n "${COMPARE_MAIL_DRIVER_BIN:-}" ] || blocked prerequisites "cargo is not installed"
    bin_dir=${COMPARE_MAIL_BIN_DIR:-target/compare-mail}
    mkdir -p "$bin_dir" || blocked prerequisites "cannot make $bin_dir"
    run_token=$(date -u +%Y%m%dT%H%M%SZ)-$$
    run_id="collect-v4-$run_token"
    candidate=${COMPARE_MAIL_SWARMAIL_BIN:-}
    if [ -z "$candidate" ]; then
      step '' cargo build --release --locked --offline --bin swarmail >&2
      [ "$step_status" -eq 0 ] || blocked build "the candidate binary does not build"
      candidate=$PWD/target/release/swarmail
    fi
    driver=${COMPARE_MAIL_DRIVER_BIN:-}
    if [ -z "$driver" ]; then
      step "$bin_dir/$run_token.bench.json" cargo bench --locked --offline --no-run --bench mail_compare --message-format=json 2>/dev/null
      [ "$step_status" -eq 0 ] || blocked build "the mail_compare driver does not build"
      driver=$(grep -o '"executable":"[^"]*mail_compare[^"]*"' "$bin_dir/$run_token.bench.json" | tail -n1 | cut -d'"' -f4)
    fi
    [ -n "$driver" ] && [ -x "$driver" ] || blocked build "the mail_compare driver does not build"
    candidate=$(freeze_binary "$candidate" "$bin_dir") || blocked build "cannot freeze the candidate binary"
    driver=$(freeze_binary "$driver" "$bin_dir") || blocked build "cannot freeze the driver"
    # The driver's own list is authoritative; the preflight must agree.
    step "$bin_dir/$run_token.targets" "$driver" targets --protocol "$protocol"
    [ "$step_status" -eq 0 ] || blocked collection-ready "the driver refuses the protocol $protocol"
    from_driver=$(cat "$bin_dir/$run_token.targets")
    [ "$from_driver" = "$declared" ] \
      || blocked competitor-binaries "the driver declares a different competitor list than the preflight read"
    step "$bin_dir/$run_token.plan" "$driver" targets --protocol "$protocol" --plan v4
    [ "$step_status" -eq 0 ] || blocked collection-ready "the driver has no chunk plan for $protocol"
    plan=$(cat "$bin_dir/$run_token.plan")
    [ -n "$plan" ] || blocked collection-ready "the driver's chunk plan for $protocol is empty"
    systemd_version=$(timeout 3 systemctl --version 2>/dev/null | head -n1)
    [ -n "$systemd_version" ] || systemd_version="not stated"
    scratch_root="$output.scratch"

    # The command of one chunk, without its bound, in this nesting: the
    # bound's timeout, then taskset, then the scope, so the driver is the
    # scope's only process (a process left in the scope's own cgroup would
    # stop it enabling controllers for leaves).
    chunk_command() {
      local coord="$1" target rest unit w
      rest="${coord#*.}"
      target="${rest%.*}"
      unit="mail-compare-$run_token-$(printf '%s' "$coord" | tr -c 'A-Za-z0-9_.\n-' '_')"
      chunk_unit="$unit"
      chunk_cmd=(systemd-run --user --scope -p Delegate=yes "--unit=$unit" --
        "$driver" run --mode collect --protocol "$protocol" --sources "$sources" ${doc_args[@]+"${doc_args[@]}"}
        --output "$output/chunks/$coord" --scratch "$scratch_root/$coord" --run-id "$run_id"
        --chunk "$coord" --systemd "$systemd_version")
      if [ "$target" = swarmail ]; then
        chunk_cmd+=(--bin "swarmail=$candidate")
      else
        chunk_cmd+=(--bin "$target=$(bin_for "$target")")
        [ -z "$(runtime_for "$target")" ] || chunk_cmd+=(--runtime "$target=$(runtime_for "$target")")
      fi
      [ -z "$commit" ] || chunk_cmd+=(--commit "$commit" --dirty false)
    }

    if [ "$dry_run" = 1 ]; then
      for coord in $plan; do
        chunk_command "$coord"
        printf '%s\t' "$coord"
        printf '%q ' "${bound[@]}" "${chunk_cmd[@]}"
        printf '\n'
      done
      exit 0
    fi

    # Every value that reaches the ledger is plain text: a control character
    # in one would need an escaper this script does not carry.
    for w in "$protocol" "$sources" "$output" "$commit" "$driver" ${bin_args[@]+"${bin_args[@]}"} ${runtime_args[@]+"${runtime_args[@]}"}; do
      case "$w" in
        *[[:cntrl:]]*) blocked arguments "an argument carries a control character" ;;
      esac
    done
    json_text() { printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g'; }
    step '' "$driver" check --protocol "$protocol" --sources "$sources" ${doc_args[@]+"${doc_args[@]}"}
    [ "$step_status" -eq 0 ] \
      || blocked collection-ready "the protocol and lock are not ready for collection (see above)"
    mkdir -p "$output/chunks" "$output/ledger" "$scratch_root" \
      || blocked prerequisites "cannot make $output"
    fp_args=(footprint --protocol "$protocol" --sources "$sources" ${doc_args[@]+"${doc_args[@]}"} --bin "swarmail=$candidate")
    for t in $declared; do
      fp_args+=(--bin "$t=$(bin_for "$t")")
      [ -z "$(runtime_for "$t")" ] || fp_args+=(--runtime "$t=$(runtime_for "$t")")
    done
    step "$output/footprint.json" "$driver" "${fp_args[@]}"
    [ "$step_status" -eq 0 ] \
      || blocked footprint "the footprint could not be computed (see above); $output is incomplete"
    any_failed=0
    for coord in $plan; do
      [ "$interrupted" = 0 ] || interrupt_exit
      chunk_command "$coord"
      # The chunk is owned from before it starts until its ledger entry is
      # written: a signal in between never abandons it.
      owned="$coord" owned_unit="$chunk_unit.scope"
      load_before=$(cat /proc/loadavg 2>/dev/null || echo unavailable)
      started=$(date +%s)
      run_step '' "${chunk_cmd[@]}"
      status=$step_status
      wall_ms=$((($(date +%s) - started) * 1000))
      load_after=$(cat /proc/loadavg 2>/dev/null || echo unavailable)
      # A timeout (124), a kill (137) or any status above 2 is an
      # infrastructure failure, and so is a chunk that claims 0 or 1 without
      # its completion index: such a chunk was never sealed. So is a chunk
      # that was interrupted. A chunk that counts as 2 has failed; any other
      # has sealed.
      case "$status" in
        0|1) [ -f "$output/chunks/$coord/index.json" ] || status=2 ;;
        *) status=2 ;;
      esac
      [ "$interrupted" = 0 ] || status=2
      if [ "$status" -eq 2 ]; then
        judge_failed "$owned_unit"
      else
        judge_sealed "$owned_unit"
        [ "$scope_clean" = true ] || status=2
      fi
      # A signal during the judgement: the entry is still written, with 2.
      [ "$interrupted" = 0 ] || status=2
      write_entry() {
        printf '{"coordinate":"%s","command":"%s","exit":%s,"wall_ms":%s,"load_before":"%s","load_after":"%s","scope_clean":%s,"driver":"%s"}\n' \
          "$(json_text "$coord")" "$(json_text "$(printf '%q ' "${bound[@]}" "${chunk_cmd[@]}")")" "$status" "$wall_ms" \
          "$(json_text "$load_before")" "$(json_text "$load_after")" "$scope_clean" "$(json_text "$driver")" \
          >"$output/ledger/$coord.json" \
          || blocked ledger "cannot write the ledger entry of $coord"
      }
      write_entry
      # The chunk is let go first and the interrupt read after: a signal
      # while it was still owned was only recorded, and is seen here, so the
      # entry is written again, with 2; one after it was let go ends the mode
      # in the handler, not owning any chunk.
      owned=''
      if [ "$interrupted" != 0 ] && [ "$status" -ne 2 ]; then
        status=2
        write_entry
      fi
      if [ "$status" -eq 2 ]; then
        # Nothing is resumed, overwritten or removed: the run is incomplete.
        blocked "chunk-$coord" "the chunk ended in an infrastructure failure (interrupted: $interrupted, scope clean: $scope_clean); $output is incomplete and is never resumed"
      fi
      [ "$status" -eq 0 ] || any_failed=1
    done
    [ "$interrupted" = 0 ] || interrupt_exit
    rmdir "$scratch_root" 2>/dev/null || true
    # What a reader needs to repeat the run: the one command, as given. It is
    # written only once every chunk has run, so an incomplete directory does
    # not carry it.
    {
      printf '%s' "scripts/compare-mail.sh $mode"
      for w in ${orig_args[@]+"${orig_args[@]}"}; do printf ' %q' "$w"; done
      printf '\n'
    } >"$output/reproduce.txt" || blocked prerequisites "cannot write $output/reproduce.txt"
    adm=(--sources "$sources" ${doc_args[@]+"${doc_args[@]}"} --bin "swarmail=$candidate")
    for t in $declared; do adm+=(--bin "$t=$(bin_for "$t")"); done
    step '' "$driver" admit-v4 --seal "$output" "$protocol" "${adm[@]}"
    [ "$step_status" -eq 0 ] || blocked admission "the run could not be sealed (see above)"
    step '' "$driver" admit-v4 "$output" "$protocol" "${adm[@]}"
    [ "$step_status" -eq 0 ] || blocked admission "the sealed run is not admitted (see above)"
    echo "compare-mail collect-v4: wrote $output ($(printf '%s\n' "$plan" | wc -l | tr -d ' ') chunks)"
    exit "$any_failed"
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
