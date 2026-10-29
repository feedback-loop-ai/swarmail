# Evals and the fair mail comparison

Issue [#163](https://github.com/feedback-loop-ai/swarmail/issues/163) (epic #164).
Specification: [fair mail comparison](../../specs/fair-mail-comparison/spec.md).
Decisions: proposed [0091](../decisions/0091-fair-mail-comparison.md),
proposed [0092](../decisions/0092-official-mail-comparison.md)
([continuation spec](../../specs/official-mail-comparison/spec.md)) and
proposed [0093](../decisions/0093-source-mail-comparison.md)
([source-attributed continuation](../../specs/source-mail-comparison/spec.md));
none is accepted.

**Status, 2026-10-29: no results.** This directory holds no baseline, final
run, comparison, cost or loss record, because none of those exists yet.
Nothing in it, or in the top-level README, may be read as a measured claim
about Swarmail, Mailpit, MailHog or any agent. The official competitor
artifacts are pinned, and each tag's source chain is now bound to captured
bytes. Every local proof below ran against the real executables. These are
untimed checks and labelled smoke runs, not collection.

## What exists, and what does not

| Deliverable (spec) | State |
|---|---|
| Source pins: [`sources.lock.json`](sources.lock.json) (lock v3) | **Artifacts pinned and tagged sources bound; build provenance unknown.** Mailpit v1.32.0 and MailHog v1.0.1 are pinned by asset and executable digest. Each tag's captured ref → annotated tag → commit chain and the commit's untruncated tree listing are committed under [`sources/`](sources/). Every repository document (licence, API, setup README, reviewed source) is bound by size, SHA-256 and git blob id to its path in that tree, and rechecked whenever the lock is used. Nothing ties either executable to its tagged commit, so `binary_build_provenance` stays unresolved and the lock makes no built-from claim. |
| Method: [`protocol.json`](protocol.json) | **Draft** (`fair-mail-comparison-draft-3`, superseding draft-2, whose bytes are kept under [`history/`](history/)). The labelled common-client poll is selected for wait-common; wait-best stays blocked. Collection is refused while `unresolved` lists anything, and protocol v2 (actual storage, enforcement, budgets and the official-artifact claim scope) is not built. |
| Wire driver: `benches/mail_compare.rs` | Built. Its polls now use the adapter's tested wait rule, and its scans refuse short nonfinal pages. A three-target smoke run against real Swarmail, Mailpit and MailHog seals and admits (`tests/mail_compare.rs`, ignored tests below). This is a smoke run, not evidence. Owned-bind corroboration and a cleanup worker that survives cancellation (T3) are **built and proved locally on Linux** (see *Method, as built*). The launch-level refusal of a foreign bind is proved through the kernel reading it rests on, not by forcing the reservation race, which needs no seam and has none. |
| Mailpit MCP adapter: `mail_compare mailpit-mcp` | **Built and proved locally.** It speaks MCP 2025-03-26 over stdio. Its tools are list, search, read and a labelled client-side poll, each a translation of the captured Mailpit API. It was tested against real Mailpit over real SMTP. Each call's time frame starts at admission (see below). Final client negotiation (#151/#152/#153) is not proved by these tests. JSON-RPC batches are refused, which falls short of 2025-03-26 (receivers must accept batches); that stays open with the final contracts. |
| One command: `scripts/compare-mail.sh` | Built. `local` runs the driver as a labelled smoke run. `controls`, `evidence` and `measure` refuse at `comparison-integration`. No mode starts an agent. |
| Eval integration: comparison target, envelope and controls (T7–T9) | **Eval sources repaired; comparison not built.** The conflict syntax #269 committed is resolved to the accepted side. Standalone fmt, locked all-target clippy and every test binary pass, each run whole and default-threaded. No comparison target, envelope or same-suite control exists. Cross-checkout #148/#201 shared-admission and CI receipts are unproved. |
| Evidence admission (T10) and the dossier (T11–T12) | **Not built, and no inputs exist.** There is no genuine #151 baseline, no reconciled final MCP contract, no final trial and no cost record. No paid trial is authorized. |

These rows are open, not waived. The story is incomplete until genuine
evidence is admitted (spec AC12–AC14). Passing local tests does not count
as comparison evidence. The historical locks stay unchanged:
[v1](history/2026-10-28-sources.lock.v1.json), the stopped candidate's
unverified draft, and [v2](history/2026-10-28-sources.lock.v2.json), whose
`source_revision` meant built-from. Each is read only by its own reader,
and nothing launches from either.

### Acceptance ledger (2026-10-29)

Every criterion is open. "Proof so far" names local tests in
`tests/mail_compare.rs` or `evals/`; none is a collection or a CI receipt.

| AC | Proof so far | Missing | Owner |
|---|---|---|---|
| AC1 method | protocol/lock validators and refusal tests; draft-2 archived | protocol v2 actual profiles, a frozen method | implementer |
| AC2 competitors | `the_tag_chain_and_every_blob_are_rechecked_against_the_captures`, real-binary tests | build provenance (upstream); finalization-time executable recheck | upstream / implementer |
| AC3 adapter | wait-frame and poll proofs on a paused clock; real SMTP → Mailpit → MCP | final client contracts (#151/#152/#153), batches | #151 / implementer |
| AC4 controls | none | comparison target and envelope, external oracle/null controls | implementer after T8 |
| AC5–AC7 throughput, waits, limits | smoke runs; scan and censoring tests | full frozen runs, actual storage/enforcement profiles, best wait | implementer / operator |
| AC8–AC9 lifecycle, privacy | launch/cleanup/privacy tests; T3 worker, hold, cancellation, forced-death, TERM-resistant, output-pressure and bind-ownership tests; real-competitor launches corroborated | CI (Linux and macOS) receipts; a forced reservation race | CI / implementer |
| AC10 schema/replay | eval suites pass on the repaired sources | comparison typing and replay | implementer |
| AC11 one command | delivery modes refuse with their phase | working controls/evidence/measure delegation | implementer |
| AC12 eval readiness | standalone fmt, clippy and full default-threaded suite | #148/#201 cross-checkout coordination; CI receipts | #148/#201, CI |
| AC13 dossier | none | genuine #151 baseline, finals and costs | operator (T11) |
| AC14 honest claims | report censoring/denominator tests | admitted data | implementer after T11 |
| AC15 gates | local house gates on the candidate | CI-owned jobs, engine verification | verify seat, CI |
| AC16 delivery integrity | none | inherited HIGH review-integrity residual with its original references, #219, five protected reviews, operator acceptance of 0090–0093 | operator, panel |

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
| `mailpit_wait` | a **client-side poll**, not a Mailpit facility: a complete paged search at once, then one 100 ms after each completed scan, until `timeout_ms` from admission; `found` (a scan that ended in time matched), `late` (first matched by a scan that ended after the cutoff; censored) or `timeout` (answered at or after the cutoff: no scan saw a match, not proof of continuous absence) | `query`, `timeout_ms` 1..30000 |

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
  never extended: a wait's last scan may finish one request bound after its
  cutoff only while that stays within the ceiling, so a 30 s wait gets no
  extra time; a call whose ceiling passed in the queue sends nothing;
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
  is retried. *Accepted per second* = accepts divided by the send window
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
unplanned row, and a row count that differs from the plan. Hashes show that
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
# three real targets against the smoke fixture (smoke, never evidence):
bash scripts/compare-mail.sh local --output target/mail-compare/smoke-3 \
  --protocol tests/mail_compare/three-target-smoke.json \
  --mailpit /path/to/pinned/mailpit --mailhog /path/to/pinned/MailHog
# the real-binary proofs (ignored by default; they fail, not skip, without the variables):
MAIL_COMPARE_MAILPIT=/path/to/pinned/mailpit MAIL_COMPARE_MAILHOG=/path/to/pinned/MailHog \
  cargo test --locked --test mail_compare -- --ignored
```

Pointing `local` at the committed draft protocol launches the full draft
workload as a smoke run, which is hours of work and still not evidence.
The driver's `check` lists every reason the draft cannot be collected.
`controls`, `evidence` and `measure` exit 2 at `comparison-integration`
until tasks T8–T10 land. `measure --dry-run` opens no wrapper configuration and launches
nothing.

## What closes the remaining rows

1. Build provenance for each executable needs genuine build evidence (an
   attestation, a reproducible build or a published build record). Until
   then a measurement needs protocol v2's declared official-artifact claim
   scope, and it makes no built-from or equivalence claim. MailHog's
   `/api/v1/messages/{id}` read still needs a primary source.
2. A best-available wait needs route, producer, registration and gap
   evidence, plus real-binary observation, before `wait-best` can run.
   MailHog's EventSource body is not captured. Protocol v2 must freeze
   Mailpit's scratch SQLite as an explicitly unmatched storage profile (or
   a matched one), and either a runtime that enforces the resource budget
   or a separately labelled unenforced profile. That revision is committed
   before any collection.
3. The comparison target, envelope and same-suite
   controls (T8–T9) and evidence admission (T10) are built in the eval
   owners. #148/#201 receive their cross-checkout shared-admission and CI
   receipts. The #151 baseline and the final MCP contracts, batch handling
   included, land.
4. Genuine, separately authorized runs are admitted with their actual cost
   records. Losses and unsupported cells are published beside the wins, and
   README numbers cite their run, task, model, configuration and denominator.

## Cost

No paid trial was run and none is authorized. Cost accounting is
*unavailable*, which is not the same as zero.
