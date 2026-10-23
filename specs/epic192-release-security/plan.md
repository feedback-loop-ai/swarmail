# Continuation plan — epic192-release-security

Implements [spec.md](spec.md); ordered proof-paired work is in [tasks.md](tasks.md).

## Technical context

One Rust crate; this change needs no executable Rust edits. Bash guards use
curl and Python for real OCI-shaped local HTTP tests. Chart verification
already uses Python/PyYAML and Helm; Git fixtures exercise history. Cargo
packages the application, licenses and pinned WHATWG snapshot; a scratch
image embeds the generated table. Current version preparation may differ
between chart and Cargo; this work does not bump either.

No plan template exists in `.specify/templates/` at the continuation base. This
plan supplies its required context, constitution check, structure, research,
data model, contracts and quickstart directly. Research inspected the current
metadata owner and its optional Helm branch, actual CI callers, Dockerfile,
Cargo includes, CSP comment, NOTICE, both panel positions, intake and inherited
spec/plan/tasks/handoff. The original chief's missing-Helm/PATH observation is
historical. Current coordinator receipts identify official Helm v3.16.3 and
one successful candidate render/coherence/refusal execution; they do not prove
all equivalence/mutations or current engine phases. Docker 29.9.2 on the host
does not imply sandbox access. No dependency or tool installation is planned.

## Constitution check

Preserve [house rules](../../docs/house-rules.md), 0001–0006, single-writer
protection, 0020/0035's exact denominator and 0045/0050's externally pinned
proof chain. Runtime data paths and persistent schema stay unchanged. Preserve
0019's already implemented dated observational errata; corrected 0051
acceptance is settled by the supplied ruling and adoption belongs to deferred #184. This
compatible repair declares no ADR. No dependency/allowlist, recipe, policy,
bundle configuration, adapter, shim or shim-hash edits. Canonical candidate
guard/proof scripts are change inputs, never replacements for the live baseline proof.
Commit unsigned under 0056 and preserve the 0057 engine route and fixed four
review lenses; security holds remain hard stops. The task-specific CI-green
GitHub-signed squash landing is coordinator-owned without separate sealing;
no seat pushes or merges, and no release act is authorized.

## Project structure and touched files

Retain implementation `26f6430` on continuation base `4970a512`. The table is
the existing ownership boundary, not an instruction to rewrite completed
repairs. Change an implementation file only when a current proof identifies
a defect, keep its regression in its existing owner and rerun affected controls.
The design commit touches exactly this directory's spec/plan/tasks; handoff
and inherited audit/receipts stay byte-identical. A later implementation seat
may append dated current dispositions to handoff without rewriting history.

| File | Implementation scope |
|---|---|
| `.github/workflows/ci.yml` | Call the chart metadata owner; strict version output; preserve and prove publication wiring; attribution proof may join an existing image job. |
| `scripts/chart-verify.sh` | Safe duplicate-aware YAML metadata operation, current/history consumers and exhaustive selftests; actual CI extraction and Helm equivalence proofs. |
| `bundles/self/scripts/publish-guard.sh` | Strict release version validation, stdin Authorization, bounded fail-closed token/status handling. |
| `bundles/self/scripts/publish-guard-test.sh` | Header-enforcing HTTP, argv observer, owned refusal socket, distinct arms, asserting 12b and workflow/mutation proofs. |
| `src/ui.rs` | CSP comment only. |
| `Cargo.toml`, root `NOTICE` | Root-anchored includes and packaged attribution; preserve SPDX and all versions/dependencies. |
| `README.md` | License sentence/link only. |
| `Dockerfile` | Attribution copy into final image only; leave H's first comment untouched. |
| `docs/decisions/0019-release-tag-integrity.md` | Append dated rerun/polling corrections, preserve accepted body. |
| Exact 20 historical `specs/**` paths in inherited audit | Preserve their completed privacy-only transformations and every unrelated claim. |
| `specs/epic192-release-security/{spec,plan,tasks,handoff}.md` | This design, receipt checklist and subsequent truthful task/evidence disposition. |

No other tracked file is authorized. Keep temporary fixtures and evidence in
ignored run-private directories. Do not edit another worktree or the baseline,
launch nested runs, create a private active journal, or mutate/import the
historical snapshot. The canonical journal remains the root `.forge/forge.db`.

## Current execution sequence

1. Bind the current tree and proof identities to the continuation preparation
   receipt. Validate inherited hash applicability and preserve original
   `implementation/audit.json` and retry records. Use new ignored evidence
   under `.forge/epic192/continuation/`; no active journal is created here.
   Preserve complete issue bodies in intake. Add dated findings rather than
   modifying original dispositions; original task marks are historical only.
2. Resolve official Helm v3.16.3 through the authorized route and verify its
   digest against the installation receipt. Record actual invocations to show
   the optional selftest branch ran. Execute `selftest`, `verify`, real Git
   history fixtures and actual extracted CI version bodies. Complete supported
   metadata equivalence using `helm show chart`, `helm package` and inspection
   of the resulting package for both fields, comments and whitespace. Run all
   negative fixtures, including first-key CI mutation and repaired/render-arm
   mutations. A tool absence, syntax error or unrelated dirty check is not red.
   If the host tool is not mounted, request these exact commands/fixtures and
   source-bound results from the coordinator; never download a replacement.
3. Rerun candidate guard/proof and inherited mutation matrix with synthetic
   credentials, real HTTP, held sockets, exact diagnostic needles and restored
   controls. Keep immutable proof replay separate. A classifier-only 000 proof
   retains the documented installed-curl reachability limit; a new tool version
   requires recording applicability rather than claiming real successful 000.
4. Run the original scope assertions from a private copy of the audit driver
   with only its output destination changed to current ignored evidence. The
   inherited driver writes audit.json, so never invoke it against that original
   destination. Add a separate continuation audit for the design delta and
   preserve original driver/receipt bytes. Rerun
   actual locked package list/archive, nested cache decoys and unanchored
   mutation; compare notice/license/data bytes, README and provenance. Preserve
   CSP executable bytes, H comment, accepted 0019 prefix and license policy.
5. Request the coordinator's real image proof with exact source/build-context
   inventory, commit/tree/dirty differences, Dockerfile/NOTICE hashes, Docker
   version, build argv, logs/exits and image ID/digest. Extract `/NOTICE`
   without adding a shell to scratch; show readability, mode 0444 and equality
   with root and crate archive. Remove only the NOTICE COPY in a disposable
   context, build and require the attribution assertion to fail specifically;
   restore identical source, rebuild and pass. Run-unique names avoid collisions
   even for identical source hashes. Record cleanup including anonymous volumes.
   Keep host origin explicit; inspect applicability after any source change.
   No socket exposure, extra privilege or private/rootless daemon attempts.
6. Collect fresh complete house gates, configured extras, candidate proofs and
   immutable proof origins, then actual engine hermetic verify, four fixed
   independent reviews and successful ship through the root-owned route.
   A replay of pinned scripts is not actual dispatch. Do not launch a nested
   run. A changed source after proof requires affected checks again and current
   phases against the final candidate. CI-only and external release evidence
   remain individually named, with no fabricated pass or waived unknown.
7. Hand off every finding/AC with current receipts and precise open items.
   Supply `26f6430`'s 0019 errata dependency to deferred #184 before its exclusive
   corrected-0051 adoption. Preserve the external release checklist and explicit
   AC17 pending status even after compatible ship. Coordinator landing grants
   no tag, release, registry write/deletion or publication permission.

## Retained implementation approach

The following describes the inherited implementation and its continuing proof
contract. These mechanisms already exist in `26f6430`; imperative instructions
apply only to a demonstrated regression or missing proof, not a second rewrite.

### 1. One chart metadata owner; independent guard validation

Retain the narrow `metadata` mode in the existing chart script, taking an optional
Chart.yaml path (or stdin) as a data argument and emitting only validated `version=...` and
`app=...` lines. CI captures successful complete output before appending it to
GITHUB_OUTPUT. Failure produces no usable partial output. The metadata reader
must not lint/render Helm or require a registry; verify/history reuse it.

Use Python's existing safe YAML facility with duplicate detection before
mapping construction, so quoted-equivalent keys cannot collapse unnoticed.
Accept a single top-level mapping with required string scalar version and
appVersion, then apply strict release SemVer. Reject duplicate metadata keys,
non-string/collection values, unknown custom tags and malformed documents.
Reject merge/alias metadata and multiple documents explicitly; do not invent a
lexical approximation for those cases. Current and baseline chart texts take
the same reader. Keep appVersion=Cargo and existing annotation/render coherence,
while chart version remains independent during preparation. Historical version
comparison must not accidentally compare old appVersion to today's Cargo.

Expose only the metadata operation CI needs. A fixture suite executes the
actual chartver run body extracted from CI with a private GITHUB_OUTPUT and
safe environment; it must detect a caller switched back to sed/head, even when
the helper itself still passes. Compare accepted fixtures against installed
Helm's parsed/package metadata. Include quoted keys/values, comments, duplicate
order, whitespace, missing/non-string values, merge/alias and document cases.
Prove PyYAML import in each new job context; do not install tools in hermetic
verification or introduce an unpinned download. Missing prerequisite or Helm
comparison is an outstanding proof, not permission to skip it. A different
reader requires a documented spec/plan revision and matching proofs.

Retain strict SemVer in the canonical guard independently of mutable chart
code: core numeric components cannot have leading zeros; prerelease identifiers
are nonempty ASCII alphanumeric/hyphen, numeric identifiers reject leading
zeros. Preserve named build-metadata refusal and exact ref checks. The Docker
version-output block must validate the same language; run its actual body on
Cargo fixture values. Small duplication at pinned boundaries is intentional;
no sourced helper outside the pinned bundle or generic validation framework.

### 2. Secret transport and observable real HTTP

Generate Basic bytes from environment/stdin using builtins or Python without
secret process arguments. Feed `Authorization: Basic ...` to curl's header
stdin interface (`--header @-`); similarly feed the bearer header to manifest
curl. Static Accept may remain argv. Validate token JSON type, nonempty value
and absence of header controls before interpolation; never dump response
secrets. Retain URL scope/service, max-time bounds, modes and exact tag forms.
Confirm installed curl's interface support before treating it as proven.

Keep the real local fixture configured to reject missing/wrong Basic, bearer, Accept,
service/scope and manifest paths. Use distinct synthetic exchange and bearer
secrets. A test-only PATH observer records argument vectors and execs real
curl, inheriting stdin unchanged; do not capture real environment/stdin or
ambient credentials. Assert no raw/encoded secret enters any observed argv.
Mutate each header away and each secret back into argv independently.

The fixture owns a listening port-0 socket and a bound non-listening refusal
socket until teardown. Signal readiness after both exist; cleanup waits for
children and releases sockets. Preserve a bounded readiness predicate, never
fixed-delay correctness. After a successful exchange, deliberately close the
manifest connection to prove the actual manifest transport-error branch.

Attempt a raw local response that yields successful curl with HTTP 000 and
record the installed curl behavior. If unreachable, the only permitted fallback
is a test-only observer that performs a successful real fixture request, checks
real curl success, and substitutes only its final status-output bytes with
`000`. It must preserve argv, stdin, request/header checks and real failure
exit codes. Label that receipt classification-only and pair it with the real
manifest-transport receipt; never call it real GHCR/HTTP-000 evidence. No
production override, wholly simulated curl or broad error-to-success coercion.
Use distinct diagnostic needles for each arm and mutation tests for each.

### 3. Exhaustive proof in the existing owners

Retain the meaningful stale-v 12b scenario and removal of redundant 12d.
Preserve both-absent, bare-present, bare-only and require-present poles.
Strengthen the repository-specific workflow checker around parsed jobs/steps
or a tightly bounded supported shape, refusing unknown bypass shapes rather
than counting references. Require correct guard conditions, order, failure
propagation and dependencies. Mutate actual workflow copies: guard after push,
missing/commented guard, skipped guard, continue-on-error at job/step scope,
ignored shell failure, image-result propagation and removed ordinary/rate
prerequisites. Run the actual repaired checker on each copy; a no-op mutation
is a test failure. This is not a replacement Actions expression evaluator;
execute relevant shell bodies where their behavior is the claim.

Inventory chart refusal sites before editing, mapping each to a unique case
and diagnostic. Extend coherence/history helpers and use real temporary Git
histories for missing before, malformed baseline and all other history arms.
Cover invalid modes/metadata and rendered-contract refusals through their
actual evaluators; Helm-dependent checks stay distinct from no-Helm selftests.
Disable each repaired refusal in a disposable copy, require its case to fail,
then restore green. Place permanent regression checks in the two existing
proof scripts/owned workflow; use ignored scratch drivers for mutation receipts
that cannot be kept there without new public interfaces.

### 4. Honest prose and distributable attribution

Preserve the corrected CSP comment and dated 0019 errata for reruns/polling;
keep prior paragraphs byte-identical, including historical status pointers.
Provide deferred #184 the errata commit and integration requirement so its
queued adoption can revalidate source hashes/applicability. No index/0051 edit.
Redact all actual home-path matches from historical specs with portable
repository-relative paths or descriptive placeholders while preserving commands'
meaning and evidence claims. Audit the diff beyond a zero-match grep.

Keep one NOTICE using `src/extract/entities.rs`'s actual provenance: WHATWG
(Apple, Google, Mozilla, Microsoft), CC-BY-4.0 URL, source entities URL,
2026-10-18 snapshot SHA-256
`d741d877ac77c4194c4ad526b5b4a19aef8dfe411ab840a466891cdbb9f362e6`,
unmodified snapshot and the transformation to 2,125 semicolon-terminated
NAMED entries. Verify the hash rather than transcribing an unsupported value.
Preserve application SPDX; include `/NOTICE` and root-anchored licenses.
README License names the component/license and links NOTICE. Keep identical
NOTICE bytes copied from the builder into final image `/NOTICE` with mode 0444.

Run actual package listing and archive creation/inspection on clean committed
candidate content (or a private candidate fixture), including nested
`.cargo-home` license decoys. Check intended exact paths and hashes. Mutate
anchors back and require the real package assertion to fail; no simulated
Cargo walker. Build an actual image when available, extract `/NOTICE` through
container/image filesystem tooling and compare; cleanup any created container
with its anonymous volumes. No shell need be added to scratch. Keep the image
attribution proof in the owned CI workflow if local Docker is unavailable,
and report it outstanding until a real result exists.

## Data model and contracts

No persistent schema, runtime model or public protocol changes. Ephemeral
records are: validated chart metadata `{version, appVersion}`; HTTP fixture
control `{token outcome, per-tag status/transport action}` and recorded request
paths/header verdicts; owned socket readiness; mutation receipts; and the
operator release receipt. Real secret bytes are never evidence fields; fixture
secrets are synthetic.
Each new ignored receipt identifies candidate commit/tree and any dirty patch,
source/build-context manifest, invoked tools/helpers with SHA-256, exact argv,
exit and stdout/stderr/log identity, fixture/mutant/restored bytes, the owning
failure diagnostic and restored success. Host receipts additionally identify
build/image/extraction and cleanup. Receipts are evidence records, not a new
schema/service or journal. Source hashes alone cannot identify an unrecorded
build context or certify execution.

The guard's existing inputs and outputs stay: v-prefixed expected ref, OCI
repository, both/bare forms and require-absent/present mode, environment ref
and image result. Only 404 admits absence and only 200 with Docker success
admits presence. All uncertainty refuses. Exact Docker both-form and chart
bare-form naming, independent preparation and successful joint-tag equality
remain. The metadata operation emits validated data, never shell source.

## Quickstart and verification

Use the launcher's exact engine and immutable baseline identity from run
context; resolve their actual mounted paths rather than using a version-equal
binary. Never edit those inputs to get a green gate. After implementation:

```bash
bash bundles/self/scripts/publish-guard-test.sh
bash scripts/chart-verify.sh selftest
bash scripts/chart-verify.sh verify
cargo package --list --locked
cargo package --locked
cargo deny check licenses
```

Run chart `history` with actual event inputs and full available history, plus
fixture histories; do not fabricate a baseline. Package creation may require a
clean committed candidate or private fixture; no release action is implied.
Run every command in the authoritative
[house-rule gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
with applicable Cargo `--locked`, including exact coverage, coverage selftest,
publish proof and keyless seal proof. Also execute the configured design-check
selftest, delivery tamper-boundary proof, recipe structural tests and fail-closed
native-tls absence proof: only the expected cargo-tree no-match diagnostic and
exit count as absence; arbitrary Cargo failure does not.

Run the immutable externally pinned chain against candidate inputs AND the
repaired candidate canonical publish proof/guard and chart proofs. For each,
record exact executable/helper paths, SHA-256, command, candidate commit/tree,
exit, log path and proof origin in ignored run evidence. Record mutated-input
hash/diff, observed named failure, restored bytes and green rerun. Tests must
fail because the claimed defect returned, not because a tool is missing.
Do not change public shims or shim hashes; a baseline green cannot validate
new proof logic. Unrelated failures go to parent with evidence.

CI-only evidence remains separately identified: chart verification/history,
real Docker persistence/context/volume smoke, network RustSec audit, tag-only
release-rate-5k and real port-80 bridge. Local package/chart/image evidence is
still required where available and is not a claim to those CI results.
Actual next GHCR receipt is external AC17. Preserve the exact checklist in the
spec. Handoff includes every finding, commits/paths, evidence, preserved
rulings, no new ADR, outstanding obligations and deferred #184 integration.

## Risks

| Continuation risk | Mitigation / retained cost |
|---|---|
| Historical checked tasks become invented current phase results | Preserve original marks/handoff; new C-tasks start unchecked and require dated source-bound evidence. Engine records alone establish verify/review/ship. |
| Helm selftest skips the equivalence branch despite exit 0 | Bind official binary and record real show/package invocations and comparisons, plus defect-specific red/restored controls. |
| Host image receipt is stale or context incomplete | Require complete build-context/source identity, exact image/extraction bytes, removal/restored builds and applicability review; keep AC16 open until supplied. |
| Concurrent proofs delete each other's containers or sockets | Run-unique names, private scratch, held sockets and owned cleanup; no shared mutable fixtures. |
| Unauthorized release is used as a global stop or false success | Continue compatible phases and retain AC17 EXPLICIT EXTERNAL PENDING; never close 18.5/#186/#192, invent a release or waive unknown residual facts. |
| Cross-lane adoption overwrites errata | Leave #184's adoption/index untouched; pass the exact errata commit and require hash/applicability checks after integration. Preserve H's comment. |

| Risk | Mitigation / retained cost |
|---|---|
| PyYAML/Helm semantic or provisioning mismatch | One duplicate-aware owner; actual CI-block and Helm comparisons; explicit unsupported YAML refusals and prerequisite checks. No download or silent fallback. |
| Candidate helper contaminates baseline proof | Guard validation stays independent; distinguish immutable baseline path/digest from repaired candidate path/digest and run both. |
| Header stdin breaks auth or lets token controls inject headers | Enforce real headers and token shape, observe actual argv, use synthetic secrets and independent mutations. |
| HTTP 000 test certifies a different branch | Record reachability, exact diagnostics and the narrowly labeled classifier proof alongside real transport; preserve both obligations. |
| Workflow text checker accepts bypass/dead code | Check actual supported steps/conditions, exercise extracted shell and mutant workflow inputs; refuse unrecognized shape instead of guessing. |
| Fixture races/flaky correctness ceilings | Child-bound ephemeral listener, held refusal socket, explicit readiness/lifecycle and private scratch; no sleep-as-sync. |
| Packaging fix hides missing attribution or cache traversal | Real Cargo listing/archive with nested decoys and anchor mutation; identical notice in actual image filesystem. Missing Docker remains open. |
| Cross-lane history/ownership conflict | Dated 0019 additions only, #184 integration receipt, README License-only edit, H's Dockerfile comment untouched. Recheck scope before committing. |
| Gates unavailable or baseline failure | Record exact command/exit and missing capability; finish independent authorized work; never lower gates or replace the pinned engine. |
| Release race/outage/partial success | Preserve fail-closed existing posture and explicit operator recovery; no transactional promise or release action. |
