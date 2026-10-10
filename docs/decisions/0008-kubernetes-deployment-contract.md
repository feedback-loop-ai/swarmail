# 0008 — The Kubernetes deployment contract

- **Status:** proposed (awaiting operator acceptance)
- **Date:** 2026-10-10
- **Supersedes:** none

## Context

Swarmail ships as one static binary and a scratch container. Teams want it
in a cluster: a shared namespace mailbox for integration suites, an
ephemeral per-run instance in CI, an agent-sidecar. The tool's own shape
already fits the platform's model — every knob is an env var, `/healthz`
and `/metrics` exist, and the state model is explicit (memory by default,
one SQLite file when asked). What it needs is a packaging that carries that
shape into Kubernetes without inventing a configuration language the binary
does not have.

## Decision

1. **Helm is the packaging** — the chart lives in `charts/swarmail`, is
   verified offline by `scripts/chart-verify.sh` (lint + render both
   profiles + assert the deployment contract; helm is the only tool), and
   is published as an OCI artifact to
   `oci://ghcr.io/feedback-loop-ai/charts/swarmail`, the same registry as
   the image.
2. **The chart adds no configuration surface.** Configuration is exactly
   the binary's env contract: the `config` map renders `SWARMAIL_*` keys
   verbatim, `envExtra` passes raw entries. No ConfigMaps, no rendered
   files, no values the server would not otherwise read.
3. **One Service, three ports** (`smtp` 1025, `pop3` 1110, `http` 8025) —
   one DNS name for every client. The Ingress — optional — exposes only the
   HTTP surface; SMTP and POP3 are plain TCP and are never ingressed.
4. **Single replica by design.** The store is one process's memory or one
   SQLite file; HA would need a shared database and a different product
   (see 0001). `replicas: 1` is the contract; scaling the Deployment would
   silently fork the mailboxes.
5. **Persistence switches the strategy to `Recreate`.** With
   `persistence.enabled`, the SQLite file lives on an `ReadWriteOnce`
   volume and `SWARMAIL_DATA_FILE=/data/swarmail.db` is set; the old pod
   must be gone before the new one mounts. Without persistence the store is
   memory and the strategy is `RollingUpdate`.
6. **Probes are `/healthz` on the `http` port**, for liveness and
   readiness alike — the endpoint is the binary's own "am I answering"
   contract, and a false readiness has no other source of truth.
7. **The pod runs hardened by default**: the image's own non-root user
   (65532), `fsGroup: 65532` so the data dir is writable, no privilege
   escalation, all capabilities dropped, RuntimeDefault seccomp,
   `readOnlyRootFilesystem` with an emptyDir for `/tmp`. The optional
   ServiceMonitor is CRD-guarded and **fails closed with a named refusal**
   when prometheus-operator is absent, never silently skips.

## Consequences

- Upgrades with persistence take one restart-length gap (Recreate), which
  is the honest price of a file-backed single-replica store; a rolling
  upgrade would lie about which mailbox answered.
- The chart cannot express multi-tenant mail routing — that is not a loss;
  per-test inboxes are the routing (0004), and the AUTH username remains
  the namespace inside the cluster exactly as outside it.
- `helm template` output is a valid plain-manifest deployment for teams
  that prefer Kustomize or raw YAML; the chart is not a gate in front of
  them.
