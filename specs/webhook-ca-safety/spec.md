> Historical #195 artifact restored on 2026-10-25. The current port and its
> proof obligations are governed by [webhook-ca-port-lows](../webhook-ca-port-lows/spec.md).
> Earlier stops and gate receipts below are dated history. 0067/0068 are
> accepted; 0069 was accepted for #220 on 2026-10-25 and is not implemented here.
> The low shared-blocking-pool availability residual is tracked by #220.

# Webhook CA safety — completion of issue #195

Epic #192 · design synthesis · 2026-10-24
Run: `epic-192-issue-195-refuse-privat-8e00b7a6`.

## Outcome and rationale

Complete the existing certificate-only webhook responses and off-worker CA
construction against accepted rulings, closing the missing protocol proofs.
An operator may supply a combined certificate/private-key file. Unauthenticated
GET and successful PUT must expose only canonical loadable certificates. Cold
CA construction must leave async workers and the cache mutex free, while SMTP
250 continues to mean immediately queryable stored mail.

The candidate already implements these mechanisms. This completion primarily
strengthens tests and corrects stale descriptions; repair production code only
where a discriminating proof establishes a violation of the accepted contract.
The design does not claim implementation completion or security clearance.

## Authority and dated clarification — 2026-10-24

The operator accepted [0067](../../docs/decisions/0067-webhook-ca-safety.md)
and [0068](../../docs/decisions/0068-webhook-get-failure.md) on 2026-10-24.
They are authorities, not new decision declarations. The supplied issue packet
and operator rulings both passed their directory's SHA256SUMS verification.
Read both run-local positions in `.brokkr/state/design/positions/` and the
matching `.forge/tasks/epic-192-issue-195-refuse-privat-8e00b7a6.md` intake:
`.brokkr/state/tasks/` is absent. Its committed counterpart is
[intake-195-completion.md](intake-195-completion.md).

This revision replaces obsolete proposed-0062 references and completed-task
claims with current completion obligations. Accepted status headers and the
authenticated operator ruling govern historical proposal/stop prose in earlier
records and recovery specs. No new semantic delta is selected, so no additional
ADR, index edit or acceptance is requested. Accepted records remain untouched.
[0069](../../docs/decisions/0069-bounded-ca-execution.md) stays proposed, assigned
to #220. The low-severity shared-blocking-pool availability residual is tracked
by #220 and does not stop #195 merely by existing. That allocation does not
clear the private-key disclosure fix: current protected security review must
assess the fix itself.

Preserve [0041](../../docs/decisions/0041-webhook-boundary.md),
[0048](../../docs/decisions/0048-rest-webhook-validation.md), frozen 0001–0003,
ingest extraction under 0005, and Store mutation-panic symmetry under 0023.
No production SMTP/Store refactor, dependency, license, route, gate or pin change
is in scope. Keep verified rustls, all supplied and built-in roots, Basic auth,
secret headers, no proxy/redirect, shared payload Bytes, four attempts with
100/400/1600 ms backoff and five-second HTTP timeout, per-sink budgets, warning
suppression and the existing stop/serve-join/drain/dispatcher-join ledger order.

## Panel synthesis and rulings

The current positions agree on production ownership. Do not inherit the old
panel's decode-only or delivery-only alternatives as current recommendations.

| Question | Ruling, rejected alternative and cost |
|---|---|
| Architecture | Adopt simplicity's existing builder, projection and publication owners and robustness's whole-bundle validation/accounting boundaries. Reject new parser, service, cache or single-flight registry. Duplicate CPU work and owned snapshots remain accepted costs. |
| Proof depth | Adopt robustness's actual PUT-handler and exact protocol/metrics proof requirements over treating existing helper tests as sufficient. Reconcile with simplicity by extending existing suites/harness. Extra deterministic fixture and seam maintenance is the cost; helper-only proof would miss handler routing and publication defects. |
| Cancellation | Require observed cancellation of the publication-owning handler/task. A cancelled helper or disconnected client alone loses as proof. An exercised owner-local cfg(test) seam may be necessary; this costs test code, not a new production lifecycle. |
| Diagnostics | Adopt simplicity's deletion of value-bearing assertions before negative controls. Fixed case identifiers and boolean comparisons lose rich response dumps but keep a failing privacy proof from disclosing the fixture. No logging framework is justified. |
| Scope and authority | Reuse `webhook-ca-safety`, as simplicity requests, and adopt robustness's clause-to-test matrix. Replace stale living authority references; retain dated historical evidence. Reject another recovery spec/ADR because accepted 0067/0068 already decide the behavior. |
| Resource isolation | Both positions defer 0069. Reject a dedicated executor, semaphore/admission cap, queue, overload 503, larger pool or builder drain here. Shared-pool saturation and surviving finite work remain measured limitations owned by #220. |

## Required behavior

**Public projection.** `WebhookTarget::redacted` keeps its synchronous embedding
API. Decode every certificate, preserve DER/order/duplicates, and reconstruct
padded base64 with at most 64 columns, LF separators and a final LF per block.
Validate the complete reconstruction through the uncached delivery builder;
labels or lazy certificate wrappers alone are insufficient. Never copy input
text, unrelated blocks or prose. Absent CA is null; present unprojectable input
is `""` on any parse/load failure, including a later invalid root. Never return
a partial prefix or raw fallback. Preserve exactly url/inbox/secret/ca_pem,
null absent options and withheld secret, and remove all URL userinfo. Original
raw targets stay intact, including admitted mixed PEM and raw cache/budget
identity. Loadability is not expiry/hostname/revocation checking. Covert data in
loadable certificates or other public fields and erasure of internal keys are
outside this policy.

**Preparation.** GET owns a snapshot and releases its configuration guard before
awaited off-worker projection. PUT validates its entire owned list and prepares
the safe echo off-worker before its async owner performs exactly one synchronous
swap and returns that echo, without another await between swap and response.
A concurrent later writer may replace it; no submission-order guarantee is
added. Workers cannot install configuration. Invalid, failed or cancelled
preparation leaves the list whole. Preparation is offline and delivery-cache
neutral: no lock, entries, recency, builds or evictions. Synchronous `validate`
remains available. Preserve extractor rules and named, value-free validation
400s, with native fixed 500s for worker failure.

**Delivery.** Cache hits clone/touch under the existing map/deque mutex; misses
release it and build off-worker. Only the awaiting owner rechecks and publishes:
reuse/touch a concurrent winner or insert once, removing the current LRU if
full. Retain 32 entries and map/deque bijection. Drop losing and evicted clients
after unlocking. The no-extra-CA shared client stays outside this cache.
Count every successful actual delivery construction once inside the job,
including duplicate losers and completed work whose waiter was cancelled.
Hits, failures and temporary API builds count zero; evictions count actual
removals. Independently observed counters are not a transactional snapshot.

**Failure and lifecycle.** Delivery join failure is the fixed value-free
`webhook CA client construction task failed` attempt error; existing retries
and exactly one terminal outcome apply. No failed build partially publishes or
poisons the cache. GET worker failure returns native 500 with
`webhook response preparation failed`; PUT uses
`webhook configuration preparation failed` before replacement. Cancelled owners
cannot publish/install/send later. Finite CPU work may complete and increase
build counts without changing terminal delivery totals. Preserve shutdown order
and Store panic behavior; do not drain builders or launch replacements after
timeout. Pool saturation can still delay SMTP parsing/commits: the cache cap
is retention, and the 22.1-second network/backoff sum excludes queue/build time.

## Acceptance Criteria

All criteria are local proof obligations except the explicitly assigned
protected-review and CI/operator ownership in AC10. Existing tests listed in
the plan are candidates for reuse, not fresh passing receipts.

1. **AC1 — Canonical privacy.** Real unauthenticated PUT/GET with generated
   loadable roots and actual private bytes under PKCS#8/RSA/EC/encrypted labels,
   unrelated non-key blocks and prose before/between/after roots returns exactly
   canonical certificate bytes, DER order and duplicates. Include noncanonical
   wrapping, repeated roots and ordinary certificate-only input. Decoded
   response strings contain no private marker/body, unrelated-body or prose
   sentinels. Assert four fields/nulls, userinfo/secret removal and byte-exact
   raw installed configuration using diagnostics that never print those bytes.
2. **AC2 — Whole-bundle fail-closed behavior.** Direct installation followed by
   real GET and synchronous `redacted` cover empty/no-cert, malformed PEM,
   valid-prefix/later-invalid PEM or DER, and real private DER relabeled as a
   certificate. Every unprojectable present input yields `""`; absence yields
   null. PUT still refuses invalid complete bundles with named value-free 400
   before replacing any target, including HTTP targets. Mixed-PEM admission and
   verified HTTPS using a later root remain working.
3. **AC3 — Actual API ownership.** Exercise real GET and PUT preparation at
   actual builder-entry holds, separately holding PUT validation and projection.
   Runtime, configuration/cache observation and fresh SMTP progress precede
   release. Old configuration remains until whole-list validation and safe echo
   complete; success swaps once and echoes that list. Real GET/PUT leave cache
   counters, entries, recency and next eviction unchanged. Demonstrated
   cancellation of the handler's publication owner leaves no late install.
4. **AC4 — Exact protocol accounting.** Coordinate two same-key cold delivery
   misses via real SMTP, hold both builders, then release and observe two real
   TLS deliveries and `/metrics` build delta +2, one cached key and zero
   evictions. With a full isolated cache, two distinct misses yield build +2,
   eviction +2 and the two expected LRU victims; hits add zero builds/evictions,
   an evicted-key rebuild adds +1/+1. Assert winner reuse, map/deque bijection,
   <=32 entries and raw-text cache identity; quiesce jobs before exact deltas.
5. **AC5 — Failure/cancellation accounting.** Live GET/PUT worker failures return
   the exact native fixed 500s and preserve the complete installed list/cache.
   Actual delivery join/build failure retries to one success or four-attempt
   exhaustion, without failed-build increments/partial publication. Cancel/stop
   during held construction: after release observe successful CPU completion
   and its allowed count, but no late cache publication, HTTP send or terminal
   change and exactly one aborted outcome. Preserve stop order and Store panic
   symmetry. Returning from the builder seam alone is not proof its later
   counter increment has finished.
6. **AC6 — Finite placement proof.** With one async worker and four blocking
   slots, hold build A while ticker, warm hit, cold distinct B, HTTP/metrics,
   fresh SMTP greeting/NOOP/DATA 250 and immediate inbox query finish before
   release; A then delivers over verified TLS. Retain >=33-root real fan-out
   with eight SMTP sessions and immediate query after each 250. Record actual
   capacities and fixed deadlines. Independent-thread watchdog release means
   failure, never success. Cache/build guards cannot span construction/await.
7. **AC7 — Served contract and residual evidence.** Real `/openapi.json` retains
   GET 200/403/500, awaited off-worker text and exact GET failure wording, without
   PUT replacement wording. Pair with live errors and unchanged request/response
   shapes. Preserve delivery/GET pool exhaustion, concurrent GET capacity and
   cancelled-GET recovery proofs with recorded capacities. Identify their low
   availability residual as #220, without inferring arbitrary-load safety or
   implementing 0069.
8. **AC8 — Discriminating, private evidence.** Locally record separate sanitized
   regression-negative/restored-green results for raw echo, on-worker build,
   under-mutex build and GET-500 removal. Safe failure diagnostics precede these
   checks; no private fixtures/responses/panic payloads enter logs or receipts.
   Restore source before normal gates. Use generated fixtures, real servers,
   event gates/deadlines and decision-0020 placement; no sleep synchronization,
   probabilistic chaos or unprotected global hooks.
9. **AC9 — Preserved contracts and complete coverage.** All operative 0067
   clauses and 0068 map to assertions and runnable commands in the plan. Keep
   existing offline/no-dial, later-root trust, Basic auth/secret, proxy/redirect,
   body-sharing/retry/budget, pruning/shutdown and frozen-guarantee proofs green.
   Every house gate and required pinned proof passes on the committed final
   candidate, using the specified local engine; exact missed lines, branches
   and functions are all zero. No exclusions, allowlist widening or gate repair.
10. **AC10 — Reviewable completion.** Current accepted-0058 full-risk routing
    reviews the disclosure fix itself with separate Astra correctness, official
    DeepSeek Flash security, Sonnet 5.5 tests, Astra guarantees and independent
    Fable 5.1 results. Preserve existing bundle routing and security hold; no
    author-generated clearance or historical-review substitution. CI owns its
    charter-listed external/OS-specific checks; operator owns acceptance,
    sealing and publication. Those are downstream ownership, not new local
    macOS/network/GitHub/namespace acceptance requirements.
