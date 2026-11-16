# Mail comparison protocol v4, story A: lock v5, enforced resources and cost to operate

Date: 2026-11-16. Status: draft, revision 2 (returned from review 2; see
[*Revisions*](#revisions)). Issue #359, part of #349 and #351 (#163, epic
#164). Run: `issue-359-comparison-protocol-v4-b0a29b93`. Branch
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

## Revisions

- **2026-11-16, revision 1, after review 1 of head `2852bd2`.** The panel
  found one defect in this spec and returned the run to design (decision
  0102, addendum 2026-10-31). The defect: V16 consulted the chunk deadline
  only before a profile started and V2 allowed 540000 ms, so a profile
  already running could outlive the wrapper's 600 s kill and Reading 5 was
  false. Corrected here: Reading 5, V2, V16, AC2, AC21. The same revision
  answers the findings reported beside it, each in the requirement that owns
  it:
  - a window that was never opened has no counters and gives no figure,
    where the first design wrote zeros (Reading 8, V13, V17, AC13, AC23);
  - `affinity_held` is read at the end of every opened window (V13);
  - a target that exits or is OOM-killed before readiness is a measured
    outcome, as 0132 §11.3 rules, not a lost run (Reading 7, V15, V16, AC22);
  - Swarmail's cap may not be null and the cap flag occurs once (V4, V6);
  - admission holds every chunk to a clean candidate and one executable per
    target, and refuses an empty plan (V20);
  - `footprint` no longer publishes an executable alone as an archive row's
    installed size (V18);
  - `hold` prints the POP3 address (V19);
  - `collect-v4` waits for the scope to go inactive, handles a signal, bounds
    every process it starts and refuses a restricted shell (V21);
  - the draft names GreenMail's owed REST path and its changed wait budget
    (V7), and three sentences are made exact (Reading 2, V23);
  - AC18 is split into what a seat can prove and the host run that gates the
    merge (V22).

  No criterion is loosened. [plan.md](plan.md) § *Revision 1* maps every
  finding to its change and task, and says where a criterion's text was
  wrong and on what evidence.

- **2026-11-16, revision 2, after review 2 of head `c2041b9`.** Security and
  tests were clean; correctness, guarantees and independent returned
  residual findings, and the guarantees member found three places where
  revision 1 contradicts accepted 0132. Each is corrected toward the
  decision:
  - **0132 §4.5 and §5.6: an installed size for every row, and in the
    headline.** Revision 1 published `null` and `owed` for the three archive
    rows and the headline had no installed size. Now every launched row has
    a number or `footprint` refuses, the walked trees are identified by a
    digest, and the run-level headline carries the installed size (Reading
    9, V17, V18, V20, AC14, AC16).
  - **0132 §11.1: no process runs longer than 10 minutes.** Revision 1's
    `timeout --kill-after=30 600` allowed 630 s and a further scope wait.
    The prefix is now `timeout --kill-after=30 560`, `CHUNK_KILL_MS` is
    560000, and V21 states the sum for every process the mode starts
    (Reading 10, V2, V21, AC2, AC17).
  - **0132 §2.3: the candidate is a committed, clean `main`.** Revision 1
    checked only committed and clean, so a clean pull-request branch passed.
    `HEAD` must now be in `main`'s history (Reading 11, V21, AC17).

  The same revision answers the findings reported beside those, each in the
  requirement that owns it:
  - no `QUIT` is sent after the chunk deadline (V16, AC21);
  - a sealed chunk that launched nothing is admitted (V16, V20, AC16);
  - a never-opened window keeps V13's order of outcomes, so an OOM is not
    relabelled (V13, AC10);
  - an idle cell ranks only on every value it publishes (V17, AC23);
  - the join is compared by the whole cgroup path, and a cleanup receipt
    names the leaf it removed (V10, V15, V20, AC9, AC11, AC16);
  - `collect-v4` runs every bounded step where a signal can reach it, keeps
    a chunk until its ledger entry is written, and calls a scope clean only
    when systemd says so (V21, AC17);
  - V22 says what a seat's typed result does and does not state about
    AC18b, which stays open and stays the merge gate.

  No criterion is loosened. Two texts were wrong and are replaced by
  stricter ones (V18's owed rule; V21's 600). [plan.md](plan.md)
  § *Revision 2* rules the two positions, maps every finding of review 2 to
  its change and task, and says which findings change nothing and why.

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
   the whole of v4. The plan is derived from the sections a protocol carries.
   This driver refuses a frozen v4 while any section is owed (V5), and
   `collect-v4` refuses a draft at `check`, so story A cannot collect at all.
   `admit-v4` admits a directory against the protocol it is given, a draft
   included; the sealed run manifest carries that protocol's id, sha256 and
   status (V20), so a draft run is labelled a draft in its own bytes and is
   never a v4 result.
3. **`nr_throttled` is a throttle count, not an affinity proof.** 0132 §3.4's
   parenthesis says it "records" a widened affinity. It records quota
   throttling, which also happens with the affinity intact. Affinity is
   evidenced by reading it back from `/proc` (V10, V13); `nr_throttled` is
   published as what it is.
4. **An empty leaf proves that no process remains in the leaf.** It does not
   prove that a same-uid process never moved itself to another cgroup the
   user owns. The pinned targets are not adversarial and 0132's non-goals
   exclude hostile confinement; the README says exactly this.
5. **The per-chunk deadline of 0132 §10 is one instant inside the driver that
   ends the chunk's work on the target.** After it nothing new starts, and
   what is in flight on the wire is cancelled, not awaited (V16). V2 bounds
   the value so that the little that may still run after it (one launch
   already in flight, the teardowns, the seal) ends before the wrapper's
   `timeout` acts (V21; 560 s since revision 2). Only an interrupt of the work in flight makes a slow target a
   measured row (0132 §11.3) and not a killed driver; a check between
   profiles does not. *(Revision 1: the first text claimed this for a check
   made only before each profile.)*
6. **Resource samples share `samples.jsonl` with the message rows** (0132
   §4.1), as a second row family that only a v4 run directory may carry.
7. **A target that exits or is OOM-killed before it is ready has failed on
   its own** (0132 §11.3: "OOM, crash"; §4.2: published as a row, never
   retried). It is a measured outcome and the chunk exits 1. What the driver
   could not establish stays a launch refusal and exit 2: a spawn, join or
   limit failure, and a target that is still alive but not ready at the
   startup deadline. The second case cannot be told from a broken launch by
   any evidence the leaf holds, and 0132 names no outcome for it.
   *(Revision 1.)*
8. **No observation, no number.** 0132 §5.2 keeps the resource figures of a
   cell that is not at equal results. That holds for a window that was
   opened, whatever ended it. A window that was never opened has no counters
   and gives no figure: its values are `null`, never zero, in the rows and in
   every figure derived from them. *(Revision 1.)*
9. **An archive row's installed tree is the extracted archive as it stands
   on the collection host.** 0132 §4.5 requires an installed size for every
   row and §5.6 puts it in the headline; the acceptance note leaves none
   owed. For Mailpit, smtp4dev and Inbucket the lock binds two things: the
   archive's digest and the executable's digest. `footprint` and
   `collect-v4` are given the extracted executable (`--bin`), not the
   archive, and 0132 §11.1's command has no archive argument. So the tree
   `footprint` sizes is the directory the pinned member was extracted into.
   What is proved of it: it contains the digest-checked executable at the
   pinned member path, and it is not shared with another target. What is
   not proved: that it holds every file of the archive and no other. That
   limit is the one 0132 already accepts for a runtime tree, which is walked
   whole while the lock digests two of its files. The design does not hide
   it: every walked tree is published with its file count and a digest of
   its listing (V18), so a second extraction of the same pinned archive can
   be compared with the one that was measured, and the README fixes the
   procedure (a new, empty directory per archive). A row without a number is
   a refusal, never a `null`. *(Revision 2: replaces revision 1's owed
   rule.)*
10. **"No process the command starts runs longer than 10 minutes" (0132
    §11.1) counts every process: the step under `timeout`, the driver, and
    the targets a dead driver leaves in its scope.** A target outlives a
    killed driver until the wrapper kills the scope, so the wrapper's own
    wait belongs in the sum. V21 states the sum for each case and each term
    has its own bound. *(Revision 2.)*
11. **"A committed, clean `main`" (0132 §2.3) means a commit in `main`'s
    history**, the tip or an earlier one, with a clean tree. `main` is
    linear and append-only and pull requests land by squash, so no commit of
    a pull-request branch is ever in that history: a feature branch refuses,
    whether or not it has `main` merged in. An earlier commit of `main` is
    accepted because `reproduce.txt` must still run after `main` has moved
    on; equality with the tip would make every published run unrepeatable
    at the next merge. *(Revision 2.)*

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
`swap_mib: 0`, `tasks: 4096`, `sample_interval_ms: 100`, and a
`chunk_deadline_ms` of at least 1 that satisfies

```
chunk_deadline_ms + startup_deadline_ms + 2 × cleanup_deadline_ms
  + CHUNK_MARGIN_MS  ≤  CHUNK_KILL_MS
```

`CHUNK_KILL_MS = 560000` is the instant, counted from the chunk's start, at
which the wrapper's `timeout` sends the chunk SIGTERM (V21), and
`CHUNK_MARGIN_MS = 30000` covers what the chunk clock does not see: the
scope's and the driver's start before it, and the sample stop, the read-back
of the rows, the report and the seal after it. Both are named constants in
`protocol.rs`. The other terms are what V16 lets run after the deadline: one
launch already in flight, and the teardowns, which the driver settles within
twice the cleanup budget. A value above the bound refuses, and the refusal
prints the ceiling it computed. With the draft's budgets (30 s, 10 s) the
ceiling is 480000, which is the draft's value. *(Revision 1: the first text
allowed any value up to 540000, a constant that bounded nothing. Revision 2:
`CHUNK_KILL_MS` was 600000, the wrapper's kill 30 s later, and 0132 §11.1's
ten minutes were exceeded; the inequality is unchanged and the ceiling moves
from 520000 to 480000.)*

Each new field is optional in the type and omitted when absent, so v2 and v3
bytes round-trip unchanged; protocols 1 to 3 refuse each new field by name.

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
150,000. Swarmail always has a cap flag, so its entry must state 150,000: a
`null` there refuses by name, and a protocol 4 launch spec for Swarmail is
never built with `--max-per-inbox 0`. A target whose lock entry has no cap
flag stays uncapped and `no_cap_observed`. *(Revision 1: a null Swarmail cap
validated and launched it uncapped.)*

**V5. The freeze rule.** A frozen protocol 4 needs `unresolved: []`, a
non-empty `clients_sha256` (name → 64 hex; refused by protocols 1 to 3) and
no `not_inspected` capability. In addition this driver names the sections it
does not implement in one constant (`api`, `waits`, `testexec`) and refuses
every frozen protocol 4 by name while that list is not empty. Stories B, C
and D each remove their entry.

**V6. Lock v5.** Lock 5 is lock 4 plus the `{max_messages}` placeholder:

- wherever a pin states a cap and its flag, the launch sets that flag to
  `{max_messages}` (argv pair, `flag={max_messages}`, or the `env:` value); a
  literal number there refuses, and the placeholder anywhere else refuses.
  The cap flag occurs exactly once in the launch, in any of its forms: a
  second occurrence refuses whatever it carries, so `--max {max_messages}
  --max 500` is refused;
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
  block with `chunk_deadline_ms` 480000, V3's four profiles, V4's caps, v3's
  targets, products, corpus, statistics, capabilities and `does_not_start`
  unchanged, and an `unresolved` list naming what stories B to E owe. Its
  budgets are v3's except `wait_operation_ms`: the inherited rule (the
  longest wait plus one request bound) gives 10000 on a protocol with no wait
  profile, where v3 has 40000; story C restores it with its profiles, and the
  `unresolved` list says so. The list also names GreenMail's read path:
  0132's acceptance answer 8 reads GreenMail over REST in v4, this draft
  still reads it over POP3 as v3 does, and story B owes the change. The
  claim's sentence about POP3 says "in this draft". V5 then keeps a v4 that
  reads GreenMail over POP3 from freezing, with no code.
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
requires the leaf and exactly the two target CPUs. The leaf is required by
its whole path: the `0::` line must equal the cgroup path of the leaf the
driver made (the driver's scope, then `target-<launch-id>`), so a cgroup of
the same name under another parent is a mismatch. On a mismatch the leaf is
killed and removed and the launch is refused. A launch receipt therefore
means the target was in the leaf, not only that the leaf had limits. The
receipt still records the leaf's name and not its path, which carries the
uid and the unit name. *(Revision 2: the comparison was of the path's last
component only.)*

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
  `oom_kill`), `affinity_held`, the scratch tree's allocated bytes
  (`st_blocks × 512`) for `idle_ready`, and an outcome.

`affinity_held` is read from `/proc` at the end of every opened window,
`ready` and `idle_ready` included: `true` when every task in the leaf has
exactly the two target CPUs, `false` when any task has another set, and
`null` when the leaf holds no task at that instant (the target is gone).
*(Revision 1: two of the three windows wrote a literal `true`.)*

Window outcomes are `measured`, `oom_killed` (the `oom_kill` count rose over
the window; a rise of `max` alone is pressure, not an OOM), `target_exited`
and `chunk_deadline`. Where more than one applies, the first in that order
after `measured` wins: an OOM is a kernel fact and is never relabelled as a
deadline. `ready` runs from leaf creation to readiness and gives the CPU time
to ready. `idle_ready` is 5 s without traffic after readiness on each
measured instance; its figure is the mean over the samples of its last 2 s.
`measure` is the profile's send-and-verify window. Peak and mean memory are
not stored on the window: the report derives them from the `sample` rows
inside the window's bounds, so there is one source. Protocols 1 to 3 refuse a
resource row.

**A window that was never opened** is still written, once, so that every
planned window has its row (V20). It has no counters: `cpu.stat` and
`memory.events` at both bounds, `affinity_held` and the scratch bytes are all
`null`, never zero. Its two bounds are the instant it was given up, its
`launch` is the leaf of the instance that failed, or `none` when no instance
was launched, and its outcome says why: `chunk_deadline`, `oom_killed` or
`target_exited`. The order above holds for it too *(revision 2)*: when the
instance's leaf shows `oom_kill` ≥ 1 in `memory.events` at the instant the
window is given up, the outcome is `oom_killed`; otherwise `target_exited`
when the target's leader has exited; otherwise `chunk_deadline`. The reader
refuses a `measured` window without counters, a
window with some counters and not others, and a `sample` row that names
launch `none`. The resource row version stays 1: no v4 row exists outside
this branch. *(Revision 1: the first design wrote zero counters and
`affinity_held: true` for a profile that was never launched.)*

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

The cleanup receipt of a leaf launch also carries two facts the driver reads
before the leaf is removed, absent for a launch without a leaf: `oom_kills`,
the leaf's lifetime `oom_kill` count from `memory.events`, and
`exited_early`, whether the target's leader had already exited when the
driver began the teardown, before any signal of its own. A leaf launch that
does not become ready still yields its launch receipt (leaf, limits and join
as read back) beside its cleanup receipt, so a startup failure under the
budget is evidenced as one. *(Revision 1, for V16's classification.)*

The cleanup receipt of a leaf launch names the leaf it emptied and removed
(`leaf`, the name its launch receipt carries; absent, and absent from the
bytes, for a launch without a leaf). A cleanup is then tied to its launch by
the leaf's identity and not by its place in a list (V20). *(Revision 2.)*

### Measurement

**V16. The cost chunk.** `run` takes `--chunk cost.<target>.r<rep>` for
protocol 4 and refuses to run protocol 4 without it. A cost chunk runs that
target's profiles of that repetition in order, each as v3 does: a warmup
instance, then a fresh measured instance, both in leaves. A v4 `collect`
refuses a missing or dirty candidate commit by name.

*The chunk deadline (revision 1).* `chunk_end` is the run clock's origin plus
`chunk_deadline_ms`. The driver takes the origin once, when it starts the
chunk and before it makes the output directory, and passes the one instant
down; there is no second timer.

- **Nothing new starts at or after `chunk_end`:** no instance is launched,
  no window is opened, no send, listing or fidelity read is begun.
- **What is in flight on the wire is cancelled at `chunk_end`, not awaited:**
  a warmup send, a measured connect or send, a listing, a fidelity read and
  the idle wait. A send is several protocol steps and a listing several
  pages, each with its own request deadline, so waiting for one "request" to
  finish is not a bound.
- **Two things are never cut:** a launch already in flight, which ends within
  the startup deadline, and a teardown, which ends within the cleanup budget.
  A cleanup cut short would be unproved. V2's bound is the sum of exactly
  these.
- After `chunk_end` the driver sends the target nothing but its teardown.
  That includes the `QUIT` that ends an SMTP session *(revision 2)*: at or
  after `chunk_end` no `QUIT` is begun and the session is dropped, and a
  `QUIT` in flight at `chunk_end` is cut like any other step. A cut or
  omitted `QUIT` changes no row: the send it follows already has its
  disposition. Before `chunk_end` a session ends as protocol 3 ends it.

What the cut leaves, in the existing vocabulary:

- an index whose send never began is `unstarted` with reason
  `chunk deadline` (the profile's own deadline still writes
  `send deadline`); a connect cancelled in flight keeps the disposition of a
  failed connect, and a send cancelled in flight is `ambiguous`, each with a
  reason that names `chunk deadline`. Every index has exactly one row, and
  nothing is retried;
- the verification row is `incomplete` with reason `chunk deadline` when the
  deadline ended the send, the listing or the fidelity reads. A listing that
  did not complete yields no visible rows, and a visible row whose fidelity
  read was not made carries no `fidelity_ok`;
- an `idle_ready` window cut before its 5 s, and a `measure` window whose
  send or verification the deadline ended, were opened: they keep their
  counters and have outcome `chunk_deadline`;
- a warmup cut by the deadline ends the profile: its instance is torn down
  and proved, the measured instance is not launched, and the profile is
  *not measured*.

*A profile that is not measured* is written by one path, whatever the cause:
every send row `unstarted` and the verification row `incomplete`, both with
the cause as their reason (`chunk deadline`, `oom killed` or
`target exited`), and its three windows as V13's never-opened rows with the
matching outcome. The causes are: `chunk_end` passed before its measured
instance could be launched; or its warmup or measured instance did not
become ready *and the target itself failed*.

*A launch that does not become ready (revision 1, Reading 7).* The driver
classifies it from its receipts (V15), and only when its cleanup is proved:

| evidence | result |
|---|---|
| `oom_kills` ≥ 1 | the profile is not measured, `oom_killed`; exit 1 |
| else `exited_early` is true | the profile is not measured, `target_exited`; exit 1 |
| else (alive, never ready), or no launch receipt | launch refusal; exit 2, unsealed |

A spawn failure, a join mismatch (V10) and a limit or leaf refusal (V9) are
launch refusals as before. An unproved cleanup is exit 2 on every path.

Inside a window, as before: when the target exits, every unsent index still
gets its row, the window's outcome is `oom_killed` or `target_exited`, and
nothing is retried. After a profile that was not measured, or whose target
died, the chunk goes on to its next profile; after `chunk_end` every later
profile is not measured.

The chunk seals and exits 1 when any window is not `measured` or any workload
failure occurred, exits 0 when none did, and exits 2 unsealed on a launch
refusal, an unproved cleanup or an interrupt. A sealed chunk may hold no
launch at all (`chunk_end` had passed at entry): it has every planned row
and window, no launch receipt and no cleanup receipt, and V20 admits it
*(revision 2)*. Protocols 1 to 3 keep their
behaviour: none of this applies without `--chunk`.

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

*No observation, no number (revision 1, Reading 8).* One rule decides
whether a repetition has a value:

| figure | has a value only from |
|---|---|
| CPU time, peak and mean memory, OOM and throttle counts, and the two per-CPU figures | a `measure` window with counters, whatever its outcome (0132 §5.2: an unranked cell keeps its figures) |
| idle memory, `anon` and the scratch footprint | an `idle_ready` window whose outcome is `measured` |
| ready ms and CPU ms to ready | a `ready` window whose outcome is `measured` |

Otherwise the repetition's value is `null`. A median, minimum and maximum
are taken over the values that exist and carry `n`, their count; with none,
the figure is `null`. No path turns an absent value into zero. A peak with no
sample inside the window is `null`.

A repetition is exact when requested = accepted = visible, every message was
seen once, and every visible message's fidelity was read and intact; a
visible row without `fidelity_ok` is not exact. A cell is *at equal results*
when every repetition is exact and every `measure` window is `measured`. Only
such a cell has `ranked: true`. Any other cell keeps the figures it has, has
`ranked: false`, and names its shortfalls from a closed list (`refused`,
`lost`, `fidelity`, `oom`, `timeout`, `target_exited`, `duplicate`,
`unexpected`, `ambiguous`):

- `oom` and `target_exited` come from the window's outcome;
- `timeout` is named for a `chunk_deadline` window, for a send row
  `unstarted` at a deadline (`send deadline`, `chunk deadline`), and for a
  verification the chunk deadline ended. Rows `unstarted` because the target
  failed before readiness name that failure, not `timeout`;
- `lost` is not named from a verification the chunk deadline ended: what was
  not looked for was not lost.

`affinity_held: false` is published as the label `affinity_widened`; `null`
gives no label; neither changes ranking, which stays 0132 §5.2's. Per target
the report gives `idle_memory_mib` (current and `anon`, with the scratch
footprint) and startup (ready ms, CPU ms to ready). These idle and startup
cells carry the same pair as the cost cells: `ranked` is true only when every
repetition has every value the cell publishes (an idle cell: current memory,
`anon` and the scratch footprint; a startup cell: ready ms and CPU ms to
ready), and `shortfalls` names why one has none, from the same list (`oom`,
`timeout`, `target_exited`). *(Revision 1: the first design published 0 for
the idle memory, startup and peak memory of a repetition that never ran, and
fed it to the median. Revision 2: an idle cell ranked on two of its three
values.)*

The headline is the cells of `headline.profile` with those two, and, in the
run-level report, the installed size of 0132 §5.6 *(revision 2)*:
`headline.installed` has one entry per launched target with
`installed_bytes`, `installed_files` and `installed_basis`, copied from the
run's `footprint.json` and citing it. It is a fixed fact (0132 §5.5): it has
no window, no repetition and no median. The does-not-start row is blocked
there as in every other cell. A chunk's own report has no footprint and
carries no `installed` key.

`mib_per_1k_retained` needs story B's fill levels: it is published as owed,
never as a number. The official MailSlurper row is blocked in every cell.

**V18. `footprint`.** `mail_compare footprint --protocol P --sources S --bin
T=PATH… [--runtime T=DIR…]` prints one JSON document with, per target: the
download bytes (the pinned archive's size; for the image row the sum of the
captured manifest's layer sizes; for the candidate the release binary), the
installed bytes, and the unpinned runtime as the label `unpinned, not
counted` where the row has one. It launches nothing and rechecks every digest
it relies on. The does-not-start row is blocked, with no number.

Every launched row has an installed size, as 0132 §4.5 requires
*(revision 2; Reading 9)*. `installed_bytes` is a positive integer or
`footprint` refuses with exit 2 and prints no document: no row carries
`null`, and the word `owed` appears nowhere in it.

- **The candidate:** the release binary alone; it needs no runtime.
- **A row whose artifact is the executable itself** (MailHog, the
  MailSlurper source build, GreenMail's jar, the Alkemio binary): the
  executable, its checked members, and, where the pin has a runtime, the
  whole runtime tree under `--runtime`.
- **A row whose artifact is an archive** (Mailpit, smtp4dev, Inbucket): the
  extracted archive, which is every regular file under the *extraction
  root*. The root is `--bin`'s path with the pin's `member` path removed
  from its end; a `--bin` whose path does not end with the pinned member
  refuses by name. The executable is inside the root and is counted once.
  smtp4dev's .NET bundle is therefore counted, as 0132 §4.5 requires.

One walk sizes every tree: regular files only, by their length; a symbolic
link is not followed and counts for nothing; a directory counts for nothing.
`installed_files` is the number of files counted.

*The walked trees are identified.* Each directory `footprint` walks (a
runtime tree, an extraction root) is listed in the row's `installed_trees`
with its `kind` (`runtime` or `archive`), `files`, `bytes` and `sha256`. The
digest is the sha256 of the tree's listing: one line per regular file,
`<sha256 of the file>  ./<path relative to the root>` and a newline, the
lines sorted by the path's bytes. That is what

```
(cd ROOT && find . -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum | sha256sum)
```

prints, so anyone can check it by hand. A file name that is not UTF-8, or
holds a newline or a backslash, refuses by name: `sha256sum` would escape
it and the two digests would differ. A row that walks no directory has an
empty `installed_trees`. No host path appears in the document; a tree is
named by its kind and its digest, and `installed_basis` is fixed text that
says which of the three rules above applied.

*No two targets share a tree.* `footprint` refuses by name, giving both
targets, when one target's walked tree contains another target's executable
or another target's walked tree, the candidate included. The paths are
compared after they are resolved (`canonicalize`). This is the one way a
careless extraction (every archive unpacked into one directory) would
inflate a row without changing a digest the lock holds.

What this does not prove is Reading 9's and the README says it: the lock
does not digest every file of an extracted tree. The digest published here
identifies what was measured; it does not pin it.

*(Revision 1 withdrew the archive rows' installed size as `null`, `owed`,
with an `executable_bytes` beside it. That contradicted 0132 §4.5 and §5.6;
the field `executable_bytes` is removed.)*

**V19. `hold`.** `mail_compare hold --protocol P --sources S --bin T=PATH
[--runtime T=DIR] --scratch DIR --samples FILE` launches one target under the
budget, prints its endpoints as one JSON line, samples, and waits. Every
endpoint is the address the launched target listens on, `host:port`, the
POP3 listener's included. On stdin EOF, or SIGTERM or SIGINT, it runs V14's
teardown and prints the launch and cleanup receipts as one JSON line. It
exits 0 only when cleanup is proved.
Any byte on stdin before EOF is refused by name after teardown, so a later
story can add a line protocol without changing what silence means.

**V20. Plan and `admit-v4`.** One function derives the chunk plan from the
protocol: for each repetition, for each launched target in that repetition's
rotation, one `cost` chunk. `targets --protocol P --plan v4` prints it, one
coordinate per line; the wrapper never computes it. `admit-v4 DIR PROTOCOL`:

- recomputes the plan, refuses an empty plan by name, and requires exactly
  one sealed, admitted chunk directory per coordinate, all naming one run
  id, protocol, lock and candidate commit, with the same host facts and
  target CPU pair. Every chunk, not only the first, must state
  `candidate_dirty: false`;
- requires that all launch receipts of one target, across every chunk, name
  one executable digest, whether or not `--bin` is passed;
- requires, per launch, the five limits of V9, the join read-back, the
  protocol's cap in the receipt of every capped target, and
  `cgroup_removed: true`; and per chunk as many cleanup receipts as launch
  receipts, every one proved. Each cleanup receipt names the leaf of the
  launch it is paired with, and no leaf is named twice in a chunk
  *(revision 2: the pairing was by position and target only)*;
- admits a sealed chunk with no launch receipt and no cleanup receipt
  *(revision 2)*: every per-launch requirement is then met by having nothing
  to check, and every other requirement of this list still holds for it (its
  planned rows and windows, its manifest, its host facts, its ledger entry).
  Revision 1's code refused it, so one chunk that V16 seals made a whole run
  inadmissible;
- reads the run's `footprint.json` *(revision 2)*, requires its protocol id
  to be the run's and one row per launched target, the candidate included,
  each with a positive integer `installed_bytes`, and refuses a missing
  row, a `null` or a zero by naming the target. `headline.installed` is
  recomputed from that file, so an edited cell is refused by its path like
  any other figure. Admission does not walk the trees again: the size is
  computed once per collection (0132 §4.5), and `footprint.json` is one of
  the files the run's `index.json` seals;
- requires every planned window row once and no unplanned one;
- requires one ledger entry per coordinate with exit 0 or 1 and a clean scope;
- rechecks every executable digest against the lock, and against the files
  when `--bin` is passed;
- recomputes every figure of V17 from the rows and refuses a report that
  differs, naming the first differing path.

`admit-v4 --seal` writes the run-level `manifest.json`, `host.json`,
`chunks.json`, `launches.json`, `report.json`, `report-cost.json` and
`index.json` as new files and refuses if any exists; plain `admit-v4`
verifies them. The run-level manifest carries `protocol_status` (`draft` or
`frozen`) beside the protocol's id and sha256 (Reading 2). *(Revision 1: the
per-chunk dirty flag, the per-target digest, the empty plan and the status.)*

**V21. `collect-v4`.** `scripts/compare-mail.sh collect-v4 --output DIR --bin
NAME=PATH… [--runtime NAME=DIR…] [--protocol P --sources S --documents DIR]
[--dry-run]`:

*Preflight.* It refuses by name, before building: not Linux; no
`systemd-run`, `systemctl`, `taskset` or `timeout`; an existing `--output`; a
protocol that is not version 4 (the message names the draft route); fewer
than 3 CPUs in `min(nproc/4, 32)`; a shell whose allowed CPUs are not all of
the host's (`nproc` differs from `nproc --all`), because the chunks are
pinned to CPUs `0-<n-1>` and would otherwise leave the set the operator gave;
and, for a real run only, a candidate that is uncommitted, dirty or not on
`main`.

*The candidate is on `main` (revision 2; 0132 §2.3, Reading 11).* `HEAD`
must be an ancestor of, or equal to, `refs/heads/main`, and also of
`refs/remotes/origin/main` where that ref exists
(`git merge-base --is-ancestor`). When neither ref exists the mode refuses
and says so. Nothing is fetched. A stale `main` therefore refuses a newer
`HEAD`, and a local `main` with commits `origin/main` does not have refuses
too: both fail closed. The check needs no record of its own: `main` is
append-only, so anyone can repeat it later on the candidate commit the
manifest names. `--dry-run` skips it with the other candidate checks.

`--dry-run` prints every chunk's coordinate and command and launches
nothing.

*The bound (revision 2; 0132 §11.1, Reading 10).* One prefix,
`timeout --kill-after=30 560 taskset -c 0-<n-1>`, is on every `cargo` and
driver process the mode starts after the preflight: both builds, both
`targets` calls, `check`, `footprint`, each chunk and both admissions. The
560 is V2's `CHUNK_KILL_MS` in seconds. Every `systemctl` call has its own
`timeout`: 1 s for `is-active`, 3 s each for `kill` and `stop`. What is left
unbounded is the shell's own short utilities (`git`, `nproc`, `date`,
`cat`, `mkdir`, `cp`, `sed`), which read local files and take nothing from
a target. The ten minutes then hold for every process, as a sum:

| case | terms | at most |
|---|---|---|
| a build, `targets`, `check`, `footprint` or an admission | SIGTERM at 560 s, SIGKILL 30 s later | 590 s |
| a chunk that failed (see below), and the targets it left in its scope | 560 + 30, one state query (1), `kill` (3), `stop` (3) | 597 s |
| a chunk that sealed, and anything left in its scope | the driver has exited by 560 s, or the chunk is the case above; twenty state queries (20 × 1) half a second apart (20 × 0.5); `kill` (3), `stop` (3) | 596 s |

*(Revision 1 used `timeout --kill-after=30 600`: 630 s for a step that
ignores SIGTERM, and a further scope wait for its targets.)*

*A run.* It requires `check` to pass, writes `footprint.json`, and runs the
plan in order. Each chunk is
`timeout --kill-after=30 560 taskset -c 0-<n-1> systemd-run --user --scope
-p Delegate=yes --unit=mail-compare-<run>-<coordinate> -- <driver> run …`,
in that nesting, so the driver is the scope's only process. It reads
`/proc/loadavg` before and after each chunk and writes one new ledger file
per chunk (coordinate, command, exit, wall time, both loads, whether the
scope was clean).

*A chunk's exit.* Exit 124, 137 or above 2 counts as 2. Exit 0 or 1 without
the chunk's `index.json` counts as 2. An interrupted chunk counts as 2. A
chunk that counts as 2 has *failed*; otherwise it has *sealed*.

*The scope is judged from what systemd says (revision 2).* The state is the
word `systemctl --user is-active <unit>` prints. Only `inactive` and
`failed` mean the scope is gone: systemd reports a scope in either state
only when it holds no process, and a scope whose leaf saw an OOM kill may
end as `failed`, which must not turn a measured OOM into a lost run. Every
other answer is *not known to be gone*: `active`, `activating`,
`deactivating`, `reloading`, any other word, no output, a failed query, or a
query cut by its 1 s bound. A non-zero exit status alone decides nothing.

- After a chunk that sealed, the wrapper asks at most twenty times, half a
  second apart, because systemd learns that a scope is empty after the
  driver has exited. The scope is clean at the first `inactive` or
  `failed`. A slow or failed query is asked again; it is not a dirty scope.
  If the twentieth answer is still not one of the two, the scope is
  unclean: it is killed and stopped, and the chunk counts as 2.
- After a chunk that failed, the wrapper asks once and does not wait: the
  run is already stopped, and a wait would only keep the scope's processes
  alive. Unless the answer is `inactive` or `failed`, the scope is killed
  and stopped and is ledgered unclean.

*(Revision 1 called any non-zero status of `is-active --quiet` inactive, so
a bus failure was ledgered as a clean scope; and it waited up to 10 s on
every chunk, a killed one included.)*

*One owner for a running step (revision 2).* Every bounded step is started
by one helper, in the background, and waited for there. A step whose output
the wrapper reads writes it to a file, never through `$(…)`, which would
put the step back in the foreground where a trap cannot act until it ends.
The wrapper knows a chunk as *owned* from the moment it is started until its
ledger entry has been written: the chunk's pid and unit are not forgotten
when the process is reaped.

*Signals.* On SIGINT, SIGTERM or SIGHUP the handler does two things only: it
records the interrupt, and if a step is running it sends that step's
`timeout` a SIGTERM, which passes it on (the driver's own interrupt path
tears down and exits 2, unsealed). Everything else happens in the main
flow:

- when the step that was running is a chunk, the main flow waits until it is
  reaped, judges its scope as a failed chunk's, writes its ledger entry with
  exit 2, starts nothing further and exits 2;
- when it is any other step, the main flow ends the mode with exit 2 as soon
  as the step is reaped, naming the interrupt;
- a signal that arrives while a chunk is owned but already reaped (during
  the scope's judgement or the ledger write) does not abandon the chunk: the
  judgement and the ledger entry are completed, the entry has exit 2, and
  the mode exits 2;
- a signal between two steps, when nothing is owned, ends the mode with
  exit 2 at once;
- no step is started once an interrupt is recorded.

A signal is therefore acted on within about 3 s (the longest `systemctl`
bound) plus the time the interrupted process takes to end. *(Revision 1 ran
the builds, `check`, `footprint` and the admissions in the foreground, where
bash holds a trap until the child ends, up to the whole bound; and it forgot
the chunk's pid before the scope wait and the ledger write, so a signal
there exited without the chunk's ledger entry and left a live scope.)*

*The end.* Exit 2 stops the run as incomplete; nothing is resumed,
overwritten or removed, and no `reproduce.txt` is written. Otherwise, after
the last chunk, it writes `reproduce.txt`, runs `admit-v4 --seal` and
`admit-v4`, and exits 2 if admission refuses, else 1 if any chunk exited 1,
else 0.

*(Revision 1: the wait for the scope, the signal path, the bound on every
process and the restricted-shell refusal were new. The `reproduce.txt` rule
is unchanged since the first design.)*

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
   shows `oom_kill` ≥ 1; the window outcome is `oom_killed`; the teardown's
   `oom_kills` is ≥ 1 (V15); nothing is retried; the leaf is emptied and
   removed.
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
   and on EOF a receipt with `cgroup_removed: true`. A second hold is ended
   by its interrupt instead of EOF and gives the same receipt.
5. `v4_real_cost_chunk_seals_and_admits`: one cost chunk of a small v4 draft
   fixture against the candidate seals with exit 0, every window has
   `affinity_held: true`, each cleanup receipt names the leaf of its launch
   receipt (revision 2), and its windows, samples and figures pass the
   chunk-level checks of V20. A second chunk whose "candidate" is a program
   that exits at once (`/bin/false`) seals with exit 1: every profile is not
   measured with `target_exited`, each launch has its launch receipt and a
   proved cleanup with `cgroup_removed: true`, and no figure of it is a
   number.

Each fails, never skips, when V8 refuses or its bytes are missing. Without
delegation the failure message is V8's exact refusal.

**The host run gates the merge.** A proof is passed only by its output from a
host where it ran. The fixture tests prove the driver's logic on plain
files; they do not prove the join through `cgroup.procs`, `cgroup.kill`, the
removal of a leaf, the OOM or the quota, and no reading of the code does.
The five therefore run on the dispatcher host, one at a time under the
command above, at the head that is to be merged, and their passing output
with the load before and after is attached to the PR. Until that is done
the story is not complete, no seat reports it complete, and the PR is not
merged. A seat whose box has no cgroupfs cannot discharge this and says so
in its result, with the refusal output of all five: that is the stated
state of AC18b, not a claim that it is met. *(Revision 1: the first text
let a seat report the proofs as "owed" without saying what that held up.)*

*What a seat's typed result states (revision 2).* The engine gives a seat
one word for "my tasks are done" and the run cannot reach review without
it. That word is about the seat's own tasks. It says nothing about AC18b
and discharges nothing: review 2 found the implement seat's `complete`
beside notes that said the story was not complete, and read it correctly as
not discharging the gate. So the rule is on the notes: a seat that could not
run the five on a kernel writes, in these words, "AC18b is open and the
story is not complete", with the refusal output, whatever its typed word
is. Only the host output closes AC18b. Revision 2 changes code the proofs
exercise (the join comparison, the cleanup receipt, the chunk's end), so the
five run at the revised head; no earlier output counts.

**V23. Docs.** `docs/evals/README.md` gains a v4 section: what story A
measures, the budget and mechanism, the refusals, the draft route and
`collect-v4`, Reading 4's sentence, "a ceiling, not a reservation" for the
CPU budget, `memory.current` labelled as including reclaimable cache beside
`anon`, and that v3 still collects from the active paths. Revision 1 makes
these sentences exact: the SMT siblings of the two target CPUs are left idle
*where they are inside the chunk's CPU set* and are recorded as
`siblings_outside_set` otherwise; affinity is read back at the end of every
window; it is a *frozen* v4 that this driver refuses, while `admit-v4`
admits a draft run and labels it a draft; a value that was not observed is
`null` and never zero; a startup that is alive but not ready stops the run
(Reading 7); a sealed run proves the directory's state at the seal, not
that a host power loss cannot lose the tail of `samples.jsonl`; and the host
run of the five proofs is what the merge waits for.

Revision 2 replaces the sentence that an archive row's installed size is
owed, and adds: every row has an installed size and the headline carries it;
how to extract an archive for it (a new, empty directory for each pinned
archive, the extracted member passed as `--bin`); what the tree digest is,
with the command that reproduces it, and Reading 9's limit in plain words
(the lock digests the archive and the executable, not every extracted file);
the prefix `timeout --kill-after=30 560` with V21's three sums in place of
every `600` the v4 section states for `collect-v4`; that the candidate must
be in `main`'s history and which refs are read; and that a scope is clean
only when systemd answers `inactive` or `failed`. The proofs' own command
keeps `timeout 600`, which is 0132's wording for them.

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
  value, refuses. A `chunk_deadline_ms` exactly at V2's ceiling passes and
  one millisecond above it refuses, naming the ceiling; the ceiling moves
  with the startup and cleanup budgets, and with the draft's budgets it is
  480000 (`CHUNK_KILL_MS` is 560000; revision 2). Each new field on a
  protocol 2 or 3
  refuses. The v2 and v3 history bytes still parse, validate and
  re-serialize to the same value.
- **AC3 (V3).** A frozen protocol 4 with a profile added, removed or changed
  from the table refuses. Any protocol 4 with 33 sessions, a wait profile, no
  headline or another headline profile refuses.
- **AC4 (V4, V6).** A protocol 4 cap other than 150,000 refuses, and so does
  a `null` cap for Swarmail. Lock 5 with a literal cap, with the placeholder
  outside the cap flag, with the cap flag twice (placeholder then literal,
  in each of the three forms), or lock 4 with the placeholder, refuses. The
  spec built from lock 5 and protocol 4 carries
  `150000` at each of the three cap flags and Swarmail's
  `--max-per-inbox 150000`; a spec with an unresolved placeholder refuses
  before spawn.
- **AC5 (V5).** A frozen protocol 4 refuses in this driver, naming the owed
  sections. With the owed list passed as empty, it still refuses for a
  non-empty `unresolved`, an empty `clients_sha256` and a `not_inspected`
  capability, each on its own.
- **AC6 (V7).** The two history copies have the frozen sha256 values and the
  active files still have them too. The draft protocol validates as a draft
  and `check` reports it not ready for collection. Its `chunk_deadline_ms`
  is 480000, and its `unresolved` list names GreenMail's REST read path and
  the wait budget. Lock 5 validates, verifies its documents through
  `--documents docs/evals`, and pairs with the draft.
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
  left, siblings outside the set. The join comparison refuses a cgroup with
  the leaf's name under another parent, and passes the leaf's own path
  (revision 2). The join itself is proved by real proofs 2 to 4.
- **AC10 (V12, V13).** A tmpfs scratch refuses (on Linux, against
  `/dev/shm`). The parsers of `cpu.stat`, `memory.stat`, `memory.events`,
  `memory.current`, `memory.peak`, `pids.current` and `cgroup.procs` pass on
  captured text and refuse malformed text. A resource row in a protocol 3 run
  directory, a duplicate or unplanned window, and a missing planned window
  each refuse. So do a `measured` window without counters, a window with
  some counters and not others, and a `sample` row that names launch `none`.
  `affinity_held` is `true`, `false` and `null` for a fixture leaf whose
  tasks all hold the pair, whose one task differs, and that holds no task,
  in each of the three window kinds. A never-opened window given up at the
  chunk deadline is `oom_killed` when its instance's leaf shows an OOM kill,
  `target_exited` when only the leader has exited, and `chunk_deadline`
  otherwise (revision 2).
- **AC11 (V14, V15).** On a fixture leaf whose `cgroup.procs` never empties,
  teardown ends at the budget with `cgroup_removed: false`, the leaf in place
  and cleanup unproved. A v3 cleanup receipt and a v3 launch receipt
  serialize byte for byte as before. A leaf launch whose receipt lacks
  `cgroup_removed` is not proved. A leaf launch's cleanup receipt carries
  `oom_kills` and `exited_early`, read from a fixture leaf and from a leader
  that exited before the teardown. It also names its leaf, with the name its
  launch receipt carries; a cleanup receipt of a launch without a leaf has
  no `leaf` key (revision 2).
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
- **AC14 (V18; rewritten in revision 2).** `footprint` on fixtures gives
  the archive size, the layer sum, the candidate's file size, the installed
  sum including a runtime tree for an executable row, the
  `unpinned, not counted` label, and a blocked row; a changed executable
  refuses. For an archive row built by the test (the member, two more files,
  a nested directory and a symbolic link):
  - `installed_bytes` is the sum of the regular files under the extraction
    root, the executable counted once and the link not at all, and
    `installed_files` is their count;
  - `installed_trees` has one entry of kind `archive` whose `sha256` equals
    a digest the test computes on its own from the listing format of V18,
    and an executable row with a runtime has one of kind `runtime`; adding,
    removing or changing one file changes the digest;
  - a `--bin` whose path does not end with the pinned member refuses by
    name, and so does a tree that holds another target's executable, a tree
    inside another target's tree, and a file name with a newline or a
    backslash;
  - no row of any document `footprint` prints has `installed_bytes` `null`
    or zero, the string `owed` or the key `executable_bytes`, and no value
    in it is a path of the fixture's directory.
- **AC15 (V19).** `hold`'s refusals (missing flags, protocol 3, bytes on
  stdin) are proved without a cgroup, and so are two facts on a launch
  without a leaf: every printed endpoint is the `host:port` the target
  listens on, the POP3 listener's included, and a hold ended by its
  interrupt runs the teardown and prints a receipt with cleanup proved. Its
  behaviour under the budget is real proof 4.
- **AC16 (V20).** On a fixture run directory built by the test, a complete
  one seals and then verifies, and each of these refuses by name: a missing
  chunk, an extra chunk, an unsealed chunk, two run ids, a dirty candidate
  in the first chunk and in a later one, a target whose executable digest
  differs between two chunks, differing host facts, a limit other than V9's,
  a missing join read-back, a cap other than the protocol's, `cgroup_removed`
  false or absent, a cleanup receipt without its launch receipt, a ledger
  exit 2, an unclean scope, a second `--seal`, and one edited figure, which
  is named by its path. A protocol whose plan is empty refuses by name and
  does not panic. The sealed manifest carries `protocol_status: draft` for a
  draft. Revision 2 adds:
  - a run directory in which one chunk sealed with no launch (every row
    `unstarted`, every window never opened with `chunk_deadline`, no
    receipt) seals and verifies, and the same directory still refuses when
    another chunk, which did launch, lacks a cleanup receipt;
  - a cleanup receipt that names another launch's leaf, two cleanup
    receipts that name one leaf, and a cleanup receipt of a leaf launch
    without a leaf name each refuse by name;
  - the sealed run-level report has `headline.installed` with one entry per
    launched target equal to the fixture's `footprint.json`, and a chunk's
    report has no such key; a `footprint.json` that lacks a launched
    target's row, or states `null` or `0` for its `installed_bytes`, or
    names another protocol, refuses naming the target or the protocol; an
    edited `headline.installed` cell is refused by its path; and a
    `footprint.json` changed after the seal makes the verify refuse.
- **AC17 (V21; revision 2 marks its additions).** With a fake driver and
  fake `systemd-run`, `systemctl`, `timeout` and `cargo` on `PATH`:
  - `--dry-run` prints 24 commands in plan order and starts nothing; the
    default protocol refuses and names the draft route; each preflight
    refusal is named, the restricted shell among them; off Linux the mode
    refuses.
  - *Candidate (revision 2).* In the fixture repository a clean commit on a
    branch that is not in `main`'s history refuses and names `main`; so does
    that branch after `main` was merged into it; a detached `HEAD` at an
    earlier commit of `main` and one at its tip both pass; a `HEAD` that is
    in `refs/heads/main` but not in an existing `refs/remotes/origin/main`
    refuses and names that ref; a repository with neither ref refuses and
    says so; `--dry-run` passes in each of these.
  - *Chunk exits.* A chunk exit 2, exit 3, a timeout (124), a kill (137), an
    exit 0 without an index and a scope that stays active each stop the run
    with exit 2 and a ledger entry, with no later chunk started and no
    `reproduce.txt`.
  - *Scope (revision 2).* After a sealed chunk: `active` once and then
    `inactive` is clean and the run goes on; `failed` is clean; a query that
    prints nothing and exits non-zero is asked again and a later `inactive`
    is clean; twenty answers none of which is `inactive` or `failed`, in
    each of three forms (`active`; no output with a non-zero status; an
    unknown word), end with the scope killed and stopped, `scope_clean`
    false in the ledger and exit 2. After a failed chunk (124) exactly one
    state query is logged before the `kill` and the `stop`, and none when it
    answers `inactive`.
  - *Signals (revision 2).* A SIGTERM to the wrapper while a chunk runs ends
    that chunk, writes its ledger entry with exit 2, starts no later chunk
    and exits 2. A SIGTERM while a build, `check`, `footprint` or an
    admission runs ends the mode with exit 2 while that fake process would
    still be running, and the fake process is gone afterwards: the test
    waits, with a deadline, for the fake's marker, signals, and requires the
    wrapper to have exited before the fake's own duration has passed. A
    SIGTERM that arrives while the chunk is reaped and its scope is being
    judged (the fake `systemctl` writes a marker and waits) still leaves the
    chunk's ledger entry, with exit 2, and a scope that was not gone is
    killed and stopped.
  - *A full run.* A full fake run writes 24 ledger files, writes
    `reproduce.txt` only after the last chunk, and calls `--seal` and then
    verify; a seal that succeeds followed by a verify that refuses exits 2.
  - *The bound (revision 2).* The logged commands show both builds, both
    `targets` calls, `check`, `footprint`, every chunk and both admissions
    under `timeout --kill-after=30 560 taskset`; the chunk's `560` equals
    `CHUNK_KILL_MS` ÷ 1000; every logged `systemctl` call is under its own
    `timeout` (1 for `is-active`, 3 for `kill` and for `stop`); and
    560 + 30 + 1 + 3 + 3 and 560 + 20 × 1.5 + 3 + 3, computed in the test
    from the numbers it read in the log and the script's poll count, are
    each at most 600. No `timeout` line logged by a `collect-v4` run carries
    `600`.
- **AC18a (V22, provable by a seat).** The five proofs exist, are ignored and
  Linux-only, assert what V22 lists (the additions of revisions 1 and 2
  included), and each fails with V8's exact refusal where there is no
  delegation.
- **AC18b (V22, the merge gate).** The passing output of each of the five
  from the dispatcher host, run one at a time under V22's command at the
  head to be merged, with the load before and after, is attached to the PR.
  The story is not complete and the PR is not merged before that. A seat
  that cannot run them says in its notes that AC18b is open and the story
  is not complete; its typed result states neither (V22, revision 2).
  Output from a head before revision 2's code does not count.
- **AC19 (V23, V24).** The README section exists and the decision inventory
  test passes. `git diff e23a8ba -- src evals Cargo.toml Cargo.lock deny.toml`
  is empty, and the only existing file changed under `docs/evals/` is
  `README.md`.
- **AC20.** Every gate of `docs/house-rules.md` § Gates passes `--locked` on
  the final tree, the coverage selftest and the publish-guard proof included.
- **AC21 (V16, the chunk deadline in flight; revision 1).** Driving the cost
  chunk's own code against a real target instance on a fixture scope
  (`run_chunk`, and its stages with a `chunk_end` the test passes), not
  hand-built rows. Each test places `chunk_end` by construction, never by
  waiting for it between two steps of a healthy target:
  - with `chunk_end` already passed at entry, nothing is launched, every
    profile is not measured with `chunk_deadline`, and the chunk seals with
    exit 1;
  - with `chunk_end` passed before the measured send, every index is
    `unstarted` with reason `chunk deadline`, no listing is made, the
    verification row is `incomplete` with that reason, and the `measure`
    window is `chunk_deadline`;
  - against a listener that accepts and never answers, a send in flight is
    cancelled at `chunk_end`: the call returns before one request deadline
    has passed, the index has exactly one row whose reason names
    `chunk deadline`, and the rows count the profile's messages exactly;
  - a warmup cut by `chunk_end` launches no measured instance: the only
    launch receipt, if there is one, is the warmup's, every cleanup is
    proved, and the profile is not measured;
  - a profile's own `send_deadline_ms`, when it comes first, still writes
    `send deadline`;
  - a fidelity read cut by `chunk_end` leaves visible rows without
    `fidelity_ok`, an `incomplete` verification and a repetition that is not
    exact, named `timeout` and not `fidelity` or `lost`;
  - no `QUIT` after the instant (revision 2), against a test-owned listener
    that records every line it receives: a reused session whose sends ended
    at `chunk_end` is dropped and the listener received no `QUIT`; a fresh
    session whose send was accepted just before a `chunk_end` that has
    passed when the `QUIT` would begin sends none, and its row stays
    `accepted`; a `QUIT` in flight against a listener that never answers it
    is cut at `chunk_end` and the call returns before one request deadline
    has passed; before `chunk_end` the listener receives `QUIT` as before;
  - in every case each planned window has its one row and the teardown is
    proved.
- **AC22 (V15, V16, a launch that does not become ready; revision 1).** The
  classification is exact on receipts: `oom_kills` ≥ 1 gives `oom_killed`
  whatever `exited_early` says; `exited_early` alone gives `target_exited`;
  neither, a missing launch receipt, or an unproved cleanup is a launch
  refusal. A chunk whose target exits at once seals with exit 1, its
  profiles not measured with `target_exited` and reason `target exited`, and
  goes on to its next profile; a join mismatch is still exit 2 and unsealed;
  a protocol 3 run of the same target is still an infrastructure failure, as
  before.
- **AC23 (V13, V17, no observation, no number; revision 1).** With hand-built
  rows: a repetition that was not measured has `null` for every figure, in
  the per-repetition list and in the cell; with one of three repetitions not
  measured the median, minimum and maximum are over the other two and `n` is
  2; with all three not measured every figure is `null` and none is `0`; the
  idle and startup cells are then unranked and name `timeout`, `oom` or
  `target_exited` as the window says; a target that exited during its idle
  window has no idle figure; an OOM-killed `measure` window that was opened
  keeps its CPU and peak figures and is unranked with `oom`; rows `unstarted`
  because the target failed before readiness do not name `timeout`. No JSON
  number in a report built from rows without counters is `0` where a figure
  is meant. An idle cell one of whose repetitions has current memory and
  `anon` but no scratch footprint is unranked, and so is one that lacks
  either of the other two (revision 2).

## Out of scope

- Stories B to E: the `api`, `waits` and `testexec` sections, operation and
  wait capability rows, the `code_link` corpus kind, fill levels and
  `mib_per_1k_retained`, clients and `clients_sha256` values, the frozen v4
  and lock 5 hashes, the swap of the active records, collection, the write-up
  and the README headline.
- GreenMail's REST read path (0132 §6.2, acceptance answer 8). In this
  story's draft the cost section verifies as v3 does, over POP3; the draft's
  `unresolved` list names the REST path as owed by story B (V7), so no v4
  can freeze, and nothing can be collected, on the POP3 path.
- A CLI-level test of the driver's mode shims in `benches/mail_compare.rs`,
  and a signal test of `hold` through a real SIGTERM. Every function the
  shims call is tested; the tests seat deferred the shims to the story that
  next touches them.
- A hostile-body, TLS, webhook or chaos campaign (0132 non-goals). The `DATA`
  writer already dot-stuffs (`wire.rs`, `dot_stuff`).
- cpuset delegation, an unenforced pass, a container runtime, a second
  supervisor, a new package.
- Reading or extracting an archive in the driver or the wrapper, an archive
  argument on `footprint` or `collect-v4`, and a lock field that pins an
  extracted tree (Reading 9; plan.md § *Revision 2* gives the cost).
- A host-load gate, a sixth real proof, a raw exit code beside the mapped
  one in the ledger, and a new outcome word for a target that is alive but
  not ready: raised again in round 3 and left as they were.

## For the operator at freeze (story E)

Story A leaves these open and labels them as such. None blocks this story.

1. `chunk_deadline_ms`: the draft says 480000, and since revision 2 that is
   V2's ceiling with the draft's budgets (30 s to start, 10 s to clean up).
   A longer chunk deadline now needs a shorter startup or cleanup budget;
   the ten minutes of 0132 §11.1 leave no other room. A chunk is one
   target's four profiles of one
   repetition (0132 §11.1), so a target slow enough to use the deadline in
   an early profile leaves its later profiles of that chunk not measured,
   published unranked as `timeout`.
2. Whether a host-load ceiling should gate a chunk. Story A records the load
   beside every chunk and gates nothing: a gate would abort a day-long run on
   a neighbour's burst, and its threshold would be a frozen protocol value
   that 0132 does not name.
3. Whether a cell labelled `affinity_widened` should rank.
4. A target that is alive but not ready within `startup_deadline_ms` under
   the budget still stops the whole run with exit 2 (Reading 7). Hold each
   target once under the budget (`mail_compare hold`) before a day-long run,
   and set the startup budget from what that shows.
5. The installed size of an archive row is measured on the tree the
   operator extracted and is identified by a digest, not pinned by the lock
   (Reading 9, V18). Extract each pinned archive into a new, empty
   directory. If the operator wants the tree bound to the lock, that is a
   lock field 0132 does not name: an amendment, not a part of this story.
   Nothing is owed here: every row has its number.
6. `collect-v4` has run only against fake tools: a draft cannot pass `check`
   and a frozen v4 does not exist yet. The first real chunk is the first
   real test of how the scope deactivates under systemd 259. A run that
   stops at its first chunk costs one empty directory.

## Stop triggers

Stop and report, without working around it, if: a requirement can only be met
by editing `src/`, `evals/`, a lockfile, `deny.toml` or an active or
historical record; lock 5 cannot verify against the captured documents
without changing one; a v3 test can only pass by changing v3 behaviour; or
the gates' coverage denominator turns out to include the bench. Revision 1
adds: stop if the cut at `chunk_end` can only be built by changing what a
protocol 1 to 3 run does, or if something other than a launch, a teardown and
the seal turns out to run after `chunk_end`, so that V2's bound would be
false. Revision 2 adds: stop if an archive row can only be sized by reading
the archive itself (a decompressor, a package, a host `tar` or `unzip`, or a
new `collect-v4` argument); if one of V21's three sums cannot be kept at or
under 600 s; or if the smoke fixture or the draft cannot stay at or under
V2's new ceiling without changing a budget.
