# Webhook CA port and LOW repairs

Date: 2026-10-25. Epic #192, issue #195. Design baseline: `deb418e`, whose
parent `aec4b46` is the supplied current-main candidate. This is a design,
not an implementation, verification receipt or security clearance.

Decision: 0077 — [Compatible webhook CA port](../../docs/decisions/0077-webhook-ca-port.md).

## Feature and rationale

Port the complete shipped #195 behavior and proofs onto current main, then
repair its three LOW non-security review findings. This base still echoes raw
`ca_pem` and builds cold delivery clients synchronously under the cache mutex.
Only fixing the LOW findings would leave the original defects in place.
Public webhook diagnostics must contain only validated certificates, API and
cold delivery preparation must leave async workers, and publication must remain
with the awaiting owner. Preserve main's newer contracts where code overlaps.

## Evidence and authority

Read both run-local positions, [committed intake](intake.md), the decision
index, and the authenticated `.forge/epic192/issue195/` packet. All 21
SHA256SUMS entries verified, including README, operator rulings, last review,
port-and-fix and the 17 patches. `.brokkr/state/tasks/` is absent; committed
intake supplies the framing. `.forge/tasks/` contains only unrelated #198
framing. The plan template is absent; the plan supplies required sections
inline. No journal, sibling worktree or external evidence was consulted.

The operator's 2026-10-25 ruling governs: 0067/0068 are accepted; 0069 is
accepted for #220 and its implementation is outside this feature. Patch 10
supplies 0067/0068 acceptance provenance; implementation restores their final
records as `docs/decisions/0067-webhook-ca-safety.md` and
`docs/decisions/0068-webhook-get-failure.md`. They are authority, not new decision
declarations. Preserve 0069's historical patch record as expressly directed;
explain its later acceptance for #220 in current framing and the index citation,
without rewriting that record. Earlier proposal-era stops and receipts remain
dated history, not current requirements.

Preserve [0041](../../docs/decisions/0041-webhook-boundary.md),
[0048](../../docs/decisions/0048-rest-webhook-validation.md),
[0070](../../docs/decisions/0070-object-only-chaos-and-seed.md), and main's
#197/#202 behavior. Proposed 0077 documents the public-contract port without
replacing or re-accepting the already accepted contracts.

## Panel synthesis and rulings

The panels agree on production architecture; there is no genuine conflict
over projection versus rejection or implementing #220. Their emphasis differs
on scope and proof. Rule explicitly as follows:

| Question | Ruling and accepted cost |
| --- | --- |
| Small diff versus full port | Adopt both panels' semantic port. Reject a LOW-only patch and whole-file replacement. Cost: reconcile all 17 patches and their proofs against newer main. |
| Public projection | Adopt whole-bundle validated canonical output. Reject private-key refusal, label-only filtering and raw/partial fallback. Cost: diagnostic output is not a PUT template, malformed direct installs hide valid prefixes, and extra offline builds consume CPU. |
| Minimal tests versus ownership evidence | Robustness wins on actual-owner cancellation, complete prior-state comparisons, winner TLS-client reuse and exact accounting. Helper-only/counts-only evidence loses. Cost: generated fixtures and owner-local cold-path harnesses. |
| Broad failure analysis versus limited runtime scope | Simplicity wins on scope and optional INFO cleanup. Retain robustness's frozen-guarantee regression checks; reject Store/SMTP redesign, durable outbox, global test serialization and generic diagnostic/injection frameworks. Cost: existing durability and scheduling limits remain. |
| Current design ownership | Reuse intake's `webhook-ca-port-lows` directory. Restore supplied historical safety/recovery/pool specs with dated pointers here. This rejects simplicity's preference to put all new text only in the old safety directory, costing one explicit authority pointer without another recovery family. |
| Panic isolation | Adopt simplicity's narrow generated-PEM key and robustness's mismatch/match/cleanup proof. Reject the unkeyed flag. Cost: unique roots and explicit one-shot ownership. |
| Shared-pool availability | Adopt the operator's #220 disposition and both panels' exclusion of 0069 implementation. Reject a new executor, cap/status, semaphore, single-flight cache, builder drain or hard cancellation. Cost: saturation can delay SMTP parsing/commits; cache capacity does not bound in-flight work. |

## Required behavior

One `WebhookTarget::redacted` projection preserves all four response keys,
withholds secret, removes URL userinfo and transforms supplied CA input. Parse
certificates with the admission parser, re-encode canonically in original order
including duplicates, and validate the complete reconstructed bundle through
the uncached delivery builder. Any parse/load failure yields empty supplied
output; absence stays null. Never copy unrelated sections/prose or private DER.
Installed raw targets and mixed-PEM admission stay unchanged. Covert content
inside loadable certificates and intentionally public fields are outside this
projection's secrecy promise.

GET snapshots after existing query admission; PUT owns its complete list.
Each performs sequential preparation in one blocking job outside config/cache
guards. Only the awaiting PUT handler installs after validation and safe-echo
preparation, once synchronously with no await before response construction.
The worker cannot install. Join failures give fixed value-free native 500s;
invalid input remains a named value-free 400 before replacement.

Delivery misses unlock before blocking construction. The async owner rechecks
and publishes or reuses a winner under a short lock; losing/evicted clients
drop after unlock. Preserve raw-text keys, 32-entry LRU, successful actual-build
counts including duplicate/cancelled waiters, actual evictions, cache-neutral
API preparation, verified direct rustls delivery, retries, budgets, shared
payloads and terminal ledger. Delivery join errors are fixed attempt failures,
not Store panic rethrows. Running CPU work can finish after cancellation but
has no installation/publication/send authority.

LOW repairs use the existing privacy helper and a non-header private-body
sentinel; eliminate non-JSON response/parser dumps without suppressing failures
or changing empty 204/304 handling. README says “never awaited by SMTP
acceptance” with the shared-pool qualification. Key API_PREPARATION_PANIC to
the intended generated raw PEM: mismatch retains it, match consumes once,
unlock precedes panic, and cleanup clears only its owning key.

## Acceptance Criteria

These are local implementation obligations, not implementation claims by this
design. The plan names proof owners and tasks pair each change with its test.

1. **AC1 — Complete compatible port.** Every patch 01–17 has the plan ledger's
   disposition. Preserve #197 query refusal before config access, #202 decoded
   metric identities/escaping and #181/0070 populated-state preservation with
   native seed 400 versus chaos extractor 422. Existing regressions pass; no
   old snapshot overwrites unrelated main behavior.
2. **AC2 — Certificate-only diagnostics.** Real GET/PUT with generated roots
   and private material under PKCS#8, RSA, EC and encrypted labels, unrelated
   sections/prose, noncanonical wrapping and duplicate roots returns exact
   canonical loadable certificates in order with duplicates. Assert four keys,
   null absent CA/withheld secret, stripped userinfo and unchanged complete raw
   installed state using fixed diagnostics. Direct-install malformed, relabeled
   private DER and valid-prefix/later-invalid inputs project exactly empty,
   synchronously and over GET. Preserve all-root TLS trust and mixed admission.
3. **AC3 — API owner and failures.** Held actual GET/PUT preparation proves
   off-worker execution without config/cache guards. PUT installs once after
   all-target preparation; cancellation and invalid/join-failed preparation
   preserve every prior URL, inbox, secret and raw PEM. Unrelated config/cache
   work progresses with spare capacity. GET returns its snapshot; preparation
   preserves cache entries, LRU and counters. Served OpenAPI retains main's
   400 and existing 200/403, adds accepted 0068's GET 500 with fixed message
   `webhook response preparation failed`, and preserves PUT's failure contract.
4. **AC4 — Delivery ownership/accounting.** Real SMTP/TLS/metrics tests prove
   unlocked off-worker builds, hit/duplicate/eviction/rebuild behavior, raw-text
   identity, winner's live TLS-client reuse and exact counter deltas in the
   serialized cache harness. Join failure, retry success, exhaustion,
   cancellation and shutdown preserve one terminal outcome; cancelled owners
   cannot late-publish/send. Review lexical guard/drop scopes; do not claim a
   behavioral destructor probe that does not exist.
5. **AC5 — Frozen semantics and honest progress.** With finite spare blocking
   capacity, held delivery/API construction permits SMTP DATA 250 followed by
   immediate queryability of each accepted message; retain >32-root fan-out.
   Preserve subscribe-before-check, oldest-only pruning, ingest extraction,
   Store mutation/panic rules, bounded DATA, budgets/payloads and shutdown
   through workspace regressions and source review. Claim no universal latency,
   resource independence or durable webhook delivery.
6. **AC6 — Safe LOW diagnostics.** The private-DER refusal case and its
   setup/state checks use safe helpers and boolean fixed-message comparisons.
   Use generated base64 body positions 64..96 when fixture length permits,
   not constant header `[..24]`. Force status, prefix, private-byte,
   state-comparison and malformed-JSON failures under in-memory output capture;
   none emits fixture bytes, request/response bodies or parser-supplied text.
   Non-JSON still fails; empty 204/304 remains absence. Retain only sanitized
   verdicts, exit codes and hashes, never raw captured output.
7. **AC7 — Honest LOW documentation.** README, webhook module and llms.txt use
   “never awaited by SMTP acceptance”, describe shared-pool dependence and
   identify #220. No unconditional “never blocking the SMTP path” claim remains
   there. A focused regression fails if that README promise is restored.
   Queue/construction time is excluded from the 22.1-second request/backoff sum.
8. **AC8 — Isolated LOW seam.** Deterministically arm request A, complete
   unrelated GET and PUT B without consuming/triggering A, observe A's fixed
   500, then a successful subsequent request. Cover matching GET and PUT;
   failed PUT preserves complete state. Mismatch retains, match consumes once,
   unlock precedes panic and cleanup cannot erase another key. Retain actual
   handler cancellation proof. No sleeps or whole-suite serialization.
9. **AC9 — Fresh discriminating evidence.** Retain sanitized red/restored-green
   controls for full projection (raw fallback and unvalidated projection),
   construction placement, served GET 500 and all three LOW repairs. Controls
   use disposable local fixtures and value-free receipts bound to command,
   exit status, restored source hashes, HEAD/tree and engine digest. Previous
   receipts do not certify this port.
10. **AC10 — Local delivery proof.** On the committed implementation, every
    local [house gate](../../docs/house-rules.md#gates--part-of-every-change)
    passes with `.forge/bin/brokkr` digest
    `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
    Include exact zero-missed line/branch/function coverage and selftest,
    publish/seal proofs, locked checks, licenses, shim integrity and fail-closed
    native-tls absence. No exclusions, lowered floor, installs or route/pin edits.

## Review and external ownership

The unchanged candidate recipe owns protected review of the entire ported
disclosure fix, API/cache lifecycle and LOW repairs. Preserve its five-office
risky-work routing and official Flash security office; this design dispatches
no reviewers or review results. Record the known low security/availability
residual as **tracked by #220**, neither new nor cleared. Report genuinely new
findings truthfully. Operator merge direction does not authorize policy edits
or hiding security facts.

CI owns macOS, network RustSec, Helm/chart proofs, Docker smoke, privileged
port-80 exchange and tag-only release proofs. Live GitHub settings, namespaces,
sealing and merge remain CI/operator owned, outside local ACs. No push, merge,
tag, publication, issue closure or decision acceptance. No production
Store/SMTP/extraction refactor, dependency change or 0069 implementation.
