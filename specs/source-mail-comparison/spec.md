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

## Clarification — 2026-10-29, operator continuation

Run `issue-163-operator-continuation--bb90d392`, base
`47c1599b2bc9ee55dea87cd20ea61ede628036c6` (descends from 14217f7; T1, T3, T5 and
the lock-v3/draft-3 half of T2 landed in 3b02d9d, 68b06b8, 5cf2729, 9d161cc and
47c1599, each reported `broken`, that is, incomplete). This entry encodes the second
design panel's rulings for the work that remains. It adds no spec family, decision,
module, crate, feature flag or dependency. It revises only the plan and task text it
makes obsolete; the rulings above, the inherited AC1–AC16, 0090–0093 (all still
`proposed`) and every historical spec, lock and receipt stay unchanged. 0093 remains
the one declared decision: every change below is a compatible refinement of its
§3–§7 and moves no frozen guarantee, accepted ruling or public server contract, so
no new record is drafted and no colliding number is created.

The two positions were read in full. They agree on owners, fixed targets, no new
machinery, no `src/` change and no claim of equivalence, re-pin or residual closure.
They disagree on four points, each ruled below with what the losing side costs:
whether a prose invariant is enough or the code must carry one law (Q3, robustness
wins); whether the advertised wait maximum may exceed what the frame can honour
(Q3, robustness wins, simplicity's "no contract change" loses the 30 s maximum);
whether T8–T10 get orchestration or named refusals (Q8, simplicity wins, robustness
loses nothing it asked for); and whether to add a reservation-race seam (Q5,
simplicity wins, robustness's forced-race proof stays a recorded residual).

### Q1 — Which roster and engine govern this delivery?

Accepted decision 0099 (claude-careful, operator, 2026-10-29, recorded with 0098 on
branch `repair/claude-careful-recipe`; neither file exists on this branch and neither
is imported, cherry-picked or linked here) governs: Haiku 5.5 intake; Opus 5.5,
official DeepSeek Flash and Fable 5.1 design; Sonnet 5.5 implementation; five
protected reviews, Fable correctness, Flash security, Flash guarantees, local Spark
GLM-5.3-Flash tests and Opus independent. T14's Terra/Astra/Codex roster is
historical; no seat uses Codex. The run executes on host engine brokkr a9893b4
(brokkr decision 0096, published `swarmail-engine-a9893b4-p3`). The candidate keeps
its pin to 731189f-p3: `bundles/self/tooling.json` binary SHA-256
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`, which the panel
re-hashed equal to `.forge/bin/brokkr`. This run does not re-pin. No a9893b4 bytes,
receipt or decision text exist in the worktree, so a re-pin could not be
digest-checked, would break `shims.sha256` discipline and would put a tooling route
change inside a comparison feature. `brokkr compile` therefore proves the pinned
engine, and every record says which engine it names.

### Q2 — Were the cited specs absent?

No. Intake's `docs/specs/...` line is a path slip: 0091 and 0093 link
`../../specs/...`, and `specs/fair-mail-comparison/`,
`specs/official-mail-comparison/` and this directory exist and were read. No
absence row is recorded anywhere.

### Q3 — One frame law, and a `late` that can happen (adopts robustness D1–D2)

**Finding.** `mailpit_mcp::Frame::new` sets `ceiling = admitted + operation`,
`cutoff = admitted + wait`, `response = min(ceiling, cutoff + request)`.
`runner::observe` hand-builds a second frame (`ceiling = response = cutoff +
step`). With the defaults (operation 30 s, request 10 s, `MAX_TIMEOUT_MS` 30 000)
the adapter's response allowance shrinks silently from 20 001 ms upward and is zero
at the advertised maximum, where `cutoff == ceiling == response`: the `late` status
the tool description advertises cannot occur, and a scan in flight at the cutoff
becomes `Budget("operation deadline")`, an error. The committed test pins exactly
this ("a 30 s wait under the 30 s ceiling gets no extra 10 s"). The plan's sentence
"for 30 s wait and 30 s ceiling, R=O, so there is no extra 10 s" is obsolete.

**Ruling.**
1. Exactly one constructor. `runner::observe` declares the driver's bounds
   (`operation = deadline + request_deadline`, `request = request_deadline`, the
   frozen poll interval) and calls `Frame::new`. The instants are the ones it
   computes today; the law is now single-source and the driver's operation bound is
   a declared protocol v2 field, not an arithmetic accident.
2. The constructor asserts, and a test proves, for every admitted wait:
   `cutoff <= ceiling`, `response <= ceiling` and `cutoff + request <= ceiling`, so
   the response allowance is always the full request bound. A wait that cannot
   satisfy it is refused at argument validation, never clamped.
3. Consequently the adapter's admitted `timeout_ms` maximum is `operation − request`
   = 20 000 ms, derived from `Bounds` in one place; the schema maximum, the
   `tools/list` bytes, the README row, protocol v2's bound table and the frame test
   change together. `late` is reachable at every admitted timeout, including the
   maximum, and a scan in flight at the cutoff gets its full request bound.
4. Timeout is still answered at or after the cutoff, never because the next interval
   would cross it; late observations stay recorded and censored.

**Rejected: widening the operation ceiling to 40 s** to keep the 30 s maximum. It
widens the aggregate budget of all four tools and the queue exposure of every call
to preserve a promise no genuine client has relied on (#151 is unresolved).
**Cost of the ruling:** an agent that wants 30 s of observation makes two calls, each
visible in the trial record; simplicity's "no contract change" is overruled because
a schema that offers a value its frame cannot honour is the quiet wrong number this
spec exists to refuse. The wire profile's 30 s `deadline_ms` is untouched: the driver
declares its own 40 s operation bound, so C2's retained bounds hold.

### Q4 — Scans: exactly one complete walk per decision, no repair inside it

Adopted (robustness D3). `complete_search` and the driver's scan keep every refusal
class (duplicate ID, changed count, short or empty nonfinal page, non-progressing
page, over-budget pages, overshoot, wrong `start` echo). A scan is never retried or
restarted mid-walk: in the `future` profile a send landing between two pages
legitimately changes the count, and that sample records `Incomplete`/error, never
a retry until the race resolves in the harness's favour. "A complete walk is not a
snapshot" stays the user-facing wording.

### Q5 — Lifecycle: abort before every bounded join; no new seam

Verified in this panel at 47c1599: `cleanup` aborts and awaits a drain the budget
cuts off and counts it unproved; `Keepers::settle` aborts a timed-out worker and
counts it unproved; `GroupGuard` kills the group on forced death and keeps scratch.
Robustness D4(c) is therefore met in code. What remains from the T3 residual: the
drain-join-failure arm (a panicking drain, injected through the existing
`#[cfg(test)]` hook) gets its own test proving `reaped == false` and a withheld
index. Robustness's request for a forced reservation-race seam is **rejected**
(simplicity wins): the foreign-bind refusal is proved through the kernel reading it
rests on, a production seam for a race the design already refuses would add
surface without changing any verdict, and the residual stays recorded, not waived.

### Q6 — The denominator is frozen before the first launch

Adopted (robustness D6). `runner::plan` declares every coordinate before anything
starts; `records::admit` must recompute the plan from the protocol and refuse a run
whose row set is a strict prefix of it, whose index arrived before every cleanup
proved, or whose report disagrees with the rows. A test kills a run after the first
leaves and proves "missing coordinates, no index", never a smaller denominator.

### Q7 — Fidelity is bytes on the raw path

Adopted (robustness D7). Raw fidelity compares the stored bytes the target returns
with the sent bytes; the derived-Text check on the MCP path is a separate
comparison with independently authored expectations. If `runner::fidelity` or
`wire::same_body` compares through a lossy `String`, the implementer changes it to
bytes and adds a non-UTF-8 body case. Percent-encoding, the 64 KiB line bound and
incomplete-DATA-stores-nothing keep their tests. The comparison path stays
loopback plaintext (D8); TLS is unmeasured and said to be.

### Q8 — What the remaining tasks build, and what they refuse to build

Adopts simplicity D2–D6 for scope, with robustness's cap-overflow refusal and
accounting split carried into the same owners.

- **T2, protocol v2.** `protocol.json` becomes an honest `"protocol_version": 2`
  (draft-3 bytes archived under `docs/evals/history/` as draft-2 was). It declares,
  as data per target, the established storage/cap/enforcement facts: Swarmail's cap
  and oldest-only prune as shipped (0003 unchanged); Mailpit `--database` scratch
  SQLite with `--max 10000`; MailHog `-storage memory` with no cap claimed unless a
  finite-cap probe shows one. It names the common 100 ms poll, the driver's wait
  operation bound (Q3), the adapter's derived bounds, and every unmatched or
  unenforced profile under its own name. Validators in `protocol.rs` refuse a
  changed method, schema or product, false parity, a mutable pin, and an
  exact-count workload whose cumulative residency (warmup + measured + wait sends
  per reused instance) could exceed a declared cap, before any launch. Finite-cap
  probes live in the existing `#[ignore]` real-binary tests. Best-wait stays
  `blocked`; no record says "no streams" or treats a dropped notification as loss.
- **T4, accounting.** `requested = attempted + unstarted`, `attempted = accepted +
  rejected + ambiguous`; `unstarted` (send-deadline cut) is never merged with a
  connect or terminator failure. Visible/duplicate/eviction/timeout stay separate
  identity-linked sets. The stress profile reports actual caps and
  scheduled/admitted/refused counts. Malformed-page classes get one finite fake
  responder test each. Full frozen runs happen only after T2 freezes in this run,
  through `scripts/compare-mail.sh`; otherwise the ledger says "collection not
  started", never a smoke relabelled.
- **T6 and T10, dispatch.** The generic `comparison-integration` refusal becomes
  one named refusal per missing prerequisite in T10's order: baseline, final
  contracts, controls, coordination receipts, cost basis, identity, full product.
  Each has a negative test that removes or corrupts the input and reads the name.
  `measure --dry-run` prints the order and opens nothing. **Rejected:** a scheduler,
  resume, retry, best-run selection or measure engine; with every input absent an
  inert orchestrator is untested-in-anger surface that T10 itself says never
  substitutes for evidence.
- **T8 and T9, agent side.** Target discrimination is one variant and its
  validation in the existing `evals/src/identity.rs` and `results.rs`: native rows
  reject comparison, legacy or mixed rows; comparison rows carry identity, settings
  and hash. `evals/src/comparison.rs` and `evals/tests/comparison.rs` are **not
  created**. Free oracle/null controls run on the two agent targets, Swarmail native
  MCP and Mailpit through the four-tool adapter. MailHog has no agent adapter; its
  agent cells are `unsupported`, never invented. If the eval runner cannot drive the
  stdio adapter without new machinery, T9 is recorded unstarted with the named
  missing piece.
- **T12, reporting.** `report.rs` and `stats.rs` extend only to cite
  run/coordinate/denominator for genuine wire data. The paid dossier and README
  agent numbers refuse without T11; no placeholder trial, no zero standing in for
  unknown spend.
- **No `src/` change**, no change to `tests/{burst,p2,rate}.rs`, Cargo manifests or
  locks, `deny.toml`, `bundles/` or the tooling pin. The exact 100/100/100 gate's
  denominator is unchanged and still run with its selftest.

### Q9 — The review-integrity residual against a9893b4 (AC-12)

Examinable from inside the box, and recorded as one ledger row in
`docs/evals/README.md`: the pinned engine 731189f-p3 and its digest equal to
`.forge/bin/brokkr`; the host engine identity a9893b4 as the task asserts it; the
absence from the worktree of any a9893b4 bytes, build receipt or decision 0096 text;
the stop event's mention of the residual without original finding IDs; #219 as a
separate open item. Not examined, and said so: the a9893b4 engine itself, 0096,
the original HIGH finding references. The row states examined and not-examined
explicitly; it records no verdict, no closure, no upstream-merge repair and no
re-pin. The HIGH residual and #219 stay open into the five-member protected review.

### Q10 — What this run can honestly finish

Local and free: T2 freeze, T4 accounting and, if the freeze is clean, the first
frozen three-target wire collection; T5's frame law and derived maximum; T3's
drain-join arm; T6/T10 named refusals; T8's enum arm; T9 on the two agent targets
where the runner can drive them; T12 arithmetic; T1's residual, roster and engine
rows; T13 on the candidate. External and unchanged: #148/#201 receipts, #151
baseline and final MCP contracts, T11 paid trials and costs, binary build
provenance, operator acceptance of 0090–0093, CI receipts, the five protected
reviews and the security hold. A `complete` result is not available to this run;
the honest terminal state is every local task proved and every external item
named open.

### Acceptance criteria added 2026-10-29

Each is testable on the candidate; none is established by this draft.

- **AC-C1 — One frame law.** `Frame::new` is the only `Frame` constructor in
  `benches/`; `runner::observe` calls it with declared bounds. Test: a search for a
  second constructor fails; a paused-clock test asserts the driver and adapter frames
  agree for equal inputs.
- **AC-C2 — Full allowance at every admitted wait.** For every `timeout_ms` in
  `1..=operation − request`, `cutoff + request <= ceiling` and `late` is produced by a
  scripted scan that completes after the cutoff and before the response bound,
  including at the maximum. Test: paused clock, the existing scripted scan.
- **AC-C3 — Derived maximum.** The schema maximum, argument validation and
  `tools/list` bytes all equal `operation − request` (20 000 ms with the frozen
  bounds); `timeout_ms = 20 001` is refused with a fixed reason; the README row and
  protocol v2 carry the same number. Test: schema snapshot and refusal test.
- **AC-C4 — Protocol v2 frozen and strict.** `protocol_version` is 2, status
  `frozen`, `unresolved` empty for the declared profiles; validators refuse a changed
  method, schema, product, false parity, mutable pin and cumulative cap overflow
  (one test each); draft-3 bytes are byte-identical under `history/`.
- **AC-C5 — Accounting identities.** Every sealed run satisfies the two sums with
  exclusive dispositions; `unstarted` and connect/terminator failures are distinct
  columns; the stress profile reports scheduled/admitted/refused. Test: independent
  recomputation from rows in `tests/mail_compare.rs`.
- **AC-C6 — Denominator before launch.** `admit` recomputes the plan and refuses a
  strict-prefix row set and an index written before every cleanup proved. Test: a
  killed run leaves missing coordinates and no index.
- **AC-C7 — Drain-join arm.** An injected panicking drain yields `reaped == false`,
  `proved() == false`, stopped admission and no index. Test: the existing
  `#[cfg(test)]` hook, real child.
- **AC-C8 — Named refusals.** `controls`, `evidence` and `measure` each refuse with
  the first missing prerequisite's name in T10's order; `measure --dry-run` prints
  the order and launches nothing. Test: one negative shell test per prerequisite
  and an inert witness for dry-run.
- **AC-C9 — Agent-side typing.** Native readers reject comparison, legacy and mixed
  rows; comparison rows validate identity, settings and hash; MailHog agent cells
  read `unsupported`. Test: standalone eval suite, whole and default-threaded.
- **AC-C10 — Honest ledger.** `docs/evals/README.md` has the roster/engine row (Q1)
  and the residual row (Q9) with examined/not-examined stated, no closure
  vocabulary, and no absence row for the specs. Test: review reads the rows; the
  decision inventory and link tests stay green.
- **AC-C11 — Unchanged boundaries.** `git diff --stat 47c1599..HEAD -- src tests/burst.rs
  tests/p2.rs tests/rate.rs Cargo.toml Cargo.lock deny.toml bundles docs/evals/history
  docs/decisions/009[0-3]*` is empty; commits are unsigned; nothing is pushed.
