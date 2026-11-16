#!/usr/bin/env bash
# Hermetic proof of scripts/publish-guard.sh — the exact script the CI
# publish paths call (decision 0019 §3). A real local HTTP server (python3,
# the chart-verify toolchain) speaks the two-endpoint OCI shape (token
# exchange + manifest GET), scripted per request via a control file; the
# poles are 200 / 404 / 500 and a closed port (connection refused). Inputs
# are injected (env + argv), there is no sleep as synchronization (the
# fixture signals readiness over a real deadline, and the refused pole is
# immediate) and no wall-clock validity assertion; everything is bound to
# 127.0.0.1. The last case pins the workflow to this script: an unwired
# guard is the finding's shape again (0020 §5, adapted to shell).
set -euo pipefail
# The proof and the guard it proves are the pinned bundle's bytes (decision
# 0050 §1): the guard is resolved beside this script, never from the
# candidate. The candidate — the working directory, its repository root —
# supplies only the workflow whose wiring is asserted.
pin_dir="$(CDPATH='' cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[ -f Cargo.toml ] || { echo "publish-guard-test: run from the candidate repository root" >&2; exit 1; }

GUARD="$pin_dir/publish-guard.sh"
WORKFLOW=.github/workflows/ci.yml

[ -f "$GUARD" ] || { echo "publish-guard-test: $GUARD missing" >&2; exit 1; }
[ -x "$GUARD" ] || { echo "publish-guard-test: $GUARD is not executable" >&2; exit 1; }
[ -f "$WORKFLOW" ] || { echo "publish-guard-test: $WORKFLOW missing" >&2; exit 1; }

work="$(mktemp -d "${TMPDIR:-/tmp}/publish-guard-test.XXXXXX")"
control="$work/control" recorded="$work/recorded" portfile="$work/port"
out="$work/out" err="$work/err"
server_pid=""
cleanup() {
  [ -n "$server_pid" ] && kill "$server_pid" 2>/dev/null
  [ -n "$server_pid" ] && wait "$server_pid" 2>/dev/null || true
  rm -rf "$work"
}
trap cleanup EXIT

: >"$recorded"
printf 'feedback-loop-ai/swarmail\n' >"$work/expected-repo"
printf 'token=200\nmanifest=404\n' >"$control"

# The fixture: binds an ephemeral port, writes the port file only once the
# socket is listening, and re-reads the control file per request so one
# server serves every pole deterministically.
cat >"$work/server.py" <<'PY'
import base64
import json
import socket
import sys
from urllib.parse import urlparse, parse_qs
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

control_path, recorded_path, port_path, expected_path = sys.argv[1:5]

basic = "Basic " + base64.b64encode(b"test-user:exchange-secret").decode()
accept = "application/vnd.oci.image.index.v1+json, application/vnd.oci.image.manifest.v1+json, application/vnd.docker.distribution.manifest.list.v2+json, application/vnd.docker.distribution.manifest.v2+json"
holder = socket.socket()
holder.bind(("127.0.0.1", 0))  # Owned, deliberately never listening.

class Fixture(BaseHTTPRequestHandler):
    def reply(self, code, body=b""):
        self.send_response(code)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _record(self):
        with open(recorded_path, "a") as f:
            f.write(self.path + "\n")

    def do_GET(self):
        if self.path == "/held":
            self.reply(200 if holder.fileno() >= 0 else 500, b"held")
            return
        self._record()
        expected_repo = open(expected_path).read().strip()
        with open(control_path) as f:
            control = dict(line.split("=", 1) for line in f.read().splitlines() if "=" in line)
        if self.path.startswith("/token"):
            parsed = urlparse(self.path)
            query = parse_qs(parsed.query)
            if (parsed.path != "/token" or query.get("service") != ["ghcr.io"]
                or query.get("scope") != [f"repository:{expected_repo}:pull"]
                or self.headers.get("Authorization") != basic):
                self.reply(403)
                return
            if control.get("token", "200") != "200":
                self.send_response(int(control["token"]))
                self.send_header("Content-Length", "0")
                self.end_headers()
                return
            body = control.get("tokenbody", json.dumps({"token": "bearer-secret"})).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return
        if self.path.startswith("/v2/"):
            if (self.headers.get("Authorization") != "Bearer bearer-secret" or self.headers.get("Accept") != accept
                or not self.path.startswith("/v2/"+expected_repo+"/manifests/")):
                self.reply(403)
                return
            if control.get("manifest") == "close":
                self.close_connection = True
                return
            if control.get("manifest") == "raw000":
                self.connection.sendall(b"HTTP/1.1 000 Unknown\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                self.close_connection = True
                return
            tag = self.path.rsplit("/", 1)[-1]
            if f"manifest:{tag}" in control:
                self.send_response(int(control[f"manifest:{tag}"]))
            else:
                self.send_response(int(control.get("manifest", "404")))
            self.send_header("Content-Length", "0")
            self.end_headers()
            return
        self.send_response(404)
        self.send_header("Content-Length", "0")
        self.end_headers()

    def log_message(self, *args):
        pass

server = ThreadingHTTPServer(("127.0.0.1", 0), Fixture)
with open(port_path, "w") as f:
    f.write(str(server.server_address[1])+" "+str(holder.getsockname()[1]))
server.serve_forever()
PY

python3 "$work/server.py" "$control" "$recorded" "$portfile" "$work/expected-repo" &
server_pid=$!

# Readiness over a real deadline — a poll with an expiry, never a fixed
# sleep the verdict depends on.
deadline=$(( $(date +%s) + 10 ))
until [ -s "$portfile" ]; do
  if [ "$(date +%s)" -ge "$deadline" ]; then
    echo "publish-guard-test: FAIL: fixture server never became ready" >&2
    exit 1
  fi
  sleep 0.05
done
read -r port closed_port <"$portfile" || true
registry="http://127.0.0.1:${port}"
export GH_TOKEN=exchange-secret GH_USER=test-user
# Test observer delegates to installed curl, preserving stdin and real exit
# codes. Only the successful-response status classifier control is synthetic.
export REAL_CURL="$(command -v curl)" ARGV_RECORD="$work/argv"
mkdir "$work/bin"
cat >"$work/bin/curl" <<'OBSERVER'
#!/usr/bin/env python3
import json, os, subprocess, sys
args = sys.argv[1:]
with open(os.environ["ARGV_RECORD"], "a") as f:
    f.write(json.dumps(args)+"\n")
if os.environ.get("ZERO_CLASSIFY") == "1" and any("/manifests/" in a for a in args):
    p = subprocess.run([os.environ["REAL_CURL"], *args], stdout=subprocess.PIPE)
    if p.returncode:
        sys.stdout.buffer.write(p.stdout)
        sys.exit(p.returncode)
    assert p.stdout == b"404", p.stdout
    sys.stdout.write("000")
else:
    os.execv(os.environ["REAL_CURL"], [os.environ["REAL_CURL"], *args])
OBSERVER
chmod +x "$work/bin/curl"
export REAL_PYTHON="$(command -v python3)"
printf '#!%s\n' "$REAL_PYTHON" >"$work/bin/python3"
cat >>"$work/bin/python3" <<'HELPER'
import json, os, sys
with open(os.environ["ARGV_RECORD"], "a") as f:
    f.write(json.dumps(sys.argv[1:])+"\n")
os.execv(os.environ["REAL_PYTHON"], [os.environ["REAL_PYTHON"], *sys.argv[1:]])
HELPER
chmod +x "$work/bin/python3"
export PATH="$work/bin:$PATH"

fail() { echo "publish-guard-test: FAIL: $*" >&2; exit 1; }

attempt() {
  local arg
  for arg in "$@"; do
    case "$arg" in feedback-loop-ai/*) printf '%s\n' "$arg" >"$work/expected-repo" ;; esac
  done
  "$@" >"$out" 2>"$err"
}

expect_proceed() { # <desc> -- <cmd...>
  local desc="$1"; shift; [ "$1" = "--" ] && shift
  attempt "$@" || fail "${desc}: the guard refused: $(cat "$err")"
}

expect_refusal() { # <desc> <expected-exit> <needle...> -- <cmd...>
  local desc="$1" want="$2" got; shift 2
  local -a needles=()
  while [ "$1" != "--" ]; do needles+=("$1"); shift; done
  shift
  set +e
  attempt "$@"
  got=$?
  set -e
  if [ "$got" -eq 0 ]; then
    fail "${desc}: the guard proceeded but must refuse"
  fi
  local needle
  [ "$got" -eq "$want" ] || fail "${desc}: expected exit ${want}, got ${got}: $(cat "$err")"
  for needle in "${needles[@]}"; do
    grep -q "$needle" "$err" || fail "${desc}: refusal must name '${needle}': $(cat "$err")"
  done
}

# Targeted unknown-state cases must reach precisely their intended stage,
# with the same repository scope and bare manifest identity as the caller.
assert_requests() { # <description> <none|token|manifest>
  local desc="$1" mode="$2" expected=""
  case "$mode" in
    none|token|manifest) ;;
    *) fail "$desc: invalid assertion mode '$mode'" ;;
  esac
  if [ "$mode" != none ]; then
    expected='/token?service=ghcr.io&scope=repository:feedback-loop-ai/swarmail:pull'
  fi
  if [ "$mode" = manifest ]; then
    expected+=$'\n/v2/feedback-loop-ai/swarmail/manifests/0.1.0'
  fi
  [ "$(cat "$recorded")" = "$expected" ] \
    || fail "$desc: unexpected registry request sequence: $(cat "$recorded")"
}

# Lifetime checks surround each refused connection; the child owns a bound
# non-listening socket until cleanup joins it. No released-port reuse.
assert_holder() {
  [ "$(curl -fsS --max-time 5 "$registry/held")" = held ] || fail "refusal socket ownership lost"
}
assert_holder

# 1. Degenerate input fails closed: no arguments, no publish.
expect_refusal "no arguments" 1 "usage" -- "$GUARD"

# 2. Exact ref + absent tag (404): proceed — and the probe hit the repo
#    path its caller asked for.
: >"$recorded"
expect_proceed "exact ref + 404" -- env GITHUB_REF_NAME=v0.1.0 PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" v0.1.0 feedback-loop-ai/swarmail
grep -q "/v2/feedback-loop-ai/swarmail/manifests/0.1.0" "$recorded" \
  || fail "exact ref + 404: the guard probed the wrong repo path: $(cat "$recorded")"

# 3. Mismatched ref (an rc tag carrying the version): refuse, naming both
#    the triggering and the expected ref.
: >"$recorded"
expect_refusal "mismatched ref" 1 "ref 'v0.1.0-rc' is not 'v0.1.0'" -- \
  env GITHUB_REF_NAME=v0.1.0-rc PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" v0.1.0 feedback-loop-ai/swarmail

# 4. Unset GITHUB_REF_NAME: refuse, named.
expect_refusal "unset ref" 1 "GITHUB_REF_NAME is unset" -- \
  env -u GITHUB_REF_NAME PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" v0.1.0 feedback-loop-ai/swarmail

# 5. Non-semver expected version: refuse — a malformed manifest never
#    publishes something else.
expect_refusal "non-semver (leading zero)" 1 "not valid semver" -- \
  env GITHUB_REF_NAME=v01.0 PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" v01.0 feedback-loop-ai/swarmail
expect_refusal "non-semver (garbage)" 1 "not valid semver" -- \
  env GITHUB_REF_NAME=vabc PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" vabc feedback-loop-ai/swarmail

# 6. Build metadata ('+'): valid semver, refused — helm push maps '+' to
#    '_' in OCI tags, so the probe could not check the pushed tag.
expect_refusal "build metadata" 1 "build metadata" -- \
  env GITHUB_REF_NAME=v0.1.0+build PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" v0.1.0+build feedback-loop-ai/swarmail

# 7. Expected ref without the v prefix: refuse, named.
expect_refusal "ref not v-prefixed" 1 "expected ref is not v-prefixed" -- \
  env GITHUB_REF_NAME=0.1.0 PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" 0.1.0 feedback-loop-ai/swarmail

# 8. Probe answers 200: the version is already published — refuse.
printf 'token=200\nmanifest=200\n' >"$control"
expect_refusal "probe 200" 1 "already published" -- \
  env GITHUB_REF_NAME=v0.1.0 PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" v0.1.0 feedback-loop-ai/swarmail

# 9. Probe answers 500: registry state unknown — refuse, never publish.
printf 'token=200\nmanifest=500\n' >"$control"
: >"$recorded"
expect_refusal "probe 500" 1 "HTTP 500" -- \
  env GITHUB_REF_NAME=v0.1.0 PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" v0.1.0 feedback-loop-ai/swarmail
assert_requests "probe 500" manifest

# 10. Token exchange fails (HTTP 500): registry state unknown — refuse.
printf 'token=500\nmanifest=404\n' >"$control"
: >"$recorded"
expect_refusal "token exchange 500" 1 "token exchange failed" -- \
  env GITHUB_REF_NAME=v0.1.0 PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" v0.1.0 feedback-loop-ai/swarmail
assert_requests "token exchange 500" token
# The unknown mode collides with this same token-only request sequence; it
# must refuse by mode before log comparison, not because the log differs.
invalid_mode=token-only-typo
invalid_mode_err="$work/invalid-mode.err"
set +e
( assert_requests "token exchange 500" "$invalid_mode" ) 2>"$invalid_mode_err"
invalid_mode_status=$?
set -e
[ "$invalid_mode_status" -ne 0 ] || fail "token exchange 500: invalid mode '$invalid_mode' was accepted"
grep -Fq "token exchange 500: invalid assertion mode '$invalid_mode'" "$invalid_mode_err" \
  || fail "token exchange 500: invalid mode refusal did not identify '$invalid_mode': $(cat "$invalid_mode_err")"

# 11. Connection refused (closed port): registry state unknown — refuse.
: >"$recorded"
expect_refusal "connection refused" 1 "token exchange failed" -- \
  env GITHUB_REF_NAME=v0.1.0 PUBLISH_GUARD_REGISTRY="http://127.0.0.1:${closed_port}" \
  "$GUARD" v0.1.0 feedback-loop-ai/swarmail
assert_requests "connection refused" none

assert_holder

# 12. The chart call site: helm push writes the chart version verbatim
#     (bare, no v-prefix option), so the guard probes exactly that — and
#     must NOT probe a v-form tag the push never writes (the wrong-name
#     trap: a stale v0.1.0 would block a legitimate 0.1.0 publish).
printf 'token=200\nmanifest=404\n' >"$control"
: >"$recorded"
expect_proceed "chart repo + 404" -- env GITHUB_REF_NAME=v0.1.0 PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" v0.1.0 feedback-loop-ai/charts/swarmail bare
grep -q "/v2/feedback-loop-ai/charts/swarmail/manifests/0.1.0" "$recorded" \
  || fail "chart repo: the guard probed the wrong repo path: $(cat "$recorded")"
if grep -q "/v2/feedback-loop-ai/charts/swarmail/manifests/v0.1.0" "$recorded"; then
  fail "chart repo: the guard probed manifests/v0.1.0, a tag the chart push never writes: $(cat "$recorded")"
fi

# 12b. A stale v-form tag refuses the docker publish even with the bare
#      form absent (the release writes both forms; either existing blocks).
printf 'token=200\nmanifest:0.1.0=404\nmanifest:v0.1.0=200\n' >"$control"
: >"$recorded"
expect_refusal "stale v-form with bare absent" 1 "v0.1.0 is already published" -- \
  env GITHUB_REF_NAME=v0.1.0 PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" v0.1.0 feedback-loop-ai/swarmail
for tag in 0.1.0 v0.1.0; do
  grep -qx "/v2/feedback-loop-ai/swarmail/manifests/$tag" "$recorded" || fail "12b: missing $tag probe"
done

# 12c. The bare form already published also refuses (the push writes both).
printf 'token=200\nmanifest:0.1.0=200\nmanifest:v0.1.0=404\n' >"$control"
expect_refusal "bare form already published" 1 "0.1.0 is already published" -- \
  env GITHUB_REF_NAME=v0.1.0 PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" v0.1.0 feedback-loop-ai/swarmail

# 12e. Both forms absent: the docker publish path proceeds, and the probe
#      visited both tag forms the push will write.
printf 'token=200\nmanifest=404\n' >"$control"
: >"$recorded"
expect_proceed "docker repo probes both forms" -- env GITHUB_REF_NAME=v0.1.0 PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" v0.1.0 feedback-loop-ai/swarmail
grep -q "/v2/feedback-loop-ai/swarmail/manifests/0.1.0" "$recorded" \
  || fail "docker repo: the bare form was not probed: $(cat "$recorded")"
grep -q "/v2/feedback-loop-ai/swarmail/manifests/v0.1.0" "$recorded" \
  || fail "docker repo: the v-form was not probed: $(cat "$recorded")"

# 12f. An unknown probe-forms value refuses (fail closed, named).
printf 'token=200\nmanifest=404\n' >"$control"
expect_refusal "unknown probe-forms value" 1 "neither 'both' nor 'bare'" -- \
  env GITHUB_REF_NAME=v0.1.0 PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" v0.1.0 feedback-loop-ai/swarmail tagish

# 14. The chart's image preflight (proposed 0051 §1, issue #141): the chart
#     publishes only once the image its appVersion names is published.
#     The image job's result arrives as DOCKER_RESULT; anything but
#     `success` refuses before the registry is asked at all.
present() { # <docker-result|-> <ref> <expected> [probe-forms] -- the preflight call
  local result="$1" ref="$2" expected="$3" forms="${4:-bare}"
  printf 'feedback-loop-ai/swarmail\n' >"$work/expected-repo"
  if [ "$result" = - ]; then
    env -u DOCKER_RESULT GITHUB_REF_NAME="$ref" PUBLISH_GUARD_REGISTRY="$registry" \
      "$GUARD" "$expected" feedback-loop-ai/swarmail "$forms" require-present
  else
    env DOCKER_RESULT="$result" GITHUB_REF_NAME="$ref" PUBLISH_GUARD_REGISTRY="$registry" \
      "$GUARD" "$expected" feedback-loop-ai/swarmail "$forms" require-present
  fi
}

# 14a. Image published (200) after a successful image job: proceed, probing
#      exactly the bare tag the chart's default image names.
printf 'token=200\nmanifest=200\n' >"$control"
: >"$recorded"
expect_proceed "require-present + success + 200" -- present success v0.1.0 v0.1.0
grep -qx "/v2/feedback-loop-ai/swarmail/manifests/0.1.0" "$recorded" \
  || fail "require-present: the bare image tag was not probed: $(cat "$recorded")"
if grep -q "/manifests/v0.1.0" "$recorded"; then
  fail "require-present: probed the v-form, which the chart never names: $(cat "$recorded")"
fi

# 14b. Every other image-job result refuses, before any registry request.
for result in failure skipped cancelled "" -; do
  : >"$recorded"
  expect_refusal "require-present + image job '${result}'" 1 "not 'success'" -- \
    present "$result" v0.1.0 v0.1.0
  [ ! -s "$recorded" ] \
    || fail "require-present + image job '${result}': the registry was asked anyway: $(cat "$recorded")"
done

# 14c. The image is missing (404): refuse — the opposite pole of the
#      absence probe, which would have proceeded here.
printf 'token=200\nmanifest=404\n' >"$control"
expect_refusal "require-present + 404" 1 "0.1.0 is not published" -- present success v0.1.0 v0.1.0

# 14d. Unknown registry state refuses: 500, a failed token exchange, a
#      refused connection.
printf 'token=200\nmanifest=500\n' >"$control"
: >"$recorded"
expect_refusal "require-present + 500" 1 "HTTP 500" -- present success v0.1.0 v0.1.0
assert_requests "require-present + 500" manifest
printf 'token=500\nmanifest=200\n' >"$control"
: >"$recorded"
expect_refusal "require-present + token 500" 1 "token exchange failed" -- present success v0.1.0 v0.1.0
assert_requests "require-present + token 500" token
assert_holder
: >"$recorded"
expect_refusal "require-present + connection refused" 1 "token exchange failed" -- \
  env DOCKER_RESULT=success GITHUB_REF_NAME=v0.1.0 PUBLISH_GUARD_REGISTRY="http://127.0.0.1:${closed_port}" \
  "$GUARD" v0.1.0 feedback-loop-ai/swarmail bare require-present
assert_requests "require-present + connection refused" none

assert_holder

# 14e. Only the bare form, only a known mode, only the exact ref.
printf 'token=200\nmanifest=200\n' >"$control"
expect_refusal "require-present + both forms" 1 "bare tag only" -- present success v0.1.0 v0.1.0 both
expect_refusal "unknown mode" 1 "neither 'require-absent' nor 'require-present'" -- \
  env DOCKER_RESULT=success GITHUB_REF_NAME=v0.1.0 PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" v0.1.0 feedback-loop-ai/swarmail bare present-ish
expect_refusal "require-present + mismatched ref" 1 "ref 'v0.1.1' is not 'v0.1.0'" -- \
  present success v0.1.1 v0.1.0
expect_refusal "require-present + non-semver" 1 "not valid semver" -- present success vabc vabc

# 14f. The explicit default mode is the absence probe, unchanged.
printf 'token=200\nmanifest=200\n' >"$control"
expect_refusal "require-absent + 200" 1 "already published" -- \
  env GITHUB_REF_NAME=v0.1.0 PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" v0.1.0 feedback-loop-ai/swarmail both require-absent

# 15. Issue #141's tree, as a fixture: Cargo and appVersion 0.1.0, chart
#     0.1.1, tag v0.1.1. The image guard refuses (the ref is not
#     v<Cargo version>); the chart's own absence guard alone would admit it
#     — that was the chart-only release — and the image preflight refuses
#     even if the image job were somehow reported green. Neither publishes.
printf 'token=200\nmanifest=404\n' >"$control"
expect_refusal "v0.1.1 tag: the image guard" 1 "ref 'v0.1.1' is not 'v0.1.0'" -- \
  env GITHUB_REF_NAME=v0.1.1 PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" v0.1.0 feedback-loop-ai/swarmail
expect_proceed "v0.1.1 tag: the chart absence guard alone admits it" -- \
  env GITHUB_REF_NAME=v0.1.1 PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" v0.1.1 feedback-loop-ai/charts/swarmail bare
printf 'token=200\nmanifest=200\n' >"$control"
expect_refusal "v0.1.1 tag: the chart's image preflight" 1 "ref 'v0.1.1' is not 'v0.1.0'" -- \
  present success v0.1.1 v0.1.0

# Distinct unknown-state proofs. A real token success followed by a closed
# manifest connection exercises transport failure, separate from HTTP errors.
printf 'token=200\nmanifest=close\n' >"$control"
: >"$recorded"
expect_refusal "manifest transport" 1 "manifest query failed" -- \
  env GITHUB_REF_NAME=v0.1.0 PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" v0.1.0 feedback-loop-ai/swarmail
assert_requests "manifest transport" manifest
# Installed-curl reachability experiment: an HTTP 000 status line is invalid,
# so curl must fail before classification. This is not successful HTTP 000.
printf 'token=200\nmanifest=raw000\n' >"$control"
: >"$recorded"
expect_refusal "raw 000 transport" 1 "manifest query failed" -- \
  env GITHUB_REF_NAME=v0.1.0 PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" v0.1.0 feedback-loop-ai/swarmail
assert_requests "raw 000 transport" manifest
echo "raw HTTP-000 experiment: $(curl --version | head -1); $(cat "$err")"
printf 'token=200\nmanifest=404\n' >"$control"
: >"$recorded"
expect_refusal "000 classifier only after real authenticated 404" 1 "HTTP 000" -- \
  env ZERO_CLASSIFY=1 GITHUB_REF_NAME=v0.1.0 PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" v0.1.0 feedback-loop-ai/swarmail
assert_requests "000 classifier only after real authenticated 404" manifest

for v in 01.0.0 1.0.0-01 1.0.0-.a 1.0.0-a. 1.0.0-a..b 1.0.0- 1.0 '1.0.0-!'; do
  expect_refusal "strict semver $v" 1 "not valid semver" -- \
    env GITHUB_REF_NAME="v$v" PUBLISH_GUARD_REGISTRY="$registry" "$GUARD" "v$v" feedback-loop-ai/swarmail
done
for v in 0.0.0 1.0.0-0 1.0.0-alpha.1 1.0.0-alpha-01; do
  expect_proceed "valid semver $v" -- \
    env GITHUB_REF_NAME="v$v" PUBLISH_GUARD_REGISTRY="$registry" "$GUARD" "v$v" feedback-loop-ai/swarmail
done
for body in '{}' '{"token":null}' '{"token":12}' '{"token":[]}' '{"token":""}' '{"token":"bad\r\nInjected: yes"}' '{"token":"bad\u0000"}' '{"token":"two words"}' 'bad-json'; do
  printf 'token=200\nmanifest=404\ntokenbody=%s\n' "$body" >"$control"
  : >"$recorded"
  expect_refusal "invalid bearer response" 1 "token exchange failed" -- \
    env GITHUB_REF_NAME=v0.1.0 PUBLISH_GUARD_REGISTRY="$registry" "$GUARD" v0.1.0 feedback-loop-ai/swarmail
  ! grep -q '/manifests/' "$recorded" || fail "invalid token reached manifest"
done
# Only synthetic credentials were used; record a verdict, never secret bytes.
python3 - "$ARGV_RECORD" <<'ARGS'
import base64, json, sys
secret = ("exchange-secret", "bearer-secret", base64.b64encode(b"test-user:exchange-secret").decode())
for line in open(sys.argv[1]):
    assert not any(s in arg for arg in json.loads(line) for s in secret), "credential entered curl argv"
print("credential-free argv (real curl and Python helpers), HTTP headers and distinct failures: OK")
ARGS

# Repository-specific workflow contract. Parsed active steps, exact supported
# conditions and run bodies fail closed on unknown bypass shapes. This is not
# a general Actions expression evaluator. Mutants use this very checker.
python3 - "$WORKFLOW" <<'WIRING'
import copy, hashlib, re, sys, yaml
workflow = yaml.safe_load(open(sys.argv[1]))
ordinary = {"quality", "test", "coverage", "deny", "dependency-audit", "brokkr-compile"}
def normalized(s):
    return re.sub(r"\s+", "", s or "")
tag = "startsWith(github.ref, 'refs/tags/v')"
build_if = "!cancelled() && github.event_name == 'push' " + " ".join("&& needs."+n+".result == 'success'" for n in ("quality", "test", "coverage", "deny", "dependency-audit", "brokkr-compile", "chart-verify", "docker-smoke")) + " && ((github.ref == 'refs/heads/main' && needs.release-rate-5k.result == 'skipped') || ("+tag+" && needs.release-rate-5k.result == 'success'))"
docker_if = "github.event_name == 'push' && needs.docker-build.result == 'success'"
chart_if = "github.event_name == 'push' && "+tag+" && needs.docker.result == 'success'"
image_guard = 'scripts/publish-guard.sh "v$VERSION" feedback-loop-ai/swarmail'
preflight = 'scripts/publish-guard.sh "v$APP_VERSION" feedback-loop-ai/swarmail bare require-present'
chart_guard = 'scripts/publish-guard.sh "v$VERSION" feedback-loop-ai/charts/swarmail bare'
helm_push = 'helm package charts/swarmail\nhelm push swarmail-*.tgz oci://ghcr.io/feedback-loop-ai/charts'
smoke_run = 'bash scripts/docker-smoke.sh'
# Each platform is built on a native runner of its own architecture and
# pushed by digest only; no tag is ever written outside the docker job.
native = {"linux/amd64": "ubuntu-latest", "linux/arm64": "ubuntu-24.04-arm"}
digest_output = "type=image,name=ghcr.io/feedback-loop-ai/swarmail,push-by-digest=true,name-canonical=true,push=true"
pushed_digest = "ghcr.io/feedback-loop-ai/swarmail@${{ steps.build.outputs.digest }}"
# The one step that writes image tags: exact text, so a changed tag set,
# a dropped platform check or a softened refusal is a deliberate edit here.
tag_push = r"""set -euo pipefail
repo=ghcr.io/feedback-loop-ai/swarmail
cd "$RUNNER_TEMP/digests"
digests=(*)
[ "${#digests[@]}" -eq 2 ] || { echo "REFUSAL: expected 2 platform digests, found ${#digests[@]}: ${digests[*]}" >&2; exit 1; }
sources=()
for d in "${digests[@]}"; do
  [[ "$d" =~ ^[0-9a-f]{64}$ ]] || { echo "REFUSAL: '$d' is not a sha256 digest" >&2; exit 1; }
  sources+=("$repo@sha256:$d")
done
if [ "$GITHUB_REF" = refs/heads/main ]; then
  tags=("$repo:main" "$repo:sha-$GITHUB_SHA")
elif [[ "$GITHUB_REF" = refs/tags/v* ]]; then
  tags=("$repo:v$VERSION" "$repo:$VERSION")
else
  echo "REFUSAL: $GITHUB_REF publishes no image" >&2; exit 1
fi
args=()
for t in "${tags[@]}"; do args+=(--tag "$t"); done
docker buildx imagetools create "${args[@]}" "${sources[@]}"
for t in "${tags[@]}"; do
  platforms="$(docker buildx imagetools inspect --raw "$t" \
    | python3 -c 'import json,sys; m=json.load(sys.stdin)["manifests"]; print(" ".join(sorted({p["platform"]["os"]+"/"+p["platform"]["architecture"] for p in m if p.get("platform",{}).get("os")!="unknown"})))')"
  [ "$platforms" = "linux/amd64 linux/arm64" ] || { echo "REFUSAL: $t names '$platforms', not linux/amd64 linux/arm64" >&2; exit 1; }
  echo "$t: $platforms"
done""".strip()
version_input = "${{ steps.ver.outputs.version }}"
admission = {
    "docker-build": (ordinary | {"chart-verify", "docker-smoke", "release-rate-5k"}, build_if),
    "docker": ({"docker-build"}, docker_if),
    "chart-publish": (ordinary | {"docker", "chart-verify", "release-rate-5k"}, chart_if),
}
def check(w):
    jobs = w["jobs"]
    # A changed default shell can ignore failure; retain the default
    # failure-propagating bash behavior at every publish boundary.
    assert "defaults" not in w, "workflow: shell defaults"
    for name, (needs, cond) in admission.items():
        j = jobs[name]
        assert set(j["needs"]) == needs, name+": prerequisites"
        assert normalized(j["if"]) == normalized(cond), name+": admission condition"
        assert "continue-on-error" not in j, name+": job continue-on-error"
        assert "defaults" not in j, name+": shell defaults"
        for step in j["steps"]:
            assert "continue-on-error" not in step, name+": step continue-on-error"
            assert "shell" not in step, name+": custom shell"
    def one_run(name, body, cond=None):
        steps = jobs[name]["steps"]
        matches = [(i,s) for i,s in enumerate(steps) if s.get("run", "").strip() == body]
        assert len(matches) == 1, name+": required run body"
        i, step = matches[0]
        assert normalized(step.get("if")) == normalized(cond), name+": guard/push condition"
        return i, step
    # Only docker-build builds, and only docker writes image tags.
    for name, j in jobs.items():
        for s in j.get("steps", []):
            if "docker/build-push-action@" in s.get("uses", ""):
                assert name == "docker-build", name+": image build outside docker-build"
            if "imagetools create" in s.get("run", "") or "docker push" in s.get("run", ""):
                assert name == "docker" and s.get("run", "").strip() == tag_push, name+": image tag write outside the guarded tag push"
    # docker-build: both platforms, each native, each an untagged digest
    # smoked on its own architecture before the docker job may tag it.
    j = jobs["docker-build"]
    assert set(j["strategy"]["matrix"]) == {"include"}, "docker-build: matrix shape"
    legs = j["strategy"]["matrix"]["include"]
    assert len(legs) == len(native) and {(l["platform"], l["runner"]) for l in legs} == set(native.items()), "docker-build: native platforms"
    assert j["runs-on"] == "${{ matrix.runner }}", "docker-build: native runner"
    builds = [(i,s) for i,s in enumerate(j["steps"]) if "docker/build-push-action@" in s.get("uses", "")]
    assert len(builds) == 1, "docker-build: one build"
    build, bs = builds[0]
    assert bs.get("id") == "build" and "if" not in bs, "docker-build: build condition"
    assert bs["with"].get("platforms") == "${{ matrix.platform }}", "docker-build: build platform"
    assert bs["with"].get("outputs") == digest_output and "tags" not in bs["with"] and "push" not in bs["with"], "docker-build: untagged digest push"
    smoke, ss = one_run("docker-build", smoke_run)
    assert build < smoke, "docker-build: smoke order"
    assert ss.get("env", {}) == {"DOCKER_SMOKE_IMAGE": pushed_digest, "DOCKER_SMOKE_PLATFORM": "${{ matrix.platform }}"}, "docker-build: smoke the pushed digest natively"
    # docker: the release guard precedes the one tag write.
    guard, gs = one_run("docker", image_guard, tag)
    push, ps = one_run("docker", tag_push)
    assert guard < push, "docker: guard order"
    assert gs.get("env", {}).get("VERSION") == version_input, "docker: version input"
    assert ps.get("env", {}).get("VERSION") == version_input, "docker: tag version input"
    # chart-publish
    pre, ps = one_run("chart-publish", preflight)
    guard, _ = one_run("chart-publish", chart_guard)
    push, _ = one_run("chart-publish", helm_push)
    assert pre < guard < push, "chart: guard order"
    assert ps.get("env", {}).get("DOCKER_RESULT") == "${{ needs.docker.result }}", "chart: docker result input"
    assert ps.get("env", {}).get("APP_VERSION") == "${{ steps.chartver.outputs.app }}", "chart: app input"
    return True
check(workflow)
count = 0
def mutant(desc, mutate):
    global count
    w = copy.deepcopy(workflow)
    mutate(w)
    assert w != workflow, "no-op mutant: "+desc
    encoded = yaml.safe_dump(w, sort_keys=False)
    try:
        check(yaml.safe_load(encoded))
    except (AssertionError, KeyError, TypeError) as error:
        count += 1
        check(workflow)
        print("workflow mutation red/restored-green: %s; sha256=%s; refusal=%s" % (desc, hashlib.sha256(encoded.encode()).hexdigest(), error))
    else:
        raise AssertionError("workflow mutation admitted: "+desc)
def step(w, job, body):
    return next(s for s in w["jobs"][job]["steps"] if s.get("run", "").strip() == body)
def build_step(w):
    return next(s for s in w["jobs"]["docker-build"]["steps"] if "docker/build-push-action@" in s.get("uses", ""))
def move(w, job, body, where):
    steps = w["jobs"][job]["steps"]
    s = steps.pop(steps.index(step(w,job,body)))
    steps.insert(0 if where == "front" else len(steps), s)
# A guard moves to the end (after what it guards); a push or smoke moves
# to the front (before its guard or its build).
for job, body, where in (("docker", image_guard, "end"), ("docker", tag_push, "front"), ("docker-build", smoke_run, "front"), ("chart-publish", preflight, "end"), ("chart-publish", chart_guard, "end")):
    mutant(job+body+" removed", lambda w: w["jobs"][job]["steps"].remove(step(w,job,body)))
    mutant(job+body+" commented", lambda w: step(w,job,body).update(run="# "+body))
    mutant(job+body+" ignored", lambda w: step(w,job,body).update(run=body+" || true"))
    mutant(job+body+" skipped", lambda w: step(w,job,body).update({"if":"false"}))
    mutant(job+body+" softened", lambda w: step(w,job,body).update({"continue-on-error": True}))
    mutant(job+body+" moved", lambda w: move(w,job,body,where))
for job in admission:
    mutant(job+" job softened", lambda w: w["jobs"][job].update({"continue-on-error": True}))
    mutant(job+" if always", lambda w: w["jobs"][job].update({"if":"always()"}))
    mutant(job+" if false", lambda w: w["jobs"][job].update({"if":"false"}))
    for prerequisite in workflow["jobs"][job]["needs"]:
        mutant(job+" missing "+prerequisite, lambda w: w["jobs"][job]["needs"].remove(prerequisite))
mutant("dropped docker success", lambda w: w["jobs"]["chart-publish"].update({"if":chart_if.replace(" && needs.docker.result == 'success'", "")}))
mutant("dropped docker result", lambda w: step(w,"chart-publish",preflight)["env"].pop("DOCKER_RESULT"))
mutant("ignored helm failure", lambda w: step(w,"chart-publish",helm_push).update(run=helm_push+" || true"))
mutant("helm publish softened", lambda w: step(w,"chart-publish",helm_push).update({"continue-on-error":True}))
mutant("helm publish condition bypass", lambda w: step(w,"chart-publish",helm_push).update({"if":"always()"}))
mutant("dropped tag version input", lambda w: step(w,"docker",tag_push)["env"].pop("VERSION"))
mutant("tag push accepts one platform", lambda w: step(w,"docker",tag_push).update(run=tag_push.replace('"linux/amd64 linux/arm64" ]', '"linux/amd64" ]')))
mutant("arm64 leg dropped", lambda w: w["jobs"]["docker-build"]["strategy"]["matrix"]["include"].pop())
mutant("arm64 leg emulated", lambda w: w["jobs"]["docker-build"]["strategy"]["matrix"]["include"][1].update(runner="ubuntu-latest"))
mutant("leg excluded", lambda w: w["jobs"]["docker-build"]["strategy"]["matrix"].update(exclude=[{"arch": "arm64"}]))
mutant("build tagged", lambda w: build_step(w)["with"].update(tags="ghcr.io/feedback-loop-ai/swarmail:main"))
mutant("build pushes tags", lambda w: build_step(w)["with"].update(outputs=digest_output.replace(",push-by-digest=true", "")))
mutant("build softened", lambda w: build_step(w).update({"continue-on-error":True}))
mutant("build condition bypass", lambda w: build_step(w).update({"if":"always()"}))
mutant("smoke not the pushed digest", lambda w: step(w,"docker-build",smoke_run)["env"].update(DOCKER_SMOKE_IMAGE="ghcr.io/feedback-loop-ai/swarmail:main"))
mutant("smoke platform unchecked", lambda w: step(w,"docker-build",smoke_run)["env"].pop("DOCKER_SMOKE_PLATFORM"))
mutant("tagged build in docker", lambda w: w["jobs"]["docker"]["steps"].append(copy.deepcopy(build_step(w))))
mutant("unguarded tag write", lambda w: w["jobs"]["docker-build"]["steps"].append({"run": "docker buildx imagetools create --tag ghcr.io/feedback-loop-ai/swarmail:main x"}))
check(workflow)
print("workflow active-step checker: %d mutation-red / restored-green receipts" % count)
WIRING

echo "publish-guard-test: OK — real HTTP, credential-free argv, strict versions and guarded publication"
