# Agent-suite comparison — implementation plan

Spec: [spec.md](spec.md). Decision: proposed
[0129](../../docs/decisions/0129-agent-suite-comparison.md), under accepted
0091–0093 and the operator's 2026-11-07 ruling.

## Technical context

- **Root crate.**
  - `benches/mail_compare.rs` with modules under `benches/mail_compare/`:
    `mailpit_mcp.rs` (833 lines), `targets.rs` (the verified launcher,
    owned binds, cleanup worker), `protocol.rs` (lock v1–v3 readers,
    `TargetKind::ALL` of three), and `wire.rs` (`Http`).
  - `tests/mail_compare.rs` includes those modules by `#[path]`.
  - The root bench and tests are outside the lib coverage denominator.
- **Eval crate (`evals/`, standalone, outside coverage).**
  - `runner.rs`: the trial body at about lines 2150–2350 starts Swarmail,
    builds `mcp_url = http://{http}/mcp`, starts the fixture, provisions and
    prepares, then launches agents.
  - `oracle.rs`: `solve` over `McpClient` (HTTP).
  - `wrappers.rs`: claude/codex/dsh configs, keyed `swarmail`, with
    `SWARMAIL_TOOLS`. A stdio `fixture` server already exists there.
  - `identity.rs`: `ComparisonRow`, schema 3, with the MailHog refusal at
    `validate`.
  - `mail.rs`: the fixture's SMTP client, AUTH PLAIN as `smtp_identity`.
  - `tasks.rs`: the hard tier is `otp-backlog`, `recipient-crowd` and
    `digest-lookalike`. Each has `agent_count` 1 and none uses chaos.
- **Wrapper.** `scripts/compare-mail.sh`: a stat-only prerequisite chain,
  then an unconditional `comparison-integration` refusal.
- **What already exists to reuse.**
  - Swarmail's compat shims serve Mailpit (`/api/v1/...`), MailHog
    (`/api/v2/messages`, `/api/v2/search`) and MailSlurper (`/mail`,
    `/mailcount`, `/mail/{id}`, `/mail/{id}/message`). They give CI-runnable
    mechanics tests with no download. They are mechanics only, never
    evidence.
  - `swarmail mcp` (`src/stdio.rs`) is a real stdio MCP server for the
    bridge test.
  - `src/mcp.rs` `MCP_BATCH_CAP = 16`, with a whole refusal and a null id.
- **Load bound.** `nproc` is 128, so at most 32 workers. Controls run with
  one worker (0076), per target, each as a named command timed with
  `/usr/bin/time` and bounded to 10 minutes by `timeout 600`. `uptime` is
  recorded before and after each.
- **Upstream bytes (rerun, 2026-11-07).** The pinned bytes are in the
  worktree under `.compare-bins/`, git-ignored and untrusted (spec, "What the
  pinned bytes establish"). They are not downloaded again. For every use:
  - each artifact is copied or extracted into its own **new** directory
    under ignored scratch on `/home`, never `/tmp`, which is a RAM tmpfs;
  - its sha256 is compared, as a hex constant, with the lock (Mailpit,
    MailHog) or the spec's recorded value (MailSlurper);
  - Python that reads them runs with `-I`, with the script and the cwd
    outside the bytes' directory;
  - only the verified member is ever run.

  `.compare-bins/2026-11-07-jre/` and `.compare-bins/2026-11-07-alkemio-mailslurper/`
  are absent, and neither is needed. A digest mismatch stops the dependent
  tasks, which are then reported **blocked**. The lock is never edited to fit
  the bytes.

## Constitution check

| Rule | How this plan holds it |
|---|---|
| 0001–0003 frozen | No `src/` change (AC12). The product record declares Swarmail's 0001/0002 semantics and exports none to a competitor. |
| 0004 per-test inboxes | Swarmail keeps them. Competitors are isolated by a fresh instance and emulate no inbox. |
| 0005 extraction at ingest | Not assumed upstream. MailHog's raw MIME passes through, and only the oracle (a control) decodes it. |
| 0091 §1 frozen product | `agent-comparison.json` is committed before the controls run. A changed method needs a new id. |
| 0091 §2 one fixture and scorer | One fixture plus `smtp_auth`. One scorer. Native v2 is untouched without `--target`. |
| 0091 §3 / 0092 §3 thin adapters | Source-verified general operations only, bodies unchanged, nothing emulated. |
| 0091 §5 controls first, no paid work | `controls` is free. `measure` needs `--authorized-product`, and delivery proves it with null agents only. |
| 0091 §7 / 0075 cost and privacy | Cost is `unavailable`, never 0. The bridge logs negotiation metadata, never bodies. |
| 0092 §6 / 0093 §7 lifecycle | The runner owns the adapter and the adapter owns the backend through `targets.rs`. Only exit 0 is a receipt, and anything else stops admission. |
| 0093 §1 lock history | v4. The v3 bytes are archived byte-identical, and v1–v3 readers keep three targets. |
| 0093 §4 waits | The common 100 ms poll, labelled. No stream client. |
| Coverage, exact 100% | The lib denominator is untouched. New bench and evals code is still tested line by line (house rule: no untested line). |
| Dependencies | None added. `Cargo.lock`, `evals/Cargo.lock` and `deny.toml` are unchanged. |
| Hold | `scripts/compare-mail.sh` changes, so the operator holds the PR. That is expected. |
| 0056 | Unsigned commits. Never push. |

## Project structure — the files this touches

```
docs/decisions/0091-fair-mail-comparison.md      + dated addendum 2026-11-07 (append only)
docs/decisions/0092-official-mail-comparison.md  + dated addendum 2026-11-07 (append only)
docs/decisions/README.md                         + 0129 row (proposed)
docs/evals/README.md                             status fix, rows updated, AUTH statement
docs/evals/agent-comparison.json                 new: frozen product record
docs/evals/agent-comparison-cost.json            new: cost basis "unavailable"
docs/evals/sources.lock.json                     lock v4 (+ mailslurper entry)
docs/evals/history/2026-11-07-sources.lock.v3.json  v3 bytes, byte-identical
docs/evals/sources/mailslurper/                  new: ref/release metadata, licence, README, the cited source files (spec R8)
docs/evals/wire-corpus/{mailpit,mailhog,mailslurper}/  new (rerun): captured upstream responses + index.json
docs/evals/agent-controls-2026-11-07/            new: controls rows, index, contracts, identity, receipts, dry-run transcript
benches/mail_compare.rs                          mailhog-mcp / mailslurper-mcp modes; --launch mode
benches/mail_compare/mcp.rs                      new: generic half + batches
benches/mail_compare/mailpit_mcp.rs              Mailpit backend only
benches/mail_compare/mailhog_mcp.rs              new
benches/mail_compare/mailslurper_mcp.rs          new
benches/mail_compare/protocol.rs                 TargetKind::Mailslurper appended; lock v4 reader; v3 reader kept
benches/mail_compare/targets.rs                  optional launch config template; third reserved port {www_addr}/{www_port} (rerun)
tests/mail_compare.rs                            #[path] includes; batch suite; shim mechanics; ignored upstream tests
evals/src/target.rs                              new: Target enum + backend tuple
evals/src/bridge.rs                              HTTP→stdio bridge (built); in-flight bound (rerun)
evals/src/runner.rs                              per-target server block; competitor prompt
evals/src/tasks.rs                               competitor header template
evals/src/oracle.rs                              MailSource trait + normalizers
evals/src/mailsource.rs                          MailSource impls (built); MailSlurper normalizer, exact recipient match (rerun)
evals/tests/{bridge,mailsource,target,smtp_auth,comparison}.rs  evals-side proofs; corpus decode tests (rerun)
evals/src/wrappers.rs                            EVAL_MCP_SERVER / EVAL_MCP_TOOLS; dsh refusal
evals/src/identity.rs                            four targets; agent in coordinate; smtp_auth; refusal removed
evals/src/report.rs                              cite_comparison keys on (target, agent, task)
evals/src/comparison.rs                          new: product record, controls index, comparison-check verifier
evals/src/main.rs                                matrix --target/--adapter/--target-bin; comparison-check subcommand
scripts/compare-mail.sh                          controls scope; verifier; --authorized-product; refusal removed
```

No file under `src/`, `.github/` or `.brokkr/` changes.

## Research and resolved questions

1. **Bridge or stdio client?** Bridge. The wrappers *could* take a stdio
   server: the fixture already uses one. But then the agent CLI would spawn
   the adapter, and with it the backend, so the runner would lose
   ownership (0092 §6), and the agents' transport would differ by target.
   The bridge keeps Streamable HTTP constant. It lets `McpClient` (oracle
   and contract probes) stay HTTP-only, and CI can test it against
   `swarmail mcp` with no download.
2. **Where the bridge runs.** As a task inside the runner process, not a
   separate process. It is then owned by the trial's own runtime and needs
   no new process-group bookkeeping. Its child is the adapter, spawned
   through the existing `spawn(..., &trial.groups, Some(ServerRole::Adapter),
   ...)`.
3. **Adapter launches the backend.** `mail_compare <t>-mcp --launch BIN
   --sources LOCK` reuses `targets::Owned::launch`: lock digest, argv,
   corroborated binds and the cleanup worker. `--endpoint URL` mode stays
   for tests and shim mechanics. Launch code is not duplicated in `evals/`.
4. **Lock version.** `validate_entries` requires exactly one entry per
   `TargetKind::ALL` member, and the v1/v2 readers iterate `ALL` too.
   Appending MailSlurper to `ALL` would break the historical readers, so
   each historical reader gets its own fixed three-target list. The
   current lock becomes v4. The wire protocol v2 `check` and `collect`
   iterate the *protocol's* targets, so MailSlurper's lock entry is never
   required of the wire bench.
5. **Mailpit `tools_sha256`.** Protocol v2 binds it. The split must keep
   `tools()` output byte-identical. A test asserts the digest against the
   committed protocol.
6. **Batch execution model.** The adapters stay single-in-flight (0092 §4
   bounds in-flight work). A batch takes one admission slot, and its members
   run in member order. Swarmail runs batch members concurrently, so the
   difference is recorded in the product record. It does not affect the
   hard tier, whose oracle does not batch.
7. **Fixture public projection.** Rows already carry fixture provenance
   (`projection_version` 1). The controls index records each trial's
   public projection digest, and the check compares them across targets
   for each (task, seed). The digests match because the fixture never
   sees the target.
8. **MailSlurper (resolved from the bytes in the rerun).** Each answer below
   cites the file it comes from in the spec's "What the pinned bytes
   establish" section.
   - **Version.** `release-1.15.0` is the newest release.
   - **AUTH.** None. The `AUTH PLAIN` line draws `221 Bye` and the
     connection closes, so `smtp_auth` is `none`, to be confirmed by the
     probe.
   - **Storage.** The `250` precedes storage, so the target is declared
     `not-promised`.
   - **Launch.** Three listeners, always started, configured from one
     `-config` file. Hence the third owned port and the config template.
   - **Service routes.** `/mail`, `/mail/:id` and their siblings. Service
     auth is off by config.
   - **Filters.** `LIKE` substring matches, so the oracle pre-filters with
     upstream's filter and then matches exactly.
   - **Bodies.** One `body` string per item, base64-decoded by `GetMail`
     when possible.

   The implementer re-reads each fact from the bytes before writing it into
   the lock, and stops and reports on any disagreement with the spec.
9. **Model-side contract.** The free controls prove the server side:
   negotiation with `swarmail-evals` as the client, capabilities, tool list
   and batches. Model-CLI negotiation is recorded per paid session by the
   bridge. The contract evidence states that limit.

## Data model

- **`Target`** (evals): `Swarmail | Mailpit | Mailhog | Mailslurper`, with
  `name()` and `comparison_name()` (`<t>-mcp`).
- **`BackendSpec`**:
  `{ target, lock_entry, adapter_mode, tools_sha256, connection,
  smtp_auth: plain|none, isolation: per-test-inbox|fresh-instance,
  wait_method, delivery: { accept_implies_queryable: promised|not-promised,
  native_wait: subscribe-before-check|none }, prompt_header_sha256,
  recipient_filter: upstream|oracle-side }`.
- **`agent-comparison.json`**:
  `{ id, version, ruling: {date, quote, issue}, tier: "hard", tasks[3],
  applicability: {task: {target: requested|not-requested}}, targets:
  [BackendSpec×4], agents: [{label, wrapper_profile}×4], trials: 5, order,
  deadline_ms, seed_policy, planned_cells: 240, controls: {agents:
  [oracle, null], trials: 5, cells: 120}, statistics, cost_ref,
  batch_semantics }`.
- **`ComparisonRow`** (schema 3): as on `main`, plus `coordinate.agent` and
  `settings.smtp_auth`.
- **Controls output**: `rows.jsonl`, `index.json` (product digest, per-row
  hashes, projection digests), `contracts/<target>.json`, `identity.json`,
  `receipts.json`.
- **`receipts.json`**: `{ head, controls_index_sha256, cleanup_receipts_sha256,
  eval_suite: {command, head, status}, ci: {status: pending} | {run} }`.

## Contracts

- `matrix --target T [--adapter P --target-bin B --sources L] --product F
  …`. Without `--target` it is unchanged.
- `mail_compare {mailpit,mailhog,mailslurper}-mcp (--endpoint URL | --launch
  BIN) --sources L`. In launch mode the announcement line is `smtp=ADDR
  tools_sha256=HEX`. The exit status is 0 only after full cleanup.
- `swarmail-evals comparison-check --mode evidence|measure|dry-run
  --baseline … --contracts … --controls-dir … --receipts … --cost …
  --identity … --product …`. It exits 0 on success, and 2 naming
  `prerequisite-<name>: <reason>`.
- `compare-mail.sh controls --output DIR --product P --mailpit B --mailhog B
  --mailslurper B`
- `compare-mail.sh measure --output DIR --config F [PREREQUISITES]
  (--dry-run | --authorized-product ID)`

## Quickstart (implementer)

```bash
cargo build --locked --bin swarmail
cargo bench --locked --no-run --bench mail_compare        # the adapter binary
cargo test --locked --test mail_compare                   # shims + batches, no downloads
(cd evals && SWARMAIL_BIN=$PWD/../target/debug/swarmail cargo test --locked)
# pinned upstreams (untrusted downloads, own dirs, digest-checked):
MAIL_COMPARE_MAILPIT=… MAIL_COMPARE_MAILHOG=… MAIL_COMPARE_MAILSLURPER=… \
  cargo test --locked --test mail_compare -- --ignored
# controls: the wrapper runs each target's matrix as its own named command
# under `timeout 600`, one worker, and records uptime and elapsed per target
scripts/compare-mail.sh controls --output … --product docs/evals/agent-comparison.json \
  --mailpit … --mailhog … --mailslurper …
scripts/compare-mail.sh measure --dry-run …               # transcript committed
```

## Rerun changes (2026-11-07), and which position each answers

The run context carries no review return, so no review finding is open.
These changes answer the rerun's two positions and the bytes, and they are
ruled in the spec's "How the rerun's positions were ruled".

| Change | Owning artifact | Answers |
|---|---|---|
| MailSlurper facts recorded from the bytes: release, digests, no AUTH, accept-before-store, three listeners, routes, `LIKE` filters, body shape | spec "What the pinned bytes establish"; R8; plan research 8 | Simplicity §1; robustness §2.1 |
| Hex-constant digest checks; archive and member digests; no `sha256sum -c` on a foreign path; stop on mismatch | spec R8; plan technical context | Robustness §2.1 |
| Tree unresolved; archive-attributed captures; only the cited files | spec R8 | Simplicity §2.1–§2.2 |
| v4 reader is the v3 reader given an expected-target list | spec R8; T3.b | Simplicity §2.3 |
| AUTH probe gates the product record; `221 Bye` and close is the expected result | spec R8, AC6; T3.c | Robustness §2.9; simplicity §1 |
| Every announced tool called once against each pin before the record freezes | spec R9, AC4; T4.c, T7.b | Robustness §2.3 |
| Wire corpus decoded in CI | spec AC4; T4.d | Robustness §2.2 |
| Hostile-argument rules on every adapter | spec R5, AC4; T4.e | Robustness §2.7 |
| Third owned port and config template for MailSlurper | spec R4, AC6; T3.b | Simplicity §1 |
| Bridge in-flight bound; pretty-printed body; adapter stderr to a file | spec R3, R4, AC3; T5.e, T5.c | Robustness §2.4, partly |
| Exact recipient match after MailSlurper's substring filter | spec R7, AC4; T6.d | Chief, from `CommonQueries.go` |
| Receipts ancestry rule; row-count and torn-tail refusals | spec R9, AC11; T7.c | Robustness §2.5, with simplicity's no-new-ledger ruling |
| README's "Not built" block replaced, not appended to | T8.c | Simplicity §0 |
| Built tasks ticked with their commits | tasks.md | Simplicity §0 |

No acceptance criterion was weakened. The only text changed rather than
added is AC10's scratch location (`/home`, not a temp dir). It follows the
host's `/tmp` quota and is stricter.

## Risks

| Risk | Mitigation |
|---|---|
| **The shelf bytes are trusted because they are present.** | Each artifact is checked against a hex constant before use: the lock for Mailpit and MailHog, the spec's recorded digest for MailSlurper. Each is extracted into a new directory, and only the verified member runs. Python runs with `-I`. A mismatch stops the dependent tasks, which report **blocked**; the lock is never fitted to the bytes. `git ls-files .compare-bins` stays empty (AC12). |
| **The source and the pinned binary disagree about AUTH** (for example, a build that differs from its tag). | The probe gates the record. A disagreement or an inconclusive probe refuses to freeze the record, and the MailSlurper items report blocked. Nothing is guessed. |
| **MailSlurper's `250` precedes storage, so the oracle sees a short list.** | Declared `not-promised`. The oracle and the labelled poll wait poll until the count is met or the deadline passes. A shortfall at the deadline is an oracle error in controls, never a pass. |
| **MailSlurper's substring `to` filter returns a lookalike recipient.** | Agents see upstream's real filter, described truthfully, which is the variable being measured. The oracle matches exactly after the pre-filter, and a test proves the exclusion (AC4). |
| **MailSlurper binds a port the launcher does not own** (the admin www listener). | A third reserved port is filled into the config template, and a launch test proves three owned binds and their release (AC6). Bind corroboration covers all three. |
| **MailSlurper writes outside its scratch** (the relative SQLite path, logs). | The config template's `dbDatabase` is an absolute path under `{data_dir}`, and the working directory is the trial's scratch. The cleanup receipt covers the scratch. |
| **The wire corpus goes stale or carries private data.** | It is captured only from fixture-generated synthetic mail, and its index binds each pin's executable digest. A test refuses an index whose digest differs from the lock, so a moved pin forces a recapture. No agent body is ever in it (0075). |
| **Committing the controls moves HEAD and makes the receipts look stale.** | The ancestry rule allows a diff touching only the evidence paths. Any code, lock, product or script change refuses (AC11). |
| **The split changes Mailpit's tool list and breaks frozen protocol v2.** | A test asserts `sha256(tools(&Bounds::default()))` equals `protocol.json`'s `budgets.adapter.tools_sha256`. The protocol bytes are diffed against `c95386e`. |
| **Lock v4 breaks the historical readers or the admitted 2026-10-30 collection.** | Each historical reader keeps its own three-target list. The v3 bytes are archived byte-identical and read by the v3 reader. Every existing protocol and lock test stays green. The wire `check` on protocol v2 with lock v4 is a test. |
| **Adapter death leaks the backend's process group.** `targets.rs` gives the backend its own group. | Only the adapter's exit 0 after full cleanup is a receipt. Kill, non-zero exit and timeout all mark cleanup unknown and stop admission (0093 §7). A test kills the adapter mid-trial (AC11). |
| **The bridge misroutes or drops replies under a misbehaving client.** | Replies are routed by id. A duplicate in-flight id is refused. Bodies and lines are bounded by the adapter's `max_line`. An exited child gives `503`. The suite runs against the real `swarmail mcp`. |
| **A batch exhausts bounds.** | Cap 16 with a whole refusal. One admission unit. Per-member budgets unchanged. The array is bounded by `max_reply`, and an over-bound member gets `reply_too_large`. Hostile-client suite (AC5). |
| **MailSlurper refuses AUTH PLAIN.** | `smtp_auth: none` drops only the AUTH step. It is stated in the README, the product record, the row settings and the lock finding. Nothing is emulated or proxied. |
| **The prompt header is read as a 0091 §2 conflict.** | The goal text and scorer are byte-identical, and the header digests are frozen. If review rules it a conflict, the run parks for the operator (spec, fairness check). It is not patched. |
| **The oracle's MIME decoding for MailHog masks a body difference.** | The decoding is in the oracle (a control), not the adapter. Agents get raw bodies. An unknown MIME shape is an oracle error, never a pass. The null control proves the oracle is not vacuous. |
| **The controls exceed the load or time bound.** | One worker. Each target is its own `timeout 600` command, with uptime recorded. The hard run of 2026-11-03 took about 0.5 s per oracle trial, so 30 trials per target fits. A breach is reported and never retried silently. |
| **The dry run is mistaken for a stub.** | Every prerequisite is produced for real. Pending items (CI receipt) are printed. Non-dry measure refuses them. The transcript is committed. |
| **Model-CLI negotiation fails in the first paid session.** | It is logged by the bridge, and a failure is a visible `harness_error` row. The contract evidence states the limit. |
| **A competitor's tool names collide with the claude allowlist format.** | The tool names come from the adapter's announced list, which is validated against `^[a-z][a-z0-9_]{0,63}$`. A test covers the generated configs. |
| **Upstream-backed tests are not in CI.** | The #287 precedent: ignored tests behind `MAIL_COMPARE_<T>`, the committed controls receipts, and CI mechanics against the Swarmail shims. *(rerun)* The committed wire corpus is also decoded in CI, so the adapter decoders and the oracle normalizers run on real upstream shapes on every run. The README names what only the receipts prove. |
