#!/usr/bin/env bash
# Verify the Helm chart without a cluster: lint, render both profiles, and
# assert the rendered contract. Exits non-zero on the first red. Used by CI
# (the chart job) and by the hermetic verify seat — helm is the only tool.
set -euo pipefail
cd "$(dirname "$0")/.."
CHART=charts/swarmail

command -v helm >/dev/null || { echo "helm is required" >&2; exit 1; }

echo "== helm lint"
helm lint "$CHART" >/dev/null

echo "== render: default profile"
helm template swarmail "$CHART" > /tmp/chart-default.yaml

echo "== render: full profile (persistence, ingress, monitor, policy, config)"
# --api-versions simulates the prometheus-operator CRDs being present; offline
# the ServiceMonitor guard fails closed with a named refusal instead.
helm template swarmail "$CHART" \
  --set persistence.enabled=true \
  --set ingress.enabled=true \
  --set serviceMonitor.enabled=true \
  --set networkPolicy.enabled=true \
  --set config.SWARMAIL_MAX_PER_INBOX=0 \
  --api-versions monitoring.coreos.com/v1 > /tmp/chart-full.yaml

echo "== render: ServiceMonitor guard fails closed without the CRD"
if helm template swarmail "$CHART" --set serviceMonitor.enabled=true >/dev/null 2>/tmp/chart-guard.err; then
  echo "FAIL: serviceMonitor.enabled rendered without the CRD present" >&2
  exit 1
fi
grep -q "prometheus-operator CRDs" /tmp/chart-guard.err

echo "== render: the replica cap refuses at render time (decision 0008 §4)"
if helm template swarmail "$CHART" --set replicaCount=2 >/dev/null 2>/tmp/chart-replica.err; then
  echo "FAIL: replicaCount=2 rendered — the cap must refuse at render time" >&2
  exit 1
fi
grep -q "replicaCount must be 1" /tmp/chart-replica.err

echo "== render: service annotations land in metadata (finding 4, issue #31)"
helm template swarmail "$CHART" --set service.annotations.example=probe > /tmp/chart-annot.yaml

python3 - <<'EOF'
import yaml, sys

def docs(path):
    return [d for d in yaml.safe_load_all(open(path)) if d]

def one(path, kind):
    m = [d for d in docs(path) if d["kind"] == kind]
    assert len(m) == 1, f"{path}: expected exactly one {kind}, got {len(m)}"
    return m[0]

d, f = "/tmp/chart-default.yaml", "/tmp/chart-full.yaml"

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
svc = one(d, "Service")
assert [(p["name"], p["port"]) for p in svc["spec"]["ports"]] == [("smtp", 1025), ("pop3", 1110), ("http", 8025)]

# The annotation probe render: service.annotations are Service metadata, not
# labels (finding 4, issue #31).
asvc = one("/tmp/chart-annot.yaml", "Service")
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
print("chart contract: all assertions green")
EOF

echo "chart-verify: OK"
