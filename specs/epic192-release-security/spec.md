# Epic192 release-security

Created: 2026-10-23. Branch: `epic192/lane-a`. Status: design draft.
Stories: [#169](https://github.com/feedback-loop-ai/swarmail/issues/169),
[#185](https://github.com/feedback-loop-ai/swarmail/issues/185),
[#186](https://github.com/feedback-loop-ai/swarmail/issues/186), including
[comment R4.1](https://github.com/feedback-loop-ai/swarmail/issues/186#issuecomment-6285339357).

## Feature and rationale

Make the existing release safeguards enforce what their documentation says,
prove the refusals that their tests currently miss, and distribute the WHATWG
attribution to crate and image consumers. Correct the UI security comment and
remove operator-local home paths from historical specifications. Release
identity, credential confidentiality and test integrity are gate/security work;
triage selected the full `engine` route. This stream does not close epic #192.

The input evidence is the run's framing and both positions at
`.brokkr/state/tasks/epic192-first-wave-lane-a-releas-bb00db91.md` and
`.brokkr/state/design/positions/{simplicity,robustness}.md`. Those are ignored
run-local evidence, not delivery artifacts. The finding map below retains all
source findings even where the issue's old line numbers or file count differ.
At inspected base `168ac299e0fe16ecdd8fa0e0218cb7bc4e0a261c`, chart readers
use sed/head, the guard admits malformed SemVer and supplies secrets in curl
argv, case 12b is a no-op, and intake reports 20 tracked specs with home paths.

## Authority and scope

This is a compatible repair, not a new semantic ruling. Strict SemVer,
exact-ref binding, bounded fail-closed probes and proof of publish ordering
restore [0019](../../docs/decisions/0019-release-tag-integrity.md).
Application licensing remains [0006](../../docs/decisions/0006-dual-license.md);
consumer attribution implements
[0049 §2](../../docs/decisions/0049-extraction-compat.md).
[0045](../../docs/decisions/0045-delivery-repairs.md) and
[0050](../../docs/decisions/0050-delivery-gates.md) govern proof ownership,
design identity and unknown residuals. No new ADR is declared; reserved 0058
is unused. If implementation requires a new policy or contradicts an accepted
ruling, revise this design and draft only proposed 0058 before that work;
a proposal supplies no acceptance authority.

The supplied operator ruling of 2026-10-23 explicitly accepts corrected 0051,
including successful joint-tag equality and independent package preparation.
The checked-in [0051 record](../../docs/decisions/0051-release-records.md) still
awaits that adoption. Deferred #184 alone validates/applies the queued adoption
patch after this lane's 0019 errata integration. A must neither apply it nor
ask for acceptance again. Current Cargo/chart release versions stay unchanged.

Preserve 0001–0005: synchronous insert and SMTP 250 queryability,
subscribe-before-check awaits, oldest-only pruning, per-test isolation and
ingest extraction. Preserve single-writer protection, exact 100% measured
coverage and security holds. No Rust behavior changes, registry transaction
layer, retry/backoff, override, tag deletion, publication, push, merge or issue
comment/closure is authorized. UI scope is its CSP comment only.

## Panel synthesis and rulings

| Question | Ruling and explicit cost |
|---|---|
| How much architecture? | Adopt simplicity's existing script owners and one NOTICE. Adopt robustness's executable adversarial proofs within those owners. Reject a new release framework, HTTP client, ledger or generic workflow interpreter. Cost: small validation duplication at separately pinned trust boundaries and the existing concurrent check/push race remain. |
| Chart parsing | Choose simplicity's existing PyYAML facility in the chart verifier, with safe parsing and duplicate-key refusal, and robustness's Helm equivalence and actual-caller tests. Reject a second lexical reader and the conservative handwritten YAML alternative for this design. Cost: Python/PyYAML availability becomes an explicit prerequisite at the metadata call site; missing tooling refuses and must be reported, not silently replaced or downloaded. |
| YAML ambiguity | Robustness wins over a duplicate-count-only fix. Detect duplicate keys before mapping collapse, including quoted spellings; reject unsupported merge/alias or multi-document metadata rather than selecting one value. Cost: those legal YAML representations may be refused; normal quoted/plain scalar forms remain supported and are checked against Helm. |
| Credential transport | Adopt simplicity's stdin header transport and robustness's real HTTP header enforcement, argv observation and invalid-token tests. Reject temporary credential files and helper arguments containing secrets. Cost: curl header-stdin support must be proved and header generation must reject control characters. |
| HTTP 000 | Adopt simplicity's narrow status-output control only if an installed-curl real response cannot reach successful-curl 000; adopt robustness's requirement to first attempt and document reachability and separately prove real manifest transport failure. A classifier-input test is explicitly not a real registry 000 receipt. Cost: two evidence classes must be maintained; no simulated client or production injection knob can replace the real transport proof. |
| Test size | Robustness wins wherever a smaller patch cannot detect its own defect: workflow bypass mutations, all chart refusal arms and package/image extraction are required. Simplicity wins against duplicate 12b/12d counts and a general Actions evaluator. Cost: more negative fixtures and execution time, without a new framework. |
| CSP and attribution | Adopt both positions' comment-only CSP correction and one packaged/image NOTICE. Reject policy tightening and application relicensing. Cost: clicked navigation and same-origin image GETs remain possible; distributions grow by the notice bytes. |
| External completion | Adopt both positions' separate release/image evidence obligations. Reject counting local HTTP, Dockerfile text or a baseline proof pass as the corresponding real-release, image or candidate-proof result. Cost: full story completion may remain pending after all compatible local work lands. |

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

All local compatible work remains required even when AC16 or AC17 awaits an
external environment/action. The check-then-push race, registry outages
blocking release and nontransactional partial publication remain existing
costs. No automatic recovery is added. Chief's result is `drafted`; only the
pinned final design check may advance design. Implementation, required gates,
four independent reviews, image evidence and the real release receipt are not
established by this document. Report finding-by-finding dispositions, commits,
proof receipts, CI-only evidence, residuals and the #184 integration request.
