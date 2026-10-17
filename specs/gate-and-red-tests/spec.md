# Spec — the gate survives its host and refuses by structure; the tests observe by signal and can go red (issues #101, #102)

- **Run:** run 12 (chief architect, synthesizing the simplicity and robustness
  positions; run-local evidence:
  `.brokkr/state/design/positions/simplicity.md`,
  `.brokkr/state/design/positions/robustness.md`, both uncommitted) ·
  **Feature slug:** `gate-and-red-tests`
- **Issues:** [#101](https://github.com/feedback-loop-ai/swarmail/issues/101)
  (the exact coverage gate builds under a size-limited `$TMPDIR`, litters it,
  and its declaration checks still admit bypass fixtures A–G) and
  [#102](https://github.com/feedback-loop-ai/swarmail/issues/102) (the
  remaining vacuous and timing-bound tests pass on the defect they are named
  for, or hang instead of failing).
- **Decisions:** **none minted** — the ruling at the end of this spec; the
  next free number stays 0034. Operative decisions are applied as written,
  not amended: 0020 (rule 6 and the test-writing disciplines), 0025 §5,
  0018 §2, 0030 §1, 0004, 0033.
- **Base:** `5671e0c` (decisions-only atop `f39813d`; production identical),
  the reviewed trees of the findings of record. Every line reference below
  was re-verified against this tree this session.
- **Shape:** a deletion-and-tightening batch. Three new production surfaces
  total — one cumulative per-inbox metrics family, two `#[cfg(test)]` scan
  atomics, and ~100 lines inside the gate script the gate already is. One
  dev-dependency (`tempfile`), with a named fallback if the operator refuses
  it. Zero new files under `src/` or `tests/`; zero new env-var vocabulary
  (`CARGO_LLVM_COV_TARGET_DIR` already exists — the defect is that the gate
  overwrites it); zero edits to `bundles/`; zero new decision documents.

## The problem

**#101 — the gate can go red because of the host, and lie because of its own
checks.** `scripts/coverage-gate.sh:426-438` builds the ~4 GB instrumented
tree under `mktemp -d "${TMPDIR:-/tmp}/swarmail-coverage.XXXXXX"` and
**unconditionally exports** `CARGO_LLVM_COV_TARGET_DIR`, overwriting any
explicit caller value. On a host whose `/tmp` is a quota-limited tmpfs the build dies with
`Disk quota exceeded` and the gate reports red on a green tree — three
preserved runs did exactly that. Every green run also litters: 2,099
`swarmail-*` entries (162 MB) in `/tmp` — 73 copies of each inline
persistence fixture, 365 `swarmail-tls-*` dirs, and the selftest root
(:272) that is never removed — from three fixture helpers that clean up
partially or not at all (`src/persist.rs:357-366`,
`src/store/persist_tests.rs:5-11`, `src/tls.rs:141-150`; SQLite's `-wal`/
`-shm` sidecars are the part that leaks). And the declaration checks are
line regexes that six bypass fixtures defeat: a `#[path]` re-export of a
harness-named file, a `#[cfg(not(test))]` twin, a guard living inside a
block comment, an `include!`, a `#[coverage(/**/off)]` / `#[coverage( off )]`
substring dodge, and a `pub mod` split across lines — each passes the gate's
own check functions today while production code leaves the measured
denominator.

**#102 — tests that cannot fail are decoration, and worse, they are lies
with green checkmarks.** The webhook budget tests watch a **3 s window**
(`tests/webhook.rs:45-52`) while the dispatcher's fifth attempt would land
at ≈ +8.5 s (`src/webhook.rs:497-499`: `0..4u32`, `100·4^(attempt-1)` ms
backoff) — the window closes *before* the code under test can be observed
to matter, the comment at :47-49 asserts otherwise, and a loaded run can
push even the fourth attempt (≈ +2.1 s) past the window spuriously.
`http_send_text_allow_eof` (`tests/common/mod.rs:526-543`) delegates to
`http_send_text`, whose `read_to_end` + `lines().next().unwrap()`
(:623-631) panics on an empty response *before* the EOF mapping can apply —
the macOS red of record, and it also corrupts a real 503-with-empty-body to
status 0, misstating 0018 §2's own outcome table.
`two_concurrent_set_event_arms_both_survive` (`src/chaos.rs:390-422`) passed
in only 38/7/25 of 20,000 runs against a reintroduced lost update, and its
doc claims "a barrier join" that does not exist.
`an_ampersand_run_decodes_in_bounded_work` (`src/extract.rs:189-198`)
produces identical output with or without the 40-character reference window
(:70-83) — it can only get slower, never red. Three "registers no waiter"
triples (`tests/api.rs:1207-1216`, `tests/mcp.rs:1534-1541`,
`tests/mcp.rs:2002-2011`) scrape `swarmail_awaits_active` *after* the
refusal — the row is gone either way, so subscribing before validating (the
0025 §5 violation) passes all three; `tests/mcp.rs:1533-1535` even concedes
it in a comment. The MCP timeout arm (`tests/mcp.rs:1067-1101`) runs a whole
first wait before the one it claims to test, so "lands mid-wait" is false
on its face. `serve_takes_its_binds_from_the_environment`
(`tests/binary.rs:212-236`) binds `127.0.0.1:0` everywhere and reads back
whatever was printed, so ignored `env =` wiring (0030 §1's contract) passes.
`smtp_send`, `SmtpConn::reply`, `read_to_end` and the stdio bridge child
waits (`tests/binary.rs:300`, `:325`) have no deadlines — a wedged peer
hangs the CI slot for an hour instead of failing in seconds. And
`tests/pop3.rs:708-714` paces a 100 ms sleep between 2 000-byte writes:
coalesced TCP segments carry the terminator inside the first fill, so the
cap-and-drain loop the test names never runs — and under the exact branch
gate that is not merely vacuous, it is a flake source.

## The ruling — synthesis of the panel

Both positions agree on the skeleton; where they conflict, the following is
ruled, with the losing side's cost named. This is not an average.

**Adopted from both, unchanged:**

1. **Observe by signal, never by clock** (robustness's thesis, simplicity's
   §2): every budget, wait and readiness assertion ends on a `/metrics` row
   read over real HTTP — the `tests/webhook.rs:588-610` exhaustion-signal
   pattern (baseline, send, wait-for-rise, read) generalized to all five
   budget tests — and `BUDGET_WINDOW` dies with its false comment. The
   dispatcher's counter discipline at `src/webhook.rs:482-485` (increments
   follow the function's last await) is exactly why observation goes
   through the server's own rendered row and never races an in-process
   atomic.
2. **Refuse by structure, never by substring** (both): the declaration walk
   hardens (comment-stripped, any-visibility, `#[path]`/`include!`-aware,
   exact-`#[cfg(test)]`-only, token-wise `coverage` matching) and — the
   part that actually ends the arms race — **`check_report_covers_src`**:
   every non-harness `src/*.rs` must appear as an `SF:` record in the
   report, or the gate refuses. A laundering trick that evades the walk
   leaves the file *absent from the report*, and absence refuses. The walk
   keeps its job — fast-fail, loud, naming the file — but is explicitly
   *not* the guarantee; the cross-check is.
3. **The gate must survive its host** (both): the instrumented build
   resolves under `target/` (gitignored, disk-backed) by default; an
   explicit `CARGO_LLVM_COV_TARGET_DIR` is honoured verbatim; the selftest
   root lives in the same disk-backed parent and is trap-removed; a
   free-space preflight refuses early, naming the path, the filesystem and
   the escape variable. No `TMPDIR` instruction is documented *better* —
   the TMPDIR rule dies.
4. **Drop guards at the three fixture sites, covering the SQLite sidecars**
   (robustness's demand, simplicity's mechanism): the db moves *inside* a
   per-test temp dir, so `-wal`/`-shm` die with it by construction and
   cleanup survives a panic; the pid+nanos naming is deleted; the four
   explicit `remove_file` triples in `src/persist.rs` are deleted, not kept
   beside the guard. The gate never scavenges stale `swarmail-*` roots it
   did not create (refuse, don't reclaim — 0033's spirit). Honest loss,
   accepted by both: a drop guard cannot survive `kill -9`; the preflight's
   job is to refuse *honestly next time*.
5. **The waits-registered counter is cumulative, per-inbox, and
   delta-only-asserted** (both, reconciled): one family
   `swarmail_awaits_registered_total{inbox}`, one increment at `park_await`
   (`src/api.rs:242` — the single registration site for REST `await` *and*
   MCP `wait_for_email`), rendered through the existing per-inbox row
   machinery. Per-inbox is not gold-plating: servers run in-process
   (`tests/common/mod.rs` `run_on`), and a **global** counter would be
   raced by parallel tests in one binary — the exact flake class #102 is
   paid to kill. Its doc comment carries the contract in so many words:
   per-process values, deltas within one process, never compared across
   binaries or restarts. The three refusal triples assert the row is
   **unchanged across the refused call** — a delta, never an absolute.
6. **The budget tests keep the stored-and-queryable assertion** (robustness,
   failure mode 1): a mail delivered against a dead target is still
   queryable through the API despite the dispatcher dropping it. That
   assertion is the witness that budget exhaustion lives entirely on the
   delivery side of the accept/commit line (0001); no budget repair may
   delete it as "not about the budget".
7. **Every bypass arm is shown red against the current gate before the
   repair** (robustness §#101.5): an arm that is green before and green
   after proves nothing about the check it claims to exercise. The red
   proofs are recorded mutation runs, same discipline the batch demands of
   the tests, pointed at the gate itself.

**Conflicts ruled:**

- **POP3 read-splitting: sizes and byte order, not a "seam"** — *the batch's
  one recorded deviation from the framing's letter (criterion 13)*. A
  server-side seam cannot exist for this test: integration tests spawn the
  real binary, `#[cfg(test)]` never compiles there, and an env-var test
  mode is a feature flag, which 0020 rule 4 refuses. A client-side pacing
  helper is the 100 ms sleep renamed — an event inside a timing window,
  the exact thing criterion 13 forbids. The letter is therefore
  *unsatisfiable*; the *intent* ("no event inside a timing window") is met
  exactly by making the first fragment larger than
  `max(MAX_COMMAND_LINE=4096, BufReader's 8192)` (the existing 20_000
  serves) with the terminator as the *last* bytes of the stream: no fill
  before the terminator can contain `\n` (a fill is ≤ 8192 bytes and the
  newline sits beyond the run), so `read_line_capped`'s `None`-branch cap
  and `drain_line` (`src/pop3.rs:222-279`) fire under any TCP coalescing,
  deterministically, with no sleep. Verified against this tree.
  *Cost of the losing side* (insisting on the word "seam"): the only
  honest seam is a client-side write-shape helper — the sleep renamed —
  plus a re-introduced timing window and a rule-4 violation. Ruled for
  sizes; the deviation is recorded here, in the committed spec, not in a
  position file.
- **Gate language: stay awk, under a recorded tripwire.** Robustness would
  rather see a small Rust helper and accepts awk *only* because of the
  red-proof arms; simplicity refuses any rewrite, build target or second
  language. Ruled: the gate grows inside the file it already is; the
  red-proof selftest arms are the insurance. *Tripwire:* if the red-proofs
  rot — an arm going green against a known defect — the gate gets rewritten
  in Rust, and that is a decision document then, not now. *Cost of
  Rust-now:* a build step in every seat's path, gate code either
  self-measured under the exact gate or outside the tested surface, and a
  fixture-H-proof the `SF:` cross-check already buys. *Cost of awk-forever:*
  a hostile parser always has one more fixture — which is precisely why the
  cross-check, not a cleverer regex, carries the guarantee.
- **Scratch cleanup: `tempfile` (dev-dependency) is the primary; one shared
  `#[cfg(test)]` guard is the fallback only.** Robustness requires
  sidecar-covering guards held for the fixture's whole life; simplicity
  prices tempfile at one manifest line with a one-sentence defense
  (*panic-safe scratch cleanup at three fixture sites is TempDir's exact
  job; hand-rolling a Drop guard is new measured code doing what a
  dev-dependency already does better*), licenses already allowlisted
  (MIT OR Apache-2.0; `fastrand`, `rustix` — permissive, dev-only, zero
  runtime surface). Ruled: tempfile. *Cost of the fallback:* a cross-module
  import (tls reaching into persist) and ~12 lines of permanent
  exact-100%-covered code whose entire job is `remove_dir_all` — accepted
  only if the operator refuses the dependency, and then *one* shared guard
  beside the db lifecycle (`src/persist.rs`, `pub(crate)`), never three
  copies and never a new `src/testsupport.rs`.
- **The counter is production-visible, not `#[cfg(test)]`-only.** Simplicity
  rules it in: integration tests cannot see crate-private seams (0020 rule
  4 keeps seams test-only), and a test-only counter would be a lie on the
  wire. Robustness's delta-only contract is folded into the doc comment.
  *Cost:* one more per-inbox metrics family — the same cardinality class
  the report already renders (`awaits_active`, `deliveries_in_flight`,
  `requests_in_flight`); a cap would be a new decision, not this batch's
  business.

**Rejected (with the price the rejection avoids):** a parser-based gate in
a new language (§ above); a global waits counter (the parallel-flake class);
per-transport counters (two spellings of one site); a per-attempt webhook
metric (the dispatcher's ledger already has `exhausted_total`); a generic
timeout framework (seven wrapped call sites read fine with one named
const); a `src/testsupport.rs` (a permanent measured file for one guard);
new env-var vocabulary (`SWARMAIL_COVERAGE_*` would be a second spelling of
`CARGO_LLVM_COV_TARGET_DIR`); any `bundles/` edit (the gate self-resolving
to a disk-backed dir is what makes the verify-seat edit unnecessary); and
any wall-clock bound assertion (0020 rule 5) — the scan counter cannot
flake, that is the whole argument.

## The design

One governing rule, from the simplicity seat, with the robustness seat's
teeth: **every scratch path a test creates is owned by a guard; every
counter is one increment at its one owner; the gate grows inside the file
it already is — and every check must be able to go red, or it is
decoration.**

The committed shape, per work item (the HOW lives in
[plan.md](plan.md), the ordered work in [tasks.md](tasks.md)):

1. **Gate build location** (#101): `:426-438` becomes a two-arm resolution —
   explicit `CARGO_LLVM_COV_TARGET_DIR` honoured verbatim; default a fresh
   `mktemp -d target/coverage/build.XXXXXX` (moving the existing
   `mkdir -p target/coverage` above it); the EXIT trap cleans only the dir
   the gate itself created. A preflight `check_free_space <dir> <min_kib>`
   (one `df`, one named constant ≈ 6 GiB — the ≈ 4 GiB build plus slack,
   the arithmetic in a comment) refuses naming the path, filesystem, and
   `CARGO_LLVM_COV_TARGET_DIR`. The selftest drives the refusal by passing
   an absurd floor — no fake mounts, no new tooling — and proves both
   resolution arms, because a default without an override arm is half a
   contract.
2. **Selftest root** (#101): same disk-backed parent
   (`target/coverage/selftest.XXXXXX`), trap-removed. The selftest's fixture
   trees are kilobytes; nothing in them needs tmpfs and nothing may outlive
   the run.
3. **Scratch drop guards** (#101): `tempfile::TempDir` at the three named
   helpers (`src/persist.rs:357-366`, `src/store/persist_tests.rs:5-14`,
   `src/tls.rs:141-150`), db *inside* the temp dir, pid+nanos naming and
   the four explicit `remove_file` triples deleted. The before/after
   `$TMPDIR` listing around the verify run is the proof of the normal path.
4. **Declaration checks** (#101): one shared comment-stripping pass, then —
   any-visibility `mod <stem>` (incl. `r#`); any `#[path]` whose *target*
   is harness-named, anywhere in the parent; any `include!` of a
   harness-named file; refusal of any cfg on such a declaration that is not
   exactly `#[cfg(test)]`; token-wise `coverage` matching
   (comments stripped + whitespace collapsed). Fixtures A–G become
   printf-sized `expect_refuse` arms — the selftest harness at :270 is
   already exactly this shape — each shown red against the current gate.
5. **The wall** (#101): `check_report_covers_src` — after the report
   exists, every non-harness `.rs` under `src/` must appear as an `SF:`
   record; ~15 lines of awk over a `find`. Absence refuses, loudly,
   naming the file. This is the check that converts laundering from "evade
   the walk" (unwinnable in the limit — two rounds of regex hardening, #50
   then #101, prove it) to "be absent from the report" — which refuses.
6. **Webhook budget tests** (#102): the five budget tests (:455, :536,
   :571, :680, :747) adopt the exhaustion-signal pattern;
   `BUDGET_WINDOW` (:45-52) and the false comment are **deleted**, and any
   record sentence repeating the claim is corrected. The budget count is
   read after exhaustion, so a fifth attempt is *seen* if the loop ever
   grows — the `0..4 → 0..5` flip then goes red, which is the regression
   proof (a mutation run, never an edit). The dispatcher's loop itself is
   *never edited*.
7. **Fallible EOF helper** (#102): `http_send_text_allow_eof` performs its
   own `read_to_end`; zero bytes (or a reset) yields `(0, "")`; a real
   status is **never rewritten** — a 503 with an empty body stays 503,
   per 0018 §2's own outcome table. ~10 duplicated lines in a test helper
   beat a mode-flag parameter; no new abstraction. Proven on a real socket
   (0004, no mocks).
8. **Chaos two-scope test** (#102): the test holds `chaos.config.write()`
   across the arms' spawn, release, and join, so every arm's read-clone
   happens before any write and a reintroduced read-clone/write split
   *fails the assert* instead of passing in 38 of 20,000 runs. The doc
   comment's false "a barrier join" sentence is deleted. Chaos at the
   poles only (probability 0 or 100) — branch-stable by construction.
9. **Bounded-work counter** (#102): two `#[cfg(test)]` atomics in
   `src/extract.rs` beside `parse_ref` (calls, and window bytes) — the
   ampersand test asserts `bytes ≤ REF_MAX × calls + slack`, slack named in
   the comment for parallel inline tests' contribution. Remove the
   40-character window and the bound blows up by ~4 orders of magnitude on
   the same input — red, deterministically, no wall clock anywhere. The
   window itself (:70-83, the security HOLD) is untouched.
10. **Waits-registered counter** (#102): the new cumulative per-inbox
    family, one increment at the single `park_await` site, rendered through
    the existing per-inbox row machinery; the three refusal triples assert
    the delta (unchanged across the refused call); validating-after-
    subscribing (0025 §5) then goes red in all three. The MCP timeout arm
    mirrors its REST twin (`tests/api.rs:703-731`): spawn the wait, poll
    its parked row, *then* send — a pre-wait total on that arm goes red.
11. **Helper deadlines** (#102): `tokio::time::timeout` with one named
    const (10 s, matching the drain-deadline fence's scale, 0027 §7)
    around `smtp_send`'s reads, `SmtpConn::reply`, `read_to_end`, and the
    stdio child waits (`tests/binary.rs:300`, `:325`) — panics naming the
    step. A timeout bounds *failure*; it does not open a validity window,
    so this *is* 0020 rule 5's "real deadline", not a violation of it.
12. **Wide windows by construction** (#102): the bind-env test sets
    concrete free ports (bind-and-release) and asserts the parsed
    startup-line ports equal the env's — ignored `env =` wiring goes red
    (0030 §1). The 408 arms (`timeout_ms=1500` → ≥ 5000, well past
    poll+send+commit). POP3 per the ruling above — sizes and byte order,
    the sleep deleted.

## Acceptance Criteria

Each criterion is testable, and each names its red — a criterion that
cannot go red is not done (the verify gate records the red→green runs).

- [ ] **AC-1 (build dir default).** With no `CARGO_LLVM_COV_TARGET_DIR`
      set, the gate builds the instrumented tree under `target/coverage/`
      (gitignored, disk-backed) — never under `$TMPDIR`. *Red proof:* the
      current gate creates `$TMPDIR/swarmail-coverage.XXXXXX`; the
      before/after listing of criterion AC-3's run shows it.
- [ ] **AC-2 (override honoured).** With `CARGO_LLVM_COV_TARGET_DIR` set,
      the gate uses the caller's value verbatim. *Red proof:* today the
      gate overwrites it (`scripts/coverage-gate.sh:433`); a selftest arm
      passes an explicit dir and asserts the gate's resolution returns it
      unchanged, red against the current export.
- [ ] **AC-3 (no `$TMPDIR` litter).** A green `cargo test --workspace
      --locked` and a gate selftest leave no new `swarmail-*` entries in
      `$TMPDIR`, verified by a before/after `$TMPDIR` listing around the
      verify run; the selftest root is removed by its trap.
- [ ] **AC-4 (disk-full refuses early).** A preflight free-space refusal
      names the path, the filesystem, and `CARGO_LLVM_COV_TARGET_DIR` — not
      "the instrumented run failed". *Red proof:* the selftest arm passing
      an absurd floor to `check_free_space` is red against the current
      gate (no preflight exists).
- [ ] **AC-5 (fixtures A–G refuse).** Each bypass fixture becomes a
      selftest arm that refuses against the new checks — block comments
      stripped before the walk; any-visibility `mod <stem>` (incl. `r#`);
      any `#[path]` whose target is harness-named; any `include!` of a
      harness-named file; a non-`test` cfg on such a declaration; token-
      wise `coverage` matching — and each arm is **red against the current
      gate** (recorded mutation run), green after the repair.
- [ ] **AC-6 (report completeness).** A non-harness `src/*.rs` file absent
      from the report's `SF:` records refuses, naming the file; a complete
      report passes. *Red proof:* the arm is red against the current gate
      (the check does not exist).
- [ ] **AC-7 (budget by signal).** No budget assertion depends on a fixed
      window; all five budget tests observe
      `swarmail_webhook_exhausted_total` rising over its pre-mail baseline
      (the :588-610 pattern) before reading the hit count; `BUDGET_WINDOW`
      and the false comment at `tests/webhook.rs:47-49` are gone; every
      budget test keeps the stored-and-queryable assertion. *Red proof:*
      flipping `src/webhook.rs`'s loop `0..4 → 0..5` turns at least the
      plain-HTTP budget test red — a fifth attempt is now *seen*.
- [ ] **AC-8 (EOF helper honest).** A peer that accepts and closes yields
      `(0, "")`; a 503 with an empty body yields 503 — proven on a real
      socket. *Red proof:* the current helper panics on the empty response
      (`unwrap` before the EOF mapping) and rewrites 503 to 0; the new
      tests fail against it.
- [ ] **AC-9 (chaos two-scope).** Holding `chaos.config.write()` across the
      arms' start and releasing it makes
      `two_concurrent_set_event_arms_both_survive` fail when the lost
      update is reintroduced (read-clone + separate write). *Red proof:*
      the reintroduction mutation run — deterministically red, not 38 in
      20,000. The false "a barrier join" sentence is gone.
- [ ] **AC-10 (bounded work, structurally).** The ampersand test asserts
      the `#[cfg(test)]` scan-counter bound (`bytes ≤ REF_MAX × calls +
      slack`) and fails when the 40-character reference window
      (`src/extract.rs:70-83`) is removed — the window itself survives
      untouched. *Red proof:* the removal mutation run (≈ 4 orders of
      magnitude on the same input, on any box, no wall clock).
- [ ] **AC-11 (refusals observe registration by delta).** Each
      "registers no waiter" triple (`tests/api.rs:1207-1216`,
      `tests/mcp.rs:1534-1541`, `tests/mcp.rs:2002-2011`) asserts
      `swarmail_awaits_registered_total{inbox}` is **unchanged across the
      refused call** — a delta, never an absolute. *Red proof:* the
      0025 §5 violation (validate after subscribing) goes red in all three,
      deterministically. The counter's doc comment states the delta-only,
      per-process contract.
- [ ] **AC-12 (MCP timeout arm real).** The MCP arm of
      `wait_results_carry_the_post_wait_total_in_both_arms` spawns the
      wait, polls its parked row, then sends — mirroring the REST twin.
      *Red proof:* reintroducing the pre-completed first wait (mail sent
      before the wait parks) turns the arm red — the total is no longer
      counted after the wait.
- [ ] **AC-13 (bind env read back).** `serve_takes_its_binds_from_the_environment`
      sets concrete free ports (bind-and-release) and asserts the parsed
      startup-line ports equal the env's values. *Red proof:* stubbing the
      `env =` wiring to loopback defaults goes red on the read-back.
- [ ] **AC-14 (deadlines name the step).** `smtp_send`, `SmtpConn::reply`,
      `read_to_end`, and the stdio child waits (`tests/binary.rs:300`,
      `:325`) fail within a bound (`tokio::time::timeout`, one named 10 s
      const) with a panic naming the step. *Red proof:* a real socket that
      accepts and never replies fails the helper within the bound, message
      naming the step — against the current helpers it hangs.
- [ ] **AC-15 (no window under ~5 s).** The 408 arms use `timeout_ms`
      ≥ 5000; POP3 read-splitting is driven by sizes and byte order (first
      fragment > max(4096, 8192), terminator last), not a 100 ms sleep, so
      no new or touched test depends on an event inside a window shorter
      than ~5 s. *Red proof (POP3):* reverting to the sleep-paced shape
      lets coalesced reads skip the cap-and-drain loop — the exact branch
      gate's None-branch is not taken on every run; the size-driven shape
      takes it on every run (the exact gate is the detector).
- [ ] **AC-16 (gates green).** `cargo fmt --all -- --check`, `cargo clippy
      --all-targets -- -D warnings`, `cargo test`, `brokkr compile --bundle
      bundles/self`, `bash scripts/coverage-gate.sh` (exact 100/100/100,
      floor never falls), `bash scripts/coverage-gate.sh selftest` (all
      arms), and `cargo deny check licenses` all pass; every new line of
      production code (the counter family, the atomics, refusal messages,
      gate functions) is executed by a test on the instrumented run.

## Decision ruling — none minted

No change here touches a frozen guarantee, flips a public contract, or
amends a ruling. The metrics family is **additive** — the same class as
`swarmail_webhook_exhausted_total`, which #51 landed without a decision;
the gate and its environment are tooling; 0020, 0025 §5, 0018 §2, 0030 §1,
0004 and 0033 are applied as written. The framing of record rules no
decision is required and the next free number stays 0034; the simplicity
seat prices zero and the robustness seat demands none. *Cost of the losing
side (minting one):* a decision that rules nothing is vocabulary without a
semantic change to carry — noise in the index every future seat must read.
If the operator later wants the registered-counter contract written up
(per-process, delta-only), that is an ordinary change with a real semantic
claim, not this batch's.

## What this batch refuses to do

- Touch the 40-character reference window (`src/extract.rs:70-83`, the
  security HOLD), the bounded-work pin tests, the `smtp=` startup-line
  contract, the 10 s drain-deadline fence (0027 §7), or the record-sentence
  corrections at `791f83d`/`47fa0e2`.
- Touch the dispatcher's `0..4` loop (`src/webhook.rs:497`) — the
  `0..5` flip is a mutation proof, never a change.
- Weaken or *unprove* a frozen guarantee: no budget test loses the
  stored-and-queryable assertion (0001); the refusal triples enforce the
  0025 §5 boundary (validate before subscribing), strengthening 0002's
  line; nothing adds a read-modify-write on the prune path (0003
  untouched); extraction stays at ingest (0005).
- Edit `bundles/` — the gate self-resolving to a disk-backed dir is what
  makes a verify-seat edit unnecessary; any residual goes to the run's
  `*-operator-edits.md`.
- Add `coverage(off)` anywhere under `src/`, a new env var, a new gate
  script, a new `src/` file, or a feature flag for tests.
