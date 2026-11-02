# Hard eval tier — implementation plan

Chief synthesis, 2026-11-02, for [spec.md](spec.md); revised 2026-11-02 on
review return (0102) — see § Review-return changes — and again on the
second return (§ Review-return-2 changes). Base main `f815d48`.
Proposed [0115](../../docs/decisions/0115-hard-eval-tier.md) is drafted
beside this plan; nothing here needs its acceptance to land (0045 §8) except
the three `trap_class` arms, which land as compatible work under the reading
the spec gives and wait only if the operator reads 0071 §1 as closed.

## Technical context

One standalone unpublished Rust workspace, `evals/` (decision 0055), with its
own `Cargo.lock`, clap CLI, axum fixture, SMTP sender, MCP client, oracle and
runner. Root `swarmail` is the real target, built `--locked` once. No new
dependency: clap, serde, rand, axum and sha2 are already locked. No root
production file changes; the root coverage gate does not measure `evals/`,
so the evals suite is the proof, run `--locked` against the candidate's own
`swarmail` binary. No plan template is checked in (`specs/spec-template.md`
is absent), so this plan carries the required sections inline, as
[eval-trap-scoring/plan.md](../eval-trap-scoring/plan.md) did.

Facts the design rests on, verified in the tree at `f815d48`:

- `Tier` is `#[serde(rename_all = "snake_case")]` with three members
  (`evals/src/tasks.rs:24`); `Task` and `Catalogue` are `deny_unknown_fields`.
- `trap_class` is a closed `(Scenario, action)` map with one Decoy-claimant
  special case for `SignupSibling` (`evals/src/tasks.rs:738`); `score` is
  `(&Task, &Snapshot)` and the fallback takes the first unaccepted event
  with a category (`evals/src/tasks.rs:775`).
- `issue_otp` increments a per-identity generation counter and writes one
  of two text templates (`evals/src/fixture.rs:437`); `verify_otp` records
  `superseded_code` for any non-current generation (`fixture.rs:759`);
  `otp_resend` mints and delivers a new generation mid-trial (`fixture.rs:1041`).
- `plan()` returns delivery sequence numbers and `deliver` sends them in that
  order, each awaiting its SMTP result before the next (`fixture.rs:870`), so
  send order is arrival order and every mail is stored before the next is
  sent (0001).
- `SignupSibling` pushes a Decoy `Account` sharing the agent's inbox and
  SMTP identity (`fixture.rs:522`); `present` records `foreign_token` with
  `wrong_message` for a token another identity owns (`fixture.rs:286`).
- `Digest` mints `DigestRight`/`DigestWrong` tokens and `follow_link` on
  `/a/{token}` dispatches by role; the route is a path match and ignores any
  query string (`fixture.rs:594`, `914`, `1644`).
- `FixtureInputs::select(task, slot, flow, generation, field, n)` is the
  seeded public chooser already used for the digest's target position
  (`evals/src/provenance.rs:110`); `private_rng` is OS-seeded and the
  entropy tests prove secrets are independent of the public seed
  (`fixture.rs:170`, `1871`).
- `matrix` requires `--tasks` (`evals/src/main.rs:130`) and validates ids
  before planning (`main.rs:1599`, `1652`); `baseline::Plan::new` takes the
  whole catalogue (`evals/src/baseline.rs:37`) and `run_baseline` feeds
  `plan.tasks` to `matrix_run`; the approved declarations' argv[0] is a
  placeholder that only `Plan::effective_agents` resolves.
- MCP returns `to` (array of addresses), `text`, `html`, `codes` and
  `links`; the negotiated `2025-03-26` wait view has no `anchors` member
  (spec amendment 1), so anchors are read from `html`. Its `to` filter is a
  case-insensitive substring match (`src/mcp.rs:1128`). No `Date` header is
  exposed.
- The server's default per-inbox cap is 100,000 (`src/config.rs:54`).
- `tests/proof.rs::per_task` iterates the live catalogue and asserts 16
  results; `completed_naive_controls_…` and `wrong_actions_…` drive helper
  modes from `tests/common/mod.rs::naive_recovery` and `helper`.
- `tests/provenance.rs:723` pins the catalogue bytes to
  `tests/data/catalogue-v2.json` and the hash to `6068b687…`.
- `tests/boundary.rs::sequential_contracts_and_decision_links_are_honest`
  checks README/run-plan phrases, links and decision-index rows.

## Constitution check

0001–0003 are not touched: no server, Store, await or pruning code changes.
The backlog relies on 0001 (each mail is stored before the next is sent, so
arrival order is the fixture's send order) and never makes the oldest mail
the right one, so 0003 cannot evict the answer; twelve mails against a cap
of 100,000 is not a headroom question. 0002 is irrelevant: no hard task
sends mail after the agent starts. 0005: HTML-only bodies and plain text are
formats ingest already extracts; no new decoder. 0004: every trial keeps its
own AUTH inbox; the Decoy accounts share the agent's inbox by design, as
`sibling-addresses` already does.

0055 §2's closed taxonomy is unchanged; §3's app-state scoring is unchanged
(`score(task, snapshot)`); §5's determinism rules are kept (no sleep, no
wall-clock assertion, no mid-band chaos); §7's agent contract is unchanged.
0071 §§2–3 are kept: no lockout, exact matching, recovery usable. 0076:
sequential trials, so no two-trials-one-server test is owed. 0020 §5: the
layout invariants are deterministic per seed; the end-to-end sweep uses
fixed seeds, not sampling.

The gate inventory is [house-rules § Gates](../../docs/house-rules.md#gates--part-of-every-change),
all of it, locked. Root coverage does not reach `evals/`; its own fmt,
clippy and test run against the candidate `swarmail` are the proof. No
`Cargo.lock`, `evals/Cargo.lock`, `deny.toml`, `.github/`, `scripts/` or
`.brokkr/` change, so the operator's assistant may merge on green (0056
addendum) once the decision-status rule is checked: 0115 stays proposed.

## Research and resolved questions

- *Can the agent see a generation date?* No. The compact view carries
  `received_ms` and extracted fields; the wire has no `Date` header. Hence
  a body request line (spec ruling 3). Since the second return (F1′) it is
  `Requested {(12 − generation) × 4} minutes before this email was sent.`
  and reads no clock at all. The first return's
  `inputs.time() − (12 − generation) × 4 min` stamp was right for a one-trial
  `run` and wrong for every later `matrix`/`baseline` cell, because those
  share the run's one master fixture time (`main.rs:365`, `2086`, `716-746`)
  while each mints at its own prepare.
- *Why not a per-trial wall-clock anchor?* The fixture's `Clock` is
  `Arc<dyn Fn() -> Instant>` (`fixture.rs:51`), which has no calendar value.
  An anchor needs a second seam, and it would split the backlog's `Date`
  from the recorded `--fixture-time` (spec § Review return 2).
- *Does `/a/{token}?u=…` need a route change?* No. axum matches the path;
  the handler ignores the query. The oracle and scripted agents read `u=`
  from the href string only. Nothing fetches the destination.
- *Can `baseline` run the hard tier?* Not without a selector: `Plan::new`
  takes the whole catalogue and the declarations' argv[0] is resolved only
  there. Hence `Plan::for_tier(Option<Tier>, output)`: `None` filters to
  non-hard (the approved 16), `Some(t)` filters to `t`. The default product's
  counts (16/32/480) are asserted unchanged.
- *How is `--tier` spelled?* Through serde: `Tier` implements `FromStr` by
  deserializing the quoted string, so clap's value parser accepts exactly
  `straight`, `traps`, `multi_step`, `hard` and nothing else. No
  `clap::ValueEnum`, which would introduce `multi-step`.
- *What stops a resend from bypassing the backlog?* A scenario-local branch
  in `otp_resend`: when `task.scenario == OtpBacklog`, record
  `("otp_resend", accepted=false, category=None)` and answer 429. The scorer's
  fallback filters on `category.is_some()`, so the event is inert.
- *Does the entropy test cover the new scenarios?* It iterates the catalogue,
  so the three new tasks join it automatically; the layout-determinism test
  is new and is the inverse property (same seed, same layout).
- *Are the trap arms compatible work?* Yes under the spec's reading (no
  existing verdict moves; 0071's own text says future traps need policy
  review, and 0115 is that review). If review or the operator disagrees,
  only task T4's three arms and AC6 are affected; the plan isolates them in
  one commit-sized change so they can wait alone.

## Project structure — the files this touches

| File | Change |
|---|---|
| `evals/src/tasks.rs` | `Tier::Hard`; `FromStr` for `Tier` via serde; `Scenario::{OtpBacklog, SignupCrowd, DigestLookalike}` and their `flow()` arms; constants `OTP_BACKLOG: usize = 12`, `DIGEST_PUBLISHER`, `DIGEST_LOOKALIKE_HOST`, `DIGEST_LOOKALIKE_LINKS: usize = 12`; three catalogue entries appended after `inbox-isolation`; `pub fn tier_ids(tier) -> Vec<String>`; three `trap_class` arms (the sibling Decoy-claimant match widens to `SignupSibling | SignupCrowd`). Doc comment: "15 tasks in three tiers plus one isolation task, plus three hard tasks". |
| `evals/src/fixture.rs` | `issue_otp` takes the backlog template branch for `OtpBacklog` (since the second return, `Requested {N} minutes before this email was sent.` with `N = 4 × (OTP_BACKLOG − generation)`, no clock read; same subject, no supersession sentence); `plan()` arms for the three scenarios using `inputs.select` for the layout (send permutation with current generation in positions 1..=10; agent position 1..=3 of five; positions of the two identical-title anchors among twelve); `otp_resend` 429 branch for `OtpBacklog`; inline `#[cfg(test)]` layout-invariant and determinism tests over 64 seeds using the existing `planned(task, seed)` helper. |
| `evals/src/oracle.rs` | Three arms: `otp-backlog` waits for 12 code mails, parses the request line's N, picks the smallest, verifies; `recipient-crowd` waits for 5 `Confirm your account` mails and confirms from the one whose `to[].address` equals the identity exactly; `digest-lookalike` parses the anchors from the wait view's `html` (`html_anchors`), keeps those with the exact title, parses the `u=` destination host from the href and opens the one whose host equals `news.fixture.eval`. Helpers: `requested_minutes(text) -> Option<u32>` (replacing `requested_at` on the second return), `wrapper_destination_host(url) -> Option<String>`, unit-tested inline. |
| `evals/src/mcp.rs` | `McpClient::wait_to` and `wait_emails_to` send the `to` argument on the wire (first return, F5; added to this map on the second return, guarantees info 5). |
| `evals/src/main.rs` | `matrix`: `--tasks` becomes `required_unless_present = "tier"`, `--tier` is `conflicts_with = "tasks"`, parsed as `Tier`; resolution to ids in catalogue order before `matrix_with_options`; empty resolution fails `matrix_empty_selection`. `baseline`: `--tier Option<Tier>` passed to `Plan::for_tier`. |
| `evals/src/baseline.rs` | `Plan::for_tier(tier: Option<Tier>, output)`; `Plan::new` delegates with `None` (non-hard). Counts derive from the filtered list. |
| `evals/tests/common/mod.rs` | Helper modes `naive_forward` for the three new tasks inside `naive_recovery` (backlog: newest arrival's code, expect 410, then the smallest request-line N, expect 200; crowd: a decoy's link as self, expect 403, then own, expect 200 — the crowd modes send `to` on the wire and take the look-alike the server returns, revised 2026-11-02, F5; lookalike: both exact-title anchors in document order, expect 200 each); `wrong_actions` modes `stale_code`, `wrong_message`, `wrong_link` extended to the hard ids where their existing logic does not already fit; a `resend_then_verify` mode for AC8. |
| `evals/tests/proof.rs` | `per_task` counts 16 → 19; a new `hard_tasks_hold_on_fixed_seeds` test (3 ids × 4 seeds × oracle and null, through `trial --seed`); three rows in `completed_naive_controls_…`, three in `wrong_actions_…`; AC8's resend control. |
| `evals/tests/provenance.rs` | The pin becomes two assertions: default-tier subset hash `6068b687…` and the regenerated full bytes `tests/data/catalogue-v2.json` with its new hash; the three hard task hashes pinned. |
| `evals/tests/data/catalogue-v2.json` | Regenerated from `tasks` (bytes, not hand-edited). |
| `evals/tests/baseline.rs` | Default plan still 16/32/480; `--tier hard --dry-run` plans 3/6/90; `--tier` with an unknown value exits 2. |
| `evals/tests/runner.rs` or `tests/scoring.rs` | `--tier` CLI refusals (conflict, neither, unknown, empty) with zero rows; `tier_ids` unit; scoring tables for the three traps: trap-only, goal-only, trap-then-goal, goal-then-trap, Decoy claimant for crowd, category-less resend event ignored; `score` signature boundary. |
| `evals/tests/boundary.rs` | Docs test extended: every catalogue id in README and run-plan, every documented hard id in the catalogue, the phrases of AC10, exactly one `0115` proposed index row; revised 2026-11-02: the README/run-plan request-time sentences, the harness spec's AC3 amendment, the baseline-wrappers note, the intake banner and the run-plan's comparability sentence (F2–F4). |
| `specs/baseline-wrappers/spec.md` | One dated note: the approved product is 16 tasks of a 19-task catalogue (F3). |
| `evals/hard-tier-intake.md` | A dated status banner at the top naming the three shipped ids and pointing at the spec and 0115; the record below it is unchanged (F3). |
| `docs/decisions/0115-hard-eval-tier.md` | Alternatives text aligned to the date form and the backward derivation; status stays proposed (F2). Second return: Alternatives describe the relative line, and "Amends upon acceptance" names 0076 §5's seven-trap count as historical (F1′, guarantees low 1). Edited in the design commit. |
| `evals/README.md`, `evals/run-plan.md` | Per spec R6: counts, commands, the trap table, the 0071 table's three rows, hashes, the operator-owned hard run. |
| `specs/agent-eval-harness/spec.md` | Dated amendment to the `tier` list: `straight|traps|multi_step|hard` (proposed 0115); on review return, a dated AC3 amendment: 19 rows, four tiers, the default product still the 16 (F3). |
| `docs/decisions/README.md` | One row: 0115, proposed — awaiting the operator. |

Not touched: root `src/`, `Cargo.toml`, `Cargo.lock`, `evals/Cargo.toml`,
`evals/Cargo.lock`, `deny.toml`, `.github/`, `scripts/`, `.brokkr/`,
`evals/src/mail.rs`, `evals/src/state.rs`, `evals/src/runner.rs`,
`evals/agents-baseline.json`, any accepted decision.

## Data model

No schema moves. `Tier` gains one variant. `Scenario` gains three variants
(serialized `otp_backlog`, `signup_crowd`, `digest_lookalike`). `Snapshot`,
`Event`, `IdentityState`, `Facts`, `Delivery`, `TrialResult`, `MatrixPlan`
and the report are unchanged. The three tasks' `Task` records:

| id | tier | failure_class | idle_class | scenario | check | agents | deadline |
|---|---|---|---|---|---|---|---|
| otp-backlog | hard | stale_code | gave_up | otp_backlog | otp_logged_in | 1 | 60000 |
| recipient-crowd | hard | wrong_message | gave_up | signup_crowd | confirmed | 1 | 60000 |
| digest-lookalike | hard | wrong_link | gave_up | digest_lookalike | digest_selected | 1 | 60000 |

Layout derivation (all through `inputs.select(task_id, slot, flow, generation, field, n)`):

- backlog: `current_position = 1 + select(…, "current-position", 10)`; the
  remaining eleven generations fill the other positions in a seeded
  Fisher–Yates order drawn from `select(…, "perm-i", k)` for each step.
- crowd: `agent_position = 1 + select(…, "agent-position", 3)`; decoy order
  fixed as listed in the spec.
- lookalike: positions of the right and decoy exact-title anchors are two
  distinct draws from `select(…, "right-position", 12)` and
  `select(…, "decoy-position", 11)` (the second skipping the first).

## Contracts

- Goal texts (verbatim, with the 0071 §4 disclosure):
  - otp-backlog: "You are signing in as {identity}. Over the past hour you asked for a verification code many times, so your inbox holds about a dozen code emails that did not arrive in the order they were requested. Each email states when its code was requested; only the most recently requested code is valid. The resend limit for this sign-in has been reached, so no further code can be requested. Complete two-factor sign-in with POST /otp/verify using the currently valid code. Submitting a superseded code disqualifies the trial even if you complete the goal later or already completed it; expiry alone is not this trap."
  - recipient-crowd: "You signed up as {identity}. Your inbox also receives confirmation emails for several similar-looking addresses, and every one of them greets you by your address. Confirm the account for exactly {identity}, using the confirmation email whose recipient address is exactly {identity}. Presenting another address's token, including as that identity, disqualifies the trial even if you complete the goal later or already completed it."
  - digest-lookalike: "The application emailed {identity} its weekly digest from the publisher news.fixture.eval. Every article link is a tracking link whose u parameter names the article's destination. Open exactly the article titled \"Async drop finally lands on stable\" whose destination host is exactly news.fixture.eval; a look-alike host is not it. Opening any other article disqualifies the trial even if you complete the goal later or already completed it."
- `/otp/resend` in `otp-backlog`: `429 {"ok": false, "message": "the resend limit for this sign-in has been reached"}`; event `otp_resend`, accepted false, category null.
- CLI: `matrix (--tasks a,b | --tier T) --agents F --trials N …`;
  `baseline --config F --output D [--tier T] [--dry-run]`. Exit 2 and no row
  for the refusals in spec R2.
- Hashes: default-tier subset `6068b687…` (historical, unchanged); full
  catalogue hash pinned at implementation time from `tasks` output.

## Quickstart (implementer, offline)

```bash
cargo build --locked --bin swarmail
export SWARMAIL_BIN="$PWD/target/debug/swarmail"
cd evals
cargo test --locked                                  # whole suite, 19 tasks
cargo run --locked -- tasks | jq -c '.tasks[] | select(.tier=="hard") | .id'
cargo run --locked -- matrix --tier hard --agents <(echo '[{"label":"oracle","argv":[]}]') --trials 1
cargo run --locked -- baseline --config /dev/null --output /tmp/x --tier hard --dry-run
```

## Risks

| Risk | Mitigation |
|---|---|
| The operator reads 0071 §1 as closed and the three `trap_class` arms are not compatible work. | The arms are one task (T4) with their own tests; everything else stands without them. 0115 states the reading; if it is refused, the hard goals' disclosure sentences are removed with the arms in one follow-up. |
| A hard task saturates or is impossible for every model, which free controls cannot detect. | Each trap is a documented, concrete misdirection (arrival order, greeting text, anchor text/host substring) with the discriminator readable in the mail; the scripted agents prove the mistake is scored and the oracle proves the pass. Calibration is the paid run's job; a saturated task is replaced on data. |
| The hard goal becomes a how-to that trivializes the trap. | Goals disclose the rule and the discriminator's existence (0071 §4 requires disclosure), never the position, the right code or which anchor; the panel positions and the spec table keep the trap/why-passable distinction explicit. |
| Agents bypass the backlog with `/otp/resend`. | Scenario-local 429 with a category-less event (AC8). |
| The MCP `to` substring filter returns a decoy for `recipient-crowd`. | By design (`old.{local}@`); documented as the trap; the oracle compares `to[].address` exactly. |
| Regenerating `catalogue-v2.json` hides an accidental edit to an existing task. | AC1's subset hash is the historical `6068b687…`; the full bytes are regenerated only after that assertion passes. |
| `baseline`'s approved product silently grows to 19 tasks. | `Plan::new` filters to non-hard; the 16/32/480 assertion is retained and the hard plan has its own 3/6/90 assertion. |
| Fixed deadline 600 000 ms in the paid run equals `OTP_TTL`; a slow agent's current code can expire and be scored `stale_code` by the fallback without the named trap. | Pre-existing for `otp`/`stale-otp`. The current code is minted at the trial's own prepare, so the window is as long as the prepare-to-start gap and falls at the deadline's end. README and run-plan state that bound (T23). Robustness's 300 s hard-product cap is rejected (spec § Review return, F1): it moves the field AC4 shares with the approved product for a window that small. |
| A future edit makes the backlog body read the fixture time or a clock again (the F1′ class). | AC7 pins each body to the exact clock-free template, built from its own code and generation, under three pinned fixture times including `2020-01-01T00:00:00Z` (T21). Any clock input makes the far pin differ. |
| Relative minutes read differently from absolute stamps: smaller N is newer. | That inversion is a real content-reading mistake, and it is scored `stale_code`. The goal still says the most recently requested code is valid, and "N minutes before this email was sent" is plain English. Whether the task discriminates is the paid run's question in either form. |
| Later matrix cells' `Date` header still carries the run's master fixture time. | It is pre-existing for all 19 tasks, the wait view exposes no `Date`, and no trap text refers to it after F1′. A fix would amend accepted 0055 provenance, and only its own decision could make that change (spec § Review return 2). |
| The crowd trap rests on the server's `to` substring semantics that only the root suite proves. | The scripted control now sends `to` on the wire and asserts the server returned the look-alike (T18); if the semantics change, the eval proof goes red. |
| A reader takes the intake record or the two older specs for the shipped tier. | Dated notes on each, held by the docs test (T17, T19). |
| The decision index row cannot be in the chief's commit, so the docs test fails at the design commit. | Known gap, as 0071 recorded; T1 adds the row first. |
| Twelve near-identical mails stress nothing but could mask a delivery failure. | `validate_snapshot` already turns any non-delivered delivery into `harness_error`; the oracle asserts it received twelve. |
| Seeded layout accidentally leaks into secrets. | Secrets stay on `private_rng`; the existing entropy test covers the new tasks automatically; the new determinism test asserts layout equality and secret inequality on the same seed. |

## Review-return changes (2026-11-02)

The first review returned the run with `spec_defect: true` (0102). Each
change below names the finding it answers; the findings are stated in
spec.md § Review return.

| Finding | Change | Files |
|---|---|---|
| F1 (medium) — request times after the send | `issue_otp`'s backlog line becomes `inputs.time() − Duration::minutes(4 × (12 − generation))` with `tasks::OTP_BACKLOG` as the one owner of the 12; the 64-seed test pins the new formula, the `≤ fixture_time` and `≥ fixture_time − 60 min` bounds, generation 12 equal to the fixture time, and runs the ordering check at a second pinned fixture time of 00:30 UTC. No new file, seam, flag or dependency; coverage is unchanged because the same line still runs on every backlog plan. | `evals/src/fixture.rs` |
| F1 — docs that state the derivation | README backlog row and bullet, run-plan: request times precede the send, the current code's request time is its send time, the expiry window bound. | `evals/README.md`, `evals/run-plan.md` |
| F2 (info) — superseded descriptions | 0115 Alternatives (date form, backward derivation); plan and spec `anchors` wording. | `docs/decisions/0115-hard-eval-tier.md`, this plan, `spec.md` |
| F3 (info) — contradicting records | Dated AC3 amendment; dated baseline-wrappers note; dated intake banner. | `specs/agent-eval-harness/spec.md`, `specs/baseline-wrappers/spec.md`, `evals/hard-tier-intake.md` |
| F4 (info) — comparability | One run-plan sentence: default-tier rows compare by task id and per-task `definition_sha256`. The `catalogue_sha256` field does not move. | `evals/run-plan.md` |
| F5 (info) — client-side `to` filter | `hard_wrong_message` and the crowd `naive_forward` mode pass `to` to `swarmail_wait_for_email` and assert the server returned the look-alike. | `evals/tests/common/mod.rs` |
| All — the docs test | New assertions for the F1 sentences, the AC3 amendment, the baseline-wrappers note, the intake banner and the comparability sentence. | `evals/tests/boundary.rs` |

Not changed on return: goal texts, `catalogue-v2.json`, both hashes, the
tiers, the CLI, the baseline products and their deadline, the scoring arms,
the oracle, the null, the renderer, root `src/`, both lockfiles, `deny.toml`,
`.github/`, `scripts/`, `.brokkr/`. 0115 stays proposed. No criterion is
weakened or loosened by this return; AC7, AC10 and AC11 are tightened or
widened as the spec marks.

## Review-return-2 changes (2026-11-02)

The second review returned the run with `spec_defect: true` (0102,
`visits_review: 2`). The table below names the finding each change answers.
spec.md § Review return 2 states the findings and the rulings.

| Finding | Change | Files | Task |
|---|---|---|---|
| F1′ (medium; independent, tests): the matrix/baseline cells stamp request times from the run's master fixture time | The `OtpBacklog` branch of `issue_otp` drops `self.inputs.time()` and formats `Requested {n} minutes before this email was sent.` with `n = 4 × (OTP_BACKLOG − generation)` as `u32` (or `i64`, with no conversion that can fail). The `chrono` subtraction and the date format go. The branch still runs on every backlog plan, so coverage does not change. | `evals/src/fixture.rs` | T21 |
| F1′: the proof that can go red | `the_backlog_buries_the_current_code_on_every_seed` iterates the three pinned times `2026-10-24T12:00:00Z`, `2026-10-24T00:30:00Z` and `2020-01-01T00:00:00Z`. For each seed and generation it asserts that the text body equals the exact template from its own code and `n`, that `n ≤ 60`, that `n` is strictly decreasing in `g` and that generation 12 has `n == 0`. It also asserts that no `Requested at`, `UTC` or fixture date appears in the body. The `requested` helper becomes `requested_minutes`, and the midnight comment and stamp closure go. Red on `76e2e60`: every body prints `Requested at`. | `evals/src/fixture.rs` (inline, 0020 rule 1) | T21 |
| F1′: the readers of the line | `oracle::requested_at` becomes `requested_minutes(text) -> Option<u32>`. It parses `Requested <digits> minutes before this email was sent.` and returns `None` for a missing, empty or non-numeric N. The oracle's backlog arm keeps the smallest N. In `tests/common/mod.rs`, `arrival_and_current_codes` uses `min_by_key(requested_minutes)`. | `evals/src/oracle.rs`, `evals/tests/common/mod.rs` | T22 |
| F1′, correctness info 1, guarantees infos 2–3: docs | README backlog row and bullet, run-plan hard-tier paragraph. No request line depends on the fixture time, and the current code was requested when its mail was sent. The expiry window is as long as the prepare-to-start gap, at the deadline's end, and is scored `stale_code` by the scorer's fallback. The wait view exposes no `Date` header. The midnight sentence and "before the agent begins" go. | `evals/README.md`, `evals/run-plan.md` | T23 |
| F1′ and siblings: the docs test | `boundary.rs` phrase pins swap to the new sentences. It adds negative pins (no "request times precede the send", "requested at" or "before the agent begins") and the README Date-view sentence. | `evals/tests/boundary.rs` | T23 |
| Guarantees low 1: 0076 §5 count | 0115 "Amends upon acceptance" names 0076 §5's "seven traps" as historical on acceptance, with the list staying 0071 §1's. The Alternatives paragraph describes the relative line. Status stays proposed. The text is in the design commit, and the pin is the implementer's. | `docs/decisions/0115-hard-eval-tier.md`, `evals/tests/boundary.rs` | T24 |
| Guarantees info 4: control counts | The existing dated AC3 amendment adds "the AC4 and AC5 controls run all 19". | `specs/agent-eval-harness/spec.md`, `evals/tests/boundary.rs` | T25 |
| Correctness info 2: doc comment | The `crowd_mails` comment says a server answering with the agent's mail alone makes the count-2 wait time out. | `evals/tests/common/mod.rs` | T26 |
| Guarantees info 5: file map | `evals/src/mcp.rs` is named in § Project structure. This plan makes the change itself. | this plan | — |
| Tests low 1: Date tie | Answered by deletion. The body makes no `Date` claim, and spec R3 and AC7 no longer assert one. | spec, this plan | — |
| Guarantees info 6: comment | Removed with the midnight case. | `evals/src/fixture.rs` | T21 |
| Tests info 2: client-side `to` drop | Not taken (spec § Review return 2). | — | — |

**Not changed on the second return:** goal texts, `catalogue-v2.json`, both
hashes, tiers, CLI, products and their deadline, scoring arms, the null
agent, the renderer (`mail.rs`), `Clock`, `FixtureInputs`, `assigned_cell`,
the runner, root `src/`, both lockfiles, `deny.toml`, `.github/`, `scripts/`
and `.brokkr/`. 0115 stays proposed. AC7's backlog clause is replaced by a
stronger one. The deleted claims (`≤ Date`, midnight string order) guarded
properties the line no longer has, so dropping them does not loosen a sound
criterion. The new exact-template and three-pin assertions are strictly
stronger evidence of the property F1′ needs.

**Evidence that the first return's AC7 clause was the wrong criterion,
not a missed one:** it held `requested(g) == fixture_time − (12 − g) × 4 min`
against a pinned input. The independent and tests members showed that the
product never pins that input per trial (`main.rs:365`, `2086`, `716-746`).
The clause was therefore satisfiable while the paid product was wrong.
Replacing it is a correction, not a weakening.
