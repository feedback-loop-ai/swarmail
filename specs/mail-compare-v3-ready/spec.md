# Mail comparison protocol v3, ready to collect from the supplied bytes

Date: 2026-11-07. Status: draft. Issue #340, part of #163 (Epic #164).
Run: `second-rerun-of-340-operator-dir-d2bccbd0` (operator direction
2026-11-07, "Pragmatic set"). Branch `repair/issue340-compare-benches`,
continuing from `2c28b8b`. Base: `main` at `c95386e`. Recipe:
`claude-careful`.

Revised 2026-11-07 on review's return (decision 0102, addendum 2026-10-31).
The review of `c5a784c` found the design itself defective. V8 required `check`
to pass with the Alkemio image pin and the source-build row declared, while V10
allowed only five widened arms. Both cannot hold. The revision:
- re-derives V10 as an enumerated table (a)–(h);
- states V8's scope;
- adds V15–V17 for the review's ordinary findings.

[plan.md § Review return](plan.md#review-return-2026-11-07) maps each finding
to the change that answers it. The frozen `protocol.json` and
`sources.lock.json` bytes do not change.

Decision: 0130 — [Eight comparison targets: source lock v4 and protocol v3](../../docs/decisions/0130-eight-target-mail-comparison.md), proposed; only the operator accepts.

Earlier designs, both still normative except where this spec overrides them by
name:
- [`specs/mail-compare-eight-targets/`](../mail-compare-eight-targets/spec.md)
  (`98fdd25`): R1–R9, AC1–AC19 and S1–S6.
- [`specs/mail-compare-eight-targets-pins/`](../mail-compare-eight-targets-pins/spec.md)
  (`dfc3d3c`): P1–P15 and S6′–S8.

The operator's rulings of 2026-11-07 override these earlier items:
- the pins spec's "no new vocabulary" ruling;
- P5, P6 and P7 (smtp4dev and Alkemio pending, protocol v3 kept a draft);
- the P12 assertion that two rows stay pending;
- the out-of-scope entries "freezing protocol v3" and "a musl runtime".

Everything else in those specs stands.

Authorities (accepted):
- [0091](../../docs/decisions/0091-fair-mail-comparison.md) §1, §3 and §4;
- [0092](../../docs/decisions/0092-official-mail-comparison.md) §1–§4 and §6;
- [0093](../../docs/decisions/0093-source-mail-comparison.md) §1–§4;
- [0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md),
  [0056](../../docs/decisions/0056-signing-policy.md) and 0045 §8.

## What

The previous run stopped on six operator asks. On 2026-11-07 the operator
answered all of them with four rulings, quoted from the brief:

1. **MailSlurper.** "Publish as a result that the official release-1.15.0
   linux executable does not start with its default SQLite storage." Also
   "bench a cgo build from the pinned release source, as a separate labelled
   row 'built from source'. It may be pinned even though it reports v1.14.1
   under tag release-1.15.0; record that fact."
2. **Alkemio.** "A musl loader may be pinned as a second runtime, taken from
   the image's own layer."
3. **GreenMail** "is bound to its Maven Central artifact and checksums. Lock v4
   may bind an official artifact that the GitHub release does not list, when
   its registry publishes checksums."
4. "**0130 §6's runtime licence list** is accepted: JRE GPLv2 with the
   Classpath exception, musl MIT."

The operator also supplied bytes under `.compare-bins/2026-11-07-supply/`
(git-ignored, untrusted, never committed):
- the smtp4dev license at commit `9fac83c1…`;
- the ref, commit and root tree (plus GreenMail's tag object) for inbucket,
  smtp4dev, greenmail and mailslurper;
- the Alkemio OCI index, manifest, config, 15 layers and musl loader;
- the GreenMail Maven files;
- the MailSlurper cgo build, with its build record and the official binary's
  failure output.

This delivery does four things:
1. It encodes the four rulings, each as the narrowest closed change in
   `benches/mail_compare/`.
2. It pins every competitor from those bytes, so no lock v4 entry stays
   `pending` and every `tagged_source` gap becomes an observation.
3. It freezes protocol v3 so that `mail_compare check` admits it.
4. It runs the real readiness proofs, bounded and unmeasured.

It collects nothing.

## Why

"Ready to collect" has one mechanical meaning in this driver: `mail_compare
check` (that is, `collection_ready` together with `SourceLock::verify_files`)
returns `Ok` on the committed protocol and lock. Each refusal it raises today
has to be closed in one of three ways:
- by an observation from the supplied bytes;
- by one of the operator's rulings;
- by a condition that a ruling or the issue's own direction entails, named in
  V10's table with its derivation and the refusal it keeps.

It is never closed by relaxing a check that neither names. A condition that
only the implementation discloses is not a derivation.

Under 0091 §1 a protocol is frozen before collection, and the frozen bytes
must stay what they were. So the freeze is identified by content hash, and the
proofs then run against those exact bytes.

The two MailSlurper subjects are kept apart:
- The official release binary is a `CGO_ENABLED=0` stub that exits at
  `CREATE TABLE mailitem`.
- The cgo build is a different executable.

Merging them would relabel an identity, which 0092 §2 forbids. Dropping the
official row would discard unstarted work, which 0091 §1 and §3 forbid. So the
official row stays a full product entry, and its published result is that it
does not start.

## The design, ruling by ruling

The plan's synthesis table says which panel position won each point and what
the losing side costs. Every digest prefix below was seen in the design seat;
the implementer re-derives each full value from the bytes (V1) and never copies
it from here.

### Ruling 1: two MailSlurper rows

- **`mailslurper` stays the official release artifact**: the zip `48f2f9ba…`
  (`locally_computed`), its executable member `0cd9416b…`, the lightweight tag
  chain. Protocol v3 gains one closed field, `does_not_start: [{target,
  cite}]`; `cite` is a document of that target's pin (the captured failure
  output). The runner never launches a listed target; the report prints its row
  as "does not start (published result: <cite>)" with every cell blocked and no
  number, and the row keeps its full-suite denominator (0091 §3, 0092 §3).
  `declared_competitors` (the launch set the wrapper and runner derive) omits
  it; the product, capabilities, rotation slot and pin stay.
- **A ninth target, `mailslurper-source-build`**, labelled "MailSlurper
  release-1.15.0, built from source (cgo; host glibc, unpinned)". New
  `Provenance::SourceBuild(home)` and `ProductScope::SourceBuild`. It reuses
  MailSlurper's launch template, config file, listeners and decoder; its
  documents live under `sources/mailslurper-source-build/` (byte copies of the
  shared ones). Its artifact is the executable `eb6cd97d…`, `locally_computed`,
  `download` = `{home}/tree/{tagged commit}` (built from the validated commit,
  as `Commit` attributions build theirs). Its `binary_build_provenance` must be
  **Observed**: commit = the tagged commit, evidence = a `BuildEvidence`
  document (the supply's `mailslurper-cgo/PROVENANCE.md`, attributed
  `Supplied`).
- **The version fact is a closed constant, not a lock field.**
  `REPORTED_VERSIONS = [(mailslurper, "release-1.15.0", "1.14.1"),
  (mailslurper-source-build, "release-1.15.0", "1.14.1")]`. The version check
  accepts output naming the tag's version, or exactly the listed version for
  exactly that target and tag. The report's version cell reads "1.14.1 reported
  / tag release-1.15.0", derived from the constant.

### Ruling 2: the musl loader, a second runtime

`validate_runtime` becomes a closed per-target table `RUNTIMES` of
`(target, entry program, members that must be checked)`:
`(greenmail, "bin/java", ["bin/java", "lib/server/libjvm.so"])` and
`(alkemio-mailslurper, "lib/ld-musl-x86_64.so.1",
["lib/ld-musl-x86_64.so.1"])`. Any other target's runtime is refused. The
launcher (`targets.rs`) takes the entry program from the table instead of
hard-coding `bin/java`, execs the pinned loader explicitly with the extracted
binary as `{artifact}`, never falls back to a host loader, never sets
`LD_LIBRARY_PATH`, and digests the loader at launch and seal like any checked
member. The Alkemio runtime's archive digest is layer `sha256:f350948e…`,
which the captured manifest `97dfa556…` must list (validator check: a loader
runtime's `sha256` is a layer of the pin's captured manifest) — that is "taken
from the image's own layer". Its checked member is the loader `0273b534…`; its
licence is `MIT`, holder from the layer's apk record. The Alkemio row runs on a
driver-rendered plain config on reserved loopback ports, never the image's TLS
`config.json`.

### Ruling 3: GreenMail through Maven Central

A closed `DigestBasis::RegistryChecksum` with a closed constant `REGISTRY =
[(greenmail, "https://repo1.maven.org/maven2/com/icegreen/greenmail-standalone/")]`.
Under it: `download` = `{registry}{version_of(tag)}/{file_name}`;
`upstream_digest` = `sha256`; `release_agrees` skips only the asset-list check
(tag, page, commitish and date still apply); and a new `DocKind::Checksum`
document holds the registry's `.sha256`, whose trimmed bytes `verify_files`
requires to equal `upstream_digest`. Its attribution is `Supplied`, and the
record must name `{download}.sha256`. Nothing authenticates the publisher: the
`.asc` stays in the supply, no key is pinned, and the claim and README say
"integrity against the registry's own checksum; authenticity not verified".
GreenMail's chain is annotated: ref → tag `5f09fccc…` → commit `3b8ea7df…`,
`tag_signature` = the captured `verification.reason` (no gpg is run).

### Ruling 4: runtime licences

`RUNTIME_LICENSES = ["GPL-2.0-only WITH Classpath-exception-2.0", "MIT"]`.
`PERMISSIVE`, `deny.toml` and the dependency allowlist are unchanged.

### Forced by "ready to collect" (each closes a named refusal)

- **`DocKind::GitCommit` and a tree rebuild.** The supplied trees are root
  trees fetched by tree sha, not by commit. `tag_chain` accepts a tree by the
  existing route (`/sha` is the commit; Mailpit and MailHog, unchanged) or by a
  `GitCommit` document whose `/sha` is the tagged commit and whose `/tree/sha`
  is the tree's `/sha`. On that route the validator **rebuilds git's tree
  object** from the listed entries (mode, path, sha, in listed order; `040000`
  serialized `40000`) with the crate's existing SHA-1 and requires it to equal
  `/sha`, so an edited listing cannot vouch for a blob.
- **Three closed attributions.** `Member { archive_sha256, member }` — bytes
  of a member of a digested archive, which must be the pin's artifact, its
  runtime archive, or a closed `SOURCE_ARCHIVES` entry (the MailSlurper source
  tarball `83f66f25…`, for exactly the two MailSlurper rows, bound to the
  tagged commit). `Output { argv }` — the pinned executable's own output for
  that argv. `Supplied { record }` — operator-supplied evidence; the record
  names the supply path and contains the document's sha256. `Member` and
  `Output` are proved by a real test that re-extracts or re-runs: the crate has
  no archive reader and none may be added.
- **Release metadata slices** are attributed `Api { url }` to the object's own
  `url` (the variant's documented meaning; `verify_files` checks the
  self-name). 0130 records that the bytes are a slice of a listing.
- **An image pin has no tagged source to observe.** This condition is entailed,
  not ruled.
  - The issue orders source lock v4 to accept "image digests". It orders
    Alkemio's image "reported as a separate labelled row: extract its binary,
    digest it and run it natively". Ruling 2 pins a runtime for that row, which
    presupposes that the row runs.
  - 0130 §2 says an image has no tag, and `validate_image` refuses an observed
    `tagged_source` on an image pin. So if `collection_ready` required an
    observed one, the ordered row could never collect.
  - The exemption is keyed on the target's closed provenance: a
    `Provenance::has_tagged_source()` predicate that is false only for
    `ThirdPartyImage`. It is not keyed on whether the lock data carries an
    `image` field, so a future pin kind cannot inherit it by accident.
  - The image's identity stays its fully checked index → manifest → layer
    digest chain.
- **A source build is not a release asset.** This condition is entailed by
  ruling 1 ("bench a cgo build from the pinned release source … It may be
  pinned"), because a supplied build cannot be listed by the GitHub release.
  - For `Provenance::SourceBuild` only, `release_agrees` skips the asset-list
    check. Tag, page, commitish and date are still checked.
  - The release-asset download check is replaced by a stronger binding:
    `{home}/tree/{tagged commit}`, plus Observed build provenance naming that
    commit.
- **`ConfigFile.cwd`** is a closed launch field (default false), observed in
  the exploratory pass: the Alkemio fork reads `./config.json` and has no flag
  for it. It loosens no refusal. The config `name` stays one leaf, with `/` and
  `..` refused.

### The freeze

Order: pin → run the real proofs as untimed exploratory probes (0092 §1) →
freeze (`status: frozen`, `unresolved: []`) and record both files' sha256 in
0130 and in a test constant → rerun the real proofs against the frozen bytes;
those reruns are the readiness record. After the freeze, the protocol and lock
bytes never change (S12).

## Acceptance Criteria

Each criterion can go red. "Real" means an `#[ignore]`d test in
`tests/mail_compare/eight_targets.rs` that runs the pinned bytes, fails (never
skips) when they are absent, and runs under V11's bounded form.

- **V1 Supply receipt and handling.**
  - `sha256sum -c SHA256SUMS` passes in each `.compare-bins/2026-11-07-supply/`
    subdirectory and at its top level before anything reads a supplied file.
    The supply is never written to or extracted into.
  - Each extraction goes into its own new directory under
    `target/compare-dl/`; scripts live in `target/compare-tools/`; Python runs
    only as `python3 -I`.
  - The Alkemio loader's layer `f350948e…` is the topmost manifest layer that
    holds `lib/ld-musl-x86_64.so.1` (the loader the image itself resolves);
    the notes show the layer listing that proves it.
  - `git ls-files .compare-bins target` is empty at every commit. The notes
    quote the receipt for every file pinned or copied.
- **V2 Every competitor pinned.** `docs/evals/sources.lock.json` (lock 4) has
  a `pin` and no `pending` for `mailpit`, `mailhog`, `mailslurper`,
  `mailslurper-source-build`, `smtp4dev`, `inbucket`, `greenmail` and
  `alkemio-mailslurper`; every digest equals V1's receipt.
  `current_lock_v4_validates_and_verifies` and
  `mailpit_mailhog_v3_documents_byte_unchanged` pass.
- **V3 Tagged sources observed.**
  - `tagged_source` is `Observed` for `mailslurper`,
    `mailslurper-source-build`, `smtp4dev` and `inbucket` (lightweight) and
    `greenmail` (annotated, with tag object and `tag_signature`). Each carries
    its ref, commit and root tree documents (GreenMail also its tag).
  - Tests refuse: a tree listing with one entry's sha or path edited (rebuilt
    hash differs); a `GitCommit` naming another tree; a `GitCommit` whose
    `/sha` is not the tagged commit; a `Commit`-attributed document whose blob
    is not in the tree. A test proves the rebuild against an authored tree
    whose id was computed by `git mktree` (fixture committed as bytes).
  - smtp4dev's `LICENSE.md` is `Commit`-attributed (blob `ac0f7a32…`),
    licence `BSD-3-Clause`.
- **V4 The two MailSlurper rows.**
  - `TargetKind::ALL` has nine entries; `parse` round-trips
    `mailslurper-source-build`; `alkemio_name_is_not_mailslurper` still passes
    and a new test proves `mailslurper-source-build` is neither `mailslurper`
    nor Swarmail's shim (decision 0118).
  - Tests refuse: `SourceBuild` with unresolved build provenance; build
    provenance naming another commit or citing no `BuildEvidence`; a
    source-build `download` other than `{home}/tree/{commit}`;
    `REPORTED_VERSIONS` used for another target or tag; output naming neither
    version; `does_not_start` naming an undeclared target, a target without a
    pin, or a cite that is not a document of that pin; `does_not_start` on a
    v2 protocol.
  - `plan()` yields no coordinate for a `does_not_start` target; the report
    row for it carries the label and no number; `declared_competitors` omits
    it.
  - `does_not_start_matches` also refuses a cite whose attribution is not
    `Supplied` or `Output`. The cite must be failure evidence: operator-supplied
    output, or the pinned executable's own output. A README or licence must
    never suppress a working row. The committed cite is `Supplied`, so the
    frozen bytes still pass (S14 if not). The test is
    `does_not_start_cite_must_be_failure_evidence`, run over a copy of the
    committed lock whose cite names the row's `Commit`-attributed README.
- **V5 Alkemio on its own loader.** The image pin chains index `55029f54…` →
  manifest `97dfa556…` → the executable's layer, executable `1c7556f3…`; the
  runtime pins loader `0273b534…` from layer `f350948e…`, licence `MIT`. Tests
  refuse: a runtime on a third target; a loader runtime missing its member; a
  loader layer the captured manifest does not list; a runtime licence off
  `RUNTIME_LICENSES`. A test asserts `PERMISSIVE`'s exact five entries, so
  `MIT` on the runtime list widens nothing else. A launch-spec test proves the
  exec'd program is the runtime root's loader and `{artifact}` the binary.
- **V6 GreenMail from the registry.** The pin is `registry_checksum`, jar
  `91d58798…`, with its `.sha256` as a `Checksum` document. Tests refuse: a
  download outside `REGISTRY`; the registry basis on another target; a
  checksum document disagreeing with `upstream_digest`; a registry pin
  without a checksum document; a `Supplied` checksum record not naming
  `{download}.sha256`. An `upstream_release_metadata` pin whose asset the
  release does not list is still refused (unchanged).
- **V7 Attributions.** Every document of every pin has an `Observed`
  attribution. Tests refuse: `Member` with an escaping path, a malformed
  digest, or an archive outside the allowed set; `Output` with an empty argv;
  `Supplied` with an empty record or one lacking the document's sha256.
  `real_member_and_output_documents_match` re-extracts every `Member` archive
  with the system `unzip`/`tar` into a fresh directory, re-runs every `Output`
  argv bounded, requires every committed document's sha256 to match, and
  reads the source tarball's pax `comment` commit id and requires it to equal
  the tagged commit.
- **V8 Protocol v3 frozen and ready.**
  - Scope: the nine targets are the candidate checkout (`swarmail`, unpinned)
    plus the eight locked competitors V2 names. `check`'s readiness is over
    those eight pins. Seven of them are source pins, so V3's tagged-source
    requirement applies to those seven. The eighth, Alkemio, is the image pin
    under V10 (f). It must still refuse an observed `tagged_source`
    (`validate_image`, unchanged).
  - `docs/evals/protocol.json`: `protocol_version` 3, `id
    fair-mail-comparison-v3`, `status: frozen`, `unresolved: []`; nine
    targets, nine cyclic rotations, nine capability and product records, the
    one `does_not_start` entry. Every row states storage, cap (or
    `no_cap_observed`) and wait method; every capability is
    `supported`/`not_supported` with a citation or `not_inspected`. The
    workloads and corpus stay equal to v2's.
  - The claim stays `official_artifact`; its statement adds the source-build
    row with its host-glibc label, the loader row and the registry-integrity
    label.
  - `cargo bench --locked --bench mail_compare -- check --protocol
    docs/evals/protocol.json --sources docs/evals/sources.lock.json` exits 0
    with "ready for collection"; a test runs the same `collection_ready` and
    `verify_files` over the committed files and requires `Ok`.
  - `frozen_v3_and_lock_v4_bytes_are_the_recorded_ones` asserts both files'
    sha256 equal constants that 0130's addendum also records. The history
    bytes under `docs/evals/history/` keep their sha256.
- **V9 Real readiness proofs, against the frozen bytes, unmeasured.** No file
  under `docs/evals/` is written by them and no v3 run directory or manifest
  exists afterwards.
  - For each launched new competitor (`smtp4dev`, `inbucket`, `greenmail`,
    `mailslurper-source-build`, `alkemio-mailslurper`):
    `real_<t>_smtp_to_native_api_round_trip` (POP3 for GreenMail),
    `real_<t>_hostile_round_trip` and `real_caps_hold_<t>` (600 sent, 600
    listed, no count lowered) pass, each with the exact-listening-set and
    cleanup proofs.
  - `real_mailslurper_official_does_not_start`: the pinned official
    executable under its pin's launch exits non-zero within the startup
    deadline, its output contains `Binary was compiled with 'CGO_ENABLED=0'`,
    no declared listener is bound, and cleanup is proved.
  - The labelled unpinned GreenMail driver proofs become the pinned ones; any
    test function removed has a one-for-one successor named in the notes.
  - **Receipts at the final head.** `c5a784c` changed the driver after the
    recorded proofs, and this revision changes it again. So every V9 test and
    `real_member_and_output_documents_match` reruns at the revision's final
    commit, against the unchanged frozen bytes. Each receipt row names
    `git rev-parse HEAD` and `HEAD^{tree}` beside its command, wall time, exit
    status and loadavg.
- **V10 No refusal loosened beyond the enumerated set.** `git diff 2c28b8b --
  benches/mail_compare/` removes no refusal. It widens no condition except the
  rows of this table. Each row names its authority and the refusal it must
  keep, and each test listed must exist and pass.

  | | Condition | Authority | Refusal kept (test) |
  |---|---|---|---|
  | (a) | `REPORTED_VERSIONS` | ruling 1 | another target or tag; output naming neither version (V4's tests) |
  | (b) | `RUNTIMES` table, loader in the image's manifest | rulings 2, 4 | a third target; a missing member; a layer absent from the manifest (V5's tests) |
  | (c) | `RegistryChecksum` asset-list skip | ruling 3 | the basis on another target; tag, page, commitish and date still checked (`skipped_asset_list_still_checks_tag_commitish_and_date`); checksum record naming `{download}.sha256` (V6's tests) |
  | (d) | `MIT` on `RUNTIME_LICENSES` | ruling 4 | `PERMISSIVE` exactly five (`permissive_list_is_exactly_the_five`) |
  | (e) | `GitCommit` tree route with rebuild | forced by V3 | edited entry, another tree, another commit, blob absent (V3's tests) |
  | (f) | an image pin's `tagged_source` gap not counted by `collection_ready` | entailed: the issue's image-digest lock and Alkemio row; 0130 §2 | every source pin still refuses an unresolved gap, and an image pin still refuses an observed one (`official_pin_still_needs_an_observed_tagged_source_but_an_image_pin_does_not`; `image_pin_refuses_observed_tagged_source_or_build_provenance`) |
  | (g) | `SourceBuild` skips the asset list; release download replaced by `{home}/tree/{commit}` | entailed by ruling 1 | V4's four `source_build_refuses_*` tests; `upstream_release_pin_with_unlisted_asset_is_still_refused` (the skip never reaches an `Official` pin); new `skipped_asset_list_still_checks_tag_commitish_and_date` (a registry pin and the source build, each with one edited field, both refused) |
  | (h) | `ConfigFile.cwd` | observed launch fact (0130 addendum) | a config name with `/` or `..` refused (`config_file_name_confined`); `config_file_cwd_runs_where_rendered` |

  Rows (f) and (g) keep their code but take the shapes named in "Forced by
  ready to collect": the provenance predicate, and the `Provenance::SourceBuild`
  arm. Where the panel found a test missing, the test is added under the name
  above, or the notes name its existing equivalent. `git diff 2c28b8b --
  tests/` deletes no test function outside V9's successor rule and V17's
  rename.
- **V11 Bounded load.** Every real proof, `--help`/version capture and smoke
  is a named `timeout 600 taskset -c 0-$(( $(nproc)/4 - 1 ))` command
  (`nproc` read at run time; 32 CPUs here), one test at a time
  (`--test-threads=1`), with `/proc/loadavg` recorded before and after. The
  notes report each command, its wall time, exit status and loadavg. Gate
  commands run under the same `taskset`; they are not measurements.
- **V12 The record.** (The revision adds a second addendum; see V12′
  below.) 0130 gains one dated addendum, "2026-11-07 (operator
  rulings, pins and freeze)", additions only (`git diff 2c28b8b --
  docs/decisions/0130*` adds lines only): the four rulings quoted; the supply
  receipts; the new constants and variants; the `does_not_start` result and
  cause (the stub; the official binary's go1.20.6 `CGO_ENABLED=0` build info;
  `SERVER_VERSION = "1.14.1"`); the nine-target list; the host-glibc and
  registry-authenticity labels; the slice note; the frozen hashes. Status
  stays `proposed`; no accepted record's body changes. The v3 section of
  `docs/evals/README.md` says what is pinned and frozen, labels every row
  (source strength, read path, search method, storage, runtime,
  does-not-start), lists what stays open (final contracts, controls, cost,
  identity, full product, #148/#201) and makes no comparison claim.
- **V13 Hygiene and gates.** `git diff c95386e -- src/ Cargo.toml Cargo.lock
  evals/Cargo.lock deny.toml` is empty. No agent session; `compare-mail.sh`
  `collect`, `controls`, `evidence` and `measure` never run with real inputs.
  Every house gate passes `--locked` (root and evals fmt/clippy, `cargo test
  --workspace`, the candidate binary, the evals tests with `SWARMAIL_BIN` from
  Cargo's artifact record, coverage at 100% and its selftest, publish-guard
  and seal proofs, `cargo deny check licenses`); without `brokkr` on PATH the
  notes say the bundle compile is verify's. Commits unsigned; nothing pushed,
  merged, published, tagged or accepted.
- **V14 Honest report.** The notes say which criteria passed with their
  outputs, that collection was not run (not this story), and that the PR is
  held for the operator (the branch changes `scripts/` and adds pointers to
  accepted records).
- **V12′ The review-return record.** 0130 gains a second dated addendum,
  "2026-11-07 (review return)". It is additions only: `git diff c5a784c --
  docs/decisions/0130*` has no `-` line, and the status stays `proposed`. It
  states:
  - **Nine, not eight.** The closed list in §1, §2 and §3 now reads nine:
    ruling 1 added `mailslurper-source-build`. The nine are listed, and the
    file name stays "eight-target".
  - **Conditions (f), (g) and (h)** with their derivations and tests. The
    earlier "one condition beyond the five ruled arms (for the operator)" and
    "the operator's ruling on the sixth condition" are superseded by an added
    sentence, not by deletion. The operator can still strike (f) or (g) when
    ruling on 0130, and S12′ then applies.
  - **GreenMail REST.** The pinned OpenAPI's `mimeMessage` reads messages, so
    0130's Context, the frozen lock's `wait.finding` and the old `wire.rs`
    comment are wrong to say it cannot. POP3 stays the read path because the
    issue orders it (0093 §4). The frozen `wait.finding` stays as frozen, and
    this addendum is its correction of record.
  - **smtp4dev's runtime label.** It lives in the report (V15) and the README,
    not in the frozen claim. This corrects the earlier sentence "labelled in
    the claim, the report and the README".
  - **Alkemio's cap basis**, from V17's probe: either "and holds under a
    burst", or "in batches of five; a burst listed N".
  - **`capabilities.server_search`** describes the product's API. The row
    label's `search` describes the driver's chosen method. Both stand.
  - **The musl `holder`** names the Alpine package maintainer. musl's
    copyright holders are in musl's COPYRIGHT. The frozen lock is unchanged.
  - **The receipt table at the final head** (V9).

  Related edits outside 0130:
  - `docs/decisions/0091`'s §4 pointer gains one appended dated pointer line
    ("2026-11-07: nine targets; see 0130's review-return addendum"). Its
    existing line is not edited.
  - The 0130 row of `docs/decisions/README.md` adds "nine targets (2026-11-07
    addenda)" to its notes.
  - `wire.rs`'s comment says the driver reads over POP3 because the issue
    orders it.
  - The README's v3 section says nine and states the Alkemio cap basis as
    observed.
- **V15 Labels where the number is read, and under the seal.**
  - `report::row_labels` gives smtp4dev the runtime label
    `self-contained .NET on host libraries (unpinned)`.
    `report_labels_name_every_unpinned_runtime` asserts it, together with the
    source build's host-glibc label.
  - `admit_run` re-derives `blocked` and `labels`, as well as `throughput` and
    `waits`, from the protocol, but only for a protocol_version-3 report. v2
    admission is unchanged, so the 2026-10-30 collection keeps its terms.
  - `admit_run_refuses_a_v3_report_with_an_edited_blocked_row`: a sealed v3
    report whose blocked row was dropped, or whose blocked cell carries a
    number, is refused. An untampered one admits.
- **V16 One launch owner, and the wrapper's flags under test.**
  - `Protocol::launch_order(rep)` (the rotation minus `does_not_start`) is the
    only filter. `plan()`, `runner::run` and `prepare` all use it, and no
    other `does_not_start(` filter remains in `benches/` (a grep in the
    notes).
  - `plan_yields_no_coordinate_for_a_does_not_start_target` asserts the exact
    count: `(|targets| − 1) ×` the per-target coordinates.
  - `run_never_launches_a_does_not_start_target`: one `runner::run` over a
    small v3 smoke protocol. The candidate is the only launched target, and a
    competitor is listed in `does_not_start` with no launch spec. The test
    requires a `Clean` verdict and a sealed report carrying that row as
    blocked. Without the filter the verdict is `Infrastructure`.
  - `flags` and a pure `launch_specs(p, s, bins, runtimes)` move, with
    unchanged behaviour, from `benches/mail_compare.rs` into `runner.rs`.
    `prepare` keeps its I/O and calls them. Tests over the committed v3
    protocol and lock:
    - `flags_repeat_bin_and_runtime_only`: two `--runtime` values are
      admitted and `--protocol` twice is refused;
    - `launch_specs_take_two_runtimes`: greenmail and alkemio-mailslurper;
    - `launch_specs_refuse_a_bin_for_a_does_not_start_target`;
    - `launch_specs_refuse_a_runtime_given_twice_for_one_target`.
- **V17 The real tests assert what the record claims.**
  - For the decoded-body rows (`mailslurper-source-build`,
    `alkemio-mailslurper`), `real_hostile` and the multipart arm of
    `real_round_trip` assert the contract 0130 records: dot-stuffing is not
    undone (`..leading dot`), each newline becomes `<br />`, and the
    multipart HTML equals `<p>html part</p>`. They no longer `eprintln!` it.
  - `eight_target_smoke_fixture_is_a_valid_draft_that_cannot_launch_yet` is
    renamed one-for-one to a true name, because it asserts that the fixture
    launches.
  - `real_mailslurper_official_does_not_start` drops its
    `started.elapsed() < startup` wall-clock assertion, since the timeout
    already bounds it (0020 §5). The bind-and-release port reservation stays,
    because it fails closed.
  - **Alkemio burst probe.** Run `real_caps_hold_alkemio_mailslurper` once
    with Alkemio removed from `loses_under_burst`, bounded by V11.
    - If 600/600 are listed, Alkemio leaves `loses_under_burst` and the lock's
      basis is true as written.
    - If mail is lost, the batching stays and V12′ records the observed count.
    - Either way the notes quote the result.

## Stop triggers (stop that row or step and report; never decide)

The earlier specs' S1–S8 hold. Added:

- **S9** The Alkemio binary will not serve plain SMTP on the rendered config
  without certificates. Generating certificates would give one row a transport
  cost no other row pays.
- **S10** A licence, setup or API document of a row can be attributed only by
  inventing a path, URL or blob. The protocol stays a draft with that gap
  named; no freeze.
- **S11** Alkemio's captured licence is not on `PERMISSIVE`.
- **S12** A real proof fails after the freeze because a frozen fact is wrong.
  Never edit the frozen bytes; report the fact and output. The fix is a new
  protocol version or the operator's ruling.
- **S14** A tightening in V4 or V15 would make the frozen bytes fail `check`
  or a test pinned to them. Do not tighten past the frozen bytes and do not
  edit them; report it as a protocol v4 item.
- **S12′** If the operator strikes condition (f) or (g) when ruling on 0130,
  v3 cannot collect as frozen. The remedy is a new protocol version or a
  ruling, never an edit of the frozen bytes. Nothing has been collected that a
  strike could invalidate.
- **S13** Anyone reads `does_not_start` as denominator shrinkage under 0091
  §1. This spec reads 0091 §1 ("preserve … unstarted work") and §3
  ("full-product entries without fabricated trial results") as governing; a
  disagreement is the operator's to rule.

## Out of scope

Any collection; paid or agent work; changes under `src/`; a new crate;
rebuilding the cgo binary; Docker; `patchelf`; TLS for any row; a glibc
runtime pin; IMAP; gpg or `.asc` verification; committing the Maven `.sha1`,
`.sha512`, `.md5`, `.asc`, pom or metadata files; renaming the "eight-target"
files; re-capturing Mailpit or MailHog; network fetches; accepting decisions;
closing #163 or #340.

Added by the revision:
- any edit of `docs/evals/protocol.json` or `docs/evals/sources.lock.json`,
  or a re-freeze;
- a new `DocKind`;
- a new operator ask for conditions (f) and (g); they are named for the
  operator's ruling on 0130 instead;
- moving `prepare`'s I/O or `main` into a module;
- a driver identity field in the sealed report. The bench builds from the
  checkout, and the manifest's `candidate_commit` and `candidate_dirty`
  already name that checkout.
