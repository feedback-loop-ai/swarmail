# Plan — agent setup guides

Revised 2026-11-01 for the review return (decision 0102 addendum; run
564da145). The first plan (2104a31) stands where this one is silent; every
section below says which review finding it answers.

## Technical context

Documentation plus one root integration suite in Rust 2024 using std,
tokio, serde_json and the crate's public `swarmail::config` CLI types —
all already in the locked tree. The suite spawns the shipped binary
(`env!("CARGO_BIN_EXE_swarmail")`) and speaks real HTTP, NDJSON and now
real SMTP (through `tests/common::smtp_send`, which already exists and
already speaks AUTH PLAIN with the username as inbox) to it, so it stays a
decision 0020 rule-3 test: public API, wire shape, no crate-private seam,
outside the coverage denominator by location.

## Constitution check

0001–0005 are described, never touched. The already-mailed rule restates
0001 (stored at `250`), 0002 (the wait scans what is already there) and
0005 (a cleared mail is gone from extraction too); it amends nothing. 0004
is accepted and is quoted. 0112 and 0111 are proposed with their compatible
implementation on main; the guide describes that binary and the suite
exercises it (accepted 0045 §8). No `src/` change, so the 100% exact floor
is unaffected. No dependency, lockfile, `deny.toml`, workflow or bundle
change, so the PR is not on 0056's hold list. Commits unsigned. No decision
proposed.

## Research and resolved choices (re-derived from main 5e8d9d8)

- Inbox name, in order (`src/smtp.rs` ~1555, 0112 §4): the routed
  recipient, else the AUTH identity, else `default`. `route_of`
  (`src/smtp.rs` ~697) admits one label `[a-z0-9-]{1,63}` under the route
  domain; anything else under the domain is `550 5.1.3`. The server offers
  `AUTH PLAIN LOGIN` and any password succeeds (0004).
- `--route-domain` / `SWARMAIL_ROUTE_DOMAIN` (`src/config.rs` ~113) is
  `Option<String>`; `swarmail serve` has none, so the guide's server routes
  by AUTH alone. `swarmail_create_inbox` (`src/mcp.rs` ~986) answers
  `{"address": "agent@<hex>.<domain>", "inbox": "<hex>"}` with a route
  domain and an `isError` whose text contains `--route-domain` without.
- `swarmail_wait_for_email` accepts `timeout_ms` from 0 to 60000
  (`src/mcp.rs` ~1159). A timed-out wait is a text result `{"matched",
  "total", "timed_out": true}` plus `near_misses` when the inbox holds any
  mail (`src/mcp.rs` ~870, `src/mcp/near_miss.rs` ~24); an empty inbox has
  no `near_misses` member. Not an `isError`, so the suite's `tool` helper
  can read it.
- `swarmail_wait_for_link`'s `inputSchema` properties include
  `anchor_text`, `host` and `path` (`src/mcp/focused.rs` ~394–402);
  `swarmail_extract_links` takes the email id only.
- Live tools on this tree: 18 — the 14 in `src/mcp.rs` (`clear_all`,
  `clear_inbox`, `create_inbox`, `delete_email`, `extract_codes`,
  `extract_links`, `get_deliverability_report`, `get_email`,
  `get_latest_email`, `search_emails`, `seed_email`, `set_chaos`,
  `clear_chaos`, `wait_for_email`) plus `wait_for_code`, `wait_for_link`,
  `list_attachments`, `get_attachment`. The suite never pins the number;
  the implementer confirms the set from `tools/list` when writing the table.
- The Straight-tier prompts render `Your Swarmail inbox: {inbox}` and tell
  the agent to pass `"inbox": "{inbox}"` (`evals/src/tasks.rs` ~145); their
  goals say the application already emailed you (~205–225). That is the
  entry state the block's bold rule is for (F2).
- `tests/common::smtp_send(addr, Option<&str> inbox, from, to, subject,
  body)` returns the final `250` line or the first 4xx/5xx; `None` sends
  without AUTH. `HELPER_DEADLINE` is 30 s. Nothing new is needed on the
  SMTP side (R7).
- The startup line carries `smtp=`, `pop3=` and `http=`; `parse_http_addr`
  already demands the first two, so returning the SMTP address too is a
  one-function change.

## Project structure and touched files

| Path | Change | Finding |
|---|---|---|
| `docs/agents/setup.md` | inbox paragraph, already-mailed rule, "Where mail lands", "The tools" table, `wrong_link`/`wait_timeout`/`stale_code` lines, section 8 bullet, "What CI proves" | F1, F2, F3, F-tools, F4 |
| `docs/agents/swarmail/SKILL.md` | body re-synced byte for byte with the block | F1, F2, F3 |
| `tests/agent_setup_docs.rs` | `trim_start` tilde check; SMTP address from the startup line; optional child env for the route domain; the SMTP route/loop test; the routed-clause test; inventory equality; selector schema check; near-miss key check; revised pins and absence pins; README pins | F1–F5, F-tools |
| `README.md` | one-sentence claim; suite-table row text | F4 |
| `docs/tasks/issue-161-agent-setup-guides.md` | commit the restart framing intake appended | — |

Nothing else. No `src/`, no `Cargo.*`, no `.github/`, no `.brokkr/`.

## Data model and contracts

```text
Fence  { info: Info, marker: Option<String>, body: String, line: usize }
Info   = Json | Toml | Yaml | Markdown | Sh | Text
Wiring = Http { url: String } | Stdio { command: String, args: Vec<String> }
Addrs  { smtp: SocketAddr, http: SocketAddr }          # from the startup line
SmtpUrl { user: String, authority: String }           # the block's one smtp:// token
```

- `extract(doc)` — unchanged except (F5): the fallthrough arm tests
  `line.trim_start()` for both "```" and "~~~" before refusing as "an
  indented or tilde fence".
- `parse_addrs(line) -> Option<Addrs>` replaces `parse_http_addr`; the
  `Server` struct carries `smtp` beside `http`.
- `start_server(command)` keeps its signature; `start_server_with(command,
  extra_env: &[(&str, &str)])` is the one new entry, used once with
  `[("SWARMAIL_ROUTE_DOMAIN", "swarmail.test")]`. `Command::env` only.
- `smtp_url(block) -> Result<SmtpUrl, Refusal>`: the inline-code tokens of
  the block that start with `smtp://`; exactly one, else refused;
  `user:password@authority` split; user must be `<inbox>`.
- `Defaults` gains `smtp_authority: serve.smtp_listen`; `assert_smtp_url`
  requires `url.authority == defaults.smtp_authority`.
- `live_url(url, smtp: SocketAddr, inbox) -> String`: `substitute_once` the
  authority, then `substitute_once` `<inbox>`; the AUTH username the test
  sends is parsed back out of the result, so what is sent is what the
  block, substituted, says.
- `Client::tool` already returns the text result of a non-error call; a
  new `Client::tool_raw` returns the full `result` so `isError` and its
  text can be asserted for `swarmail_create_inbox` on the default server.
- `wait0(client, inbox, filter) -> Value`: `swarmail_wait_for_email` with
  `timeout_ms: 0` merged into `filter`. The only wait the new tests use.
- `inventory_names(guide) -> BTreeSet<String>`: backticked first-column
  cells of the table under `## 8. The tools` (section numbering per the
  implementer; the heading text "The tools" is what the reader finds).
- `near_miss_field(block) -> Result<String, Refusal>`: the backticked
  token on the `- \`wait_timeout\`` bullet that is not a tool name.

## Implementation approach

1. **Block and guide first (F1, F2, F3).** Rewrite the block's opening per
   spec R3 using these sentences verbatim, since the pins quote them:
   - "If your task or environment names your swarmail inbox, use that name
     and pass it as `"inbox"` to every tool."
   - "Otherwise choose a name of lowercase letters, digits and `-`, and
     point the application's SMTP at `smtp://<inbox>:x@127.0.0.1:1025`: the
     SMTP AUTH username names the inbox and any password is accepted."
   - "Mail sent without AUTH lands in the inbox `default`, not in yours."
   - "On a server started with `--route-domain`, you may instead call
     `swarmail_create_inbox` and give the flow the address it returns."
   - "**If the flow has already mailed you, do not clear: wait, or read,
     straight away.** The wait returns mail that is already in your inbox."
   - "Otherwise work in four steps, in this order: clear, act, wait,
     extract."
   Step 2 (Act): "Trigger the flow exactly once, with the application
   pointed at your inbox as above." `wrong_link`: "Wait with
   `swarmail_wait_for_link` and select the link by its `anchor_text`,
   `host` and `path`, not by its position in a list." `wait_timeout`: "Read
   the timed-out answer's `near_misses` (mail that arrived but did not
   match, listed when the inbox holds any) before you re-trigger; a second
   trigger makes the first code stale." `stale_code`: "take the code from
   the newest mail". Then copy the block into SKILL.md.
2. **"Where mail lands" after the start command (F1).** Four sentences: the
   AUTH rule and `default` (0004, linked); the route-domain rule with
   `--route-domain` / `SWARMAIL_ROUTE_DOMAIN`, routed recipient outranks
   AUTH, unknown label refused (0112, linked, marked proposed); the default
   server has no route domain, so `swarmail_create_inbox` answers an error
   naming the flag there; inbox names stay in `[a-z0-9-]` so they are legal
   on both routes. Section 8's clear bullet becomes "Unless the flow has
   already mailed you, clear before you act, never between act and extract:
   the clear deletes the mail you are waiting for." (AC-7 pin text).
3. **"The tools" table (F-tools).** One row per live tool, name in
   backticks in the first column, one-line purpose in the second; the
   attachments pair and the focused waits get rows like the rest. Put it
   after the block and before the guarantees.
4. **"What CI proves" and README (F4).** The guide: the five snippet round
   trips prove transport (clear, seed, wait, extract through the declared
   transport); one test drives the taught loop over real SMTP with AUTH as
   the block says, covers the already-mailed case and both wrong-route
   controls, and proves the route-domain clause with the knob in the
   environment. README: the single R5 sentence; the suite row rewritten.
5. **Suite, in this order.** (a) F5 one-liner plus three negative rows.
   (b) `parse_addrs`, `Server.smtp`, `start_server_with`. (c) `smtp_url`,
   `assert_smtp_url`, `live_url` and their in-process red controls. (d)
   `the_taught_route_and_loop_hold_over_real_smtp`: one default server;
   inbox A: clear, send with AUTH = A (code 1), `wait0` finds it, extract
   equals code 1; inbox B: send with AUTH = B (code 2) **before any clear**,
   `wait0` finds it, extract equals code 2, then `wait0` with
   `subject: "no-such-subject"` is `timed_out: true` and has the block's
   near-miss key, then clear B and `wait0` is `timed_out: true` without that
   key; controls: send with `None` AUTH → `wait0(A)` timed out,
   `wait0("default")` found; send with AUTH `other` → `wait0(A)` timed out,
   `wait0("other")` found. (e) `the_route_domain_clause_mints_and_routes`:
   default server → `create_inbox` `isError` names `--route-domain`; routed
   server → `create_inbox`, send to its address with AUTH `other`,
   `wait0(minted)` found, `wait0("other")` timed out. (f) Inventory
   equality and the selector/near-miss schema checks folded into the
   existing live `tools/list` test. (g) Pins: `GUARANTEES`, `BLOCK_RULES`
   and `LABELS` updated; new `FORBIDDEN_IN_BLOCK` absence pins; README
   presence and absence pins.
6. **Gates last.** The quickstart below, then the oracle/null baseline, then
   the report with AC-3 recorded as not measured.

## Quickstart and verification

```bash
cargo test --locked --test agent_setup_docs
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --locked --bin swarmail
(cd evals && cargo fmt --all -- --check)
(cd evals && cargo clippy --all-targets --locked -- -D warnings)
(cd evals && SWARMAIL_BIN="$PWD/../target/debug/swarmail" cargo test --locked)
brokkr compile --bundle .brokkr/recipes/self
bash scripts/coverage-gate.sh && bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh && bash scripts/seal-branch-test.sh
cargo deny check licenses
evals/target/debug/swarmail-evals trial --task signup --agent oracle   # then otp, magic-login; then --agent null
```

## Risks

| Risk | Mitigation | Finding |
|---|---|---|
| The SMTP URL form is never tried against a real client library (STARTTLS-only clients, LOGIN-only clients). | 0004 itself teaches `smtp://proj-42:x@swarmail:1025`; the server offers PLAIN and LOGIN; the suite proves the server side of the exact bytes taught, and the guide says client behaviour is operator-owned (AC-8). | F1 |
| A zero-timeout wait is read as "sleep 0 and hope". | It is the opposite: 0001 says the mail is stored before the `250`, 0002 says the wait scans after subscribing, so the found/timed-out answer is deterministic. The suite's doc comment says so and no wait on a green path has a longer deadline than needed. | F1, F2 |
| 0112 is proposed; if the operator rejects it the routed clause is wrong. | The clause is one sentence, one table row and one test; the guide labels 0112 proposed; removing all three is a one-commit change and the AUTH form stands alone. | F1 |
| 0111 changes the near-miss key. | The block names the key, the test reads the live answer; a rename goes red in this suite the day it merges, which is what the pin is for. | F-tools |
| The inventory table drifts when a tool is added. | Equality with `tools/list` makes the omission red; the fix is one row. That is the chosen cost (ruling 18). | F-tools |
| `trim_start` also catches a legitimately indented fence a future author wants. | The convention is column 0 only (ruling 2); the refusal message names the line. | F5 |
| Two servers in one test (`the_route_domain_clause_mints_and_routes`). | Each is ephemeral and reaped; the routed one is started only after the default-server `isError` assertion, so no two servers of this test live at once. | F1 |
| Parallel tests in one binary collide on inboxes. | Each test owns its server; inbox names per test; the `default` and `other` inboxes are read only on the server that test started. | F1 |
| Executed bytes differ from shipped bytes (one token each for HTTP and SMTP authority, plus `<inbox>`). | Default-equality on the literal before substitution; exactly-once replacement; stated in the suite's doc comment and the guide. | F1 |
| The README sentence is shortened past the truth. | It claims the wiring snippets are driven and the install commands are marked; the test pins both the new sentence and the absence of the old one. | F4 |
| The one-real-model criterion stays unmet. | Reported as not measured in the PR and the report; #161 not claimed closed. | — |
| macOS CI differences. | std-only process handling; `common::smtp_send` is used by other suites on both OSes already. | — |
