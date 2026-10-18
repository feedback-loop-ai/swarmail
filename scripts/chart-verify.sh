#!/usr/bin/env bash
# Verify the Helm chart without a cluster: lint, render both profiles, and
# assert the rendered contract. Exits non-zero on the first red. CI-only
# (the `chart-verify` job): the hermetic verify seat installs nothing and
# does not run it. Every render lands in one private `mktemp -d` scratch
# removed on exit, so two concurrent runs never share a path (issue #104).
set -euo pipefail
cd "$(dirname "$0")/.."
CHART=charts/swarmail

command -v helm >/dev/null || { echo "helm is required" >&2; exit 1; }

S="$(mktemp -d "${TMPDIR:-/tmp}/chart-verify.XXXXXX")"
trap 'rm -rf "$S"' EXIT
export CHART_SCRATCH="$S"

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
assert c["image"].endswith(":0.1.0")                        # tag defaults to appVersion
assert c["imagePullPolicy"] == "IfNotPresent"
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
