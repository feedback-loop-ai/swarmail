# 0027 — Living listeners and bounded sessions: the accept loop survives, the health surface tells the truth, and shutdown arrives on time

- **Status:** accepted (operator, 2026-10-16 — "accept all")
- **Date:** 2026-10-15
- **Supersedes:** none
- **Amends:** [0022](0022-bounded-data-raw-bytes.md) — the plaintext idle posture ("the
  plaintext idle posture is unbounded by documented design", the deferred hardening follow-up)
  is amended **in place**: plaintext SMTP and POP3 sessions gain per-I/O idle deadlines
  (5 min / 10 min). [0018](0018-shutdown-observed-waits-and-explicit-bounds.md) §2 is amended
  **in place**, by the 0019 protocol: its "no abort-on-timer" sentence governs the SMTP
  session posture (unchanged — `abort_all` + drain); the **HTTP drain** gains the deadline
  that closes its one unbounded waiter, at 0018's own margin. 0018 and 0022 carry the dated
  `Amended by` marker; this document carries the amendment. (A landing task so the markers
  land with the code, exactly as 0024's did.)
- **Preserves:** [0001](0001-synchronous-insert-lossless.md) — the idle deadline is never
  armed across the commit; a mid-DATA expiry stores nothing (EOF semantics, verbatim); a
  forced exit mid-commit is SQLite crash territory — journal replay, refused-or-committed
  (0024 §4's outcomes); [0002](0002-awaits-subscribe-before-checking.md) — no new channel, no
  re-registered handler: SIGTERM joins the existing watcher and feeds the same `interrupted`
  watch; `/healthz` reads the watch statelessly; [0003](0003-oldest-only-pruning.md) — every
  new refusal (`421`, `-ERR too many connections`, `452 4.5.3`, expiry close) precedes any
  insert or append: nothing was promised, so nothing is torn; [0014](0014-one-ordered-mutation-boundary.md)
  — the boundary is untouched by every refusal (connection-level, pre-commit); [0020](0020-concurrency-tests-and-the-coverage-gate.md)
  — the gate stays exact 100/100/100, the seams are `#[cfg(test)]`-only with installing tests,
  no new cold path exists.

## Context

Issue #36: both hand-written accept loops return on any `accept()` error
(`src/smtp.rs:121-128`, `src/pop3.rs:62`), killing the listener and every live session for the
life of the process while HTTP keeps answering and `/healthz` lies
(`src/api.rs:410-415`, hardcoded `"status": "ok"`). Plaintext sessions have no idle bound
(0022's recorded posture), there is no session cap (`SESSIONS` is an id counter), and `RCPT
TO` appends without limit. Issue #37: `main` waits only on SIGINT — Docker/Kubernetes stop
with SIGTERM and the scratch image runs PID 1, where an unhandled signal is ignored, so every
`docker stop` burns its full grace period; the handler future completes after one signal, so a
second signal's disposition is nondeterministic by construction; and
`axum::serve(...).with_graceful_shutdown` waits for open connections with no deadline — one
client that stalls its request body holds `stop()` forever. Ground truth from the vendored
axum 0.8.9 (`serve/mod.rs`): each connection is a spawned, detached task; on the shutdown
signal the serve future awaits *every* task's completion through a close watch; a
never-completing body never completes; no per-connection handle exists. The two issues are one
load-bearing system: an accept loop that retries without idle bounds spins forever at EMFILE
because nothing ever releases an fd — the cap and the idle lease are what make the retry
survivable, and they land together or the retry is a spinner.

## Decision

1. **The accept loops retry forever; the backoff is fixed and races the stop.** One policy for
   all accept errors — no errno taxonomy: on error the loop sets its listener's degraded flag,
   warns **at most once per backoff step** (never once per error — a sustained fault must not
   become a log flood), sleeps `ACCEPT_BACKOFF` (1 s, a per-module const, the
   `MAX_COMMAND_LINE` precedent) inside a `select!` against the loop's existing shutdown arm,
   and continues. A stop arriving inside a backoff sleep resolves the sleep at once; the drain
   never waits out a backoff. The flag clears on the next successful accept.
2. **The escalation point is readiness, at both poles.** A per-listener degraded flag
   (`Arc<AtomicBool>` pair, owned by the running server — never a global) is surfaced by the
   existing `/healthz`: `503 {"status":"degraded","degraded":[…]}` while any listener is not
   accepting, `200 {"status":"ok"}` when all are. The chart's probes (0026, all three on
   `/healthz`) make the persistent case *loud*: liveness red → the supervisor restarts the
   container — the recovery process exit would buy, through machinery that exists, and the
   restart is the operator's action for a resource-starved listener. Process exit is refused
   as the escalation: the red-proof is unobservable through a `#[cfg(test)]`-only seam (which
   does not ship in the spawned binary), and the exit destroys every live session on the first
   misclassified error — the exact collateral the retry exists to prevent. Log-only is
   refused: a probe-observable that keeps answering 200 is the lie the issue indicts. The
   HTTP listener gets no flag — its death is self-announcing (connection refused). At
   stop-start `/healthz` answers `503 {"status":"stopping"}` (stopping wins over degraded) so
   load balancers drain before the socket closes, and the drain deadline has an observable.
3. **Sessions are bounded where they live.** The cap is the live `JoinSet::len()` — one source
   of truth, checked after the top-of-loop reap (the landed reap discipline becomes
   load-bearing: an un-reaped completion must never cause a spurious refusal) — **256 per
   listener**, sized so 3 × 256 + ε stays under the common 1024 soft-fd floor (the issue's own
   repro axis). Over the cap the loop writes the refusal **under the shared deadline** (a
   zero-window peer must not hang the accept loop on a polite goodbye), drops the stream, and
   continues — SMTP `421 4.7.0 Too many connections, closing transmission channel`, POP3
   `-ERR too many connections` — before any session state exists: no append, no insert, no
   boundary traffic (0003/0014 untouched). Residual: a burst landing inside one reap window
   can refuse one honest connection, which retries; never a lost mail.
4. **The idle deadline is per-I/O and rides every peer await, reads and writes alike.** One
   shared bounded-I/O wrapper owns the single `Elapsed`→`TimedOut` arm; every peer read and
   reply write in both protocols' session paths goes through it. Values: SMTP **5 min** (RFC
   5321 §4.5.3.2's receiver timeout), POP3 **10 min** (RFC 1939 §3's autologout minimum).
   Semantics, stated honestly: no single read or write may wait longer than the bound for
   peer progress — it bounds silence *and* the pathological trickle. Expiry: between commands
   the server says goodbye (`421 4.4.2 … idle timeout, closing transmission channel` /
   `-ERR idle timeout`) then closes; **mid-DATA expiry is the EOF arm verbatim** — silent
   close, nothing stored — so a deadline can never manufacture a stored-but-unacknowledged
   state by itself; a timed-out *write* closes silently (a peer that won't read cannot hear a
   goodbye), through the existing error path with no new arm. **The deadline is never armed
   across the commit** (0001): the insert is neither raced nor timed. The 0022 reader stands
   byte-exact — no `fill_buf` surgery, no per-byte reset.
5. **`RCPT TO` is capped at 100** (RFC 5321 §4.5.3.1.8's minimum). The check **precedes** the
   append; recipient 101 answers exactly `452 4.5.3 Too many recipients`, the session
   continues, and the mail may still complete to the recipients already accepted — the refused
   recipient is no insert, no eviction, nothing stored (0003 untouched by construction).
6. **SIGTERM and SIGINT take one graceful path; a second signal is a designed exit.** The
   existing spawned watcher (registered before the servers bind, per 0002) owns both signal
   streams for the process lifetime (under `cfg(unix)`: `SignalKind::interrupt()` +
   `SignalKind::terminate()`), `select!`s their first `recv()` into the **same** `interrupted`
   watch, then `select!`s again: any further signal is `std::process::exit(70)` — the
   operator's explicit "now" — never a nondeterministic default disposition. Exit vocabulary:
   **0 graceful, 70 forced**, documented beside the other exit codes. The forced exit is
   crash-safe by 0001's arithmetic: a `250` implies the commit completed before the reply byte
   hit the wire; a commit in flight when `exit(70)` lands is journal-replay territory —
   refused-or-committed, never partial. The startup registration window (a signal before the
   watcher's first poll kills at the default disposition) is accepted: microseconds, nothing
   stored, and a container killing a half-bound server deserves the kill.
7. **The drain deadline is 0018's whole fence, and it ends the wait.** `api::serve` derives both
   futures from the watch `AppState` already carries (`shutdown_signal(rx)` — 0018 §1's
   primitive, resolving at once if stop already fired, so the clock starts correctly) and races
   the graceful serve against stop + `sleep(DRAIN_DEADLINE)`; `DRAIN_DEADLINE` = **10 s** —
   0018 §2's whole stop budget (busy bound 5 s + margin 5 s). *Amended from the first
   landing's 5 s margin by evidence:* the two tails overlap, they do not stack — a commit
   already in flight when the stop fires runs to completion (0018 §2) and burns up to its busy
   bound *inside* the drain window, so a deadline at the margin alone can cut a lawful refusal
   before its 503 reaches the socket; macOS CI caught exactly that, a refused seed closed with
   no answer at all under the runner's slower scheduling. At the deadline the serve future is
   dropped — the drain
   stops waiting. Because axum's per-connection tasks are spawned and detached with no
   per-connection handle, the deadline cannot politely close them from here; what closes the
   remaining sockets is the process exit that immediately follows in the shipped binary, and
   the runtime drop for an embedder — the 0024 §6 posture, extended to connection tasks and
   named as the residual. Worst-case process wall = **0018's fence (10 s)**, with any detached
   blocking tail already spent inside it; the deadline is derived from the fence, not
   invented beside it. No process-wall *assertion* is added — the fence is proven on the
   server's own clock (the stalled-body `stop()` deadline, the queued-commit refusal test)
   plus end-to-end liveness in the binary legs, per the landed `tests/binary.rs` discipline.
8. **`STOPSIGNAL SIGTERM` in the Dockerfile; the chart is untouched.** The scratch image
   declares the signal its handler actually handles. Kubernetes' default grace period (30 s)
   already clears the fence, so the chart needs nothing; bare `docker stop` (default 10 s
   grace) meets the fence at the knife's edge, and a SIGKILL landing exactly there converts
   the tail into crash-replay — honest, refused-or-committed — so the loss is grace, never
   correctness.
9. **No knobs.** Every bound is a compiled default, test-overridable in-process
   (`SmtpConfig` fields on the `tls_handshake_timeout` precedent; two plain parameters on
   `pop3::serve` — no `Pop3Config` for a two-number tuple); `config.rs` ends at zero diff
   (`..SmtpConfig::default()`) and the CLI byte-identical. No env, no flag, no new metric
   (0022 §5 carries); a knob is a one-`#[arg]` follow-up if an operator story materializes.

## Rejected alternatives — and what rejecting them costs

- **An errno taxonomy (immediate retry for `EINTR`/`ECONNABORTED`, else backoff) and a capped
  exponential ladder (1 ms → 500 ms) with an injectable schedule.** Rejected. A classification
  is a new failure mode — a misclassified fatal spins as "transient", a misclassified transient
  kills every live session, the exact collateral the retry exists to prevent — and a ladder
  needs mutable schedule state plus a test-only knob. *Cost:* under a burst of
  `ECONNABORTED`, accept throughput floors at one accept per backoff step for the storm's
  duration; each failed accept already lost its connection, the flag and the per-step warn
  name the condition, and the fence is one constant's value if a rate is ever demonstrated.
- **`/healthz` staying 200 with a `degraded` detail field during backoff ("calm, not
  lying").** Rejected — it keeps claiming healthy at exactly the surface the issue indicts,
  and it only works paired with a fatal/transient classification this decision refuses.
  *Cost:* a probe scrape racing a transient blip may sample one 503; the flag auto-clears and
  probe thresholds smooth one sample. Honesty over probe-stability.
- **Process exit as the escalation point.** Refused on two grounds: the red-proof is
  unobservable (a `#[cfg(test)]` seam does not ship in the spawned binary), and the exit
  destroys every live session on the first misclassification. *Cost:* none honest — the 503
  buys the same recovery through the supervisor, with the sessions alive to be drained by it.
- **Per-byte idle reset (deadline re-armed inside the `fill_buf` loop).** Rejected. *Cost:*
  none — it was the wrong fork: a trickler that keeps bytes moving inside one read call for
  longer than the bound is closed at the bound, the *safer* side (one fewer accepted
  residual), at the price of not indulging a deliberately slow sender; 0022's reader stands
  untouched.
- **Session cap 1024.** Rejected for 256. *Cost:* a fleet needing more than 768 concurrent
  sessions total is refused wire-honestly and retryably; the cap's purpose (fds under the soft
  floor) outranks the headroom.
- **A chart `terminationGracePeriodSeconds` pin and a process-wall ≤ fence assertion.**
  Rejected — the pin is pointless twice over (k8s default 30 s already clears the fence; bare
  docker is not chart-deployed), and the wall-clock assertion is refused by the landed
  `tests/binary.rs` discipline (a loaded runner stretches past 10 s; the budget is proven on
  the server's own clock). *Cost:* none — SIGKILL at the fence is crash-replay-honest.
- **Hard exit after `stop()` (the "Option B" fence closer).** Named, not smuggled: its own
  decision with the operator's eye, only if the 10 s arithmetic ever fails a real deployment.
- **Operator knobs (`--max-sessions`, `--idle-timeout-ms`, `--accept-backoff-ms`), a global
  byte budget, a total-lifetime cap, jitter on the backoff, an emergency-fd trick, an
  HTTP accept-loop rewrite, a new metric for degradation, a second (internal) stop channel.**
  All refused — each is surface the evidence does not demand, a 0003 collision (a global byte
  budget changes which mail is evicted), a second source of "stopping" (0002's one watch), or
  a rewrite whose cost is the moat itself.

## Consequences

- A transient accept error is a warn, a flag, and a 1 s pause — not a listener death and not
  an aborted session storm; the `ulimit -n 64` repro survives and delivers.
- A persistently unaccepting listener is *red* on every probe surface (503, named), never
  quietly half-alive; the supervisor owns the recovery, with the live sessions intact.
- Plaintext sessions can no longer be held forever by silence, by a zero window, or by a
  trickler — and cannot be held across the commit, ever (0001 verbatim).
- A container stop is graceful on SIGTERM, observable on `/healthz` (`stopping`), bounded by
  the fence in the worst case, and force-exitable by an operator who has said so twice.
- A stalled body can no longer hold `stop()` hostage; the drain ends at the margin, and the
  sockets close with the process that follows.
- Honest residuals, priced: one 503 sample on a blip (auto-cleared), the ECONNABORTED
  throughput floor during a storm, one honest connection refused per bursty reap window
  (retryable), the trickler closed at the bound rather than indulged, detached connection
  tasks dying at process exit (and with the runtime for embedders), and the startup
  registration window (microseconds, nothing stored).

## Clarification — 2026-10-18 (controlling)

This clarification controls where it conflicts with the text above, which stays as the
accepted historical ruling. It implements the posture of
[0036](0036-smtp-trickle-lifetime.md), which is **proposed** — acceptance is the operator's;
nothing here claims it. Issues #77–#80.

- **§1 and §2 — recovery follows an observed poll outcome.** *Superseded:* "The flag clears on
  the next successful accept." The flag clears when an accept poll is observed waiting
  (pending) or succeeds, and is set by an error; the end of the backoff alone never clears it,
  so back-to-back errors publish no healthy moment between them. Waiting is an observation,
  not proof that descriptors are available; a later failure sets the flag again.
- **§2 — the stopping answer is conditional.** *Superseded:* the unconditional "At stop-start
  `/healthz` answers `503 {"status":"stopping"}`". HTTP graceful shutdown closes admission:
  a new probe after that gets connection refusal. An already accepted connection gets
  `503 {"status":"stopping"}` only if its request reaches the handler after the stop; some
  close without a response. The 10 s fence of §7 is unchanged.
- **§3 — one reap, at admission.** *Superseded:* the cap "checked after the top-of-loop reap"
  and the Residual "a burst landing inside one reap window can refuse one honest connection,
  which retries". Each listener's only reap runs after a successful accept, immediately
  before the cap check, so completions that became ready while accept was parked never cause
  a refusal. A task that completes after that reap can still race admission; a peer's EOF is
  not proof the task has completed. Completed entries may wait, bounded by the cap, until the
  next admission or shutdown.
- **§4 and §9 — independent idle defaults.** SMTP keeps 300 s; POP3 uses its own 600 s
  (`pop3::DEFAULT_IDLE_TIMEOUT`) through `Config::pop3_idle_timeout`, set in the CLI
  conversion and passed by `run_on`. *Superseded:* §9's "`config.rs` ends at zero diff" —
  `Config` gains that one field (a Rust struct-literal source change) and the `serve --help`
  text is corrected to name both defaults. No flag, env variable or `Pop3Config` is added.
- **§4 and Consequences — no session or DATA lifetime.** *Superseded:* "it bounds silence
  *and* the pathological trickle" and "Plaintext sessions can no longer be held forever by
  silence, by a zero window, or by a trickler". The bound is per read and per write: one
  incomplete operation is cut off, but a peer that completes a command or DATA line inside
  each window earns another, indefinitely. There is no session lifetime, no DATA lifetime and
  no minimum transfer rate. At the defaults, complete-line tricklers can occupy all 256 SMTP
  slots indefinitely, and each DATA connection can retain a payload approaching 50 MiB —
  about 12.5 GiB across 256 connections before allocation and other overhead (an exposure
  estimate, not an RSS ceiling). An over-limit DATA drain has no total time bound while
  complete lines keep arriving, and POP3 complete-command trickles can hold its slots the
  same way. Silence and zero windows remain bounded; no availability guarantee against a
  deliberately slow client follows from the cap or the idle bounds.

## Pointer — 2026-10-18 (proposed 0046)

[0046](0046-smtp-pop3-edges.md), which is **proposed**, would amend §4 (issues #121,
#131): a failed SMTP session drops its transport with no graceful close, and a session
that ended well gets one more idle window for its close; a POP3 reply write is bounded per
successful socket write, each write that moves bytes renewing the window, instead of one
deadline over the whole reply. Nothing above is rewritten by this pointer.

## Later-decision markers — 2026-10-19 (issue #144)

Appended 2026-10-19 by the issue #144 provenance audit, after the operator's
2026-10-19 ruling accepting 0034–0050 and 0055. The text above — the ruling,
the 2026-10-18 clarification and the 0046 pointer — stays as written; each
marker names a later record with that record's own verb and scope, and
nothing above is rewritten by it.

- *(Amended by [0036](0036-smtp-trickle-lifetime.md) (accepted 2026-10-19):
  §§1–4, §9 and Consequences, through the "Clarification — 2026-10-18
  (controlling)" above. **Acceptance provenance:** that clarification said
  it implemented the posture of 0036 while 0036 was proposed and claimed no
  acceptance. 0036 is now accepted (operator, 2026-10-19), so the
  clarification is controlling on the authority of an accepted record. Its
  text is unchanged.)*
- *(Amended by [0046](0046-smtp-pop3-edges.md) (accepted 2026-10-19): §4's
  operation-level POP3 write bound and SMTP cleanup scope, as the 0046
  pointer above describes — a failed SMTP session drops its transport with
  no graceful close, a session that ended well gets one more idle window
  for its close, and a POP3 reply write is bounded per successful socket
  write instead of by one deadline over the whole reply. The pointer's
  "proposed" is historical: 0046 is accepted.)*
