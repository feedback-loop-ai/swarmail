# Agent setup guides and skills, exercised in CI — issue #161

## Clarifications

- **2026-11-01 — review return (run 564da145, decision 0102 addendum).**
  The review of implementation 66ded9c found the committed design defective
  (F1 HIGH, F2 MEDIUM) beside three low findings (F3–F5) and a currency
  gap (the tool surface and timeout near misses merged since). This revision
  corrects the spec in place: the inbox route and the already-mailed entry
  state become requirements with wire proofs (R3, R7, AC-R1…AC-R10), the
  transport-only round trip is relabelled as such (AC-2), and rulings 7, 8,
  10, 11 and 15–19 record what changed and why. The first design commit
  (2104a31) and its text stay in history; nothing below weakens an earlier
  criterion — each change adds a proof the old suite could not fail.

## Feature and rationale

A stranger with a fresh project must be able to point Claude Code, Codex,
Cursor or dsh at a running swarmail by copying bytes out of one guide, drop
one `AGENTS.md` block into their project that teaches the clear → act →
wait → extract loop and its traps, and trust that the bytes they copied are
the bytes CI ran — and that the loop the block teaches is the loop that
works on the wire: the application's mail reaches the inbox the agent reads,
and an agent spawned after the flow already mailed it does not delete that
mail. Before this change the only wiring text was an unexercised `jsonc`
block in the README; after 66ded9c the guide existed but taught an inbox
nothing would ever fill ("pick an inbox name that is yours alone") and an
unconditional clear that destroys the mail every Straight-tier eval task
delivers before the agent starts.

This change is documentation plus a root integration suite. It adds no
production line, no dependency, no CI workflow edit and no decision. Part of
epic #164; priority P2. The intake framing is
`docs/tasks/issue-161-agent-setup-guides.md` (first run and restart
sections; the restart section wins where they differ). Authority for the
behaviour the guides describe:

- accepted 0001, 0002, 0005 (the frozen guarantees the loop relies on);
- accepted 0004 — the SMTP AUTH username names the inbox; an
  unauthenticated session delivers to `default` (F1);
- proposed 0112 — on a server started with `--route-domain` /
  `SWARMAIL_ROUTE_DOMAIN`, a recipient `<local>@<label>.<domain>` names
  inbox `<label>` and outranks AUTH; a label under the domain that is not
  `[a-z0-9-]{1,63}` is refused `550 5.1.3`; `swarmail_create_inbox` mints
  `<hex>@<inbox>.<domain>` and is an `isError` naming `--route-domain`
  without one; the default server has no route domain (F1). The guide
  describes the binary main ships; under accepted 0045 §8 that is
  compatible implementation and amends nothing;
- proposed 0111 — a timed-out wait on an inbox that holds non-matching mail
  carries `near_misses`; an empty inbox answers without it (F-tools);
- accepted 0012, 0029, 0039/0044, 0047 (bridge and reply semantics), the
  eval harness decisions 0055, 0071, 0076 as read-only context, and 0102
  (this return's procedure).

No decision is proposed: the guides describe rulings and change none.

## Panel synthesis and rulings

Both run-local positions of each run were read (`.brokkr/state/design/
positions/simplicity.md` and `robustness.md`; neither is a tracked
citation). In the restart both agree the fix is a revision on the same five
paths, that the block must say how mail reaches the inbox and must not
clear mail that is already there, that the proof must send real SMTP the
way the block tells the application to send, and that AC-3 stays open.
They differ on how much wire the suite drives. Rulings 1–6, 9, 12–14 stand
from the first design; 7, 8, 10 and 11 are revised; 15–19 are new.

1. **Layout: simplicity's.** One guide `docs/agents/setup.md`, one skill
   `docs/agents/swarmail/SKILL.md`, one suite `tests/agent_setup_docs.rs`,
   a README edit. No per-client files, no `docs/agents/` index, no change to
   this repository's own `AGENTS.md` or `.claude/`. Rejected: any helper in
   `src/` — it would put lines on the measured path for test convenience.

2. **Fence scanner: simplicity's narrow subset with robustness's
   fail-closed semantics.** A fence is a line that is exactly three
   backticks at column 0 plus an info string; everything else fence-shaped
   (`~~~`, indented fences, four or more backticks) is red, an unterminated
   fence is red, an unknown info string is red, CR anywhere in a guide file
   is red. The allowed info strings are `json`, `toml`, `yaml` (driven as
   wiring), `markdown` (the AGENTS.md block, exactly one per guide), `sh`
   and `text`. No CommonMark parser is regrown (#248 is the precedent) and
   no regex crate is added. **F5 (2026-11-01):** "everything fence-shaped"
   includes a tilde fence at any indentation; the first implementation
   tested `~~~` only at column 0, so `  ~~~json` was silently skipped. Both
   markers are now tested after `trim_start`.

3. **The `not-executed` marker lives on the line immediately above its
   fence, not inside it.** Inside a JSON or TOML fence an HTML comment
   corrupts the very bytes the reader copies. Rules: `<!-- not-executed:
   <reason> -->` with a non-empty reason; a marker not immediately followed
   by a fence is red; a marker on a `json`, `toml`, `yaml` or `markdown`
   fence is red (wiring is always executed — a marker is not an exercise);
   an unmarked `sh` fence must be the start command the suite runs; an
   unmarked `text` fence is red.

4. **Port: ephemeral loopback with one checked substitution, plus a
   default-equality assertion.** Both positions reject binding 8025 (up to
   eight runs share a host under 0057). The suite writes each wiring snippet
   verbatim into a fresh temporary project, reads it back, reduces it to
   `Http(url)` or `Stdio(command, args)`, and asserts the literal against
   the binary's own defaults obtained by parsing `swarmail::config::Cli`
   from `["swarmail", "serve"]` and `["swarmail", "mcp"]` — never a second
   copy of `8025` in test code. Only then is the authority `127.0.0.1:8025`
   replaced by the ephemeral one, in a replacement that fails unless the
   token occurs exactly once. The stdio snippet is executed with its argv
   unchanged and `SWARMAIL_URL` supplied through `Command::env`. The start
   command `swarmail serve` runs with its argv unchanged and the three
   `SWARMAIL_*_LISTEN` knobs set to `127.0.0.1:0`; the bound ports are read
   from the server's own startup line as `tests/binary.rs` does. The same
   rule now covers the SMTP authority the block teaches (ruling 15).

5. **TOML and YAML: no parser, a validating subset reader.** `toml` is in
   neither lockfile and AC-9 bars adding one. TOML: `[table]` headers and
   `key = <JSON value>` lines; YAML: `- <JSON flow node>` rows, the grammar
   `evals/src/wrappers.rs` emits. Blank lines and `#` comments are allowed;
   any other line is red. This validates snippets we author; it is not a
   parser and the guide says so.

6. **Wire client: plain JSON.** The HTTP transport is non-streaming JSON-RPC
   at `POST /mcp`; the client POSTs `application/json` and reads one body.
   The stdio client speaks NDJSON to the child. Each exchange has a
   deadline; each child is killed and reaped on every exit path, stderr
   drained and printed on failure.

7. **Red controls for the docs checks run in-process; red controls for
   wire facts run on the wire with a zero deadline (revised 2026-11-01).**
   The first ruling sent every control through mutated strings. That is
   right for the scanner, reducers, default-equality, tool-name and prose
   pins, and it stays. It cannot show where mail lands: F1 is a wire fact.
   Robustness's controls are adopted for the route — a message sent with no
   AUTH is in `default` and not in the named inbox; a message sent with
   AUTH `other` is in `other` — but with simplicity's discipline: every such
   control is a `swarmail_wait_for_email` with `timeout_ms: 0` on mail that
   is already stored (0001) or provably absent, so no control costs a
   timeout and none is a sleep (0020 §5). The live schema admits
   `timeout_ms: 0` (`minimum: 0`); a timed-out wait is an ordinary text
   result with `timed_out: true`, not an `isError`.

8. **Seed: the transport probe, not the route proof (revised 2026-11-01).**
   The five wiring round trips keep `swarmail_seed_email` after the clear,
   because each proves one snippet's *transport* and the guide teaches the
   seed tool; the spec and the guide now say that is all they prove. The
   taught *loop* and *route* are proved once, over real SMTP, by the test of
   ruling 15 — one proof of the route, not five, because transport and route
   are independent axes (simplicity). Robustness's hostile dot-stuffed DATA
   body and chaos-at-DATA pole are rejected again on a different ground than
   before: the guide claims nothing about dot-stuffing or 4xx retries, and
   `src/smtp.rs`'s tests and `tests/burst.rs` own both; a docs suite proves
   the docs. Cost named: the block stays silent on 4xx retry and the suite
   runs chaos at probability 0.

9. **The 0001 sentence is the verified one.** A `250` (or a successful seed)
   means the mail is queryable now by the running server; when a data file
   is configured the mail is on disk before that reply; without one, a
   restart loses mail. Never re-send after a `250`. The suite now proves the
   first clause on the wire: a `timeout_ms: 0` wait after the `250` finds
   the mail (ruling 15).

10. **Loop teaching (revised 2026-11-01, F2).** The four verbs stay and
    "clear before you act, never between act and extract" stays for a flow
    the agent is about to trigger. The block branches on the entry state
    before the steps: **if the flow has already mailed you, do not clear;
    wait, or read, straight away** — the wait returns mail already stored
    (0001, 0002), and a clear would delete it with no way back (0005). The
    word "always" leaves the order sentence. `stale_code` says "the newest
    mail", not "the newest mail after your clear". Adopt robustness's 0002
    and 0005 sentences unchanged. After a wait timeout the block teaches:
    read the timed-out answer's `near_misses` — mail that arrived but did
    not match, present when the inbox holds any (0111) — then look before
    you re-trigger, because a second trigger makes the first code
    `stale_code`. Cursor-resume and webhook teaching stay rejected.

11. **AC-6 source is `evals/src/state.rs`, read as text.** The closed enum
    is `FailureClass` (eight variants); the six agent-caused names the block
    carries are members and the block names no class outside the enum. The
    root suite does not depend on the evals crate. **F3 (2026-11-01):** the
    `wrong_link` line names `swarmail_wait_for_link`'s selectors
    `anchor_text`, `host` and `path`; `swarmail_extract_links` returns plain
    URLs and has none. The suite reads those three property names from the
    live `inputSchema` of `swarmail_wait_for_link` and asserts the block
    names that tool beside them.

12. **Unverified formats are labelled, not asserted.** Claude Code
    `.mcp.json` (HTTP and stdio), Codex `[mcp_servers.swarmail]` keys and
    dsh's insert row are sourced from `evals/src/wrappers.rs`; the Codex
    file path, Cursor's `.cursor/mcp.json`, the skill install path and the
    skill frontmatter are unsourced and carry a visible "unverified" label.
    CI proves parse and transport only (AC-8).

13. **No TLS snippet, no evals in CI, no scripted docs-following agent.**
    A TLS fixture would expire under a test; evals CI landed through #201
    and is not this change's; a config-reading agent would duplicate the
    oracle to prove what ruling 15's test proves directly.

14. **Out of scope, recorded as residual.** `docs/index.html`'s copy-button
    `.mcp.json` is the landing page, not a guide file. AC-3 (one real model
    wired from the docs) cannot be closed here: the shipped wrappers run
    private configs and refuse a project `.mcp.json` or `AGENTS.md`, and no
    paid trial runs. It stays open and is reported as not measured.

15. **The route is taught in two forms and the universal one is proved
    from the block's own bytes (new, F1).** The block says: use the inbox
    your task or environment names (the eval runner prints `Your Swarmail
    inbox: …` and says to pass it as `"inbox"`); otherwise choose a name of
    lowercase letters, digits and `-` and point the application's SMTP at
    `smtp://<inbox>:x@127.0.0.1:1025` — the AUTH username names the inbox,
    any password is accepted, and mail sent without AUTH lands in `default`
    (0004). The AUTH form comes first because it works on the server the
    guide starts (`swarmail serve` has no route domain). The routed form is
    one conditional clause: on a server started with `--route-domain`,
    `swarmail_create_inbox` mints an address, and mail to it reaches the
    minted inbox whatever the AUTH username (0112). The suite takes the one
    `smtp://` token from the block, asserts its authority equals the parsed
    `serve` default `smtp_listen` (never a second `1025`), substitutes the
    ephemeral SMTP address and the inbox exactly once each, and sends with
    `tests/common::smtp_send` using AUTH equal to the URL's user. Simplicity
    wanted no routed send in this suite; robustness wanted both modes.
    Ruling: both modes are driven, but the routed one is the smallest proof
    that the clause is true (ruling 16) — `tests/inbox_routing.rs` keeps the
    550, case-fold, RSET and concurrency shapes. Simplicity's cost: one more
    server lifecycle (about 40 lines). Robustness's cost: no routed
    behaviour beyond "mint, send, read" is re-proved here.

16. **The routed clause is proved with the guide's start command and the
    documented knob (new, F1).** The same `swarmail serve` argv runs with
    `SWARMAIL_ROUTE_DOMAIN=swarmail.test` in the child's environment (the
    knob the guide names beside the flag; never `set_var`, never a second
    `sh` fence). `swarmail_create_inbox` returns an address; one message
    sent to it with AUTH set to a *different* name arrives in the minted
    inbox on a `timeout_ms: 0` wait. On the default server the same call is
    an `isError` whose text names `--route-domain`, which is why the block
    makes the clause conditional. Rejected: a `swarmail serve --route-domain`
    fence (ruling 3 allows one executed `sh` fence) and any 550 or case-fold
    assertion here (one owner).

17. **The already-mailed path is proved, then its control (new, F2).** On a
    fresh inbox the suite delivers a code over SMTP first and performs no
    clear: a `timeout_ms: 0` wait finds the mail and `swarmail_extract_codes`
    returns the code — the Straight-task entry state on the wire. The
    control: `swarmail_clear_inbox`, then `timeout_ms: 0` is `timed_out`
    with no `near_misses` (the inbox is empty, 0111), which is why the rule
    exists. The block's rule sentence is pinned; removing it goes red.

18. **The tool inventory is equal to the live list, both ways (new,
    F-tools).** The issue says bring the guide current with main's 18 tools
    and that the check must match the live list. Simplicity wanted no
    enumeration and no count; robustness wanted containment of the named
    tools both ways. Ruling: the guide gains one "The tools" section, a
    table with one row per live tool, and the suite asserts the set of
    names in that table **equals** the live `tools/list` — a vanished tool
    and an unlisted new tool are both red, and no number is pinned (the
    `>= 15` floor goes). Every `swarmail_[a-z_]+` token elsewhere in both
    files must be a member of the live list, as before. The block itself
    names only the loop's tools: `swarmail_clear_inbox`,
    `swarmail_seed_email`, `swarmail_create_inbox`, the three waits and the
    two extracts; attachments are inventory rows, not loop steps.
    Simplicity's cost: a new tool needs a table row — which is the point.

19. **The near-miss field name comes from the block and is checked against
    live output (new, 0111).** The block's `wait_timeout` line names the
    field in backticks. The suite performs a real `timeout_ms: 0` wait with
    a non-matching `subject` on an inbox that holds one mail and asserts the
    timed-out answer has a member of exactly that name (and `timed_out:
    true`). Red control: a block naming `near_miss` is refused by the
    in-process pin against the live key set. Not a copied constant in the
    test: the block is the source, the live answer is the authority.

## Requirements

- R1. `docs/agents/setup.md` carries, in this order: starting the server
  (`swarmail serve` as the one executed `sh` fence; install commands marked
  not-executed) followed by a "Where mail lands" paragraph stating the 0004
  and 0112 rules of ruling 15 with the default server's lack of a route
  domain, the `--route-domain` flag and `SWARMAIL_ROUTE_DOMAIN` knob, and
  the `[a-z0-9-]` label alphabet; one section per client — Claude Code
  `.mcp.json` HTTP, Claude Code `.mcp.json` stdio, Codex `config.toml`,
  Cursor `.cursor/mcp.json`, dsh `--patch` overlay; the skill; the
  `AGENTS.md` block as the guide's single `markdown` fence; "The tools"
  table (ruling 18); the guarantees the loop relies on; and "What CI
  proves" (AC-8's boundary, the unverified labels, and the honest sentence
  that the five snippet round trips prove transport while one test drives
  the taught loop over real SMTP). Every URL is `http://127.0.0.1:8025/mcp`;
  the stdio snippet is `{"command": "swarmail", "args": ["mcp"]}` with no
  `env`. The dsh row keeps `toolCallTimeoutMs: 60000`.
- R2. `docs/agents/swarmail/SKILL.md` is YAML frontmatter (`name:
  swarmail`, one-line `description:`) followed by a body byte-identical to
  the guide's `markdown` fence.
- R3. The `AGENTS.md` block, in this order: the one-paragraph purpose; the
  inbox paragraph of ruling 15 (environment-assigned name first; the AUTH
  URL `smtp://<inbox>:x@127.0.0.1:1025` as the one `smtp://` token in the
  block; "Mail sent without AUTH lands in the inbox `default`"; the
  conditional `swarmail_create_inbox` clause naming `--route-domain`); the
  already-mailed rule of ruling 17 in bold; then "Otherwise work in four
  steps, in this order: clear, act, wait, extract" and the four numbered
  steps — **Clear.** (clear before you act, never between act and extract),
  **Act.** (trigger exactly once, pointing the application at your inbox as
  above; seed with `swarmail_seed_email` instead if you must), **Wait.**
  (`timeout_ms` at most 60000; the focused waits; the 0002 sentence),
  **Extract.** (the 0005 sentence); the 0001 sentence; `### Failure classes`
  with exactly the six agent-caused classes `wrong_message`, `stale_code`,
  `wrong_link`, `wait_timeout`, `gave_up`, `tool_misuse`, where `wrong_link`
  names `swarmail_wait_for_link` with `anchor_text`, `host` and `path`,
  `wait_timeout` names `near_misses`, and `stale_code` says "the newest
  mail". The block contains neither "Pick an inbox name that is yours
  alone" nor "always in this order" nor "with a recipient in your inbox".
- R4. `tests/agent_setup_docs.rs` extracts every fence of both guide files
  through one scanner (ruling 2 with F5), drives every wiring fence through
  `initialize`, `notifications/initialized`, `tools/list`, clear, seed,
  wait and extract against the real binary on ephemeral loopback ports
  (the transport probe), and additionally proves on the wire: the AUTH
  route from the block's URL, the already-mailed path and its clear
  control, the near-miss field, the two wrong-route controls, the routed
  clause under `SWARMAIL_ROUTE_DOMAIN`, and `swarmail_create_inbox`'s
  refusal on the default server (rulings 15–19). Per-test inbox and
  temporary directory, deadlines and child reaping as before.
- R5. README: the "Wire it up" paragraph claims only what is proven, in one
  sentence of the form "The wiring snippets in `docs/agents/setup.md` are
  written into a fresh project and driven against the real binary by
  `tests/agent_setup_docs.rs` in CI; install commands are marked
  not-executed." The sentence "Every snippet in it is written into a fresh
  project and driven" is gone. The suite-table row for
  `tests/agent_setup_docs.rs` names the SMTP-driven loop proof and the
  already-mailed case beside the transport round trips.
- R6. `Cargo.toml`, `Cargo.lock`, `deny.toml`, `evals/Cargo.lock`,
  `.github/workflows/*`, `src/**` and the pinned bundle are byte-unchanged.
- R7. The suite's SMTP client is `tests/common::smtp_send` (AUTH PLAIN with
  the username as inbox, `None` for no AUTH); no new SMTP code and no new
  helper is written where an existing one serves.

## Acceptance Criteria

- AC-1 Oracle baseline (seat-run evidence, not docs proof). Every Straight-
  tier task in `evals/src/tasks.rs` passes `swarmail-evals trial --agent
  oracle` offline, exit 0; `--agent null` fails each. The report says this
  proves the tasks on this tree and nothing about the docs.
- AC-2 Wired from the docs — transport. For each of the five wiring fences,
  the suite writes the bytes verbatim into a fresh temporary project, reads
  them back, asserts them against the parsed CLI defaults, substitutes
  exactly one token (HTTP) or supplies `SWARMAIL_URL` by `Command::env`
  (stdio), and completes the R4 round trip with the seeded code extracted.
  This proves each snippet's transport only; the guide says so. Red
  control: a snippet with a changed port, host, path, command or argument
  is refused by the default-equality check.
- AC-3 One real model — open, not measured. No paid trial is run. The
  report records why the shipped wrappers cannot measure docs wiring and
  that the criterion is not passed; the PR does not claim #161 closed.
- AC-4 Snippet coverage, fail closed. Every fence in both guide files is
  executed, is the executed start command, or carries a valid marker on the
  line above. Red controls: unmarked `text` fence; marker with empty
  reason; marker followed by a blank line; marker on a `json` fence; `~~~`
  fence at column 0; **indented `  ~~~json` fence; indented `  ```json`
  fence (F5)**; unterminated fence; unknown info string; CR byte.
- AC-5 Tool-name drift. Every `swarmail_[a-z_]+` token in both guide files
  is a name in the live `tools/list`. Red control: a misspelt name is
  refused.
- AC-6 Loop block. The `markdown` fence names the four steps in order and
  each of the six classes; each named class is a snake_case member of
  `FailureClass` read from `evals/src/state.rs`; no other class token
  outside the enum appears. The SKILL.md body equals the fence byte for
  byte. Red controls: a removed step, a removed class, an invented class.
- AC-7 Guarantee text. Whitespace-normalized pins for the 0001 sentence of
  ruling 9, the 0002 wait-registers-first sentence, the 0005 ingest
  sentence, the clear-before-act rule **and the already-mailed rule**, each
  with a red control that removes it. The guide's guarantees section states
  the clear rule as conditional on not having been mailed already.
- AC-8 Client acceptance. The guide and the report state that CI proves
  parse and transport only for the snippets; Cursor, the Codex file path,
  the skill path and frontmatter are labelled unverified in the guide text,
  and the test pins the presence of those labels.
- AC-9 Gates. Every gate in house rules § Gates is green and locked,
  including the evals crate's fmt, clippy and tests against the candidate's
  own binary, the coverage gate and selftest, publish-guard, seal and deny.
  Coverage stays 100% exact. R6 holds.
- AC-R1 The route (F1). The block carries exactly one `smtp://` token, whose
  authority equals the parsed `swarmail serve` default `smtp_listen` and
  whose user is `<inbox>`. The suite substitutes the ephemeral SMTP address
  and a fresh inbox name exactly once each, sends one message with
  `common::smtp_send` and AUTH equal to the URL's user carrying a six-digit
  code, and — after the `250` — a `swarmail_wait_for_email` with
  `timeout_ms: 0` on that inbox returns it (0001 on the wire) and
  `swarmail_extract_codes` returns the code. Red controls on the same
  server, each a `timeout_ms: 0` wait: a message sent with no AUTH is in
  `default` and not in the named inbox; a message sent with AUTH `other` is
  in `other` and not in the named inbox. Red control in-process: a block
  whose URL authority is `127.0.0.1:1026`, or which carries two `smtp://`
  tokens, or none, is refused.
- AC-R2 Assignment (F1). The block's inbox paragraph begins with the
  environment-assigned rule ("If your task or environment names your
  swarmail inbox, use that name and pass it as `"inbox"` to every tool"),
  pinned; the sentence "Mail sent without AUTH lands in the inbox
  `default`" is pinned; the conditional clause naming `--route-domain` and
  `swarmail_create_inbox` is pinned. Red controls: a block containing "Pick
  an inbox name that is yours alone" or "with a recipient in your inbox" is
  refused; a block missing any pinned sentence is refused.
- AC-R3 Already mailed (F2). The block says, in bold, "If the flow has
  already mailed you, do not clear: wait, or read, straight away." and the
  order sentence contains no "always". On the wire: a code is delivered
  over SMTP to a fresh inbox before any clear; with no clear, a
  `timeout_ms: 0` wait returns it and the extract yields the code. Control:
  after `swarmail_clear_inbox`, a `timeout_ms: 0` wait is `timed_out: true`
  with no `near_misses` member. Red controls in-process: a block without
  the bold rule, or with "always in this order", is refused.
- AC-R4 The loop over SMTP (AC-2's replacement as the loop proof). The
  taught loop — clear, act (the SMTP send of AC-R1), wait, extract — runs
  once end to end over real SMTP on the guide's own server and yields the
  sent code; the seed tool is not on this path. The guide's "What CI
  proves" says the five snippet round trips prove transport and this test
  proves the loop and the route. Red control: the AUTH-mismatch send of
  AC-R1 leaves the named inbox empty.
- AC-R5 Selectors (F3). The block's `wrong_link` line names
  `swarmail_wait_for_link` and the words `anchor_text`, `host` and `path`;
  the suite asserts those three are property names in the live
  `inputSchema` of `swarmail_wait_for_link` and that
  `swarmail_extract_links`'s schema has none of them. Red control: a
  `wrong_link` line that names `swarmail_extract_links` beside the
  selectors is refused.
- AC-R6 README (F4). The README carries the R5 sentence and not "Every
  snippet in it is written into a fresh project and driven"; both pinned,
  the second as an absence.
- AC-R7 Fences (F5). `extract` refuses `"  ~~~json\n{}\n  ~~~\n"`,
  `"  ~~~\n"` and `"  ```json\n{}\n  ```\n"`, each as a row of the
  scanner's negative table, and still accepts the guide's shapes.
- AC-R8 Tools and near misses. "The tools" table's names equal the live
  `tools/list` as a set (on this tree, 18 names including
  `swarmail_create_inbox`, `swarmail_list_attachments`,
  `swarmail_get_attachment`, `swarmail_wait_for_code` and
  `swarmail_wait_for_link`; the number is not pinned). The block names the
  near-miss field in backticks on its `wait_timeout` line; a real
  `timeout_ms: 0` wait with a non-matching `subject` on an inbox holding one
  mail answers `timed_out: true` with a member of that name. Red controls:
  a table missing a live tool or carrying a non-live name is refused; a
  block naming `near_miss` is refused against the live key set.
- AC-R9 The routed clause (F1, 0112). With `SWARMAIL_ROUTE_DOMAIN=
  swarmail.test` in the child's environment and the start command's argv
  unchanged, `swarmail_create_inbox` returns the minted inbox name and its
  address; one message to that address sent with AUTH `other` is found on
  the minted inbox by a `timeout_ms: 0` wait and is not in `other`. On the
  default server `swarmail_create_inbox` is an `isError` whose text names
  `--route-domain`, and the guide says that is why the clause is
  conditional.
- AC-R10 AC-3 not measured. No paid trial; the PR and the implementation
  report say AC-3 is not measured and #161 is not fully closed.
