# Eight-target mail comparison: source lock v4 and protocol v3

Date: 2026-11-07. Status: draft. Issue #340, part of #163 (Epic #164).
Base: `main` at `c95386e`. Recipe: `claude-careful`.

Decision: 0130 — [Eight comparison targets: source lock v4 and protocol v3](../../docs/decisions/0130-eight-target-mail-comparison.md), proposed; only the operator accepts.

Authorities (accepted, not amended here except as 0130 proposes):
[0091](../../docs/decisions/0091-fair-mail-comparison.md),
[0092](../../docs/decisions/0092-official-mail-comparison.md),
[0093](../../docs/decisions/0093-source-mail-comparison.md),
[0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md),
[0056](../../docs/decisions/0056-signing-policy.md).
The historical specs `fair-mail-comparison`, `official-mail-comparison` and
`source-mail-comparison` stay as written.

## What

Widen the fair wire comparison driver (`benches/mail_compare*`) from three
targets to eight. The new rows are MailSlurper `release-1.15.0`, smtp4dev
`3.16.1`, Inbucket `v3.2.0`, GreenMail `2.1.15`, and a separately labelled row
for Alkemio's patched MailSlurper image
(`ghcr.io/alkem-io/test-suites/mailslurper:smtps-4c6aecc-p2`, index
`sha256:55029f54…`). They join Swarmail, Mailpit `v1.32.0` and MailHog
`v1.0.1`. The issue asks for this work:

- **Source lock v4.** It binds lightweight tags, image digests and a pinned
  runtime, and keeps lock v3 readable and launchable.
- **Protocol v3.** It is frozen before any collection, states every row's
  storage, cap, wait method and capability facts, and starts a separate run.
  The 2026-10-30 v2 collection stays as recorded.
- **Per tool:** a launch template and a native-API decoder.
- **GreenMail:** a std/tokio POP3 reader with no new crate, run natively on a
  pinned JRE.
- **Alkemio:** the image's binary, extracted, digested and run natively.

This delivery builds and proves the tooling. It collects nothing, starts no
agent session and runs nothing paid. Smoke runs and real-binary proofs are
free, labelled and load-bounded.

## Why

On 2026-11-07 the operator widened #163 to these tools. The issue states this
as "#163 approved and widened", which is the issue's paraphrase of the ruling.
The driver cannot express three of the facts involved:

- a target outside its closed three;
- a lightweight tag, which has no tag object for 0093 §1's chain;
- a non-HTTP read path (GreenMail).

Bolting these on would break the evidence the driver exists for. A v3 lock
could be read as v4. A listener the driver does not model could leak and
still yield a "closed" receipt. A default retention could cap a run mid-way
and be recorded as a competitor loss. A third-party image could pass as an
official artifact. The design widens the targets and keeps every contract
honest. Decision 0130 records the semantic change.

## Fairness check (0091 / 0092)

No conflict with the fairness rules was found, so this is not a stop.

- Each row keeps its own declared storage, cap and wait method.
- "Not supported" keeps the full denominator (0092 §3).
- Unmatched storage and the unenforced local profile stay labelled as such
  (0092 §2, 0093 §3).
- Protocol v3 is a separate run (0091 §1).

Two boundaries are absolute:

1. No v2 and v3 cells are merged into one table or statistic.
2. The Alkemio row is never counted as an official competitor.

The implementer must **stop and report, not decide**, on any trigger S1–S6
under *Stop triggers*.

## Requirements

- **R1 Targets.**
  - `TargetKind` holds exactly eight variants, with serde names `swarmail`,
    `mailpit`, `mailhog`, `mailslurper`, `smtp4dev`, `inbucket`, `greenmail`
    and `alkemio-mailslurper`.
  - A typed `provenance()` returns `Candidate`, `Official(repository)` or
    `ThirdPartyImage(reference)` and replaces the `Option` meaning of
    `official_repository()`.
  - Target sets are scoped by version: `TargetKind::V2` (the original three)
    and `TargetKind::ALL` (the eight).
  - No name or file of the upstream MailSlurper row, or of the Alkemio row,
    collides with Swarmail's own MailSlurper shim (`tests/mailslurper.rs`,
    decision 0118).
- **R2 Lock v4.** These are the rules of decision 0130 §2.
  - `lock_version` accepts 3 and 4.
  - v3 refuses every v4 field. v4 requires the fields its pin kind needs.
  - Pin kinds:
    - Lightweight pins bind ref → commit.
    - Annotated pins keep the 0093 §1 chain.
    - Image pins bind index → manifest → layer → member.
    - Runtime pins bind the archive and its checked members.
  - Listener sets, launch environment and config files are as 0130 states.
  - A *pending* entry may stand for a target whose bytes are not available.
  - Protocol v2 pairs only with lock v3, and protocol v3 only with lock v4.
- **R3 Protocol v3.**
  - It has v2's shape, plus all eight targets, per-target `capabilities` and a
    per-product `scope`.
  - Workloads, corpus, budgets, resources and statistics shapes are copied
    from v2 unchanged.
  - `orders` are the eight cyclic rotations of the target list.
  - Every cap is set at launch, or is `no_cap_observed`.
  - It is frozen only when nothing is unresolved.
- **R4 Lifecycle.**
  - All of a target's listeners are reserved, substituted, bind-proved and
    proved closed.
  - The owned group listens on exactly the declared set.
  - Readiness is per target: an HTTP listing `200`, or for GreenMail a POP3
    `+OK` greeting. Both come after the SMTP `220`.
  - Every launch-checked member is digested at launch and again at seal.
- **R5 Decoders.**
  - Each new target's list, find and fetch follow its captured API documents
    and the pinned executable, as the existing mapping rule says
    (`targets.rs`).
  - Incomplete, duplicate, moving or non-progressing scans stay errors
    (0092 §4, 0093 §6).
  - A target without a verified search is scanned completely and filtered on
    the client, with the method labelled per row.
- **R6 GreenMail POP3.**
  - The reader lives in `benches/mail_compare/wire.rs` on tokio, with no new
    crate. It sends `USER`/`PASS`, `STAT`, `UIDL`, `TOP n 0`, `RETR` and
    `QUIT`, and never `DELE`.
  - It opens a fresh session per scan.
  - It holds a per-line cap, a per-message cap at `max_response_bytes`, the
    request deadline and dot-unstuffing, and it is cancellation-safe.
  - IMAP is recorded as not implemented.
- **R7 Native runs.**
  - Every target runs as the driver's native child. The reason is recorded in
    0130 §5, not cited to 0092 §6.
  - The JRE and every download sit in their own new directory, with the digest
    verified before use and Python run only with `-I`.
- **R8 Wrapper.** `scripts/compare-mail.sh` derives the required competitor
  binaries from the protocol through the driver, with no fixed `mailpit
  mailhog` loop. It accepts `--bin NAME=PATH` and keeps `--mailpit` and
  `--mailhog` as aliases.
- **R9 Records.**
  - The protocol v2 and lock v3 bytes are moved under `docs/evals/history/`
    with sha256 unchanged.
  - `docs/evals/protocol.json` becomes v3 and `docs/evals/sources.lock.json`
    becomes v4.
  - The new captures go under `docs/evals/sources/<target>/`. No v3 document
    under `docs/evals/sources/mailpit|mailhog/` changes.
  - The README gets a v3 pointer section with no comparison claim.
  - 0130 gets its index row. 0091 and 0093 get dated pointer addenda.

## Acceptance Criteria

Each criterion can go red. "Test" means a `#[test]` in `tests/mail_compare.rs`
unless stated. "Real" means an `#[ignore]`d test that runs the pinned bytes and
fails without them; it never skips.

- **AC1 Decision record.**
  - `docs/decisions/0130-eight-target-mail-comparison.md` has
    `- **Status:** proposed`.
  - `docs/decisions/README.md` has its row.
  - 0091 and 0093 each gain one dated 2026-11-07 pointer addendum citing #340's
    paraphrase of the ruling and saying it is a paraphrase.
  - `git diff c95386e -- docs/decisions/0091* docs/decisions/0093*` shows added
    lines only.
- **AC2 Historical bytes.**
  - `docs/evals/history/` holds the pre-change `protocol.json` (v2) and
    `sources.lock.json` (v3), each with sha256 equal to the bytes at `c95386e`.
  - A test proves that the history v3 lock still passes `validate` and
    `verify_files` against `docs/evals`. It also proves that
    `collection_ready(history v2 protocol, history v3 lock)` refuses for no
    reason other than those it refused for at `c95386e`. The 2026-10-30
    collection record is byte-unchanged.
- **AC3 Version gates.** Tests prove each of these:
  - v3 refuses each v4 field: `ref_kind`, `image`, `runtime`, `listeners`,
    `launch_env`, `config_file` and a pending entry.
  - v4 refuses a tagged pin without `ref_kind`.
  - v3 and protocol v2 refuse each of the five new targets.
  - v4 and protocol v3 refuse a missing target.
  - Pairing protocol v2 with lock v4, or protocol v3 with lock v3, is refused.
  - Each refusal names the field or target.
- **AC4 Lightweight provenance.** Tests use authored captures.
  - A lightweight pin validates when the ref names the commit, the commit's
    untruncated tree holds every attributed document, and there is no tag
    object.
  - It is refused when:
    - it carries a `tag_object`, a `tag_signature` or a `git_tag` document;
    - the ref's `object.type` is `tag`;
    - the ref names another commit or another tag;
    - the tree is truncated;
    - an attributed document is absent from the tree.
  - The release tag shapes `release-1.15.0`, `3.16.1` and `v3.2.0` pass;
    `latest`, `main`, `release-`, `v1` and `1.15.0-rc` are refused.
  - The annotated Mailpit and MailHog pins still validate unchanged.
- **AC5 Image provenance.** Tests prove each of these:
  - An image pin validates only when sha256(captured index bytes) equals
    `index_digest`, the index names `manifest_digest` for the declared
    platform, and the manifest names `layer_digest`.
  - Any mismatch is refused.
  - A missing `manifest_digest` is refused.
  - An observed `tagged_source` or `binary_build_provenance` is refused.
  - `scope: official_artifact` on an image target is refused, and so is
    `third_party_image` on an official one.
- **AC6 Listener ownership.** Tests use the real Swarmail binary and scripted
  children through existing seams.
  - A launch whose template names an undeclared slot, or omits a declared
    listener, is refused.
  - A launched child that also listens on an undeclared loopback port is
    refused with `UndeclaredListener` and cleaned up.
  - The cleanup facts report each declared listener. `listeners_closed` is
    false when any one stays open.
  - Duplicate roles are refused, and so is a role outside
    `smtp|http|pop3|imap|ui`.
- **AC7 Launch environment and config file.** Tests prove each of these:
  - A `launch_env` name that is `PATH`, `HOME`, `TMPDIR`, `LANG`, `XDG_*` or
    not upper-snake is refused.
  - Slots in values and config lines are substituted.
  - A config file is written inside the owned scratch before spawn and removed
    with it.
  - A config name with `/` or `..` is refused.
- **AC8 Protocol v3.**
  - `docs/evals/protocol.json` has `protocol_version` 3, id
    `fair-mail-comparison-v3`, `supersedes: fair-mail-comparison-v2`, and a
    `change_reason` citing 0130.
  - Every one of the eight targets has one retention row (backend, cap or
    `no_cap_observed`, prune, compression), one observation per wait profile,
    one product row with a `scope`, and one `capabilities` record.
  - Tests refuse each of these:
    - a v3 that omits a target, a capability key or a product scope;
    - a `not_supported` or `supported` without a citation;
    - a citation that names no document in that target's pin;
    - a v3 reusing the v2 id;
    - a cap below the workload;
    - a merged report over a v2 and a v3 run.
  - The throughput and wait profiles, corpus, budgets and resources are equal
    field-by-field to the history v2 bytes. A test diffs them.
- **AC9 Freeze honesty.**
  - `status` is `frozen` only when `unresolved` is empty and every competitor
    entry is a verified pin.
  - Otherwise it is `draft`, and each missing fact is named in `unresolved`.
  - `check --protocol docs/evals/protocol.json --sources
    docs/evals/sources.lock.json` exits 0 only if frozen and ready. Otherwise
    it lists every refusal.
- **AC10 Decoders.**
  - For each of MailSlurper, smtp4dev, Inbucket, GreenMail and Alkemio, tests
    feed `decode_page` (or the POP3 parser) independently authored bodies, not
    the decoder's own output. They cover a complete page, a truncated body, a
    wrong `start` or page number, a repeated id, a moving total, an oversized
    body and a malformed body.
  - Each real test delivers over real SMTP to the pinned executable. It reads
    the message back through the native API (POP3 for GreenMail), and subject,
    text and HTML equal what was sent.
- **AC11 POP3 reader.** Tests against a scripted local responder prove each of
  these:
  - dot-unstuffing, including a body line `..` and a line that is a lone `.`
    inside a stuffed body;
  - `-ERR` at each step;
  - an over-long line;
  - an oversized message;
  - a responder that stalls (deadline);
  - cancellation mid-`RETR` leaves no task behind;
  - no `DELE` is ever sent;
  - a `STAT` count that disagrees with `UIDL` is `Incomplete`.
  The paused clock is used and nothing sleeps as synchronization (0020 §5).
  A real test reads from a real GreenMail under the pinned JRE.
- **AC12 Caps as launched.**
  - Each new pin's `storage.max_messages` equals its launch value
    (`cap_flag` or env). It is at least every profile's per-instance
    residency, or it is `no_cap_observed` with a basis.
  - smtp4dev's default retention and Inbucket's default per-mailbox cap are
    recorded with their captured citations and overridden at launch.
  - A real smoke shows no eviction at the workload count.
- **AC13 Runtime pin.**
  - GreenMail's pin carries a JRE `runtime` with the archive sha256 and
    checked members (`bin/java`, `lib/server/libjvm.so`), plus the jar as
    `{artifact}`.
  - A test proves that a mismatch of any checked member or the jar refuses the
    launch.
  - The runtime license is checked against the 0130 §6 runtime list, never
    against `PERMISSIVE`.
- **AC14 Alkemio row.**
  - The full index digest is recorded from the labelled source, never
    completed by guess.
  - The binary is extracted into its own empty directory, its sha256 is
    recorded, and it runs natively under target `alkemio-mailslurper` with
    product scope `third_party_image`.
  - No record, label or file merges it with `mailslurper`.
- **AC15 Wrapper.** Free tests run the real script with no paid input and
  prove each of these:
  - `compare-mail.sh local` with a v3 protocol requires every declared
    competitor by name.
  - An unknown `--bin` target is refused.
  - `--mailpit` and `--mailhog` still work.
  - `controls`, `evidence` and `measure` refuse exactly as before.
- **AC16 Bounded load.**
  - Every smoke or real-binary measurement in this delivery runs as a named
    command under `timeout 600` and `taskset` on at most `nproc/4` CPUs.
  - `/proc/loadavg` is recorded before and after.
  - The commands, durations and load figures are reported in the delivery
    notes.
- **AC17 Nothing paid, nothing added.**
  - No agent session starts. `controls`, `evidence` and `measure` never run
    with real inputs. `local` runs only as a labelled smoke.
  - `git diff c95386e -- Cargo.toml Cargo.lock evals/Cargo.lock deny.toml src/`
    is empty.
  - Commits are unsigned. Nothing is pushed, merged, published, tagged or
    accepted.
- **AC18 Docs.**
  - `docs/evals/README.md` has a v3 / lock v4 section that makes no comparison
    claim. It names each row's labels (lightweight source, third-party image,
    POP3 read path, client-side filtering, unmatched storage, unenforced local
    profile).
  - The section lists what stays open: collection, final contracts, controls,
    cost, identity, full product, #148/#201, any pending row and best-wait.
- **AC19 Gates.** Every house gate passes, `--locked`:
  - the evals crate against the candidate binary;
  - the coverage gate at 100% and its selftest;
  - the publish-guard and seal proofs;
  - `cargo deny check licenses`.
  The bundle compile runs in verify if this sandbox has no brokkr, and the
  delivery says so.

## Stop triggers (stop and report, never decide)

- **S1** A wait or search facility cannot be source-verified, and filling the
  capability would mean assuming it. It stays `not_inspected`.
- **S2** Lightweight provenance validates only if a tag object or a built-from
  claim is invented.
- **S3** Any row would read as matched or enforced while its storage or
  resource profile is not.
- **S4** A tool needs a listener that cannot be moved to a reserved loopback
  port, so its default fixed port stays bound.
- **S5** The Alkemio license or the JRE's falls outside the list 0130 names,
  or the captured GreenMail API document contradicts "REST cannot read
  messages".
- **S6** The `compare-bins/2026-11-07*` bytes, or their digests, are not
  visible. Rows stay pending, the protocol stays draft, and everything else
  proceeds.

## Out of scope

- Any collection, or the v3 matrix run.
- Paid or agent work.
- Changes under `src/`.
- MCP adapters for the new tools, stream or websocket or SignalR wait clients,
  IMAP, and Docker.
- A hostile measured corpus. Decoder fidelity on hostile bodies is proved in
  tests instead (0130 synthesis).
- Crash or durability claims.
- Accepting or renumbering decisions.
- Closing #163 or #340.
