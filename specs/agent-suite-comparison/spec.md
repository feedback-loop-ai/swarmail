# The agent-suite comparison: Swarmail, Mailpit, MailHog and MailSlurper on one eval suite

Decision: 0129 — [the agent-suite comparison](../../docs/decisions/0129-agent-suite-comparison.md) (proposed)

- **Issue:** #339 (epic #164; prerequisite of #163's paid comparison)
- **Run:** `issue-339-agent-suite-comparison-e4de0540`, from `main` `c95386e`,
  `claude-careful` recipe
- **Authority:** the operator's ruling of 2026-11-07, quoted in #339:
  "compare with all that we have shims for". It sets four targets
  (Swarmail, Mailpit, MailHog, MailSlurper) and four agents (claude-opus,
  claude-sonnet, codex-sol, codex-luna), with no GLM. The paid plan,
  confirmed separately, is the hard tier × 4 targets × 4 agents × 5 trials =
  240 sessions.
- **Accepted rulings this builds on:**
  [0091](../../docs/decisions/0091-fair-mail-comparison.md),
  [0092](../../docs/decisions/0092-official-mail-comparison.md) and
  [0093](../../docs/decisions/0093-source-mail-comparison.md), all accepted
  by the operator on 2026-11-02, plus
  [0076](../../docs/decisions/0076-sequential-eval-trials.md) and
  [0117](../../docs/decisions/0117-hard-eval-tier.md).

## Feature and rationale

On `main`, `scripts/compare-mail.sh controls|evidence|measure` always stop at
`comparison-integration`. Eval tasks T7–T10 were never built. There are
three reasons:

- The eval runner can hand an agent only Swarmail's `POST /mcp`.
- The only competitor adapter, Mailpit's, speaks stdio and refuses batches.
- No product record, controls, contract evidence, identity record, receipts
  or cost basis exists.

This story builds the free half of the comparison. It records the
operator's ruling, selects a target in the runner, adds adapters for
MailHog and MailSlurper, adds batch support to every adapter, and adds each
prerequisite the wrapper checks. It ends with real free controls on all four
targets and one end-to-end `measure --dry-run`. No session that bills a
model runs, and no paid matrix runs.

The comparison is an instrument, so it is held to two requirements. Every
difference between targets is either the variable being measured (the
backend and its tools) or a stated asymmetry. And nothing Swarmail
guarantees is exported to a competitor as an equivalence claim.

### Fairness check (the issue's stop-and-report clause)

There is no conflict between the ask and 0091/0092, so the work goes ahead.
The reasons:

- 0091's synthesis rejects a "MailHog agent eval". The operator overrode
  that for this story on 2026-11-07. A dated addendum records the override
  (R1). It is the operator's ruling, not a weakening by this design.
- MailSlurper is not named in 0091/0092. It enters under 0092 §3's contract
  unchanged.
- The prompt header names the target truthfully. The goal text, scorer,
  trap classes and failure classes stay byte-identical (R6). Giving a
  Mailpit agent the Swarmail header would be the fabrication. 0091 §2 asks
  for "common unchanged v2 business meaning", and the goal and the scoring
  carry that meaning.

If review rules either of these a conflict, the run parks for the operator.
The point is not patched around.

### How the two design positions were ruled

Simplicity argued for the smallest seam: one target switch, one bridge, one
fixture, no `src/` change, no 0129. Robustness argued for a target-typed
tuple frozen in the product record, a bounded batch policy, owned lifecycle,
declared delivery semantics and a ledger that keeps the denominator.
Neither is averaged. The rulings:

| Question | Ruling | Winner | What the loser costs or gives up |
|---|---|---|---|
| Agent transport | A loopback HTTP bridge in the runner process. The runner owns the adapter, the adapter owns the backend through `targets.rs`, and only an adapter exit 0 after full cleanup is a receipt. | Simplicity's bridge, with robustness's ownership | Robustness's separately supervised bridge process is not built. One hop of overhead is stated. Stdio entries in the wrappers are rejected because they would make the agent CLI the backend's owner. |
| Backend choice | A typed `Target` that selects a frozen backend tuple (lock entry, adapter, `tools_sha256`, `smtp_auth`, isolation, connection, wait method, delivery semantics, prompt header). The tuple is frozen in the product record. | Robustness | Simplicity's bare `--target` string grows a closed enum and a table. |
| Fixture per backend | One fixture plus the tuple's `smtp_auth` fact. 0091 §2 forbids an alternate harness. | Simplicity | Robustness's per-backend fixture is read as per-backend fixture *configuration*, not as new scenario code. |
| Isolation | A fresh owned backend per trial, sequential (0076). No emulated inbox. Competitor cells only for one-identity, chaos-free tasks, which is the whole hard tier. | Both agree. Robustness's per-trial inboxes are rejected for competitors. | A competitor has no inbox. A shared mailbox inside one trial is its real model, and the record states it. |
| Batches | Cap 16. Empty or over-cap gets one null-id error and executes nothing, exactly as in `src/mcp.rs`. One admission unit, members in order, per-member budgets, a bounded array, `initialize` refused inside a batch. A hostile-client suite. | Robustness on the refusal, simplicity on the cap | Simplicity's per-overflow correlated errors are rejected as asymmetric with Swarmail. |
| Delivery semantics | Declared per target in the product record. No per-trial `ready_after_accept` measurement. | Robustness on declaring them, simplicity on not measuring | The wire bench already measures waits. A per-trial observation would be a second wait method. |
| MailSlurper provenance | Source-lock **v4**. The v3 bytes and reader are kept. Wire protocol v2 is unchanged. No MailSlurper wire bench. | Intake and robustness | Simplicity's in-place v3 entry is rejected because the v3 reader requires exactly three targets, and changing it changes v3's meaning (0093 §1). |
| Decision record | Proposed 0129 for the new experiment contracts. The ruling itself is in addenda. | Robustness and intake | 0092's context says a thin adapter is a new public experiment interface that needs a proposal. Simplicity's "no 0129" would leave two adapters, a lock version and the mode scopes with no record. |
| Cost basis | `unavailable` with a reason. No usage capture. Every started slot keeps its row, which is proved by a test of the existing matrix journal. | Simplicity on capture, robustness on the denominator | Robustness's new start-of-trial ledger is not built, because the matrix already writes a row per slot, and a test proves it instead. |
| Comparison row | Schema 3 keeps its shape. The coordinate gains `agent`. Four target names. The MailHog refusal is removed. | Neither: both proposed more | Simplicity's embedded full v2 row is rejected because it would carry Swarmail-specific target identity into a competitor row (0091 context). |
| `measure --dry-run` | On genuine prerequisites it verifies contents, prints the 240-slot plan, launches nothing and exits 0. CI-only or operator-only items print as pending. Non-dry measure refuses a pending item and refuses without `--authorized-product`. | Simplicity's passing dry run, robustness's refusal of the paid start | Intake's always-refusing dry run cannot be the issue's "prove it". The prerequisites are produced for real, never stubbed. |
| Hostile DATA, cert expiry, webhook, chaos campaigns | Not built. None is on the hard-tier path: loopback plaintext SMTP, no webhook, no chaos. The adapters' bounds refuse and never repair. | Simplicity | The comparison claims nothing about fault tolerance. 0091's synthesis already rejects a cross-product fault suite. |

## Requirements

### R1. Record the ruling

- Add a dated addendum (2026-11-07) to accepted 0091 and to accepted 0092.
  Each quotes the ruling and states:
  - the four targets and four agents, and that there is no GLM;
  - that 0091's rejection of a "MailHog agent eval" does not apply to this
    story;
  - that MailSlurper joins under 0092 §3 unchanged;
  - (0092 only) that this story closes its batch gap;
  - a pointer to proposed 0129.
- The text above each addendum stays byte-identical.
- In `docs/evals/README.md`, 0091–0093 are stated as accepted by the
  operator on 2026-11-02, not as proposed.
- The decision index gains the proposed 0129 row.

### R2. Targets in the eval crate

- `evals` gains a closed `Target` enum: `swarmail`, `mailpit`, `mailhog`,
  `mailslurper`. It selects a backend tuple.
- `matrix --target T` selects the backend. An unknown value is refused by
  name before anything launches.
- Without `--target`, `matrix` behaves byte-identically to today and writes
  native v2 rows.
- With `--target`, `matrix` writes schema-3 comparison rows only.
- A competitor target requires `--adapter PATH` (the `mail_compare`
  executable), `--target-bin PATH` and the lock. It refuses by name in each
  of these cases:
  - `--brokkr`;
  - the dsh wrapper;
  - a task the product record declares not applicable for that target.

### R3. The bridge

- `evals/src/bridge.rs` is a loopback-only Streamable HTTP `POST /mcp`
  endpoint, served inside the runner process.
- Each request body is written as one line to the owned adapter's stdin.
  The replies are routed back by id. A batch is forwarded as one line and
  its array reply is returned as one body.
- A body holding only notifications gets `202`. `GET` gets `405`.
- The same Host/Origin guard as Swarmail's endpoint applies (0043).
- An id already in flight gets a correlated `-32600`.
- An adapter that has exited yields `503`. The trial then ends as a visible
  harness failure.
- The bridge logs each session's `clientInfo` and `protocolVersion` into the
  trial's private artifacts. It logs no bodies (0075).

### R4. Runner server block per target

- `swarmail` keeps its existing arm.
- A competitor spawns `mail_compare <t>-mcp --launch BIN --sources LOCK` as
  an owned process. That process launches the backend through the existing
  `targets::Owned` launcher, prints one announcement line (`smtp=…
  tools_sha256=…`) and then serves MCP on stdio.
- When stdin closes, the adapter runs the existing cleanup. It exits 0 only
  when the cleanup facts are all true.
- Any other end makes cleanup unknown and stops admission.
- The announced `tools_sha256` must equal the product record's value, or the
  trial refuses.
- The fixture sends to the announced SMTP address with the target's
  `smtp_auth`.

### R5. Adapters

- `benches/mail_compare/mailpit_mcp.rs` is split into a generic `mcp.rs` and
  a Mailpit backend. The generic half holds bounds, admission, line reads,
  frame and poll wait, scan, budget, `serve`, reply shaping, launch mode and
  batches. The backend interface is `tools(&Bounds)`, `parse_call` and
  `run_call`.
- Mailpit's `tools()` output stays byte-identical, and its digest equals
  protocol v2's `budgets.adapter.tools_sha256`.
- `mailhog_mcp.rs` and `mailslurper_mcp.rs` follow the same interface. They
  expose list/search/read and a labelled poll wait, but only over operations
  whose route and response shape are verified against source bytes the lock
  binds. Each tool's description names its upstream route.
  - An operation upstream does not serve is not offered.
  - Nothing Swarmail-only is emulated: no inbox, no extraction, no
    `await`, no chaos, no delete or admin, no hints.
  - Upstream bodies pass through unchanged, and MailHog's raw MIME stays
    raw.
- Every adapter answers batches under 0129 §6.

### R6. Prompt and wrappers

- `PROMPT_TEMPLATE` stays byte-identical for Swarmail.
- A competitor template renders the same `{goal}` with a header that names
  the target, states one mailbox per trial, and drops the inbox line and the
  `"inbox"` argument instruction.
- Both headers' digests are frozen in the product record.
- For competitor targets the runner passes `EVAL_MCP_SERVER` (the server
  key) and `EVAL_MCP_TOOLS` (the adapter's own announced tool names). The
  claude allowlist and the codex server table are built from them.
- `SWARMAIL_TOOLS` stays for the Swarmail target.
- The dsh wrapper refuses a competitor target by name.

### R7. Oracle per backend

- The oracle's mail access sits behind a `MailSource` trait. The Swarmail
  implementation is today's calls, unchanged.
- Each competitor implementation calls only that adapter's own tools. It
  polls the adapter's list or search until the count is met, reads the
  messages, and maps them to `{to, subject, text, html}`.
- The recipient filter uses the backend's own source-verified filter where
  upstream has one. Where it has none, the oracle filters the complete
  listing itself, and the product record states this.
- MIME decoding needed for MailHog's raw bodies (7bit/8bit,
  quoted-printable, base64, multipart/alternative) lives in the oracle,
  which is a control. It does not live in the adapter. An unknown shape is
  an oracle error, never a pass.
- No task logic is duplicated, and no dependency is added.

### R8. MailSlurper provenance and AUTH PLAIN

- MailSlurper's latest official release is pinned in source-lock v4. The
  entry carries:
  - the asset digest (upstream if published, otherwise locally computed and
    labelled);
  - the executable digest;
  - the tagged source chain and tree;
  - the licence, README and service-API/SMTP source bytes, bound by size,
    SHA-256 and blob id;
  - build provenance, unresolved;
  - a launch config template if MailSlurper reads its ports from a file.
- Lock v3's bytes move to `docs/evals/history/2026-11-07-sources.lock.v3.json`
  with a v3 reader. The v1, v2 and v3 readers keep a three-target
  expectation.
- Whether MailSlurper accepts the fixture's AUTH PLAIN is answered twice:
  once from its reviewed SMTP source, and once by an unscored setup probe
  against the pinned binary. The answer is written to the lock finding, the
  product record's `smtp_auth`, the row settings and the README.
- Every download goes into its own new directory and is verified by digest
  before use. Python runs only with `-I`. If the bytes cannot be obtained,
  the MailSlurper items are reported blocked. They are never built from
  memory.

### R9. The prerequisites and the wrapper

- `docs/evals/agent-comparison.json` is the frozen product record (0129 §8).
  It is committed before the controls run.
- `docs/evals/agent-comparison-cost.json` is
  `{"basis":"unavailable","reason":…}`.
- `compare-mail.sh controls --output DIR --product P --mailpit B --mailhog B
  --mailslurper B` runs, per target and sequentially, `matrix --target T
  --tier hard` with the builtin `oracle` and `null` agents for the product's
  trial count. It writes:
  - the rows;
  - an index bound to the product digest;
  - per-target contract evidence: negotiated `protocolVersion`,
    capabilities, `tools/list` digest and a batch probe (array in, array
    out);
  - `identity.json`, with every measured executable and the adapter checked
    against the lock and the product;
  - `receipts.json`: HEAD, a digest of the per-slot cleanup and admission
    receipts, the locked eval-suite command and its status, and
    `ci: {"status":"pending"}` or an operator-supplied run reference.
- `swarmail-evals comparison-check` verifies contents and cross-bindings and
  names the first failure:
  - the baseline is the published #151 report;
  - the contracts and controls come from one controls dir bound to this
    product;
  - the receipts' HEAD equals the candidate;
  - the cost basis is genuine;
  - identity re-measures equal;
  - the product is valid.
- `evidence` and `measure` keep the fixed stat order, run the verifier and
  then delegate. The `comparison-integration` refusal is removed.
- `measure --dry-run` follows 0129 §9.
- Non-dry `measure` requires `--authorized-product ID` equal to the product
  id, refuses a pending item by name, and delegates `matrix --target …` per
  target in product order.

### R10. Rows and the denominator

- In `ComparisonRow`, the coordinate gains `agent`.
- `COMPARISON_TARGETS` holds four entries. Swarmail's connection is
  `native-mcp`. Each competitor's is `bridged-stdio-adapter`.
- The `mailhog` refusal is deleted. A bare or unknown name is still
  refused.
- Settings gain `smtp_auth`.
- A trial killed mid-run keeps its row as a visible failure.
- `report::cite_comparison` keys on (target, agent, task).

## Acceptance Criteria

- **AC1 — ruling recorded.**
  - `git diff c95386e -- docs/decisions/0091-fair-mail-comparison.md
    docs/decisions/0092-official-mail-comparison.md` shows only added
    lines, each in a section headed "Dated addendum — 2026-11-07" that
    quotes "compare with all that we have shims for".
  - `docs/evals/README.md` no longer calls 0091–0093 "proposed".
  - The decision-index test (`tests/store_contract_docs.rs`) passes with the
    0129 row.
- **AC2 — targets.**
  - `matrix --target bogus` exits non-zero naming the value, and launches
    nothing.
  - Each of the following refuses by name: `--target mailhog --brokkr`, a
    dsh profile with a competitor, and a non-applicable task with a
    competitor.
  - A `matrix` run without `--target` writes rows byte-identical in shape to
    `main`'s, and the existing suite stays green.
  - A sealed MailHog comparison row validates. A row with target `mailhog`
    or an unknown target is refused. Native readers refuse schema-3 rows.
- **AC3 — bridge.**
  - An evals test drives the bridge against the real `swarmail mcp` stdio
    transport (`SWARMAIL_BIN`, no download): `initialize`, `tools/list`, a
    tool call, a batch, a notifications-only `202`, `GET` `405`, a foreign
    Host refused, a duplicate in-flight id `-32600`, and an exited child
    `503`.
  - A runner test proves the adapter's cleanup receipt: exit 0 is a
    receipt. A non-zero exit and a kill each stop admission.
- **AC4 — adapters.**
  - Mailpit's `tools()` digest equals protocol v2's
    `budgets.adapter.tools_sha256`.
  - Every existing `tests/mail_compare.rs` test passes.
  - The MailHog and MailSlurper adapters pass mechanics tests over real
    SMTP against Swarmail's own MailHog and MailSlurper compat shims, in
    CI. Their ignored tests pass against the pinned upstream executables
    (`MAIL_COMPARE_MAILHOG`, `MAIL_COMPARE_MAILSLURPER`).
  - Each tool's upstream route is cited and bound in lock v4.
  - No tool name or description offers inbox, await, extraction, delete or
    admin.
- **AC5 — batches.** A hostile-client suite runs on the generic half:
  - an array of 1 and of 16 members gets an ordered array;
  - 17 members, and an empty array, each get one null-id error with no call
    executed (asserted by an upstream request counter);
  - notifications-only writes nothing;
  - `initialize` inside a batch gets `-32600`;
  - duplicate ids are answered as sent;
  - an over-bound reply gives that member `reply_too_large`;
  - a full queue gives busy errors for all members;
  - a 64 KiB line is refused;
  - EOF mid-batch ends cleanly.
- **AC6 — MailSlurper.**
  - Lock v4 validates, and `mail_compare check --protocol
    docs/evals/protocol.json --sources docs/evals/sources.lock.json` passes.
  - `docs/evals/protocol.json` is byte-identical to `c95386e`.
  - The archived v3 lock is byte-identical to `c95386e`'s
    `docs/evals/sources.lock.json`, and its reader accepts it.
  - The v1/v2 reader tests are unchanged and pass.
  - The AUTH PLAIN answer is stated, with its source citation and probe
    transcript, in the README, the lock finding and the product record.
- **AC7 — oracle and null controls.** `compare-mail.sh controls` on the
  real pinned executables of all four targets gives:
  - every oracle trial passes, and every null trial fails with the task's
    idle class;
  - for each (task, seed), the fixture's public projection digest is the
    same on all four targets;
  - the index binds the product digest.
  The controls output is committed under `docs/evals/agent-controls-2026-11-07/`
  (rows, index, contracts, identity, receipts). Each per-target run is a
  named, timed command of at most 10 minutes with one worker, and the load
  before and after is reported.
- **AC8 — prerequisites.**
  - `controls` runs with only `--product` and the executables.
  - `evidence` and `measure` refuse, by the name of the first prerequisite
    in the fixed order, when it is missing; when its contents fail the
    verifier (one negative test per prerequisite); and when the receipts'
    HEAD differs or identity re-measurement differs.
  - The string `comparison-integration` is gone from `compare-mail.sh`.
- **AC9 — dry run.** One `compare-mail.sh measure --dry-run` with every
  prerequisite produced for real (no stub file) exits 0. It prints the
  240-slot plan, plus each pending CI-only or operator-only item. It starts
  no process other than the verifier, which a test asserts, and it does not
  open `--config`. The transcript is committed beside the controls.
- **AC10 — no paid start.**
  - Non-dry `measure` without `--authorized-product`, with a wrong id, or
    with a pending item, is refused by name, and no agent is spawned.
  - The delegation is proved only with an inert config whose agents are
    builtin `null`, on a synthetic one-trial product in a temp dir.
  - No paid session runs in this story.
- **AC11 — denominator and cost.**
  - A test kills a competitor trial's adapter mid-trial. The slot keeps a
    row as a visible failure, and admission stops until cleanup is
    confirmed.
  - The cost record is `unavailable`. No row or report writes a cost of 0.
- **AC12 — scope.**
  - `git diff c95386e -- src/` is empty.
  - `Cargo.lock`, `evals/Cargo.lock` and `deny.toml` are unchanged.
  - No file under `.github/` or `.brokkr/` changes.
  - The whole house gate inventory passes `--locked`, including the evals
    crate against the candidate's `swarmail`, the coverage gate and its
    selftest, the publish-guard and seal proofs, and `cargo deny check
    licenses`. If the sandbox has no brokkr binary, the bundle-compile gate
    is reported as run by verify.

## Non-goals and deferrals

- No paid session, no paid matrix and no comparison claim. #163 runs the 240
  sessions after the operator confirms them.
- No wire bench for MailSlurper, no wire protocol v3, and no best-wait
  stream client (0093 §4 keeps it blocked).
- No competitor cells outside the hard tier. No dsh/GLM support for
  competitors: spark is down, and the DashScope quota is out until
  2026-11-22.
- No `.github/` change. CI keeps its jobs. Upstream-backed proofs are
  ignored tests plus the committed controls receipts, as in #287.
- Real model-CLI negotiation against the bridge is not proved free. It is
  logged in the first paid session's trial artifacts, and a failure there
  is a visible `harness_error` row.

## Open items for the operator (not blocking the design)

- The dry-run reading (exit 0 on genuine prerequisites, with pending items
  printed) is ruled here under 0129 §9. Intake flagged it for confirmation.
- The `controls` scope (product record and executables only) is ruled
  here. Intake flagged it too.
- Acceptance of 0129 is the operator's. The PR is held regardless, because
  `scripts/compare-mail.sh` changes.
