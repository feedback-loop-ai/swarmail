# 0130 — Eight comparison targets: source lock v4 and protocol v3

- **Status:** proposed
- **Date:** 2026-11-07
- **Issue:** #340, part of #163 (Epic #164)
- **Specification:** [Eight-target mail comparison](../../specs/mail-compare-eight-targets/spec.md)
- **Amends:** accepted [0091](0091-fair-mail-comparison.md) §4's target count ("three actual server binaries", "fixed targets") and accepted [0093](0093-source-mail-comparison.md) §1's annotated ref → tag → commit chain, by addition only, if accepted.
- **Applies without changing:** accepted [0092](0092-official-mail-comparison.md) §1–§6, [0093](0093-source-mail-comparison.md) §2–§7, 0001–0005, 0020, 0045 §8, 0055 (evals outside the coverage denominator), 0056 and the dependency policy.
- **Number:** 0130, reserved by the dispatcher for this story. Only the operator accepts.

## Context

On 2026-11-07 the operator asked to bench throughput and wait latency against
MailSlurper, smtp4dev, Inbucket and GreenMail, as well as Mailpit and MailHog.
Issue #340 states the ruling as "#163 approved and widened". That phrase is the
issue's paraphrase. This record cites the issue and does not quote the operator
directly.

The driver's targets are a closed list of three (`TargetKind::ALL`). 0091 §4
fixes "three actual server binaries", and its synthesis accepts "fixed targets"
as the cost. Source lock v3 (0093 §1) can bind only an annotated tag: a captured
ref that names a tag object, which names a commit. MailSlurper, smtp4dev and
Inbucket tag their releases lightweight, so the ref names the commit directly
and no tag object exists. GreenMail's REST API cannot read messages. GreenMail
also needs a Java runtime, which no current pin can express. The Alkemio
patched MailSlurper is an OCI image with no release and no upstream tag.

Protocol v2 was frozen and collected on 2026-10-30. That collection stays as
recorded (0091 §1, 0092 §2, 0093 §3).

## Proposed ruling

1. **Eight targets, still fixed.** The closed target list grows to eight:
   `swarmail`, `mailpit`, `mailhog`, `mailslurper`, `smtp4dev`, `inbucket`,
   `greenmail` and `alkemio-mailslurper`. Each target has a typed provenance:
   the candidate, an official upstream repository, or a third-party image.
   Target sets are scoped by version. Protocol v2 and lock v3 accept exactly
   the original three. Protocol v3 and lock v4 require all eight. Every
   historical record keeps the meaning it was written with. The list stays
   closed, with no trait, plugin or registry. 0091's "fixed targets" cost
   stands at eight.
2. **Source lock v4** is lock v3 plus fields that only version 4 may carry.
   Version 3 refuses each of them, and version 4 requires those the pin's
   kind needs. No v3 field changes meaning.
   - A tagged pin states `ref_kind`. `annotated` keeps 0093 §1's chain exactly.
     For `lightweight`, the captured ref must name the commit
     (`object.type == "commit"`). The pin carries no tag object, no tag
     signature and no `git_tag` document. A lightweight tag's signature is
     **absent**, not "unsigned": there is no object that could be signed. A
     lightweight pin therefore carries weaker source evidence than an
     annotated one, and the report says so.
   - The release tag is evidence, not a pattern. It must equal the captured
     ref. Its shape is a version from a closed set: `v1.2.3`, `release-1.2.3`
     or `1.2.3`. `latest`, branch names and anything else are refused.
   - An `image` source binds a third-party OCI image. It records the reference,
     index digest, platform manifest digest, layer digest and executable
     member. The index and manifest bytes are captured documents, and their
     sha256 is their digest (content addressing). The executable is extracted
     once into its own empty directory, its digest basis is
     `locally_computed`, and it is rechecked at every launch. An image pin
     never has an observed `tagged_source` or `binary_build_provenance`.
   - A `runtime` pin binds an interpreter that the target needs but that is not
     a target itself. GreenMail's JRE is the only one. The pin records the
     archive digest, the code-bearing members checked at launch (at least the
     `java` launcher and `libjvm.so`), the version output and the license.
   - A launch states its listeners as a closed set of roles: `smtp`, `http`,
     `pop3`, `imap` and `ui`, each at most once. Every listener is a reserved
     loopback port. After readiness, the owned group must listen on **exactly**
     the declared addresses. A listener on an undeclared address, such as a
     tool's fixed default port, refuses the launch. Cleanup proves every
     declared listener closed. A launch may also state an environment (with
     names the launcher owns refused) and a config file rendered into the
     owned scratch.
   - A declared competitor whose bytes are not available may stand as a
     **pending** entry. Such an entry records its byte-independent launch
     facts and a reason. It is valid structure, never launches, and refuses
     collection by name.
3. **Protocol v3** keeps v2's shape, workloads, corpus, budgets, resource
   profile and statistics unchanged. It adds three things: the eight targets;
   a per-target `capabilities` record with fixed keys (`server_search`,
   `native_wait`, `retention_cap`, `raw_source`); and a per-product `scope`
   (`official_artifact`, or `third_party_image` for Alkemio). Each capability
   is `supported` with a citation, `not_supported` with a citation, or
   `not_inspected`. A citation must name a document or reviewed source in that
   target's lock pin. A capability nobody reviewed is `not_inspected` and is
   never assumed.
   - Every row states its storage and cap as launched. The cap is either the
     value set explicitly at launch or `no_cap_observed`. No default retention
     may apply silently: the launch sets the cap, and the freeze-time overflow
     refusal covers it.
   - Protocol v3 has the id `fair-mail-comparison-v3` and supersedes v2 with a
     change reason that cites this record. It is a separate run with a separate
     denominator. No table, statistic or claim merges v2 and v3 cells.
   - Protocol v2 pairs only with lock v3, and protocol v3 only with lock v4.
   - Protocol v3 freezes only when every fact is observed. Until then it is a
     draft with named `unresolved` entries.
4. **Waits** stay on 0093 §4's common client: immediate scan, then a 100 ms
   post-scan poll. No stream client is added.
   - A target without a verified server search is scanned completely and
     filtered on the client by token. The method is labelled per row.
   - GreenMail is read over POP3: a fresh session per scan, no `DELE` and no
     cache between scans. The POP3 client is written in the driver on the tokio
     already in the tree, with no new crate. It honours the request deadline,
     byte bounds and cancellation. IMAP is recorded as not implemented.
5. **Native runs, for their own reason.** Every target runs as a native child
   of the driver, GreenMail on its pinned JRE and the Alkemio row from its
   extracted binary. The reason is evidence, not 0092 §6 as cited. 0092 §6
   rules on lifecycle ownership, not on the choice of runtime. The reason has
   three parts:
   1. Only the driver's own child lets the one owner of 0092 §6 and 0093 §7
      corroborate binds through the process group, reap the leader and observe
      cessation. A container is the Docker daemon's child, so owning it would
      need the second supervisor that 0092's synthesis rejects.
   2. A pinned JRE and a digested jar can be rechecked at every launch. A
      mutable image base cannot.
   3. All v2 rows ran natively, so v3 keeps one runtime shape across rows.
   The cost: the Alkemio row measures the extracted executable, not the image
   as Alkemio runs it, and its label says so.
6. **Licenses.** Measured products keep the bench's permissive list. The JRE is
   neither linked nor measured, but its terms still have to be checkable. Its
   license (OpenJDK: GPL-2.0 with the Classpath exception) is checked against
   a separate closed runtime list that holds exactly that expression. The list
   is named here so that the operator can refuse it. If the operator refuses,
   GreenMail stays pending. `deny.toml` and the dependency allowlist do not
   change. If the Alkemio image's captured terms fall outside the permissive
   list, the row stays pending and is reported. The list is not widened.
7. **Claims.** For any competitor, v3 claims throughput and observed
   visibility within the declared deadline, and nothing more. It claims no
   durability, no accept==stored and no equivalence. The Alkemio row is never
   counted as an official competitor row.

## Synthesis and costs

The simplicity position wins on shape:
- the closed enum with match arms;
- one lock type and one protocol type, gated by version;
- v2's workloads and corpus copied unchanged, so v2 and v3 differ only in
  targets and per-target facts;
- POP3 instead of IMAP;
- no Docker, no stream clients, no MCP adapters for the new tools.

The robustness position loses on two points. Representable but invalid field
combinations are refused by tested validators rather than by the type system.
The measured corpus also gains no hostile edge kind; that fidelity is proved in
decoder tests instead.

The robustness position wins on evidence:
- version-scoped target sets, version pairing, and historical bytes that stay
  launchable;
- the release tag held as evidence;
- the typed third-party provenance;
- N listeners with an exact listening-set proof and per-listener cleanup;
- per-target readiness;
- an async, cancellable POP3 reader;
- caps frozen as launched;
- decoder tests where the gates can see them.

That costs more validators, negative tests and lifecycle tests than the small
design.

## Proof and delivery

The linked spec's acceptance criteria and paired tasks are normative.
Implementation adds this record's index row and dated pointer addenda to 0091
§4 and 0093 §1, with no body edits. It moves the protocol v2 and lock v3 bytes
to `docs/evals/history/` with their sha256 unchanged. It runs every house gate
`--locked`. The operator holds the PR because it changes `scripts/`. No paid
work, collection, push, merge, publication, tag, issue closure or acceptance
is authorized.

## 2026-11-07 addendum (rerun, pre-collection)

This addendum adds to the record above and edits none of it. It cites the
operator's 2026-11-07 direction as the issue states it: the upstream bytes are
in the worktree under `.compare-bins/` (git-ignored, never committed, untrusted
data), every fact comes from them, and no upstream fact is written from memory.
Nothing was collected, no agent session ran and nothing paid ran. Every real
run below was a bounded, unmeasured proof under `taskset` on at most `nproc/4`
(32) CPUs with `timeout`; none is evidence for any table. The corrections here
come before any v3 collection, so 0091 §1 holds: the measured method (workloads
and corpus equal to v2's) is unchanged, and the launch facts were wrong in
places the draft could not have known.

The status stays *proposed*. Protocol v3 stays a *draft*.

### Byte receipt

Each artifact was digested in place before anything read it, then compared with
the corroboration beside it. Archives were listed first (a member that is
absolute or contains `..` refuses; none did), extracted into their own new
directories under `target/compare-dl/`, and read only with `python3 -I`.


| Artifact (worktree path under `.compare-bins/`) | Expected sha256 | Observed sha256 | Verdict |
|---|---|---|---|
| mailpit archive: `2026-11-07/mailpit-linux-amd64.tar.gz` | `4b5993c978e8604c70d43927e2ad103b311bba3b1822fa0cf1d4ee4ec890a93c` | `4b5993c978e8604c70d43927e2ad103b311bba3b1822fa0cf1d4ee4ec890a93c` | match (v3 lock) |
| mailpit executable: `2026-11-07/mailpit` | `262cfa28fae4554be198d1fea5ef83c0f8299317876d658623711c629a8edec8` | `262cfa28fae4554be198d1fea5ef83c0f8299317876d658623711c629a8edec8` | match (v3 lock) |
| mailhog executable: `2026-11-07/MailHog_linux_amd64` | `e2ed634ded49929f089b20045581955ed217672078fd86082dd7a6c67c5d09c7` | `e2ed634ded49929f089b20045581955ed217672078fd86082dd7a6c67c5d09c7` | match (v3 lock) |
| mailslurper zip: `2026-11-07-mailslurper/mailslurper-1.15.0-linux.zip` | `(none)` | `48f2f9ba090f5385301477406b4f6e08f7bfbba0619c635ee591d6e6af3e55f6` | no corroboration published: locally computed (release lists no digest) |
| mailslurper source: `2026-11-07-mailslurper-src/mailslurper-release-1.15.0-src.tar.gz` | `83f66f25ac000fc2f5680514719b7b75a29dafccb7e782597077325fb900359f` | `83f66f25ac000fc2f5680514719b7b75a29dafccb7e782597077325fb900359f` | match (the brief) |
| smtp4dev zip: `2026-11-07-smtp4dev/Rnwood.Smtp4dev-linux-x64-3.16.1.zip` | `3b9395b9ca6e6f5ac0b0a73d953949ed4780a1b794611a97df1a0c7baf909c46` | `3b9395b9ca6e6f5ac0b0a73d953949ed4780a1b794611a97df1a0c7baf909c46` | match (captured release metadata) |
| inbucket archive: `2026-11-07-inbucket/inbucket_3.2.0_linux_amd64.tar.gz` | `0d7d76ed7e66b0700c38aeef7ad1f086e4a159ce554abd92c95052d0838e0e24` | `0d7d76ed7e66b0700c38aeef7ad1f086e4a159ce554abd92c95052d0838e0e24` | match (inbucket_3.2.0_checksums.txt) |
| greenmail jar: `2026-11-07-greenmail/greenmail-standalone-2.1.15.jar` | `91d58798698cac357f015365240383b936ced2f2a1a9f7db37d9e3e1a31391b1` | `91d58798698cac357f015365240383b936ced2f2a1a9f7db37d9e3e1a31391b1` | match (.sha256 file beside it) |
| temurin jre archive: `2026-11-07-jre/OpenJDK21U-jre_x64_linux_hotspot_21.0.13_9.tar.gz` | `a535a1df9c6284d9618872984e026ae0352d1f7e9e0605425c0e6114f47351dd` | `a535a1df9c6284d9618872984e026ae0352d1f7e9e0605425c0e6114f47351dd` | match (SHA256SUMS) |
| alkemio executable: `2026-11-07-alkemio-mailslurper/root/mailslurper` | `1c7556f3fd8da6fb6d2f5a66a1cff258c805407028aeb9eda0a2656f46db6379` | `1c7556f3fd8da6fb6d2f5a66a1cff258c805407028aeb9eda0a2656f46db6379` | match (SHA256SUMS) |


The MailSlurper zip has no published digest, so its pin could only be
`locally_computed`. The Alkemio index digest
`sha256:55029f542c19d0fe570450dc5a30f54dbdb24d9113a6e0e1bf96da000ebd0838` is
the last line of the dispatcher's `SHA256SUMS`, a recorded line: no index,
manifest or layer bytes were supplied, so it was never verified here.

### What the bytes decided

| Row | Outcome | Why, from the bytes |
|---|---|---|
| mailpit, mailhog | unchanged | digests equal the v3 lock; their documents stay byte-unchanged |
| inbucket | **pinned** | real round trip, hostile bodies and the 600-message cap pass against the pinned executable |
| mailslurper | **pending** | the supplied executable's SQLite storage is a `CGO_ENABLED=0` stub |
| smtp4dev | **pending** | the release archive carries no license document |
| greenmail | **pending** | the captured release has no assets, so the jar cannot be bound to a release asset |
| alkemio-mailslurper | **pending** | the executable needs a musl loader that no supplied bytes carry |

#### mailslurper (pending)

| Fact | Draft | Captured | Citation |
|---|---|---|---|
| `-config {config_path}` | drafted | confirmed | the executable's `-h`: `-config string  Absolute location of the config.json`, with `-logformat` and `-loglevel` |
| config keys (`wwwAddress`, `serviceAddress`, `smtpAddress`, `dbEngine`, `dbDatabase`, `maxWorkers`, `autoStartBrowser`) | drafted | confirmed | the json tags of `Configuration` in `pkg/mailslurper/Configuration.go` of the source tarball |
| retention cap | "no cap flag is known" | none in the config, only a manual prune route | the same struct; `DELETE /mail` takes a prune code |
| SQLite storage | "unobserved" | **fatal at startup** | real launch, loopback, `timeout 6`: `Binary was compiled with 'CGO_ENABLED=0', go-sqlite3 requires cgo to work. This is a stub` at `CREATE TABLE mailitem`; the source offers SQLite, MySQL and MSSQL only |
| version | not drafted | the executable logs `Starting MailSlurper Server v1.14.1`, and the tagged source's `SERVER_VERSION` is `"1.14.1"` | the same run; `cmd/mailslurper/mailslurper.go` |
| wait | "none to cite" | no wait or push route | every route registration in `setupServicesListener.go` and `setupAdminListener.go` |
| release tag | `release-1.15.0` | confirmed, lightweight | the captured ref names commit `068acffd…` with `object.type` `commit` |

The version report matters to a pin: lock v4 requires the executable's own
version output to name the release tag's version, and this executable reports
1.14.1 under tag `release-1.15.0`. That is another reason the row stays pending
rather than being written with a composed string.

#### smtp4dev (pending)

| Fact | Draft | Captured | Citation |
|---|---|---|---|
| listeners | smtp, http, pop3, imap | smtp, http | `--imapport=` and `--pop3port=` are documented as disabling those servers |
| `--pop3port {pop3_port}`, `--imapport {imap_port}` | drafted | replaced by `--pop3port=`, `--imapport=` | `--help`: `Specify --imapport="" to disable the IMAP server` (and POP3) |
| `--db {data_dir}/smtp4dev.db` | drafted | `--db smtp4dev.db` | `--help`: the path is relative to `XDG_CONFIG_HOME` on non-Windows, which the launcher sets inside the owned scratch |
| `--bindaddress 127.0.0.1` | not drafted | added | `--help`; `appsettings.json` has `AllowRemoteConnections` true |
| `--urls`, `--smtpport`, `--messagestokeep 10000`, `--nousersettings` | drafted | confirmed | `--help`; `--messagestokeep` is per mailbox |
| default retention | "unread" | 100 | `appsettings.json` (sha256 `40518101f5a3e58273e24fa767e606717c2a1827f5ae2400f256724a80953478`): `NumberOfMessagesToKeep` 100, `Port` 25, `ImapPort` 143, `Pop3Port` 110, `Urls` `http://localhost:5000` |
| license | "unobserved" | **none** | 49 archive members, none a LICENSE, NOTICE or COPYING; the release metadata names none |
| release tag | `3.16.1` | confirmed, lightweight | the captured ref names commit `9fac83c1…`, `object.type` `commit` |

The executable was run only for `--help` and `--version`, under `timeout 60`
and `taskset`, with `DOTNET_BUNDLE_EXTRACT_BASE_DIR` set to a fresh directory
because the single-file bundle extracts itself there. It was never started as a
server. `--help` (output sha256 `50f97ab9b16e2a40e00d5ddd96cc261f367c33e2cbe70357b604920980f10a8e`) reads,
for the flags the launch uses:

```
      --smtpport=VALUE       Set the port the SMTP server listens on. Specify 0
                               to assign automatically
      --db=VALUE             Specifies the path where the database will be
                               stored relative to APPDATA env var on Windows or
                               XDG_CONFIG_HOME on non-Windows. Specify "" to
                               use an in memory database.
      --messagestokeep=VALUE Specifies the number of messages to keep per
                               mailbox
      --imapport=VALUE       Specifies the port the IMAP server will listen on -
                                allows standard email clients to view/retrieve
                               messages. Specify --imapport="" to disable the
                               IMAP server.
      --pop3port=VALUE       Specifies the port the POP3 server will listen on -
                                allows standard email clients to retrieve
                               messages. Specify --pop3port="" to disable the
                               POP3 server.
      --nousersettings       Skip loading of appsetttings.json file in %APPDATA%
      --urls=VALUE           The URLs the web interface should listen on. For
                               example, http://localhost:123. ...
```

`--version` is not a flag of this executable (`[FTL] Unrecognised command line
arguments: --version`); the banner reads `version 3.16.1+9fac83c10cb7cb58105bc0d9d891d36150b8605f`.


#### inbucket (pinned)

| Fact | Draft | Captured | Citation |
|---|---|---|---|
| `INBUCKET_SMTP_ADDR`, `_WEB_ADDR`, `_POP3_ADDR`, `_STORAGE_TYPE=memory`, `_STORAGE_MAILBOXMSGCAP=10000` | drafted | all confirmed | `inbucket -help`, the table of environment variables (`sources/inbucket/inbucket-help.txt`) |
| default cap | "unread" | 500 per mailbox | `INBUCKET_STORAGE_MAILBOXMSGCAP  500  Maximum messages per mailbox` |
| other defaults the launch moves or leaves | not drafted | SMTP `0.0.0.0:2500`, POP3 `0.0.0.0:1100`, web `0.0.0.0:9000`, retention period `24h` (left as is) | the same table |
| version | not drafted | `inbucket -version` prints `3.2.0` | captured output |
| listeners | smtp, http, pop3 | confirmed by the real launch | the exact-listening-set check passed and cleanup proved all three closed |
| release tag | `v3.2.0` | confirmed, lightweight | the captured ref names commit `74224969…`, `object.type` `commit` |

The real launch ran with the owned scratch as its working directory, where the
default `INBUCKET_WEB_UIDIR` (`ui/dist`) does not exist, and the server started
and served its REST API. The release metadata publishes the archive's sha256, so
the digest basis is `upstream_release_metadata`, and the archive's
`inbucket_3.2.0_checksums.txt` agrees.

Three limits of the lock vocabulary showed here, each recorded in the pin and
none relaxed:

1. `artifact.member` accepts a leaf name only, so it holds `inbucket`, the leaf
   of the archive member `inbucket_3.2.0_linux_amd64/inbucket`.
2. The captured release metadata is a five-release listing (an array). The
   validator reads one release object, so the pin holds the listing's first
   element byte for byte (its bytes 1 to 34087), with an `Unresolved`
   attribution that names the listing's file, sha256 and byte range. The request
   that returned the listing was not recorded, so no API URL is attributed.
3. `tagged_source` is `Unresolved`: the ref shows a lightweight tag, and the
   lightweight chain needs a `git_tree` that was not captured. Every file
   document is therefore a member of the digested archive, attributed by member
   path and archive digest.

#### greenmail (pending)

| Fact | Draft | Captured | Citation |
|---|---|---|---|
| release tag | `2.1.15` | `release-2.1.15`, **annotated** | the captured ref `refs/tags/release-2.1.15`: `object.type` `tag`, tag object `5f09fccc06da96fdc7f4d9e1abafb43ceeae61db`; no `git_tag` was captured |
| listeners | smtp, pop3 | smtp, pop3, http | the jar's `GreenMailApiServerBuilder` names `greenmail.api.hostname` and `greenmail.api.port`; the REST listener binds (bounded run, `ss -ltn`) |
| `-Dgreenmail.setup.smtp`, `-Dgreenmail.setup.pop3` | drafted | **removed** | with the port properties the jar refuses to start: `Server 'smtp' was found at least twice in setup config`; the jar's usage example sets the port properties alone |
| `-Dgreenmail.<protocol>.hostname` and `.port` | drafted | confirmed | the jar's `GreenMailStandaloneRunner` usage text |
| `-Dgreenmail.auth.disabled` | drafted | confirmed | the same text; the real POP3 login as `rcpt@mail-compare.invalid` succeeds on an empty store and reads `STAT +OK 0` before any delivery |
| `-jar {artifact}` under a pinned JRE | drafted | confirmed | the jar's `META-INF/maven/com.icegreen/greenmail-standalone/pom.xml` names `GreenMailStandaloneRunner` as the main class |
| the jar's origin | not drafted | **absent from the bytes** | the captured release has `assets: []`, and no supplied document records where `greenmail-standalone-2.1.15.jar` was fetched |
| license | "uncaptured" | Apache 2.0 stated | `greenmail-openapi.yml` `info.license` (the jar also carries `META-INF/NOTICE.markdown`); nothing is pinned |

Lock v4 requires an official pin's `artifact.download` to equal the
`releases/download/<tag>/<file>` URL of an asset the captured release metadata
lists, and neither exists for this jar. Writing a release-asset URL for it would
be an upstream fact from memory. So the row stays pending, and the proof below
is labelled as a driver proof: the entry's own launch run against the
digest-checked jar under the digest-checked JRE, never a pin and never evidence.

The pinned-JRE facts, for whoever pins it later: Temurin `21.0.13+9` from the
archive in the receipt; `bin/java` sha256
`7afc28801f744050aeba646b20e7314265279f76f9d0273b21a2b66411e1bf6d` and
`lib/server/libjvm.so` sha256
`032ff9adc105a8a6f34c43ad4aab90746f8e1d55f6d7ebc9cc80007273b9992d`, relative to the extracted
`jdk-21.0.13+9-jre/`; `bin/java -version` prints
`OpenJDK Runtime Environment Temurin-21.0.13+9 (build 21.0.13+9-LTS)`. Stop
trigger S5 was evaluated against the runtime list of section 6: the extracted
`legal/java.base/LICENSE` (sha256
`4b9abebc4338048a7c2dc184e9f800deb349366bdf28eb23c2677a77b4c87726`) is the GNU GPL version 2 text and
carries the `"CLASSPATH" EXCEPTION TO THE GPL`, so its text supports the
expression `GPL-2.0-only WITH Classpath-exception-2.0`. Neither
`RUNTIME_LICENSES` nor `PERMISSIVE` changed, and whether that expression is
permitted is still the operator's ruling.

A launch finding made the proof possible. GreenMail answers POP3 before its REST
API binds, and the launcher checked each declared bind once, right after
readiness, so it refused the launch with `http bind is absent`.
`targets::bind_within` now waits out only an *absent* declared bind until the
startup deadline and answers any holder, foreign or not, at once. An authored
fixture (`a_declared_bind_that_arrives_after_readiness_is_waited_for`) fails
without it. The exact-listening-set check is unchanged.

#### alkemio-mailslurper (pending)

| Fact | Draft | Captured | Citation |
|---|---|---|---|
| executable | "uncaptured" | sha256 `1c7556f3fd8da6fb6d2f5a66a1cff258c805407028aeb9eda0a2656f46db6379` | `SHA256SUMS`; matches |
| runnable natively | assumed | **no**: interpreter `/lib/ld-musl-x86_64.so.1`, `NEEDED libc.musl-x86_64.so.1` | `readelf -l` and `-d`; neither the directory nor this host has the loader |
| version | not drafted | 1.14.1 | `root/version.json` |
| config | drafted plain-text MailSlurper config | **removed** | `root/config.json` binds `0.0.0.0` ports 4436, 4437 and 1025 and names `keyFile` and `certFile` PEMs |
| index digest | `sha256:55029f54…` by prefix | the full digest is the dispatcher's recorded line | `SHA256SUMS`, last line; never verified |

No musl loader is borrowed, pinned or downloaded, the binary is not patched, and
Docker is not used. The executable was never run.

### Asks of the operator (the rows this rerun could not finish)

1. **MailSlurper.** The supplied Linux executable cannot store mail. Supply a
   cgo build of `release-1.15.0` (or rule how one may be built from the pinned
   source tarball), or a database server a launch may use. Rule whether a pin
   whose executable reports 1.14.1 under tag `release-1.15.0` is acceptable.
2. **smtp4dev.** Supply the license document of `rnwood/smtp4dev` at commit
   `9fac83c10cb7cb58105bc0d9d891d36150b8605f` (the file, or a `git_tree` of that
   commit with the blob). Nothing else blocks the pin.
3. **GreenMail.** Supply where the jar was fetched (and the file's publisher
   record), and rule how lock v4 may bind an official artifact that the GitHub
   release does not list (today `validate_common` requires a listed release
   asset). Rule the runtime license list of section 6, whose expression the JRE
   text supports.
4. **Alkemio.** Supply the OCI index bytes for
   `sha256:55029f542c19d0fe570450dc5a30f54dbdb24d9113a6e0e1bf96da000ebd0838`, the
   linux/amd64 manifest and the layer holding `/lib/ld-musl-x86_64.so.1`, and
   rule whether a musl loader may be pinned as a second runtime (section 2
   allows only GreenMail's JRE) and how the TLS-enabled config is to be run.
5. **All new official rows.** `git_tree` captures (and GreenMail's `git_tag`)
   would turn each `tagged_source` gap into an observation with no code change.
   A single-release capture for each official row would replace the byte-slice
   of the listing, and the vocabulary has no field for an archive member path
   below the root.

Collection stays refused: `mail_compare check` on the committed protocol and
lock exits non-zero and lists each gap above.
