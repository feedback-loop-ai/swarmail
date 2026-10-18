# Delivery repairs

- **Date:** 2026-10-18
- **Status:** design draft; implementation and operator acceptance remain separate.
- **Scope:** run 22, issues #103–#106; Swarmail only.
- **Decision: 0045** — [Delivery proofs and documentation corrections](../../docs/decisions/0045-delivery-repairs.md), **proposed**.

## Feature and rationale

Make the delivery system prove its existing promises and make installation
instructions describe artifacts that actually exist. A green verify verdict must
come from pinned gate implementations, a designed verdict from the committed
artifacts, and a release publication from successful gates including its rate
proof. Documentation must describe the current server, including its limits.

The framing is `.brokkr/state/run22-feature.txt` and the verbatim issue bodies
under `.brokkr/state/epic70/{103,104,105,106}.md`. The requested tasks directory
contains only run-12 framing; those records do not redefine this batch. Both
run-local positions were read. Decision 0044 exists, contrary to one sentence in
the feature file, so this design uses the next free number, 0045.

This is preparation for delivery, not a release. No tag, image, chart or crate
is published by this work. No issue is closed. Runtime mail semantics, store
ordering, extraction, listener behavior and wire validation stay intact.

## Panel rulings

| Dispute | Ruling and reason | Cost of the rejected position |
|---|---|---|
| Gate placement | Adopt simplicity's single canonical implementation inside the pinned bundle, including executable helper inputs. Public script paths become small compatibility entry points. Reject robustness's hash-then-execute of mutable worktree implementations: the framing expressly requires execution of pinned bytes. | Relocation requires explicit workspace/helper roots and a small wrapper-integrity check. Two full copies are forbidden; this avoids the drift robustness correctly warned about. |
| Design selection | Adopt robustness's refusal to guess the artifact identity; reject simplicity's retained directory mtimes. Identify the single spec in the chief's HEAD commit, not a filesystem or commit timestamp. | A separate unrelated commit between chief and check refuses and requires re-entry. No silent fallback to a previously valid spec. |
| Rate gate and policy warning | Adopt both positions' explicit per-prerequisite checks and robustness's missing-input refusal. An unconditional residual fall-through or publish `always()` is unacceptable. | More explicit predicates and fixtures; a missing fact or skipped required gate blocks progress. |
| Version preparation | Adopt simplicity: bump the chart package to 0.1.1, retain the current application version, and document checkout installs. Reject a mandatory Cargo/Chart equality invariant and an application release bump in this batch; 0019 deliberately owns two independent binds. | The future release preparer must align versions if one tag is to publish both. A tag against mismatched versions can still yield the partial-publication outcome already named by 0019. |
| Docker proof | Adopt robustness's real PR-level named-volume smoke, with graceful and hard-kill restarts and context exclusions including WAL sidecars. | Additional Docker build time and a CI-only check; the hermetic seat cannot replace it. |
| Zero TLS timeout | Adopt simplicity's documentation correction. Reject robustness's new parse-time refusal of zero. | The existing accepted-but-unusable zero setting remains possible; docs and help explicitly describe it. A behavior repair needs its own framed acceptance criterion. |
| OpenAPI | Reconcile: extend the existing schema pin and connect the named response/default examples to real-server tests. This adds proof, not a new wire behavior or schema generator. | A small maintained mapping of documented operations to wire examples. |
| Process statements | Preserve signed author commits and operator authority. Record GitHub-signed squash merges separately. Proposed documents may describe compatible implementation, but cannot silently supersede accepted rulings, including 0001–0005. | A conflicting semantic amendment waits for operator acceptance; observed unsigned history is not blanket signing permission. |

Issue #104's literal “packages: write ... only on tags” conflicts with accepted
0018/0019's continuous `:main` publication and the repaired quickstart. Rule in
favor of preserving continuous publication: only actual publishing jobs receive
package write, with the Docker job limited to main or release tags and chart
publication limited to release tags. Verification and PR jobs are read-only.
This is an explicit qualification of that issue's criterion, not a claim that
all writers are tag-only. The cost is a package-writing main job, as today.

## Requirements

### Trusted delivery

The pinned bundle owns coverage, coverage selftest, the publish guard and its
proof, and the coverage scanner's source, manifest and lockfile. No unpinned
worktree script/helper may decide a verify pass. Candidate application source,
chart and workflow remain the objects tested, never replaced with baseline
copies. Verify refuses altered or missing public compatibility entry points
before invoking gates. It resolves pinned resources relative to its dispatched
bundle path and build output relative to the candidate workspace. The existing
by-effect coverage algorithm, cfg coupling and exact 100/100/100 floor remain.

The design sequence ends in a deterministic exec check after chief. Chief emits
`drafted`; only the check emits `designed` or `fail`. Remove any direct
drafted-to-implement bypass. Validate the three nonempty committed artifacts,
acceptance heading, risks heading, checkbox tasks and each declared proposed
decision. Run the dialect prerequisite checker against that same explicit spec.
Restore its missing `common.sh` prerequisite rather than claim the currently
broken scaffold runs. The check fails on ambiguity, dirty artifacts, missing
decision, missing scaffold or invalid structure. It must work in fresh clones
with equal directory mtimes.

Residual review proceeds only with explicitly present `has_security_residual =
false` and a present severity no higher than medium. Security hold, security
residuals, higher severity and missing/malformed facts cannot reach ship. Keep
the protected review phase and four independent routes unchanged. Compile must
emit no policy warning; normal manifest output is allowed.

### CI and release preparation

House rules become the one documented inventory of common gates: fmt, clippy,
tests, bundle compile, exact coverage and its selftest, publish-guard proof,
license check, fail-closed native-tls absence, and the new gate-integrity/design
selftests. CI and verify execute that inventory. Document differences once:
CI additionally owns Helm, Docker smoke, network-backed advisory audit and the
tag-only release rate proof; verify is offline and installs nothing.

Both release publishers require successful ordinary gates and successful
`release-rate-5k` from the same workflow invocation. Chart publication also
requires chart verification; image publication also requires Docker smoke.
Continuous main publication permits rate `skipped` only on an actual main push
with all other required results successful. Failed, cancelled or unexpectedly
skipped prerequisites never publish; neither do PRs or other branches.

Validate tooling JSON types, brokkr SHA (exactly 40 lowercase hex digits),
version and Rust toolchain (stable numeric `major.minor.patch`, no newline or
leading-zero numeric component) before writing any outputs. This deliberately
supports the current stable pin, not prerelease tooling. Validate Cargo/chart
versions with 0019's supported semver rules before workflow output as well.
All file-derived shell inputs travel via environment variables, never expression
interpolation into `run:` source. Build the pinned brokkr SHA with `--locked`
and the explicitly recorded Rust toolchain. Verify checks the launched version
against the pinned record before gates, and commit provenance if exposed. A
matching version alone is not proof of SHA identity.

Split chart verification from tag-only chart publication. Set finite test and
coverage timeouts (initially 30 and 60 minutes). Chart verification gets one
unique temporary directory, an EXIT cleanup, and assertions for all three
public listen addresses and startupProbe in default/full renders. Chart edits
since the last reachable release tag must carry a newer chart package version;
fetch the required history and fail explicitly if the baseline is unavailable.

### Installation and documentation

Current quickstarts use the known existing `:main` image. Current Helm installs
use the checked-out chart with an explicit `image.tag=main` override, so the
installed chart contains 0026's startupProbe/TLS/peer settings. A source-build
instruction replaces the nonexistent crates.io install. Never present the
prepared chart 0.1.1 as already published. Historical release prose gets a dated
correction; executable examples must be current. Mutable `:main` is identified
as mutable; use a verified existing commit tag or local build for reproduction.

The scratch image contains `/data` owned by 65532:65532 and declares `VOLUME
/data`; persistence stays opt-in via `SWARMAIL_DATA_FILE`. A fresh named volume
works without root startup. Document the local-build persistence example until
ordinary post-merge CI has published the repaired image: existing registry bytes
do not acquire the fix retroactively. Exclude `.brokkr/`, `.specify/`, recursive
database files, WAL/SHM companions and SQLite/lock sidecars from build contexts.

The finite #106 correction checklist is:

| Surface | Required correction / source of truth |
|---|---|
| House rules, bundle/policy descriptions, agents | 0009/0020 accepted statuses, exact lines/branches/functions, four review members, real `bundles/self` commands and links; signing/proposed-decision rule above. |
| License policy | Same allowlist in house rules and deny.toml; tracked justification for existing CDLA-Permissive-2.0 webpki-roots trust-store data. Remove unused Zlib/CC-BY-4.0 entries if still unused; no new dependency or unexplained widening. |
| Decisions 0015/0018/0023 | Dated addenda reference 0045: failed QUIT suffix stays marked, HTTP drain is 10 seconds, and 0030's off-worker placement amendments are linked. Preserve historical accepted body text. |
| CLI and SMTP comments | Plaintext SMTP and POP3 have their actual idle bounds; loopback arms Host and Origin checks; zero TLS handshake timeout remains accepted and causes immediate timeout; non-UTF-8 SMTP commands close without reply. |
| README / Dockerfile | 10-second drain, Docker default SIGTERM, stop grace greater than 10 seconds (example 30); `gen-cert` output goes to stdout, not the blanket stderr claim. |
| README / llms.txt | Describe off-worker work from current call sites, including run-21 compat page construction, seed construction, SMTP parsing and mutations; do not preserve the issue's obsolete “only seed and parse” claim. |
| HTTP bodies | Actual REST and MCP 2 MiB limits versus SMTP 50 MiB; document MCP's run-21 JSON-RPC error envelope, not the obsolete plain-text claim. |
| Chart table | Current network-policy peer semantics, TLS, startupProbe and listen defaults/config knobs. |
| Coverage prose | Exact 100/100/100 and evidence location, without stale absolute totals. |
| OpenAPI | Await limit default 100/minimum 0; await count minimum 0 meaning at least one; assert's actual default/inert limit semantics. List query-validation 400s for inbox messages/count/assert/await and compat v1/v2 lists, health 503, applicable Host/Origin 403s, and actual MCP/body-limit response statuses and media types. |

## Acceptance Criteria

1. **AC1 — Pinned execution:** in the actual verify dispatch, an unchanged
   fixture passes the integrity preflight; altered/missing entry points refuse,
   and substituted worktree helper inputs never determine the verdict. Tests
   name refused files and prove canonical pinned coverage/helper and guard
   bytes execute even when a candidate offers a substitute.
2. **AC2 — Design verdict:** the wired final exec step accepts a valid committed
   design and fails missing risks, missing/empty artifacts, missing acceptance,
   missing tasks, uncommitted spec/decision, absent or non-proposed declared
   decision, multiple candidate slugs and missing dialect prerequisites.
   Equal mtimes never select another spec. Chief `drafted` alone cannot advance.
3. **AC3 — Residual boundary:** fixtures for absent facts, security=true and
   severity above medium cannot reach ship; explicit false plus low/medium can.
   Bundle compile succeeds with no policy warning, and every charter uses the
   correct bundle path.
4. **AC4 — Release dependencies:** the workflow and its proof cover green tag,
   failed/cancelled/skipped rate, each failed/skipped ordinary prerequisite,
   green main with skipped rate, PR and non-main branch. Only green tag releases
   and green main continuous builds publish; permissions follow the explicit
   main exception above. No publishing action is executed by the tests.
5. **AC5 — Tooling pin:** valid stable pins pass; missing fields, wrong types,
   malformed SHA/version/toolchain, newline and shell-metacharacter payloads
   refuse before outputs. A mismatched binary refuses before gates. CI uses
   the recorded SHA/toolchain and `--locked`; no file-derived value becomes
   shell source.
6. **AC6 — Common proofs:** both CI and verify wire the common gate inventory,
   including coverage selftest; test/coverage jobs have finite timeouts and
   the documented environment differences are exhaustive for this batch.
7. **AC7 — Chart:** default/full chart verification passes concurrently in
   independent scratch directories; missing any listen address or startupProbe
   makes its fixture fail. Chart changes without a version bump fail, bumped
   changes pass, and unavailable tag history refuses explicitly.
8. **AC8 — Usable installs:** all active Docker/Helm/cargo install examples in
   README, release notes and site refer to available artifacts or an explicit
   local build/checkout. The current chart renders 0026/0030 settings. Nothing
   claims chart 0.1.1 was uploaded; app versions are not bumped for this task.
9. **AC9 — Persistence/context:** a PR smoke builds locally without pushing,
   runs as uid 65532 on a fresh named volume, receives mail over real SMTP,
   observes acceptance and queries it, then retrieves accepted mail after both
   SIGTERM restart and a separate hard-kill restart. A build-context probe with
   seeded excluded sentinel files contains no state directory or database/
   sidecar files. Cleanup removes only that test's containers and volume.
10. **AC10 — Documentation contract:** every row in the correction checklist is
    reconciled to current code; `serve --help` pins idle bounds and Host/Origin;
    OpenAPI tests pin defaults/minimums/statuses and real-server examples prove
    the named statuses and zero/default pagination semantics. No runtime TLS
    timeout or other wire behavior changes to make a sentence true.
11. **AC11 — History and policy:** 0045 remains proposed, accepted decisions
    retain historical text plus dated correction pointers, status/index links
    are consistent, and house rules/deny.toml agree using tracked rationale.
12. **AC12 — Delivery:** all house gates, coverage selftest, publish proof and
    new boundary tests pass before implementation success. No coverage floor
    reduction, exclusion, new production dependency, publication or issue
    closure is part of this batch. The chief commits only this spec, plan,
    tasks and 0045; implementation follows on a branch in a later phase.

## Explicit residuals

0019's check-then-push race and independent manifest binds remain. The
version-only brokkr check cannot attest source provenance without binary support.
The gate pin does not attest every compiler/configuration/cache input (including
`.cargo/config.toml`, toolchain configuration and dependency cache); these remain
reviewed inputs, not a claim of a complete supply-chain proof. Coverage retains
0035's named technical limits. Structural design checks cannot prove semantic
truth. Existing root-owned volumes may need operator ownership repair; no root
entrypoint is added. No new release registry probe or network service is added.
