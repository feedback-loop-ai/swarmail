# Source-attributed mail comparison continuation

Date: 2026-10-29. Status: draft. Epic #164 / issue #163.
Base: `14217f7c9f30718c450aa25fa04ccbac064dddba`, tree
`44e4f4cb89aa37b90c90d3a219bc64551cdd86a4`.

Decision: 0093 — [Source attribution and observation contracts](../../docs/decisions/0093-source-mail-comparison.md), proposed; only the operator accepts or reconciles numbering.

## What and why

Finish the existing real comparison through its current owners: independent
three-target wire driver, thin Mailpit MCP adapter, standalone eval runner and
sole scorer, one shell entry point, and docs/evals evidence. New source captures
resolve tagged-source attribution and permit grounded stream discussion. They do
not prove executable build provenance, equivalent waits, fair same-suite results
or delivery completion. Make those distinctions executable admission rules and
complete every locally actionable inherited gap.

[Fair comparison R1–R6 and AC1–AC16](../fair-mail-comparison/spec.md),
[official continuation C1–C5](../official-mail-comparison/spec.md), and
[all T1–T14 and refinements](../official-mail-comparison/tasks.md) remain normative.
The acceptance criteria are reproduced below without weakening their meaning.
This adds requirements without rewriting historical specs, evidence or receipts.
A source report, smoke run, controls result or common subset cannot complete #163.
No paid trial, push, merge, publication, tag, issue closure or acceptance is
authorized. Engine verification and protected review own their results.

The requested `.brokkr/state/tasks/` directory is absent. Framing is the supplied
task and retained intake `.forge/results/3a57845b-6158-44fa-9ad2-ca0a13dfd04c.json`.
Both `.brokkr/state/design/positions/simplicity.md` and `robustness.md` were read;
they stay ignored run-local arguments. The entire stopped
`.forge/evidence/issue163-official-followup-stop/implement-event.json` was read and
its SHA-256 independently matched
`d7c392fd74485ac452b8552162ff7a5d3aaf99390ad92d82bf31b0b89775519d`.
Its clean-root-gate, four competitor-test and 1,926-row smoke statements concern
the stopped candidate, not new receipts. It also records standalone eval failure
and incomplete story.

## Explicit panel rulings

The positions substantially agree; these choices do not invent disagreement.
Where the smaller implementation would omit a proof, robustness governs admission.

| Choice | Adopted ruling | Rejection and cost |
|---|---|---|
| Architecture | Simplicity: fixed targets, current modules and one owner per lifecycle. | Reject new crate, backend hierarchy, supervisor framework, provenance service, dashboard, scorer, MailHog agent mode or dependencies. Fixed targets cost extensibility. |
| Attribution | Both: captured primary bytes; robustness requires explicit claim scope and negative validation. | Reject filling a built-from field with a tag commit. New lock version costs a strict reader and historical compatibility; no reproducible-build project. |
| Streaming | Simplicity's bounded common polling; robustness's source/route/readiness threshold for best-wait. | Reject speculative event infrastructure and “no stream” claims. Best-wait stays open; polling costs requests, scan work and latency. |
| Ownership | Robustness: corroborate owned binds and retain async ownership through finalization. | Reject live-child-plus-response readiness and kill-and-return cleanup. Extra tests and conservative refusals cost time; no confinement claim. |
| Scans/waits | Robustness: quiescent final reconciliation, independent IDs, aggregate budgets and exact cutoff semantics. | Reject stable count as snapshot proof and early timeout as interval absence. Churn/budget exhaustion may leave incomplete results. |
| Resources | Simplicity's separately declared unmatched/unenforced local profile, with robustness's actual-setting/cumulative-retention proof. | Reject manufacturing parity through a platform. Requested matched/enforced profiles can remain blocked. |
| Full story | Both: complete T7–T10 in existing owners and build T12 reporting; retain T11 dependency. | Reject stubs, private coordination, filtered tests, smaller denominators and controls as paid evidence. Sequential trials are slower; final claims can remain open. |
| Failure scope | Preserve robustness's crash/cap/chaos/DATA/TLS/webhook constraints with existing regressions and changed-boundary tests. | Reject storage redesign, durable outbox and new cross-product fault campaign. No new durability/TLS-equivalence claim. |

## Evidence checked at design

All ten manifest SHA-256 values and Git blob IDs matched captured untruncated trees;
ref → annotated-tag → full-commit chains agree. Manifest SHA-256 is
`138c1e03af40498a32c6482bbe75a50eaaa72ed8ddc95dec80cc7cbfc04271d7`.
All four license/API bodies match earlier captured and tracked bytes. Both setup
READMEs and all four websocket source bodies were reviewed. All eight official
SHA256SUMS entries, executable/archive-member equality, Mailpit archive LICENSE,
brokkr binary and three tooling patches passed local consistency checks.

| Target | Tag object | Tagged commit | Executable SHA-256 |
|---|---|---|---|
| Mailpit v1.32.0 | `c00f93d0a78967e9f84c291eb34ab73386fdbe0c` | `ce798cfd781ef07b321427fc5082b3eebdcae460` | `262cfa28fae4554be198d1fea5ef83c0f8299317876d658623711c629a8edec8` |
| MailHog v1.0.1 | `056259cd4133e22624340741776709bb46eb9ae9` | `0441dd494b03c9255a9b8e90e3458ebb115eacff` | `e2ed634ded49929f089b20045581955ed217672078fd86082dd7a6c67c5d09c7` |

Mailpit archive SHA-256
`4b5993c978e8604c70d43927e2ad103b311bba3b1822fa0cf1d4ee4ec890a93c`
matches captured release metadata. MailHog metadata has no digest; its digest is
locally observed only. Both tags report unsigned. No fresh network observation,
signature authentication, reconstructed commit object or binary-build attestation
was performed. Acquisition assertions remain supplied evidence, not independent
authentication. Source identity is not executable-source equivalence.

Mailpit hub/client source broadcasts Type/Data notifications, skips broadcast with
no registered clients, buffers 256 messages per client and removes slow clients.
Upgrade precedes registration; its writer has a ten-second deadline and pings.
MailHog's captured vendored hub also removes slow clients with 256-entry queues;
its connection has write and pong deadlines. Neither subset establishes durable
replay, query subscription acknowledgement, store/producer ordering or lossless
matching wait. Notification loss is not proof of stored-mail loss. MailHog's README
advertises EventSource; the websocket subset does not establish its active route
or exclude other mechanisms. Route/producer/EventSource bodies and MailHog APIv1/
detail primary bodies remain missing. Tree paths and README links are not reviewed
bodies. README setup facts do not establish every flag; checked binary help and
launch/configuration probes remain separate evidence.

## Continuation requirements

### C1 — Version claims without inventing provenance

Source-lock v3 separates `tagged_source` (repository, tag, exact tag object, full
commit and captured chain identities) from required `binary_build_provenance`,
explicitly unresolved until genuine build evidence exists. Retain v1/v2 bytes and
matching historical interpretations; never reinterpret v2 `source_revision`,
currently described as the commit the release was built from. Distinguish archive,
executable, metadata, document and acquisition facts. Retain capture/ref/tag/tree/
README/source hashes and immutable URLs in the existing lock/evidence directories,
with license notices. No second authoritative manifest or repository vendoring.

A frozen official-artifact profile may measure the checked executable with tagged
source context and explicitly unknown build provenance. It cannot claim it was
built from that source or satisfy a profile requiring that proof. Missing primary
facts required by an operation still block dependent evidence collection; untimed
probes cannot upgrade it. Negative tests reject altered/missing bytes, false digest
basis, mismatched tag chains, mutable-only revisions and false build assertions.
Recheck executable identity before finalization.

### C2 — Freeze actual profiles and observation semantics

Protocol v2 binds per-target storage/compression/caps, declared versus enforced
resources, full products/applicability, schema/corpus/lock identities, all page/
byte/request/operation bounds and a change reason before collection. Preserve six
permutations/repetitions, 100 warmups, 3,000 measured messages, fresh/reused 1/10/100
sessions, separate bounded 1,000-session stress, and 100 present/100 future/20 empty
waits per repetition. Retain 300 s send, 60 s verification, 30 s wait and 1 s empty
bounds unless a separate committed method revision justifies changing them.

Mailpit scratch SQLite is not matched memory; declared 2 CPU/2 GiB is not enforcement.
Freeze a separately named actual local profile and leave requested matched/enforced
profiles open. Prove retention for cumulative warmup/measured residency per reused
instance; `--max 10000` does not establish pruning semantics. Stress reports actual
server caps and scheduled/admitted/refused work, not 1,000 accepted sessions.

Select immediate complete scans followed by 100 ms after each completed scan for
the labeled common-client and adapter polling method. This is an experimental
choice, not a source-derived optimum. Include scan duration, requests, bytes and
overhead; bind actual adapter bounds and tools/list bytes. Keep native Swarmail
await separate and subscribe-before-check unchanged. Preserve requested `wait-best`
as blocked pending complete mechanism-selection and real-binary evidence. Do not
delete it or relabel common polling as best. A later event method needs a new
committed method, subscription before scan, registration readiness, gap/reconnect
rescans and honest pruning limits; it is outside the selected implementation.

One absolute operation ceiling composes requests/pages/scans. A wait has a distinct
observation cutoff and bounded response allowance, neither extending that ceiling.
Early budget/transport/cancellation failure is incomplete/error, never valid timeout.
Do not stop early when one poll interval remains. Retain late observations and
censor them for the requested interval. Timeout means no matching observation under
this polling method, never continuous absence or a snapshot.

### C3 — Faithful translation and complete accounting

Keep four closed ordinary MCP tools, finite correlated errors, bounded pending/
active work and input/output, responsive cancellation/EOF. Preserve Mailpit matching
count versus mailbox total, derived Text on HTML-only detail and marks-read side
effect. Untimed raw fidelity uses independently authored plain/HTML/multipart
expectations; do not substitute raw bodies for ordinary MCP responses. Preserve
MailHog executable items/ID and matching-total observations beside its Swagger
discrepancy and missing v1 attribution. No extraction/cache/prefetch, synthetic
isolation/chaos, task hints or fixture-admin access.

Validate offsets, totals, unique IDs, progress, full nonfinal pages, aggregate bounds
and safe encoding. Malformed/truncated/duplicate/changing/short-nonfinal/over-budget
scans cannot succeed. Stable totals and unique IDs do not establish a snapshot
under equal-count churn. Final verification requires owned senders drained,
quiescent owned storage and independent expected-ID reconciliation. Live scans
report observations; unsupported atomic snapshots remain unsupported. List/search
MCP responses remain pages, not whole-mailbox claims.

Require requested = attempted + unstarted and attempted = accepted + rejected +
ambiguous with exclusive dispositions. Visible IDs, duplicates, known evictions
and timeouts are separate identity-linked sets. Retain every repetition/failure/
unstarted cell, both send/completion windows, scheduling, signed observation-minus-
ack offsets, sample sizes, nearest-rank quantiles, null empty quantiles and censoring.
No ambiguous DATA retries, favorable-run selection or timing pass thresholds.

### C4 — One surviving lifecycle owner

Corroborate both listeners against launched process identity plus protocol readiness.
A released port reservation is not ownership. Occupied/unknown/ambiguous endpoints
refuse; never adopt unrelated services. One async worker survives startup and workload
cancellation through TERM/KILL, direct reap, drain join, group/listener cessation,
identity recheck and scratch disposal. Parent requests stop and awaits it. Drop,
kill success, leader exit, EOF or timed-out join cannot issue a cleanup receipt.
Inspect nested join failures; retain ownership and stop further admission when
cleanup is unproved. Incomplete prefixes get no final index. Forced death/runtime
teardown requires separately confirmed cleanup. These are cooperative lifecycle
bounds, not hostile-host authentication or unconditional kernel-level cessation.

Preserve closed environments, exclusive 0700 leaves/0600 artifacts, no overwrite/
resume, bounded safe records and no raw diagnostics, credentials or private argv.
Benign real-process controls prove ownership and unrelated-resource survival;
no attack reproduction or widened access.

### C5 — Complete integration and retain external obligations

Reconcile conflicted eval results/artifacts/provenance and tests while preserving
both accepted behaviors and every assertion. Require standalone fmt, locked clippy,
full default-threaded tests and actual #148/#201 independent-process/checkout
shared-admission plus required CI receipts. Private locks or serialized/filtered
tests cannot satisfy accepted 0076. No new scheduler or scorer.

Integrate concrete Swarmail/Mailpit target typing and strict comparison envelopes
through current runner, fixture, provenance, retention, admission, oracle/report
and sole v2 scorer. Preserve native v2, historical baseline/readers, replay, seven
traps, public task inputs and within-trial isolation tests. Unsupported semantics
stay in the full catalogue/denominator, with a separately reported common subset;
no ninth failure class. Accepted AUTH is not isolated inbox semantics. Reconcile
#151/#152/#153 final MCP negotiation/output/batch contracts through actual client
evidence; local adapter tests cannot establish compatibility.

Implement every applicable external oracle/null control on both real targets,
working one-command integration, prerequisite-by-prerequisite evidence refusal and
operator-only measure delegation proved with free inert commands. Delivery modes
launch no paid trial; dry-run launches no trial and reads no credentials. Distinguish
smoke/local, controls, evidence and separately authorized measure. Genuine historical
#151 baseline, final full-product model trials and costs remain T11 external inputs.
T12 builds and validates available safe reporting now, but numeric dossier/README
agent claims wait for admitted genuine inputs. Unknown costs/tokens/turns are
unavailable, never zero or inferred actuals.

Carry the inherited HIGH security-relevant review-integrity residual with original
references and #219 separately. The supplied stop event names it but contains no
detailed original finding IDs; none were recovered in the supplied source packet.
Recovery remains T1, with HIGH/open status meanwhile. Do not reproduce attacks,
invent verdicts, widen boundaries or infer repaired pinned-engine behavior from
upstream merge. Preserve the requested mixed route and five independent protected
roles, worst verdict and hard security hold. 0090–0093 remain proposed; only the
operator reconciles/accepts. CI and candidate-bound engine verification remain
separate receipts.

## Acceptance Criteria

The inherited AC1–AC16 below remain normative and unchecked. C1–C5 above refine
their proofs. A draft or design-check result establishes none of them.


- **AC1 — Frozen method:** Complete committed protocol binds each run; validation
  refuses missing fields, altered hashes and post hoc subset/repetition changes.
  Setup-only outputs cannot become measurements.
- **AC2 — Real competitors:** All three targets execute with verified source/artifact/
  platform/license records. Wrong digest/version, missing server and mutable-only
  pins refuse; no silent skip or compat-shim substitute.
- **AC3 — Adapter:** Real Mailpit SMTP/HTTP/MCP tests prove ordinary list/read/search,
  body fidelity, pagination, identity limitations, chosen waits, timeout/error bounds
  and absence of fixture-admin/task-specific capabilities.
- **AC4 — Same-suite controls:** Applicable tasks retain fixture/scorer/difficulty
  and matching public inputs on both targets. All oracles pass and all nulls fail
  their declared idle class through the external path; every catalogue cell and
  unsupported/infrastructure case remains accounted.
- **AC5 — Throughput:** Real three-target runs reconcile attempts, accepts, rejects,
  ambiguity, visible/duplicate IDs; state units/windows and retain failures, corpus,
  resources, warmup and counts.
- **AC6 — Waits:** Real three-target present/future/empty tests never return unrelated
  mail; retain timeouts and signed offsets and reproduce quantiles without
  performance-based pass thresholds.
- **AC7 — Limits:** The 1,000-session profile is bounded; saturation/refusal/timeout
  remains data. Storage/cap differences are not pooled and exact-count workloads
  cannot knowingly overflow retention.
- **AC8 — Lifecycle:** Success, early failure, cancellation and death prove owned
  reap/drain/cleanup; uncertainty stops admission. Unowned resources survive and
  partial output receives no completion index.
- **AC9 — Privacy:** Benign real-process canaries prove closed environments and no
  child diagnostic/credential/private-argv leakage, private exclusive modes,
  collision refusal, bounded responses and safe rendering; existing tests retain strength.
- **AC10 — Schema/replay:** Strict native v2 behavior remains; comparison target
  typing, hashes, coordinates and replay reject mixed/foreign/duplicate/truncated
  evidence. Existing trap/provenance/concurrency proofs remain intact.
- **AC11 — One command:** Clean documented prerequisites reproduce local/integrated
  free checks and reports with one entry point; a separately authorized operator can
  rerun the full final product through its explicit measure mode. Failures identify
  the blocked phase;
  evidence mode never starts paid work or substitutes local-only success.
- **AC12 — Eval readiness:** Reconciled eval sources pass standalone fmt, locked
  clippy and full default-threaded tests, plus actual #148/#201 host-wide coordination
  and required CI receipts. Conflict markers/private locks are not proof.
- **AC13 — Genuine dossier:** docs/evals contains actual #151 baseline, actual final
  same-suite Swarmail/Mailpit trials, fresh valid controls, method, actual costs and
  safe raw data bound to exact versions/contracts/settings. Unsupported full-suite
  semantics or missing required costs/baseline leave this criterion open.
- **AC14 — Honest claims:** Every declared run/cell/loss appears with its denominator.
  Numeric README agent claims resolve to admitted data and reproducible arithmetic;
  measured business failures are evidence, not grounds to rerun.
- **AC15 — Guarantees/gates:** All house gates, exact coverage/selftest, publish/seal
  proofs, permissive licenses, fail-closed native-tls absence, benchmark build/run and
  independent eval/real-server checks have required receipts. No guarantee, historical
  spec, gate, coverage policy or dependency allowance is weakened.
- **AC16 — Delivery integrity:** Carry inherited HIGH security-relevant review-integrity
  findings unchanged into protected review; obtain original references without invented
  IDs/closure and retain #219 separately. Require the five independent mixed-route
  results and hard security hold. Source merge is not a published pinned engine.
  0090/0091 stay provisional; operator alone reconciles/accepts. No paid trial, push,
  merge, publication, tag or issue closure occurs in this delivery.

## Current completion ledger

All criteria remain open pending their own candidate-bound evidence. Preserve one
ledger in docs/evals/README.md during implementation; attach commit/tree, test or
admitted run identity, missing prerequisite and responsible owner per row. Historical
passes and source consistency checks cannot check an implementation task.

| Criterion | New proof focus | Current unresolved requirement / owner |
|---|---|---|
| AC1 | C1/C2 lock v3, protocol v2, complete frozen product | Implementer: schemas, actual profiles, freeze/admission proofs |
| AC2 | Tag chains versus build provenance; real executable identity | Implementer: bound captures/runtime tests; upstream build provenance stays unknown |
| AC3 | C3 bounded faithful adapter and C2 cutoff | Implementer: real SMTP→HTTP→MCP; final clients depend on #151/#152/#153 |
| AC4 | Same-suite external controls and unsupported denominator | Implementer after T7/T8; full isolation cannot be invented for Mailpit |
| AC5 | Full three-target profile and independent accounting | Implementer: retained complete runs, not historical smoke |
| AC6 | Common/native separation; present/future/empty and censoring | Implementer: real runs; best-wait evidence still missing |
| AC7 | Storage/enforcement/retention and bounded stress | Implementer: local actual profile; requested parity remains open |
| AC8 | Owned binds and surviving cleanup worker | Implementer: benign real-process negative proofs |
| AC9 | Existing privacy and no-clobber composition | Implementer: unchanged canaries, bounded output and safe reports |
| AC10 | Strict native/comparison typing and replay | Implementer: reconciled eval owners and preserved tests |
| AC11 | Working controls/evidence/measure delegation | Implementer: actual integration and independent refusal proofs |
| AC12 | Standalone full suite and shared coordination | Implementer/#148/#201 and CI: actual independent-process/checkout receipts |
| AC13 | Historical baseline, genuine finals and cost-backed dossier | Separately authorized operator/provider work; T11 external and T12 partial |
| AC14 | All losses/cells/denominators and numeric citations | Implementer: report validators now; genuine claims wait for T11 |
| AC15 | Complete locked gate inventory and unchanged guarantees | Implementer/verify/CI: new receipts; not performed by this design |
| AC16 | Original HIGH references, #219, engine, five reviews, acceptance | Operator/engine/panel/CI; no source-merge or design-pass substitution |

The inherited AC16 reference to 0090/0091 also applies to the continuation proposals
0092/0093. No existing proposal is accepted or renumbered. The exact chief file
allowlist excludes the decision index: T1 must add 0093's single proposed index row.
The resulting temporary inventory-gate gap is outstanding, not waived. Full house
gate success must not be reported before it and all other required work are proved.
