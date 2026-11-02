# Kubernetes (Helm)

[← README](../../README.md) · [Guide index](README.md)

A shared namespace mailbox, an ephemeral per-run instance, an agent-sidecar —
the chart packages the same binary with no configuration language of its own
([decision 0008](../decisions/0008-kubernetes-deployment-contract.md),
*accepted*). Install the published chart:

```
helm install swarmail oci://ghcr.io/feedback-loop-ai/charts/swarmail --version 0.2.0 \
  --namespace swarmail --create-namespace
helm test swarmail -n swarmail        # the /healthz hook, from a curl pod
```

It runs the `:0.2.0` image, published by the `v0.2.0` release run on
2026-11-02 together with this chart. To track `main` instead, install from a
checkout:

```
git clone https://github.com/feedback-loop-ai/swarmail && cd swarmail
helm install swarmail ./charts/swarmail --set image.tag=main --set image.pullPolicy=Always \
  --namespace swarmail --create-namespace
helm test swarmail -n swarmail        # the /healthz hook, from a curl pod
```

The published charts are OCI `0.1.0`, which predates the startupProbe, the
TLS settings, `networkPolicy.ingressPeers` and the listen configuration, and
`0.2.0`; `0.1.1`, `0.1.2` and `0.1.3` were prepared and never published, and
the publish guard never republishes a version (decision 0019). Chart `0.2.0`
carries `appVersion` `0.2.0`, aligned with the crate (decision 0051). The
checkout install sets `image.tag=main` explicitly — and
`image.pullPolicy=Always`, because `:main` moves: under the chart's default
`IfNotPresent` a node that cached an older `:main` would keep running it.

**Releases publish the image before the chart.** A `v*` tag publishes only
when it is exactly `v<Cargo.toml version>` for the image and exactly
`v<Chart.yaml version>` for the chart, and the chart's `appVersion` must
equal Cargo's version; the chart job needs the image job's success and
checks that the image it names is published. So a tag that matches only
the chart — `v0.1.2` on the tree before this release, whose crate was
`0.1.0` — publishes neither. Preparing a chart version is not a release: CI requires a chart
version bump for every chart edit (`scripts/chart-verify.sh`), and the
release itself is the operator's aligned tag. The two pushes are not one
transaction: if the chart push fails after the image pushed, the image
stays published and a rerun refuses it as a republish (decision 0019) —
recovery is a deliberate registry act by the operator.

The chart adds nothing the binary does not read — configuration is exactly
the env contract, passed through `config: {SWARMAIL_MAX_PER_INBOX: "0"}`
or `envExtra`. One Service carries all three ports, so one DNS name serves
every client; the optional Ingress exposes only the HTTP surface (SMTP and
POP3 are plain TCP). The contract worth knowing before you scale it — you
cannot: the chart refuses a `replicaCount` above 1 at render time, naming
the contract ([0008 §4](../decisions/0008-kubernetes-deployment-contract.md)):

| Value (default) | What it changes |
|---|---|
| `replicaCount` (`1`) | must stay 1 — a second pod would fork the mailboxes or mount the same RWO claim twice; anything above 1 fails `helm template`. The strategy is **Recreate at every setting** ([0008 §5](../decisions/0008-kubernetes-deployment-contract.md)): a rolling surge would run two pods behind one Service, and the store is single-replica by design ([0001](../decisions/0001-synchronous-insert-lossless.md)) |
| `image.repository` / `image.tag` / `image.pullPolicy` (`ghcr.io/feedback-loop-ai/swarmail` / `""` = the chart's `appVersion` / `IfNotPresent`) | the image; set `pullPolicy: Always` with a moving tag such as `main` |
| `config` (the three binds, `0.0.0.0:1025` / `0.0.0.0:1110` / `0.0.0.0:8025`) | `SWARMAIL_*` env entries, rendered as given — the binds are stated because the binary's own defaults are loopback ([0030 §1](../decisions/0030-loopback-plane-bounded-results.md)); add any other knob, e.g. `SWARMAIL_MAX_PER_INBOX` (Helm merges it beside the binds) |
| `envExtra` (`[]`) | raw env entries passed through verbatim (`valueFrom` and friends) |
| `service.type` / `service.ports` (`ClusterIP`, `1025`/`1110`/`8025`) | the Service; the ports are the Service side, the container always listens on 1025/1110/8025 |
| `persistence.enabled` (`false`) | SQLite on a PVC mounted at `/data` (`SWARMAIL_DATA_FILE=/data/swarmail.db`); `existingClaim` (`""`), `storageClass` (`""`), `accessModes` (`[ReadWriteOnce]`), `size` (`1Gi`) |
| `startupProbe.periodSeconds` / `.timeoutSeconds` / `.failureThreshold` (`10` / `2` / `90`) | the restore window before the kubelet restarts the pod — period × threshold, 15 minutes by default, because the data file restores fully before the first bind ([0026 §3](../decisions/0026-deployment-boundary-peers-tls-sizing.md)) |
| `probes.initialDelaySeconds` / `.periodSeconds` / `.timeoutSeconds` / `.failureThreshold` (`2` / `10` / `2` / `3`) | the liveness and readiness probes on `/healthz` |
| `tls.enabled` / `tls.secretName` (`false` / `""`) | mounts an operator-supplied Secret's `tls.crt`/`tls.key` read-only at `/etc/swarmail/tls` and sets `SWARMAIL_TLS_CERT`/`SWARMAIL_TLS_KEY`, so SMTP offers STARTTLS; the chart mints nothing, and `enabled` without a `secretName` fails `helm template` by name ([0026 §2](../decisions/0026-deployment-boundary-peers-tls-sizing.md)) |
| `ingress.enabled` (`false`) | the UI/API/MCP surface on your ingress class (`className`, `host` `swarmail.local`, `tls`, `annotations`); SMTP/POP3 stay cluster-internal |
| `serviceMonitor.enabled` (`false`) | a prometheus-operator ServiceMonitor on `/metrics` (`interval` `30s`, `labels`) — **fails closed with a named refusal** if the CRDs are absent |
| `networkPolicy.enabled` / `networkPolicy.ingressPeers` (`false` / `[]`) | ingress on the three ports from the same namespace only; `ingressPeers` adds `from:` peers (`namespaceSelector`, `podSelector`, `ipBlock`) verbatim to that one rule ([0026 §1](../decisions/0026-deployment-boundary-peers-tls-sizing.md)); the policy is Ingress-only |
| `resources` (requests `50m` CPU / `32Mi`, limit `256Mi` memory) | size `limits.memory` from observed use: memory is not globally bounded (0038) |
| `tests.enabled` (`true`) | the `helm test` `/healthz` hook pod |
| `podAnnotations`, `podLabels`, `imagePullSecrets`, `nodeSelector`, `tolerations`, `affinity`, `nameOverride`, `fullnameOverride` | the usual scheduling and naming passthroughs (all empty by default) |

`helm template` output is valid plain manifests — Kustomize and raw-YAML
teams are first-class, not gated behind the chart. The chart is verified
offline by `scripts/chart-verify.sh` (lint + both profiles + contract
assertions), which the CI `chart-verify` job runs on every push and PR,
together with its chart-version bump check; publishing is the separate,
tag-only `chart-publish` job.
