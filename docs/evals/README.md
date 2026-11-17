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

**Status, 2026-11-16: the hard and expert tiers' paid run is published.** [`expert-2026-11-07.md`](expert-2026-11-07.md): 160 trials on main `c95386e` (2026-11-07), eight tasks (three hard, five expert) × four models × five trials, with its rows, report, post-hoc controls and failure evidence in [`expert-2026-11-07/`](expert-2026-11-07/). 151 passed: hard 57/60, expert 94/100. By model, claude-opus 40/40, claude-sonnet 39/40, codex-sol 40/40 and codex-luna 32/40. The failures are 5 `stale_code` (codex-luna × `otp-late`, 0/5) and 4 `gave_up`. One model fails at least 10%, not #325's two. No GLM ran; the DashScope GLM cell is a separate run, started 2026-11-16. Tokens per success and transcripts are not measured. That file proposes the #164 re-prioritization for the operator. The statuses below are kept as they were.

**Status, 2026-11-16: the 2026-11-07 throughput collection is published as raw measurement.** [`throughput-2026-11-07.md`](throughput-2026-11-07.md) publishes run `collect-20261107T112319Z-3344019`, protocol v2 (`fair-mail-comparison-v2`), on committed main `c95386e` with a clean tree: Swarmail, Mailpit and MailHog, 759,944 rows. `mail_compare admit` against [`history/2026-10-30-protocol.v2.json`](history/2026-10-30-protocol.v2.json) admits it, with the driver of main and with the driver of `c95386e`. The page gives each profile's medians with their spread and denominators, and Mailpit's 142 refusals at 1,000 fresh sessions (0 of 6 repetitions exact). It is raw measurement (`"evidence": false`, an unmatched storage profile, an unenforced local resource profile). Its waits are the common 100 ms poll for every target, so Swarmail's push wait is not measured (#351). It records no CPU time or memory, so it says nothing about cost to operate (protocol v4, #349). The report, manifest and index are committed beside the page. The 284 MB `samples.jsonl` is not committed; the page records where it is kept and its digest. What the top-level README may claim from this run waits for the proposed decision #349 calls for, which only the operator accepts. Until then the README and [`docs/guide/benchmarks.md`](../guide/benchmarks.md) quote no figure from it, and the statuses below are kept as they were.

**Status, 2026-11-09: the paid agent-suite comparison ran.** [`agent-comparison-2026-11-09.md`](agent-comparison-2026-11-09.md): 240 sessions on main `2e5c289`, four targets × three hard-tier tasks × four models × five trials. 236 passed: Swarmail 60/60, MailHog 60/60, Mailpit 59/60 and MailSlurper 57/60, which no test separates. Session wall time is reported there but measures the model and its context, not the server. Cost is unavailable. The throughput comparison is tracked in #349, and the status below is kept as it was.

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
claude-sonnet, codex-sol and codex-luna, no GLM. Decision 0129 (accepted) holds
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
`timeout 600` inside one `taskset -c 0-31` wrapper (32 CPUs, nproc/4 of this
host), with one worker and `uptime` before and after
(`timing/`: each matrix took 39 to 51 s, and the host's one-minute load average
was 1.2 to 1.9 around them, on a shared 128-CPU host). 120 rows: on every target
15 oracle passes and 15 null failures with the task's idle class (`gave_up`); one
(task, agent, attempt) is the same fixture on all four targets; each target's
contract lists one outcome for every announced tool (Swarmail's seven
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

**The order of the paid run.** `measure` runs one matrix per target, in the
record's target order, and each matrix runs its tasks, then agents, then
attempts: the record's `order` is "target, then task, then agent, then attempt",
and the dry run prints that plan. A task and attempt still see the same fixture
on every target, because each cell's seed derives from (master seed, task index,
task, agent, attempt) and every target's matrix is the whole hard tier. The cost
of this order is that a target's slots are consecutive in time, so a slow drift
of the host or of a vendor's service lands on one target rather than on every
target alike; the report must say so with any difference it claims.

**The MailHog adapter has no read-by-id tool.** MailHog's only route to one
message, `GET /api/v1/messages/{id}`, is in no document the lock binds and no
reviewed source states it (its primary attribution is open, as above), so it is
not source-verified and the adapter does not offer it. The list and search tools
already return each message's headers, body and raw MIME. The wire driver's own
fidelity read of that route is unchanged and stays listed as unresolved in
protocol v3.

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
| Method: [`protocol.json`](protocol.json) is protocol v3, **frozen** on 2026-11-07 and not yet collected (*Eight targets* below); protocol v2 is [`history/2026-10-30-protocol.v2.json`](history/2026-10-30-protocol.v2.json), byte-identical | **Frozen as protocol v2** (`fair-mail-comparison-v2`, superseding draft-3; the draft-2 and draft-3 bytes are kept under [`history/`](history/) and still read as version 1). It declares the limited official-artifact claim (no built-from claim), per-target backend, cap, prune and compression facts under the explicitly **unmatched** storage profile, the **unenforced local** resource profile, the common 100 ms poll, the driver's 40 s wait operation bound and the adapter's derived bounds and schema digest. `wait-best` is not a declared profile: it stays blocked. Two frozen collections are admitted as raw measurement, and neither is evidence: the 2026-10-30 run on an uncommitted tree (*Collection (2026-10-30)* below), and the 2026-11-07 run on committed `c95386e`, published in [`throughput-2026-11-07.md`](throughput-2026-11-07.md). The #151 baseline now exists ([`baseline-2026-11-01.md`](baseline-2026-11-01.md)); no comparison control or final contract does. |
| Wire driver: `benches/mail_compare.rs` | Built. Its polls now use the adapter's tested wait rule, and its scans refuse short nonfinal pages. A three-target smoke run against real Swarmail, Mailpit and MailHog seals and admits (`tests/mail_compare.rs`, ignored tests below). This is a smoke run, not evidence. Owned-bind corroboration and a cleanup worker that survives cancellation (T3), including the failed-drain-join arm, are **built and proved locally on Linux** (see *Method, as built*). Admission now recomputes the plan from the protocol, requires a proved cleanup receipt per launch and refuses a report that is not the arithmetic of its rows. The launch-level refusal of a foreign bind is proved through the kernel reading it rests on, not by forcing the reservation race, which needs no seam and has none. |
| Mailpit MCP adapter: `mail_compare mailpit-mcp` | **Built and proved locally.** It speaks MCP 2025-03-26 over stdio. Its tools are list, search, read and a labelled client-side poll, each a translation of the captured Mailpit API. It was tested against real Mailpit over real SMTP. Each call's time frame starts at admission (see below). Final client negotiation (#151/#152/#153) is not proved by these tests. JSON-RPC batches are now answered under Swarmail's bound (#339). |
| One command: `scripts/compare-mail.sh` | Built. `local` runs the driver as a labelled smoke run; `collect` runs the frozen protocol after the driver's own readiness check and admits it against that protocol. `controls` needs only the product record and the executables and runs the free oracle and null rows; `evidence` and `measure` stat the prerequisites in their fixed order (baseline, final contracts, controls, coordination receipts, cost basis, identity, full product), build the eval crate (`eval-build`) and run the verifier, which names the first prerequisite whose contents do not hold. A paid `measure` also needs `--authorized-product` equal to the product id. The `comparison-integration` refusal is gone. Only an authorized `measure` starts paid agents; it ran once, on 2026-11-09 ([`agent-comparison-2026-11-09.md`](agent-comparison-2026-11-09.md)). |
| Eval integration: comparison target, envelope and controls (T7–T9) | **Built; the free controls ran on all four targets.** `matrix --target` serves Swarmail natively and each competitor through its adapter behind the runner's bridge; the oracle solves each hard-tier task with that backend's own tools, and the null agent fails with the idle class. See the 2026-11-07 status above and [`agent-controls-2026-11-08/`](agent-controls-2026-11-08/). The paid run followed on 2026-11-09 ([`agent-comparison-2026-11-09.md`](agent-comparison-2026-11-09.md)), with the controls rerun on its head in [`agent-controls-2026-11-08-main/`](agent-controls-2026-11-08-main/). Cross-checkout #148/#201 shared-admission and CI receipts are unproved. |
| Evidence admission (T10) and the dossier (T11–T12) | **Reporting arithmetic built; one paid run cited, no dossier.** `report::cite_comparison` cites protocol, attempts and the whole denominator for every cell and refuses mixed protocols and repeated coordinates; `report::dossier` refuses, naming the first of baseline, final contracts and cost basis that is missing. Both are proved on synthetic rows only, so they are arithmetic proofs and not data. The prerequisites are now verified by content (`comparison-check`), in the fixed order, on real bytes in the eval crate's tests and on the committed controls. The #151 baseline now exists ([`baseline-2026-11-01.md`](baseline-2026-11-01.md)), but nothing admits it as a comparison input; there is no reconciled final MCP contract, and no comparison cost record (the basis is `unavailable`). The operator-confirmed paid comparison ran on 2026-11-09: its 240 rows are cited by `comparison-cite` in [`agent-comparison-2026-11-09/cite.json`](agent-comparison-2026-11-09/cite.json), and no dossier is built from them. |

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
On 2026-11-16 a v2 collection of committed bytes (`c95386e`, clean tree)
was published as raw measurement
([`throughput-2026-11-07.md`](throughput-2026-11-07.md)). It supplies the
"collection of committed bytes" that AC5–AC7 lists as missing. The row's
other missing items stay open.
The rows below are kept as recorded; no comparison criterion has closed.

Every criterion is open. "Proof so far" names local tests in
`tests/mail_compare.rs` or `evals/`; none is a collection or a CI receipt.

| AC | Proof so far | Missing | Owner |
|---|---|---|---|
| AC1 method | protocol v2 frozen; one refusal test each for a changed method, schema, product, false parity and cumulative cap overflow; draft-2 and draft-3 archived byte-identical | the admitted 2026-10-30 collection exists as raw measurement only (the method is not yet used for any claim) | implementer / operator |
| AC2 competitors | `the_tag_chain_and_every_blob_are_rechecked_against_the_captures`, real-binary tests | build provenance (upstream); finalization-time executable recheck | upstream / implementer |
| AC3 adapter | wait-frame and poll proofs on a paused clock; real SMTP → Mailpit → MCP | final client contracts (#151/#152/#153), batches | #151 / implementer |
| AC4 controls | the runner serves every target through the bridge; oracle and null controls ran on all four ([`agent-controls-2026-11-08/`](agent-controls-2026-11-08/)) | none: the paid run is [`agent-comparison-2026-11-09.md`](agent-comparison-2026-11-09.md) | operator |
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
prerequisite. The `comparison-integration` refusal is gone (accepted 0129 §9).
`measure --dry-run` verifies the prerequisites, prints the plan, opens no
wrapper configuration and launches nothing.

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

### Collection (2026-11-07)

One `collect` ran protocol v2 on committed main `c95386e` with a clean tree
(`candidate_dirty` false): run `collect-20261107T112319Z-3344019`, 126
throughput cells, 759,944 rows, all 270 launches' cleanup proved. Its record
is [`throughput-2026-11-07.md`](throughput-2026-11-07.md), with the report,
manifest and index under [`throughput-2026-11-07/`](throughput-2026-11-07/).
It is raw measurement, under the same limits as the 2026-10-30 collection
above. It is a separate run, and no figure merges the two.

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

## Protocol v4: cost to operate under an enforced budget (story A, #359)

This section makes no comparison claim. It records what story A of accepted
decision [0132](../decisions/0132-mail-comparison-protocol-v4.md) built.
Nothing was collected, no agent session started and nothing paid ran.

**Records.** Protocol v3 and lock v4 are still the active
[`protocol.json`](protocol.json) and [`sources.lock.json`](sources.lock.json),
and v3 still collects from those paths: the eval crate's tests and the
adapters' launch mode read the active lock, and v3 is frozen but not yet
collected. Their bytes are also under [`history/`](history/)
(`2026-11-07-protocol.v3.json`, `2026-11-07-sources.lock.v4.json`), identical,
and the tests that mean "v3" read those copies, so the v4 freeze (story E) is a
replacement of two files. The protocol v4 draft
([`draft/protocol.v4.json`](draft/protocol.v4.json)) and source lock v5
([`draft/sources.lock.v5.json`](draft/sources.lock.v5.json)) are a draft: this
driver refuses a *frozen* protocol 4 while the `api`, `waits` and `testexec`
sections are owed, and `collect-v4` refuses a draft at `check`, so nothing can
be collected for a v4 yet. `admit-v4` admits a directory against the protocol it
is given, a draft included, and the sealed run manifest carries the protocol's
`protocol_status` (`draft` or `frozen`), so a draft run is labelled a draft in
its own bytes and is never a v4 result. Protocol v4 pairs only with lock v5. No
v4 cell merges with a v2 or v3 cell.

**What it measures.** Under one enforced budget, per target and cost profile
(`cost-c1-reused`, `cost-c8-reused`, `cost-c32-reused`, `cost-c32-fresh`; 3
repetitions of 3,000 messages after 100 warmup): visible mail per
target-CPU-second (and CPU-ms per 1,000 visible mail), the sampled peak of
`memory.current` with the peak `anon` beside it, idle memory after readiness
(the mean of the last two seconds of five without traffic) with the scratch
tree's allocated bytes, and startup (ready ms and CPU-ms to ready). Each is
the nearest-rank median over repetitions with the minimum and maximum beside
it. A cell is ranked only *at equal results*: requested = accepted = visible,
seen once, fidelity intact, and every measured window `measured`. Any other
cell keeps its figures, is not ranked, and names its shortfall (`refused`,
`lost`, `fidelity`, `oom`, `timeout`, `target_exited`, `duplicate`,
`unexpected`, `ambiguous`). `mib_per_1k_retained` needs the API section's fill
levels (story B) and is published as owed, never as a number. The official
MailSlurper row is blocked in every cell. `mail_compare footprint` computes the
fixed facts (download bytes, installed bytes including every pinned runtime
tree, and a label, never a zero, for a runtime nothing pins) from the pinned
bytes with no measurement window. Every row has an installed size, and the run
headline carries each launched target's, copied from the run's
`footprint.json`; where a size cannot be established `footprint` refuses and
prints no document, and never publishes a `null`.

**Installed size, and how to extract an archive.** The candidate's installed
size is its release binary alone. For a row whose artifact is the executable
(MailHog, the MailSlurper source build, GreenMail's jar, the Alkemio binary) it
is the executable, its checked members and the whole runtime tree given with
`--runtime`. For the three archive rows (Mailpit, smtp4dev, Inbucket) it is
every regular file of the extracted archive: the *extraction root* is
`--bin`'s path with the pin's `member` path removed from its end (a `--bin`
that does not end with the member refuses by name), so extract each pinned
archive into a new, empty directory of its own, one per archive, and pass the
extracted member as `--bin`. Two targets that share a tree (one target's tree
holding another's executable or tree, the candidate included, after the paths
are resolved) refuse by name. A symbolic link and a directory count for
nothing. Every tree `footprint` walked is listed beside its row in
`installed_trees` with its `kind` (`runtime` or `archive`), `files`, `bytes` and
`sha256`, the digest of the tree's listing, which is what

```
(cd ROOT && find . -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum | sha256sum)
```

prints, so it can be checked by hand; a file name that is not UTF-8, or holds
a newline or a backslash, refuses because `sha256sum` would escape it. What
this does not prove, in plain words: the lock digests the archive and the
executable, not every file of an extracted tree, so the installed tree is
identified by its digest and not pinned. A second extraction of the same
pinned archive can be compared with the one that was measured, and nothing
more is claimed.

**A figure that was not observed is `null`, never zero.** A window that was never
opened (the chunk deadline came first, or the target was killed for memory or
had exited before it was ready) has no counters: its CPU, memory, affinity and
scratch fields are `null` in the rows, and so is every figure derived from it,
in the per-repetition lists and in the cells. A median, minimum and maximum
are taken over the repetitions that have a value and carry `n`, their count; a
cell with none has `null` and no number. CPU, memory and the OOM and throttle
counts come from a `measure` window that was opened, whatever ended it (an
unranked cell keeps them); idle memory and the scratch footprint come from an
`idle_ready` window that was `measured`, and startup from a `ready` window that
was. The idle and startup cells carry `ranked` and `shortfalls` as the cost
cells do.

**The chunk deadline.** One instant per chunk, taken from the run clock, ends
the chunk's work on the target: after it nothing new starts, and a connect, a
send, a listing, a fidelity read or the idle wait still in flight is cancelled,
not awaited. Only a launch already begun and the teardown are never cut, and
`chunk_deadline_ms` is bounded so that those and a margin fit inside the
wrapper's `timeout`, which sends SIGTERM at 560 s. A slow target is therefore a
measured `timeout` row and exit 1, not a killed driver. A target that exits or is OOM-killed before it is
ready is a measured outcome too (`target_exited`, `oom`) and the chunk goes on
to its next profile; one that is alive but not ready at the startup deadline
cannot be told from a broken launch, and stops the run with exit 2, so hold each
target once under the budget (`mail_compare hold`) before a long run.

**The budget and how it is enforced.** Every launch runs in its own cgroup v2
leaf, owned by the driver, under the scope `systemd-run --user --scope -p
Delegate=yes` makes for it: 2 CPUs (`cpu.max` `200000 100000`, and affinity to
two distinct physical cores; their SMT siblings are left idle where they are
inside the chunk's CPU set, and are recorded as `siblings_outside_set`
otherwise, because the driver cannot keep a CPU outside its own set idle),
2048 MiB
(`memory.max`, no swap, `memory.oom.group` 1) and 4096 tasks. The driver
writes each limit, reads it back and records it in the launch receipt; the
child joins the leaf and sets its affinity between fork and exec, and the
driver reads `/proc/<pid>/cgroup` and `Cpus_allowed_list` back after the spawn
and refuses a mismatch. A sampler reads the leaf every 100 ms from its
creation. Teardown runs the group sequence of 0092 §6 and 0093 §7 first, then
writes `cgroup.kill`, requires `cgroup.procs` to be empty and removes the leaf;
`cgroup_removed` joins the cleanup receipt, and a leaf launch is proved only
with it true. Off Linux, or without a delegated cgroup v2 hierarchy with `cpu`,
`memory` and `pids`, every v4 mode that launches refuses with exactly
`resources: no delegated cgroup v2 (cpu memory pids)` and there is no
unenforced fallback. A scratch directory on tmpfs is refused, because tmpfs
pages are charged to the leaf as `shmem`.

**What the figures do and do not say.**

- The CPU budget is a ceiling, not a reservation: a target that wants less
  uses less, and a busy neighbour on the host can still change a cell. The
  host's load is recorded beside every chunk and gates nothing.
- `memory.current` includes reclaimable page cache, so a target can sit at
  its limit without an OOM. The peak `anon` is published beside it, and only a
  rise of `oom_kill` over a window is an OOM; a rise of `max` alone is
  pressure.
- `nr_throttled` counts quota throttling, which also happens with the
  affinity intact. It is not evidence of the affinity, which is read back
  from `/proc` at launch and at the end of every window that was opened
  (`ready`, `idle_ready` and `measure`); a window in which a task held more
  than the two target CPUs is labelled `affinity_widened`, and ranking stays
  what it was. A window whose leaf held no task at that instant has `null`, no
  observation.
- An empty leaf proves that no process remains in the leaf. It does not prove
  that a same-uid process never moved itself to another cgroup the user owns.
  The pinned targets are not adversarial and this is not hostile confinement.
- A sealed run proves the directory's state at the seal: the index lists every
  file with its digest. It does not prove that a host power loss cannot lose
  the tail of `samples.jsonl` before the seal was written.
- Storage stays unmatched: each target keeps its own backend, and every
  capped target is launched with the same 150,000 cap through lock v5's
  `{max_messages}` placeholder.

**Reproduce.** A plan of the draft, launching nothing, then the proofs
that need a delegated host:

```bash
scripts/compare-mail.sh collect-v4 --dry-run --output /var/tmp/v4-dry \
  --protocol docs/evals/draft/protocol.v4.json \
  --sources docs/evals/draft/sources.lock.v5.json --documents docs/evals \
  --bin mailpit=… --bin mailhog=… --bin mailslurper-source-build=… \
  --bin smtp4dev=… --bin inbucket=… --bin greenmail=… --runtime greenmail=… \
  --bin alkemio-mailslurper=… --runtime alkemio-mailslurper=…

# one real proof, as the only process of a delegated scope (cargo must not
# be inside the scope):
bin=$(cargo test --locked --test mail_compare --no-run --message-format=json \
  | grep -o '"executable":"[^"]*mail_compare-[^"]*"' | tail -n1 | cut -d'"' -f4)
cat /proc/loadavg
timeout 600 taskset -c 0-31 systemd-run --user --scope --quiet -p Delegate=yes -- \
  "$bin" --ignored --exact v4::v4_real_oom_at_two_gib --test-threads=1
cat /proc/loadavg
```

`collect-v4` refuses by name before building: not Linux, a missing
`systemd-run`, `systemctl`, `taskset` or `timeout`, an existing `--output`, a
protocol that is not version 4 (the message names this draft route), fewer than
3 CPUs in `min(nproc/4, 32)`, a shell whose `nproc` differs from `nproc --all`
(the chunks are pinned to CPUs `0-<n-1>`), and, for a real run, a candidate
that is uncommitted, dirty or not on `main`. On `main` means that `HEAD` is an
ancestor of, or equal to, `refs/heads/main`, and of `refs/remotes/origin/main`
where that ref exists (`git merge-base --is-ancestor`), so a pull-request
branch is refused whether or not it has `main` merged in, an earlier commit of
`main` is accepted (a published run must stay repeatable after `main` moves
on), neither ref is a refusal, a ref that exists and is not a readable commit
is a refusal, a `git status` that fails is a refusal, and nothing is fetched. Every process it starts
after that (the two `cargo` builds, `targets`, `check`, `footprint`, each chunk
and both admissions) runs under `timeout --kill-after=30 560 taskset -c
0-<n-1>`: 560 s is the driver's `CHUNK_KILL_MS`, SIGTERM at 560 s and SIGKILL
30 s later. Every `systemctl` call has its own `timeout` (1 s to ask, 3 s to
kill with SIGKILL, 3 s to stop), so the ten minutes of 0132 §11.1 hold as three sums: a
build, `targets`, `check`, `footprint` or an admission takes at most 560 + 30 =
590 s; a chunk that failed, and the targets it left in its scope, at most 560 +
30 + 1 + 3 + 3 = 597 s; a chunk that sealed at most 560 + 20 × (1 + 0.5) + 3 +
3 = 596 s. Each chunk is one driver in its own scope: `timeout --kill-after=30
560 taskset -c 0-<n-1> systemd-run --user --scope -p Delegate=yes --unit=… --
<driver> run …`. A scope is judged from the word `systemctl --user is-active`
prints, and it is clean only when that word is `inactive` or `failed`; no
output, a failed or slow query or any other word is not known to be gone. After
a chunk that sealed it asks up to twenty times, half a second apart, and the
scope is clean at the first of the two words; after a chunk that failed it asks
once and does not wait. A scope that is not gone is killed and stopped and the
chunk counts as 2. A SIGINT, SIGTERM or SIGHUP goes at once to the step that
is running, a build, `check`, `footprint` or an admission included, and ends
the mode with exit 2; a chunk is judged as a failed one and written to the
ledger with exit 2, even when the signal came during its scope's judgement or
the write of its entry, and no later chunk starts. `reproduce.txt` is written only after the last chunk. Exit 1 is a measured
outcome of a target; exit 2 stops the run as incomplete, and nothing is resumed
or overwritten. `mail_compare hold` launches one target
under the budget for the story that follows; `mail_compare admit-v4 [--seal]`
recomputes the plan and every figure from the rows and names the first path
that differs.

**The host run gates the merge.** The fixture tests prove the driver's logic on
plain files. They do not prove the join through `cgroup.procs`, `cgroup.kill`,
the removal of a leaf, the OOM or the quota, and no reading of the code does.
The five real proofs (`v4_real_oom_at_two_gib`, `v4_real_cpu_held_within_two`,
`v4_real_setsid_escapee_removed`, `v4_real_hold_swarmail_under_budget`,
`v4_real_cost_chunk_seals_and_admits`) therefore run on a host with a delegated
cgroup v2 scope, one at a time, and their passing output with the load before
and after is attached to the pull request. Each fails, never skips, without
that scope, with the exact refusal above. The host needs `python3` on its
`PATH` as well: `v4_real_oom_at_two_gib` allocates its memory with it, and
without it that proof fails at the spawn, not with the refusal above. Until that output exists the story is
not complete and the change is not merged. Stories B (API level), C
(`wait-best`), D (test execution) and E (freeze and collection) are separate
runs.

## Protocol v4: API latency, native and MCP (story B, #360)

Story B of accepted [0132](../decisions/0132-mail-comparison-protocol-v4.md)
§6 builds the API section on story A's enforced lifecycle. It measures the
cost of an *exact* requested answer: a wrong, partial or inexact answer is a
failure row, never a fast one, and every unsupported operation is a published
row. Nothing here is a comparison figure; collection is story E's.

**One client per tool.** `benches/mail_compare/clients/` holds one module per
API family (Swarmail, Mailpit, MailHog, MailSlurper, smtp4dev, Inbucket,
GreenMail) over one bounded keep-alive transport and one strict pager: an
answer is complete only when paging reached the server's own count with every
id distinct, every page before the last full and the count unchanged; a
repeated id, a moving count, a short page, an overshoot, a redirect, a body
over 16 MiB or an operation over 1 GiB is an error. The byte bounds are
checked before a chunk is appended, a declared length over them refuses
before any byte is read, and one operation's pages share one budget. An id
the target supplies becomes one path segment with every byte outside
`A-Z a-z 0-9 - _ ~` percent-encoded, dots included; an id of dots alone is
refused, and so is any path the URL parser would rewrite, before dispatch
(wire tests assert the request target the server received). The eval
crate's MCP client holds the same bounds on every reply, initialization and
JSON-RPC errors included, and refuses redirects. The three MailSlurper
products share one family and keep their own citations. The eval crate
compiles the same modules by `#[path]` (`evals/src/lib.rs`), with only the
packages it already names directly (`reqwest`, `serde`, `serde_json`, `tokio`,
`base64`), so each tool has exactly one client; its MIME reader
(`clients/mime.rs`) and SMTP sender (`clients/smtp.rs`) exist because the eval
crate holds neither. The frozen oracle extractors `six_digit_code` and
`labelled_url` moved, unchanged, to `clients/extract.rs`; `evals/src/oracle.rs`
re-exports them, so the oracle and the API clients run one function. The
client digests, which `clients_sha256` freezes in story E (a test keeps this
table equal to the files):

| Client module | sha256 |
|---|---|
| `swarmail.rs` | `f2dca32df491471bdcd0509f4bcb0266d34385e86661c7daaa1b1f038bd3e143` |
| `mailpit.rs` | `d852e93846009002f589d01516bd6104e398be892f05cf5b31926088d198801b` |
| `mailhog.rs` | `3613f7f31277724083abca93b42e363f6bcaa888ba71aff99f9950967fa63702` |
| `mailslurper.rs` | `5a4b9bb1dce0c3f601d9c3ec704537e537b82c3caad5d8f5e4cf68e548fc0cfe` |
| `smtp4dev.rs` | `122ca842737d847470fcfa713e16eb5b38944ce5b2e57c5e12a1297ef7496f9e` |
| `inbucket.rs` | `77f5026c806aac85629160b6d02b9ddcef6d96ff9b0aa96c5be2ac0dc4a6b62c` |
| `greenmail.rs` | `9fd00f29b47a020c582c1ef5f434ed2900f638f945815209e1fc5c76bd06e85d` |

**Every route is cited and rechecked.** Each (target, surface, operation) is
`supported` with its routes and citations, `not_supported` with a citation,
`not_inspected` with the evidence that is missing, `no_mcp_surface` or
`does_not_start`. A citation names a lock v5 document of that target's pin
and a literal the document must contain; before an API chunk launches, the
driver rechecks every citation of its target on both surfaces against the
documents `verify_files` has just rechecked (path, size, digest), and a
candidate citation (`repo:`) against the tree. Missing evidence refuses; it is
never published as `not_supported`. What the captured documents establish:

- **Swarmail** reads its own inboxes (`GET /api/v1/inboxes/{inbox}/messages`,
  with `subject` for search), one message by id, codes and links from the
  fields ingest extracted (0005), delete by id and the narrowest clear, its
  inbox. Its fixtures are delivered per inbox through SMTP AUTH (0004), and the
  fill goes unauthenticated to `default`: per-test inboxes are the product
  feature measured, and the report says so.
- **Mailpit** (Swagger): no recipient filter is cited (the query syntax is
  behind a link the lock does not capture), so `list` is a complete scan
  filtered on the client; `search` sends the subject token to the documented
  search; delete by id and clear-everything are the documented
  `DELETE /api/v1/messages`.
- **MailHog** (Swagger, v2 only): `search kind=to` lists and searches; no read
  by id is captured, so `get` scans the store for the id (labelled
  `client_scan`); delete and clear are `not_inspected`, because `APIv1.md`,
  which its README names for them, is not captured.
- **MailSlurper** (route registrations and service controller): the `to`
  filter and `pageNumber`; read by id; no delete by id (`not_supported`: the
  only delete route prunes by date); clear is `not_inspected`, because the
  prune codes are not captured. Its one decoded body is v3's documented
  fidelity discrepancy, so exactly one absent body part, with everything
  else exact, is `inexact_fidelity`; a part it returns but gets wrong, both
  parts absent, or an absent part beside any other mismatch is a
  `wrong_result`.
- **smtp4dev** (`Rnwood.Smtp4dev.xml`): the file documents controller members,
  not route templates. Each citation binds the member (`GetSummaries` with
  `searchTerms`, `GetMessageSourceRaw`, `Delete`, `DeleteAll`); the templates
  under `/api/messages` are those the wire driver checked against the pinned
  executable, and the real host proof exercises them. Its narrowest clear is
  everything.
- **Inbucket** (REST example client): a mailbox per recipient, listed whole,
  source by id, delete by id and purge of one mailbox, so its clear is a
  recipient clear.
- **GreenMail** is read over REST, not POP3 (acceptance answer 8): the user's
  folder carries each message's MIME source, so `get` reads the folder and
  picks the uid; there is no delete of one message (`not_supported`), and its
  only clear is the purge of every mail.
- **MCP**: Swarmail's own tools at `POST /mcp`; the 0129 adapters of Mailpit,
  MailHog and the MailSlurper source build behind the bridge (no delete or
  clear: `not_supported`, 0092 §3, 0129 §5; MailHog's adapter has no read
  tool, so its `get` pages `mailhog_list`); `no_mcp_surface` for smtp4dev,
  Inbucket, GreenMail and the Alkemio image. Nothing on this surface falls
  back to native HTTP; untimed setup and postchecks read the native API.

**Exact answers, authored first.** Every fixture is a `code_link` message
authored before the server sees it (`api.rs`, `author`): subject, from, to,
text and HTML, one six-digit code and one link under `Confirm your account:`,
the eval fixture's templates, and a token whose five digits cannot be read as
the code. Setup resolves each fixture's server id by listing its own scope and
matching the token; nothing about its content is learned from the server.
`list` must equal the measured recipient's ten ids after full paging and an
exact recipient match (a substring filter's look-alike is dropped), `search`
(recipient AND subject) exactly one id, `get` every field as authored, and
the extractions the authored values.

**Fill and fixtures.** Each chunk is one fresh instance at one fill level (0,
1,000, 10,000, 100,000): the v3 corpus, byte for byte (a test compares it with
`wire::authored`), over eight reused SMTP sessions, round robin to 100
background recipients that are never measured. Accepted and retained counts
are confirmed within 300 s, or the level is `fill_not_reached` with the count
reached and every cell blocked; nothing is retried or reduced. Then
`idle_fill_N`: five quiet seconds, the last two sampled, and the scratch's
allocated bytes. Fixtures follow, outside every window: the read set (10),
each delete cell's 120 victims and 10 survivors in its own scope, each
recipient-clear cell's 120 ten-message scopes and one sentinel scope, and on
the MCP surface one fresh message per wait attempt. For Swarmail that is 4,010
messages (4,370 on MCP), computed before launch so fill plus fixtures stays
below the 150,000 cap; the record carries both counts separately.

**Cells.** Reads first (list, search, get, extract_code, extract_link at 1, 8
and 32 closed-loop workers, each worker on its own connection), then the MCP
future wait, then deletes, then recipient clears, then the single global
clear where that is the narrowest documented clear (one invocation, no warmup,
no quantiles). Each cell is 20 warmup operations under their own 5 s bound
(a failed warmup blocks the cell), then one queue of 100 measured operations
or an absolute 5 s deadline. A worker never starts an index at or after the
deadline; every started operation ends in exactly one row (`exact`,
`inexact_fidelity`, `wrong_result`, `failed` with its class, or `timeout`);
untaken slots are `unstarted`. What happens at the deadline depends on the
operation's effect (0132 §6.5). A read still in flight is dropped there — its
connection discarded with it — and ends as a `timeout` row; later cells
still run, because a slow shared-store scan must not censor them. A client
cannot prove the server stopped that read, so a remote resource tail is
possible and is disclosed, not drained. A delete, clear or future wait
(whose SMTP delivery is part of it) that timed out, lost its transport after
dispatch or whose delivery was ambiguous leaves its effect unknown: every
later cell is blocked, the launch is abandoned and torn down, and no ledger
and no global clear run. A completed refusal is a known non-effect and
abandons nothing. A failed read warmup blocks only its own cell. Every start
is recorded before it runs, so a worker that does not return within five
seconds of the deadline still leaves a `timeout` row and abandons the launch
as unproved cancellation; no phase runs past the chunk deadline, and a
warmup it cuts (or that ends on it) makes its cell a `chunk_deadline` row,
never a measured cell that started nothing. After each destructive cell an
untimed batch postcheck checks its scope, within its own 120 s and never
past the chunk's verification end (the chunk end natively; the ledger's end
on MCP); a postcheck cut there turns that cell's exact rows into
`postcheck_chunk_deadline` failures — unverified, not uncertain — instead of
tearing the chunk down. Fixture preparation stops at the cells' end, and a
preparation cut there makes every cell a `chunk_deadline` row. At batch end,
before the global clear, both surfaces run the same complete ledger, read
untimed through the native API: the store must hold, id for id, the
background the fill confirmed, the read set, every survivor, every victim
whose delete was refused or never started, every clear scope whose clear
was refused or never started, the sentinel, and each acknowledged wait
delivery exactly once — and none of the ids an exact delete or clear
removed, and nothing unaccounted for. Equal counts with substituted ids
fail. A mismatch turns every destructive and wait success into a
`wrong_result` and fails the chunk.

**Figures.** Per cell: planned = started + unstarted, started = exact +
failures, nearest-rank p50/p95/p99 over exact successes only (null when there
are none), inexact latencies kept apart, exact successes per second of the
client window, and target CPU-µs per *started* operation from the cell's
`cpu.stat` window (null when nothing started). A cell is
`at_equal_results` only when every planned operation started and was exact
*and* its resource window closed clean with its counters. The global clear
is one raw `sample_ns` with null quantiles. The raw rows, both intervals
(the client's and the owner's resource window) and the window's counters stay
in the chunk, and `admit-v4` recomputes the `api` figures from `api.json` and
the window rows, after checking that `api.json` is the plan's cells in order
and that every count adds up. At run level, `admit-v4 --seal` also writes
`report-api.json`: every API chunk's admitted figures in plan order, and a
row for every (surface, target, repetition, fill) that launches nothing (the
official MailSlurper's `does_not_start` and the MCP surface's
`no_mcp_surface`), with its status, null figures and no invented receipt;
the official MailSlurper's rows cite the captured failure its pin records
(`sources/mailslurper/mailslurper-official-failure.txt`).

**One verdict for both surfaces.** A chunk exits 1 — sealed, the target's own
failure — when any measured row is `wrong_result`, `failed` or `timeout`,
when a readiness, idle, idle-fill or cell window closed on an OOM kill or
the target's exit, when the target is found dead or OOM-killed after its last
reply, when the ledger fails, or when a target crashed before readiness; the
MCP holder seals that last case exactly as the native chunk does. A cell
blocked by the chunk deadline is the harness's stopping reason, not the
target's failure: on both surfaces it is published blocked and alone does
not fail the chunk. A future wait's answer is read from the tool's own
envelope — Swarmail's `{emails}` or `{timed_out}`, an adapter's `{status,
messages_count, messages}` with the family's native items — and a `late` or
timed-out answer is a `wait_late` or `wait_timeout` failure. A
capability row is not an execution, and documented inexact fidelity is
unranked evidence, not a failure. Infrastructure, interruption or an
unproved cleanup exits 2 unsealed.

**The MCP surface's owner.** `swarmail-evals api-mcp --driver PATH` takes
`run`'s chunk flags and drives `mail_compare hold --control api-v1`, which
launches the target under the budget, takes `ready` and `idle_ready`, prints
its endpoints, and then obeys JSON lines on stdin: `window_start` and
`window_end` (one cell at a time, each window on the holder's clock),
`idle_fill` (once, before any cell) and `record` (last; checked against the
plan and the windows). An unknown, duplicate, unmatched, out-of-order or
oversized line ends the hold with teardown and no seal; a hold without
`--control` keeps its silent-stdin meaning. The holder ends its control
input itself at the chunk deadline, so a caller that hangs cannot keep the
target. The caller stops first: it reads the protocol's required
`resources.chunk_deadline_ms` from an instant taken before it spawns the
holder, stops its cells 40 s before that end (a cut or unstarted cell is a
`chunk_deadline` row, as on the native surface), bounds the batch ledger to
end 10 s before it, and so sends its record while the holder still reads.
The control round trip is inside each resource window. The eval
side's send and receive instants of every command, with the holder's reply
instant, are kept in `api.json`; admission requires one idle-fill bracket
once the fill is reached and one start/end pair per measured cell, in order
on the runner's clock, each holder instant equal to its window's own, and
never subtracts one clock from the other. Every path after the holder starts
— a refusal, a failed adapter or bridge start, a rejected record, an
interruption — closes the control input and awaits the holder's receipt and
exit within a bound; the receipt must name the chunk's target and leaf and
prove the leaf removed, the process reaped, its group gone, its listeners
closed and its scratch removed, or the chunk is incomplete (exit 2). The
adapter receives the chunk's `--documents`, which the holder verified, and
rechecks the same bytes. Benchmark clients each draw JSON-RPC ids from a
disjoint range, initialization included, so 32 of them share one bridge;
the bridge's and the adapters' admission limits stay as they are, and a
refusal is a failure row by source, never retried: `busy_bridge` (the
bridge's 16 slots), `busy_adapter` (an adapter's queue), `rate_limited`
(Swarmail's own MCP tool rate, 1,024 calls a second, a product limit the
frozen launch does not raise, which refuses part of a 32-worker cell and
whose window carries into the next cell).

**Commands.** The wrapper's `collect-v4` runs the 96 native chunks
(`api.native.<target>.r<n>.f<fill>`) as `mail_compare run` and the 48 MCP
chunks (`api.mcp.<target>.r<n>.f<fill>`) as `swarmail-evals api-mcp`, each in
its own scope under the same bound, after the 24 cost chunks. The full v4
collection still refuses: the `waits` and `testexec` sections are owed
(stories C and D), and the draft lists every unresolved item, including the
`not_inspected` API cells above. An adapter serves the draft lock from its
documents root exactly as the chunk passes it:

```bash
mail_compare mailpit-mcp --sources docs/evals/draft/sources.lock.v5.json \
  --documents docs/evals --endpoint http://127.0.0.1:PORT
```

(likewise `mailhog-mcp` and `mailslurper-mcp`; without `--documents` the
documents are read beside the lock, and a missing, resized, altered or
escaping document refuses before anything is served —
`every_adapter_serves_the_draft_lock_from_its_documents_root`).

**Proof.** `cargo test --locked --test mail_compare` runs the deterministic
tests (citations against the bytes, decoders against the real wire corpus,
the pager against a bounded responder, the scheduler under a paused clock,
exactness, postchecks, plan, admission, the control lines) and the whole
native cell sequence against the real candidate; `cd evals && cargo test
--locked --test apibench` runs 32 id-disjoint clients through a real bridge
and every MCP cell, the coordinated future wait included, against the real
candidate; `--test apibench_repairs` and `--test mcp_bounds` drive the
producers against scripted loopback MCP, SMTP and holder processes (cutoffs,
wait ordering, busy sources, receipts, a fill not reached) and the transport
bounds. The host proofs are ignored and run one at a time under `timeout 600
taskset -c 0-31` with the load recorded: `api_fill_100k_real` (100,000
retained ids, held to a complete scan; it needs no delegation); one native
proof per launchable target, `api_native_real_<target>` for `swarmail`,
`mailpit`, `mailhog`, `mailslurper_source_build`, `smtp4dev`, `inbucket`,
`greenmail` and `alkemio_mailslurper`; and `api_mcp_real_swarmail` with
`api_mcp_bridge_real_{mailpit,mailhog,mailslurper_source_build}`, which
launch the real adapters behind the bridge. They need `MAIL_COMPARE_BIN`
(the built driver), `SWARMAIL_EVALS_BIN` for MCP, `MAIL_COMPARE_<TARGET>`
for a competitor's pinned bytes (`MAIL_COMPARE_JRE`, `MAIL_COMPARE_MUSL` for
GreenMail's and the Alkemio image's roots) and a delegated cgroup v2 scope,
and fail, never skip, without them. Each passes only exactly: the planned
matrix and capability rows, every row exact or a documented outcome (busy
or rate refusals on MCP, MailSlurper's absent part), exit 0 exactly when
nothing failed, an intact ledger, clean windows with counters, samples,
proved cgroup removal and the recomputed report; exit 0-or-1 or a few
measured cells is not a pass.

**The host run gates the merge, again.** The deterministic tests and the
real-candidate runs above prove the clients, the scheduler, the checks and
the bridge; they do not prove an API chunk under its leaf, the `api-v1`
control loop between `api-mcp` and `hold`, or any competitor's routes against
its pinned bytes (smtp4dev's route templates, GreenMail's REST folder,
Inbucket's and MailSlurper's field names are the captured documents' and the
v3 driver's, not yet exercised by this section). `api_native_real` and
`api_mcp_bridge_real` therefore run on the delegated host, for the candidate
and for each launched competitor's chunk, one at a time, and their passing
output with the load before and after is attached to the pull request. The
seat that built this story had no delegated scope: both proofs reached the
refusal `resources: no delegated cgroup v2 (cpu memory pids)`, and
`api_fill_100k_real` passed there (39.8 s; load 3.85 before, 4.49 after).
That evidence is of the earlier tree. On 2026-11-17 the review's repairs
(R1–R18 in `specs/api-latency/tasks.md`) replaced those two proofs with the
per-route proofs above; none of them has yet run on the delegated host, so
every API host log is still owed before merge.

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
