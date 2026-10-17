# 0036 — SMTP trickle lifetime and listener recovery

- **Status:** proposed — acceptance is the operator's.
- **Date:** 2026-10-18
- **Issues:** #77, #78, #79, #80 (run 14, epic #70).
- **Spec:** [Listener recovery](../../specs/listener-recovery/spec.md).
- **Amends:** [0027](0027-living-listeners-bounded-sessions.md) §§1–4,
  §9 and Consequences, through the dated clarification specified below.
- **Preserves:** frozen 0001–0003, per-test isolation 0004, extraction at
  ingest 0005, ordered awaited commits, and decisions 0020, 0034 and 0035.
  0036 is the next free number; no existing record is renumbered.

## Context

Both listeners reap before parking in accept, so the first subsequent peer
counts tasks that finished during the wait and can be refused despite a
free slot. Their degraded flags clear only after a successful accept,
leaving an idle recovered listener red. `run_on` passes SMTP's 300-second
idle value to POP3, despite the promised 600 seconds. HTTP graceful shutdown
closes admission rather than serving new stopping probes throughout drain.

The per-I/O deadline bounds one operation, not a session or a DATA phase.
A complete command or DATA line inside each window allows another window.
Issue #80 explicitly permits documenting that residual. The earlier
unqualified claim that tricklers cannot hold sessions forever is false.

## Ruling

1. **One reap at admission.** Move each nonblocking `try_join_next` loop to
   after successful accept and immediately before the cap check. Keep
   JoinSet as the count owner, exact full-cap replies, bounded refusal
   writes and abort-and-drain shutdown. Completed entries may remain while
   idle, bounded by the cap. A completion after the final reap can still
   race admission; EOF is not proof of Tokio task completion. Do not retain
   a retry-based test that accepts the stale-count defect.
2. **Health follows an observed poll outcome.** Pending accept and
   successful accept clear the existing flag; error sets it and enters
   the existing interruptible one-second backoff. Merely finishing the
   backoff never clears it. Consecutive immediate errors have no false
   healthy transition between them. Pending means waiting without an
   observed error, not proven descriptor availability. A later accept may
   fail and become degraded again. Existing HTTP status and stopping
   precedence remain.
3. **Document actual HTTP drain behavior.** Once HTTP graceful shutdown
   closes admission, new probes get connection refusal. An already
   accepted connection may get 503 stopping only if its request reaches
   the handler after stop; a response is not guaranteed. Keep the current
   10-second fence and existing shutdown watch. Do not add an HTTP listener
   or extend admission merely to preserve the old promise.
4. **Independent protocol defaults.** SMTP stays at 300 seconds and POP3
   uses 600 seconds, per read/write. Introduce
   `pop3::DEFAULT_IDLE_TIMEOUT` and `Config::pop3_idle_timeout`, set in CLI
   conversion and passed by `run_on`. Keep POP3's existing plain serve
   arguments. No CLI/env option or Pop3Config is added; existing session
   caps stay as wired. This amends 0027 §9's zero-diff config/help goal:
   help must tell the truth and Rust config literals acquire one field.
5. **Choose the documented lifetime residual.** Add no session lifetime,
   DATA lifetime or minimum DATA rate. Incomplete operations keep their
   existing deadline; complete-line progress permits indefinite session
   and DATA retention. At default settings an actor can occupy all 256
   SMTP slots and retain payload approaching 50 MiB per DATA connection,
   about 12.5 GiB aggregate before allocation and other overhead. This
   is not an RSS ceiling. Even oversized drain can continue indefinitely
   if complete lines arrive within each window. POP3 complete-command
   trickles can similarly occupy its slots. No claim of adversarial-client
   availability follows from the cap or idle bounds.
6. **Correct the contract without erasing history.** Implementation adds
   a dated controlling clarification to 0027, explicitly superseding its
   success-only recovery, top-of-loop admission, unconditional stopping
   response, unrestricted trickler claim and zero-diff config/help clauses.
   Its historical text and accepted status remain. The clarification
   links to this proposed record; it does not claim operator acceptance.
   README describes current behavior and the quantified residual, and the
   decision index lists 0036 as proposed.

## Rejected alternatives and costs

- **Duplicate reaps:** an extra housekeeping site is unnecessary for the
  admission decision. Cost: completed task entries remain until the next
  accept or shutdown, with bounded retention.
- **Clear before polling accept:** an immediate repeated failure would
  briefly report healthy after each backoff. Cost of choosing Pending:
  a small poll wrapper and tests, and no claim that idle health proves
  future acceptance under descriptor pressure.
- **EOF alone as a task-completion barrier:** the runtime completion flag
  may lag socket closure. Cost: an exercised cfg(test)-only observer in the
  deterministic regression, alongside public wire tests.
- **Only pass a POP3 constant at startup:** fewer fields, but the normal
  server startup path loses independently configurable in-process bounds.
  Cost of the chosen field: public Rust struct-literal source changes and
  corresponding tests; no command-line configuration expansion.
- **A 30-minute DATA bound:** would limit retention per connection, but
  does not prevent reconnection or command-only slot occupation. No workload
  requirement selects 30 minutes. A zero-remaining Tokio timeout can still
  accept a ready read, so the panel's proposed expression is not a strict
  total cutoff without further rules. Rejecting it accepts indefinite DATA
  buffer retention as quantified above. A future bound must prove repeated
  line trickles, slot release, buffered-boundary semantics and isolation of
  a commit already in flight; a silent-peer test alone is insufficient.
- **Whole-session lifetime or minimum rate:** cuts long legitimate sessions
  or adds rate-window state and policy without a supplied requirement.
  Cost: intentional indefinite complete-command occupation remains; this
  decision documents the exposure rather than eliminating it.
- **Keep HTTP accepting during drain:** changes lifecycle and can prolong
  stop to the full fence. Cost of retaining admission closure: new probes
  cannot rely on an HTTP stopping body.

## Consequences and proof

No new timer surrounds insertion or POP3 deletion. SMTP's silent mid-DATA
expiry, byte-exact reader, complete terminator, strict envelope and AUTH
rules remain intact. A final 250 still follows the awaited successful
commit and means queryable. None of the frozen guarantees is weakened.

Spec AC1–AC10 require real-server admission, health and independently wired
timeout regressions; immediate-error and default pins; accurate help and
drain documentation; and all six gates at exact coverage. For #80 the
red/green proof is a documentation contract test, because runtime behavior
is deliberately unchanged under the issue's permitted alternative. Its
availability and memory exposure are disclosed, not counted as repaired.

This record proposes the posture; it does not assert implementation,
operator acceptance, a successful commit, or delivery completion.
