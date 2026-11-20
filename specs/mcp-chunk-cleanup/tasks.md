# Implementation and proof order

- [x] Establish the external MCP runner and exec handshake in the collector and reuse it from the host harness. Prove distinct fresh scope identities, pre-exec client membership and adapter inheritance, with only hold delegated. Retain api_section::only_the_holder_enters_the_delegated_scope. Exercise direct Swarmail and refusal before spawn on non-Linux.
- [x] Implement recursive client judgment and bounded diagnostics on every attempted launch. Test populated nested scope, empty, verified removal, missing handshake, mismatched identity, inaccessible/malformed observation and PID disappearance. Inject stalled observation and diagnostics against one controlled clock and assert the five-second cap and remaining 600-second budget, including holder checks and containment.
- [x] Split checked MCP index preparation from atomic no-clobber publication. With valid holder evidence, pause after preparation and prove public admission still fails. Exercise successful exits 0 and 1, invalid receipt, dirty holder, timeout, startup/control failure, interruption before commit, ledger write failure and duplicate destination. Verify failed attempts leave no new public seal and unrelated finalizers still admit.
- [x] Bind the client verdict to the v4 ledger, failure output and admission. Test a clean record and independently mutate or remove its identity, coordinate and verdict. Verify refusal and the original holder proof independently. Exercise native admission without a client scope.
- [x] Run an event-ordered fake adapter through the collector fixture. Wait for its TERM-ignoring child (backgrounded with its stdio detached, not `setsid`: inheritance across `setsid` is the kernel's and is observed only on the host) to acknowledge readiness and keep it alive through observation. Assert exit 2, named PID/process and coordinate in the persisted report, no admissible index, no next launch or retry, and bounded teardown. Repeat with the child reaped before judgment and assert clean evidence and successful publication. CI must run the pair without host delegation or protocol-length sleeps.
- [ ] Extend all three exact bridged tests and direct candidate MCP proof listed in plan.md. Assert actual client placement and clean receipt, distinct delegated holder and admitted chunk, before the MailSlurper shortfall branch. Keep existing measurement and receipt checks. List ignored tests and run each with a required prerequisite unset to prove discovery and named refusal locally. Record delegated execution as open.
- [x] Replace the README’s process-group disclosure and publish per-test environment, digest checks, exact commands and load capture. Append R30 closing R29 independent 4 with proof names and dispatcher status. Cross-check commands against test discovery and refusal diagnostics. Dispatcher runs each alone under timeout 600 taskset -c 0-31 and attaches output before merge.
- [x] Record the sibling-scope rationale and actual elapsed time for every changed test binary in the delivery ledger; explain any new test over about 15 s. Commit implementation increments unsigned and run gates in the foreground. Prove the full house gate inventory including both locked suites, evals against Cargo’s candidate artifact, every pinned recipe, exact coverage and selftest, publish and seal guards, license and native-tls checks. Distinguish CI-only checks and dispatcher evidence. No collection, freeze, publication or push.

## Delivery ledger (implement, 2026-11-20)

Every task is done locally except the delegated-host half of task 6, which is the dispatcher's and is not an implementation failure. Task 6's proof code, discovery and named refusals are done; the four host proofs have not run on a delegated host (this seat's `systemctl --user` is offline: no user manager, `/proc/self/cgroup` is `0::/`).

**Scope choice: a second, sibling scope.** api-mcp runs in a fresh, non-delegated `mail-compare-<token>-<coordinate>-client.scope` beside the holder's delegated scope, not checked from the chunk's own process tree. Cgroup membership is the kernel's and is inherited across fork, exec, `setsid` and reparenting, so an adapter's detached child stays countable after its parent is gone; a process-tree walk loses exactly that child, and a second group signal says nothing about who was in the group. The bridge lives inside api-mcp, so judging api-mcp's scope covers it. The holder's shim still moves `hold` out, so it stays its delegated scope's only process.

**How it is shared.** The collector and the host proofs share the driver's two new modes rather than a bash function: `client-enter` (handshake, then exec in place) and `client-judge` (the bounded judgment and its receipt). The host proofs build the client scope with the same `systemd-run --user --scope --quiet --unit=…-client -- <driver> client-enter …` argv and judge it with the same `client-judge`.

**Deviation from plan step 4.** Plan step 4 said to keep a private pending index. Instead, an MCP chunk writes its whole output directory privately, at `<output>.scratch/<coordinate>.pending`. After both judgments and the ledger write, the collector publishes it with one no-clobber `rename(2)` (`mv -T -n` onto a name checked absent). This gives the same all-or-nothing view of `chunks/` and the same unpublished, durable partial rows for a failed chunk. It needs no change to the holder, `api-mcp`, `apibench.rs` or the eval crate.

**Budget.** Every step after a chunk's command is clipped to one monotonic 600 s from chunk start, read from `/proc/uptime` (`budget_cs`). The client judgment is a 4 s `timeout` with a 3.5 s inner deadline, capped at 5 s. A known survivor is contained before the holder's scope is asked. `collect_v4_fits_a_worst_case_mcp_chunk_and_both_judgments_in_ten_minutes` proves that a 590 s chunk with a survivor and a live holder scope still ends within 600 s on a controlled clock.

**Elapsed times, local, on the shared host:**
- `tests/mail_compare.rs` binary: 47.4 s in the full `cargo test --workspace --locked`, 611 passed and 49 ignored.
- `client_section::*`: 13 tests in 0.02 s.
- Each new `v4::collect_v4_*` client-scope rig test: about 0.4–0.6 s.
- `v4::*` as a whole: 25 s, 256 passed.
- `api_section::the_mcp_host_proofs_are_discovered_and_refuse_without_their_bytes`: 0.06 s.

No new test is over 15 s. The evals suite is unchanged: 1399 s, all green, against Cargo's `target/debug/swarmail`.

**Gates run here, all clean:**
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo test --workspace --locked`
- `cargo build --locked --bin swarmail`
- evals fmt, clippy and `SWARMAIL_BIN=<that binary> cargo test --locked`
- `scripts/coverage-gate.sh`: exact 100/100/100
- `scripts/coverage-gate.sh selftest`
- `scripts/publish-guard-test.sh`
- `scripts/seal-branch-test.sh`
- `cargo deny check licenses`
- `cargo tree -i native-tls`: matches no package

**Gates not run here.** `brokkr compile` over `.brokkr/recipes/*/` did not run, because this seat's box carries no brokkr binary at all (`command -v brokkr` and a filesystem search find none; a reviewer's box carries 0.15.1, which is not the pinned 0.12.0). This change does not touch `.brokkr/`; the verify seat and CI run it with the pinned 0.12.0. The CI-only contexts are Helm, Docker smoke, RustSec and the macOS test job. On macOS the new /proc- and cgroup-reading tests are ignored by the Linux `cfg_attr`. The portable refusal test runs there.

**Held for the operator.** This PR changes `scripts/compare-mail.sh`, so under house rules it is held for the operator. No package was added and no allowlist changed.


## Rework after review 1 (implement, 2026-11-20)

The panel returned five residual verdicts, all low or info. This pass fixes both security-relevant lows and the cheap stale-wording items. The other lows stay as tracked debt, listed below.

**Fixed:**
- **A false "removed" (security, correctness, guarantees, independent 1).** A `NotFound` on the client cgroup's `cgroup.events` used to count as verified removal. It is now clean only when the parent cgroup is still under the statfs-verified mount. A missing parent is `unknown` ("the observer cannot see the client scope"), and so are an unreadable parent and a parent check cut at the deadline. On the other side, `client-enter` now refuses before writing its handshake unless its own pid is listed in `<mount>/<cgroup>/cgroup.procs` (`client_scope::confirm_member`). A cgroup namespace or a bind-mounted subtree that hides the scope from `/sys/fs/cgroup` therefore leaves no handshake and is judged `unknown`.
  - Proofs: `client_section::a_removed_client_cgroup_whose_parent_the_mount_lacks_is_unknown` (a path the mount never held), `client_section::a_removed_scope_whose_parent_check_fails_or_stalls_is_unknown_within_the_deadline` (paused clock) and `client_section::client_enter_confirms_its_pid_through_the_mount`.
- **Unescaped cgroup names (security 2).** A child cgroup's name is now escaped before it reaches `Member.cgroup`, a note or the console. `Receipt::summary` also escapes every control character, so a receipt read back still prints as one line. Proof: `client_section::a_child_cgroup_name_is_escaped_in_the_receipt_and_its_summary`.
- **Stale wording (correctness 4, guarantees 2, independent 4).** The README's collect-v4 bounds paragraph and the comment in `scripts/compare-mail.sh` no longer say that api-mcp has no scope.
- **Test hygiene (correctness 6).** `the_member_list_is_capped_and_says_so` reads `/proc`, so it now carries the Linux `cfg_attr`.
- **Ledger accuracy (correctness 7, tests task 5 wording).** Both are corrected above.

**Tracked debt (low, not security):**
- The driver's `client-enter` and `client-judge` CLI wiring runs only in the open host proofs. The driver is a bench target, which `cargo test` does not build.
- No test drives the fake adapter's child through `setsid`, and no ignored host proof runs a real surviving child.
- Some diagnostic arms have no fixture: malformed `cgroup.procs`, a cgroup removed while it is being read, the walk and depth caps, and the wrapper's control-character truncation. None of them changes a verdict.
- No local test reaches `admit_v4`'s call to `check_client`.
- Other seams: `COMPARE_MAIL_UPTIME` is honoured in production, a receipt is cut off at 64 KiB, a host proof's `.output()` can wait on a survivor's inherited pipe, and `Kernel::children` silently stops at 256 entries.

Elapsed: `client_section::*`, now 17 tests, takes 0.02 s.

## Rework after review 2 (implement, 2026-11-20)

The panel returned one medium and lows. This pass fixes the medium and two cheap lows; the rest stay tracked debt as listed above.

**Fixed:**
- **GNU-only `mv -T` (independent 1, medium).** The collector's MCP publication used `mv -T -n`, which BSD `mv` rejects, so the ungated `collect_v4_dispatches_api_chunks_to_run_and_to_api_mcp` would exit 2 on macOS CI, and the ungated squat test passed there for the wrong reason. The collector now probes `mv --version` once: GNU keeps `-T -n`; any other `mv` gets `-n` alone, and a new post-check refuses a chunk that landed inside a name taken during the rename (`chunks/<coordinate>/<coordinate>.pending`). Production (Linux, GNU coreutils) is unchanged. collect-v4 itself still refuses a real non-Linux host by name (`collect_v4_names_each_preflight_refusal`); this only keeps the fake-uname rig portable, so the dispatch and squat tests stay ungated and test the right thing on macOS. Proofs, on Linux with a fake BSD `mv` that rejects `--version` and `-T`: `v4::collect_v4_publishes_an_mcp_chunk_with_an_mv_that_has_no_dash_t` (exit 0, published whole) and `v4::collect_v4_refuses_an_mcp_chunk_that_lands_inside_a_name_taken_during_its_rename` (the fake `mv` takes the destination first; exit 2, no `index.json` at the taken name). Both fail against the previous script.
- **Unreadable clock (independent 2).** `mono_cs` checks the integer part on its own, so an unreadable or empty clock reads as 0 rather than raising bash's `10#` arithmetic error and failing every bound. Proof: `v4::collect_v4_reads_an_unreadable_clock_as_zero_and_still_runs` (fails against the previous script).
- **Deadline wording (correctness 5, guarantees 2, independent 6).** R30 and the script comment now state 3.5 s inside a 4 s `timeout`, as the collector passes it (default 3 s, at most 5 s).

**Still tracked debt (low):** admit_v4's call to `check_client` reached by no local MCP fixture; the real `client-judge` receipt bytes never fed to the script's classifier; no real-kernel survivor proof; a receipt over 64 KiB replaced by a synthetic unknown (still exit 2, unsealed); the R29 historical sentence's wording; plan step 4's stale description (the deviation is recorded above).

Elapsed: the three new rig tests take 0.44 s together.

## Rework after review 3 (implement, 2026-11-20)

The panel returned one medium (guarantees 1) and lows. This pass fixes the medium and most of the lows.

**Fixed:**
- **Containment skipped at the 600 s clip (guarantees 1, medium; independent 2).** A failed MCP chunk's worst case was 590 + 4 + (3 + 3 client) + (1 + 3 + 3 holder) s. With every step clipped to the chunk's 600 s, the holder's scope was never asked, killed or stopped. Now the client judgment, every `is-active` query and every poll leave the last 6 s to containment (`contain_cs`). A client scope that is not proved clean is killed and stopped in the same `systemctl kill` and `systemctl stop` calls as the holder's scope (`judge_failed <holder> <client>`). At the clip, the holder's query is what gets skipped. A scope that was not asked is not known gone, so it is killed and counts as unclean. `v4::collect_v4_fits_a_worst_case_mcp_chunk_and_both_judgments_in_ten_minutes` now asserts one kill and one stop naming both units, each at `timeout 3`, after the chunk, and no holder query. It fails against the previous script (only the client scope was killed).
- **Inherited pipes and the judgment's exit in the host proofs (guarantees 2, correctness 6).** `real_chunk` now sends the chunk's stdout and stderr to files and waits only for its exit status. `judge_client` runs `client-judge` under `timeout 4` with `--budget-ms 3500`, as the collector does, with its output in files, and the proof requires exit 0.
- **"Before any other assertion" (guarantees 3, independent 4).** `real_chunk` now requires the client receipt clean, with exit 0, before `records::admit` or any record is read. The README and R30 wording now say exactly that.
- **Proofs that run everywhere (guarantees 4).** The README now says which proofs carry the Linux ignore on macOS.
- **admit_v4's call to check_client (correctness 1, independent 1).** `v4::admit_v4_reads_every_entrys_client_evidence` gives a cost chunk's ledger entry a client receipt, and `admit_v4` must refuse it by name. Deleting the call site turns the test red. The MCP arm's unit binding stays proved at the `check_client` level.
- **A child cgroup removed mid-walk (correctness 2).** Both `NotFound` arms of `diagnose` now record "removed before its members/children were listed (an identity race)". `complete` stays true, like a vanished pid's race. A cgroup the kernel removed held no process and no child at its removal, so no member is missing from the list, and the first observation keeps the verdict a survivor. Proof: `client_section::a_child_cgroup_removed_during_the_walk_is_a_noted_identity_race`.
- **An over-cap receipt (correctness 3).** A receipt larger than 64 KiB is not read. The ledger's `client` is unknown, and its observation says the receipt was truncated, gives its size and names the file that keeps it whole. Proof: `v4::collect_v4_records_a_client_receipt_past_64_kib_as_truncated`.
- **The rename-race test (correctness 4).** The fake `mv` now plants the racer's own `index.json` in the destination it takes, so only the landed-inside check can refuse. The test fails with that check removed (verified).
- **Wording (correctness 10).** The README now names where a taken-name race leaves the rows.

**Still tracked debt (low):** the driver's `client-enter`/`client-judge` CLI wiring runs only in the open host proofs (bench target). There is no real-kernel survivor proof. No collector-level test drives an interrupted MCP chunk. The budget-exhausted arms of `judge_client` (an inner budget of 0, a synthesized 124) have no test. `mono_cs` reads a malformed clock as 0. The `mv` probe tests `--version`, not `-T`.

**Elapsed:** `client_section::*`, now 18 tests: 0.02 s. The 51 `collect_v4_*` tests: 11.9 s together. The new admission test: 0.05 s. The full `mail_compare` test binary: 54 s (621 passed, 49 ignored). No new test waits out a bound in real time.

**Gates on this head:** fmt, clippy, `cargo test --workspace --locked`, `cargo build --locked --bin swarmail`, the evals fmt, clippy and tests against that binary, `coverage-gate.sh` (exact 100/100/100) and its selftest, `publish-guard-test.sh`, `seal-branch-test.sh` and `cargo deny check licenses` are all clean. `brokkr compile` did not run in this seat: no brokkr binary is installed here, and this diff changes nothing under `.brokkr/`. The pinned compile is the verify seat's.
