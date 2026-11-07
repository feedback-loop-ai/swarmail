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

**Second rerun, 2026-11-07** (`second-rerun-of-339-operator-dir-e9c4d893`,
from `6e0098b`). The operator ruled that the comparison's MailSlurper is the
cgo build in `.compare-bins/2026-11-07-supply/mailslurper-cgo/`, labelled
"built from source" (spec, "The second 2026-11-07 rerun"). Do not redo the
ticked boxes. Do the open ones in this order: T1.d, T3.d, T3.e, T3.c, T4.c,
T4.d, T5.a (rest), T5.c, T7.b, T7.c, T7.d, T8.a, T8.b, T8.c, T9.a. The supply
is untrusted: copy each file into a new directory under ignored scratch on
`/home`, compare it with the supply's `SHA256SUMS` hex as a constant, run
nothing from it except the verified `mailslurper` (`eb6cd97d…58ff`), and run
any Python over it with `-I`. Nothing paid runs.

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

- [ ] T1.d *(rerun 2)* Record the operator's ruling:
  - add a second section, "Addendum — 2026-11-07 (operator ruling: the
    source-built MailSlurper)", to proposed 0129, and leave the first
    addendum unedited. It quotes the ruling and states:
    - the official binary's failure, with its zip and member digests and
      exit status 1;
    - the build: binary, source and builder-image digests, `CGO_ENABLED=1`,
      `-trimpath`, and verified modules;
    - that the build reports v1.14.1 under `release-1.15.0`;
    - that "built from source" claims these bytes were built from the
      pinned, tag-named source as recorded, and does not claim to
      reproduce upstream's release;
    - that it answers the first addendum's open question;
    - the lock fields R11 adds (`built_from_source`, `build`,
      `official_release`, `git_commit`) and the observed root tree;
    - the glibc 2.31 floor.
  - Add one pointer bullet to 0091's existing 2026-11-07 addendum: for
    MailSlurper in this story, §1's official evidence is met by the official
    source plus the labelled source-built artifact, under the operator's
    2026-11-07 ruling recorded in 0129.

  **Proof:**
  - `git diff --numstat 6e0098b -- docs/decisions/0129-agent-suite-comparison.md
    docs/decisions/0091-fair-mail-comparison.md` shows 0 deleted lines;
  - `git diff --numstat c95386e -- docs/decisions/0091-fair-mail-comparison.md`
    shows 0 deleted lines;
  - `git diff 6e0098b -- docs/decisions/0092-official-mail-comparison.md
    docs/decisions/0093-source-mail-comparison.md` is empty;
  - `cargo test --locked --test store_contract_docs` passes.

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
- [ ] T3.d *(rerun 2)* The source-built lock entry (spec R11):
  - in `protocol.rs`, add `ArtifactKind::BuiltFromSource`, the optional
    `Artifact.build` and `Artifact.official_release`, and
    `DocKind::GitCommit`;
  - let a lightweight tag's tree be observed only when the `git_commit` and
    `git_tree` documents bind it;
  - add the root-blob check;
  - commit the build evidence under `docs/evals/sources/mailslurper/build/`
    and the commit and tree JSON under `docs/evals/sources/mailslurper/git/`,
    each verified first against the supply's `SHA256SUMS` constants;
  - rewrite the MailSlurper entry of `docs/evals/sources.lock.json` in place
    (still v4), and reword `version_note`.

  Never commit the binary, the build logs, the supply's scripts or the image
  index.
  **Proof:**
  - one negative test in `tests/mail_compare.rs` for each R11 refusal;
  - the existing `archive` and `executable` tests and the v1–v3 reader tests
    pass unchanged;
  - a test rebuilds the tree object from the committed tree JSON with
    `git hash-object -t tree --stdin` and gets `5f29f08a…0729`;
  - `the_tag_chain_and_every_blob_are_rechecked_against_the_captures` covers
    the build and git documents;
  - `mail_compare check --protocol docs/evals/protocol.json --sources
    docs/evals/sources.lock.json` exits 0;
  - `git diff c95386e -- docs/evals/protocol.json` is empty;
  - `git ls-files .compare-bins` is empty.
- [ ] T3.e *(rerun 2)* Recompute the tarball against the tree, once. Verify
  `83f66f25…359f`. Extract it into a new scratch directory on `/home`. Run
  `git init`, `git add -A` and `git write-tree` there, and compare the
  result with `5f29f08a…0729`. A mismatch stops the MailSlurper items, which
  are reported blocked. Never edit the tree or the extraction to agree.
  **Proof:** the named transcript (commands, digests, the tree id),
  committed beside the controls.
- [ ] T3.c The AUTH PLAIN probe, an unscored setup step, run against the
  pinned binary through the launcher. *(rerun 2)* The pinned binary is the
  source build `eb6cd97d…58ff` (T3.d). The transcript names that digest and
  says that the published executable does not start and was not probed. The expected result is: EHLO
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
  `MAIL_COMPARE_MAILSLURPER`. *(rerun 2: Mailpit and MailHog ran in the
  first rerun, and their transcripts are in
  `docs/evals/wire-corpus/real-runs/`. Run MailSlurper against `eb6cd97d…`
  now, and re-run the other two only if their adapters changed.)* Each calls every announced tool once,
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
  - *(rerun 2)* The index binds each target's config template sha256, and a
    test refuses a mismatch with the lock.
  - *(rerun 2)* The MailSlurper corpus holds a body that `GetMail`
    base64-decodes and one it does not. A normalizer test maps each. If a
    branch cannot occur, the index records why.

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
  task that is not applicable. *(rerun 2)* Add the input bindings of spec
  R2: regular-file and bounded reads of `--sources` and `--product`, and
  `--target-bin` hashed against the product tuple, which must equal the
  lock. All of this happens before anything is spawned.
  **Proof:**
  - unit tests per refusal, each asserting that nothing launched;
  - *(rerun 2)* tests for a FIFO and a directory as `--sources`, an
    over-bound `--product`, a wrong-digest `--target-bin` (with both
    digests in the message), and a product that disagrees with the lock;
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
  - the header digests equal T6.a's;
  - *(rerun 2)* MailSlurper's tuple says `artifact_kind:
    "built_from_source"`, `version_report: "v1.14.1"` and
    `executable_sha256` `eb6cd97d…`, copied from the lock.
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
  - *(rerun)* a torn final line;
  - *(rerun 2)* a product tuple whose `artifact_kind` or `version_report`
    differs from the lock;
  - *(rerun 2)* a controls directory with no rows for one target.

  A positive test accepts an ancestor whose diff touches only the evidence
  paths.
- [ ] T7.d `scripts/compare-mail.sh`:
  - `controls` needs only `--product` and the executables;
  - `evidence` and `measure` run the stat chain, then the verifier;
  - `--authorized-product`;
  - the `comparison-integration` refusal is removed;
  - *(rerun 2)* `eval_prerequisite` runs `cargo build --locked --offline` in
    `evals/` and names a failure `eval-build`, then runs the verifier.

  **Proof:**
  - wrapper tests per mode, each refusing by name;
  - *(rerun 2)* a wrapper test in which the eval build fails, and the
    refusal is `eval-build`;
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
  identity, receipts, the T3.c, T3.e and T4.c transcripts).
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
  - the unresolved tree; *(rerun 2: replace this with the observed root
    tree and the T3.e recomputation)*
  - *(rerun 2)* that the MailSlurper measured is "built from source"
    (`eb6cd97d…`), how it was built, that it reports v1.14.1 under
    `release-1.15.0`, that it links glibc 2.31 or newer, and that the
    published linux executable does not start (`CGO_ENABLED=0` stub);
  - the unused shelf upstreams;
  - what only CI or the operator proves.

  Update the T7–T10 and AC rows.
  **Proof:** `tests/store_contract_docs.rs` link checks pass, and
  `grep -c 'Not built, and why' docs/evals/README.md` gives 0.
  *(rerun 2)* `grep -c 'built from source' docs/evals/README.md` is at least
  1, and the README no longer says the MailSlurper items are blocked.

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
  - *(rerun 2)* Each measurement is a named command, timed with
    `/usr/bin/time` and run under `timeout 600` with at most 32 workers.
    `uptime` is recorded before and after it, and the load is reported. The
    PR notes say that the operator holds the PR, because
    `scripts/compare-mail.sh` changes, and they flag the lock contention
    with #340.

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
