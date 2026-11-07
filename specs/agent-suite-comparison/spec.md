# The agent-suite comparison: Swarmail, Mailpit, MailHog and MailSlurper on one eval suite

Decision: 0129 — [the agent-suite comparison](../../docs/decisions/0129-agent-suite-comparison.md) (proposed)

- **Issue:** #339 (epic #164; prerequisite of #163's paid comparison)
- **Run:** `issue-339-agent-suite-comparison-e4de0540`, from `main` `c95386e`,
  `claude-careful` recipe. Continued by the rerun
  `rerun-of-339-operator-direction--305a5aec` (operator direction
  2026-11-07) on `repair/issue339-compare-agents` from `51c9f98`, with the
  pinned upstream bytes now present under `.compare-bins/` (section
  "The 2026-11-07 rerun").
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

## The 2026-11-07 rerun

The first run built most of this spec (ten commits, `d5961bb`..`51c9f98`)
and stopped on one missing input: the pinned upstream bytes. The operator
put them in the worktree under `.compare-bins/`. That directory is
git-ignored, is never committed, and is untrusted data. The rerun finishes
the spec; it does not redesign it. Every requirement below stands unless
this section or a requirement marked *(rerun)* changes it.

The bytes raise no new fairness conflict with 0091/0092. MailSlurper
differs from the other targets in three ways: it has no AUTH, its recipient
filter is a substring match, and it stores after the accept. Those
differences are the backend variable being measured. They are stated in the
record, the rows and the README, and the adapter emulates nothing to close
them. 0129 §2 already provides `smtp_auth: none`. So the stop-and-report
clause is not triggered.

### What the pinned bytes establish

Every fact here was read from the bytes on 2026-11-07, by digest and by path
relative to the worktree. Archives were listed or read with `tar -O` /
`unzip -l` and nothing was extracted or executed. Python ran with `-I`. No
upstream fact is written from memory, and the implementer re-reads each one
from the bytes before it enters the lock.

- **Mailpit and MailHog match lock v3.** In `.compare-bins/2026-11-07/`:
  - `mailpit-linux-amd64.tar.gz` has sha256
    `4b5993c978e8604c70d43927e2ad103b311bba3b1822fa0cf1d4ee4ec890a93c`;
  - `mailpit` has sha256
    `262cfa28fae4554be198d1fea5ef83c0f8299317876d658623711c629a8edec8`;
  - `MailHog_linux_amd64` has sha256
    `e2ed634ded49929f089b20045581955ed217672078fd86082dd7a6c67c5d09c7`.

  These are lock v3's artifact and executable digests. That directory
  carries no digest file of its own, so the lock is the expected value.
- **MailSlurper release.**
  `.compare-bins/2026-11-07-mailslurper/mailslurper-1.15.0-linux.zip` has
  sha256 `48f2f9ba090f5385301477406b4f6e08f7bfbba0619c635ee591d6e6af3e55f6`
  and size 6574992.
  - The release metadata,
    `.compare-bins/2026-11-07-meta/mailslurper_mailslurper-releases.json`,
    lists `release-1.15.0` as the newest release (published
    2023-08-03T20:26:53Z, not prerelease, not draft). It publishes no asset
    digest. So the asset digest is `locally_computed`, as MailHog's v3
    entry already is.
  - The zip holds `LICENSE`, `README.md`, `config.json`, two SQL files,
    `createcredentials`, `logo.png` and `mailslurper`. The member run is
    `mailslurper`, and its executable digest is computed at lock time from
    a fresh extraction. `createcredentials` is never run.
- **MailSlurper source.**
  - `.compare-bins/2026-11-07-mailslurper-src/mailslurper-release-1.15.0-src.tar.gz`
    has sha256
    `83f66f25ac000fc2f5680514719b7b75a29dafccb7e782597077325fb900359f`.
    That equals the hex in the adjacent `SHA256SUMS`. That file names an
    absolute path on another machine, so the hex is compared as a constant
    and `sha256sum -c` is never run against it.
  - `.compare-bins/2026-11-07-meta/mailslurper_mailslurper-ref.json` gives
    `refs/tags/release-1.15.0` → object type `commit`,
    `068acffdcd8960f248ea438c3b7ee51ead601bd2`. It is a lightweight tag,
    so no tag object exists to capture.
  - No commit object and no tree listing were captured.
- **No SMTP AUTH.**
  - `pkg/mailslurper/SmtpCommands.go` maps only `helo`, `ehlo`, `rcpt to`,
    `mail from`, `send`, `rset`, `quit`, `data` and `noop`.
  - In `pkg/mailslurper/SMTPWorker.go`, an unknown command (lines 149–152)
    goes to the worker's error channel. The worker then calls `SayGoodbye`,
    which sends `221 Bye`, and closes the connection (lines 190–196;
    `pkg/mailslurper/Writer.go:30–31`; `SMTPWorkerResponses.go:14`).
  - EHLO is answered by the single line
    `250 Hello. How very nice to meet you!` (`SMTPWorkerResponses.go:17`),
    which advertises no extension.

  So the fixture's AUTH PLAIN would end the session. The source answer is
  `smtp_auth: none`, and R8 requires the probe to confirm it.
- **Accept precedes storage.** `pkg/mailslurper/DataCommandExecutor.go:123`
  sends the `250` inside `Process`. Only after `Process` returns does
  `SMTPWorker.go` (lines 216–219) put a copy on the `Receiver` channel, and
  `DatabaseReceiver.Receive` stores it. The declared delivery semantics are
  `accept_implies_queryable: not-promised`.
- **Three listeners, one config file.**
  - `cmd/mailslurper/mailslurper.go:50` declares `-config` ("Absolute
    location of the config.json").
  - Lines 69–72 always set up the database, SMTP, the admin (www) listener
    and the service listener.
  - `cmd/mailslurper/config.json` names `wwwAddress`/`wwwPort`,
    `serviceAddress`/`servicePort` and `smtpAddress`/`smtpPort`. It also
    names `dbEngine: "SQLite"` with a relative `dbDatabase`, and
    `authenticationScheme: ""` (service auth off).
- **Service routes** (`cmd/mailslurper/setupServicesListener.go:51–59`):
  - `GET /mail/:id`, `GET /mail/:id/message` and `GET /mail/:id/messageraw`;
  - `DELETE /mail`;
  - `GET /mail`, `GET /mailcount` and the attachment route;
  - `GET /version` and `GET /pruneoptions`.
- **List and filter semantics.**
  - `GET /mail` pages a fixed 50 items (`controllers/ServiceController.go`,
    around line 140).
  - The `message`, `from` and `to` filters are SQL `LIKE '%term%'`
    substring matches. `message` matches body or subject, `to` matches
    `toAddressList` (`pkg/mailslurper/CommonQueries.go:146–171`).
  - `GET /mail/:id` base64-decodes `body` when it can, and copies the
    result into `HTMLBody` (`ServiceController.go` `GetMail`).
  - A mail item is JSON with one `body` string beside `contentType`,
    `boundary` and `transferEncoding` (`pkg/mailslurper/MailItem.go`).
- **Not present.** The framing names `.compare-bins/2026-11-07-jre/` and
  `.compare-bins/2026-11-07-alkemio-mailslurper/`. Neither exists in this
  worktree. Neither is needed: the JRE serves only GreenMail, and MailSlurper
  runs from its own release zip with `-config`.

### How the rerun's positions were ruled

Both positions agree on the frame: finish the committed design, keep the four
targets, no `src/` change, no dependency, nothing paid. They differ on how
much proof the new boundary needs. The rulings:

| Question | Ruling | Winner | What the loser costs or gives up |
|---|---|---|---|
| Scope of the rerun | Finish the committed spec. Tick the built tasks with their commits. Replace the README's "Not built, and why" block with the outcome rather than adding a second block. | Simplicity | Robustness's freeze chain is adopted only where a row below adopts it. |
| Byte verification | The hex is compared to an expected constant, never `sha256sum -c` against a foreign path. Archive *and* member digests. A recorded `digest_basis`. A mismatch stops the run and is reported; the lock is never adjusted to the bytes. | Robustness | None. Simplicity read the same digests. |
| MailSlurper tree | The tree stays **unresolved**, with its reason. Source captures are attributed `from: "archive"`, carrying the archive digest and its pax commit comment. Each capture is bound by size, sha256 and a locally computed blob id. No `git_tree` is synthesised from a tarball. Only the cited files are captured. | Simplicity | The commit→tree link is unproved, and that is labelled. Fetching a tree needs network access this seat does not hold. |
| AUTH PLAIN | The source answer (`none`) must be confirmed by a probe against the pinned binary. A probe that is inconclusive or disagrees with the source refuses to freeze the product record, and the MailSlurper items report blocked. | Robustness on the refusal, simplicity on one ignored test | The probe is a gate, not a footnote. |
| Tool surface | Every announced tool is called once, read-only, against each pinned binary by the existing ignored real-binary tests, before the product record is committed. A failing tool is removed from the adapter first, never emulated. The controls' contract evidence repeats one call per tool. | Robustness on the probe, simplicity on no separate tool | The record cannot freeze until the ignored tests have run against the pins. |
| CI execution of real upstream shapes | A bounded **wire corpus**. The ignored real-binary tests capture each competitor's responses to fixture-generated mail (list, read, empty result, not-found, and a filtered search). They are committed under `docs/evals/wire-corpus/<target>/`, with an index of sha256s and the capture's executable digest. CI tests decode the corpus through the adapter's decoders and the oracle's normalizers on every run. | Robustness | Simplicity pays for about fifteen small JSON files and two tests, and the corpus is recaptured whenever a pin moves. Swarmail's shims stay the CI mechanics; the corpus catches drift between a shim and upstream. |
| Hostile tool arguments | Ids are validated (length and charset) and path-encoded. Over-cap sizes are refused, not clamped. No URL comes from an argument. Redirects are off. Each rule is tested on the generic half and on every backend. | Robustness | A few tests. Anything already enforced is proved, not rebuilt. |
| Bridge | Already bounded: body ≤ 64 KiB, batch ≤ 16, a 120 s reply deadline, one writer channel, and each line re-serialised from parsed JSON so no embedded newline survives. Two additions: a bound on requests in flight, and adapter stderr going to a private trial file, never an undrained pipe. | Robustness, partly | Robustness's separate cold-path catalogue is reduced to those two gaps plus a test with a pretty-printed body. |
| Ledger | No new start-of-trial ledger (the earlier ruling stands). `comparison-check` refuses a controls directory whose row count differs from its planned slots, and refuses a torn final line by name. | Simplicity, with a denominator check | If the runner process itself crashes, its controls are rerun (they are free), not salvaged from a torn file. |
| Receipts | HEAD-bound, with the ancestry rule in R9. Evidence is never spliced across product ids or HEADs. | Robustness | None. |
| Extra shelf upstreams | smtp4dev, Inbucket and GreenMail are not used. They are outside the four named targets, and Swarmail has no shim for them. | Both | The report says they exist and are unused. |
| Wrapper usage capture | None. The cost basis is `unavailable`. | Both | — |

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
- *(rerun)* At most 16 requests are in flight. Over that bound, a request
  gets a correlated JSON-RPC busy error and nothing is forwarded. A body
  that arrives pretty-printed (with embedded newlines) is forwarded as one
  re-serialised line.

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
- *(rerun)* The adapter's stderr goes to a file in the trial's private
  artifacts, never to an undrained pipe.
- *(rerun)* MailSlurper's launch reserves a **third** owned port (www),
  because its admin listener always starts. An unowned bind would escape
  the cleanup receipt. The launcher fills `{www_addr}`/`{www_port}` only
  when a template names them. It writes the lock-bound config template
  into the trial's scratch, with the SQLite database under `{data_dir}`,
  service auth off and the browser off. It runs
  `mailslurper -config <absolute path>` with the scratch as its working
  directory. The Mailpit and MailHog launches are unchanged.

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
- *(rerun)* MailSlurper's adapter offers:
  - list: `GET /mail?pageNumber=N`, pages of 50, upstream's fixed size;
  - search: the `to`, `from` and `message` filters of the same route;
  - read: `GET /mail/:id`;
  - the labelled poll wait.

  It does not offer `DELETE /mail`, prune options, attachments,
  login/logout or version. The search tool's description states upstream's
  semantics truthfully: a SQL `LIKE` substring match, with the term wrapped
  in `%…%` and not escaped (`CommonQueries.go`). The adapter neither narrows
  nor escapes the term. That is upstream's real filter, and the record
  states it.
- *(rerun)* Hostile arguments, on every adapter:
  - an id longer than 128 bytes, or outside `[A-Za-z0-9._-]`, is refused
    before any request, and an accepted id is path-encoded;
  - an over-cap page, count or byte size is refused, never clamped;
  - no tool argument names a URL or host;
  - the upstream client follows no redirect;
  - an unknown or oversized upstream JSON shape is a typed adapter error,
    never a panic.

  Where an upstream id can fall outside that charset, the implementer
  widens the charset to the observed id alphabet, cited from the wire
  corpus. The id is never percent-decoded or passed through raw.

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
- *(rerun)* MailSlurper's normalizer reads `GET /mail/:id`'s `body` according
  to its `contentType`. Where the body is multipart, it reuses the MIME
  decoder already in `evals/src/mailsource.rs` and adds no second one. The
  oracle uses the upstream `to` filter only as a pre-filter. It then keeps
  only messages whose `toAddresses` contains the recipient **exactly**,
  because the upstream filter is a substring match. A wire-corpus test over
  real MailSlurper responses decides which shapes occur; nothing is
  assumed.

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
- *(rerun)* The bytes are the ones in "What the pinned bytes establish". The
  MailSlurper v4 entry records:
  - `release-1.15.0`, the zip digest `48f2f9ba…55f6` with
    `digest_basis: "locally_computed"` and `upstream_digest: null`;
  - `artifact.kind: "archive"`, `member: "mailslurper"`, and the member's
    `executable_sha256`, computed from a fresh extraction into a new
    directory;
  - the tagged source as observed: `tag_kind: "lightweight"`,
    `tag_object: null`, commit `068acffd…1bd2`;
  - the source archive, `83f66f25…359f`;
  - the tree as **unresolved**, with the reason "no commit object or tree
    listing was captured; the archive's pax comment names the commit but
    does not prove its tree";
  - build provenance, unresolved;
  - the licence SPDX id as `LICENSE`'s text decides it;
  - the launch config template;
  - the AUTH finding: "MailSlurper 1.15.0 has no SMTP AUTH command; the
    fixture omits the AUTH step for it; every other target receives AUTH
    PLAIN".
- *(rerun)* `docs/evals/sources/mailslurper/` commits only the cited files:
  - `LICENSE` and `README.md`;
  - under `cmd/mailslurper/`: `mailslurper.go`, `config.json`,
    `setupServicesListener.go` and `controllers/ServiceController.go`;
  - under `pkg/mailslurper/`: `SmtpCommands.go`, `SMTPWorker.go`,
    `SMTPWorkerResponses.go`, `Writer.go`, `HelloCommandExecutor.go`,
    `DataCommandExecutor.go`, `DatabaseReceiver.go`, `CommonQueries.go`,
    `MailSearch.go`, `MailItem.go` and `MailCollectionResponse.go`;
  - the release and ref metadata.

  Each is bound by size, sha256 and locally computed blob id. `vendor/`,
  `www/` and the tarball itself are never committed.
- *(rerun)* The v4 reader is the v3 reader with an expected-target list.
  v1–v3 pass the three-target list and v4 passes the four. No reader code is
  copied.

### R9. The prerequisites and the wrapper

- `docs/evals/agent-comparison.json` is the frozen product record (0129 §8).
  It is committed before the controls run. *(rerun)* It is also committed
  only after two things have passed against the pinned executables: the
  ignored real-binary adapter tests (every announced tool called once) and
  the MailSlurper AUTH probe. Its `smtp_auth`, `tools_sha256` and delivery
  semantics are copied from those results, never written ahead of them.
- `docs/evals/agent-comparison-cost.json` is
  `{"basis":"unavailable","reason":…}`.
- `compare-mail.sh controls --output DIR --product P --mailpit B --mailhog B
  --mailslurper B` runs, per target and sequentially, `matrix --target T
  --tier hard` with the builtin `oracle` and `null` agents for the product's
  trial count. It writes:
  - the rows;
  - an index bound to the product digest;
  - per-target contract evidence: negotiated `protocolVersion`,
    capabilities, `tools/list` digest, a batch probe (array in, array
    out) and *(rerun)* one read-only call per announced tool, each with its
    outcome;
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
  - the receipts' HEAD is the candidate, or an ancestor of it whose diff to
    the candidate touches only `docs/evals/agent-controls-*/`,
    `docs/evals/README.md` and `specs/`. *(rerun:* committing the evidence
    necessarily moves HEAD. A change to any code, lock, product or script
    path makes the receipts stale, and they are refused.*)*
  - the controls rows number exactly the product's planned control slots,
    and the file has no torn final line *(rerun)*;
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
  - *(rerun)* The 17th concurrent in-flight request gets a correlated busy
    error and is not forwarded. A pretty-printed request reaches the
    adapter as one line. The adapter's stderr lands in the trial's
    private file.
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
  - *(rerun)* Each ignored real-binary test calls every announced tool once
    against its pin. Those tests passed before `agent-comparison.json` was
    committed, and their log is part of the controls evidence.
  - *(rerun)* `docs/evals/wire-corpus/{mailpit,mailhog,mailslurper}/`
    exists. Its index's sha256s match the files, and its executable digests
    match the lock. CI tests decode every file through the adapter decoders
    and the oracle normalizers.
  - *(rerun)* Each hostile-argument rule of R5 has a test on every adapter.
    An upstream request counter shows that a refused call made no request.
  - *(rerun)* MailSlurper's search description states the `LIKE` substring
    semantics. An oracle test shows that a lookalike recipient returned by
    the upstream `to` filter is excluded by the exact match.
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
  - *(rerun)* The answer is `none`, as the source predicts. The record
    refuses to freeze if the EHLO reply advertises AUTH. It also refuses
    unless the probe result is "the `AUTH PLAIN …` line drew `221 Bye`, and
    the server closed the connection, with no `235` and no `334`".
  - *(rerun)* The lock binds:
    - the zip digest `48f2f9ba…55f6` (`locally_computed`);
    - the extracted member's digest;
    - the source archive `83f66f25…359f`;
    - the lightweight tag at `068acffd…1bd2`;
    - an unresolved tree, with its reason.

    The blob, size and sha256 checks pass over every capture.
  - *(rerun)* A MailSlurper launch test proves three owned binds, and that
    cleanup releases all three.
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
    builtin `null`, on a synthetic one-trial product in a scratch dir on
    `/home` (never the `/tmp` tmpfs).
  - No paid session runs in this story.
- **AC11 — denominator and cost.**
  - A test kills a competitor trial's adapter mid-trial. The slot keeps a
    row as a visible failure, and admission stops until cleanup is
    confirmed.
  - The cost record is `unavailable`. No row or report writes a cost of 0.
  - *(rerun)* `comparison-check` refuses, each by name:
    - a controls directory with one row missing;
    - one with a torn final line;
    - receipts whose HEAD is not an ancestor of the candidate;
    - receipts whose diff to the candidate touches a path outside the
      evidence allowance.
- **AC12 — scope.**
  - `git diff c95386e -- src/` is empty.
  - `Cargo.lock`, `evals/Cargo.lock` and `deny.toml` are unchanged.
  - No file under `.github/` or `.brokkr/` changes.
  - *(rerun)* No file under `.compare-bins/` is tracked:
    `git ls-files .compare-bins` is empty.
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
- *(rerun)* smtp4dev, Inbucket and GreenMail are not targets, even though
  their bytes are on the shelf. Adding one is a new product record and the
  operator's call. MailSlurper runs from its own release zip, not from
  Alkemio's image, whose directory is absent from this worktree.
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
