# 0093 — Source attribution and observation contracts

- **Status:** proposed — awaiting the operator
- **Date:** 2026-10-29
- **Issue:** Epic #164 / #163, continuation from 14217f7c9f30718c450aa25fa04ccbac064dddba
- **Specification:** [Source-attributed comparison](../../specs/source-mail-comparison/spec.md)
- **Supplements:** proposed [0091](0091-fair-mail-comparison.md) and [0092](0092-official-mail-comparison.md), without accepting or rewriting them.
- **Preserves:** accepted 0001–0005, 0014 with its factual corrections, 0050/0072 proof, 0055/0063/0071 eval semantics, 0073 lifecycle, 0075 privacy, 0076 shared admission, 0056 signing and all gate/dependency policies.
- **Number:** next after 0092 in this checkout; not a reservation across concurrent work. Only the operator reconciles any collision or accepts decisions.

## Context

The new captures bind exact annotated tag objects, full commits, setup READMEs and
websocket bodies. They do not establish that either official executable was built
from that commit. Source-lock v2's source_revision is explicitly a built-from claim;
filling it from a tag would silently change its public meaning. Both implementations
have notifications, but the captured subsets establish neither durable matching
wait nor best available facility. Stable totals across pages cannot prove an atomic
snapshot under churn. The adapter also replaces its operation deadline with wait
plus request time and can stop before the requested cutoff.

These are semantic corrections to experiment provenance, admission and MCP wait
contracts, not changes to frozen server guarantees. This proposal records the new
contract explicitly. Compatible work may proceed under 0045 §8; any contradiction
with an accepted ruling waits for that ruling's operator-approved amendment.

## Proposed ruling

1. Introduce strict source-lock v3 with tagged_source and separate required typed
   binary_build_provenance. Preserve historical v1/v2 bytes and readers; do not
   reinterpret their fields or migrate past runs. Bind source/acquisition metadata,
   ref/tag/tree captures, immutable source URLs, full commit, license/API/setup/
   reviewed-source bytes and exact executable/asset identities in the existing lock.
   MailHog's digest remains locally observed; Mailpit's upstream digest covers its
   archive. Tags/capture hashes do not authenticate a build or the host.
2. Permit explicitly declared official-artifact measurements with verified executable
   bytes and attributed tagged-source context while binary build provenance remains
   explicitly unresolved. Such evidence makes no built-from/equivalence claim and
   cannot satisfy a profile that requires it. Required missing operation semantics
   still block dependent collection; setup probes remain unscored. Reject falsely
   asserted provenance and recheck executable bytes at finalization.
3. Freeze protocol v2 actual per-target storage/caps/compression, resource enforcement,
   budgets and identities before collection. Separate unmatched/unenforced profiles
   from requested matched/enforced ones; retain every cell and failed repetition.
   Retain immutable historical protocol/lock meaning and explicit change reasons.
4. Select common-client/adapter immediate scans with a frozen 100 ms post-scan poll
   interval. Keep native Swarmail subscribe-before-check separate. Best-wait stays
   blocked pending route/producer/registration/gap and actual binary evidence; source
   review alone closes only the not-inspected gap. No new stream client is selected.
   Do not assert no streams or equate dropped notifications with stored-mail loss.
5. Compose one absolute operation ceiling with an observation cutoff and bounded
   response allowance. Neither extends the ceiling. Polling timeout means no match
   observed under the declared method, not continuous absence. Do not return timeout
   early because the next interval would cross cutoff. Preserve late/censored samples;
   incomplete scans, budget/transport failure and cancellation are errors, not empty
   success. Bind actual MCP schema/bounds and test final client contracts separately.
6. Reject incomplete/changing/duplicate/nonprogressing/short-nonfinal scans. Stable
   counts and unique IDs do not prove snapshots; final reconciliation drains owned
   senders and compares quiescent storage with independent expected identities. Live
   scans remain observations. Preserve ordinary upstream bodies and read side effects;
   no extraction, isolation/chaos emulation, cache or task/admin capability.
7. Corroborate owned binds, retain one async worker through cancellation, reap/drain,
   identity checks, observed cessation and disposal. Unknown cleanup stops admission
   and finalization. Kill requests, Drop and timed-out joins are not receipts. Forced
   death leaves incomplete evidence pending confirmed cleanup. This adds no hostile
   confinement, stale-lock recovery, supervisor platform or guarantee of kernel time.
8. Preserve full inherited AC1–AC16/T1–T14, native v2/scorer/replay, shared cross-checkout
   admission, final contracts, genuine baseline/finals/costs and full loss accounting.
   No paid execution is authorized. HIGH review-integrity residual/original references,
   separate #219, actual CI, candidate-bound pinned engine, five independent protected
   reviews and hard security hold remain obligations. Never infer repair from upstream
   merge, invent findings/verdicts or accept/renumber existing proposals.

## Reconciled positions and costs

Adopt simplicity's existing owners and fixed targets; reject extra platforms,
services, dependencies, event engines and broad fault campaigns. The cost is less
extensibility and potentially blocked best-wait/parity claims. Adopt robustness's
explicit provenance versions, quiescent identity checks, budgets and retained cleanup
over smaller but unsupported success claims. The cost is more strict validation,
negative tests, conservative refusals and visible incomplete runs. Both positions
retain all same-suite obligations and reject fabricated paid evidence.

## Proof and authority

The linked spec, plan and tasks require source-chain/claim negative tests, actual
three-target/SMTP→Mailpit→MCP proof, independent arithmetic, benign process ownership,
full standalone eval/control/coordination proofs and the whole house inventory.
Historical passes are not current receipts. Unknown facts remain unknown.

Chief commits exactly spec.md, plan.md, tasks.md and this proposed record. T1 adds
its single proposed index row; inventory gates remain outstanding until then.
Structural design validation is separate from candidate verification. Keep 0090–0092
proposed and historical specs unchanged. Commit unsigned; never push, merge,
publish, tag, close issues or accept decisions. The engine alone rules advancement.
