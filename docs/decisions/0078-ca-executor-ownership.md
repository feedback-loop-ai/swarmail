# 0078 — Resolve bounded CA execution ownership and admission

- **Status:** proposed
- **Date:** 2026-10-25
- **Scope:** Epic #192, issue #220.
- **Specification:** [bounded CA executor](../../specs/bounded-ca-executor/spec.md).
- **Clarifies:** choices required by accepted [0069](0069-bounded-ca-execution.md), including its §5/0041 §5 relationship; records authorized additive resource refusals.
- **Preserves:** 0001–0005, 0023/0030 and Store panic symmetry; 0041 §5; accepted 0067 privacy/publication/no-builder-drain and 0068 fixed GET failure.
- **Number:** 0077 already exists in this candidate; 0078 is next free. Reserved gaps are not reused.

## Authority

The operator accepted 0069 on 2026-10-25, committed before this design. That
acceptance supersedes historical proposal/stopping language, but requires numeric
capacity, accounting and lifecycle selection before dependent code. 0067/0068
are accepted too. Their shared-pool/no-dedicated-executor limitation is superseded
narrowly by 0069; privacy and shutdown guarantees remain.

This proposal records compatible choices, without editing accepted records,
accepting itself or clearing security findings. It adds no second acceptance
prerequisite for already-authorized compatible implementation. A contradiction
of an accepted ruling still waits for operator amendment; frozen weakening is
refused regardless of acceptance or tests.

## Selected contract

1. One process-wide pair of dedicated native threads: W = J = 2, no pending
   queue, B = 67,108,864 logical charged bytes. All server GET/PUT preparation
   and custom-CA delivery misses share nonblocking admission. Reserve an idle
   worker/job/whole charge before extra snapshots/keys. Contention, exhaustion,
   overflow, oversized input, closed scope or failed initialization refuses.
   No shared-pool fallback or new dependency.
2. The specification's exact checked schedule is normative: Q = 65536 + 512*N
   + Vi + Vo + 4*S + 8*O + 8*(P+C) + 512*R + 65536*A, with its definitions and
   ownership table. Keep the full charge through handoff, running/abandoned work,
   retained reply and synchronous transfer/disposal. Allocation-category weights
   are not allocator/RSS measurements. HTTP extraction, installed state,
   pre-existing delivery snapshots, transferred cache/response allocations,
   transport, stacks and synchronous embedders remain outside. Do not narrow
   PEM admission or truncate output to fit.
3. Initialize once before server admission. Partial failure closes/joins only
   idle startup workers and retains unavailable state, without automatic retry
   or refusing unrelated SMTP startup. Successful workers are intentionally
   detached and survive server stops/runtime replacement; process exit owns final
   teardown. Offline construction captures no originating Tokio runtime.
4. Server-owned revocable scope closes at stop entry, owner drop/startup failure.
   Callers hold references without lifetime authority. Closure linearizes against
   admission and synchronous publication: pre-close effects stand, post-close
   results discard. No await under the short guard. Running cancellation never
   refunds work: the envelope/reply owns one reservation and final destructor.
   Workers cannot install/publish/send. A live owner authorizes sends; already
   begun requests retain existing abort/join behavior.
5. Preserve 0041 §5 stop → serve join → drained → dispatcher join and 0067 §4
   no-builder-drain. No per-server join, drain, reaper or replacement loop. Aborted
   delivery joins do not join builders. Finite work disposes without its old
   runtime. Catch ordinary job unwind with fixed errors and worker reuse; never
   format payloads, replace global hooks or change Store panic re-raise. Endless
   work can consume both slots until process restart.
6. Apply 0069's native GET/PUT 503 `webhook CA preparation unavailable`, keeping
   guard/query/extractor precedence, invalid-400 and preparation-500 outcomes
   (GET under 0068). Live owner swaps once after full validation/safe echo.
   Refused delivery admission consumes one ordinary attempt; existing retry,
   backoff, budget and terminal owners remain.
7. Preserve sole builder/cache/winner-recheck owner, complete certificate privacy
   and admission, 32-entry LRU and actual build/eviction counters. First successful
   reservation wins across classes/servers, with no starvation freedom. Oversized
   valid installed state may receive permanent GET refusal until replaced/cleared.
   No SMTP pressure refusal or Store/SMTP migration.

## Alternatives and costs

Both panels' process owner/two slots win over per-server pools or shutdown joins.
Simplicity's 64 MiB wins over robustness's 128 MiB, paying earlier large-job refusal
for a smaller admitted-charge budget; neither is a throughput optimum. Robustness's
revocable scope wins over watch-only checks, paying a small synchronization
boundary for provable close/publication races. Simplicity's copy removal and
robustness's category accounting win over duplicate keys or a cache-only bound.
The explicit tariff replaces an unexplained multiplier without pretending opaque
library heaps have proven allocation maxima.

Reject fairness queues, shared-pool semaphores, second runtimes, generic schedulers,
single-flight caches, extra authentication/metrics and public knobs. Pay resident
threads, cross-server contention, 503 bursts, possible delivery loss and stuck-job
capacity until restart. Isolation covers blocking-pool slots, not OS scheduling.

## Proof and disposition

AC1–AC8 and paired tasks require saturated real GETs plus persistent SMTP DATA/
query progress before release, cancellation/accounting/close/runtime interleavings,
privacy/TLS/cache/dispatcher regressions and all pinned house gates. Negative
controls restore cleanly and never become production modes. Coverage stays exact.

The chief commits exactly this proposal and three spec artifacts. Implementation
may add its proposed index row. Accepted records/routing stay unchanged. The
sequence check owns structural validation; authorized current review offices own
correctness/security/tests/guarantees/independent findings. Only the operator
accepts decisions or seals/pushes the eventual branch.
