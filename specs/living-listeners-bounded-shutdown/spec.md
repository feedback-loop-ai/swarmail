# Spec — living listeners and a bounded shutdown (issues #36 and #37)

Issues [#36](https://github.com/feedback-loop-ai/swarmail/issues/36) (the accept loops die on
transient errors; sessions are unbounded) and
[#37](https://github.com/feedback-loop-ai/swarmail/issues/37) (SIGTERM is unwired; the drain is
unbounded), delivered as one batch at HEAD `cefe392`. The contrarian panel argued
`.forge/design/positions/simplicity.md` and
`.forge/design/positions/robustness.md`
(both `pass`); this spec is the reconciled ruling — it adopts, rejects and prices each side, and
rules where they genuinely conflict. The batch is semantic: it amends 0022's unbounded plaintext
idle posture and 0018 §2's no-abort-on-timer sentence, so it lands with
[0027-living-listeners-bounded-sessions.md](../../docs/decisions/0027-living-listeners-bounded-sessions.md)
(`proposed`; the operator accepts post-review, and the code and the decision land in the same
batch). Build on 0018 (the 10 s stop budget: busy 5 s + margin 5 s) and 0024 (at most one busy
wait) — do not re-litigate them.

## The problem

Two P1 defects of one lifecycle surface. **The accept loops die on any error**:
`src/smtp.rs:121-128` and `src/pop3.rs:62` both write `res = listener.accept() => res?` — one
`ECONNABORTED` (everyday), one `EMFILE` (the issue's `ulimit -n 64` repro), one `EINTR` — ends
the serve task; the `JoinSet` drops, aborting **every live session**, each possibly mid-DATA
holding bytes the client believes are accepted. HTTP keeps answering, so `/healthz`
(`src/api.rs:410-415`, hardcoded `"status": "ok"`) lies for the life of the process. **Sessions
are unbounded**: the plaintext idle posture is unbounded by documented design (0022 line 133 —
the follow-up this run is), there is no session cap (`SESSIONS` at `src/smtp.rs:78` is an id
counter, not a cap), and `RCPT TO` appends without limit (`src/smtp.rs:565`). **Shutdown arrives
late or never**: `src/main.rs:32-36` watches only `ctrl_c()`, so `docker stop`'s SIGTERM hits
the default disposition — and in the scratch image the process is PID 1, where an unhandled
signal is *ignored*, so every stop burns its full grace period; a second Ctrl-C after the
handler future completes is dropped, making the second signal's disposition nondeterministic by
construction; and `axum::serve(...).with_graceful_shutdown` (`src/api.rs:269-273`) waits for
open connections with **no deadline** — one client that sends headers and stalls its body holds
`stop()` forever (ground truth from the vendored axum 0.8.9: on the shutdown signal each
spawned connection task calls `graceful_shutdown()` and the serve future then awaits
*every* task's completion; a never-completing body never completes; there is no per-connection
handle).

## The ruling — how the positions were reconciled

Both positions hold the load-bearing frame, and this spec holds it with them: **the three #36
fixes are one load-bearing system and land as one decision** (robustness's composition argument,
which simplicity's own gate arithmetic concedes: an accept loop that retries without idle bounds
spins forever at EMFILE because nothing ever releases an fd; the cap and the idle lease are what
make the retry survivable), **#37's deadline is derived from 0018's fence, not invented beside
it**, and every refusal added here precedes any insert (0003 untouched) and never arms across
the commit (0001 untouched). Six points genuinely conflict; each is ruled below with the losing
side's cost named.

**Adopted from both: one watcher, two signals, one force-exit.** The existing spawned watcher
(`src/main.rs:32-36`) is extended — never re-registered — to `select!` over SIGINT and, under
`cfg(unix)`, `SIGTERM` (`SignalKind::terminate()`), both feeding the **same** `interrupted`
watch, so the whole shutdown tree (waits at their adapters per 0018 §1, the store boundary per
0024, the serve loops, the feed) inherits SIGTERM in two lines. The watcher **keeps owning both
signal streams for the process lifetime** and selects a second time: a further signal during
the drain is the operator's explicit "now" — a designed `std::process::exit(70)`, never a
disposition accident. Exit vocabulary is documented: **0 graceful, 70 forced**. The forced exit
is crash-safe by 0001's arithmetic, recorded in 0027: a `250` implies the commit completed
before the reply byte hit the wire; a commit *in flight* when `exit(70)` lands is SQLite crash
territory — journal/WAL replay on restart, refused-or-committed, never partial (0024 §4's
outcomes survive unchanged). The accepted residual (both positions): a signal arriving between
process start and the watcher's first poll kills at the default disposition — microseconds,
nothing stored, a container killing a half-bound server deserves the kill.

**Ruled against simplicity: the backoff sleeps race the stop arm, and the log is per step.**
Simplicity's error arm — log, set flag, sleep a fixed 1 s, `continue` — lets a stop arriving
*inside* the backoff sleep extend the drain by up to the constant, and under sustained EMFILE
its per-error `warn!` is a self-inflicted DoS on log-based alerting. Ruled: the backoff sleep is
wrapped in a `select!` against the shutdown arm the loop already owns (the signal can never wait
out a backoff), and the log fires at most **once per backoff step**, not once per error. Cost to
simplicity: ~4 lines, no more.

**Ruled against robustness: no errno taxonomy, and a fixed backoff, not a ladder.** Robustness
classifies `EINTR`/`ECONNABORTED` as immediate retries with everything else on a capped
exponential ladder (1 ms → 500 ms). Refused. A classification is a new failure mode — a
misclassified fatal spins as "transient", a misclassified transient kills every live session,
the exact collateral C36.1 exists to prevent — and a ladder needs mutable schedule state plus an
injectable schedule for tests (a knob for tests alone is surface). One policy, one `const` per
module (the `MAX_COMMAND_LINE` per-module precedent), retry-forever, loudly: a genuinely
unfixable error retries at one warn per second with `/healthz` red — visible, bounded CPU,
honest. **What robustness loses:** under a burst of `ECONNABORTED`, accept throughput floors at
one accept per backoff step for the storm's duration; each failed accept already lost its
connection, the flag and the log name the condition, and the fence is `1 s` — a value change,
not a redesign, if the burden is ever demonstrated.

**Ruled against robustness: the escalation point during backoff is a 503, not a
200-with-detail.** This is the sharpest conflict. Robustness argues a listener in backoff will
serve again, so `/healthz` stays 200 with a `degraded` detail field — "calm, not lying". Refused,
on the framing's own letter: C36.2's red-proof is "with the accept loop forced dead through the
seam, the health surface stops claiming healthy" — a 200-with-detail *keeps claiming healthy at
exactly the surface the issue indicts* ("HTTP keeps answering, so `/healthz` lies"), and it only
works paired with a fatal/transient classification this design refuses. Ruled: one
**per-listener degraded flag** (an `Arc<AtomicBool>` pair owned by the running server — not a
global; per-server state keeps tests race-free per 0020 rule 5), set when an accept fails,
cleared on the next successful accept; `/healthz` answers **503** with
`{"status":"degraded","degraded":["smtp","pop3"],...}` while any listener is not accepting.
Under the chart's probes (0026 wired all three to `/healthz`) the liveness probe turns red and
the supervisor restarts — the same recovery process exit would buy, through machinery that
exists, and the restart *is* the operator's action for a resource-starved listener. **What
robustness loses:** a probe scrape racing a transient blip may sample one 503 (probe thresholds
smooth it; the flag auto-clears — honesty over probe-stability: a probe that never flaps is a
probe that lies during the storm). **What robustness wins anyway:** the 503 body names the
degraded listeners (the autopsy is in the body), and the stop-start flip — the moment the stop
watch fires, `/healthz` answers **503 `{"status":"stopping"}`** so load balancers drain before
the socket closes (robustness's quiet break #10, which simplicity never named; the drain
deadline finally has an observable). Process exit is refused as the escalation for the two
grounds simplicity proved: the red-proof is unobservable through a `#[cfg(test)]`-only seam
(which does not ship in the spawned binary), and the exit destroys every live session on the
first misclassification. The HTTP listener gets no flag — its death is self-announcing
(connection refused: every probe fails).

**Ruled against robustness: the idle deadline is per-I/O, not per-byte — but writes are armed.**
Robustness wants the deadline to reset on any received byte (a `fill_buf`-loop surgery inside
0022's reader) and shows a real hole in simplicity's shape: a deadline guarding only reads lets
the server park a session task forever *writing* to a zero-window peer — the leak reopened
server-side. Ruled: both sides split. The deadline is armed at the **call sites of every peer
read and reply write** (simplicity's shape — 0022's bounded reader stands byte-exact, no
internals touched), via **one shared bounded-I/O wrapper** whose timeout arm exists once and is
driven warm by the read-side tests (robustness's write-arm adopted at simplicity's line cost).
The semantics, stated honestly: **no single read or write may wait longer than the bound for
peer progress** — it bounds silence *and* the pathological trickle. The RFC's own granularity is
per-read-operation, so this is the anchored shape. **What robustness loses:** a client trickling
one byte per 299 s inside one read call is *closed* at the bound rather than indulged forever
within the cap — the safer side of the fork, and one fewer accepted residual (its "slow-drip
trickler holds a slot indefinitely" cost sentence is thereby improved on, not inherited). What
is never armed: the commit. The `250`-after-commit ordering is 0001 itself; a timer that could
fire across an in-flight commit would manufacture the half-landed state 0024 §4 forbids. Expiry
semantics: between commands the server says goodbye — SMTP `421 4.4.2 … idle timeout, closing
transmission channel`, POP3 `-ERR idle timeout` — then closes; **mid-DATA expiry inherits
EOF-means-nothing-stored verbatim** (a silent close, nothing stored), so the deadline can never
manufacture a stored-but-unacknowledged state by itself; a timed-out *write* closes silently
(a peer that won't read cannot hear a goodbye).

**Ruled against simplicity: the session cap is 256 per listener, not 1024.** The cap is the
fence that makes retry-forever survivable at the fd level; 3 listeners × 256 + ε stays under the
common 1024 soft-fd floor — the issue's own repro axis. Simplicity's 1024 could blow a 1024
limit with the listeners and stdio alone. **What simplicity loses:** a fleet needing more than
768 concurrent sessions total is refused — wire-honestly (`421 4.7.0 …` / `-ERR too many
connections`) and retryably, with in-process test overrides. The cap is the live
`sessions.len()` — one source of truth (a second counter would drift from the set it mirrors
and the drift would be a lie on the wire) — checked **after** the top-of-loop reap (the landed
reap-at-top discipline becomes load-bearing: an un-reaped completed session must never cause a
spurious `421` to an honest client), and the refusal is written under the same idle deadline
(a zero-window peer must not hang the accept loop on a polite goodbye) **before any session
state exists** — nothing appended, nothing stored, 0003 untouched. Residual (robustness, priced):
a burst landing inside one reap window can refuse one honest connection that retries.

**Ruled against robustness: POP3's idle bound is 10 minutes, not 5.** RFC 1939 §3's inactivity
autologout minimum is 10 minutes; 5 would violate the RFC the listener claims. SMTP takes 5
minutes (RFC 5321 §4.5.3.2's receiver timeout). **What robustness loses:** two constants
instead of one, and POP3 holds idle slots five minutes longer — the cap is the fence either way.

**Ruled on the drain: the deadline ends the *wait*, and the arithmetic says 5 s.** Both
positions converge on the deadline being 0018's margin (the budget's 5 s margin, given a
job — not a new number) and on amending 0018 §2 in place (dated marker, 0019 protocol): "no
abort-on-timer" governs the *SMTP session posture*, which is unchanged (`abort_all` + drain);
the HTTP drain gains the deadline that closes its one unbounded waiter. The mechanism is ruled
from the vendored source, not from hope: `api::serve` derives **both** futures from the watch
`AppState` already carries (`state.shutdown.clone()` before `router(state)` consumes it — no
signature change, no new channel, `shutdown_signal` is 0018 §1's own primitive and resolves at
once if stop already fired, so the clock starts correctly even for a late drain) and races the
graceful serve against `shutdown` + `sleep(DRAIN_DEADLINE)`; at the deadline the serve future is
dropped — the drain stops waiting. Because axum spawns and detaches its per-connection tasks and
exposes no per-connection handle (0024's own note), the deadline cannot politely close them from
here; what closes remaining sockets is the process exit that immediately follows in the shipped
binary (and the runtime drop for an embedder) — the 0024 §6 runtime-drop posture, extended to
connection tasks and named in 0027. **Robustness's "watchdog aborts the serve task" and
simplicity's "drop the serve future" are the same mechanism under the source's light** — a task
and its future drop together; the ruling is the *ground truth* about what force-close is
actually available. **Worst-case arithmetic (adopted from robustness, priced):** process wall =
drain (≤ 5 s) + one detached blocking tail (≤ 5 s, 0024 §6) = 10 s = 0018's fence. Robustness's
two closing moves for that knife's edge are **both refused**: the chart
`terminationGracePeriodSeconds` pin is pointless twice over — k8s' default grace (30 s) already
clears the fence, and bare `docker stop` is not chart-deployed at all — and a SIGKILL landing
exactly at the fence converts the tail into crash-replay, which is honest (refused-or-committed),
so the loss is grace, never correctness; the process-wall ≤ fence *assertion* is refused by the
repo's own landed discipline (`tests/binary.rs`: "a loaded runner stretches past 10s — the
budget itself is proven by the API-level held-lock tests on the server's own clock"). The fence
is proven on the server's clock (the stalled-body `stop()` deadline, the queued-commit refusal
test) plus end-to-end liveness in the binary legs. Hard-exit-after-stop stays *Option B* — its
own decision with the operator's eye, named in 0027 so it is never smuggled.

**Adopted from both, uncontested:** the RCPT cap is 100 (RFC 5321 §4.5.3.1.8's minimum), the
check **precedes** the append (`src/smtp.rs:565`), beyond it the server answers
**`452 4.5.3 Too many recipients`** and changes nothing — the session continues and the mail may
still complete to the recipients already accepted (a refused recipient is no insert, no
eviction, no boundary traffic: 0003/0014 stay out by construction). The accept-error seam is a
`#[cfg(test)]`-only, counted, one-shot-per-arm seam on its owning module (the `REAPED` precedent,
0020 rule 4) with its installing test — it touches no mutation boundary and no wire truth; it
makes the loop's error arm reachable, which is exactly rule-4's shape. `STOPSIGNAL SIGTERM`
lands in the Dockerfile; the chart needs nothing beyond the image default (verified: the chart
pins no grace period and relies on the 30 s default). No knobs anywhere — no
`--max-sessions`, `--idle-timeout-ms`, `--accept-backoff-ms`, no env vars (0022 §5's ruling
carries): compiled defaults, test-overridable in-process (`SmtpConfig` fields on the
`tls_handshake_timeout` precedent — `config.rs` ends at **zero diff** via
`..SmtpConfig::default()`; `pop3::serve` gains two plain parameters — no `Pop3Config` for a
two-number tuple); a knob is a one-`#[arg]` follow-up if an operator story ever materializes.
No new files, no new dependencies, no new routes or metrics; the only new wire strings are the
degraded/stopping bodies, the two refusal lines and the two expiry goodbyes.

## Why a decision document

The batch is semantic twice over: it amends 0022's recorded unbounded plaintext idle posture
(the explicitly deferred follow-up) and 0018 §2's no-abort-on-timer sentence (scoped to the
SMTP posture; the HTTP drain gains the deadline). It also fixes a new public posture: the
`/healthz` response contract gains the `degraded` and `stopping` states, the wire gains refusal
and expiry lines, and the process gains a documented exit vocabulary. 0027 records all of it,
the rejected alternatives with their prices, and the preserved guarantees. Its markers on 0018
and 0022 are an implementation task so the markers land with the code, exactly as 0024 did.

## Acceptance Criteria

Each criterion names the observable and the test that proves it; each must be able to go red.
All tests run against real servers over the real protocol (no mocks, 0004); 0020 rule 5 binds
every test edit — real deadlines, no sleep-as-synchronization, per-test fresh stores/inboxes,
chaos at the poles only, windows only through seams. The exact 100/100/100 coverage gate stays
exact: every new line of production code is executed by the test named here.

1. **LISTENER-SURVIVES-ACCEPT-ERROR (C36.1)** — with the counted `#[cfg(test)]` seam injected
   (both loops), one `accept()` fails, the loop logs a warn, sets its degraded flag, backs off,
   and then a **real client delivers a mail on the same listener** which is queryable
   immediately, with the flag observed set-then-cleared. Inline, branch-stable (the seam is
   flipped before the loop is polled), `src/smtp.rs` and `src/pop3.rs` inline tests. Red today:
   the first error returns from `serve` and the client's connect or send fails.
2. **HEALTH-STOPS-CLAIMING-HEALTHY (C36.2)** — with the seam forced persistent (both loops
   covered), the degraded flag stays set and `/healthz` answers **503** naming the degraded
   listener (wire, in-process: both serve loops and the real `api::serve` on one shared flag
   pair); direct-handler tests pin all four states (`ok`, `degraded`, `stopping`, and
   `stopping` overriding `degraded`). Red today: `/healthz` is hardcoded `"status": "ok"`.
3. **STOP-FLIPS-HEALTHZ-FIRST (C37.3's observable)** — the moment the stop watch fires,
   `/healthz` answers 503 `{"status":"stopping"}`; proven by the direct-handler test and on the
   wire inside the stalled-body test (healthz still answers, and answers *stopping*, while the
   stalled body holds the drain). Red today: 200 through shutdown.
4. **IDLE-BOUND-ENFORCED (C36.3a)** — with the idle bound overridden small in-process: an SMTP
   client that connects and goes silent before a command is answered `421 4.4.2 … idle timeout`
   and then reads EOF, having stored nothing; a POP3 client likewise answered `-ERR idle
   timeout`; a client that opens `DATA` and stalls is closed **silently** with exactly nothing
   stored (the EOF-means-nothing-stored semantics). The reply-write arm is the same wrapper's
   single timeout arm, driven warm by these read-side legs. `tests/smtp_edges.rs` /
   `tests/pop3`-shaped in-process legs. Red today: the server waits forever.
5. **SESSION-CAP-REFUSED-WITHOUT-COLLATERAL (C36.3b)** — with the cap overridden to 1
   in-process: the second SMTP connection is answered `421 4.7.0 …` and closed, the **first
   session is untouched and still delivers a queryable mail**; the POP3 twin answers
   `-ERR too many connections`; the cap counts `JoinSet::len()` after the top-of-loop reap (no
   spurious refusal after a session completes). Red today: the cap does not exist.
6. **RCPT-LIMIT-REFUSES-AND-CHANGES-NOTHING (C36.4)** — over the real wire, recipient 101
   answers exactly `452 4.5.3`, the session continues, and `DATA` completes a mail carrying
   **exactly the first 100 recipients** — no partial append, nothing stored for the refused
   one. `tests/smtp_edges.rs`. Red today: the 101st is appended.
7. **SIGTERM-IS-SIGINT (C37.1)** — `tests/binary.rs` spawns the real binary, delivers a mail to
   a data-file-backed store, sends `SIGTERM` (`libc::kill`), asserts clean **exit 0** within
   the liveness deadline, restarts on the same data file, and asserts the mail is queryable
   (0001 riding the new path). Red today: SIGTERM kills at the default disposition (non-zero).
8. **SECOND-SIGNAL-FORCES-EXIT (C37.4)** — with a stalled body holding the drain, the first
   `SIGTERM` starts the graceful path (observed on the binary's own stdout: the `shutting
   down` line — real synchronization, never a sleep) and a second `SIGTERM` force-exits with
   the documented code **70** well inside the liveness deadline. Red today: the second signal
   is ignored or kills nondeterministically.
9. **IMAGE-STOPS-ON-SIGTERM (C37.2)** — the Dockerfile declares `STOPSIGNAL SIGTERM`, and the
   SIGTERM leg of criterion 7 proves the handler exists for the signal `docker stop` sends; the
   chart is verified to need nothing beyond the image default (no grace-period pin — k8s'
   default 30 s clears the fence). The image build itself is the publish pipeline's, never a
   gate's.
10. **DRAIN-DEADLINE-BOUNDS-STOP (C37.3)** — `tests/lifecycle.rs` opens a real HTTP connection
    that sends headers and never finishes its body, calls `stop()`, and asserts it **returns
    within a real deadline** (upper bound only; the framing's letter), with `/healthz` answering
    `stopping` meanwhile (criterion 3). Red today: `stop()` never returns. The arithmetic the
    test family pins: drain ≤ 5 s (0018's margin), plus at most one busy tail (0024), inside
    0018's 10 s fence — proven on the server's own clock, never by a process-wall assertion.
11. **DECISION-LANDED** — 0027 (`proposed`) exists with the rulings above, and the dated
    `Amended by` markers are added to 0018 §2 and 0022's posture line; the wire bounds (idle
    values, session cap, RCPT limit, exit codes, healthz states) are documented where the other
    wire bounds live (README). The batch is refused from landing without the decision (the
    framing's own condition).

Coverage posture (0020): the gate stays exact 100/100/100; no `coverage(off)`; **no new harness
file is expected** — no new line is cold-path-only. The one genuine cold candidate (a *write*
deadline expiring) is structurally unreachable-by-design in a test (a peer that never reads
still buffers small replies) and is therefore *not written as a distinct arm*: the shared
wrapper's single timeout arm is the expiry code, and every arm that exists is driven warm. The
map from new lines to their tests lives in [plan.md](plan.md) § Coverage.
