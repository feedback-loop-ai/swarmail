#!/usr/bin/env bash
# publish-guard.sh — the release publish path's only enforcement (decision
# 0019). Refuses unless the triggering ref is exactly the expected one AND
# the registry probe reports the version absent (404). 200 refuses: already
# published. Anything else — 5xx, refused connection, failed token
# exchange — refuses: registry state unknown. This is a pre-push check,
# NOT registry-side immutability; a concurrent publish can still race the
# probe, and re-publishing a version means deleting the registry tag.
#   usage: publish-guard.sh <v<semver>> <oci-repo>
#   env:   GITHUB_REF_NAME (the triggering ref; unset refuses)
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
echo "$version" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$' \
  || refuse "'${version}' is not valid semver"

base="${PUBLISH_GUARD_REGISTRY:-https://ghcr.io}"
bearer="$(curl -fsS --max-time 30 -u "${GH_USER:?GH_USER required}:${GH_TOKEN:?GH_TOKEN required}" \
  "${base}/token?service=ghcr.io&scope=repository:${repo}:pull" \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["token"])')" \
  || refuse "registry state unknown (token exchange failed)"
code="$(curl -sS --max-time 30 -o /dev/null -w '%{http_code}' \
  -H "Authorization: Bearer ${bearer}" -H 'Accept: application/vnd.oci.image.index.v1+json, application/vnd.oci.image.manifest.v1+json, application/vnd.docker.distribution.manifest.list.v2+json, application/vnd.docker.distribution.manifest.v2+json' \
  "${base}/v2/${repo}/manifests/${version}")" \
  || refuse "registry state unknown (manifest query failed)"
case "$code" in
  404) exit 0 ;;
  200) refuse "v${version} is already published" ;;
  000|*) refuse "registry state unknown (HTTP ${code:-none})" ;;
esac
