# Epic192 release-security continuation

Created: 2026-10-23. Continuation clarification: 2026-10-23.
Branch: `epic192/lane-a-continuation`. Status: continuation design draft.
Slug: `epic192-release-security` (retain the existing feature identity).
Stories: [#169](https://github.com/feedback-loop-ai/swarmail/issues/169),
[#185](https://github.com/feedback-loop-ai/swarmail/issues/185),
[#186](https://github.com/feedback-loop-ai/swarmail/issues/186), including
[comment R4.1](https://github.com/feedback-loop-ai/swarmail/issues/186#issuecomment-6285339357).

## Feature and rationale

Finish the compatible release/security repairs and their missing proofs,
retaining implementation `26f6430c2c1799cbee119a5da3a8322386580d5a` from clean
continuation base `4970a5125d0750613cf856b97e5e0720cc13b2e6`. Existing code may
need no change. Credential confidentiality, actual workflow guard ordering,
metadata interpretation, attribution and proof integrity still require the
full design and four-provider review route. Small diff size does not reduce
that risk. Compatible delivery does not close the real release obligation.

The prior run `epic192-first-wave-lane-a-releas-bb00db91` stopped
`IMPL-BROKEN-TWICE` at event 387 before actual verify, independent review or
ship. Its original base `168ac299e0fe16ecdd8fa0e0218cb7bc4e0a261c`, branch
`epic192/lane-a`, accepted design and checked tasks are historical evidence.
The exact original findings, AC dispositions and retry observations remain in
[handoff.md](handoff.md), unchanged by this design. The original AC01–AC18 and
release checklist below are retained verbatim; current obligations refine
proof provenance and completion without waiving any criterion.

Current intake and complete authoritative issue bodies are preserved in
`.brokkr/state/tasks/epic192-lane-a-continue-compatib-70aa2893.md`; both current
positions are `.brokkr/state/design/positions/simplicity.md` and
`.brokkr/state/design/positions/robustness.md`. These ignored inputs remain
uncommitted. Preserve `.forge/epic192/implementation/audit.json`, its exact
20 privacy-only historical spec transformations and every recorded script /
workflow mutation receipt. New evidence supplements them, never overwrites
failure history or claims an earlier engine phase ran.

## Authority and scope

This continuation restores and proves existing contracts; it changes no
frozen guarantee, public runtime contract or accepted ruling. No new ADR is
declared. [0019](../../docs/decisions/0019-release-tag-integrity.md) owns strict
SemVer, exact-ref binding, bounded probes and fail-closed registry state.
[0006](../../docs/decisions/0006-dual-license.md) preserves application SPDX;
[0049 §2](../../docs/decisions/0049-extraction-compat.md) supplies WHATWG
provenance. [0045](../../docs/decisions/0045-delivery-repairs.md),
[0050](../../docs/decisions/0050-delivery-gates.md) and
[0057](../../docs/decisions/0057-codex-delivery-recipes.md) require actual
proof dispatch, deterministic design identity and protected independent review.
If a discovered repair instead changes a ruling, revise this design and draft
the next free decision as proposed; contradictory implementation awaits the
operator's acceptance. No decision number is consumed by this design.

The operator's 2026-10-23 acceptance of corrected 0051, including successful
joint-tag equality and independent preparation, is settled. The checked-in
[0051 record](../../docs/decisions/0051-release-records.md) awaits adoption by
deferred #184 alone, after integrating A's `26f6430` additive 0019 errata and
validating source hashes/applicability. A does not apply that patch, edit the
index or seek acceptance again. Preserve accepted historical decision bodies.

Preserve 0001–0005, single-writer protection and all runtime policies:
synchronous insert/SMTP 250 queryability, subscribe-before-check awaits,
oldest-only pruning, per-test isolation and ingest extraction. Keep exact
100% measured coverage, all gates and hard security holds. No Rust behavior,
dependency/version/allowlist, baseline/pin/shim/hash, recipe or journal changes;
no broader formatting or other candidate edits. H owns the Dockerfile's first
criterion comment; A owns only its NOTICE packaging. Work seats commit
unsigned and never push. The explicit task-specific CI-green GitHub-signed
squash landing belongs to the coordinator without a separate operator seal;
this is not a general amendment of 0056 or authority for any seat to merge.
No release, tag, publication, registry deletion or version bump is authorized.

## Panel synthesis and rulings — 2026-10-23

The positions agree on the architecture and required proof burden. The rulings
below settle their competing priorities without inventing a provider dispute.

| Question | Adopt / reject / reconcile, with the cost of the losing option |
|---|---|
| Restart or continue? | Adopt simplicity's existing feature directory, script owners and implementation. Adopt robustness's fresh proof requirements. Reject restarting implementation or treating old green receipts as current phase results. Cost: historical and current dispositions must coexist explicitly, and unchanged code still incurs current verification time. |
| Minimal code versus adversarial evidence | Robustness's defect-sensitive proof standard governs whenever a smaller test cannot detect the defect. Simplicity's minimal machinery governs placement: existing proofs plus ignored private drivers. Reject unqualified selftest green, arbitrary nonzero mutants and a new evidence service. Cost: more targeted executions and receipt review, with no new product framework. |
| Metadata ownership | Retain one duplicate-aware PyYAML owner in `scripts/chart-verify.sh` and independently pinned guard validation. Require real Helm/package and actual CI-caller equivalence for supported forms. Reject a second parser, yq and first-key fallback. Cost: explicit Python/PyYAML and Helm prerequisites, small validation duplication, and refusal of unsupported legal YAML. |
| Workflow semantics | Retain the repository-specific checker, actual relevant shell-body executions and all workflow mutations. Reject reference-count-only assurance and a general Actions interpreter. Cost: unfamiliar workflow shapes refuse and require deliberate proof maintenance. |
| Missing image capability | Adopt both positions' source-bound coordinator build/extraction/removal/restoration receipts, with host origin explicit. Reject Dockerfile text as proof and socket/privilege/daemon workarounds. Cost: coordination and a concrete open AC16 until actual evidence arrives; host evidence is not CI smoke. |
| Attribution and CSP | Retain one packaged/image NOTICE, application SPDX and comment-only CSP truth. Reject relicensing, a second notice system and runtime tightening. Cost: notice distribution bytes and the disclosed clicked-navigation/same-origin-image limitations remain. |
| Runtime robustness | Adopt robustness's invariant/risk analysis as preservation boundaries; adopt simplicity's rejection of new queues, retries, durable outbox or release state machine. Cost: SMTP ambiguity after commit, bounded webhook losses, legitimate oldest pruning and current transport/resource limits remain existing behavior. None is redefined by release proof work. |
| Release completion | Both positions require AC17 explicitly external pending while compatible work continues. Reject both global blocking solely on an unauthorized release and fabricated closure. Cost: compatible ship cannot close finding 18.5, story #186 or epic #192. Unknown host/CI/security facts are still not waived. |

## Current evidence and continuation requirements

The coordinator's `.forge/epic192/continuation/helm-install-receipt.json`
records official Helm v3.16.3, archive checksum verification and binary digest
`6a1dffedcf78a687aedc71a918cff0af8f0988184488dddb3615e24abc4e7f2b`.
`coordinator-chart-verify.json` in the same directory binds base `4970a512`
and chart-script digest
`5ff500f5031ed05bb1810013fb189a868cfc4da5b10467ee32f2a41ed8cba62e`
to actual `verify` exit 0. It proves that execution's render/coherence/existing
refusal checks only. It supersedes the old missing-Helm observation for that
execution, not all equivalence/mutation obligations or current engine verify.
Host Docker 29.9.2 availability is supplied context, not an actual image proof.

| Requirement / original AC | Testable continuation requirement and present disposition |
|---|---|
| C01 / AC01–AC18 provenance | Preserve every original finding and AC, historical handoff/audit/receipts and complete source issue bodies. Each new disposition records current candidate commit/tree, exact commands/exits/logs, executable/helper/tool identities and proof origin. Distinguish inherited, current candidate, coordinator host, engine, CI-only and external release evidence. All current tasks start unchecked. |
| C02 / AC02, AC05, AC11 Helm | Execute installed pinned Helm comparisons for both version and appVersion, plain/single/double quotes, comments and whitespace; actual extracted CI bodies, metadata owner and actual Helm/package metadata agree. Include current and real-Git historical fixtures. Duplicates in either order/quoted form, malformed/missing/type values, aliases/merges and multiple documents refuse before usable output. Reintroduce first-key selection and disable repaired refusal/render arms in private copies: owning assertions fail for the defect, restored bytes pass. Prove Helm-dependent branches ran; an optional branch skipped on PATH cannot satisfy this. Remaining equivalence/mutation evidence is pending. |
| C03 / AC03, AC05, AC07, AC08, AC10, AC12, AC13 preservation | Rerun actual candidate guard/proof and all recorded defect-sensitive controls: argv secrets; dropped Basic/Bearer/Accept; loose SemVer; missing v probe; released refusal socket; distinct unknown-state arms; and guard removal/reorder/skip/softening, ignored shell failures, prerequisite loss and failed/skipped/cancelled Docker propagation. Preserve synthetic-only secrets, real HTTP, exact diagnostics, complete red/restored-green identities and existing 000 reachability evidence. Successful-request status substitution remains classification-only, paired with real manifest transport failure. Inherited successes are not current results. |
| C04 / AC01, AC04, AC06, AC09, AC14, AC15 scope/package | Revalidate original exact audit (UI executable bytes, H comment, accepted 0019 prefix, 20 path-only transformations, unchanged versions/dependencies/allowlist). Home-prefix grep exits exactly 1 with empty output. Actual locked Cargo listing/archive and cache-decoy anchor red/restored controls retain exact root licenses/data/NOTICE. Verify WHATWG hash/transformation, README License and deny. Preserve original audit bytes even if a separate continuation audit is needed for these design edits. |
| C05 / AC16 image | Obtain actual build-bound extraction of `/NOTICE`, readable with mode 0444, byte-identical to root and actual crate archive NOTICE. Receipt binds candidate/tree, dirty differences, complete source/build-context identity, Dockerfile/NOTICE hashes, tool/build command and image ID/digest. Removing only NOTICE COPY in a disposable build fails the attribution assertion; restored source rebuild passes. Use run-unique fixtures and prove cleanup of containers/anonymous volumes/images. No denied-capability result counts. Actual evidence is pending; request the precise host proof through the coordinator's authorized route. |
| C06 / AC17 external release | **EXPLICIT EXTERNAL PENDING**: #186 finding 18.5 needs the next authorized real guarded GHCR 404-then-publish receipt against the unchanged checklist below. Preparation is complete historically, collection is not. Carry this exact open item in continuation disposition, release checklist and final report; do not close 18.5/story #186/epic #192 or encode it as passed/waived. |
| C07 / AC18 delivery | Require current complete hermetic verification, separate repaired candidate proof execution, actual four fixed independent reviews and successful hermetic ship for compatible continuation. Qwen Max correctness, Opus security, official DeepSeek Flash guarantees and DashScope GLM 5.3 tests have no fallback. Only actual engine records prove phases. Preserve worst-verdict/security hold and unknown-fact refusals. Record each CI-only obligation separately; neither inherited local gates nor host receipts certify it. All current phases remain pending at design. |

## Acceptance Criteria

Each ID is normative and maps to a paired task/test. Implementation must record
pass, fail or outstanding with evidence; unchecked external obligations are
not waived by a green design validator.

| ID / finding | Testable acceptance |
|---|---|
| AC01 — #169 1.1 | CSP comment describes restrictions on resource loads, same-origin SSE, allowed same-origin/data images, allowed same-origin image GETs and the limitation on clicked navigation. Remove the absolute no-beacon claim. Compare the CSP string and preview behavior to the parent: only comment bytes change. |
| AC02 — #169 1.2 | CI and chart coherence/history consume one YAML metadata reader owned by `scripts/chart-verify.sh`. Require a single mapping, unique string metadata keys and scalar string version/appVersion; reject missing, malformed, duplicate (including quoted keys and reversed order), non-string and ambiguous metadata before any output is appended. Positive plain/single/double-quoted values, comments and whitespace produce the same values as actual Helm/package metadata. Execute the actual CI chartver run body with a private GITHUB_OUTPUT; restoring first-key selection makes it fail. Test both version and appVersion, current and historical manifests. |
| AC03 — #169 1.3 | Basic credential bytes and bearer headers reach curl over stdin; neither raw secret, encoded Basic credential nor bearer occurs in curl or helper argv, URLs, trace output or retained evidence. An observer delegates to real curl and records argv only using distinct synthetic secrets. Malformed, empty/non-string and header-control token responses refuse without a manifest request. Reverting either secret to argv fails the proof. |
| AC04 — #169 1.4 | `git grep -n -e '[/]home[/]' -- specs/` has no output and exit 1, not an arbitrary Git error. Redact every actual occurrence, including split-line path fragments, preserving all other historical content. Do not restrict the repair to the issue's old count of 18 files. |
| AC05 — #185 17.1 | Actual guard rejects `01.0.0`, `1.0.0-01`, `1.0.0-.a`, `1.0.0-a.`, `1.0.0-a..b`, incomplete cores and invalid characters; accepts `0.0.0`, `1.0.0-0`, `1.0.0-alpha.1`, `1.0.0-alpha-01`. Build metadata remains refused and exact v-prefixed ref binding remains. Chart and workflow version consumers pass the same language fixtures. Reintroducing the loose validator makes regressions fail. |
| AC06 — #185 17.2 | Append dated 0019 correction: reruns use their original workflow revision; only revisions containing the guard enforce it. Preserve all accepted historical body bytes. Supply the errata commit to deferred #184 before its adoption patch. |
| AC07 — #185 17.3 | Real workflow checker requires Docker release guard before release push; relevant job/guard/publish steps cannot soften or bypass failures through continue-on-error, incompatible if/success conditions or shell ignored failures. Retain chart image preflight and absence guard before helm push, Docker success propagation and ordinary/rate/chart/smoke dependencies. Mutations moving/removing/skipping/softening guards or dropping prerequisites are each rejected by the repaired checker; restored workflow passes. Continuous main/sha publication retains its accepted behavior. |
| AC08 — #185 17.4 | Real local HTTP fixture enforces exact synthetic Basic on exchange, exact distinct Bearer plus full OCI/Docker Accept list on manifests, correct service/scope/repository/tag paths. Positive 404 absence and success+200 presence requests work. Dropping Basic, Bearer or Accept independently turns the owning proof red. |
| AC09 — #185 17.5 | Append dated 0019 correction distinguishing readiness polling with a predicate/deadline and intervening sleeps from sleep-as-synchronization. Preserve historical body and no correctness dependence on fixed sleep duration. |
| AC10 — #185 17.6 | Case 12b invokes the guard with bare=404 and v-form=200, asserts the exact stale-v refusal and both requested paths. Consolidate redundant 12d evidence. Deleting the second probe fails the case; both-absent success and bare-only chart probing remain tested. |
| AC11 — #186 18.1 | Inventory every current/new chart refusal and drive it through the owning coherence/history/metadata evaluator with exact diagnostics and private real Git fixtures. Include malformed appVersion, push without before, malformed baseline version, unknown event/mode, missing history, tag/new/rewritten branch failures, metadata refusals and existing rendered-contract refusals where applicable. Disabling each repaired check fails its owning regression; restoration passes. |
| AC12 — #186 18.2 | Separate named proofs for token exchange failure, manifest transport failure after successful exchange, HTTP 500 and successful-curl HTTP 000 classification (plus existing admission/refusal poles). No generic needle can certify the wrong arm. Record installed-curl 000 reachability experiment; if unreachable use the bounded classification-only control specified in the plan, with independent real transport proof. Mutating each classification makes its owning proof fail. |
| AC13 — #186 18.3 | Refused connection uses a bound, non-listening loopback socket owned for the entire attempt, with explicit readiness and teardown; never release then reuse a chosen port. HTTP server binds port 0. Prove holder lifetime covers the guard attempt and that the resulting refusal names token/transport stage correctly. No scheduler-dependent elapsed ceiling or shared fixture state between runs. |
| AC14 — #186 18.4 | Cargo includes root-anchored `/LICENSE-MIT`, `/LICENSE-APACHE`, `/NOTICE`; correct the basename-uniqueness comment. Actual `cargo package --list --locked` and archive inspection retain intended licenses/data/notice and exclude nested cache decoys. Run with a private in-tree `.cargo-home` containing matching license basenames; restoring unanchored patterns makes the defect-sensitive package proof red, restoration green. No dependency, allowlist or version change. |
| AC15 — #186 R4.1 metadata + README | Keep application SPDX `MIT OR Apache-2.0` using the issue's packaged-notice alternative. Root NOTICE identifies the WHATWG snapshot and derived NAMED table, copyright, CC-BY-4.0 license URL, original source URL, recorded date/hash and transformation. README License has the required sentence and NOTICE link. Inspect the actual crate archive for accurate notice, data and license bytes; `cargo deny check licenses` stays green. |
| AC16 — #186 R4.1 image | Final scratch image carries identical readable NOTICE bytes at `/NOTICE`, beside `/swarmail`. Inspect the filesystem of an actually built image and compare bytes/hash, without needing an image shell. Preserve H's first-comment citation and the image's runtime configuration. An unavailable real image proof remains outstanding; a COPY line is not proof. |
| AC17 — #186 18.5 | Prepare the receipt checklist below, then link the next operator-authorized real GHCR release's 404-then-publish receipt. Local fixtures do not close this AC. No publication/tag/push is authorized by this spec. |
| AC18 — delivery | Execute all house-rule gates and configured extra proofs with the launcher-pinned engine/toolchain. Separately run repaired candidate proofs and the immutable baseline chain; record commands, exits, exact script paths/digests, candidate identity and mutation-red/restored-green receipts. Never use baseline success to certify candidate repairs. Preserve shims, shim hashes, recipes and review/security holds; failures outside ownership are reported, not repaired here. |

## Release receipt checklist — external, not executed by this lane

The operator's eventual receipt must link all of the following. Preparation of
this checklist is local deliverable; actual receipt collection remains open.

- Approved release commit and signed triggering ref, Cargo version, Chart
  version and appVersion deliberately aligned for that joint tag run;
  workflow revision and run URL, plus quality/test/coverage/deny/audit/compile,
  chart, smoke and same-run release-rate-5k job results/URLs.
- Exact registries and tags: `ghcr.io/feedback-loop-ai/swarmail:<version>`
  and `:v<version>`, plus `ghcr.io/feedback-loop-ai/charts/swarmail:<version>`.
- Timestamped, credential-redacted successful token exchanges and pre-push
  404 results for both image forms and the bare chart form; record ordering
  against the relevant publish operations and the guard/script commit.
- Successful image publication and image job result; subsequent chart image
  preflight 200 for the bare appVersion; chart absence preflight then successful
  chart push. Link logs and immutable artifact/digest identities.
- Post-publication 200/readback evidence for exact tag forms, image/chart
  digests and packaged metadata/notice identities. This is receipt evidence,
  not a new automated post-push enforcement gate or immutability promise.
- Any partial failure, rerun or manual recovery recorded honestly. A partial
  release never becomes a fabricated complete receipt.

## Completion boundaries

The chief commits only this spec, plan and tasks and returns `drafted`; the
pinned deterministic final check alone rules design. This commit establishes
no implementation, full verification, independent review, ship, CI or release
result. The current ordered tasks are distinct from historical checked tasks.

All compatible work and required current engine phases remain necessary.
Missing host or CI proof must be named concretely and resolved through its
owner; no unknown safety fact takes a clean or safe-residual shortcut. AC17
alone does not stop progress on compatible work, and stays **EXPLICIT EXTERNAL
PENDING** through compatible ship until a separately authorized release supplies
it. Existing check/push races, registry outage refusal and partial publication
remain accepted costs, with no automatic recovery or immutability promise.
Report current phase receipts, every finding's disposition and the deferred
#184 errata-first integration dependency without fabricating prior delivery.
