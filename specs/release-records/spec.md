# Release safety and truthful records

Branch: `epic120-release-docs`  
Date: 2026-10-19  
Phase: resumed design confirmation; implementation present, fresh verification outstanding.
Issues: epic #120, batch 6, #141, #142 and #144.

Decision: 0051 — [Release prerequisites and decision provenance](../../docs/decisions/0051-release-records.md), **proposed**.

## Resume ruling — 2026-10-19, re-drive 3

Run `epic-120-batch-6-of-6-re-drive-3-7b1226d5` resumes from `3b218b0`,
which committed the preceding reconciliation above implementation `8478924`.
Both current positions and the intake confirm the existing design; the
earlier dated ruling below is retained as history. This confirmation changes
no acceptance criterion, runtime contract or decision status. Keep
`release-records` and proposed 0051; no new decision is warranted.

Adopt simplicity's reuse of the existing owners and robustness's preservation
of both commits and the prepared version triple. Neither panel identifies a
new implementation gap. Reject speculative repairs, version changes, new
gate owners and automatic registry recovery for the costs already ruled
below. The current positions have no substantive feature conflict. On the
delivery choice, robustness permits a no-op chief at `3b218b0`; choose a
dated three-artifact confirmation to satisfy this commission's explicit
commit requirement. Its cost is another signed documentation commit, not a
claim that the prior artifacts were defective.

Fresh evidence must name the final assessed HEAD, including this confirmation
if committed, rather than hard-code robustness's inspected `3b218b0` as the
future verification target. Prior gate receipts and reported signatures are
not fresh attestations. Preserve the branch without re-cutting or rewriting;
implementation may remain unchanged after its renewed audit. Local gates,
signature verification, review and ship remain downstream obligations; actual
event-baseline/Helm verification and real Docker smoke remain CI obligations.
No unavailable proof is passed by this design, whose result is `drafted`.

## Resume ruling — 2026-10-19, re-drive 2

The first design and implementation are present in signed commit
`847892409134bee082e8a3edcdb0243eb720a361` on `epic120-release-docs`.
The first run stopped on signing, and the operator's assistant committed its
unchanged staged tree when signing returned. Its local green gates are prior
evidence, not this run's verification verdict. The original design's statement
that implementation remained outstanding is superseded by this dated note.

Read both current panel positions and the intake sources below. Adopt both
positions' recommendation to confirm this design and reuse the implementation.
Adopt simplicity's required reconciliation of spec/task status; robustness
called it optional, but the chief's committed handoff must distinguish work
present from proof outstanding. The cost is one signed documentation commit.
No unmet issue criterion was identified by the panel or this design audit;
implementation changes only on a demonstrated gap, with a regression and a
new signed commit. Do not redo the repairs merely to create an implementation
diff. Keep the existing slug and proposed 0051; this confirmation introduces
no new semantics and allocates no additional decision number.

Retain the original panel rulings below. In particular, reject another chart
gate owner, global prose crawler, automatic registry recovery and forced
Cargo/chart version equality, at the costs recorded there. Qualify robustness's
claim that the pinned builder closes smoked-versus-pushed drift: it fixes the
base image, not every build input or byte identity. Retain the spec's explicit
limit. Status-word automation for a future acceptance of 0051 remains outside
this resume; that future change must update its current-status references.

The observed version triple is Cargo/appVersion **0.1.0**, prepared chart
**0.1.2**. Keep the original chart-0.1.1/v0.1.1 regression fixture and also
check the present tree's mismatched tags. Helm render and real Docker smoke
are CI-only proofs, still outstanding here; their absence locally does not
block this design or require replacement tests. Fresh local verification,
review and ship remain later phase obligations. This chief returns `drafted`,
not a completed-feature or validation verdict.

## Framing and authority

Read both run-local positions, `simplicity.md` and `robustness.md`, and the
complete epic and issue bodies, including #144's operator ruling. The three
files under `.brokkr/state/tasks/` frame earlier coverage runs, not batch 6;
the current commission and issue files are this batch's intake. The original
design base was main `1d45e5799c7ed1a1311fab501b3173c678d4f2c0`, a descendant of
review baseline `2a325c9`. Earlier batches' repairs are requirements, not work
to repeat or undo. Fetch/branch/commit execution is separate delivery evidence.

The operator's 2026-10-19 ruling in #144 accepts 0034–0050 and 0055 as filed,
including existing dated amendments. Record that acceptance without asking
again, reverting implementations, or extending it to this new decision.
0051 was allocated within epic #120's reservation: 0055 explicitly reserves
0049–0054. It is already committed and remains proposed. This resume's chief
commit touches only this specification, plan and tasks; the existing decision,
index/status/marker repairs and implementation remain in place.

## Feature and rationale

A release must never publish a chart after its required image publication
failed, was skipped or was cancelled. Installation examples must clean up
their ephemeral storage. Documentation and decision records must describe
the tree after batches 1–5, with executable checks for the finite contracts
that have repeatedly drifted. This closes the three issues' criteria without
turning a release/documentation batch into a mail-server rewrite.

Synchronous commit before SMTP acceptance, subscribe before scan, oldest-only
pruning and extraction at ingest remain unchanged. Preserve 0033's writer
lock and 0047's alias handling, the five-second busy bound, awaits cap,
webhook and bridge behavior, Origin guard, envelope sender, exact extraction
pins, 0026's startup probe, 0030's chart knobs, and 0019's republish refusal.
No dependency is added or allowance widened. No push, tag, publication, PR,
merge, issue closure or external comment is authorized by this design.

## Panel rulings

| Question | Ruling and explicit cost |
|---|---|
| Implicit success gating or explicit release admission? | Adopt robustness: `chart-publish` needs Docker and explicitly requires `needs.docker.result == 'success'`; the chart preflight independently refuses other results and requires the referenced image manifest to exist. Retain simplicity's existing guard owner. Cost: one tested guard mode and an extra bounded registry read; a registry outage blocks a release. |
| One version everywhere? | Adopt simplicity: chart preparation and application versions stay independent; `appVersion` must equal Cargo, and rendered/default/annotation image claims must agree. Existing exact tag binds imply alignment for a successful joint tag release. Reject a permanent Cargo=Chart invariant and any automatic Cargo bump. Cost: today's `v0.1.1` releases neither artifact, and an operator must deliberately prepare a releasable pair. |
| New pinned chart-bump gate or existing chart owner? | Adopt simplicity's CI-only `scripts/chart-verify.sh` ownership, including a selftest mode for history/version checks. Reject a new mandatory hermetic gate and shim family. 0050's pinned verify chain remains untouched except its existing publish-guard implementation/proof. Cost: chart-history proof remains a CI-only responsibility, explicitly listed in house rules. |
| Container smoke scope? | Adopt both positions' real non-root named-volume restart test, including robustness's hard-kill case; adopt simplicity's actual anonymous-volume cleanup and Docker context sentinel proof. Keep `VOLUME /data`. Cost: a Docker build and two persistence restart cycles in CI. A cached publication rebuild is not byte identity with the smoked image. |
| Every sentence pinned or a finite correction audit? | Adopt simplicity's finite matrix below, strengthen OpenAPI with robustness's behavior-to-schema proof, remove volatile counts and unsupported claims. Reject an all-document prose crawler and benchmark CI. Cost: historical prose still needs review; performance medians remain dated measurements. |
| New decision schema and test file? | Extend existing `tests/store_contract_docs.rs`: bidirectional membership, status, links and targeted amendment pairs. Reject robustness's migration of every header, expected-number-gap constant, and generic English amendment parser. Cost: semantic completeness of amendment claims still needs an explicit audit; number gaps are legal and not invented records. |
| Future proposals? | Adopt simplicity and accepted 0045 §8: compatible implementation may land under a proposal; contradictions to any accepted ruling, including 0001–0005, wait for operator acceptance. Reject a new release-age deadline for proposals and a blanket ban on compatible clarifications. Cost: proposals may remain pending; status never supplies authority. Frozen guarantees cannot be weakened merely by an ADR or passing tests. |
| Test placement? | Follow accepted 0020, correcting robustness's suggestion that a private seam alone forces a harness move. Deterministic branch-stable tests stay inline; only concrete cold failure paths justify sibling harness files. |

## Release requirements (#141)

R1. Keep all existing release prerequisites and same-run tag rate proof.
Docker additionally needs successful chart verification and Docker smoke;
chart publication additionally needs successful Docker. No dependency cycle.
Chart verification/smoke run with read-only credentials on pushes and PRs;
only existing publishers hold package-write permission.

R2. Before chart publication, validate versions, exact chart ref, absent chart
tag and present image `ghcr.io/feedback-loop-ai/swarmail:<appVersion>`. The
presence check is a mode of the canonical pinned publish guard, retaining
bounded status-code classification: 200 passes presence, 404 refuses missing,
all other/transport/token errors refuse unknown. Existing absence checks
still require 404 and refuse 200; Docker still checks both bare and v tags.
The mode also requires an explicit successful Docker result supplied from
the job dependency, so a future job-condition edit cannot bypass ordering.
Do not allow skipping an existing release or force-republish overrides.

R3. Chart verification compares `appVersion` with Cargo and derives the
rendered tag and Artifact Hub image expectation from it. Chart edits require
a strictly increased valid chart version against the entire event baseline,
including edits earlier in a multi-commit PR. Missing required history is a
named refusal. A chart-only preparation bump is allowed and is not a release.
Do not bump Cargo or revert the prepared chart 0.1.2; any later necessary
chart edit must satisfy the same bump check and remain identified as unpublished.

R4. Both ephemeral Docker quickstarts use `--rm` and a stop grace longer than
the ten-second server drain (30 seconds). Documented named volumes persist
and have explicit cleanup instructions. The `image.tag=main` Helm example
also sets `image.pullPolicy=Always`. Remove the expired next-main workaround.
Exclude `**/target/` from Docker context, including standalone tools/evals.

R5. Smoke the actual image as its configured uid, with fresh named storage,
loopback ephemeral host ports, deadline-bounded protocol readiness, real SMTP
DATA acceptance and REST readback. After both graceful stop and hard kill,
restart on the same volume and verify the accepted mail. Exercise anonymous
volume removal and root/nested context exclusions through real Docker.
Cleanup owns only the test's unique resources and runs on failure too.

## Documentation correction register (#142)

Each row is an implementation obligation. Re-check against the current tree;
already-correct portions stay correct. The proof column distinguishes an
executable pin from an audited factual correction, rather than pretending
every historical sentence is a machine-verifiable claim.

| ID | Correction and source of truth | Required evidence |
|---|---|---|
| D01 | CONTRIBUTING, AGENTS and README link the one house-rules inventory; retain locked commands, coverage selftest, publish-guard proof, pinned verify ownership, native-tls absence and CI-only distinctions. | Targeted links/list audit against CI and pinned verify commands; existing delivery tests. |
| D02 | Signing prose distinguishes required signed branch authorship, documented historical unsigned exceptions and GitHub-signed squash merges. | Audit history without rewriting it; no standing unsigned permission. |
| D03 | Replace root-bundle compile paths in agents README and implementer/review-correctness/reviewer charters with `bundles/self`. | Search the named files; pinned compile. |
| D04 | Bundle/policy descriptions name all four review members; agents README links real bundle/policy paths, cites 0009 for seat library and marks it accepted. Resolve obsolete retired-reviewer statement against actual seat files without changing routes or active wiring. | Link/seat inventory audit and existing constitution tests. |
| D05 | Remove Zlib/CC-BY-4.0 allowances only after checking current license use; fix contradictory CDLA comment and corresponding house-rules list. | `cargo deny check licenses`, current locked dependency evidence, no widening. |
| D06 | v0.1.0 release notes and site distinguish measured criterion medians from gating CI assertions; same-run release rate remains a real assertion. | Audit actual workflow; remove false benchmark-CI claim, add no benchmark gate. |
| D07 | Replace absolute release-note immutability claim with dated append-only errata policy; preserve existing corrections, identify their historical provenance. | Dated errata separating shipped measurements, corrections and unreleased changes; no fabricated release. |
| D08 | Remove README's unsupported crates.io shipping claim; keep source/image installation instructions true. | Current publication configuration and recorded evidence; no new publish. |
| D09 | README names `chart-verify`, accepted 0008, exact 100/100/100 invariant and `target/coverage/` reports instead of drifting counts. Include `tests/store_contract_docs.rs` and any other omitted current root suite. | Workflow and actual `tests/*.rs` inventory; targeted suite/table check. |
| D10 | README chart values table includes current peers, TLS, startupProbe and bind/config options and correct defaults. | Compare `values.yaml`, templates and both existing rendered profiles; retain all knobs. |
| D11 | Describe `gen-cert` stdout exception and actual stderr behavior of other commands. | Existing binary tests plus direct `--help`/generated-certificate evidence where needed. |
| D12 | Document Docker default ten-second grace versus ten-second drain, recommend 30 seconds; default stop signal is SIGTERM, correct Dockerfile comment. | Docker smoke and Docker invocation inspection; no timing-validity assertion. |
| D13 | Document 2 MiB REST request body bound and seed's `413 text/plain`; distinguish MCP JSON-RPC envelope. | Existing/new real oversized-body requests linked to OpenAPI response entries. |
| D14 | Document non-UTF-8 SMTP command closure without reply. | Existing SMTP edge regression; no parser change. |
| D15 | Document accepted zero TLS handshake timeout and immediate handshake failure. | Existing TLS/config proof; no new parse refusal. |
| D16 | One inbox per SMTP session; second AUTH returns 503. Warn that Python smtplib can accept this without raising: login(A), login(B) still sends into A; open a fresh connection for B. | Existing repeat-AUTH wire proof and recorded client reproduction. Leave protocol unchanged. |
| D17 | Correct `33ac7d1`'s full-entity claim: it shipped a subset; batch 4 now ships the full 2,125-name table. | Dated correction citing history and current table/proof. |
| D18 | Correct `582c73a`'s latest-query/shared-search and Re[n] claims: query is refused, latest uses Store::latest, literal Re[2]: is not stripped. | Dated correction tied to current compat/thread tests. |
| D19 | Correct `40ef305`'s named-secret-byte claim: refusal uses fixed value-free text. | Dated correction and existing webhook refusal proof. |
| D20 | Add #118's missing entry: unknown query key 400s (scoped accurately; compat listing/search tolerance remains), `messages_unread_count` to `messages_unread`, MailHog envelope From/kind=from, and byte-budget refusal. | Release errata/unreleased entry tied to #118 and current compat tests. |
| D21 | Serve help says Host and Origin checks; degraded comments in `src/lib.rs`/`src/api.rs` say clear on observed pending accept, not next success. | Help-output pin and existing listener-recovery tests; comments only otherwise. |
| D22 | OpenAPI await limit default 100, minimum 0, maximum 250; count accepts 0 with minimum-one wait semantics. Assert limit is accepted but inert, without an invented effective page default/minimum-one restriction or 250 cap. | Schema/parser/domain assertions plus real omitted/zero/capped await pages, count=120 returns 100, count=0 wait behavior, assert inert-field cases. |
| D23 | OpenAPI documents guard 403 for every exposed operation, health 503, GET messages 400, clear blank-inbox 400/commit-failure 503, threads/feed blank-inbox 400, and applicable body 413/415 responses. Preserve batch 3's existing 400 corrections. | Wire statuses compared to each relevant method's published response map; no blanket schema-number test alone. |
| D24 | Thread.subject describes the oldest member with a subject and its actual optional shape. | Two-message thread and missing-subject cases compared with schema, using actual thread behavior. |

Closed-issue correction comments requested by #142 are prepared as a dated
local release/documentation record for operator handoff. This run sends no
GitHub comments and does not rewrite commit messages.

## Decision record requirements (#144)

Record the operator-authorized acceptance in each listed current status and
the index, preserving accepted body text and dated corrections (particularly
0047's lock correction and 0050's evaluator amendment). Add missing 0035 and
0055 rows, and index 0051 as proposed during implementation. Current status
references in house rules, AGENTS, runtime comments, chart and OpenAPI follow
those statuses; historical uses of “proposed” remain dated historical text.

Audit every asserted amendment/appended pointer, including earlier records,
and append dated linked markers or a dated correction when the claim is
wrong. The mandatory starting relationships are in the plan. Do not insert
misleading pointers just to satisfy a string test. Clarifications in 0027 and
0031 now rest on accepted proposals; append their acceptance provenance.
Correct 0035's 1,2,3,5 enumeration with a dated mapping of old item 5 to
sequential item 4, retaining old citation identity and inventing no lost rule.

House rules state 0045 §8's compatible-work rule explicitly for all 0001–0005.
The decisions directory contains accepted rulings and proposals awaiting the
operator. Existing historical decisions are not globally reformatted.

## Acceptance Criteria

- **AC1 (#141):** Tag release fixtures show Docker failure/skipped/cancelled
  refuses chart admission, successful same-run prerequisites admit it, and
  the original Cargo/appVersion 0.1.0 versus chart 0.1.1 fixture at v0.1.1
  publishes neither. The present prepared chart 0.1.2 likewise cannot publish
  alone at v0.1.1 or v0.1.2; a v0.1.0 rerun retains republish refusal.
  Preserve exact refs, both Docker tags, rate proof and republish refusal; a missing/unknown image refuses the actual chart preflight.
- **AC2 (#141):** Changed/unbumped, decreased or malformed chart versions fail
  CI; valid increasing versions and unchanged charts pass. Multi-commit PR,
  push, tag, new-branch and unavailable-baseline cases are explicit tested
  inputs. appVersion/Cargo mismatch fails and a coherent changed application
  version renders successfully without a hard-coded old tag.
- **AC3 (#141):** Real Docker proves both quickstarts leave no anonymous
  volume, named-volume mail accepted with 250 survives graceful and hard
  stops under the image uid, and root/nested target sentinels never enter
  build context. The documented main Helm flags render Always.
- **AC4 (#142):** D01–D21 each have a recorded source/proof disposition; every
  named sentence matches current code, all gate references share house rules,
  and “asserted in CI” applies only to actual gating jobs. Historical claims
  and #118 have dated local corrections with operator handoff identified.
- **AC5 (#142):** D22–D24 match actual parsers, response behavior and thread
  shape. Extended OpenAPI pins fail when old defaults/minimums/status omissions
  or newest-subject wording are restored, and pass for the repaired schema.
- **AC6 (#144):** Every numbered decision file has exactly one resolving
  index row, and every row has a file; numbers/rows are unique and statuses
  match. Tests refuse missing/dangling/duplicate entries and either one-sided
  status change, with a coherent pair passing independently of today's mix
  of accepted/proposed records.
- **AC7 (#144):** All amendment/appended claims are audited; dated links are
  present on both sides or explicit dated corrections explain inaccurate
  claims. Tests remove each required pointer in turn and fail. 0045's
  promised 0015/0018/0023 corrections and real 0030 pointers exist. Accepted
  historical body text is preserved; authorized statuses and current citations
  are consistent; 0035 numbering is clarified without rewriting history.
- **AC8 (#144):** House rules explicitly permit compatible proposed work and
  require acceptance before contradicting accepted rulings, including
  0001–0005; frozen guarantees remain protected. 0051 stays proposed unless
  the operator separately accepts it.
- **AC9 (all):** Every behavior repair has a discriminating regression
  failing on 2a325c9's behavior and passing after repair, with green controls
  and issue/test mappings in implementation commit bodies. Existing earlier
  batch repairs remain green. Run the final-tree gate inventory and relevant
  CI-only proofs; unavailable execution is outstanding, never a pass.

## Explicit limits

Two package pushes are not transactional. Image-first ordering can leave an
image without a chart; chart success requires image presence at the check,
not protection against later deletion or the existing concurrent-probe race.
Rerunning all jobs after partial publication hits republish refusal and may
require deliberate operator registry recovery; no automatic deletion is added.
The smoke and publication are separate cached builds, even with the existing
digest-pinned builder: package repositories/build inputs can still differ.
The smtplib second-AUTH hazard and zero TLS timeout remain documented runtime
limitations. This batch records and tests facts; it does not accept itself,
publish a release, or claim completion from a structural design check alone.
