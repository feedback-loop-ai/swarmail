# 0067 — Public webhook certificates and off-worker CA construction

- **Status:** accepted (operator, 2026-10-24)
- **Date:** 2026-10-24
- **Scope:** epic #192, issue #195.
- **Specification:** [webhook CA safety](../../specs/webhook-ca-safety/spec.md).
- **Clarifies/extends:** accepted [0041](0041-webhook-boundary.md) §§2–3 and [0048](0048-rest-webhook-validation.md) §§1/3: public certificate diagnostics and CA construction placement.
- **Preserves:** accepted admission/delivery rules, 0001–0005, Store panic symmetry in 0023, 0041 shutdown/accounting and 0048 offline complete-root validation.
- **Number:** Corrected to 0067 after the operator reserved 0062–0066 for other records. Its original branch-local number 0062 remains only in historical run evidence.

## Context

Accepted 0048 §1 admits unrelated private-key sections beside loadable roots.
Copying that input as public CA material discloses it through GET/PUT. Selecting
CERTIFICATE labels alone can expose relabeled private DER when an embedder
bypasses PUT. Cold delivery clients also build on async workers under a global
mutex. Repairing only thread placement or only lock scope leaves the other
stall. New awaits require explicit publication and failure-accounting rules.

## Proposed clarification and execution contract

1. Public `ca_pem` means canonical, delivery-stack-loadable certificates, not
   original input. Decode/re-encode every certificate in order with duplicates;
   validate the whole reconstructed bundle through the uncached builder. Never
   copy unrelated blocks/prose. Absent stays null; supplied unprojectable input
   yields an empty string on any parse/load failure, without partial/raw fallback.
   Keep response keys, secret withholding and URL-userinfo removal. Covert data
   inside loadable certificates or other public fields is outside this policy.
2. Preserve installed raw configuration, mixed-PEM admission, all-root trust
   and cache/budget identities. Offload GET snapshot projection and PUT whole-list
   validation/safe-echo preparation, outside configuration/cache guards. Only
   the async owner swaps after preparation succeeds, then echoes that installed
   list. Validation/projection stay offline/cache-neutral. Synchronous embedder
   `validate` and `redacted` remain available.
3. Delivery misses build off-worker without a cache guard. The awaiting task
   rechecks/publishes under the existing map/deque mutex, reusing a concurrent
   winner. Retain 32-entry LRU, accept duplicate builds and drop losers/evicted
   clients outside the guard. Count successful actual delivery constructions,
   including losers/cancelled waiters, and only actual evictions. GET/PUT
   temporary builds affect neither counter.
4. Map delivery join failures to fixed value-free attempt errors, preserving
   retries and exactly-one terminal accounting. Store keeps re-raising mutation
   panics under 0023. API join failures give value-free server errors before PUT
   replacement. Cancelled waiters cannot publish/send later; finite CPU work may
   finish and advance build counts but cannot change terminal delivery totals.
   Keep shutdown order, without a builder drain, replacement-job loop or hard
   cancellation promise.
5. Prove SMTP DATA/queryability during held distinct-root construction with
   spare blocking capacity, paired with >32-root real-wire fan-out. Shared-pool
   exhaustion may still delay SMTP parsing/commits. Cache retention is not a
   global work limit; the 22.1-second network/backoff sum excludes queue/build
   time. No dedicated executor, global admission cap or zero-latency guarantee.

## Authority boundary

This records compatible public-CA visibility and requested placement repairs,
not a narrower 0048 admission domain. 0041 rejected CPU offload for its earlier
batch; its operative rules do not require a construction thread. The earlier
REST-validation spec deferred validation offload to a follow-up. Here it is
included because safe projection constructs clients too. These interpretations
accept no decision: only the operator may accept this proposed record. Under
0045 §8 compatible work may proceed; any concrete accepted-ruling conflict
must wait for its exact amendment.

Private-key refusal is rejected. If selected later, it would amend 0048 §1's
unrelated-section admission and require acceptance before contradictory code.
Proposed 0061 and earlier blanket acceptances confer no permission. Frozen
guarantees cannot be weakened even with a proposal.

## Alternatives and consequences

Both positions support response-only protection and unlocked off-worker builds.
Robustness wins over simplicity on whole-bundle validation against relabeled
DER, API offload and explicit join-failure accounting. Costs: extra offline CPU,
owned snapshots and a small local join boundary. Simplicity's single cache and
no new service/dependency win; duplicate work, target-count-dependent transient
resources, shared-pool contention and finite work surviving cancellation remain.

Diagnostic formatting changes and malformed directly installed bundles can hide
otherwise good roots. Original keys remain in memory. Public roots, verified
later-root TLS, Basic auth/secret delivery, no proxies/redirects, shared payload,
retries, per-target budgets, warnings and ledger order remain. SMTP/storage
ordering is unchanged.

## Proof and lifecycle

Spec AC1–AC10 and paired tasks require real key material, unauthenticated wire
responses, builder-site races, real TLS/SMTP, failure/cancellation accounting,
three mutation receipts and every house gate. The chief commits only this
proposed record and the three design files. Implementation adds a proposed
index row without editing accepted records. This document implies no operator
acceptance, implementation completion or gate result.
