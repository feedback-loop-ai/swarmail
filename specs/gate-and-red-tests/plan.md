# Plan — gate-and-red-tests: the gate survives its host, refuses by structure; the tests observe by signal

Implementation plan for
[spec.md](spec.md) (issues #101, #102). Base: `5671e0c` (production identical
to `f39813d`). The ordering is load-bearing: **the gate repairs land first**
(the brief marks #101 fix-first — the other regressions' red proofs must be
trustworthy, and every red-proof run passes through the gate), then the #102
test repairs in decreasing blast radius.

## Technical context

- The exact coverage gate (`scripts/coverage-gate.sh`) is 482 lines of bash
  and awk: refusal-style checks (`refuse`, fail-closed), a selftest mode
  (`expect_pass`/`expect_refuse` arms over fresh fixture trees, :270+), a
  `check_no_harness_leak` over the report's `SF:` records, and the
  instrumented run via `cargo +nightly llvm-cov` with a per-run evidence dir
  under `target/coverage/run.XXXXXX`.
- The test suites speak the real protocol against real servers spawned
  in-process (`tests/common/mod.rs`, `run_on`); helpers live in
  `tests/common/mod.rs`; the refusal triples and timeout arms are plain
  async tests. 0004 forbids mocks; 0020 rule 5 forbids wall-clock validity
  assertions and sleep-as-synchronization; chaos probabilities sit at the
  poles (0 or 100).
- The gate's failure posture is recorded in its own comment (:134-136): a
  false positive is loud and names the file; the failure direction is
  refuse, never silently mis-count. Every repair below stays in that
  posture.

## Constitution check

| Guarantee / ruling | Proximity | Verdict |
|---|---|---|
| 0001 (accept == stored) | The budget tests' stored-and-queryable assertion is **kept** on every budget test — the witness that exhaustion lives on the delivery side. | preserved, witness kept |
| 0002 (subscribe before check) | The refusal triples enforce the 0025 §5 boundary (validate before subscribing); the new counter makes the boundary observable from the refusal side. | strengthened |
| 0003 (oldest-only pruning) | No path from budget exhaustion or the counters into pruning. | untouched |
| 0005 (extraction at ingest) | The 40-char window (:70-83) survives untouched; the cfg(test) scan counter is additive measurement beside `parse_ref`. | untouched, pinned harder |
| 0020 (harness rule 6, disciplines) | The walk hardening + `SF:` cross-check enforce rule 6 structurally; the deadlines are rule 5's "real deadline"; no feature flag is introduced. | enforced |
| 0025 §5, 0018 §2, 0030 §1, 0004, 0033 | Applied as written; the EOF helper stops misstating 0018 §2's own outcome table. | applied |
| `bundles/` protected tree | No edit; the gate self-resolving to a disk-backed dir makes the verify-seat change unnecessary. | untouched |
| Coverage floor | Exact 100/100/100; the batch *adds* measured lines (the counter family, the atomics, gate functions) each with a test that executes them on the instrumented run. | floor holds, never falls |

No decision document is minted (the spec's ruling); the next free number
stays 0034.

## Project structure — every file the batch touches

**Gate (#101):**

- `scripts/coverage-gate.sh` — the only gate file:
  - two-arm `resolve_build_dir` (explicit `CARGO_LLVM_COV_TARGET_DIR`
    verbatim / default `mktemp -d target/coverage/build.XXXXXX` after
    `mkdir -p target/coverage`; a recorded flag for which arm created the
    dir, the EXIT trap cleaning only a gate-created default) replacing
    `:426-438`'s `$TMPDIR` mktemp + unconditional export;
  - `check_free_space <dir> <min_kib>` preflight (one `df -Pk`, named
    constant `MIN_BUILD_FREE_KIB ≈ 6 GiB` with the ≈ 4 GiB build + slack
    arithmetic in a comment, refusal naming path + filesystem + the escape
    variable), called against the resolved dir before the build;
  - selftest root moved to `mktemp -d target/coverage/selftest.XXXXXX`,
    trap-removed (:272);
  - the shared comment-stripping pass + hardened
    `check_harness_declarations` walk (any-visibility `mod <stem>` incl.
    `r#`; `#[path]` whose *target* is harness-named anywhere in the parent;
    `include!` of a harness-named file; non-exact-`#[cfg(test)]` cfg on
    such a declaration) replacing the `^[[:space:]]*mod <stem> ;`-only walk
    (:162-185);
  - `check_no_coverage_off` (:71-88) made token-wise: comments stripped,
    whitespace collapsed, then `coverage` + `off` matched with optional
    intervening space — fixture E's `#[coverage(/**/off)]` dies at
    stripping, `#[coverage( off )]` at the collapse;
  - new `check_report_covers_src <report> <src>` beside
    `check_no_harness_leak` (~15 lines of awk over a `find`): every
    non-harness `.rs` under `src/` must appear as an `SF:` record;
  - selftest arms for fixtures A–G, both resolution arms, the absurd-floor
    refusal, and the cross-check (complete report passes, one-missing
    report refuses).

**Fixtures (#101):**

- `src/persist.rs` — `db_path` (:357-366) returns a path *inside* a
  `tempfile::TempDir`; the four explicit `remove_file` triples
  (:351-353, :504-506, :727-729, :748-750) deleted; pid+nanos naming
  deleted.
- `src/store/persist_tests.rs` — same treatment for its `db_path` (:5-14).
- `src/tls.rs` — same treatment for `scratch` (:141-150).
- `Cargo.toml` — one dev-dependency: `tempfile` (MIT OR Apache-2.0; lock
  delta `fastrand` + `rustix`, both allowlisted). *Named fallback if the
  operator refuses:* one ~12-line `#[cfg(test)]` guard owned beside the db
  lifecycle in `src/persist.rs` (`pub(crate)`), shared by all three sites —
  never three copies, never `src/testsupport.rs`.

**Tests + counters (#102):**

- `tests/webhook.rs` — the five budget tests (:455, :536, :571, :680,
  :747) adopt the exhaustion-signal pattern; `BUDGET_WINDOW` (:45-52) and
  the false comment deleted; stored-and-queryable assertions kept.
- `tests/common/mod.rs` — `http_send_text_allow_eof` (:526-543) rewritten
  to its own `read_to_end` (zero bytes / reset → `(0, "")`; a real status
  never rewritten); deadlines (`tokio::time::timeout`, one named 10 s
  const) around `smtp_send`'s reads (:34-100), `SmtpConn::reply` (:139,
  :268), `read_to_end` (:336), and the raw-HTTP read (:623-631).
- `src/api.rs` — the one new production surface: cumulative per-inbox
  `swarmail_awaits_registered_total{inbox}`, one increment at `park_await`
  (:242), rendered through the existing per-inbox row machinery beside
  `awaits_active` (:740-760); doc comment carries the per-process,
  delta-only contract in so many words.
- `tests/api.rs` — the `nocap` refusal triple (:1207-1216) observes the
  delta; the 408 arm's `timeout_ms` 1500 → ≥ 5000 (:703-731).
- `tests/mcp.rs` — both refusal triples (:1534-1541, :2002-2011) observe
  the delta; the timeout arm (:1067-1101) mirrors the REST twin (spawn the
  wait, poll its parked row, then send; `timeout_ms` ≥ 5000).
- `tests/binary.rs` — `serve_takes_its_binds_from_the_environment`
  (:212-236) uses concrete free ports (bind-and-release) and asserts the
  parsed startup-line ports equal the env's; the stdio child waits
  (:300, :325) get the named-deadline bound; a real-socket proof that an
  accept-and-close yields `(0, "")` and a 503-with-empty-body stays 503.
- `src/chaos.rs` — `two_concurrent_set_event_arms_both_survive`
  (:390-422) holds `chaos.config.write()` across spawn, release, join;
  the false "a barrier join" sentence deleted.
- `src/extract.rs` — two `#[cfg(test)]` atomics (calls, window bytes)
  beside `parse_ref`; the ampersand test (:189-198) asserts
  `bytes ≤ REF_MAX × calls + slack` (slack named in the comment).
- `tests/pop3.rs` — the read-splitting arm (:708-714): one fragment >
  `max(4096, 8192)` (20_000 serves), terminator last, no sleep.

Nothing else. No new files under `src/` or `tests/`; no `bundles/` edit;
no new env var; no new route; no new decision.

## Approach — the load-bearing details

1. **Two-arm resolution must be a pure, selftest-callable function.** The
   selftest calls check functions directly (`expect_refuse '…'
   check_no_coverage_off "$fx/src"`), so the resolution lives in
   `resolve_build_dir()` — reads the env, prints the resolved path, sets a
   `created_by_gate` flag — rather than being inlined into the run path.
   Both arms are then provable without running an instrumented build: the
   selftest asserts the default lands under `target/coverage/` and an
   explicit value is returned verbatim (today's overwrite is the red).
2. **Comment-stripping before the walk, once.** One `strip_comments` pass
   (block comments removed with line semantics preserved) feeds the
   declaration walk *and* the token-wise `coverage` match; a
   `#[cfg(test)]` inside a comment is not a guard (fixtures C and G), and
   `#[coverage(/**/off)]` is not a dodge anymore (fixture E). Line
   comments were already handled; block comments are the gap.
3. **The cross-check's harness exclusion reuses 0020's reserved names.**
   `check_report_covers_src` skips exactly the harness-named files
   (`tests.rs`, `*_tests.rs`, `*-tests.rs`) that rule 6 reserves and
   cargo-llvm-cov's default ignore drops — those are refused elsewhere
   unless declared. A legit `src/` file absent from the report refuses,
   loudly; the remedy is editing the file or a decision, never an
   exclusion list.
4. **The counter is one increment at its one owner.** `park_await` is the
   single registration site for both transports (its own doc comment says
   so), so the family renders from the existing per-inbox registry
   machinery — no new render plumbing, no per-transport spelling, and the
   increment is race-free where a global would be flaky (in-process
   parallel servers).
5. **The budget tests read the count *after* the exhaustion poll.** The
   pattern is baseline → send → `wait_for_metric_from(exhausted_total,
   base)` → read the channel's hit count. A fifth attempt lands at
   ≈ +8.5 s and is now inside the observation (no window exists to
   close); the `0..4 → 0..5` mutation goes red. The stored-and-queryable
   assertion rides along unchanged.
6. **The scan counter's bound is an inequality with named slack, not an
   exact count.** Parallel inline tests contribute ≤ `REF_MAX` bytes per
   their own scans — a margin three-plus orders of magnitude below the
   defect's signal (unwindowed ≈ len² byte-scans vs ≤ 40 × calls).
   Branch-stable, no wall clock, 0020 rule 5 clean.
7. **The POP3 arm's red proof is the exact branch gate itself.** The old
   sleep-paced shape lets a coalesced segment carry the terminator inside
   the first fill — the `None`-branch cap and `drain_line` are then not
   taken, which the exact 100%-branch gate detects as a miss on a
   coalesced run (a flake today). The size-driven shape takes the branch
   on every run, deterministically. The mutation proof is the reverted
   shape missing the branch.
8. **Red proofs are recorded runs, in this order:** each fixture A–G
   against the current gate; the `0..4 → 0..5` flip; each reintroduced
   defect of AC-9..AC-13; the wedged-peer deadline; the reverted POP3
   shape. Each lands red→green in the run's evidence, cited #101/#102.

## Risks

- **A determined adversary launders past the tightened walk (fixture H is
  always conceivable).** *Mitigation:* the `SF:` cross-check carries the
  guarantee — evasion moves the failure to a check with no regex to evade;
  the walk keeps only the fast-fail diagnostics with better messages. The
  cross-check arm is itself a selftest arm, so the wall is red-proven too.
- **The cross-check false-refuses a `src/` file that legitimately emits no
  records.** *Mitigation:* accepted as impossible-today and
  loud-if-ever — every current `src/*.rs` compiles functions; the refusal
  names the file (the gate's stated posture :134-136), and the remedy is
  editing the file or a decision, never an exclusion list (exclusions are
  the hole being closed).
- **`tempfile` widens the dev graph by two small crates and the operator
  may refuse it.** *Mitigation:* dev-only (zero runtime surface, nothing
  ships), licenses allowlisted (MIT OR Apache-2.0; `fastrand`,
  `rustix`), the one-sentence defense is in the spec; the named fallback
  (one shared `#[cfg(test)]` guard beside the db lifecycle) is scoped and
  bounded — and `cargo deny check licenses` gates the manifest change.
- **The registered counter's per-inbox map grows with distinct inboxes
  over a server's life.** *Mitigation:* same cardinality class the report
  already renders per-inbox (`awaits_active`, `deliveries_in_flight`,
  `requests_in_flight`); swarmail is a dev-facing mock; a cap would be a
  new decision, not this batch's business. The doc comment records the
  per-process, delta-only contract so no writer asserts absolutes.
- **The scan-counter bound is an inequality, not an exact count.**
  *Mitigation:* the slack is named in the test's comment and is 3+ orders
  of magnitude below the defect's signal; branch-stable under the poles
  discipline, so it cannot flake — the counter cannot be "deleted as
  flaky", which is the fate that would silently un-pin the security
  boundary.
- **Widened windows and helper deadlines make some failures slower.**
  *Mitigation:* accepted by the framing's own criterion 13 (~5 s floor):
  a deadline bounds a wedged peer's *hang*, not a green run — green paths
  stay event-driven; the honest alternative is a full instrumented gate
  rerun per flake (minutes plus the build).
- **SIGKILL litter is not solved.** *Mitigation:* honest loss, accepted by
  both seats — a drop guard dies with the process; the litter moves into a
  gitignored tree `cargo clean` reclaims, and the preflight refuses
  *honestly next time*; the gate never scavenges `swarmail-*` roots it
  did not create (0033's spirit).
- **Cert expiry stays untested.** *Mitigation:* per-run generation under a
  drop-guarded root makes expiry unobservable by construction; the only
  test that could exercise it is a wall-clock assertion or a time bomb,
  both forbidden (0020 rule 5) — the gap is recorded here rather than
  hidden behind a test that would flake and get deleted.
- **The awk grows before the language improves.** *Mitigation:* the
  red-proof selftest arms are what make the awk defensible; the spec
  records the tripwire — if an arm goes green against a known defect, the
  gate is rewritten in Rust via a decision document.
- **Gate-script self-measurement.** The gate is shell, not Rust — it is
  outside `src/` and thus outside the coverage report by construction, not
  by exclusion; its own correctness lives in the selftest arms, which is
  why every new check ships with at least one red arm and one green arm.

## Verification

The six standing gates plus the batch's own proofs (the framing's verify
gate, verbatim):

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh        # exact 100/100/100; build under target/ by default
bash scripts/coverage-gate.sh selftest   # all arms, A–G + resolution + cross-check, red-proofed
cargo deny check licenses
bash scripts/publish-guard-test.sh   # untouched, part of the seat run
```

Plus the red proofs of record (each red→green, cited #101/#102): fixtures
A–G against the current gate; the `0..4`→`0..5` flip; each reintroduced
defect of AC-9..AC-13; the wedged-peer deadline; the reverted POP3 shape;
and the before/after `$TMPDIR` listing of AC-3. During development the
gate runs with a disk-backed `TMPDIR`
(`TMPDIR=/home/valentin/scratch/cov-tmp`); post-repair acceptance is the
gate passing with the default resolution under `target/` — no TMPDIR
instruction anywhere.
