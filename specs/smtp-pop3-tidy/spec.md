# SMTP and POP3 internal cleanup

Issue #189, epic #192; design synthesis dated 2026-10-24. Baseline:
`9b15fda841f6ea304326df6df23adf6a3c2f77cb`. Retain that candidate-local
engine preflight commit and recipe commit
`450bef65c581b3a4791cf94dbc145912814bd6ed`.

## What and why

Resolve findings 21.1–21.6 through local cleanup and focused regression
proofs. SMTP AUTH should validate borrowed bytes before owning a valid
identity. SMTP cap refusal needs one deadline owner. Existing SMTP and POP3
sessions must demonstrably survive accept errors, and test error injection
must restore its previous state on every ordinary scope exit. Move the
STARTTLS stalled-reply regression into the measured inline suite using a
branch-stable form; remove the duplicate persistence-test attribute.

No new protocol behavior, configuration, dependency, schema, runtime listener
framework or storage policy is requested. This is compatible implementation
and stronger proof of accepted contracts, not a semantic amendment. No new
decision is declared. Authority is [0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md),
[0027](../../docs/decisions/0027-living-listeners-bounded-sessions.md) as amended
by [0036](../../docs/decisions/0036-smtp-trickle-lifetime.md), AUTH rulings
[0028](../../docs/decisions/0028-wire-octets-auth-refusals-and-the-maildrop-snapshot.md),
[0034](../../docs/decisions/0034-data-terminator-boundary.md) and
[0046](../../docs/decisions/0046-smtp-pop3-edges.md), and coverage rulings
[0035](../../docs/decisions/0035-coverage-gate-refuses-by-effect.md) and
[0050](../../docs/decisions/0050-delivery-gates.md).

Synchronous accept == stored, subscribe-before-check, oldest-only pruning and
ingest extraction (0001–0005) remain binding. A final DATA 250 follows completed
storage; no new deadline may race a commit. Retain the JoinSet session owner,
one admission-time reap, interruptible one-second accept backoff, health
recovery on observed Pending/success, and abort-and-drain shutdown. Failed
SMTP sessions drop without additional peer I/O; successful cleanup stays
bounded. POP3 session writes retain their per-positive-write progress bound,
and cap refusals retain their whole-operation bound.

## Panel synthesis and rulings

Both positions were read from `.brokkr/state/design/positions/`, together with
the run's intake framing in `.brokkr/state/tasks/`. Their source inspections
support this design; they are not executed implementation proofs.

| Question | Ruling and rejected alternative | Cost |
|---|---|---|
| Runtime scope | Adopt simplicity's existing owners and robustness's preservation constraints. Reject a shared listener framework, generic injection library, feature flags and broader failure-policy work. | Two small protocol-local guards and two protocol-specific survival tests remain duplicated. |
| Survival strength | Robustness's stateful proof wins over simplicity's minimum command exchange: keep an SMTP envelope and authenticated POP3 snapshot across the fault, then use both while retry is held. Both require real connections and existing probes. | More fixture setup; a fresh connection after recovery or an unfinished task handle alone earns no survival credit. |
| Injection cleanup | Adopt both panels' previous-value Drop guard and robustness's explicit thread affinity, polled cancellation and nonzero nesting proofs. Reject task-local migration of all counted-error callers. | Count injection still supports only one independent installing scope per protocol/thread; lexical nesting is LIFO, not concurrent-listener isolation. |
| STARTTLS placement | Adopt simplicity's bounded completion-wait candidate and robustness's measured coverage and behavioral mutation conditions. Reject an unchanged move justified only by the POP3 twin, and permanent exclusion justified only by the old comment. | Inline migration must earn coverage and red-proof evidence. If a necessary cold path cannot be made branch-stable, the requested move remains unresolved. |
| AUTH allocation | Adopt borrowed strict validation and an owned successful result. Reject allocator instrumentation or Cow machinery. | PLAIN's pre-validation byte copy disappears; successful PLAIN still owns a String. LOGIN may allocate a new String instead of reusing its decoded Vec. No universal allocation-count improvement is claimed. |
| Cap refusal | Adopt direct use of SMTP write_line as the sole timeout owner. Reject changing POP3's distinct session-write policy or extracting another refusal helper. | Whole-write-plus-flush timeout semantics remain; tiny wire replies alone cannot prove backpressure expiry. |
| Failure ledger | Retain robustness's existing-loss boundaries as constraints, not new work. | No exactly-once delivery, durable webhook replay, global memory ceiling, minimum rate or session lifetime is added. |

The acceptance phrase “inline under rule 2” means evaluate whether the cold-path
exception applies. Rule 1 makes branch-stable tests inline; rule 2 requires a
harness for a genuine cold path. Accepted 0046 records the previous fixture's
cold-path classification and preserves that general rule. Rewriting the
fixture into a measured branch-stable form applies the rule; it does not
reverse it or edit historical decisions. If that cannot be demonstrated,
return the exact unresolved conflict for operator resolution, with no coverage
exception or claim that #189 is complete. A contradictory semantic proposal
would require its own proposed decision and operator acceptance before work
that contradicts an accepted ruling.

## Acceptance Criteria

1. **AC1 — Measured STARTTLS regression (21.1).**
   `smtp::tests::starttls_stalled_reply_releases_slot` is inline in
   `src/smtp.rs`; its obsolete sibling file and declaration are removed once
   empty. Retain generated trusted TLS credentials, real STARTTLS, an unread
   open TLS half, traffic sufficient for backpressure, actual server session
   task completion while accept is parked, and fresh 220 admission before
   releasing the stalled client. Setup, flood, observation and fresh admission
   have an independent finite failure bound and failure-safe task ownership.
   The inline body is measured with zero missed lines, branches and functions.
   Restoring unbounded failed-session TLS shutdown makes the same regression
   fail behaviorally within the bound; compilation failure or premature client
   closure is not a red proof.
2. **AC2 — Borrowed AUTH validation (21.2).** `auth_inbox` accepts `&[u8]`,
   validates strict UTF-8 and unchanged `validate_inbox`, then returns an owned
   valid identity. PLAIN passes its authcid slice without `to_vec`; LOGIN
   borrows its decoded buffer. Helper and real-wire tests retain invalid
   base64/UTF-8, blank/control names, exact three-field PLAIN, arbitrary
   passwords, verbatim padded names, state precedence, refusal without identity
   effects and one successful AUTH per TLS epoch. Successful PLAIN/LOGIN
   delivery reaches the exact named inbox. Caller inspection proves removal
   of the specific temporary copy; no allocation benchmark is required.
3. **AC3 — One cap-refusal bound (21.3).** SMTP calls `write_line` directly
   for cap refusal, with exactly one `bound_io` covering write_all plus flush.
   Preserve TCP_NODELAY-before-reply, exact
   `421 4.7.0 Too many connections, closing transmission channel\r\n`, socket
   release, no session spawn and no store effect. Full-cap, later-admission
   and bounded-write/flush expiry tests pass. POP3 cap and progressing-session
   behavior are unchanged.
4. **AC4 — SMTP session survival (21.4).** Greet and select a fresh inbox on
   one real client, start its envelope before the fault, then inject one accept
   error using its listener probe. Wake parked accept with a second socket;
   observe actual backoff Pending with retry readiness held false. Complete
   RCPT/DATA on the original client while held, verify exact envelope/body and
   queryable returned ID in that fixture's Store immediately after final 250.
   Only then release retry and prove subsequent admission and health recovery.
5. **AC5 — POP3 session survival (21.4).** Seed a fresh maildrop, greet and
   authenticate a real client before injecting the same observed/held fault.
   STAT/LIST and RETR on that original connection while held must report the
   expected snapshot and exact message. Release retry only afterward and prove
   admission/recovery. For both AC4 and AC5, a mutation aborting active sessions
   in the accept-error arm must fail the owning regression within its independent
   bound. Retain separate stop-during-backoff abort-and-drain proofs.
6. **AC6 — Scoped error seam (21.5).** Both counted thread-local injectors
   return a must-use, non-Send scoped guard restoring the previous count in
   non-panicking Drop. Every caller binds it, including the API persistent-error
   health test; there is no tail-only manual reset. Tests prove normal/early
   return, controlled unwind, nonzero lexical predecessor restoration, and
   cancellation after the installing future has actually been polled and is
   pending. A subsequent real serve on the same thread greets normally with no
   inherited error/degradation. Normal cleanup stops/aborts and joins listeners
   before guard teardown; failure/cancellation cannot detach an injecting owner
   that consumes a later installation. No cleanup after process kill or
   panic=abort is promised.
7. **AC7 — Persistence declaration (21.6).** Exactly one `#[cfg(test)]`
   guards `mod persist_tests`; its name, file, contents and production exclusion
   remain intact. Cargo discovers and passes the existing persistence suite.
8. **AC8 — Delivery evidence.** Real protocols, fresh stores/inboxes,
   ephemeral ports, generated TLS and explicit observations establish ordering;
   no sleep synchronization, mock server, coverage exclusion, dependency
   widening or mass harness migration is allowed. Restore every mutant before
   final verification. All local gates in the authoritative
   [house-rules inventory](../../docs/house-rules.md#gates--part-of-every-change)
   and fail-closed native-tls proof must pass on the final implementation.
   Report external CI separately and do not claim #189 complete while required
   proof remains pending. Preserve recipe history, unsigned commits and the
   no-push/no-merge/no-tag/no-publish boundary.

## Evidence boundary

This is a design handoff. Acceptance criteria and checkbox tasks describe
future implementation evidence, not results already obtained. Triage selected
the engine lane; this spec neither reroutes it nor dispatches providers.
The design sequence's deterministic final check owns its verdict.
