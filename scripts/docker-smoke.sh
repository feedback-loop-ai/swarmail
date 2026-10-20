#!/usr/bin/env bash
# The image as it runs, through real Docker (issue #141, proposed 0051 §3).
# CI-only (the `docker-smoke` job): the hermetic verify seat installs
# nothing and does not run it; an operator can run it locally the same way.
#
#   1. Build the repository's image once, under a name unique to this run.
#   2. Persistence under the image's own uid: a fresh named volume, a mail
#      accepted with 250 over real SMTP, read back over REST after a
#      graceful `docker stop` and, separately, after a hard `docker kill`
#      — each time from a new container on the same volume.
#   3. The quickstart's cleanup: both documented ephemeral forms (`--rm`,
#      30 s stop grace) leave no anonymous `/data` volume behind; the old
#      form without `--rm` is shown to leak one (the control), which this
#      script then removes itself.
#   4. The build context: a fixture context with the repository's
#      `.dockerignore` and `target/` sentinels at the root, under tools/
#      and under evals/, plus a non-target control, exported through a
#      real `COPY . ` — only the control may arrive.
#
# Readiness is a protocol observation polled against a deadline, never a
# sleep the verdict depends on. Every container, volume and image this
# script creates carries its run id and is removed on exit, success or
# failure; nothing global is pruned. Ports are ephemeral, loopback-only.
set -euo pipefail
cd "$(dirname "$0")/.."

command -v docker >/dev/null || { echo "docker-smoke: docker is required" >&2; exit 1; }
command -v python3 >/dev/null || { echo "docker-smoke: python3 is required" >&2; exit 1; }

run="smoke-$$-$(date +%s)"
image="swarmail-${run}:local"
volume="swarmail-${run}-data"
work="$(mktemp -d "${TMPDIR:-/tmp}/docker-smoke.XXXXXX")"
containers=() volumes=("$volume") images=("$image")

cleanup() {
  local c v i
  for c in "${containers[@]}"; do docker rm -f "$c" >/dev/null 2>&1 || true; done
  for v in "${volumes[@]}"; do docker volume rm -f "$v" >/dev/null 2>&1 || true; done
  for i in "${images[@]}"; do docker image rm -f "$i" >/dev/null 2>&1 || true; done
  rm -rf "$work"
}
trap cleanup EXIT

fail() { echo "docker-smoke: FAIL: $*" >&2; exit 1; }
step() { echo "== $*"; }

# Poll <cmd...> until it succeeds or <seconds> pass.
until_ok() { # <seconds> <what> <cmd...>
  local deadline=$(( $(date +%s) + $1 )) what="$2"; shift 2
  until "$@" >/dev/null 2>&1; do
    [ "$(date +%s)" -lt "$deadline" ] || fail "$what did not happen within the deadline"
    sleep 0.2
  done
}

host_port() { # <container> <container-port>
  docker port "$1" "$2/tcp" | head -1 | sed 's/.*://'
}

# The SMTP and REST halves of the protocol, in one small client.
cat >"$work/client.py" <<'PY'
import json
import smtplib
import sys
import urllib.request

op, host = sys.argv[1], sys.argv[2]
if op == "ready":
    smtp_port, http_port = int(sys.argv[3]), int(sys.argv[4])
    with urllib.request.urlopen(f"http://{host}:{http_port}/healthz", timeout=2) as r:
        assert r.status == 200, r.status
    s = smtplib.SMTP(host, smtp_port, timeout=5)
    code, _ = s.noop()
    s.quit()
    assert code == 250, code
elif op == "send":
    smtp_port, inbox, marker = int(sys.argv[3]), sys.argv[4], sys.argv[5]
    s = smtplib.SMTP(host, smtp_port, timeout=10)
    s.ehlo()
    s.login(inbox, "smoke")
    msg = f"From: smoke@x.io\r\nTo: rcpt@x.io\r\nSubject: {marker}\r\n\r\n{marker}\r\n"
    s.mail("smoke@x.io")
    s.rcpt("rcpt@x.io")
    code, reply = s.data(msg.encode())
    s.quit()
    if code != 250:
        sys.exit(f"DATA answered {code} {reply!r}, not 250")
elif op == "has":
    http_port, inbox, marker = int(sys.argv[3]), sys.argv[4], sys.argv[5]
    # The whole inbox, not an await: await answers only its first `count`
    # matches, newest first, so an older mail would never be seen.
    url = f"http://{host}:{http_port}/api/v1/inboxes/{inbox}/messages?limit=250"
    with urllib.request.urlopen(url, timeout=10) as r:
        body = json.load(r)
    subjects = [e.get("subject") for e in body.get("emails", [])]
    if marker not in subjects:
        sys.exit(f"{marker} is not in {inbox}: {subjects}")
else:
    sys.exit(f"unknown op {op}")
PY
client() { python3 "$work/client.py" "$@"; }

# start <name> [docker-run options...] — a detached container of the image
# with loopback ephemeral ports, ready over both protocols.
start() {
  local name="$1"; shift
  containers+=("$name")
  docker run -d --name "$name" -p 127.0.0.1::1025 -p 127.0.0.1::8025 "$@" "$image" >/dev/null
  smtp="$(host_port "$name" 1025)" http="$(host_port "$name" 8025)"
  until_ok 60 "$name readiness (healthz + SMTP NOOP)" client ready 127.0.0.1 "$smtp" "$http"
}

anon_volume() { # <container> — the anonymous volume mounted at /data
  docker inspect -f '{{range .Mounts}}{{if eq .Destination "/data"}}{{.Name}}{{end}}{{end}}' "$1"
}

gone() { ! docker container inspect "$1" >/dev/null 2>&1; }

step "build the image ($image)"
docker build -q -t "$image" . >/dev/null

step "the image runs as its own non-root uid"
user="$(docker image inspect -f '{{.Config.User}}' "$image")"
[ "$user" = "65532:65532" ] || fail "the image's user is '$user', not 65532:65532"

step "persistence: a 250 survives a graceful stop (named volume, uid 65532)"
docker volume create "$volume" >/dev/null
persist=(--stop-timeout 30 -v "$volume:/data" -e SWARMAIL_DATA_FILE=/data/swarmail.db)
start "${run}-a" "${persist[@]}"
client send 127.0.0.1 "$smtp" smoke graceful-marker
client has 127.0.0.1 "$http" smoke graceful-marker
docker stop "${run}-a" >/dev/null
docker rm "${run}-a" >/dev/null
docker volume inspect "$volume" >/dev/null || fail "the named volume did not survive its container's removal"
start "${run}-b" "${persist[@]}"
client has 127.0.0.1 "$http" smoke graceful-marker || fail "the gracefully stopped mail did not restore"

step "persistence: a 250 survives a hard kill"
client send 127.0.0.1 "$smtp" smoke kill-marker
docker kill "${run}-b" >/dev/null
docker rm "${run}-b" >/dev/null
start "${run}-c" "${persist[@]}"
client has 127.0.0.1 "$http" smoke kill-marker || fail "the mail accepted before the kill did not restore"
client has 127.0.0.1 "$http" smoke graceful-marker || fail "the earlier mail did not survive the kill"
docker stop "${run}-c" >/dev/null
docker rm "${run}-c" >/dev/null

step "the quickstart leaves no anonymous volume (both documented forms)"
# README's two ephemeral `docker run` lines, as options: the bare one and
# the one with every port. Both carry --rm and the 30 s stop grace.
for form in quick full; do
  name="${run}-${form}"
  if [ "$form" = full ]; then
    start "$name" --rm --stop-timeout 30 -p 127.0.0.1::1110
  else
    start "$name" --rm --stop-timeout 30
  fi
  anon="$(anon_volume "$name")"
  [ -n "$anon" ] || fail "$form: no anonymous /data volume to watch"
  volumes+=("$anon")
  docker stop "$name" >/dev/null
  until_ok 30 "$form: --rm removing the container" gone "$name"
  if docker volume inspect "$anon" >/dev/null 2>&1; then
    fail "$form: the anonymous volume $anon outlived its --rm container"
  fi
done

step "control: without --rm, docker rm -f leaks the anonymous volume"
start "${run}-leak"
anon="$(anon_volume "${run}-leak")"
volumes+=("$anon")
docker rm -f "${run}-leak" >/dev/null
docker volume inspect "$anon" >/dev/null 2>&1 \
  || fail "control: the anonymous volume vanished without --rm; the cleanup check above proves nothing"

step "the build context carries no target/ directory at any depth"
ctx="$work/context"
mkdir -p "$ctx/target/release" "$ctx/tools/coverage-fn-scan/target/debug" "$ctx/evals/target/debug" "$ctx/tools/keep"
cp .dockerignore "$ctx/.dockerignore"
echo sentinel >"$ctx/target/release/SENTINEL"
echo sentinel >"$ctx/tools/coverage-fn-scan/target/debug/SENTINEL"
echo sentinel >"$ctx/evals/target/debug/SENTINEL"
echo control >"$ctx/tools/keep/CONTROL"
printf 'FROM scratch\nCOPY . /ctx\n' >"$ctx/Dockerfile"
docker build -q --output "type=local,dest=$work/exported" "$ctx" >/dev/null
[ -f "$work/exported/ctx/tools/keep/CONTROL" ] || fail "the context control file did not arrive; the export proves nothing"
leaked="$(cd "$work/exported" && find . -name SENTINEL)"
[ -z "$leaked" ] || fail "target/ sentinels entered the build context: $leaked"

echo "docker-smoke: OK — persistence across stop and kill, no leaked volume, no target/ in context"
