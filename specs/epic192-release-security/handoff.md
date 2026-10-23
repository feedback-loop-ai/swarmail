# Lane A implementation handoff — 2026-10-23

Implementation commit: `26f6430c2c1799cbee119a5da3a8322386580d5a`
(unsigned, `N`), on `epic192/lane-a`, after chief design
`877728dc6d9eacba16dfdcaad0ea5c2e3fe8afe0`. The journal-supplied design
result was `designed`, strategy `engine`. The subsequent unsigned documentation commit contains this handoff and task
disposition; its identity is reported by the final Git log. No release or epic
completion is claimed. The implementation result is **broken**, because the required real
Helm/image/release evidence below remains unavailable; this is neither a
frozen-guarantee refusal nor a scope reclassification.

## Finding-by-finding disposition

Evidence names below are relative to the ignored run directory
`.forge/epic192/implementation/` at the repository root. They are real local
receipts, not CI/reviewer/journal receipts.

| Finding / AC | Disposition and evidence |
|---|---|
| #169 1.1 / AC01 | CSP comment corrected for inline resources, SSE, same-origin/data images, same-origin image GETs and clicked navigation. `audit.json` proves every non-doc-comment byte of ui.rs is unchanged. Full Rust/protocol suite passes. |
| #169 1.2 / AC02 | CI, current and historical chart reads use the duplicate-aware safe YAML owner. Actual CI bodies cover quoted/plain scalars, comments, duplicate version/appVersion in both orders, missing/type/malformed values, aliases, merges and multiple documents. `final-chart-selftest.log`; `ci-reader-duplicate-red-restored-green.json` shows the actual baseline sed/head body admits a duplicate while the restored current body refuses with no output. Actual Helm equivalence is **outstanding**: Helm is absent. |
| #169 1.3 / AC03 | Basic and bearer headers use stdin; helper arguments also observed. Invalid tokens refuse before manifests. `final-publish.log`, `mutations.json` and individual red/restored logs cover Basic/bearer/helper argv regressions and missing headers. Only synthetic credentials used. |
| #169 1.4 / AC04 | All 20 matching historical spec files redacted, including split-line paths. `audit.json` proves exact path-only transformations; Git grep with the home-prefix regex exits exactly 1, empty stdout/stderr. |
| #185 17.1 / AC05 | Guard, chart reader and real CI version bodies reject leading zeroes, empty identifiers and malformed versions; admit the specified valid prereleases. Build metadata and exact-ref rules retained. `final-chart-selftest.log`, `final-publish.log`, loose-semver red/restored receipts. |
| #185 17.2 / AC06 | Dated 0019 rerun correction appended. `audit.json` verifies the accepted body is an identical prefix. Deferred #184 must integrate this commit before validating its queued adoption patch. |
| #185 17.3 / AC07 | Parsed active workflow steps pin order, supported conditions, success propagation, prerequisites, both image forms, continuous tags and failure handling. `final-publish.log`: 51 named serialized workflow mutations rejected, with hashes/diagnostics and restored checks. |
| #185 17.4 / AC08 | Real HTTP fixture enforces exact Basic, bearer, Accept, service, expected repository scope and manifest paths. Positive absence/presence requests succeed. Missing Basic/bearer/Accept and wrong scope mutations fail, restored proofs pass. |
| #185 17.5 / AC09 | Dated 0019 polling correction appended; readiness uses a predicate/deadline with sleeps between polls. No fixed-sleep correctness dependency added. |
| #185 17.6 / AC10 | Case 12b invokes the real guard for bare=404/v=200 and checks both paths and exact stale-v refusal. Redundant 12d removed. Removing the v probe fails the regression. |
| #186 18.1 / AC11 | Actual metadata/coherence/history evaluators cover the refusal inventory below, including malformed appVersion, absent push before and malformed baseline. Candidate selftest/history pass; repaired-arm mutations fail and restoration passes. Helm render/refusal execution remains **outstanding**. |
| #186 18.2 / AC12 | Separate token, manifest-transport, HTTP-500 and HTTP-000 needles. curl 8.18.0 rejects the raw 000 response with exit 1 before classification. The 000 classifier proof changes only successful real authenticated 404 status output, preserving request/headers/stdin/errors; it is **classification-only**, not a real successful HTTP-000 or GHCR receipt. `final-publish.log`; independent real manifest transport failure and mutation receipts. |
| #186 18.3 / AC13 | Child holds a bound, non-listening socket throughout refusal attempts; lifetime checked around attempts; cleanup joins child. Listener binds port 0. Releasing the holder makes the ownership regression fail. No bind-release-reuse or elapsed correctness ceiling remains. |
| #186 18.4 / AC14 | Root license/NOTICE anchors; actual Cargo listing and archive verification pass. Private in-tree cache decoys stay out. Unanchored mutation makes real Cargo list the decoys: Cargo exits 0, the owning package-contract assertion exits 1; restored assertion exits 0. `packaging.json`, `package-contract-red-restored-green.json`, `actual-ci-package.json`, `root-package-list.log`, `root-package.log`. |
| #186 R4.1 / AC15 | Application SPDX preserved; packaged NOTICE names WHATWG copyright, source, CC-BY-4.0 URL, date/hash, unmodified snapshot and derived table transformation. README License links it. Actual fixture and committed-root archives match all license/data/notice bytes. `package-archive-hashes.json`; `final-deny.log`: licenses OK. |
| #186 R4.1 / AC16 | Dockerfile copies identical readable NOTICE to `/NOTICE`; CI builds/extracts/compares it and checks mode 0444, cleaning container/volume/image. Actual image extraction and its removal mutation are **outstanding**: `docker-info.log` exits 1, daemon socket absent. A COPY instruction is not counted as proof. H's first comment is unchanged. |
| #186 18.5 / AC17 | Exact receipt checklist is in spec.md. Real GHCR 404-then-publish receipt remains **outstanding**, with no publication authorized or attempted. |
| AC18 | All nine house-rule gates and local configured extra proofs pass; both proof origins distinguished below. Missing Helm/image/release and CI evidence are not waived. No engine verify/review/ship result is invented. |

The initial unanchored-package attempt stopped at Git's dirty check and is
**not** counted as a negative proof. The corrected mutation uses
`--allow-dirty` only in its private edited fixture, and requires actual cache
paths in Cargo's output. A nested package boundary can hide those decoys, so
the final fixture deliberately contains license files without a nested Cargo
manifest. Root committed `cargo package --list --locked` and
`cargo package --locked` both exit 0 without that flag. The CI packaging step
executes the same real-listing/mutation/archive contract; its actual run body
was extracted and executed locally (`actual-ci-package.json`, exit 0).

Changed implementation files are the CI workflow, Cargo includes, root NOTICE,
README License, Dockerfile NOTICE copy, CSP comment, the two canonical publish
scripts, chart verifier, appended 0019 errata, and the 20 privacy-only spec files.
`audit.json` lists every exact changed path and checks ownership. This handoff
and tasks.md are the only subsequent tracked documentation changes.

## Proof origins, commands and identity

The immutable source recipe is `../../baseline/bundles/codex-fast` relative
to the candidate root; its inherited proofs are
`../../baseline/bundles/self/scripts`. The workspace namespace does not mount
that external directory. Those resources were read without modifying the
source, copied byte-for-byte into the ignored
`.forge/epic192/implementation/immutable-proof-copy/`, hash-verified and made
read-only. `baseline-digests.json` records all 16 resource digests;
`baseline-gates.json` records exact executed copy paths/commands/exits/logs.
This is a replay of pinned bytes, not a claimed engine verify dispatch.

The launcher-installed `.forge/bin/brokkr` and external launched engine both
hash to `e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`,
matching tooling.json. No version-equal substitute or installation was used.
Both bundle compilations are exit 0 without policy-sweep warnings.

| Candidate input/proof | SHA-256 |
|---|---|
| bundles/self/scripts/publish-guard.sh | `183a9c8bb15c6d41ea4a0a887404cb1aebca1cf7001f430e2e329355ff27a764` |
| bundles/self/scripts/publish-guard-test.sh | `f3fcf3813c3127267ecc4537e0e3d5325bda2d52f6813cd68fa88d14c3fb25f2` |
| scripts/chart-verify.sh | `5ff500f5031ed05bb1810013fb189a868cfc4da5b10467ee32f2a41ed8cba62e` |
| .github/workflows/ci.yml | `596544fe44a76c771d2fa3fd4900923af5002c31cabcbf231f5c27cb7cc02a8c` |

`gates.json` records exact initial commands/exits/logs. Every house command
exits 0: fmt, locked all-target Clippy, locked workspace tests, pinned bundle
compile, coverage, coverage selftest, publish proof, keyless seal proof and
cargo-deny licenses. Configured design-check selftest, delivery tamper proof,
recipe structural tests and the expected-no-match native-tls proof also pass.
`final-recipe.log` proves 6 constitution and 12 recipe tests after workflow
changes. The native-tls command uses `cargo tree --locked -i native-tls` and
accepts only its nonzero no-match diagnostic, not an arbitrary Cargo error.

Baseline coverage, coverage selftest, design selftest, delivery tamper,
publish, seal and shim integrity all exit 0 (`baseline-gates.json`). Baseline
coverage is exact: **12,446/12,446 lines; 724/724 branches; 1,379/1,379
functions**. Full lcov and summary remain in
`target/coverage/run.4xZdNl/`; candidate-origin results are in
`target/coverage/run.qm4ixU/`. Public shims and their inventory are unchanged.

`final-proofs.json` binds final candidate publish/chart selftest/history,
baseline publish, bundle compile, deny and both actual root Cargo package
commands to implementation commit 26f6430; all exit 0. Local history uses a
synthetic PR event with the real lane base 168ac299e0fe, not a fabricated CI
receipt. `mutations.json` contains 21 named disposable-script/workflow
red/restored-green records with source/mutant hashes and logs, plus the
narrow duplicate-only actual CI receipt named above. Candidate and immutable
publish proof passes are separate claims.

## Chart refusal inventory

| Owner | Executed cases / remaining evidence |
|---|---|
| Metadata | Unique string mapping keys; duplicate version/appVersion and reversed/quoted duplicates; required scalar release versions; missing/type/malformed fields; root shape, aliases/anchors, merges, multiple/malformed documents, missing file. Actual CI output stays empty on refusal. |
| Coherence | App/Cargo mismatch, stale image annotation, malformed chart version and malformed appVersion; diagnostic needles checked. |
| History | PR base absent/missing; push before absent/missing; unknown event; new/rewritten branch without merge-base; absent earlier release tag; malformed current/baseline metadata; equal/decreased/prerelease chart changes; positive unchanged/new-chart/bumped branch/tag cases. Real private Git repositories. |
| CLI | Unknown mode exits 2 with usage; local verify exits 1 naming missing Helm. |
| Helm/render contract | Existing ServiceMonitor-without-CRD and replica refusals plus rendered assertions are wired but **not run here**. Helm equivalence/render/refusal mutation proof must run in an equipped environment. |

## Preserved rulings and integration obligations

Audit proves UI executable bytes, accepted 0019 body, Dockerfile except NOTICE
copy (including H's citation), README prefix and Cargo except includes are
unchanged. All runtime Store/SMTP/await/prune/extraction/single-writer code is
untouched. Preserve 0001–0006, 0019, 0020/0035, 0045/0049/0050, unsigned 0056
and 0057's protected review/security hold. No new ADR; reservation 0058 unused.
No version, dependency, license allowlist, public shim, recipe, effort/model,
baseline or active/historical journal changes. No push, merge, tag, publish,
issue closure/comment or nested run.

Deferred #184 must integrate **26f6430's 0019 errata** before validating source
hashes/current applicability of its queued accepted-0051 adoption. The supplied
operator acceptance is settled and was not requested again. This lane does
not adopt that patch or edit the decision index. H's independent first-comment
citation fix can integrate without replacing this lane's NOTICE copy.

Next required evidence: installed Helm coherence/render/equivalence and
negative proofs; real built-image NOTICE extraction/removal/restoration;
engine-owned deterministic verification and all four fixed review lenses;
CI chart/history, Docker persistence/context/volume smoke, network RustSec,
tag-only release-rate-5k and real port-80 bridge results; then the operator's
explicitly authorized release and linked GHCR receipt against spec.md's
checklist. No local fixture closes the real release AC. The supplied historical
snapshot was not mutated or used as an active journal; no historical receipt
or supersede is inferred. Only the operator seals/lands after remaining gates
and reviews are satisfied.
