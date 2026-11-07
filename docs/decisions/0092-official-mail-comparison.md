# 0092 — Official comparison provenance and bounded translation

- **Status:** accepted (operator, 2026-11-02 — "accept all as recommended",
  given in chat to the dispatcher).
- **Date:** 2026-10-28
- **Issue:** Epic #164 / #163, continuation of stopped candidate 4a116dbc2ab0b8851eefb4ce8c2a8804dc619f01
- **Specification:** [Official-artifact comparison](../../specs/official-mail-comparison/spec.md)
- **Supplements:** proposed [0091](0091-fair-mail-comparison.md), without accepting it; explicit provenance and bounded Mailpit adapter/measurement contracts.
- **Preserves:** accepted [0001](0001-synchronous-insert-lossless.md), [0002](0002-awaits-subscribe-before-checking.md), [0003](0003-oldest-only-pruning.md), [0004](0004-per-test-inboxes-via-smtp-auth.md), [0005](0005-extraction-at-ingest.md), [0055](0055-agent-eval-harness.md), [0063](0063-eval-provenance.md), [0071](0071-eval-trap-scoring.md), [0073](0073-matrix-cancellation.md), [0075](0075-eval-privacy.md), [0076](0076-sequential-eval-trials.md), exact gates and dependency policy.
- **Number:** next after 0091 in this checkout, provisional across parallel work; operator reconciles and accepts.

## Context

Official provisioned bytes make competitor work possible but expose distinctions
the stopped driver's source and method contracts cannot honestly collapse. The
Mailpit release metadata supplies an archive digest; MailHog supplies none. Its
local digest is useful integrity evidence, not an upstream-published checksum.
Release target_commitish values are branches, not full source commits. Tagged API
bytes are not proof of executable response shape, setup settings or stream absence.
The supplied MailHog v2 Swagger disagrees with observed listing fields and does
not describe the v1 detail endpoint. Mailpit search matches differ from mailbox
totals, and detail read marks a message read.

A thin adapter is a new public experiment interface. Its provenance, bounded
partial/error results and method labels need a proposal even though no frozen
server guarantee changes. This record neither accepts 0090/0091 nor authorizes
weakening any accepted ruling. Compatible implementation may proceed under
0045 §8; contradictions wait for explicit operator acceptance.

## Proposed ruling

1. Distinguish archive/asset and executable digests, upstream-metadata versus
   locally computed digest basis, release/platform, full source revision, captured
   document bytes and acquisition attribution. Use source-lock v2 for these required
   fields; do not silently reinterpret v1. Retain old bytes and historical readers.
   Missing required source/setup/wait facts block the corresponding collection.
   Untimed exploratory probes remain possible and can never be admitted as scores.
   Hashes prove local consistency, not adversarial same-host authentication.
2. Extend the existing protocol explicitly to v2 where per-target storage,
   resource enforcement and operation budgets require new required fields. Freeze
   it before collection, binding the source lock and exact adapter/MCP schemas.
   Preserve the full product, every old run and all unsupported cells. An unmatched
   storage or unenforced-resource profile is separate and plainly labeled; it does
   not satisfy a requested matched/enforced profile. No relabeling or post hoc tuning.
3. Keep one bench executable and one cohesive Mailpit adapter module using existing
   HTTP and lifecycle owners. The local contract is MCP 2025-03-26, text content
   containing JSON, closed list/search/read/wait tools and bounded correlated errors.
   Final client negotiation/output evidence remains a prerequisite, not inferred
   from this choice. Only source-verified general operations may be exposed. Wait
   requires explicit source review; no task hints, extraction/cache/index/prefetch,
   synthetic inboxes/chaos or fixture/admin operations. Read's side effect remains
   documented and tested; unsupported tasks retain their full-suite denominator.
4. Target-specific decoders distinguish list totals, matching totals and page
   counts, preserve ordinary bodies, and encode all path/query inputs. Bound per-I/O
   and aggregate bytes/pages/requests/deadlines, pending/in-flight work and output.
   Incomplete/nonprogressing/mutating scans never become empty or complete success.
   Use actual pinned binary tests and independently authored expected bodies;
   preserve source/runtime discrepancies and unresolved primary attribution.
5. Prefer common-client immediate-scan polling as an explicitly labeled observation
   method, with frozen interval/request costs. Inspect primary wait/stream evidence
   before choosing the best available/adapter mechanism. If streams are justified,
   subscribe before initial scan, prove readiness and rescan after gaps; otherwise
   keep the profile blocked. Swarmail's native subscribe-before-check remains
   separate. Retain signed ack offsets, censored timeouts and null empty quantiles.
6. Retain one async owner through cancellation, reap/drain, group/listener cessation
   and scratch disposal. Corroborate owned bind/readiness; child liveness alone is
   insufficient. Unknown ownership/cleanup refuses further admission and completion
   indexes. Drop may best-effort kill but cannot issue a cleanup receipt. Forced
   death stays incomplete pending independent cleanup confirmation. This is bounded
   cooperative lifecycle ownership, not a confinement platform.
7. Preserve every inherited AC1–AC16/T1–T14 and all external prerequisites. Reuse
   accepted native v2/scorer/replay/privacy/sequential eval owners; no shadow harness,
   ninth failure class or baseline rewrite. #148/#201, #151, final contracts, genuine
   final trials/costs, CI, published engine and independent protected review remain
   distinct requirements. No paid work, publication, push, merge or acceptance occurs.

## Synthesis and consequences

Adopt simplicity's narrow existing architecture and reject speculative frameworks,
extra dependencies and a new crash/TLS/webhook campaign. The cost is fixed targets
and limited extension points. Preserve production guarantee regressions; no storage
fast path, webhook durability invention or ingest-extraction bypass is authorized.

Where simplicity's minimal implementation conflicts with robustness's evidence
boundary, robustness wins: fail incomplete scans, require corroborated readiness,
retain cleanup ownership and explicit provenance. This costs more validators,
real-process tests, visible failures and delayed claims. These checks stay within
the existing owners, not a second supervisor or trust service.

Reject machinery to manufacture resource/storage parity. Accept a separately frozen
honest local profile or a blocked matched profile. Polling may be smaller but cannot
claim native wait behavior or best available performance without source review.
Missing evidence and unsupported semantics can leave full same-suite acceptance open.

## Proof and delivery

The linked spec and ordered paired tasks require actual SMTP → competitor HTTP →
MCP tests, all three wire targets, independent accounting, lifecycle/refusal proofs,
existing regressions, full standalone eval checks and the unchanged house inventory.
The design commit touches only its three spec files and this proposed record.
Implementation T1 adds the one proposed 0092 decision-index row and proves the
inventory; until then that gate is outstanding, not waived. 0091 already has its
row. Structural design validation cannot stand in for whole-candidate verification.
Commit unsigned; operator alone accepts/reconciles decisions and seals for merge.

## Dated addendum — 2026-11-02 (at acceptance)

The operator accepted this record on 2026-11-02. The text above is unchanged
and remains the record of what was proposed. This addendum states what has
moved since it was written, checked against `main` at `c336c95`; it rules
nothing beyond the acceptance.

- **The tooling shipped; the evidence is owed.** `44175fe` (#287) landed the
  wire driver (`benches/mail_compare.rs`), the Mailpit adapter
  (`benches/mail_compare/mailpit_mcp.rs`), `scripts/compare-mail.sh` and the
  comparison row type (`evals/src/identity.rs`).
  [`docs/evals/README.md`](../evals/README.md) records the controls (T9.a)
  as unstarted and no comparison evidence, final contract, final trial or
  cost record; its rows are open, not waived. No paid trial is authorized.
  Acceptance makes no comparison claim.
- **v3 is the current lock.** The v2 source lock this record introduced is
  kept as history (`docs/evals/history/2026-10-28-sources.lock.v2.json`);
  the lock in use is `docs/evals/sources.lock.json`, `lock_version` 3, under
  [0093](0093-source-mail-comparison.md).
- **The adapter falls short of §3's contract.** §3 makes the adapter's local
  contract MCP 2025-03-26. `docs/evals/README.md` records that the adapter
  refuses JSON-RPC batches, which falls short of 2025-03-26, and that this
  stays open with the final contracts.

## Dated addendum — 2026-11-07 (the operator's widening, issue #339)

This addendum only adds. The text above, and the 2026-11-02 addendum, are
unchanged. It records the operator's ruling of 2026-11-07 on #163, quoted in
issue #339: "compare with all that we have shims for". Authority is that
ruling, not this note.

- **Four targets, four agents, no GLM.** Swarmail, Mailpit, MailHog and
  MailSlurper, on claude-opus, claude-sonnet, codex-sol and codex-luna.
  There is no GLM (spark is down; the DashScope GLM quota is out until
  2026-11-22).
- **The MailHog agent-eval rejection of
  [0091](0091-fair-mail-comparison.md) does not apply to this story.** The
  ruling lifts it, and the comparison row's refusal of MailHog follows.
- **MailSlurper joins under §3 unchanged.** Its adapter is held to the same
  contract as Mailpit's: source-verified general upstream operations only,
  bodies unchanged, no Swarmail-only capability emulated, no hint, no cache.
- **The batch gap closes in this story.** The 2026-11-02 addendum records
  that the adapter refuses JSON-RPC batches, short of MCP 2025-03-26. Issue
  #339 closes that gap in every adapter, under the bound proposed in
  [0129](0129-agent-suite-comparison.md) §6.
- **Contracts** that implement the ruling are in proposed 0129; this
  addendum rules nothing beyond recording the operator's words.
