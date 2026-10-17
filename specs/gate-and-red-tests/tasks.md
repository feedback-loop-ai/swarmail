# Tasks — gate-and-red-tests (issues #101, #102)

Ordered. The gate repairs land first (the brief marks #101 fix-first:
every later red proof runs through the gate and must be trustworthy).
Each task names the test that proves it; every red proof is a recorded
mutation run (red against the named defect, green after the repair),
cited #101/#102 in the test's comment. Gate repairs before test repairs;
deletions beside their guards.

- [ ] **T1 — Two-arm build-dir resolution in the gate.** Extract
      `resolve_build_dir()` (explicit `CARGO_LLVM_COV_TARGET_DIR` honoured
      verbatim; default `mktemp -d target/coverage/build.XXXXXX` after
      `mkdir -p target/coverage`; a recorded flag for which arm created
      the dir) and replace `:426-438`'s `$TMPDIR` mktemp + unconditional
      export; the EXIT trap cleans only a gate-created default.
      *Test:* selftest arms — default resolves under `target/coverage/`,
      an explicit value is returned unchanged (red against today's
      overwrite at `scripts/coverage-gate.sh:433`); plus one full gate
      run on this host landing under `target/` with no new `$TMPDIR`
      entry (AC-1, AC-2).
- [ ] **T2 — Free-space preflight.** `check_free_space <dir> <min_kib>`
      (one `df -Pk`, named constant ≈ 6 GiB with the ≈ 4 GiB build +
      slack arithmetic in a comment), called against the resolved dir
      before the build, refusing with the path, filesystem, and
      `CARGO_LLVM_COV_TARGET_DIR` named.
      *Test:* selftest arm passing an absurd floor expects refusal and
      the message names the environment (red against the current gate —
      no preflight exists) (AC-4).
- [ ] **T3 — Selftest root to the disk-backed parent.** Move the selftest
      root to `mktemp -d target/coverage/selftest.XXXXXX`, trap-removed
      (replacing `:272`).
      *Test:* the before/after `$TMPDIR` listing around a selftest run
      shows no new entry, and the root is gone after the trap (AC-3).
- [ ] **T4 — Scratch drop guards at the three fixture sites.**
      `tempfile::TempDir` in `src/persist.rs` `db_path` (:357-366),
      `src/store/persist_tests.rs` `db_path` (:5-14), `src/tls.rs`
      `scratch` (:141-150) — db *inside* the temp dir so `-wal`/`-shm`
      die with it by construction; delete the pid+nanos naming and the
      four explicit `remove_file` triples (:351-353, :504-506, :727-729,
      :748-750). (If the operator refuses the dep: one shared ~12-line
      `#[cfg(test)]` guard beside the db lifecycle, `pub(crate)` — never
      three copies.) *Test:* the inline persistence and TLS suites pass
      with the guards executing every new line (the exact gate demands
      it), and the before/after `$TMPDIR` listing around `cargo test
      --workspace --locked` shows no new `swarmail-*` entries (AC-3).
- [ ] **T5 — Comment-stripping pass + hardened declaration walk.** One
      shared `strip_comments` (block comments, line semantics preserved)
      feeding `check_harness_declarations`: any-visibility `mod <stem>`
      (incl. `r#`), any `#[path]` whose *target* is harness-named
      anywhere in the parent, any `include!` of a harness-named file,
      and refusal of any cfg on such a declaration that is not exactly
      `#[cfg(test)]`. *Test:* fixtures A–D, F, G as printf-sized
      `expect_refuse` selftest arms, **each shown red against the
      current gate** before the repair (AC-5).
- [ ] **T6 — Token-wise `coverage` matching.** `check_no_coverage_off`
      strips comments, collapses whitespace, then matches
      `coverage`/`off` with optional intervening space. *Test:* fixture
      E (`#[coverage(/**/off)]` and `#[coverage( off )]`) refuses in a
      selftest arm, red against today's substring grep; the clean
      source arm still passes (AC-5).
- [ ] **T7 — The wall: `check_report_covers_src`.** After the report
      exists (beside `check_no_harness_leak`), every non-harness `.rs`
      under `src/` must appear as an `SF:` record — ~15 lines of awk
      over a `find`, harness exclusion reusing 0020's reserved names.
      *Test:* selftest arms — a report missing one non-harness `src`
      file refuses naming it (red against the current gate), a complete
      report passes (AC-6).
- [ ] **T8 — Webhook budget tests to the exhaustion signal.** The five
      budget tests (:455, :536, :571, :680, :747) adopt the
      :588-610 pattern (baseline → send → wait for
      `swarmail_webhook_exhausted_total` over its pre-mail value → read);
      delete `BUDGET_WINDOW` (:45-52) and the false comment; correct any
      record sentence repeating the claim; keep the stored-and-queryable
      assertion in every budget test. The dispatcher's `0..4` loop is
      never edited. *Test:* the mutation run `0..4 → 0..5` turns at
      least the plain-HTTP budget test red — the fifth attempt is *seen*
      (AC-7); the suite stays green at `0..4`.
- [ ] **T9 — Fallible EOF helper.** Rewrite `http_send_text_allow_eof`
      (`tests/common/mod.rs:526-543`) to its own `read_to_end`: zero
      bytes or a reset → `(0, "")`; a real status never rewritten (a
      503 with an empty body stays 503). *Test:* real-socket tests in
      `tests/binary.rs` — a listener that accepts and closes yields
      `(0, "")`, one answering 503 with an empty body yields 503; both
      fail against the current helper (it panics before the mapping and
      corrupts 503 to 0) (AC-8).
- [ ] **T10 — Chaos two-scope write.** Hold `chaos.config.write()` across
      `two_concurrent_set_event_arms_both_survive`'s spawn, release and
      join (`src/chaos.rs:390-422`); delete the false "a barrier join"
      sentence. *Test:* the mutation run reintroducing the read-clone +
      separate write fails the assert deterministically — not 38 in
      20,000 (AC-9).
- [ ] **T11 — Bounded-work scan counter.** Two `#[cfg(test)]` atomics in
      `src/extract.rs` beside `parse_ref` (calls, window bytes); the
      ampersand test (:189-198) asserts `bytes ≤ REF_MAX × calls +
      slack` (slack named in the comment). The 40-char window
      (:70-83) is untouched. *Test:* the mutation run removing the
      window blows the bound by ~4 orders of magnitude on the same
      input — red on any box, no wall clock (AC-10).
- [ ] **T12 — Waits-registered counter.** Add the cumulative per-inbox
      family `swarmail_awaits_registered_total{inbox}` to `src/api.rs`:
      one increment at `park_await` (:242), rendered through the
      existing per-inbox row machinery; doc comment states the
      per-process, delta-only contract. *Test:* the family's rows appear
      on `/metrics` for a parked wait and grow exactly once per
      registration (the row-poll arms already drive `park_await` —
      the exact gate demands the new lines measured) (AC-11).
- [ ] **T13 — Refusal triples observe the delta.** `tests/api.rs`
      :1207-1216, `tests/mcp.rs` :1534-1541 and :2002-2011 assert
      `swarmail_awaits_registered_total{inbox}` unchanged across the
      refused call — a delta, never an absolute. *Test:* the mutation
      run subscribing before validating (the 0025 §5 violation) goes red
      in all three triples, deterministically (AC-11).
- [ ] **T14 — MCP timeout arm mirrors the REST twin.** `tests/mcp.rs`
      :1067-1101: spawn the wait, poll its parked row, *then* send;
      `timeout_ms` 1500 → ≥ 5000 (both arms' windows widened). *Test:*
      the mutation run reverting to the pre-completed first wait (mail
      sent before the wait parks) turns the arm red — the total is no
      longer counted after the wait (AC-12); the 408 arm's own window
      widening lands with it (AC-15).
- [ ] **T15 — Bind env read back.** `serve_takes_its_binds_from_the_environment`
      (`tests/binary.rs:212-236`): concrete free ports (bind-and-release)
      in the three `SWARMAIL_*_LISTEN` vars; assert the parsed
      startup-line ports equal the env's values. *Test:* the mutation
      run stubbing the `env =` wiring to loopback defaults goes red on
      the read-back (AC-13).
- [ ] **T16 — Helper deadlines name the step.** One named 10 s const;
      `tokio::time::timeout` around `smtp_send`'s reads, `SmtpConn::reply`
      (:139, :268), `read_to_end` (:336), the raw-HTTP read (:623-631),
      and the stdio child waits (`tests/binary.rs:300`, :325) — panics
      naming the step. *Test:* a real socket that accepts and never
      replies fails `smtp_send` within the bound with the step named —
      against the current helper it hangs (AC-14); the stdio child-wait
      bound likewise against a never-exiting child.
- [ ] **T17 — POP3 read-splitting by sizes and byte order.**
      `tests/pop3.rs:708-714`: one fragment > `max(4096, 8192)` (the
      existing 20_000 serves) with the terminator as the last bytes of
      the stream; the 100 ms sleep deleted. No seam, no pacing helper —
      the spec's recorded deviation. *Test:* the arm still asserts
      `-ERR line too long` and the stream stays in sync; the red proof
      is the reverted sleep-paced shape — a coalesced run skips the
      cap-and-drain `None`-branch, which the exact branch gate detects
      as a miss, while the size-driven shape takes it on every run
      (AC-15).
- [ ] **T18 — Full gate + red-proof sweep.** Run the six standing gates
      plus `scripts/coverage-gate.sh selftest` (all arms) and the
      before/after `$TMPDIR` listing; collect the recorded red→green
      runs (T5, T6, T8–T17 mutations) into the run's evidence, each
      citing #101/#102. *Test:* `bash scripts/coverage-gate.sh` exact
      100/100/100 with the default build dir under `target/`, floor
      never fallen; `cargo deny check licenses` clean; every new
      production line (counter, atomics, gate functions) executed on
      the instrumented run (AC-16).
