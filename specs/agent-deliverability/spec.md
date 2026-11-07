# Deliverability for agents (issue #261)

Decision: 0131 — [Deliverability for agents: the shipped structured report, a repair eval task, docs and fixtures](../../docs/decisions/0131-agent-deliverability.md) (proposed; no holds)

- **Epic:** #251, phase 3, P2. Issue #261.
- **Run:** `issue-261-deliverability-for-age-f23c2fc5`, `claude-careful`, from
  `main` `50095c8` (carries #253 DNS policy, #254 DKIM, #255 SPF/DMARC, #258
  links, #260 spam rules).
- **Authorities read:** `docs/house-rules.md`; accepted
  [0085](../../docs/decisions/0085-deliverability-report.md),
  [0089](../../docs/decisions/0089-bulk-sender-checklist.md),
  [0105](../../docs/decisions/0105-mcp-report-output.md),
  [0120](../../docs/decisions/0120-deliverability-dns-policy.md),
  [0121](../../docs/decisions/0121-deliverability-link-checks.md),
  [0122](../../docs/decisions/0122-deterministic-spam-rules.md),
  [0125](../../docs/decisions/0125-offline-dkim-verification.md),
  [0126](../../docs/decisions/0126-spf-dmarc-alignment.md); the eval
  decisions [0055](../../docs/decisions/0055-agent-eval-harness.md),
  [0071](../../docs/decisions/0071-eval-trap-scoring.md),
  [0117](../../docs/decisions/0117-hard-eval-tier.md),
  [0119](../../docs/decisions/0119-eval-brokkr-recipe.md),
  [0123](../../docs/decisions/0123-expert-eval-tier.md) (all accepted); the
  code in `src/mcp.rs`, `src/deliverability*`, `evals/src/{tasks,fixture,mail,oracle,runner}.rs`,
  `docs/guide/deliverability.md`, `src/static/llms.txt`.
- **Not built here:** 0122 §2's `weight` and `total` (issue #337 builds them
  in parallel and takes its own `rule_set_id`).

## What and why

Agents that send mail through an application under test need three things
from Swarmail's deliverability report: a machine-readable shape, a proven
loop that turns findings into a fixed mail, and documentation that tells
them how to read the report without misreading it.

1. **Structured MCP output — already shipped; recorded and guarded.** 0085
   §6 deferred `outputSchema`/`structuredContent` for the report to "a later,
   small follow-up". Accepted 0105 delivered it: on 2025-06-18 and 2025-11-25
   the report tool declares a closed `outputSchema` (one owner,
   `OUTPUT_SCHEMAS` in `src/mcp.rs`) and returns the same value as
   `structuredContent` and as the compact JSON text block; 2025-03-26 keeps
   the text block alone, byte for byte. This feature changes no production
   code for it. It records the discharge (0131 §1) and adds two test-only
   guards: the served modern description must name the current
   `RULE_SET_ID`, and text block, `structuredContent` and REST must agree on
   a report whose `dns` member is populated.
2. **The report bytes do not change.** The shape stays
   `{rule_set_id, findings, dns}`; no `rule_set_id` is taken (0121 §5, 0122
   §1.4); nothing pins a full report body or a literal id, so #337 lands
   independently.
3. **An eval task, `newsletter-findings`.** The fixture application sends
   the agent a marketing newsletter that carries three named findings, one
   per location kind: `link.scheme.http.v1` (link), `html.img.alt-missing.v1`
   (mime_part) and `spam.subject.all_caps.v1` (header). The agent reads the
   report through Swarmail MCP, edits the template the application exposes,
   and re-sends. The application asks Swarmail for the new mail's report and
   records success only when no finding in the `link.`, `html.` or `spam.`
   families remains. The task sits in a new additive tier, `deliverability`,
   outside the default product, so no approved or recorded catalogue pin
   moves.
4. **Docs and published fixtures.** An agent-facing section in the
   deliverability guide and in `llms.txt`, and a published pair of `.eml`
   fixtures (failing, fixed) proven over real SMTP.

## Requirements

### R1 — MCP contract (no production change)

- R1.1 The report tool's descriptor, output schema, result bytes and error
  bytes are unchanged on every revision. March has no `outputSchema` and no
  `structuredContent`; its text block is byte-identical to `main` `50095c8`.
- R1.2 A unit test in `src/mcp.rs` asserts the modern report description
  contains `crate::deliverability::RULE_SET_ID`.
- R1.3 A real-server test sends the published failing fixture into an inbox
  that has a declared DNS policy (so the report's `dns` member has at least
  one lookup) and asserts, on 2025-06-18 and 2025-11-25, that
  `structuredContent` equals the parsed text block and equals the REST body,
  and on 2025-03-26 that there is no `structuredContent` and the parsed text
  block equals REST.

### R2 — Eval catalogue

- R2.1 `Tier` gains `deliverability` (serialized `deliverability`), not in
  `DEFAULT_TIERS`; `Tier::from_str`'s refusal names all six tiers.
- R2.2 `Scenario` gains `NewsletterFindings` (serialized
  `newsletter_findings`); `flow()` is `Flow::Marketing`; `uses_chaos()` is
  false. Its doc comment states that `marketing_completed` means "a re-sent
  newsletter's report is clean" in this scenario.
- R2.3 One task is appended after `confirm-injection`: id
  `newsletter-findings`, tier `deliverability`, `failure_class` `gave_up`,
  `idle_class` `gave_up`, check `marketing_completed`, one agent, the default
  deadline, the common prompt template. No `Check`, `Flow`, `Facts` field,
  `action` constant, `FailureClass` or trap row is added; `trap_class` is
  unchanged.
- R2.4 The goal discloses the whole rule without naming the defects: the
  newsletter the application sent `{identity}` has deliverability findings;
  read its report through Swarmail; edit the template with
  `GET /newsletter` and `POST /newsletter {"subject","html"}` (which re-sends
  it to `{identity}`); the task completes when a re-sent newsletter's report
  has no finding whose `rule_id` starts with `link.`, `html.` or `spam.`; the
  offer link (`href="http://shop.example/autumn"`, or its `https` form) must
  stay in the HTML; nothing about this task disqualifies the trial. It names
  no rule id.
- R2.5 The default-tier subset still serializes to `6068b687…`, the
  default-plus-hard subset to `5b312d98…`, the first twenty-four entries are
  unchanged; the full-catalogue golden and hash are regenerated from
  `tasks`, and the new task's definition hash is pinned.
- R2.6 `matrix --tier deliverability` and `baseline --tier deliverability`
  resolve to exactly `["newsletter-findings"]`; the baseline product is one
  task, two controls and thirty paid slots; no paid run is started.

### R3 — Fixture application

- R3.1 Template. A fixed, unseeded template: subject `AUTUMN LAMP OFFER`;
  HTML-only body

  ```html
  <html><body>
  <h1>Autumn sale</h1>
  <p>Our Autumn Lamp is back.</p>
  <p><img src="https://shop.example/lamp.png"></p>
  <p><a href="http://shop.example/autumn">Claim the Autumn Lamp offer</a></p>
  </body></html>
  ```

  rendered through `mail::render_with` (Message-ID and Date from the
  trial's render context, as today). Only the seed-derived headers differ
  between seeds.
- R3.2 Prepare. `prepare` provisions the template per agent, sends it as a
  `Flow::Marketing` delivery with message kind `newsletter`, reads its
  report, and self-checks that the report's findings in the three families
  are exactly `link.scheme.http.v1`, `html.img.alt-missing.v1` and
  `spam.subject.all_caps.v1`. If the
  Swarmail address is missing, the send does not end in `250 … stored as
  {id}`, the report does not read, or the set differs, prepare answers a
  non-200 and the trial is a harness error. Prepare records no event.
- R3.3 Routes exist only in this scenario. `GET /newsletter` returns
  `{"subject","html"}` for the claimed identity. `POST /newsletter` takes a
  strict JSON body `{"subject": string, "html": string}` (unknown keys
  refused). Both carry the identity claim the other routes use. In every
  other scenario neither route is mounted and `GET /` is byte-identical to
  `main`; in this scenario `GET /` lists the two operations.
- R3.4 Input refusals, each an unaccepted, category-less `marketing` event
  with nothing stored or sent: a body that is not the strict shape; a
  subject longer than 200 bytes or containing any C0 control, DEL or
  U+0085/U+2028/U+2029; HTML longer than 16384 bytes or containing NUL or CR;
  HTML containing neither `href="http://shop.example/autumn"` nor
  `href="https://shop.example/autumn"`; a POST after sixteen accepted
  re-sends in the trial (HTTP 429; the prepared newsletter does not count). A frozen trial answers the existing 409 and records
  nothing.
- R3.5 Send. An accepted POST stores the template, enqueues one
  `Flow::Marketing` `newsletter` delivery to the agent's own address, sends
  it outside the lock and returns only after the final DATA reply. The stored
  id is taken from the final `250 2.0.0 OK: stored as {id}` reply; the
  snapshot's `Attempt` shape is unchanged.
- R3.6 Verdict. The fixture reads
  `GET http://{swarmail_http}/api/v1/messages/{id}/deliverability` (no query,
  bounded timeout, bounded body) and parses it strictly. If it reads and no
  finding's `rule_id` starts with `link.`, `html.` or `spam.`: one accepted
  `marketing` event and `marketing_completed = true` (never reverted). If any
  such finding remains, or the report cannot be read (non-200, transport
  error, timeout, invalid JSON, missing `findings`): one unaccepted,
  category-less `marketing` event and no fact. The POST reply says only
  whether the newsletter was sent; it never carries findings or a verdict.
  If the trial froze meanwhile, nothing is recorded.
- R3.7 The fixture learns Swarmail's HTTP address from a new optional
  `fixture --swarmail-http <addr>` argument; the runner passes it only for
  scenarios that read reports (`Scenario::reads_reports()`, true only for
  `NewsletterFindings`).

### R4 — Oracle and controls

- R4.1 The oracle: reads the one newsletter in its inbox over MCP; calls
  `swarmail_get_deliverability_report` and parses the text block strictly;
  applies the fix each family finding names (`link.scheme.http.v1` → the
  located URL to `https`; `html.img.alt-missing.v1` → `alt="Autumn Lamp"` on
  the `img`; `spam.subject.all_caps.v1` → sentence-case subject) and exits
  nonzero on any other family id; `GET` then `POST /newsletter`; finds the
  new id as the inbox's mail it has not seen (no sleep: the POST returned
  after `250`, 0001); re-reads that report over MCP and exits nonzero if any
  family finding remains or the call is an error.
- R4.2 Free proofs through the real trial path against the candidate's
  `SWARMAIL_BIN`: the oracle passes; the null agent fails `gave_up` with no
  fact set.
- R4.3 Scripted controls, each through the real trial path, each failing
  `gave_up` with `marketing_completed` false: unchanged resend; partial fix
  (two of three); traded finding (all three fixed, subject given `!!`);
  dropped offer (the anchor removed; the POST is refused, nothing sent);
  cleared inbox (`swarmail_clear_inbox`, then an unchanged resend).

### R5 — Docs and published fixtures

- R5.1 `docs/guide/deliverability.md` gains `## For agents`: the tool and its
  `{id}` argument; `structuredContent` under the closed `outputSchema` on
  2025-06-18/2025-11-25 and the same JSON in one text block, the text block
  alone on 2025-03-26; the loop read → fix by `rule_id`/location/`fix_hint`
  → send a **new** message → read its report (stored mail is never edited,
  0005); filter by `rule_id` family, since under the default DNS policy
  authentication and checklist findings remain and `no_findings_at_or_above`
  cannot say "content is clean"; an `isError`, not-found or "unavailable"
  report is not a clean report; empty findings do not imply deliverability;
  the report is bounded (256 findings, 65536 compact bytes) and excess is an
  error, never truncation; `rule_set_id` moves when bytes move and clients
  ignore unknown members; a trimmed excerpt (no full body); pointers to the
  published fixtures and to the eval task. No rule table is copied.
- R5.2 `src/static/llms.txt` gains a paragraph with the same points inside
  its existing deliverability section; its section heading is not edited.
- R5.3 `tests/fixtures/deliverability-agent/` holds `newsletter-failing.eml`
  (the R3.1 template with fixed headers), `newsletter-fixed.eml` (the R4.1
  repair) and `README.md` (provenance: hand-written for #261, reserved
  `.example`/`.test` names only, no real person or address, regenerated only
  in a reviewed change). They ship with the crate through the existing
  `/tests/**` include.
- R5.4 `evals/README.md` documents the tier, the task, `matrix --tier
  deliverability` and `baseline --tier deliverability` (1 task, 2 controls,
  30 paid slots, operator-only); `docs/decisions/README.md` gains the 0131
  row.

## Acceptance Criteria

1. `git diff 50095c8 -- src/ ':!src/static/llms.txt'` contains only test code
   (an inline `#[cfg(test)]` test in `src/mcp.rs`); `RULE_SET_ID` is still
   `swarmail-html/7`; existing MCP revision, spam and deliverability suites
   pass unchanged.
2. The R1.2 unit test passes, and fails when the description literal and
   `RULE_SET_ID` disagree (shown once by a local mutation during
   implementation, recorded in the plan's implementation notes).
3. The R1.3 test passes over real SMTP and MCP: modern `structuredContent`
   == parsed text == REST with a non-empty `dns.lookups`; March has no
   `structuredContent` and parsed text == REST.
4. `swarmail-evals tasks` prints 25 tasks; the provenance test pins
   `6068b687…` and `5b312d98…` unchanged, the regenerated full-catalogue hash
   and the new task's definition hash; `tier_ids(Deliverability)` is
   `["newsletter-findings"]`.
5. `every_task_passes_with_the_oracle` and
   `every_task_fails_with_null_and_its_declared_idle_class` run 25 tasks;
   `newsletter-findings` passes with the oracle and fails `gave_up` with null.
6. Each R4.3 control fails `gave_up` through the real trial path, with
   `marketing_completed` false in the retained state.
7. Fixture tests against a real Swarmail: prepare's family set is exactly the
   three ids; prepare without `--swarmail-http` or against an unreachable
   report endpoint is a non-200; every R3.4 refusal sends nothing and
   records one unaccepted category-less event; a report read that fails
   after prepare (404, 503, transport error, invalid JSON; the verdict
   function's failure arms, unit-tested) is never clean;
   `GET /` for every other task equals its `main` bytes and `/newsletter` is
   404 there.
8. The published-fixture test: `newsletter-failing.eml` sent over SMTP has
   exactly the three family ids; `newsletter-fixed.eml` has none; the eval
   template's subject and HTML equal the failing fixture's decoded subject
   and HTML.
9. The guide and served `llms.txt` contain the R5.1 points (pinned strings in
   `tests/deliverability.rs`); `tests/api.rs` still proves the served
   `llms.txt` equals the file.
10. Every house gate passes locked: root fmt, clippy, test and build; evals
    fmt, clippy and test against the candidate's `swarmail`; the coverage
    gate at 100% exact and its selftest; publish-guard and seal proofs;
    `cargo deny check licenses`. No `Cargo.toml`, `Cargo.lock`,
    `evals/Cargo.lock` or `deny.toml` change. The brokkr bundle-compile gate
    runs in verify if the seat has no brokkr binary, and the seat says so.
11. No paid trial was run; nothing was pushed, merged, published or
    accepted.
