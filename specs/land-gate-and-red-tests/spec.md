# Spec — the landing: #101's first half and #102's red tests verified, reviewed and merged (issues #101, #102)

- **Run:** run 12fb — chief synthesis seat
  (`brokkr-state-run12fb-feature-txt-9d308846`). Run-local evidence, all
  uncommitted: the framing `.brokkr/state/run12fb-feature.txt` and the two
  positions `.brokkr/state/design/positions/simplicity.md` and
  `.brokkr/state/design/positions/robustness.md`.
- **Feature slug:** `land-gate-and-red-tests`
- **Issues:** [#101](https://github.com/feedback-loop-ai/swarmail/issues/101)
  — **first half only**: the exact coverage gate lives off its own disk
  space — and [#102](https://github.com/feedback-loop-ai/swarmail/issues/102)
  — the remaining vacuous and timing-bound tests are able to fail. **Neither
  issue is closed or cite-closed by this run**: the operator cites.
- **Design authority:**
  [`specs/gate-and-red-tests/spec.md`](../gate-and-red-tests/spec.md). Its
  declaration-bypass half — AC-5, AC-6 and design items 4–5 (the
  declaration recognizer and the `SF:` report wall) — is **not in this
  batch**; it is run 12g's scope. The three recognizer specs
  (`stripper-prove-or-refuse`, `load-shape-token-stream`,
  `nested-attr-close-gate`) stand superseded at their Status fields, kept
  as the record.
- **Base under review:** `aba0be2` on `run12b-landing` — 13 commits over
  `origin/main` @ `5671e0c` (the framing's 12 plus the batch commit;
  **14** after this spec's one pre-ruled sharpening lands — the PR body
  reconciles the count).
- **Decisions:** **none minted** — the ruling at the end of this spec; the
  next free number stays 0034.
- **Shape:** a landing, not a build. The batch being landed carries ~89 net
  production lines, zero new files under `src/` or `tests/`, zero new
  env-var vocabulary, one dev-dependency (`tempfile`), and the deletion of
  a 694-line bash recognizer. **The landing's own delta is zero production
  lines and at most one named test-only commit** (the B5 sharpening, ruled
  below); no new spec-note edits to reviewed files; no `bundles/` edit; no
  decision; no issue closed.

## The problem

Run 12f hard-stopped with `IMPL-BROKEN-TWICE`; the operator verified the
seat's tree and landed it as `aba0be2`. This run's job is to independently
verify that tree, review it against `origin/main` scoped to what the batch
adds, and merge it — and the job has four ways to go wrong:

1. **Landing tests that cannot fail** — the exact defect #102 is paid to
   kill. Verification must apply the named mutants and watch them go red;
   a green badge without a recorded red is decoration.
2. **Review-scope erosion** — reporting main's standing declaration-bypass
   class (fixtures A–G, harness-named loads, `coverage(off)` spellings) as
   a residual of this batch. That class is main's behaviour and run 12g's
   scope; a reviewer who books it here repeats the category error the
   framing forbids.
3. **Landing drift** — "while we're here" edits absorbing silently into a
   merge the operator verified byte-for-byte. A landing that finds it needs
   a change must say so in the open, in a named commit and the PR body, or
   stop and report.
4. **A vacuity edge in the batch's own red tests** (the robustness seat's
   B5): the three delta-zero refusal arms read
   `metric_row_value(...).unwrap_or(0)` on both sides and assert equality —
   if the `swarmail_awaits_registered_total` family were ever renamed or
   its raise deleted, both sides read 0 and the arms pass while proving
   nothing. Today a sibling test (`tests/mcp.rs`, the `observed` arm's +1)
   catches a rename, so the *suite* is sound — but each *arm*'s meaning is
   mortgaged to a test in another file staying alive, which is a hidden
   coupling of exactly the class the batch exists to kill.

## The ruling — synthesis of the panel

Both positions land the batch; neither demands a redesign. Where they
conflict, the following is ruled, with the losing side's cost named. This
is not an average.

**Adopted from both — the four normative commitments** (the robustness
seat's verdict, which the simplicity seat's ratifications satisfy):

1. **Fail closed, and name the death.** The gate's preflight refusal names
   the path, the filesystem, its mount point and the escape variable
   (`scripts/coverage-gate.sh:229-250`); a store commit failure, an SMTP
   over-cap `552` and a webhook over budget each name the concrete fact.
   A refusal that cannot be acted on is a crash wearing a message.
2. **Delete only what you created.** The EXIT trap removes only a build
   directory the gate itself made (`BUILD_DIR_OWNED`, the trap installed
   only when the gate created the dir). This is the ownership discipline
   that makes a thousand parallel sessions safe.
3. **Observe without reordering.** `AWAITS_REGISTERED` is one `raise()` at
   the single `park_await` site (`src/api.rs:255`), after validation
   (0025 §5) — it observes 0002's ordering and must never change it.
   `RefScanStats` rides the ingest parse (0005) — two integer adds on a
   pass that runs anyway. The commit still completes before the `250`
   (0001); pruning stays oldest-only (0003).
4. **A test that cannot fail is a lie with a green badge.** Every arm the
   batch adds is mutant-killable, and the landing proves it: the chaos
   read-clone lost update, the removed extract reference window, and the
   webhook budget loop `0..4 → 0..5` are applied, observed red, and never
   committed.

**Adopted — the gate's disk contract, as landed** (verified in the tree
this session): `MIN_BUILD_FREE_KIB` is 6 GiB stated once (`:227`); every
remaining `mktemp` lives under `target/coverage/` (build `:270`, evidence
`:562`, selftest root); an explicit `CARGO_LLVM_COV_TARGET_DIR` is honoured
verbatim and never trap-removed (`:252-272`); the selftest arms prove the
default arm ignores `TMPDIR` and that the refusal message names the path,
the filesystem and the escape variable (`:454-483`); the export precedes
`cargo llvm-cov clean --workspace` (`:527` before `:566`), so no stale
instrumented executable can join the merge in either arm.

**Adopted — the normative sentences the spec carries forward** (B1–B4,
B6–B8; each already true of the landed tree, now binding):

- **A full disk may make the gate red; it may never make the gate green.**
  The preflight is legibility, not a guarantee — it is TOCTOU by nature.
  Nobody may later "harden" it into a reservation (`fallocate`) or treat a
  mid-run ENOSPC as a reason to skip coverage and continue.
- **SIGKILL leaks are the accepted loss.** A build directory leaked between
  `mktemp` and trap install is inert garbage on a gitignored path; there is
  **no sweep** of `target/coverage/build.*` — cleanup-by-pattern would
  delete a concurrent run's live directory. The preflight floor and the
  escape variable are the pressure valve.
- **Export-before-clean is the gate's most protected ordering.** A caller's
  explicit directory is not unique; the clean that runs *after* the export
  is what keeps a caller-owned directory candidate-bound. Moving that
  ordering — for speed, for tidiness, by accident — makes the gate report
  a stale source graph's coverage with a straight face.
- **Cumulative counters are asserted as per-inbox deltas across a window
  the test controls, never as absolute values.** Servers run in-process; an
  absolute assert would race every other test's awaits.
- **Drop guards clean on unwind, not on abort.** Under `panic=abort`,
  SIGKILL or a power cut the fixtures litter; that is not the harness mode.
  No atexit-style global janitor — it reintroduces the sweep race in
  miniature.
- **mawk is the floor.** No `?` quantifiers, no gawk-only ERE, in anything
  the gate runs; a refusal whose whole point is its message is tested
  through the message.
- **Hermetic or it lies.** The gates run under the verify seat's network
  denial, with no `TMPDIR` override, fetching nothing.

**Conflict ruled — land-exactly vs the B5 sharpening: B5 is adopted, as the
single named deviation.** Each of the three delta-zero refusal arms
(`the_wait_refuses_over_the_cap_registers_no_waiter_and_accepts_the_boundary`
in `tests/api.rs`; `the_wait_registers_no_waiter_when_staging_refuses` and
`the_staging_refuses_over_the_cap_and_honors_the_boundary` in
`tests/mcp.rs`) gains a prologue that proves the metric family alive *in
its own server* before its refusal delta is read: deliver one mail to a
per-test throwaway inbox, complete one legal await on it (the mail is
already there, so the wait resolves on its first scan — and 0002's
register-then-scan discipline makes an immediately-resolving wait a counted
one), and assert that inbox's registered delta is exactly **+1**. The
delta-zero on the refusal inbox then means something even if the sibling
`observed` arm dies; each arm is self-contained. The prologue is itself a
0002 witness on the happy path. *Cost of the losing side (land exactly):*
three red tests stay mortgaged to a sibling file — a future rename of the
family defangs them silently while they stay green, the exact vacuity #102
exists to kill — and the alternative the robustness seat offered (record
the dependency in a comment) is *also* an edit, one that documents a
weakness instead of removing it. *Cost of adoption, named for symmetry:*
one more signed commit on the branch (13 → 14) and a gate re-run — both
paid anyway by this run's independent verification. The sharpening is
test-only: no production line moves, no new file, no new env var, no
decision, no bundle edit. **This is the only permitted delta.**

**Conflict ruled — the authority-spec status note: rejected.** A one-line
note on `specs/gate-and-red-tests/spec.md` marking its declaration-bypass
half as 12g's was commission-optional and the batch skipped it; the landing
does not add it. The record lives in the three superseded markers, in this
spec, and — mandatorily — in the PR body, which the framing requires to
summarize "the superseded recognizer specs and run 12g's scope". *Cost of
the losing side (adding the note):* a landing that edits a reviewed spec
file for documentation re-opens "what exactly did the operator verify";
and a reader of the authority spec alone sees unchecked AC-5/AC-6 and a
live-looking recognizer design until 12g's spec supersedes them.

**Recorded — the gate body's imprecise comment, noted, not edited.** The
comment above the export (`scripts/coverage-gate.sh`, "A unique target
directory is stronger than cleaning shared coverage state") is unqualified
by arm; in the explicit arm uniqueness does not hold and the clean after
export is the guarantee — which the very next comment before the clean
states honestly ("A report is candidate-bound only when no instrumented
executable or profile from an earlier source graph can participate in the
merge"). The code is right; the comment is imprecise, not false. Not
edited on a landing. *Cost of the losing side (editing it):* a behavioural
file touched for a comment, re-opening verification for zero behaviour.

**Recorded — where the implementation landed leaner than its spec** (the
return discipline; recorded here, not by editing the authority spec):
`AWAITS_REGISTERED` reuses the existing `Registry` machinery — one static,
one `raise()` at the park site, and a `kind` parameter on
`registry_rows` (`src/api.rs:185`, `:255`, `:784-788`) — where the
authority spec's §10 expected new render machinery; and `RefScanStats` is
a threaded value (`src/extract.rs:47-99`) where §9 expected "two
`#[cfg(test)]` scan atomics". The landed tree has no new statics beyond
the registry itself and no atomics at all. Leaner than spec is the good
direction; the record is here so a future seat does not hunt for the
atomics.

**The scoping rule for reviewers:** report only what this batch changes or
breaks. The declaration-bypass class — #101's fixtures A–G, harness-named
loads via `#[path]`/`include!`/cfg tricks, `coverage(off)` spellings — is
main's standing behaviour and run 12g's scope. Booking it as a residual of
this batch is a scoping error, not a finding. Bar:
`has_security_residual=false`; no residual above medium for what the batch
adds. Anything found beyond the pre-ruled sharpening **stops the landing**
— it is reported back, never absorbed.

## The design — the landing procedure

Five steps, in order; the HOW lives in [plan.md](plan.md), the ordered
work in [tasks.md](tasks.md).

1. **Verify `aba0be2` independently.** All six house gates exactly as the
   house rules state them, with `export PATH="$HOME/.cargo/bin:$PATH"` and
   **no `TMPDIR` override** for the coverage gate, plus
   `bash scripts/coverage-gate.sh selftest`; the no-litter count of
   `swarmail-*` under `/tmp` (including `-wal`/`-shm` sidecars) before and
   after a gate run plus a selftest run. A gate that still needs a
   `TMPDIR` override is a **finding** — the landing stops; it is never a
   fix to apply. Re-read each change against the design authority's
   acceptance, scoped: AC-1–4 and AC-7–16 are this batch's; AC-5/AC-6 are
   12g's.
2. **Prove the red tests can fail.** Apply the three named mutants
   temporarily, observe each red, revert; none committed. Report each by
   name.
3. **Review, scoped.** Diff against `origin/main`, judged on what the
   batch adds; the scoping rule above applies. The frozen guarantees are
   checked by hunk: no change inside `Store::insert`'s
   commit-before-reply path, no reorder of subscription against scan, no
   change to prune eviction, extraction still at ingest.
4. **Land the one pre-ruled sharpening** (B5, the shape above) as a single
   signed commit whose body names it, then re-run all six gates at the tip.
5. **Merge.** PR from `run12b-landing` to `main`, title exactly
   *"Make the tests able to fail and the gate live off its own disk space
   (#101, #102)"*; body summarizing #102's signal-based tests, the #101
   first-half hardening, the superseded recognizer specs and run 12g's
   scope, and reconciling the commit count (14 = the framing's 12 + the
   batch commit + the sharpening). Merge `--squash --delete-branch`; then
   `git fetch origin -q && git checkout main -q && git reset --hard
   origin/main -q`. Report the merge SHA. **No issue is closed and nothing
   is cite-closed** — the operator cites.

## Acceptance Criteria

Each criterion is testable and names its proof.

- [ ] **AC-1 (six gates green, hermetic, no override).** All six house
      gates pass at `aba0be2` exactly as the house rules state them, plus
      `bash scripts/coverage-gate.sh selftest`, with the coverage gate run
      **without any `TMPDIR` override** and reporting exact
      100/100/100. *Proof:* the commands' exit statuses and the gate's
      exact-count line in the run's evidence. A gate needing an override
      fails this criterion and stops the landing.
- [ ] **AC-2 (no `/tmp` litter).** Around one coverage-gate run plus one
      selftest run, the count of `swarmail-*` entries under `/tmp`
      (including `-wal`/`-shm` sidecars) does not grow. *Proof:* the
      before/after `find /tmp -maxdepth 1 -name 'swarmail-*' | wc -l`
      delta is 0.
- [ ] **AC-3 (the red tests can fail).** The three named mutants — the
      chaos read-clone lost update, the removed extract reference window,
      the webhook budget loop `0..4 → 0..5` — are each applied, observed
      red, and reverted; none is committed. *Proof:* each mutant's red run
      recorded by name in the report.
- [ ] **AC-4 (scoped review, clean bar).** The review covers the diff
      against `origin/main` scoped to what the batch adds; it concludes
      `has_security_residual=false` with no residual above medium; the
      declaration-bypass class is reported as run 12g's scope, not a
      residual. *Proof:* the review record in the run notes.
- [ ] **AC-5 (the family proven alive before its refusal deltas).** Each
      of the three delta-zero arms named in the ruling carries the
      aliveness prologue: one legal, immediately-resolving await on a
      per-test throwaway inbox whose registered row rises by exactly +1
      before the refusal delta is read. *Red proof (mutation run, never
      committed):* renaming the family at `src/api.rs:785`, or deleting
      the `raise` at the single park site `src/api.rs:255`, turns all
      three prologues red while the pre-sharpening arms stay green — the
      vacuity edge is closed.
- [ ] **AC-6 (frozen guarantees byte-untouched).** The landing's diff
      contains no hunk inside `Store::insert`'s commit-before-reply path,
      no reorder of subscription against scan at the park site, and no
      change to prune eviction; the counter raise is additive at the
      existing registration site; extraction stays at ingest. *Proof:*
      hunk inspection named in the report, plus the existing 0001/0002/
      0003 assertions staying green.
- [ ] **AC-7 (deviations ledger).** The branch tip is `aba0be2` plus
      exactly one signed commit — the B5 sharpening — and nothing else.
      Any other defect found in verify or review stops the landing and is
      reported; nothing is absorbed silently. *Proof:* `git log` on the
      branch; the squashed diff equals the verified diff plus that commit.
- [ ] **AC-8 (the landing itself).** The PR carries the exact title above
      and a body containing the mandated summary and the commit-count
      reconciliation; it merges `--squash --delete-branch`; the local
      `main` is reset to `origin/main`; the merge SHA is reported; no
      issue is closed or cite-closed. *Proof:* `gh pr view`, the
      post-merge git state, the reported SHA.
- [ ] **AC-9 (gates green at the tip).** After the sharpening commit, all
      six gates pass again and the coverage gate reports exact
      100/100/100 — the sharpening is test-only, so the production report
      does not move. *Proof:* the six commands' exit statuses at the tip.
- [ ] **AC-10 (no new surface).** No decision document, no `bundles/`
      edit, no new env var, no new file under `src/` or `tests/`, no
      dependency beyond the branch's `tempfile`; the sharpening edits only
      `tests/api.rs` and `tests/mcp.rs`. *Proof:* the squashed diff's file
      list.

## Decision ruling — none minted

No change here touches a frozen guarantee, flips a public contract, or
amends a ruling. The batch's metrics family is **additive** — the same
class as `swarmail_webhook_exhausted_total`, landed without a decision —
and its delta-only, per-process contract is already written in the
counter's doc comment; the gate and its environment are tooling; 0020,
0025 §5, 0018 §2, 0030 §1, 0004 and 0033 are applied as written; 0001,
0002, 0003 and 0005 are untouched (AC-6). The design authority ruled the
same way and the next free number stays 0034; both positions concur.
*Cost of the losing side (minting one):* a decision that rules nothing is
vocabulary without a semantic change to carry — noise in the index every
future seat must read. If the operator later wants the registered-counter
contract written up as a ruling, that is an ordinary change with a real
semantic claim, not this landing's.

## What this landing refuses to do

- **Resurrect the recognizer in any size** — not the 694-line lexer, not a
  small arm, not a "cheap" text cross-check. The bypass class is run 12g's
  by the framing; landing-time fixes repeat without 12g's design work the
  exact design the operator ruled out. The three specs stay superseded.
- **Add a knob to the 6 GiB floor** or any new env-var vocabulary — the
  escape valve exists (`CARGO_LLVM_COV_TARGET_DIR`, named in the refusal);
  one owner, one number, one contract.
- **Deliver #101's second half** (the `SF:` report cross-check, fixtures
  A–G) — run 12g's arm 1 by effect.
- **Edit `bundles/`, mint a decision, close or cite-close an issue.**
- **Weaken a frozen guarantee** — none is in play, and the landing's diff
  is checked by hunk to prove it (AC-6).
- **Absorb a finding silently.** Anything unexpected stops the landing and
  is reported in the open; the PR body carries the named findings and the
  commit-count reconciliation.
