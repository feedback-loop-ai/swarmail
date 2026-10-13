# Release-tag integrity: the version publishes only from its own ref, and never twice

Feature: the release-tag integrity residual recorded by the review of
`close-the-remaining-post-p1-find-4a39f25f` (REVIEW-RESIDUAL-SECURITY; the
finding is in `.forge/results/fe1642a3-baa4-41dd-9f7c-02e5f802712f-security.json`).
Decision: 0019 — [release-tag integrity](../../docs/decisions/0019-release-tag-integrity.md)
(status *proposed*, awaiting the operator), which amends 0018 §6 in place and
rescinds 0018 §Refused's registry-probe item.

The finding, in one sentence: the docker job tags `ghcr.io/…/swarmail:<version>`
with the version read from `Cargo.toml` and the chart job pushes whatever
`Chart.yaml` holds, and neither checks that the triggering `v*` ref matches
that version — so any `v*` tag (a typo such as `v0.1.1` before the bump, a
`v0.1.0-rc` on a later commit, or a deleted and re-pushed tag) rebuilds and
silently overwrites the `:<version>` image and chart, while two workflow
comments claim enforcement that does not exist. GHCR makes no tags immutable.
The slice is **CI-only**: no production Rust line may change, so the coverage
floor cannot fall, and `bundles/self/` is untouched.

Both positions converge on the load-bearing idea — the enforcement must be an
executable predicate, not a comment — and on exact-string ref equality. They
genuinely conflict on the probe's failure semantics and on three robustness
mechanisms; each ruling names the winner, the loser's cost, and the proof.

## Rulings

**R1 — The bind: exact string equality, per publish path, validated once**
*(both positions converge; robustness's adversarial case list is adopted as
the test matrix; robustness's four-way equality assert loses)*.

The docker release push step runs the guard with `v` + the `Cargo.toml`
version; the chart publish step runs it with `v` + the `Chart.yaml` version;
the guard refuses unless `GITHUB_REF_NAME` equals that expected ref as a full
string. The job-level `if` (unchanged from 0018 §6) pins the ref *shape* —
`refs/tags/v*` — so a branch named `v0.1.0` never reaches the guard; the guard
pins the exact *name*. Composed, that is robustness's full-ref bind without a
second parser. Not `startsWith`, not a glob: `v0.1.0-rc` refuses for version
`0.1.0`, and correctly publishes `:0.1.0-rc` when the manifest says
`0.1.0-rc`; a typo'd `v0.1.1` before the bump refuses; nothing publishes a
version no manifest declares. The extracted version is validated once, inside
the guard (strict semver, named refusal): a malformed manifest refuses, it
never publishes something else. No file-derived version travels through
`${{ }}` into a `run:` block — versions ride `env:` (robustness's discipline,
adopted; it costs nothing and closes the one injection-shaped input this path
has). The four-way equality assert (Cargo + Chart `version` + `appVersion` +
the artifacthub annotation) is **refused**: the two binds make cross-manifest
drift loud at the same push (bump the crate without the chart and the chart
guard refuses — a red run), and chart-verify's rendered-tag assert already
owns chart-internal coherence; a guard that re-derives every copy of the
version owns none of them. Cost of refusing (named): chart-internal drift is
refused by chart-verify, not by the publish path — the right owner.

**R2 — The probe: fail-closed status classification, not a boolean**
*(robustness wins; simplicity's boolean probe loses; simplicity's no-text-grepping
survives the reconciliation)*.

Before any push, the publish path asks the registry whether the version
already exists, over the OCI distribution protocol, classified by status
code: `404` → absent → proceed; `200` → refuse, named "already published";
anything else (5xx, refused connection, failed token exchange) → refuse,
named "registry state unknown". **No skip-on-exists anywhere** — a re-run of
a green release is red, never quietly reconfirmed, and a stale re-pushed tag
refuses at the probe (the query answers against history: versions published
before this mechanism, manual pushes, old workflow re-runs). Two bounded HTTP
reads (token exchange, manifest GET), no retry loop — a retry with backoff is
a sleep wearing a timeout. The losing side is named: simplicity's probe
(`"$@" >/dev/null 2>&1 &&` refuse) treats a failed read as *absent*, so a
registry that fails reads while accepting writes silently overwrites — the
exact finding, and a violation of simplicity's own fail-closed anti-cut rule.
The reconciliation keeps simplicity's correct refusal of error-text grepping:
status codes are stable protocol, not upstream prose. Cost of winning (named):
a registry read outage blocks releases — availability spent before integrity,
the house direction; and `curl` joins the tool set (preinstalled on runners,
no Cargo dependency, `deny.toml` untouched).

**R3 — The structural ledger, the digest verify, the concurrency group: all
refused** *(robustness's three extra mechanisms lose; each cost named)*.

- **Claim-before-push git refs** (`refs/tags/published/…` + `contents:
  write`): the framing makes the registry query the *preference* and the
  structural one-time-publication guard the *fallback when the runner cannot
  query the registry*. This runner can — it is logged in to push. The claim's
  price: a privilege increase on the publish tokens, a burned version when a
  crash lands between claim and push, an audited-unlock ritual, and a third
  state (record written, push failed) to reconcile. Cost of refusing, named
  and honest: the check-then-push TOCTOU window stays open — two concurrent
  publishes of one version can both see `404`, and GHCR is last-writer-wins.
  That residual is stated in the workflow comments rather than hidden.
- **Post-push digest verification**: the right instinct (0001's shadow —
  green means queryable), but 0001 is a store contract; extending it to the
  registry is a semantic change owed its own decision, and this finding's
  threat (overwrite) is refused upstream of any digest comparison. Cost of
  refusing: a push landing wrong bytes (build-cache poisoning, a registry
  bug) is not caught by CI.
- **A `concurrency:` group serializing releases**: a new CI policy (queues,
  cancels) beyond tag integrity. Cost of refusing: the race window above is
  not narrowed by serialization.

**R4 — The proof rides the existing gate** *(simplicity wins the wiring;
robustness's dedicated job loses; both converge that an unproved guard is
the next untested comment)*.

`scripts/publish-guard-test.sh` — hermetic, poles only, per the house test
disciplines adapted to shell: a real local HTTP server speaking the real
two-endpoint OCI shape (token exchange + manifest GET) scripted to answer
200/404/500, a refused-connection pole, injected inputs, no sleeps, no
wall-clock assertions. The matrix: exact ref passes; mismatched ref refuses
naming both refs; unset `GITHUB_REF_NAME` refuses; non-semver expected
version refuses; probe 200 refuses "already published"; probe 500 refuses
"unknown"; connection refused refuses "unknown"; the guard probed the repo
path its caller asked for; the workflow still references the guard script in
both publish paths (the drift pin — an unwired guard is the finding's shape
again). Wiring: one `run:` line in the CI `quality` job, with the job's
display name extended to carry it honestly — the job **both publish jobs
already `needs:`**, so the guard's proof sits inside the gate chain
publication depends on and the 0018 §6 `needs:` lists are preserved
byte-identical. Cost of robustness's dedicated `release-guard` job (losing
side, named): a new job plus a `needs:` extension to host a thirty-line
hermetic test that needs no isolation from fmt/clippy; its one advantage —
attribution — is carried by the renamed job display instead.

**R5 — The comments state exactly the enforcement, plus one NOT-enforced
line** *(both positions converge; robustness's not-enforced line adopted)*.

The docker comment's "publishes only from a `v<version>` ref" becomes the
truth: exact-ref bind plus a fail-closed probe, pre-push, not registry-side
immutability. The chart comment's "an OCI chart of a given version is
immutable" sentence is **deleted, not softened** — GHCR enforces no such
thing; the workflow does what it does. Each comment gains one not-enforced
line: `:main`/`:sha-<sha>` remain re-publishable by design; a concurrent
publish can still race the probe; re-publishing a version means deleting the
registry tag (the deliberate recovery); the guard defends against accident
and drift, not against an actor who can already push tags — the finding's own
threat model.

**R6 — Decision 0019 drafts the amendment; 0018's edits land with the close**
*(both positions converge that the amendment ships with the change; the
mechanism is the house pattern)*.

0018 §Refused refuses the very mechanism this finding requires; shipping the
probe while that refusal stands hands review a contradiction between the
constitution and the tree. 0019 (proposed) states the enforcement contract
and rescinds the refusal with the reasons its two grounds no longer hold —
the credentials objection is moot (the probe uses the credential the push
itself requires; nothing new is dragged anywhere), and the race objection is
priced rather than pretended away. The in-place edits to 0018 §6 and its
Refused list are landing tasks of the closing run, exactly as 0018 amended
0008. Cost (named): between this commit and the landing, the tree carries a
proposed decision (0018) whose refusal is superseded by another proposed
decision (0019) — the operator resolves both at acceptance.

## Refused in this run (the anti-cut list)

- **chart-verify's hardcoded `":0.1.0"`** — the fifth copy of the version,
  which robustness would derive from `appVersion` in this run. Out of scope:
  the finding is tag integrity, not version coherence, and the framing closes
  exactly one residual. Cost of refusing, named: on the next lockstep bump
  the assert goes stale and red on every push until edited — loud, never
  silent; the release recipe edits it alongside the four manifests.
- From simplicity's standing list, adopted wholesale (each priced there): a
  repo write-back ledger, release-notes-existence guards, a new release
  workflow or reusable workflow (it would reopen the same-invocation gate
  chain), cosign/SLSA/digest pinning/oras/crane, an override knob,
  `values.schema.json` coherence asserts, any edit to `bundles/self/`
  (including the verify seat), **any Rust line** (the coverage denominator is
  untouched, so the exact-100% floor cannot fall), a second guard script or
  per-job duplicate, and grepping probe error text.
- From robustness's own list, adopted: no `contains`/`endsWith`/`startsWith`
  bind, no skip-on-exists, no `${{ }}` interpolation of file-derived versions
  into `run:`, no guard logic in YAML expressions (it lives in the script the
  test executes), and no reopening of the persistence boundary, the Run 2
  closures, or any other finding.

## Acceptance Criteria

1. `scripts/publish-guard.sh` refuses (exit 1, reason on stderr) when
   `GITHUB_REF_NAME` is unset, when it differs from the expected ref as a
   full string (including `v0.1.1` at version `0.1.0` and `v0.1.0-rc` at
   version `0.1.0`), and when the expected version is not strict semver; it
   proceeds (exit 0) only on an exact match.
2. The guard probes the registry for the version and: HTTP `404` → proceed;
   HTTP `200` → refuse "already published"; any other status or a failed
   token exchange or transport → refuse "registry state unknown". No arm of
   the guard skips a push.
3. `scripts/publish-guard-test.sh` proves AC 1–2 at the poles against a real
   local HTTP server (200 / 404 / 500 / connection refused; exact,
   mismatched, unset ref; non-semver; the probed repo path) and pins the
   workflow to the guard script; it is hermetic (localhost only), has no
   sleeps, and runs green.
4. The docker release push step (v* tag refs only) invokes the guard with
   `v<Cargo.toml version>` before pushing; the chart publish step invokes it
   with `v<Chart.yaml version>` before pushing; a guard refusal fails the job
   before any push step executes.
5. No file-derived version travels through `${{ }}` into a `run:` block in
   the two guard steps: the ref is read by the guard from `GITHUB_REF_NAME`,
   the expected version arrives via `env:`.
6. The two publish-path comment blocks state exactly the enforced mechanism
   and each carries one explicit not-enforced line; the "immutable" claim is
   gone from the tree.
7. Both publish jobs' `needs:` lists are byte-identical to HEAD — the 0018 §6
   gate wiring (commit 6ed8e3e) is preserved, untouched.
8. `git diff` shows zero changes under `src/`, `tests/`, `Cargo.toml`,
   `Cargo.lock`, `deny.toml`, `bundles/self/`, and `scripts/coverage-gate.sh`;
   `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
   `cargo test`, `bash scripts/coverage-gate.sh`, `cargo deny check licenses`
   and `brokkr compile --bundle bundles/self` all stay green.
9. `docs/decisions/0019-release-tag-integrity.md` exists with status
   `proposed`, is referenced from this spec, and its landing tasks (the 0018
   in-place edits, the decisions-index row) are written into `tasks.md`.

## Verify gate

The guard test is the scriptable check the acceptance prefers: it executes
the same script the workflow calls, at the poles, in a job publication
`needs:`. The wiring itself (the two call sites, the comment truth, the
preserved `needs:`) is declarative and is proven by an inspection transcript
committed to the run's evidence — `grep`-able facts about
`.github/workflows/ci.yml`, not prose claims. The Rust gates are expected
unchanged-green (no production line moves), and `brokkr compile --bundle
bundles/self` stays clean because the bundle is untouched.
