# 0130 — Eight comparison targets: source lock v4 and protocol v3

- **Status:** accepted (operator, 2026-11-08 — "Accept and merge"; see the dated acceptance note at the end)
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

## 2026-11-07 addendum (operator rulings, pins and freeze)

This addendum adds to the record above and edits none of it. The status stays
*proposed*; only the operator accepts. It records what the operator ruled on
2026-11-07 ("Pragmatic set", the second rerun's brief), the bytes the operator
supplied under `.compare-bins/2026-11-07-supply/` (git-ignored, untrusted,
never committed), what was built from both, and the freeze. Nothing was
collected, no agent session ran and nothing paid ran. It supersedes, by
addition, the rerun addendum's "protocol v3 stays a draft", its four pending
rows and its six asks of the operator.

### The rulings, quoted from the brief

1. **MailSlurper.** "Publish as a result that the official release-1.15.0
   linux executable does not start with its default SQLite storage. It was
   built with CGO_ENABLED=0 and dies with 'Binary was compiled with
   CGO_ENABLED=0, go-sqlite3 requires cgo to work. This is a stub'. It also
   reports v1.14.1, because the release source hard-codes it. Also bench a
   cgo build from the pinned release source, as a separate labelled row 'built
   from source'. It may be pinned even though it reports v1.14.1 under tag
   release-1.15.0; record that fact."
2. **Alkemio's MailSlurper image row.** "A musl loader may be pinned as a
   second runtime, taken from the image's own layer."
3. **GreenMail** "is bound to its Maven Central artifact and checksums. Lock v4
   may bind an official artifact that the GitHub release does not list, when
   its registry publishes checksums."
4. "**0130 §6's runtime licence list** is accepted: JRE GPLv2 with the
   Classpath exception, musl MIT."

Rulings 2 and 4 amend §2 and §6 of this record, by addition: the closed
`RUNTIMES` table holds two runtimes, and `RUNTIME_LICENSES` holds
`"GPL-2.0-only WITH Classpath-exception-2.0"` and `"MIT"`. `PERMISSIVE`,
`deny.toml` and the dependency allowlist are unchanged (a test asserts
`PERMISSIVE`'s exact five entries, and that `deny.toml` does not name the
Classpath exception).

### The supply, and its receipt

`sha256sum -c SHA256SUMS` passed in every supply subdirectory and at the top
level (143 files) before anything read a supplied file. The supply was never
written to or extracted into; each archive was listed first (no absolute or
`..` member), then extracted into its own new directory under
`target/compare-dl/`, and read only with `python3 -I` from a script kept in
`target/compare-tools/`. `git ls-files .compare-bins target` is empty at every
commit.

| Item | sha256 |
|---|---|
| MailSlurper official zip (member `mailslurper`) | `48f2f9ba090f5385301477406b4f6e08f7bfbba0619c635ee591d6e6af3e55f6` (`0cd9416b766deb26eed8aa01ad0873d06725a3464fdb9ac3cc82201e2c232627`) |
| MailSlurper source tarball (pax comment `068acffdcd8960f248ea438c3b7ee51ead601bd2`) | `83f66f25ac000fc2f5680514719b7b75a29dafccb7e782597077325fb900359f` |
| MailSlurper cgo build (go1.18.1, `-trimpath`, verified modules) | `eb6cd97d114f881e6a6f7170ddbe1fe914d8f52f025839631e441807f2a458ff` |
| smtp4dev zip (member `Rnwood.Smtp4dev`) | `3b9395b9ca6e6f5ac0b0a73d953949ed4780a1b794611a97df1a0c7baf909c46` |
| Inbucket archive | `0d7d76ed7e66b0700c38aeef7ad1f086e4a159ce554abd92c95052d0838e0e24` |
| GreenMail jar (Maven Central) and its `.sha256` | `91d58798698cac357f015365240383b936ced2f2a1a9f7db37d9e3e1a31391b1` |
| Temurin 21.0.13+9 JRE archive | `a535a1df9c6284d9618872984e026ae0352d1f7e9e0605425c0e6114f47351dd` |
| Alkemio OCI index / linux-amd64 manifest | `55029f542c19d0fe570450dc5a30f54dbdb24d9113a6e0e1bf96da000ebd0838` / `97dfa5567da8acf1c83c3f11b233078bd0dc831e317d58e97d56461d67ba4e86` |
| Alkemio executable (layer `0dfdf228…`, 13th of 15) | `1c7556f3fd8da6fb6d2f5a66a1cff258c805407028aeb9eda0a2656f46db6379` |
| musl loader `lib/ld-musl-x86_64.so.1` (layer `f350948e60ec3e45e17bc37d4f8a5d1c5c728d634bca374d9dc183fea996e87e`, musl 1.1.24-r10, apk `L:MIT`) | `0273b5344fc0b9f687b95ec987ecea9c2be7ef01bf8c111fb99117bc0e2e91a3` |
| git trees (rebuilt to their ids by the validator): inbucket v3.2.0, smtp4dev 3.16.1, greenmail release-2.1.15 (tag object `5f09fccc…`), mailslurper release-1.15.0 | roots `ee202b29…`, `9622c9bb…`, `bd4125c4…`, `5f29f08a…` |

The loader's layer is the topmost of the manifest's 15 layers that holds
`lib/ld-musl-x86_64.so.1` (layers 0, 3 and 5 hold it; layer 5 is `f350948e…`;
no `.wh.` entry or opaque-directory marker touches `lib/`), and the supplied
`musl/ld-musl-x86_64.so.1` equals that layer's member byte for byte.

### New constants and variants (`benches/mail_compare/protocol.rs`)

| Name | What it is |
|---|---|
| `TargetKind::MailslurperSourceBuild` (`mailslurper-source-build`), `Provenance::SourceBuild`, `ProductScope::SourceBuild` | the ninth target: MailSlurper release-1.15.0 built from source (cgo, host glibc, unpinned runtime), never a relabelled official row, Alkemio's image or Swarmail's shim (decision 0118) |
| `Protocol.does_not_start: [{target, cite}]` | version 3 only; `cite` is a document of that target's pin; the runner never launches a listed target, `plan()` yields it no coordinate, `declared_competitors` omits it, and the report prints "does not start (published result: …)" with every cell `blocked` and no number; the row keeps its pin, product, capabilities and rotation slot |
| `REPORTED_VERSIONS` | closed `(target, tag, version)` list: both MailSlurper rows at `release-1.15.0` report `1.14.1`; data cannot excuse a version check, only this list can |
| `RUNTIMES` | closed table `(target, entry program, checked members, image layer?)`: GreenMail `bin/java` with `bin/java` and `lib/server/libjvm.so`; Alkemio `lib/ld-musl-x86_64.so.1` with that loader, whose archive digest must be a layer of the pin's captured manifest |
| `DigestBasis::RegistryChecksum`, `REGISTRY`, `DocKind::Checksum` | GreenMail's jar at `https://repo1.maven.org/maven2/com/icegreen/greenmail-standalone/2.1.15/…`, with the registry's `.sha256` as a supplied document whose trimmed bytes must equal `upstream_digest`; `release_agrees` skips only the asset-list check |
| `DocKind::GitCommit` and `tree_object_id` | the supplied trees are root trees fetched by tree id: a commit object links the tagged commit to the tree, and the validator rebuilds git's tree object from the listing (`040000` written `40000`) and requires it to equal the tree's own id |
| `Attribution::Member`, `Output`, `Supplied` and `SOURCE_ARCHIVES` | an archive member (the pin's artifact, its runtime or the MailSlurper source tarball bound to its commit), a program's own output, or operator-supplied evidence naming its supply path and carrying its sha256; proved by a real test that re-extracts or re-runs |
| `ConfigFile.cwd` | the child's working directory is the directory its config is rendered in, for a program that reads a fixed relative name (the Alkemio fork reads `./config.json` and has no `-config` flag) |

`Member` and `Output` are checked offline only for shape; their content is
proved by `real_member_and_output_documents_match`, which this delivery ran and
which must pass before any collection (the bench has no archive reader and
none was added). The release-metadata documents are slices of a listing
(byte-for-byte elements of the captured releases arrays); each is attributed
`Api` to the object's own `url`, and `verify_files` checks that self-name.

### The published result: the official MailSlurper release does not start

`real_mailslurper_official_does_not_start` runs the pinned official
executable (`0cd9416b…`, a statically linked, stripped Go binary) as its pin
launches it, on the rendered config. It exits non-zero within the startup
deadline with `Binary was compiled with 'CGO_ENABLED=0', go-sqlite3 requires
cgo to work. This is a stub`, at `CREATE TABLE mailitem`; no declared listener
is ever bound; and the launcher's own path refuses it with cleanup proved. The
cause is in the binary itself: its embedded build info (the supply's
`official-release-run/embedded-buildinfo.txt`, read with `strings`) records
go1.20.6, `CGO_ENABLED=0`, `-ldflags="-s -w"` and `vcs.modified=true` at
commit `068acffd…`. It logs `Starting MailSlurper Server v1.14.1` because the
release source hard-codes `SERVER_VERSION = "1.14.1"`. Both documents are
retained under `sources/mailslurper/` as supplied evidence, and the protocol's
`does_not_start` entry cites the failure output. Reading it as shrinkage of
the denominator under 0091 §1 would be wrong: §1 ("preserve … unstarted work")
and §3 ("full-product entries without fabricated trial results") govern, the
row keeps its pin, product, capabilities, rotation slot and a full cell per
profile, and a disagreement is the operator's to rule.

### What the real runs showed (unmeasured, bounded)

Every launched row passed a round trip, a hostile set and a 600-message cap
test against its real pinned bytes, first as untimed exploratory probes before
the freeze and again, recorded, against the frozen bytes. The probes corrected
the draft before anything froze (decision 0091 §1: no collection had run):

- **MailSlurper's listing count is `totalRecords`, not `totalMailItems`**
  (`{"mailItems":[…],"totalPages":1,"totalRecords":1}`), and its ids are uuids.
- **A path segment must keep `-._~`.** `wire::query_value` percent-encoded `-`
  (`NON_ALPHANUMERIC`), so `/mail/feb8d58f%2D…` returned an empty item; the
  encoding now keeps RFC 3986's unreserved characters, as its comment always
  said. The same helper encodes query values, which every server decodes.
- **smtp4dev pages with `page`, not `pageNumber`** (its API XML names the
  parameter; the real server ignored the wrong one and returned page 1 for
  every page). The decoder's echoed `currentPage` check caught it as
  `Malformed`, never as a shorter success.
- **MailSlurper keeps one decoded body and no raw message.** It stores the HTML
  part when there is one, else the plain text with each newline turned into
  `<br />` (`DataCommandExecutor.decodeBody` in the pinned source); a multipart
  message's plain part is dropped; and it does not undo SMTP dot-stuffing
  (`.leading dot` comes back `..leading dot`). Their body fidelity is therefore
  not exact by construction, and a collection reports it per message through
  `fidelity_ok`; the real tests assert subject, listing and the contract each
  keeps (an HTML message's HTML as sent, a plain message's text with only
  `<br />` added), and print the rest.
- **MailSlurper acknowledges a mail before it stores it and loses some under
  SQLite lock contention.** In a manual probe (not a measurement) 600 mails
  sent in 0.23 s were all answered `250`, and the store settled at 573: its log
  holds 27 `database is locked` errors from `StoreMail`. A 600-message burst in
  the cap test listed 596. The cap proof therefore sends in batches of five and
  waits for each to be listed, so only eviction could lower the count; the
  burst loss is an observation about the tool, and a collection will show it as
  accepted-but-missing mail in the cell. The Alkemio fork carries a
  single-connection SQLite patch and was not probed for the same burst.
- **Alkemio's binary runs on the pinned loader and serves plain SMTP on the
  rendered config** (no certificate: S9 did not trigger), reading `./config.json`
  from its working directory, hence `ConfigFile.cwd`.
- **smtp4dev's `--db ""` selects an in-memory database** (its `--help`); the
  pinned launch keeps the scratch file the draft chose, so the row is stored
  as `scratch_file`. The row's version output is its own banner
  `3.16.1+9fac83c1…`, a self-report, not build evidence.

Two facts the issue states differ from the pinned bytes, flagged for the
operator, not decided here. **GreenMail's REST API can read messages**: the
pinned jar's `greenmail-openapi.yml` lists `GET
/api/user/{emailOrId}/messages/{folderName}` returning `uid`, `subject`,
`contentType` and `mimeMessage`, while the issue says it "cannot read
messages". The driver still reads over POP3, as the issue orders, and the
capability record says `raw_source` is supported, citing that document.
**GreenMail's own license text is not in the supplied bytes**: the tagged tree
lists `license.txt` (blob `680cd03b…`, 11385 bytes) but no bytes of it were
supplied, the jar carries none (its `META-INF/NOTICE` is its dependencies'), and
its poms state no license. The pin's `License` document is therefore the jar's
`greenmail-openapi.yml` (`info.license: Apache 2.0` and the URL), a member of
the digested jar retained twice under two paths (`greenmail-license-declaration.yml`
and `greenmail-openapi.yml`), and the lock records `Apache-2.0`. That is a
declaration, not the license text; replacing it with the blob when its bytes
are supplied changes only the lock's License row.

### The registry label

GreenMail's pin is integrity against the registry's own checksum; authenticity
is not verified. The `.asc` stays in the supply, no key is pinned and no gpg
ran, so nothing authenticates the publisher: the registry serves both the jar
and its `.sha256`, and the claim, the report's row label and the README say so.
GreenMail's chain is annotated and observed: ref → tag object `5f09fccc…` →
commit `3b8ea7df…`, with `tag_signature` the captured `verification.reason`
(GitHub's field; no signature was verified here).

### The nine targets and their labels

| Row | Source | Runtime | Read path | Pin |
|---|---|---|---|---|
| swarmail | candidate | — | HTTP JSON | the checkout |
| mailpit v1.32.0, mailhog v1.0.1 | official, annotated tag | — | HTTP JSON | unchanged bytes |
| mailslurper release-1.15.0 | official, lightweight tag, **does not start** | — | — | zip `48f2f9ba…`, member `0cd9416b…` |
| mailslurper-source-build | built from source (cgo; host glibc, unpinned), lightweight tag | host glibc, unpinned | HTTP JSON | executable `eb6cd97d…`, build record supplied |
| smtp4dev 3.16.1 | official, lightweight tag | .NET bundle on host libraries | HTTP JSON (paged) | zip `3b9395b9…` |
| inbucket v3.2.0 | official, lightweight tag | — | HTTP JSON (per mailbox) | archive `0d7d76ed…` |
| greenmail release-2.1.15 | official jar, registry checksum, annotated tag | pinned JRE | POP3 (fresh session per scan) | jar `91d58798…` |
| alkemio-mailslurper | third-party image, extracted binary | pinned musl loader | HTTP JSON | executable `1c7556f3…` |

The cgo row's runtime is the host's glibc and the smtp4dev row's is the .NET
bundle on host libraries: neither is a matched-runtime claim, and both are
labelled in the claim, the report and the README.

### One condition beyond the five ruled arms (for the operator)

`collection_ready` no longer counts an image pin's `tagged_source` gap.
`validate_image` forbids observing one ("an image pin has no tagged source to
observe"), so with the old check no image pin could ever be ready, and the
frozen protocol could not pass `mail_compare check` while it declares the
Alkemio row. The image's identity is its index → manifest → layer digest chain.
A test shows an official pin whose tagged source is unresolved is still
refused. This is a sixth widened condition beyond `REPORTED_VERSIONS`,
`RUNTIMES`, the `RegistryChecksum` asset-list skip (which the source-build row
shares: its executable is no release asset), `MIT` on the runtime list and the
`GitCommit` tree route; the operator may strike it, which leaves the Alkemio
row unable to collect. `scripts/compare-mail.sh`'s preflight, which reads the
protocol's target list as text, also omits a `does_not_start` target, or it
would ask for a binary the driver refuses.

### The freeze

Order: pin, untimed exploratory real probes, freeze, then the recorded real
proofs against the frozen bytes. `docs/evals/protocol.json` is `status:
frozen`, `unresolved: []`, with nine targets, nine cyclic rotations, nine
capability and product records and one `does_not_start` entry; its workloads
and corpus equal v2's. Frozen by content hash, recorded here and in the test
constants `FROZEN_PROTOCOL_V3_SHA` and `FROZEN_LOCK_V4_SHA`:

- `docs/evals/protocol.json` sha256 `229babaac849ae832ecb1a9f8ec742c970d0f36edc9d26d4ff9bb9c3c0d7224e`
- `docs/evals/sources.lock.json` sha256 `ed201599dd57441cbcf0c7e66e7c82a56dd217ef683b7bf4e72ae7ce9d3a04b1`

`cargo bench --locked --bench mail_compare -- check --protocol
docs/evals/protocol.json --sources docs/evals/sources.lock.json` exits 0 with
"ready for collection". After the freeze the bytes never change (stop trigger
S12: a frozen fact that proves wrong is a new protocol version or the
operator's ruling, not an edit). The history bytes keep their sha256.

### Recorded real proofs

Each ran as `timeout 600 taskset -c 0-31 cargo test --locked --test
mail_compare -- --ignored --exact eight_targets::<name> --test-threads=1`
(`nproc` 128, so `nproc/4` is 32 CPUs), with `/proc/loadavg` read before and
after; all exited 0, none is evidence, none wrote under `docs/evals/` and no v3
run directory exists. Wall time and 1-minute load before and after:

| Test | Wall | Load before → after |
|---|---|---|
| `real_smtp4dev_smtp_to_native_api_round_trip` | 4 s | 3.52 → 3.32 |
| `real_smtp4dev_hostile_round_trip` | 4 s | 3.32 → 3.21 |
| `real_caps_hold_smtp4dev` | 9 s | 3.21 → 3.68 |
| `real_inbucket_smtp_to_native_api_round_trip` / `_hostile_round_trip` / `real_caps_hold_inbucket` | 1 s / 1 s / 0 s | 3.68 → 3.54 |
| `real_greenmail_smtp_to_native_api_round_trip` / `_hostile_round_trip` | 4 s / 3 s | 3.54 → 3.82 |
| `real_caps_hold_greenmail` | 27 s | 3.82 → 4.24 |
| `real_mailslurper_source_build_smtp_to_native_api_round_trip` / `_hostile_round_trip` | 0 s / 1 s | 4.24 → 4.24 |
| `real_caps_hold_mailslurper_source_build` | 10 s | 4.24 → 3.89 |
| `real_alkemio_mailslurper_smtp_to_native_api_round_trip` / `_hostile_round_trip` | 1 s / 0 s | 3.89 → 3.89 |
| `real_caps_hold_alkemio_mailslurper` | 7 s | 3.89 → 3.74 |
| `real_mailslurper_official_does_not_start` | 0 s | 3.74 → 3.74 |
| `real_member_and_output_documents_match` | 3 s | 3.74 → 4.00 |

The labelled unpinned GreenMail driver proofs (`real_greenmail_driver_proof_*`)
became the pinned ones, one for one: `…_pop3_round_trip` →
`real_greenmail_smtp_to_native_api_round_trip`, `…_hostile_round_trip` →
`real_greenmail_hostile_round_trip`, `…_caps_hold` → `real_caps_hold_greenmail`;
`real_greenmail_pop3_under_pinned_jre` was renamed to that same function, so both removed names resolve to it. The official
MailSlurper's three real tests became the cgo row's
(`real_mailslurper_*` → `real_mailslurper_source_build_*`), beside the new
`real_mailslurper_official_does_not_start`, `real_smtp4dev_hostile_round_trip`,
`real_alkemio_mailslurper_hostile_round_trip` and
`real_member_and_output_documents_match`.

### What stays open

Collection and the v3 matrix; the final MCP contracts, comparison controls,
cost, identity and the full product (the wrapper's `controls`, `evidence` and
`measure` prerequisites); #148/#201; a best wait for any row; build provenance
for every executable but the cgo build's; GreenMail's license text (above); the
operator's ruling on the sixth condition (above) and on the two issue premises
(above). The PR is held for the operator: it changes `scripts/` and adds
pointers to accepted records.

### The wrapper smoke (unmeasured, never evidence)

After the freeze, `scripts/compare-mail.sh local` ran the nine-target smoke
fixture (`tests/mail_compare/eight-target-smoke.json`, tiny profiles) once,
under `timeout 600 taskset -c 0-31` with the load at 5.01 before and 4.90
after, in 32 s wall, with every `--bin` and both `--runtime` flags (GreenMail's
JRE and the Alkemio loader root). It exercised the `does_not_start` skip (the
official MailSlurper took no `--bin`, launched nothing and has a blocked report
row), the loader runtime and the cwd config together. It exited 1 by design:
the cgo build's and Alkemio's `smoke-c1-fresh` cells admitted 20 of 20 and lost
none, but 13 of 20 bodies differed from what was sent (the decoded-body
observation above), so those cells are "not an exact count"; every cleanup was
proved. It found two defects that the offline tests had not: the wrapper's
preflight required an executable file for a jar, and the driver refused a second
`--runtime` (one flag could repeat only for `--bin`). Both are fixed (a runtime
runs its file, which is then only read; `--runtime` repeats, once per target)
and `wrapper_reads_a_runtime_run_file_without_exec` proves the first. A
collection will exit 1 for the same fidelity reason.

## 2026-11-07 addendum (review return)

This addendum adds to the record above and edits none of it. The status stays
*proposed*; only the operator accepts. Review of `c5a784c` returned the change
to design and then to implementation (decision 0102); this records what that
return corrected, derived or found. The frozen `docs/evals/protocol.json` and
`docs/evals/sources.lock.json` are byte-unchanged (sha256 above, held by
`frozen_v3_and_lock_v4_bytes_are_the_recorded_ones`); nothing was collected,
no agent session ran and nothing paid ran.

### Nine targets, not eight

The closed list in §1, §2 and §3 reads eight. Ruling 1 added a ninth when it
asked for a separate "built from source" MailSlurper row. The nine are
`swarmail`, `mailpit`, `mailhog`, `mailslurper`, `mailslurper-source-build`,
`smtp4dev`, `inbucket`, `greenmail` and `alkemio-mailslurper`: the candidate
and eight competitors (`TargetKind::ALL` holds nine). The file name stays
"eight-target". The sections above that say eight are read as nine by this
sentence, not edited.

### The conditions protocol v3's readiness rests on

`mail_compare check` (that is, `collection_ready` and `verify_files`) passes on
the frozen pair because of four rulings and three further conditions. The
earlier addendum named one of the three as a "sixth widened condition" and left
it "for the operator", and recorded "the operator's ruling on the sixth
condition" as open. Those sentences are superseded, by this addition, with the
derivations below: each condition is entailed by the issue or a ruling, not
decided here, and each keeps a named refusal. The operator can still strike
(f) or (g) when ruling on this record; v3 then cannot collect as frozen, and
the remedy is a new protocol version or a ruling, never an edit of the frozen
bytes (nothing has been collected that a strike could invalidate).

| | Condition | Authority | Refusal kept (test) |
|---|---|---|---|
| (a) | `REPORTED_VERSIONS` | ruling 1 | another target or tag; output naming neither version |
| (b) | `RUNTIMES` table, loader in the image's manifest | rulings 2, 4 | a third target; a missing member; a layer absent from the manifest |
| (c) | `RegistryChecksum` skips the asset list | ruling 3 | the basis on another target; the checksum record naming `{download}.sha256`; tag, page, commitish and date still checked (`skipped_asset_list_still_checks_tag_commitish_and_date`) |
| (d) | `MIT` on `RUNTIME_LICENSES` | ruling 4 | `PERMISSIVE` exactly five (`permissive_list_is_exactly_the_five`) |
| (e) | `GitCommit` tree route with rebuild | forced by an observed tagged source | edited entry, another tree, another commit, blob absent |
| (f) | an image pin's `tagged_source` gap is not counted by `collection_ready` | entailed: the issue's "image digests" lock and its labelled Alkemio row; §2 (an image has no tag) | every source pin still refuses an unresolved gap, and an image pin still refuses an observed one (`official_pin_still_needs_an_observed_tagged_source_but_an_image_pin_does_not`, `image_pin_refuses_observed_tagged_source_or_build_provenance`) |
| (g) | `Provenance::SourceBuild` skips the asset list; its release download is replaced by `{home}/tree/{commit}` | entailed by ruling 1 ("bench a cgo build from the pinned release source … It may be pinned"): a supplied build is no release asset | `source_build_refuses_unresolved_build_provenance`, `…_provenance_on_another_commit`, `…_missing_build_evidence`, `…_other_download`; `upstream_release_pin_with_unlisted_asset_is_still_refused` (the skip never reaches an `Official` pin); `skipped_asset_list_still_checks_tag_commitish_and_date` (GreenMail and the source build, each with one edited field) |
| (h) | `ConfigFile.cwd` | an observed launch fact: the Alkemio fork reads `./config.json` and has no flag for it | `config_file_name_confined` (a name with `/` or `..` is refused); `config_file_cwd_runs_where_rendered` |

Condition (f) now has a closed shape. `collection_ready` skips the
tagged-source gap only when `!Provenance::has_tagged_source()`, which is false
only for `ThirdPartyImage`. Earlier it asked whether the lock data carried an
`image` field, so a pin kind could have inherited the exemption by accident;
`has_tagged_source_is_false_only_for_the_image_provenance` covers all nine
targets and gives a source pin an `image` field to show it gains nothing. The
image's identity stays its index → manifest → layer digest chain.

### Tightenings the review return added

None loosens a refusal, and the frozen bytes pass each of them:

- **A does-not-start cite must be failure evidence.** `does_not_start_matches`
  refuses a cite whose attribution is not `Supplied` or `Output`, so a README
  or a licence can never suppress a working row. The committed cite
  (`mailslurper-official-failure.txt`) is `Supplied`.
  `does_not_start_cite_must_be_failure_evidence`.
- **A sealed v3 report's blocked rows and labels are re-derived.** `admit_run`
  compares `blocked` and `labels` as well as `throughput` and `waits` for a
  protocol_version 3 report only; v2 admission keeps its terms, because the
  2026-10-30 collection predates both keys.
  `admit_run_refuses_a_v3_report_with_an_edited_blocked_row` (a dropped blocked
  row, a blocked cell given a number, an edited outcome, an edited label and a
  dropped `labels` key are refused; an untampered report admits). A label
  change after a v3 collection therefore needs a new protocol version.
- **One launch-order owner.** `Protocol::launch_order(rep)` (the rotation minus
  the does-not-start rows, which keep their slot) is the only filter: `plan`,
  `run`, the wrapper's launch specs and `declared_competitors` use it, and
  `grep -n 'does_not_start(' benches/` shows only `does_not_start`'s own
  definition, that one call and the validator's own name. `plan_yields_no_coordinate_for_a_does_not_start_target` now asserts the
  exact count, and `run_never_launches_a_does_not_start_target` runs the
  candidate alone against a protocol that lists every competitor as not
  starting (it went red, with `Infrastructure("mailpit: no launch spec")`, when
  `run` was made to bypass the filter).
- **The wrapper's flag binding is under test.** `flags` and a pure
  `launch_specs(p, s, bins, runtimes)` moved from `benches/mail_compare.rs` into
  `runner.rs` with unchanged behaviour (`prepare` keeps its file and scratch
  I/O): `flags_repeat_bin_and_runtime_only`, `launch_specs_take_two_runtimes`,
  `launch_specs_refuse_a_bin_for_a_does_not_start_target`,
  `launch_specs_refuse_a_runtime_given_twice_for_one_target`.
  `report_labels_name_every_unpinned_runtime` asserts the smtp4dev and
  source-build runtime labels.

### Corrections of record

- **GreenMail's REST API can read messages.** Context above says it cannot, the
  frozen lock's `wait.finding` says so, and so did `wire.rs`'s comment. The
  pinned OpenAPI's `mimeMessage` route returns a message, so all three are
  wrong; this is their correction of record. The frozen `wait.finding` stays as
  frozen. POP3 stays the read path because the issue orders it (0093 §4), and
  the comment now says that.
- **smtp4dev's runtime label.** The earlier addendum says the cgo and smtp4dev
  runtimes are "labelled in the claim, the report and the README". smtp4dev's is
  in the report (`self-contained .NET on host libraries (unpinned)`, added
  here) and the README, not in the frozen claim.
- **`capabilities.server_search` and the row label's `search`.** The first
  describes the product's own API (what it offers). The second describes the
  method the driver chose (a complete scan filtered on the client, where no
  search is verified). Both stand.
- **The musl `holder`** in the frozen lock names the Alpine package maintainer
  (apk record, Timo Teras). musl's copyright holders are in musl's COPYRIGHT,
  which was not supplied. The licence (MIT) is the apk record's; the frozen lock
  is unchanged.
- **Multipart bodies read back as the subject alone.** The design expected the
  MailSlurper family's multipart HTML to read back as `<p>html part</p>`. On the
  real binaries a multipart message comes back with no text and no HTML: the one
  stored body is re-wrapped under the multipart content type without boundary
  lines, so the driver's MIME reader finds no part. The proofs now assert what
  was observed instead of printing it: dot-stuffing is not undone
  (`..leading dot`), a `<br/>` follows each sent line, and a multipart message has
  no readable body.
- **The cgo build sometimes closes a fresh SMTP session before its banner.**
  Found by the receipts below (2 of 9 runs of its cap proof, before the fix).
  Nothing is sent when a session fails to open, so the cap proof opens the
  session again (up to five attempts, no sleep) and still asserts every delivery
  accepted and every token listed; 12 of 12 runs passed afterwards, and so did 10 of 10
  runs each of the build's round-trip and hostile proofs and of Alkemio's
  round trip. A collection counts
  the behaviour as a send failure in the cell.

### Alkemio's cap basis, observed

The cap proof had sent Alkemio's mail in batches of five on the assumption that
its fork loses mail under SQLite lock contention like the cgo build. The
bounded probe (`timeout 600 taskset -c 0-31 cargo test --locked --test
mail_compare -- --ignored --exact
eight_targets::real_caps_hold_alkemio_mailslurper --test-threads=1`, load 2.12
before and 2.35 after, 5 s wall) listed 600 of 600 in one burst, and six further
runs (load 2.24 → 2.26, 3.4 s each) did too: seven of seven. Alkemio therefore
left `loses_under_burst`, and the frozen lock's cap basis ("no cap is set … 600
sent, 600 listed") is true as written, and holds under a burst. The cgo build
loses mail in a burst and stays batched.

### Renames and removals

`eight_target_smoke_fixture_is_a_valid_draft_that_cannot_launch_yet` is renamed
one for one to `eight_target_smoke_fixture_is_a_valid_draft_that_launches`: it
asserts that the fixture's every declared row has a pin. The wall-clock
assertion in `real_mailslurper_official_does_not_start` (`started.elapsed() <
startup`) is deleted, because the `tokio::time::timeout` around the child
already bounds it (0020 §5); its bind-and-release port reservation stays, since
it fails closed. No other test function was deleted in this return.

### Receipts at the final head

Every real test reran after the last code change, one at a time, as `timeout 600
taskset -c 0-31 cargo test --locked --test mail_compare -- --ignored --exact
eight_targets::<name> --test-threads=1` (`nproc` 128, so `nproc/4` is 32 CPUs),
against the unchanged frozen bytes. Head `c76a12c012af6d36554b77d2caa3adc6fb50ec44`, tree
`d2abd518822ddb2a8ad0bb7613c25cda87548769` (the commits after it change only `docs/` and `specs/`). No file under
`docs/evals/` was written, and no v3 run directory exists.

| Test | Exit | Wall | Load (1, 5, 15 min) before → after |
|---|---|---|---|
| `real_smtp4dev_smtp_to_native_api_round_trip` | 0 | 8s | 3.45 3.52 3.58 → 3.90 3.62 3.61 |
| `real_inbucket_smtp_to_native_api_round_trip` | 0 | 1s | 3.90 3.62 3.61 → 3.90 3.62 3.61 |
| `real_greenmail_smtp_to_native_api_round_trip` | 0 | 4s | 3.90 3.62 3.61 → 3.67 3.58 3.60 |
| `real_mailslurper_source_build_smtp_to_native_api_round_trip` | 0 | 1s | 3.67 3.58 3.60 → 3.67 3.58 3.60 |
| `real_alkemio_mailslurper_smtp_to_native_api_round_trip` | 0 | 0s | 3.67 3.58 3.60 → 3.67 3.58 3.60 |
| `real_smtp4dev_hostile_round_trip` | 0 | 4s | 3.67 3.58 3.60 → 3.78 3.60 3.60 |
| `real_inbucket_hostile_round_trip` | 0 | 1s | 3.78 3.60 3.60 → 3.78 3.60 3.60 |
| `real_greenmail_hostile_round_trip` | 0 | 2s | 3.78 3.60 3.60 → 3.78 3.60 3.60 |
| `real_mailslurper_source_build_hostile_round_trip` | 0 | 1s | 3.78 3.60 3.60 → 3.78 3.60 3.60 |
| `real_alkemio_mailslurper_hostile_round_trip` | 0 | 1s | 3.78 3.60 3.60 → 3.78 3.60 3.60 |
| `real_caps_hold_smtp4dev` | 0 | 9s | 3.78 3.60 3.60 → 3.59 3.57 3.59 |
| `real_caps_hold_inbucket` | 0 | 1s | 3.59 3.57 3.59 → 3.46 3.54 3.58 |
| `real_caps_hold_greenmail` | 0 | 27s | 3.46 3.54 3.58 → 2.62 3.33 3.51 |
| `real_caps_hold_mailslurper_source_build` | 0 | 10s | 2.62 3.33 3.51 → 2.61 3.31 3.50 |
| `real_caps_hold_alkemio_mailslurper` | 0 | 4s | 2.61 3.31 3.50 → 2.64 3.30 3.50 |
| `real_mailslurper_official_does_not_start` | 0 | 0s | 2.64 3.30 3.50 → 2.64 3.30 3.50 |
| `real_member_and_output_documents_match` | 0 | 3s | 2.64 3.30 3.50 → 2.64 3.30 3.50 |

The wrapper smoke ran once more on the call path that moved
(`scripts/compare-mail.sh local`, the nine-target smoke fixture, every `--bin`
and both `--runtime` flags, `timeout 600 taskset -c 0-31`): load 4.75 before and
5.96 after, 75 s wall, exit 1 as recorded (the cgo build's and Alkemio's
`smoke-c1-fresh` cells are "not an exact count"), every cleanup proved, the
official MailSlurper not launched.

## Dated acceptance note — 2026-11-08

The operator ruled on 2026-11-08, in chat to the dispatcher, "Accept and
merge", on this record as put to them: the nine fixed targets, source lock v4,
protocol v3, the competitor claims limited to throughput and observed
visibility within the deadline, and the conditions (f), the image pin's
`tagged_source` gap not counted by `collection_ready`, and (g), the source
build's tree route in place of a release download, kept and not struck. The
operator also approved the PR's `scripts/compare-mail.sh` change, which lifts
the 0045 §8 hold. The body above is unchanged; where it says "proposed", "if
accepted" or "for the operator", it is read as accepted by this note. The two
issue premises the pinned bytes contradict (GreenMail's REST API reads
messages; GreenMail's license text is not in the supplied bytes) stand as
recorded in the addenda above. Collection and every paid session are still not
authorized by this note; #163's paid comparison waits for the operator's
confirmation of its plan.

2026-11-08: proposed [0129](0129-agent-suite-comparison.md) (issue #339, the agent-suite comparison) integrates with this record by addition. Its agent target `mailslurper` runs this record's `mailslurper-source-build` row, and it adds no lock entry, field or version; see 0129's 2026-11-08 addendum. Nothing here changes.
