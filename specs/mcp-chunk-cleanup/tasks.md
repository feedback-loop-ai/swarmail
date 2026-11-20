# Implementation and proof order

- [x] Establish the external MCP runner and exec handshake in the collector and reuse it from the host harness. Prove distinct fresh scope identities, pre-exec client membership and adapter inheritance, with only hold delegated. Retain api_section::only_the_holder_enters_the_delegated_scope. Exercise direct Swarmail and refusal before spawn on non-Linux.
- [x] Implement recursive client judgment and bounded diagnostics on every attempted launch. Test populated nested scope, empty, verified removal, missing handshake, mismatched identity, inaccessible/malformed observation and PID disappearance. Inject stalled observation and diagnostics against one controlled clock and assert the five-second cap and remaining 600-second budget, including holder checks and containment.
- [x] Split checked MCP index preparation from atomic no-clobber publication. With valid holder evidence, pause after preparation and prove public admission still fails. Exercise successful exits 0 and 1, invalid receipt, dirty holder, timeout, startup/control failure, interruption before commit, ledger write failure and duplicate destination. Verify failed attempts leave no new public seal and unrelated finalizers still admit.
- [x] Bind the client verdict to the v4 ledger, failure output and admission. Test a clean record and independently mutate or remove its identity, coordinate and verdict. Verify refusal and the original holder proof independently. Exercise native admission without a client scope.
- [x] Run an event-ordered fake adapter through the collector fixture. Wait for its session-detached, TERM-ignoring child to acknowledge readiness and keep it alive through observation. Assert exit 2, named PID/process and coordinate in the persisted report, no admissible index, no next launch or retry, and bounded teardown. Repeat with the child reaped before judgment and assert clean evidence and successful publication. CI must run the pair without host delegation or protocol-length sleeps.
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

**Gates not run here.** `brokkr compile` over `.brokkr/recipes/*/` did not run, because this box carries no brokkr binary. This change does not touch `.brokkr/`; the verify seat and CI run it with the pinned 0.12.0. The CI-only contexts are Helm, Docker smoke, RustSec and the macOS test job. On macOS the new /proc- and cgroup-reading tests are ignored by the Linux `cfg_attr`. The portable refusal test runs there.

**Held for the operator.** This PR changes `scripts/compare-mail.sh`, so under house rules it is held for the operator. No package was added and no allowlist changed.

