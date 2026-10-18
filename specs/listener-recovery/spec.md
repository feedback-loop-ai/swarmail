# Listener recovery and honest timeout contracts

Date: 2026-10-18. Design for run 14, issues #77–#80 (epic #70).
Implementation remains outstanding; this document does not report gate or
delivery completion.

## Feature and rationale

Offer a completed SMTP or POP3 session's slot to the next accepted peer,
clear stale degraded health when the listener actually resumes waiting,
and give POP3 its promised independent 600-second idle default. Describe
the remaining SMTP trickle exposure and HTTP drain behavior accurately.

The framing is `.brokkr/state/run14-feature.txt` and the four issue bodies
under `.brokkr/state/epic70/`. All three intake documents currently under
`.brokkr/state/tasks/` concern superseded run-12 delivery work, not run 14.
Both run-local panel positions were read. Their evidence is summarized here
so implementation does not depend on gitignored files surviving.

This changes public health/configuration contracts and corrects accepted
ruling 0027. Proposed decision
[0036 — SMTP trickle lifetime and listener recovery](../../docs/decisions/0036-smtp-trickle-lifetime.md)
records the semantic choice. Acceptance belongs to the operator. Decisions
0034 and 0035 are preserved, including their numbering and status.

## Panel rulings

| Disagreement | Ruling and reason | Cost of rejecting the alternative |
|---|---|---|
| #77: simplicity adds a second reap; robustness moves the reap. | Adopt robustness: one nonblocking reap immediately after successful accept, before the cap comparison, in each listener. The count's authority stays next to its use. | Finished task entries can remain while accept is idle or backing off, bounded by the session cap; the next admission or shutdown reclaims them. No second reap provides earlier housekeeping. |
| #77 proof: robustness treats QUIT/EOF as completion; simplicity requires actual task completion. | Adopt simplicity's stronger proof. EOF does not prove Tokio has marked the task complete. Use a test-only completion observer for the deterministic regression; retain real-wire smoke coverage. | A small exercised test seam costs more than an EOF-only test; it removes a scheduler assumption rather than hiding refusal with retries. |
| #78: clear before polling, or on observed pending accept. | Adopt robustness: clear on `Pending` or successful accept; set on error. Backoff expiry alone never clears. | A polling wrapper and its tests cost more than moving one store. Idle health still does not prove a resource-starved listener can accept a future connection. |
| #78 drain: both positions prefer accurate documentation. | Adopt both: keep existing HTTP graceful shutdown and its fence; new probes are not promised a stopping response. | A new connection after listener closure is refused; no guaranteed HTTP body throughout drain. |
| #79: fixed POP3 argument, or independent server config field. | Adopt robustness, resolving its unnamed-field gap: `Config::pop3_idle_timeout`, initialized from `pop3::DEFAULT_IDLE_TIMEOUT` (600 s), passed by `run_on`. SMTP remains 300 s. | One public Rust struct field and literal-site updates; embedders must initialize it. This buys independent end-to-end test overrides without a new CLI option or `Pop3Config`. |
| #80: documented residual, or 30-minute DATA deadline. | Adopt simplicity's explicitly permitted documentation alternative. Keep per-I/O bounds; add no lifetime or rate limit. | Complete-line tricklers can hold all SMTP slots and substantial DATA buffers indefinitely. This is an availability exposure, not a hostile-client mitigation claim. |

The DATA deadline would limit one buffer's retention but would not prevent
immediate reconnection or command-only slot exhaustion. Its 30-minute value
has no supplied workload requirement. More importantly, the proposed
`timeout(min(idle, remaining), read)` polls a ready inner read even at zero
budget: it is not a strict phase cutoff. Its suggested test sends one line
then waits, and therefore does not establish repeated-line trickle behavior;
the existing shutdown/commit test does not establish a new deadline's
commit isolation. These are solvable design obligations, not reasons to
pretend a deadline is impossible. This batch chooses the documented option
and explicitly accepts its larger retention cost pending operator review.

## Required behavior

### Admission

`JoinSet` remains the sole session-count owner. After successful accept,
drain ready completions using `try_join_next`, without awaiting running
sessions, then compare the remaining length with the existing cap (256 by
default). At a genuinely full cap, keep the exact current SMTP
`421 4.7.0 Too many connections, closing transmission channel` and POP3
`-ERR too many connections` replies, CRLF and bounded writes, before spawning
any session. Shutdown keeps its existing abort-and-drain path.

The guarantee concerns completions ready when that reap observes them.
A task completing after the last reap can still race admission. Peer EOF
can precede Tokio task completion; do not assign this interval an assumed
duration or promise it cannot occur. Completions already ready while accept
was parked must not cause the next peer to be refused.

### Health and drain

Each listener owns its existing degraded flag. An accept poll returning
`Pending` clears it; successful accept clears it; error sets it and retains
the one-second backoff, at most one warning per step, and interruptible stop.
Do not clear before polling or solely because the backoff clock expires.
Consecutive immediate errors with no intervening pending/success outcome
must not publish a healthy transition.

`/healthz` keeps 503 degraded while either flag is set, 200 ok otherwise,
and stopping precedence for a request that reaches the handler after stop.
Pending means observed waiting, not proof of free file descriptors or a
guarantee of future accept success. HTTP graceful shutdown closes admission;
new probes after closure get connection refusal. Existing connections may
observe 503 stopping if their request reaches the handler, but some close
without a response. No new connection is guaranteed a stopping response
merely because drain is in progress. The existing 10-second fence stands.

### Independent idle defaults

SMTP retains a 300-second per-read/per-write bound, including its existing
silent mid-DATA expiry. POP3 uses its own 600-second per-read/per-write bound.
`Config::from(&ServeArgs)` supplies the POP3 default; `run_on` passes that
field to the existing POP3 serve argument. Overriding either protocol's
idle timeout does not change the other's. Keep the existing session-cap
wiring; no CLI/env option, dependency, new config type or metric is added.
Update help, rustdoc and README statements that still claim unbounded
plaintext idleness. Adding the public field is an intentional Rust source
compatibility cost; the command-line option set stays unchanged.

### Explicit lifetime residual

No whole-session lifetime, DATA lifetime or minimum transfer rate is added.
One incomplete read/write remains bounded; completing commands or DATA
lines inside each window permits another window indefinitely. Byte trickles
that never complete a read do not reset that operation's deadline.

At the default cap, 256 SMTP connections can occupy every slot indefinitely.
DATA can retain payload buffers approaching 50 MiB per connection, about
12.5 GiB across 256 connections before allocation, parsing and other
overhead. This is a payload exposure estimate, not an RSS ceiling. Once
over-limit, drain-to-terminator also has no total time bound if complete
lines keep arriving. POP3 command trickles likewise can retain its 256
slots indefinitely; do not claim its per-I/O deadline is a session lease.
These risks remain even when all correctness tests pass.

README must state the current posture. Preserve accepted 0027 as history
and append a dated, clearly controlling clarification linked to proposed
0036, covering §§1–4, §9 and Consequences. Explicitly supersede the old
unqualified trickler claim and retryable stale-reap explanation. Index 0036
as proposed during implementation, not as accepted.

## Acceptance Criteria

- **AC1 — First admission after completion (#77):** on each protocol, fill
  a small overridden cap, establish that one session task completed while
  accept was parked, connect exactly once, and assert the greeting. Repeat
  after a whole full burst completes. No retry-through-refusal success path.
- **AC2 — Honest full cap (#77):** while all session tasks remain live, the
  next connection receives exactly the existing refusal and EOF. No store
  mutation occurs. Existing cap tests lose their retry-based recovery loops.
- **AC3 — Idle recovery (#78):** inline tests on both listeners inject one
  error with nothing queued, observe degraded, then observe clear after the
  next backoff and pending accept without connecting any peer. Use a bounded
  state observation, not elapsed-time equality or a synchronization sleep.
  Only after clear, connect and prove protocol service.
- **AC4 — No clock-only green (#78):** deterministically drive successive
  immediate errors and assert at the retry boundary that the flag remains
  set; a pending/success outcome clears it. Preserve stop-during-backoff
  coverage and HTTP status/stopping-precedence tests.
- **AC5 — Drain honesty (#78):** README and the dated 0027 clarification
  explicitly state connection refusal after HTTP admission closes, and
  condition 503 stopping on reaching the handler. Existing real-server
  lifecycle/drain tests remain green; no listener extension is introduced.
- **AC6 — Defaults and wiring (#79):** equality tests pin SMTP=300 s,
  POP3=600 s and CLI-to-Config POP3 initialization without waiting minutes.
  A real server started through `run_on` with only the POP3 field shortened
  reaches `-ERR idle timeout` then EOF; SMTP retains its configured bound.
  Pin the reciprocal override independence and retain SMTP mid-DATA EOF
  with zero stored messages.
- **AC7 — Help and compatibility (#79):** real `serve --help` describes
  SMTP 5 minutes and POP3 10 minutes, and no longer claims plaintext has no
  timeout. README, current 0027 clarification, rustdoc and code agree; no
  new CLI/env knob is exposed. All in-tree `Config` literals compile.
- **AC8 — Deliberate residual (#80):** 0036 exists with status proposed and
  is linked/indexed. README and the controlling 0027 clarification state
  unbounded command/DATA complete-line trickles, 256 occupied SMTP slots,
  50 MiB per DATA payload and approximately 12.5 GiB aggregate before
  overhead. A documentation contract regression rejects the reviewed
  claims and passes on the correction. No new runtime cutoff is claimed.
- **AC9 — Red/green evidence:** preserve a specific reviewed-behavior
  failure and repaired pass per child. For #80 the regression is explicitly
  documentary, because that child permits unchanged runtime behavior;
  historical prose outside a controlling amendment is not a failure by
  itself. A compile error or environment failure is not red proof.
- **AC10 — Invariants and gates:** all six house gates pass, including
  exact 100/100/100 coverage without a TMPDIR override. Preserve 0001–0005,
  off-worker awaited commits, shutdown ordering, 0034's DATA/envelope/AUTH
  contracts, the 40-character reference window, eager interrupt, fallible
  `search_needle`, loopback guard, staged compat kind, page/awaits caps and
  30-second helper deadline with the derived SMTP outer bound. No coverage
  exclusion, dependency allowlist change or unconditional ANSI.
