# Observable metric and webhook budget proofs

Issue #178 under epic #192. Design date: 2026-10-24.
Run: `epic-192-next-178-luna-xhigh-del-5b30174f`.

## Feature and rationale

Repair the five observation defects in the existing REST, MCP and webhook
regressions. A zero registration delta is meaningful only after the same
counter and inbox have demonstrated a positive delta. A retry count proves
webhook attempts; independently querying accepted fixtures proves storage.
Equivalent URL spellings must actually be compared. Removing redundant sink
plumbing must retain the observation that makes a final count trustworthy.

This is a bounded test-only change: `tests/api.rs`, `tests/mcp.rs`,
`tests/webhook.rs`, and the existing inline test in `src/webhook.rs`.
Production behavior, public response shapes, dependencies, coverage rules,
recipes and accepted rulings remain unchanged. No new decision is declared:
strengthening proofs of existing semantics does not amend those semantics.

## Authority and delivery boundary

The governing authorities are [0001](../../docs/decisions/0001-synchronous-insert-lossless.md)
(accept means stored), [0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md)
(subscribe before scan), [0004](../../docs/decisions/0004-per-test-inboxes-via-smtp-auth.md)
(isolated real-protocol inboxes), [0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md)
(test placement), [0025](../../docs/decisions/0025-shared-domain-validation.md)
(refusal before effects), [0029 §5](../../docs/decisions/0029-supplied-values-are-validated-wherever-they-reach-a-wire.md)
(normalized per-target budgets), [0041](../../docs/decisions/0041-webhook-boundary.md)
and [0048](../../docs/decisions/0048-rest-webhook-validation.md)
(delivery boundaries), and [0050](../../docs/decisions/0050-delivery-gates.md)
(executed proofs). Proposed 0061 belongs to other work and grants no authority here.
Oldest-only pruning and ingest-time extraction also remain intact.

Triage already selected `engine`; retain that full design/review route under
[0057](../../docs/decisions/0057-codex-delivery-recipes.md). This is a fresh
candidate starting at `9b15fda841f6ea304326df6df23adf6a3c2f77cb`, retaining
recipe commit `450bef65c581b3a4791cf94dbc145912814bd6ed`, not a resumed stopped
attempt. Direct implementation remains Luna xhigh without fallback; test
review remains DashScope GLM 5.3 and DeepSeek review Flash only. No push,
merge, tag, publication, journal reconstruction or operator acceptance is
part of this design. Commit unsigned under accepted 0056.

## Panel synthesis and rulings

Both run-local positions were read in full: `.brokkr/state/design/positions/simplicity.md`
and `.brokkr/state/design/positions/robustness.md`, with the run's intake in
`.brokkr/state/tasks/`. They agree on the core repair; the choices below resolve
their differences instead of manufacturing a disagreement over that agreement.

- Adopt simplicity's four-file boundary and existing local helper pattern.
  Decline robustness's permission for a shared cross-file helper: local
  duplication keeps protocol actions and baselines visible together. Cost:
  a few repeated assertions; no registry reset, global lock or probe framework.
- Select simplicity's valid zero-timeout, nonmatching wait for every liveness
  setup rather than robustness's suggested seeded found arm on MCP. Require
  the real timeout response and exactly +1, not mere row presence. Cost: this
  setup does not prove a parked-wait race or found arm; retain those existing
  tests and readiness sequences as their separate proof owners.
- Adopt robustness's complete fixture identities and distinct returned ids,
  beyond simplicity's minimal single-mail subject/recipient readback. Count
  alone and eventual polling lose. Cost: indexed batch fixtures, concrete
  identity assertions and one REST list call per scenario after its sends.
- Adopt both positions' capture-channel closure replacement at zero window.
  Inspection confirms the listener task owns the remaining sender and no
  connection task is spawned at `Duration::ZERO`. Decline an extra done
  channel or completion handle now. Cost: this proof depends on that ownership
  and applies only to the zero-window case; any later ownership change must
  supply an equivalent observed terminal event, never an early atomic zero.
- Adopt robustness's small different-target inequality alongside simplicity's
  two equivalent spellings and retained variant/fallback checks. Cost: one
  additional measured assertion; gain: a constant valid-URL key cannot pass.
  Reject a normalization matrix, network normalization test or parser rewrite.
- Adopt both positions' new-exhaustion-before-hit-count ordering and temporary
  mutation controls. Reject shortened listener lifetimes, deleted closure
  coverage and sleep-based finality. Cost: existing finite retry horizons and
  real-server execution time remain; arbitrary scheduler delay is not solved.

## Acceptance Criteria

1. **AC1 — Live zero deltas.** Each existing refusal probe for REST `nocap`
   and MCP `nowaiter`/`capbox` first scrapes its exclusive label, performs a
   valid wait on its own protocol, verifies the timeout result, and observes
   exactly baseline + 1 on `swarmail_awaits_registered_total`. Only the initial
   pre-liveness scrape may interpret an absent row as zero. Post-liveness and
   refusal scrapes require the row. Refusals then show exactly +0 against a
   fresh post-control baseline, retaining their envelopes, field/range checks,
   active-gauge cleanup and legal-boundary/seeded-mail assertions. Renamed or
   removed instrumentation must fail each probe's liveness check.
2. **AC2 — Cumulative observations.** Every cumulative registration assertion
   in the affected tests compares before/after values for the same label in
   the same process. Specifically repair `observed408 == 1`, the REST
   malformed-timeout setup's absolute 1, and MCP `next177_establish_registration`'s
   absolute 1. Take observed408's baseline before spawning its wait; preserve
   its active readiness before the nonmatching SMTP delivery and the existing
   found-arm delta. No global registry reset or suite serialization. Active
   gauges, message counts and fresh per-server webhook ledgers retain their
   own semantics; this does not request a repository-wide metrics refactor.
3. **AC3 — Accepted fixtures are queryable.** Strengthen HTTP 500 retry,
   untrusted TLS retry, trusted TLS 500 retry, exact-bound 50-mail, and
   two-target cap+8 tests with an immediate real REST list read after their
   SMTP send(s) return, before awaiting retry exhaustion or budget progress.
   Require HTTP 200, exact total and array length, and fixture identity from
   the SMTP inputs. Singletons compare recipient, subject and text; batches
   have unique indexed subjects and compare the complete expected identities
   without ordering assumptions, with distinct nonempty returned ids.
   Request an explicit sufficient limit (at most 250) and retain disabled
   retention in these fixtures. No polling for missing mail, direct Store
   access, or expected identities derived from webhook captures. Preserve
   HTTP inbox-filter isolation, all existing stalled/refused/redirect storage
   witnesses, exact drop arithmetic, healthy-target progress and four attempts.
4. **AC4 — URL identity.** In `sink_keys_normalize_and_fall_back`, compare
   `http://EXAMPLE.com:80/hook` and `http://example.com/hook` as distinct input
   strings and assert equal parsed keys. Retain the Url variant, Raw variant
   and stable invalid-spelling fallback checks. Assert a different path's key
   is unequal. Raw valid-URL identity or a constant valid key must fail.
5. **AC5 — Simpler helpers, intact observations.** HTTP, TLS/TLS-config and
   redirect sink helpers return address, capture receiver and live Hits only.
   Remove final-count channel creation/send/return, update every destructuring
   caller and explanatory comment, and replace `hits()` with direct relaxed
   atomic loads at the existing valid observation points. For HTTP, redirect
   and TLS-config zero-window cases, boundedly await capture cancellation,
   then assert Hits == 0; explain sender ownership. Ordinary retry tests still
   baseline exhaustion before sending, wait for a new exhaustion and only then
   read Hits. Preserve 25-second listeners, accept-error retries, concurrent
   connections, first-completed whole-request capture, pre-handshake increments,
   TLS drop-first behavior and redirect destination shutdown/no-leak checks.
   A fifth-attempt mutant remains observable and fails the exact count.
6. **AC6 — Discriminating proof and delivery.** Collect isolated mutation-red
   and restored-green receipts for missing/renamed registration instrumentation
   in all three refusal probes, omitted accepted mail in singleton and batch
   readbacks, raw URL keys and constant valid keys, and a fifth attempt in
   HTTP/TLS budget tests. Each red must fail its intended assertion, not merely
   compilation or an unrelated timeout. Restore mutations before final gates.
   Run targeted and full affected suites, then all authoritative
   [house-rule gates](../../docs/house-rules.md#gates--part-of-every-change)
   including exact coverage/selftest, publish/seal proofs, license and
   fail-closed native-tls absence, with locked builds and pinned tooling.
   Protected verification/review and external CI evidence remain mandatory;
   design validation alone never closes #178 or epic #192.

## Limits

An immediate read is a live queryability witness, not crash/restart durability
or exactly-once SMTP. Webhooks remain lossy notifications with per-target,
not global, budgets. These tests do not newly prove hostile DATA, chaos,
shutdown or every subscribe/scan interleaving; retain their existing owners.
Use generated TLS fixtures and verification, not calendar-dependent fixtures
or insecure trust. Capacity arithmetic still has a finite exhaustion horizon;
do not weaken exact assertions to conceal slow-run failures.
