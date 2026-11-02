# Plan: eval flake determinism (issue #302)

Spec: [spec.md](spec.md). Base: main `c336c95`. Line numbers below are for
`c336c95`.

## Technical context

- **Crate.** All of the work is in the standalone `evals/` workspace (decision
  0055). It sits outside the root coverage denominator and the root
  `cargo deny`.
- **Toolchain and runtime.** Rust, edition and toolchain as pinned. The tests
  use tokio's multi-thread runtime. `libc` is already a dependency and is
  already used in `evals/tests/common/mod.rs`.
- **Code kind.** Every changed line is either test-only code
  (`cfg(all(test, target_os = "linux"))`) or test-crate code. The release
  build of both crates is unaffected.
- **Trials.** Trials serialise per process through `trial_scope` (0076), and
  this plan leaves that unchanged.
- **No mocks.** Real `swarmail` binaries and real fixtures stay in the loop
  (0004).

## Constitution check

| Rule | Status |
|---|---|
| Frozen guarantees 0001/0002/0003 | Untouched. Case 3 was investigated as a possible 0001/0005 defect and ruled out (spec § Why case 3 is not…). |
| 0005, extraction at ingest | Untouched. No `src/` change. |
| 0020 §5: poles only, no sleep-as-sync, no wall-clock validity | Case 1 makes the injected fault a 100 % pole for every group that stays live after TERM. No sleep, retry or timeout is added. Case 3's forced seed is derived deterministically, not from wall-clock time. |
| 0020 placement | No cold path is needed, so nothing moves to a `_tests.rs` harness. |
| Coverage floor 100 % exact | Root `src/` is unchanged, so the denominator does not move. `evals/` is outside it (0055). |
| Dependencies | None added. Both lockfiles and `deny.toml` are unchanged. |
| 0056 merge hold | Not triggered: no `.github/`, `scripts/`, `.brokkr/`, toolchain, lockfile, `deny.toml` or decision-status change. |
| Decision document | None. This is not a semantic change. |
| Proposed records 0094, 0111 (0045 §8) | The case-3 control reads `candidates[].anchor_text` (proposed 0094) and its classification uses `near_misses`/`missed` (proposed 0111) as observed current behaviour. That is compatible implementation under accepted 0045 §8: no accepted ruling is contradicted, and nothing depends on or implies their acceptance. If either is amended or rejected, re-derive the case-3 control. |
| 0076 sequential trials | Unchanged, and it shapes the load proof: evals copies serialise on the per-uid admission `flock`, so load comes from CPU hogs and root real-protocol test binaries (spec AC4). |

## Project structure (files touched)

```
evals/src/runner.rs          # cfg(test) fault seam + test_probes assertion message
evals/tests/common/mod.rs    # helper arm unsafe_output_then_server_death
evals/tests/fixture.rs       # Rig::with_inputs; digest-forty control + forced-collision rig
```

## Research: evidence behind the rulings

### Case 1

- **The survivor.** The survivor is `trap "" TERM; : > child-ready; exec sleep
  1000` (`runner.rs:4037`). `agent-pids` is written only after `child-ready`
  exists, and `launch_probe` waits for it (`:4181`) and records exactly two
  identities (`:4196-4202`).
- **The only killer.** Before `:4619`, the only thing that can kill the
  survivor is the post-TERM SIGKILL escalation (`:1236-1240`).
- **The fault slot.** The fault is the static `CLEANUP_OBSERVATION_TARGET`
  (`:77`). `cleanup_once` stores its own pid in it after TERM (`:1213`), and
  `group_members` consumes it with `compare_exchange(group.pid, 0)` (`:936`).
- **The race.** `cleanup_pids` runs every owned group's cleanup in parallel.
  The probe trial owns the agent group and Swarmail `serve`, so the last
  store wins. The agent group's CAS then misses, so it escalates and kills
  the survivor. The `serve` group fails instead, which still satisfies
  `failed_owners_only` (`:208-232`). `forced_expiry` returns directly
  (`:1217-1222`), so it cannot lose its fault.
- **No other user.** Only this test sets `observation_error` (`grep -n
  '"observation_error"'` finds only `:4097`, `:4132` and `:4590`). Nothing
  needs exactly one failed owner, so the per-group contingency the
  robustness position named is not needed.

### Case 2

- **The failing line.** a4f552d `tests/runner.rs:1620` is the
  `agent_exit_status == Some(0)` assertion. Its message is `stderr(&out)`,
  which is why the report quoted `service_exited_during_trial`.
- **The helper.** The helper (`common/mod.rs:715-772`) SIGKILLs `serve` at
  `:759` and then returns 0. It installs no signal disposition.

### Case 3

- **The seed.** `Rig::new` passes `inputs: Default::default()`, a random seed
  (`provenance.rs:61-65`).
- **The displaced title.** `right = select(task, slot, "digest", 0,
  "target-position", 40)` (`evals/src/fixture.rs:600-603`). Index `right`
  carries `DIGEST_ITEM` instead of `digest_decoy_title(right)`.
- **The control.** `digest_decoy_title(1)` = "Async drop RFC enters final
  comment period (part 1)", and `digest_decoy_title(14)` is part 2 of the same
  topic (`:811-832`).
- **The candidate list.** `link_result` emits `candidates[].anchor_text` on
  `ambiguous` (`src/mcp/focused.rs:280`), the field described by proposed
  0094 and used here as current behaviour under accepted 0045 §8. `LIST_CAP` (250) exceeds the digest's
  occurrence count, so the list is complete.

## Data model, contracts

No data model or public contract changes. The MCP tool contract, the result
schema and the error categories stay byte-identical.

## Implementation

### Case 1: delete the slot, fail at the group (answers the lost-fault cause)

1. Delete `static CLEANUP_OBSERVATION_TARGET` (`:76-78`) and the CAS block at
   the top of `group_members` (`:935-949`).
2. In `cleanup_once`, replace the store block (`:1209-1215`) with a twin of
   the `forced_expiry` block:

   ```rust
   #[cfg(all(test, target_os = "linux"))]
   if std::env::var_os(CLEANUP_OBSERVE_ERROR_ENV).is_some() {
       let members = group_members(&self)?;
       if !members.is_empty() {
           return Err(format!(
               "cannot inspect owned process group {}: injected observation error",
               self.pgid
           ));
       }
   }
   ```

   The string stays byte-identical (it uses `pgid`, as before).
3. **Unchanged.** The retention loop in `ensure_cleanup` (`:1096-1103`) calls
   `group_members` directly, as before, so it still observes real cessation.
   The test body is also unchanged.
4. In the test, extend only the `any(alive)` message with the owned
   identities and each one's `alive` state.

### Case 2: the witness owns its exit (answers the reaping-order cause)

At the top of the `#[cfg(target_os = "linux")]` block in
`unsafe_output_then_server_death`, before the `/proc` walk, add:

```rust
// SAFETY: single-threaded test agent; ignoring TERM lets it report its own
// exit status (0 = found and killed serve, 64 = did not) in either reaping
// order. `signal` is async-signal-safe and has no preconditions here.
unsafe { libc::signal(libc::SIGTERM, libc::SIG_IGN) };
```

The helper's stdio is already flushed (`:723-724`), so `_exit` is not needed.
The runner's `TERM_GRACE` (5 s) stays the outer bound.

### Case 3: control from observed mail, plus a forced collision (answers the data collision)

1. Add `Rig::with_inputs(task, count, inputs: FixtureInputs)`. `Rig::new`
   delegates to it with `Default::default()`, so its 21 callers do not change.
2. In the loop, for `digest-forty` with `negative == true`:
   - read the `anchor_text` set from `ambiguous["candidates"]`;
   - choose the part-1 title if it is present, otherwise part 2;
   - `assert!` that the chosen title is present among the pre-call
     candidates, with a message naming the task, `negative` and the chosen
     title (no constant-only conjunct; the existing `WrongLink` assertion is
     the runtime proof that the control is not the target);
   - pass the matching escaped regex (`^… \(part N\)$`) as `anchor_text`.

   The other tasks keep their literal decoys.
3. Add one forced-collision iteration: a digest-forty negative rig built with
   `Rig::with_inputs`. Its seed is found by a bounded deterministic search:
   `format!("{n:064x}")` for `n` in `0..10_000`, taking the first `n` where
   `inputs.select("digest-forty", 0, "digest", 0, "target-position", 40) ==
   1`. The search panics if it finds none.
   - Slot 0 is the single provisioned account. The red-before run confirms the
     derivation actually lands `right == 1`: the old control must go red
     there.
   - That rig runs the same body (both calls `timeout_ms: 0`, `selected`,
     `WrongLink`), so the part-2 branch runs on every invocation.
4. Leave the positive rigs, the stale-otp half and the other three tasks
   unchanged.

## Position reconciliation

The two panel positions agree on all three causes and fixes. This design
adopts both, with these rulings:

| Point | Ruling | What the losing side costs |
|---|---|---|
| Intake AC1's release file (the issue's wording) | **Rejected** (both positions): it cannot stop SIGKILL, and an early panic would leave a blocked child behind. | The issue's literal "Done when" is not met. The PR explains why. |
| Robustness: probe the siblings for an exactly-one-failed-owner dependency | **Resolved by evidence**: `observation_error` has exactly one user. The per-group contingency is dropped. | None. |
| Robustness: "prove which assertion fires before patching" (case 2) | **Kept as a red-before guard**, but the failing line is already identified from a4f552d `:1620`. The return-to-design condition stays. | One uncommitted perturbation run. |
| Case 3 control: two literals (both positions) vs "any observed decoy" | **Two literals.** `evals/` has no regex crate, and adding one would take an operator ruling and a 0056 hold. | The test encodes the decoy layout (indices 1 and 14). The presence assertion makes any drift fail loudly. |
| Case 3: only random seeds (both positions) vs adding a forced `right == 1` rig | **Chief adds the forced rig.** Without it, the fallback branch the fix introduces would run in only 1 of 40 runs. That would leave the fix's own code mostly unexercised, and proving it would need hundreds of runs. The random rigs stay, so the seed is not frozen (robustness's objection was to *replacing* the random draw). | One small constructor (`Rig::with_inputs`) and a bounded seed search. Simplicity's "no new helper" gives way here. |
| Load proof sizes (intake ≥50 runs/3 copies; simplicity ≥100/≥200; robustness ≥100/≥400) | **≥100 for cases 1 and 2 and ≥200 for case 3** (load per spec AC4; the original "under ≥4 copies" was superseded on 2026-11-02 because 0076 serialises evals copies). The forced rig makes the collision deterministic, so 400 random draws are no longer needed to show it. | Robustness loses its 400-run sample of the random draw. That is acceptable because the forced rig covers the branch on every run. |
| Pre-#291 `NotFound` sighting at the old `:4039` | **Out of scope.** It does not reproduce on this tree. New evidence opens a new issue. | If it is a second cause, it stays latent until it is seen again. |

## Quickstart (proof; PR text only, nothing committed)

```bash
cargo build --locked --bin swarmail
BIN=$(cargo build --locked --bin swarmail --message-format=json \
      | jq -r 'select(.executable!=null and .target.name=="swarmail").executable' | tail -1)
cd evals && CARGO_TARGET_DIR=$PWD/../target/evals-proof SWARMAIL_BIN=$BIN \
  cargo test --locked --no-run --message-format=json > ../target/evals-proof/bins.json
# load (not serialised by 0076's admission flock): nproc CPU hogs, e.g.
#   for i in $(seq "$(nproc)"); do (while :; do :; done) & done
# plus >=4 loops of the prebuilt root real-protocol test binaries
#   (cargo test --locked --no-run --test burst --test p2 --test rate, run directly)
# sample `cat /proc/loadavg` during the measurement; 1-min value must be >= nproc
# measure: loop each test binary with `--exact <name>` N times, count passes,
#   then run the whole evals suite once under the same load; kill the load after
```

## Risks

1. **The case-1 interleaving is inferred from the code, and the red-before is
   probabilistic** (about 1 in 4 on `c3b4ab7`'s run: the perturbation delays
   whichever group the scheduler stores first).
   *Mitigation:* the fix removes the shared slot, so it holds by construction;
   the red-before showed the predicted `any(alive)` failure at a measured rate,
   recorded in the PR. The shipped test detects a reintroduced shared slot.
   No speculative hold is added.
2. **The case-1 fault now fails every group still live after TERM**, so
   several failed owners are possible.
   *Mitigation:* `forced_expiry` already has this shape against the same test
   body. `failed_owners_only` waits on every group and does not count them.
   `safe_cleanup_category` matches by prefix, so the category stays the same.
3. **Case 2: a helper starved for longer than `TERM_GRACE` is still
   SIGKILLed.**
   *Mitigation:* that would be a real escalation on a host that cannot
   schedule a runnable process for 5 s, so a red result there is correct. No
   test relies on the duration.
4. **Case 2: the perturbation fires the outcome assertion instead.**
   *Mitigation:* stop. Do not ship the helper patch as cover. Return to design
   as a runner ordering bug.
5. **Case 3: generator drift** (the topic list or the 40-item layout changes).
   *Mitigation:* the presence assertion fails loudly with a named message.
   The forced-seed search panics if it finds no `right == 1` seed within its
   bound.
6. **Case 3: the forced-seed derivation does not match the generator's
   `select` call** (task id, slot or flow).
   *Mitigation:* the red-before run on the old control must reproduce
   `timed_out` with 0 candidates on that rig. If it does not, the derivation
   is wrong and must be fixed before shipping.
7. **The load proof could be serialised** by cargo's lock or by 0076's per-uid
   admission `flock` (copies of the evals suite queue on it), or fill `/tmp`,
   a RAM tmpfs with a quota.
   *Mitigation:* prebuild once, run the binaries directly, keep every target
   dir on `/home`, and take load from CPU hogs and root real-protocol test
   binaries that the admission lock does not touch; record the load average
   to show the host was saturated.
8. **Hidden product behaviour change.**
   *Mitigation:* the AC5 diff audit: no root `src/` file changes, and every
   `evals/src` hunk is under `cfg(all(test, target_os = "linux"))` or inside
   `test_probes`.
9. **The revision's one-line code edit voids load counts measured on
   `c3b4ab7`.**
   *Mitigation:* case 3 (200/200) and the full suite are re-run on the
   shipped head. Case 1 and case 2 counts may be reused only with the
   AC4 diff proof that their code is byte-identical; the PR names the head of
   every count.
10. **Proposed 0094 or 0111 is amended or rejected later.**
    *Mitigation:* the spec says so and names what to re-derive; the presence
    assertion fails loudly if `anchor_text` disappears from `ambiguous`
    answers (the `candidates` read panics with a named message).
11. **The PR omits a required disclosure** (the AC4 deviation, the
    probabilistic red-before, the release-file rejection).
    *Mitigation:* AC9 names each item, so review checks the PR text against a
    list instead of against memory.
## Revision 2026-11-02: answers to the review return on `c3b4ab7`

Review (decision 0102, spec-defect addendum) confirmed the three causes and
fixes and returned the spec. Each finding, the change that answers it and the
artifact that owns it:

| Finding (member, severity) | Change | Owner |
|---|---|---|
| 0111 listed as accepted; 0094 and 0045 §8 not cited (guarantees, spec defect) | § Authorities split into accepted and proposed; 0045 §8 basis and re-derivation clause added; 0111 marked proposed in the case-3 evidence; constitution-check row added here | spec § Authorities, this plan, tasks R1 |
| AC4's "4 copies of the evals suite" cannot load the host under 0076 (correctness, independent, tests; low) | AC4 restated toward accepted 0076 with load it permits, counts unchanged, load average recorded, counts required on the shipped head | spec AC4, Quickstart, Risk 7/9, tasks R4 |
| `title(part) != DIGEST_ITEM` compares constants (correctness, info) | Conjunct deleted; `WrongLink` named as the runtime proof | spec AC3, Case 3 step 2, tasks R2 |
| Digest-forty tuple decoy literal is dead for that task (independent, info) | Kept, with a comment that `digest_forty_control` replaces it in every digest-forty negative rig | tasks R2 |
| Case 1 red-before was probabilistic (correctness, independent; info) | AC1 and Risk 1 state it; AC9 requires the measured rate in the PR | spec AC1, AC9 |
| Case 2 `> TERM_GRACE` residual only in notes (correctness, info) | Lifted into AC2 as an accepted residual (Risk 3 unchanged) | spec AC2 |
| PR must state the AC4 deviation and the release-file rejection (correctness, independent) | AC9 lists every required PR item | spec AC9, tasks R5 |

No criterion is loosened. AC4's load clause moves toward accepted 0076 (the
return rule for a criterion that conflicts with an accepted decision); its
counts stay, and it now also requires them on the shipped head. AC3 loses a
conjunct that could not go red, which proved nothing.

### Position reconciliation for the revision

| Point | Ruling | What the losing side costs |
|---|---|---|
| Authorities: both positions agree (split lists, cite 0045 §8) | **Adopted**, with robustness's re-derivation clause. | Nothing. |
| AC4 load: robustness "≥32 hogs, load ≥30" vs simplicity "≥`nproc` hogs + ≥4 root loops" | **Simplicity's form, robustness's saturation check made host-relative:** `nproc` hogs, ≥4 root real-protocol loops, 1-min load average ≥ `nproc`. A fixed 32/30 bakes one host's size into the spec. | Robustness loses fixed numbers; the load-average floor keeps its intent (a saturated host) on any host. |
| Vacuous `!= DIGEST_ITEM` conjunct: simplicity "text-only, reword AC3, no code" vs robustness "runtime comparison, or delete outright" | **Delete it** (robustness's second option). A guard that cannot fire must not stay in a test whose purpose is honest determinism; the `WrongLink` assertion already proves at runtime that the control is not the target, so no new comparison is added. | Simplicity loses the text-only revision: implement reopens for a one-line test edit, and the case-3 load count (200/200) and the full-suite run must be re-measured on the new head, plus every gate. Robustness loses the extra runtime comparison, which would only duplicate `WrongLink`. |
| Dead tuple literal (independent, info) | **Keep with a comment**; replacing it with an `Option` or sentinel adds vocabulary for no proof. | A literal that is never read for that task remains, now labelled. |
| AC1 red-before wording, AC2 residual (robustness) | **Adopted**, text only. | Nothing. |
| PR-text checklist (simplicity) | **Adopted** as AC9. | Nothing. |
| No load-proof script, gate or decision document (simplicity) | **Adopted.** The issue keeps the proof in the PR; nothing here changes a ruling. | None. |
