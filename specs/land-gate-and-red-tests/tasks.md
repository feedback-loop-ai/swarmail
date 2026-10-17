# Tasks — land-gate-and-red-tests (issues #101, #102)

Ordered. Verify before review, review before sharpening, sharpen before
landing; nothing lands that was not verified. Each task names the test
that proves it. The red proofs in T3 and T5 are recorded mutation runs —
observed red, then reverted, never committed. Any red gate, any defect
beyond the pre-ruled sharpening, **stops the landing and is reported**;
nothing is absorbed silently.

- [ ] **T1 — Verify the batch at `aba0be2` independently.** Run all six
      house gates exactly as the house rules state them (with
      `export PATH="$HOME/.cargo/bin:$PATH"`), plus
      `bash scripts/coverage-gate.sh selftest`, and the coverage gate
      **with no `TMPDIR` override**. Re-read each change against the
      design authority's acceptance, scoped per the spec (AC-1–4, AC-7–16
      in scope; AC-5/AC-6 are 12g's). A gate needing an override is a
      finding that stops the landing.
      *Test:* all seven commands exit 0; the coverage gate prints exact
      100/100/100; the selftest's hardening arms (explicit dir verbatim,
      default under `target/coverage/` ignoring `TMPDIR`, the refusal
      naming path/filesystem/escape variable, unstatable path, small
      floor) all green (AC-1).
- [ ] **T2 — No-litter proof.** Count `swarmail-*` entries under `/tmp`
      (including `-wal`/`-shm` sidecars) before and after one coverage
      gate run plus one selftest run.
      *Test:* the before/after `find /tmp -maxdepth 1 -name 'swarmail-*'
      | wc -l` delta is 0 (AC-2).
- [ ] **T3 — Prove the red tests can fail.** Apply the three named
      mutants one at a time, observe each red, revert; commit none: (a)
      the chaos read-clone lost update (`src/chaos.rs`), (b) the removed
      extract reference window (`src/extract.rs`), (c) the webhook budget
      loop `0..4 → 0..5` (`src/webhook.rs`).
      *Test:* each mutant's red run recorded by name in the report; the
      tree back at `aba0be2` afterwards (`git status` clean) (AC-3).
- [ ] **T4 — Review, scoped.** Diff against `origin/main`, judged only on
      what the batch adds. Name the frozen-guarantee hunks (0001/0002/
      0003/0005 untouched per the plan's constitution check); apply the
      scoping rule — the declaration-bypass class is run 12g's, never a
      residual of this batch; note the pre-export comment imprecision
      without editing it. Bar: `has_security_residual=false`, no residual
      above medium.
      *Test:* the review record states the bar's conclusion, the scoped
      findings, and the named hunks (AC-4, AC-6).
- [ ] **T5 — Land the B5 sharpening (the only permitted delta).** In each
      of the three delta-zero arms —
      `the_wait_refuses_over_the_cap_registers_no_waiter_and_accepts_the_boundary`
      (`tests/api.rs`),
      `the_wait_registers_no_waiter_when_staging_refuses` and
      `the_staging_refuses_over_the_cap_and_honors_the_boundary`
      (`tests/mcp.rs`) — add, before the refusal delta is read: deliver
      one mail to a per-test throwaway inbox via SMTP; complete one legal
      await on it through the arm's own plane (it resolves immediately —
      the mail is already there, and 0002's register-then-scan makes an
      immediately-resolving wait a counted one); assert that inbox's row
      `swarmail_awaits_registered_total{inbox=<alive>}` delta is exactly
      +1 (`unwrap_or(0) == 1` is red when the family is absent, renamed
      or never raised). Commit as one signed commit citing #102.
      *Test (mutation run, never committed):* renaming the family at
      `src/api.rs:785` — or deleting the `raise` at the single park site
      `src/api.rs:255` — turns all three prologues red while the
      pre-sharpening arms would stay green; revert the mutant; `cargo
      test` green at the tip (AC-5).
- [ ] **T6 — Re-gate at the tip.** Run all six house gates plus the
      selftest again after the sharpening commit.
      *Test:* all seven commands exit 0; the coverage gate still reports
      exact 100/100/100 — the sharpening is test-only, so the production
      report does not move (AC-9).
- [ ] **T7 — Merge and report.** Create the PR `run12b-landing` → `main`
      with the exact title *"Make the tests able to fail and the gate
      live off its own disk space (#101, #102)"* and a body that
      summarizes #102's signal-based tests, the #101 first-half
      hardening, the superseded recognizer specs and run 12g's scope, and
      reconciles the commit count (14 = the framing's 12 + the batch
      commit + the sharpening), listing the mutants run and any named
      findings. Merge `--squash --delete-branch`; then
      `git fetch origin -q && git checkout main -q && git reset --hard
      origin/main -q`. Report the merge SHA. Close no issue; cite-close
      nothing.
      *Test:* `gh pr view` shows the exact title and the mandated body
      sections; the post-merge local `main` is at `origin/main` with the
      reported SHA; no issue state changed (AC-7, AC-8, AC-10).
