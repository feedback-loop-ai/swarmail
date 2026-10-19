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
  rm -rf "$work"
}
trap cleanup EXIT

: >"$recorded"
printf 'token=200\nmanifest=404\n' >"$control"

# The fixture: binds an ephemeral port, writes the port file only once the
# socket is listening, and re-reads the control file per request so one
# server serves every pole deterministically.
cat >"$work/server.py" <<'PY'
import json
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

control_path, recorded_path, port_path = sys.argv[1:4]

class Fixture(BaseHTTPRequestHandler):
    def _record(self):
        with open(recorded_path, "a") as f:
            f.write(self.path + "\n")

    def do_GET(self):
        self._record()
        with open(control_path) as f:
            control = dict(line.split("=", 1) for line in f.read().splitlines() if "=" in line)
        if self.path.startswith("/token"):
            if control.get("token", "200") != "200":
                self.send_response(int(control["token"]))
                self.send_header("Content-Length", "0")
                self.end_headers()
                return
            body = json.dumps({"token": "test-token"}).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return
        if self.path.startswith("/v2/"):
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
    f.write(str(server.server_address[1]))
server.serve_forever()
PY

python3 "$work/server.py" "$control" "$recorded" "$portfile" &
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
port="$(cat "$portfile")"
registry="http://127.0.0.1:${port}"
export GH_TOKEN=test-token GH_USER=test-user

fail() { echo "publish-guard-test: FAIL: $*" >&2; exit 1; }

attempt() { "$@" >"$out" 2>"$err"; }

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

# The refused pole: a port we just held and released, verified closed —
# immediate, not a service we hope is down.
closed_port="$(python3 - <<'PY'
import socket

holder = socket.socket()
holder.bind(("127.0.0.1", 0))
port = holder.getsockname()[1]
holder.close()
probe = socket.socket()
probe.settimeout(2)
refused = probe.connect_ex(("127.0.0.1", port)) != 0
probe.close()
if not refused:
    raise SystemExit(f"port {port} is unexpectedly accepting")
print(port)
PY
)"

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
expect_refusal "probe 500" 1 "registry state unknown" -- \
  env GITHUB_REF_NAME=v0.1.0 PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" v0.1.0 feedback-loop-ai/swarmail

# 10. Token exchange fails (HTTP 500): registry state unknown — refuse.
printf 'token=500\nmanifest=404\n' >"$control"
expect_refusal "token exchange 500" 1 "registry state unknown" -- \
  env GITHUB_REF_NAME=v0.1.0 PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" v0.1.0 feedback-loop-ai/swarmail

# 11. Connection refused (closed port): registry state unknown — refuse.
expect_refusal "connection refused" 1 "registry state unknown" -- \
  env GITHUB_REF_NAME=v0.1.0 PUBLISH_GUARD_REGISTRY="http://127.0.0.1:${closed_port}" \
  "$GUARD" v0.1.0 feedback-loop-ai/swarmail

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
printf 'token=200\nmanifest=404\n' >"$control"
python3 - "$control" <<'PY'
import sys

# The fixture keys on the path only; teach it to answer per-tag-form.
path = sys.argv[1]
PY

# 12c. The bare form already published also refuses (the push writes both).
printf 'token=200\nmanifest:0.1.0=200\nmanifest:v0.1.0=404\n' >"$control"
expect_refusal "bare form already published" 1 "0.1.0 is already published" -- \
  env GITHUB_REF_NAME=v0.1.0 PUBLISH_GUARD_REGISTRY="$registry" \
  "$GUARD" v0.1.0 feedback-loop-ai/swarmail

# 12d. The v-form already published refuses too.
printf 'token=200\nmanifest:0.1.0=404\nmanifest:v0.1.0=200\n' >"$control"
expect_refusal "v-form already published" 1 "v0.1.0 is already published" -- \
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

# 13. The drift pin: the workflow must reference the guard in both publish
#     paths, or the enforcement is a comment again.
guard_refs="$(grep -c 'scripts/publish-guard.sh' "$WORKFLOW" || true)"
[ "${guard_refs:-0}" -ge 2 ] \
  || fail "drift pin: $WORKFLOW references the guard ${guard_refs} times; both publish paths must call it"
call_sites="$(grep -c 'run: scripts/publish-guard.sh' "$WORKFLOW" || true)"
[ "${call_sites:-0}" -ge 2 ] \
  || fail "drift pin: $WORKFLOW invokes the guard from ${call_sites} run: steps; both publish paths must call it"

echo "publish-guard-test: OK — all poles green (refusals fire, the 404 arms proceed, both tag forms are probed, the workflow is wired)"
