# Mail comparison protocol v3, ready to collect from the supplied bytes

Date: 2026-11-07. Status: draft. Issue #340, part of #163 (Epic #164).
Run: `second-rerun-of-340-operator-dir-d2bccbd0` (operator direction
2026-11-07, "Pragmatic set"). Branch `repair/issue340-compare-benches`,
continuing from `2c28b8b`. Base: `main` at `c95386e`. Recipe:
`claude-careful`.

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
has to be closed by an observation from the supplied bytes or by one of the
operator's rulings, and never by relaxing a check the rulings do not name.

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
- **V10 No refusal loosened.** `git diff 2c28b8b --
  benches/mail_compare/protocol.rs` removes no refusal and widens no
  condition except the five ruling-scoped arms (`REPORTED_VERSIONS`,
  `RUNTIMES`, the `RegistryChecksum` asset-list skip, `MIT` on
  `RUNTIME_LICENSES`, the `GitCommit` tree route), each with its refusal
  tests above. `git diff 2c28b8b -- tests/` deletes no test function outside
  V9's successor rule.
- **V11 Bounded load.** Every real proof, `--help`/version capture and smoke
  is a named `timeout 600 taskset -c 0-$(( $(nproc)/4 - 1 ))` command
  (`nproc` read at run time; 32 CPUs here), one test at a time
  (`--test-threads=1`), with `/proc/loadavg` recorded before and after. The
  notes report each command, its wall time, exit status and loadavg. Gate
  commands run under the same `taskset`; they are not measurements.
- **V12 The record.** 0130 gains one dated addendum, "2026-11-07 (operator
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
