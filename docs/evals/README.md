# Evals and the fair mail comparison

Issue [#163](https://github.com/feedback-loop-ai/swarmail/issues/163) (epic #164).
Specification: [fair mail comparison](../../specs/fair-mail-comparison/spec.md).
Decisions: accepted [0091](../decisions/0091-fair-mail-comparison.md),
accepted [0092](../decisions/0092-official-mail-comparison.md)
([continuation spec](../../specs/official-mail-comparison/spec.md)) and
accepted [0093](../decisions/0093-source-mail-comparison.md)
([source-attributed continuation](../../specs/source-mail-comparison/spec.md)),
all by the operator on 2026-11-02. The operator's 2026-11-07 widening of #163
("compare with all that we have shims for") is recorded as dated addenda on
0091 and 0092; its contracts are in proposed
[0129](../decisions/0129-agent-suite-comparison.md)
([spec](../../specs/agent-suite-comparison/spec.md)).

**Status, 2026-11-02: one baseline, no comparison result.** The #151 paid
baseline exists: [`baseline-2026-11-01.md`](baseline-2026-11-01.md), 395 of
400 trials on main `a4f552d`, with its raw rows and report beside it. The
#309 hard tier has its paid run too: [`hard-2026-11-03.md`](hard-2026-11-03.md),
74 of 75 scored trials on main `8732445`. Its
claims are the eval suite's and are scoped in that file. This directory still
holds no final comparison run, comparison cost or loss record, because none of
those exists yet. One frozen collection is sealed and admitted as raw
measurement (*Collection (2026-10-30)* below); no figure from it is quoted or
read as a result. Nothing about the comparison, here or in the top-level
README, may be read as a measured claim about Swarmail, Mailpit or MailHog.
(Until 2026-11-01 this paragraph read "no results": no baseline existed.) The official competitor
artifacts are pinned, and each tag's source chain is now bound to captured
bytes. Every local proof below ran against the real executables. These are
untimed checks and labelled smoke runs, not collection.

**Status, 2026-11-07 (issue #339): the agent-suite comparison is built and
its free half has run. No paid session has, and nothing below is a comparison
result.** The operator widened #163 ("compare with all that we have shims
for"): Swarmail, Mailpit, MailHog and MailSlurper, on claude-opus,
claude-sonnet, codex-sol and codex-luna, no GLM. Decision 0129 (proposed) holds
the contracts. The runner serves all four targets; `scripts/compare-mail.sh
controls` ran the builtin oracle and null agents on the hard tier against each
of them; and one `measure --dry-run` verified every prerequisite and printed
the 240-slot plan. The planned paid run (the hard tier × 4 targets × 4 agents
× 5 trials) is the operator's to confirm and is #163's to run.

**Update, 2026-11-08: integrated with accepted 0130.** `main` took #340's
nine-target wire driver (accepted
[0130](../decisions/0130-eight-target-mail-comparison.md): source lock v4,
protocol v3) while this story was in review. The lock, the protocol, `history/`
and `sources/` here are `main`'s bytes, and the agent suite reads them: it adds
no lock version, entry or schema of its own. The agent suite's MailSlurper is
0130's `mailslurper-source-build` row, "built from source". The controls below
were rerun on the merged head. What 0129 owns of the lock work is superseded by
0130's lock v4, as 0129's 2026-11-08 addendum records.

**The MailSlurper that is measured is "built from source".** The operator
ruled on 2026-11-07 (0129's second addendum) that the published linux
executable's failure is a result, and that the comparison's MailSlurper is a
cgo build of the pinned release source. Accepted
[0130](../decisions/0130-eight-target-mail-comparison.md) (2026-11-08) pinned
that same executable in [`sources.lock.json`](sources.lock.json) as its own row,
`mailslurper-source-build`, and kept the official release as the separate
`mailslurper` row, which protocol v3 records as not starting. The agent suite's
target `mailslurper` names the service API its adapter drives; its executable is
the `mailslurper-source-build` row, and a product record bound to the official
row is refused by name. The lock is main's, byte for byte; this section only
reads it.

- *The published executable does not start.* `mailslurper-1.15.0-linux.zip`
  (`48f2f9ba…55f6`, member `0cd9416b…2627`) was built with `CGO_ENABLED=0`; on
  its default SQLite storage it exits with status 1 ("go-sqlite3 requires cgo
  to work. This is a stub"). The lock keeps it as the official `mailslurper` row,
  with its stderr
  ([`sources/mailslurper/mailslurper-official-failure.txt`](sources/mailslurper/mailslurper-official-failure.txt)),
  never launched by either driver.
- *The build.* `mailslurper` (`eb6cd97d…58ff`, 19942072 bytes) was built from
  the pinned source archive (`83f66f25…359f`) in
  `golang:1.18.1-bullseye@sha256:3b1a72af…` with `CGO_ENABLED=1`, verified
  modules and `go build -trimpath`
  ([`sources/mailslurper-source-build/mailslurper-cgo-provenance.md`](sources/mailslurper-source-build/mailslurper-cgo-provenance.md)).
  It links glibc 2.31 or newer. It is one build and not a reproducible one: a
  rebuild that differs is a new pin. It reports **v1.14.1 under tag
  `release-1.15.0`**, because the tagged source hard-codes that string and the
  published executable logs the same; the lock, the product record and this file
  say so. "Built from source" claims that these bytes were built from the
  pinned, tag-named source as recorded. It does not claim to reproduce
  upstream's release.
- *The tree is observed.* The lock binds the release commit `068acffd…1bd2` and
  its root tree listing (`5f29f08a…0729`). The verified source tarball,
  extracted into a new directory and added to a fresh git index, writes the same
  tree
  ([`transcripts/tarball-tree.txt`](agent-controls-2026-11-08/transcripts/tarball-tree.txt)).
- *The sources 0130 does not bind.* The lock binds the service controller, the
  route registration, the licence, the README and the config. The other files
  the adapter cites (SMTP and storage code, the query builder, `go.mod` and
  `go.sum`) are committed under
  [`agent-sources/mailslurper/`](agent-sources/mailslurper/), outside the
  0130-governed `sources/` tree. A digest table in `tests/mail_compare.rs` binds
  each file to its member of the source archive.

What the first real runs of that build showed, and the oracle and adapter now
read as they are (the captured corpus, [`wire-corpus/mailslurper/`](wire-corpus/mailslurper/),
is decoded in CI):

- **The AUTH asymmetry.** MailSlurper 1.15.0 has no SMTP AUTH command; the
  fixture omits the AUTH step for it; every other target receives AUTH PLAIN.
  The reviewed source says so, and the probe against the built executable
  (three runs, identical) confirms it: EHLO advertises no AUTH, and the `AUTH
  PLAIN` line draws `221 Bye`, twice (the source sends one goodbye per worker
  error, and the command goroutine's `break` only leaves its `select`), then a
  close, with no `235` and no `334`
  ([`transcripts/mailslurper-auth-probe.txt`](agent-controls-2026-11-08/transcripts/mailslurper-auth-probe.txt)).
  The published executable was not probed. The first run of the probe asserted
  exactly one `221 Bye`, and it failed on the second; the assertion now accepts
  one or two and nothing else.
- **Declared delivery semantics.** MailSlurper sends its `250` before it stores
  the mail, so `accept_implies_queryable` is `not-promised`; the labelled poll
  waits until the count is met. Its `to`, `from` and `message` filters are
  unescaped SQL `LIKE` substring matches (`ann@` also returns `joann@`), so the
  oracle uses the filter as a pre-filter and then keeps an exact `toAddresses`
  match.
- **What it stores.** One `body` string per mail; `boundary` and
  `transferEncoding` come back empty whatever the mail said. A plain-text line
  break is stored as `\n<br/>` and the text is HTML-escaped (`&` is `&amp;`). A
  multipart mail keeps **one part, its HTML part**: the plain-text alternative is
  not stored, and the listing still carries the part's base64 that `GET /mail/{id}`
  decodes. An unknown id is an empty item at HTTP 200, not an error. This is the
  backend variable being measured, stated and not hidden: an agent on MailSlurper
  has no text alternative for `recipient-crowd`'s mail (the confirmation link is in
  its HTML anchor), and the control reads that anchor only when a backend exposes
  no text at all.
- **The unused shelf upstreams.** smtp4dev, Inbucket and GreenMail (and the JRE)
  are on the shelf; they are outside the four named targets and Swarmail has no
  shim for them, so none is a target here.

**What was built and proved** (tests named in the commits): the adapters' generic
half behind a small backend interface, JSON-RPC batches in every adapter under
Swarmail's own bound, MailHog and MailSlurper adapters (source-verified general
operations only, nothing Swarmail-only emulated, hostile arguments refused before
any request), the wire corpus for all three competitors (MailSlurper's recaptured under
0130's pin and config file), the loopback bridge (16 in flight, one-line forwarding,
and a later `initialize` answered from the first because several clients reach one
stdio session), `matrix --target` (inputs bound before anything is spawned, the
adapter and its cleanup receipt, schema-3 rows only), the product and cost records
([`agent-comparison.json`](agent-comparison.json),
[`agent-comparison-cost.json`](agent-comparison-cost.json): cost `unavailable`,
never 0), and `swarmail-evals comparison-check`, which reads each prerequisite's
contents in the fixed order and names the first that does not hold.
`scripts/compare-mail.sh` no longer stops at `comparison-integration`.

**The free controls** ([`agent-controls-2026-11-08/`](agent-controls-2026-11-08/)):
`compare-mail.sh controls` ran once, for real, on the final head (the
receipts name it; it is after the merge of `main`'s accepted 0130) against the candidate's
`swarmail` and the three pinned executables, each target as its own command under
`timeout 600 taskset -c 0-31` with one worker and `uptime` before and after
(`timing/`: each matrix took 37 to 51 s, and the host's one-minute load average
was 2.3 to 2.9 around them, on a shared 128-CPU host). 120 rows: on every target
15 oracle passes and 15 null failures with the task's idle class (`gave_up`); one
(task, agent, attempt) is the same fixture on all four targets; each target's
contract lists one outcome for every announced tool (Swarmail's six
state-changing tools are listed `skipped_mutating`, and the others are called
read-only); the locked evals suite passed at the receipts' head. The MailSlurper
controls read `mailslurper-source-build`, the row 0130 pinned. The same run
re-proved the AUTH probe, every announced tool of each adapter and the digest
table of the adapter-cited sources against the verified pins and the source
archive
([`transcripts/`](agent-controls-2026-11-08/transcripts/)). `compare-mail.sh
measure --dry-run` with every prerequisite genuine exits 0, prints the 240-slot
plan and names two pending items, the CI run and the operator's authorization
([`dry-run.txt`](agent-controls-2026-11-08/dry-run.txt)). The 2026-11-07 controls
this replaces bound the old head's `swarmail`, adapter and harness digests, which
the merge changed, so they were rerun and removed, not patched.

**What only CI or the operator proves:** the brokkr bundle-compile gate (this
sandbox has no brokkr binary; it runs in verify), CI's required contexts, the
receipts' `ci` item (pending), acceptance of 0129, the operator's confirmation of
the 240 paid sessions (#163), and the model CLIs' own negotiation with the bridge,
which the bridge logs in the first paid session and the free controls do not
claim. The PR is held for the operator, because `scripts/compare-mail.sh`
changes. #340's lock is integrated, not contended: this branch changes no frozen
byte of it.

## What exists, and what does not

| Deliverable (spec) | State |
|---|---|
| Source pins: [`sources.lock.json`](sources.lock.json) is lock v4 since 2026-11-07 (see *Eight targets* below); the lock v3 the 2026-10-30 collection ran under is [`history/2026-10-30-sources.lock.v3.json`](history/2026-10-30-sources.lock.v3.json), byte-identical, and its Mailpit and MailHog rows below are unchanged | **Artifacts pinned and tagged sources bound; build provenance unknown.** Mailpit v1.32.0 and MailHog v1.0.1 are pinned by asset and executable digest. Each tag's captured ref → annotated tag → commit chain and the commit's untruncated tree listing are committed under [`sources/`](sources/). Every repository document (licence, API, setup README, reviewed source) is bound by size, SHA-256 and git blob id to its path in that tree, and rechecked whenever the lock is used. Nothing ties either executable to its tagged commit, so `binary_build_provenance` stays unresolved and the lock makes no built-from claim. |
| Method: [`protocol.json`](protocol.json) is protocol v3, **frozen** on 2026-11-07 and not yet collected (*Eight targets* below); protocol v2 is [`history/2026-10-30-protocol.v2.json`](history/2026-10-30-protocol.v2.json), byte-identical | **Frozen as protocol v2** (`fair-mail-comparison-v2`, superseding draft-3; the draft-2 and draft-3 bytes are kept under [`history/`](history/) and still read as version 1). It declares the limited official-artifact claim (no built-from claim), per-target backend, cap, prune and compression facts under the explicitly **unmatched** storage profile, the **unenforced local** resource profile, the common 100 ms poll, the driver's 40 s wait operation bound and the adapter's derived bounds and schema digest. `wait-best` is not a declared profile: it stays blocked. One frozen collection is admitted as raw measurement (see *Collection (2026-10-30)* below); it is not evidence. The #151 baseline now exists ([`baseline-2026-11-01.md`](baseline-2026-11-01.md)); no comparison control or final contract does. |
| Wire driver: `benches/mail_compare.rs` | Built. Its polls now use the adapter's tested wait rule, and its scans refuse short nonfinal pages. A three-target smoke run against real Swarmail, Mailpit and MailHog seals and admits (`tests/mail_compare.rs`, ignored tests below). This is a smoke run, not evidence. Owned-bind corroboration and a cleanup worker that survives cancellation (T3), including the failed-drain-join arm, are **built and proved locally on Linux** (see *Method, as built*). Admission now recomputes the plan from the protocol, requires a proved cleanup receipt per launch and refuses a report that is not the arithmetic of its rows. The launch-level refusal of a foreign bind is proved through the kernel reading it rests on, not by forcing the reservation race, which needs no seam and has none. |
| Mailpit MCP adapter: `mail_compare mailpit-mcp` | **Built and proved locally.** It speaks MCP 2025-03-26 over stdio. Its tools are list, search, read and a labelled client-side poll, each a translation of the captured Mailpit API. It was tested against real Mailpit over real SMTP. Each call's time frame starts at admission (see below). Final client negotiation (#151/#152/#153) is not proved by these tests. JSON-RPC batches are now answered under Swarmail's bound (#339). |
| One command: `scripts/compare-mail.sh` | Built. `local` runs the driver as a labelled smoke run; `collect` runs the frozen protocol after the driver's own readiness check and admits it against that protocol. `controls` needs only the product record and the executables and runs the free oracle and null rows; `evidence` and `measure` stat the prerequisites in their fixed order (baseline, final contracts, controls, coordination receipts, cost basis, identity, full product), build the eval crate (`eval-build`) and run the verifier, which names the first prerequisite whose contents do not hold. A paid `measure` also needs `--authorized-product` equal to the product id. The `comparison-integration` refusal is gone. No mode starts a paid agent. |
| Eval integration: comparison target, envelope and controls (T7–T9) | **Built; the free controls ran on all four targets.** `matrix --target` serves Swarmail natively and each competitor through its adapter behind the runner's bridge; the oracle solves each hard-tier task with that backend's own tools, and the null agent fails with the idle class. See the 2026-11-07 status above and [`agent-controls-2026-11-08/`](agent-controls-2026-11-08/). No paid trial ran. Cross-checkout #148/#201 shared-admission and CI receipts are unproved. |
| Evidence admission (T10) and the dossier (T11–T12) | **Reporting arithmetic built; no inputs exist.** `report::cite_comparison` cites protocol, attempts and the whole denominator for every cell and refuses mixed protocols and repeated coordinates; `report::dossier` refuses, naming the first of baseline, final contracts and cost basis that is missing. Both are proved on synthetic rows only, so they are arithmetic proofs and not data. The prerequisites are now verified by content (`comparison-check`), in the fixed order, on real bytes in the eval crate's tests and on the committed controls. The #151 baseline now exists ([`baseline-2026-11-01.md`](baseline-2026-11-01.md)), but nothing admits it as a comparison input; there is no reconciled final MCP contract, no final comparison trial and no comparison cost record. No paid comparison trial is authorized. |

These rows are open, not waived. The story is incomplete until genuine
evidence is admitted (spec AC12–AC14). Passing local tests does not count
as comparison evidence. The historical locks stay unchanged:
[v1](history/2026-10-28-sources.lock.v1.json), the stopped candidate's
unverified draft, and [v2](history/2026-10-28-sources.lock.v2.json), whose
`source_revision` meant built-from. Each is read only by its own reader,
and nothing launches from either.

### Acceptance ledger (2026-10-29)

*Since this dated ledger (2026-11-02):* the #151 baseline landed
([`baseline-2026-11-01.md`](baseline-2026-11-01.md)), #152 and #153 merged
(#288, #282), #201's eval CI gating landed (#300) and #219 closed (#267).
The rows below are kept as recorded; no comparison criterion has closed.

Every criterion is open. "Proof so far" names local tests in
`tests/mail_compare.rs` or `evals/`; none is a collection or a CI receipt.

| AC | Proof so far | Missing | Owner |
|---|---|---|---|
| AC1 method | protocol v2 frozen; one refusal test each for a changed method, schema, product, false parity and cumulative cap overflow; draft-2 and draft-3 archived byte-identical | the admitted 2026-10-30 collection exists as raw measurement only (the method is not yet used for any claim) | implementer / operator |
| AC2 competitors | `the_tag_chain_and_every_blob_are_rechecked_against_the_captures`, real-binary tests | build provenance (upstream); finalization-time executable recheck | upstream / implementer |
| AC3 adapter | wait-frame and poll proofs on a paused clock; real SMTP → Mailpit → MCP | final client contracts (#151/#152/#153), batches | #151 / implementer |
| AC4 controls | the runner serves every target through the bridge; oracle and null controls ran on all four ([`agent-controls-2026-11-08/`](agent-controls-2026-11-08/)) | the paid run (#163) | operator |
| AC5–AC7 throughput, waits, limits | smoke runs; scan and censoring tests; accounting recomputed from raw rows; storage facts probed against the real binaries; one frozen collection interrupted on 2026-10-29, then one completed, sealed and admitted on 2026-10-30 as raw measurement with Mailpit's own 421 refusals recorded (*Collection (2026-10-30)*) | best wait; a collection of committed bytes (the admitted run is `candidate_dirty` true); a collection under a matched storage and enforced resource profile; any claim beyond raw measurement | operator |
| AC8–AC9 lifecycle, privacy | launch/cleanup/privacy tests; T3 worker, hold, cancellation, forced-death, TERM-resistant, output-pressure and bind-ownership tests; real-competitor launches corroborated | CI (Linux and macOS) receipts; a forced reservation race | CI / implementer |
| AC10 schema/replay | eval suites pass on the repaired sources; comparison variant validated and refused by native readers | retained replay of comparison rows | implementer |
| AC11 one command | delivery modes refuse with their phase; one negative test per prerequisite; a real `controls` run and a real `measure --dry-run`; the paid delegation proved only with a stand-in verifier and no agent | a paid `measure` (operator authorization) | operator |
| AC12 eval readiness | standalone fmt, clippy and full default-threaded suite | #148/#201 cross-checkout coordination; CI receipts | #148/#201, CI |
| AC13 dossier | none | genuine #151 baseline, finals and costs | operator (T11) |
| AC14 honest claims | report censoring/denominator tests; citation arithmetic and dossier refusal on synthetic rows | admitted data | implementer after T11 |
| AC15 gates | local house gates on the candidate | CI-owned jobs, engine verification | verify seat, CI |
| AC16 delivery integrity | none | inherited HIGH review-integrity residual with its original references, #219, five protected reviews, operator acceptance of 0090–0093 | operator, panel |

### Roster, engine and residual rows (2026-10-29)

| Row | Examined | Not examined | State |
|---|---|---|---|
| Roster and engine (clarification Q1) | Accepted decision 0099 (claude-careful, operator, 2026-10-29, recorded with 0098 on `repair/claude-careful-recipe`) governs this delivery: Haiku 5.5 intake, Sonnet 5.5 implementation, five protected reviews (Fable correctness, Flash security, Flash guarantees, local Spark GLM-5.3-Flash tests, Opus independent), no Codex seat; the T14 Terra/Astra roster is historical. The run executes on host engine brokkr a9893b4 (brokkr decision 0096). The candidate keeps its pin to 731189f-p3: `bundles/self/tooling.json` binary SHA-256 `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`, equal to `.forge/bin/brokkr` as hashed in this checkout. | The a9893b4 engine itself, brokkr decision 0096 and the 0098/0099 texts: no a9893b4 bytes, build receipt or decision text exist in this worktree, and neither 0098 nor 0099 exists on this branch or is imported here. | Not re-pinned. `brokkr compile` proves the pinned engine; every record names which engine it means. **Superseded 2026-10-30:** main's #279/#280 re-pinned the candidate to `swarmail-engine-a9893b4-p4` (`.brokkr/recipes/self/tooling.json`, binary SHA-256 `29dc3309…`); the `.brokkr/state/bin/brokkr` the earlier implement seat hashed was `ab935558…`, which was *not* that pin, so that seat's `brokkr compile` ran on an unpinned binary and is not the pinned proof. **Corrected 2026-10-30:** in this checkout `.brokkr/state/bin/brokkr` now hashes to `29dc3309…`, equal to `tooling.json`, and the verify seat's compile for head `25c0854` ran that pinned binary. Likewise 0098 and 0099 now exist on this branch as accepted decisions, the folder layout is `.brokkr/` (0100), and the `bundles/self` and `.forge` paths in this row are historical. |
| Inherited HIGH review-integrity residual and #219 | The stop event mentions the residual without original finding IDs; #219 is a separate open item; the pinned engine's identity and digest as above. | The a9893b4 engine and decision 0096 against the residual; the original HIGH finding references, which are absent here. | **Open**: no panel verdict and no closure by this delivery, and no upstream-merge repair. The committed, proposed 0081 note records closure for the pinned binary by a re-run on those bytes (the record's claim, not this change's). It goes to the five-member protected review. Pointer, 2026-10-30: the dated notes in *proposed* 0081 (0081 is not accepted) record the routing proof on the three-patch pin and then on the four-patch pin the candidate carries; see *Still open after this record*. |

## Competitor sources and observed behavior

Checked on 2026-10-29 inside the delivery sandbox, against read-only GitHub
API captures the host supplied on 2026-10-28. There was no network, so
nothing was fetched or authenticated here. The hashes and git blob ids show
local consistency only. Both tags are unsigned, and a tag's commit is the
source's identity, not proof of what either executable was built from.

| Fact | Mailpit | MailHog |
|---|---|---|
| Release | [v1.32.0](https://github.com/axllent/mailpit/releases/tag/v1.32.0), published 2026-10-22; `target_commitish` is the branch `develop` | [v1.0.1](https://github.com/mailhog/MailHog/releases/tag/v1.0.1), published 2020-08-11; `target_commitish` is the branch `master` |
| Asset | `mailpit-linux-amd64.tar.gz`, sha256 `4b5993c978e8604c70d43927e2ad103b311bba3b1822fa0cf1d4ee4ec890a93c`, **equal to the digest in the release metadata** | `MailHog_linux_amd64`, sha256 `e2ed634ded49929f089b20045581955ed217672078fd86082dd7a6c67c5d09c7`, **locally computed**, because the release metadata publishes no digest |
| Executable | archive member `mailpit`, sha256 `262cfa28fae4554be198d1fea5ef83c0f8299317876d658623711c629a8edec8`; `version` prints `mailpit v1.32.0 compiled with go1.27.3 on linux/amd64` | the asset itself; `-version` prints `MailHog version: 1.0.1` |
| Tagged source | annotated tag `c00f93d0a78967e9f84c291eb34ab73386fdbe0c` → commit `ce798cfd781ef07b321427fc5082b3eebdcae460` ([ref](sources/mailpit/mailpit-ref.json), [tag](sources/mailpit/mailpit-tag.json), [tree](sources/mailpit/mailpit-tree.json)) | annotated tag `056259cd4133e22624340741776709bb46eb9ae9` → commit `0441dd494b03c9255a9b8e90e3458ebb115eacff` ([ref](sources/mailhog/mailhog-ref.json), [tag](sources/mailhog/mailhog-tag.json), [tree](sources/mailhog/mailhog-tree.json)) |
| Build provenance | unknown: no attestation, reproducible build or build record | unknown: no attestation, reproducible build or build record |
| Setup document | [README.md](sources/mailpit/mailpit-readme.md) at the tagged commit | [README.md](sources/mailhog/mailhog-readme.md) at the tagged commit |
| Licence | MIT, Ralph Slooten ([text](sources/mailpit/mailpit-license.txt), byte-identical to the archive's `LICENSE`) | MIT, Ian Kent ([text](sources/mailhog/mailhog-license.txt)) |
| API document | OpenAPI 2.0 "Mailpit API" v1 ([captured](sources/mailpit/mailpit-swagger.json)) | Swagger 2.0 "MailHog API" 2.0.0 ([captured](sources/mailhog/mailhog-swagger.json)) |
| Storage as launched | `--database` in the owned scratch (a SQLite file, **not memory**) and `--max 10000`. Without the flag Mailpit keeps 500, which cannot hold a 3,000-message exact count. | `-storage memory`; `-h` lists no retention cap |
| Wait or stream facility | A websocket notification hub (`server/websockets/hub.go`, [retained](sources/mailpit/mailpit-websockets-hub.go); `client.go` reviewed and bound by digest). See below. No best wait is selected. | A websocket hub in the vendored MailHog-Server package (`hub.go`, `connection.go`, reviewed and bound by digest); the README also advertises EventSource. See below. No best wait is selected. |

What the reviewed source shows, and what it does not (the lock's `wait.finding`
states each in full):

- **Mailpit.** Notifications go to registered websocket clients only. With
  no client counted the broadcast is skipped. Each broadcast is handed to
  the hub from a new goroutine, so separate broadcasts have no fixed order.
  A client whose 256-message queue is full is dropped. The upgrade response
  is written before the client is registered, and nothing acknowledges
  registration.
- **MailHog.** The hub drops a connection whose 256-entry queue is full, and
  `Broadcast` blocks until the hub takes the message. Here too, the upgrade
  response is written before registration, which nothing acknowledges.
- **Neither** reviewed subset shows which route serves the hub, which
  producer broadcasts new mail or when relative to storage, or any replay
  after a gap. MailHog's EventSource body was not captured. So these
  sources establish no lossless subscribe-before-scan wait, and they do not
  exclude other mechanisms. A dropped notification is a lost notification,
  not lost stored mail. `client.go` (Gorilla WebSocket header, BSD-style
  licence text not captured) and the vendored MailHog-Server files (licence
  file not captured) are not committed here. Each is bound in the lock by
  size, SHA-256 and git blob id, cross-checked against the retained tree.

Where the documents and the pinned executables disagree, the decoders follow
the executables and the disagreement stays recorded. These differences were
observed against the real binaries:

- **Mailpit.** `total` is always the mailbox size, and the match count of a
  search is `messages_count`; scans end on `messages_count`. Every list or
  search page also carries an undocumented `count`. `/api/v1/message/{ID}`
  marks the message read: an `is:unread` search drops by one per read. For
  an HTML-only message the served `Text` is derived from the HTML. The
  driver's storage-fidelity check therefore parses the documented
  `/api/v1/message/{ID}/raw` with the crate's own MIME parser, while the
  adapter passes the served Text through unchanged. An unknown ID gives
  `404`. A bad `start` or `limit` is silently coerced (`limit=-1` returns
  everything), so both clients bound and validate their own inputs. Without
  `--smtp-auth-accept-any` Mailpit does not advertise AUTH. With it, the
  AUTH user is recorded as `Username`, but there is one shared mailbox.
- **MailHog.** The v2 Swagger documents `messages` and lowercase `id`, and
  calls the search `total` the number of stored messages. The executable
  answers with `items` and uppercase `ID`, and its search `total` is the
  match count. `/api/v1/messages/{id}`, the stored raw message that
  fidelity is checked against, is described by no captured document, and
  its primary attribution remains open. An unknown ID gives `200 null`.
  AUTH PLAIN is accepted for any user.

## The Mailpit MCP adapter

`mail_compare mailpit-mcp --endpoint http://127.0.0.1:PORT --sources
docs/evals/sources.lock.json` serves MCP 2025-03-26 on stdin and stdout.
The caller (later, the eval owner) starts and owns Mailpit. The adapter
refuses any endpoint that is not a literal loopback socket, and it refuses
a lock that fails validation or document recheck, or that pins no Mailpit.
Results are Mailpit's own JSON in one text content block.

| Tool | Translation | Closed arguments |
|---|---|---|
| `mailpit_list` | one page of `GET /api/v1/messages` | `start` 0..100000 (0), `limit` 1..250 (50) |
| `mailpit_search` | one page of `GET /api/v1/search` | `query` 1..4096 bytes, `start`, `limit` |
| `mailpit_read` | `GET /api/v1/message/{ID}`, which **marks the message read** | `id` 1..512 bytes, path-encoded |
| `mailpit_wait` | a **client-side poll**, not a Mailpit facility: a complete paged search at once, then one 100 ms after each completed scan, until `timeout_ms` from admission; `found` (a scan that ended in time matched), `late` (first matched by a scan that ended after the cutoff; censored) or `timeout` (answered at or after the cutoff: no scan saw a match, not proof of continuous absence) | `query`, `timeout_ms` 1..20000 (the 30 s operation ceiling less one 10 s request bound) |

The adapter has no admin, send, delete, release, tag, chaos, screenshot or
link-check tool. It adds no task hints, extraction, cache, index or
prefetch, and it emulates no inboxes. Its bounds are engineering ceilings,
not upstream limits:

- 64 KiB per input line;
- eight admitted tool calls, one running at a time, with the input still
  read while it runs;
- 1 MiB per reply;
- 10 s and 16 MiB per HTTP request, and 16 MiB for a whole operation;
- 100 pages per scan and 1,000 requests per operation;
- a 30 s operation ceiling counted from admission, queue time included, and
  never extended: a wait's last scan may finish one full request bound after
  its cutoff, so the admitted `timeout_ms` maximum is derived as the ceiling
  less that bound (20 s) and a longer wait is refused, never clamped; a
  client that wants more observation makes a second call; a call whose
  ceiling passed in the queue sends nothing;
- a 10 s output and cleanup bound.

A repeated ID, a moving count, a short or empty page before the count, or a
wrong `start` echo is an error, never a shorter success. The next page
starts where the received ones end, never at a fixed stride. A complete
walk is still not a snapshot: churn that keeps the count can pass these
checks, so exact-count verification runs only on quiescent storage. A
scan still running at the allowance, a budget, a transport failure and
cancellation are errors, never an empty success. Cancellation and
EOF abort the running call, and unanswered queued calls are dropped. Errors
are fixed strings that echo no input.

## Method, as built

The protocol fixes the targets, the authored corpus (seed and kinds), every
profile's counts, sessions, connection reuse, deadlines and repetitions, the
counterbalanced target orders (one permutation per repetition), retention,
resources and the statistics. The validator refuses all of the following:
unknown, duplicate or missing fields; out-of-range counts; a workload that
would overflow a declared retention cap; an order that is not a permutation;
a poll with no frozen interval; a native wait claimed for a target that has
none implemented; and a superseding protocol that gives no change reason.
Collection also needs a `frozen` status, an empty `unresolved` list, a budget
that is actually enforced, retention equal to each competitor's launched cap,
and a complete pin for every competitor, and the best wait for `wait-best`.
Lock v3 validates each pin: the official repository, tag page and
release-asset URL; a tagged source with a full tag object and commit (a tag
name refuses), proved by the captured ref, tag and tree; a build provenance
that is unresolved or cites a captured build-evidence document (a tag or
source capture refuses); repository documents attributed to their path at
the tagged commit and API captures to the official API; a digest basis that agrees with the release metadata (an
upstream basis needs the published digest, while a locally computed one may
carry none); archive versus executable digests; a version report naming the
tag; a permissive licence; captured documents confined to `sources/<target>/`
and rechecked by digest; and a launch template whose cap flag equals the
stated retention. Any missing fact is a named gap, never a placeholder.

Protocol v2 adds the sections version 1 never carried, and the validator
holds each to what the code is built with: the claim scope (official
artifact, or built-from, which refuses while binary build provenance is
unresolved), a storage profile (`matched_memory` refuses unless every target
uses the memory backend and one cap), per-target backend, cap, prune
behavior and compression, a resource profile (an unenforced budget collects
only as `unenforced_local`), the common poll every poll observation uses,
the driver's wait operation bound (the longest wait plus one request bound),
the adapter's bounds, wait method and `tools/list` digest, and one product
per target that must equal the lock's release tag and executable digest. A
wait instance's present and future sends count against every declared cap,
beside each throughput instance's warmup and measured mail. Version 1 drafts
carry none of these and can never collect.

Facts the protocol states that a probe of the pinned executables supports,
and what it does not: Mailpit's `--max` flag sets its cap, and with a cap of
5 and 8 messages sent it did not evict within 20 s, so eviction is not
synchronous with acceptance, its order and timing are not established
(`prune: unestablished`), and the workload never reaches the 10,000 cap.
MailHog's help lists no cap flag, and 1,200 messages all stayed listed; that
bounds what was observed (`no_cap_observed`), not unlimited storage. Mailpit's
compression is its default level 1, read from its help and not set by the
launch.

Every profile runs on a fresh instance the driver owns. The child gets its own
process group and a closed environment: `PATH`, `LANG`, a private `HOME`,
`TMPDIR` and `XDG_*`, and nothing inherited. stdin and stderr are `/dev/null`,
the open-file ceiling is enforced through `setrlimit`, and scratch
directories are 0700. Readiness means the SMTP banner `220` and a `200` from
the HTTP listing before the startup deadline, and then both binds
corroborated: on Linux every listening socket that accepts at each address
(wildcard and v4-mapped ones included) must be held by a member of the
owned process group, read from `/proc`. A foreign, ambiguous (shared with
a non-member), absent or unreadable bind refuses the launch after
cleaning it up. Elsewhere ownership is unknown, so every launch refuses,
and the driver's launching tests are ignored off Linux.

One cleanup worker per launch owns the child from spawn to disposal. The
owner tells it to stop, or is dropped (a cancelled launch, an interrupted
run), and either way the worker performs the whole cleanup within one
composed budget: half of it is the leader's grace after SIGTERM, and the
forced group kill, the group's disappearance, the output drain and the
listener probes share the rest. Teardown reaps the child and checks that
the group and both listeners are gone and the scratch is removed. A drain
the budget cuts off is aborted and awaited, and a failed one proves
nothing. The driver settles every worker before it exits, an interrupted
run included, and reports whether cleanup was proved. An interrupted run is
never sealed either way. A worker that fails proves nothing, and its group
guard still kills the group. So does a worker dropped with the runtime
(forced death), and that leaves the scratch for the operator. If cleanup
cannot be proved, all later work stops. Before `index.json` is written,
each launched executable is hashed again, and bytes that changed since
launch refuse the seal.

Measurements use the controller's monotonic clock in nanoseconds:

- **Throughput.** Closed-loop workers each keep one delivery outstanding.
  Every message gets exactly one row: `accepted`, `rejected`,
  `failed_before_terminator`, `ambiguous` (anything after the final dot
  left the client) or `unstarted` (cut off by the send deadline). Nothing
  is retried. A competitor's own refusal of a session (a reply code before the
  terminator, such as Mailpit's `421 4.3.2 Too many connections`) is a
  measured `failed_before_terminator` row that keeps its code and reads
  `connect: refused`. It is counted in `refused`, makes the cell inexact and
  is never retried, throttled or tuned away. No documented Mailpit flag or
  `MP_*` variable governs that limit in the pinned release (binary
  observation only: the SMTP server source is not in the source capture, so
  the cap's value and semantics are unsourced), so no equivalent setting was
  applied: that would be an undocumented fairness change. The launches are
  therefore asymmetric, and the argv in each launch receipt shows it: the
  candidate is started off its default, `--max-sessions 1000` (its compiled
  cap of 256 would refuse the 1,000-session profile), while Mailpit and
  MailHog run as shipped with no session setting. The comparison does not
  claim the refusals' cause. *Accepted per second* = accepts divided by the send window
  (first start to the last worker's end). *Verified per second* = accepted
  ids observed on the server divided by the window from the first start to
  verification completion. After both windows close, every visible id is
  checked for its subject, text and HTML. This check is untimed.
- **Waits.** *Present*: the mail is acknowledged, then the wait is invoked,
  and latency = observed − invoked. *Future*: the wait is issued first, and
  latency = observed − send start. Issuing a request is not the server
  subscribing. *Empty*: a token that is never sent. Its expected outcome is
  a timeout, which is censored. A match first seen by a poll scan that
  ended after the cutoff is `late`: kept, but censored like a timeout. The
  signed offset observed − ack is kept as is and is never clamped. An
  answer carrying any other message is `unrelated`. *Native* (Swarmail's await) and *poll* (the common client
  path at a frozen interval) are separate profiles.
- **Statistics.** Nearest-rank p50/p95/p99 over the successful samples,
  with sample and censored counts beside them. An empty set gives `null`.
  No latency threshold is a correctness assertion.

A run directory is created new (0700) and every file in it is created once
(0600). The manifest is written before launch, the rows stream into
`samples.jsonl`, and `report.json` is derived purely from those rows.
`index.json` (SHA-256 and length per artifact) is written only after cleanup
is proved and every row has passed validation against the manifest's plan.
Admission refuses any of the following: a directory with no index, an
altered, extra or missing artifact, a truncated, foreign, duplicate or
unplanned row, and a row count that differs from the plan. A run counts
only through `mail_compare admit DIR PROTOCOL`, which also recomputes the
plan from the protocol (a strict prefix declared by the manifest never
admits), needs a proved cleanup receipt for every launch and checks the
report's cells against the rows, refusing a report of another version by
name and a differing cell by its first differing path. Each throughput cell reports requested
(`attempted + unstarted`), unstarted, the scheduled sessions, the retention
cap, admitted (accepts) and refused (rejected plus failed before the
terminator), and every report states its claim scope and storage and
resource profiles. Hashes show that
the files agree with each other. They do not authenticate the files against
a hostile user on the same uid.

Smoke runs (`--mode smoke`, the `local` command) are labelled as such and
carry `"evidence": false`. Smoke numbers measured while building this are
not published here. They come from one machine, one target and a draft
method.

## Reproduce

Prerequisites: Linux with `/proc`, the pinned Rust toolchain, a vendored or
pre-fetched cargo registry (builds run with `--locked --offline`), and this
checkout. For competitor runs you also need the pinned Mailpit and MailHog
executables, with the digests in `sources.lock.json`. Use disk-backed output
and scratch paths, not a RAM tmpfs.

```bash
bash scripts/compare-mail.sh                       # usage; launches nothing
# three real targets against the smoke fixture (smoke, never evidence); the
# protocol-2 era lock lives under history/, so name where its documents are:
bash scripts/compare-mail.sh local --output target/mail-compare/smoke-3 \
  --protocol tests/mail_compare/three-target-smoke.json \
  --sources docs/evals/history/2026-10-30-sources.lock.v3.json --documents docs/evals \
  --mailpit /path/to/pinned/mailpit --mailhog /path/to/pinned/MailHog
# the real-binary proofs (ignored by default; they fail, not skip, without the variables):
MAIL_COMPARE_MAILPIT=/path/to/pinned/mailpit MAIL_COMPARE_MAILHOG=/path/to/pinned/MailHog \
  cargo test --locked --test mail_compare -- --ignored
```

Pointing `local` at a full protocol launches its whole workload as a smoke
run, which is still not evidence (the committed protocol is now the frozen v3,
whose smoke over `tests/mail_compare/eight-target-smoke.json` launches every
declared row; protocol v2 and lock v3 under `history/` still can, with
`--documents docs/evals`). The driver's `check`
reports whether the protocol and lock admit collection. `collect --output
DIR --mailpit BIN --mailhog BIN` runs the frozen protocol once (never
resumed or overwritten) and then admits it against that protocol; it takes
about 14 minutes on the box this was developed on, so run it somewhere that
is not bounded to 10 minutes. `collect` exits 1 when the run is sealed and
admitted but a workload failed somewhere (for example a competitor's refusal);
only exit 2 means the run did not complete.
`controls`, `evidence` and `measure` exit 2 naming the first missing
prerequisite, then at `comparison-integration` until tasks T8–T10 land.
`measure --dry-run` prints the prerequisite order, opens no wrapper
configuration and launches nothing.

### Collection attempt (2026-10-29)

One `collect` run was started on candidate 5971ae2 (clean tree) with the two
digest-checked executables, and was interrupted by SIGINT at the seat's
10-minute command bound: 501,941 rows written of about 760,000 (382,086
fixed-count send, wait and verification rows plus one visible row per id),
through repetition 3 of 6, with all 179 launches' cleanup proved. It stayed
incomplete and unsealed, has no index and was never admitted, so none of its
rows is data and none is quoted. Two earlier attempts were killed when
their command ended and were deleted unread. The ledger row for the frozen
collection therefore reads **not completed**; nothing was relabelled.

### Collection (2026-10-30)

One `collect` ran the frozen protocol (`fair-mail-comparison-v2`) to
completion in the implement seat, on the implement seat's *uncommitted* tree
(the manifest records `candidate_commit` `62c6a47` and `candidate_dirty`
true, so the run is not bound to committed bytes and the verify seat did not
re-collect; it is not the committed-tree collection proposed 0091 §1 calls
for), with `swarmail --max-sessions 1000` and the
4096-connection backlog.

- Run `collect-20261029T232943Z-5` (the driver's own clock-stamped id; it
  was started 2026-10-29 23:29 UTC by the box's clock), 126 throughput
  cells, 760,077 rows, report version 2, every launch's cleanup proved.
- `scripts/compare-mail.sh collect` exited **1** (`WorkloadFailures`: Mailpit
  `stress-c1000-fresh` reps 1, 2 and 5 are not exact counts). Exit 2 did not
  occur. The script's own admission passed, and an explicit
  `mail_compare admit <dir> docs/evals/protocol.json` exited **0**.
- `index.json` SHA-256 `eb8db6b129ff86ea6861f476bd384a2e121e8a738b42a1c4000c5219f6a928ba`
  (manifest `b0ac6f41…`, report `230e5e6d…`, samples `24fb2508…`). The run
  directory (about 270 MiB) stays under `target/mail-compare/` and is not
  committed.
- Mailpit `stress-c1000-fresh`, 3,000 requested per repetition: rep 0, 3 and 4
  admitted 3,000 with 0 refused; rep 1 admitted 2,996 and refused 4; rep 2
  admitted 2,999 and refused 1; rep 5 admitted 2,996 and refused 4. All 9
  refusals are rows with `failed_before_terminator`, code 421 and reason
  `connect: refused`; none is ambiguous or unstarted. They were not retried.
  Swarmail and MailHog were exact in all six repetitions of that profile. No
  other cell of the 126 is inexact.
- Executables, checked before launch: Mailpit
  `262cfa28fae4554be198d1fea5ef83c0f8299317876d658623711c629a8edec8` (equal
  to `sources.lock.json`) and MailHog
  `e2ed634ded49929f089b20045581955ed217672078fd86082dd7a6c67c5d09c7`
  (**locally observed only**; its digest is the lock's executable pin, but no
  upstream receipt ties that file to the tag).
- The earlier refusal, "the report disagrees with the rows it was derived
  from", was not caused by workload failures. `admit` compared an in-memory
  report with one parsed from disk, and serde_json's default float parser
  does not round-trip every f64 rate; the dependency's `float_roundtrip`
  feature (no new crate, `Cargo.lock` unchanged) makes the comparison exact
  again. The refusal now names its first differing path.

This is raw measurement: `evidence` is false, the storage profile is
explicitly unmatched, the resource profile is unenforced local, and the
claim scope is the official artifacts as shipped. It establishes no
equivalence between the products, no best wait and no result worth quoting.
The Mailpit 421 refusals are the artifact's observed behavior under 1,000
fresh sessions on this box, recorded as measured data; their cause (a
connection limit, a backlog) is not established here.

### Still open after this record

#148 cross-checkout coordination (unproved; #201's eval CI gating has since
landed, #300); the final MCP contracts
(the #151 paid baseline has since landed, 2026-11-01:
[`baseline-2026-11-01.md`](baseline-2026-11-01.md)); operator acceptance of 0091–0093; the operator's ruling
on whether `--max-sessions` stands as 0027 §9's follow-up (0027's 2026-10-30
pointer records the story and claims no acceptance); a collection of
committed bytes, since the admitted run's manifest says `candidate_dirty`
true; CI; the five-member
protected review; the T7–T12 controls and dossier (no oracle or null control
has run, no paid comparison trial is authorized); a best-available wait; binary build
provenance for both executables; source review establishes no equivalence.
The inherited HIGH cross-attempt review-integrity residual stays on the
board for the protected review to rule. The dated notes in *proposed* 0081
(0081 is not accepted) record the full routing proof exiting 0 with every
cross-attempt case parked `awaiting_operator`: first (2026-10-29) on the
a9893b4 + three patches pin, then (2026-10-30) on the pinned a9893b4 + four
patches binary `29dc3309…` at main `9785f81`, which closes the residual for
this pin by a re-run on these bytes. The verify seat's compile for `25c0854`
used that same pinned binary. This change neither re-ran the proof nor claims
a merge repairs the engine; it cites the committed record, and the other
rows above stay open (#219 has since closed, #267).

## Eight targets: protocol v3 and source lock v4 (2026-11-07, #340)

This section makes no comparison claim. It records what was built for the
widening the issue states as "#163 approved and widened" (the issue's
paraphrase of the operator's 2026-11-07 direction), under **proposed**
decision [0130](../decisions/0130-eight-target-mail-comparison.md), which only
the operator accepts. Nothing was collected, no agent session started and
nothing paid ran.

**Records.** [`protocol.json`](protocol.json) is now protocol v3
(`fair-mail-comparison-v3`, superseding v2) and [`sources.lock.json`](sources.lock.json)
is lock v4. They are a separate run with a separate denominator: protocol v2,
lock v3 and the 2026-10-30 collection above stay as recorded (their bytes are
under [`history/`](history/), sha256 unchanged), and no table or statistic merges
a v2 cell with a v3 cell. Protocol v2 pairs only with lock v3, and protocol v3
only with lock v4. Protocol v3 copies v2's corpus, throughput and wait
profiles, budgets, resources and statistics unchanged; it adds the targets
(eight competitors beside the candidate, the MailSlurper source build among
them), per-target facts, a `capabilities` record, a product `scope` and the one
`does_not_start` result.

Read `protocol.json` and `sources.lock.json` in the 2026-10-30 record above
as the two `history/` files; for instance
`mail_compare admit <dir> docs/evals/history/2026-10-30-protocol.v2.json`.

**State: frozen, ready to collect, not collected.** Under the operator's
2026-11-07 rulings (recorded in decision 0130's second 2026-11-07 addendum)
every competitor row is pinned from the supplied bytes, every tagged source is
an observation, and protocol v3 is frozen: `status: frozen`, `unresolved: []`,
nine targets, nine cyclic rotations, nine capability and product records and
one `does_not_start` entry. `mail_compare check` exits 0 with "ready for
collection". The two files are identified by content hash (a test holds both):
`protocol.json` sha256
`229babaac849ae832ecb1a9f8ec742c970d0f36edc9d26d4ff9bb9c3c0d7224e`,
`sources.lock.json` sha256
`ed201599dd57441cbcf0c7e66e7c82a56dd217ef683b7bf4e72ae7ce9d3a04b1`. After the
freeze neither changes: a wrong frozen fact is a new protocol version or the
operator's ruling. Collection itself was not run; that is a separate story.

| Row | Source strength | Read path | Search | Storage | Runtime | Result |
|---|---|---|---|---|---|---|
| swarmail | the candidate checkout | HTTP JSON | server | memory | — | — |
| mailpit v1.32.0 | official, annotated tag | HTTP JSON | server | scratch file | — | pinned (bytes unchanged) |
| mailhog v1.0.1 | official, annotated tag | HTTP JSON | server | memory | — | pinned (bytes unchanged) |
| mailslurper release-1.15.0 | official, **lightweight** tag (root tree rebuilt to its id) | — | — | SQLite (scratch file) | — | **does not start**: the release executable is a `CGO_ENABLED=0` stub (a published result; the row keeps its pin and a blocked cell per profile, no number) |
| mailslurper-source-build | **built from source** (cgo), lightweight tag, build record supplied | HTTP JSON | client-side filter | SQLite (scratch file) | host glibc, **unpinned** | pinned; reports v1.14.1 under tag release-1.15.0 |
| smtp4dev 3.16.1 | official, **lightweight** tag | HTTP JSON (paged by `page`) | client-side filter | scratch file, cap raised to 10000 | self-contained .NET on host libraries (unpinned; the report's row label) | pinned |
| inbucket v3.2.0 | official, **lightweight** tag | HTTP JSON (per mailbox, unpaged) | client-side filter | memory, per-mailbox cap raised to 10000 | — | pinned |
| greenmail release-2.1.15 | official jar, annotated tag; **integrity against the Maven Central registry's own checksum, authenticity not verified** | **POP3** (fresh session per scan: login, `STAT`, `UIDL`, `TOP` per message; no `DELE`) | client-side filter | memory | Temurin 21.0.13+9 JRE, pinned | pinned |
| alkemio-mailslurper | **third-party image**, extracted binary, never an official row | HTTP JSON | client-side filter | SQLite (scratch file) | musl loader, pinned from the image's own layer | pinned |

Every row keeps its own storage, cap and wait method as launched: an
explicitly **unmatched** storage profile and the **unenforced local** resource
profile, stated in every report. A tool without a verified search is scanned
completely and filtered by token on the client; the method is a label on the
row, in `report.json`'s `labels` (source, runtime, version, integrity and
does-not-start labels included). Waits are the common 100 ms poll for every new
row; none has a source-verified server wait. A lightweight tag has no tag
object and no signature to record (it is absent, not "unsigned"), so a
lightweight pin carries weaker source evidence than an annotated one. The
claim stays throughput and observed visibility within a deadline: no
durability, no accept==stored and no equivalence.

**What was proved against real bytes.** Each launched row (smtp4dev, Inbucket,
GreenMail, the cgo build and Alkemio's binary) passed, in unmeasured and
bounded `#[ignore]`d tests that fail without their bytes, a round trip through
its native read path, a hostile set (dot-leading lines, a lone `.` line, an RFC
2047 subject, a 998-octet line, a boundary string in the body), and a
600-message cap test under the exact-listening-set and cleanup proofs; the
official MailSlurper executable was shown not to start;
`real_member_and_output_documents_match` re-extracted every archive member and
re-ran every captured program output. Decision 0130's addendum lists each
correction these runs made (MailSlurper's `totalRecords`, smtp4dev's `page`, an
unescaped `-` in a path segment) and each observation about a tool. Two bear on
how to read a table later: **MailSlurper keeps one decoded body, no raw message,
and does not undo SMTP dot-stuffing** (its body fidelity is reported per
message, not exact), and **it acknowledges a mail before it stores it and lost
some under SQLite lock contention in a burst probe** (a collection will show
accepted-but-missing mail in the cell). A collection therefore exits 1 (a sealed
run whose workload failed somewhere): the MailSlurper-family cells are never
"exact" because of the fidelity mismatches, and the report says where. A message whose part holds its own
boundary string mid-line is refused rather than returned cut short; the
measured corpus never contains one.

**Bounded runs.** Every real run is unmeasured and bounded under `timeout 600`
and `taskset` on at most `nproc/4` CPUs, one test at a time, with the load
recorded before and after:

```bash
cap=$(( $(nproc) / 4 )); cpus="0-$(( cap - 1 ))"
cat /proc/loadavg
timeout 600 taskset -c "$cpus" env MAIL_COMPARE_INBUCKET=… cargo test --locked \
  --test mail_compare -- --ignored --exact eight_targets::real_inbucket_smtp_to_native_api_round_trip --test-threads=1
timeout 600 taskset -c "$cpus" env MAIL_COMPARE_ALKEMIO_MAILSLURPER=… MAIL_COMPARE_MUSL=… cargo test --locked \
  --test mail_compare -- --ignored --exact eight_targets::real_alkemio_mailslurper_smtp_to_native_api_round_trip --test-threads=1
cat /proc/loadavg
```

The local smoke over `tests/mail_compare/eight-target-smoke.json` launches
every declared row (each has a pin) and is never evidence:

```bash
timeout 600 taskset -c "$cpus" scripts/compare-mail.sh local --output "$PWD/target/smoke-v3" \
  --protocol tests/mail_compare/eight-target-smoke.json \
  --bin mailpit=… --bin mailhog=… --bin mailslurper-source-build=… --bin smtp4dev=… --bin inbucket=… \
  --bin greenmail=… --runtime greenmail=… --bin alkemio-mailslurper=… --runtime alkemio-mailslurper=…
```

The official MailSlurper row takes no `--bin`: the protocol publishes that it
does not start, so the driver never launches it. Download each artifact into
its own new directory, verify its sha256 before use, and read it only with
`python3 -I` from a script kept elsewhere.

**Still open after this record.** Collection and the v3 matrix; the final MCP
contracts, the comparison controls, cost, identity and the full product (the
`controls`, `evidence` and `measure` prerequisites above); #148/#201; a best
wait for any row; build provenance for every executable but the cgo build's;
GreenMail's license text (its pin carries the jar's own Apache 2.0 declaration,
not the license text, which was not supplied) and its REST API, which the
pinned OpenAPI shows can read messages although the issue says it cannot (the
driver reads over POP3 as ordered); and the operator's ruling on decision 0130,
which includes the three conditions that readiness rests on beyond the four
rulings (named below). Nothing here is a comparison claim.

**Alkemio's cap basis, observed.** Unbatched, Alkemio's fork listed all 600 of
600 mails in a burst, seven bounded runs out of seven (2026-11-07, one at a
time), so its cap proof sends one burst like every other row and its lock
basis ("no cap is set; 600 sent, 600 listed") holds as written. The cgo
MailSlurper build does lose mail in a burst, so its cap proof still sends in
batches of five. The cgo build also sometimes closes a fresh SMTP session
before its banner (2 of 9 runs of its cap proof): a collection will count that
as a send failure in its cell.

**Three conditions readiness rests on, named.** Beyond the operator's four
rulings, `mail_compare check` passes the frozen pair only because of three
conditions entailed by the issue and the rulings, each with the refusal it
keeps (decision 0130's review-return addendum derives them): an image pin's
tagged-source gap is not counted (the issue orders an image-digest lock and a
labelled Alkemio row, and an image has no tag), the source build skips only the
release asset-list check (a supplied build is no release asset; tag, page,
commitish and date are still checked and the download must be the tagged
commit's tree), and a config file may run in its own directory (`cwd`, the
Alkemio fork reads `./config.json`). The operator may strike any of them when
ruling on 0130; nothing was collected that a strike could invalidate.

## What closes the remaining rows

1. Build provenance for each executable needs genuine build evidence (an
   attestation, a reproducible build or a published build record). Until
   then a measurement needs protocol v2's declared official-artifact claim
   scope, and it makes no built-from or equivalence claim. MailHog's
   `/api/v1/messages/{id}` read still needs a primary source.
2. A best-available wait needs route, producer, registration and gap
   evidence, plus real-binary observation, before `wait-best` can run.
   MailHog's EventSource body is not captured. The reviewed websocket
   sources show notification hubs only: Mailpit's `Broadcast` returns at
   once with no client registered and otherwise hands the message to the hub
   from a new goroutine, and each client's 256-slot queue drops a client
   that falls behind; MailHog's hub blocks its caller until its loop takes a
   broadcast and unregisters a connection whose queue is full. A dropped
   notification is a lost notification, not lost stored mail, and no replay
   after a gap is shown. Source review does not establish equivalence or a
   lossless wait. A runtime that enforces the 2 CPU / 2 GiB budget would
   replace the declared `unenforced_local` profile.
3. The comparison target, envelope and same-suite
   controls (T8–T9) and evidence admission (T10) are built in the eval
   owners. #148 receives its cross-checkout shared-admission receipt
   (#201's CI gating landed with #300). The final MCP contracts, batch handling included, land (the
   #151 baseline landed on 2026-11-01).
4. Genuine, separately authorized runs are admitted with their actual cost
   records. Losses and unsupported cells are published beside the wins, and
   README numbers cite their run, task, model, configuration and denominator.

## Cost

No paid comparison trial was run and none is authorized. Cost accounting is
*unavailable*, which is not the same as zero. The #151 baseline's paid trials
were a separate run, approved by the operator on 2026-11-01; they record no
token or cost field ([`baseline-2026-11-01.md`](baseline-2026-11-01.md)).
