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

## Review return (2026-11-07)

The review of `c5a784c` (all five members `residual`, `spec_defect: true`, max
medium, no security residual) sent the run back to design. The code and the
frozen bytes were found sound. The defect was the spec: V8 required `check` to
pass with the Alkemio image pin and the source-build row, while V10 allowed
only five widened arms. The implementer widened two more conditions and
disclosed them in 0130 as open for the operator. So 0130 called v3 "frozen"
and the condition it rests on "pending" in the same file.

This revision changes no frozen byte. `docs/evals/protocol.json` (`229babaa…`)
and `docs/evals/sources.lock.json` (`ed201599…`) stay as frozen. Every change
lands in code, tests, 0130's second addendum, a 0091 pointer line, the
decisions index or the README. Nothing finished at `c5a784c` is redone except
the receipts (V9).

### Synthesis of the two positions

| Point | Simplicity | Robustness | Ruling | What the losing side costs |
|---|---|---|---|---|
| The two unnamed conditions | name them in V10 as entailed by the operator's text; no new ask; keep the freeze | the same, as an enumerated derivation table (a)–(h), and supersede the "open" sentence | **Both, in robustness's form.** V10 becomes a table of condition, authority and the refusal kept | none; the table is text plus one new test |
| New operator ask, or unfreeze | refuse both | refuse both; only a strike makes a new version | **Refused.** The PR is held for the operator anyway, and 0130 stays proposed, so the operator's ruling on 0130 is the ruling on (f) and (g) | if the operator strikes (f), v3 cannot collect as frozen (S12′); nothing collected is lost |
| Code shape of (f) | keep `pin.image.is_none()` | a predicate over the closed provenance | **Robustness.** `Provenance::has_tagged_source()`; the lock data cannot opt in by carrying an `image` field | a few lines |
| "Nine" vs "eight" | addendum plus 0091 pointer | the same, plus the index row and a V8 scope sentence | **Robustness**: all three, by addition | none |
| smtp4dev runtime label | one `row_labels` arm | the same, plus a sealed-report test | **Both** (V15) | none |
| `does_not_start` cite | require `Supplied`/`Output` attribution, no new `DocKind` | require failure output, but only if the frozen bytes pass | **Simplicity's mechanism, robustness's order**: check the committed cite first (it is `Supplied`), then tighten; S14 otherwise | a README-kind failure document stays possible in a later lock; the attribution, not the kind, carries the meaning |
| `admit_run` | add `blocked` and `labels` keys | the same | **Both**, gated on protocol v3 so v2 admission keeps its terms | a v3 report's labels are sealed contract; a label change after a v3 collection needs a new protocol version |
| Launch filter | one owner plus an exact plan count; no `run()` test | a `run()` test over a tiny protocol | **Both.** A single owner removes the duplication, but `run()` could still bypass it, and only a `run()` test turns red then. That test runs the candidate alone (real binary, existing smoke machinery), so it adds seconds | one more real smoke-sized run in `cargo test` |
| `flags`/`prepare` | move `flags` only; leave `prepare` | move both into an includable module | **Middle.** Move `flags` and the pure `--bin`/`--runtime` → `LaunchSpec` binding (`launch_specs`) into `runner.rs`. `prepare` keeps its I/O (file reads, scratch creation) | `prepare`'s file-loading and mode switch stay proved by the wrapper smoke only |
| Hostile/multipart assertions, rename, wall-clock deletion | yes | yes | **Both** (V17) | none |
| Alkemio batching | one bounded unbatched probe, then keep or drop the batching on the evidence | fix the wording only | **Simplicity.** A fact replaces an assumption for one bounded run | one real run (≈10 s) |
| Receipts | rerun only the touched tests | rerun all at the final head and name the tree | **Robustness.** The driver changed after the recorded proofs at `c5a784c`, and changes again here. The proofs take seconds | about 2 minutes of bounded runs |
| Driver identity in the sealed report | not raised | add the driver tree | **Rejected.** The bench is built from the checkout the manifest's `candidate_commit`/`candidate_dirty` already name. A report field would change the report schema after the freeze for no new fact | none |
| `ConfigFile.cwd` escape test | n/a | refuse an absolute or escaping cwd | **Not applicable.** `cwd` is a bool, and the config name is already confined (`config_file_name_confined`) | none |

### Finding → change

| Finding (member, severity) | Change | Where |
|---|---|---|
| V8/V10 contradiction: image-pin gap and source-build asset/download skips unnamed (correctness 1 medium; guarantees 2; independent 1; security medium) | V10 table (f), (g), (h); "Why" names entailed conditions; "Forced by ready to collect" adds three bullets; V8 scope sentence; provenance predicate; new `skipped_asset_list_still_checks_tag_commitish_and_date` | spec V8, V10; tasks R1, R2 |
| 0130 §1/§3 and the 0091 §4 pointer say eight (guarantees 1) | V12′ addendum; 0091 pointer line; index row | spec V12′; tasks R9 |
| GreenMail REST stale text (guarantees low a) | V12′ correction of record; `wire.rs` comment | tasks R9 |
| `does_not_start` cite may be any document (guarantees low b) | attribution must be `Supplied`/`Output`; refusal test | spec V4; tasks R3 |
| `admit_run` ignores blocked rows and labels (guarantees low c) | compare `blocked`, `labels` for v3; refusal test | spec V15; tasks R4 |
| `server_search` vs row label (guarantees info; correctness 6) | one sentence in V12′ | tasks R9 |
| smtp4dev runtime labelled only in the README (independent 2) | `row_labels` arm and test; V12′ wording | spec V15; tasks R4 |
| T2.7 rename missed (independent 3) | rename | spec V17; tasks R6 |
| Alkemio batched without evidence (correctness 2; tests 4; independent 4) | bounded unbatched probe; keep or drop on the result; V12′ records it | spec V17; tasks R7 |
| `flags`/`prepare` untested; repeated `--runtime` fix untested (tests 1) | `flags` and `launch_specs` into `runner.rs`, four tests | spec V16; tasks R5 |
| `run()` filter untested (tests 2) | `Protocol::launch_order`; `run_never_launches_a_does_not_start_target` | spec V16; tasks R5 |
| hostile/multipart bodies only printed (tests 3) | exact assertions | spec V17; tasks R6 |
| wall-clock assertion and port race (tests 5) | delete the assertion; keep the fail-closed reservation, recorded | spec V17; tasks R6 |
| plan test uses `any`/`all` (tests 6) | exact count | spec V16; tasks R5 |
| musl `holder` (correctness 3) | V12′ note; lock unchanged | tasks R9 |
| GreenMail licence is a declaration; GitCommit link trust; `query_value` (correctness 4–6) | already disclosed in 0130; no change | — |
| `c5a784c` landed after the freeze and the receipts (correctness 7) | rerun every receipt at the final head with HEAD and tree | spec V9; tasks R8 |

No acceptance criterion was dropped or loosened. V10's count changed from
"five" to an enumerated table. That corrects the criterion: as written, it
could not hold together with V8. Every reviewer showed this from the code
(`protocol.rs` 2883–2885 against 3652–3655), and the rows it adds each keep a
named refusal.

### Files the revision touches

- `benches/mail_compare/protocol.rs`: `Provenance::has_tagged_source`,
  `Protocol::launch_order`, the `does_not_start_matches` attribution check.
- `benches/mail_compare/runner.rs`: `plan`/`run` use `launch_order`; `flags`
  and `launch_specs` move here.
- `benches/mail_compare.rs`: `prepare` calls them.
- `benches/mail_compare/report.rs`: the smtp4dev runtime arm; `admit_run` v3
  keys.
- `benches/mail_compare/wire.rs`: the comment.
- `tests/mail_compare.rs`, `tests/mail_compare/eight_targets.rs`.
- `docs/decisions/0130-eight-target-mail-comparison.md` (addendum),
  `docs/decisions/0091-fair-mail-comparison.md` (pointer line),
  `docs/decisions/README.md` (the 0130 row), `docs/evals/README.md`.
- Never: `docs/evals/protocol.json`, `docs/evals/sources.lock.json`,
  `docs/evals/history/`, `src/`, `Cargo.*`, `deny.toml`.

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

### Risks added by the review return

- **The operator strikes (f) or (g).** → S12′: v3 cannot collect as frozen.
  The remedy is a protocol v4 or a ruling, never an edit. Both conditions
  stand in their own named paragraph so they can be struck by name.
- **A tightening fails the frozen bytes.** → The committed cite is `Supplied`,
  so the attribution check passes it. The implementer runs `check` and
  `frozen_v3_and_lock_v4_bytes_are_the_recorded_ones` after each tightening.
  On failure, S14: revert the tightening and report it as a v4 item.
- **`admit_run`'s new keys refuse an honest report.** → Both keys are pure
  functions of the protocol. They apply to v3 only, and no v3 report exists
  yet. The refusal test also admits an untampered report.
- **Moving `flags` and `launch_specs` changes the wrapper's behaviour.** → The
  move keeps behaviour unchanged. The four V16 tests and one rerun of the
  wrapper smoke (bounded, unmeasured, `compare-mail.sh local` with the smoke
  fixture, as 0130 records) prove the call path.
- **The `run()` test adds wall time and a real launch to `cargo test`.** → It
  launches only the candidate, with the existing smoke machinery. It is
  Linux-gated like `a_real_smoke_run_reconciles_every_message_and_wait_and_seals`.
- **The unbatched Alkemio probe loses mail and is read as a defect.** → It is
  an observation about the fork, recorded with its count. The batching stays
  and the cap claim stays "no eviction".
- **Load.** → Every rerun is V11-bounded and runs one test at a time, with the
  loadavg recorded.
