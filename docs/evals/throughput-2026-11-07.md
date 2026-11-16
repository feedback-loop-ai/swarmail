# Throughput comparison, 2026-11-07: Swarmail, Mailpit and MailHog (protocol v2)

This page publishes the frozen protocol v2 collection of 2026-11-07. It is
the first work item of #349: publish the run with its losses, and keep the
raw samples out of git with a recorded digest. It is **raw measurement**, not
a claim. The report says `"evidence": false`, the storage profile is
explicitly **unmatched**, the resource profile is **unenforced local**, and
the claim scope is the official release executables as shipped. What the
top-level README may claim from it is for a proposed decision the operator
accepts (#349, "Settle what the README may claim"). Until then this page is
cited, and the README quotes nothing from it.

The run is `collect-20261107T112319Z-3344019`. It used protocol
`fair-mail-comparison-v2`, on committed main `c95386e` with a clean tree
(`candidate_dirty` false), and wrote 759,944 rows. It is the first v2
collection of committed bytes. The 2026-10-30 collection ran on an
uncommitted tree (`docs/evals/README.md`, *Collection (2026-10-30)*). It
stays as recorded and is not merged with this one.

**Admission: admitted.** On 2026-11-16, `mail_compare admit <run dir>
docs/evals/history/2026-10-30-protocol.v2.json` exited 0 with "admitted with
759944 rows", once with the driver built at main `f05f2ee` and once with the
driver built at `c95386e`, the collecting commit. Admission recomputes the plan
from the protocol and checks every artifact against `index.json`. It requires
a proved cleanup receipt for each of the 270 launches, and it rebuilds every
throughput and wait cell from the rows and compares it with `report.json`.
As a negative check, admitting the same directory against the current
protocol v3 is refused ("the manifest names a different protocol", exit 2).

## Outcome

- **Swarmail's median verified rate is the highest of the three in all seven
  throughput profiles.** The gap runs from 1.55× MailHog at one fresh session
  (2,661/s against 1,717/s) to 5.8× MailHog and 11.3× Mailpit at 100 reused
  sessions (29,370/s against 5,050/s and 2,604/s). Each median is over six
  repetitions of 3,000 messages.
- **Every repetition's lowest Swarmail rate beats every other target's
  highest rate**, in every profile (the min–max ranges in the table below do
  not overlap).
- **Mailpit's counts are not exact at 1,000 fresh sessions (`stress-c1000-fresh`),
  0 of 6 repetitions.** 142 of 18,000 requested messages were refused, all
  with Mailpit's own `421` before the terminator. Swarmail and MailHog were
  exact in all six repetitions. Every other cell of the 126 is exact.
- **The waits decide nothing.** Every target was waited on with the same
  100 ms poll, so the future-mail wait is about 100 ms for all three.
  Swarmail's push wait was not measured.
- **The run measured no cost to operate.** The 2-CPU, 2,048 MiB budget was
  declared but not enforced, and CPU time and memory were not recorded.

## What ran

| | |
|---|---|
| Run | `collect-20261107T112319Z-3344019`, `mode: collect`. Prelaunch 2026-11-07 11:23:19 UTC; sealed about 14 minutes later |
| Protocol | `fair-mail-comparison-v2`, sha256 `735da43a…a82231d`, byte-identical to [`history/2026-10-30-protocol.v2.json`](history/2026-10-30-protocol.v2.json) |
| Source lock | lock v3, sha256 `6523abf4…b03c`, byte-identical to [`history/2026-10-30-sources.lock.v3.json`](history/2026-10-30-sources.lock.v3.json) |
| Candidate | `c95386e` ("Prepare the v0.4.0 release"), clean tree. The wrapper built the `swarmail` binary (sha256 `022517ef…51ad74`) and launched it as `serve … --max-sessions 1000 --max-per-inbox 100000` |
| Mailpit | v1.32.0 official executable, sha256 `262cfa28…a8edec8` (equal to the lock; upstream archive digest). Launched as shipped with `--database {data_dir}/mailpit.db --max 10000` and the lock's fixed flags, and no session setting |
| MailHog | v1.0.1 official executable, sha256 `e2ed634d…d09c7` (equal to the lock's pin; **locally observed**, since no upstream receipt ties it to the tag). Launched with `-storage memory` |
| Throughput profiles | seven profiles: 1, 10 and 100 sessions, each with fresh and reused connections, and a stress profile of 1,000 fresh sessions. Each profile runs 6 repetitions × 3,000 measured messages per target, after 100 warmup messages on a separate, discarded instance. Each measured cell gets a freshly launched instance |
| Wait profile | `wait-common`: 6 repetitions × (100 present, 100 future, 20 empty) per target, 30 s deadline (1 s for empty), observed by the common 100 ms poll |
| Order | targets run one at a time. Each repetition uses one of the protocol's six rotations of the three targets, so each target runs first, second and third twice |
| Host | Linux 7.0.0-38-generic, x86_64. The manifest records 32 CPUs (the driver's available parallelism). The driver and every target shared those CPUs. No per-target limit applied |
| Launches | 270 (90 per target), each with a proved cleanup receipt (reaped, group gone, listeners closed, scratch removed) |

Two rates are reported for each cell (`docs/evals/README.md`, *Method*):

- *Verified per second* is the accepted ids found on the server, divided by
  the time from the first send start to the end of verification. This is the
  rate at which mail became visible to a reader.
- *Accepted per second* is the `250`s received, divided by the send window.

After both windows, every visible id was checked for its subject, text and
HTML. The run found no fidelity mismatch, no duplicate, no ambiguous send,
no unstarted send and no accepted message missing on any target.

## Throughput: verified per second

Each cell shows the median over the six repetitions, then the min–max range,
then admitted/requested summed over the six, then the number of exact
repetitions. For an even count of six, the median is the mean of the third
and fourth values. The values are rounded to whole messages per second.

| Profile | Sessions | Swarmail | Mailpit | MailHog |
|---|---|---|---|---|
| `throughput-c1-fresh` | 1 | 2,661 (2,210–2,853) · 18000/18000, 6/6 exact | 1,209 (1,156–1,234) · 18000/18000, 6/6 exact | 1,717 (1,612–1,914) · 18000/18000, 6/6 exact |
| `throughput-c10-fresh` | 10 | 11,386 (8,932–13,147) · 18000/18000, 6/6 exact | 2,481 (2,370–2,518) · 18000/18000, 6/6 exact | 3,762 (2,812–4,119) · 18000/18000, 6/6 exact |
| `throughput-c100-fresh` | 100 | 19,341 (16,216–20,437) · 18000/18000, 6/6 exact | 2,463 (2,394–2,474) · 18000/18000, 6/6 exact | 3,522 (2,859–4,359) · 18000/18000, 6/6 exact |
| `throughput-c1-reused` | 1 | 5,711 (5,311–6,060) · 18000/18000, 6/6 exact | 1,556 (1,524–1,636) · 18000/18000, 6/6 exact | 3,113 (2,805–3,513) · 18000/18000, 6/6 exact |
| `throughput-c10-reused` | 10 | 20,666 (16,102–23,802) · 18000/18000, 6/6 exact | 2,648 (2,539–2,683) · 18000/18000, 6/6 exact | 5,214 (4,244–5,743) · 18000/18000, 6/6 exact |
| `throughput-c100-reused` | 100 | 29,370 (21,201–31,505) · 18000/18000, 6/6 exact | 2,604 (2,512–2,642) · 18000/18000, 6/6 exact | 5,050 (4,247–5,395) · 18000/18000, 6/6 exact |
| `stress-c1000-fresh` | 1000 | 17,813 (13,341–18,800) · 18000/18000, 6/6 exact | **2,420 (2,372–2,480) · 17858/18000, 0/6 exact** | 3,658 (2,763–3,982) · 18000/18000, 6/6 exact |

Swarmail has the widest spread. At `throughput-c100-reused`, its slowest
repetition (21,201/s) is 28% below its median. Every rate is one repetition
on one host with no per-target limit, so the ranges are the measured spread
and not a confidence interval.

## Throughput: accepted per second and send latency

The table shows the median accepted rate over the six repetitions, with its
min–max range. The last column is the median of the six per-repetition
nearest-rank p50 send latencies (`ack − start` per accepted send), in
milliseconds, given as Swarmail / Mailpit / MailHog. The denominators are
those of the table above.

| Profile | Swarmail | Mailpit | MailHog | Send latency p50, ms |
|---|---|---|---|---|
| `throughput-c1-fresh` | 2,741 (2,324–2,926) | 1,237 (1,183–1,263) | 1,814 (1,690–2,027) | 0.17 / 0.46 / 0.20 |
| `throughput-c10-fresh` | 12,803 (9,783–14,884) | 2,597 (2,478–2,648) | 4,261 (3,067–4,706) | 0.36 / 1.92 / 1.23 |
| `throughput-c100-fresh` | 24,302 (21,758–26,697) | 2,583 (2,506–2,593) | 3,954 (3,108–5,039) | 1.80 / 27.56 / 16.90 |
| `throughput-c1-reused` | 6,174 (5,555–6,462) | 1,603 (1,570–1,688) | 3,430 (3,055–3,898) | 0.15 / 0.45 / 0.21 |
| `throughput-c10-reused` | 26,011 (19,660–30,593) | 2,784 (2,670–2,824) | 6,181 (4,886–6,859) | 0.33 / 1.93 / 1.52 |
| `throughput-c100-reused` | 40,995 (33,954–43,882) | 2,736 (2,636–2,779) | 5,920 (4,836–6,321) | 1.62 / 26.39 / 14.47 |
| `stress-c1000-fresh` | 21,758 (19,270–23,015) | 2,539 (2,489–2,604) | 4,102 (3,003–4,467) | 4.34 / 238.19 / 158.63 |

A target's verified rate is below its accepted rate because the verified
window also includes the time to find every accepted id on the server.

## Exact-count failures

There is one, Mailpit at `stress-c1000-fresh`, with 3,000 messages
requested per repetition:

| Repetition | 0 | 1 | 2 | 3 | 4 | 5 | Total |
|---|---|---|---|---|---|---|---|
| Admitted | 2,977 | 2,981 | 2,973 | 2,982 | 2,972 | 2,973 | 17,858 |
| Refused | 23 | 19 | 27 | 18 | 28 | 27 | **142** |

All 142 refusals are rows with `failed_before_terminator`, code `421` and
reason `connect: refused`. That is Mailpit's own refusal of the session,
which the method records and never retries (`docs/evals/README.md`,
*Method*). None is ambiguous or unstarted, and every admitted message was
found on the server. The launches are asymmetric, as the method states.
Swarmail runs with `--max-sessions 1000`, since its compiled cap of 256
would refuse this profile. Mailpit runs as shipped, because no documented
setting in the pinned release governs the limit. This run does not establish
the cause of the refusals. The 2026-10-30 collection (uncommitted tree) saw
9 such refusals in 3 of its 6 repetitions of the same profile. Here there
are 142, in all 6.

## Waits

The waits use the common 100 ms poll on every target, over 600 present,
600 future and 120 empty tokens per target, across the 6 repetitions.

| Target | Present p50 / p95 / p99 | Future p50 / p95 / p99 | Future − ack, p50 | Empty | Poll requests |
|---|---|---|---|---|---|
| Swarmail | 0.110 / 0.198 / 0.365 ms | 101.34 / 101.91 / 103.21 ms | 100.77 ms | 120/120 timeout | 3,000 |
| Mailpit | 0.699 / 0.819 / 1.069 ms | 103.08 / 103.71 / 103.93 ms | 102.31 ms | 120/120 timeout | 3,000 |
| MailHog | 0.342 / 0.683 / 0.933 ms | 102.37 / 103.01 / 103.40 ms | 102.07 ms | 120/120 timeout | 2,999 |

All 1,800 present and 1,800 future waits were observed. None timed out or
was late, and none returned an unrelated message. The future wait is the
poll interval: a mail sent just after a scan is found on the next scan,
about 100 ms later, whatever the server. The present wait is one search
request against a store of at most 200 messages. **Neither ranks the
servers' waits.**

## Caveats

- **The waits are the common 100 ms poll for every target, Swarmail
  included.** Swarmail's native push wait (`await`, which subscribes before
  it checks) was not measured. `wait-best` is not a declared v2 profile. The
  API-level and best-wait comparison is #351.
- **No cost to operate.** The protocol declares a per-target budget of
  2 CPUs and 2,048 MiB with `"enforced": false` (`unenforced_local`). The
  driver applied no limit, and it recorded no CPU time and no memory per
  target. The driver and the target shared the 32 CPUs it saw. A target that
  spends more cores could rank higher here and lower per CPU-second. Results
  per CPU-second, peak memory and an enforced-budget run are protocol v4 work
  (#349). Until that work lands, this run says nothing about efficiency.
- **Startup time is not in this record.** `launches[].ready_ns` is the run
  clock's reading when readiness was observed, and no spawn time is recorded
  beside it, so it is not a startup duration.
- **Storage is unmatched.** Mailpit writes an SQLite file in its scratch
  directory (`--max 10000`). Swarmail (cap 100,000) and MailHog keep mail in
  memory. Each measured instance received 3,000 messages, below every cap, so
  nothing was pruned. The run claims no durability, no accept-equals-stored
  and no equivalence between the products.
- **Session limits are asymmetric**, as described under the exact-count
  failures above.
- **The MailHog executable is locally observed.** Its digest is the lock's
  pin, but no upstream receipt ties it to the v1.0.1 tag. Neither executable
  carries a built-from claim.
- **One host, one run.** These are six repetitions on one machine with no
  per-target CPU or memory limit. A new collection is a new run with its own
  denominator, and no figure here is merged with the 2026-10-30 collection or
  with any protocol v3 cell.
- **Losses and refusals are published with the wins.** Mailpit's 142
  refusals are above. Neither Swarmail nor MailHog lost a message, and no
  profile has Swarmail below a competitor.

## Data

Beside this page, under [`throughput-2026-11-07/`](throughput-2026-11-07/):

- `index.json`, `manifest.json` and `report.json`: the run's own sealed
  files, byte-identical. Their sha256 values are `8217d164…881b2d` for the
  index, `cac66c37…386363` for the manifest and `fbd134e3…dd94` for the
  report. The last two match `index.json`. The manifest lists the plan;
  the report lists every cell with its quantiles, every launch's argv
  template and every cleanup receipt.
- [`summary.json`](throughput-2026-11-07/summary.json): every figure on this
  page, by profile and target. It gives the median, min, max and count of
  each rate, with the summed denominators, and the wait cells.
- [`runs.jsonl`](throughput-2026-11-07/runs.jsonl): the 126 per-repetition
  throughput cells, one line each.
- [`summary.jq`](throughput-2026-11-07/summary.jq): the program that
  derives both files from `report.json`. It only groups cells that admission
  has already proved to be the arithmetic of the rows.

**The raw samples are not committed.** `samples.jsonl` holds 759,944 rows,
284,097,067 bytes, sha256
`70d0c4f83f6b4c1cd0c23eab2b8526512277b7935885aac675679f9a68219780` (the
`index.json` entry). It is kept on the dispatcher host that collected it,
under `swarmail-landing/compare-out-2026-11-07/collect-1/`, beside its
original `index.json`, `manifest.json` and `report.json`. The rows carry
tokens, outcomes, reply codes and monotonic timestamps. They carry no argv,
path, credential or message body.

## Reproduce

Check the committed copies, then admit and summarize the run directory
(`DIR`, which holds the four files including `samples.jsonl`), from main:

```bash
(cd docs/evals/throughput-2026-11-07 && sha256sum manifest.json report.json)   # equal to index.json's entries
B=$(cargo bench --locked --offline --no-run --bench mail_compare --message-format=json 2>/dev/null \
  | grep -o '"executable":"[^"]*mail_compare[^"]*"' | tail -n1 | cut -d'"' -f4)
"$B" admit DIR docs/evals/history/2026-10-30-protocol.v2.json    # exit 0: admitted with 759944 rows
jq -S -f docs/evals/throughput-2026-11-07/summary.jq DIR/report.json > summary.json
jq -c --argjson runs true -f docs/evals/throughput-2026-11-07/summary.jq DIR/report.json > runs.jsonl
```

To collect a new v2 run (a separate run with its own denominator, about
14 minutes, with the pinned executables at the lock's digests), run this
from a checkout of `c95386e`, where `protocol.json` and `sources.lock.json`
are the v2 and lock v3 bytes:

```bash
bash scripts/compare-mail.sh collect --output DIR --mailpit /path/to/pinned/mailpit --mailhog /path/to/pinned/MailHog
```

On main, name the historical files instead:
`--protocol docs/evals/history/2026-10-30-protocol.v2.json --sources
docs/evals/history/2026-10-30-sources.lock.v3.json --documents docs/evals`.
`collect` exits 1 when the run is sealed and admitted but a workload
failed somewhere, as Mailpit's stress cells did here. Exit 2 means the run did
not complete.
