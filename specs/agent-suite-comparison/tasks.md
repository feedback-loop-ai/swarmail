# Agent-suite comparison — tasks

Order matters. Each task names the test that proves it. Bound every
measurement to at most 32 workers (nproc/4 on this 128-CPU host) and 10
minutes. Name and time each measurement, and report the load. Commit
unsigned. Never push.

**Rerun, 2026-11-07** (`rerun-of-339-operator-direction--305a5aec`). A
ticked box was built by the first run, in the commit named. Its proof is
re-run in T9, and a box whose proof fails there is unticked and fixed. An
unticked box is open. The pinned bytes are under `.compare-bins/`, which is
untrusted and never committed (plan, technical context).

## T1 — Records (AC1)

- [x] T1.a The 2026-11-07 dated addenda on 0091 and 0092 (`d5961bb`).
  **Proof:** `git diff --numstat c95386e -- <both files>` shows 0 deleted
  lines.
- [x] T1.b The proposed 0129 row in `docs/decisions/README.md` (`d5961bb`).
  **Proof:** `cargo test --locked --test store_contract_docs`.
- [x] T1.c `docs/evals/README.md` states 0091–0093 as accepted (`d5961bb`).
  The final row update moves to T8.c.
  **Proof:** `grep -nE 'proposed \[009[123]\]' docs/evals/README.md` finds
  nothing.

## T2 — Generic adapter half and batches (AC4, AC5)

- [x] T2.a The split into `mcp.rs` with a `Backend` trait, and the Mailpit
  backend (`0acc0b5`).
  **Proof:** every existing `tests/mail_compare.rs` test passes, and the
  Mailpit `tools()` digest equals protocol v2's
  `budgets.adapter.tools_sha256`.
- [x] T2.b Batches under 0129 §6 (`0acc0b5`).
  **Proof:** the hostile-client suite in `tests/mail_compare.rs`, including
  `cancellation_eof_queue_and_output_bounds_hold`.
- [x] T2.c `--launch BIN` mode with the announcement and the exit-0 cleanup
  receipt (`9783675`).
  **Proof:** the launch-mode tests against the Swarmail binary.

## T3 — MailSlurper provenance and lock v4 (AC6)

- [x] T3.a Verify and capture. In new directories under ignored scratch on
  `/home`:
  - re-hash the zip (`48f2f9ba…55f6`) and the source tarball
    (`83f66f25…359f`) and compare each with the constant (spec, "What the
    pinned bytes establish");
  - extract `mailslurper` from the zip and hash it;
  - extract only the spec R8 capture files from the tarball.

  Re-read every cited fact from those bytes, and stop and report on any
  disagreement. Commit the captures and the ref/release metadata under
  `docs/evals/sources/mailslurper/`, each bound by size, sha256 and blob id
  (`git hash-object`). Never commit `.compare-bins/`, `vendor/`, `www/` or
  the tarball.
  **Proof:** the extended
  `the_tag_chain_and_every_blob_are_rechecked_against_the_captures` passes
  over the captures, and `git ls-files .compare-bins` is empty.
- [x] T3.b Lock v4:
  - archive v3's bytes to
    `docs/evals/history/2026-11-07-sources.lock.v3.json`;
  - make the v4 reader the v3 reader given an expected-target list (v1–v3
    three targets, v4 four), with no copied reader code;
  - append `TargetKind::Mailslurper`;
  - write the MailSlurper entry per spec R8: `locally_computed` zip digest,
    member digest, lightweight tag, archive-attributed sources, unresolved
    tree and build provenance, the licence from `LICENSE`, the config
    template;
  - in `targets.rs`, add the third reserved port `{www_addr}`/`{www_port}`
    and the config template written into scratch.

  **Proof:**
  - `cmp` of the archive against `git show c95386e:docs/evals/sources.lock.json`;
  - the v1/v2/v3 reader tests pass unchanged;
  - a v4 validation test passes, and refuses a v4 lock that synthesises a
    tree for MailSlurper;
  - `mail_compare check --protocol docs/evals/protocol.json --sources
    docs/evals/sources.lock.json` exits 0;
  - `git diff c95386e -- docs/evals/protocol.json` is empty;
  - a MailSlurper launch test proves three owned binds and their release on
    cleanup, and that Mailpit and MailHog launches are unchanged.
- [ ] T3.c The AUTH PLAIN probe, an unscored setup step, run against the
  pinned binary through the launcher. The expected result is: EHLO
  advertises no AUTH, and `AUTH PLAIN …` draws `221 Bye` and a close, with
  no `235` and no `334`. Any other result blocks T7.b and the MailSlurper
  items, and is reported. Record the finding in the lock.
  **Proof:** the ignored test `mailslurper_auth_plain_probe`
  (`MAIL_COMPARE_MAILSLURPER`) asserts the transcript. The transcript is
  committed beside the controls.

## T4 — MailHog and MailSlurper adapters (AC4)

- [x] T4.a `mailhog_mcp.rs` and the `mailhog-mcp` mode (`9783675`; ignored
  real-binary test `51c9f98`).
  **Proof:** the shim mechanics tests and the no-forbidden-tool test.
- [x] T4.b `benches/mail_compare/mailslurper_mcp.rs` and the
  `mailslurper-mcp` mode (spec R5, rerun): list `GET /mail?pageNumber=`,
  search `to`/`from`/`message`, read `GET /mail/:id`, and the labelled poll
  wait. The search description states the `LIKE` substring semantics. No
  delete, prune, attachment, login or version. Set
  `adapter_mode(Mailslurper)` in `evals/src/target.rs` to
  `Some("mailslurper-mcp")`.
  **Proof:**
  - mechanics tests over real SMTP against Swarmail's `mailslurper_router`
    shim (CI);
  - the no-forbidden-tool test, extended to MailSlurper;
  - a description test for the `LIKE` statement;
  - the `evals/tests/target.rs` adapter-mode test.
- [ ] T4.c Run the ignored real-binary tests against the verified pins,
  one at a time: `MAIL_COMPARE_MAILPIT`, `MAIL_COMPARE_MAILHOG` and
  `MAIL_COMPARE_MAILSLURPER`. Each calls every announced tool once,
  read-only. Remove any failing tool from its adapter, never emulate it,
  and re-run.
  **Proof:** the named, timed transcripts with `uptime` before and after,
  committed beside the controls, with every tool's outcome listed.
- [ ] T4.d The wire corpus. The T4.c tests write each competitor's
  responses to fixture-generated mail (list, read, empty result,
  not-found, filtered search) into `docs/evals/wire-corpus/<target>/`, with
  an `index.json` of file sha256s and the capture's executable digest.
  **Proof:**
  - a `tests/mail_compare.rs` test decodes every file through the
    adapter's decoders;
  - an `evals/tests/mailsource.rs` test maps every read through the
    oracle's normalizer;
  - a test refuses an index whose executable digest differs from the lock.

  All of these run in CI.
- [x] T4.e Hostile arguments (spec R5, rerun) on the generic half and every
  backend. The charset is widened only to an id alphabet observed in the
  corpus, and cited.
  **Proof:** per adapter, an upstream request counter stays 0 for:
  - an overlong id;
  - an id with `/`, `..`, `?`, `%` or a space;
  - an over-cap page, count or size.

  Also: the upstream client has redirects off (a test against a loopback
  302), and an oversized or unknown upstream shape gives a typed error.

## T5 — Targets, bridge and runner (AC2, AC3, AC11)

- [x] T5.a (part) The `Target` enum and the backend table in
  `evals/src/target.rs` (`0d93fa4`).
  **Proof:** `evals/tests/target.rs`.
- [ ] T5.a (rest) `matrix --target/--adapter/--target-bin/--sources`, with
  the named refusals: an unknown value, `--brokkr`, a dsh profile, and a
  task that is not applicable.
  **Proof:**
  - unit tests per refusal, each asserting that nothing launched;
  - the existing matrix suite passes unchanged without `--target`.
- [x] T5.b `evals/src/bridge.rs` under spec R3 (`345f847`).
  **Proof:** `evals/tests/bridge.rs` against the real `swarmail mcp`.
- [ ] T5.c The runner's per-target server block (spec R4):
  - `ServerRole::Adapter`;
  - the announcement parse;
  - a `tools_sha256` check against the product;
  - the fixture's `smtp_auth`;
  - the cleanup receipt from the exit status;
  - *(rerun)* adapter stderr to a private trial file.

  **Proof:**
  - `matrix --target swarmail --tier hard` with `oracle` and `null`, end to
    end, in the evals suite;
  - a test that a kill or a non-zero adapter exit stops admission and keeps
    the slot's row as a visible failure (AC11);
  - a test that a mismatched `tools_sha256` refuses;
  - a test that the adapter's stderr lands in the trial file.
- [x] T5.d The fixture's `smtp_auth: none` path (`b4b980d`).
  **Proof:** `evals/tests/smtp_auth.rs`.
- [x] T5.e *(rerun)* Bridge additions: an in-flight bound of 16, with a
  correlated busy error over it, and pretty-printed bodies forwarded as one
  line.
  **Proof:** `evals/tests/bridge.rs` gains:
  - a 17-concurrent-request test, in which the adapter sees 16;
  - a test that a body with embedded newlines reaches the adapter as one
    line.

## T6 — Prompt, wrappers and oracle (AC2, AC7)

- [x] T6.a The competitor header template, with `PROMPT_TEMPLATE` unchanged
  (`0d93fa4`).
  **Proof:** the digest and render tests. The product-digest equality
  follows in T7.b.
- [x] T6.b `EVAL_MCP_SERVER`/`EVAL_MCP_TOOLS`, tool-name validation, and the
  dsh refusal (`5a5396d`).
  **Proof:** the generated-config tests in `evals/src/wrappers.rs`.
- [x] T6.c The `MailSource` trait, with Swarmail, Mailpit and MailHog
  implementations and MailHog MIME decoding (`3c1d3e1`).
  **Proof:** `evals/tests/mailsource.rs`.
- [x] T6.d *(rerun)* The MailSlurper `MailSource`. It reads `body` according
  to `contentType`, reusing the existing MIME decoder for multipart. It uses
  the upstream `to` filter as a pre-filter only, then keeps an exact
  `toAddresses` match.
  **Proof:**
  - normalizer tests over the T4.d corpus;
  - a lookalike-recipient test, in which the substring match returns it and
    the oracle excludes it;
  - an unknown-shape test that errors.

## T7 — Product record, rows and verifier (AC8, AC10, AC11)

- [x] T7.a Four targets, `coordinate.agent`, `settings.smtp_auth`, the MailHog
  refusal removed, and citation by (target, agent, task) (`91328e3`).
  **Proof:** `evals/tests/provenance.rs` and `evals/tests/report.rs`.
- [ ] T7.b Commit `docs/evals/agent-comparison.json` and
  `docs/evals/agent-comparison-cost.json`. Do this only after T3.c and T4.c
  pass. `smtp_auth`, `tools_sha256` and the delivery semantics are copied
  from their results: MailSlurper is `none` and `not-promised`, and the
  recipient filter is `upstream-substring` with an exact oracle match.
  **Proof:**
  - `comparison-check` validates the product;
  - a test refuses a product whose `planned_cells` is not
    tasks × targets × agents × trials = 240;
  - a test refuses a product whose tuple's `tools_sha256` differs from the
    adapter's;
  - the header digests equal T6.a's.
- [ ] T7.c `evals/src/comparison.rs` and the `comparison-check` subcommand,
  with the dry-run, evidence and measure modes.
  **Proof:** one negative test per prerequisite in the fixed order, for
  missing, wrong-content and wrong-binding cases:
  - controls not bound to the product;
  - cost recorded as 0;
  - identity re-measure differs;
  - *(rerun)* receipts HEAD not an ancestor;
  - *(rerun)* an ancestor diff touching `evals/`, `benches/`, `scripts/`,
    `src/`, the lock or the product;
  - *(rerun)* a missing control row;
  - *(rerun)* a torn final line.

  A positive test accepts an ancestor whose diff touches only the evidence
  paths.
- [ ] T7.d `scripts/compare-mail.sh`:
  - `controls` needs only `--product` and the executables;
  - `evidence` and `measure` run the stat chain, then the verifier;
  - `--authorized-product`;
  - the `comparison-integration` refusal is removed.

  **Proof:**
  - wrapper tests per mode, each refusing by name;
  - `grep -c comparison-integration scripts/compare-mail.sh` gives 0;
  - a non-dry measure with an inert builtin-`null` config, on a synthetic
    one-trial product in scratch on `/home`, delegates;
  - without `--authorized-product`, with a wrong one, or with a pending CI
    item, the same measure refuses and spawns no agent (AC10).

## T8 — The free proof (AC7, AC9)

- [ ] T8.a Run `compare-mail.sh controls` on all four targets, with the
  candidate's `swarmail` and the three verified pins:
  - one worker;
  - each target its own named command under `timeout 600`, with
    `/usr/bin/time` and `uptime` before and after.

  Commit `docs/evals/agent-controls-2026-11-07/` (rows, index, contracts,
  identity, receipts, the T3.c and T4.c transcripts).
  **Proof:**
  - every oracle trial passes, and every null trial fails with its idle
    class;
  - the projection digests are equal across targets;
  - the index binds the product digest;
  - every contract file lists one outcome per announced tool;
  - the load and elapsed time are reported.
- [ ] T8.b Run one `compare-mail.sh measure --dry-run` with every
  prerequisite genuine. Commit its transcript.
  **Proof:**
  - exit 0, with the 240-slot plan printed and the pending items named;
  - a test asserts that the dry run spawns nothing but the verifier and
    never opens `--config`.
- [ ] T8.c *(rerun)* `docs/evals/README.md`: replace the 2026-11-07 "Not
  built, and why" block with the outcome. Write in it:
  - the AUTH asymmetry sentence of spec R8;
  - MailSlurper's declared accept-before-store and its substring filter;
  - the unresolved tree;
  - the unused shelf upstreams;
  - what only CI or the operator proves.

  Update the T7–T10 and AC rows.
  **Proof:** `tests/store_contract_docs.rs` link checks pass, and
  `grep -c 'Not built, and why' docs/evals/README.md` gives 0.

## T9 — Gates (AC12)

- [ ] T9.a Run the whole house inventory `--locked`:
  - root fmt, clippy, test and `build --bin swarmail`;
  - `evals` fmt, clippy and test, with `SWARMAIL_BIN` set to the
    candidate's binary;
  - `scripts/coverage-gate.sh` and its selftest;
  - `scripts/publish-guard-test.sh` and `scripts/seal-branch-test.sh`;
  - `cargo deny check licenses`.

  If the sandbox has no brokkr binary, report that the bundle-compile gate
  runs in verify. Unticks any box above whose proof fails.
  **Proof:** the gate transcript, plus:
  - `git diff c95386e -- src/ .github/ .brokkr/ Cargo.lock evals/Cargo.lock
    deny.toml` is empty;
  - `git ls-files .compare-bins` is empty.

## Rerun outcome, 2026-11-07 (`rerun-of-339-operator-direction--305a5aec`)

Ticked above and built this run: T3.a and T3.b (`841bfd2`), T4.b and T4.e
(`6191a9d`, with `adapter_mode` set in the oracle commit), T5.e (the bridge
commit) and T6.d (the oracle commit). Their proofs ran in T9's gate run.

Partly done, and why the box stays open:

- **T3.c (the AUTH probe) and T4.c (real-binary tests): Mailpit and MailHog
  done, MailSlurper blocked.** The probe and the ignored MailSlurper test are
  written and proved against a stand-in and scripted servers, but the pinned
  linux executable was built with `CGO_ENABLED=0` and exits on its SQLite
  config ("go-sqlite3 requires cgo to work. This is a stub"), so neither ran
  against it. The real Mailpit and MailHog runs passed, with their transcripts
  in `docs/evals/wire-corpus/real-runs/`.
- **T4.d (the wire corpus): Mailpit and MailHog committed and decoded in CI;
  MailSlurper has none** (nothing to capture from).

Not started: T5.a (rest), T5.c, T7.b (blocked on T3.c and T4.c for
MailSlurper), T7.c, T7.d, T8.a, T8.b. `scripts/compare-mail.sh` keeps its
`comparison-integration` refusal, because the runner cannot yet serve a
competitor. T8.c is done as a README block, which the T8.c box would otherwise
leave open. T9.a ran every gate in the inventory except the brokkr bundle
compile, which has no binary in this sandbox and runs in verify.
