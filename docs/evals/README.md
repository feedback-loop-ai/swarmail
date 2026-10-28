# Evals and the fair mail comparison

Issue [#163](https://github.com/feedback-loop-ai/swarmail/issues/163) (epic #164).
Specification: [fair mail comparison](../../specs/fair-mail-comparison/spec.md).
Decisions: proposed [0091](../decisions/0091-fair-mail-comparison.md) and
proposed [0092](../decisions/0092-official-mail-comparison.md)
([continuation spec](../../specs/official-mail-comparison/spec.md)); neither
is accepted.

**Status, 2026-10-28: no results.** This directory holds no baseline, final
run, comparison, cost or loss record, because none of those exists yet.
Nothing in it, or in the top-level README, may be read as a measured claim
about Swarmail, Mailpit, MailHog or any agent. The official competitor
artifacts are now pinned and every local proof below ran against the real
executables. These are untimed checks and labelled smoke runs, not
collection.

## What exists, and what does not

| Deliverable (spec) | State |
|---|---|
| Source pins: [`sources.lock.json`](sources.lock.json) (lock v2) | **Artifacts pinned, sources incomplete.** Mailpit v1.32.0 and MailHog v1.0.1 are pinned by asset and executable digest. Captured release metadata, licences and API documents are committed under [`sources/`](sources/) and rechecked byte for byte whenever the lock is used. Still missing, and named in the lock: each full source revision, document acquisition attribution, a setup document and any wait/stream source. Collection refuses until these are closed. |
| Method: [`protocol.json`](protocol.json) | **Draft** (`fair-mail-comparison-draft-2`, superseding draft-1). Competitor retention is stated as launched, and a 100 ms common-client poll interval is proposed. Collection is refused while `unresolved` lists anything. |
| Wire driver: `benches/mail_compare.rs` | Built. On 2026-10-28 it ran a three-target smoke run against real Swarmail, Mailpit and MailHog. Every send was reconciled and every message was visible once with faithful stored bodies. Waits were observed or timed out as expected, and the run was sealed and admitted (`tests/mail_compare.rs`, ignored tests below). This was a smoke run, not evidence. |
| Mailpit MCP adapter: `mail_compare mailpit-mcp` | **Built and proved locally.** It speaks MCP 2025-03-26 over stdio. Its tools are list, search, read and a labelled client-side poll, each a translation of the captured Mailpit API. It was tested against real Mailpit over real SMTP. Final client negotiation (#151/#152/#153) is not proved by these tests. JSON-RPC batches are refused, which falls short of 2025-03-26 (receivers must accept batches); that stays open with the final contracts. |
| One command: `scripts/compare-mail.sh` | Built. `local` runs the driver as a labelled smoke run. `controls`, `evidence` and `measure` refuse and name the blocking phase. No mode starts an agent. |
| Eval integration: comparison target, envelope and controls (T7–T9) | **Not built.** The standalone eval crate does not compile: `evals/src/{artifacts,provenance,results}.rs` and `evals/tests/{identity,provenance}.rs` contain unresolved conflict syntax. Shared-host coordination for #148/#201 is unproved. |
| Evidence admission (T10) and the dossier (T11–T12) | **Not built, and no inputs exist.** There is no genuine #151 baseline, no reconciled final MCP contract, no final trial and no cost record. No paid trial is authorized. |

These rows are open, not waived. The story is incomplete until genuine
evidence is admitted (spec AC12–AC14). Passing local tests does not count
as comparison evidence. The stopped candidate's unverified v1 lock is kept
unchanged at [`history/2026-10-28-sources.lock.v1.json`](history/2026-10-28-sources.lock.v1.json).
Only its own reader reads it, and nothing launches from it.

## Competitor sources and observed behavior

Checked on 2026-10-28 inside the delivery sandbox. There was no network, so
nothing was fetched or authenticated upstream. The hashes show local
consistency only.

| Fact | Mailpit | MailHog |
|---|---|---|
| Release | [v1.32.0](https://github.com/axllent/mailpit/releases/tag/v1.32.0), published 2026-10-22; `target_commitish` is the branch `develop` | [v1.0.1](https://github.com/mailhog/MailHog/releases/tag/v1.0.1), published 2020-08-11; `target_commitish` is the branch `master` |
| Asset | `mailpit-linux-amd64.tar.gz`, sha256 `4b5993c978e8604c70d43927e2ad103b311bba3b1822fa0cf1d4ee4ec890a93c`, **equal to the digest in the release metadata** | `MailHog_linux_amd64`, sha256 `e2ed634ded49929f089b20045581955ed217672078fd86082dd7a6c67c5d09c7`, **locally computed**, because the release metadata publishes no digest |
| Executable | archive member `mailpit`, sha256 `262cfa28fae4554be198d1fea5ef83c0f8299317876d658623711c629a8edec8`; `version` prints `mailpit v1.32.0 compiled with go1.27.3 on linux/amd64` | the asset itself; `-version` prints `MailHog version: 1.0.1` |
| Licence | MIT, Ralph Slooten ([text](sources/mailpit/mailpit-license.txt), byte-identical to the archive's `LICENSE`) | MIT, Ian Kent ([text](sources/mailhog/mailhog-license.txt)) |
| API document | OpenAPI 2.0 "Mailpit API" v1 ([captured](sources/mailpit/mailpit-swagger.json)) | Swagger 2.0 "MailHog API" 2.0.0 ([captured](sources/mailhog/mailhog-swagger.json)) |
| Storage as launched | `--database` in the owned scratch (a SQLite file, **not memory**) and `--max 10000`. Without the flag Mailpit keeps 500, which cannot hold a 3,000-message exact count. | `-storage memory`; `-h` lists no retention cap |
| Wait or stream facility | None in the captured documents; not reviewed. Absence from Swagger is not proof that none exists. | None in the captured documents; not reviewed |

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
| `mailpit_wait` | a **client-side poll**, not a Mailpit facility: a complete paged search at once and then every 100 ms; `found` with every match, or `timeout`, each only after a complete scan | `query`, `timeout_ms` 1..30000 |

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
- a 30 s operation deadline (a wait gets its timeout plus one request bound);
- a 10 s output and cleanup bound.

A repeated ID, a moving count, a short or empty page before the count, or a
wrong `start` echo is an error, never a shorter success. Cancellation and
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
and a complete pin for every competitor. Lock v2 validates each pin: the
official repository, tag page and release-asset URL; a full commit id (a tag
or branch refuses); a digest basis that agrees with the release metadata (an
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
the HTTP listing before the startup deadline. Teardown sends SIGTERM and then
SIGKILL to the group, reaps the child and checks that the group and both
listeners are gone and the scratch is removed. If cleanup cannot be proved,
all later work stops. A cancelled run kills its group but proves nothing, so
it is never sealed.

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
  a timeout, which is censored. The signed offset observed − ack is kept as
  is and is never clamped. An answer carrying any other message is
  `unrelated`. *Native* (Swarmail's await) and *poll* (the common client
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
`controls`, `evidence` and `measure` exit 2 at `eval-build`. Once the eval
sources compile, they will exit 2 at `comparison-integration` until tasks
T7–T10 land. `measure --dry-run` opens no wrapper configuration and launches
nothing.

## What closes the remaining rows

1. An operator with network access binds each competitor's full source
   revision for the pinned tag. The operator also binds the acquisition URL
   and revision of every captured document and captures a setup document.
   The tag and the branch named by `target_commitish` cannot stand in for a
   revision. MailHog's `/api/v1/messages/{id}` read needs a primary source.
2. The operator reviews each competitor's wait or stream source (or
   documents its absence) before any best-available or adapter wait claim.
   They then freeze the poll interval, either a storage profile in which
   Mailpit's scratch SQLite is explicitly unmatched or a matched one, and
   either a runtime that actually enforces the resource budget or a
   separately labelled unenforced profile. That becomes a new protocol
   revision (with the v2 fields of decision 0092 §2), committed before any
   collection.
3. The eval conflict repair is coordinated and #148/#201 receive their
   shared-host receipts. The #151 baseline and the final MCP contracts land,
   including batch handling. Then T7–T10 can be built and proved.
4. Genuine, separately authorized runs are admitted with their actual cost
   records. Losses and unsupported cells are published beside the wins, and
   README numbers cite their run, task, model, configuration and denominator.

## Cost

No paid trial was run and none is authorized. Cost accounting is
*unavailable*, which is not the same as zero.
