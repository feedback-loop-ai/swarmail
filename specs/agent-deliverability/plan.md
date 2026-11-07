# Plan — deliverability for agents (issue #261)

Spec: [spec.md](spec.md). Decision: [0131](../../docs/decisions/0131-agent-deliverability.md) (proposed, no holds).

## Technical context

- Root crate `swarmail` (Rust, MSRV and toolchain unchanged). The report
  producer (`src/deliverability.rs`, `RULE_SET_ID = "swarmail-html/7"`), the
  MCP surface (`src/mcp.rs`: `OUTPUT_SCHEMAS`, `text_result`,
  `MODERN_REPORT_DESCRIPTION`) and REST
  (`GET /api/v1/messages/{id}/deliverability`, `PUT /api/v1/inboxes/{inbox}/dns`)
  already do what the agent needs. Root production lines changed: **zero**;
  `src/static/llms.txt` is served data.
- `evals/` is the standalone, unpublished harness workspace (0055), outside
  the root coverage denominator. Its MCP client pins 2025-03-26
  (`evals/src/mcp.rs`); the report tool is already in the wrapper allowlist
  (`evals/src/wrappers.rs` `SWARMAIL_TOOLS`); `reqwest` is already a
  dependency. No dependency is added anywhere.
- Each trial spawns its own Swarmail (`--max-per-inbox 100000`) and its own
  fixture (`evals/src/runner.rs` `run_body`); trials are sequential (0076)
  and isolated (0115). No load proof is needed; none is run.

## Constitution check

| Rule | How this plan holds it |
|---|---|
| 0001 accept == stored | The fixture keys the verdict on the id in the final `250 2.0.0 OK: stored as {id}`; the oracle finds the new mail right after the POST returns, with no wait. |
| 0002 subscribe before check | No new wait path. The oracle needs none (0001); docs point agents to `swarmail_wait_for_email` for mail they did not send synchronously. |
| 0003 oldest-only pruning | Per-trial inbox, cap 100000, at most 17 mails; verdict by id, never "the inbox is clean". |
| 0004 real protocol | Every new test runs real SMTP/MCP/HTTP against a real Swarmail; no mocks. |
| 0005 extraction at ingest | Repair is a new delivery; stored mail is never edited or re-analysed on other bytes. |
| Coverage 100% exact | No production line added; the only root code is an inline `#[cfg(test)]` test. |
| 0020 test placement | R1.2 inline in `src/mcp.rs`; R1.3 and fixtures in `tests/deliverability.rs` (public API only); no sleeps as synchronisation, no wall-clock assertions. |
| Dependencies | None added; no lockfile or `deny.toml` change, so no merge hold on that ground. |
| Decisions | 0131 proposed; amends no accepted ruling; extends the harness spec's tier list as 0117/0123 did. |
| No paid trials | Only the oracle, null and scripted controls run. |

## Research — the panel's positions, ruled

| Question | Simplicity | Robustness | Ruling | What the losing side costs |
|---|---|---|---|---|
| MCP structured output | Shipped by 0105; no code | Same, plus a description drift guard and a populated-`dns` equality witness | **Both:** no production code; adopt both test-only guards | Two tests (cheap). Without them a #337 bump could leave the served description stale silently. |
| Dated pointer on 0085 | No | Yes | **No edit to 0085.** 0105's header already supplements it; 0131 links both | A reader of 0085 alone does not see the discharge; the decisions index row names it. |
| `rule_set_id` | None | None, and "reserve the next for #337" | **None, and no reservation.** 0121 §5/0122 §1.4 have each byte-moving run take its own id | — |
| Tier | New `deliverability` | `hard` | **`deliverability`.** `hard` changes accepted 0117's three-task product and the recorded `5b312d98…` catalogue (a hold); `expert` fails 0123 §1 | No "hard" label; the tier is a capability tier with its own operator-run product. |
| Vocabulary | Reuse marketing flow/check/fact/action | New `Check` | **Reuse.** A new `Check` adds a value with no new fact behind it; a new fact changes the snapshot field set (0071 §4) | `marketing_completed` carries a scenario-specific meaning, documented at the scenario. |
| Verdict owner | Swarmail REST read by fixture | Oracle-side assertions | **Fixture reads Swarmail REST** (scoring is app-state, 0055); **oracle also re-reads over MCP** and fails loudly | One loopback GET per send and one CLI argument. |
| Locating the stored mail | DATA `250 … stored as {id}` | Per-send token search | **Reply id** (already pinned by `tests/api.rs`); exact and needs no scan | — |
| Report-read failure | `harness_error` | Never clean | **Before the agent: harness error (prepare self-check). After: unaccepted, never clean, never `harness_error`** — an agent can cause 404/503, and an agent-caused failure must not become unscorable | A genuinely broken analyzer mid-trial scores `gave_up`, not `harness_error`; the prepare self-check and the oracle proof catch a broken analyzer first. |
| What "clean" means | No `link.`/`html.`/`spam.` finding | Named ids absent and no error-severity finding | **Families.** Error-severity checklist/authentication findings are about DNS, not the mail the agent edits | Authentication and checklist outcomes are not measured by this task; the guide says so. |
| DNS policy for the trial | Default | Declared map with `spf_client_ip` | **Default.** No scored finding depends on DNS, so loopback v4/v6 cannot flip a verdict | The eval exercises no authentication rule. |
| Guard | Offer substring | Refuse empty template | **The offer anchor's literal `href`** plus input bounds, control-character refusal and a 16-send cap (robustness's hostile-input concern, applied to agent input) | A comment-wrapped anchor passes the guard; content is not judged semantically. |
| Trap | None | None | **None** (0071 §1 untouched) | Cheating earns `gave_up`, not a named class. |
| Controls | Oracle/null + fixture tests | Five scripted controls | **Both**, controls adapted: "edit without send" does not exist because POST stores and sends | — |
| Published fixtures | `tests/fixtures/` | `evals/` | **`tests/fixtures/deliverability-agent/`**: `evals/` is unpublished (0055); `/tests/**` already ships | — |
| App surface in other tasks | (unstated) | (unstated) | **Routes and index lines only in this scenario**, so every other task's agent-visible app is byte-identical to `main` | A conditional route in `start`. |

## Project structure (files touched)

```
src/mcp.rs                                   inline #[cfg(test)]: description names RULE_SET_ID (R1.2)
src/static/llms.txt                          agent paragraph in the deliverability section (R5.2)
tests/deliverability.rs                      R1.3 witness, published-fixture test, llms/guide pins (R5)
tests/fixtures/deliverability-agent/         newsletter-failing.eml, newsletter-fixed.eml, README.md
docs/guide/deliverability.md                 ## For agents (R5.1)
docs/decisions/README.md                     0131 row
evals/src/tasks.rs                           Tier::Deliverability, Scenario::NewsletterFindings, reads_reports(), task row, NEWSLETTER_* consts
evals/src/fixture.rs                         template state, prepare + self-check, GET/POST /newsletter, verdict, conditional routes/index
evals/src/mail.rs                            send_stored(): (SendResult, Option<String>) from the final reply; send() delegates
evals/src/main.rs                            fixture --swarmail-http <addr> (optional)
evals/src/runner.rs                          pass --swarmail-http when task.scenario.reads_reports()
evals/src/oracle.rs                          newsletter arm (R4.1)
evals/tests/common/mod.rs                    helper modes for the R4.3 controls
evals/tests/proof.rs                         counts 24→25; R4.3 controls test
evals/tests/fixture.rs                       R3 fixture tests against a real Swarmail
evals/tests/provenance.rs                    pins kept; golden regenerated; new task hash
evals/tests/data/catalogue-v2.json           regenerated from `tasks`
evals/tests/boundary.rs                      README phrase pins for the new tier
evals/README.md                              tier, task, matrix/baseline lines
```

## Data model

- Catalogue (schema 2, unchanged): `tier` gains `"deliverability"`;
  `scenario` gains `"newsletter_findings"`. Strict deserialization keeps
  refusing anything else.
- Snapshot (schema 2, unchanged field set): events use action `marketing`
  (accepted or unaccepted, `category: null`) with `flow: "marketing"`;
  deliveries use flow `marketing`, message kind `newsletter`. `Attempt`
  stays `{stage, code}`; the stored id never enters the snapshot.
- Fixture private state: per agent identity, the current template
  `{subject: String, html: String}` and a trial-wide newsletter send count.
- Fixture constants (in `tasks.rs` so the oracle, fixture and tests share
  them): `NEWSLETTER_SUBJECT`, `NEWSLETTER_HTML`, `NEWSLETTER_OFFER_HREFS`
  (`["href=\"http://shop.example/autumn\"", "href=\"https://shop.example/autumn\""]`),
  `NEWSLETTER_FAMILIES` (`["link.", "html.", "spam."]`),
  `NEWSLETTER_FINDINGS` (the three ids), `NEWSLETTER_MAX_SENDS = 16`,
  `NEWSLETTER_MAX_SUBJECT = 200`, `NEWSLETTER_MAX_HTML = 16384`.

## Contracts

- `GET /newsletter` (this scenario only; identity claimed as elsewhere) →
  `200 {"subject": "...", "html": "..."}`.
- `POST /newsletter {"subject": string, "html": string}` →
  `200 {"ok": true, "message": "newsletter sent"}` after the final DATA
  reply; `400 {"ok": false, ...}` on R3.4 refusals; `429` past 16 sends;
  `409` when frozen; a non-`250` send leaves the delivery failed and the
  trial unscorable (existing semantics). The reply never carries findings.
- `GET /` in this scenario appends the two operations; elsewhere unchanged.
- `swarmail-evals fixture --swarmail-http <host:port>`: optional; required in
  effect by `newsletter_findings` (prepare refuses without it).
- Swarmail contracts consumed, unchanged: SMTP final reply
  `250 2.0.0 OK: stored as {id}`; REST
  `GET /api/v1/messages/{id}/deliverability`; MCP
  `swarmail_get_deliverability_report {"id"}`.

## Quickstart

```bash
cargo build --locked --bin swarmail
cd evals
swarmail-evals tasks | jq '.tasks[-1].id'           # "newsletter-findings"
SWARMAIL_BIN=<candidate swarmail> cargo test --locked --test proof
SWARMAIL_BIN=<candidate swarmail> cargo run --locked -- trial --task newsletter-findings --agent oracle
SWARMAIL_BIN=<candidate swarmail> cargo run --locked -- trial --task newsletter-findings --agent null
```

No `baseline --tier deliverability` without `--dry-run`: paid runs are the
operator's.

## Implementation approach

1. Root tests first (R1.2, R1.3) — they pass on `main` today and stay
   green; they are guards, not features.
2. Published fixtures and their test (R5.3), which also fixes the template
   bytes the eval reuses.
3. Catalogue additions and pins (R2), then `mail::send_stored`, the fixture
   (R3), the runner argument, the oracle (R4.1), then the controls (R4.3).
4. Docs (R5.1, R5.2, R5.4) and their pins.
5. Every house gate, locked.

Fixture verdict sequence in `POST /newsletter`: lock → claim, validate,
count, store template, `enqueue` → unlock → `deliver` (SMTP) → read the stored
id → report GET with a 10 s client timeout and a 1 MiB body cap → lock → if
not frozen, record one `marketing` event (accepted iff clean) and set the
fact if clean → reply. `prepare` uses the same send and read, then the
self-check, before answering.

## Risks

| Risk | Mitigation |
|---|---|
| #337 lands first and changes the report (weight/total, new id). | Nothing here pins a report body or literal id; the eval reads `findings[].rule_id` only and ignores unknown members; the description guard fails until #337 updates the served text, which is its job. Rebase onto `main` before merge and rerun the gates. |
| A future rule in the three families fires on the fixed template. | The oracle proof, the prepare self-check and the published-fixture test fail on the day; regenerate fixtures in a reviewed change. |
| The fixture's REST read fails because of the agent (inbox cleared, analyzer saturated). | Recorded as unaccepted and not clean; never `harness_error`; the agent may resend (cap 16). |
| A broken analyzer would only show as `gave_up` mid-trial. | The prepare self-check makes it a harness error before the agent starts; the oracle proof would fail. |
| Agent input injects headers or ends DATA early. | Subject control characters refused; HTML NUL/CR refused; bodies go through the existing `text_part` (7bit or base64) and `dot_stuff`. |
| The new route or index line changes other tasks' measurement conditions. | Mounted and listed only in this scenario; a fixture test compares every other task's `GET /` bytes to `main`'s. |
| Catalogue pin drift. | Provenance test keeps `6068b687…` and `5b312d98…`; the `5b312d98` filter becomes "default tiers or hard", not "not expert". Golden regenerated from `tasks`, never hand-edited. |
| Wrapper limits (paid agents use `eval_http`, 64 KiB input). | HTML capped at 16 KiB so an escaped body fits. |
| Fixture outbound HTTP blocked by trial isolation. | Proven by the real-trial oracle test; it reaches loopback Swarmail exactly as SMTP already does. |
| Agent fixes by inspecting the template, not the report. | Accepted (0131 Consequences): the oracle proves the report suffices. |
| No brokkr binary in the implement seat. | The bundle-compile gate runs in verify; the seat says so. |

## Implementation notes

Filled by the implementer, 2026-11-07, on `eec703a` (from `main` `50095c8`).

**R1.2 mutation (T1).** `mcp::tests::modern_report_description_names_the_current_rule_set`
passes. With `MODERN_REPORT_DESCRIPTION`'s "rule set swarmail-html/7" edited to
`swarmail-html/6` it fails (`assertion failed:
MODERN_REPORT_DESCRIPTION.contains(crate::deliverability::RULE_SET_ID)`); the
literal was restored and `git diff 50095c8 -- src/ ':!src/static/llms.txt'`
has no removed line and only that test added. `RULE_SET_ID` is unchanged.

**No production change (T2).** Root production lines changed: zero. The root
changes are one inline `#[cfg(test)]` test in `src/mcp.rs`, the served
`src/static/llms.txt` paragraph, `tests/deliverability.rs` (R1.3 witness,
published-fixture test, guide and llms pins) and `tests/fixtures/deliverability-agent/`.

**Gates, all run locked on the candidate.**

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | clean (root and `evals/`) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | clean (root and `evals/`) |
| `cargo test --workspace --locked` | every suite green (719 lib tests, 29 in `tests/deliverability.rs`) |
| `cargo build --locked --bin swarmail` | built; its `target/debug/swarmail` is `SWARMAIL_BIN` below |
| `cd evals && SWARMAIL_BIN=… cargo test --locked` | every suite green: lib 151, `proof` 15 (oracle 25/25 pass, null 25/25 fail, the five newsletter controls), `provenance` 29, `fixture` 27, `scoring` 31, `boundary` 23, `baseline` 9, `runner` 38, `report` 41, `wrappers` 22, `recipe` 15, `identity` 5, `trial_isolation` 15 |
| `bash scripts/coverage-gate.sh` | pass, exact 100/100/100 (31260 lines, 2134 branches, 3126 functions) |
| `bash scripts/coverage-gate.sh selftest` | pass |
| `bash scripts/publish-guard-test.sh` | OK |
| `bash scripts/seal-branch-test.sh` | 7 cases and 10 mutants passed |
| `cargo deny check licenses` | licenses ok |
| `brokkr compile --bundle` over `.brokkr/recipes/*/` | **not run: this seat has no `brokkr` binary.** The verify seat runs it. No file under `.brokkr/` changed. |

No `Cargo.toml`, `Cargo.lock`, `evals/Cargo.lock`, `deny.toml`, `.github/`,
`scripts/` or `.brokkr/` change; no dependency added; no paid trial run; no
load proof was needed or run (trials are sequential per 0076).

**Evidence for the pins.** The first 24 catalogue entries hash to `a05c85d3…`
(the expert tier's full-catalogue pin) when re-serialized from `tasks`, and
independently from the golden file with python; the default tier is still
`6068b687…`, the default-plus-hard subset `5b312d98…`; the full 25-task
catalogue is `732cad37…` and `newsletter-findings`' definition `3c680f6e…`
(both computed from `tests/data/catalogue-v2.json` with python, not copied
from the code under test). `GET /` of two other tasks was captured from the
unmodified fixture of `eec703a` (built in a scratch tree) and equals the
new fixture's bytes (the port aside); that capture is now the `MAIN_INDEX`
constant, asserted for every other task.

**Deviations and decisions the design left open.**

1. *Goal wording (R2.4).* The prompt renderer treats every `{…}` as a
   placeholder, so the goal cannot carry the literal body `{"subject",
   "html"}`; it says "a JSON body of the two string members subject and html"
   instead. Every other R2.4 phrase is present and a unit test pins them.
2. *`mail::send_stored`.* Returns `(SendResult, Option<String>)`; `stored_id`
   accepts only `2.0.0 OK: stored as {id}` with an id of `[A-Za-z0-9_-]+`.
3. *Prepare refusal codes.* A missing `--swarmail-http` is 412 (nothing
   enqueued or sent); an unreadable or wrong baseline report is 500, after the
   mail was sent. Both are harness errors in the runner.
4. *`runner::fixture_args`.* The fixture argv is built by one small function
   so "no `--swarmail-http` for any other task" is a unit test over the whole
   catalogue, not an inspection of a spawned process.
5. *`entropy_proofs` exemption.* `production_construction_keeps_secrets_independent_of_public_seed`
   asserted that every task mints secret material; `newsletter-findings` mints
   none (its state is the public template), so it is asserted to mint none.
6. *Harness spec amendment.* `specs/agent-eval-harness/spec.md` gained a dated
   (2026-11-07, proposed 0131) tier-list and AC3 amendment, as 0117 and 0123
   did; `evals/tests/boundary.rs` pins it. No accepted record's text changed.
7. *Guide excerpt.* The "For agents" excerpt shows `"rule_set_id": "…"`, not a
   literal id, so #337's bump leaves the guide untouched; the description guard
   (T1) is the one place that fails if the constant moves without the served text.
8. *Coverage.* `evals/` is outside the root denominator (0055); every new
   root line is test code, so the exact gate is unchanged at 100%.
