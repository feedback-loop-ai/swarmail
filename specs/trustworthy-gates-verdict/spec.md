# Run 6c — the verdict the trustworthy-gates batch never received, earned at its final tree

Feature: complete the run-6c completion — the "Make the gates, tests and delivery
trustworthy" batch (issue #58 children #50–#53) is implemented, gated, tested, repaired of
its two named doc defects (`54e9315`, `8877543`), and root-caused of the hang that killed
its verdict twice: under the hermetic seat's cleared environment the serve's startup line
carried ANSI escapes inside its tracing fields, so `tests/binary.rs`' `parse_addrs` (the
literal `smtp=` contract) never matched and every serve-spawning test waited forever. The
operator fixed it at the root — `7c2c446`: the tracing subscriber emits ANSI only when
stdout is a terminal — raised the verify budget to 10800 s, and re-proved the house gates
on the fixed tree (fmt, clippy, 19 test targets, coverage exact, deny, brokkr compile).
Run 6b's own spec (`996574d`) never got past its verify seat. Run 6c is the third and
narrowest attempt: **verify runs the full hermetic gate set to exit 0 at the run's final
HEAD, the review panel judges the delivered batch against the four children's acceptance
criteria, and ship signs on a clean, drift-free tree.** The tree of record is `7c2c446`;
the run's final HEAD is `7c2c446` plus this spec trio plus at most the one repair ruled
below.

Why this is still a trust matter: a verdict claimed without its proof is the same species
of lie the batch exists to kill. So run 6c has no design of its own. The design of record
is `specs/trustworthy-gates-tests-delivery/spec.md` (R1–R15) as realized in the delivered
commits, and the completion rulings of `specs/trustworthy-gates-completion/spec.md` carry
except where this spec supersedes them. Framing: the run brief plus
`.forge/intake/run6c-completion-framing-87213da4.md` (there is no `.forge/tasks/` in this
tree). Both panel seats read the delivered tree this session
(`.forge/design/positions/simplicity.md`, `.forge/design/positions/robustness.md` —
run-local evidence, uncommitted) and returned **pass** with one genuine conflict between
them, ruled below with the loser's cost named.

## Positions and rulings

**R1 — Scope: the delivered tree stands; zero-delta discipline holds with one named
exception.** *(Simplicity's audit adopted as the scope ruling; robustness concurs on
everything but one sentence, ruled in R2.)* The batch is delivered, gated and tested; a
completion run that rewrites working machinery to satisfy an aesthetic is a new risk with
no gate demand behind it. Adopted verbatim, simplicity's refusal list, re-verified against
the current tree: no new gauge, registry, endpoint, bench or `#[cfg(test)]` seam in `src/`;
no retention policy for `target/coverage/run.*`; no parser rewrite of the gate's awk
checks; no dependency ride, re-pin or version bump (`Cargo.lock` and `deny.toml` are
untouched by the run); no doc ride-alongs beyond the one repair ruled below; no reopening
of the three recorded residuals — the global webhook cap that lets one hostile sink starve
every target with an unattributed dropped row, attacker-shaped aggregate memory owned by
the deployment boundary (decision 0026's documented relation), and unbounded idle peers on
the trusted-harness plaintext posture — each is a *next-feature candidate, recorded here,
not this run's work*; no re-staging of the exactness proof as redundant wall-clock; no
seat edit to `agents/`, `adapters/`, `bundles/` (the operator channel
`.forge/run6c-operator-edits.md` exists). Cost of the refusal — the losing side is the
temptation to fix the residuals now: churn without a gate behind it, a reopened batch, and
a verdict delayed a fourth time.

**R2 — The panel-named defect is repaired: the accept-loop drain comment is off by one
stage.** *(Robustness's finding, adopted; simplicity's "the run adds nothing" loses for
exactly one comment — this is a defect the panel named, the framing's own V5 exception
covers it, and it is the same species the 6b panel repaired in `54e9315`.)* Verified in
tree by the chief: the `JoinSet` drain comment in `src/smtp.rs`'s `serve` (the
parenthetical ending "a session mid-DATA has no commit to land, so the closed socket is
the only goodbye it gets") is true for a session that has not yet crossed the DATA
terminator, and false for one parked at the insert's await — `insert_async` offloads the
whole mutation to `spawn_blocking` (`src/store.rs`, `Store::offload`), and a task abort
cannot cancel a blocking closure: the orphaned commit keeps the store's `Arc`, lands its
row (memory apply and the WAL mirror commit under the 0014 boundary), and no `250` is ever
written. The client's retry is a duplicate, never a loss — exactly the honest asymmetry
0001 demands — and the parked `_in_flight` guard releases at abort, before the orphaned
commit resolves: a shutdown-only observability skew, on a process that is leaving anyway.
The wire truth is already proven — `tests/lifecycle.rs::an_in_flight_commit_neither_holds_
stop_nor_half_lands` (R11 of 0018) asserts the commit is not cancelled, stop is not held,
and retry is lossless — so the behavior is right and the comment's reasoning is wrong
about the very stage the drain exists to explain. A test author or reviewer trusting the
comment reasons backwards about shutdown durability. The repair is doc-only: no behavior
change, no frozen guarantee touched, no ruling unsealed, **no decision document**. Cost of
refusing it: the trust run ships a citation that lies about shutdown semantics, in the
file the review panel reads to certify the drain.

**R3 — The counts of record move: 5826/320/729 → 5827/320/729.** *(Chief's own finding —
neither seat carried it; both treated 5826 as the pinned counts, which is now stale.)*
`7c2c446` added one measured line to `src/main.rs` — the
`.with_ansi(std::io::stdout().is_terminal())` call — and the operator's own exact-gate run
on the fixed tree (evidence dir `target/coverage/run.BUoDkq`, written seconds before the
commit) reads **5827/320/729, exact, missed == 0**; branches and functions are unchanged
(320/729). Run 6b's detector (5826/320/729) was pinned to the `a9efa40` tree; a completion
spec carrying the stale number would stop the run on a legitimate delta. So: the verify
seat's run at the run's final HEAD must read 5827/320/729, and that run beside the
operator's `run.BUoDkq` is two independent exact runs at the new counts — the
counts-stability half of the parent spec's AC50.1 proof, re-established at the tree that
will ship. The captured concurrent pair (`run.1DTHyN` + `run.Tg4n8z`, both 5826, 0.33 s
apart) remains the demonstration of the per-run-evidence mechanism at the batch tree;
`scripts/coverage-gate.sh` is untouched by everything since, so the mechanism carries.
Re-staging a fresh concurrent pair is refused as redundant wall-clock (simplicity's
refusal 7) — unless the seat's run drifts from 5827/320/729, in which case the run stops
and the truth is named: the delta was not what it claimed. The R2 repair is a comment and
moves no measured line (doc comments are not DA records); the detector proves exactly
that.

**R4 — The 10800 s budget is sizing, not masking, and AC-6c.2 is its proof.** *(Both
seats converge; adopted with the chief's own re-run added.)* The hang was root-caused
(sandbox replica + strace), fixed at the root in the emitter (`7c2c446`), and the
formerly-hung binary suite passes in the exact sandbox — the chief re-ran it this session:
9 passed in 5.13 s under cargo's captured (piped, `NO_COLOR`-less) stdout, the exact
condition that hung run 6/6b for 77 minutes. The fix is in the emitter, not the harness:
a `NO_COLOR` ritual in the test would have un-hung the tests while leaving CI, systemd and
`docker logs` parsing escapes — refused, because the defect is the output format's
environment-dependence and the root fix removes the dependence for every piped reader.
The deadline must not fire again: if it does, the attempt fails honestly (max_attempts 2
stands) and a second kill is reported to the operator — never economized into a green.

**R5 — Red is red, and could-not-check refuses.** *(Both seats' red lines, carried
verbatim.)* No gate retries a red verdict — the rerun habit launders the cold-line miss
and every future red with it. A red gate is repaired the way `cc555d6` and `7c2c446` were:
named, minimal, its own commit, its test landing with it — then the gates that read the
touched paths re-run, and the full seat again if `src/` changed. The fail-closed posture
of the selftest, the publish-guard proof and the native-tls probe stays exactly as
delivered: exit 0 → refuse; the named "did not match any packages" report → pass; anything
else → refuse — never a bare `!` negation. security-hold is a hard stop (constitutional,
protected phase).

**R6 — Review closes the gap the design panel named — it never got its read.**
*(Robustness's ask, carried from the 6b spec's R7, which died at verify before its review
ran; simplicity's argument is the reason no new test is demanded, not a substitute for the
read.)* The correctness member reads the batch's `src/` deltas before voting: the three
labeled gauge registries and their drop-guards (`src/api.rs`, ~170 lines), the park wiring
in `src/mcp.rs`/`src/smtp.rs`, the pre-bind SIGINT watcher in `src/main.rs`, and this run's
new delta — the ANSI-conditional subscriber. Robustness names `park_delivery`'s guard
lifecycle as "the one place a new lie could hide"; simplicity answers every line is under
the exact gate and executed by the very polls that demand it. Both are right; the ruling
is the read. Cost: ~180 lines read. Cost of refusing it: the panel's pass rests on an
unread delta.

**R7 — Review mechanics of record.** The four members of `bundles/self/bundle.json`
(correctness, security, guarantees, tests) judge the delivered batch against the four
children's acceptance criteria (issues #50, #51, #52, #53 of
https://github.com/feedback-loop-ai/swarmail/issues/58), the parent spec
(`specs/trustworthy-gates-tests-delivery/` — R1–R15 and its ACs), and accepted decision
0026. The issues' line references are from baseline `ac8c326`; main has moved — **locate
by shape, not by line**. No member closes an issue or opens a PR; the operator lands via
PR and closes the children with CI citations. Verdicts are typed per member (clean |
residual | security-hold) and aggregated by the bundle.

## Acceptance Criteria

**Verify — the parked work, completed at the final tree**

- AC-6c.1: `bash bundles/self/scripts/verify-seat.sh` exits 0 at the run's final HEAD
  within the 10800 s budget, with every declared gate green and none skipped: `cargo fmt
  --all -- --check`; clippy `--workspace --all-targets --locked -- -D warnings`; `cargo
  test --workspace --locked`; `brokkr compile --bundle bundles/self`; the exact coverage
  gate reading **5827/320/729** (literal integer equality, missed == 0 in lines, branches
  and functions — a rounded verdict is red); `coverage-gate.sh selftest`;
  `scripts/publish-guard-test.sh`; `cargo deny check licenses`; the fail-closed
  native-tls probe. A red gate is reported red, never retried into green; a deadline kill
  is a failed attempt, reported as such.
- AC-6c.2: The serve-spawning suite completes in minutes under the seat's cleared
  environment: no serve-spawning test waits forever, and `parse_addrs` matches the piped
  startup line (the literal `smtp=` contract) with no ANSI escapes and no `NO_COLOR`
  ritual. The seat's captured output shows `tests/binary.rs` finished; the chief's own
  re-run at the tree of record is the demonstration (9 passed in 5.13 s; the operator's
  sandbox replica measured 5.4 s).

**The repair — the run's only delta**

- AC-6c.3: The accept-loop drain comment in `src/smtp.rs` states the true shutdown policy,
  stage-precise: a session that has not crossed the DATA terminator has no commit to land
  (the closed socket is the only goodbye it gets); a session parked at the insert's await
  lands its commit — `spawn_blocking` is not cancelled by the abort — stored but
  unacknowledged, the client's retry a duplicate, never a loss (0001's honest asymmetry),
  and the parked in-flight guard releases before the orphaned commit resolves. The comment
  agrees with the DATA arm in `src/smtp.rs`, with `insert_async`'s offload to
  `spawn_blocking` in `src/store.rs`, and with
  `tests/lifecycle.rs::an_in_flight_commit_neither_holds_stop_nor_half_lands`, which
  already asserts the wire truth. No behavior changes anywhere in the run.
- AC-6c.4: Nothing else changes: measured against the tree of record (`7c2c446`), the
  run's commits are this spec trio plus the one named repair; `src/`'s only delta is that
  comment; no commit touches `tests/`, `scripts/`, `charts/`, `.github/`, `Cargo.toml`,
  `Cargo.lock` or `deny.toml` unless a red gate or a review-named defect forces it, and
  each such commit is named, minimal, and lands with the test that proves it (decision
  0020's disciplines bind any such edit: no sleep as synchronization, no wall-clock
  validity assertions, chaos at the poles only, per-test fresh stores/inboxes, windows
  only through seams). No decision document is drafted: nothing semantic changes, and
  decision 0026 is already accepted.

**Review and ship**

- AC-6c.5: The review panel returns typed verdicts (clean | residual | security-hold per
  member; the bundle's aggregate) judging the batch against the four children's acceptance
  criteria (C50.1–C50.5, C51.1–C51.6, C52.1–C52.5, C53.1–C53.5), the parent spec, and
  accepted decision 0026, locating by shape; the correctness member's read includes the
  deltas named in R6; any defect the panel names lands per AC-6c.4's discipline and
  re-enters verification (the full seat again if `src/` changed); security-hold stops the
  run.
- AC-6c.6: Ship signs only with `git status --porcelain` empty and no drift between the
  tree verify proved and the tree ship signs: the eight frozen required context names in
  `.github/workflows/ci.yml` byte-identical (`format, clippy, publish guard`,
  `test (ubuntu-latest)`, `test (macos-latest)`, `coverage`, `deny`, `RustSec dependency
  audit`, `helm chart`); `agents/`, `adapters/`, `bundles/` untouched by seats; no issue
  closed and no PR opened by seats; issue #32's delivery (`3e0a996`, `51bac94`,
  `ac8c326`) untouched and not reopened.
- Standing gates: the six house gates clean at the run's final HEAD — fmt, clippy, `cargo
  test`, `cargo deny check licenses`, `brokkr compile --bundle bundles/self`, and the
  exact coverage gate (subsumed by AC-6c.1's seat run; the floor stays exact 100/100/100,
  no `coverage(off)`, no exclusion; the denominator never shrinks).
