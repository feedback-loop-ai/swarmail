# Plan — mail comparison protocol v3, ready to collect

Spec: [spec.md](spec.md). Decision: proposed
[0130](../../docs/decisions/0130-eight-target-mail-comparison.md) (addendum
only). Continues `2c28b8b`; nothing finished there is redone.

## Technical context

- Rust bench `benches/mail_compare/` (outside the root coverage denominator,
  decision 0055); its suite is `tests/mail_compare.rs` +
  `tests/mail_compare/eight_targets.rs`. No `src/` change, no new crate:
  SHA-1 (`protocol::sha1`, `git_blob_id`) and SHA-256 already exist; archive
  reading in the real tests uses the system `unzip`/`tar` as the previous run
  did.
- Data: `docs/evals/protocol.json` (v3 draft → frozen),
  `docs/evals/sources.lock.json` (lock 4), `docs/evals/sources/<target>/`.
- Bytes: `.compare-bins/2026-11-07-supply/` plus the earlier
  `.compare-bins/2026-11-07*` directories — git-ignored, untrusted.
- Host: nproc 128 → at most 32 CPUs per measurement, ≤ 600 s each.

## Constitution check

- Frozen guarantees 0001–0003, 0005: untouched (no `src/`). A competitor's
  `250` never borrows the candidate's accept==stored meaning; the claim stays
  throughput and observed visibility only (0130 §7).
- 0091 §1: the protocol is frozen before any collection and identified by
  content hash; the does-not-start row preserves unstarted work; nothing is
  dropped. §3: unsupported work keeps full-product entries without fabricated
  results. §4 is amended only through 0130 (pointer already present).
- 0092 §2: no relabelled identity (the cgo build is its own target; the
  official artifact keeps its row); unmatched runtime facts are labelled
  (host glibc, registry integrity only). §6: one lifecycle owner, unchanged.
- 0093 §1: annotated chain unchanged; lightweight chain as 0130 §2 proposes;
  no fabricated tag object. §4: common-client poll unchanged; no stream client.
- 0020: new refusal tests are inline-style integration tests in the bench
  suite; the existing POP3 cancel-mid-RETR and stall tests stand.
- Dependency policy: nothing added; runtime licences live on the runtime list
  only (ruling 4), not `deny.toml`.
- 0056: unsigned commits, no push. The PR is held (it changes `scripts/` and
  carries pointers to accepted records).

## Synthesis — the panel's positions, ruled

| Point | Simplicity | Robustness | Ruling | What the losing side costs |
|---|---|---|---|---|
| MailSlurper identities | new `mailslurper-source-build`; official row kept, `does_not_start` | two subjects, official "not benchmarked", cgo row labelled | **Simplicity's shape, both agree on substance.** `mailslurper` stays the official artifact (0092 §2 forbids relabelling it) | Robustness's reading that `mailslurper` might become the cgo build is rejected; a ninth target grows 0130 §1's closed list |
| Version 1.14.1 vs tag | closed `REPORTED_VERSIONS` constant | typed lock-level disagreement record, required when versions differ | **Simplicity.** Data must not be able to excuse a check; the operator ruled one tag | The lock bytes alone do not show the disagreement; the report cell and 0130 show it (robustness's visibility, kept) |
| Official failure | `does_not_start`, a frozen published result | observation with stderr and exit status, never a zero | **Both**, plus a real test that reproduces exit, stub line and no listener | One protocol field and one real test |
| Musl loader | per-target `RUNTIMES` table in code | `RuntimeKind::{Jre, Loader}`; explicit loader invocation, digest at use | **Simplicity's table + robustness's launch discipline and layer check** (the loader layer must be in the captured manifest) | A third runtime is a code change, by design |
| GreenMail registry | `RegistryChecksum` basis, `REGISTRY` constant, `Checksum` doc | `registry_artifact` binding with coordinates and `authenticity: not_verified` | **Simplicity's mechanism; robustness's honesty as a label** in claim, README and 0130 | No `authenticity` field in the lock; coordinates are implied by the constant URL |
| Maven extras | commit only `.sha256` | capture `.sha512`, `.asc` too | **Simplicity.** Only `.sha256` is consumed; nothing verifies `.asc` without a pinned key | A repo-only reader cannot re-check the `.asc`; the supply keeps it with its receipt |
| Git trees | `GitCommit` link; **no** tree-hash rebuild ("no consumer") | rebuild every tree before trusting it | **Robustness.** The rebuild has a consumer: every `Commit` attribution's blob id is trusted through the listing. ~40 lines on the existing SHA-1 | More validator code and two tests |
| Nested docs, help output, operator evidence | `Member`, `Output`, `Supplied` attributions | silent | **Simplicity**, with a closed `SOURCE_ARCHIVES` for the MailSlurper tarball and a `Supplied` record that must carry the document's digest | `Member`/`Output` are proved only by the real test, not offline (same trust as `Image` today) |
| Freeze order | freeze, then real tests | freeze by content hash, then proofs; never edit after | **Robustness's immutability, with an exploratory pass first**: real proofs run as untimed probes on the pinned lock, then freeze + hashes, then the recorded rerun | Two passes of the real proofs (bounded; minutes) |
| Digest at use | not raised | hash every executed byte before launch | **Already true**: `LaunchSpec::from_pin` + `Owned::launch` check the executable and checked members; the loader joins them | none |
| Readiness ≠ collection | not raised | no run directory, unmeasured | **Robustness**, as V9 | none |
| Chaos / POP3 cancel | not raised | cancellation during `RETR` | **Already proved** (`pop3_cancel_mid_retr_leaves_no_task_behind`); no new seam | none |
| cgo glibc | label "host glibc, unpinned" | silent | **Simplicity**; precedent: smtp4dev's .NET bundle already runs on host libs | The source-build row's runtime is not pinned; labelled |
| TLS for Alkemio | plain rendered config; stop if refused | plain rendered config | **Both** (S9) | none |

## Data model changes (`benches/mail_compare/protocol.rs`)

- `TargetKind::MailslurperSourceBuild` (`"mailslurper-source-build"`); `ALL`
  → 9. `Provenance::SourceBuild(&'static str)`; `official_home` returns the
  home for it. `ProductScope::SourceBuild`; `products_match` derives it.
- `Protocol.does_not_start: Option<Vec<DoesNotStart { target, cite }>>`
  (v3 only; v2 refuses it).
- `DigestBasis::RegistryChecksum`; `DocKind::Checksum`; `DocKind::GitCommit`
  (an API object).
- `Attribution::Member { archive_sha256, member }`, `Output { argv }`,
  `Supplied { record }`.
- Constants: `REPORTED_VERSIONS`, `RUNTIMES`, `REGISTRY`, `SOURCE_ARCHIVES`;
  `RUNTIME_LICENSES` gains `"MIT"`.
- Validators: `validate_common` (version exception, registry basis, source
  build download), `release_agrees` (asset skip under the registry basis
  only), `validate_runtime` (table; loader layer in manifest),
  `validate_pin` (attribution arms; `SourceBuild` needs observed build
  provenance), `tag_chain` (`GitCommit` route + tree rebuild),
  `verify_files` (checksum bytes), `Protocol::validate` (`does_not_start`),
  `capabilities_match`/`collection_ready` (the does-not-start target keeps a
  complete pin).

## Project structure — files touched

- `benches/mail_compare/protocol.rs` — the above.
- `benches/mail_compare/targets.rs` — runtime entry from `RUNTIMES`; the
  `MailslurperSourceBuild` arm wherever `Mailslurper | AlkemioMailslurper`
  match.
- `benches/mail_compare/runner.rs` — `plan()` skips `does_not_start`.
- `benches/mail_compare/report.rs` — row labels: source build, loader
  runtime, registry integrity, does-not-start, reported version.
- `docs/evals/sources.lock.json`, `docs/evals/sources/{mailslurper,
  mailslurper-source-build,smtp4dev,inbucket,greenmail,alkemio-mailslurper}/`.
- `docs/evals/protocol.json` (frozen), `docs/evals/README.md`.
- `docs/decisions/0130-eight-target-mail-comparison.md` (addendum).
- `tests/mail_compare/eight_targets.rs`, `tests/mail_compare.rs`,
  `tests/mail_compare/eight-target-smoke.json` (the smoke fixture follows the
  new pins).
- No change to `scripts/compare-mail.sh`: it derives its required binaries
  from `declared_competitors`.

## Quickstart (implementer)

```bash
cd .compare-bins/2026-11-07-supply && sha256sum -c SHA256SUMS && cd -
N=$(( $(nproc)/4 )); TS="taskset -c 0-$((N-1))"
$TS cargo test --locked --test mail_compare
cargo bench --locked --bench mail_compare -- check \
  --protocol docs/evals/protocol.json --sources docs/evals/sources.lock.json
cat /proc/loadavg; timeout 600 $TS cargo test --locked --test mail_compare \
  -- --ignored --exact <real test> --test-threads=1; cat /proc/loadavg
```

(`compare-mail.sh` runs the same bench executable it resolves from `cargo bench --no-run`; the notes quote the exact command.)

## Risks

- **The Alkemio binary refuses plain SMTP or the loader cannot run it.** →
  S9: stop and report; no certificates, no patchelf, no host loader.
- **A frozen fact proves wrong in the rerun.** → The exploratory pass before
  the freeze catches launch facts; after it, S12 (never edit frozen bytes).
- **Tree rebuild mismatch** (mode serialization, ordering). → Rebuild in
  listed order with `040000`→`40000`; prove against an authored `git mktree`
  fixture before applying to the supply; if a supplied tree still does not
  rebuild, its row's `tagged_source` stays unresolved and S10 applies (no
  freeze).
- **A nested document cannot be attributed.** → `Member` of an allowed
  archive; if none holds it, S10.
- **`does_not_start` read as shrinkage.** → S13; the row, pin, product and
  denominator all stay; 0130 records the operator's ruling.
- **Ninth target breaks fixed-size assumptions** (rotations, `tag()` index,
  smoke fixture, `[_; 8]` arrays). → `ALL`-driven code; tests for nine
  rotations and nine capability records; grep for `8` literals.
- **`Member`/`Output` checked only by the real test.** → Accepted (no archive
  crate); the real test runs in this story and is required before any
  collection; 0130 says so.
- **Host glibc under the cgo row; .NET host libs under smtp4dev.** → Labelled
  in claim and README; no matched-runtime claim.
- **Registry checksum is same-origin.** → Labelled integrity-only.
- **Load.** → V11's bound, one test at a time; abort a proof that exceeds 600
  s and report it.
- **Untrusted bytes.** → V1: digest first, fresh directories, `python3 -I`,
  scripts outside the byte directories, nothing committed from
  `.compare-bins`.
- **Held PR misread as mergeable.** → V14 states the hold reasons.
