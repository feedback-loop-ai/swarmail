# Agent-suite comparison — tasks

Order matters. Each task names the test that proves it. Bound every
measurement to at most 32 workers (nproc/4) and 10 minutes. Name and time
each measurement, and report the load. Commit unsigned. Never push.

## T1 — Records (AC1)

- [ ] T1.a Append "Dated addendum — 2026-11-07" to
  `docs/decisions/0091-fair-mail-comparison.md` and
  `docs/decisions/0092-official-mail-comparison.md`. Each quotes "compare
  with all that we have shims for", names the four targets and four agents
  (no GLM), lifts 0091's MailHog-agent-eval rejection for this story only,
  admits MailSlurper under 0092 §3 unchanged, and points to proposed 0129.
  The 0092 addendum also says the batch gap closes here.
  **Proof:** `git diff --numstat c95386e -- <both files>` shows 0 deleted
  lines.
- [ ] T1.b Add the proposed 0129 row to `docs/decisions/README.md`.
  **Proof:** `cargo test --locked --test store_contract_docs`.
- [ ] T1.c `docs/evals/README.md`: state 0091–0093 as accepted (operator,
  2026-11-02) and update the T7–T10 and AC rows to what this story proves.
  **Proof:** `grep -nE 'proposed \[009[123]\]' docs/evals/README.md` finds
  nothing, and the doc link checks in `tests/store_contract_docs.rs` pass.

## T2 — Generic adapter half and batches (AC4, AC5)

- [ ] T2.a Split `benches/mail_compare/mailpit_mcp.rs` into `mcp.rs` (the
  generic half and a `Backend` trait: `tools`, `parse_call`, `run_call`)
  and the Mailpit backend. Update the `#[path]` includes in
  `tests/mail_compare.rs`.
  **Proof:** every existing `cargo test --locked --test mail_compare` test
  passes. A new test asserts that the Mailpit `tools()` digest equals
  `docs/evals/protocol.json` `budgets.adapter.tools_sha256`.
- [ ] T2.b Batches under 0129 §6 in the generic half.
  **Proof:** a hostile-client suite in `tests/mail_compare.rs`:
  - 1 and 16 members give an ordered array;
  - 17 members and an empty array each give one null-id error, with an
    upstream request counter of 0;
  - notifications-only writes nothing;
  - `initialize` inside a batch gives `-32600`;
  - duplicate ids are answered as sent;
  - an over-bound member gets `reply_too_large`;
  - a full queue gives busy errors for all members;
  - a 64 KiB line is refused;
  - EOF mid-batch ends cleanly;
  - a member's cancellation cancels only that member.
- [ ] T2.c `--launch BIN` mode: launch through `targets::Owned`, announce
  `smtp=… tools_sha256=…`, serve stdio, clean up on stdin EOF, and exit 0
  only on full cleanup facts.
  **Proof:** tests against the Swarmail binary as the launched target
  (mechanics): exit 0 after EOF, and a non-zero exit when cleanup is forced
  to fail through the existing cleanup seam.

## T3 — MailSlurper provenance and lock v4 (AC6)

- [ ] T3.a Obtain MailSlurper's latest official release asset, tag, ref,
  tree, licence, README and the service-API and SMTP source files. Each
  download goes into its own new directory and is digest-verified before
  use, and Python runs only with `-I`. Commit the captures under
  `docs/evals/sources/mailslurper/`. If the bytes cannot be obtained, mark
  T3, T4b and T8 **blocked**, report what is missing, and continue with the
  other tasks.
  **Proof:** the lock's blob and size checks pass over the captures (an
  extended `the_tag_chain_and_every_blob_are_rechecked_against_the_captures`).
- [ ] T3.b Move the current lock's bytes to
  `docs/evals/history/2026-11-07-sources.lock.v3.json`. Add the v4 reader,
  append `TargetKind::Mailslurper` to `ALL`, and give the historical v1–v3
  readers their own three-target list. Add the optional launch `config`
  template to `targets.rs` if MailSlurper needs it.
  **Proof:**
  - `cmp` of the archive against `git show c95386e:docs/evals/sources.lock.json`;
  - the v1/v2/v3 reader tests pass unchanged;
  - a v4 validation test passes;
  - `mail_compare check --protocol docs/evals/protocol.json --sources
    docs/evals/sources.lock.json` exits 0;
  - `git diff c95386e -- docs/evals/protocol.json` is empty.
- [ ] T3.c Answer the AUTH PLAIN question from the reviewed SMTP source and
  an unscored setup probe against the pinned binary. Record the answer in
  the lock finding.
  **Proof:** an ignored test `mailslurper_auth_plain_probe`
  (`MAIL_COMPARE_MAILSLURPER`) asserts the recorded answer. The probe
  transcript is committed beside the controls.

## T4 — MailHog and MailSlurper adapters (AC4)

- [ ] T4.a `benches/mail_compare/mailhog_mcp.rs` and the `mailhog-mcp` mode:
  list (v2 messages), search (v2 search, kind from/to/containing), read (the
  observed v1 message route, with its attribution stated as open) and a
  labelled poll wait. Bodies pass through unchanged, and the routes are
  cited and bound in the lock.
  **Proof:**
  - mechanics tests over real SMTP against Swarmail's MailHog shim (in CI);
  - an ignored real-binary test (`MAIL_COMPARE_MAILHOG`);
  - a test that no tool name or description offers inbox, await,
    extraction, delete or admin.
- [ ] T4.b `benches/mail_compare/mailslurper_mcp.rs` and the
  `mailslurper-mcp` mode, over the source-verified `/mail` pages, filters
  and `/mail/{id}` only.
  **Proof:** the same three kinds of test, against Swarmail's MailSlurper
  shim and `MAIL_COMPARE_MAILSLURPER`.

## T5 — Targets, bridge and runner (AC2, AC3, AC11)

- [ ] T5.a `evals/src/target.rs`: the `Target` enum and the `BackendSpec`
  table. `matrix --target/--adapter/--target-bin/--sources`, with the named
  refusals (unknown value, `--brokkr`, dsh profile, a non-applicable task).
  **Proof:**
  - unit tests per refusal, each asserting that nothing launched;
  - the existing matrix suite passes unchanged without `--target`.
- [ ] T5.b `evals/src/bridge.rs` under spec R3.
  **Proof:** an evals test against real `swarmail mcp` (`SWARMAIL_BIN`)
  covering `initialize`, `tools/list`, a call, a batch, notifications-only
  `202`, `GET` `405`, a foreign Host refused, a duplicate in-flight id
  `-32600`, and an exited child `503`. A metadata test shows that
  `clientInfo`/`protocolVersion` are logged and bodies are not.
- [ ] T5.c Runner per-target server block (spec R4): `ServerRole::Adapter`,
  the announcement parse, a `tools_sha256` check against the product, the
  fixture's `smtp_auth`, and the cleanup receipt from the exit status.
  **Proof:**
  - `matrix --target swarmail --tier hard` with `oracle` and `null` end to
    end in the evals suite;
  - a test that a kill or non-zero adapter exit stops admission and keeps
    the slot's row as a visible failure (AC11);
  - a test that a mismatched `tools_sha256` refuses.
- [ ] T5.d Fixture `smtp_auth: none` path in `mail.rs`: it drops only the
  AUTH step.
  **Proof:** a unit test over a recorded SMTP dialogue against a real
  `swarmail serve` (an AUTH-optional listener), plus a test that the `plain`
  dialogue is byte-identical to `main`'s.

## T6 — Prompt, wrappers and oracle (AC2, AC7)

- [ ] T6.a The competitor header template in `tasks.rs`.
  `PROMPT_TEMPLATE` is unchanged.
  **Proof:**
  - a digest test that `PROMPT_TEMPLATE` equals `main`'s;
  - a render test that each competitor prompt contains the goal
    byte-for-byte and no `inbox` instruction;
  - the header digests equal the product record's.
- [ ] T6.b Wrappers: `EVAL_MCP_SERVER`/`EVAL_MCP_TOOLS` for claude and codex,
  tool-name validation, and the dsh refusal for competitors.
  **Proof:** generated-config unit tests for each target, plus the existing
  Swarmail config tests unchanged. No model is launched.
- [ ] T6.c `MailSource` trait in `oracle.rs`. The Swarmail implementation
  is unchanged, and there are Mailpit, MailHog and MailSlurper
  normalizers, with MIME decoding for MailHog in the oracle.
  **Proof:**
  - normalizer unit tests over fixture-generated messages;
  - an unknown-MIME-shape test that errors;
  - the existing oracle tests unchanged;
  - competitor oracles passing in T8.

## T7 — Product record, rows and verifier (AC8, AC10, AC11)

- [ ] T7.a `identity.rs`: four targets and their connections,
  `coordinate.agent`, `settings.smtp_auth`, and the `mailhog` refusal
  removed. `report::cite_comparison` keys on (target, agent, task).
  **Proof:**
  - a sealed MailHog row validates;
  - target `mailhog` and an unknown target are refused;
  - native readers refuse schema 3;
  - the citation arithmetic tests are extended with the agent coordinate.
- [ ] T7.b Commit `docs/evals/agent-comparison.json` (spec R9, plan data
  model) and `docs/evals/agent-comparison-cost.json`.
  **Proof:** `comparison-check` validates the product, and a test refuses a
  product whose `planned_cells` is not
  tasks × targets × agents × trials = 240, or whose tuple's `tools_sha256`
  differs from the adapter's.
- [ ] T7.c `evals/src/comparison.rs` and the `comparison-check` subcommand,
  with the dry-run, evidence and measure modes.
  **Proof:** one negative test per prerequisite in the fixed order, for
  missing, wrong-content and wrong-binding cases (receipts HEAD, identity
  re-measure, controls not bound to the product, cost recorded as 0).
- [ ] T7.d `scripts/compare-mail.sh`:
  - `controls` needs only `--product` and the executables;
  - `evidence`/`measure` run the stat chain, then the verifier;
  - `--authorized-product`;
  - the `comparison-integration` refusal removed.
  **Proof:**
  - wrapper tests per mode, refusing by name;
  - `grep -c comparison-integration scripts/compare-mail.sh` gives 0;
  - a non-dry measure with an inert builtin-`null` config on a synthetic
    one-trial product delegates. Without, or with a wrong,
    `--authorized-product`, or with a pending CI item, it refuses and spawns
    no agent (AC10).

## T8 — The free proof (AC7, AC9)

- [ ] T8.a Run `compare-mail.sh controls` on all four real pinned targets,
  one worker, each target under `timeout 600` with `/usr/bin/time` and
  `uptime` before and after. Commit
  `docs/evals/agent-controls-2026-11-07/`.
  **Proof:**
  - every oracle trial passes and every null trial fails with its idle
    class;
  - the projection digests are equal across targets;
  - the index binds the product digest;
  - the load and elapsed time are reported.
- [ ] T8.b Run one `compare-mail.sh measure --dry-run` with every
  prerequisite genuine. Commit its transcript.
  **Proof:**
  - exit 0, the 240-slot plan printed and the pending items named;
  - a test asserts that the dry run spawns nothing but the verifier and
    never opens `--config`.

## T9 — Gates (AC12)

- [ ] T9.a Run the whole house inventory `--locked`:
  - root fmt, clippy, test and `build --bin swarmail`;
  - `evals` fmt, clippy and test with `SWARMAIL_BIN` set to the
    candidate's binary;
  - `scripts/coverage-gate.sh` and its selftest;
  - `scripts/publish-guard-test.sh` and `scripts/seal-branch-test.sh`;
  - `cargo deny check licenses`.
  If the sandbox has no brokkr binary, report that the bundle-compile gate
  runs in verify.
  **Proof:** the gate transcript, plus:
  - `git diff c95386e -- src/ .github/ .brokkr/ Cargo.lock evals/Cargo.lock
    deny.toml` is empty.
