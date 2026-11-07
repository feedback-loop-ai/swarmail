# 0131 — Deliverability for agents: the shipped structured report, a repair eval task, docs and fixtures

- **Status:** proposed
- **Date:** 2026-11-07
- **Scope:** Epic #251 phase 3, issue #261 (P2). Run
  `issue-261-deliverability-for-age-f23c2fc5`, `claude-careful`, from `main`
  `50095c8` (which carries #253, #254, #255, #258 and #260). The dispatcher
  assigned the number 0131. No other number is used.
- **Specification:** [agent deliverability](../../specs/agent-deliverability/spec.md);
  [plan](../../specs/agent-deliverability/plan.md), [tasks](../../specs/agent-deliverability/tasks.md).
- **Authority:** accepted [0085](0085-deliverability-report.md) (§6's deferred
  structured output), accepted [0105](0105-mcp-report-output.md) (which
  delivered it), accepted [0055](0055-agent-eval-harness.md),
  [0071](0071-eval-trap-scoring.md), [0117](0117-hard-eval-tier.md),
  [0119](0119-eval-brokkr-recipe.md) and [0123](0123-expert-eval-tier.md) (the
  eval harness, its trap and scoring rules and its tiers), accepted
  [0121](0121-deliverability-link-checks.md) §5 and
  [0122](0122-deterministic-spam-rules.md) §1.4 (the `rule_set_id` rule).
- **Amends upon acceptance:** the agent eval harness specification's `tier`
  list (`straight|traps|multi_step|hard|expert` gains `deliverability`), as
  0117 and 0123 each did. That list is a specification, not the ruling text
  of an accepted decision; an added tier value is additive under accepted
  0071 §4 as accepted 0117 reads it ("preserving existing memberships").
- **Holds:** none. No accepted record's ruling text changes; nothing here
  contradicts an accepted ruling, so the implementation may land while this
  record is proposed (accepted 0045 §8). No `Cargo.toml`, `Cargo.lock`,
  `evals/Cargo.lock` or `deny.toml` change; no dependency.
- **Preserves:** 0001–0005; accepted 0085 (report shape
  `{rule_set_id, findings, dns}`, bounds, laziness, the March text block);
  accepted 0105 (one output-schema owner; modern `structuredContent` is the
  same value as the text block; March has neither schema nor
  `structuredContent`); accepted 0055 (closed eight-class taxonomy, oracle and
  null proofs, app-state scoring, standalone `evals/`); accepted 0071 §1 (no
  trap row added), §§2–6; accepted 0076 (sequential trials); accepted 0115
  (trial isolation); accepted 0117 (the hard tier keeps its three tasks, its
  bytes and its product); accepted 0123 (the expert tier keeps its five);
  the approved default product `6068b687…` and the recorded
  default-plus-hard catalogue `5b312d98…`; `RULE_SET_ID` `swarmail-html/7`.

## Context

Issue #261 asks for three things: expose the deliverability report to agents
as structured MCP output (the outputSchema / structuredContent follow-up 0085
§6 deferred), add an eval task in which an agent repairs a failing mail from
the report alone, and publish agent-facing docs and fixtures.

On `main` `50095c8` the first item has already shipped. Accepted 0105 (#153,
accepted 2026-11-02) gave `swarmail_get_deliverability_report` its closed
`outputSchema` in the one `OUTPUT_SCHEMAS` owner (`src/mcp.rs`), and
`text_result` sends the same `Value` as `structuredContent` and as the JSON
text block on the 2025-06-18 and 2025-11-25 revisions, while 2025-03-26 keeps
the text block alone, byte for byte. Witnesses: `tests/spam_rules.rs`
(modern `structuredContent` equals REST; March has none and its text equals
REST), `tests/mcp_revisions/structured.rs`, and the llms.txt pin in
`tests/deliverability.rs`.

The design panel argued two positions. Simplicity: no production change, one
task in a new additive tier, reuse the marketing vocabulary, let Swarmail's
own report be the verdict. Robustness: the same discharge, plus a drift guard
for the served description, a populated-`dns` equality witness, a baseline
self-check, "availability is not cleanliness", per-trial isolation and five
free controls, but the task in the `hard` tier. This record rules between
them.

## Ruling

1. **The MCP follow-up is discharged by accepted 0105.** No new schema, tool,
   resource, prompt, rendering or serializer. The report object stays
   `{rule_set_id, findings, dns}`; the text block stays the compact JSON bytes
   on every revision; March gains nothing. Two test-only additions guard the
   contract: a unit test that the served modern report description names
   `crate::deliverability::RULE_SET_ID` (today a hand-written
   `swarmail-html/7`, which a rule-set bump would otherwise leave stale), and
   a real-server witness, on a mail whose report has a populated `dns`
   member, that modern `structuredContent` equals the parsed text block and
   REST, and that the March text equals REST. 0085 is not edited: 0105's
   header already says it supplements 0085's deferred output, and this
   record links both.

2. **No report byte changes, so no `rule_set_id`.** Under 0121 §5 and 0122
   §1.4 the id is a version trigger, taken by the run whose change moves the
   bytes. #261 moves none and takes none; it reserves nothing on #337's
   behalf, because #337 takes its own next never-shipped id under the same
   rule. No fixture, doc example or eval check pins a full report body or a
   literal `rule_set_id`, so #337's `weight`, `total` and id land without
   touching this work, except the description guard of §1, which is meant to
   fail if #337 bumps the constant without updating the served text.

3. **One eval task, `newsletter-findings`, in a new tier `deliverability`.**
   - *Tier.* `deliverability` is an additive tier value outside
     `DEFAULT_TIERS`, appended after `confirm-injection`. It names the
     capability measured: acting on a deliverability report. `hard` is
     refused: 0117's hard product is three tasks and its recorded catalogue
     is `5b312d98…`; adding a fourth hard task changes that accepted product
     (a hold on accepted 0117). `expert` is refused: one report read can
     suffice, which fails 0123 §1's criterion. Both subset pins stay
     unchanged. The new product is `baseline --tier deliverability`: one
     task, two controls, thirty paid slots, run only by the operator.
   - *Vocabulary.* One new `Scenario` value, `newsletter_findings`. It reuses
     `Flow::Marketing`, `Check::MarketingCompleted`, the `marketing_completed`
     fact and `action::MARKETING`, so the snapshot field set is unchanged
     (0071 §4). In this scenario `marketing_completed` means "a re-sent
     newsletter's report is clean"; the scenario's doc comment says so.
     There is no named trap; 0071 §1's list is untouched.
   - *The application owns delivery.* The fixture serves the newsletter
     template at `GET /newsletter` and accepts `POST /newsletter
     {subject, html}`, which stores the template, renders it and sends it
     over real SMTP to the agent's own address, returning only after the
     final DATA reply (0001). The routes, and their lines in `GET /`, exist
     only in this scenario; every other task's application surface is
     byte-identical.
   - *Swarmail is the only rule owner.* After a `250 2.0.0 OK: stored as
     {id}` the fixture reads `GET /api/v1/messages/{id}/deliverability` from
     the trial's own Swarmail and judges the families `link.`, `html.` and
     `spam.` of `rule_id`. None present: an accepted `marketing` event and
     the fact, which never reverts. Any present: an unaccepted,
     category-less `marketing` event. The fixture never re-implements a
     rule.
   - *Availability is not cleanliness.* Once the agent can act, a report that
     cannot be read (404, 503, a parse failure, a timeout) is an unaccepted,
     category-less event — never clean and never `harness_error`, because an
     agent can cause it (clearing the inbox, loading the analyzer). Before
     the agent starts, `prepare` self-checks: the baseline report must read
     and carry exactly `link.scheme.http.v1`, `html.img.alt-missing.v1` and
     `spam.subject.all_caps.v1` in those families; otherwise prepare fails
     and the trial is a harness error.
   - *Bounds and guards.* The offer anchor (`href="http://shop.example/autumn"`
     or its `https` form) must stay in the HTML; a POST without it is
     refused, nothing is sent. Subject and HTML are bounded and control
     characters are refused, so agent input cannot inject headers. At most
     sixteen re-sends are accepted per trial.
   - *Scoring and proof.* The scorer is unchanged. An unfixed, partly fixed
     or finding-trading resend leaves the fact false and scores the declared
     idle class `gave_up`. Proof is free only: the oracle passes and the null
     agent fails through the real trial path, and scripted controls
     (unchanged resend, partial fix, traded finding, dropped offer, cleared
     inbox) all fail `gave_up`. No paid trial runs.

4. **Docs and published fixtures.** `docs/guide/deliverability.md` gains a
   "For agents" section and `src/static/llms.txt` a paragraph inside its
   existing deliverability section, neither copying a rule table. The crate
   publishes `tests/fixtures/deliverability-agent/` (failing and fixed
   `.eml`, a README with provenance), proven over real SMTP by rule id, never
   by full report bytes. Text that calls accepted decisions "proposed" is
   left for #337 except where this change edits that sentence anyway.

5. **Cut.** A second schema or "agent rendering"; structured output on
   March; a rule-id filter on `no_findings_at_or_above` (a contract change
   to accepted 0085 §7); a new `Facts` field, `Check`, `Flow` or action
   constant; a fixture-side copy of the rules; seeded defect variants; a
   named trap; switching the harness MCP client to a modern revision.

## Consequences

- The eval proves the text-block path (the harness MCP client pins
  2025-03-26, `evals/src/mcp.rs`); the root witnesses prove
  `structuredContent`. The agent
  under test negotiates its own revision.
- The fixture reads Swarmail's REST, which a real application would not. That
  is the price of one rule owner; it is loopback-only, per trial.
- The task is coupled to the rule set: a future rule in the three families
  that fires on the repaired template fails the oracle proof and the fixture
  test on the day it lands, which is the intended alarm.
- "From the report alone" means the report suffices (the oracle proves it),
  not that it is required; an agent may read the template instead.
- Content preservation beyond the offer anchor is not judged.
- The `deliverability` tier's difficulty is unmeasured until the operator
  budgets a run.
