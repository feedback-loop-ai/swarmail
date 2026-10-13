# Plan — release-tag integrity

HOW for [spec.md](spec.md). The slice is CI-only: two scripts under
`scripts/`, one workflow edit, one decision draft (already committed by this
design phase). No Rust line moves — `src/`, `tests/`, `Cargo.toml`,
`Cargo.lock`, `deny.toml` and `bundles/self/` end at zero diff, so the
coverage denominator is untouched and the exact-100% floor cannot fall.

## Technical context

- `.github/workflows/ci.yml` is the only workflow. The docker job runs for
  `main` and `v*` tag refs; its `steps.ver` step already extracts the
  `Cargo.toml` version with the house sed idiom; the release push step tags
  `ghcr.io/feedback-loop-ai/swarmail:<version>` under a bare
  `startsWith(github.ref, 'refs/tags/v')`. The chart job lints/renders
  (`scripts/chart-verify.sh`) and, for `v*` refs, logs in and pushes the
  packaged chart to `oci://ghcr.io/feedback-loop-ai/charts`.
- GHCR speaks the OCI distribution protocol: a token exchange
  (`GET https://ghcr.io/token?service=ghcr.io&scope=repository:<repo>:pull`,
  Basic auth with `github.actor` + the per-run `GITHUB_TOKEN`) yields a
  bearer; `GET https://ghcr.io/v2/<repo>/manifests/<tag>` with that bearer
  and the OCI `Accept` media types answers the existence question *by status
  code* — `200` exists, `404` absent, anything else unknown. The chart is an
  OCI artifact at repo `feedback-loop-ai/charts/swarmail`, tagged with its
  `version:` — the same endpoint shape.
- Tools: `curl`, `python3` and `bash` are preinstalled on `ubuntu-latest` and
  already leaned on by `scripts/chart-verify.sh`. No new Cargo dependency;
  `deny.toml` untouched.

## Constitution check

- **Frozen guarantees (0001–0003)**: untouched — this slice cannot reach the
  store, the wait path, or pruning. The nearest analogue is honored *by
  inheritance*: 0001's "accept == stored" becomes the release's "refused ==
  not pushed" (the guard runs before any push step, and a refusal fails the
  job first).
- **0004 (real protocol, no mocks)**: the guard test speaks real HTTP against
  a real local server — the poles of the OCI contract, not a stub of the
  guard logic; the guard under test is the exact script the workflow calls.
- **0011 §5 disciplines, adapted to shell**: chaos at the poles only (200 /
  404 / 500 / connection-refused; exact / mismatched / unset ref), inputs
  injected (env vars + argv), no sleeps, no wall-clock validity assertions,
  no shared mutable state (each case gets a fresh fixture).
- **0018 §6**: the five-gate `needs:` wiring is preserved byte-identical; the
  slice only adds a guard step inside each publish path and a test line
  inside the `quality` job that wiring already depends on.
- **Dependencies**: none added. **Coverage**: no production Rust line, so the
  floor cannot fall; the guard's own branches are each executed by its test —
  the vacuity discipline applied voluntarily.
- **0018 §Refused conflict**: resolved by decision 0019 (proposed); the
  in-place 0018 edits are landing tasks, not silent contradiction.

## Project structure

| Path | Change |
|---|---|
| `scripts/publish-guard.sh` | **new** — the bind + probe predicate (~45 lines) |
| `scripts/publish-guard-test.sh` | **new** — the hermetic poles test (~90 lines) |
| `.github/workflows/ci.yml` | two guard steps (docker, chart), a chart-version extraction step, two comment rewrites, one test line + display rename in `quality` |
| `docs/decisions/0019-release-tag-integrity.md` | committed by the design phase (done) |
| `docs/decisions/0018-*.md`, `docs/decisions/README.md` | **landing tasks** (close run): §6 amend + Refused rescind in place; the 0019 index row |

## 1. `scripts/publish-guard.sh` — the only enforcement

One script, two call sites. Contract: `publish-guard.sh <expected-ref>
<oci-repo>`; exit 0 = publish may proceed, exit 1 = refuse with the named
reason on stderr. Fail closed on every degenerate input.

```bash
#!/usr/bin/env bash
# publish-guard.sh — the release publish path's only enforcement.
# Refuses unless the triggering ref is exactly the expected one AND the
# registry probe reports the version absent (404). 200 refuses: already
# published. Anything else — 5xx, refused connection, failed token
# exchange — refuses: registry state unknown. This is a pre-push check,
# NOT registry-side immutability; a concurrent publish can still race.
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
```

Classification is on the printed status code, never on curl's exit code
alone; a transport failure prints no code and lands in the `unknown` arm
(`000` is curl's no-response code, kept in the unknown arm for symmetry). The
semver check doubles as URL-path hygiene: a version that cannot be a version
cannot be a path segment either. One bounded attempt per request, no retry
loop.

## 2. The workflow edits

**Docker job** — one guard step between the login step and the release push
step (both already exist; the push's `with:` block is untouched):

```yaml
      - name: Guard the release tag (exact ref match; refuse a republish)
        if: startsWith(github.ref, 'refs/tags/v')
        env:
          VERSION: ${{ steps.ver.outputs.version }}
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          GH_USER: ${{ github.actor }}
        run: scripts/publish-guard.sh "v$VERSION" feedback-loop-ai/swarmail
```

A guard refusal fails the job before the push step is reached (sequential
steps). `main` never reaches it (the step-level `if`). The version rides
`env:` — never `${{ }}` inside `run:`.

**Chart job** — one extraction step mirroring the `steps.ver` idiom, one
guard step before the publish step:

```yaml
      - name: Read the chart version
        id: chartver
        run: echo "version=$(sed -n 's/^version: \(.*\)/\1/p' charts/swarmail/Chart.yaml | head -1)" >> "$GITHUB_OUTPUT"
      - name: Guard the chart version (exact ref match; refuse a republish)
        if: startsWith(github.ref, 'refs/tags/v')
        env:
          VERSION: ${{ steps.chartver.outputs.version }}
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          GH_USER: ${{ github.actor }}
        run: scripts/publish-guard.sh "v$VERSION" feedback-loop-ai/charts/swarmail
```

A malformed `Chart.yaml` version yields an empty or non-semver value → the
guard refuses → fail closed. The chart-internal coherence (rendered tag ==
appVersion) stays owned by `scripts/chart-verify.sh`, unchanged.

**Comments** — rewrite the two publish-path comment blocks to state exactly
the enforcement (decision 0019 §4):

```yaml
  # Publication is one gated run (decision 0018 §6): the docker build cannot
  # start until the five gates of the same invocation are green, and it runs
  # only for main and v* tag refs. `main` pushes the rolling `:main` and the
  # exact-commit `:sha-<sha>` only. The `:<version>` tag publishes only when
  # the triggering ref is exactly `v<Cargo.toml version>` (publish-guard.sh,
  # full-string match) and the registry probe reports that tag absent (404):
  # an existing tag refuses, an unreadable registry refuses. A pre-push
  # check, not registry-side immutability — a concurrent publish can still
  # race, and re-publishing a version means deleting the registry tag.
  # NOT enforced: anything about :main/:sha-<sha> (re-publishable by design),
  # and any actor who can already push tags or delete packages.
```

```yaml
  # The Helm chart: lint + render both profiles + assert the deployment
  # contract (scripts/chart-verify.sh, helm-only, no cluster) on every push
  # and PR. The packaged chart publishes only from a `v*` tag ref (decision
  # 0018 §6) — and only when the triggering ref is exactly `v<Chart.yaml
  # version>` (publish-guard.sh, full-string match) and the OCI probe
  # reports that version absent. A pre-push check, not immutability: GHCR
  # enforces none, a concurrent publish can still race, and re-publishing a
  # version means deleting the registry tag.
```

**The `quality` job** — one test line beside fmt/clippy and an honest display
name (the job id, and both publish jobs' `needs:` lists, are untouched):

```yaml
  quality:
    name: format, clippy, publish guard
    ...
      - run: bash scripts/publish-guard-test.sh
```

## 3. `scripts/publish-guard-test.sh` — the poles

A real local HTTP server (python3, the chart-verify toolchain) scripted via a
control file it re-reads per request, so one server serves every pole
deterministically; `PUBLISH_GUARD_REGISTRY` points the guard at
`http://127.0.0.1:<port>`. The fixture implements the same two-endpoint shape
the guard speaks: `/token` → `{"token": "test-token"}`, and
`/v2/<repo>/manifests/<tag>` → the scripted status, recording the last
requested path for the repo-path assertion. The connection-refused pole is a
closed port. Matrix (each case = fresh env + argv, deterministic verdict):

| # | Input | Expected |
|---|---|---|
| 1 | ref `v0.1.0` == expected, probe `404` | exit 0 (proceed) |
| 2 | ref `v0.1.0-rc`, expected `v0.1.0` | exit 1, names both refs (exactness) |
| 3 | `GITHUB_REF_NAME` unset | exit 1, named |
| 4 | expected `v01.0` / `vabc` | exit 1, "not valid semver" |
| 5 | probe `200` | exit 1, "already published" |
| 6 | probe `500` | exit 1, "registry state unknown" |
| 7 | probe connection refused | exit 1, "registry state unknown" |
| 8 | chart repo argument (`feedback-loop-ai/charts/swarmail`) | fixture recorded that path |
| 9 | drift pin: `grep -c 'scripts/publish-guard.sh' .github/workflows/ci.yml` ≥ 2 | green |

No sleeps (the server answers in-process; the refused pole is immediate), no
wall-clock assertions, chaos at the poles only. A mutation spot-check at
review time — disable any one refusal arm, watch the test go red — is a
one-off demonstration, not a committed broken matrix.

Executed in-session during design: this exact script text passed all ten
poles against a local fixture server, and — run unseamed against the
production `ghcr.io` with bogus credentials — refused with
`registry state unknown (token exchange failed)`, i.e. fail-closed off the
fixture too.

## Quickstart

```bash
bash scripts/publish-guard-test.sh          # the guard's proof, hermetic
bash scripts/coverage-gate.sh               # unchanged report, floor untouched
cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings
cargo test && cargo deny check licenses
brokkr compile --bundle bundles/self
# wiring inspection: grep the workflow for the two call sites, the env
# indirection, the comment blocks and the untouched needs: lists.
```

## Risks

- **The ghcr token-exchange/Accept shape drifts, or the probe misreads the
  registry.** Mitigation: the probe contracts on status codes only (stable
  protocol), and *every* misread lands on "unknown → refuse" — a wrong probe
  produces a red release, never a silent overwrite. `PUBLISH_GUARD_REGISTRY`
  is the documented seam the hermetic test uses; the fixture mirrors the
  two-endpoint shape so a script-side regression (not a registry drift) is
  caught by the gate.
- **The probe/push TOCTOU window** (seconds): two concurrent publishes of one
  version can both see `404`; GHCR is last-writer-wins. Mitigation: stated
  verbatim in the workflow comment (R3's priced residual); no in-repo
  mechanism can close it without the refused ledger's costs.
- **A red re-run of a previously green release** (probe now sees `200`).
  Mitigation: this is the intended audible refusal, not a bug — the comment
  and 0019 name the recovery (delete the registry tag, or re-tag a new
  version). Never skip-on-exists.
- **A partially green release** (image published, chart refused, or the
  reverse): the run is red and stays red until the operator deletes the
  registry tag or re-tags. Mitigation: named in the comments; the refusal
  names its arm so the operator knows which half landed.
- **Extraction fragility** (a `Chart.yaml` or `Cargo.toml` whose first
  `version:` line is not the declared version). Mitigation: the sed mirrors
  the existing `steps.ver` idiom; the guard validates strict semver and
  refuses on anything else — a malformed manifest can never publish.
- **`GITHUB_REF_NAME` reliance.** Mitigation: GitHub sets it for every job;
  the unset case refuses (test case 3); no expression-syntax dependency that
  a shell test cannot reproduce.
- **Guard/workflow drift** (an inline copy, or an unwired guard). Mitigation:
  the script is the only implementation and the test's drift pin (case 9)
  goes red if the workflow stops calling it.
- **The `quality` display rename confuses branch-protection dashboards.**
  Mitigation: the job id and both `needs:` lists are byte-identical; only the
  `name:` display changes.
- **Between this commit and the close, 0018 §Refused and the tree disagree**
  (0018 refuses the probe; the spec mandates it). Mitigation: decision 0019
  records the amendment; the landing tasks edit 0018 in place — the same
  pattern 0018 used for 0008.
