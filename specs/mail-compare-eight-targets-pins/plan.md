# Plan — finishing the eight-target comparison from the pinned bytes (#340 rerun)

Spec: [spec.md](spec.md). Parent: [../mail-compare-eight-targets/plan.md](../mail-compare-eight-targets/plan.md).
Decision: [0130](../../docs/decisions/0130-eight-target-mail-comparison.md) (proposed).
This is a rerun under operator direction, not a return from review. There is
no `context.returned_from`, so no review findings are answered here.

## Summary

The code exists. What is missing is evidence: three pins to fill, two rows to
hold open with honest reasons, and real-binary proofs. The work is almost
entirely data, under `docs/evals/`, plus four bounded test changes in
`tests/mail_compare/eight_targets.rs`. Production code under `benches/`
changes only when a real run against the pinned bytes forces it. Each such
change carries the failing real test plus an authored-fixture test
(0092 §4). `src/` is not touched.

## Technical context

- **Bytes.** `.compare-bins/2026-11-07*/`, git-ignored through
  `.git/info/exclude` (`/.compare-bins/`). They are untrusted. Digests are
  read in place, extraction goes into fresh directories under
  `target/compare-dl/`, scripts go under `target/compare-tools/`, and Python
  runs with `-I`. `/tmp` is a RAM tmpfs and is not used.
- **Host.** `nproc` was 128 at design, which bounds the work at 32 CPUs. Read
  `nproc` again at run time.
- **Vocabulary that already exists** (`benches/mail_compare/protocol.rs`):
  - `Fact::{Observed, Unresolved}` on `tagged_source`, document attributions,
    reviewed sources and `best_wait`;
  - `DocKind::{ReleaseMetadata, License, Api, Setup, Source, GitRef, ...}`;
  - `Runtime` with `checked_members`, and the `RUNTIME_LICENSES` list;
  - `ListenerRole::{Smtp, Http, Pop3, Imap, Ui}`;
  - `launch_checks`, which requires `smtp` plus the read role (`pop3` for
    GreenMail, `http` otherwise) and allows extra declared roles;
  - `collection_ready`, which refuses on every gap.
- **Digest at use already exists.** `LaunchSpec::from_pin` (`targets.rs`
  ~97–143) passes the pin's `executable_sha256` and the runtime's checked
  members. `Owned::launch` (~425–444) digests the artifact, or the jar when a
  runtime is used, and refuses `DigestMismatch`. It also digests each member
  and refuses `MemberDigestMismatch`. The real tests therefore prove identity
  by digest, whatever path `MAIL_COMPARE_*` names.

## Constitution check

- **0001–0005.** Not touched, and nothing changes under `src/`.
- **0091 §1.** No collection. The method is unchanged: the workloads and
  corpus equal v2's (parent AC8). Corrections to the launch facts are made
  *before* any v3 collection and are recorded as dated pre-collection
  addenda in 0130. **§2:** no fabricated metadata. A missing fact stays
  `Unresolved` or `pending`.
- **0092 §2/§3/§4.** Profiles keep their labels, denominators stay full, and
  decoder fixes carry independent fixtures.
- **0093 §1/§4.** Neither chain is weakened. A wait nobody inspected is
  `not_inspected`.
- **0045 §8.** The pins are compatible implementation under proposed 0130.
  The PR stays held for the operator: it changes `scripts/`, and 0130 amends
  accepted 0091 §4 and 0093 §1.
- **Fairness (0091/0092).** No conflict with the ask, so no stop. Two rows
  stay open for want of bytes, not because of a fairness rule.

## How the positions were reconciled

| # | Question | Ruling | What the losing side costs |
|---|---|---|---|
| 1 | Is this a data fill (simplicity) or a re-derivation of drafted launches (robustness)? | **Robustness.** Every launch, environment, config, listener and cap fact is re-derived from the bytes and cited. No drafted fact survives by default. Corrections go into a dated 0130 addendum. | Simplicity: a 0130 edit, and a citation per fact rather than a digest paste. |
| 2 | New vocabulary: a source-archive `DocKind`, a relaxed `validate_image`, Fact-wrapped image digests | **Simplicity: none.** Gaps stay `Unresolved` with reasons, and `collection_ready` refuses them. Robustness agrees that no validator may be relaxed (its item 9). | Robustness: none. The pinned rows show their weaker evidence openly. |
| 3 | smtp4dev | **Simplicity: pending.** The zip has no license document, and an SPDX id from memory would break 0091 §2. The launch is still corrected from the binary's captured `--help` (robustness 1). | The issue's smtp4dev row is not delivered as a run. |
| 4 | Alkemio | **Both: pending.** The binary needs a musl loader that nobody supplied, and there are no index or manifest bytes. The full index digest is recorded as the dispatcher's line, never as verified. No musl runtime is proposed in 0130 now: that would be a semantic change, and the operator should rule it with the bytes in hand. | The issue's Alkemio row is not delivered. Robustness's "measure the index digest" cannot be met from these bytes and is stated as an operator ask. |
| 5 | Digest at use in `real_spec` (robustness 2) | **Rejected as already done.** `Owned::launch` digests the artifact and its members against the pin before spawn (see Technical context). An existing test covers each refusal (`checked_member_or_jar_mismatch_refuses_launch`). | None. A redundant check would add lines and add no proof. |
| 6 | GreenMail's API listener (robustness 4) | **Robustness.** Declare `http` for the API on a reserved port, through the `greenmail.api.*` properties found in the jar. The exact-listening-set check is the proof. If an undeclared port remains, S8 applies. | Simplicity: a third listener in one pin. |
| 7 | GreenMail POP3 provisioning (robustness 5) | **Robustness.** The real test logs in and reads `STAT +OK 0` before sending. | Simplicity: about five test lines. |
| 8 | POP3 cost per scan billed to GreenMail (robustness 6) | **Rejected as a method change, adopted as a label.** A fresh session per scan, `UIDL` to map numbers, and `TOP` per message are POP3's inherent cost under 0130 §4's no-cache rule. They are not driver waste. The cap proof keeps `list_all`, so it exercises the read path the measurement uses. The README row names the cost. | Robustness: GreenMail's numbers include login, UIDL and TOP per scan, visibly labelled. A `STAT`-only proof would test a path the measurement never runs. |
| 9 | A hostile kind in the v3 corpus (robustness 7) | **Rejected again** (parent ruling 8). It would break parent AC8 (v3 workloads equal v2's), and the operator asked for throughput and wait. **Adopted in a new form:** the hostile set runs against each pinned *real* binary, unmeasured (P10). | Robustness: a tool that mangles edge bodies is caught by a test, not in a measured table. |
| 10 | Caps (robustness 3, simplicity's "restricted by its own failure") | **Robustness.** The caps come from captured flags or env, and the 600-message count is never lowered. The single test is split per target, so each pinned row's proof can be green while pending rows fail by name. | Simplicity: five test functions in place of one. |
| 11 | Freeze protocol v3 | **Simplicity: draft.** A freeze spends the v3 id, and the gaps are known. Robustness did not ask for a freeze. | The issue's "frozen before any collection" is met as "no collection while draft". The freeze is a later step once the operator supplies the bytes. |
| 12 | Keep the old pending reasons legible (robustness 9) | **Partly.** The lock states current facts only: a new field for past reasons would be new vocabulary. The history lives in git and in 0130's dated addendum, which quotes each superseded draft fact. | Robustness: past reasons are not in the lock itself. |
| 13 | Mailpit and MailHog | **Simplicity.** Re-verify the digests only. Their v3 documents stay byte-unchanged. | — |
| 14 | A hand smoke of smtp4dev (simplicity) | **Narrowed.** Only bounded `--help` and version captures, to correct the pending launch. No server is started for evidence. | — |
| 15 | Load | **Both.** `taskset` to `nproc/4` for everything. Measurements also get `timeout 600` and loadavg before and after. Gates are bounded by `taskset` and `CARGO_BUILD_JOBS` but are not timed out. | — |

## Approach, per row

Each row's facts are read from the bytes and never from memory. Where this
plan names a file, it names where to look, not what the file says.

- **mailslurper.**
  - Documents, all members of the digested zip and source tarball:
    - from the zip: `LICENSE` (`license`), `README.md` and `config.json`
      (`setup`);
    - from the source tarball (`api`): the service controller and its route
      registration;
    - `git_ref`;
    - `release_metadata`.
  - The launch flag and config keys are read from the tarball's entry point
    and config loader, and confirmed by the binary's `--help` or its refusal
    of an unknown flag.
  - SQLite runs at `{data_dir}`. There is no cap flag unless the source shows
    one, so the cap is `no_cap_observed`, and the per-row 600 proof is the
    bounded probe.
  - `wait.reviewed` lists the retained source bodies with their locally
    computed `git_blob`. Run
    `git hash-object --no-filters <file>` on the extracted member, outside
    any repository.
- **inbucket.**
  - Documents: `LICENSE`, `README.md`, `etc/rest-apiv1.sh` (`api`),
    `etc/linux/inbucket.service` or another captured configuration document
    (`setup`), `git_ref` and `release_metadata`.
  - The environment names and the per-mailbox cap name are read from those
    documents, or from the binary's `-help` output, which is captured as a
    `setup` document.
  - The cap is set at least as high as every profile's residency.
- **greenmail.**
  - Documents: `greenmail-openapi.yml` (`api`), and the `license` document
    that the jar's bytes support. Candidates are `pom.xml`'s `<licenses>`,
    `NOTICE.markdown` and the openapi `info.license`. If none names a
    license on the permissive list, S5 applies and the row goes back to
    pending.
  - The other documents are `git_ref` and `release_metadata`.
  - The properties come from the jar: `greenmail.setup.*`, the hostnames and
    ports, `greenmail.api.*`, and the auth property if one exists.
  - Listeners are `smtp`, `pop3` and `http`, where `http` is the API.
  - The runtime is the JRE pin:
    - the archive digest;
    - `checked_members` `bin/java` and `lib/server/libjvm.so`, digested after
      extraction;
    - the version output from `bin/java -version`, captured;
    - the license, from `legal/java.base/LICENSE` and `ASSEMBLY_EXCEPTION`,
      checked against `RUNTIME_LICENSES`.
- **smtp4dev (pending).**
  - Capture `--help` and the version output of the extracted executable,
    under the bounded form.
  - Correct `listeners` and `launch` from them. Disable POP3 and IMAP if a
    flag allows it, or declare them otherwise.
  - Cite `appsettings.json` for the defaults.
  - Rewrite the reason (P5).
  - Quote the captured output in 0130's addendum.
- **alkemio-mailslurper (pending).** Rewrite the reason (P6). Do not attempt
  to execute the binary.
- **Protocol v3.**
  - Update `unresolved`, `capabilities`, and the storage caps of the pinned
    rows (P7).
  - `status` stays `draft`.
  - Keep `tests/mail_compare/eight-target-smoke.json` consistent: the
    existing equality test compares its `storage` and `capabilities` with the
    committed protocol.
- **0130.** Add one dated section, `## 2026-11-07 addendum (rerun,
  pre-collection)`. It holds the per-row corrections table, the two open rows
  and the operator asks below. The body above it is unchanged.
- **README.** Update the v3 pointer section (P12).

## Files touched

```
docs/evals/sources.lock.json                    three pins; two pending reasons rewritten
docs/evals/sources/{mailslurper,inbucket,greenmail}/   captured documents (bytes from the digested archives)
docs/evals/protocol.json                        v3 draft: unresolved, capabilities, caps
tests/mail_compare/eight-target-smoke.json      kept equal to the committed storage/capabilities
tests/mail_compare/eight_targets.rs             split cap test; hostile real tests; GreenMail
                                                empty-STAT precondition; smoke-fixture pending set
docs/decisions/0130-eight-target-mail-comparison.md   dated rerun addendum (additions only)
docs/evals/README.md                            v3 pointer section updated
benches/mail_compare/*.rs                       only if a real run forces a fix (with its tests)
```

Nothing under `src/`, `Cargo.*`, `evals/Cargo.lock`, `deny.toml`, `.github/`,
`scripts/` or `.brokkr/` changes in this run.

## Operator asks (quoted in the delivery notes)

1. **smtp4dev:** supply the license document of `rnwood/smtp4dev` at commit
   `9fac83c10cb7cb58105bc0d9d891d36150b8605f` (the file, or a `git_tree` of
   that commit together with the blob). Nothing else blocks the pin.
2. **alkemio-mailslurper:** supply the OCI index bytes for
   `sha256:55029f542c19d0fe570450dc5a30f54dbdb24d9113a6e0e1bf96da000ebd0838`,
   the linux/amd64 manifest, and the layer(s) holding
   `/lib/ld-musl-x86_64.so.1`. Rule whether a musl loader may be pinned as a
   second runtime: 0130 §2 currently allows only GreenMail's JRE. Note that
   `version.json` in the image reads 1.14.1.
3. **All new rows:** `git_tree` captures, plus GreenMail's `git_tag`, would
   turn the `tagged_source` gaps into observations without code changes.
4. **0130 §6:** the runtime license list awaits the operator's ruling.

## Quickstart (bounded, labelled, never evidence)

```bash
n=$(( $(nproc) / 4 )); cpus="0-$(( n - 1 ))"; export CARGO_BUILD_JOBS=$n
dl=$(mktemp -d -p target/compare-dl inbucket.XXXX)        # one fresh dir per archive
tar -xzf .compare-bins/2026-11-07-inbucket/inbucket_3.2.0_linux_amd64.tar.gz -C "$dl"
cat /proc/loadavg
time timeout 600 taskset -c "$cpus" env MAIL_COMPARE_INBUCKET="$dl/inbucket_3.2.0_linux_amd64/inbucket" \
  cargo test --locked --test mail_compare -- --ignored --exact real_inbucket_smtp_to_native_api_round_trip
cat /proc/loadavg
```

Check that `echo "$cpus"` prints `0-31` on a 128-CPU host before the first
measurement. The GreenMail form adds `MAIL_COMPARE_GREENMAIL` (the jar) and
`MAIL_COMPARE_JRE` (the extracted JRE root).

Design-seat observation, to re-verify: the MailSlurper and Inbucket
executables extracted from their archives are statically linked ELF files, so
they need no loader. The smtp4dev executable is a glibc PIE.

## Risks

| Risk | Mitigation |
|---|---|
| A drafted launch fact survives under a verified digest, so a guessed method looks proven | P3: every fact cites a captured document or captured `--help`. 0130's addendum lists each draft fact as confirmed or changed. Review rejects an uncited fact. |
| The implementer fills a gap from memory (an SPDX id, a flag, the GreenMail tag, the index digest) | The brief and 0091 §2 forbid it. Gaps stay `Unresolved` or `pending` with a reason. The digests here are design observations that T0 re-verifies, and they are never copied. |
| A validator is relaxed to make a row green | P2's diff conditions, and existing negative tests kept. `git diff 4012f31 -- tests/` deletes no test. |
| Untrusted archives exploit the reader (zip-slip, planted interpreter modules) | Each archive is extracted into a fresh empty directory with `tar`/`unzip`, never into the download directory. Scripts are kept elsewhere. `python3 -I` only. Members are listed before extraction, and an absolute or `..` member refuses. |
| A pinned row's real launch opens a port nobody declared (GreenMail's API, or smtp4dev's IMAP/POP3) | Declared listeners plus the exact-listening-set check at launch. An undeclared port refuses (`UndeclaredListener`) and is cleaned up. S8 if a port cannot be moved. |
| A default retention evicts mail during the cap proof, and someone lowers the count | Caps come from captured flags or env. The 600 count is fixed (P9). Each row's cap test is separate, so a failure names its row. |
| GreenMail rejects POP3 login on an empty store, which would read as a competitor loss | The real test proves login and `STAT +OK 0` before sending (P4). If that fails, the launch's auth properties are wrong: fix them from the jar's bytes, never by changing the reader. |
| A tool's real behaviour on hostile bodies differs from what was sent | S7: report the row with the bytes. Neither the assertion nor the corpus is weakened. Authored-fixture tests separate a driver bug from tool behaviour. |
| The JRE's captured license text does not match the runtime list's expression | S5: GreenMail stays pending. Neither `RUNTIME_LICENSES` nor `PERMISSIVE` is edited to fit. |
| `.compare-bins/` or extracted bytes get committed | `git ls-files .compare-bins target` is empty before every commit. Only the documents named in a pin are copied into `docs/evals/sources/<target>/`, with their digests in the lock. |
| Load spikes on the shared host (an earlier unbounded proof reached load 177) | `taskset` to `nproc/4` and `CARGO_BUILD_JOBS=nproc/4` for everything. Each measurement gets `timeout 600`, and loadavg is read before and after. The real-binary tests run one at a time with `--exact`, never as a batch of servers. |
| The JVM or the .NET runtime ignores affinity and spawns more workers | Affinity is inherited by every thread. `taskset` bounds CPUs, not threads. Loadavg receipts show the effect, and a reading above `nproc/4` is reported. |
| Two rows stay open and the delivery is reported as complete | P15: the result notes say "not complete", with the asks quoted. The verify and review seats read the pending entries directly. |
| 0130 is edited beyond an addendum and loses its proposal history | P3: additions only, checked with `git diff 4012f31 -- docs/decisions/0130*`. |
| The smoke fixture drifts from the committed protocol | The existing equality test on `storage` and `capabilities`, and P12's exact pending set. |
