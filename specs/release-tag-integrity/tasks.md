# Tasks — release-tag integrity

Ordered. Every task names the test that proves it; no task is done until its
proving test runs green against the change and the refusal arms demonstrably
fire. No task opens `src/`, `tests/`, `Cargo.toml`, `Cargo.lock`,
`deny.toml`, or `bundles/self/` — the slice is CI-only and the coverage
denominator is untouched.

- [ ] Write `scripts/publish-guard.sh`: the bind (`GITHUB_REF_NAME` full-string
  equality against the expected `v<semver>` ref, unset refuses), the
  once-only strict-semver validation of the expected version, and the
  fail-closed OCI probe (token exchange + manifest GET, bounded `--max-time`;
  `404` → proceed, `200` → refuse "already published", anything else → refuse
  "registry state unknown"; classification on the printed status code, never
  on curl's exit code). The workflow's comment block states this contract
  verbatim. *Proving test*: `scripts/publish-guard-test.sh` cases 1–7 —
  every branch the script has is executed by the test, including the unset-ref
  and non-semver refusals.
- [ ] Write `scripts/publish-guard-test.sh`: a real local python3 HTTP server
  scripted via a per-request control file (200 / 404 / 500 poles plus a
  closed-port refused pole), `PUBLISH_GUARD_REGISTRY` pointing the guard at
  it, the recorded-path assertion that the guard probed the repo its caller
  asked for, and the drift pin (`grep -c 'scripts/publish-guard.sh'
  .github/workflows/ci.yml` ≥ 2). Hermetic — localhost only, no sleeps, no
  wall-clock assertions. *Proving test*: the script itself, green end to end;
  plus a one-off review-time mutation spot-check (disable any one refusal arm
  → the test goes red), demonstrated in the run's evidence, not committed.
- [ ] Wire the docker job: the guard step (`if: startsWith(github.ref,
  'refs/tags/v')`) between the login step and the release push step, with
  `VERSION: ${{ steps.ver.outputs.version }}`, `GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}`,
  `GH_USER: ${{ github.actor }}` in `env:` and
  `run: scripts/publish-guard.sh "v$VERSION" feedback-loop-ai/swarmail` — the
  push step itself untouched, so a guard refusal fails the job before any
  push. *Proving test*: the workflow inspection transcript (the step's
  position, its `if:`, the env indirection — no file-derived version inside a
  `${{ }}` in a `run:` block) recorded in the run's evidence.
- [ ] Wire the chart job: a `chartver` extraction step mirroring the
  `steps.ver` sed idiom (`sed -n 's/^version: \(.*\)/\1/p'
  charts/swarmail/Chart.yaml | head -1`), then the guard step before the
  publish step with `run: scripts/publish-guard.sh "v$VERSION"
  feedback-loop-ai/charts/swarmail`; the publish step itself untouched. A
  malformed Chart.yaml version refuses (non-semver → guard refusal). *Proving
  test*: the same inspection transcript (position, `if:`, env indirection,
  the extraction mirroring the house idiom) recorded in the run's evidence.
- [ ] Rewrite the two publish-path comment blocks to state exactly the
  enforcement — the exact-ref bind, the fail-closed probe, pre-push-not-
  immutability, the race, the delete-the-tag recovery — and delete the
  "immutable" claim; each block carries one explicit NOT-enforced line
  (`:main`/`:sha-<sha>` re-publishable by design; the concurrent-publish
  race; the actor who can already push tags). *Proving test*: the inspection
  transcript quotes both blocks; a grep asserts the tree contains no
  "immutable" claim about the chart path and each block contains its
  not-enforced line.
- [ ] Wire the proof into the gate: `run: bash scripts/publish-guard-test.sh`
  added to the CI `quality` job (the job both publish jobs `needs:`), with
  the job's display `name:` extended to `format, clippy, publish guard` — job
  id and both publish `needs:` lists byte-identical to HEAD (the 0018 §6
  wiring of commit 6ed8e3e, preserved). *Proving test*: the inspection
  transcript (the test line inside `quality`; a diff shows the `needs:` lists
  unchanged) plus the guard test green in a CI run of the closing PR.
- [ ] Land decision 0019's dependents: edit `docs/decisions/0018-*.md` §6 in
  place (the exact-ref bind and the probe replace the `v*`-prefix posture) and
  replace the §Refused registry-probe item with the rescindal pointer to 0019;
  add the 0019 row to `docs/decisions/README.md`. *Proving test*: the
  deterministic design check — 0019 exists with status `proposed` and is
  referenced from the spec — plus a read of 0018 showing no remaining refusal
  of the shipped mechanism.
- [ ] Run the full gate set and record the evidence: `bash
  scripts/publish-guard-test.sh` (the scriptable refusal proof the acceptance
  prefers), `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo test`, `bash scripts/coverage-gate.sh` (report unchanged —
  no production Rust line moved, so the floor cannot fall), `cargo deny check
  licenses`, `brokkr compile --bundle bundles/self`; `git diff --stat`
  confirming zero changes under `src/`, `tests/`, `Cargo.toml`, `Cargo.lock`,
  `deny.toml`, `bundles/self/`. *Proving test*: the gate outputs and the
  zero-diff stat, attached to the run's evidence as the inspection transcript
  of record.
