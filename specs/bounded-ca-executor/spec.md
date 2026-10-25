# Bounded webhook CA executor

Date: 2026-10-25. Status: design draft. Epic #192, issue #220.

Decision: 0078 — [CA executor ownership and admission](../../docs/decisions/0078-ca-executor-ownership.md), proposed.

## Feature and rationale

Server-managed webhook GET projection, PUT complete validation/projection and
custom-CA delivery cache misses use one bounded process-wide executor, separate
from Tokio's blocking pool. SMTP parsing and Store commits retain their awaited
offload. Holding every CA worker must leave real SMTP DATA acceptance, persistent
commit and immediate HTTP queryability progressing.

The #195 implementation demonstrated shared-pool coupling, including running
projection surviving cancellation. A 32-entry retained-client cache does not
bound that work. This design implements accepted [0069](../../docs/decisions/0069-bounded-ca-execution.md)
and preserves [0067](../../docs/decisions/0067-webhook-ca-safety.md) privacy,
publication and no-builder-drain behavior and [0068](../../docs/decisions/0068-webhook-get-failure.md)
GET failure semantics. The checksummed 2026-10-25 operator ruling and committed
accepted header supersede historical proposed/stopping prose. 0078 records the
resolved choices; it accepts nothing and adds no acceptance prerequisite to
compatible implementation under 0069.

Both files in `.forge/epic192/issue220/SHA256SUMS` verified. Intake is
`.forge/tasks/epic-192-issue-220-give-server-m-b73df672.md`; the legacy
`.brokkr/state/tasks/` directory is absent. Both run-local positions under
`.brokkr/state/design/positions/` were read. No external evidence is required.

## Panel rulings

| Question | Ruling and losing-side cost |
| --- | --- |
| Ownership and queue | Adopt both: two dedicated native threads, W = J = 2, process lifetime, no pending queue, second runtime or dependency. Reject per-server pools, supervisors and join-on-stop: they multiply retained workers or change stop completion. Pay resident stacks and cross-server contention. |
| Byte ceiling | Select simplicity's B = 67,108,864 charged bytes (64 MiB), rejecting robustness's 128 MiB. Neither has throughput evidence. The smaller policy admits fewer large concurrent jobs; valid inputs below the HTTP body limit can refuse. The exact tariff below resolves both positions' unfinished sizing requests. |
| Accounting | Adopt robustness's logical tariff and simplicity's redundant-copy removal. Replace an unexplained single 32*S multiplier with named categories. Opaque client allocations have declared weights, not measured sizes; no allocator/RSS ceiling follows. |
| Admission | Adopt robustness's try-lock refusal: contention can return unavailable. First successful reservation wins, without class quota or fairness. Reject a fairness queue/reserved delivery worker; healthy deliveries can lose attempts to GET pressure. |
| Lifetime authority | Adopt robustness's revocable server scope over simplicity's watch-only admission check. Keep shutdown-watch notification, but serialize close against synchronous publication. Pay a private scope/owner token and short publication critical section to prevent retained callers publishing after closure. |
| Panic/startup | Adopt unwind containment, fixed errors and no replacement loop. Adopt robustness's all-or-nothing cleanup of partially created idle workers before admission; this adds no builder drain to stop. |
| Proof | Adopt both persistent actual-GET saturation proofs and robustness's runtime-reuse/close races. Reject in-memory/spare-slot evidence as the isolation proof; pay deterministic harness work. |

## Requirements and resource policy

All server GET/PUT preparation uses the facility, even without custom roots.
Delivery uses it only on custom-CA cache misses; hits and built-in client retain
their existing path. Public synchronous `WebhookTarget::validate`/`redacted`,
direct `Webhooks::set`, installed configuration and pre-existing delivery
snapshots remain outside server-job admission claims.

Admission atomically reserves an idle slot, one job and its entire charge or
refuses immediately. No permit wait, pending list, uncharged submission, per-job
thread spawn or shared-pool fallback. Reserved handoffs, running/abandoned jobs
and completed retained results count toward J. An idle worker with a retained
result is unavailable. All servers/runtimes in the process share one owner.

### Exact charge schedule

B is a **logical charged-job budget**, not an allocation measurement. Every
intermediate sum/product and aggregate addition uses checked `usize` arithmetic.
Overflow or over-budget refuses before extra snapshots/keys. Constants are
internal policy with no public knob. Charge never shrinks during a job.

Sizing inspects without parsing/copying. N is target count (1 for delivery).
U, I, H and P sum capacities of URL, inbox, secret and PEM strings entering API
jobs. GET inspects installed string capacities, deliberately overcharging clones
that need only lengths. For delivery set U = I = H = 0 and P to borrowed PEM
length before allocating its key. S = U + I + H + P; absent fields contribute
zero. Vi is input vector backing: GET N * size_of::<WebhookTarget>(); PUT its
extracted Vec capacity * that size; delivery zero. Vo is N * that size for API
output, zero for delivery. Installed GET vector spare capacity is not cloned.

R = sum floor(pem.len()/27) over supplied PEMs (one for delivery). The 27-byte
`-----BEGIN CERTIFICATE-----` marker alone conservatively bounds root count;
no extra parser or refusal of unrelated sections is introduced. Define:

```text
E = 4 * (floor(P/3) + R)
C = 55*R + E + floor(E/64)
O = 8*U + 256*N + I + C
A = 1 if any ca_pem is supplied (including empty), otherwise 0
Q = 65536 + 512*N + Vi + Vo + 4*S + 8*O
    + 8*(P+C) + 512*R + 65536*A
```

The same conservative schedule applies to delivery, including unused projection
allowance; no second accounting regime. Invalid/non-certificate input can be
overcharged; this never changes PEM admission semantics.

| Charge | Ownership and derivation |
| --- | --- |
| 65536 + 512*N + Vi + Vo | Envelope/reply/error/control metadata, per-target bookkeeping and both container backings. Even an empty list reserves nonzero capacity. Fixed values are policy weights. |
| 4*S | Raw snapshot/moved PUT list or delivery key through execution/result retention, header-validation copy, and temporary key duplication at cache publication. Return delivery's key with its client instead of retaining a second key across await. |
| 8*O | Output/formatting capacity weight: URL processing at eight input bytes plus 256 per target (including fixed invalid-URL placeholder/empty fields), inbox copy and canonical PEM output. Outer eight charges backing growth and simultaneous formatting copies. Weights are not an allocator/Unicode-normalization upper-bound claim. Secret output is absent. |
| 8*(P+C) | Original/reconstructed PEM parsing, decoded certificate backing, base64 scratch and root/client payload representations. Validation and projection run sequentially in one job; temporary validation clients drop before projection clients. No per-target parallel builds. |
| 512*R + 65536*A | Root/container metadata and one simultaneous custom-client construction's opaque fixed overhead, as logical weights. Root payload bytes belong to the preceding term; no reqwest/rustls private-heap size claim. |

The canonical output length envelope C follows from total DER length <= P,
sum ceil(DER_i/3) <= floor(P/3)+R, 54 header/footer bytes per root and at most
floor(E/64)+R line-ending bytes. It includes duplicates and tiny certificates.
URL weights deliberately have a different meaning: charged processing/storage
categories, without a private-allocator maximum claim. No extra materialized
copy may be added without assigning it to this table and rechecking the tariff.
Tests pin equation, canonical envelope and ownership, not RSS <= B.

Keep Q through completion/receipt until synchronous transfer or disposal ends.
API output transfers to the response owner, PUT raw data to installed state,
and delivery winner/keys to the sole cache. Losers/abandoned objects drop before
refund. Excluded: HTTP extraction before admission, installed configuration,
pre-existing dispatcher snapshots/shared mail payloads, transferred cache or
response allocations, serialization/transport, stacks, allocator overhead and
synchronous embedders. Exclusions never authorize uncharged CA-job-owned copies.

A valid oversized direct-set snapshot can receive GET 503 indefinitely until
replaced/cleared. Oversized PUT/delivery also refuse as resources, not malformed
PEM. No truncation, partial echo, first-root shortcut, stricter mixed-PEM policy
or new body limit is authorized.

### Lifetime and cancellation

The facility initializes once before server admission. Successful workers remain
for process lifetime; handles are intentionally detached. No server, dispatcher,
retained Webhooks handle or reaper joins them. Partial initialization failure
closes/joins only idle startup workers and permanently marks the facility
unavailable, without fallback/retry. Server startup can continue; CA requests
refuse. Public synchronous helpers remain usable.

A private owner token closes its server scope at stop entry, owner drop and
startup failure. API/dispatcher tasks hold revocable references, not owner tokens.
Admission and synchronous PUT/cache publication linearize against closure.
Pre-close publication stands; post-close results discard. No await separates
live check and publication. A live owner must authorize delivery's next send;
already begun sends retain existing abort/join behavior, not network retraction.

The envelope alone owns the reservation. Before-entry cancellation/closed scope
drops input once. Entry races select one owner; running cancellation leaves work
charged. Replies retain charge until consumed/dropped; failed send drops the
charged reply. Finite abandoned work reclaims without its original Tokio runtime.
Workers cannot install, publish, send or change terminal delivery totals; they
can count successful actual delivery builds.

Catch ordinary job unwinds outside locks; preserve fixed worker errors and reuse
the worker. Never format supplied values/panic payloads, replace the global panic
hook or change Store's `resume_unwind`. Keyed, value-free panic seams stay keyed.
Allocation abort/endless work are not recoverable panics; two stuck jobs can
disable CA work until process restart.

This resolves 0069's §5 relationship: [0041 §5](../../docs/decisions/0041-webhook-boundary.md)
stop → serve join → drained → dispatcher join and 0067's no-builder-drain stay.
Close is a short state transition, never a builder wait. Keep sweep-through-Lagged
to Empty/Closed and exactly-one terminal accounting. One stopped server leaves
other scopes live, but its abandoned charges can contend with them. Existing
tracked-HTTP shutdown behavior remains; no new stop-completion promise or worker
join is added.

### Public and delivery contracts

GET/PUT unavailable admission (contention, oversized/overflow, closure or failed
facility) returns native-envelope 503 exactly `webhook CA preparation unavailable`.
Guard/query/extractor precedence including 403/400/413/415 remains. Admitted
worker failures retain GET 500 `webhook response preparation failed` and PUT
500 `webhook configuration preparation failed`; admitted invalid PUT stays 400.
Failed/refused/cancelled PUT never swaps; success validates all targets, prepares
the complete safe echo and swaps once. GET/PUT remain offline/cache-neutral.

Refused delivery misses consume an ordinary attempt with the same unavailable
text. Preserve four attempts/backoffs, per-sink cap/shared payload and one
terminal outcome. Recheck concurrent winners under the sole cache mutex; keep
32-entry LRU, actual build/eviction counts and outside-lock loser/eviction drops.
SMTP never awaits webhooks or gains pressure refusal. Served OpenAPI documents
503 separately on GET/PUT and preserves existing statuses and schemas.

## Acceptance Criteria

These are implementation proof obligations, not design-phase success claims.

1. **AC1 — Isolation:** All three server paths use dedicated workers. With W
   actual HTTP GET builders held and Tokio max_blocking_threads = 1, real SMTP
   session/DATA reaches persistent commit and final 250, then real HTTP retrieves
   that exact mail before explicit CA release. Assert both holds active/watchdog
   unused at success; reopen the database afterward. A control restoring shared
   pool placement fails this proof; no production control switch is introduced.
2. **AC2 — Capacity:** At reserved, entered, abandoned, retained and disposed
   transitions, jobs <= 2, running <= 2 and sum Q <= 67,108,864. Prove individual
   and aggregate below/equal/above limits, overflow, zero/tiny fields, many roots,
   String/Vec spare capacity, mixed GET/PUT/delivery and configuration churn.
   Reservation precedes extra snapshot/key allocation; no queue/waiter list.
3. **AC3 — HTTP:** Exhausted/closed real GET/PUT give exact native 503; guards
   and extractors retain precedence. Keep fixed 500/invalid-400 proofs, one swap
   and full safe echo, and byte-identical raw installed state on every failure
   or cancellation. Served-schema tests independently catch either missing 503
   and the existing GET 500.
4. **AC4 — Cancellation:** Pre-entry, entry-race, running, completed-unreceived
   and receive/drop cases prove one destructor/refund, no early reuse and finite
   reclamation. Cancel an actual held GET handler and repeat AC1's persistent
   SMTP/query proof while its charge stays held. HTTP-client cancellation alone
   does not prove server-handler cancellation.
5. **AC5 — Publication/delivery:** Only live owners install/publish/authorize
   sends. Pressure recovery and four-attempt exhaustion preserve final ledgers;
   duplicate winner, loser/cancelled actual-build count, LRU/eviction, outside-lock
   drops and full-cache API neutrality remain proven with real protocols.
6. **AC6 — Lifecycle/panic:** Close refuses new jobs/late publication; handoffs
   and retained replies dispose correctly. Stop keeps dispatcher order and can
   finish with an abandoned held delivery builder, without builder drain or late
   terminal changes. Two servers/new runtimes reuse one process facility. Prove
   native-thread construction, partial initialization failure, closed handoff,
   panic recovery and finite reclamation after originating-runtime teardown.
7. **AC7 — Guarantees:** Preserve 0001–0003/0005, awaited whole mutation/parse
   and Store panic re-raise (0023/0030), complete/mixed-PEM admission, later-root
   verified TLS, >32-root real-wire fan-out, private-DER/secret/userinfo withholding,
   canonical all-or-empty output and direct/no-proxy/no-redirect policy. Keep
   synchronous embedder APIs and #195's keyed panic/safe-diagnostic fixes.
8. **AC8 — Delivery quality:** New production branches remain measured under
   exact coverage. Use 0020 placement, keyed exercised seams and synchronization.
   Run all house gates, coverage selftest, publish/seal proofs, pinned compile,
   fail-closed native-tls absence and unchanged protected review. Bind receipts
   to the executed candidate; structural design validation is the engine's fact.

CI owns macOS, Helm/chart history, Docker, network RustSec and privileged port-80
proofs; operator owns live settings/sealing. No invented local criterion depends
on those. No OS scheduling isolation, latency target, fairness, durable webhooks
or whole-server memory bound is promised. No push, merge, tag, publication,
issue closure or decision acceptance occurs in this run.
