# Mail comparison protocol v4, story A: lock v5, enforced resources and cost to operate

Date: 2026-11-16. Status: draft. Issue #359, part of #349 and #351 (#163,
epic #164). Run: `issue-359-comparison-protocol-v4-b0a29b93`. Branch
`feat/compare-v4-resources-359`, from `e23a8ba`. Recipe: `claude-careful`.

Authority: accepted
[0132](../../docs/decisions/0132-mail-comparison-protocol-v4.md) (operator,
2026-11-16), § Implementation story A, with §1, §3–§5, §10–§12 and the
acceptance note. This spec declares no new decision. It implements an accepted
record and changes no ruling; where it reads 0132 one way rather than another,
[*Readings*](#readings-of-0132-this-design-commits-to) says so.

Other authorities (accepted): 0091 §1, §6 and §7; 0092 §2 and §6; 0093 §1, §3
and §7; 0130 §2, §3 and §5; 0020; 0045 §8; 0055; 0056.

## What

Story A builds the part of protocol v4 that measures what a mail mock costs to
run:

- protocol version 4 and source lock version 5 in
  `benches/mail_compare/protocol.rs`, beside the frozen versions;
- a new `benches/mail_compare/resources.rs`: every v4 launch runs in a
  driver-owned cgroup v2 leaf limited to 2 CPUs, 2048 MiB, no swap and 4096
  tasks, pinned to two cores by affinity, sampled every 100 ms, and removed
  with `cgroup.kill`;
- the launcher's join and the cleanup receipt's `cgroup_removed`;
- resource rows, the `cost` section, 0132 §5's figures and v4 admission;
- the `hold`, `footprint` and `admit-v4` driver modes;
- `collect-v4` in `scripts/compare-mail.sh`;
- five real proofs on a delegated host, and the v4 section of
  `docs/evals/README.md`.

Stories B (API level), C (`wait-best`), D (test execution) and E (freeze and
collection) are separate runs. Nothing here collects, freezes or publishes.

## Why

Protocols v2 and v3 declare a 2 CPU / 2048 MiB budget with `enforced: false`
and record no CPU time or memory for any target. The README headline the
operator wants (#349) rests on cost to operate, so the budget has to be
enforced by the kernel and the figures have to be recomputable from rows.

## Readings of 0132 this design commits to

Each is a reading the panel disputed or the intake left open. None changes a
ruling of 0132.

1. **The active records stay v3 in this story.** `docs/evals/protocol.json`
   and `docs/evals/sources.lock.json` are not edited. 0132 §1.4 moves the v3
   bytes "when v4 and lock v5 land"; a v4 that cannot freeze has not landed.
   Story A adds byte-identical history copies and points the v3 tests at
   them, so story E's swap is a replacement of two files. The v4 draft and
   the lock v5 live under `docs/evals/draft/` until then.
2. **Story A's plan is the `cost` section: 24 chunks.** 0132 §11.2's 264 is
   the whole of v4. The plan is derived from the sections a protocol carries,
   and this driver refuses a frozen v4 while any section is owed (V5), so a
   24-chunk run can never be admitted as "v4".
3. **`nr_throttled` is a throttle count, not an affinity proof.** 0132 §3.4's
   parenthesis says it "records" a widened affinity. It records quota
   throttling, which also happens with the affinity intact. Affinity is
   evidenced by reading it back from `/proc` (V10, V13); `nr_throttled` is
   published as what it is.
4. **An empty leaf proves that no process remains in the leaf.** It does not
   prove that a same-uid process never moved itself to another cgroup the
   user owns. The pinned targets are not adversarial and 0132's non-goals
   exclude hostile confinement; the README says exactly this.
5. **The per-chunk deadline of 0132 §10 is a protocol value below the
   wrapper's 600 s kill** (V2, V16), so a slow target is a measured row
   (0132 §11.3) and not a killed driver.
6. **Resource samples share `samples.jsonl` with the message rows** (0132
   §4.1), as a second row family that only a v4 run directory may carry.

## Requirements

### Protocol and lock

**V1. Versions and pairing.** `PROTOCOL_VERSION_4 = 4`, id
`fair-mail-comparison-v4`, `LOCK_VERSION_5 = 5`. Protocol 4 supersedes
`fair-mail-comparison-v3` and its `change_reason` cites `0132`. It is held to
every protocol 3 rule except id, predecessor and reason: the nine targets,
the nine cyclic rotations, one capability record per target, product scope,
and `does_not_start`, which must list `mailslurper`. Protocol 4 pairs only
with lock 5, protocol 3 only with lock 4, protocol 2 and the drafts only with
lock 3. The types are extended in place by version, as protocol 3 was; there
is no second type tree.

**V2. The v4 resources block.** Protocol 4 requires exactly: `enforced: true`,
`profile: enforced`, `mechanism: cgroup_v2_user_scope`,
`cpu_placement: affinity_plus_quota`, `cpus: 2`, `memory_mib: 2048`,
`swap_mib: 0`, `tasks: 4096`, `sample_interval_ms: 100`, and
`chunk_deadline_ms` in 1..=540000. Each new field is optional in the type and
omitted when absent, so v2 and v3 bytes round-trip unchanged; protocols 1 to 3
refuse each new field by name.

**V3. The cost section.** Protocol 4's `throughput` list is the cost section
(the report names it `cost`). A frozen protocol 4 carries exactly these four
profiles, each with 3 repetitions, 100 warmup messages, 3,000 messages,
`send_deadline_ms` 300000 and `verify_deadline_ms` 60000 (v3's values):

| name | sessions | connection |
|---|---|---|
| `cost-c1-reused` | 1 | reused |
| `cost-c8-reused` | 8 | reused |
| `cost-c32-reused` | 32 | reused |
| `cost-c32-fresh` | 32 | fresh |

Any protocol 4, draft or frozen, refuses a profile above 32 sessions and
refuses a wait profile (story C adds them). `headline.profile` is required,
must be `cost-c8-reused` (acceptance answer 3), and must name a declared
profile when the protocol is frozen. A draft protocol 4 may carry other
bounded profiles, as protocol 3's smoke fixtures do.

**V4. Caps.** In protocol 4 every retention entry that states a cap states
150,000, Swarmail's included (launched `--max-per-inbox 150000`). A target
whose lock entry has no cap flag stays uncapped and `no_cap_observed`.

**V5. The freeze rule.** A frozen protocol 4 needs `unresolved: []`, a
non-empty `clients_sha256` (name → 64 hex; refused by protocols 1 to 3) and
no `not_inspected` capability. In addition this driver names the sections it
does not implement in one constant (`api`, `waits`, `testexec`) and refuses
every frozen protocol 4 by name while that list is not empty. Stories B, C
and D each remove their entry.

**V6. Lock v5.** Lock 5 is lock 4 plus the `{max_messages}` placeholder:

- wherever a pin states a cap and its flag, the launch sets that flag to
  `{max_messages}` (argv pair, `flag={max_messages}`, or the `env:` value); a
  literal number there refuses, and the placeholder anywhere else refuses;
- lock 4 and lock 3 refuse the placeholder by name;
- the placeholder is resolved once, when the launch spec is built, to the
  protocol's cap for that target (`retention_matches` already holds it equal
  to the pin's stated cap). A spec with an unresolved placeholder is refused
  before any process starts.

The second lock 5 addition of 0132 §1.2, the captured API and wait documents,
needs no code: they are further `documents` entries under the unchanged rules
of 0093 §1 and 0130 §2, added by the stories that cite them.

**V7. Records in the tree.**

- `docs/evals/history/2026-11-07-protocol.v3.json` and
  `docs/evals/history/2026-11-07-sources.lock.v4.json` are byte-identical
  copies of the active files. `FROZEN_PROTOCOL_V3_SHA` and
  `FROZEN_LOCK_V4_SHA` (they exist: `tests/mail_compare/eight_targets.rs:17`)
  hold both the copies and the active files, and every test that means "v3"
  or "lock 4" reads the history copy.
- `docs/evals/draft/protocol.v4.json` is the v4 draft: `status: draft`, V2's
  block, V3's four profiles, V4's caps, v3's targets, products, budgets,
  corpus, statistics, capabilities and `does_not_start` unchanged, and an
  `unresolved` list naming what stories B to E owe.
- `docs/evals/draft/sources.lock.v5.json` is lock 4 with `lock_version` 5,
  the three capped launches (Mailpit `--max`, smtp4dev `--messagestokeep`,
  Inbucket `INBUCKET_STORAGE_MAILBOXMSGCAP`) rewritten to the placeholder and
  their stated caps set to 150,000. It verifies against `docs/evals/sources/`
  through `--documents docs/evals`.
- No `FROZEN_PROTOCOL_V4_SHA` or `FROZEN_LOCK_V5_SHA` yet: a draft is not
  frozen by hash. Story E adds them.

### Enforcement

**V8. Detection and refusal.** Before any v4 launch the driver requires: Linux;
`/sys/fs/cgroup` is `cgroup2fs`; its own cgroup (`/proc/self/cgroup`, the
`0::` line) lists `cpu`, `memory` and `pids` in `cgroup.controllers`. Failing
any of these, and off Linux, every v4 mode that launches refuses with exactly
`resources: no delegated cgroup v2 (cpu memory pids)` and exit 2. There is no
unenforced fallback, in `collect` or in `smoke`. The driver then makes
`<scope>/driver`, moves itself into it, enables `+cpu +memory +pids` in the
scope's `cgroup.subtree_control`, and reads that file back; a controller
missing from the read-back is the same refusal. A scope that holds any other
process, or already holds a `driver` leaf, refuses by its own name. Protocols
1 to 3 never touch cgroupfs.

**V9. Leaf and limits.** One leaf `<scope>/target-<launch-id>` per launch,
created before spawn, with these values written and then read back:

| file | value |
|---|---|
| `memory.max` | `2147483648` |
| `memory.swap.max` | `0` |
| `memory.oom.group` | `1` |
| `cpu.max` | `200000 100000` |
| `pids.max` | `4096` |

A read-back that differs, or a leaf that lacks one of these files,
`cgroup.kill` or `memory.peak`, refuses the launch by naming the file. The
five strings as read back go into the launch receipt.

**V10. Join, and the proof of it.** The leaf's `cgroup.procs` is opened by the
driver before the fork (`O_WRONLY | O_CLOEXEC`). Between fork and exec the
child does only: `write(fd, "0")`, `sched_setaffinity` to the two target CPUs,
then the existing `setrlimit(RLIMIT_NOFILE)`. Nothing in the closure
allocates, formats or builds a path, and every return value is checked: a
failure fails the spawn. After the spawn the driver reads
`/proc/<pid>/cgroup` and `Cpus_allowed_list` from `/proc/<pid>/status` and
requires the leaf and exactly the two target CPUs. On a mismatch the leaf is
killed and removed and the launch is refused. A launch receipt therefore
means the target was in the leaf, not only that the leaf had limits.

**V11. CPU layout.** A pure function of the driver's allowed CPU set and the
kernel's `thread_siblings_list`: the two target CPUs are the lowest allowed
CPU and the lowest allowed CPU that is not its sibling; their siblings inside
the set are left idle; the rest is the client set, and the driver pins itself
to it before it starts its runtime, so the sampler is charged to the driver.
Fewer than two distinct cores, or no client CPU left, refuses by name.
Siblings outside the allowed set cannot be kept idle by the driver; they are
recorded as such. The layout is recorded in every launch receipt and in the
chunk manifest's host facts.

**V12. Scratch on disk.** A v4 run refuses a scratch directory on tmpfs
(`statfs`), by name, before any launch.

**V13. Sampling and windows.** `samples.jsonl` of a v4 run directory carries
the message rows unchanged and a second family, the resource rows:

- `sample`: launch id, target, `t_ns` on the run's clock, `memory.current`,
  `anon`, `file` and `kernel` from `memory.stat`, and `pids.current`. One
  every 100 ms from leaf creation to teardown, and one taken at each window
  bound, so no window is empty.
- `window`: launch id, target, profile, repetition, kind (`ready`,
  `idle_ready`, `measure`), `start_ns`, `end_ns`, `cpu.stat` at both bounds
  (`usage_usec`, `user_usec`, `system_usec`, `nr_throttled`,
  `throttled_usec`), `memory.events` at both bounds (`max`, `oom`,
  `oom_kill`), `affinity_held` (every task in the leaf still has exactly the
  two target CPUs at the window's end), the scratch tree's allocated bytes
  (`st_blocks × 512`) for `idle_ready`, and an outcome.

Window outcomes are `measured`, `oom_killed` (the `oom_kill` count rose over
the window; a rise of `max` alone is pressure, not an OOM), `target_exited`
and `chunk_deadline`. `ready` runs from leaf creation to readiness and gives
the CPU time to ready. `idle_ready` is 5 s without traffic after readiness on
each measured instance; its figure is the mean over the samples of its last
2 s. `measure` is the profile's send-and-verify window. Peak and mean memory
are not stored on the window: the report derives them from the `sample` rows
inside the window's bounds, so there is one source. Protocols 1 to 3 refuse a
resource row.

**V14. Teardown.** The 0092 §6 / 0093 §7 sequence is unchanged and runs
first. Then, inside the same composed cleanup budget: write `1` to
`cgroup.kill`, poll `cgroup.procs` until it is empty or the budget ends (at
least one read after the kill), read `memory.peak`, and `rmdir` the leaf.
`cgroup_removed` is true only when the leaf was empty and was removed. A leaf
that cannot be emptied is left in place, cleanup is unknown, the run is not
sealed and the driver exits 2.

**V15. Receipts.** The cleanup receipt gains `cgroup_removed` and the leaf's
`memory.peak`; the launch receipt gains the leaf name, the limits read back,
the CPU layout, the cgroup and affinity read back after the spawn, and the
environment template. All are absent, and absent from the bytes, for a
launch without a leaf, so a v3 receipt serializes as it does today. Because
V6 resolves the cap when the spec is built, the receipt's `argv_template`
(and environment template) shows the cap that ran, `150000`, while ports and
private paths stay slots, as 0093's privacy rule requires. For a leaf launch,
cleanup is proved only with `cgroup_removed: true`; a missing value is not
true.

### Measurement

**V16. The cost chunk.** `run` takes `--chunk cost.<target>.r<rep>` for
protocol 4 and refuses to run protocol 4 without it. A cost chunk runs that
target's profiles of that repetition in order, each as v3 does: a warmup
instance, then a fresh measured instance, both in leaves. A v4 `collect`
refuses a missing or dirty candidate commit by name. Inside a chunk:

- when the target exits during a window, every unsent index still gets its
  row, the window's outcome is `oom_killed` or `target_exited`, nothing is
  retried, and the chunk goes on to its next profile;
- when `chunk_deadline_ms` has passed, a profile not yet started is not
  launched: its send rows are `unstarted` with reason `chunk deadline` and
  its `measure` window has outcome `chunk_deadline`;
- the chunk seals and exits 1 when any such outcome or workload failure
  occurred, exits 0 when none did, and exits 2 unsealed on a launch refusal,
  an unproved cleanup or an interrupt.

The chunk manifest carries the collection's run id, the chunk coordinate and
the host facts (kernel, systemd version as passed by the wrapper, CPU model,
`nproc`, the sibling layout, the CPU ids per role, the frequency governor).

**V17. The §5 figures.** A v4 report is `report_version` 3; protocols 2 and 3
keep version 2 and their admission terms. Per repetition and cost profile it
gives the v2 reconciliation, the target CPU time over the `measure` window
(user, system, total), sampled peak and mean of `memory.current` and `anon`,
and the OOM and throttle counts. Per target and cost profile it gives, as the
nearest-rank median over repetitions with minimum and maximum:

- `visible_per_cpu_second` = distinct accepted messages observed ÷ CPU
  seconds, and `cpu_ms_per_1k_visible`; either is `null`, never zero or
  infinite, when its divisor is zero;
- `peak_memory_mib` and peak `anon`.

A cell is *at equal results* when every repetition is exact (requested =
accepted = visible, seen once, fidelity intact) and every `measure` window
is `measured`. Only such a cell has `ranked: true`. Any other cell keeps its
figures, has `ranked: false`, and names its shortfalls from a closed list
(`refused`, `lost`, `fidelity`, `oom`, `timeout`, `target_exited`,
`duplicate`, `unexpected`, `ambiguous`). `affinity_held: false` is published
as the label `affinity_widened`; it does not change ranking, which stays
0132 §5.2's. Per target the report gives `idle_memory_mib` (current and
`anon`, with the scratch footprint) and startup (ready ms, CPU ms to ready).
The headline is the cells of `headline.profile` with those two.
`mib_per_1k_retained` needs story B's fill levels: it is published as owed,
never as a number. The official MailSlurper row is blocked in every cell.

**V18. `footprint`.** `mail_compare footprint --protocol P --sources S --bin
T=PATH… [--runtime T=DIR…]` prints one JSON document with, per target: the
download bytes (the pinned archive's size; for the image row the sum of the
captured manifest's layer sizes; for the candidate the release binary), the
installed bytes (the executable, its checked members and the whole pinned
runtime tree), and the unpinned runtime as the label `unpinned, not counted`
where the row has one. It launches nothing and rechecks every digest it
relies on. The does-not-start row is blocked, with no number.

**V19. `hold`.** `mail_compare hold --protocol P --sources S --bin T=PATH
[--runtime T=DIR] --scratch DIR --samples FILE` launches one target under the
budget, prints its endpoints as one JSON line, samples, and waits. On stdin
EOF, or SIGTERM or SIGINT, it runs V14's teardown and prints the launch and
cleanup receipts as one JSON line. It exits 0 only when cleanup is proved.
Any byte on stdin before EOF is refused by name after teardown, so a later
story can add a line protocol without changing what silence means.

**V20. Plan and `admit-v4`.** One function derives the chunk plan from the
protocol: for each repetition, for each launched target in that repetition's
rotation, one `cost` chunk. `targets --protocol P --plan v4` prints it, one
coordinate per line; the wrapper never computes it. `admit-v4 DIR PROTOCOL`:

- recomputes the plan and requires exactly one sealed, admitted chunk
  directory per coordinate, all naming one run id, protocol, lock and clean
  candidate commit, with the same host facts and target CPU pair;
- requires, per launch, the five limits of V9, the join read-back, the
  protocol's cap in the receipt of every capped target, and
  `cgroup_removed: true`;
- requires every planned window row once and no unplanned one;
- requires one ledger entry per coordinate with exit 0 or 1 and a clean scope;
- rechecks every executable digest against the lock, and against the files
  when `--bin` is passed;
- recomputes every figure of V17 from the rows and refuses a report that
  differs, naming the first differing path.

`admit-v4 --seal` writes the run-level `manifest.json`, `host.json`,
`chunks.json`, `launches.json`, `report.json`, `report-cost.json` and
`index.json` as new files and refuses if any exists; plain `admit-v4`
verifies them.

**V21. `collect-v4`.** `scripts/compare-mail.sh collect-v4 --output DIR --bin
NAME=PATH… [--runtime NAME=DIR…] [--protocol P --sources S --documents DIR]
[--dry-run]`:

- refuses by name, before building: not Linux; no `systemd-run`, `systemctl`,
  `taskset` or `timeout`; an existing `--output`; a protocol that is not
  version 4 (the message names the draft route); fewer than 3 CPUs in
  `min(nproc/4, 32)`; a dirty or uncommitted candidate (real run only);
- `--dry-run` prints every chunk's coordinate and command and launches
  nothing;
- otherwise requires `check` to pass, writes `footprint.json`, and runs the
  plan in order. Each chunk is
  `timeout --kill-after=30 600 taskset -c 0-<n-1> systemd-run --user --scope
  -p Delegate=yes --unit=mail-compare-<run>-<coordinate> -- <driver> run …`,
  in that nesting, so the driver is the scope's only process;
- reads `/proc/loadavg` before and after each chunk and writes one new ledger
  file per chunk (coordinate, command, exit, wall time, both loads, whether
  the scope was clean);
- after each chunk, a scope that is still active is killed and stopped, and
  the chunk counts as exit 2. Exit 124, 137 or above 2 counts as 2. Exit 0
  or 1 without the chunk's `index.json` counts as 2;
- exit 2 stops the run as incomplete; nothing is resumed, overwritten or
  removed. Otherwise it writes `reproduce.txt`, runs `admit-v4 --seal` and
  `admit-v4`, and exits 2 if admission refuses, else 1 if any chunk exited 1,
  else 0.

The host load is recorded, not gated (see *For the operator at freeze*).

### Proof

**V22. Real proofs.** Five tests, `#[ignore]`d and Linux-only, each run alone
as the only process of a delegated scope:

```
bin=$(cargo test --locked --test mail_compare --no-run --message-format=json \
  | grep -o '"executable":"[^"]*mail_compare-[^"]*"' | tail -n1 | cut -d'"' -f4)
cat /proc/loadavg
timeout 600 taskset -c 0-31 systemd-run --user --scope --quiet -p Delegate=yes -- \
  "$bin" --ignored --exact v4::<name> --test-threads=1
cat /proc/loadavg
```

`cargo` is not the process inside the scope: it would stay in the scope's own
cgroup, and the test could then not enable controllers (V8).

1. `v4_real_oom_at_two_gib`: a memory hog in a leaf is killed; `memory.events`
   shows `oom_kill` ≥ 1; the window outcome is `oom_killed`; nothing is
   retried; the leaf is emptied and removed.
2. `v4_real_cpu_held_within_two`: four spinners for a fixed 5 s window. With
   affinity held, every task's `Cpus_allowed_list` is the two target CPUs and
   the `usage_usec` delta is at most 2 × wall × 1.05 + 100 ms. With spinners
   that widen their own affinity, the same bound holds, `nr_throttled` rises,
   and `affinity_held` is false.
3. `v4_real_setsid_escapee_removed`: a child that `setsid`s a sleeper and
   exits. After the process-group kill `cgroup.procs` is not empty; after
   V14 it is, the leaf is removed, and the escapee's pid is gone.
4. `v4_real_hold_swarmail_under_budget`: `hold` on the candidate Swarmail:
   the limits read back, the join read-back, one accepted and visible mail,
   and on EOF a receipt with `cgroup_removed: true`.
5. `v4_real_cost_chunk_seals_and_admits`: one cost chunk of a small v4 draft
   fixture against the candidate seals, and its windows, samples and figures
   pass the chunk-level checks of V20.

Each fails, never skips, when V8 refuses or its bytes are missing. Without
delegation the failure message is V8's exact refusal. A proof is recorded as
passed only with its output from a host where it ran; a seat whose box has no
cgroupfs reports the proofs as owed, with the refusal output, and does not
claim them.

**V23. Docs.** `docs/evals/README.md` gains a v4 section: what story A
measures, the budget and mechanism, the refusals, the draft route and
`collect-v4`, Reading 4's sentence, "a ceiling, not a reservation" for the
CPU budget, `memory.current` labelled as including reclaimable cache beside
`anon`, and that v3 still collects from the active paths.

**V24. Unchanged.** No file under `src/` or `evals/` changes. `Cargo.toml`,
`Cargo.lock`, `evals/Cargo.lock` and `deny.toml` are byte-identical. No
existing file under `docs/evals/` changes except `README.md`; the active
protocol and lock keep their sha256. No accepted decision's body is edited.

## Acceptance Criteria

Each is proved by the tests named in [tasks.md](tasks.md). "Refuses" always
means a refusal whose text names the cause.

- **AC1 (V1).** Protocol 4 parses and validates with its id, predecessor and
  reason; each of them wrong refuses. Every cross pairing of protocols 2, 3
  and 4 with locks 3, 4 and 5 refuses; the three right pairings pass.
- **AC2 (V2).** Each field of the v4 resources block, absent or at another
  value, refuses. Each new field on a protocol 2 or 3 refuses. The v2 and v3
  history bytes still parse, validate and re-serialize to the same value.
- **AC3 (V3).** A frozen protocol 4 with a profile added, removed or changed
  from the table refuses. Any protocol 4 with 33 sessions, a wait profile, no
  headline or another headline profile refuses.
- **AC4 (V4, V6).** A protocol 4 cap other than 150,000 refuses. Lock 5 with
  a literal cap, with the placeholder outside the cap flag, or lock 4 with
  the placeholder, refuses. The spec built from lock 5 and protocol 4 carries
  `150000` at each of the three cap flags and Swarmail's
  `--max-per-inbox 150000`; a spec with an unresolved placeholder refuses
  before spawn.
- **AC5 (V5).** A frozen protocol 4 refuses in this driver, naming the owed
  sections. With the owed list passed as empty, it still refuses for a
  non-empty `unresolved`, an empty `clients_sha256` and a `not_inspected`
  capability, each on its own.
- **AC6 (V7).** The two history copies have the frozen sha256 values and the
  active files still have them too. The draft protocol validates as a draft
  and `check` reports it not ready for collection. Lock 5 validates, verifies
  its documents through `--documents docs/evals`, and pairs with the draft.
- **AC7 (V8).** On fixture trees: no cgroup2 root, a missing controller, and
  a controller missing from the `subtree_control` read-back each give the
  exact refusal; an occupied scope and an existing `driver` leaf refuse by
  their own names; the adoption writes happen in V8's order. Off Linux the
  exact refusal is returned. A protocol 3 run reads no cgroup file.
- **AC8 (V9).** On a fixture leaf the five values are written, read back and
  recorded; a differing read-back and each missing file refuse, naming the
  file.
- **AC9 (V10, V11).** The parsers of `/proc/<pid>/cgroup` and
  `Cpus_allowed_list` (list and range forms) and the layout function are
  proved on fixtures: SMT and non-SMT maps, one core only, no client CPU
  left, siblings outside the set. The join itself is proved by real proofs 2
  to 4.
- **AC10 (V12, V13).** A tmpfs scratch refuses (on Linux, against
  `/dev/shm`). The parsers of `cpu.stat`, `memory.stat`, `memory.events`,
  `memory.current`, `memory.peak`, `pids.current` and `cgroup.procs` pass on
  captured text and refuse malformed text. A resource row in a protocol 3 run
  directory, a duplicate or unplanned window, and a missing planned window
  each refuse.
- **AC11 (V14, V15).** On a fixture leaf whose `cgroup.procs` never empties,
  teardown ends at the budget with `cgroup_removed: false`, the leaf in place
  and cleanup unproved. A v3 cleanup receipt and a v3 launch receipt
  serialize byte for byte as before. A leaf launch whose receipt lacks
  `cgroup_removed` is not proved.
- **AC12 (V16).** The plan of the committed draft is 24 cost chunks: eight
  launched targets in each repetition's rotation, none for `mailslurper`.
  Protocol 4 without `--chunk`, protocol 3 with it, an unknown coordinate,
  and a v4 `collect` without a clean commit each refuse.
- **AC13 (V17).** With hand-built rows: the figures equal independently
  computed values; a zero CPU delta and zero visible give `null`; one inexact
  repetition, and one window that is not `measured`, each make the cell
  unranked with the right shortfall; the median, minimum and maximum over
  three repetitions are right; `mib_per_1k_retained` is owed; the blocked row
  has no number; a v4 report is version 3 and a protocol 3 report is still
  version 2.
- **AC14 (V18).** `footprint` on fixtures gives the archive size, the layer
  sum, the candidate's file size, the installed sum including a runtime tree,
  the `unpinned, not counted` label, and a blocked row; a changed executable
  refuses.
- **AC15 (V19).** `hold`'s refusals (missing flags, protocol 3, bytes on
  stdin) are proved without a cgroup; its behaviour under the budget is real
  proof 4.
- **AC16 (V20).** On a fixture run directory built by the test, a complete
  one seals and then verifies, and each of these refuses by name: a missing
  chunk, an extra chunk, an unsealed chunk, two run ids, a dirty candidate,
  differing host facts, a limit other than V9's, a missing join read-back, a
  cap other than the protocol's, `cgroup_removed` false or absent, a ledger
  exit 2, an unclean scope, a second `--seal`, and one edited figure, which
  is named by its path.
- **AC17 (V21).** With a fake driver and fake `systemd-run` and `systemctl`
  on `PATH`: `--dry-run` prints 24 commands in plan order and starts nothing;
  the default protocol refuses and names the draft route; each preflight
  refusal is named; a chunk exit 2, a timeout, an exit 0 without an index and
  a still-active scope each stop the run with exit 2 and a ledger entry, with
  no later chunk started; a full fake run writes 24 ledger files and
  `reproduce.txt` and calls `--seal` and then verify. Off Linux the mode
  refuses.
- **AC18 (V22).** The five proofs exist, are ignored and Linux-only, and each
  fails with V8's exact refusal where there is no delegation. Their passing
  output from the dispatcher host, with the load before and after, is
  attached to the PR before the operator merges it.
- **AC19 (V23, V24).** The README section exists and the decision inventory
  test passes. `git diff e23a8ba -- src evals Cargo.toml Cargo.lock deny.toml`
  is empty, and the only existing file changed under `docs/evals/` is
  `README.md`.
- **AC20.** Every gate of `docs/house-rules.md` § Gates passes `--locked` on
  the final tree, the coverage selftest and the publish-guard proof included.

## Out of scope

- Stories B to E: the `api`, `waits` and `testexec` sections, operation and
  wait capability rows, the `code_link` corpus kind, fill levels and
  `mib_per_1k_retained`, clients and `clients_sha256` values, the frozen v4
  and lock 5 hashes, the swap of the active records, collection, the write-up
  and the README headline.
- GreenMail's REST read path (0132 §6.2). The cost section verifies as v3
  does, over POP3.
- A hostile-body, TLS, webhook or chaos campaign (0132 non-goals). The `DATA`
  writer already dot-stuffs (`wire.rs`, `dot_stuff`).
- cpuset delegation, an unenforced pass, a container runtime, a second
  supervisor, a new package.

## For the operator at freeze (story E)

Story A leaves these open and labels them as such. None blocks this story.

1. `chunk_deadline_ms`: the draft says 540000.
2. Whether a host-load ceiling should gate a chunk. Story A records the load
   beside every chunk and gates nothing: a gate would abort a day-long run on
   a neighbour's burst, and its threshold would be a frozen protocol value
   that 0132 does not name.
3. Whether a cell labelled `affinity_widened` should rank.

## Stop triggers

Stop and report, without working around it, if: a requirement can only be met
by editing `src/`, `evals/`, a lockfile, `deny.toml` or an active or
historical record; lock 5 cannot verify against the captured documents
without changing one; a v3 test can only pass by changing v3 behaviour; or
the gates' coverage denominator turns out to include the bench.
