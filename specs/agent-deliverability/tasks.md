# Tasks — deliverability for agents (issue #261)

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md) · Decision: [0131](../../docs/decisions/0131-agent-deliverability.md).
Ordered; each task names the test that proves it. Nothing paid runs.

## Phase 1 — MCP contract guards (root, test-only)

- [x] T1. Add an inline `#[cfg(test)]` test in `src/mcp.rs` asserting
  `MODERN_REPORT_DESCRIPTION.contains(crate::deliverability::RULE_SET_ID)`.
  Mutate the literal locally once to see it fail; record that in plan.md's
  implementation notes; revert. **Proof:** `cargo test --locked --lib
  mcp::` (R1.2, AC 2).
- [x] T2. Confirm no root production line changes: `git diff 50095c8 --
  src/ ':!src/static/llms.txt'` shows only the T1 test; `RULE_SET_ID`
  unchanged. **Proof:** existing `tests/mcp_revisions.rs`,
  `tests/spam_rules.rs`, `tests/deliverability.rs` pass unchanged (R1.1,
  AC 1).

## Phase 2 — Published fixtures and the populated-`dns` witness

- [x] T3. Write `tests/fixtures/deliverability-agent/newsletter-failing.eml`
  (subject `AUTUMN LAMP OFFER`, the R3.1 HTML, HTML-only, fixed
  `Date`/`Message-ID`, `From`/`To` on reserved `.example`/`.test` names, no
  `DKIM-Signature`), `newsletter-fixed.eml` (the R4.1 repair: `https` link,
  `alt="Autumn Lamp"`, subject `Autumn lamp offer`) and `README.md`
  (provenance and regeneration rule). **Proof:** T4.
- [x] T4. In `tests/deliverability.rs`, send both fixtures over real SMTP and
  assert the failing report's `link.`/`html.`/`spam.` ids are exactly
  `{link.scheme.http.v1, html.img.alt-missing.v1, spam.subject.all_caps.v1}`
  and the fixed report has none — by rule id only, no full body. **Proof:**
  `cargo test --locked --test deliverability` (R5.3, AC 8).
- [x] T5. In the same file, declare a DNS policy for the test inbox with
  `PUT /api/v1/inboxes/{inbox}/dns` (TXT records for the fixture's domains),
  send the failing fixture, and assert `dns.source == "inbox"`, non-empty
  `dns.lookups`; on 2025-06-18 and 2025-11-25 `structuredContent` ==
  parsed text block == REST; on 2025-03-26 no `structuredContent` and parsed
  text == REST. **Proof:** `cargo test --locked --test deliverability`
  (R1.3, AC 3).

## Phase 3 — Eval catalogue

- [x] T6. `evals/src/tasks.rs`: add `Tier::Deliverability` (not in
  `DEFAULT_TIERS`; `from_str` error names six tiers), `Scenario::NewsletterFindings`
  (`flow()` = `Marketing`, not chaos, doc comment on the reused fact),
  `Scenario::reads_reports()`, the `NEWSLETTER_*` constants of plan.md's
  data model (subject and HTML equal to T3's failing fixture), and the
  `newsletter-findings` task row with the R2.4 goal, appended after
  `confirm-injection`; update the catalogue doc comment. **Proof:** unit
  tests in `tasks.rs` — `tier_ids(Tier::Deliverability) ==
  ["newsletter-findings"]`, `"deliverability".parse::<Tier>()` round-trips,
  the goal names no rule id and contains every R2.4 phrase,
  `validate_catalogue` passes (R2.1–R2.4).
- [x] T7. Regenerate `evals/tests/data/catalogue-v2.json` and the
  full-catalogue hash from `tasks`; change the `5b312d98…` subset filter to
  "default tiers or hard"; assert the first 24 entries unchanged; pin the new
  task's definition hash. **Proof:** `cargo test --locked --test
  provenance` keeps `6068b687…` and `5b312d98…` (R2.5, AC 4).
- [x] T8. An evals test reads `../tests/fixtures/deliverability-agent/newsletter-failing.eml`
  and asserts its decoded subject and HTML equal `NEWSLETTER_SUBJECT` and
  `NEWSLETTER_HTML`. **Proof:** `cargo test --locked` in `evals/` (AC 8).

## Phase 4 — Fixture, runner, mail

- [x] T9. `evals/src/mail.rs`: add `send_stored(...) -> (SendResult,
  Option<String>)` returning the id from the final `250 2.0.0 OK: stored as
  {id}` line; `send` delegates and discards it; `Attempt` unchanged.
  **Proof:** a unit/real-server test that the id equals the id Swarmail lists
  for that mail; existing snapshot tests unchanged (R3.5).
- [x] T10. `evals/src/main.rs` + `runner.rs`: optional `fixture
  --swarmail-http <addr>`; the runner passes the trial's HTTP address only
  when `task.scenario.reads_reports()`. **Proof:** T11's prepare tests; a
  runner test that other tasks' fixture argv carries no
  `--swarmail-http` (R3.7).
- [x] T11. `evals/src/fixture.rs` prepare for `NewsletterFindings`: per-agent
  template state, a `Flow::Marketing` `newsletter` delivery, the report read
  and the R3.2 self-check (non-200 on any failure; no event recorded).
  **Proof:** `evals/tests/fixture.rs` against a real Swarmail — prepare
  succeeds with exactly the three family ids; without `--swarmail-http`, and
  against a closed HTTP port, prepare answers non-200 (R3.2, AC 7).
- [x] T12. Mount `GET`/`POST /newsletter` and list them in `GET /` only for
  this scenario. **Proof:** fixture test — for every other catalogue task,
  `GET /` bytes equal the pre-change bytes (captured as a test constant) and
  `GET /newsletter` is 404 (R3.3, AC 7).
- [x] T13. `POST /newsletter` validation and cap (R3.4), send (R3.5) and
  verdict (R3.6) in the lock order plan.md gives; reply never carries
  findings. **Proof:** fixture tests — each refusal sends nothing and records
  one unaccepted category-less `marketing` event; the 17th re-send answers
  429; a fixed template sets `marketing_completed` with one accepted event;
  an unchanged template records an unaccepted event and no fact; a frozen
  trial answers 409 and records nothing; a subject with `\r\n` is refused.
  The verdict is one pure function of the read's outcome (HTTP status or
  transport error, body); inline unit tests drive its failure arms — 404,
  503, transport error/timeout, invalid JSON, missing `findings` — each to
  "not clean", so no sleep or fault seam is needed (AC 7).

## Phase 5 — Oracle and controls

- [x] T14. `evals/src/oracle.rs`: the R4.1 arm (strict parse; fix by
  `rule_id`; nonzero exit on an unknown family id; new id by "not seen
  before"; MCP re-read; nonzero exit if any family finding remains or the
  call is an error). Unit-test the pure repair helpers. **Proof:**
  `every_task_passes_with_the_oracle` (count 25) (R4.1, AC 5).
- [x] T15. Update `evals/tests/proof.rs` counts 24 → 25. **Proof:**
  `every_task_fails_with_null_and_its_declared_idle_class` — the null agent
  fails `newsletter-findings` with `gave_up` and no fact (R4.2, AC 5).
- [x] T16. Helper modes in `evals/tests/common/mod.rs`:
  `newsletter_unchanged`, `newsletter_partial`, `newsletter_traded`,
  `newsletter_offer_dropped`, `newsletter_cleared`; a proof test runs each
  through the real trial path. **Proof:** each fails `gave_up` with
  `marketing_completed` false in the retained state (R4.3, AC 6).

## Phase 6 — Docs

- [x] T17. `docs/guide/deliverability.md` `## For agents` with every R5.1
  point and a trimmed excerpt; no rule table copied. **Proof:** a
  `tests/deliverability.rs` test pins the section's key strings in the
  guide (R5.1, AC 9).
- [x] T18. `src/static/llms.txt` paragraph inside the existing deliverability
  section (heading untouched). **Proof:** extend the llms expectation list in
  `tests/deliverability.rs`; `tests/api.rs` served-bytes test still passes
  (R5.2, AC 9).
- [x] T19. `evals/README.md` tier/task/matrix/baseline lines (1 task,
  2 controls, 30 paid slots, operator-only) and catalogue count (16 + 3 + 5 +
  1); `docs/decisions/README.md` 0131 row. **Proof:** extend
  `evals/tests/boundary.rs` README phrase pins with `--tier deliverability`
  and the product line (R5.4).

## Phase 7 — Gates

- [x] T20. Run every house gate locked and record outputs in plan.md's
  implementation notes: root fmt/clippy/test/build; evals fmt/clippy/test
  with `SWARMAIL_BIN` = the candidate's `swarmail` from Cargo's artifact
  record; `bash scripts/coverage-gate.sh` and `selftest`;
  `bash scripts/publish-guard-test.sh`; `bash scripts/seal-branch-test.sh`;
  `cargo deny check licenses`. State that the brokkr bundle compile is
  deferred to verify if no brokkr binary is present. Confirm no
  `Cargo.toml`/`Cargo.lock`/`evals/Cargo.lock`/`deny.toml` diff.
  **Proof:** the gate outputs (AC 10, AC 11).
