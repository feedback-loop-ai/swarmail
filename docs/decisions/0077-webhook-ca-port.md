# 0077 — Port accepted webhook CA contracts without widening scope

- **Status:** proposed
- **Date:** 2026-10-25
- **Scope:** Epic #192, issue #195, current-main port and three LOW repairs.
- **Specification:** [Webhook CA port and LOW repairs](../../specs/webhook-ca-port-lows/spec.md).
- **Clarifies:** integration and proof of supplied accepted 0067/0068 contracts; does not amend their semantics or accept a ruling.
- **Preserves:** 0001–0003/0005, 0020, 0023, 0041/0048 as clarified by 0067/0068, main's #197/#202/#181 behavior and unchanged delivery routing.
- **Number:** 0077 is next free in the authenticated operator ledger; lower reserved/absent numbers are not reused.

## Context and authority

Current main has none of #195's implementation. Its 17 supplied commits no
longer apply cleanly where API, SMTP, schema, tests and index changed. Public
CA echo and construction semantics change relative to this base, although the
intended contract is already ruled. This proposal makes the port boundary
explicit; it does not reopen mixed-PEM refusal or bounded execution choices.

Checksummed `.forge/epic192/issue195/operator-rulings.md` and patch 10 record
0067/0068 acceptance on 2026-10-24. The 2026-10-25 ruling requires the three LOW
repairs, assigns the low shared-blocking-pool residual to #220 and accepts
0069 for that later issue. Restore final 0067/0068 at their final names during
implementation; preserve the supplied historical 0069 record as directed and
cite the later ruling in current framing. No seat rewrites accepted bodies,
re-accepts decisions or clears the security finding.

## Proposed port clarification

1. Bring main to accepted 0067: whole canonical validated projection with
   empty fail-closed supplied output; raw admission/state preserved; owned
   off-worker API preparation and delivery construction; waiter-only
   swap/publication/send; existing identities, counters and delivery lifecycle.
2. Restore accepted 0068 GET preparation-failure documentation while retaining
   main's query-refusal 400 and admission-before-effects behavior. Preserve
   #202 metrics and #181/0070 object-only body contracts.
3. Repair only identified LOW defects: private-DER and malformed-JSON
   diagnostics are value-free, README states the supported SMTP relationship,
   API injection is identity-keyed and one-shot. No new production admission
   or lifetime policy.
4. Preserve complete patch behavioral/proof history through a disposition
   ledger and dated pointers. Fresh evidence proves the whole disclosure fix
   and owner boundaries, not just the LOW diff. Old receipts prove no new tree.
5. Keep shared-pool availability low and tracked by #220. No executor, capacity
   cap, overload status, single-flight registry, builder drain or hard
   cancellation. Preserve protected review and truthful security results;
   the operator owns the instructed merge disposition.

## Alternatives and consequences

Choose semantic port over old-tree replacement or LOW-only patch, paying merge
inspection/full regression costs to preserve main and introduce the disclosure
repair. Choose robustness's actual-owner/state and failure-path proofs over
helper-only evidence. Choose simplicity's existing owners/dependencies and
limited scope over a new executor or generic fault framework. Repeated builds,
duplicate work, snapshots and finite work surviving cancellation remain costs.
Cache capacity bounds retention only; shared-pool delay remains tracked.

Canonical projection is diagnostic data, not round-trip configuration. It
promises no secrecy for covert content inside loadable certificates or other
intentionally public fields. Raw installed keys stay in memory. Frozen
semantics, Store panic rules, webhook durability and latency guarantees do
not change.

## Proof and lifecycle

Spec AC1–AC10 and paired tasks require real-protocol projection/admission,
API/cancellation, cache/accounting, scheduling and safe diagnostics; main's
regressions, fresh sanitized controls and every house gate. CI/operator-only
proofs retain their owners. Protected review assesses the full port and records
the known #220 residual.

Only this proposal and three owning design files belong to the chief's final
commit. Implementation may index 0077 as proposed and restore already-accepted
supplied records; it accepts nothing. Compatible work proceeds under 0045 §8
and existing 0067/0068 authority. This proposal creates no new acceptance
prerequisite for that authorized work. An actual accepted-ruling contradiction
is outside this design; frozen weakening is refused regardless of tests.
