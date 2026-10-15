# 0018 — Shutdown-observed waits, explicit wire bounds, and the thread fallback's exact scope

- **Status:** accepted (operator, 2026-10-13 — delivered by the post-P1
  closure run at 6ed8e3e and reviewed by its panel; the §6 posture and the
  landing task were superseded by 0019's run)
- **Date:** 2026-10-13
- **Supersedes:** none
- **Amends:** 0008 §4/§5 in place (proposed; the deployment-contract edits are a landing task of the closing run)
- **Amended by:** 0019 — §6's `v*`-prefix posture and the §Refused registry-probe item are superseded by the exact-ref bind and the fail-closed probe (a landing task of the closing run)
- **Amended by:** [0024](0024-shutdown-refusal-at-the-boundary.md) — §1's
  last sentence and §2's mutation mechanics: the store becomes a **reader**
  of the stop watch and `record()` refuses to **start** a commit once
  stopping, so a queued HTTP mutation never burns its own busy bound after
  shutdown begins — the process burns at most one busy bound (plus ε per
  queued release) in every interleaving, a commit already started runs to
  completion, and the refusal answers the fixed 0012 §2 text (a landing
  task of the issue-#32 slices 1/5/6 run).
- **Amended by:**
  [0025](0025-shared-domain-validation.md) — §3's refusal posture extends
  from the *bound* to the *supplied malformed value*: one shared validator
  (`ChaosRule::validate`, `checked_wait_timeout_ms`) now owns the rule
  both transports enforce, a present-but-unparseable `timeout_ms` refuses
  instead of silently defaulting, and an omitted value keeps its
  documented default (a landing task of the issue-#32 slice-2 run).
- **Amended by:**
  [0027](0027-living-listeners-bounded-sessions.md) — §2's HTTP drain is
  no longer unbounded: `api::serve` races the graceful serve against a
  5-second deadline, so a request that can never finish cannot hold stop
  hostage (the deadline ends the wait, not the sockets; the `abort_all` +
  drain session posture is unchanged). SIGTERM joins SIGINT in the same
  watcher, and a second signal during the drain exits 70.

## Context

Six post-P1 findings, none red to a gate: the session `JoinSet`s accumulate
completed entries (they join only in the shutdown arm); a parked `await`
holds `stop()` for the remaining client timeout because the wait races
nothing; `timeout_ms` is unbounded and the MCP chaos arm narrows wire values
(`as u8`, `unwrap_or(0)`); REST await reports a `total` captured before its
wait; `group_threads` welds explicit chains together through their subjects;
and the contract documents (`openapi.json`, MCP schemas, `llms.txt`) drift
from the server. The P1 run's standing order holds: the successor slice
reuses the boundary — `store.rs`/`persist.rs` end at zero diff, and a bulk
bypass (`delete_many_async`) stays refused.

## Decision

1. **Waits observe shutdown at the handler, on the feed's watch.** REST
   `await` and the MCP `swarmail_wait_for_email` race their wait against the
   existing `AppState.shutdown` watch (the same receiver the SSE feed
   consumes — one source of "stopping", never a second channel). The shared
   primitive is `shutdown_signal(rx)`: resolve at once when the stop has
   already fired (`borrow_and_update`), else on the next change — 0002's
   per-iteration re-check applied to the stop watch, never a single entry
   check. A dropped sender resolves as stopping: the last sender is gone
   only when the server is going away. The answer is `503` with the fixed
   text `shutting down` in REST and an `error_result` with the same text in
   MCP — the one new wire string. `WaitOutcome` gains no variant; the store
   is untouched. The store alone does not observe shutdown; any future wait
   surface must race the same watch at its adapter.
2. **The stop budget is the busy bound plus margin — 10 seconds — and an
   in-flight commit neither holds stop nor half-lands.** Stop completes
   independently of any client's wait timeout. A commit already in flight is
   not cancelled: the blocking pool runs it to completion; it lands after
   the socket died (the mail is in the store, the client saw EOF, not `250`;
   retry is lossless) or refuses; nothing half-lands. The accept loop's
   harvest (reap ready completions, never park) and the shutdown arm
   (`abort_all` + drain) are the lifecycle posture; no grace period, no
   abort-on-timer, no ack that outlives the drain.
   *(Amended by [0027](0027-living-listeners-bounded-sessions.md): the
   HTTP drain gains its deadline — `api::serve` races the graceful serve
   against a 10-second sleep (this fence: busy bound plus margin), so a
   request that can never finish cannot hold stop hostage. The deadline
   ends the wait, not the sockets; the `abort_all` + drain posture for
   sessions is unchanged.)*
3. **Wire bounds are refusals, never clamps.** `timeout_ms` above
   `MAX_AWAIT_TIMEOUT_MS` (60000) is refused — REST `400`, MCP `isError`,
   both naming the maximum — before any subscribe. MCP
   `swarmail_set_chaos` validates `probability` (present, integer,
   0..=100 — named refusals at each stage: presence, `u8::try_from`, the
   100 ceiling) and rejects wrong-typed `delay_ms` (absent stays 0 — honest
   "no delay"). No narrowing cast on any wire value; the input schemas
   publish `minimum`/`maximum` so compliant clients refuse at the tool
   layer. A clamp would be a silent narrowing — the same disease as the
   casts this decision removes.
4. **Await totals describe the response's observed state — counted after
   the wait, in both arms — with a named residual.** The REST response's
   `total` is taken at response construction; the MCP wait results carry
   the same `total`. The residual: the count is not atomic with the wait's
   last scan, so the pair can disagree by the inserts landing in that
   window — bounded by one insert, invisible to any deterministic test.
   Snapshot-atomic totals would edit the boundary's wait; that stronger
   invariant owes its own red criterion and decision.
5. **Explicit chains are authoritative; the subject fallback is
   orphan-only; merges pay in docs, not alias state.** A mail with ids
   threads by its References even when the parent is absent from the inbox;
   subject affinity groups chain-less orphans of the same normalized
   subject and never bridges two chains. A bridge mail referencing two
   threads merges them; the surviving key is the merged thread's anchor
   (oldest first mail, `receive_ms` then id) and every other pre-merge key
   answers a loud `404`, never a wrong thread. No alias table: the view is
   derived per request, and aliases would claim a stability the `VIEW_CAP`
   window already denies.
6. **Publication is one gated run, and each version publishes only from its
   own ref.** The docker and chart publish jobs `needs:` the five gate jobs
   of the same workflow invocation; each publish path binds its own
   manifest's declared version to the triggering ref by full-string equality
   and probes the registry before pushing, failing closed on an existing tag
   or an unreadable registry (0019); `main` pushes `:main` and `:sha-<sha>`
   only. A released version is never silently replaced by a branch build —
   with `Cargo.toml` at `0.1.0`, a main-publish would rebuild the same tag
   perpetually. Releases require pushing `v<version>` and an absent registry
   tag: a typo'd ref, a re-pushed stale tag and a registry that cannot be
   read all refuse before any push.

## Consequences

- `stop()` is operator-fast in every case: parked awaits answer `503`
  immediately, mid-DATA sessions die with their sockets, and a burning busy
  timeout does not hold the shutdown — at most one commit lands or refuses
  after the fact.
- Clients that asked for more than 60 seconds of await now get a `400`
  naming the bound — a loud breaking change for huge-timeout callers, and
  the point.
- MCP operators who set chaos with missing or mistyped probability now get
  an error naming the range instead of silence.
- A client holding a pre-merge thread key sees `404` until it re-lists —
  loud, recoverable, documented in the README's key-mobility clause.
- The contract documents and the server agree; a generated client stops
  meeting refusals the spec said cannot exist.
- Two chains that today merge on subject equality separate. Anything that
  keyed on today's over-merge sees two threads — the README being
  reconciled to the wire's truth, not a regression.

## Refused

- `wait_for` gaining a cancellation token, a `Shutdown` outcome variant, or
  any other signature — the adapter-level select is the whole change.
- (Rescinded by 0019) A registry-probe guard against republishing: both of
  this list's grounds fail — the probe uses the credential the push itself
  already requires (`packages: write`; nothing new is dragged anywhere), and
  the check-then-push race is priced, not pretended away (a concurrent
  publish of one version can still both see `404`; GHCR is last-writer-wins).
  The probe now stands in both publish paths, fail closed. See
  [0019](0019-release-tag-integrity.md).
- `values.schema.json`, a configurable await cap, a session-supervisor
  module, a shared harvest abstraction, deliberate-broken-value CI
  matrices, and any edit to `src/store.rs`/`src/persist.rs` for these
  findings.
