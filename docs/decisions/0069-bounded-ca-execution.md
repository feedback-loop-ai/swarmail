# 0069 — Propose bounded CA execution and stop before admission changes

- **Status:** accepted (operator, 2026-10-25); implemented by #220. Split from #195 by the operator on 2026-10-24, so #195 shipped without it.
- **Date:** 2026-10-24
- **Scope:** Epic #192 issue #195 continuation; availability and resource ownership.
- **Specification:** [webhook pool review](../../specs/webhook-pool-review/spec.md).
- **Proposes amendment to:** accepted [0041](0041-webhook-boundary.md) §2 and its CPU-pooling/global-budget rejection, and [0048](0048-rest-webhook-validation.md) §§1–3, only for server-managed CA execution admission and resource-refusal behavior. Lifecycle selection must also resolve the §5 relationship below before implementation.
- **Corrects continuation interpretation of:** proposed [0067](0067-webhook-ca-safety.md) §§2–5 and proposed [0068](0068-webhook-get-failure.md) context/resource evaluation/stopping rule. Neither is accepted, amended in place or treated as a security waiver.
- **Preserves:** 0001–0005, 0023/0030 offload and Store panic symmetry, complete offline validation, private-key-safe diagnostics, direct verified delivery and existing accounting.
- **Number:** next free after 0068; historical gaps are not reused.

## Context and dated correction

At d8dcf4bae43ea2488751b3528753f8ff18ecafe4, f2fc20b's GET OpenAPI 500 repair
and real GET saturation/cancellation tests already exist. 0068's omission and
missing-matrix context is historical, not a request to repeat that patch. The
selector correction changes only tests/delivery_recipes.rs and must survive.

Earlier d8dcf4b tests and this run's simplicity-position execution on 4f794ba
demonstrate shared-pool coupling: held GET CA work
uses the only blocking slot, or both of two slots, and prevents SMTP parse
progress until release. A third slot permits the same finite exchange before
release. An aborted/joined projection waiter leaves its entered closure running.
These tests preserve no-early-250 and immediate queryability on recovery; they
do not establish arbitrary-load availability, default-pool latency, fairness or
a persistent-commit saturation result. Store also uses this pool in source.

The 32-entry cache bounds retained clients, not outstanding jobs, duplicate
builders, configuration snapshots or temporary clients. Delivery budgets do not
cover GET/PUT and do not globally bound distinct sinks. The 22.1-second
network/backoff sum excludes build and queue time. Synchronous embedders remain
outside server-only bounds. Finite execution surviving cancellation is an
observed limitation, not accepted security risk.

The old aggregate remains has_security_residual=true, severity low. Its old
Qwen/Flash/Opus/GLM panel and its “accepted cost”/nonblocking language supply no
current authority. Only accepted 0050/0058 and the authorized security office
control that finding. This proposal supplies no substitute safety fact.

## Proposed direction — not implementation authority

Select a separately owned, bounded server CA execution facility covering GET
projection, PUT complete validation/projection and delivery cache misses. This
is the robustness direction, constrained by simplicity's single builder and
single cache/publication owner. No new service framework, second cache, auth
rollout, public tuning surface or dependency is selected. Reuse the existing
complete offline builder and public projection; never skip their work on an
admitted input just to relieve pressure.

The following requirements define the amendment to be completed for operator
review. They are not an implementation-ready worker design. Numeric defaults,
accounting formula and lifecycle choices listed below are blocking, and their
absence must stop dependent code rather than invite implementer discretion.

1. **One capacity owner.** Set finite worker bound W, outstanding-job bound J
   (reserved handoff, running, abandoned work and retained results) and charged
   job-owned byte bound B, with J = W >= 1 and B > 0. There is no pending
   CA queue: admission reserves an idle worker and all required capacity, or
   refuses immediately. A completed result retains its job reservation until
   handoff/drop; an idle thread alone does not make that capacity available.
   Bound all server-managed paths,
   not only API calls. Acquire capacity before cloning another configuration
   snapshot or submitting work. Never acquire it inside shared spawn_blocking,
   occupy SMTP/Store blocking slots while waiting, or enqueue uncharged work.
   Admission is nonblocking; if any reservation cannot be made, refuse it.
   A bounded handoff to the reserved worker remains charged. No waiting queue
   or separate waiter list is permitted.
2. **Bytes and copies are explicit.** The final formula must charge every
   job-owned string/snapshot/copy, including delivery keys and both validation
   and projection inputs; define ownership of outputs until handoff/drop and
   temporary DER/roots/clients. W and bounded input constrain simultaneous
   parsing, but B is not an allocator-wide memory ceiling. HTTP body extraction,
   installed configuration, transport and synchronous embedder allocations are
   outside this budget and must be disclosed. Admission sizing may inspect
   borrowed installed configuration before cloning; it cannot allocate an
   uncharged duplicate to learn its size. Retain the existing HTTP body limit.
3. **Proposed HTTP refusal.** Capacity exhaustion/closed intake returns GET/PUT
   503 in the native envelope with fixed `webhook CA preparation unavailable`.
   No PEM, URL, secret, worker error or queue detail is included. Preserve the
   existing route guard and extractor ordering and their 403/400/413/415 cases;
   resource refusal is not malformed-input 400 or preparation-failure 500.
   Keep GET/PUT's existing fixed 500 meanings for execution/join failure after
   admission. Refused or cancelled PUT leaves installed configuration intact;
   success validates the entire list, projects safely and swaps exactly once.
   Do not return empty/partial diagnostics as overload success. Document future
   503 separately on GET and PUT only when that behavior is authorized.
4. **Proposed delivery refusal.** Failed CA admission is a fixed value-free
   attempt error `webhook CA preparation unavailable` owned by the existing
   retry/terminal logic. It consumes an attempt, retains existing retries and
   backoffs and ends in exactly one success/exhausted/aborted outcome. Per-sink
   budgets and shared payloads remain. This can reduce successful opportunities
   for otherwise healthy targets under global pressure; it is the explicit
   global-admission tradeoff, not unchanged delivery semantics. Never refuse or
   defer SMTP acceptance for a webhook result.
5. **Cancellation and publication.** One reservation owner keeps job/byte
   charges until actual work and owned retained results finish or are dropped.
   Cancellation before worker entry must drop charged input exactly once; a
   race with entry must transfer, never duplicate or release, ownership.
   Cancelling a running waiter cannot release capacity early or hard-cancel its thread.
   Only a live awaiting owner may install PUT data, publish a cache winner or
   send delivery. Recheck the winner under the existing cache mutex; keep the
   32-entry LRU, drop losers outside the guard and count actual builds honestly.
   API preparation remains offline and delivery-cache-neutral.
6. **Lifecycle must be resolved before code.** State the facility's owner for
   server and embedding use, when intake closes, who disposes admitted handoffs,
   how finite running work remains charged/reclaimed, who joins worker threads,
   and how panic leaves capacity usable without logging supplied values. Do not
   silently add a shutdown wait/drain or claim aborted delivery tasks drain
   workers. Preserve dispatcher stop → serve join → drained → dispatcher join,
   sweep through Lagged to Empty/Closed and exactly-one terminal accounting.
   A join delaying stop, a detached supervisor or process-lifetime ownership
   each has observable costs; the final proposal must select and justify one.
   Endless CPU work cannot be safely killed by ordinary thread cancellation.
7. **Limited isolation claim.** The intended proof is that CA jobs cannot
   consume SMTP/Store Tokio blocking slots, with finite admitted CA resources.
   Dedicated workers do not guarantee OS CPU/memory scheduling, fair admission,
   default-runtime latency, durable webhooks or bounded total server memory.
   Do not extend the claim to synchronous embedder validate/redacted calls.

## Exact authority map and unresolved operator choices

| Authority | Proposed relationship |
| --- | --- |
| 0041 §2, alternatives rejecting pooling/global delivery budget; 0029 §5 delivery budgets | Separate CA execution and global admission add scheduling states and cross-target refusal. Seek explicit narrow amendment; preserve rustls verification, direct/no-proxy/no-redirect builder policy and existing per-sink budgets. |
| 0041 §5 shutdown | Preserve its dispatcher order. A new executor's intake closure/reclamation is additional lifecycle policy; if the selected join changes stop completion or adds a drain, explicitly amend §5 before code. No implicit lifecycle amendment is accepted here. |
| 0048 §1 complete uncached offline validation | Keep validation of every admitted bundle and unrelated-section/mixed-PEM admission; add only explicit resource admission/refusal. Never use cache hits or truncation instead of full validation. |
| 0048 §§2–3 request/refusal and diagnostic contracts; 0041 §3 echoes | New 503 is an additive public refusal outcome; successful response keys/nulls, userinfo/secret withholding and full safe public certificates stay. |
| 0023 §§1–5 and 0030 parse offload | No amendment: retain whole synchronous mutation and awaited parse/commit, panic symmetry and commit-before-250. Moving SMTP/Store or adding pressure 451 is excluded. |
| 0001–0005 | Preserve without qualification. A proposal, operator acceptance or green test cannot authorize weakening a frozen guarantee. |
| 0045 §8; 0050 §4; 0058 | Proposals confer no acceptance or clearance. New semantic work also stops under this task's explicit boundary even if literal compatibility is arguable. |

Before operator acceptance and implementation, complete all of these choices:

- Numeric W/B defaults and their measurement/rationale, with J = W. The synthetic
  matrix provides no production sizing evidence. No pending queue or public knob
  is selected. More burst refusals are an explicit cost; revising J > W requires
  another explicit proposal revision, not implementer discretion.
- A testable byte-accounting formula including copies/output ownership and
  overflow-safe arithmetic; quantify what remains outside it.
- Over-budget single jobs, especially arbitrary direct-set embedder snapshots:
  select a documented 503/resource posture or a bounded representation that can
  admit them. A fixed B can permanently refuse previously valid large snapshots;
  do not hide that compatibility cost or quietly narrow 0048's valid PEM domain.
- API versus delivery fairness under the shared CA admission owner. Nonblocking
  reservations alone do not prevent starvation; choose its policy and refusal
  cost explicitly, without pretending per-sink budgets decide it.
- Exact worker panic recovery, server/embedding ownership and shutdown/join
  mechanism, and any resulting precise 0041 §5 amendment.

These are explicit unresolved decisions, not acceptance criteria claimed met.
No default values, queue machinery or lifecycle code may be inferred from this
record. The present authorized output is this corrected proposal and stop.

## Alternatives and consequences

Reject a shared-pool semaphore as isolation: even one correctly charged job can
occupy the only blocking slot, and API-only permits omit delivery. Waiter-owned
permits are false bounds after cancellation. Larger Tokio pools shift the
threshold; async timeouts leave running work alive. An unbounded dedicated queue
moves rather than bounds the problem. An external process scheduler or generic
service framework adds unrequested scope.

Reject deleting whole-bundle projection validation, returning raw/partial PEM,
refusing private-key sections, truncating later roots, using the delivery LRU
for PUT validation or caching only PUT echoes. These break privacy/admission or
miss unchecked set/churn/cold delivery. Optimizations may reduce work, but have
not demonstrated a bounded remedy; they are not security clearance.

Simplicity wins on reusing current code/proofs and stopping before new semantics.
Robustness wins on the future isolation/admission direction and retaining its
full lifecycle/accounting burden. The cost is an unresolved security finding,
held delivery and operator decisions; the reward is no hidden compatibility
change masquerading as a local guard. Keep accepted 0041/0048 and proposed
0067/0068 untouched. This existing proposed record corrects continuation
interpretation, not historical receipts or operator status.

## Required proof and disposition

Preserve the current real served-contract red/restored-green receipts, generated
private-key-safe wire tests and actual GET capacity/cancellation baseline.
Later authorized implementation must prove exact W/J/B limits before allocation,
HTTP 503 and unchanged configuration, delivery retry/terminal behavior, both
cancellation sides, duplicate/config-churn accounting, worker panic recovery,
shutdown/embedding cleanup and no late publication/send. Use real HTTP/SMTP
with all CA workers held while SMTP parses, persistently commits, answers 250
and is immediately queryable. Require explicit release, no watchdog-induced
success, no sleep synchronization and correct 0020 harness placement. Retain
complete-root, later-root TLS, >32-root fan-out and all frozen guarantee proofs.

All house gates and pinned execution proofs remain necessary on the final
candidate, followed by five separate accepted-0058 offices: Astra correctness,
official DeepSeek Flash security, Sonnet 5.5 tests, Astra guarantees and
independent Fable 5.1. Never Pro, old four-role substitution or fallback.
True/absent/malformed security facts stop; only authorized security resolves
its finding. No runtime implementation, full-verification or security-success
claim follows from this draft. Only the operator accepts a completed proposal.

The chief commits this record and exactly three spec artifacts. The prior
numbering/index correction already records 0069 as proposed; its current proof
is tracked in tasks.md and ignored command receipts. No push, PR, merge or issue
closure.

## Dated correction — 2026-10-24 run 6086030b

Baseline 4f794ba already contains this proposed record and its index row.
Current simplicity and robustness both request its reuse; no duplicate decision
is justified. The chief selects simplicity's J = W, no-pending-queue alternative
over a bounded pending queue, while retaining robustness's complete accounting,
refusal and lifecycle requirements. This sacrifices burst absorption and may
increase cross-target refusal. It removes queue ordering and dequeue ownership,
not worker-entry, result-lifetime or shutdown races. W/B, overflow-safe accounting,
oversized snapshots, fairness, panic recovery and exact join ownership remain
unresolved prerequisites for operator review. No semantic implementation is
permitted by this selection, and status remains proposed.
