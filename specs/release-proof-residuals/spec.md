# Release proof residuals

Status: design draft for #186 criteria 18.1–18.4 under epic #192.
This design authorizes bounded local proof/package repairs, not release or
issue closure. Criterion 18.5 remains **EXPLICIT EXTERNAL PENDING**.

## Feature and rationale

Make each requested chart/publish refusal distinguish its actual failure arm,
prove the refusal fixture owns its socket throughout use, and prove Cargo
packages intended root contents without synthetic in-tree cache decoys.
Fail-closed exit alone is insufficient: the wrong validation stage can refuse
and let a weak assertion pass. Actual Cargo membership, rather than a guessed
glob interpretation, establishes the packaging boundary.

The inspected base is `df8630f` (PR212). It already contains the three requested
chart cases, held non-listening refusal socket, manifest-close experiment,
HTTP-000 classification control and root license/NOTICE anchors. These are
retained and freshly proved, not rewritten to manufacture a diff. The known
remaining assertion gap is the three require-present unknown-state cases in
`bundles/self/scripts/publish-guard-test.sh`, which accept only the generic
`registry state unknown` diagnostic.

## Authority and scope

Preserve [house rules](../../docs/house-rules.md), accepted
[0019](../../docs/decisions/0019-release-tag-integrity.md),
[0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md),
[0045](../../docs/decisions/0045-delivery-repairs.md),
[0050](../../docs/decisions/0050-delivery-gates.md),
[0056](../../docs/decisions/0056-signing-policy.md) and
[0057](../../docs/decisions/0057-codex-delivery-recipes.md).
[0051](../../docs/decisions/0051-release-records.md) remains **proposed** in
its header and index; neither the task's shorthand nor inherited handoff prose
accepts it. Preserve PR212's implemented image-before-chart behavior and all
0051 release boundaries. This design changes no ruling, public contract or
frozen guarantee; no new ADR is declared. A discovered semantic/security
change requires escalation and, where applicable, a proposed ADR and operator
resolution under 0045 §8 before contradictory implementation.

Implementation owners are `scripts/chart-verify.sh`, canonical
`bundles/self/scripts/publish-guard-test.sh`, `Cargo.toml`, and directly
necessary package documentation/fixtures. A demonstrated defect alone can
justify editing canonical `bundles/self/scripts/publish-guard.sh` within the
existing contract. Public publish dispatchers, shim hashes, CI, recipes/policy,
versions, dependencies, allowlists, chart semantics and Rust production code
are outside scope. Root owns integration and CI. The recorded `engine` triage
keeps the full route: Qwen Max correctness, Claude Opus security, official
DeepSeek Flash guarantees and DashScope GLM-5.3 tests, without Pro or fallback.
The chief does not dispatch reviewers or choose the next phase.

## Panel synthesis and rulings

The chief read both run-local positions (`simplicity.md`, `robustness.md`) and
the run's intake framing under `.brokkr/state/`; they remain uncommitted.

| Question | Ruling and cost |
|---|---|
| Existing repairs versus replacement | Adopt simplicity's existing-owner, evidence-first repair and robustness's defect-specific controls. Reject duplicate chart cases, a second HTTP fixture, socket broker and generic mutation framework. Cost: existing fixes still need fresh independent evidence; a small diff does not imply a cheap proof. |
| Diagnostic precision | Adopt robustness's stage/status and per-case request identity checks; adopt simplicity's three targeted presence-needle replacements. Token HTTP failure and refused token connection may share `token exchange failed` because they enter the same arm. Reject a new production error solely to distinguish those scenarios. Cost: no separate production taxonomy for transport versus HTTP exchange failures. |
| Proof matrix | Require wrong-reason and inappropriate-admission mutations for targeted arms and independently target each of the three presence assertions. Shared manifest-close/000 arms need one demonstrated shared-path proof, not a cartesian duplication by mode. Add mode-specific cases only if a control reveals mode-specific unproved behavior. Cost: receipts must establish applicability instead of asserting every combination ran. |
| Cargo anchors: genuine disagreement | Adopt robustness's explicit leading `/` for all intended includes, including slash-containing source/test/bench/asset/data patterns. Reject simplicity's restriction to changing only patterns proved leaky: the user's explicit root-boundary request warrants this small declarative clarification. Accept simplicity's empirical constraint: behavior-equivalent patterns are recorded as equivalent, never called newly fixed defects. Cost of this ruling: up to five behavior-preserving manifest edits and membership comparison; cost of the losing alternative: root intent remains implicit and Cargo-specific. |
| Refusal lifetime | Both agree: retain the existing child-held non-listening port-0 socket, predicate readiness and joined cleanup. Adopt robustness's holder-close negative control. Reject release/reuse and probabilistic stress as proof. Cost: one held descriptor and platform-specific real socket evidence. |
| Larger release remedies | Reject retries, overrides, skip-on-existing, rollback, serialization, digest enforcement and a release ledger. Retain the named outage, partial-release and concurrent check/push costs. These repairs do not purchase registry immutability or future-release evidence. |

## Acceptance Criteria

Each criterion requires current evidence from the final candidate; inherited
claims and this draft are not passes.

- **AC1 / 18.1 — chart discrimination.** The real chart selftest separately
  exercises malformed appVersion (`appVersion is not a release semver string`),
  push without baseline (`without CHART_PUSH_BEFORE`), and malformed historical
  baseline (`baseline chart metadata invalid`). Each has otherwise valid
  inputs; the baseline case has valid current metadata, an existing named
  baseline and a chart change. Real temporary Git histories retain the entire
  event baseline, never substitute HEAD's parent. Independently disable or
  misroute each owning validation in a disposable copy: its named case must
  fail the real selftest, then exact restored bytes must pass. All existing
  metadata, rendered-contract and history assertions remain intact.
- **AC2 / 18.2 — named publish refusals.** Every exercised unknown-state
  assertion requires exit 1 and its guard-owned arm needle: token HTTP/refused
  exchange `token exchange failed`; successful token exchange followed by
  manifest connection close `manifest query failed`; successful-curl HTTP
  500 `HTTP 500`; successful-curl 000 classification `HTTP 000`. Tighten all
  three generic presence-mode assertions. Clear request records per targeted
  scenario; verify the exact token scope/repository and manifest tag where a
  request occurs. Token failure never reaches a manifest; a refused connection
  reaches neither HTTP endpoint. Manifest-stage cases demonstrate exchange
  success and the intended manifest request. Wrong-reason mutations that still
  exit 1, plus inappropriate-admission controls, must fail their owning named
  assertions, with restored real selftests passing. Independently prove each
  changed presence assertion; an earlier absence failure cannot certify it.
- **AC3 / 18.2 — HTTP-000 boundary.** Retain the installed-curl raw HTTP-000
  experiment and record curl identity, actual exit and transport diagnostic.
  Invalid HTTP status transport rejection is not successful classification.
  The narrow existing observer may substitute only the status output of a
  successful real authenticated manifest 404 with 000; preserve stdin,
  request headers, real errors and exit propagation. Label this proof
  classification-only, keep independent real manifest-close evidence, and
  detect a mutation that misclassifies 000. No substituted network client or
  live-registry/TLS readiness claim is permitted.
- **AC4 / 18.3 — owned refusal.** One child binds a non-listening loopback
  socket to port 0 before publishing readiness and retains that same socket
  continuously across all refused attempts until cleanup terminates and joins
  it. HTTP also binds port 0. Ownership assertions surround the refused
  attempts; readiness uses a predicate/deadline, with run-private state.
  Closing the holder in a disposable control must fail specifically with
  `refusal socket ownership lost`, even if the now-unowned port still refuses
  connections. Restoration passes. No fixed port, release/rebind, fixed sleep
  as synchronization or elapsed-time validity assertion.
- **AC5 / 18.4 — actual package boundary.** Intended explicit includes use
  `/src/**`, `/tests/**`, `/benches/**`, `/assets/**`,
  `/scripts/data/html-entities.json`, `/LICENSE-MIT`, `/LICENSE-APACHE` and
  `/NOTICE`; never `./`. Run actual offline `cargo package --list --locked`
  on the candidate and a disposable candidate copy containing synthetic
  `.cargo-home` decoys for every include family, without nested Cargo.toml
  boundaries. Assert all intended root members and Cargo's automatic
  manifest/lock/readme members remain and no cache-decoy path appears. Compare
  before/after lists and explain every difference. Individually unanchor the
  implicated basename patterns in private negative controls: Cargo may exit
  zero, but the membership assertion must fail and name actual leaked members.
  For already root-relative slash patterns, report measured equivalence rather
  than inventing a red control. Inspect an actual `.crate` archive for exact
  root license, NOTICE and entity-data bytes and preserved package metadata.
  No real Cargo cache/credentials are copied; dependency/version/SPDX and
  attribution contracts remain unchanged.
- **AC6 — preserved release proof.** Keep exact triggering-ref binds,
  requested repository/manifest identity, image bare and v-form absence
  probes, bare-only chart probe, 404-only absence and 200-only presence
  admission, `DOCKER_RESULT=success`, ordinary/same-run-rate prerequisites and
  image-before-chart ordering. Keep Basic/Bearer/Accept and credential-free
  argv assertions, bounded reads, and all existing workflow mutation checks.
  Full current selftests pass; mutations for the repaired assertions cannot
  be detected solely by unrelated syntax, prerequisite or dirty-tree errors.
- **AC7 — current local delivery evidence.** Record commands, exits, owning
  diagnostics, tool identities and candidate/script digests for every
  red/restored-positive pair and packaging proof. Run every gate in
  [the sole house inventory](../../docs/house-rules.md#gates--part-of-every-change),
  including locked clippy/tests, exact pinned brokkr compile, exact coverage
  and selftest, publish/seal proofs, deny and fail-closed native-tls absence.
  Preserve checked shims; record repaired candidate execution separately from
  dispatched immutable verification. Verify offline and install nothing.
  Missing tools or failed gates remain outstanding, never a pass. CI-only
  obligations retain their owners and are not inferred from local fixtures.
- **AC8 / 18.5 — external evidence remains open.** The handoff carries the
  exact checklist below marked **EXPLICIT EXTERNAL PENDING**. No local result
  closes #186 or #192, simulates a release receipt or authorizes tags, registry
  credentials, GHCR publication/deletion, push or merge.

## Outstanding future guarded-release receipt

Preserve the [existing receipt checklist](../epic192-release-security/spec.md#release-receipt-checklist--external-not-executed-by-this-lane).
The separately authorized future release must supply all of:

1. Approved release commit and signed triggering ref, deliberately aligned
   Cargo/Chart/appVersion; workflow revision/run URL and quality/test/coverage/
   deny/audit/compile, chart, smoke and same-run release-rate-5k results/URLs.
2. Exact `ghcr.io/feedback-loop-ai/swarmail:<version>` and `:v<version>`,
   plus `ghcr.io/feedback-loop-ai/charts/swarmail:<version>` identities.
3. Timestamped credential-redacted successful token exchanges and pre-push
   404 observations for both image forms and the bare chart, tied to guard
   revision, requested manifest identity and publication ordering.
4. Successful image publication before bare-appVersion presence 200, then
   chart absence preflight and chart push; immutable artifact/digest identities.
5. Post-publication exact-tag 200/readback, image/chart digests and packaged
   metadata/notice identities. These are receipt observations, not new
   automated post-push enforcement or an immutability promise.
6. Honest partial failures, reruns and manual recovery. A partial release
   cannot be reported as a complete receipt.

## Completion boundary

This chief commits only spec.md, plan.md and tasks.md in this directory and
returns `drafted`. The sequence's deterministic pinned check owns the design
verdict. Implementation, full gates, independent review, ship and external/CI
receipts are subsequent obligations. Frozen 0001–0003 and ingest-time extraction
0005 remain unchanged; no runtime repair is authorized by this design.
