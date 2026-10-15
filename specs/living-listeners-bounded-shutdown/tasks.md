# Tasks — living listeners and a bounded shutdown

Ordered; each task names the test that proves it and may not be marked done without it.
Scope guard: no new files under `src/`, no new dependencies, no new routes or metrics, no
knob (0022 §5), no edits to `insert`/`insert_async` or the commit→`250` leg, no
`coverage(off)`, no new harness file (no cold path exists in this design). Gates run at the
end and are part of the definition of done: `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets --locked -- -D warnings`,
`cargo test --workspace --locked`, `brokkr compile --bundle bundles/self`,
`scripts/coverage-gate.sh` (exact 100/100/100), `cargo deny check licenses`. Test
disciplines per 0020: no sleep as synchronization, real deadlines (upper bounds only),
per-test fresh stores/inboxes, chaos at the poles only, seams `#[cfg(test)]`-only with
installing tests. The ruling lives in [`spec.md`](spec.md); the mechanism and the coverage
driver map in [`plan.md`](plan.md).

- [ ] 1. Add the counted one-shot accept-error seam to `src/smtp.rs` (`#[cfg(test)]`, the
       `REAPED` precedent) **and its installing inline test**: inject one accept error, then
       deliver a real mail over the same listener and assert it is queryable, with the
       degraded flag observed set-then-cleared. Proves C36.1 — red today (the first error
       returns from `serve`).
- [ ] 2. Change the SMTP accept arm to the retry-forever shape: set the flag, warn **once
       per backoff step**, `select!` a fixed 1 s `ACCEPT_BACKOFF` against the loop's
       existing shutdown arm, `continue`. Task 1's test goes green; extend it to prove a
       stop arriving inside a backoff sleep resolves in ≪1 s (the raced backoff).
- [ ] 3. The POP3 twin: the same seam, installing test, and error arm in `src/pop3.rs`.
       Proves C36.1 on POP3 — red today.
- [ ] 4. Wire the flag pair: `run_on` creates `smtp_degraded`/`pop3_degraded`
       (`Arc<AtomicBool>`), passes them to the two serve loops and `AppState`; no other
       signature changes. Proves mechanical wiring — the existing suite stays green.
- [ ] 5. `/healthz` gains the four-state contract (`ok`/`degraded`/`stopping`, `stopping`
       wins; 503 for the last two, bodies naming the degraded listeners). Proves C36.2/C37.3's
       observable — direct-handler tests for all four states **plus the wire red-proof**:
       persistent seam on the SMTP loop and the real `api::serve` on one shared flag pair →
       a real HTTP 503 naming `smtp` (red today: hardcoded `"status": "ok"`).
- [ ] 6. The shared bounded-I/O wrapper (single `Elapsed`→`TimedOut` arm) and the SMTP idle
       wraps: every peer read and reply write in the session path;
       `SmtpConfig::idle_timeout` (default 5 min, RFC 5321 §4.5.3.2). Command-path expiry
       answers `421 4.4.2 … idle timeout, closing transmission channel` then ends; DATA
       expiry takes the exact EOF statement (nothing stored); timed-out writes end silently
       through the existing `?`. The deadline is **never** armed across the insert. Proves
       C36.3a — in-process legs with the idle bound overridden small: silent client → `421`
       → EOF, nothing stored; `DATA`-then-stall → EOF, store count exact (red today: the
       server waits forever).
- [ ] 7. The POP3 idle wrap (`-ERR idle timeout`), default 10 min (RFC 1939 §3), via two
       plain parameters on `pop3::serve`. Proves C36.3a — in-process leg in
       `tests/pop3.rs`: silent client answered `-ERR` then EOF (red today).
- [ ] 8. Session caps: `SmtpConfig::max_sessions` (default 256) and POP3's `max_sessions`;
       the check reads the live `JoinSet::len()` **after** the top-of-loop reap; the refusal
       is written under the shared deadline **before any session state exists** — SMTP
       `421 4.7.0 …`, POP3 `-ERR too many connections`. Proves C36.3b — in-process legs
       with the cap at 1: the second connection is refused and the **first session still
       delivers a queryable mail**; no spurious refusal after a session completes (red
       today: no cap).
- [ ] 9. The RCPT cap: refuse recipient 101 with exactly `452 4.5.3` **before** the append;
       the session continues. Proves C36.4 — `tests/smtp_edges.rs` wire leg: 101 RCPTs,
       `452 4.5.3` on the 101st, `DATA` completes a mail carrying exactly 100 recipients
       (red today: the 101st is appended).
- [ ] 10. The drain deadline: `api::serve` clones `state.shutdown` before `router(state)`
       consumes it, races the graceful serve against `shutdown_signal(rx)` +
       `sleep(DRAIN_DEADLINE)` (5 s = 0018's margin, documented), returning `Ok(())` at the
       deadline. Proves C37.3 — `tests/lifecycle.rs` stalled-body leg: a real connection
       that never finishes its body holds the drain until the deadline, `stop()` returns
       within a real upper-bound deadline, and `/healthz` answers `stopping` while the drain
       holds (red today: `stop()` never returns).
- [ ] 11. The watcher: under `cfg(unix)` own both signal streams in the existing spawned
       task, `select!` SIGINT/SIGTERM → the same `interrupted` watch; a further signal →
       `std::process::exit(70)` (documented vocabulary: 0 graceful, 70 forced). Proves
       C37.1/C37.4 — `tests/binary.rs`: SIGTERM → clean exit 0 → restart on the same data
       file → the mail is queryable; and the second-signal leg: a stalled body holds the
       drain, the first SIGTERM is sequenced on the binary's own `shutting down` stdout
       line (`src/main.rs:50`) — never a sleep — and the second SIGTERM force-exits 70 well
       inside the liveness deadline (both red today).
- [ ] 12. `STOPSIGNAL SIGTERM` in the Dockerfile; verify the chart needs nothing beyond the
       image default (no grace-period pin). Proves C37.2 — the SIGTERM legs of task 11 (the
       signal `docker stop` sends is the handled one); the chart check is a grep-verified
       no-op recorded in the run notes.
- [ ] 13. Documentation: dated `Amended by 0027` markers on 0018 §2 and 0022's posture line
       (0019 protocol — pointed at, never rewritten); the 0027 row in
       `docs/decisions/README.md`; README's wire-bound rows for idle values, session cap,
       RCPT limit, exit codes, and the `/healthz` states. Proves AC 11 — the docs agree
       with the wire (the compat/contract suites stay green: no route, schema or method is
       touched).
- [ ] 14. Run every gate; attribute any red to its named test and repair before reporting.
       Proves the six gates above, plus the verify-seat proof filters: `cargo test --test
       binary -- --nocapture`, `--test lifecycle`, `--test smtp_edges`, `--test pop3`.
