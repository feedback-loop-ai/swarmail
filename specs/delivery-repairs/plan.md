# Implementation plan — delivery repairs

## Technical context

One Rust crate with real-protocol integration suites; Bash delivery scripts,
GitHub Actions, a scratch Docker image and a Helm chart. The current brokkr
record is 0.12.0 at `67a1886e5fcfa777e5bdc7eb3c153bb1fa53bbcf`.
No new production dependency or application data migration is needed. Bash
checks must work with the host's mawk. No plan template is present in the local
scaffold; this plan supplies the dialect's required context, constitution,
structure, research, model, contracts, quickstart and risks directly.

## Constitution check

Preserve 0001 synchronous insert/acceptance visibility, 0002 subscription before
scan, 0003 oldest-only prune and 0005 ingest extraction. Preserve 0019's exact
release-ref bind, no-republish guard and named race. Preserve 0020's placement
rules and 0035's by-effect coverage checks and exact line/branch/function floor.
Keep the review phase protected, security-hold terminal and 0013's routes.

The proposed [0045](../../docs/decisions/0045-delivery-repairs.md) records delivery
and documentation contract corrections; it does not accept itself or amend the
frozen behavior. Accepted decision bodies get dated addenda, not rewritten
historical text. Only the three design files and 0045 belong to the chief's
commit. Runtime/CI/docs repairs listed below belong to implementation.

## Research and prerequisite findings

Both panel positions and all four issue bodies inform the spec's explicit
rulings. Baseline inspection confirms worktree coverage/guard execution, no
final design exec step, a `drafted` bypass in policy, chart 0.1.0 and absent
`.specify/scripts/bash/common.sh`. Baseline `brokkr compile` succeeds, emits a
manifest covering bundle scripts/tooling, and reports the residual-policy
warning; therefore “clean compile” means no warning, not no stdout.

Before edits to the constitution, use a disposable bundle fixture and the
recorded brokkr to prove all three engine assumptions:

1. A nested marker/helper is included in the compiled manifest, changing it
   changes the pin, and exec dispatch exposes those pinned bytes. Manifest
   membership alone is not proof of runtime mount behavior.
2. A sequence accepts a final exec driver and returns its verdict rather than
   chief's intermediate `drafted`. Exercise success and failure through actual
   dispatch. If step drivers are unavailable, use a separate deterministic
   `design-check` seat/phase between design and implement, with first failure
   returning to design and second consecutive validation failure stopping.
   The fallback must preserve attempt accounting across the transition. Amend
   the owning plan/policy descriptions to that concrete shape; never leave
   drafted → implement in either shape. If the pinned engine cannot express
   this safely, report the blocker; a model check is not a fallback.
3. Inspect and compile supported policy predicates for explicit false and
   severity <= medium; exercise missing-input valuations, not just syntax.
   If conjunction/negation is unsupported, enumerate supported safe valuations
   or add a deterministic aggregate validation boundary that refuses absent/
   invalid facts before policy. Silence annotations and an aggregate promise
   without an executable refusal are rejected. An engine pin change, if
   required, must be explicit and justified rather than invented grammar.

## Project structure and ownership

| Paths | Implementation responsibility |
|---|---|
| `bundles/self/scripts/verify-seat.sh`, `bundles/self/gates/` | Pinned coverage/guard implementations, their scanner crate and integrity/selftest support. |
| `scripts/{coverage-gate,publish-guard,publish-guard-test}.sh` | Stable thin entry points; no duplicate algorithm. |
| `tools/coverage-fn-scan/` | Relocate canonical source/Cargo.toml/Cargo.lock into the bundle; repair callers and tracked references. No duplicate helper source remains. |
| `bundles/self/scripts/design-check.sh`, `scripts/design-check.sh` | Move checker into the bundle; retain only a compatibility dispatcher if a caller still needs the old path. |
| `bundles/self/{bundle,policy,tooling}.json` | Deterministic sequence/phase, explicit residual admission, recorded Rust build toolchain. |
| `.specify/scripts/bash/{check-prerequisites,common}.sh`, `dialects/` | Repair missing prerequisite support and select the same explicit feature slug. Pin executable validator dependencies used by the gate. |
| `.github/workflows/ci.yml`, `scripts/chart-verify.sh` | Dependencies, permissions, tooling validation/build, selftests, timeouts, safe Helm scratch and render checks. |
| `scripts/docker-smoke.sh`, small chart-version check if needed | Docker persistence/context proof and baseline-tag/version check, each with one owner. |
| `Dockerfile`, `.dockerignore`, `charts/swarmail/Chart.yaml` | Owned `/data`, excluded state, chart package 0.1.1; retain deployment knobs and appVersion. |
| `README.md`, `docs/{house-rules,index.html}`, `docs/releases/v0.1.0.md` | Install examples, limits, process, chart table and dated historical corrections. |
| `agents/README.md`, `agents/charters/*.md`, `deny.toml` | Tracked links, correct bundle command, panel/signing language, aligned license rationale. |
| `src/config.rs`, `src/smtp.rs`, `src/static/{llms.txt,openapi.json}` | Help/comments and public documentation; no wire behavior rewrite. |
| `src/api.rs` existing inline schema tests; `tests/{api,mcp,config_cli,starttls,smtp_edges,pop3}.rs` as needed | Owner-local schema/help and real-server contract proofs. Reuse existing test filenames when a named suite is absent. |
| `docs/decisions/{README,0015-*,0018-*,0023-*}.md` | Index 0045 and append dated amendment pointers preserving accepted bodies. |

## Implementation approach

### 1. Execute one pinned implementation

Move full gate scripts under `bundles/self/gates/`; move the scanner crate
there as well. Resolve a trusted gate-resource root from the pinned seat's own
path and pass a separate explicit candidate-workspace root. Keep measurement
and scratch under the workspace `target/coverage/`; helper builds remain
offline/locked and use the pinned manifest and lockfile. Do not resolve helper
source through candidate-relative `tools/` paths or trust a stale helper binary.

Small public shims dispatch to the canonical scripts for developers/CI. Keep
their expected bytes as small pinned templates (not copies of full scripts).
Verify compares the candidate shims to those templates before any expensive
gate and refuses drift, satisfying the changed-script criterion. Missing,
symlink-substituted or redirected entry points refuse. The seat then invokes
canonical pinned paths directly, regardless of candidate shims. Do the same
for executable prerequisite support; canonical trusted helper selection must
not depend on mutable search paths or user-supplied environment overrides.

Selftests use isolated fixture workspaces: green twin, changed/no-op shim,
changed scanner source, changed guard target and missing resource. Assert the
actual seat's preflight refuses or dispatches the pinned helper, never the
substitution. Coverage's full existing selftest is retained after relocation.
The chart script is a CI proof, not a shell that decides hermetic verify; no
hash inventory of unrelated candidate documents is introduced.

### 2. Check the chief's exact committed artifact set

Immediately after chief, record HEAD and inspect its changed paths: require
exactly one `specs/<slug>/` candidate with all three design artifacts in that
commit. Do not walk mtimes or rank commit dates. Validate the committed blob
contents and confirm the corresponding working files and declared decision
have no staged, unstaged or untracked replacement. Resolve declared decision
references within `docs/decisions/`, reject absent/ambiguous files, and require
status proposed. Use the same explicit slug for dialect prerequisites. Repair
the missing common helper with minimal feature-path/validation support; do not
import a workspace orchestration framework or require numeric slug prefixes.

The test harness creates isolated real Git repositories with equal mtimes,
older valid specs and a newly committed invalid spec, multiple slugs, dirty
decisions and missing artifacts. Exercise the wired exec path, its result
file, and retry/stop transitions; shell exit alone is insufficient because
drivers communicate verdicts through result JSON. All result records have
only `result`, `inputs` and `notes`, with facts inside `inputs`.

### 3. Repair workflow conditions and tooling parsing

Retain existing jobs where possible. Split `chart` into `chart-verify` and
tag-only `chart-publish`. Add read-only `docker-smoke`. Both publishing paths
need quality, test, coverage, deny, dependency-audit, brokkr-compile and rate;
add their artifact proof as specified in AC4. Each prerequisite is checked
explicitly if a status function is needed to override GitHub's skipped-needs
behavior. Main can accept only rate's skipped result; tags require rate
success. Event/ref guards exclude pull requests. A status function alone is
never an admission condition. Preserve existing exact-ref/registry guards.

Keep the permission exception for continuous main explicit in workflow comments
and house rules. Grant no publishing credentials to chart verify or Docker smoke.
Extend the existing publish-guard proof for the final job names and dependency
matrix, retaining its real local registry server at 200/404/error poles.

Parse JSON with a real parser in CI; for verify use an already available parser
or a narrowly validated pinned record format, refusing duplicate/missing/invalid
fields. Do not invent a permissive grep parser that accepts multiline output.
Share validation behavior/fixtures rather than a second policy vocabulary.
Record the numeric Rust version actually proven to build the pinned engine.
Add a tracked explanation of the pin and its version-only provenance limit.

The chart bump check compares tracked chart changes against the last reachable
release tag using full history. It requires a strictly newer package version
when chart content changed. Fixtures cover unchanged, changed/unbumped,
changed/bumped and missing-history cases. No registry lookup is needed.

### 4. Prove the container and chart contracts

Create an empty data directory in the existing build stage, COPY it to `/data`
with 65532 ownership and then declare VOLUME. Do not add a root entrypoint.
Add recursive exclusions for database families and companions, including
`**/*.db-*`, `**/*.db.lock` and `**/*.sqlite*`, plus both state directories.

The Docker smoke builds with load and no push, uses unique container/volume
names and bounded readiness polling, and binds the container's listeners
explicitly off loopback. Use real SMTP acceptance plus HTTP readback with
per-test inboxes. Deliver before each restart and require those acknowledged
messages after graceful stop (30-second grace) and SIGKILL. Assert UID and the
volume's writability through actual successful persistence. Trap cleanup.
Seed root/nested state/database sentinels in an isolated context fixture; a
probe stage copies the context and asserts their absence. Never copy operator
state into a probe image just to inspect it.

Helm gets `mktemp -d`, trap cleanup, and environment-passed scratch paths for
its existing Python assertions. Two concurrent invocations must not collide.
Retain all current TLS/peer/probe/security contracts, adding the missing binds.

### 5. Reconcile the finite documentation inventory

Use the spec's correction table as the inventory, trace each statement to
current code, and preserve dated clarification entries. No broad crawler or
generated docs system. Extend existing inline schema tests with defaults,
minimums and operation-specific responses; real-server tests exercise 400,
403, 413 and 503 where the current route actually returns them, plus MCP error
envelopes and await/assert zero/default semantics. Test help through the binary.
Use existing deterministic seams for degraded/stopping state; no sleeps as
synchronization or global mutable test configuration. Tests needing cold
failure diagnostics follow 0020 placement, not blanket harness relocation.

## Data model and contracts

No message schema, store API or persisted row changes. The only added tooling
data is `brokkr.rust` in tooling.json, a validated stable toolchain version.
The chief's commit is the design identity; there is no mutable latest-spec
database. Driver output remains a JSON object with `result`, `inputs`, `notes`.
Chief: `drafted`; deterministic design validator: `designed|fail`; verify:
`pass|fail`. Runtime API corrections describe existing behavior only.

## Quickstart and validation

After implementation, run from the repository root with
`export PATH="$HOME/.cargo/bin:$PATH"`:

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
cargo deny check licenses
bash bundles/self/scripts/verify-seat.sh selftest
bash bundles/self/scripts/design-check.sh selftest
bash scripts/chart-verify.sh
bash scripts/docker-smoke.sh
```

Also execute the common native-tls absence proof, tooling-parser fixtures,
policy valuations and chart-bump fixtures. Coverage runs without a TMPDIR
override. CI owns network advisory audit and Docker/Helm evidence; offline
verify must document those differences, never report them as locally proved.
Gate evidence belongs in run-local logs, not the chief's four-file commit.

For installation proof build `swarmail:delivery-repairs` locally, run with a
fresh named volume and `SWARMAIL_DATA_FILE=/data/swarmail.db`, and render/install
`./charts/swarmail --set image.tag=main` from a checkout. No release credential
or publish command is needed. Final implementation delivery uses the normal
branch/PR workflow; this design seat neither pushes nor merges.

## Risks

| Risk | Mitigation and remaining cost |
|---|---|
| Relocated scripts accidentally test the bundle instead of the application | Separate resource/workspace roots; fixture assertions on measured files and output roots; run full by-effect selftest and real coverage. |
| Pin covers shell but misses executable helper | Include scanner manifests/lock/source and prerequisite helpers; mutate each dependency in dispatch fixtures. Inspect manifest and runtime mount separately. |
| A live run pins the old constitution | Never edit pins to resume past drift. A changed constitution requires normal operator-controlled re-entry with new pins; no weakening of drift checks. |
| Unsupported engine sequence/predicate grammar | Resolve first with the pinned binary and fixture dispatch; use the bounded phase fallback only if proven. Record blockers instead of bypassing checks. |
| Validating the wrong or dirty spec | Require a single HEAD design artifact set; test older valid decoys, equal mtimes and dirty decisions. Unrelated post-chief commits refuse. |
| Skipped-rate dependency blocks main or bypasses red coverage | Explicit event/ref and every-needs result checks; full publishing matrix including cancellations. |
| Chart-only bump makes a future joint release uneven | Document separate versions and 0019's refusal; future release preparation aligns manifests deliberately. No claim of atomic multi-artifact publication. |
| Docker smoke adds runner cost / existing volumes stay root-owned | One local build per relevant CI run; unique resource cleanup; document ownership repair and local-build requirement until image publication. |
| Stale issue claims overwrite later fixes | Trace against current run-21 code, especially MCP errors and compat offloading; pin examples to wire tests. |
| New test code creates unreachable measured diagnostics | Keep stable schema assertions inline; place cold diagnostics according to 0020, never exclude production code. |
| Documentation implies operator acceptance | Leave 0045 proposed; distinguish compatible implementation from a changed accepted ruling; preserve all frozen guarantees. |
| Sandbox cannot execute gates or write Git objects | Do not claim green/committed. Preserve the four drafts and a patch, record exact refusals, and stop retrying blocked gates. |
