# Implementation plan — eval privacy recorded-gap repair

Date: 2026-10-24. Run: `continue-epic-192-issue-196-from-119060a2`.
This plan implements the returned artifact-permission clarification and
AC1–AC23 in [spec.md](spec.md).
It replaces the earlier verification-only plan: recovered review supplies concrete
defects. Historical receipts remain historical; none is a new execution.

## Technical context

The unpublished Rust 2024 eval crate has its own workspace/lock and uses existing
Clap, Tokio, Serde, reqwest and tempfile. Unix Linux/macOS remains supported. Root
is a separate single crate; its exact coverage denominator must not change.
Accepted [0055](../../docs/decisions/0055-agent-eval-harness.md) requires separate
eval evidence with real SMTP/HTTP/MCP. No dependency or manifest change is needed.
Only a spec template exists under `.specify/templates/`; no plan template exists.
The required technical context, constitution check, structure, research, data
model, contracts and quickstart are included here.

## Constitution check

Preserve 0001–0005: synchronous acceptance/storage, subscribe-before-check,
oldest-only pruning, AUTH inbox isolation and extraction at ingest. No root code,
root tests, coverage exclusions, locks, allowlists, gates, bundles or recipes are
repair targets. Preserve deterministic test placement under 0020, executed/pinned
proof under 0050, unsigned commits under 0056 and accepted 0058 review routing.
No global environment mutation, sleep synchronization, probabilistic chaos or
mock business acceptance is authorized.

The chief commits exactly `specs/eval-privacy/{spec,plan,tasks}.md`. Existing
proposal [0062](../../docs/decisions/0062-eval-privacy.md) stays unchanged and
proposed; this repair restores existing semantics and introduces no new ruling.
Under accepted 0045 §8 an accepted-ruling conflict returns for an operator ruling;
frozen weakening is refused regardless of green tests. The pinned check owns
advancement. A draft is not implementation, gate, review or platform completion.

## Research and resolved questions

Read all three intakes, both current positions, every recovered review, feature,
issue and manifest; verified all seven hashes/lengths. Read the charter, decision
index and applicable rulings, existing design and historical mutation record in
`specs/agent-eval-harness/plan.md`. The first design inspected source at 0f0613f
and established these historical defects (current inherited edits already attempt the first four, unverified):

- `Cmd::Trial.argv`, `TrialRequest.argv`, `run_body` and `agent_argv` use String,
  although matrix/run_child carry OsString. Lookup-only tests miss worker Clap.
- The Cargo script has literal backslash-n bytes; generic swarmail_unavailable
  accepts spawn failure. Valid oversized JSON plus successful real Cargo isolates
  overflow from malformed JSON/build failure.
- The canary marker precedes output and Linux-only /proc death discovery. The
  non-Linux helper returns 64; fixture_env_probe leaves oracle code after return.
  Separate portable ordinary failure and Linux death controls restore real outcomes.
- `build_swarmail` reads HOME before explicit Cargo/Rustup homes.
- Historical mutation receipts identify pre-c232602 blobs and exits, useful leads
  which do not automatically bind the changed canary.

Adopt simplicity's local ownership and robustness's full execution evidence.
Reject new protocols/codecs/supervisors, broad lifecycle/category cleanup and a
mutation framework. The spec's table owns every retained LOW/INFO disposition.

## Project structure and touched files

| Path | Planned action |
|---|---|
| specs/eval-privacy/spec.md, plan.md, tasks.md | Sole chief commit; preserve dated history, settle repairs and proofs. Later record truthful evidence disposition. |
| evals/src/main.rs | Native trial argv, one conversion of textual matrix input, unchanged literal worker args; extend row tests. |
| evals/src/runner.rs | Native TrialRequest/validation/run_body/agent_argv/resolver path; lazy HOME defaults; private exclusive artifact creation/publication and focused owning tests. Preserve maps/readers/lifecycle. |
| evals/tests/runner.rs | Producer/count/completion/short-control proof, portable artifact canary, Linux death proof, native argv and HOME regressions; subprocess umask/mode, conflict and artifact-failure controls; concurrent bounded test output collectors. |
| evals/tests/report.rs | Real matrix CLI with non-UTF-8 PATH, markers/bytes/row/exit assertions; retain cardinality cases. |
| evals/tests/common/mod.rs | Counted both-stream emission/completion, cfg-contained Linux probe/death; no production fault variables. |
| Other existing eval request/resolver callers | Necessary type adaptations found with rg/compiler only, including proof/fixture callers if present. |
| evals/README.md; specs/agent-eval-harness/plan.md | Downstream reconciliation of current evidence and historical claims only; preserve dated receipts/limits. No chief edit here. |
| .forge/ evidence/results; .brokkr/state/ inputs | Uncommitted provenance/receipts; never fabricate or rewrite journal history. |
| Tooling record, candidate-local .forge/bin/brokkr, pinned scripts/shims | Read-only proof inputs; never change gates to obtain a pass. |

Returned research — 2026-10-24: HEAD is b0ccd5a plus the five inherited modified
files listed in the spec. Full packet reread and seven manifest matches confirmed.
Current write_artifacts still creates directories and writes predictable temporary
paths without explicit private modes; rename preserves those modes. The returned
0664/0775 CLI observation is attributed implementer evidence, not a chief rerun.
Current output_with_deadline pipes both streams but waits before collecting them;
repair that test-local helper for discriminating flood/mutation proof. Existing
evals/Cargo.toml already locks tempfile 3.27.0; no new dependency is needed.

No new tracked source file is planned. Extend existing helpers. Root protocols,
Admin response allocation, matrix shutdown and general diagnostics stay excluded.

## Data model and contracts

Use `Vec<OsString>` for trial CLI/request argv and `&[OsString]` through execution.
Keep `report::AgentSpec` JSON `Vec<String>`, labels and task IDs textual. Convert
JSON args once, then resolve/preserve native program and literal later bytes.
Reuse the existing resolver owner, with a small adapter only if necessary. No
lossy/display conversion, shell joining, serialized resolved-path protocol,
environment forwarding or worker subcommand. Adapt callers locally; schema 1,
real statuses, scoring precedence and matrix cardinality remain unchanged.

Preserve caller cwd/PATH resolution before clearing/changing cwd, built-ins and
already-absolute worker paths. Arguments cannot contain NUL as before. Keep exact
maps, eight generated inputs, fixture-only secret, private dirs and wrapper auth.
HOME is optional with both explicit homes: missing-selector branches alone read
it and call existing defaults/absolute-locator logic. Empty selectors still refuse;
relative locators and optional toolchain/target selectors keep existing behavior.

Test receipts contain synthetic count/status/marker facts, not transcripts.
Checked complete writes of a known fixture >1 MiB, followed by a byte receipt,
establish emission. Flush before completion. Missing markers, short writes or
Cargo error fail. Test marker variables never become production map exceptions.

## Private artifact implementation

Keep write_artifacts as owner. Small private helpers beside it may separate leaf
creation and one-file publication so tests can inspect the actual pre-payload
operation. Avoid a filesystem trait/framework, new module or public option.

1. Create missing selected-root parents as needed, without permission changes to
   existing ancestors. Exclusively create the final trial leaf with Unix
   DirBuilder mode 0700 and non-recursive leaf creation. Normalize only that
   newly owned empty leaf to exact 0700 before any payload creation. A pre-existing
   leaf of any kind refuses; no adopt/chmod/retry under another identity.
2. Use the already locked tempfile builder/NamedTempFile operation inside that
   private leaf, with restrictive exclusive creation. Verify its pinned Unix
   implementation creates no broader than 0600; set exact 0600 through as_file's
   descriptor before write_all. Temporary paths are owned by this operation,
   not reused predictable names. No global umask changes or raw error formatting.
3. Fully write one serialized payload, then use persist_noclobber for the final
   name. Verify pinned Linux/macOS no-replace semantics, including any hard-link
   implementation and temporary-unlink limitation. This is complete-file atomic
   visibility, not guaranteed atomic removal of the temporary name or crash
   durability. Never fall back to ordinary rename/truncation. Retain the existing
   state/result/diagnostics/metadata order and safe content.
4. Map all errors to existing artifact_write internally/artifact_write_failed
   publication. Preserve run_trial's existing score/row/exit. Handle persistence
   error ownership so only owned temporaries are dropped; any safe residual stays
   private. A failed normalization may leave only a stricter empty object, never
   payload or group/other access. Unsupported filesystems fail safely. Do not recursively remove a
   directory, repair existing entries or add recovery/transaction behavior.

Before implementation, confirm those tempfile guarantees from the already
available locked source and test them on supported platforms. If the chosen API
cannot meet AC22/23, report the exact obstacle rather than silently adopting
weaker semantics or broadening scope. A local alternative no-replace primitive
needs equally strong Linux/macOS proof; the default ruling remains tempfile.

Proof uses an actual snapshot-producing ordinary failure in a subprocess with
its own umask; pre-create an accessible selected root for the restrictive mask.
Require four final files, exact modes and row/exit, plus canary and metadata
checks. Source-order audit plus execution of the actual creation helper before
write establishes no broad temporary window. Owning tests exercise collision and
write/publication failure deterministically; if a seam is necessary, keep it
cfg(test)-only at the owner with an installing test per 0020. Prefer existing
inline eval unit tests, no production environment flag or global mutation.
Temporary-name collision controls test the selected exclusive primitive without
forcing predictable production names; final conflicts test no-clobber rather
than an obsolete rename arm. Check unrelated sentinel bytes/modes and every
owned residual. Record permission old-writer red/restored-green independently
of AC8 inheritance/raw-publication/metadata-error controls.

## Implementation and proof sequence

1. Bind HEAD plus inherited source-diff, packet and tool identities. Preserve the
   five unverified modified files; do not misreport the baseline as clean.
   Keep old reviews/gates separate;
   reproduce relevant failures with nonzero tests. Read every request/resolver
   caller before type changes; triage remains engine.
2. Remove String transport in existing owners. Matrix CLI test uses a UTF-8 bare
   name in an invalid-UTF-8 PATH directory with unchanged agents JSON. Require
   execution/completion and literal bytes, one matching row, actual agent status
   and aggregate exit. Direct trial includes an invalid-UTF-8 later argument.
   Retain built-ins, missing/relative/absolute lookup, wrapper auth and product
   tests; reinstate the old worker boundary narrowly to prove discrimination.
3. Make HOME defaults lazy. Prove real offline/locked build with both absolute
   homes and HOME removed, tools otherwise discoverable. Exercise each missing
   default without HOME and each empty selector as safe refusal. Retain real
   custom-target discovery and exact environment observations.
4. Fix flood script newlines and quote paths safely; display formatting is not
   shell escaping. Generate valid unknown-reason JSON with a synthetic canary,
   stream >1 MiB through stdout using checked writes, then real Cargo with its
   unchanged args. Require launch, count, post-write marker, real Cargo success
   and completed drains. Below-limit equivalent must succeed. Use a bounded
   test deadline and kill/reap cleanup: unbounded `.output()` is not a liveness
   fence for blocking preparation. Collect the test CLI's two pipes concurrently
   while waiting, with bounded
   capture plus incremental byte/canary scanning and total counts; continue
   discarding after the bound. Join collectors at EOF, retain a deadline and
   kill/reap on expiry. A leak mutation must fail no-echo/byte assertions instead
   of merely filling a driver's pipe. Keep scanner state across chunk boundaries.
   Scan outputs/artifacts for raw bytes. Removing
   overflow rejection must fail the intended assertion; stopped drainage must
   fail completion/deadline, not hang the suite.
5. Extend unsafe_output for >64 KiB each stream, split invalid bytes and canaries
   inside the old retained-tail region. Count/complete only after full writes and
   flush. Portable signup expects Fail/GaveUp, agent 0 and trial exit 1 with
   artifacts. Recursively scan every final/temp file; check null executable
   metadata and private modes. Separate Linux death control requires actual kill
   success of the runner's own server and HarnessError. Entire probe/oracle
   continuation lives in cfg blocks with complementary non-Linux expressions.
   No warning suppression or disabled portable coverage.
6. Complete the private artifact implementation above and AC21–AC23 proofs,
   including permissive/restrictive subprocess masks, initial modes, conflicts,
   deterministic failure/residual modes and original-writer red/restored green.
   Preserve private content and existing artifact-error scoring behavior.
7. Add independent wrong-label, exit-mismatch and CR row tests alongside existing
   validation. Bind historical mutation regions/receipts or reproduce missing
   inheritance, raw-publication and unsafe metadata/error controls. Reassess the
   changed canary. Each mutation fails its intended assertion; restoration passes.
   Leave no mutation before final gates.
8. Execute focused/full eval, root/pinned and available macOS proofs. Record Python
   availability for its existing readiness test. Resolve scoped regressions,
   reconcile evidence/docs and obtain five AC14 reviews on the changed verified
   candidate. Keep unrelated debt visible; engine owns review/shipping decisions.

## Quickstart and evidence requirements

These examples are not receipts. Use installed offline caches and actual recorded
candidate-local tooling; verify installs nothing.

```bash
cargo test --offline --locked --manifest-path evals/Cargo.toml --test runner oversized_cargo_record_is_drained_and_fails_without_echo
cargo test --offline --locked --manifest-path evals/Cargo.toml --test runner child_output_canaries_are_drained_without_publication
cargo test --offline --locked --manifest-path evals/Cargo.toml --test report
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --offline --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings
cargo test --offline --manifest-path evals/Cargo.toml --locked
cargo test --offline --manifest-path evals/Cargo.toml --locked --test proof
```

Require nonzero focused counts and full suite counts/exits. Retain real oracle,
null idle classes, wrong-action, overlapping-isolation, environment, HTTP-error,
readiness, lifecycle, cleanup and matrix controls. Root green cannot prove evals.
Each receipt names commit/tree (and mutation diff), source/test binding, command,
actual exit, OS, log and relevant engine/resource identity. Empty logs or matching
digests do not replace execution.

Run every current [house gate](../../docs/house-rules.md#gates--part-of-every-change)
through its owning pinned route, including locked checks, exact coverage/selftest,
publish/seal, licenses and fail-closed native-tls absence. Preserve AC11/T8
design-check/selftest, actual prescribed design/verify dispatch, policy/delivery
tamper proofs, checked shims, recorded-engine identity and warning-free compile.
Do not duplicate/weaken the sole inventory. Actual engine execution is mandatory;
no hand-written policy emulator or sibling binary.

Accepted 0058 requires separate Astra correctness, official DeepSeek Flash
security, Sonnet 5.5 tests, Astra guarantees and Fable 5.1 independent results.
Never DeepSeek Pro, author substitution or unauthorized fallback. Protected review,
worst verdict and security hold remain; historical clean reviews do not count.

## Supported-platform and CI handoff

T4 stays open until actual macOS preparation with supported system/rustup/toolchain
discovery, ordinary/custom targets and repaired controls, including private artifact
creation/publication under both umasks. Actual macOS all-target
clippy/eval tests establish portable cfg/canary behavior. Linux or cross-compilation
is not that evidence. Missing hardware is a gap, not a waived criterion; do not
assume CI supplies it.

Keep charter CI-owned chart/history/selftest, real Docker persistence/context,
network RustSec, tag/rate/publication and privileged port-80 separately attributed
with actual status. Handoff distinguishes repaired defects, retained debt, local
proof, historical source-bound evidence and missing platform/CI proof. No push,
PR, merge, issue closure or model measurement.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Umask exposes temporary or final bytes. | Restrictive exclusive creation, descriptor-based exact modes before payload, private leaf and subprocess mask/pre-write tests; never post-write repair or global umask. |
| Conflict clobbers unrelated data. | Exclusive leaf and temporary creation plus no-replace publication; directory/file/symlink/hard-link sentinels, owned cleanup only. |
| Publication API overstates atomicity. | Inspect pinned tempfile and test Linux/macOS; complete-file visibility only, private residuals allowed, no whole-bundle or fsync claim. |
| Restrictive mask removes owner access. | Normalize newly owned empty leaf/open descriptor before payload; accessible selected parent fixture, exact-mode proof under 0777. |
| Hostile ancestry is mistaken for confinement. | State trusted stable caller-selected ancestry and unconfined/same-user limits; no recursive chmod or sandbox promise. |
| Resolver succeeds but worker loses OS bytes. | Native types throughout; actual matrix/direct-trial CLI tests and byte markers; accept local caller adaptations. |
| Spawn/parse failure masquerades as overflow proof. | Valid oversized/short records, counted writes, completion and real Cargo success; overflow mutation fails the intended assertion. |
| Stopped drainage or a leak blocks test-driver pipes. | Concurrent bounded collectors with incremental scans/counts and EOF joins, deadline and kill/reap cleanup; timeout is failure, not overflow/no-echo proof. |
| Marker overstates emitted bytes. | Count successful checked writes; flush before completion. Start marker or intended count alone is insufficient. |
| MacOS asserts fictional death or loses coverage. | Portable ordinary-failure artifacts and cfg-contained Linux death control; actual macOS receipts, T4 open meanwhile. |
| HOME repair changes policy/defaults. | Local lazy selector branches; explicit/missing/empty proofs and exact maps, no inheritance fallback. |
| Changed tests inherit invalid mutation credit. | Bind exact regions/receipts or rerun missing controls; intended red/restored green, clean final diff. |
| LOW debt disappears behind success claims. | Preserve every finding, including orphan cleanup and category/cap debt, for reviewers; return if inseparable. |
| Fixtures leak output or widen maps. | Explicit test-owned files/wrappers, synthetic canaries, all-artifact scans, restored exact private modes and unchanged maps. |
| Wrong gate or reviewer identity supplies clearance. | Pinned identity plus actual dispatch; five separate 0058 roles, security hold and no invented fallback. |
| Draft/Linux green becomes full completion. | Report drafted only here; required tasks/platform/CI evidence remain explicit. No decision acceptance or frozen weakening. |
