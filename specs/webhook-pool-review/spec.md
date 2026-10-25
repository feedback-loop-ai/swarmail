> Historical #195 artifact restored on 2026-10-25. The current port and its
> proof obligations are governed by [webhook-ca-port-lows](../webhook-ca-port-lows/spec.md).
> Earlier stops and gate receipts below are dated history. 0067/0068 are
> accepted; 0069 was accepted for #220 on 2026-10-25 and is not implemented here.
> The low shared-blocking-pool availability residual is tracked by #220.

# Webhook pool review — issue #195 continuation

Date: 2026-10-24. Inspected baseline: `4f794ba06d3c470e1e92ff01e82d74e5855096bb`.
Tree: `21302f8ccf5f853d61493678bff854b5bf37c87f`.
Run: `continue-epic-192-issue-195-from-6086030b` (engine lane).

Decision: 0069 — [Bounded CA execution proposal and implementation stop](../../docs/decisions/0069-bounded-ca-execution.md), proposed.

## Feature and rationale

Preserve the completed GET webhook failure contract and private-key protection;
correct the resource-remediation proposal and stop before introducing new
concurrency/admission semantics. Continue this existing spec family, without
another implementation of the same regression. No bounded availability remedy
within existing authority has been established by the alternatives examined.
This does not prove every possible compatible optimization impossible. The
unfulfilled requirement is isolation and finite resource ownership for
server-managed CA work across GET, PUT and delivery.

`f2fc20b` already repaired GET OpenAPI 500 and added the real capacity and
cancellation matrix. `d8dcf4b` preserves that work and corrects only the stale
Sol/high selector assertion in `tests/delivery_recipes.rs`. Preserve both.
GET currently documents 200/403/500 and awaited off-worker preparation; its
handler uses the fixed `webhook response preparation failed` native envelope.
There is no reproduced remaining omission to patch.

GET/PUT preparation and cold delivery construction use `spawn_blocking` shared
with SMTP parsing and Store mutations. The 32-entry LRU bounds retained delivery
clients, not executing/queued work, snapshots, duplicate builds or temporary
clients. The 22.1-second delivery network/backoff sum excludes build/queue time.
Finite blocking work can survive cancellation of its awaiting task. Synchronous
embedder validation/projection and unchecked `Webhooks::set` remain relevant
callers, outside a server-only execution bound.

## Evidence and authority

This synthesis reads both `.brokkr/state/design/positions/{simplicity,robustness}.md`,
the historical intake files under `.brokkr/state/tasks/`, and current intake
`.forge/tasks/continue-epic-192-issue-195-from-6086030b.md`.
The recovered packet is `.forge/epic192/issue195/recovered-review-evidence/`.
Its 12 manifest-listed files have matching sizes and SHA-256 hashes;
`issue-195.json` and `prior-run-inspect.json` are present but unlisted. The issue
snapshot says OPEN; no live issue refresh is claimed by this capability-limited
seat. Reconcile newer operator/issue framing before later implementation.

The historical aggregate for `d0fb113c89d95844eec0b449a32d03764fbbb1b5` is
`has_security_residual=true`, `max_residual_severity=low`,
`fixes_applied=false`, `spec_defect=false`, ending at `REVIEW-RESIDUAL-UNSAFE`.
Qwen/Flash/Opus/GLM notes are historical findings only. Correctness's informational
GET/PUT mismatch and the old security note's nonblocking/accepted-cost language
cannot override that security input or accepted 0050/0058.

Current simplicity reports the served GET regression (1 pass) and construction
harness (14 passes) on 4f794ba. Current robustness reports the served regression
(1 pass) on that same baseline and explicitly reports no new saturation run.
These are member execution reports, distinct from this chief's receipts under
`.forge/results/chief-6086030b/`. Earlier d8dcf4b executions remain historical.
The existing evidence establishes shared-pool coupling, recovery and entered
work surviving waiter cancellation. It does not establish natural attack
thresholds, default-runtime measurements, fairness, persistent-commit saturation
or a global resource bound. HTTP disconnect is not the cancellation experiment.

The chief independently rechecked all 12 packet hashes and current source blobs;
source/tests are unchanged from d8dcf4b and that selector fix is an ancestor.
The issue snapshot and journal inspection remain unlisted by the manifest.
No live remote issue refresh is available. A new authenticated issue update
must be reconciled before a later implementation claims current issue coverage.

The recovered GET-500-only removal receipt fails at the missing-response
assertion (exit 101); restored green exits 0. They are authenticated historical
receipts, not mutations freshly executed by this chief. The raw-echo red receipt
intentionally omits key-bearing response output. Preserve that privacy limitation.
Neither donor #201 gates nor historical panel gates verify this candidate.

Accepted [0041](../../docs/decisions/0041-webhook-boundary.md) §§2–7,
[0048](../../docs/decisions/0048-rest-webhook-validation.md) §§1–3,
[0045](../../docs/decisions/0045-delivery-repairs.md) §8,
[0050](../../docs/decisions/0050-delivery-gates.md) §4 and
[0058](../../docs/decisions/0058-codex-review-routing.md) govern.
[0067](../../docs/decisions/0067-webhook-ca-safety.md) and
[0068](../../docs/decisions/0068-webhook-get-failure.md) remain proposed.
0069 corrects their continuation/resource interpretation without editing or
accepting either. 0068's missing-GET context describes its historical baseline;
it is no longer current work. 0069 already exists at this baseline and is already indexed as proposed; refine
it in place. Do not create 0070 or reuse reserved historical numbers merely to
repeat this semantic proposal.
Accepted 0041/0048 remain untouched. The task's stop on new concurrency/admission
semantics applies even to a mechanism arguably compatible with literal rulings.

## Explicit synthesis and costs

| Conflict or agreement | Ruling | Rejected alternative and cost |
| --- | --- | --- |
| Present scope | Adopt simplicity's existing spec family, preserve the completed GET/privacy/selector repairs, and stop before new runtime semantics. Both members agree on this boundary. | Reject another executor implementation or duplicate contract patch in this turn. Remediation remains unresolved and delivery remains held. |
| Proposal identity | Adopt both current positions: refine existing proposed 0069, retaining proposed 0067/0068 and accepted 0041/0048 unchanged. | Reject the earlier synthesis's invented disagreement about editing 0068 and its instruction to add another record. No new number or index edit is needed. |
| Future isolation | Adopt robustness's separate, bounded CA owner across GET, PUT and delivery, constrained to one complete builder, projection policy and cache/publication owner. | Reject a shared-pool semaphore, pool enlargement or timeout as a demonstrated isolation remedy. The proposed owner adds admission, accounting and lifecycle costs requiring authority. |
| Pending queue | Adopt simplicity's no-pending-queue proposal: J = W, reserving an idle worker and owned resources before cloning/submitting. An admitted handoff and an unconsumed result remain charged. | Reject J > W for this proposal. Burst absorption is lost and healthy calls may be refused sooner; no fairness or throughput guarantee follows. Reintroducing a queue requires a new explicit proposal revision. |
| Lifecycle and sizing | Adopt robustness's insistence on complete ownership through cancellation, panic, result disposal and shutdown. W/B defaults, byte formula, oversized input posture, fairness and join policy remain operator-review blockers. | Reject interpreting no queue as a complete design or permission to detach unaccounted work. The proposal cannot yet be accepted as implementation-ready. |
| Optimization first | Adopt simplicity's one builder and cache; preserve complete offline projection/validation, including direct installs. | Reject deleting validation, PUT-only projection caching, new single-flight/cache owners or partial-success overload handling. Repeated CPU work persists pending remediation. |
| Security disposition | Adopt both members' explicit unresolved historical true/low input and five-office 0058 route for later protected review. | Reject low-severity clearance, historical nonblocking language or old-panel substitution. Drafting and passing structural checks do not permit shipping. |

## Acceptance Criteria

1. **AC1 — Honest current contract.** Served `/openapi.json` retains GET webhook
   200/403/500 with GET-specific awaited-preparation text and fixed error wording,
   without PUT/replacement language. Retain the served test and verified
   removal-red/restored-green receipts. Do not change production metadata absent
   a new reproduction. Preserve d8dcf4b's selector correction.
2. **AC2 — Privacy and compatibility.** Preserve real generated-key GET/PUT tests,
   malformed direct-install projection, complete canonical public certificates,
   null fields, withheld secrets/userinfo, raw installed input and mixed-PEM
   admission. Preserve full uncached offline cache-neutral validation, later-root
   verified TLS and >32-root fan-out. GET/PUT join failures remain value-free 500;
   invalid/cancelled PUT cannot replace configuration. No raw/partial fallback,
   private-section refusal, root truncation or empty-on-overload success.
3. **AC3 — Causal resource evidence.** Retain and execute the existing one-slot,
   two-GET/two-slot, two-GET/three-slot and exact waiter-cancellation proofs.
   Receipts identify candidate/blobs, commands/exits, target/root/PEM sizes,
   actual builder entry before parse submission, Pending DATA/count zero where
   saturated, release/finish and 250/immediate count one. Explicit release,
   watchdog=false, joined cleanup and cache/config/LRU neutrality are mandatory.
   State the absence of default-runtime, fairness, global-memory and persistent
   commit saturation proofs. No sleep synchronization, mocks or new seam without
   a test installing it (0020).
4. **AC4 — Corrected proposal and hard stop.** Refine and commit existing proposed 0069 linked here.
   Distinguish current facts from proposed behavior; enumerate worker,
   outstanding-job and owned-byte bounds with J = W and no pending queue;
   cover API/delivery and cancelled work;
   locate admission before snapshots/submission; specify proposed overload
   outcomes and unresolved sizing/lifecycle decisions; map affected accepted
   clauses precisely. STOP before new executor, permit/queue, overload, fairness
   or shutdown implementation. No implicit permission from proposals or tests.
   Completing numeric/lifecycle choices and operator acceptance are later
   prerequisites, not runtime tasks authorized by this draft.
5. **AC5 — Existing guarantees remain.** Keep synchronous mutation ownership,
   full commit and memory visibility before DATA 250, Store panic symmetry,
   subscribe-before-scan, oldest-only pruning and ingest extraction. Preserve
   unlocked off-worker construction, winner recheck, LRU bijection, shared
   payload, verified rustls/no proxies/no redirects, retries and per-sink budgets.
   Keep stop → serve join → drained → dispatcher join and exactly-one terminal
   accounting. No webhook dependency in SMTP acceptance, early 250, write-behind,
   pressure 451 or query-time extraction.
6. **AC6 — Provenance and unresolved security.** Verify all 12 packet hashes and
   mark unlisted files unauthenticated by that manifest. Preserve historical
   true/low security input and receipt privacy. Bind fresh evidence to its actual
   candidate and state limitations. True, absent, null or malformed safety facts
   fail closed under 0050; no authored safe fact or inferred downgrade. No runtime
   mitigation or security resolution is claimed by this design.
7. **AC7 — Design artifacts and validation.** HEAD touches exactly the three
   artifacts in this spec directory and proposed 0069. All four are nonempty,
   committed and clean; the pinned design preflight passes and 0069 resolves
   exactly once as proposed in the unchanged index. Record local gate outcomes
   and failures honestly. Every house gate is required even for documentation;
   a structural pass is not full verification or a protected review verdict.
8. **AC8 — Later completion requires fresh authority and proof.** After the
   semantic stop is lifted, a genuine implementation must satisfy 0069's
   bounds/lifecycle/isolation proof, every house-rule gate and pinned verifier
   proof on its final candidate. Fresh protected review is five separate
   accepted-0058 offices: Astra correctness, official DeepSeek Flash security,
   Sonnet 5.5 tests, Astra guarantees, independent Fable 5.1. Never DeepSeek Pro
   or unauthorized fallback. CI-only proofs stay CI-owned. The engine rules
   advancement; this draft dispatches no substitute panel, pushes nothing,
   opens no PR, merges nothing and closes no issue.

## Dated clarification — 2026-10-24 continuation

The previous 57d9754-based design and f2fc20b implementation evidence remain in
Git history and the dated tasks evidence. Their instructions to add GET 500 and
the capacity matrix are satisfied implementation history, not pending work.
Earlier full verification failed on the selector assertion; d8dcf4b fixes that
assertion without importing #201's verification. This revision selects a
proposal-stage stop, not another investigation-only promise of issue closure.
`drafted` attests committed design artifacts only; runtime remediation, full
verification and security resolution remain outstanding.

## Dated clarification — 2026-10-24 run 6086030b

The previous continuation entry above remains historical. This revision follows
4f794ba's numbering/index correction and uses the two positions from this run.
Their proposals agree on reusing 0069; the old table's disagreement was stale.
No pending queue is now the proposed choice, with its refusal cost explicit.
All other unresolved choices remain blocked. Historical gate failures and prior
chief executions are preserved in tasks.md with their original candidate scope;
current command receipts identify their own tree and exits. Nothing in this
revision accepts a decision or clears the true/low security finding.
