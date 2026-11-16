# Plan — mail comparison protocol v4, story A

Spec: [spec.md](spec.md). Authority: accepted
[0132](../../docs/decisions/0132-mail-comparison-protocol-v4.md), story A.
Base `e23a8ba`. No new decision record: nothing here amends a ruling.

## Technical context

- The bench driver `benches/mail_compare.rs` and its modules under
  `benches/mail_compare/` (`harness = false`). Its suite is
  `tests/mail_compare.rs`, which compiles the same modules by `#[path]`, with
  `tests/mail_compare/eight_targets.rs` as a sibling module. The bench is
  outside the root coverage denominator (`src/` only), so the cost of new
  code is clippy `-D warnings` over both compilations, not coverage.
- `libc` 0.2 and `tokio` are already dev-dependencies. No package is added.
- The suite runs on ubuntu and macOS in CI (`test (macos-latest)` is a
  required check). Nothing in CI or in a seat's box has a delegated cgroup:
  the design seat's box has no `/sys/fs/cgroup` at all (read 2026-11-16), and
  its `/tmp` and `/dev/shm` are tmpfs.
- Collection host: `nproc` 128, kernel 7.0, systemd 259, `user@.service`
  delegates `pids memory cpu` (0132 §3.4). `/tmp` there is a RAM tmpfs.
- Facts checked in the tree for this design:
  - `FROZEN_PROTOCOL_V3_SHA` and `FROZEN_LOCK_V4_SHA` exist
    (`tests/mail_compare/eight_targets.rs:17-19`, asserted at `:4566`). The
    intake and the robustness position say they do not; they do.
  - `wire.rs:327` `dot_stuff` already implements RFC 5321 transparency.
  - `LaunchFacts.argv_template` keeps the template on purpose: expanded
    values carry private paths and ports (`targets.rs:167-177`).
  - `report::admit_run` refuses any report whose version is not
    `REPORT_VERSION` (`report.rs:449`).
  - The active lock is compiled into the eval crate's tests
    (`evals/tests/comparison.rs:14`, `competitor.rs:15`) and is what the
    adapters' `--launch` mode builds a launch from (`mcp.rs:92-104`).
  - Warmup already runs on its own instance (`runner.rs:427`).

## Constitution check

- **0001–0005.** No file under `src/` changes; no server behaviour changes.
  A competitor's `250` borrows nothing from the candidate's accept == stored.
  An OOM or exit mid-window is never retried, so a lost accepted message
  cannot be masked.
- **0091 §1, §6.** Every started operation keeps its row; an incomplete run
  is never resumed or overwritten. **§7.** Every figure keeps its run id,
  cell and denominator.
- **0092 §6, 0093 §7.** One owner, parent and reaper. The teardown sequence
  is unchanged and runs first; the cgroup step is added after it. systemd
  creates the driver's scope and supervises no target.
- **0093 §3.** v4 is the enforced profile; v2 and v3 stay
  `unenforced_local` and never touch cgroupfs.
- **0130.** Frozen v3 and lock 4 bytes are not edited; no v3 cell merges
  with a v4 cell.
- **0132.** Implemented as accepted. The six readings in the spec are
  readings, not amendments; three open values go to the operator at freeze.
- **0020.** Applies to `src/`; the bench suite follows its writing
  disciplines anyway: poles only, no sleep as synchronization (the 5 s idle
  point and the spinner window are the measurement, bounded by a real
  deadline), a fresh directory per test.
- **0045 §8.** Compatible work: a new protocol and lock version beside the
  frozen ones.
- **0055.** `evals/` is untouched. **0056.** Unsigned commits, no push.
- **Dependencies.** Nothing added; `deny.toml` unchanged.
- **Held PR.** It changes `scripts/compare-mail.sh`; the operator approved
  that change by accepting 0132, and the merge stays the operator's.

## Synthesis — the panel's positions, ruled

| # | Point | Simplicity | Robustness | Ruling | What the losing side costs |
|---|---|---|---|---|---|
| 1 | Type shape | Extend `Protocol` and `SourceLock` in place by version | silent | **Simplicity** | none |
| 2 | Active records | Put the v4 draft at `docs/evals/protocol.json` now, `git mv` v3 to history | Copy to history now, swap at story E | **Robustness.** The active lock is read by the eval crate's tests and the adapters' launch mode, and v3 is frozen and not yet collected | Two copies of the same bytes until E, and a `draft/` directory. The swap now would put `{max_messages}` in front of the agent-suite adapters and make `collect` refuse by default |
| 3 | Frozen v4 / lock 5 hashes | Wait for story E | Add the v3 / lock 4 constants | **Simplicity**; the v3 constants already exist and are extended to the history copies | none |
| 4 | Sections and plan size | `cost` only; later stories add theirs | Recompute the 264-chunk plan in Rust | **`cost` only, plan owned by Rust, and a frozen v4 refused while a section is owed.** The intake's 264-chunk criterion is overridden | The 264 total is not asserted until story D lands; the owed list is what prevents a short "v4" |
| 5 | `pre_exec` | fd opened before fork; the child only writes | Every syscall checked, abort on failure, read-back from `/proc` after spawn | **Both** | none |
| 6 | Affinity evidence | Set it | Read it at start, mid-window and end; label or refuse | **Read at launch and at each window's end; label only** | A widen-then-narrow inside one window is unseen; `cpu.max` still bounds the rate |
| 7 | `cgroup_removed` gate | `Option`, proved unless `Some(false)` | Required by both gates; absence is not true; bump `REPORT_VERSION` | **`Option` in the type, `Some(true)` required for a leaf launch; report version 3 for protocol 4 only** | A global bump would stop the admitted 2026-10-30 collection, and any v3 collection, from admitting |
| 8 | Receipt argv | The template | The substituted argv | **The template, with the cap resolved when the spec is built** | The receipt shows `150000` but not the ports or paths, which 0093's privacy rule keeps out |
| 9 | Peak and mean memory | Derived from samples at report time | silent | **Simplicity** | none |
| 10 | Sampler start | implicit | Before readiness | **Robustness**: from leaf creation | none |
| 11 | CPU pair | Layout computed per chunk | Chosen once per run | **A deterministic function, and admission requires one pair across the run** | No run-level plumbing; a changed pair is caught at admission, not at launch |
| 12 | Host load | Record it | Gate on a frozen `max_loadavg` | **Record.** Listed for the operator at freeze | A cell measured on a busy host is published with its load, not refused |
| 13 | Detection refusals | One string | Several named, and a mountinfo check for overlay on tmpfs | **The exact string for delegation; file-named refusals for the leaf; `statfs` only** (0132 §3.8) | An overlay whose upper layer is tmpfs would pass |
| 14 | Driver death | not raised | Sealed-chunk rule and a real driver-kill proof | **Sealed-chunk rule, and the wrapper stops a scope that outlives its chunk; proved with fakes in CI** | systemd's own stop semantics are trusted, not re-proved by a real proof |
| 15 | Wrapper nesting | `systemd-run … -- timeout … taskset … driver` | silent | **Rejected.** `timeout` would stay in the scope's own cgroup, and a cgroup with a member process cannot enable controllers for its children | none: `timeout … taskset … systemd-run … -- driver` is the same bound |
| 16 | Real proofs | Three plus `hold` | Three plus driver kill | **Five: the three of 0132, `hold`, and one cost chunk** | Two more host runs, each a few seconds |
| 17 | Hostile `DATA` body | not raised | A dot-stuffing unit test | **Already true** (`wire.rs:327`) | none |
| 18 | `pids.max` hit | not raised | A `pid_limit` label | **Rejected for this story** | A target that hits the task limit shows as its failures or `target_exited`, unexplained |
| 19 | `hold` on a signal | EOF only | Same teardown on SIGTERM and SIGINT | **Robustness** | none |
| 20 | Test home | `tests/mail_compare/v4.rs` | inline in `tests/mail_compare.rs` | **Simplicity** | none |

Three points neither position raised, added here:

- **A per-chunk deadline in the driver** (Reading 5). Four profiles at v3's
  300 s send deadline can exceed the wrapper's 600 s kill, which would turn a
  slow target into a lost run.
- **macOS.** `resources.rs` must compile there and refuse.
- **The seat cannot run the real proofs** (Risk 1).

## Design

### Module map

| File | Change |
|---|---|
| `benches/mail_compare/protocol.rs` | V1–V6: constants, optional fields (`Resources`, `Protocol.headline`, `Protocol.clients_sha256`), `validate_v4` sharing protocol 3's rules, the owed-section constant with a validator entry that takes the owed list, lock 5 in `SourceLock::validate`, `cap_check` and `launch_checks`, `pairing`, `collection_ready` |
| `benches/mail_compare/resources.rs` (new) | V8–V14, V18: detection and adoption over a root path, `Leaf` (create, limits, read-back, kill, remove), the pre-fork join handle, the `/proc` read-backs, the two parsers (flat `key value` files; single-value files), layout, `statfs` check, allocated-bytes walk, sampler, snapshots, footprint |
| `benches/mail_compare/targets.rs` | V6, V10, V14, V15: `from_pin` takes the cap; `Owned::launch` takes an optional leaf context; `cleanup` gains the cgroup step; `LaunchFacts`, `CleanupFacts`, `LaunchError::Leaf` |
| `benches/mail_compare/records.rs` | V13, V16: `ResourceRow`, the two-family reader for protocol 4, planned windows, `Manifest.chunk` and `Manifest.host` (optional, omitted when absent) |
| `benches/mail_compare/runner.rs` | V16, V20: `plan_v4`, `--chunk`, the cost chunk loop with windows, outcomes and the chunk deadline |
| `benches/mail_compare/stats.rs` | V17: nearest-rank median with minimum and maximum over `f64` |
| `benches/mail_compare/report.rs` | V17, V20: the `cost`, `idle`, `startup` and `headline` keys, report version by protocol, `admit_v4` and seal |
| `benches/mail_compare.rs` | `hold`, `footprint`, `admit-v4`, `targets --plan v4`, `run --chunk`; each a thin shim over a module function |
| `scripts/compare-mail.sh` | V21: the `collect-v4` case and its usage text. bash 3.2 syntax only, as the file states |
| `tests/mail_compare.rs` | declares `resources` by `#[path]` and `mod v4`; v3 constants read the history copies |
| `tests/mail_compare/v4.rs` (new) | every v4 test and the five real proofs |
| `tests/mail_compare/eight_targets.rs` | the frozen-hash assertion also covers the history copies |
| `tests/mail_compare/v4-smoke.json` (new) | a small protocol 4 draft for real proof 5 |
| `docs/evals/draft/protocol.v4.json`, `docs/evals/draft/sources.lock.v5.json` (new) | V7 |
| `docs/evals/history/2026-11-07-protocol.v3.json`, `…-sources.lock.v4.json` (new) | V7, byte-identical copies |
| `docs/evals/README.md` | V23 |

Not touched: `src/`, `evals/`, `Cargo.toml`, both lockfiles, `deny.toml`,
`docs/evals/protocol.json`, `docs/evals/sources.lock.json`, every existing
history file, `docs/decisions/`.

### Data model

New or extended shapes. Every new field on an existing type is
`Option`, `#[serde(default, skip_serializing_if = "Option::is_none")]`.

```
Resources      + mechanism, cpu_placement, swap_mib, tasks,
                 sample_interval_ms, chunk_deadline_ms
Protocol       + headline { profile }, clients_sha256 { name: hex64 }
LaunchFacts    + leaf (name only), limits { memory_max, memory_swap_max,
                 memory_oom_group, cpu_max, pids_max } (strings as read),
                 cpu_layout { target_cpus, idle_siblings, client_cpus,
                 siblings_outside_set }, joined { cgroup, cpus_allowed },
                 env_template
CleanupFacts   + cgroup_removed, memory_peak_bytes
Manifest       + chunk (coordinate), host { kernel, systemd, cpu_model,
                 nproc, siblings, cpu_layout, governor }
ResourceRow    kind = sample | window   (deny_unknown_fields; see V13)
Chunk          Cost { target, repetition }; name "cost.<target>.r<rep>"
```

The leaf name, not the cgroup path, goes into receipts: the path carries the
uid and the unit name. `joined.cgroup` is the path's last component.

A protocol 4 samples reader tells the families apart by the `kind` key, which
no message row has; the protocol 1 to 3 reader is unchanged, so a resource
row fails its `deny_unknown_fields` there.

### Contracts

```
mail_compare targets   --protocol P --plan v4
mail_compare run       … --chunk cost.<target>.r<rep> [--systemd VERSION]
mail_compare hold      --protocol P --sources S [--documents DIR] --bin T=PATH
                       [--runtime T=DIR] --scratch DIR --samples FILE
mail_compare footprint --protocol P --sources S [--documents DIR]
                       --bin T=PATH… [--runtime T=DIR…]
mail_compare admit-v4  [--seal] DIR PROTOCOL [--bin T=PATH…]
compare-mail.sh collect-v4 --output DIR --bin NAME=PATH… [--runtime NAME=DIR…]
                       [--protocol P --sources S --documents DIR] [--dry-run]
```

Exit codes keep the driver's meaning: 0 done, 1 sealed with workload
failures, 2 refusal or unproved lifecycle, never sealed. With `--chunk` only
that chunk's target needs a `--bin`.

Run directory written by `collect-v4`:

```
DIR/footprint.json
DIR/chunks/<coordinate>/{manifest.json,samples.jsonl,report.json,index.json}
DIR/ledger/<coordinate>.json
DIR/reproduce.txt
DIR/{manifest.json,host.json,chunks.json,launches.json,report.json,
     report-cost.json,index.json}          written by admit-v4 --seal
```

Refusal texts that tests pin exactly:
`resources: no delegated cgroup v2 (cpu memory pids)`. Every other refusal
starts with `resources: ` and names its file or cause.

### Implementation notes

- **Testable without a kernel.** Every `resources.rs` function that reads or
  writes cgroupfs, `/proc` or `/sys` takes its root as a path. Fixture trees
  of plain files in a temporary directory prove detection, adoption order,
  the limit writes and read-backs, the parsers and the teardown that never
  empties. What only the kernel can do (the join, `cgroup.kill`, `rmdir` of
  a leaf, the OOM, the quota) is proved by the real proofs. Do not bend
  production code so that a plain directory imitates `rmdir` of a cgroup.
- **Controller read-back.** `cgroup.subtree_control` reads back as
  `cpu memory pids`; compare token sets, ignoring a leading `+`, so the same
  check holds on a fixture.
- **Off Linux.** The `libc` calls that exist only on Linux
  (`sched_setaffinity`, `cpu_set_t`, the `statfs` magic numbers) sit behind
  `#[cfg(target_os = "linux")]`, as `targets.rs` already does for its `/proc`
  readers. The other branch returns V8's exact refusal.
- **Affinity of the driver.** `sched_setaffinity(0, …)` sets one thread.
  Pin the driver to the client set before the tokio runtime is built, so
  every worker thread inherits it. The real proofs do the same at the top of
  the test, before they build their runtime.
- **The join.** `pre_exec` runs in the forked child of a multi-threaded
  process: the closure captures the raw fd, a `cpu_set_t` by value and the
  `rlimit`, and calls only `write`, `sched_setaffinity` and `setrlimit`. The
  driver closes its copy of the fd after the spawn. If the host refuses the
  write through the inherited fd, the permitted alternative is 0132 §3.3's
  own wording: `open` and `write` in the child on a path precomputed as a
  `CString` before the fork. Nothing else changes.
- **The cap.** `LaunchSpec::from_pin` gains the cap as an argument and
  resolves `{max_messages}` in the launch and its environment. The wire run
  passes the protocol's retention cap for the target; the adapters' launch
  mode has no protocol and passes the pin's own stated cap, which is unused
  under lock 4 and right under lock 5. `launch_checks` keeps refusing an
  unknown slot, so an unresolved placeholder is `LaunchError::Template`.
- **Cleanup budget.** The cgroup step shares `limits.cleanup`'s `until`. The
  kill is always written and `cgroup.procs` is read at least once after it,
  even when the earlier steps used the whole budget.
- **A target that dies in a window.** `Owned` already knows (`exited`). The
  protocol 4 path turns that into rows and a window outcome instead of an
  infrastructure verdict; the protocol 1 to 3 path keeps today's behaviour.
- **Report version.** `REPORT_VERSION` stays 2. Protocol 4 writes and admits
  version 3; `admit_run` keeps refusing a protocol 4 run, which only
  `admit_v4` admits.
- **The ledger.** bash writes one small JSON object per chunk with a string
  escaper for `\` and `"`; a value with a control character is refused at
  preflight. `admit-v4 --seal` assembles `chunks.json` from the ledger files.
- **A test override for the driver.** The wrapper's existing
  `COMPARE_MAIL_*_BIN` overrides gain one for the bench driver, used by the
  fake-driver tests; the ledger records the driver path that ran.
- **Scratch in tests.** `tempfile::tempdir()` is on tmpfs in the seats and
  on the collection host. The real proofs and any test that needs a
  disk-backed scratch use `env!("CARGO_TARGET_TMPDIR")`.
- **Draft data.** Generate `docs/evals/draft/*.json` from the active files
  by a reviewed transformation (version, id, reason, resources, profiles,
  caps, headline, unresolved; the three launches), keep the key order and
  two-space layout of the originals, and let AC6's tests be the proof. The
  history copies are `cp`, checked with `sha256sum` before and after.

## Quickstart

```bash
# plan of the committed draft (no launch)
scripts/compare-mail.sh collect-v4 --dry-run --output /home/…/v4-dry \
  --protocol docs/evals/draft/protocol.v4.json \
  --sources docs/evals/draft/sources.lock.v5.json --documents docs/evals \
  --bin mailpit=… --bin mailhog=… …

# the suite, as CI runs it
cargo test --locked --test mail_compare v4_

# one real proof, on a host with a delegated user scope (spec V22)
bin=$(cargo test --locked --test mail_compare --no-run --message-format=json \
  | grep -o '"executable":"[^"]*mail_compare-[^"]*"' | tail -n1 | cut -d'"' -f4)
cat /proc/loadavg
timeout 600 taskset -c 0-31 systemd-run --user --scope --quiet -p Delegate=yes -- \
  "$bin" --ignored --exact v4::v4_real_oom_at_two_gib --test-threads=1
cat /proc/loadavg
```

`cargo` must not be the process inside the scope: it would stay in the
scope's own cgroup and the test could not enable controllers.

## Order of work

Seven groups, each committed green (`fmt`, `clippy --all-targets --locked`,
`cargo test --locked --test mail_compare`) before the next:

1. Protocol 4, lock 5, records in the tree (V1–V7).
2. `resources.rs` on fixtures: detection, leaf, parsers, layout (V8, V9, V11,
   V12, parsers of V13).
3. The launcher join, teardown and receipts (V6's resolution, V10, V14, V15).
4. Rows, the cost chunk and the figures (V13, V16, V17).
5. `footprint`, `hold`, the plan and `admit-v4` (V18–V20).
6. `collect-v4` (V21).
7. The real proofs, the README, all gates (V22–V24, AC20).

## Risks

1. **The seat cannot run the real proofs.** A seat's box has no cgroupfs, so
   AC18's passing output cannot come from the implement or verify seat.
   *Mitigation:* each proof fails there with V8's exact refusal, which the
   seat can and must show. The seat reports the five host runs as owed in
   its result notes and does not mark them passed. The dispatcher runs them
   on the host, one at a time under the bounded command, and attaches the
   output before the operator's merge; the PR is held for `scripts/` anyway.
2. **Host behaviour this design asserts but could not execute:** that
   `systemd-run --user --scope -p Delegate=yes` leaves the command as the
   scope's only process with `cpu memory pids` delegated; that a write of
   `0` through an fd the parent opened migrates the child; that
   `cgroup.kill` exists (kernel ≥ 5.14) and `memory.peak` (≥ 5.19).
   *Mitigation:* V8 and V9 refuse by name instead of assuming; the join has
   a stated alternative; real proofs 3 and 4 exercise all of it on the
   first host run. If the scope shape itself is wrong, stop and report.
3. **A v3 regression through shared code.** `launch`, `cleanup`, the
   receipts, the manifest and the samples reader all gain v4 paths.
   *Mitigation:* every addition is an `Option` that is absent for protocols
   1 to 3; AC2 and AC11 hold the old bytes; the whole existing suite stays
   green unchanged except for the constants that now read history copies.
4. **macOS compile or test failure** from Linux-only `libc` items.
   *Mitigation:* `cfg` gates, a non-Linux refusal test, and Linux-only
   attributes on every test that reads `/proc`, `/sys` or `/dev/shm`.
5. **clippy over two compilations.** Each module is built into the bench and
   into the test crate. *Mitigation:* no speculative types for stories B to
   D; mode shims call module functions the tests also call.
6. **A slow target meets the chunk deadline.** *Mitigation:* that is a
   measured `chunk_deadline` outcome and exit 1, not a killed driver; the
   value is the operator's at freeze.
7. **Host load is recorded, not gated.** A neighbour can change a cell.
   *Mitigation:* the load is in every ledger entry and the README states the
   budget is a ceiling, not a reservation; the gate is an operator question.
8. **`memory.current` includes page cache**, so a target can sit at the
   limit without an OOM. *Mitigation:* peak `anon` is published beside it
   and the README labels the difference; 0132 §5.3's primary field stands.
9. **The installed-size walk is not digest-bound** beyond the lock's checked
   members. *Mitigation:* `footprint` rechecks the members, reports the file
   count with the sum, and the README says what is and is not bound.
10. **Draft data drifts from v3.** A hand edit of the draft could change a
    v3 fact. *Mitigation:* a test compares the draft with the v3 bytes
    key by key and allows only the listed differences.
11. **Size.** This is the largest story of 0132. *Mitigation:* the seven
    groups above, each green and committed; a group that cannot be finished
    is reported as unfinished, never folded into "complete".
12. **The wrapper is only exercised with fakes until story E.** A frozen v4
    is refused here, so no real `collect-v4` run exists yet. *Mitigation:*
    AC17's fake-driver tests cover every branch of the loop, and real proof
    5 runs a real chunk through the driver.
