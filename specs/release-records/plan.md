# Implementation plan — release safety and truthful records

See [spec](spec.md) for requirements/ACs and
[proposed 0051](../../docs/decisions/0051-release-records.md) for release
semantics. This plan retains the implementation contracts and records the
resume approach; it is not a fresh verification verdict.

## Resume approach — 2026-10-19, re-drive 2

Confirm commit `8478924` on the existing `epic120-release-docs` branch; do
not recut it, rewrite it, or recreate its implementation. Only the three
existing spec artifacts change in this chief commit. Decision 0051 remains
committed, proposed and unchanged: no additional ruling is introduced.

First reconcile requirements against the existing owners and the prior commit's
issue/test map. Then run the final-tree local gates through implement/verify
and hand the evidence to review. If no gap is found, implement makes no change.
A demonstrated gap gets the smallest repair at its existing owner, the paired
regression, and a new signed commit. The file table below is the implemented
ownership map and the permitted repair scope, not a new list of required edits.

Retain the prior regression evidence rather than replaying mutations merely
to manufacture a diff. Fresh gate results must identify the assessed tree.
Helm render and real Docker smoke stay CI-only: report them as outstanding
until CI executes them, without blocking this offline design handoff. Keep
all other CI-only proof boundaries in the authoritative house inventory.

## Technical context

One Rust application crate, existing shell/Python delivery helpers, GitHub
Actions, OCI image/chart publication, Helm 3.16.3 and Docker. Existing tests
use actual SMTP/HTTP servers. The current builder is already digest-pinned;
there is no new dependency or reason to repin it in this batch. Host awk is
mawk. Verify is offline and installs nothing; Docker/Helm execution belongs
to the CI-only proof path. Keep the pinned proof ownership from batch 5.

No plan template exists under `.specify/templates/` (only spec-template.md).
This document supplies the requested technical context, constitution check,
project structure, research, data model, contracts and quickstart directly;
no additional design artifacts or unrelated workspace template are needed.

## Constitution check

No mail storage/protocol/extraction algorithm changes. The only production
Rust edits expected are help text and comments, plus status references whose
current wording is pinned by documentation tests. Do not move tests out of
coverage for convenience or add coverage exclusions. Private-seam access does
not force a harness: 0020's concrete cold-path rule does. Preserve exact
100/100/100, five-second busy bound, all prior repairs and permissive licensing.
Tests use per-test inboxes, deterministic poles and protocol readiness.

The operator already accepted the specified old proposals. Status metadata
can record that authority; accepted bodies are changed only by appended dated
amendments. 0051 is proposed. No new model routes, permissions, runtime API,
package, release action or unpinned verify entry point is introduced.

## Research and current-tree findings

- At `8478924`, chart publication needs Docker success explicitly and supplies
  `needs.docker.result` to the canonical guard's require-present mode. Docker
  needs chart verification and smoke; main's skipped-rate allowance remains
  confined to Docker. All existing exact-ref/republish checks remain.
- Cargo/appVersion are 0.1.0 and chart is prepared 0.1.2. The verifier derives
  its image expectation from appVersion and validates it against Cargo.
  The original 0.1.1 fixture remains a regression, not the current version.
- `/data` remains a VOLUME owned by uid 65532. Quickstarts carry --rm and
  30-second grace; recursive target exclusions and real Docker positive
  controls are implemented. The builder digest is pinned, but smoke and
  publication are separate builds, not identical-byte attestation.
- OpenAPI and existing API tests now cover the documented domains/statuses
  and oldest subject. Assert limit stays inert; runtime algorithms stay intact.
- `tests/store_contract_docs.rs` owns bidirectional membership/status checks
  and finite amendment fixtures. 0035, proposed 0051 and accepted 0055 are
  indexed; the authorized acceptance and dated pointers are present.
- Both panel positions find no implementation gap. The three files under
  `.brokkr/state/tasks/` concern older coverage runs; their branch/landing
  instructions do not override this resume commission and issue evidence.

## Project structure and implemented-file ownership

| Files | Implemented responsibility / repair owner |
|---|---|
| `.github/workflows/ci.yml` | Acyclic release needs/conditions; chart baseline inputs/selftest; read-only Docker smoke; existing guard mode call with explicit dependency result. |
| `bundles/self/scripts/publish-guard.sh`, `publish-guard-test.sh` | Present-image/Docker-success chart preflight and actual HTTP proof; preserve canonical ownership and public shim bytes. |
| `scripts/chart-verify.sh` | Shared version coherence check, dynamic render/annotation expectations, event-history bump check and selftest mode, main example render. |
| `scripts/docker-smoke.sh` | Single reusable real Docker lifecycle/context proof used by CI and local operators. |
| `README.md`, `.dockerignore`, `Dockerfile` | Cleanup/main pull flags, recursive excludes, signal comment, D-register install/release/runtime corrections. |
| `charts/swarmail/Chart.yaml`, `values.yaml` if required | Status/comment reconciliation and corresponding preparation version bump; no loss of values or probes. Artifact Hub image tracks appVersion. |
| `CONTRIBUTING.md`, `AGENTS.md`, `docs/house-rules.md` | One gate inventory, accurate authority/status vocabulary, proposal rule, signing/license clarification. |
| `agents/README.md`, affected `agents/charters/*.md`, bundle/policy descriptions | Paths, links, accepted decisions and four-member panel descriptions only; no active wiring changes. |
| `deny.toml` | Remove currently unused allowances after evidence; clarify CDLA comment. |
| `docs/releases/v0.1.0.md`, `docs/index.html` | Dated corrections, benchmark evidence boundaries, historical claims and #118 entry. |
| `src/static/openapi.json`, `src/config.rs`, `src/lib.rs`, `src/api.rs` | Schema, help and listener comments; extend existing schema tests. |
| `src/store.rs`, chart/listener comments and other current status references if found | Text-only synchronization required by existing status-aware doc tests; no algorithm changes. |
| Existing API/SMTP/TLS/listener/compat/binary tests and `tests/store_contract_docs.rs` | Reuse wire evidence, add missing discriminating checks, inventory/status/marker fixtures and finite documentation pins. |
| `tests/delivery_constitution.rs` and existing publish-guard wiring assertions | Pin dependency/input/guard wiring and preserve batch-5 pinned-chain checks. |
| `docs/decisions/*.md`, index | Authorized status metadata and dated linked addenda, missing rows, proposed 0051 index/pointers. |

Only the three `specs/release-records/` artifacts belong in this resume
chief commit. All paths above already have their implementation; edit them
only for a demonstrated unmet criterion during the implementation phase.
Positions and result/evidence files remain ignored run-local state.

## Release contracts and implementation

Keep the existing guard interface and absence default. Add an explicit
fourth argument `require-present` for the chart image check, with `bare`
probe form and `DOCKER_RESULT=success` mandatory in this mode; invalid modes,
other probe forms or missing/failed/skipped/cancelled results refuse before
publication. The existing absent mode remains backward compatible.
Pass `DOCKER_RESULT` from `${{ needs.docker.result }}` via environment, never
shell interpolation. Supply the image version from validated appVersion.
Ref binding remains exact; in a joint tag run the same ref must satisfy both
Cargo/appVersion and Chart version. Sequence chart preflight before package
push: coherence, successful dependency/present image, absent chart tag.
Mode tests use the existing real local HTTP server and status poles, with
unknown token/transport errors tested; no production registry writes.

Workflow pins must prove needs, tag-only condition, exact explicit success
condition, environment binding and guard-before-push ordering. Exercise the
guard's actual dependency truth table rather than writing an imitation of
GitHub's expression evaluator. A restored old independent-chart workflow must
fail wiring assertions. Retain all ordinary gates and rate success on tags;
main's Docker path alone keeps its accepted skipped-rate allowance.

Chart verification has two reusable components within its existing owner:
version/history checks that can run without Helm in `selftest`, and real Helm
render assertions. Parse strict version inputs; use numeric semver precedence
(including prerelease ordering; no lexical `0.1.10 < 0.1.9` error) and refuse
unsupported/malformed versions or build metadata consistently with publishing.
No new pip/Cargo dependency. Inputs ride arguments/environment, never eval.

Compare every tracked path under `charts/swarmail/` at candidate and base,
including adds/deletes. Baseline contract:

| Event | Baseline and failure behavior |
|---|---|
| PR | `pull_request.base.sha`, with the checked-out PR merge candidate; fetch required base object and compare the whole change, not HEAD's parent. |
| Ordinary push | Nonzero `event.before`; fetch it if needed. Unavailable/invalid history refuses, including an unavailable pre-force-push object. |
| New branch push (zero before) | Resolve merge-base of candidate and fetched main. Refuse if it cannot be established; do not treat zero as no chart changes. |
| Tag push | Nearest prior reachable `v*` release tag excluding tags on candidate, resolved through candidate's parent history; fetch required tags/history. Refuse missing/ambiguous release baseline. This repository already has release history; first-ever-release exceptions are out of scope. |

Validate current versions even with no chart diff. If files differ, require
strict version increase over base. Tests use throwaway real Git histories for
each event, including a multi-commit chart edit hidden behind a last docs
commit, shallow/missing history, non-ancestor push base, prereleases and a
bump-only edit. `chart-verify` runs this selftest and the actual check in CI.
Retain current template assertions and add coherent-app-version render twins.
Render the main example's literal option values without executing arbitrary
Markdown as shell. Check the README line and corresponding render together.

The smoke builds once to a unique local image name; CI publication may rebuild
through its existing cache. Start containers as the image user with --rm,
named volume, persistence env and random loopback host port mappings. Inspect
actual mapped ports, poll health/SMTP readiness until a deadline, and read
mail after final DATA 250. Graceful-stop and hard-kill cases each get their own
accepted marker before stop; recreate a container on the same named volume
and assert exact marker readback. Inspect user identity and volume ownership
through successful non-root writes, not an override to root.

For ephemeral examples inspect the anonymous volume name before stop and
assert it no longer exists afterward. Assert the named volume survives
container removal, then explicitly remove it. For context proof create a
private fixture context with the repository ignore file and sentinels in root,
tools and eval target directories plus a non-target control; build/export
with real Docker COPY behavior and inspect that only the control remains.
Use traps and unique resource ownership on both success and failure; do not
run global prune. Bounded readiness polling may wait between attempts, but
elapsed sleep never substitutes for a successful protocol observation.

## Documentation, schema and data model

No application data model, database migration or stored record changes.
The only modified public description is OpenAPI; server behavior is source
of truth. Treat each documented operation as a method/path response map and
each parameter as its actual parser domain and applied default, not one
universal page type. Inert assert limit has no effective paging default.

Extend current API pins to link real responses with schema entries. Reuse
existing servers/helpers/seams for default/zero/cap cases, hostile Origin,
degraded health, commit refusal, blank inbox and oversized bodies. Each
requested missing status must be exercised on its affected route/method;
guard 403 presence covers every OpenAPI operation and representative actual
router coverage must include the affected route families. Generate thread
fixtures that distinguish oldest/newest subjects and optional/missing subject.
Use existing deterministic inline tests where possible; a concrete cold path
goes in the owning sibling harness under cfg(test), not into production code.

Complete D01–D24 with a source and proof disposition in implementation's
task notes. Keep existing repairs and unsupported behavior claims distinct.
Use dated errata for historical statement corrections; do not move or rewrite
accepted ADR bodies, historical commits, or claim a new application release.
External issue correction is an operator handoff, not a tool action here.

## Decision record reconciliation

Use existing two status-header forms. Update the operator's named set only,
with dated acceptance provenance; index 0051 as proposed. Add inventory
enumeration excluding README, reject malformed/duplicate numbers, duplicate
or dangling index rows, missing rows and status disagreements in both
directions. Do not require contiguous numbering. Build disposable fixtures
for transitions instead of requiring any specific real decision to remain
proposed. Mutation tests must restore the old missing-0035/0055 case and fail.

Audit all amendment/appended claims, not just the index. Starting pairs
(newer record → older record) include:

| Newer | Required older scope/pointer audit |
|---|---|
| 0030 | 0018 and 0023 off-worker/placement changes |
| 0034 | 0022 SIZE and 0028 repeated AUTH |
| 0035 | 0020 §3 leak direction |
| 0036, 0037 | 0036→0027; 0037→0028 §1 |
| 0038 | 0021 and 0026 §3 memory/timestamp scope |
| 0039, 0040, 0041 | 0039→0029 §4; 0040→0031; 0041→0029 §5 |
| 0042, 0043 | 0042→0033 §1; 0043→0030 §2 |
| 0044 | 0029, 0030, 0031 and its update to 0039 |
| 0045 | 0007, 0009, 0015 §1, 0018; recorded 0030→0018/0023 pointers |
| 0046 | 0027, 0028 and updates to 0034/0036/0044 |
| 0047 | 0033, 0029 and updates to 0039/0042/0043/0044 |
| 0048 | 0025, 0029 and updates to 0041/0044 |
| 0049 | 0040, 0044, 0038; clarifications of 0030/0031 |
| 0050 | 0035 and 0045 refinements |
| 0051 | 0019 and 0045; explicitly proposed pointers |

Review earlier headers/inline claims too; the table is a floor, not permission
to leave a false promise elsewhere. Preserve exact relationship semantics:
amends, clarifies, supplements and records are not synonyms. Add a finite
test-owned list of audited pairs with expected link and marker scope. Check
the successor's declared relationship and predecessor's dated pointer, or
explicit dated correction, in both directions; remove each side in fixtures
to prove detection. No generic Markdown schema migration or prose inference.
Add current-status citation checks only for named process/contract references,
without rejecting historical proposed quotes in old decisions.

## Quickstart and validation

Design review: read spec AC1–AC9 against tasks; run the pinned prerequisite
helper with `SPECIFY_FEATURE_DIR=specs/release-records`. After the signed chief
commit is HEAD, the sequence's actual pinned design check must identify these
three committed spec artifacts and the already committed proposed 0051.
An uncommitted directory cannot satisfy that check.

Implementation proofs: execute publish-guard-test, chart-verify selftest and
relevant Rust API/binary/doc and delivery suites. CI executes chart-verify
with the actual event base and Helm, plus Docker smoke. For any new repair,
apply the old behavior/claim as an isolated mutation and record the named
failure plus repaired pass; using current fixtures for review-baseline
behavior avoids reverting unrelated earlier batches.
No test that simply repeats a new implementation is sufficient evidence.

Final tree, in requested order, with `cargo fmt --all` before each check:
fmt check; locked workspace/all-target clippy with warnings denied; locked
workspace tests; coverage and coverage selftest (no TMPDIR override); licenses.
Also run the house inventory's publish-guard proof and fail-closed native-tls
absence proof. Pinned brokkr compile belongs to verify; if local execution is
refused, record it and commit rather than bypassing the pin. CI owns chart,
Docker, RustSec, release-rate-5k and the existing real port-80 bridge proof.
Do not replace an unavailable CI proof with a green text assertion.

Only new signed commits on the existing `epic120-release-docs`, preserving
`8478924` and its base descended from 2a325c9. Implementation commit bodies
name which regression proves which issue. Leave the final branch clean, never push. If Cargo itself is
refused, immediately stop, save `.brokkr/state/epic120-release-docs.patch`,
leave uncommitted and report the block. If signing is impossible, stop rather
than using an unsigned commit. Filesystem refusal of branch/index writes is
likewise a delivery blocker, not permission to commit elsewhere or on main.

## Risks

| Risk | Mitigation and residual |
|---|---|
| Chart appears without its required image | Explicit needs/result, actual preflight success input and bounded present-image probe; later registry deletion and concurrent probe races remain outside control. |
| Partial image-only release cannot rerun all jobs | Document 0019's deliberate operator recovery; preserve refusal, add no deletion/skip override. |
| Baseline selection hides earlier chart edits | Event-specific full-history comparison, missing-object refusal and multi-commit/shallow fixtures. No parent-only PR shortcut. |
| New check blocks valid development preparation | Compare appVersion with Cargo; keep Chart version independent until exact joint release binds. Bump chart only if chart files change. |
| Docker smoke passes using root or leaks resources | Inspect configured uid, fresh owned resources, real readback and volume/context controls, trap cleanup; no global prune. |
| CI takes longer or local Docker/Helm unavailable | One local build per smoke and existing cache; CI proves unavailable execution before completion. No false local pass. |
| Documentation fix alters accepted behavior | D-register sources and wire pins; no store/SMTP/extraction algorithm edits. Keep inert fields and runtime hazards explicit. |
| Acceptance update breaks brittle tests or rewrites history | Fixture-driven status tests, existing header syntax, dated append-only corrections; retain 0047/0050 amendments verbatim. |
| Markdown tests certify links but miss false semantics | Finite claim/pair audit and human review; test membership/status/link obligations, never claim prose truth is automated. |
| Extra source-test branches lower exact coverage | Reuse deterministic tests, follow 0020 cold-path placement and run exact gate/selftest without exclusions. |
| Prior green evidence mistaken for current success | Keep prior commit/test map separate from fresh verify results and CI-only execution; do not mark outstanding proofs passed. |
| Design cannot be committed in sandbox | Retain complete artifacts and explicit failure evidence; no unsigned or alternate-repository workaround. HEAD validation remains outstanding. |
