> Historical #195 artifact restored on 2026-10-25. The current port and its
> proof obligations are governed by [webhook-ca-port-lows](../webhook-ca-port-lows/spec.md).
> Earlier stops and gate receipts below are dated history. 0067/0068 are
> accepted; 0069 was accepted for #220 on 2026-10-25 and is not implemented here.
> The low shared-blocking-pool availability residual is tracked by #220.

# Webhook CA verification recovery

Created: 2026-10-24. Status: draft. Epic #192, issue #195.
Run: `continue-epic-192-issue-195-usin-2b482394`.
Stopped predecessor: `epic-192-issue-195-verification--757ff57d`.
Starting candidate: `1b1f0df365c4336f1bd01d215e43ccab3002bfad`.
Implementation baseline: `02072482292d4ee7cc1a0cbbd518e21f6224b60b`.

## Feature and rationale

Recover trustworthy verification of the existing webhook CA privacy and
construction repair. A public GET or PUT response must expose only validated
public certificates while preserving original configured input. Cold delivery
clients must build on `spawn_blocking`, outside the cache mutex, before the
async owner re-locks and rechecks publication. SMTP acceptance still means
stored and immediately queryable.

The stopped predecessor received VERIFY-FAIL in
`.forge/results/d148a45b-47fa-430c-87ea-e5badf7fe1fb.json`: the actual launched
`/tmp/brokkr-main-latest-54d5/target/release/brokkr` had digest
`b9f646b6a9aa9e083bc71b51330fe52505f7416ffbf8e7f3e0d374b79458bc5e`, outside
the recorded build. The earlier original run's receipt
`.forge/results/4d4da8e7-fe92-4565-86c8-4fde50227082.json` rejected the
host-installed engine with digest
`408c3c4d69d4363afcfcfb1246e5a70b8fc705b412d80f19e35d58144100a086`.
Neither establishes a runtime defect. Preserve both failures and historical
test receipts. Correct actual engine launch and verifier dispatch, not merely
the executable used for a manual compile. Fresh proof
must assess the current candidate through the existing engine-class route,
including deterministic design validation, hermetic verification and protected
four-lens review. No journal rewrite, fabricated resume or reduced review lane.

This recovery adds evidence requirements, not another runtime change. The
[original specification](../webhook-ca-safety/spec.md) and its existing
[proposed 0062](../../docs/decisions/0067-webhook-ca-safety.md) retain their
runtime scope and status. This spec introduces no semantic amendment or new
decision declaration. Historical checked tasks do not discharge fresh gates.

## Authority and lawful continuation

Accepted [0048](../../docs/decisions/0048-rest-webhook-validation.md) §1
explicitly preserves unrelated-section admission. Select the user's allowed
certificate-only echo alternative; do not refuse otherwise valid mixed PEM.
0048 §3 and [0041](../../docs/decisions/0041-webhook-boundary.md) §3 require
public CA diagnostics, with secrets/userinfo withheld and no replay guarantee.
Whole-bundle certificate projection fulfills that purpose without changing
installed bytes. 0041's rejected CPU-offload alternative scoped its earlier
batch; its operative rules do not require a builder thread. This requested
follow-up preserves delivery policy, budgets and shutdown ownership.

Under [0045](../../docs/decisions/0045-delivery-repairs.md) §8 compatible work
may proceed while 0062 remains proposed. This interpretation does not accept
0062. If a concrete contradiction of 0041/0048 is discovered, identify its
exact clause, draft the next free numbered proposed amendment and stop the
dependent work for operator acceptance. Frozen guarantees remain inviolable.

[0050](../../docs/decisions/0050-delivery-gates.md) requires a chief HEAD
commit touching exactly one spec directory. Therefore this design necessarily
creates a descendant of starting candidate 1b1f0df: record that new full HEAD,
prove its only changes from that candidate are these three recovery documents
and its production blobs still equal implementation 0207248, and bind subsequent
gates and review to the descendant. Call the implementation bytes unchanged, never the
commit unchanged. Any later repair changes candidate identity again and
requires fresh verification and review. The engine selects transitions under
[0057](../../docs/decisions/0057-codex-delivery-recipes.md); this seat does not
launch or impersonate subsequent phases. No push, merge or issue close.

## Explicit panel synthesis

The current run-local `simplicity.md` and `robustness.md` under
`.brokkr/state/design/positions/` both advocate preserving the repair and
correcting actual verifier provenance. Both current positions agree on the
compatible implementation; there is no manufactured conflict between them. Adopt simplicity's recovery-first scope,
one parser/builder/cache owner and reuse of existing decisive tests. Reject
interpreting minimal scope as permission to skip fresh evidence or retain an
obsolete candidate identity. Cost: a small documentation commit and another
full proof cycle, despite unchanged production code.

Adopt robustness's whole-bundle loadability, direct-install protection,
publication recheck, cancellation and final-accounting obligations as mandatory
proof. Reject replacing them with label filtering or eventual fan-out success.
Cost: extra offline projection work and controlled lifecycle tests. Both
positions now support these obligations; they are not a new chief invention.

Rule for simplicity on potential resource hardening: no single-flight map,
semaphore, dedicated executor, builder actor, second cache, durable outbox,
new configuration, dependency or metrics family. The losing stronger-isolation
alternative would prevent some duplicate work or pool contention, but brings
new fairness, shutdown and cancellation contracts. Accept finite duplicate
builds, transient target-dependent resources and shared-pool delays. Robustness
wins on exposing and testing those limits rather than implying isolation.

Reject private-key input refusal: it changes 0048 admission unnecessarily.
Its forgone benefit is removing private sections from internal state; raw key
bytes may remain in memory. Reject new X.509/expiry/network preflight and
covert-secret detection claims: the existing delivery stack owns loadability.

## Acceptance Criteria

- **AC1 — Candidate and engine provenance.** Record baseline and final full
  commit IDs, clean tracked/untracked status, changed paths and relevant source
  blob IDs. Before boxed execution, hash the candidate-local `.forge/bin/brokkr`
  against `bundles/self/tooling.json` and pass its absolute path. The recorded
  digest is `e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`;
  the actual engine launch and boxed verifier dispatch must use
  `/home/valentin/source/feedback-loop-ai/swarmail-epic192-delivery/NEXT-195-fast/.forge/bin/brokkr`.
  Inspect the dispatched executable/digest evidence: a matching read hash or
  manual compile alone cannot satisfy this criterion. If a different executable
  is still supplied, retain the refusal and correct the launch context without
  changing tooling pins, gate scripts, policy or journal. Preserve both concluded
  failures. HEAD's three design artifacts must pass the pinned check.
- **AC2 — Public projection.** Real unauthenticated GET and successful PUT of
  generated roots mixed with real private-key bytes return no private labels
  or distinctive private-body sentinel anywhere in decoded responses. Exercise
  before/between/after-root sections and PRIVATE KEY, RSA, EC and ENCRYPTED
  labels. State honestly that relabelled generated bytes test filtering, not
  independent valid encodings of all key types. Output contains canonical
  CERTIFICATE blocks only, preserving root order/duplicates; four keys and
  absent-option nulls remain. Installed raw configuration is unchanged.
- **AC3 — Fail-closed admission and trust.** Direct `Webhooks::set` cannot
  bypass projection: no roots, malformed later roots, invalid DER or private
  DER labelled CERTIFICATE yield an empty supplied `ca_pem`, never partial or
  raw fallback. Absent remains null. Invalid PUT roots, including on HTTP
  targets, yield named value-free 400s without replacing any previous target.
  Mixed unrelated sections stay accepted; verified HTTPS using a later root
  succeeds. GET/PUT preparation is offline and delivery-cache-neutral. Existing
  Basic auth, secret delivery and no-proxy/no-redirect policy remain proved.
- **AC4 — Actual placement and SMTP control.** With one async worker, four
  blocking slots and actual cold build A held at its builder seam, require
  ticker, new SMTP banner/NOOP/DATA 250, immediate fresh-inbox count=1, metrics,
  warm hit and distinct cold build B before releasing A. Then A's verified
  TLS delivery succeeds. This proves both off-worker and unlocked construction;
  watchdog release cannot count as success. Preserve the discriminating
  direct-worker and under-mutex historical controls, with source provenance.
- **AC5 — Distinct-PEM fan-out.** Run 33 generated distinct-root real TLS sinks
  and eight concurrent fresh-inbox SMTP sessions with retention headroom and
  explicit deadlines. Every DATA 250 has immediate count=1; all 264 deliveries
  and each sink's eight receipts occur, successful builds/evictions advance,
  and retained entries are 32. Pair with AC4; eventual delivery alone is
  insufficient. Separately demonstrate queued SMTP parsing under exhausted
  blocking capacity and recovery after controlled release, without early 250.
- **AC6 — Cache publication.** Concurrent same/different-key misses recheck
  after unlocked construction, reuse a winner and preserve map/deque bijection,
  recency and <=32 entries. Drop displaced/losing clients outside the mutex.
  Count actual successful delivery builds, including duplicates and completed
  work whose waiter cancelled, and only actual evictions. Failed builds publish
  nothing. API temporary clients do not alter entries, counters or recency;
  built-in roots remain outside the cache.
- **AC7 — Lifecycle and atomic configuration.** GET projects an owned snapshot;
  PUT validates/prepares the entire list before one async-owner swap and echoes
  that list. Invalid, failed or cancelled preparation never installs later.
  Value-free join failures enter existing delivery retries and exactly one
  terminal outcome. Cancelled delivery waiters never publish/send late; finite
  detached work may increment builds but not terminal totals. Preserve stop,
  serve-task join, drained, dispatcher join; sweep through Lagged and retain
  abort/join accounting. Store mutation panic behavior remains unchanged.
- **AC8 — Guarantees and limits.** Preserve synchronous accept/store,
  subscribe-before-scan, oldest-only pruning and ingest extraction. No webhook
  completion dependency is added to SMTP acceptance. Preserve shared payloads,
  verified trust, four attempts, timeouts/backoffs, per-target budgets and
  warning policy. The 22.1-second network/backoff sum excludes queue/build time;
  neither the 32-entry cache nor this proof promises a global work bound,
  crash-complete webhook ledger or thousand-session latency isolation.
- **AC9 — Fresh gates and protected review.** Execute the entire authoritative
  [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change),
  locked and offline as required, with exact 100% lines/branches/functions,
  coverage selftest, publish/seal proofs, recorded-engine compile and licenses.
  Include fail-closed native-tls absence, pinned proof copies, shim integrity
  and verifier-required selftests. Preserve CI-only evidence distinctions.
  Obtain actual hermetic verifier and four independent protected review
  outcomes bound to the final candidate; unknown facts and security holds
  cannot be bypassed. Missing execution or any failure remains outstanding.

## Evidence status at drafting

Both current panel documents, the current intake and both historical intake
framings were read. Source inspection supports the selected continuation; it is not a passing test receipt. Both
historical verifier results name wrong executable digests, and the current
candidate-local hash matches the tooling record. No protected review or fresh
hermetic verifier result is claimed by this design. This seat returns only
`drafted`; completion requires AC9 and the engine's subsequent decisions.
