# Plan — mail comparison protocol v4, story A

Spec: [spec.md](spec.md). Authority: accepted
[0132](../../docs/decisions/0132-mail-comparison-protocol-v4.md), story A.
Base `e23a8ba`. No new decision record: nothing here amends a ruling.

Revision 1 (2026-11-16, returned from review 1) is the next section. It
overrides the round-1 text below wherever the two differ; the round-1 text
is kept, marked where it was wrong, because the code at `2852bd2` was built
to it.

## Revision 1 — after review 1 (2026-11-16)

Review 1 read head `2852bd2` and returned the run to design with
`spec_defect: true` (decision 0102, addendum 2026-10-31): four members
`residual`, security `clean`, highest severity medium, no security finding.
The design is revised, not restarted. The first design commit `fcbc0c6` and
the seven implementation commits stay as they are; this revision is a new
commit on top, and the code changes it asks for are group R of
[tasks.md](tasks.md).

Findings are cited by member and number as the panel wrote them: `C` is
correctness (1–10), `G` guarantees (1–6 and its info note `Gi`), `I`
independent (`I0` is its spec defect, then 1–8), `T` tests (`a`, `b`).

### The defect

`I0`, `C4`, `G2`: V16 checked the chunk deadline once before each profile and
V2 allowed 540000 ms. One profile's own worst case is over 600 s (100 warmup
sends, each several protocol steps of up to 10 s; 300 s of send; 60 s of
verify; a listing of up to 16 pages; 3,000 fidelity reads), so the wrapper's
`timeout 600` could kill the driver, map 124 to exit 2 and lose a run that is
never resumed. 0132 §11.3 makes a target's own timeout a measured row with
exit 1. Reading 5 claimed that and V16 could not deliver it.

No static value repairs it, because a single profile can exceed 600 s. The
deadline has to reach the work in flight.

### Synthesis, round 2 — the positions ruled

Both positions argue a revision and agree on most of it. Where they differ:

| # | Point | Simplicity | Robustness | Ruling | What the losing side costs |
|---|---|---|---|---|---|
| R-1 | Shape of the deadline | One instant `chunk_end`; existing deadlines become `min(own, chunk_end)`; no cancellation machinery | A guard in every loop, local deadline `min(own, chunk_end − teardown margin)` | **One instant (simplicity), and in-flight I/O is cancelled at it (robustness's "interruptible").** Clipping a loop's deadline does not stop the await inside it: a send is up to eight steps of 10 s and a listing up to 16 pages (`runner.rs` `max_pages = messages / 250 + 4`), GreenMail's a POP3 session per call | Simplicity's "no cancellation" would leave an overrun of minutes after the deadline, so its V2 inequality (one `request_deadline_ms`) was false as written. Robustness's per-loop margin is replaced by one margin in V2 |
| R-2 | V2's bound | An inequality with startup, one request, cleanup and a 10 s seal margin | "Worst-case profile + cleanup + seal must fit", 540000 stays admissible | **An inequality over exactly what V16 leaves uncut:** startup + 2 × cleanup + a 30 s margin ≤ 600000. The draft moves to 480000 | Robustness's 540000 is above the ceiling (520000) and goes. 40 s less measuring time per chunk than the ceiling allows; the operator may raise it at freeze, inside the bound |
| R-3 | A never-run window | A rule: figures only from `measured` windows; values `null` | The same, plus "real launch" | **Structural: a never-opened window has no counters at all (`null`), and figures need counters.** Cost figures come from any opened `measure` window; idle and startup only from a `measured` one | Simplicity's "only from `measured`" would drop the CPU and peak of an OOM-killed window, which 0132 §5.2 keeps for an unranked cell. Six window fields become optional |
| R-4 | Count behind a median | not raised | "median of two, not zero" | **`n` beside every median, minimum and maximum** | One field; without it a median of two reads like a median of three |
| R-5 | AC18 | Owed to the dispatcher host; no spec change | The merge gate; delete the "owed" sentence; a seat without cgroupfs escalates | **Robustness, in two parts.** AC18a is what a seat can prove; AC18b is the host run and gates the merge. The "owed" sentence is replaced | The story cannot be reported complete by any seat in a box without cgroupfs. That is the true state, and the PR is held for `scripts/` anyway |
| R-6 | A target dead before readiness | Classify from the leaf's evidence; alive-not-ready stays exit 2 | The same boundary: spawned and joined, then exited or OOMed, is measured | **Both, with the evidence in receipts** (`oom_kills`, `exited_early`, and the launch receipt kept for a not-ready launch) | none between the positions. A harness mistake that makes a target exit at once now seals as `target_exited` instead of stopping the run; the row has no number and names the cause |
| R-7 | `reproduce.txt` | Amend V21 to the code: write it before the first chunk | Keep V21: only after the last chunk | **Robustness.** The criterion is sound and the code missed it; a criterion is not moved to the code | An incomplete directory does not carry the command; each ledger entry already carries its chunk's full command |
| R-8 | Signal to the wrapper | A trap that stops the unit and exits 2; no ledger entry | A trap that writes the ledger entry with exit 2 | **Robustness**, and the signal goes to the chunk's `timeout` so the driver runs its own interrupt path | A few more lines of bash 3.2 and a background `wait` |
| R-9 | `footprint` for archive rows | `null` with an `owed` basis | Walk the extracted tree, or owed | **Simplicity**, with the executable's size kept under its own name | The installed size of Mailpit, smtp4dev and Inbucket is owed until story E fixes an extraction path. A walk of an unpinned directory would publish a confident wrong number |
| R-10 | `taskset -c 0-<n-1>` | Accept; fails closed | Derive the mask from the allowed set | **Neither: refuse a restricted shell by name**, and bound every process the mode starts | `collect-v4` cannot run inside a narrowed CPU set. Deriving a mask in bash 3.2 is more code than the refusal, and the collection host is unrestricted (0132 acceptance 7) |
| R-11 | `hold` on a signal | Accept the test gap; real proof 4 covers EOF | A signal test | **A test through `hold`'s own `interrupt` argument on a launch without a leaf, and a second leg in proof 4.** No signal seam | The process-level SIGTERM of the `hold` shim stays untested (out of scope, named) |
| R-12 | GreenMail in the v4 draft | An `unresolved` entry and the claim's wording | Block or owe the GreenMail row in the report | **Simplicity.** The draft cannot be collected (`check` refuses a draft) and V5 refuses to freeze with an unresolved entry | none: no GreenMail figure can exist on the POP3 path |
| R-13 | A real driver-kill proof, `pid_limit`, a load gate, hostile bodies, certificates | not raised | raised, mostly as "not asked for" | **Unchanged from round 1**: fakes for the driver's death, no `pid_limit` label, load recorded, the rest out of scope | as round 1 |

### Every finding, and where it is answered

| Finding | Severity | Answer | Spec | Task |
|---|---|---|---|---|
| `I0` `C4` `G2` chunk deadline cannot stop a running profile | defect | One instant; cancel in flight; V2 bounds what is left | Reading 5, V2, V16, AC2, AC21 | R1.1–R1.4 |
| `I0` (last line), `C6` the loop's failure paths and deadline branch have no test | low | Tests that drive `run_chunk` itself | AC21, AC22 | R1.3, R1.4, R2.3 |
| `C1`, `I` (AC18) no real proof has passed on a kernel | medium | AC18 split; the host run gates the merge | V22, AC18a, AC18b | R9.1–R9.3 |
| `C2` `G3` a skipped profile publishes zeros | medium | Never-opened windows have no counters; figures need them; idle and startup cells get `ranked` and `shortfalls` | Reading 8, V13, V17, AC10, AC23 | R3.1–R3.4 |
| `C3` `I4` `G3` `affinity_held` is a literal on two windows and on skipped ones | low | Read at every opened window's end; `null` when no task | V13, AC10 | R3.2 |
| `G1` `C8` an OOM or crash before readiness loses the run | medium | Classified from receipts; measured, exit 1 | Reading 7, V15, V16, AC11, AC22 | R2.1–R2.3 |
| `G4` `C10` `installed_bytes` is the executable alone for an archive row | medium | `null`, an `owed` basis, `executable_bytes` | V18, AC14 | R5.1 |
| `I1` `C5` `is-active` read at once after the driver exits | medium | Bounded wait for inactive before judging | V21, AC17 | R6.1 |
| `I2` a `null` Swarmail cap launches uncapped | low | Refused in the validator and when the spec is built | V4, AC4 | R4.1 |
| `I3` a second literal at the cap flag validates | low | The cap flag occurs once | V6, AC4 | R4.2 |
| `G5` later chunks' `candidate_dirty` unchecked | low | Every chunk | V20, AC16 | R4.3 |
| `I7` `admitted[0]` panics on an empty plan; candidate digest not compared across chunks | low | Refusal by name; one digest per target across chunks | V20, AC16 | R4.3 |
| `C7` `hold` prints `"pop3":"pop3"`; its signal path is untested | low | The address; a test through the interrupt argument | V19, AC15 | R5.2 |
| `I5` `reproduce.txt` written early; no trap | low | Written after the last chunk; INT, TERM and HUP handled | V21, AC17 | R6.2, R6.3 |
| `I6` `taskset -c 0-<n-1>` assumes the allowed set; builds, footprint and admission outside the bound | low | Restricted shell refused; every process under the bound | V21, AC17 | R6.4 |
| `G6` the v4 draft's claim asserts the POP3 read path | low | `unresolved` entry; "in this draft" | V7, AC6 | R7.1 |
| `I8` `wait_operation_ms` changed without a note; AC17 lacks exits 137 and 3, a timeout with a live scope, seal-then-refused-verify; some refusal tests assert only `is_err` | info | V7 says why; AC17 gains the cases; the tests assert the named field | V7, AC17 | R7.1, R6.5, R4.4 |
| `Gi` "can never be admitted as v4" is not what `admit-v4` does | info | Reworded; the sealed manifest carries `protocol_status` | Reading 2, V20, V23 | R4.3, R8.1 |
| `C9` "SMT siblings left idle" stated unconditionally | info | Conditional sentence | V23 | R8.1 |
| `C8` (operator note) alive-but-not-ready is exit 2 | info | Named for the operator | Reading 7, *For the operator at freeze* 4 | R8.1 |
| `T b` the rig's git fixture inherits the seat's git environment | info | The v4 rig owns its git environment | — | R6.6 |
| `T a` the mode shims in `benches/mail_compare.rs` have no CLI test | info | Not designed for; see below | *Out of scope* | — |
| security | clean | no finding | — | — |

### Criteria whose text changed, and why that is not a loosening

- **V2 / AC2, the 540000 ceiling.** Wrong, not strict: review showed
  (`I0`'s arithmetic, confirmed in `runner.rs`) that no constant ceiling
  bounds a chunk while in-flight work is uncut. The replacement is tighter
  for the draft's budgets (520000) and is a property the tests can check.
- **V16's deadline sentence.** Wrong as `I0`, `C4` and `G2` show. Replaced
  by a stronger contract; AC21 is new.
- **V16's "exits 2 on a launch refusal", for a target that died.** Corrected
  toward accepted 0132 §11.3 and §4.2 (`G1`): the authority says an OOM or a
  crash is a measured row and exit 1. The rest of the sentence stands.
- **V13's `affinity_held` and V17's figures.** The first text did not say
  what a window without an observation holds; the code filled in `true` and
  `0`. The rule now stated is `stats.rs`'s own ("a figure that has no value
  is absent from the list, never zero") and 0132 §5.2. AC13 is unchanged;
  AC23 is added.
- **V18 / AC14, installed bytes.** The first text counted "the executable,
  its checked members and the whole pinned runtime tree" for every row.
  For an archive row that is an undercount against 0132 §4.5 (`G4`:
  smtp4dev's runtime is inside its archive). The figure is withdrawn for
  those rows, not redefined: `null` and `owed`.
- **V22 / AC18.** Not loosened: the host run was a merge condition and is
  now also a completion condition, stated in two parts so that a seat's
  result cannot blur them.
- **V21's `reproduce.txt`.** Unchanged; the code moves.

### How the changes are built

**The deadline.** `run_chunk` computes `chunk_end` once from the run clock's
origin (`Clock.0`, which `execute` takes before the run) and passes it to
`warmup`, `measured_instance` and `throughput`. A test places `chunk_end`
exactly by passing a clock whose origin is older, with no sleep.

- A small helper races a future against `chunk_end`
  (`tokio::time::timeout_at`) and reports "cut". It wraps: each warmup
  `open`/`send`, the idle wait, each measured `Smtp::open` and `send`, each
  `list_all`, each `fetch`. Dropping the future is the cancellation; an SMTP
  session cut this way is dropped, never reused.
- The workers' send deadline becomes `min(send_deadline, chunk_end)`, and
  the row's reason says which one ended it.
- Launches and `shutdown` are not wrapped.
- Protocols 1 to 3 pass no `chunk_end`; their paths are byte-for-byte what
  they were. The existing `probe.is_some()` distinction is the model.

**The not-ready launch.** `LaunchError::NotReady` gains the launch facts of a
leaf launch (optional; absent without a leaf). `cleanup` records
`exited_early` from what it already knows (`reaped_early`) and the leaf's
teardown reads `memory.events` beside `memory.peak`, before the `rmdir`. One
pure function maps the cleanup receipt to `Option<WindowOutcome>`; `warmup`
and `measured_instance` call it, push both receipts, and return "not
measured" instead of an error. `skip_profile` takes the outcome, the reason
and the launch name.

**Rows and figures.** `WindowRow`'s counters, `affinity_held` and
`scratch_bytes` become optional together; `window_row` has one constructor
for an opened window and one for a never-opened one, so a row with some
counters cannot be built. `Probe::mark` reads affinity at both kinds of end
bound through one function, which returns `None` for an empty task list.
`figures` returns optional values per repetition; `spread` returns `n`; the
idle and startup cells take `ranked` and `shortfalls` from the function the
cost cells use. `report.rs`'s `mean` and the `unwrap_or(0)` on peaks lose
their zero defaults.

**The wrapper.** One array holds the bound prefix and every started process
uses it. The chunk runs in the background and is `wait`ed, so a trap can
signal it; the wait is repeated until the child is reaped. The scope check
is a bounded loop. `reproduce.txt` moves below the loop. bash 3.2 syntax
only, as the file states.

### Files this revision touches

| File | Change |
|---|---|
| `benches/mail_compare/protocol.rs` | V2's two constants and the inequality; Swarmail's cap; the cap flag once |
| `benches/mail_compare/runner.rs` | `chunk_end` and the cut helper; `skip_profile` generalized; the not-ready classification; affinity at every bound; `hold`'s endpoints; no `--max-per-inbox 0` for protocol 4 |
| `benches/mail_compare/targets.rs` | `CleanupFacts.oom_kills`, `exited_early`; `NotReady` keeps the launch facts |
| `benches/mail_compare/resources.rs` | the teardown reads `oom_kill`; `footprint`'s archive rows |
| `benches/mail_compare/records.rs` | optional window counters and the reader's three refusals |
| `benches/mail_compare/stats.rs`, `report.rs` | `n`; optional per-repetition figures; idle and startup `ranked` and `shortfalls`; the `timeout` and `lost` rules; exactness needs fidelity read; `admit_v4`'s four checks and `protocol_status` |
| `scripts/compare-mail.sh` | V21's four changes (approved with 0132; the PR stays held) |
| `tests/mail_compare/v4.rs`, `tests/mail_compare/v4-smoke.json` | the tests of group R; the fixture's `chunk_deadline_ms` under the new bound |
| `docs/evals/draft/protocol.v4.json` | `chunk_deadline_ms` 480000; two `unresolved` entries; "in this draft" |
| `docs/evals/README.md` | V23's exact sentences |

Still untouched: `src/`, `evals/`, `Cargo.toml`, both lockfiles, `deny.toml`,
the active and historical records, `docs/decisions/`. No package is added.
The draft files were added by this branch, so AC19's `git diff e23a8ba`
statement still holds.

### Not designed for, with the cost

- **A CLI test of the mode shims** (`T a`). The bench is `harness = false`
  and has no `CARGO_BIN_EXE`; a test would have to build it through `cargo`
  from inside a test. Every function the shims call is tested. Cost: a typo
  in a shim's flag plumbing is found by the first host run, not by CI.
- **A real driver-kill proof and a real `collect-v4` chunk.** A draft cannot
  pass `check`, so no real wrapper run exists before story E. Cost: named to
  the operator (*For the operator at freeze* 6).
- **Alive but not ready.** Stays exit 2 (Reading 7). Cost: named to the
  operator (4).


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

## Synthesis, round 1 — the panel's positions, ruled

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
  slow target into a lost run. *(Round 1 built this as a check before each
  profile, which does not bound a running one: the defect of review 1. See
  Revision 1.)*
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
               + oom_kills, exited_early                      (revision 1)
Manifest       + chunk (coordinate), host { kernel, systemd, cpu_model,
                 nproc, siblings, cpu_layout, governor }
ResourceRow    kind = sample | window   (deny_unknown_fields; see V13)
WindowRow      cpu_start, cpu_end, events_start, events_end, affinity_held:
                 all present (opened) or all null (never opened); the
                 affinity alone may be null on an opened window whose leaf
                 holds no task                                (revision 1)
Spread         + n                                            (revision 1)
CostRep, IdleRep, StartupRep   figures optional               (revision 1)
IdleCell, StartupCell          + ranked, shortfalls           (revision 1)
Run manifest   + protocol_status                              (revision 1)
Footprint row  + executable_bytes; installed_bytes nullable   (revision 1)
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

1. **The seat cannot run the real proofs.** A seat's box has no cgroupfs
   (the design seat's has none: `/sys/fs/cgroup` is absent, read again
   2026-11-16), so AC18b's passing output cannot come from the implement or
   verify seat. *Mitigation (revision 1):* AC18 is two criteria. A seat
   proves AC18a and shows V8's exact refusal for all five. AC18b is the
   dispatcher's host run at the head to be merged, one proof at a time
   under the bounded command with the load before and after; it gates the
   merge and the story's completion, and task R9.3 is the dispatcher's. No
   seat reports the story complete before it. The PR is held for `scripts/`
   anyway. A review that finds AC18b open finds what this plan says is open.
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
6. **A slow target meets the chunk deadline.** *Mitigation (revision 1):*
   the deadline cancels the work in flight, so this is a measured
   `chunk_deadline` outcome and exit 1; V2 bounds the value by what may
   still run afterwards, and AC21 drives each cut. The wrapper's `timeout`
   remains as a backstop whose 124 is exit 2. If the model behind V2's bound
   is wrong on the host (a teardown or a seal slower than its budget), the
   result is that backstop, an incomplete run, never a wrong figure.
7. **Host load is recorded, not gated.** A neighbour can change a cell.
   *Mitigation:* the load is in every ledger entry and the README states the
   budget is a ceiling, not a reservation; the gate is an operator question.
8. **`memory.current` includes page cache**, so a target can sit at the
   limit without an OOM. *Mitigation:* peak `anon` is published beside it
   and the README labels the difference; 0132 §5.3's primary field stands.
9. **The installed-size walk is not digest-bound** beyond the lock's checked
   members. *Mitigation:* `footprint` rechecks the members, reports the file
   count with the sum, and the README says what is and is not bound.
   *(Revision 1:* an archive row's extracted tree is not walked at all; its
   installed size is `null` and owed, V18.)
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
13. **Cancelling a send in flight leaves the server's state unknown**
    (revision 1). The message may have been stored. *Mitigation:* the row is
    `ambiguous`, never `accepted`; nothing is listed after `chunk_end`, so
    it cannot appear as `unexpected`; the instance is discarded with its
    profile; the cell is unranked.
14. **A harness fault now seals as `target_exited`** (revision 1): a target
    that exits at once because of a wrong flag is the target's failure by
    the evidence. *Mitigation:* the row has no number and names the outcome;
    both receipts are kept; `check` and the lock's launch checks run first;
    the operator holds each target once before a long run (*For the operator
    at freeze* 4).
15. **How long a scope stays active after its driver exits is not known for
    systemd 259** (revision 1; `C5` and `I1` could not verify it either).
    *Mitigation:* a 10 s bounded wait, far above an asynchronous
    notification; a scope that outlasts it is treated as unclean, which is
    exit 2 and a stopped run, the safe side. The first real chunk shows the
    real figure (*For the operator at freeze* 6).
16. **Optional window counters touch the reader, the report and admission at
    once** (revision 1). *Mitigation:* one constructor per row shape, three
    reader refusals (AC10), and AC23's check that no report built from
    never-opened windows contains a zero where a figure is meant.
17. **A release build inside `timeout 600` on at most 32 CPUs may not
    finish** (revision 1). *Mitigation:* the refusal is named
    (`blocked build`), nothing has been measured at that point, and
    `COMPARE_MAIL_SWARMAIL_BIN` and `COMPARE_MAIL_DRIVER_BIN` take prebuilt
    binaries.
18. **The in-flight cut tests could become races** (revision 1).
    *Mitigation:* each places `chunk_end` by construction, not by timing: a
    clock whose origin is already older than the deadline, or a listener
    that never answers (a pole, 0020 §5), with the assertion that the call
    returned before one request deadline. No test waits for a deadline to
    arrive between two steps of a healthy target.
