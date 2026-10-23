#!/usr/bin/env bash
# publish-guard.sh — the release publish path's only enforcement (decision
# 0019). Refuses unless the triggering ref is exactly the expected one AND
# the registry probe reports the version absent (404). 200 refuses: already
# published. Anything else — 5xx, refused connection, failed token
# exchange — refuses: registry state unknown. This is a pre-push check,
# NOT registry-side immutability; a concurrent publish can still race the
# probe, and re-publishing a version means deleting the registry tag.
#   usage: publish-guard.sh <v<semver>> <oci-repo> [probe-forms] [mode]
#   probe-forms: `both` (default) or `bare`. A docker release pushes the
#          version in BOTH tag forms (`:0.1.0` and `:v0.1.0` — the docs'
#          `v`-form and the tooling's bare form), so its guard refuses
#          unless both are absent. `helm push` writes the chart version
#          verbatim (bare only — it has no v-prefix option), so the chart
#          call site passes `bare`: probing `v<version>` there would check
#          a tag the push never writes — the same wrong-name trap as '+'
#          build metadata below.
#   mode:  `require-absent` (default) — the pre-push republish refusal
#          above — or `require-present`, the chart's image preflight
#          (proposed 0051 §1): the chart's default image must already be
#          published before the chart that names it is. In this mode 200
#          proceeds, 404 refuses (missing) and anything else refuses
#          (unknown); the probe form must be `bare` (the tag the chart's
#          appVersion names) and DOCKER_RESULT must be exactly `success`
#          — the image job's own result, handed over from the job
#          dependency, so a later edit of the job's condition cannot
#          publish a chart past a failed, skipped or cancelled image.
#   env:   GITHUB_REF_NAME (the triggering ref; unset refuses)
#          DOCKER_RESULT            require-present only: the image job's
#                                   result (`needs.docker.result`)
#          GH_TOKEN, GH_USER        registry credentials (the push's own)
#          PUBLISH_GUARD_REGISTRY   base URL; default https://ghcr.io
#                                   (the hermetic test's seam)
set -euo pipefail
expected="${1:?usage: publish-guard.sh <v<semver>> <oci-repo>}"
repo="${2:?usage: publish-guard.sh <v<semver>> <oci-repo>}"
refuse() { echo "publish-guard: refusing: $*" >&2; exit 1; }

ref="${GITHUB_REF_NAME:-}"
[ -n "$ref" ] || refuse "GITHUB_REF_NAME is unset"
[ "$ref" = "$expected" ] || refuse "ref '${ref}' is not '${expected}'"

version="${expected#v}"
[ "$version" != "$expected" ] || refuse "expected ref is not v-prefixed"
# SemVer core and prerelease identifiers, including numeric leading-zero rules.
[[ "$version" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?(\+[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$ ]] \
  || refuse "'${version}' is not valid semver"
pre="${version%%+*}"
if [[ "$pre" = *-* ]]; then
  pre="${pre#*-}"
  IFS=. read -r -a identifiers <<<"$pre"
  for id in "${identifiers[@]}"; do
    [[ "$id" =~ ^0[0-9]+$ ]] && refuse "'${version}' is not valid semver"
  done
fi
# Build metadata is valid semver but unprovable here: helm push maps '+' to
# '_' in OCI tags, so the probe would check a tag the push never writes,
# and a docker tag cannot carry '+' at all. Refuse rather than check the
# wrong name (the review's low residual on 0be6330).
case "$version" in
  *+*) refuse "'${version}' carries build metadata ('+'); the probe cannot check the tag the push would write" ;;
esac

probe_forms="${3:-both}"
case "$probe_forms" in
  both|bare) ;;
  *) refuse "probe forms '${probe_forms}' is neither 'both' nor 'bare'" ;;
esac

mode="${4:-require-absent}"
case "$mode" in
  require-absent) ;;
  require-present)
    [ "$probe_forms" = bare ] \
      || refuse "require-present probes the bare tag only, not '${probe_forms}'"
    docker_result="${DOCKER_RESULT:-}"
    [ "$docker_result" = success ] \
      || refuse "the image job's result is '${docker_result:-unset}', not 'success'; a chart never publishes ahead of its image"
    ;;
  *) refuse "mode '${mode}' is neither 'require-absent' nor 'require-present'" ;;
esac

base="${PUBLISH_GUARD_REGISTRY:-https://ghcr.io}"
# Header bytes travel on stdin, including into helpers. Never put either
# credential in argv; reject token controls before constructing a header.
: "${GH_USER:?GH_USER required}" "${GH_TOKEN:?GH_TOKEN required}"
bearer="$(python3 -c 'import base64,os,sys; sys.stdout.write("Authorization: Basic " + base64.b64encode((os.environ["GH_USER"]+":"+os.environ["GH_TOKEN"]).encode()).decode()+"\n")' \
  | curl -fsS --max-time 30 --header @- \
    "${base}/token?service=ghcr.io&scope=repository:${repo}:pull" \
  | python3 -c 'import json,sys
try:
    token = json.load(sys.stdin)["token"]
    if not isinstance(token, str) or not token or any(ord(c) <= 32 or ord(c) >= 127 for c in token):
        raise ValueError("invalid token")
    sys.stdout.write(token)
except (ValueError, KeyError, TypeError):
    sys.exit("invalid registry token response")')" \
  || refuse "registry state unknown (token exchange failed)"
probe() { # $1: the exact tag form the push will write
  local code
  code="$(printf 'Authorization: Bearer %s\n' "$bearer" | curl -sS --max-time 30 -o /dev/null -w '%{http_code}' \
    --header @- -H 'Accept: application/vnd.oci.image.index.v1+json, application/vnd.oci.image.manifest.v1+json, application/vnd.docker.distribution.manifest.list.v2+json, application/vnd.docker.distribution.manifest.v2+json' \
    "${base}/v2/${repo}/manifests/$1")" \
    || refuse "registry state unknown (manifest query failed)"
  case "$mode:$code" in
    require-absent:404 | require-present:200) return 0 ;;
    require-absent:200) refuse "$1 is already published" ;;
    require-present:404) refuse "$1 is not published; the chart's image must exist first" ;;
    *) refuse "registry state unknown (HTTP ${code:-none})" ;;
  esac
}

case "$probe_forms" in
  bare) probe "$version" ;;
  both) probe "$version"; probe "v${version}" ;;
esac
exit 0
