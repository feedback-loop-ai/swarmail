#!/usr/bin/env bash
# Verify the Helm chart without a cluster: lint, render both profiles, and
# assert the rendered contract. Exits non-zero on the first red. CI-only
# (the `chart-verify` job): the hermetic verify seat installs nothing and
# does not run it. Every render lands in one private `mktemp -d` scratch
# removed on exit, so two concurrent runs never share a path (issue #104).
#
#   usage: chart-verify.sh [verify|history|selftest|metadata [path]]
#   verify   (default) version coherence, then lint, render and assert.
#   history  the chart-bump check against the CI event's baseline (issue
#            #141, proposed 0051 §2): a change to any file under the chart
#            needs a strictly greater chart version than the baseline's.
#            Inputs ride the environment, never `eval`:
#              CHART_EVENT        github.event_name (pull_request|push)
#              CHART_REF          github.ref
#              CHART_PR_BASE      pull_request.base.sha (pull_request)
#              CHART_PUSH_BEFORE  event.before (push; all zeros = new branch)
#              CHART_MAIN_REF     a new branch's merge-base partner
#                                 (default origin/main)
#            Missing history refuses by name; it is never "no change".
#   selftest the coherence and history checks against throwaway real Git
#            repositories — no Helm needed; it gates the history check.
set -euo pipefail
cd "$(dirname "$0")/.."
CHART=charts/swarmail
MODE="${1:-verify}"

S="$(mktemp -d "${TMPDIR:-/tmp}/chart-verify.XXXXXX")"
trap 'rm -rf "$S"' EXIT
export CHART_SCRATCH="$S"

refuse() { echo "chart-verify: refusing: $*" >&2; return 1; }

# Exit 0 iff $1 > $2 in semver precedence (numeric fields compare as
# numbers — 0.1.10 > 0.1.9 — and a prerelease ranks below its release).
version_gt() {
  python3 - "$1" "$2" <<'PY'
import sys

def key(v):
    core, _, pre = v.partition("-")
    nums = tuple(int(n) for n in core.split("."))
    if not pre:
        return (nums, 1, ())
    ids = tuple((0, int(i), "") if i.isdigit() else (1, 0, i) for i in pre.split("."))
    return (nums, 0, ids)

sys.exit(0 if key(sys.argv[1]) > key(sys.argv[2]) else 1)
PY
}

# One YAML owner for CI, current coherence and historical manifests. Refuse
# ambiguous documents before emitting either output (including quoted keys).
chart_metadata() { # [path|-] — stdin by default
  python3 -c '
import re, sys, yaml
from yaml.events import AliasEvent
class UniqueLoader(yaml.SafeLoader):
    def construct_mapping(self, node, deep=False):
        seen = set()
        for k, _ in node.value:
            if k.tag != "tag:yaml.org,2002:str":
                raise ValueError("metadata keys must be strings; merges unsupported")
            key = self.construct_object(k, deep=deep)
            if key in seen:
                raise ValueError("duplicate metadata key: " + key)
            seen.add(key)
        return super().construct_mapping(node, deep=deep)
def valid(v):
    if not isinstance(v, str) or not re.fullmatch(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?", v):
        return False
    pre = v.partition("-")[2]
    return all(not (i.isascii() and i.isdigit() and len(i)>1 and i[0]=="0") for i in pre.split("."))
try:
    text = sys.stdin.read() if sys.argv[1] == "-" else open(sys.argv[1]).read()
    for event in yaml.parse(text):
        if isinstance(event, AliasEvent) or getattr(event, "anchor", None):
            raise ValueError("metadata aliases/anchors unsupported")
    data = yaml.load(text, Loader=UniqueLoader)
    if not isinstance(data, dict):
        raise ValueError("metadata must be one mapping")
    for key in ("version", "appVersion"):
        if not valid(data.get(key)):
            raise ValueError("Chart.yaml " + key + " is not a release semver string")
    print("version=" + data["version"])
    print("app=" + data["appVersion"])
except (ValueError, yaml.YAMLError, OSError) as e:
    sys.exit("chart-verify: refusing: " + str(e))
' "${1:--}"
}
chart_field() {
  local metadata
  metadata="$(chart_metadata)" || return 1
  case "$1" in
    version) sed -n "s/^version=//p" <<<"$metadata" ;;
    appVersion) sed -n "s/^app=//p" <<<"$metadata" ;;
  esac
}

# appVersion is Cargo's version, and the Artifact Hub image the chart
# advertises is the one its default render pulls (proposed 0051 §2). The
# chart's own version is free while a package is being prepared.
check_coherence() { # <root>
  local root="$1" cargo app chart image
  cargo="$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -1)"
  app="$(chart_field appVersion <"$root/$CHART/Chart.yaml")" || return 1
  chart="$(chart_field version <"$root/$CHART/Chart.yaml")" || return 1
  [ "$app" = "$cargo" ] \
    || refuse "Chart.yaml appVersion '${app}' is not Cargo.toml's version '${cargo}'" || return 1
  image="$(sed -n 's/^ *image: \(ghcr\.io\/feedback-loop-ai\/swarmail:.*\)$/\1/p' "$root/$CHART/Chart.yaml")"
  [ "$image" = "ghcr.io/feedback-loop-ai/swarmail:${app}" ] \
    || refuse "the artifacthub.io/images annotation names '${image}', not ghcr.io/feedback-loop-ai/swarmail:${app}" || return 1
}

# The event's baseline commit, or a named refusal.
history_base() { # <repo>
  local repo="$1" base
  case "${CHART_EVENT:-}" in
    pull_request)
      base="${CHART_PR_BASE:-}"
      [ -n "$base" ] || { refuse "pull_request event without CHART_PR_BASE"; return 1; }
      ;;
    push)
      case "${CHART_REF:-}" in
        refs/tags/v*)
          # The nearest earlier release tag reachable from the tagged
          # commit's parents — never a tag on the candidate itself.
          base="$(git -C "$repo" describe --tags --abbrev=0 --match 'v[0-9]*' HEAD^ 2>/dev/null)" \
            || { refuse "no earlier v* release tag is reachable from ${CHART_REF}'s parent (fetch tags and full history)"; return 1; }
          ;;
        *)
          base="${CHART_PUSH_BEFORE:-}"
          [ -n "$base" ] || { refuse "push event without CHART_PUSH_BEFORE"; return 1; }
          local main="${CHART_MAIN_REF:-origin/main}"
          if [[ "$base" =~ ^0+$ ]]; then
            base="$(git -C "$repo" merge-base HEAD "$main" 2>/dev/null)" \
              || { refuse "new branch: no merge-base with '${main}' (fetch full history)"; return 1; }
          elif [ "${CHART_REF:-}" != refs/heads/main ] \
            && ! git -C "$repo" rev-parse --verify --quiet "${base}^{commit}" >/dev/null; then
            # A rewritten branch: a force-push left its old head out of the
            # history (sealing re-signs every commit in place). The branch is
            # judged from its merge-base with main, as a new branch is. main
            # itself keeps refusing a missing before: its history only grows.
            base="$(git -C "$repo" merge-base HEAD "$main" 2>/dev/null)" \
              || { refuse "rewritten branch: no merge-base with '${main}' (fetch full history)"; return 1; }
          fi
          ;;
      esac
      ;;
    *) refuse "CHART_EVENT '${CHART_EVENT:-}' is neither pull_request nor push"; return 1 ;;
  esac
  git -C "$repo" rev-parse --verify --quiet "${base}^{commit}" >/dev/null \
    || { refuse "baseline '${base}' is not in the history (shallow or missing; fetch full history)"; return 1; }
  echo "$base"
}

# A chart change needs a strictly greater chart version than the
# baseline's — the whole event's change, not just its last commit.
check_history() { # <repo>
  local repo="$1" base now was
  base="$(history_base "$repo")" || return 1
  now="$(git -C "$repo" show "HEAD:$CHART/Chart.yaml" | chart_field version)" || return 1
  if git -C "$repo" diff --quiet "$base" HEAD -- "$CHART"; then
    echo "chart-verify: history: no chart change since ${base:0:12}"
    return 0
  fi
  if ! git -C "$repo" cat-file -e "${base}:$CHART/Chart.yaml" 2>/dev/null; then
    echo "chart-verify: history: the chart is new since ${base:0:12} (version ${now})"
    return 0
  fi
  was="$(git -C "$repo" show "${base}:$CHART/Chart.yaml" | chart_field version)" || { refuse "baseline chart metadata invalid"; return 1; }
  version_gt "$now" "$was" \
    || { refuse "the chart changed since ${base:0:12} but its version '${now}' is not greater than '${was}' — bump charts/swarmail/Chart.yaml"; return 1; }
  echo "chart-verify: history: chart ${was} -> ${now} since ${base:0:12}"
}

selftest() {
  local fail=0 T="$S/selftest"
  unset CHART_EVENT CHART_REF CHART_PR_BASE CHART_PUSH_BEFORE CHART_MAIN_REF
  mkdir -p "$T"
  bad() { echo "chart-verify selftest: FAIL: $*" >&2; fail=1; }
  g() { git -C "$T/repo" -c user.name=selftest -c user.email=selftest@invalid \
          -c commit.gpgsign=false -c tag.gpgsign=false "$@" >/dev/null 2>&1; }
  # chart <version> <appVersion> [image] — write the fixture chart.
  chart() {
    mkdir -p "$T/repo/$CHART"
    printf 'apiVersion: v2\nname: swarmail\nversion: %s\nappVersion: "%s"\nannotations:\n  artifacthub.io/images: |\n    - name: swarmail\n      image: %s\n' \
      "$1" "$2" "${3:-ghcr.io/feedback-loop-ai/swarmail:$2}" >"$T/repo/$CHART/Chart.yaml"
  }
  commit() { g add -A && g commit -q --allow-empty -m "$1" && git -C "$T/repo" rev-parse HEAD; }
  fresh() {
    rm -rf "$T/repo"; mkdir -p "$T/repo"; g init -q -b main
    printf '[package]\nname = "swarmail"\nversion = "%s"\n' "${1:-0.1.0}" >"$T/repo/Cargo.toml"
  }
  hist() { # <expect pass|fail> <desc> [needle] — env carries the event
    local want="$1" desc="$2" needle="${3:-}" got=pass
    check_history "$T/repo" >"$T/out" 2>"$T/err" || got=fail
    [ "$got" = "$want" ] || bad "history: ${desc}: expected ${want}, got ${got}: $(cat "$T/err")"
    if [ -n "$needle" ] && ! grep -qF -- "$needle" "$T/err"; then
      bad "history: ${desc}: refusal must name '${needle}': $(cat "$T/err")"
    fi
  }
  coh() { # <expect pass|fail> <desc> [needle]
    local got=pass
    check_coherence "$T/repo" >/dev/null 2>"$T/err" || got=fail
    [ "$got" = "$1" ] || bad "coherence: $2: expected $1, got ${got}: $(cat "$T/err")"
    if [ -n "${3:-}" ]; then
      grep -qF "$3" "$T/err" || bad "coherence: $2: missing diagnostic $3: $(cat "$T/err")"
    fi
  }

  # Numeric precedence; strict validation is exercised through CI below.
  version_gt 0.1.10 0.1.9 || bad "0.1.10 must rank above 0.1.9"
  version_gt 0.1.9 0.1.10 && bad "0.1.9 must not rank above 0.1.10"
  version_gt 1.0.0 1.0.0-rc.2 || bad "a release must rank above its prerelease"
  version_gt 1.0.0-rc.10 1.0.0-rc.9 || bad "rc.10 must rank above rc.9"
  version_gt 1.0.0-rc.1 1.0.0-alpha || bad "rc.1 must rank above alpha"
  version_gt 0.1.1 0.1.1 && bad "equal versions must not rank above each other"

  # Coherence: appVersion is Cargo's; the annotation is appVersion's image.
  fresh 0.1.0; chart 0.1.1 0.1.0; coh pass "chart 0.1.1 preparing over app 0.1.0"
  fresh 0.2.0; chart 0.1.1 0.2.0; coh pass "a changed app version, coherent"
  fresh 0.2.0; chart 0.1.1 0.1.0; coh fail "appVersion behind Cargo" "not Cargo.toml"
  fresh 0.1.0; chart 0.1.1 0.1.0 ghcr.io/feedback-loop-ai/swarmail:0.0.9; coh fail "a stale annotation image" "artifacthub.io/images"
  fresh 0.1.0; chart 0.1 0.1.0; coh fail "a malformed chart version" "version is not a release semver string"

  fresh; chart 0.1.1 0.1; coh fail "a malformed appVersion" "appVersion is not a release semver string"

  # Real metadata evaluator and actual workflow run bodies. Copies live only
  # in the fixture; the script resolves its root just as it does in CI.
  mkdir -p "$T/repo/scripts"
  cp scripts/chart-verify.sh "$T/repo/scripts/"
  python3 - "$T/repo" .github/workflows/ci.yml <<'PYTEST'
import os, pathlib, subprocess, sys, yaml
root = pathlib.Path(sys.argv[1]); workflow = yaml.safe_load(open(sys.argv[2]))
chartfile = root / "charts/swarmail/Chart.yaml"
blocks = {s["id"]: s["run"] for j in workflow["jobs"].values() for s in j.get("steps", []) if s.get("id") in ("ver", "chartver")}
def run(body, ok, needle=""):
    output = root / "output"
    output.write_text("")
    p = subprocess.run(["bash", "-c", body], cwd=root, env={**os.environ, "GITHUB_OUTPUT": str(output)}, capture_output=True, text=True)
    assert (p.returncode == 0) == ok, (needle or "CI version body", p.returncode, p.stderr, body)
    if not ok:
        assert output.read_text() == "", "invalid metadata reached GITHUB_OUTPUT"
        assert needle in p.stderr, (needle, p.stderr)
    return output.read_text()
valid = ("0.0.0", "1.0.0-0", "1.0.0-alpha.1", "1.0.0-alpha-01")
invalid = ("01.0.0", "1.0.0-01", "1.0.0-.a", "1.0.0-a.", "1.0.0-a..b", "0.1", "x.y.z", "1.0.0+build")
for v in valid + invalid:
    (root / "Cargo.toml").write_text('[package]\nname = "fixture"\nversion = "'+v+'"\n')
    chartfile.write_text("version: "+v+"\nappVersion: "+v+"\n")
    for body in blocks.values():
        run(body, v in valid)
(root / "Cargo.toml").write_text('[package]\nname = "fixture"\nversion = "1.0.0"\n')
# Invalid appVersion must refuse even if Cargo carries the same invalid
# string; otherwise the equality check would disguise missing validation.
for v in invalid:
    (root / "Cargo.toml").write_text('[package]\nname = "fixture"\nversion = "'+v+'"\n')
    chartfile.write_text("version: 1.0.0\nappVersion: "+v+"\n")
    run(blocks["chartver"], False, "appVersion")
(root / "Cargo.toml").write_text('[package]\nname = "fixture"\nversion = "1.0.0"\n')
for q in ('', "'", '"'):
    chartfile.write_text("version: "+q+"1.0.0"+q+" # comment\nappVersion: "+q+"1.0.0"+q+"\n")
    assert run(blocks["chartver"], True) == "version=1.0.0\napp=1.0.0\n"
    # When Helm is installed this is a real parser equivalence check.
    import shutil
    if shutil.which("helm"):
        chartfile.write_text("apiVersion: v2\nname: fixture\n"+chartfile.read_text())
        p = subprocess.run(["helm", "show", "chart", str(chartfile.parent)], capture_output=True, text=True, check=True)
        helm = yaml.safe_load(p.stdout)
        assert helm["version"] == helm["appVersion"] == "1.0.0"
run("bash scripts/chart-verify.sh metadata missing-chart.yaml", False, "No such file")
base = "version: 1.0.0\nappVersion: 1.0.0\n"
for text, needle in [
    (base+'"version": 2.0.0\n',"duplicate"),
    ('"version": 2.0.0\n'+base,"duplicate"),
    (base+'"appVersion": 2.0.0\n',"duplicate"),
    ('"appVersion": 2.0.0\n'+base,"duplicate"),
    ("version: 1.0.0\n", "appVersion"),
    ("appVersion: 1.0.0\n", "version"),
    ("version: 1.0\nappVersion: 1.0.0\n", "version"),
    ("version: 1.0.0\nappVersion: []\n", "appVersion"),
    ("- item\n", "mapping"),
    (base+"---\n"+base, "single document"),
    (base+"x: &x hi\ny: *x\n", "aliases/anchors"),
    (base+"<<: {x: y}\n", "merges unsupported"),
    (base+"[bad", "expected <block end>"),
]:
    chartfile.write_text(text)
    run(blocks["chartver"], False, needle)
print("chart metadata and actual CI run bodies: OK")
PYTEST

  # History, pull_request: the whole PR, not its last commit.
  fresh; chart 0.1.0 0.1.0; base="$(commit base)"
  chart 0.1.0 0.1.0; echo "x: 1" >"$T/repo/$CHART/values.yaml"; commit "edit chart" >/dev/null
  echo docs >"$T/repo/README.md"; commit "docs last" >/dev/null
  CHART_EVENT=pull_request CHART_PR_BASE="$base" hist fail "an unbumped edit behind a docs commit" "not greater than '0.1.0'"
  chart 0.1.1 0.1.0; commit bump >/dev/null
  CHART_EVENT=pull_request CHART_PR_BASE="$base" hist pass "the same PR, bumped"
  chart 0.1.0-rc.1 0.1.0; commit "prerelease below release" >/dev/null
  CHART_EVENT=pull_request CHART_PR_BASE="$base" hist fail "a prerelease of the baseline version" "not greater"
  chart 0.0.9 0.1.0; commit decrease >/dev/null
  CHART_EVENT=pull_request CHART_PR_BASE="$base" hist fail "a decreased version" "not greater"
  chart 0.1 0.1.0; commit malformed >/dev/null
  CHART_EVENT=pull_request CHART_PR_BASE="$base" hist fail "a malformed version" "not a release semver"
  CHART_EVENT=pull_request CHART_PR_BASE= hist fail "no PR base" "without CHART_PR_BASE"
  CHART_EVENT=pull_request CHART_PR_BASE=0123456789abcdef0123456789abcdef01234567 hist fail "a PR base not in history" "not in the history"
  CHART_EVENT=workflow_dispatch hist fail "an unknown event" "neither pull_request nor push"

  # History, push: unchanged passes; a chart-only bump passes; a missing
  # or zero-without-main before refuses; a force-push base still compares.
  fresh; chart 0.1.0 0.1.0; base="$(commit base)"
  echo docs >"$T/repo/README.md"; commit docs >/dev/null
  CHART_EVENT=push CHART_REF=refs/heads/main CHART_PUSH_BEFORE="$base" hist pass "no chart change"
  chart 0.1.1 0.1.0; commit "bump only" >/dev/null
  CHART_EVENT=push CHART_REF=refs/heads/main CHART_PUSH_BEFORE="$base" hist pass "a bump-only chart edit"
  CHART_EVENT=push CHART_REF=refs/heads/main CHART_PUSH_BEFORE=0123456789abcdef0123456789abcdef01234567 \
    hist fail "a before object missing (shallow or force-pushed away)" "not in the history"
  g checkout -q -b side "$base"; chart 0.1.0 0.1.0; echo "y: 2" >"$T/repo/$CHART/values.yaml"; side="$(commit side)"
  g checkout -q main
  CHART_EVENT=push CHART_REF=refs/heads/main CHART_PUSH_BEFORE="$side" hist pass "a non-ancestor before, compared whole (0.1.1 > 0.1.0)"
  echo "x: 3" >"$T/repo/$CHART/values.yaml"; commit "unbumped after force push" >/dev/null
  CHART_EVENT=push CHART_REF=refs/heads/main CHART_PUSH_BEFORE="$side" hist pass "a non-ancestor before with a bumped version"
  CHART_EVENT=push CHART_REF=refs/heads/main CHART_PUSH_BEFORE="$(git -C "$T/repo" rev-parse HEAD^)" \
    hist fail "an unbumped edit after the bump" "not greater than '0.1.1'"

  CHART_EVENT=push CHART_REF=refs/heads/main CHART_PUSH_BEFORE= hist fail "no push before" "without CHART_PUSH_BEFORE"
  CHART_EVENT=push CHART_REF=refs/heads/feature CHART_PUSH_BEFORE=0123456789abcdef0123456789abcdef01234567 CHART_MAIN_REF=missing \
    hist fail "rewritten branch without main" "rewritten branch: no merge-base"
  fresh; chart 0.1 0.1.0; base="$(commit malformed-baseline)"
  chart 0.1.1 0.1.0; commit repaired >/dev/null
  CHART_EVENT=pull_request CHART_PR_BASE="$base" hist fail "malformed baseline" "baseline chart metadata invalid"

  # History, new branch (zero before): the merge-base with main, refused
  # when main is unknown — zero is never "no change".
  fresh; chart 0.1.0 0.1.0; commit base >/dev/null
  g checkout -q -b feature; echo "z: 1" >"$T/repo/$CHART/values.yaml"; commit "unbumped on a new branch" >/dev/null
  CHART_EVENT=push CHART_REF=refs/heads/feature CHART_PUSH_BEFORE=0000000000000000000000000000000000000000 CHART_MAIN_REF=main \
    hist fail "a new branch with an unbumped chart" "not greater"
  # A rewritten branch (its before force-pushed away, as sealing does):
  # judged from the merge-base with main, never refused for the gone head.
  CHART_EVENT=push CHART_REF=refs/heads/feature CHART_PUSH_BEFORE=0123456789abcdef0123456789abcdef01234567 CHART_MAIN_REF=main \
    hist fail "a rewritten branch with an unbumped chart" "not greater"
  CHART_EVENT=push CHART_REF=refs/heads/feature CHART_PUSH_BEFORE=0000000000000000000000000000000000000000 CHART_MAIN_REF=origin/main \
    hist fail "a new branch without main's history" "no merge-base"
  chart 0.2.0 0.1.0; commit "bumped on the branch" >/dev/null
  CHART_EVENT=push CHART_REF=refs/heads/feature CHART_PUSH_BEFORE=0000000000000000000000000000000000000000 CHART_MAIN_REF=main \
    hist pass "a new branch with a bumped chart"
  CHART_EVENT=push CHART_REF=refs/heads/feature CHART_PUSH_BEFORE=0123456789abcdef0123456789abcdef01234567 CHART_MAIN_REF=main \
    hist pass "a rewritten branch with a bumped chart"

  # History, tag: the nearest earlier release tag, never the candidate's
  # own; a baseline without the chart is an addition; no tag refuses.
  fresh; echo pre >"$T/repo/README.md"; commit "before the chart" >/dev/null
  g tag v0.0.1
  chart 0.1.0 0.1.0; commit "chart added" >/dev/null; g tag v0.1.0
  CHART_EVENT=push CHART_REF=refs/tags/v0.1.0 hist pass "a chart new since the earlier release tag"
  echo "x: 1" >"$T/repo/$CHART/values.yaml"; commit "unbumped" >/dev/null; g tag v0.1.1
  CHART_EVENT=push CHART_REF=refs/tags/v0.1.1 hist fail "a release tag over an unbumped chart" "not greater than '0.1.0'"
  chart 0.1.1 0.1.0; commit "bumped" >/dev/null; g tag -f v0.1.1
  CHART_EVENT=push CHART_REF=refs/tags/v0.1.1 hist pass "a release tag over a bumped chart"
  fresh; chart 0.1.0 0.1.0; commit only >/dev/null; echo x >"$T/repo/README.md"; commit two >/dev/null; g tag v0.1.0
  CHART_EVENT=push CHART_REF=refs/tags/v0.1.0 hist fail "no earlier release tag" "no earlier v* release tag"

  if bash scripts/chart-verify.sh unknown-mode >"$T/out" 2>"$T/err"; then
    bad "unknown mode admitted"
  else
    [ "$?" -eq 2 ] && grep -q 'usage: chart-verify.sh' "$T/err" || bad "unknown mode diagnostic/exit"
  fi
  [ "$fail" -eq 0 ] || { echo "chart-verify selftest: RED" >&2; return 1; }
  echo "chart-verify selftest: OK — versions, coherence and every event baseline"
}

case "$MODE" in
  metadata) chart_metadata "${2:--}"; exit ;;
  selftest) selftest; exit ;;
  history) check_history . ; exit ;;
  verify) ;;
  *) echo "usage: chart-verify.sh [verify|history|selftest|metadata [path]]" >&2; exit 2 ;;
esac

command -v helm >/dev/null || { echo "helm is required" >&2; exit 1; }

echo "== version coherence (appVersion = Cargo; the annotation image = appVersion)"
check_coherence .
APP_VERSION="$(chart_field appVersion <"$CHART/Chart.yaml")"
export APP_VERSION

echo "== helm lint"
helm lint "$CHART" >/dev/null

echo "== render: default profile"
helm template swarmail "$CHART" > "$S/chart-default.yaml"

echo "== render: full profile (persistence, ingress, monitor, policy peers, TLS, config)"
# --api-versions simulates the prometheus-operator CRDs being present; offline
# the ServiceMonitor guard fails closed with a named refusal instead.
helm template swarmail "$CHART" \
  --set persistence.enabled=true \
  --set ingress.enabled=true \
  --set serviceMonitor.enabled=true \
  --set networkPolicy.enabled=true \
  --set 'networkPolicy.ingressPeers[0].namespaceSelector.matchLabels.kubernetes\.io/metadata\.name=monitoring' \
  --set networkPolicy.ingressPeers[1].ipBlock.cidr=10.0.0.0/8 \
  --set tls.enabled=true \
  --set tls.secretName=swarmail-tls \
  --set config.SWARMAIL_MAX_PER_INBOX=0 \
  --api-versions monitoring.coreos.com/v1 > "$S/chart-full.yaml"

echo "== render: the policy's locked default (peers unset = today's posture)"
helm template swarmail "$CHART" --set networkPolicy.enabled=true > "$S/chart-policy-default.yaml"

echo "== render: NOTES.txt both ways (claim name honors existingClaim)"
# NOTES.txt is shown on install, not rendered by `helm template` — so the
# same body is rendered from a scratch copy of the chart, wrapped as a
# YAML block scalar (the engine renders it either way; only the parse of
# the output needs a doc shape). The copy is scratch; nothing is written
# back into the chart.
cp -r "$CHART" "$S/chart-notes"
{ echo '{{- define "notes.body" -}}'; cat "$CHART/NOTES.txt"; echo '{{- end -}}'; } \
  > "$S/chart-notes/templates/_notes.tpl"
printf 'notes: |\n{{ include "notes.body" . | indent 2 }}\n' \
  > "$S/chart-notes/templates/zz-notes.yaml"
helm template notes "$S/chart-notes" --set persistence.enabled=true \
  --set persistence.existingClaim=operator-claim > "$S/notes-claimed.yaml"
helm template notes "$S/chart-notes" --set persistence.enabled=true > "$S/notes-generated.yaml"

echo "== render: ServiceMonitor guard fails closed without the CRD"
if helm template swarmail "$CHART" --set serviceMonitor.enabled=true >/dev/null 2>"$S/chart-guard.err"; then
  echo "FAIL: serviceMonitor.enabled rendered without the CRD present" >&2
  exit 1
fi
grep -q "prometheus-operator CRDs" "$S/chart-guard.err"

echo "== render: the replica cap refuses at render time (decision 0008 §4)"
if helm template swarmail "$CHART" --set replicaCount=2 >/dev/null 2>"$S/chart-replica.err"; then
  echo "FAIL: replicaCount=2 rendered — the cap must refuse at render time" >&2
  exit 1
fi
grep -q "replicaCount must be 1" "$S/chart-replica.err"

echo "== render: service annotations land in metadata (finding 4, issue #31)"
helm template swarmail "$CHART" --set service.annotations.example=probe > "$S/chart-annot.yaml"

echo "== render: a changed appVersion renders its own tag (nothing hard-coded)"
# A scratch twin of the chart with a coherent, different application
# version: the default image must follow it (issue #141).
cp -r "$CHART" "$S/chart-twin"
sed -i.orig 's/^appVersion: .*/appVersion: "9.8.7"/' "$S/chart-twin/Chart.yaml"
helm template swarmail "$S/chart-twin" > "$S/chart-twin.yaml"

echo "== render: the README's :main install line, flags verbatim"
# The flags are read off the README's helm line, never executed as shell:
# the documented `:main` install must pull on every start (a node that
# cached an older `:main` would otherwise keep it — issue #141).
readme_flags="$(sed -n '/^helm install swarmail \.\/charts\/swarmail /,/^[^ ]/p' README.md \
  | grep -o -- '--set [^ \\]*' | sed 's/^--set //')"
grep -qx 'image.tag=main' <<<"$readme_flags" || { echo "FAIL: README's helm line no longer sets image.tag=main" >&2; exit 1; }
set_args=()
while IFS= read -r flag; do set_args+=(--set "$flag"); done <<<"$readme_flags"
helm template swarmail "$CHART" "${set_args[@]}" > "$S/chart-main.yaml"

python3 - <<'EOF'
import os, yaml, sys

S = os.environ["CHART_SCRATCH"]

def docs(path):
    return [d for d in yaml.safe_load_all(open(path)) if d]

def one(path, kind):
    m = [d for d in docs(path) if d["kind"] == kind]
    assert len(m) == 1, f"{path}: expected exactly one {kind}, got {len(m)}"
    return m[0]

d, f = f"{S}/chart-default.yaml", f"{S}/chart-full.yaml"

# The stated binds (decision 0030 §1): the binary defaults to loopback, so a
# render without these three would pass every other check while every probe
# and Service connection fails. Asserted in both profiles (issue #104).
BINDS = {
    "SWARMAIL_SMTP_LISTEN": "0.0.0.0:1025",
    "SWARMAIL_POP3_LISTEN": "0.0.0.0:1110",
    "SWARMAIL_HTTP_LISTEN": "0.0.0.0:8025",
}
for path in (d, f):
    benv = {e["name"]: e.get("value") for e in (one(path, "Deployment")["spec"]["template"]["spec"]["containers"][0].get("env") or [])}
    for k, v in BINDS.items():
        assert benv.get(k) == v, f"{path}: {k} is {benv.get(k)!r}, expected {v!r}"
    assert "startupProbe" in one(path, "Deployment")["spec"]["template"]["spec"]["containers"][0], path

# Default profile: Service + Deployment + the healthz test hook, nothing else.
assert [x["kind"] for x in docs(d)] == ["Service", "Deployment", "Pod"], docs(d)
dep = one(d, "Deployment")
spec = dep["spec"]
assert spec["replicas"] == 1
# Recreate at every persistence setting (decision 0008 §5): a rolling surge
# would run two pods behind one Service — the lie the replica cap forbids.
assert spec["strategy"]["type"] == "Recreate"
c = spec["template"]["spec"]["containers"][0]
assert sorted(p["containerPort"] for p in c["ports"]) == [1025, 1110, 8025]
app = os.environ["APP_VERSION"]
assert c["image"] == f"ghcr.io/feedback-loop-ai/swarmail:{app}", c["image"]  # tag defaults to appVersion
assert c["imagePullPolicy"] == "IfNotPresent"
twin = one(f"{S}/chart-twin.yaml", "Deployment")["spec"]["template"]["spec"]["containers"][0]
assert twin["image"] == "ghcr.io/feedback-loop-ai/swarmail:9.8.7", twin["image"]
# The README's `:main` line: the rolling tag, pulled on every start.
mc = one(f"{S}/chart-main.yaml", "Deployment")["spec"]["template"]["spec"]["containers"][0]
assert mc["image"] == "ghcr.io/feedback-loop-ai/swarmail:main", mc["image"]
assert mc["imagePullPolicy"] == "Always", mc["imagePullPolicy"]
assert all(k in c for k in ("livenessProbe", "readinessProbe"))
assert all(c[k]["httpGet"] == {"path": "/healthz", "port": "http"} for k in ("livenessProbe", "readinessProbe"))
assert c["securityContext"]["allowPrivilegeEscalation"] is False
assert spec["template"]["spec"]["securityContext"]["runAsNonRoot"] is True
assert not [e for e in (c.get("env") or []) if e["name"] == "SWARMAIL_DATA_FILE"]  # no persistence by default
# The startup probe is the one addition even a no-values install gets
# (decision 0026 §3): the restore window, on the documented healthz probe.
sp = c["startupProbe"]
assert sp["httpGet"] == {"path": "/healthz", "port": "http"}, sp
assert (sp["periodSeconds"], sp["timeoutSeconds"], sp["failureThreshold"]) == (10, 2, 90), sp
# ... and the unset TLS path renders no volume, no mount and no TLS env.
assert not [e for e in (c.get("env") or []) if e["name"].startswith("SWARMAIL_TLS_")]
assert not [v for v in (spec["template"]["spec"].get("volumes") or []) if v["name"] == "tls"]
assert not [m for m in (c.get("volumeMounts") or []) if m["name"] == "tls"]
svc = one(d, "Service")
assert [(p["name"], p["port"]) for p in svc["spec"]["ports"]] == [("smtp", 1025), ("pop3", 1110), ("http", 8025)]

# The annotation probe render: service.annotations are Service metadata, not
# labels (finding 4, issue #31).
asvc = one(f"{S}/chart-annot.yaml", "Service")
assert asvc["metadata"].get("annotations") == {"example": "probe"}, asvc["metadata"]
assert "example" not in (asvc["metadata"].get("labels") or {}), asvc["metadata"]

# Full profile: the deployment contract switches to the durable shape.
assert sorted(x["kind"] for x in docs(f)) == [
    "Deployment", "Ingress", "NetworkPolicy", "PersistentVolumeClaim",
    "Pod", "Service", "ServiceMonitor"], docs(f)
fdep = one(f, "Deployment")
assert fdep["spec"]["strategy"]["type"] == "Recreate"       # Recreate at every setting
assert fdep["spec"]["replicas"] == 1
env = {e["name"]: e.get("value") for e in (fdep["spec"]["template"]["spec"]["containers"][0].get("env") or [])}
assert env["SWARMAIL_DATA_FILE"] == "/data/swarmail.db"
assert env["SWARMAIL_MAX_PER_INBOX"] == "0"                 # config passthrough
assert one(f, "ServiceMonitor")["spec"]["endpoints"][0]["path"] == "/metrics"
assert one(f, "Ingress")["spec"]["rules"][0]["http"]["paths"][0]["backend"]["service"]["port"]["name"] == "http"

# The deployment boundary (decision 0026): peers render verbatim into the
# single ingress rule beside the same-namespace default, and TLS wiring is
# proved (volume + read-only mount + the binary's own env names) — never a
# certificate's lifetime (0020 rule 5).
fspec = fdep["spec"]["template"]["spec"]
fc = fspec["containers"][0]
froms = one(f, "NetworkPolicy")["spec"]["ingress"][0]["from"]
assert {"podSelector": {}} in froms, froms                              # the default posture stays
assert {"namespaceSelector": {"matchLabels": {"kubernetes.io/metadata.name": "monitoring"}}} in froms, froms
assert {"ipBlock": {"cidr": "10.0.0.0/8"}} in froms, froms
fenv = {e["name"]: e.get("value") for e in (fc.get("env") or [])}
assert fenv["SWARMAIL_TLS_CERT"] == "/etc/swarmail/tls/tls.crt", fenv
assert fenv["SWARMAIL_TLS_KEY"] == "/etc/swarmail/tls/tls.key", fenv
tls_vol = [v for v in (fspec.get("volumes") or []) if v["name"] == "tls"]
assert tls_vol == [{"name": "tls", "secret": {"secretName": "swarmail-tls"}}], tls_vol
tls_mount = [m for m in (fc.get("volumeMounts") or []) if m["name"] == "tls"]
assert tls_mount == [{"name": "tls", "mountPath": "/etc/swarmail/tls", "readOnly": True}], tls_mount
assert (fc["startupProbe"]["periodSeconds"], fc["startupProbe"]["failureThreshold"]) == (10, 90)

# The policy's locked default: peers unset renders exactly today's
# same-namespace posture and nothing else — default-off is byte-for-byte.
pd = one(f"{S}/chart-policy-default.yaml", "NetworkPolicy")
assert pd["spec"]["policyTypes"] == ["Ingress"], pd["spec"]
assert pd["spec"]["ingress"][0]["from"] == [{"podSelector": {}}], pd["spec"]

# NOTES.txt tells the truth about the claim both ways: the generated name
# only when the claim is generated, the operator's claim when supplied.
def notes_text(path):
    rendered = [d["notes"] for d in yaml.safe_load_all(open(path)) if isinstance(d, dict) and "notes" in d]
    assert len(rendered) == 1, f"{path}: expected exactly one notes doc"
    return rendered[0]

claimed = notes_text(f"{S}/notes-claimed.yaml")
generated = notes_text(f"{S}/notes-generated.yaml")
assert 'on the "operator-claim" claim' in claimed, claimed[-400:]
assert "swarmail-data" not in claimed, claimed[-400:]
# the scratch release is "notes", so the generated claim is its fullname:
assert 'on the "notes-swarmail-data" claim' in generated, generated[-400:]
assert "operator-claim" not in generated, generated[-400:]
print("chart contract: all assertions green")
EOF

echo "chart-verify: OK"
