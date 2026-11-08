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

**Third rerun, 2026-11-08** (`third-rerun-of-339-integrate-mai-60435935`,
from `4d89de1`). Main moved to `55f211d`, where accepted 0130 governs the
wire driver (spec, "The 2026-11-08 third rerun"; R12, R13; AC14–AC18). Do
not redo the ticked boxes T1–T9. Do T10–T17 in order. A T1–T9 proof that
names `c95386e` as the base for `src/`, locks, `.github/` or `.brokkr/`
reads `origin/main` after T10 (plan, "Criteria this rerun reads
differently"). A T3 or T13 lock-schema proof that R12 supersedes is not
rerun; AC14/AC15 replace it. Stop and report on any R12.l condition.
Nothing paid runs.

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

- [x] T1.d (second rerun, 14fc67c) *(rerun 2)* Record the operator's ruling:
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
- [x] T3.d (second rerun, e60da58) *(rerun 2)* The source-built lock entry (spec R11):
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
- [x] T3.e (second rerun, b5f83f3 (transcript), the tree itself in e60da58) *(rerun 2)* Recompute the tarball against the tree, once. Verify
  `83f66f25…359f`. Extract it into a new scratch directory on `/home`. Run
  `git init`, `git add -A` and `git write-tree` there, and compare the
  result with `5f29f08a…0729`. A mismatch stops the MailSlurper items, which
  are reported blocked. Never edit the tree or the extraction to agree.
  **Proof:** the named transcript (commands, digests, the tree id),
  committed beside the controls.
- [x] T3.c (second rerun, e60da58) The AUTH PLAIN probe, an unscored setup step, run against the
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
- [x] T4.c (second rerun, 5694dd2) Run the ignored real-binary tests against the verified pins,
  one at a time: `MAIL_COMPARE_MAILPIT`, `MAIL_COMPARE_MAILHOG` and
  `MAIL_COMPARE_MAILSLURPER`. *(rerun 2: Mailpit and MailHog ran in the
  first rerun, and their transcripts are in
  `docs/evals/wire-corpus/real-runs/`. Run MailSlurper against `eb6cd97d…`
  now, and re-run the other two only if their adapters changed.)* Each calls every announced tool once,
  read-only. Remove any failing tool from its adapter, never emulate it,
  and re-run.
  **Proof:** the named, timed transcripts with `uptime` before and after,
  committed beside the controls, with every tool's outcome listed.
- [x] T4.d (second rerun, 5694dd2) The wire corpus. The T4.c tests write each competitor's
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
- [x] T5.a (rest) (second rerun, 82d7735) `matrix --target/--adapter/--target-bin/--sources`, with
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
- [x] T5.c (second rerun, 82d7735, 4fad21a) The runner's per-target server block (spec R4):
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
- [x] T7.b (second rerun, 82d7735) Commit `docs/evals/agent-comparison.json` and
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
- [x] T7.c (second rerun, 4fad21a) `evals/src/comparison.rs` and the `comparison-check` subcommand,
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
- [x] T7.d (second rerun, 4fad21a, 13d0750, ad4526f, 2cbc1a6) `scripts/compare-mail.sh`:
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

- [x] T8.a (second rerun, the final controls commit) Run `compare-mail.sh controls` on all four targets, with the
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
- [x] T8.b (second rerun, a766784) Run one `compare-mail.sh measure --dry-run` with every
  prerequisite genuine. Commit its transcript.
  **Proof:**
  - exit 0, with the 240-slot plan printed and the pending items named;
  - a test asserts that the dry run spawns nothing but the verifier and
    never opens `--config`.
- [x] T8.c (second rerun, this commit) *(rerun)* `docs/evals/README.md`: replace the 2026-11-07 "Not
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

- [x] T9.a (second rerun, see the outcome below) Run the whole house inventory `--locked`:
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

## T10 — Merge `main` and restore the frozen bytes (AC14) *(rerun 3)*

- [x] T10.a (third rerun, 58f3617) Run `git merge origin/main` on `4d89de1`. Never rebase, squash
  or amend. Before resolving any other file, run
  `git checkout origin/main -- docs/evals/sources.lock.json
  docs/evals/protocol.json docs/evals/history/`. Restore each file main
  tracks under `docs/evals/sources/` (`git ls-tree -r --name-only
  origin/main docs/evals/sources`). Then `git rm` the branch's
  `docs/evals/history/2026-11-07-sources.lock.v3.json`, after checking that
  its sha256 equals main's `2026-10-30-sources.lock.v3.json`; if it does
  not, stop and report.
  **Proof:** `sha256sum docs/evals/sources.lock.json docs/evals/protocol.json`
  gives `ed201599dd57441cbcf0c7e66e7c82a56dd217ef683b7bf4e72ae7ce9d3a04b1`
  and `229babaac849ae832ecb1a9f8ec742c970d0f36edc9d26d4ff9bb9c3c0d7224e`.
  `git diff origin/main -- docs/evals/sources.lock.json docs/evals/protocol.json
  docs/evals/history/ docs/evals/sources/` is empty. Main's frozen-sha test
  passes: `cargo test --locked --test mail_compare -- frozen`. Check the
  filter against the test's name in `eight_targets.rs`.
- [x] T10.b (third rerun, 58f3617) Resolve every content conflict by the rules in R12.c–R12.k. No
  marker may survive.
  **Proof:** `git grep -nE '^(<<<<<<<|>>>>>>>)( |$)'` is empty, and
  `git log --merges -1 --format=%P` names `55f211d`.

## T11 — The driver is main's; the adapters sit on it (AC15) *(rerun 3)*

- [x] T11.a (third rerun, 58f3617, 859c465) `benches/mail_compare/protocol.rs` and `targets.rs` take
  main's side whole. Re-add only what a compile error in the adapter modes
  demands, at most one read accessor (R12.c). Remove the branch's lock-v4
  layer as R12.c lists it.
  **Proof:** `git diff origin/main -- benches/mail_compare/protocol.rs`
  adds no type, field, lock or protocol rule (empty is expected), and `git diff origin/main -- benches/mail_compare/targets.rs`
  shows only the accessor (if any) and T14.a's constant.
  `cargo clippy --workspace --all-targets --locked -- -D warnings` is
  clean.
- [x] T11.b (third rerun, 859c465) `benches/mail_compare.rs` keeps main's usage text and dispatch
  and adds the adapter modes. `mailslurper-mcp` launches
  `TargetKind::MailslurperSourceBuild` through `LaunchSpec::from_pin`.
  Every added `TargetKind` match is exhaustive. An adapter entry point
  that receives a target other than its own refuses it by name and spawns
  nothing.
  **Proof:** a new test in `tests/mail_compare.rs` builds the
  `mailslurper-mcp` backend spec from main's lock and asserts its
  executable digest `eb6cd97d…58ff` and its kind
  `MailslurperSourceBuild`. If an entry point takes a target, a refusal
  test asserts the target's name in the error and no spawn.
  `git diff origin/main -- benches tests evals | grep -nE '^\+.*\b_ =>'`
  shows no arm of a `TargetKind`/`Target` match.
- [x] T11.c (third rerun, 58f3617) In `tests/mail_compare.rs`, keep every wire and lock test from
  main verbatim. Delete only the branch's tests of the removed layer (plan,
  step 3), and list their names in the commit body. Port the adapter
  mechanics tests and the ignored real-binary tests to main's types.
  **Proof:** `cargo test --locked --test mail_compare` passes. The deleted
  names in the commit body match plan step 3's classes, and no test from
  `origin/main` is missing:
  `git diff origin/main -- tests/mail_compare.rs tests/mail_compare/ | grep -E '^-.*fn [a-z_]+\('`
  lists only lines that are re-added or modified, never a removed main test.

## T12 — The sources main does not bind (AC15) *(rerun 3)*

- [x] T12.a (third rerun, 58f3617, 859c465) Hash every branch file under `docs/evals/sources/mailslurper/`
  against main's documents. `git rm` the sha256-equal ones. Move the twelve
  adapter-cited sources and `go.mod`/`go.sum` (and any non-equal file) to
  `docs/evals/agent-sources/mailslurper/`. Add the `(path, archive member,
  sha256)` table to `tests/mail_compare.rs`, with a default rehash test and
  an `#[ignore]` re-extraction test against archive `83f66f25…359f`, using
  main's archive helpers (make them `pub(super)` if needed). Repoint every
  adapter-source citation (doc comments, README) to the new paths.
  **Proof:** the default table test passes. The ignored test passes with
  the archive present, as a named, timed command. `git diff origin/main --
  docs/evals/sources/` stays empty. `grep -rn 'sources/mailslurper/[A-Z]'
  benches tests docs/evals/README.md` finds nothing.

## T13 — The eval crate, the product record and the wrapper (AC15) *(rerun 3)*

- [x] T13.a (third rerun, 58f3617) Resolve the evals conflicts by keeping both sides (R12.h). Fold
  `send_auth` into main's `send_stored(…, SmtpAuth)`.
  **Proof:** `cd evals && cargo test --locked` (with `SWARMAIL_BIN`), and
  `grep -rn 'fn send_auth' evals/` finds nothing.
- [x] T13.b (third rerun, 859c465) Add `Target::lock_entry()`, an exhaustive match. Repoint
  `validate_tuple` and `tuple_agrees_with_lock` to it. Edit the MailSlurper
  tuple's `lock_entry` to `"mailslurper-source-build"` and its
  `artifact_kind` to `"executable"`, leaving every other field alone. If
  another field disagrees with main's entry, stop (R12.l).
  **Proof:** new evals tests:
  - `lock_entry()` for all four targets;
  - a tuple with `lock_entry: "mailslurper"` is refused by name;
  - a tuple with any other wrong entry is refused by name;
  - the committed product validates against main's lock (`comparison-check`).

  `git diff 4d89de1 -- docs/evals/agent-comparison.json` shows exactly two
  changed lines.
- [x] T13.c (third rerun, 859c465) `scripts/compare-mail.sh` uses main's grammar (R12.i) and drops
  `--mailslurper`. The agent modes require a `--bin` for each competitor
  `lock_entry` in the product record.
  **Proof:** the wrapper tests:
  - `--mailslurper x` exits non-zero as an unknown flag;
  - `controls` without `--bin mailslurper-source-build=…` refuses by that
    name;
  - main's wire-mode wrapper tests pass unchanged.
- [x] T13.d (third rerun, 859c465) The wire corpus (R12.j). Compare the corpus index's MailSlurper
  template sha256 with main's `mailslurper-source-build` `config_file`. If
  they are equal, only the index's lookup key changes. Otherwise, recapture
  with the ignored capture test under `timeout 600 taskset -c 0-31`.
  **Proof:** the corpus index tests pass against main's lock. If a
  recapture ran, its transcript is in the gates transcript.

## T14 — The bounded launch-slot retry (AC16) *(rerun 3)*

- [x] T14.a (third rerun, 859c465) Add `pub const NO_LISTENER_ADDRESSES: &str = "no listener
  addresses"` to `targets.rs` and use it at every producer.
  **Proof:** `grep -rn '"no listener addresses"' benches tests` finds only
  the definition.
- [x] T14.b (third rerun, 859c465) Move `4d89de1`'s loop into `launch_shim_with_fresh_slot(chooser)`,
  with its semantics unchanged: fresh slot, at most 5 attempts,
  `facts.proved()` on each refusal, any other error panics at once. It
  returns `(slot, Owned, attempts)`. The MailSlurper adapter test uses it.
  **Proof:**
  `the_mailslurper_adapter_lists_and_reads_over_real_smtp_into_swarmails_shim`
  passes.
- [x] T14.c (third rerun, 859c465) Add the forced-collision test: hold a real `TcpListener` on
  the first candidate across the launch.
  **Proof:** the test asserts `attempts == 2`, proved cleanup on the
  refused attempt, and that the held listener still accepts a connection
  afterwards. It runs in the default suite.
- [x] T14.d (third rerun, 859c465) Add the exhaustion test: hold five listeners and feed them as
  the five candidates.
  **Proof:** a `#[should_panic(expected = "swarmail launches")]` test, plus
  a counter in the chooser showing exactly five candidates were drawn.
- [x] T14.e (third rerun, 859c465) After the merge, run
  `grep -n 'bind("127.0.0.1:0")' tests/mail_compare.rs tests/mail_compare/*.rs`.
  Apply the helper to each hit that reserves, releases and passes a port to
  a child that is expected to bind it. Leave main's
  `real_mailslurper_official_does_not_start` byte-identical.
  **Proof:** the grep's hits, each classified in the commit body.
  `git diff origin/main -- tests/mail_compare/eight_targets.rs` changes
  only helper visibility (T12.a), if anything.

## T15 — Records (AC17) *(rerun 3)*

- [x] T15.a (third rerun, 022c9d2) Write the 0129 addendum "Addendum — 2026-11-08 (integration
  with accepted 0130)" with every point R12.k lists. The product sha and
  the controls directory are filled in after T16. Leave the status
  `proposed` and every earlier section unedited.
  **Proof:** `git diff 4d89de1 -- docs/decisions/0129-agent-suite-comparison.md`
  shows only added lines, and `grep -n 'Status:\*\* proposed'` still
  matches.
- [x] T15.b (third rerun, 022c9d2) Add dated pointers: in 0091, main's text first and then the
  branch's addenda; one 2026-11-08 line in 0130. In the decisions index,
  keep main's rows and 0129's row, and reword 0129's row as R12.k says. In
  `docs/evals/README.md`, put main's text first and then the branch's
  section, naming `mailslurper-source-build`, citing 0130 and saying
  "built from source".
  **Proof:** `git diff origin/main --numstat -- docs/decisions/0091-*.md
  docs/decisions/0092-*.md docs/decisions/0093-*.md docs/decisions/0130-*.md`
  shows 0 deleted lines. `cargo test --locked --test store_contract_docs`
  passes.

## T16 — Controls on the final code head (AC18) *(rerun 3)*

- [x] T16.a (third rerun, the controls commit) When every code change is committed, rerun the oracle and null
  controls for each target into `docs/evals/agent-controls-2026-11-08/`.
  Use one worker per target, each as its own named command under
  `timeout 600 taskset -c 0-31`, with `uptime` before and after.
  Regenerate the dry-run transcript beside it. `git rm -r
  docs/evals/agent-controls-2026-11-07/`, and repoint the path constants in
  `evals/tests/comparison.rs` and the README links.
  **Proof:** `comparison-check` accepts the new directory. AC7's
  assertions hold: every oracle trial passes, every null trial fails with
  the idle class, and the projection digests are equal across targets. The
  evals suite passes with the repointed constants. Only evidence paths
  change after the controls commit.

## T17 — Gates (AC18) *(rerun 3)*

- [x] T17.a (third rerun, the controls commit) Run the whole house inventory `--locked` on the final tree:
  - root fmt, clippy, test and `build --bin swarmail`;
  - `evals` fmt, clippy and test against the candidate's own `swarmail`;
  - `scripts/coverage-gate.sh` and its selftest;
  - `scripts/publish-guard-test.sh` and `scripts/seal-branch-test.sh`;
  - `cargo deny check licenses`.

  Report that the bundle-compile gate runs in verify if `which brokkr` is
  empty. Report a flake outside this branch with its output, and never
  patch it in `src/`.
  **Proof:** the regenerated gates transcript, with each command, its
  time and the load. Also:
  - `git diff origin/main -- src/ Cargo.lock evals/Cargo.lock deny.toml
    .github .brokkr` is empty;
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

## Second rerun outcome, 2026-11-07 (`second-rerun-of-339-operator-dir-e9c4d893`)

Every open box above is built and proved in the commit named. The free half ran
for real: `compare-mail.sh controls` on all four targets and one
`measure --dry-run` with every prerequisite genuine
(`docs/evals/agent-controls-2026-11-07/`). Nothing paid ran.

What differs from the plan, and why:

- **The AUTH probe accepts one or two `221 Bye` lines.** The pinned build draws
  two, all three runs. The reviewed source explains the second (the command
  goroutine's `break` only leaves its `select`, so it reads again and the read error
  on the closed connection draws another goodbye). EHLO advertises no AUTH, there is
  no `235` and no `334`, and the server closes. T3.c's stop rule asked for exactly
  `221 Bye`; this reading keeps its meaning and is stated in the lock, 0129's
  addendum and the README.
- **The real shapes overrode the first rerun's normalizer.** MailSlurper's
  `boundary` and `transferEncoding` are always empty, a plain body carries `\n<br/>`
  and HTML escapes, a multipart keeps only its HTML part, and an unknown id is an
  empty item at 200. The oracle's MailSlurper normalizer and the adapter's read were
  rewritten from the captured corpus, and `recipient-crowd`'s control reads the HTML
  anchor when a backend exposes no text part (T6.d, T4.d).
- **The adapter is spawned by `spawn_adapter`, not as a `ServerRole::Adapter`.** The
  announcement is read from the adapter's own stdout before the bridge takes the
  pipe, so no second reader exists. The behaviour spec R4 asks for is the same.
- **The bridge answers a later `initialize` from the first.** Several clients reach
  one stdio session and every adapter refuses a second `initialize`.
- **`evidence` is the verifier plus a citation** of `<final>/rows.jsonl` by
  `comparison-cite`; with no paid run there is nothing to cite, and the mode refuses
  by name (`final-rows`).
- **The eval-suite receipt has its own bound.** The locked evals suite takes about
  twenty minutes at 32 threads, so it is not under the ten-minute `timeout 600`
  that holds each target's controls.
- **Test seams in the wrapper** (`COMPARE_MAIL_EVALS_BIN`, `_SWARMAIL_BIN`,
  `_ADAPTER_BIN`, `_EVAL_SUITE`) name prebuilt stand-ins so the script's own logic
  is proved without a build; the verifier's checks are proved on real bytes in the
  eval crate. A real `controls` run never uses them.
- **The receipts went stale three times** (verifier's git-status reading, the
  candidate binary relinked by a later test run, the harness embedding the commit),
  each a code change after the controls; the controls ran on each final head and the
  last run is the committed one.

Gates, at the final code head `2cbc1a6` (`transcripts/gates.txt` has the commands,
times and load): fmt, clippy, workspace tests, build, the evals fmt/clippy/suite,
the coverage gate and its selftest, the publish-guard and seal proofs and
`cargo deny check licenses` are green. The coverage gate refused twice on a flake
(an existing `src/store.rs` test branch; this branch's own MailSlurper shim launch
answering `NotReady` under instrumented load) and passed on the third run, exact
100/100/100. **`brokkr compile` was not run: this sandbox has no brokkr binary, so
the verify seat runs it.** `git diff c95386e -- src/ .github/ .brokkr/ Cargo.lock
evals/Cargo.lock deny.toml` is empty and `git ls-files .compare-bins` is empty.
The operator holds the PR (`scripts/compare-mail.sh` changes), and it contends with
#340 for the lock's MailSlurper entry.

## Third rerun outcome, 2026-11-08 (`third-rerun-of-339-integrate-mai-60435935`)

Every T10–T17 box above is built and proved in the commit named. Nothing paid
ran.

What differs from the plan, and why:

- **No read accessor was needed.** `protocol.rs` is `main`'s file, byte for byte
  (`git diff origin/main -- benches/mail_compare/protocol.rs` is empty), and
  `targets.rs` differs from `main`'s only by `NO_LISTENER_ADDRESSES` and its three
  uses. The adapter driver reads `Pin` through `main`'s own public fields.
- **The adapter's target is a trait constant, not an argument.** `Backend::TARGET`
  carries each backend's lock row (`MailslurperSourceBuild` for MailSlurper), and
  `mcp::adapter_target` is the exhaustive refusal for every other target; a test
  covers each refused target by name and the lock-without-the-row case.
- **`wire::path_segment` is gone.** `main`'s `query_value` now keeps the unreserved
  characters, which is what `path_segment` did, so `wire.rs` is `main`'s file too.
- **Main's `pinned()` helper builds its spec with `LaunchSpec::from_pin`** and reads
  the committed lock 4, so the source-built MailSlurper's three listeners and config
  file launch as the wire run launches them. Its environment name follows
  `eight_targets.rs`: `MAIL_COMPARE_MAILSLURPER_SOURCE_BUILD`, so that
  `MAIL_COMPARE_MAILSLURPER` stays the official release's.
- **A bare `controls` keeps `main`'s refusal** (`wrapper_paid_modes_refuse_as_before`
  is `main`'s test and stays verbatim): it names the first prerequisite of the
  chain, with a stderr hint that `controls` needs `--product` and a `--bin` for
  each competitor lock entry of it.
- **The deliverability task is refused for a competitor by name**, in the runner
  and in `check_tasks`. `main`'s `fixture_args` keeps its signature; the runner
  passes the SMTP address where a competitor has no Swarmail HTTP plane, never
  read because no task a competitor can take reads reports.
- **The corpus needed a recapture.** 0130's `config_file` is a different template
  from the old lock's, so the MailSlurper corpus was recaptured with the ignored
  capture test under the verified pin.
- **The controls ran twice.** The first run (head `cc1dbe8`) was valid until the
  records were committed: the verifier calls any change outside the controls, the
  README and `specs/` stale, and `docs/decisions/` is none of those. The records
  were committed first and the controls rerun on that head (`022c9d2`); the first
  run's directory was set aside and never committed.
- **One flake outside this branch.** `a_term_resistant_child_is_killed_within_one_composed_cleanup_budget`
  failed once in three full workspace runs (a launcher-reserved, released port was
  answered by another test's listener); 30 of 30 alone, and unchanged by this
  branch. Its output is in `agent-controls-2026-11-08/transcripts/gates.txt`.
- **`brokkr compile` was not run**: this sandbox has no brokkr binary, so the verify
  seat runs it.

The `main`-side gates that this branch's tests share (`cargo test --locked --test
mail_compare -- frozen`) pass. The operator still holds the PR (`scripts/compare-mail.sh`
changes), accepts 0129, supplies the CI run for the receipts' `ci` item and confirms
the 240 paid sessions.
