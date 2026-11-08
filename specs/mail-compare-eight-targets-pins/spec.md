# Eight-target mail comparison, finished from the pinned bytes

Date: 2026-11-07. Status: draft. Issue #340, part of #163 (Epic #164).
Run: `rerun-of-340-operator-direction--d9e1a271` (operator direction
2026-11-07). Branch `repair/issue340-compare-benches`, continuing from
`4012f31`. Base: `main` at `c95386e`. Recipe: `claude-careful`.

Decision: 0130 — [Eight comparison targets: source lock v4 and protocol v3](../../docs/decisions/0130-eight-target-mail-comparison.md), proposed; only the operator accepts.

Parent design: [`specs/mail-compare-eight-targets/`](../mail-compare-eight-targets/spec.md)
(`98fdd25`). It stays as written, and the code was built to it
(`7237c93`…`4012f31`). Its requirements R1–R9, acceptance criteria AC1–AC19 and
stop triggers S1–S6 stay normative. This spec does not restate them. It
decides how the rows that design left `pending` under S6 are finished from the
bytes that are now in the worktree, and which rows stay open.

Authorities (accepted): [0091](../../docs/decisions/0091-fair-mail-comparison.md),
[0092](../../docs/decisions/0092-official-mail-comparison.md),
[0093](../../docs/decisions/0093-source-mail-comparison.md),
[0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md),
[0056](../../docs/decisions/0056-signing-policy.md), 0045 §8.

## What

The previous run built the types, validators, decoders, POP3 reader, N-listener
lifecycle, wrapper and docs. It stopped at S6 because the pinned upstream bytes
were not visible. Five lock v4 entries are therefore `pending`
(`docs/evals/sources.lock.json`: `mailslurper`, `smtp4dev`, `inbucket`,
`greenmail`, `alkemio-mailslurper`). Their launch facts were, in their own
words, "drafted from the tool's upstream documentation as recalled".

The bytes are now under `.compare-bins/` (git-ignored, untrusted, never
committed). This delivery:

1. verifies each artifact's digest before anything reads it;
2. **re-derives every launch, environment, config, listener and cap fact from
   those bytes**, and carries no drafted fact forward without a citation;
3. turns `mailslurper`, `inbucket` and `greenmail` (with its JRE runtime) into
   full lock v4 pins, using only the vocabulary that already exists;
4. keeps `smtp4dev` and `alkemio-mailslurper` `pending`, with byte-grounded
   reasons, and reports each one to the operator with the exact bytes or
   ruling it needs;
5. runs the real-binary proofs for the pinned rows, bounded and labelled.

It collects nothing, starts no agent session and runs nothing paid.

## Why

The operator directed the rerun on 2026-11-07: finish the blocked and pending
items "using these bytes", reference them by digest and by worktree-relative
path, and "never write upstream facts from memory". A digest that is verified
against a launch that someone recalled from memory would give a guessed method
the look of a proven one. So the launch is re-derived along with the digest.

A row that the bytes cannot finish stays open. Its alternative would be a
license or a loader that nobody captured, which 0091 §2 refuses however green
the tests are.

## What the bytes show (observed in the design seat; implement re-verifies)

Every path is relative to the worktree. Every digest is sha256. The design seat
observed these with `sha256sum`, `unzip -l`, `tar -t`, `readelf` and `file`. The
implementer must re-verify each one (T0) and must not copy them from here.

| Row | Artifact | sha256 | Corroboration in the bytes |
|---|---|---|---|
| mailpit | `.compare-bins/2026-11-07/mailpit-linux-amd64.tar.gz`, `.compare-bins/2026-11-07/mailpit` | `4b5993c9…ec890a93c`, `262cfa28…629a8edec8` | both equal the committed v3 lock |
| mailhog | `.compare-bins/2026-11-07/MailHog_linux_amd64` | `e2ed634d…c67c5d09c7` | equals the committed v3 lock |
| mailslurper | `.compare-bins/2026-11-07-mailslurper/mailslurper-1.15.0-linux.zip` | `48f2f9ba…d6e6af3e55f6` | none is published: `locally_computed` |
| mailslurper source | `.compare-bins/2026-11-07-mailslurper-src/mailslurper-release-1.15.0-src.tar.gz` | `83f66f25ac000fc2f5680514719b7b75a29dafccb7e782597077325fb900359f` | equals the brief; the tarball starts with a pax global header |
| smtp4dev | `.compare-bins/2026-11-07-smtp4dev/Rnwood.Smtp4dev-linux-x64-3.16.1.zip` | `3b9395b9…7baf909c46` | named as an asset in `rnwood_smtp4dev-releases.json` |
| inbucket | `.compare-bins/2026-11-07-inbucket/inbucket_3.2.0_linux_amd64.tar.gz` | `0d7d76ed…d0838e0e24` | equals `inbucket_3.2.0_checksums.txt` |
| greenmail | `.compare-bins/2026-11-07-greenmail/greenmail-standalone-2.1.15.jar` | `91d58798…e1a31391b1` | equals the `.sha256` file beside it |
| JRE | `.compare-bins/2026-11-07-jre/OpenJDK21U-jre_x64_linux_hotspot_21.0.13_9.tar.gz` | `a535a1df…6114f47351dd` | equals `SHA256SUMS` |
| alkemio | `.compare-bins/2026-11-07-alkemio-mailslurper/root/mailslurper` | `1c7556f3…f46db6379` | equals `SHA256SUMS`; that file's last line is the dispatcher's record `image ghcr.io/alkem-io/test-suites/mailslurper@sha256:55029f542c19d0fe570450dc5a30f54dbdb24d9113a6e0e1bf96da000ebd0838` |

The facts below decide each row:

- **Refs** (`.compare-bins/2026-11-07-meta/*-ref.json`):
  - `mailslurper` `refs/tags/release-1.15.0` names commit `068acffd…` with
    `object.type` `commit`, so it is lightweight.
  - `smtp4dev` `refs/tags/3.16.1` names commit `9fac83c1…`, `commit`,
    lightweight.
  - `inbucket` `refs/tags/v3.2.0` names commit `74224969…`, `commit`,
    lightweight.
  - `greenmail`'s ref is **`refs/tags/release-2.1.15`**, with `object.type`
    **`tag`** (tag object `5f09fccc…`), so it is **annotated**. The pending
    entry's `release_tag: "2.1.15"` came from the draft and is wrong.
  - None of the five rows has a captured `git_tree`. No `git_tag` document
    exists for GreenMail.
- **Licenses.**
  - The MailSlurper zip has `LICENSE`.
  - The Inbucket tarball has `inbucket_3.2.0_linux_amd64/LICENSE`.
  - The GreenMail jar has `greenmail-openapi.yml`,
    `META-INF/NOTICE.markdown` and
    `META-INF/maven/com.icegreen/greenmail/pom.xml`.
  - **The smtp4dev zip has no license document**: 49 members, none named
    LICENSE, NOTICE or COPYING, and the release metadata names none.
- **Defaults.** The smtp4dev zip's `appsettings.json` carries
  `"NumberOfMessagesToKeep": 100`, `"Port": 25`, `"ImapPort": 143`,
  `"Pop3Port": 110` and `"Urls": "http://localhost:5000"`.
- **GreenMail listeners.** The standalone jar's classes name the properties
  `greenmail.api.hostname` and `greenmail.api.port` beside the
  `greenmail.setup.*` properties. That is evidence of an API listener the
  pending entry does not declare.
- **Alkemio.**
  - `root/mailslurper` is an ELF executable with interpreter
    `/lib/ld-musl-x86_64.so.1` and `NEEDED libc.musl-x86_64.so.1`. Neither the
    supplied directory nor this host has that loader, so the executable cannot
    run natively from the supplied bytes.
  - `root/version.json` reads `"version": "1.14.1"`.
  - No index, manifest or layer bytes were supplied. The index digest exists
    only as the dispatcher's recorded line, and it equals the full digest in
    the brief.

## Rulings (the plan's table gives each with what the losing side costs)

- **Re-derive, not staple.** Every launch fact of the five rows comes from a
  captured document, from the pinned executable's own `--help` or version
  output, or from the real launch's observed behaviour. The executable wins
  over a document. Each correction against the draft is listed in a dated
  2026-11-07 pre-collection addendum to proposed 0130.
- **No new vocabulary.** No new type, `DocKind`, pin kind or runtime, and no
  relaxed validator. A gap stays `Fact::Unresolved` with its reason, and
  `collection_ready` keeps refusing it.
- **Pin:** `mailslurper`, `inbucket`, and `greenmail` together with its JRE
  runtime.
- **Stay pending, stop and report:** `smtp4dev` (no license document) and
  `alkemio-mailslurper` (no musl loader, and no index or manifest bytes).
- **Protocol v3 stays `draft`.** Its `unresolved` list names exactly the
  byte-grounded gaps. Freezing now would spend the `fair-mail-comparison-v3` id
  with known gaps. "Frozen before any collection" holds because
  `collection_ready` refuses a draft.
- **Measured method unchanged.** The workloads and corpus stay equal to v2, as
  the parent spec's AC8 requires. Fidelity on hostile bodies is proved against
  the real pinned binaries in unmeasured tests.

## Acceptance Criteria

Each criterion can go red. "Real" means an `#[ignore]`d test in
`tests/mail_compare/eight_targets.rs`. It runs the pinned bytes, fails (and
never skips) without them, and runs under the bounded form of P11.

- **P1 Byte receipt and handling.**
  - Before any read, each artifact in the table above is digest-checked in
    place against its listed corroboration. A mismatch refuses that row, with
    no fallback to other bytes on disk.
  - Each archive is extracted into its own new empty directory under
    `target/compare-dl/` (on `/home`, never `/tmp`). The scripts written for
    this work live in a different directory (`target/compare-tools/`). Python
    over these bytes runs only as `python3 -I script.py <path>`.
  - The delivery notes quote a receipt of path, expected digest, observed
    digest and verdict for each artifact.
  - `git ls-files .compare-bins target` is empty at every commit.
- **P2 Three pins, no relaxation.**
  - `docs/evals/sources.lock.json` holds full v4 pins (`pin`, no `pending`)
    for `mailslurper`, `inbucket` and `greenmail`. Each artifact `sha256`,
    `executable_sha256` and runtime digest equals the P1 receipt.
  - Each pin carries its captured ref as a `git_ref` document (whole bytes,
    attributed `Api` to the `url` inside them) and its captured releases
    listing as `release_metadata` (whole bytes; the request URL is
    unrecorded, so the attribution is `Unresolved` with that reason).
  - Each new row's `tagged_source` is `Unresolved`, because no `git_tree` was
    captured, and the existing lightweight and annotated chains both need one.
    The reason states what the captured ref shows: the ref kind it implies
    (`object.type` `commit` means lightweight, `tag` means annotated) and the
    object it names. For GreenMail it also names the missing `git_tag`, and
    for MailSlurper the source tarball's digest as locally computed
    corroboration. No `ref_kind`, tag object or commit is written as observed
    without the chain that proves it.
  - `greenmail`'s release tag is `release-2.1.15`, the captured ref.
  - Every file document of a new pin is a member of a digested archive. Its
    attribution is `Unresolved("member <path> of <archive> sha256 <digest>")`,
    because the commit is unresolved and the validator refuses a `Commit`
    attribution then.
  - The tests `current_lock_v4_validates_and_verifies` and
    `mailpit_mailhog_v3_documents_byte_unchanged` pass.
  - `git diff 4012f31 -- benches/mail_compare/protocol.rs` removes no refusal
    and loosens no condition, and `git diff 4012f31 -- tests/` deletes no test
    function.
- **P3 Launch facts from bytes.**
  - Every argv entry, `launch_env` pair, `config_file` line, listener role
    and cap in the five rows has a citation: a document in the pin, or a
    captured `--help`/version output stored as a pin document.
  - No line of a pending entry's draft survives unless the bytes confirm it.
  - 0130 gains a dated `2026-11-07 (rerun, pre-collection)` addendum. For each
    row it lists every fact that changed from draft to captured (the old
    value, the new value and its citation), every draft fact the bytes
    confirmed, and the rows left pending with their reasons.
  - The addendum adds lines only to 0130 (`git diff 4012f31 -- docs/decisions/0130*`).
- **P4 GreenMail runs as captured.**
  - Its pin declares `smtp`, `pop3` and `http`. The `http` role is the API
    listener on a reserved loopback port, set through the properties found in
    the jar. It is neither the read path nor the readiness probe.
  - The real launch's exact-listening-set check passes, and cleanup proves all
    three listeners closed.
  - Before sending anything, the real GreenMail test logs in over POP3 as the
    workload recipient and reads `STAT` `+OK 0`.
  - The JRE runs from its extracted, member-checked root (`bin/java`,
    `lib/server/libjvm.so`). Its license is checked against the 0130 §6
    runtime list from the captured `legal/` text. If that text does not
    support the list's expression, S5 applies.
- **P5 smtp4dev stays pending, honestly.**
  - The entry stays `pending`. Its reason names the missing license document
    (49 members, none a license), the artifact digest, the default retention
    of 100 from `appsettings.json`, and the commit `9fac83c1…`. Its listeners
    and launch are corrected from the pinned executable's captured `--help`.
    That output is quoted in 0130's addendum, because a pending entry carries
    no documents.
  - The executable is never started as a server for evidence.
  - `real_smtp4dev_smtp_to_native_api_round_trip` fails, naming the reason.
- **P6 Alkemio stays pending and unmerged.**
  - The entry stays `pending`, and its reason names: the musl interpreter and
    `NEEDED` library, `version.json` 1.14.1, the executable digest
    `1c7556f3…`, the full index digest as a *dispatcher-recorded line* (never
    verified and never completed by guess), and the missing index, manifest
    and layer bytes.
  - No musl loader is borrowed, pinned or downloaded, the binary is not
    patched with `patchelf`, and Docker is not used.
  - `alkemio_name_is_not_mailslurper` still passes.
- **P7 Protocol v3 draft.**
  - `docs/evals/protocol.json` stays `status: draft`. Its `unresolved` list
    names each remaining gap: the two pending rows, the `tagged_source` gaps
    of the three new pins, each `best_wait`, and the runtime-list ruling.
  - `mail_compare check --protocol docs/evals/protocol.json --sources
    docs/evals/sources.lock.json` exits non-zero and lists each of these
    gaps.
  - Every `capabilities` value of the three pinned rows is either
    `supported`/`not_supported` with a citation to a document in that pin, or
    `not_inspected`. MailSlurper's `native_wait` is `not_supported` only if a
    retained reviewed source from the source tarball shows the full route set.
    Otherwise it is `not_inspected` (S1).
  - The storage row's cap equals the pin's launch value.
- **P8 Real round trips.** `real_mailslurper_smtp_to_native_api_round_trip`,
  `real_inbucket_smtp_to_native_api_round_trip` and
  `real_greenmail_pop3_under_pinned_jre` pass. The `MAIL_COMPARE_*` variables
  name the extracted executables; the launch's own digest check against the
  pin (`targets.rs`, `Owned::launch`) is the identity proof.
- **P9 Caps at the workload, per row.**
  - `real_caps_hold_at_workload_count` is split into
    `real_caps_hold_<target>`, one per new target. Each sends 600 and lists
    600.
  - The three pinned rows pass. The `smtp4dev` and `alkemio-mailslurper` tests
    fail, naming their pending reasons.
  - No count is lowered, and every cap comes from P3's citations.
- **P10 Hostile bodies on the real binaries.**
  - `real_<target>_hostile_round_trip` for each pinned row sends the
    deterministic hostile set that `hostile_bodies_round_trip` already uses
    (dot-leading lines, a lone `.` line in a stuffed body, an 8-bit subject, a
    998-octet line, the boundary string in the body) and reads every message
    back through the native read path.
  - Each test asserts listing, fetch and decode with no driver error, and
    asserts that the subject, text and HTML equal what was sent.
  - The measured corpus does not change.
- **P11 Bounded load.**
  - Every real-binary test, `--help` capture and smoke is a named command of
    the form `timeout 600 taskset -c 0-$(( $(nproc)/4 - 1 ))` (taking `nproc`
    at run time; 128 at design gives 32 CPUs). `/proc/loadavg` is recorded
    before and after.
  - Gate commands run under the same `taskset` and `CARGO_BUILD_JOBS`. They
    are not measurements, so the 600 s timeout does not apply to them.
  - The notes report each command with its wall time, exit status and
    loadavg.
- **P12 Smoke fixture and docs.**
  - `eight_target_smoke_fixture_is_a_valid_draft_that_cannot_launch_yet`
    asserts exactly `smtp4dev` and `alkemio-mailslurper` pending, and the
    other rows pinned.
  - The v3 section of `docs/evals/README.md` names the three pinned rows'
    labels: lightweight or annotated-unresolved source, POP3 read path with
    per-scan login, UIDL and TOP per message, client-side find, unmatched
    storage, and unenforced local profile. It names the two open rows with
    their asks and makes no comparison claim.
- **P13 Nothing paid, nothing added.**
  - No agent session starts, and `controls`, `evidence` and `measure` never
    run with real inputs.
  - `git diff c95386e -- Cargo.toml Cargo.lock evals/Cargo.lock deny.toml src/`
    is empty.
  - Commits are unsigned. Nothing is pushed, merged, published, tagged or
    accepted.
- **P14 Gates.** The full house inventory passes `--locked`: root and evals
  fmt and clippy; `cargo test --workspace`; the candidate binary; the evals
  tests with `SWARMAIL_BIN` from Cargo's artifact record; the coverage gate at
  100% and its selftest; the publish-guard and seal proofs; and
  `cargo deny check licenses`. If no `brokkr` is on PATH, the notes say that
  the bundle compile is verify's.
- **P15 Honest report.**
  - The result and notes say #340 is **not complete**: two rows are blocked.
  - The notes give the operator's asks verbatim (plan.md, *Operator asks*).
  - The notes say the PR is held for the operator, because it changes
    `scripts/` and amends accepted 0091 §4 and 0093 §1 through 0130.

## Stop triggers (stop that row and report, never decide)

The parent spec's S1–S6 hold. This run sharpens S6 and adds S7 and S8:

- **S6′** A fact the row needs is not in the supplied bytes. Examples are a
  license document, a loader, index or manifest bytes, or a property's
  effect. The row stays `pending` with the gap named, and the other rows
  proceed.
- **S7** A real round trip or hostile test of a pinned row fails, and the
  authored-fixture tests show the driver correct for those bytes, so the tool
  itself stored or served different bytes. Report the row with the sent and
  returned bytes. Do not weaken the assertion or change the measured corpus.
- **S8** A pinned row's real launch shows an undeclared listener that no
  captured property can move. That is S4 for that row: it returns to
  `pending`.

## Out of scope

- Any collection, and freezing protocol v3.
- Paid or agent work.
- Changes under `src/`.
- New vocabulary or relaxed validators.
- A musl runtime, Docker, or `patchelf`.
- Re-capturing Mailpit or MailHog.
- Network fetches.
- IMAP, stream clients, or MCP adapters.
- Accepting decisions.
- Closing #163 or #340.
