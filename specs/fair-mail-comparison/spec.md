# Fair mail comparison and eval evidence

Date: 2026-10-28. Status: draft. Epic #164, [issue #163](https://github.com/feedback-loop-ai/swarmail/issues/163).
Design base: main 7bf779f with mixed-route candidate 4919b31.

Decision: 0091 — [Fair comparison evidence](../../docs/decisions/0091-fair-mail-comparison.md), proposed; numbering awaits operator reconciliation.

## Purpose and evidence boundary

Build the complete real infrastructure and eventual evidence dossier for a fair,
reproducible comparison. Every agent claim must trace to genuine baseline/final
trials, every performance number to real server samples, and every loss to retained
accounting. A microbenchmark, adapter smoke or proposal alone does not satisfy #163.

The exact captured issue and four handoff checksums were verified at design. This
is a local capture, not a fresh GitHub observation. The requested run-local tasks
directory is absent; the retained intake result and both simplicity/robustness
positions supply the framing. Their rulings are preserved below without tracked
links into ignored run-local evidence.

There are three separately reported deliverables:

1. **Local infrastructure:** pinned real-server setup, bounded wire clients, throughput/
   wait benchmarks, thin Mailpit MCP adapter, records/reports and one command.
   Wire infrastructure must build independently of the conflicted eval crate.
2. **Eval integration:** the same fixture tasks and scorer through existing eval
   owners with Swarmail or Mailpit. Restored compilation and full coordination
   proof remain prerequisites; adapter-local tests can precede integration.
3. **Actual evidence:** genuine #151 baseline, genuine final Swarmail/Mailpit trials,
   controls, costs, safe raw data and numeric README citations.

At design, #148/#201 readiness and host-wide full-suite coordination are unproved,
eval source/tests contain conflict syntax, and #151 baseline and reconciled final
MCP contracts are absent. Local free tests are authorized; paid trials and external
publication are not. No scripted control, newer-server rerun or local pass may
become a historical baseline or full-story completion.

## Panel synthesis and rulings

| Question | Adopted ruling | Rejected alternative and cost |
|---|---|---|
| Architecture | Simplicity: existing eval owners, one wire driver, thin command, one evidence home. | No generic backend/plugin framework, extra crate, database, dashboard, scorer or supervisor. Fixed targets cost flexibility. |
| Accounting | Robustness: freeze every catalogue cell and applicability before collection; retain failures, ambiguity and unstarted work. | A common subset cannot replace the full comparison. More validation and visible unsupported tasks can block full same-suite acceptance. |
| Ownership | Robustness's bounded I/O, observed cleanup and manifests within simplicity's owners. | Unchecked shell exit codes cannot prove evidence/lifecycle. Permanent failure-path tests cost maintenance. |
| Adapter | Simplicity's upstream translation constrained by robustness's verified capabilities and identity tests. | No extraction/index/cache, task hints or synthetic inbox/chaos equivalence. Some tasks may remain unsupported. |
| Stress | Preserve existing crash/cap/chaos/MIME/TLS/webhook regressions, test new boundaries, add a separate bounded 1,000-session profile. | Reject a new cross-product crash/webhook/TLS research suite or product rewrite. No new durability/outbox/TLS-equivalence claim. |
| Completion | Both positions: local proof and actual evidence admission are separate. | No fabricated baseline/cost stubs or provisional README wins. Sequential work is slower; full claims wait. |

Preserve accepted [0001](../../docs/decisions/0001-synchronous-insert-lossless.md),
[0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md),
[0003](../../docs/decisions/0003-oldest-only-pruning.md),
[0004](../../docs/decisions/0004-per-test-inboxes-via-smtp-auth.md) and
[0005](../../docs/decisions/0005-extraction-at-ingest.md). No comparison fast path
changes acceptance, wait ordering, pruning, AUTH identity or ingest extraction.

## Requirements

### R1 — Freeze the experiment before collection

Commit a versioned protocol fixing the entire catalogue/task-definition/scorer
hashes; baseline/final source and executable identities; models/declarations and
measured family tool asymmetries; negotiated MCP revisions/output forms; adapter
schema/revision; fixture seed/time/derivation; target versions/settings; capability
and applicability evidence; corpus; counts; order; warmup; sessions; deadlines;
resources; storage/caps; statistics and failure handling. Bind its bytes and commit
in every run. Unresolved required values refuse collection. Untimed setup/correctness
probes are separate. Changed methods produce new protocols/runs with a change reason;
retain old runs, no best-rerun selection or automatic retries.

Use only official upstream repository/release/API/setup sources for competitors.
Record source URL/revision, document identity, exact release, artifact SHA-256,
platform/architecture and license evidence; images bind manifest and platform
digest. Tags alone, third-party recipes and Swarmail compatibility endpoints do not
prove competitor identity/capability. No competitor version, license or capability
is asserted verified by this design.

### R2 — Same tasks and a thin adapter

Reuse the unchanged fixture, difficulty, seven-trap precedence and sole app-state
scorer under [0055](../../docs/decisions/0055-agent-eval-harness.md) and
[0071](../../docs/decisions/0071-eval-trap-scoring.md). Freeze every requested
catalogue cell, including unsupported cases. Verify SMTP AUTH/address/inbox semantics
by real sends with competing identities; accepted AUTH is not proof of isolation.
Retain #151's measured client restrictions/asymmetries, not a fictional common
client surface.

The Mailpit MCP adapter exposes only general upstream list/search/read and justified
wait translation. Preserve text/HTML, filters, pagination and bounded errors. Add no
Swarmail extraction fields, synthesized attachments, cache/index/prefetch, task hints,
fixture tokens/admin access, inbox emulation or chaos. Inspect official wait/stream
facilities before choosing polling. If polling is necessary, freeze its justified
interval, immediate scan, bounded pagination, connection reuse, request counts and
deadlines before collection; include overhead and label adapter observation latency.

Keep goals identical except declared connection/tool instructions. Oracle transport
adaptation may read ordinary bodies to execute the same task, but must not teach the
adapter answers or access private admin state. Oracle/null traverse the real external
command path; null remains inert. Unsupported capabilities are comparison-plan
entries, not fabricated trial rows or oracle passes. Unexpected setup failures are
infrastructure failures, not business losses. Report full requested accounting and
a predeclared common subset separately. If faithful full-suite semantics cannot be
achieved, leave that acceptance criterion open.

### R3 — Real wire benchmarks

One independently buildable driver in benches runs actual Swarmail, Mailpit and
MailHog servers against the same bounded authored plain/HTML/multipart corpus,
message identifiers, session schedule and comparable resources. Verify byte/field
fidelity outside timed work. Separate memory/durable and capped/uncapped profiles;
disclose unmatched settings.

Record attempted messages, final-DATA accepts, explicit SMTP rejects, ambiguous
post-terminator outcomes, verified unique visible IDs, duplicates, expected evictions,
elapsed seconds and client scheduling delay. Accepted throughput is accepts divided
by the timed send window; verified completion throughput is accepted-and-observed
IDs divided by the window through verification completion. Publish both windows.
No ambiguous-send retry or retry-until-N-success. Identity/count mismatches fail the
exact-count profile and stay in data. Caps exceed its workload or are explicitly
disabled. Competitor acceptance is not presumed synchronous storage.

Wait profiles cover already-present, future delivery and empty timeout. The common
controller records monotonic invocation, send start, receipt of final DATA response
and matching observation. Future-mail latency is observation minus send start;
present-mail latency is observation minus invocation. Also retain signed
observation-minus-acknowledgement offsets; observation can precede receipt of 250.
Never clamp or take absolute values. Client launch is not server subscription
readiness. Separate native wait and common-client/adapter observations. Report
nearest-rank p50/p95/p99 with sample sizes and censored timeout counts; empty success
sets have null quantiles. No tight timing threshold is a correctness assertion.

Freeze a separate 1,000-concurrent-session bounded profile; inability to finish,
saturation and refusal remain results. Wire concurrency does not authorize
concurrent eval trials.

### R4 — Ownership, privacy and records

Own all loopback listeners, fresh data/output leaves and processes. Protocol readiness,
sessions, requests/pages/bodies, output, deadlines and resource ceilings are bounded.
One owner retains children through termination, reaping and drainage on success,
failure, cancellation and partial startup. Clean only owned resources; never clear
arbitrary endpoints or kill host-wide processes. Unproved cleanup is infrastructure
failure and stops later work. Forced death leaves incomplete evidence; lock release
does not establish cessation.

Preserve [0075](../../docs/decisions/0075-eval-privacy.md): closed child environments,
fresh private HOME/TMPDIR/XDG, no ambient credentials/proxies/config, discarded bounded
child diagnostics, finite safe errors, exclusive 0700 leaves/0600 artifacts and
no-clobber publication. Raw evidence means typed measurements and allowed synthetic
fixture/scoring state, not transcripts, raw child output, credentials, arbitrary
mail HTML or private argv. Render validated fields safely. Public corpus bytes are
deliberately authored fixtures, not user mail.

Retain a prelaunch manifest and streamed prefix; finalize the hash index only after
cleanup and validation. Partial/interrupted/malformed evidence stays unchanged and
incomplete. Missing/extra/duplicate/mixed/foreign/truncated rows refuse full admission.
Diagnostic accounting may describe incomplete runs without becoming a completed
report. No resume, overwritten artifacts or invented suffix. Hashes establish
consistency, not hostile same-uid authenticity or fsync durability.

### R5 — Accepted eval contracts and explicit target typing

Preserve [0063](../../docs/decisions/0063-eval-provenance.md) identity, fixture-input
and replay rules and [0076](../../docs/decisions/0076-sequential-eval-trials.md)
admission before work through cleanup/finalization. Every trial remains unconfined
with concurrency one; multi-agent isolation inside a trial remains supported.
A private lock or test-threads=1 is not #201's cross-process/checkout, default-threaded
full-suite proof.

Native v2 results/readers and baseline behavior stay unchanged. Add an explicitly
discriminated comparison envelope with typed target identity/settings and common
v2 business facts; never label a Mailpit executable or settings as Swarmail. Reuse
execution/scoring/validation owners, not a shadow harness. Native readers reject
comparison envelopes; comparison readers preserve strict common v2 and snapshot
validation. Unsupported capability is applicability, not a ninth failure class.
0091 proposes this new contract; any contradiction with accepted rulings waits for
operator acceptance. Preserve genuine baseline bytes and their historical readers;
no migration or pooling incompatible scores.

### R6 — One command and the eventual dossier

Implement bash scripts/compare-mail.sh with explicit local, controls and evidence
modes (plan.md). Select preinstalled pinned prerequisites in a documented clean
environment; refuse absent/mismatched tools and never silently install or start paid
agents. Local mode proves independent infrastructure. Controls mode adds integrated
free evals when prerequisites build. Evidence mode admits existing genuine authorized
runs, executes the frozen local benchmark workflow and regenerates derived reports.
Missing #148/#201 proof, baseline, reconciled contracts, fresh controls, actual runs
or delivery receipts refuses full mode; it cannot downgrade to local success.
For full rerun reproducibility, an explicit operator-only measure mode in the same
entry point delegates to the existing eval owner after separately granted authority,
valid baseline/contracts/gate receipts and fresh controls. It uses existing private
wrapper selection, creates fresh outputs and never rewrites the baseline. This
delivery only builds/proves that path with free inert controls; it executes no
paid trial. Dry-run performs no trial launch or credential read.

Eventually docs/evals holds method, source/license lock, evidence index, genuine
baseline/final safe raw bytes, manifests, reports, actual cost accounting and all
losses. Preserve original admitted bytes/hashes. Cost records actual accounting
source/basis/currency, safe provider/journal usage evidence and pricing version/date
where relevant. Distinguish charges, usage and estimates. Unknown cost/tokens/turns
are unavailable, not zero or inferred from time; missing required cost evidence
keeps completion open.

README numeric agent claims cite exact run/task/model/revision/denominator and
limitations. Losses and unsupported coverage appear beside wins. Internal ingest
microbench numbers remain separate from network/agent performance. Preparing
tracked evidence is not authorization for external publication.

## Acceptance Criteria

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

## Completion accounting

These are obligations, not achievements asserted by this draft. AC1–AC3 and AC5–AC11
have substantial independent local work; AC4 needs restored eval compilation.
AC12–AC14 require prerequisite and genuine evidence receipts. AC15–AC16 govern all
delivery. Report each separately: design validation and local green subsets do not
mean full-story completion.
