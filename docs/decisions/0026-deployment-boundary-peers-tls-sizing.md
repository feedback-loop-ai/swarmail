# 0026 — The deployment boundary admits peers, mounts TLS, and sizes its own restore

- **Status:** proposed (awaiting operator acceptance)
- **Date:** 2026-10-14
- **Supersedes:** none
- **Amends:** 0008 §6 in place (the probe contract gains a startup probe
  beside liveness and readiness) and extends 0008 §1's chart with three
  deployment-shape values; 0008 §2's "no configuration surface" is
  untouched — peers, volumes and probes are deployment shape, and the TLS
  env entries are the binary's own `SWARMAIL_*` contract rendered verbatim.

## Context

The review of issue #58 (#53) found the chart's deployment boundary lying
to its operators in three ways. The NetworkPolicy's ingress admits only
same-namespace pods (`podSelector: {}`) — an external ingress controller
and Prometheus are black-holed, and the second victim is the worse one:
the policy silently kills the scrape, so the ledger counters (`dropped`,
`exhausted`, `lagged`, `aborted`) become unreadable exactly when a hung
sink makes them matter. The failure mode is *silence*, not an error. The
STARTTLS posture is unusable — the chart cannot mount a TLS Secret (no
`extraVolumes`/`extraVolumeMounts`), so a documented env contract
(`SWARMAIL_TLS_CERT`/`SWARMAIL_TLS_KEY`) cannot be fed. And the restore
window is unsized: a full restore of the default 1 Gi PVC happens before
listeners bind (the correct posture — accept == stored on the first
bind), while liveness fires at ~32 s with a 256 Mi limit and no
startupProbe — an OOM or restart mid-restore becomes a crash loop that
re-restores forever and never binds, and the doc is read only after the
first crash.

## Decision

1. **Ingress peers are configurable and locked by default.** A
   `networkPolicy.ingressPeers` values list (entries rendered verbatim
   under `from:` — `namespaceSelector`, `podSelector`, `ipBlock`) extends
   the single rendered rule. The empty default renders exactly today's
   `from: [- podSelector: {}]` — default-off stays default-off; an
   operator who never sets the list inherits today's posture, byte for
   byte. `scripts/chart-verify.sh` asserts both: the locked default in the
   default profile, and that configured peers render into `from:` and are
   admitted in the full profile. No Egress section and no per-port peer
   matrices are added — `policyTypes` stays Ingress-only (nothing
   demanded them; an unrequested egress policy is a knob pretending to be
   a feature).
2. **TLS is mountable, never minted.** `tls.enabled`/`tls.secretName`
   mount the operator-supplied Secret and set `SWARMAIL_TLS_CERT`/
   `SWARMAIL_TLS_KEY` to the mounted file paths — the binary's own env
   names. With the values unset the chart renders no volume, no mount and
   no TLS env, and the plaintext listener is byte-identical. The chart
   mints nothing and never bakes cert material or validity into a
   fixture; `chart-verify.sh` asserts the *wiring* (volume, mount, env
   path), never cert lifetime (0020 rule 5).
3. **The restore window is sized, and the memory relation is documented
   rather than invented.** The pod gains a startupProbe on `/healthz`
   (the same endpoint and port as the other probes, per 0008 §6) whose
   budget — its own `periodSeconds` × `failureThreshold` — is documented
   against a full restore of the default PVC. Beside `resources`, the
   values document the relation instead of pretending a default: the
   store holds the restored state in memory, so the limit must cover it,
   and `SWARMAIL_MAX_PER_INBOX` is the knob that bounds it. An operator
   with a 1 Gi PVC cannot be handed a 256 Mi default and a shrug; they
   are handed the relation. No conditional resource-override block — a
   silent surprise is better documented than coded. Restore-before-bind
   is unchanged: it is the deployment edge of 0001, not a knob.

## Consequences

- A Prometheus outside the namespace can scrape again — but only because
  the operator wrote the peer that admits it, and the chart-verify
  assertion proves the rendered policy admits what the values claim.
  A black-holed scrape is now a *visible* misconfiguration, not silence.
- STARTTLS becomes deployable as documented; the unset path is provably
  the old chart.
- A restore that takes longer than the documented budget still crash-
  loops — the budget is a value an operator sizes to their PVC, and the
  relation tells them how. What is refused is the *undocumented* crash
  loop, not every possible one.
- The chart's rendered surface grows by three values blocks; installs
  that set none of them get today's pod with one addition — the startup
  probe, which only ever *extends* the window in which a slow restore is
  tolerated.

## Refused

- **An invented memory default** ("bump the limit until 1 Gi fits"): the
  restored state is operator data; any constant that claims to fit it is
  a guess wearing a default. The relation is the honest artifact.
- **A conditional resources-override block**: a value that silently
  rewrites itself under other values is a surprise an operator meets in
  production, not in a diff.
- **An Egress section / per-port peer matrices**: nothing demanded them;
  they would add policy surface without a named failure mode to answer.
- **A readiness-gate on restore completion** (refuse traffic until
  restore): already the posture — restore precedes bind; the startup
  probe sizes the *window*, it does not change the ordering.
- **Documentation only**: a documented crash loop is still a crash loop,
  and the doc is read after the first one. The probe and the relation are
  required *in addition* to the docs, never instead.
