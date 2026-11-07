# Tasks — finishing the eight-target comparison from the pinned bytes (#340 rerun)

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md) · Decision: [0130](../../docs/decisions/0130-eight-target-mail-comparison.md) (proposed)
Parent tasks: [../mail-compare-eight-targets/tasks.md](../mail-compare-eight-targets/tasks.md). Its T1–T16, T19 and T20 were built in `7237c93`…`4012f31` and are not redone. This list finishes its T0, T17, T18, T21 and T22.

Work in order. Every measurement uses plan.md's bounded form and records
loadavg before and after. Commit unsigned in small commits, never push, and
run `git ls-files .compare-bins target` before every commit; it must print
nothing. If a stop trigger (S1–S6, S6′, S7, S8) fires, stop that row and
report it while the other rows proceed.

## Phase 0 — bytes

- [ ] **T0 Digest receipt (P1).**
  - For each artifact in spec.md's table: `sha256sum` it in place and compare
    it with its corroboration (the lock, `SHA256SUMS`, `checksums.txt`, the
    `.sha256` file, or a release metadata asset). Do this before anything
    else reads it.
  - List each archive's members, and refuse any member that is absolute or
    contains `..`.
  - Extract into a fresh `mktemp -d -p target/compare-dl <row>.XXXX`. Keep
    the scripts in `target/compare-tools/`, and run Python only as `python3 -I`.
  - *Proof:* a receipt table in the delivery notes (path, expected digest,
    observed digest, verdict), plus empty `git ls-files .compare-bins target`
    output.
- [ ] **T1 Byte facts (P3, P5, P6).**
  - Capture these, each command as `timeout 60 taskset -c "$cpus" …`:
    - `--help` and version output of the MailSlurper, Inbucket and smtp4dev
      executables;
    - `bin/java -version` of the JRE;
    - the GreenMail property names, from the jar's bytes.
  - Record, for the Alkemio executable, its ELF interpreter, its `NEEDED`
    entries and `version.json`, using `readelf`/`file` and not executing it.
  - *Proof:* the captured outputs exist as files under `target/compare-dl/`,
    and the ones a pin cites are copied into `docs/evals/sources/<target>/`
    with their digests.

## Phase 1 — records

- [ ] **T2 MailSlurper pin (P2, P3, P7).** Write the pin following plan.md's
  per-row approach:
  - the documents;
  - `tagged_source: Unresolved`, with a reason naming ref `068acffd…`
    (lightweight), the missing `git_tree`, and the tarball's digest;
  - the launch, config file and listeners, from the source and `--help`;
  - `no_cap_observed`, unless the source shows a cap;
  - `wait.reviewed` from retained source bodies;
  - capabilities, each cited or `not_inspected`.
  - *Proof:* `current_lock_v4_validates_and_verifies`.
  - *Proof:* `mail_compare check` lists only the gaps named in P7 for this
    row.
- [ ] **T3 Inbucket pin (P2, P3, P7).** The same shape as T2, with ref
  `74224969…` (lightweight). The cap env name and value are cited, and the
  value is at least the largest profile residency.
  - *Proof:* `current_lock_v4_validates_and_verifies`.
  - *Proof:* `v3_cap_below_workload_refused` (existing) still passes against
    the new storage row.
- [ ] **T4 GreenMail and JRE pin (P2, P3, P4).**
  - The release tag is `release-2.1.15`. `tagged_source` is `Unresolved`,
    with a reason naming the annotated ref, tag object `5f09fccc…`, and the
    missing `git_tag` and `git_tree`.
  - Listeners: `smtp`, `pop3`, `http` (the API), through properties found in
    the jar.
  - The license document is one the jar's bytes support (S5 otherwise).
  - The runtime pin has the archive digest, `checked_members` `bin/java` and
    `lib/server/libjvm.so`, the version output and the license from `legal/`.
  - *Proof:* `current_lock_v4_validates_and_verifies`.
  - *Proof:* `runtime_license_checked_against_runtime_list_only` (existing).
  - *Proof:* `checked_member_or_jar_mismatch_refuses_launch` (existing).
- [ ] **T5 smtp4dev stays pending (P5).**
  - Correct `listeners` and `launch` from the captured `--help`. Disable POP3
    and IMAP if a flag allows it, or declare them otherwise.
  - Rewrite the reason to name: no license document (49 members), the
    digest `3b9395b9…`, the default retention of 100 from `appsettings.json`,
    and the commit `9fac83c1…`.
  - *Proof:* `pending_entry_validates_but_refuses_collection_and_launch`
    (existing).
  - *Proof:* `real_smtp4dev_smtp_to_native_api_round_trip` fails, naming the
    license gap.
- [ ] **T6 Alkemio stays pending (P6).**
  - Rewrite the reason to name: the musl interpreter and `NEEDED`,
    `version.json` 1.14.1, the executable digest `1c7556f3…`, the full index
    digest as the dispatcher's recorded line, and the missing index,
    manifest and layer bytes.
  - Pin no loader, do not use `patchelf`, and do not use Docker.
  - *Proof:* `alkemio_name_is_not_mailslurper` (existing).
  - *Proof:* `real_alkemio_mailslurper_smtp_to_native_api_round_trip` fails,
    naming the reason.
- [ ] **T7 Protocol v3 draft (P7).**
  - Update `docs/evals/protocol.json`'s `unresolved`, the pinned rows'
    `capabilities`, and the storage caps.
  - `status` stays `draft`.
  - Bring `tests/mail_compare/eight-target-smoke.json`'s `storage` and
    `capabilities` into line with the committed protocol.
  - *Proof:* `current_protocol_v3_shape_and_v2_workloads_equal` (existing):
    the workloads and corpus are still equal to v2.
  - *Proof:* a new test, `committed_check_lists_every_open_gap`, runs
    `check` on the committed files and asserts a non-zero exit that lists
    each P7 gap by name.
  - *Proof:* `capability_without_or_with_foreign_citation_refused`
    (existing).
- [ ] **T8 0130 addendum (P3).**
  - Append `## 2026-11-07 addendum (rerun, pre-collection)`, holding:
    - the per-row corrections table: draft value → captured value →
      citation;
    - the confirmed draft facts;
    - the two pending rows;
    - plan.md's operator asks.
  - *Proof:* `git diff 4012f31 -- docs/decisions/0130*` shows `+` lines only.
  - *Proof:* `every_decision_file_has_exactly_one_resolving_index_row`
    (`tests/store_contract_docs.rs`) passes.

## Phase 2 — proofs

- [ ] **T9 Smoke fixture test (P12).** Edit
  `eight_target_smoke_fixture_is_a_valid_draft_that_cannot_launch_yet` in
  place. It must assert that exactly `smtp4dev` and `alkemio-mailslurper`
  are pending and that `mailslurper`, `inbucket` and `greenmail` are pins.
  - *Proof:* the test itself, run with `cargo test --locked --test
    mail_compare`.
- [ ] **T10 GreenMail empty-store precondition (P4).** In
  `real_greenmail_pop3_under_pinned_jre`, before sending, open a POP3 session
  as the workload recipient and assert `STAT` is `+OK 0 …`.
  - *Proof:* the real test passes under the bounded form, with
    `MAIL_COMPARE_GREENMAIL` set to the jar and `MAIL_COMPARE_JRE` set to the
    extracted JRE root.
- [ ] **T11 Per-row cap tests (P9).**
  - Replace `real_caps_hold_at_workload_count` with
    `real_caps_hold_{mailslurper,smtp4dev,inbucket,greenmail,alkemio_mailslurper}`.
    Each sends 600, lists 600 and proves cleanup.
  - Lower no count.
  - *Proof:* the three pinned tests pass, and the two pending tests fail,
    naming their reasons. Each runs as its own bounded command.
- [ ] **T12 Hostile real round trips (P10).**
  - Add `real_{mailslurper,inbucket,greenmail}_hostile_round_trip`. Each one
    sends the hostile set from `hostile_bodies_round_trip` through
    `send_all`, then lists, fetches and decodes every message, asserting that
    the subject, text and HTML equal what was sent.
  - If a test fails while the authored fixtures pass, apply S7.
  - *Proof:* each test passes, or the row is reported under S7 with the sent
    and returned bytes.
- [ ] **T13 Real round trips (P8).** Run
  `real_mailslurper_smtp_to_native_api_round_trip`,
  `real_inbucket_smtp_to_native_api_round_trip` and
  `real_greenmail_pop3_under_pinned_jre`, each `--exact` under the bounded
  form.
  - If a run forces a fix under `benches/`, the fix carries an
    authored-fixture test in `tests/mail_compare/` that fails before it.
  - *Proof:* each passes. The receipt gives command, wall time, exit status
    and loadavg before and after.
- [ ] **T14 README (P12).** Update the v3 pointer section with the pinned
  rows' labels, the POP3 cost label, and the two open rows with their asks.
  - *Proof:* review; `grep` finds no number comparing tools in the section.

## Phase 3 — gates and report

- [ ] **T15 House gates (P13, P14).** Run the full inventory `--locked`
  under `taskset -c "$cpus"` with `CARGO_BUILD_JOBS=$n`:
  - root fmt and clippy;
  - `cargo test --workspace`;
  - `cargo build --bin swarmail`;
  - evals fmt, clippy and tests, with `SWARMAIL_BIN` from Cargo's artifact
    record;
  - `scripts/coverage-gate.sh` and its `selftest`;
  - `scripts/publish-guard-test.sh` and `scripts/seal-branch-test.sh`;
  - `cargo deny check licenses`.
  Then run `git diff c95386e -- Cargo.toml Cargo.lock evals/Cargo.lock
  deny.toml src/`.
  - *Proof:* each command exits 0, and the diff is empty. If `brokkr` is not
    on PATH, report that the bundle compile is verify's.
- [ ] **T16 Report (P11, P15).**
  - The delivery notes give the T0 receipt, every bounded command with its
    loadavg, and the rows pinned and the rows open.
  - They quote the operator asks verbatim.
  - They state that #340 is **not complete**, and that the PR is held for the
    operator (`scripts/`; the 0091 §4 and 0093 §1 amendments through 0130).
  - *Proof:* the result file's notes contain each of these. No result says
    `complete` while a row is pending.
