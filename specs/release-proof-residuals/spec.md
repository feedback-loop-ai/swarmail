# Release proof residuals

Status: continuation design draft, 2026-10-24, for issue #186 under epic #192.
Run: `epic-192-issue-186-residual-foll-fa975081`. Slug: `release-proof-residuals`.
Criterion **18.5: EXPLICIT EXTERNAL PENDING**; #186/#192 remain open.

## Feature and rationale

Close the internal request-assertion helper's unknown-mode fallback and make
current proof provenance auditable without rewriting historical evidence.
`assert_requests` currently treats every value other than `none` as token-bearing
and adds a manifest only for `manifest`. An unknown mode therefore succeeds
against a token-only log. A refusal against an empty log would miss this bug.

The inspected continuation base is
`f00d8b532d7e255086913a53b21aa47c27fbe7c6`, tree
`1c4d0f3edc82a037033386e8a7f09802c77d7351`. Its local history is engine pin
`238372d`, design `79c3df9`, implementation `667b825`, then receipt `f00d8b5`.
PR212/`df8630f` is the original design's historical base, not this continuation's
base. The old receipt's `2c6f3e1`/`2dce0b4` identities remain historical claims;
no equivalence to local commits is inferred.

The three presence needles, request assertions and Cargo anchors already exist
in this base. The chart, holder and HTTP-000 proofs remain inherited obligations
whose evidence must be reconciled, not new defects to manufacture. Original
AC1–AC8 below retain their proof requirements; continuation AC9–AC14 delimit the
new work. Historical local proof is not current independent verify or release
proof. The requested 785-test base must be established from an actual run log,
not substituted for the historical receipt's 784 count.

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

The continuation implementation touches only canonical
`bundles/self/scripts/publish-guard-test.sh` and this directory's
`proof-receipt.md`, `plan.md`, and `tasks.md`. This chief changes only the three
design artifacts. Production guard, chart script, package helper, Cargo.toml,
public shims/hash inventory, CI, tooling pin, recipes/policy, versions,
dependencies, allowlists, Rust and accepted decisions are outside the residual
repair. A discovered further defect returns for bounded scope review.

Triage has already selected `engine` (TRIAGE-ENGINE). Under 0057 §§6–7, gate
repairs retain four independent configured reviewers: DashScope Qwen Max
correctness, Claude Opus security, official DeepSeek Flash guarantees and
DashScope GLM 5.3 tests. Four is the smallest permitted review for this
classification. No Pro, fallback or chief-side review dispatch. Provider failure
stops; the engine owns phase selection and protected review/ship.

## Panel synthesis and rulings — 2026-10-24

Read both positions in `.brokkr/state/design/positions/` and the intake framing
in `.brokkr/state/tasks/epic-192-issue-186-residual-foll-fa975081.md`. These are
run-local evidence, not committed artifacts. The two positions agree on the
substantive correction; there is no genuine policy conflict to average away.
The following rulings reconcile implementation size with evidentiary strength.

| Question | Adoption, rejection and cost |
|---|---|
| Helper shape | Adopt simplicity's one local `case` guard and robustness's closed vocabulary. Preserve the existing comparisons. Reject a mode framework, new fixture process or production error taxonomy. Cost: the tiny Bash helper remains manually maintained. |
| Regression strength | Adopt both positions' token-log collision using the existing real token-stage case, subshell refusal and mode-specific diagnostic. Adopt robustness's guard-only reversion control. Reject empty-log refusal as proof. Cost: an additional full negative/restored proof pair; omitting it would permit the old bug to pass. |
| Receipt identity | Adopt robustness's candidate/tree/command/exit/digest binding and simplicity's append-only existing receipt. Reject cosmetic replacement of old pin/count/path strings, a new receipt schema or provenance database. Cost: a longer addendum and human evidence audit; a cosmetic update would destroy historical applicability. |
| Scope and runtime analysis | Adopt simplicity's four implementation-file boundary. Retain robustness's frozen-guarantee analysis as preservation constraints, without converting it into runtime work. Reject reopening packaging, holder, chart, storage, webhook or TLS design absent a demonstrated defect. Cost: existing limitations and missing historical evidence remain visible. |
| Completion and review | Adopt both positions' journal-backed per-obligation reconciliation and full engine review. Reject checking compound T7/T8 from local logs or downgrading review because the diff is small. Cost: missing records leave boxes open and four reviews cost more than one; the smaller route would violate the selected gate-repair classification. |
| Release authority | Adopt explicit pending 18.5 and the unchanged six-category future receipt. Reject live GHCR access and new release policy. Cost: outage refusal, concurrent check/push races, partial image/chart publication and external receipt delay remain. |

## Acceptance Criteria

Original AC1–AC8 remain evidence obligations; their recorded historical work is
reconciled by AC12. Neither inherited claims nor this draft are passes. The
continuation does not authorize repeating implementation already present.

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

## Continuation acceptance criteria — 2026-10-24

- **AC9 — closed helper modes.** Exactly `none`, `token`, and `manifest`
  retain their existing exact sequence comparisons. Every unrecognized supplied
  mode refuses through `fail` before comparing log bytes, with nonzero exit and
  a diagnostic naming the invalid mode and case description. All nine existing
  recognized call sites and their request scope/tag checks remain intact.
- **AC10 — discriminating refusal regression.** After an existing real
  token-only scenario passes its recognized-mode assertion, the same unmodified
  log is tested with an invalid mode in a subshell. Explicit status and stderr
  assertions require refusal for that mode, not request mismatch. The complete
  canonical loopback proof passes. In a disposable adjacent copy, removing
  only AC9's guard makes the new assertion fail specifically because the invalid
  mode was admitted; exact restoration passes. No fabricated log, overridden
  client/read operation, additional server or dependence on conditional errexit.
- **AC11 — append-only current provenance.** Append a dated addendum to
  [proof-receipt.md](proof-receipt.md); its prior bytes at `f00d8b5` remain an
  unchanged prefix. Label source `597f4868e3fb971b62e6a368a2d8aa23271f4531`,
  binary `e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`,
  784 tests, old run/commit identities and absolute `NEXT-186` sibling path as
  historical, not current execution instructions. New evidence identifies
  current worktree, base/candidate/tree and changes, exact commands/exits/logs,
  canonical script digests and actual candidate-local engine. Bind source
  `731189f9097308c01ae4f2b4c9caea5f41224844`, all three ordered patch paths and
  full digests from tooling.json, and binary
  `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
  Identity checking is not a claim of rebuilding the engine. Record actual
  passed/failed/ignored suite counts and aggregate for the 785-test base and
  final candidate; explain any discrepancy. The shell check need not increase
  Rust's count. Missing logs are outstanding, never invented or borrowed.
- **AC12 — evidence-bound tasks and base.** Plan names the actual current
  base and distinguishes original ancestry. Every newly checked task cites an
  authentic run ID/event or journal-attached execution receipt, candidate
  applicability and all its obligations. Partial compound tasks remain unchecked
  or split into evidenced local and outstanding owner obligations. Candidate
  logs cannot complete immutable verify (T7), review/ship/root CI (T8), or
  external 18.5. No journal is fabricated, imported or inferred from prose.
  Missing authentic journal access remains an explicit evidence gap.
- **AC13 — final candidate proof.** Execute every gate in the sole house
  inventory with locked commands, exact 100% coverage and its selftest, pin-bound
  compile, publish/seal proofs and license check. Check shims and native-tls
  absence fail-closed: failed tree enumeration is not absence. Record candidate
  proof separately from actual immutable verify dispatch; use the pinned proof
  chain for verify, offline without installation. Re-run affected proofs after
  code changes; design-only preflight never proves the later helper repair.
  Failures, missing tools and CI-only obligations remain named and open.
- **AC14 — external boundary.** Preserve `18.5: EXPLICIT EXTERNAL PENDING`
  and all six checklist categories below. The external checkbox stays open.
  No GHCR access, publication, push, merge, tag, release-policy change, decision
  acceptance, or claim that AC18.5, #186 or #192 is done.

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
