# Evals and the fair mail comparison

Issue [#163](https://github.com/feedback-loop-ai/swarmail/issues/163) (epic #164).
Specification: [fair mail comparison](../../specs/fair-mail-comparison/spec.md).
Decision: proposed [0091](../decisions/0091-fair-mail-comparison.md), not accepted.

**Status, 2026-10-28: no results. Infrastructure only.** This directory holds
no baseline, no final run, no comparison, no cost and no loss, because none of
those exists yet. Nothing in it, and nothing in the top-level README, may be
read as a measured claim about Swarmail, Mailpit, MailHog or any agent.

## What exists, and what does not

| Deliverable (spec) | State |
|---|---|
| Wire driver: `benches/mail_compare.rs` | Built. Runs the frozen throughput and wait profiles against servers it launches and owns. It has been proved only against the real Swarmail binary (`tests/mail_compare.rs`). |
| Method: [`protocol.json`](protocol.json) | **Draft.** Collection is refused until every `unresolved` entry is closed and the status is `frozen`. |
| Source pins: [`sources.lock.json`](sources.lock.json) | **Mailpit and MailHog are unverified.** No official source, release, licence, API or setup document and no artifact digest has been captured. The delivering seat had no network access, no container runtime and no competitor binary. |
| One command: `scripts/compare-mail.sh` | Built. `local` runs the driver, while `controls`, `evidence` and `measure` refuse and name the phase that blocks them. No mode starts an agent. |
| Mailpit MCP adapter (task T5) | **Not built.** A thin adapter may expose only general upstream operations that have been checked against the source. Writing one before the official API is captured, with no Mailpit to test it against, would invent capabilities. |
| Eval integration: comparison target, envelope and controls (T7–T9) | **Not built.** The standalone eval crate does not compile: `evals/src/{artifacts,provenance,results}.rs` and `evals/tests/{identity,provenance}.rs` contain unresolved conflict syntax. Shared-host coordination for #148/#201 is unproved. |
| Evidence admission (T10) and the dossier (T11–T12) | **Not built, and no inputs exist.** There is no genuine #151 baseline, no reconciled final MCP contract, no final trial and no cost record. No paid trial is authorized. |

These rows are open, not waived. The story is incomplete until genuine
evidence is admitted (spec AC12–AC14). Passing local tests does not count
as comparison evidence.

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
that is actually enforced, and a verified pin for every competitor. The pin
needs an official repository, a full commit id (a tag alone refuses), a
permissive SPDX licence, documents under the official repository, archive and
executable SHA-256, the platform and a launch template.

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
checkout. For a comparison you also need the pinned Mailpit and MailHog
executables, whose digests must match a verified `sources.lock.json`. Use
disk-backed output and scratch paths, not a RAM tmpfs.

```bash
bash scripts/compare-mail.sh                       # usage; launches nothing
bash scripts/compare-mail.sh local --output target/mail-compare/run-1 \
  --mailpit /path/to/pinned/mailpit --mailhog /path/to/pinned/MailHog
# the candidate alone, against the test fixture (smoke, never evidence):
bash scripts/compare-mail.sh local --output target/mail-compare/smoke-1 \
  --protocol tests/mail_compare/swarmail-smoke.json \
  --sources tests/mail_compare/swarmail-only.lock.json
```

Today the first `local` line exits 2 with `blocked at phase
"competitor-binaries"`, and the driver's `check` lists every unresolved
protocol value. `controls`, `evidence` and `measure` exit 2 at `eval-build`.
Once the eval sources compile, they will exit 2 at `comparison-integration`
until tasks T7–T10 land. `measure --dry-run` opens no wrapper configuration
and launches nothing.

## What closes the remaining rows

1. An operator, with network access, captures Mailpit and MailHog from their
   official repositories only: commit, release, licence, API and setup
   documents, archive and executable digests, platform and the launch argv.
   The operator records these in `sources.lock.json` and then checks the
   HTTP mapping in `benches/mail_compare/targets.rs` against the captured API
   documents. The official repository homes that the validator expects
   (`TargetKind::official_repository`) also come from the author's own
   knowledge. They were not captured, so the first capture must confirm
   them.
2. The operator inspects each competitor's wait or stream facilities, then
   freezes the poll interval, retention and storage settings and a runtime
   that actually enforces the resource budget. A new protocol revision is
   committed before any collection.
3. The eval conflict repair is coordinated and #148/#201 receive their
   shared-host receipts. The #151 baseline and the final MCP contracts land.
   Then T5 and T7–T10 can be built and proved.
4. Genuine, separately authorized runs are admitted with their actual cost
   records. Losses and unsupported cells are published beside the wins, and
   README numbers cite their run, task, model, configuration and denominator.

## Cost

No paid trial was run and none is authorized. Cost accounting is
*unavailable*, which is not the same as zero.
