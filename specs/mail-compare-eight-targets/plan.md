# Plan — eight-target mail comparison (#340)

Spec: [spec.md](spec.md). Decision: [0130](../../docs/decisions/0130-eight-target-mail-comparison.md) (proposed).
This is a first design, not a return from review: there is no
`context.returned_from`.

## Summary

Grow the closed `TargetKind` to eight and keep every record versioned honestly:

- Lock v4 sits beside lock v3 in one gated struct.
- Protocol v3 sits beside protocol v2 in the existing gated struct.
- Lock v3 and protocol v2 stay readable, launchable and byte-identical in
  `docs/evals/history/`.

The work stays inside the owners that already exist. It adds no trait, no new
`.rs` file under `benches/`, no crate, no Docker and no stream client. Inside
those owners it widens three things:

- the launch, so that it owns N listeners exactly;
- readiness, so that each target has its own probe;
- reading, with a POP3 client in `wire.rs` for GreenMail.

## Technical context

- **Language and crate.** Rust, one crate. The bench target `mail_compare`
  has `harness = false`. Its modules are compiled into `tests/mail_compare.rs`
  through `#[path]`.
- **Dependencies.** Only what is already in the tree: tokio (`net`, `io`,
  `process`, `time`, `test-util` in dev), serde/serde_json, `mail_parser`,
  ring through rustls for sha256, libc, and the existing in-file SHA-1.
  **No new dependency.** `Cargo.toml`, `Cargo.lock`, `evals/Cargo.lock` and
  `deny.toml` stay untouched.
- **Coverage.** `benches/**` is outside the 100% denominator, and `src/` is
  not touched. The coverage gate therefore stays at its floor, and the tests
  in `tests/mail_compare.rs` are what police the driver.
- **Runtime facts.** The bytes are expected under the dispatcher's
  `swarmail-landing/compare-bins/2026-11-07*`. Neither design seat could see
  that path; `..` holds only this worktree. S6 governs if implement cannot see
  it either.
- **Host.** At framing, `nproc` was 128, so the load bound is 32 CPUs. Take
  `nproc` at run time.

## Constitution check

- **Frozen guarantees 0001–0003 and 0004/0005.** Not touched, and nothing
  changes under `src/`. They matter only as the meaning of Swarmail's own row.
  No competitor row is described in their terms; 0130 §7 limits claims to
  throughput and observed visibility.
- **0091 §1.** The method is complete before collection, and v3 is a separate
  run. **§4:** the count grows from three to eight through 0130, by addition
  only.
- **0092 §2/§3/§4/§6.** Profiles are labelled, the denominator is kept,
  decoders use independent bodies, and one owner remains.
- **0093 §1.** Extended by 0130 §2 for lightweight, image and runtime pins.
  **§4:** the 100 ms common poll is unchanged. **§6/§7:** scan and lifecycle
  rules are unchanged.
- **0020.** Tests live in `tests/mail_compare.rs` because the benches have no
  inline modules. Seams are `#[cfg(test)]` only. Chaos happens only at the
  poles, and nothing sleeps as synchronization.
- **0045 §8.** Is the work compatible with accepted rulings while 0130 is
  proposed? The target-count widening and the lightweight chain amend accepted
  0091 §4 and 0093 §1, so the code may be built but the PR is held for the
  operator. It is also held because it changes `scripts/compare-mail.sh`
  (house rules, *Commits and history*). Say both in the delivery notes.
- **0056.** Unsigned commits, never pushed.

## How the positions were reconciled

Each ruling below says who wins and what the losing side costs.

| # | Question | Ruling | Cost to the losing side |
|---|---|---|---|
| 1 | Lock v4 as a new type (robustness) or one gated struct (simplicity) | **Simplicity on the shape, robustness on the gates.** One `SourceLock` struct with v4 fields that must be absent at v3 and required at v4. The new facts use typed enums (`RefKind`, `PinSource`) rather than loose strings. Robustness's risks are each closed by a test: the version set is `{3,4}`, so lock v3 is not killed; a total gate refuses v3 bytes read as v4; protocol↔lock pairing is enforced; the history v3 lock is proved launchable. This follows the protocol record's own v1/v2 precedent of gated sections. | Robustness: invalid field combinations are representable and refused by the validator rather than by the compiler. Every gate needs its own negative test (AC3). |
| 2 | Release tag | **Robustness.** The tag must equal the captured ref. Its shape is checked against a closed list, `v`, `release-` or bare dotted numerics, and `is_release_tag` is not loosened. | Simplicity: one more validator. |
| 3 | Third-party image | **Robustness.** A typed `provenance()` and pin `source: image`, plus **simplicity's** per-product `scope` in the protocol. Both have to agree. | None that matters: the change is small and closes a hole that only convention was holding shut. |
| 4 | Listeners: one aux slot (simplicity) or N roles (robustness) | **Robustness, bounded.** The roles are a closed set of five, each used at most once. Added to that is a check neither position named in full: after readiness, the group's listening set must equal the declared set. That catches a tool's fixed default port (S4), which the one-aux-slot design could not. | Simplicity: a larger launch and cleanup surface, plus a `/proc` listening-set read. |
| 5 | Readiness | **Robustness.** Each target has its own probe: HTTP `200`, or for GreenMail the POP3 `+OK`. GreenMail's REST endpoint is not probed, because it is not part of the read path. | — |
| 6 | POP3 on std (simplicity, framing) or tokio (robustness) | **Robustness on I/O, simplicity on placement.** The client is written on tokio in `wire.rs`. Blocking `std::net` would stall the runtime and defeat the cleanup deadline (0093 §7). The issue and framing ask for "std only, no new crate"; tokio is already in the tree, so "no new crate" holds. | Simplicity: none. The framing's literal "std" is read as "nothing beyond the existing tree", and that reading is stated here. |
| 7 | IMAP | **Both: no IMAP.** It is recorded as not implemented. | GreenMail is read over one path only. |
| 8 | Hostile corpus in v3 (robustness) | **Simplicity on the measured corpus.** v3 copies v2's corpus and workloads, so the two differ only in targets and per-target facts, and the operator asked for throughput and wait. **Robustness on tests:** hostile bodies (dot-stuffing, an 8-bit subject, a 998-octet header, the boundary string in the body) are decoder and POP3 tests (AC10/AC11). | Robustness: a tool that mangles edge-case bodies is not caught *in measurement*. A fidelity method would be a later protocol version. |
| 9 | Caps | **Robustness.** Every cap is set at launch, and defaults are cited and overridden. That includes smtp4dev's default retention and Inbucket's per-mailbox cap. The existing overflow refusal holds them at freeze time. | — |
| 10 | Backend vocabulary | **Simplicity.** `scratch_file` covers SQLite, file and maildir stores in the owned scratch; Mailpit v2 already uses it for SQLite. The engine name goes in `compression` and the basis text. | Robustness: the enum alone does not tell a reader which engine is behind a row. |
| 11 | Decoder placement and tests | **Simplicity** places decoders as match arms in `targets.rs`, with page decoding lifted to a free `decode_page(kind, &Value, start)`. **Robustness** places the tests in `tests/mail_compare.rs`, with negative cases. | — |
| 12 | Unpacked trees (JRE, smtp4dev) | **A middle path, because neither position was right for the JRE.** Simplicity would recheck only the launched executable, but for a JVM that is a launcher stub, while robustness was silent. A `checked_members` list (at most 8) is rechecked at launch: the JRE's `bin/java` and `lib/server/libjvm.so` plus the jar, and smtp4dev's executable plus its main assembly if the release is not single-file. Other siblings are labelled as not rechecked. | Neither side gets a full tree digest. |
| 13 | Re-checking the captured ref at finalization (robustness) | **Rejected.** That would need network access at seal time. The lock binds captured bytes, and an upstream tag moved later means a new lock. Finalization already rechecks the executable digests. | Robustness: a moved upstream tag is not detected inside a run. |
| 14 | Refusing profiles whose `sessions` exceed host CPUs (robustness) | **Rejected.** Sessions are async connections, not CPU workers. The CPU bound is `taskset` affinity, which the JVM and .NET runtimes also honour. | — |
| 15 | Native-run reason | **Merged.** Simplicity: one owner with no second supervisor. Robustness: a pinned JRE and jar are more verifiable than a mutable base image. Added: all v2 rows ran natively. See 0130 §5. Neither position cites it to 0092 §6. | — |
| 16 | Licenses | **A new ruling for the operator (0130 §6).** It adds a separate runtime license list holding only the JRE's expression. `PERMISSIVE` and `deny.toml` stay unchanged. If the Alkemio license falls outside them, that is S5. | It is a proposal the operator may refuse, in which case GreenMail stays pending. |
| 17 | Bytes not visible here | **Simplicity's draft path, made expressible.** A v4 *pending* entry carries the facts that do not depend on bytes: tag, listeners, launch, env, config and a reason. It never launches, and collection refuses it. Protocol v3 stays `draft` with named `unresolved`. | None. It is the honest shape. |

## Project structure (files touched by implementation)

```
benches/mail_compare/protocol.rs   TargetKind×8, provenance(), V2 set; RefKind,
                                   PinSource{release,image}, Runtime, Listener,
                                   LaunchEnv, ConfigFile, Pending; lock v3/v4
                                   gate; protocol v3 sections (capabilities,
                                   product scope); pairing; v4 tag shape
benches/mail_compare/targets.rs    N-listener reservation/substitution, exact
                                   listening set, per-target ready(), arms for 5
                                   targets, decode_page(), checked members,
                                   config-file/env rendering, cleanup per listener
benches/mail_compare/wire.rs       Pop3 client (tokio), dot-unstuffing, bounds
benches/mail_compare/runner.rs     LaunchSpec from either lock version; client-side
                                   find when server_search ≠ supported
benches/mail_compare/report.rs     refuse merged v2/v3 input; row labels
benches/mail_compare.rs            `targets` subcommand (declared competitors)
scripts/compare-mail.sh            --bin NAME=PATH, protocol-derived requirements
tests/mail_compare.rs              every AC test (+ #[ignore] real tests)
tests/mail_compare/eight-target-smoke.json   v3 draft smoke protocol
tests/mail_compare/pop3/…, captures/…        authored fixtures (not decoder output)
docs/evals/protocol.json           v3
docs/evals/sources.lock.json       v4
docs/evals/history/<date>-protocol.v2.json, <date>-sources.lock.v3.json
docs/evals/sources/{mailslurper,smtp4dev,inbucket,greenmail,alkemio-mailslurper}/
docs/evals/README.md               v3/lock v4 pointer section
docs/decisions/README.md           0130 row
docs/decisions/0091-…, 0093-…      dated pointer addenda (additions only)
```

Name each history copy by the date its bytes were first committed, using `git
log --diff-filter=A --format=%as -- docs/evals/protocol.json` and the
equivalent for the lock. The expected dates are 2026-10-29 or 2026-10-30.
Verify them; do not guess.

## Research (facts the implementer establishes from captured bytes, never assumes)

| Tool | Read from captures and executable | Expected shape (to confirm) |
|---|---|---|
| MailSlurper `release-1.15.0` | Config schema (`config.json`), the service API's listing and paging fields, raw-source endpoint, SQLite settings, whether any search or wait exists | Three listeners: `smtp`, `http` (service API), `ui` (www). Config file is rendered into scratch. SQLite at `{data_dir}`. `native_wait: not_supported` only if a reviewed source shows none, otherwise `not_inspected`. No cap means `no_cap_observed`. |
| smtp4dev `3.16.1` | `--help` of the pinned binary, captured `appsettings.json` defaults (MessagesToKeep), the paged REST listing fields, the source endpoint, the IMAP/POP3 listener flags | Database in scratch, or in memory if the flags allow. The cap is set explicitly above the workload. IMAP and POP3 are disabled if a flag allows, otherwise bound to reserved ports (`imap`, `pop3`). |
| Inbucket `v3.2.0` | Env-var configuration (`INBUCKET_*`), the per-mailbox REST API (list, source), the default `mailboxmsgcap`, the storage type | Configured through `launch_env` only. One mailbox, `rcpt` (from `wire::TO`). Memory store with the cap raised. POP3 on a reserved `pop3` port. The listing has no total, so it is a single unpaged response, complete only by quiescent reconciliation (0093 §6). |
| GreenMail `2.1.15` | The standalone jar's system properties (ports, host, auth, which services start), the captured API document backing "REST cannot read messages" (S5), the minimum JRE | `java -D… -jar {artifact}`, with SMTP and POP3 only on reserved ports and the API off if a property allows (else `http`). Memory store. POP3 login as `rcpt@mail-compare.invalid`. |
| Alkemio image | The index and manifest bytes, the platform, the layer and the member path, the config and env the patched binary reads, the license terms in the image | The same config family as MailSlurper is likely, but not assumed. Its own template. |
| JRE | Vendor, version, archive digest basis, member digests, license text | The pinned path is digest-checked, and its affinity is inherited from `taskset`. |

Every endpoint path a decoder uses must be cited to a captured document. Where
an executable disagrees with its document, the executable wins and the README
lists the disagreement, as the existing rule says.

## Data model (additions)

- `TargetKind`: adds `Mailslurper`, `Smtp4dev`, `Inbucket`, `Greenmail` and
  `AlkemioMailslurper` (`#[serde(rename = "alkemio-mailslurper")]`), plus the
  sets `V2: [_; 3]` and `ALL: [_; 8]`. `provenance()` returns
  `Candidate | Official(&str) | ThirdPartyImage(&str)`.
- Lock v4 pin (all new fields are `Option` in serde and gated by version in
  `validate`):
  - `source`: `release` (the existing `Release`) or `image` (`reference`,
    `index_digest`, `manifest_digest`, `platform`, `layer_digest`, `member`).
    At v3 it is the bare `release`, as today.
  - `tagged_source`: adds `ref_kind: annotated|lightweight`. `tag_object` and
    `tag_signature` are required for annotated and refused for lightweight.
  - `listeners`: a set of `smtp|http|pop3|imap|ui`. `smtp` is always present,
    with exactly one read role (`http` or `pop3`).
  - `launch_env: [[NAME, value]]` and `config_file: {name, lines}`.
  - `runtime: {name, download, sha256, digest_basis, checked_members[{path,sha256}], version_output, license}`.
  - `artifact.checked_members` (at most 8).
  - `DocKind` adds `ImageIndex` and `ImageManifest`.
- Lock v4 entry: `pin: Some(..)`, or `pending: {release_tag, listeners, launch,
  launch_env, config_file, reason}`.
- Protocol v3:
  - `capabilities: [{target, server_search, native_wait, retention_cap,
    raw_source}]`. Each value is `{status: supported|not_supported|not_inspected,
    cite?}`, and `cite` names a pin document path or reviewed-source path.
  - `products[].scope: official_artifact|third_party_image`.
- Launch slots: `{<role>_addr}` and `{<role>_port}` for each declared role,
  plus `{data_dir}`, `{scratch}` and `{artifact}`. At most 32 argv entries and
  a closed slot set.
- `CleanupFacts`: adds `listeners: [{role, closed}]`. `listeners_closed` is the
  conjunction of the per-listener results. Its serialized name is unchanged.
- `LaunchError`: adds `UndeclaredListener` and `MemberDigestMismatch`.

## Contracts

- `mail_compare check --protocol P --sources S` lists every refusal.
  Version pairing is checked first.
- `mail_compare targets --protocol P` is new. It prints one declared
  competitor name per line, for the wrapper.
- `mail_compare run … --bin NAME=PATH` is unchanged, and accepts all eight
  names.
- The flags `compare-mail.sh local|collect --bin NAME=PATH`, `--mailpit` and
  `--mailhog` stay as aliases. The exit codes keep their meaning.

## Quickstart (free, bounded, labelled smoke — never evidence)

```bash
cap=$(( $(nproc) / 4 )); cpus="0-$(( cap - 1 ))"
cat /proc/loadavg                                  # before
time timeout 600 taskset -c "$cpus" cargo test --locked --test mail_compare
time timeout 600 taskset -c "$cpus" env MAIL_COMPARE_GREENMAIL=… MAIL_COMPARE_JRE=… \
  cargo test --locked --test mail_compare -- --ignored greenmail
time timeout 600 taskset -c "$cpus" scripts/compare-mail.sh local \
  --protocol tests/mail_compare/eight-target-smoke.json --output "$PWD/target/smoke-$(date +%s)" \
  --bin mailpit=… --bin mailhog=… --bin mailslurper=… --bin smtp4dev=… --bin inbucket=… \
  --bin greenmail=… --bin alkemio-mailslurper=…
cat /proc/loadavg                                  # after
```

Each download goes into a fresh `mktemp -d` under `target/compare-dl/`, never
into `/tmp`, which is a RAM tmpfs. Its digest is checked before any use. Any
Python inspection runs as `python3 -I script.py <path>`, with the script kept
outside the download directory.

## Risks

| Risk | Mitigation |
|---|---|
| The v3 lock and v2 protocol stop validating or launching after the gate change | The version set `{3,4}`, a total gate, a pairing check, and AC2's history test running `validate`, `verify_files` and `collection_ready` on the history bytes. No document under `sources/mailpit|mailhog` may change, and a test digests them against the v3 lock. |
| v3 bytes are silently accepted as v4, or the reverse | Every v4 field is refused at v3 and every required one is demanded at v4. The AC3 matrix has one negative test per field. |
| A loosened tag check admits `latest` or a branch | Equality with the captured ref plus a closed shape list. The negative shapes in AC4 cover it. |
| A tool binds a default fixed port, which leaks, collides with parallel runs and still yields a "closed" receipt | The exact listening-set check after readiness. A per-listener cleanup proof. S4 stops the work if a port cannot be moved. |
| Default retention evicts mail mid-run and is recorded as a competitor loss | Caps are set explicitly at launch, the freeze-time overflow refusal holds, the default is cited, and AC12 runs a real no-eviction smoke. |
| The POP3 reader hangs or stalls the runtime | tokio I/O, a per-request deadline, line and message caps, a cancellation test (AC11), and no blocking I/O. |
| POP3 snapshot semantics or the cost per poll make GreenMail look slower or faster unfairly | A fresh session per scan, so each scan is complete. The login cost is measured and labelled. The read path is named in the README row. |
| Client-side filtering on tools with no search is mistaken for server search | `capabilities.server_search` is cited per row, and the README labels it. |
| The Alkemio row is read as official MailSlurper, or is confused with Swarmail's 0118 shim | `ThirdPartyImage` provenance, `scope: third_party_image` with a cross-check, a distinct name and directory, and AC14. |
| A JRE or smtp4dev sibling library is swapped after the digest | `checked_members` are rechecked at launch and at seal. Siblings not rechecked are labelled. Hashes are not same-host authentication (0092 §1). |
| The license falls outside the lists | The runtime list goes into 0130 §6 for the operator to rule on. S5 stops the work. `PERMISSIVE` and `deny.toml` are never widened. |
| Bytes are not visible in implement's sandbox (S6) | Pending entries, a draft v3 with named `unresolved` entries, real tests left open and reported. All validators, decoders, POP3 and lifecycle work still land on authored fixtures. |
| A load spike on the shared host | `taskset` to `nproc/4`, `timeout 600`, loadavg recorded before and after, smoke-sized profiles only, and no collection. |
| A merged v2/v3 report | The report refuses inputs whose protocol ids differ, with a test. |
| Review surface of `tests/mail_compare.rs` (4.3 kloc) | New tests are grouped by AC in clearly named sections, with fixtures in files. Nothing is copied from decoder output. |
| The PR changes `scripts/` and amends accepted 0091 §4 / 0093 §1 | Held for the operator under house rules and 0045 §8, and stated in the delivery notes. Nothing is merged on green. |

## Findings answered

None. This is a first design, not a return from review.
