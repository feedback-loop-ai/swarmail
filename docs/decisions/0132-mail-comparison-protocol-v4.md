# 0132 — Mail comparison protocol v4: cost to operate, API level, best wait and test execution

- **Status:** accepted (operator, 2026-11-16: "Accept all as proposed"; see
  the dated acceptance note at the end)
- **Date:** 2026-11-16
- **Scope:** issues #349 (cost to operate, the README headline) and #351 (API
  level, each tool's best wait, test execution), both part of #163 (epic
  #164). Drafted from `main` `f05f2ee`.
- **Authority:** the operator's ruling of 2026-11-16, as the dispatcher's
  brief relays it: #349 and #351 go into one combined v4 decision. That is
  the brief's paraphrase, not a quotation. The same brief carries two
  framing rules from the operator, applied throughout: agent-session wall
  time is model time to first token, turns and context, and never ranks a
  mail server; and any load proof stays within 32 workers and 10 minutes.
- **Builds on:** accepted [0091](0091-fair-mail-comparison.md) §1–§7,
  [0092](0092-official-mail-comparison.md) §1–§6,
  [0093](0093-source-mail-comparison.md) §1–§7,
  [0129](0129-agent-suite-comparison.md) (adapters, bridge, product record,
  fresh-instance isolation) and [0130](0130-eight-target-mail-comparison.md)
  (nine fixed targets, source lock v4, protocol v3, conditions (f) and (g));
  accepted [0002](0002-awaits-subscribe-before-checking.md),
  [0004](0004-per-test-inboxes-via-smtp-auth.md),
  [0005](0005-extraction-at-ingest.md), [0055](0055-agent-eval-harness.md),
  [0076](0076-sequential-eval-trials.md), [0112](0112-recipient-inbox-routing.md),
  [0115](0115-trial-isolation.md) and [0117](0117-hard-eval-tier.md).
- **Amends upon acceptance, by addition and for protocol v4 only:** accepted
  0093 §4 ("Best-wait stays blocked … No new stream client is selected") and
  accepted 0130 §4 ("No stream client is added"). Protocol v4 adds a
  `wait-best` profile whose stream rows follow §7's evidence rule. Protocol
  v3's waits stay as frozen.
- **Holds:** §7's stream wake-up clients wait for the operator's acceptance of
  this record. Everything else is compatible work under accepted 0045 §8: it
  adds a protocol version and a lock version beside the frozen ones and
  changes no accepted ruling. A PR that changes `scripts/` is held for the
  operator in any case (house rules, *Commits and history*).
- **Preserves:** 0001–0005 (no file under `src/` changes), 0020, 0045 §8,
  0055 (`evals/` stays standalone and outside the coverage denominator),
  0056, 0063, 0071, 0075, 0076, 0115, 0129 §5's adapter contract (no delete,
  clear or admin tool is added to any adapter), the dependency policy (no
  package is added to `Cargo.lock` or `evals/Cargo.lock`, and `deny.toml` is
  unchanged), and the frozen bytes of protocols v2 and v3, locks v3 and v4,
  and every collection made under them.
- **Number:** 0132, the next free number on `origin/main` `f05f2ee`, where
  0131 is the highest. No open PR or remote branch carried a higher one on
  2026-11-16. The number is provisional until the dispatcher integrates it.
  Only the operator accepts.

## Context

Protocols v2 and v3 measure SMTP accept → visible throughput and a wait
profile. Neither measures what a user of a mail mock pays for.

- **Cost to operate is not measured.** The `resources` block declares 2 CPUs
  and 2048 MiB with `"enforced": false` (profile `unenforced_local`). The
  driver records no CPU time and no memory for any target. The only
  resource-shaped figure is startup time, `launches[].ready_ns`. Protocol v3
  is frozen (`229babaa…`), so cost needs a new protocol version (#349).
- **Every wait is the same poll.** `waits[0].observation` gives every
  target, Swarmail included, the common 100 ms search poll (0093 §4).
  Swarmail's native wait, which subscribes before it checks (0002), has never
  been measured. This is why the future-mail wait in the unpublished
  2026-11-07 v2 collection was about 100 ms on every tool, as #349 records.
  `wait-best` is declared blocked in `docs/evals/README.md`.
- **Per-operation API latency is not measured at all**: list, search, get,
  extract, delete and clear (#351).
- **Test execution is not measured.** The free oracle controls
  (`agent-controls-2026-11-08-main/`) solve every hard-tier task on four
  targets with no model, but they keep only pass/fail and whole-matrix time.
- **The agent suite cannot rank servers on speed.** The 2026-11-09 paid run
  (#350) passed 236 of 240 sessions with no significant gap. Its session wall
  time is model time, and #349's first comment rules it out as a server speed
  result. The README headline therefore rests on throughput, waits, test
  execution and cost to operate.

Two premises of #351 are corrected by the repository's own records. First,
#351 says GreenMail's REST API "cannot read messages"; accepted 0130's
review-return addendum records, as a correction of record, that the pinned
jar's OpenAPI lists a message read route. Second, #351 asks for client
concurrency 1, 10 and 100; the operator's bound of 32 workers rules out 100,
and §6 uses 1, 8 and 32.

## Proposed ruling

### 1. Protocol v4, lock v5, and what stays frozen

1. **Protocol v4** has the id `fair-mail-comparison-v4`, `protocol_version`
   4, and supersedes v3 with a change reason that cites this record. It is a
   separate run with a separate denominator. No table, statistic or claim
   merges a v4 cell with a v2 or v3 cell. v4 cells are collected under an
   enforced budget (§3), v2 and v3 cells under the unenforced local profile,
   and no figure is carried from one to the other.
2. **Source lock v5** is lock v4 plus two additions that only version 5 may
   carry; version 4 refuses each, and no v4 field changes meaning:
   - a launch placeholder, `{max_messages}`, in the launch argv or launch
     environment of a target that has a retention cap flag. The validator
     requires the placeholder wherever lock v4 holds a literal cap, and
     substitutes the protocol's per-target cap. This replaces v3's literal
     10,000 for Mailpit (`--max`), smtp4dev (`--messagestokeep`) and Inbucket
     (`INBUCKET_STORAGE_MAILBOXMSGCAP`).
   - the captured documents that §6 and §7 cite: each tool's operation routes
     and its wait or push channel. The rules of 0093 §1 and 0130 §2 apply to
     each document unchanged (path, size, sha256, attribution, recheck at
     use).
3. **Pairing.** Protocol v4 pairs only with lock v5. Protocol v3 still pairs
   only with lock v4, and v2 only with lock v3.
4. **History.** When v4 and lock v5 land, the v3 and lock v4 bytes move to
   `docs/evals/history/2026-11-07-protocol.v3.json` and
   `docs/evals/history/2026-11-07-sources.lock.v4.json`, sha256 unchanged.
   The tests `FROZEN_PROTOCOL_V3_SHA` and `FROZEN_LOCK_V4_SHA` follow the
   files. #349's v3 collection does not wait for v4: whichever lands first,
   v3 is collected from its own frozen bytes, by the `--documents docs/evals`
   route the README already shows for v2.

### 2. Targets and products

1. **v3's nine targets, unchanged:** `swarmail`, `mailpit`, `mailhog`,
   `mailslurper`, `mailslurper-source-build`, `smtp4dev`, `inbucket`,
   `greenmail` and `alkemio-mailslurper`, with v3's products, executable
   digests, runtimes, labels and nine cyclic rotations. Every v3 label stays
   on its row (source strength, unpinned runtimes, registry integrity, the
   third-party image).
2. **The official MailSlurper row stays `does_not_start`.** Every v4 cell of
   that row is published as blocked, with no number, citing the same failure
   document as v3.
3. **The candidate** is `cargo build --release --locked --bin swarmail` of a
   committed, clean `main`. A dirty candidate refuses v4 collection by name.
   The 2026-10-30 v2 collection was admitted as raw measurement only because
   its tree was dirty; v4 makes that a refusal. Swarmail's launch flags
   (`--max-sessions`, `--max-per-inbox`) are part of the frozen protocol and
   appear in every launch receipt, as in v2.
4. **Caps.** Every target that has a cap flag is launched with the same cap,
   150,000 messages (Swarmail `--max-per-inbox 150000`). That is at least the
   largest fill (§6, 100,000) plus every measured message. A target with no
   cap flag stays `no_cap_observed`. The storage profile stays **unmatched**:
   each target keeps its own backend, and v4 says so in every report.

### 3. Resource enforcement: cgroup v2 under a delegated user scope

Every target in every v4 section runs under one enforced budget: **2 CPUs,
2048 MiB of memory, no swap, 4096 tasks**. The mechanism is a cgroup v2 leaf
per launch, owned by the driver.

1. **Setup.** The wrapper starts each chunk's driver (§11) as
   `systemd-run --user --scope -p Delegate=yes --unit=mail-compare-<run>-<chunk> -- <driver …>`.
   The driver checks at start that it runs in a cgroup v2 hierarchy
   (`cgroup2fs`) whose delegated controllers include `cpu`, `memory` and
   `pids`. It moves itself into a leaf `<scope>/driver`, enables
   `+cpu +memory +pids` in the scope's `cgroup.subtree_control`, and creates
   one leaf `<scope>/target-<launch-id>` per launch.
2. **Limits on the target leaf:** `memory.max` 2147483648, `memory.swap.max`
   0, `memory.oom.group` 1, `cpu.max` `200000 100000`, `pids.max` 4096. The
   driver reads each value back after writing it, and records it in the
   launch receipt.
3. **Joining.** In the child's `pre_exec`, before `exec`, the child writes
   `0` to the leaf's `cgroup.procs` with `libc::open`/`libc::write`
   (async-signal-safe), then sets its CPU affinity with
   `libc::sched_setaffinity` to the chunk's two target CPUs, then applies the
   existing `setrlimit(RLIMIT_NOFILE)`. Every thread and every descendant is
   born inside the leaf and inherits the affinity.
4. **CPU placement.** cpuset is not delegated to user managers on the
   dispatcher host. It was read on 2026-11-16: `cgroup2fs`, kernel 7.0,
   systemd 259, and `user@.service` has `Delegate=pids memory cpu`. Two CPUs
   are therefore enforced twice. Affinity confines the target to two CPUs,
   which is also what Go's `GOMAXPROCS`, the JVM and .NET read as the
   processor count. `cpu.max` is the backstop if a process widens its own
   affinity, and `cpu.stat`'s `nr_throttled` records it if that happens. The
   two target CPUs are two distinct physical cores of the chunk's CPU set
   (`topology/thread_siblings_list`), and their SMT siblings stay idle. The
   clients and the driver get the rest of the set. The layout is recorded in
   `host.json` (§12).
5. **Cleanup gains a proof.** Teardown keeps the 0092 §6 / 0093 §7 sequence
   (SIGTERM to the group, grace, SIGKILL, reap, listener and scratch proofs),
   then writes `1` to the leaf's `cgroup.kill`, requires `cgroup.procs` to be
   empty, and removes the leaf. An empty leaf proves that no process escaped
   the process group, which a process-group kill alone cannot show (a child
   that calls `setsid`). `cgroup_removed` joins the cleanup receipt. If the
   leaf cannot be emptied, cleanup is unknown, and admission stops.
6. **Why this mechanism, for every target.** All nine targets run as native
   children of the driver (0130 §5): Go executables, GreenMail on the pinned
   JRE, smtp4dev's self-contained .NET bundle, Alkemio's binary on the pinned
   musl loader, and the Rust candidate. A cgroup sees processes and threads,
   not runtimes, so one mechanism enforces and accounts all of them alike,
   including the JVM's and .NET's helper threads. The driver stays the one
   owner, parent and reaper. systemd creates the driver's scope once and
   supervises nothing of the target, so this adds no second supervisor and is
   not the hostile confinement that 0093 §7 rejects. The code is `std::fs` on
   cgroupfs plus `libc`, which the bench already uses, so no package is
   added.
7. **Refusal.** Off Linux, or without a delegated cgroup v2 hierarchy with
   `cpu`, `memory` and `pids`, v4 collection refuses by name
   (`resources: no delegated cgroup v2 (cpu memory pids)`). It never falls
   back to an unenforced run. The `Resources` record of v4 states
   `enforced: true`, `profile: enforced`, `mechanism: cgroup_v2_user_scope`,
   the limits, and `cpu_placement: affinity_plus_quota`.
8. **Scratch on disk.** Each launch's owned scratch must sit on a disk-backed
   filesystem. A tmpfs scratch refuses (`statfs`), because tmpfs pages are
   charged to the leaf as `shmem` and would distort the memory figures.

### 4. What is sampled, and at which loads

1. **Per launch, continuously.** A sampler task in the driver, on the client
   CPUs, reads the target leaf every 100 ms: `memory.current`, `anon`, `file`
   and `kernel` from `memory.stat`, and `pids.current`. At the start and end
   of every measurement window it reads `cpu.stat` (`usage_usec`,
   `user_usec`, `system_usec`, `nr_throttled`, `throttled_usec`) and
   `memory.events` (`max`, `oom`, `oom_kill`). At teardown it reads the
   leaf's lifetime `memory.peak`. The sampler's own CPU is charged to the
   driver, not to the target. Samples are rows in `samples.jsonl`.
2. **Per window.** Every measured window carries its target CPU time (user,
   system, total), the sampled peak and mean of `memory.current` and `anon`,
   and the OOM and throttle counts over the window. A window is a throughput
   cell's send-and-verify window (§5), an API cell (§6), a wait profile
   instance (§7) or a test-execution suite (§8). An `oom_kill` or a
   target-side crash during a window is a measured outcome of that cell,
   `oom_killed` or `target_exited`. It is published as a row and never
   retried.
3. **Idle points.** After readiness, the driver waits 5 s with no traffic and
   records the mean `memory.current` and `anon` over the last 2 s
   (`idle_ready`). The API section records the same point after each fill
   level is reached (`idle_fill_0`, `idle_fill_1000`, `idle_fill_10000`,
   `idle_fill_100000`). Each idle point also records the scratch tree's
   allocated bytes (`st_blocks × 512`): the store's disk footprint.
4. **Startup.** `ready_ns` as today, plus the target CPU time consumed up to
   readiness.
5. **Fixed facts per target**, computed once per collection from the pinned
   bytes by a `footprint` mode, with no measurement window:
   - *download bytes*: the archive, jar or executable a user fetches. For the
     image row, the sum of the compressed layer sizes in the captured
     manifest. For the candidate, the release binary itself.
   - *installed bytes*: the executable plus every pinned runtime tree it
     needs (the extracted JRE for GreenMail, the loader for Alkemio).
   - *runtime*: what the tool needs that is not pinned (host glibc for the
     MailSlurper source build, host libraries for smtp4dev's bundle). It is
     listed as "unpinned, not counted", never counted as zero.
6. **Loads.** The `cost` section reuses v3's throughput method (closed-loop
   sessions, one delivery outstanding each, exactly one row per message)
   under the enforced budget, at four profiles. Each profile has 100 warmup
   messages and 3,000 measured messages, 3 repetitions, a fresh instance per
   profile, v3's corpus and the same send and verify deadlines:
   `cost-c1-reused`, `cost-c8-reused`, `cost-c32-reused` and
   `cost-c32-fresh`. v3's 100- and 1,000-session profiles are not repeated in
   v4: they stay v3's, and 32 is the operator's bound.

### 5. The headline metric: resources per visible mail, at equal results

1. **Visible mail.** As in v2: an accepted message whose id is observed on the
   server after the send window, with subject, text and HTML checked.
2. **Equal results.** A cell is *at equal results* when every requested
   message was accepted, every accepted message is visible, and every visible
   message passed the fidelity check: requested = accepted = visible, with no
   refusal, loss or fidelity mismatch. Only cells at equal results are ranked.
   A cell that is not at equal results still publishes its resource figures,
   with the shortfall named beside them (refused, lost, fidelity, OOM,
   timeout). Its figure is never ranked against an exact cell. On v3's
   observations, the MailSlurper family's cells will not be at equal results,
   because of body fidelity, and that will be shown.
3. **Per target and cost profile**, the median over repetitions, with the
   minimum and maximum beside it:
   - `visible_per_cpu_second`: visible mail ÷ target CPU-seconds (user plus
     system) over the window. Its inverse, `cpu_ms_per_1k_visible`, is
     published beside it.
   - `peak_memory_mib`: the sampled peak of `memory.current` over the window,
     with peak `anon` beside it.
4. **Per target, from the idle points:** `idle_memory_mib` (`idle_ready`) and
   `mib_per_1k_retained` = (`idle_fill_N` − `idle_fill_0`) ÷ N × 1000 at the
   largest fill N the target reached, labelled with that N. Each is given for
   `memory.current` and for `anon`, with the disk footprint per 1,000
   retained messages beside them.
5. **Per target, fixed:** download bytes, installed bytes, unpinned runtime,
   and startup (ready ms, and CPU-ms to ready).
6. **The proposed headline row** is `cost-c8-reused`: visible mail per
   CPU-second and peak memory under the enforced 2 CPU / 2 GiB budget, then
   idle memory, MiB per 1,000 retained messages, installed size and startup.
   Every figure cites its run id, cell and denominator (0091 §7). Which
   profile leads is the operator's ruling (see the open questions).

### 6. API level, no model

1. **Operations.** Each operation has an exact-result check. A wrong or
   partial answer is a failure row (`wrong_result`), never a fast row.
   - `list`: all mail of the measured recipient. That recipient holds K = 10
     messages. The check is that the returned id set equals the expected ten
     after the client's documented filter and paging to completion.
   - `search`: recipient AND subject token. The check is exactly the one
     expected id.
   - `get`: one message by id. Subject, from, to, text and HTML must equal
     what was sent, under v3's fidelity rules. A mismatch the v3 records
     already document is `inexact_fidelity`: its latency is kept, and the
     cell is not at equal results.
   - `extract_code` and `extract_link`: the six-digit code, and the URL under
     a fixed label, of one message. Swarmail answers from fields extracted at
     ingest (0005). For a competitor, the client reads the message and runs
     the frozen extractor (the eval oracle's `six_digit_code` and
     `labelled_url`), and that time is counted.
   - `delete`: one message by id. After the window, an untimed check confirms
     that it is gone and that the other measured mail is not.
   - `clear`: the narrowest documented clear, labelled with its `scope`:
     `recipient` (or Swarmail's inbox) where the tool offers one, else `all`.
     After the window, an untimed check confirms the scope's mail is gone and
     nothing outside a `recipient` scope is.
2. **Surfaces.**
   - `native`: each tool's documented HTTP API, with every route cited in a
     lock v5 document. Swarmail uses its REST API. GreenMail uses the REST
     read route of its pinned OpenAPI (0130's correction of record), not v3's
     POP3 path, which stays v3's.
   - `mcp`: the surface the agent suite uses. Swarmail is served natively at
     `POST /mcp`. Mailpit, MailHog and the MailSlurper source build are
     served through their 0129 adapters behind the eval runner's bridge, so
     the bridge hop is in the time and is stated. Every target speaks
     revision 2025-03-26, the adapters' only revision. smtp4dev, Inbucket,
     GreenMail and the Alkemio image have no adapter. Their `mcp` rows are
     published as `no MCP surface`.
3. **Unsupported is a row.** Each (target, surface, operation) is
   `supported` with a citation, or `not_supported` with a citation. The
   adapters' missing `delete` and `clear` are `not_supported (adapter, 0092 §3,
   0129 §5)`. A missing per-recipient listing, search or clear is
   `not_supported`, and the client does what the tool's documents allow (a
   complete scan filtered on the client, or a global clear), labelled. Per-test
   inboxes, server-side extraction and server-side waits are capability rows
   of their own. Protocol v4 freezes only when no cell is `not_inspected`.
4. **Store fill levels:** 0; 1,000; 10,000; 100,000. Fill mail is v3's corpus
   from the frozen seed, sent over SMTP at 8 reused sessions, outside every
   measured window. It goes round-robin to 100 background recipients that are
   never the measured recipient. With Swarmail's per-test inboxes (0004), and
   Inbucket's per-mailbox store, the fill therefore lands outside the
   measured inbox, while a shared-mailbox tool lists or scans it. That is the
   product difference being measured, and the report says so. A target that
   cannot reach a fill level within 300 s publishes `fill_not_reached` with
   the count reached, and its cells at that level are blocked.
5. **Client concurrency:** 1, 8 and 32 closed-loop clients, each on its own
   keep-alive connection. A cell is 20 warmup operations (discarded), then
   100 measured operations or 5 s, whichever comes first. Operations cut off
   by the 5 s bound are `unstarted` and counted. `clear` with `scope: all` is
   measured once per fill level and repetition, as the last step. Its samples
   are published individually, not as quantiles. The measured messages carry
   a code and a link: v4's corpus adds one kind, `code_link`, a multipart
   message built from the eval fixture's templates.
6. **Reported per cell:** p50, p95 and p99 latency (nearest rank, successful
   operations only), the success, failure and unstarted counts by class,
   achieved operations per second, and target CPU-µs per operation from the
   cell's `cpu.stat` delta.

### 7. Waits: `wait-best`, beside `wait-common` and `wait-tight`

1. **Three wait profiles**, each with present, future and empty instances as
   in v2 (100, 100 and 20 per repetition; a 30 s deadline; 1 s for empty), 3
   repetitions, each profile on a fresh instance under the enforced budget:
   - `wait-common`: v3's common 100 ms poll for every target, Swarmail
     included. Kept so that the poll-versus-push difference is explicit.
   - `wait-best`: each target's best documented wait (2 below).
   - `wait-tight`: the same poll at 10 ms for every target, labelled as no
     tool's documented method. It shows what a faster poll buys, and its CPU
     cost on the target, so a push-versus-poll gap is not merely the choice
     of 100 ms.
2. **Selection for `wait-best`**, in this order:
   1. `native`: a blocking server wait route that is cited. Swarmail's REST
      await (0002) is the only one known today.
   2. `stream`: a documented push channel (websocket, server-sent events or
      similar) whose route, producer call site and client registration are
      cited in lock v5 documents, and which a real-binary proof against the
      pinned bytes shows firing after a delivery. The client discipline does
      not depend on the channel being lossless. It subscribes and awaits the
      channel's cited readiness signal, then performs one complete scan, then
      rescans after every event, after every reconnect, and on a 1,000 ms
      backstop timer. The scan is authoritative, and an event only wakes it.
      Each observation records how it was seen (`initial_scan`, `event`,
      `reconnect` or `backstop`), so a dropped notification shows as a
      backstop observation. This meets 0092 §5 ("subscribe before initial
      scan, prove readiness and rescan after gaps") and removes the reason
      0093 §4 kept best-wait blocked: the reviewed Mailpit and MailHog hubs
      drop slow clients and replay nothing, so losslessness can never be
      shown, but under this discipline it does not need to be.
   3. `poll`: otherwise, the common 100 ms poll, labelled "no documented
      wait", with the citation that shows the absence (a `not_supported`
      `native_wait`) or the gap.
   The repository's records name candidates without deciding them. They are
   the Mailpit and MailHog channels already reviewed
   (`docs/evals/README.md`, *What closes the remaining rows* §2), and the
   `native_wait` rows protocol v3 leaves `not_inspected` (smtp4dev, Inbucket,
   the Alkemio image). Every row is decided from pinned bytes before freeze,
   and no upstream fact is written from memory (0130's rerun addendum).
3. **Measures.** The primary figure is SMTP `250` → the waiting client's
   return (`ack_to_return`) for future mail. v2's present and future
   latencies and its signed ack offsets are kept as defined. Each wait also
   records three outcomes: `missed` (no return by the deadline: censored, as
   v2's timeout), `duplicate` (the expected message returned more than once
   in one answer) and `unrelated` (another message returned). The target's
   CPU during each profile is in its window (§4), so the cost of tight
   polling is visible.
4. **Swarmail over MCP.** `swarmail_wait_for_email` is measured in the API
   section's `mcp` surface as a `wait` operation (future mode only, at
   concurrency 1, 8 and 32). The competitors' adapter waits are measured
   beside it: each is the labelled poll of 0129 §3.

### 8. Test execution

1. **The reference suite** is five flows of the eval fixture, solved by the
   oracle's existing scripted solutions: `otp-backlog`, `recipient-crowd` and
   `digest-lookalike` (the hard tier, 0117), and `reset-login` and
   `magic-login`. No new scenario code is written. A test is: the fixture app
   sends (the trigger), the client waits with the target's `wait-best`
   method, reads, the oracle's logic asserts and acts, the fixture's
   app-state scorer judges (0055), and the client cleans up.
2. **The run.** 100 tests per run, 20 per flow, in an order drawn from a
   frozen seed. Parallelism 1, 8 and 32: that many workers draw from one
   queue. Each worker has its own fixture app instance, started before the
   suite clock. Each run gets a fresh target instance under the enforced
   budget. There are 3 repetitions. The per-test deadline is 30 s, and the
   suite deadline is 540 s.
3. **Isolation, as each tool offers it.** Swarmail gives each test its own
   inbox through the fixture's SMTP AUTH username (0004). A shared-mailbox
   tool gives each test its own recipient, `test-<n>@suite.invalid` with `n`
   unpadded. The client uses the tool's documented filter where one is cited,
   then an exact recipient match. The recipient scheme is frozen and published
   with the protocol.
4. **Cleanup** uses the narrowest documented scope: Swarmail clears its own
   inbox; a tool with delete by id deletes the test's own messages. A tool
   with only a global clear runs it after each test at parallelism 1. At 8
   and 32 it runs no per-test cleanup and one global clear after the suite,
   because a parallel suite on one shared mailbox cannot do otherwise. That
   policy is a label on the row.
5. **Counted.** The outcome classes are `pass`, `timeout`, `wrong_message`
   (the assertion ran on mail that is not the test's own, or on the wrong
   content), `interference` (the test failed because another test's mail or
   cleanup reached it: foreign mail passed the client's filters, or its own
   accepted mail was removed by a foreign scope) and `harness` (fixture,
   transport or launcher). `foreign_seen` is reported per test: other tests'
   messages that the tool's own filter returned and the exact match dropped.
   It is an observation, not a failure.
6. **Measured:** suite wall time (first test start to last test end), per-test
   wall time, per-test wait time (wait call → return) and `ack_to_return`,
   failures by class, the target's CPU-seconds and peak memory over the suite
   window (§4), and the client process's CPU-seconds (from `rusage`) as a
   secondary column, which is where a polling client's own cost shows.

### 9. Fairness rules

These rules add to 0091's.

1. Same host, same chunk shape, same CPU layout and the same enforced budget
   for every target. Target order follows v3's cyclic rotations by
   repetition.
2. **One client per tool.** It is written to that tool's pinned documents,
   with every route cited. It is used in the API section, `wait-best` and
   test execution alike, reviewed in the protected review, and frozen by
   digest (`clients_sha256`, one per client module) before collection.
3. The generic client techniques are allowed to every tool alike: HTTP
   keep-alive, the largest documented page size, and a server-side filter
   where one is cited. No tool's client gets a cache, prefetch, index, hint or
   extraction that another tool's client is denied. Server-side extraction
   and per-test inboxes are product features, measured as such and labelled.
4. Every answer is checked exactly. Warmup is discarded. Every started
   operation keeps its row (0091 §1).
5. No tuning after data is seen. A changed method, fact or client is a new
   protocol version (0130's stop trigger S12).
6. **Losses are published with wins.** Every cell is published, including
   blocked, `not_supported`, `no MCP surface`, `fill_not_reached`,
   `oom_killed` and inexact cells. The write-up lists, before the wins, every
   metric on which Swarmail is not best.
7. Agent-session wall time is never used as a server measure, in v4 or in the
   headline.

### 10. Frozen before collection

`status: frozen` and `unresolved: []` require all of the following. Each is
held by a content hash in the tests (`FROZEN_PROTOCOL_V4_SHA`,
`FROZEN_LOCK_V5_SHA`) and recorded in the write-up.

- the protocol: sections, profiles, counts, fill levels, concurrency and
  parallelism levels, deadlines, operation definitions and their checks, the
  `code_link` corpus kind, the seed, and the test recipient scheme;
- the capability table, with no `not_inspected` cell;
- each target's `wait-best` method, with its justification and citations;
- `clients_sha256`;
- the budget, the mechanism, the CPU-layout rule and the sampling interval;
- statistics, the headline definition and the headline profile;
- the chunk plan and the per-chunk deadline;
- lock v5, including every capped target's 150,000 cap;
- the candidate commit, at collection time, in the manifest.

### 11. Collection: one command, bounded chunks

1. `scripts/compare-mail.sh collect-v4 --output DIR --bin NAME=PATH…
   --runtime NAME=DIR…` reproduces the whole v4 run. It runs a fixed,
   protocol-derived chunk plan. A chunk is one driver or eval process, for
   one of: (section `cost`, target, repetition), (`api`, surface, target,
   repetition, fill level), (`waits`, target, repetition) or (`testexec`,
   target, parallelism, repetition). Each chunk runs inside the systemd scope
   of §3, under `timeout 600` and `taskset` on at most `nproc/4` CPUs, never
   more than 32. `/proc/loadavg` is read before and after each chunk. No
   process the command starts runs longer than 10 minutes or uses more than
   32 CPUs.
2. **Size**, for 8 launchable targets and 3 repetitions: 24 `cost`, 96
   `api`/`native`, 48 `api`/`mcp`, 24 `waits` and 72 `testexec` chunks, 264
   in all. Most finish well inside their bound. The expected total is about
   one working day on the dispatcher host, and the ceiling is the bound
   times the count. The wrapper prints the plan, and `--dry-run` launches
   nothing.
3. **Exit codes.** A target's own failure in a chunk (OOM, crash, a timeout,
   a fill not reached) is a measured row, and the chunk exits 1. An
   infrastructure, cleanup or interrupt failure exits 2 and stops the run as
   incomplete. An incomplete run is never resumed, overwritten or given an
   invented suffix (0091 §6). A new run starts empty.
4. **Admission.** `mail_compare admit-v4 DIR PROTOCOL` recomputes the plan,
   requires one sealed chunk per planned coordinate and a cleanup receipt
   (with `cgroup_removed`) per launch, rechecks every executable digest, and
   recomputes every derived figure of §5 from the rows. A report that differs
   is refused, naming its first differing path.

### 12. Evidence and receipts under `docs/evals/`

- `protocol.json` (v4, frozen) and `sources.lock.json` (v5). The v3 and
  lock v4 bytes move to `history/` (§1.4).
- `sources/<target>/`: the added lock v5 documents (§1.2).
- `v4-<YYYY-MM-DD>.md`: the dated write-up. It holds the method as run,
  losses first, every table with run id and denominator, the claim scope
  (official artifacts as pinned, unmatched storage, the enforced 2 CPU /
  2 GiB budget), and the one reproduce command.
- `v4-<YYYY-MM-DD>/`, with these files:
  - `manifest.json`, `index.json` and `report.json`, plus one
    `report-<section>.json` per section;
  - `chunks.json`: every chunk's coordinate, command, exit, wall time and load
    before and after;
  - `launches.json`: every launch's argv, limits read back, CPU layout and
    cleanup receipt;
  - `footprint.json`: the fixed facts of §4.5;
  - `host.json`: kernel, systemd version, CPU model, `nproc`, the SMT layout,
    the CPU ids per role and the frequency governor;
  - `samples.sha256`: the sha256 and byte count of `samples.jsonl`, and where
    it is kept. Raw samples above 50 MiB stay out of git, as #349 does for
    the 284 MB v2 file. Smaller ones are committed.
  - `reproduce.txt`: the exact command.
- `docs/guide/benchmarks.md` and `docs/evals/README.md` point at the
  write-up. Their "no comparison figure may be quoted" lines change only by
  the operator's ruling (§13).

### 13. Publication and the README headline

1. v4 results feed #349's headline. The README claim, and lifting the "no
   figure may be quoted" lines, remain the operator's ruling. This record
   proposes only the shape and the rules.
2. **Shape.** Directly under the tagline, one table. It leads with cost to
   operate (§5.6's row), followed by:
   - the API `get` and `search` p50 and p95 at fill 10,000 and concurrency 8,
     native surface;
   - the `wait-best` future-mail `ack_to_return` p50 and p95, with each
     tool's method named;
   - test-execution suite wall time and failures at parallelism 8.
   Each figure links its run id and cell. Swarmail's losses are in the same
   table as its wins. One line says that agent success is level across
   servers (the 2026-11-09 run), and nothing ranks servers by session time.
3. Every quoted number names its denominator, configuration and claim scope
   (0091 §7), and anyone can reproduce it with the one command of §11. v2 or
   v3 throughput may be cited beside v4 only as a separately labelled
   unenforced run, never merged into a v4 figure.

## Non-goals

- No agent session, paid run or model in any v4 section. The cost of the
  agent surface (18 Swarmail tools against the adapters' 3–4) needs usage
  capture (0129 §10) and is not measured here.
- No change under `src/`, to a frozen guarantee or to any server behaviour.
  No package is added to either lock, and `deny.toml` is unchanged.
- No MCP adapter for smtp4dev, Inbucket, GreenMail or the Alkemio image, and
  no new tool in an existing adapter.
- No matched storage profile, no durability or accept==stored claim for any
  competitor (0130 §7), and no equivalence claim.
- No Docker or other container runtime, no hostile confinement, and no second
  supervisor.
- No TLS, webhook, chaos or hostile-body campaign across targets.
- No profile above 32 sessions, clients or workers. v3 keeps its 100- and
  1,000-session profiles.
- No collection on macOS or without delegated cgroup v2.
- No edit of any v2 or v3 byte, and no merge of v2, v3 and v4 cells.

## Rejected, with the cost

- **Docker or OCI limits (`--cpus`, `--memory`).** A container is the
  daemon's child, which needs the second supervisor 0092's synthesis rejects.
  Only the Alkemio row has an image: the others would need images built,
  which are artifacts no lock pins. 0130 §5 already chose native runs. Cost:
  the cgroup code is ours to write and test.
- **`setrlimit` for memory and CPU.** `RLIMIT_AS` bounds virtual address
  space, which the JVM, Go and .NET reserve far beyond what they touch, so it
  would fail them spuriously. `RLIMIT_CPU` is cumulative, not a rate.
- **`rusage` of the reaped group as the source.** It exists only after exit,
  its `ru_maxrss` is the largest single process and not the group, it gives
  no samples over time, and it enforces nothing. It stays only as the client
  process's secondary CPU column.
- **One `systemd-run --scope` per target launch.** It would create hundreds
  of transient units, and their accounting would have to be found and read
  across units. Leaves under the driver's one delegated scope keep creation,
  limits, sampling and removal in one owner.
- **Asking for cpuset delegation (`Delegate=cpuset` on `user@.service`).** It
  would be a host change the realm does not own. Affinity plus `cpu.max` is
  available today. The operator may still add it (see the open questions).
- **An unenforced, accounting-only pass beside the enforced one.** Its
  figures depend on the host's core count (128 here), so they cannot support
  "under the same budget". Cost: v4 shows no scaling beyond 2 CPUs.
- **A tight poll as the competitors' `wait-best`.** No tool's documents name
  it. It is published as its own labelled profile instead.
- **Concurrency 10 and 100 (#351).** 100 exceeds the operator's bound, and
  1, 8 and 32 match the test-execution parallelism.

## Implementation, sized as delivery stories

Story A is compatible work under 0045 §8, and so are stories B and D. Story C
waits for acceptance (*Holds*). Each story runs every house gate `--locked`.
Every real proof is `#[ignore]`d, fails rather than skips without its bytes,
and runs one at a time under `timeout 600 taskset -c 0-31` with the load
recorded. A PR that changes `scripts/` is held for the operator.

1. **A — protocol v4, lock v5, enforcement and cost.**
   - `benches/mail_compare/protocol.rs`: `PROTOCOL_VERSION_4` and its
     sections, the v4 `Resources` fields, the capability table, the headline
     definition, `clients_sha256`, lock v5's placeholder, pairing, and the
     validators and refusals.
   - new `benches/mail_compare/resources.rs`: cgroup v2 detection, leaf
     creation, limits and read-back, the `pre_exec` join and affinity, the
     sampler, the `cpu.stat`, `memory.stat` and `memory.events` parsers,
     `cgroup.kill` cleanup, and footprint facts.
   - `targets.rs`: the launcher joins the leaf, and the cleanup receipt gains
     `cgroup_removed`.
   - `runner.rs`, `records.rs`, `report.rs`, `stats.rs`: resource rows, the
     derived §5 figures, and v4 admission.
   - `benches/mail_compare.rs`: the `hold` mode (launch one target under the
     budget, print its endpoints as one JSON line, sample, and tear down with
     a receipt on stdin EOF), the `footprint` mode and `admit-v4`.
   - `scripts/compare-mail.sh`: `collect-v4` with its chunk plan, scope,
     bounds and ledger.
   - `tests/mail_compare.rs`: validator refusals and parser fixtures, plus
     real proofs that a memory hog is `oom_killed` at 2 GiB, that a spinner
     stays within 2 CPUs, and that a `setsid` escapee is removed by
     `cgroup.kill`.
   - The v4 section of `docs/evals/README.md`.
2. **B — API level.**
   - `benches/mail_compare/clients/`: one module per API family, holding the
     native operations with typed exact results, every route cited.
   - `benches/mail_compare/api.rs`: fill, cells and checks for the `native`
     surface.
   - The eval crate compiles the same client modules by `#[path]`, using only
     packages both locks already hold, so each tool has one client. Its
     `api-mcp` subcommand (`evals/src/apibench.rs`, `evals/src/main.rs`)
     measures the `mcp` surface through `McpClient` and the bridge, against a
     target held by `mail_compare hold` or by an adapter.
3. **C — `wait-best` (waits for acceptance).**
   - The lock v5 wait documents.
   - Closing every `not_inspected` `native_wait` row.
   - `clients/stream.rs`: a minimal RFC 6455 client and a server-sent-events
     reader on the tokio already in the tree, with the accept key checked
     through `ring`, which is already locked.
   - `Observation::Stream` and the backstop.
   - The `wait-tight` profile.
   - A real-binary proof per stream row.
4. **D — test execution.**
   - `evals/src/testexec.rs` and a `MailSource` over the shared clients
     (`evals/src/native_source.rs`), reusing the fixture, the oracle
     solutions and the scorer unchanged.
   - The parallel queue, the isolation and cleanup policies, and outcome and
     `foreign_seen` accounting.
   - The `testexec` subcommand.
   - A real Swarmail run and one real competitor run, each bounded.
5. **E — freeze and collection (operator-run, after A–D merge).**
   - Freeze v4 and lock v5.
   - Run `collect-v4` on committed, clean `main`, admit, and publish under
     §12.
   - The README headline follows as the operator's separate ruling (§13).

## Proof and delivery

Each story adds this record's index row if it is missing, and validates the
decision inventory (`tests/store_contract_docs.rs`). No accepted decision's
body is edited. Pointer addenda on 0093 and 0130 are added only when this
record is accepted. No paid work, collection, push, merge, publication, tag,
issue closure or acceptance is authorized by this record.

## Open questions for the operator, at acceptance

1. **Concurrency 1, 8 and 32** for the API section, in place of #351's 1, 10
   and 100. Proposed because of the 32-worker bound.
2. **One enforced profile only.** Should v4 also run an unenforced,
   accounting-only pass?
3. **The headline profile.** `cost-c8-reused` is proposed. Should a cell that
   is not at equal results (for example the MailSlurper family's fidelity)
   appear unranked, or not appear?
4. **`wait-tight` at 10 ms.** Keep it, or drop it.
5. **The stream backstop interval** of 1,000 ms.
6. **The 150,000 cap.** It changes Mailpit's, smtp4dev's and Inbucket's
   launches from v3's 10,000 through lock v5's placeholder.
7. **Enforcement.** Confirm that a driver-owned cgroup leaf is the "requested
   enforced profile" 0092 §2 anticipates, and not the "machinery to
   manufacture resource/storage parity" its synthesis rejects. Storage stays
   unmatched. Alternatively, add `Delegate=cpuset` to `user@.service` on the
   collection host, so that CPUs are pinned by the cgroup itself.
8. **GreenMail's v4 read path:** REST, per 0130's correction of record. v3
   keeps POP3.
9. **Test cleanup** for a tool with only a global clear: per test at
   parallelism 1, once after the suite at 8 and 32.
10. **Time.** About one working day of host time for 264 bounded chunks at 3
    repetitions. Fewer repetitions shorten it.
11. **Raw samples:** commit them below 50 MiB, keep them out of git above.
12. **MCP adapters** for smtp4dev, Inbucket, GreenMail and the Alkemio image
    stay a non-goal. Their `mcp` rows read `no MCP surface`.

## Acceptance — 2026-11-16 (operator)

The operator accepted this record as proposed ("Accept all as proposed"),
answering every open question above with the record's own proposal:

1. API concurrency is 1, 8 and 32.
2. One enforced profile only; no unenforced accounting pass.
3. The headline profile is `cost-c8-reused`. A cell not at equal results is
   published unranked, with its shortfall named.
4. `wait-tight` at 10 ms is kept.
5. The stream backstop interval is 1,000 ms.
6. The 150,000 cap holds, through lock v5's placeholder.
7. A driver-owned cgroup v2 leaf is the requested enforced profile of 0092
   §2. Storage stays unmatched, and the collection host keeps its current
   delegation (no `Delegate=cpuset`); CPUs are pinned by affinity.
8. GreenMail is read over REST in v4; v3 keeps POP3.
9. A tool with only a global clear cleans per test at parallelism 1, and once
   after the suite at 8 and 32.
10. About one working day of host time at 3 repetitions is accepted.
11. Raw samples are committed below 50 MiB and kept out of git above.
12. MCP adapters for smtp4dev, Inbucket, GreenMail and the Alkemio image stay
    a non-goal.

Story C no longer waits: with acceptance, stories A to D are all admissible
work, and the pointer addenda on 0093 and 0130 are added with this note.
Story E (freeze and collection) stays operator-run after A to D merge, and
the README headline stays a separate operator ruling (§13).

## Addendum 2026-11-18 (operator ruling): host findings of story B

The first delegated-host runs of story B's real proofs raised two questions
the accepted text did not answer. The operator ruled on both on 2026-11-18;
the text above is unchanged.

- **MailSlurper's rewritten single body is `inexact_fidelity`.** Under v3's
  record that MailSlurper "keeps one decoded body" and that its "body
  fidelity is reported per message, not exact": for the MailSlurper family,
  a multipart message answered with one body (the HTML part, possibly with
  attributes such as `rel="nofollow"` added) and the other part absent is
  `inexact_fidelity`. Its latency is kept, and the cell is not at equal
  results. The ruling is family-specific and otherwise exact: a wrong
  subject, sender or recipient, or a body that is not the sent HTML apart
  from the documented attribute rewrite, stays `wrong_result`. The frozen
  extractors read the kept HTML, as text.
- **An idle gap before each MCP cell.** Swarmail's MCP tool rate is a fixed
  window of 1,024 calls a second, and on the host a fast chunk filled one
  window within a second and every later cell's warmup was refused in it.
  Before each MCP cell the surface now idles at least one second, outside
  the warmup and every measured or resource window, and within the chunk's
  deadline (a cell the deadline reaches during the gap is cut, never
  started). Every cell therefore starts in a fresh window, and a refusal it
  shows is its own load's. The native surface has no such limit and no gap.

## Addendum 2026-11-19 (operator ruling): story B's open host rows and MCP cleanup

Story B's final host run (PR #383, 13 proofs at 050b522) left two questions
the accepted text did not settle. The operator ruled on both on 2026-11-19;
the text above is unchanged.

- **The source-built MailSlurper's API routes report `fill_not_reached`.**
  At §6.4's 1,000-message fill, the source-built MailSlurper never shows
  every message it accepted within 300 s, natively or through its bridge.
  That is §6.4's documented outcome, and it is these two routes' result at
  the protocol fill. The fill is not lowered for one tool. A lower level
  would no longer measure what every other target is measured on. The
  routes' operation matrix, exactness and waits are therefore unmeasured,
  and the report says so. The native and bridged host proofs pass exactly
  on that outcome, and on nothing weaker. A run that reaches the fill is
  still measured and judged like any other.
- **An MCP chunk's bridge and adapter cleanup is proved, not excepted.**
  collect-v4 keeps the holder as the only process of its chunk's delegated
  scope. `api-mcp`, the bridge and the adapters run outside it, and their
  cleanup today rests on `timeout`'s process-group signal. Before story E's
  collection, a bounded check after every MCP chunk proves that no process
  the chunk started survives it. A survivor is an infrastructure failure
  (exit 2, unsealed). The work is #385. Until it lands, no MCP chunk is
  collected.
