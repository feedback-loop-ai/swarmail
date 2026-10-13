# 0019 — Release-tag integrity: the version publishes only from its own ref, and never twice

- **Status:** proposed (awaiting operator acceptance)
- **Date:** 2026-10-13
- **Supersedes:** none
- **Amends:** 0018 §6 in place (the `v*`-prefix posture becomes an exact-ref bind
  guarded by a registry probe) and rescinds 0018 §Refused's registry-probe item;
  the 0018 edits are a landing task of the closing run

## Context

The security review of the post-P1 closure
(`close-the-remaining-post-p1-find-4a39f25f`, REVIEW-RESIDUAL-SECURITY) left one
shippable residual: the release image is tagged with the version read from
`Cargo.toml` and the chart is pushed with whatever `Chart.yaml` holds, and
neither publish path checks that the triggering `v*` ref matches that version —
so any `v*` tag (a typo such as `v0.1.1` before the bump, a `v0.1.0-rc` on a
later commit, or a deleted and re-pushed tag) rebuilds and silently overwrites
the ghcr `:<version>` image and the chart of that version. GHCR does not make
tags immutable by default. The workflow comments claimed the version publishes
"only from a `v<version>` ref" and that the OCI chart is "immutable"; neither
was enforced. 0018 §Refused refused exactly the mechanism this finding
requires — the registry probe — on two grounds (a race, and registry
credentials in a job a workflow `if` enforces) that the finding itself
falsifies.

## Decision

1. **Each publish path binds its own manifest's declared version to the
   triggering ref, by full-string equality.** The docker release push refuses
   unless `GITHUB_REF_NAME` equals `v` + the `Cargo.toml` version; the chart
   publish refuses unless `GITHUB_REF_NAME` equals `v` + the `Chart.yaml`
   version. Not `startsWith`, not a glob: `v0.1.0-rc` refuses for `0.1.0`, and
   correctly publishes `:0.1.0-rc` when the manifest says `0.1.0-rc`. The
   job-level `if` (unchanged) pins the ref *shape* — `refs/tags/v*` — so a
   branch named `v0.1.0` never reaches the guard; the guard pins the exact
   *name*. The extracted version is validated once, inside the guard (strict
   semver, named refusal): a malformed manifest refuses, it never publishes.
   No file-derived version travels through `${{ }}` into a `run:` block —
   versions ride env vars, so manifest bytes can never become shell source.
2. **The registry is the only ledger, and the probe fails closed.** Before any
   push, the publish path asks the registry whether the version already exists
   — the OCI distribution protocol over HTTP, classified by status code:
   `404` proceeds, `200` refuses named ("already published"), anything else —
   5xx, refused connection, failed token exchange — refuses named ("registry
   state unknown"). There is no skip-on-exists and no boolean probe: a check
   that treats "cannot determine" as "absent" is the finding again. Status
   codes are protocol, not upstream error prose, so the classification is
   never text-grepping. Each read is bounded (`curl --max-time`); there is no
   retry loop — a retry with backoff is a sleep wearing a timeout. The probe
   uses the credential the push itself already requires (`packages: write`);
   no new permission, no new secret surface. The registry is the authority on
   what was published: the query answers against *history* — versions
   published before this mechanism, manual pushes, and re-runs of old release
   runs (which now go red, never quietly re-confirm).
3. **The enforcement is one script the workflow calls, and its proof rides a
   gate publication already depends on.** `scripts/publish-guard.sh` is the
   only implementation (no inline YAML copy to drift); the docker and chart
   jobs are its two call sites. Its hermetic test,
   `scripts/publish-guard-test.sh` — real local HTTP server at the poles
   (200/404/500, connection refused; exact/mismatched/unset ref; non-semver;
   the probed repo path), no sleeps, injected inputs — runs in the CI
   `quality` job, which both publish jobs `needs:` (the 0018 §6 wiring,
   preserved verbatim). A publish step may not execute guard logic that is
   red. The test also pins the wiring: the workflow must reference the guard
   in both publish paths, or the test is red.
4. **The comments state exactly what is enforced, and one line of what is
   not.** The "only from a `v<version>` ref" claim becomes the truth (exact
   ref bind plus probe); the OCI-chart "immutable" sentence is deleted — GHCR
   enforces no such thing. Each publish comment carries a not-enforced line:
   `:main`/`:sha-<sha>` stay re-publishable by design; a concurrent publish
   can still race the probe; re-publishing a version means deleting the
   registry tag (a deliberate registry action — the recovery, not an
   oversight); the guard defends against accident and drift, not against an
   actor who can already push tags.

## Consequences

- A typo'd `v0.1.1`, an `rc` tag containing the version, a re-pushed stale
  tag, and a re-run of an old release all go red before any push. The refusal
  names the arm; nothing is ever silently replaced.
- A registry read outage blocks releases (the probe refuses "unknown").
  Availability is spent before integrity, the same direction as refusing an
  over-cap `timeout_ms` instead of clamping it.
- A partially green release (image published, chart refused, or vice versa)
  leaves a red run until the operator deletes the registry tag or re-tags —
  loud, deliberate, never a skip.
- The probe's check-then-push window remains open by seconds: two concurrent
  publishes of one version can both see `404`, and GHCR is last-writer-wins.
  No in-repo mechanism closes it (GHCR has no immutable tags); the comment
  says exactly this instead of claiming more than the mechanism enforces.
- `:main` and `:sha-<sha>` are untouched: a rolling tag and a commit-pinned
  tag are re-publishable by design.

## Refused

- **A claim-before-push git-ref ledger** (`refs/tags/published/…`) and the
  `contents: write` it costs: the framing makes the registry query the
  preference and the structural guard the fallback *when the runner cannot
  query* — this runner can (it is logged in to push). The claim's own crash
  mode (claim written, push failed — the version burns until an operator
  deletes an audited ref) and its third state (record written, push failed)
  buy closure of a window measured in seconds, against history the probe
  already answers. Named cost of refusing: the concurrent-publish race above.
- **A post-push digest verification** re-fetching the manifest by tag: the
  right instinct (0001's shadow — green means queryable), but 0001 is a store
  contract and extending it to the registry is a semantic change owed its own
  decision; the finding's threat (overwrite) is refused upstream of any
  digest. Named cost: a push landing wrong bytes is not caught by CI.
- **A `concurrency:` group serializing releases**: a new CI policy (queues,
  cancels) beyond tag integrity; the guard-step-before-push is the tightest
  binding this finding buys.
- **A four-way version-equality assert** (Cargo, Chart version, appVersion,
  the artifacthub annotation) in the guard: the two binds make cross-manifest
  drift loud at the same push (a bump without a chart bump refuses the chart),
  and chart-verify's rendered-tag assert already owns the chart-internal
  coherence. A guard that re-derives every copy of the version owns none of
  them.
- **Error-text grepping** of `buildx`/`helm`/curl output to distinguish
  "absent" from "error": brittle to upstream wording; status codes are the
  contract. The same reason no `docker buildx imagetools inspect` boolean
  probe is used: it cannot distinguish 404 from 500.
- **An override knob** ("force republish"): the recovery is deleting the
  registry tag — a deliberate registry action, which is the correct shape for
  the one legitimate re-publish. A knob re-opens the finding through its own
  front door.
- **Any production Rust line, any movement of the coverage floor, any edit to
  `bundles/self/`**: the slice is CI-only; the denominator is untouched.
