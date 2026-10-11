# Tasks: Commits off async workers (`off-worker-commits`)

Ordered. Every task names the test that proves it; nothing lands without its proof. The full
gate list (`cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test`, `brokkr compile --bundle .`, `cargo deny check licenses`,
`scripts/coverage-gate.sh` at literal missed == 0) runs at task 10 and at verify.

- [ ] 1. Add the private `offload` helper to `src/store.rs` (`tokio::task::spawn_blocking` with an
  explicit `match` + `resume_unwind`, per plan's Design) and the four public awaited wrappers
  `insert_async` / `delete_async` / `clear_async` / `clear_all_async` (`self: Arc<Self>`,
  offloading the **whole** mutation).
  Proof: `#[should_panic]` panic-symmetry pin — a panicking pre-mirror gate through `clear_async`
  re-raises in the awaiting caller; this also deterministically covers the helper's `Err` arm for
  `scripts/coverage-gate.sh`.

- [ ] 2. Add the ordering pin (wave-2 propagation, deterministic): current-thread
  `#[tokio::test]` in `src/store.rs`'s test module — pre-mirror gate holds the commit,
  `Persist::fail_writes()` set, the awaited mutation must not resolve while held, must resolve
  after release, memory stays truthful, and the row is absent on reopen.
  Proof: the pin itself is red against fire-and-forget and reply-before-commit; green after
  task 1.

- [ ] 3. Add the structural stall pin (criterion 1): current-thread `#[tokio::test]` — the gate
  holds the mutation on a blocking-pool thread while the test task's deadline-bounded 1 ms sleeps
  keep completing; a std-thread watchdog releases the gate after ~2 s so an inline implementation
  is a fast red (wedged ticks), a fire-and-forget is a red (resolved-while-held), and
  `block_in_place` is a red panic.
  Proof: the pin itself, plus its red directions documented in the test's doc comment.

- [ ] 4. Add the real-protocol propagation leg (in `src/store.rs`'s test module — the only place
  `fail_writes()` and the private `persist` field are reachable): a real `TcpListener` +
  `crate::smtp::serve` + real `TcpStream` speaking SMTP against a store on a temp data file with
  `fail_writes()` set. Assert the `250` still arrives (log-and-accept preserved), the mail is
  queryable over the wire, and a reopen loses the row (wave-2 degraded restart copy).
  Proof: the leg itself; criterion 2 of the spec.

- [ ] 5. Switch the SMTP, API and MCP call sites to the awaited wrappers:
  `src/smtp.rs:455`, `src/api.rs:349`, `:367`, `:459`, `src/mcp.rs:126`, `:134`, `:136`, `:185`.
  `src/smtp.rs:456`'s `250` write must stay immediately after the awaited call — do not move it.
  Proof: `cargo test --test smtp_edges --test api --test mcp --test burst --test rate` (wire
  behaviour unchanged, every wrapper line executed).

- [ ] 6. Switch the POP3 and compat call sites: `src/pop3.rs:357`; `src/compat.rs:254`, `:269`
  (one-liners) and `:268` (restructure the `filter(...).count()` into a counted loop of awaited
  `delete_async` calls — no `delete_many` method).
  Proof: `cargo test --test pop3 --test compat` (RFC 1939 update-state deletes and the
  MailHog/Mailpit shims answer identically).

- [ ] 7. Correct the false doc claim: `src/persist.rs:12-13` ("cannot stall the runtime") states
  the commit runs on a blocking thread when called from an async handler; mirror the phrase in
  `insert`'s doc comment. Zero code changes in `src/persist.rs`.
  Proof: the structural stall pin (task 3) makes the corrected sentence *true*; `cargo doc`
  builds clean; `git diff src/persist.rs` shows comments only.

- [ ] 8. Add the hard-kill durability leg to `tests/binary.rs`: spawn the real binary with
  `--data-file` on a temp dir, burst real SMTP mail recording every id observed on a `250`,
  `libc::kill(child.id(), SIGKILL)` mid-burst, respawn on the same file, assert every recorded id
  is in the restart copy (HTTP API). No wall-clock margins.
  Proof: the leg itself — green on old and new code; red against commit-after-`250`,
  write-behind, and `synchronous = NORMAL` (demonstrated once by temporarily inverting the
  ordering in a scratch build, then reverted).

- [ ] 9. Add the real-server write-heavy leg (correctness under concurrency, never wall-clock
  asserts): server with `--data-file`, more concurrent pipelined writer sessions than runtime
  workers, concurrent liveness `NOOP`s, then full count-and-restart verification of every
  `250`-answered mail.
  Proof: the leg itself on old and new code (both green — it pins correctness under the move).

- [ ] 10. Add `benches/writer_stall.rs` (+ the `[[bench]]` entry in `Cargo.toml`,
  `harness = false`) per the measurement contract: one process, one real data file, current-thread
  runtime, 1 ms ticker overshoot (max/p99), sync `insert` phase vs awaited `insert_async` phase,
  size-amplified rows. Record the verbatim numbers in a docs note or the PR body; if the win is
  not clear, say so and let the operator rule — no tuning, no batching, no pragma changes.
  Proof: the bench runs standalone (`cargo bench --bench writer_stall`), the numbers are recorded
  verbatim, and `benches/ingest.rs` is untouched.

- [ ] 11. Run the full gate list and leave it green:
  `cargo fmt --all -- --check`; `cargo clippy --all-targets -- -D warnings`; `cargo test`;
  `brokkr compile --bundle .`; `cargo deny check licenses`; `scripts/coverage-gate.sh`
  (literal 100/100/100, missed == 0 — the wrappers and helper fully executed, no `coverage(off)`).
  Proof: the six commands' own exit codes; `target/coverage/coverage-summary.json` equality.
