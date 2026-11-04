# Expert eval tier — ordered tasks

Chief synthesis, 2026-11-04. AC numbers refer to [spec.md](spec.md); file
and contract detail is in [plan.md](plan.md). Each task is paired with the
test that proves it. A task is done when that test is red before the change
and green after. Nothing paid runs in any task.

- [ ] **T1: record the proposal.** Add the 0123 row ("proposed — awaiting
  the operator") to `docs/decisions/README.md`. Add the dated tier-list
  amendment to `specs/agent-eval-harness/spec.md`
  (`straight|traps|multi_step|hard|expert`; "24 catalogue rows in five tiers;
  the default product is the 16"). **Proof:** `tests/boundary.rs` asserts
  exactly one `| [0123]` index row containing `proposed`, and the amended
  tier list. No accepted record's text or status changes (AC15).
- [ ] **T2: add the tier, and pin the existing bytes.**
  - Add `Tier::Expert` and `DEFAULT_TIERS`, and update `from_str`'s error
    text.
  - Make every "default tier" filter positive: `baseline.rs:49`,
    `tests/baseline.rs:46`, `tests/provenance.rs:743` and
    `tests/boundary.rs:266/298`.
  - Pin both subset hashes: `6068b687…` for the default tiers and
    `5b312d98…` for default plus hard.
  - Assert that the first nineteen entries are unchanged and keep the three
    per-hard-task hashes.
  - Regenerate `tests/data/catalogue-v2.json` and the full-catalogue pin
    from `tasks`.

  **Proof:** `tests/provenance.rs` and the `tests/scoring.rs` tier tests
  (`"expert"` parses; `Expert`, `experts` and `expert-tier` refuse in both
  serde and `FromStr`). They go red if any existing byte moves (AC1, AC2).
- [ ] **T3: add the five scenarios, tasks and the difficulty table.**
  - Add the five `Scenario` variants and their `flow()` arms, the
    constants, and the five `task(...)` entries with the goals verbatim
    from spec § Goals.
  - Add `Family`, `Difficulty` and `EXPERT_DIFFICULTY` with spec's proxy
    values.

  **Proof:** the `tests/scoring.rs` catalogue test checks five `expert`
  entries after `digest-lookalike`, in order, with their fields; each goal
  contains `disqualifies the trial`; `validate_catalogue` passes;
  `tier_ids(Expert)` returns the five ids. A table test checks one row per
  expert id and none for other tiers, a non-empty trap and mistake,
  `expected_class == failure_class`, and at least four distinct families
  (AC4, AC5).
- [ ] **T4: add the five named-trap rows.** Add the `trap_class` arms of
  plan § Project structure. This is the only task that depends on 0123's
  reading of 0071 §1. It is kept separate so it can wait alone. **Proof:**
  `tests/scoring.rs` tables for each scenario:
  - trap only fails with the class;
  - trap then goal, and goal then trap, both fail with the class;
  - goal only passes;
  - a wrong action, flow or class traps nothing;
  - the same event under a non-expert scenario does not trap through the
    new row (AC9).
- [ ] **T5: build `otp-correction`.**
  - Add `issue_otp_with`, the prepare arm per plan § Contracts, and the
    scenario's 429 resend.
  - **Proof:** inline `fixture.rs` tests over 64 seeds check that:
    - there are two OTP deliveries, mail 1 with the code template and mail
      2 with the correction subject;
    - mail 1's only 4–10-digit run is generation 1's code;
    - mail 2 has no run of four or more digits and names the old and new
      digit pairs;
    - neither mail contains the current code, and
      `X[..4] + new == current`.
  - **Proof:** handler tests check that `X` records `superseded_code` (410)
    and the corrected code verifies (200). They also check that resend
    answers 429 with an unaccepted, category-less event and no new
    delivery, while `otp` and `stale-otp` resends are unchanged (AC6,
    AC13).
- [ ] **T6: build `digest-correction`.**
  - Add the prepare arm per plan § Contracts.
  - **Proof:** inline tests over 64 seeds check that:
    - there are two Digest deliveries; the digest has 13 anchors and
      exactly two with the exact title, one per section, and exactly one
      of them is `DigestRight`;
    - the correction has no `http` and names the section that holds the
      `DigestWrong` exact-title link;
    - both section choices occur across the seeds, and the same seed gives
      the same layout.
  - **Proof:** a handler test checks that the retracted link records
    `digest_decoy`/`wrong_link` and the right one sets `digest_selected`
    (AC6).
- [ ] **T7: build `otp-late`.**
  - Add the prepare arm with the restart, `last_mailed_code`, and the
    scenario's resend per plan § Contracts.
  - **Proof:** inline tests check that:
    - prepare delivers two code-template mails, neither with the current
      code, and both codes record `superseded_code`;
    - after one resend there are two new deliveries in the order fresh then
      reminder, the fresh one carries the current code, and the reminder
      carries generation 2's code under `Reminder: finish signing in`;
    - after a second resend the reminder carries the first resend's code
      and the newest fresh code verifies;
    - `/deliveries/release` releases nothing (AC6, AC13).
- [ ] **T8: build `confirm-attachment` and `confirm-injection`.**
  - Add the two prepare arms per plan § Contracts.
  - **Proof:** inline tests over 64 seeds check that:
    - attachment: one `Mixed` delivery; the body's only URL is a
      `DecoyLink`; the decoded attachment holds the `Confirm` link and the
      body does not;
    - injection: two Signup deliveries, the `Re:` mail newest, its only URL
      a `DecoyLink`, and no confirm link in it.
  - **Proof:** an all-expert test over 64 seeds checks that every URL in
    every mail and attachment starts with the app URL, and that no mail
    contains a backtick, `$ `, `sudo`, `curl`, `/etc/`, `~/`, `password` or
    an `@` host other than the fixture's own addresses (AC6, AC14).
- [ ] **T9: teach the oracle and count its cost.**
  - Add the five oracle arms per plan § Contracts.
  - Add `correction_digits` and `retracted_section`.
  - Add the counters: `McpClient::tool_calls` and the `App` request
    counter.
  - Add `run_oracle_costed`, with `run_oracle` delegating to it unchanged.
  - Add `McpClient::get_attachment`.
  - **Proof:** inline unit tests for both helpers (the fixture's wording,
    a missing line, extra digits, the wrong section name). A counter test
    checks that `initialize` and the notification do not count and each
    `call` counts once. T10's real-process sweep is the end-to-end proof
    (AC10, AC11).
- [ ] **T10: oracle, null and cost on fixed seeds.**
  - Add `expert_tasks_hold_on_fixed_seeds`. It runs each expert id on the
    four fixed seeds through `trial --seed` and checks:
    - the oracle passes, every delivery is `delivered`, and `check` exits 0;
    - null fails with `gave_up` and no goal fact;
    - the `expert_cost` helper's written cost equals `EXPERT_DIFFICULTY`.
  - `per_task` and the recipe-path control cover all 24 tasks.
  - **Proof:** `tests/proof.rs` green (AC10, AC11).
- [ ] **T11: targeted mistakes, shortcuts and visibility.** Add the helper
  modes of plan § Project structure. **Proof:** `tests/proof.rs`:
  - `wrong_actions_fail_in_their_observable_classes` gains six rows: both
    `otp-late` mistakes and one per other task.
  - `completed_naive_controls_recover_the_goal_but_lose_the_trial` gains
    five rows (trap action, flow, count 1, class, `check` agrees).
  - `expert_uncorrected` asserts that `swarmail_wait_for_code` selects `X`,
    and `expert_reminder` that it selects the reminder's code after resend.
  - `expert_body_link` asserts that `swarmail_wait_for_link` selects the
    decoy, and that the wait view's `text`, `html`, `links` and `anchors`
    lack the confirm link while `get_attachment` has it.
  - `expert_double_resend` resends twice and then passes (AC7, AC8, AC9,
    AC13).
- [ ] **T12: selection and products.** Add `matrix` and
  `baseline --tier expert`. **Proof:**
  - `tests/report.rs`: `matrix --tier expert` with oracle and null emits
    the five ids in order (5 pass, 5 `gave_up`), and
    `--tier expert --tasks otp` exits 2 with empty stdout.
  - `tests/baseline.rs`: the default dry run is still 16/32/480 with the
    same task order; `--tier expert --dry-run` is 5/10/150 with the same
    deadline, concurrency, steps and files; and the expert controls run
    for real before any paid launch, mirroring the hard test (AC12).
- [ ] **T13: documentation.** Update `evals/README.md` and
  `evals/run-plan.md`:
  - the expert section: tier name and reason, the five-row trap table, the
    proxy table, the 0071 table's five rows, and the cut families with the
    budget derivation `C = n + max(2, ⌈n/2⌉)`;
  - `--tier expert` for both commands, the 150-slot product, the
    statistical-power note, the expiry note, and that the paid expert run
    is the operator's.

  **Proof:** `tests/boundary.rs` checks that every expert id is in both
  files, that every id a document marks `expert` is an expert task, that
  each trap's action and class pair is present, and that the anchor
  phrases are there (AC15).
- [ ] **T14: run every house gate and commit.** Run the full inventory in
  [house-rules § Gates](../../docs/house-rules.md#gates--part-of-every-change),
  locked:
  - `cargo fmt`, `clippy`, `test` and `build --bin swarmail`;
  - the evals fmt, clippy and test against the candidate's own `swarmail`
    from Cargo's artifact record;
  - `brokkr compile` for every `.brokkr/recipes/*/`;
  - `scripts/coverage-gate.sh` and its `selftest`;
  - `publish-guard-test.sh`, `seal-branch-test.sh` and
    `cargo deny check licenses`.

  Confirm there is no diff in either lockfile, `deny.toml`, `.github/`,
  `scripts/`, `.brokkr/`, the toolchain or root `src/`. No load proof is
  needed. If one is ever added, cap it at nproc/4 workers for 10 minutes,
  name it and time it. **Proof:** every command exits 0. `git diff --stat
  main` names only the files in plan § Project structure and this design.
  Commit unsigned and never push. Report `complete` only when all of it is
  green (AC16).
