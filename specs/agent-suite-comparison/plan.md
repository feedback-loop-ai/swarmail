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

- **The supply (second rerun, 2026-11-07).** The cgo build and its evidence
  are in `.compare-bins/2026-11-07-supply/mailslurper-cgo/`. The release's
  commit and root tree are in `.compare-bins/2026-11-07-supply/git-trees/`
  (spec, "What the supply establishes"). The same rules apply:
  - copy each file into a new directory under ignored scratch on `/home`;
  - compare its sha256 with the supply's `SHA256SUMS` hex, as constants;
  - run only the verified `mailslurper` (`eb6cd97d…58ff`).

  The supply's scripts (`build-in-container.sh`, `launch_probe.sh`,
  `smtp_send_one.py`, `treehash.py`) are read and never run. The official
  binary is never launched again.
- **The build links glibc.** The binary is dynamically linked against glibc
  2.31 or newer. This host runs it, as the supply's launch shows. The
  adapter launches the backend outside the agent sandbox, and the trial
  sandbox binds `/lib`, `/lib64` and `/usr` anyway (`evals/src/isolation.rs`).

## Constitution check

| Rule | How this plan holds it |
|---|---|
| 0001–0003 frozen | No `src/` change (AC12). The product record declares Swarmail's 0001/0002 semantics and exports none to a competitor. |
| 0004 per-test inboxes | Swarmail keeps them. Competitors are isolated by a fresh instance and emulate no inbox. |
| 0005 extraction at ingest | Not assumed upstream. MailHog's raw MIME passes through, and only the oracle (a control) decodes it. |
| 0091 §1 frozen product | `agent-comparison.json` is committed before the controls run. A changed method needs a new id. |
| 0091 §1 official evidence *(rerun 2)* | The MailSlurper source stays official and bound (tag commit, root tree, source archive). The artifact is the operator-ruled source build, labelled `built_from_source`. The official artifact's digests and failure stay in the lock as `official_release`. The ruling is the operator's. 0129's second addendum records it, and 0091's 2026-11-07 addendum points to it. |
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
docs/evals/sources/mailslurper/build/            new (rerun 2): PROVENANCE.md, go-version-m.txt, go.sum, go.mod, official-release-stderr.log
docs/evals/sources/mailslurper/git/              new (rerun 2): the release commit and root tree JSON
docs/decisions/0129-agent-suite-comparison.md    + second dated addendum 2026-11-07 (rerun 2; the first is not edited)
docs/evals/wire-corpus/{mailpit,mailhog,mailslurper}/  new (rerun): captured upstream responses + index.json
docs/evals/agent-controls-2026-11-07/            new: controls rows, index, contracts, identity, receipts, dry-run transcript
benches/mail_compare.rs                          mailhog-mcp / mailslurper-mcp modes; --launch mode
benches/mail_compare/mcp.rs                      new: generic half + batches
benches/mail_compare/mailpit_mcp.rs              Mailpit backend only
benches/mail_compare/mailhog_mcp.rs              new
benches/mail_compare/mailslurper_mcp.rs          new
benches/mail_compare/protocol.rs                 TargetKind::Mailslurper appended; lock v4 reader; v3 reader kept;
                                                 (rerun 2) ArtifactKind::BuiltFromSource, Artifact.build/official_release,
                                                 DocKind::GitCommit, the observed lightweight tree, the root-blob check
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

## Second rerun changes (2026-11-07), and which position each answers

The run context carries no review return, so no review finding is open.
These changes answer the operator's ruling of 2026-11-07 and the two
positions of `second-rerun-of-339-operator-dir-e9c4d893`. They are ruled in
the spec's "How the second rerun's positions were ruled".

| Change | Owning artifact | Answers |
|---|---|---|
| The ruling is recorded: a second 0129 addendum, and a pointer bullet in 0091's 2026-11-07 addendum | spec R11, AC1; T1.d | The operator's ruling; simplicity §3 |
| Fairness re-checked: no stop, because the operator ruled the conflict by name | spec "Fairness check, again" | Simplicity §1; robustness §0 |
| Lock v4 edited in place; `ArtifactKind::BuiltFromSource` | spec R11, AC13; T3.d | Simplicity §2.1–§2.2 |
| `build` record, with the source digest equal to the source archive's and the builder image pinned by digest | spec R11, AC13; T3.d | Robustness §2.1 |
| `official_release` kept in the lock, never launched | spec R11, AC6, AC13; T3.d | Robustness §2.2 |
| Build-evidence documents with an unresolved attribution and its reason; no new attribution variant | spec R11; T3.d | Simplicity §2.3 |
| `DocKind::GitCommit`, the observed lightweight tree, the root-blob check, and the tree recomputed from JSON in a test | spec R11, AC13; T3.d | Simplicity §2.4; robustness §2.4 |
| The tarball recomputed to the tree once, stopping on a mismatch | spec AC13; T3.e | Robustness §2.4 |
| The version report in the lock, the product and the README, checked by the verifier | spec R9, R11, AC13; T7.b, T7.c | Robustness §2.3; simplicity §2.5 |
| `--target-bin`, `--sources` and `--product` bound before any spawn | spec R2, AC2; T5.a | Robustness §2.5 |
| `eval-build` named, and every target's control rows required | spec R9, AC8; T7.c, T7.d | Robustness §2.6 |
| The corpus binds the config template and both base64 branches | spec AC4; T4.d | Robustness §2.7 |
| The AUTH probe's subject is the source build | spec AC6; T3.c | Robustness §2.9 |

**Criteria changed rather than added.** One. AC6's first-rerun binding list
asked for "an unresolved tree, with its reason". The supply now holds the
release's commit object and its non-truncated root tree, so the tree can be
observed. The criterion now requires the observed tree and its
recomputation. That asks for more, not less, and the evidence is the
`git-trees/` capture. AC6's zip and member digests are not dropped. They
move into `official_release`, which AC6 and AC13 still require. Every other
criterion stands.

**Rejected from the positions, with the cost.**
- A lock v5 (no position asked for it).
- A separate `origin` field: the artifact kind already carries it.
- A `--list-targets` enumeration: the four targets' control rows are
  stronger, executed evidence.
- A `host_supplied_capture` attribution: the unresolved reason says it.
- Copying the version into row settings: rows bind the product digest.
- Reproducing the build in a gate.

## Third rerun changes (2026-11-08), and which position each answers

Run `third-rerun-of-339-integrate-mai-60435935`, from `4d89de1`, merges
`main` at `55f211d` (accepted 0130). The spec section "The 2026-11-08
third rerun" gives the rulings and R12/R13 the requirements. No review
returned this run; the positions argued a fresh integration.

### Technical context

- **The base moved.** Main gained #340's nine-target wire driver (lock v4,
  protocol v3, frozen by sha in `tests/mail_compare/eight_targets.rs`),
  #345's deliverability work in `src/` and the evals crate, and #344. None
  of it changes a frozen guarantee: 0001, 0002, 0003 and 0005 are untouched
  by this branch, because it changes no file under `src/`. Any `src/` diff
  against `origin/main` after the merge is a stop (R12.l).
- **The constitution check still passes.** No dependency is added. No
  `Cargo.lock`, `evals/Cargo.lock`, `deny.toml`, `.github/` or `.brokkr/`
  change comes from this branch. Commits are unsigned and never pushed.
  0129 stays proposed. Accepted records get dated pointers only. The PR is
  still held for the operator, because `scripts/compare-mail.sh` changes.
- **No brokkr binary in this sandbox** (`which brokkr` is empty). The
  bundle-compile gate runs in verify, and the report says so.

### Order of work

The order is frozen bytes first, types second, behaviour third and
evidence last. Each step answers a named point.

1. **Merge, then restore the frozen bytes** (simplicity §1, robustness
   §2.1/§2.6). Run `git merge origin/main`. Then run
   `git checkout origin/main -- docs/evals/sources.lock.json
   docs/evals/protocol.json docs/evals/history/` and restore each main
   file under `docs/evals/sources/`. Check the two sha256s before resolving
   anything else. This closes the silent `ef8309cf…` auto-merge trap.
2. **Take main's `protocol.rs` and `targets.rs` whole**
   (`git checkout --theirs`), then compile the branch's adapter modes
   against them (simplicity §2.2–2.3, robustness §2.2). Re-add only what a
   compile error demands. `mailslurper-mcp` maps to
   `TargetKind::MailslurperSourceBuild`, and its spec comes from
   `LaunchSpec::from_pin`.
3. **Resolve `tests/mail_compare.rs`.** Use main's side for every wire and
   lock test. Delete the branch's lock-layer tests: the
   `the_v4_lock_pins_mailslurper_as_built_from_source…` block through the
   tree and document refusals, the duplicate v3-archive tests, and the
   fake-MailSlurper launch tests that only re-prove main's config-file
   launch. Port the adapter tests and `4d89de1`'s retry to main's types.
   Then add R13: the constant, `launch_shim_with_fresh_slot`, the
   forced-collision test and the exhaustion test (robustness §2.3, in the
   part adopted). Finally, run R13's grep.
4. **Move the unbound sources** to `docs/evals/agent-sources/mailslurper/`
   and add the fourteen-row table and its two tests (R12.e). First hash
   every branch file under `docs/evals/sources/mailslurper/` against main's
   documents, and delete only the sha256-equal ones.
5. **The evals crate** (simplicity §2.4/§2.7). Resolve the conflicts by
   keeping both sides. Fold `send_auth` into `send_stored`. Add
   `Target::lock_entry()` and repoint `validate_tuple` and
   `tuple_agrees_with_lock`. Edit the product record's two fields
   (R12.g), and add the two refusal tests (AC15).
6. **The wrapper** (simplicity §2.6). Use main's grammar, keep the
   branch's agent modes, and have the required `--bin` set come from the
   product record. Update the wrapper tests that used `--mailslurper`.
7. **The wire corpus** (R12.j). Compare the template sha256. Recapture,
   free and bounded, only if it differs.
8. **Records** (R12.k): 0129's addendum, the pointers on 0091 and 0130,
   the index row and the README.
9. **Controls on the final code head** (simplicity §2.8, robustness
   §4). Rerun the oracle and null controls for each target, one worker,
   `timeout 600 taskset -c 0-31`, into
   `docs/evals/agent-controls-2026-11-08/`. Regenerate the dry run. Then
   `git rm` the 2026-11-07 directory and repoint the constants. The commits
   that follow touch only evidence paths, which the receipt ancestry rule
   allows.
10. **Gates**: the whole inventory, `--locked`, recorded in the gates
    transcript with times and load.

### Files this rerun touches

- `benches/mail_compare.rs`, `benches/mail_compare/protocol.rs`,
  `benches/mail_compare/targets.rs` (main's, plus R12.c's additions).
- `tests/mail_compare.rs` (merge, R12.e table, R13).
- `evals/src/{target,comparison,fixture,mail,main,runner}.rs`,
  `evals/tests/{fixture,comparison}.rs`, plus the wrapper tests wherever
  they live.
- `scripts/compare-mail.sh`.
- `docs/evals/agent-comparison.json`, `docs/evals/README.md`,
  `docs/evals/agent-sources/mailslurper/` (new),
  `docs/evals/agent-controls-2026-11-08/` (new),
  `docs/evals/agent-controls-2026-11-07/` (removed),
  `docs/evals/history/2026-11-07-sources.lock.v3.json` (removed), and the
  branch's `docs/evals/sources/mailslurper/` copies (removed, R12.e).
- `docs/decisions/0129-agent-suite-comparison.md` (addendum),
  `0091-fair-mail-comparison.md` and `0130-eight-target-mail-comparison.md`
  (pointers), `docs/decisions/README.md` (rows).
- `docs/evals/wire-corpus/`, only if R12.j finds the template differs.

### Criteria this rerun reads differently, and why

- **AC6's protocol and lock bullets, and AC13, are superseded, not
  weakened.** They asserted bytes this branch wrote: protocol `c95386e`,
  and lock v4 with `built_from_source`. Accepted 0130 now freezes different
  bytes for the same files, and a criterion that conflicts with an accepted
  decision moves toward the decision. What they protected is kept: the
  executable digest, the provenance, the official failure and the
  "built from source" label. Main's lock binds all of these, and AC14 and
  AC15 assert them.
- **AC12's base moves from `c95386e` to `origin/main`.** After the merge,
  `git diff c95386e -- src/` is main's #345 work, not this branch's. The
  criterion's meaning ("this branch changes no `src/`, lock, deny, CI or
  brokkr file") is kept exactly, and AC18 restates it against the merged
  base.
- **AC7's directory moves to 2026-11-08,** because the receipts bind
  digests that the merge changes. The assertions do not change.

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
| **The source build is read as upstream's release** *(rerun 2)*. | Its kind is `built_from_source`, the operator's label. Its digest basis is `locally_computed` and `upstream_digest` is null. The official zip, its member and its failure stay bound in `official_release`. The README, the product record and 0129's addendum say "built from source". The validator refuses a `built_from_source` artifact without `build` or `official_release`. |
| **The binary is not tied to the tag** *(rerun 2)*. | `build.source_sha256` must equal the source archive's digest. The tarball is recomputed to the tag's root tree once, and a mismatch stops the run. The tree JSON is re-hashed in a CI test. |
| **A rebuild gives different bytes** *(rerun 2)*. | Expected: the build is recorded as one build, not a reproducible one. A different binary is a new pin and a new product id, never a digest edited to match. |
| **The v1.14.1 report is "fixed" by a later reader** *(rerun 2)*. | It is recorded in the lock note, the product tuple and the README, and the verifier refuses a product that disagrees with the lock. The official binary logs the same string, from the same hard-coded source constant. |
| **The binary's glibc dependency fails on another host** *(rerun 2)*. | The failure is loud: the launcher's corroborated binds never appear, and the launch is refused. It cannot mis-measure. CI launches no MailSlurper binary; it decodes the committed corpus. The glibc floor (2.31) is stated in the 0129 addendum and the README. |
| **A substituted `--target-bin`** *(rerun 2)*. | The runner hashes it against the product before spawning anything, and names both digests. The adapter's launch check is the second line. |
| **Merge contention with #340 and 0130**, which use the same supply and may touch the lock's MailSlurper entry for the bench *(rerun 2; realised in rerun 3, when 0130 landed first and this branch merges it under R12)*. | This story changes only the v4 agent-comparison entry and leaves `protocol.json` alone. Whichever lands second rebases onto the other's lock. The PR notes tell the operator. Nothing is coordinated in code. |
| **The comparison-integration refusal goes without a real replacement** *(rerun 2)*. | `eval-build` is built and named, and `comparison-check` requires every target's planned control rows. The `grep` for the string only guards against its return. |
| **Upstream-backed tests are not in CI.** | The #287 precedent: ignored tests behind `MAIL_COMPARE_<T>`, the committed controls receipts, and CI mechanics against the Swarmail shims. *(rerun)* The committed wire corpus is also decoded in CI, so the adapter decoders and the oracle normalizers run on real upstream shapes on every run. The README names what only the receipts prove. |
| **The merge silently rewrites the frozen lock** *(rerun 3)*. `sources.lock.json` auto-merges to `ef8309cf…` with no marker. | Step 1 restores main's bytes before anything else. AC14 asserts the sha256 and an empty `git diff origin/main` over every frozen path, and main's `FROZEN_LOCK_V4_SHA` test fails any later edit. |
| **A conflict is "resolved" by deleting the assertion that fails** *(rerun 3)*. | Main's frozen-method and frozen-sha tests are kept verbatim. Only the branch's tests of its own removed lock layer are deleted, and each of them is listed in plan step 3. Agent-suite tests are ported, never deleted. Review compares the deleted test names with that list. |
| **The agent suite binds the official, unstartable MailSlurper** *(rerun 3)*. | `mailslurper-mcp` maps to `MailslurperSourceBuild`. `Target::lock_entry()` is one exhaustive match. The verifier refuses a tuple with `lock_entry: "mailslurper"` or any other entry (AC15), and the adapter launch asserts the digest `eb6cd97d…58ff`. |
| **A wildcard arm gives a new target another target's behaviour** *(rerun 3)*. | No `_ =>` on `TargetKind` or `Target` in added code, checked by AC15's grep. The agent matrix is the declared four. |
| **The cited MailSlurper sources lose their binding** *(rerun 3)*. | R12.e moves them to `agent-sources/`, a branch-owned path outside 0130's tree. A digest table and a default test bind them, and an ignored test re-extracts them from the pinned archive. Only files sha256-equal to main's documents are deleted. |
| **The port retry is disabled by a reword, or its arm never runs** *(rerun 3)*. | One shared constant is used at producer and consumer. The forced-collision and exhaustion tests drive the retry arm and the failure arm on every default run (AC16). |
| **The retry hides a real startup failure** *(rerun 3)*. | It is bounded at five. Each refusal proves cleanup, and the fifth panics with the error. A non-collision startup exit still fails, at most four fast relaunches later. Follow-up (not this run): pass `--mailslurper-listen 127.0.0.1:0` and read the bound address from Swarmail's startup log (`src/main.rs` logs `mailslurper = …`). That deletes the race, but it needs main's startup reader extended. |
| **`real_mailslurper_official_does_not_start` flakes on a stolen port** *(rerun 3)*. | It is ignored and opt-in, and it is main's accepted code. A theft can only make it fail falsely, because its pass needs the cgo stub line on stderr. It is left byte-identical, and a failure there is reported, not retried. |
| **The coverage gate flakes again on a test outside this branch** *(rerun 3)*. The second rerun saw one in `src/store.rs`. | It is reported with its output. It is never patched in `src/` (R12.l), and never hidden by serialising the suite. A rerun of the gate is a named, timed command, and every run's verdict is recorded. |
| **Load during the proofs** *(rerun 3)*. | Every added repeat and every controls command is at most `nproc/4` CPUs (`taskset -c 0-31`), under `timeout 600`, with uptime before and after. The coverage gate and the evals suite run as the house runs them, one at a time and never beside each other. |
| **Stale controls receipts after the merge** *(rerun 3)*. | The controls rerun on the final code head. Later commits touch only evidence paths. `comparison-check` refuses anything else. |
| **The product record id is read as a method change** *(rerun 3)*. | Only `lock_entry` and `artifact_kind` change. The bytes measured are the same, and the 0129 addendum states it. If review rules otherwise, the remedy is a new id string and nothing else. |
| **A product-tuple field disagrees with main's entry** *(rerun 3)*. | Stop and report (R12.l). Neither the tuple nor the lock is edited to agree. |
