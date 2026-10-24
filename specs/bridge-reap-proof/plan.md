# Implementation plan: bridge reap proof

Implements [spec.md](spec.md); ordered proof work is in [tasks.md](tasks.md).
The repository has no `.specify/templates/plan-template.md`; this plan
includes the required technical context, constitution check, project
structure, research, data model, contracts, quickstart and risks here.

Updated: 2026-10-24 for evidence recovery after the two stopped attempts.
Run: `epic-192-issue-179-evidence-reco-81b99d67`. Original implementation
mechanics remain historical rationale; only the residual procedure below
is pending implementation work. Preserve `c8364ed` and all AC1–AC8/R1–R6.

## Technical context

Single Rust crate; existing Unix-specific integration test in the binary
harness. `tests/binary.rs` imports `tokio::process::Child`; Cargo.lock pins
Tokio 1.53.2. Existing libc supplies SIGKILL and ExitStatusExt supplies the
signal observation. No new dependency, helper abstraction or platform API.

## Constitution check

Accepted 0020 permits this integration harness's cold diagnostics and
requires observable synchronization. Preserve every frozen guarantee and
accepted 0001–0005 behavior. Accepted 0050 requires actual execution and
pinned proof ownership; 0056 requires unsigned commits. Intake already
selected the full engine route under 0057 because this is a lifecycle and
concurrency proof, irrespective of the small diff. No new semantic ADR.

## Project structure and files touched

| Path | Role |
| --- | --- |
| `tests/binary/helper_reliability.rs` | Preserve completed candidate bytes; temporary mutants for missing proof only, restored after each execution. |
| `specs/bridge-reap-proof/{spec,plan,tasks}.md` | Chief updates exactly these three; subsequent residual work only adds directly necessary evidence/clarifications to plan.md. |
| `target/bridge-reap-proof/receipts/epic-192-issue-179-evidence-reco-81b99d67/` | Ignored search record, provenance index, raw receipts and restoration records; explicitly handed to current reviewers. |
| `tests/binary.rs` | Read-only owner of `McpChild`, `finish_with_deadline`, `kill_and_reap`. |
| `Cargo.lock` and installed Tokio source | Read-only semantic evidence. |

No other source/note files are authorized. Keep raw execution receipts in
the target receipt root above. Keep only a concise truthful index/reference
in this plan; no embedded raw logs and no raw evidence writes under `.forge`.
The mandated engine result is the explicit control-record exception.

## Research and source evidence

The original chief (2026-10-23) recorded inspection of installed Tokio 1.53.2:

- `src/process/mod.rs:1216–1226`: `id()` reads `FusedChild`; it returns
  None only for Done and does not poll/reap. Documentation explicitly
  names avoidance of reused-PID confusion.
- `src/process/mod.rs:1379–1394`: a successful `wait()` caches exit status
  by transitioning to Done.
- `src/process/mod.rs:1398–1427`: Unix `try_wait()` can reap and transition
  a live handle; the Done arm simply returns the stored status.
- `src/process/unix/mod.rs:164–165`: live `try_wait` delegates to the owned
  std child. Do not generalize Tokio's optional id to `std::process::Child`.

In `tests/binary.rs`, `kill_and_reap` requests kill and awaits bounded
`child.wait()`. `finish_with_deadline` awaits that cleanup on failure before
returning its diagnostic. Fixtures flush readiness, optionally close
stdout, then park indefinitely without descendants. The initial live
`try_wait` and Pending poll do not finish the owned handle. Only completed
helper polling can leave it Done before the new observation. OS signal
arrival alone cannot change the outer handle to Done.

Historical baseline SHA-256 (original chief inspection on 2026-10-23,
not execution proof or current executable authority):

| Input | SHA-256 |
| --- | --- |
| `tests/binary/helper_reliability.rs` | `2fdb0a3fd1cba70580cfd93b900cfeb121bfd704df9e836a2b3ff1844515cf69` |
| `tests/binary.rs` | `7da125cd5945bfc7f511c0c6fdaed52eb8baeb08497c5416ab29ede867752554` |
| `.forge/bin/brokkr` | `e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c` |

### Current authority and preservation baseline — 2026-10-24

Root re-pin `238372d` records the authoritative build in
`bundles/self/tooling.json`. Current chief inspection independently found
candidate-local `.forge/bin/brokkr` SHA-256
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`, equal
to that file's `brokkr.binary_sha256`. The historical `e525babf…` value
above is retained as an earlier inspection record, not rewritten to imply
that past executions used today's binary. No rebuild or re-pin is needed
while current equality holds; version equality alone is insufficient.

Preserve the completed `c8364ed208612b72955cf271c2341e7c16f40c3f` bytes:

| Input | Current SHA-256 |
| --- | --- |
| `tests/binary/helper_reliability.rs` | `5dc1b0573686e53d9444273dc14db8c292596cf12c8fd098c1d8ce962269ddcb` |
| `tests/binary.rs` | `7da125cd5945bfc7f511c0c6fdaed52eb8baeb08497c5416ab29ede867752554` |

The recovery chief independently rechecked both source hashes, their empty
diff against `c8364ed`, and current local engine equality on 2026-10-24.
These are inspection facts, not new mutation, gate or reviewer receipts.

### Current evidence reference — 2026-10-24

Run `epic-192-issue-179-evidence-reco-81b99d67` uses the ignored
[index](../../target/bridge-reap-proof/receipts/epic-192-issue-179-evidence-reco-81b99d67/index.md) and
[chief search record](../../target/bridge-reap-proof/receipts/epic-192-issue-179-evidence-reco-81b99d67/chief-search.txt)
(search SHA-256 `05b622a9bdbea9d4f62ad1a7d4583806c26c22380300bf6865af46cbf9f34132`).
The search record names exact inspected paths, methods, outcomes and read
summary hashes. Local artifact storage was empty; local/predecessor journal
and sibling receipt paths returned boxed FileNotFoundError. Accessible
ledger/results provide leads and historical summaries, not raw reds.
This bounded search does not establish host deletion or exhaust further
accessible references; implementation records follow-up searches separately.

All four matrix cells remain unproved here. Original provenance and actual
reviewer access remain unestablished. No mutant, restoration execution,
fresh test, full-gate or outer-verify receipt is claimed by this design.
Update this reference with actual receipt/index hashes, restoration and
fresh verification references during implementation; retain precise gaps
until discharged. Never clean these target receipts before review completes.

### Current implementation evidence — 2026-10-24

The current run's ignored [evidence index](../../target/bridge-reap-proof/receipts/epic-192-issue-179-evidence-reco-81b99d67/index.md)
records the four sequentially regenerated AC5 rows, raw outputs, successful
restorations, focused green tests and pinned verification receipts. Its
[implementation search record](../../target/bridge-reap-proof/receipts/epic-192-issue-179-evidence-reco-81b99d67/implementation-search.txt)
keeps exact predecessor paths and access limits; current-run reds do not
establish original predecessor provenance. The required four reviewer
workspaces must read and hash every indexed receipt after dispatch; access
is not claimed here. Retain the ignored receipt tree through review.

## Residual implementation procedure — 2026-10-24

1. Record initial HEAD, clean status, both preservation hashes and engine
   equality. The historical implementation order below is already present;
   do not replay it. Keep all AC1–AC8 and this spec's R1–R6 binding.
2. Follow `.forge/ledger/epic-192-issue-179-verify-and-re-ca74e365.md` and
   actual accessible journal/worktree references for predecessor runs
   `epic-192-residual-lane-next-179--826aa227`,
   `epic-192-179-follow-up-remove-th-91e4bdaa` and
   `epic-192-issue-179-verify-and-re-ca74e365`. Record inspected paths,
   journal event/receipt references, outcomes and any access boundary.
   The local ledger and seq-375 correctness result are search leads, not
   raw red proof. An empty local artifact directory does not prove loss
   elsewhere. Do not create, import or rewrite a journal for this repair.
3. Verify recovered receipts against the evidence contract below. Copy
   verified original bytes without altering predecessor storage; record
   source and copied hashes and provenance. Explain any source differences
   and prove relevant test/helper equivalence. A summary or unbound log is
   not valid. Mark each matrix row recovered or still unproved with reason.
4. Regenerate only unproved rows using the existing negative-control method
   below. Save exact candidate bytes outside the mutation target first;
   install unconditional restoration on normal/error exits. Run one mutant
   and one mode at a time, never concurrently with another experiment or
   gate in this worktree. Preserve actual command exit separately from log
   collection success. Keep failed attempts; do not overwrite them. After
   each experiment compare restored bytes/hash independently. Following an
   interruption, restore and verify before any further proof; a trap alone
   is not crash-proof. Only the final finish invocation and temporary
   existing-loop mode selection may change. Keep the lifecycle guard and
   explicit same-handle cleanup intact. A mismatch blocks the next case;
   never refresh the baseline from dirty source. If the owning process was
   externally killed before cleanup was recorded, leave that lifecycle
   receipt incomplete even after restoring source. Do not signal a saved PID.
5. Assemble the small ignored index, all raw files and search/restoration
   records, including saved candidate bytes, under the target receipt root
   in the project table.
   Add a dated plan reference describing that current-run evidence when it
   exists; do not predeclare successful recovery or execution. Expose the
   files through the existing review evidence handoff, and require each
   actual reviewer workspace to read/hash every indexed receipt. Record
   the observed reviewer access in run-local evidence. Paths readable only
   in this workspace do not close the finding. Retain files through review.
6. Check exact candidate restoration and permitted final diff, then run
   Quickstart, the complete house gates and actual pinned offline verify.
   Historical red executions establish falsifiability, never fresh green
   verification. The engine owns subsequent protected review dispatch;
   no nested run, provider substitution or route change is authorized.

### Evidence contract and four-row matrix

Use one human-readable index, not a new schema or evidence library. Each
row identifies recovered versus regenerated origin; original run/candidate,
journal/worktree and receipt path; relevant test/helper hashes; mutant diff
or bytes; selected mode and exact command; selected test count and actual
execution exit; raw-output path/hash; lifecycle diagnostic and cleanup
outcome; saved/restored hashes and restoration record. Copied hashes prove
byte identity, not authentic origin: retain the provenance chain. For newly
executed rows, label the current candidate and execution rather than
attributing them to predecessor history.

| Mutant | Independently selected mode | Required red |
| --- | --- | --- |
| `bridge_timeout_returns_without_reaping` | `bridge-open` | Lifecycle diagnostic, mode, successful explicit cleanup, nonzero test exit. |
| `bridge_timeout_returns_without_reaping` | `bridge-closed` | Same, after real readiness and closed-mode EOF. |
| `bridge_timeout_returns_with_live_child` | `bridge-open` | Lifecycle diagnostic, mode, successful explicit cleanup, nonzero test exit. |
| `bridge_timeout_returns_with_live_child` | `bridge-closed` | Same, after real readiness and closed-mode EOF. |

Compilation failure, zero tests, setup failure, cleanup failure, outer timeout,
wrong-mode output or a later string mismatch leaves a row unproved.
No saved-PID probe, new dependency, permanent switch, sleep synchronization, suite serialization
or helper refactor is permitted. If proof requires exceeding this scope,
report the evidence through the existing bounded return rather than weaken
acceptance or silently enlarge the work.

## Data model and contracts (implemented design)

No public or persisted data changes. Local evidence consists of the finish
Result, a boolean completion snapshot, cached ExitStatus and diagnostic
text. The same Child owns the process throughout. The boolean records
whether the helper completed its obligation before the observer intervened;
it must never be recomputed after fallback cleanup to decide success.

Historical implementation order, already present at `c8364ed`, inside each
existing mode iteration:

1. Preserve readiness, closed-mode EOF, initial live status and Pending
   operation choreography. Delete saved `pid`.
2. Keep `let result = bridge.finish_with_deadline(Duration::ZERO).await`
   without `unwrap_err` until lifecycle validation has occurred.
3. Immediately capture `let completed = bridge.child.id().is_none()`.
   If false, await `kill_and_reap(&mut bridge.child)` and then panic with
   AC4's named lifecycle text, mode and cleanup result. No status read may
   precede this capture. Explicitly preserve the failed fact.
4. On the true branch only, unwrap the expected error, retain both timeout
   and successful-cleanup diagnostic assertions, obtain cached status and
   require SIGKILL. Remove the obsolete later id check and raw waitpid
   block. Add the short explanatory comment beside the first observation.

The status check cannot reap on the passing path because the handle was
already Done. A live child and a killed-but-unobserved child both fail the
same completion check. This is proof of helper completion, not an assertion
that the missing-reap mutant necessarily reached a kernel zombie state.

## Negative controls and restoration

Save the implemented green file bytes and hash before each mutant. Replace
only the final finish invocation in this test, leaving its new observation
and cleanup intact. Supply an explicitly typed synthetic Err with diagnostic
strings that would otherwise satisfy existing checks; type it as the actual
finish Result if inference requires it. Synthetic text exists only in the
mutant and is labeled mutation-only.

| Mutant | Replacement action | Required failure |
| --- | --- | --- |
| `bridge_timeout_returns_without_reaping` | `start_kill` on this Child, no wait/try_wait, then synthetic result. | Completion is false; AC4 diagnostic after successful bounded cleanup. |
| `bridge_timeout_returns_with_live_child` | No kill or wait, then synthetic result. | Same lifecycle diagnostic and cleanup while fixture was left live. |

For each mutant run bridge-open and bridge-closed separately by temporarily
selecting one existing loop mode. A panic on the first loop iteration must
not count as proof of the second. Neither control changes helper definitions
or adds a permanent environment switch/mutation framework. Record four
nonzero test executions that reached the intended diagnostic, not compiler
failures or string mismatches. Record cleanup success and source restoration
hashes; restore the full two-mode loop and exact saved bytes after every
experiment. A final diff must contain no mutant, synthetic error or selector.

## Quickstart and verification

After implementation and exact restoration:

```bash
cargo test --locked --test binary helper_reliability::never_exiting_bridge_child_is_timed_out_killed_and_reaped -- --exact --nocapture
cargo test --locked --test binary helper_reliability
```

Require actual nonzero selected test counts; the deliberately ignored
fixture is launched by its parent, not separately counted as a passing
regression. Keep normal test concurrency; no PID-churn stress is necessary.

Then execute the entire authoritative
[house gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
on this candidate: fmt, locked workspace clippy/tests, pinned compile,
exact coverage and selftest, publish-guard and keyless seal proofs, and
licenses. Before execution, hash this candidate's `.forge/bin/brokkr` and
require equality with `brokkr.binary_sha256` in the current
`bundles/self/tooling.json` (the dated current-authority value above).
Refuse a missing or mismatched binary; use that exact local binary for
`compile --bundle bundles/self`; version equality is insufficient.
Run fail-closed native-tls absence: Cargo tree command failure is failure,
not evidence of absence. Execute actual outer offline verify through the
pinned proof chain with checked shim digests, not a candidate-script-only
substitute. Preserve failed receipts as well as restored green evidence.
No prior candidate's verify receipt transfers. Verify stays offline and
installs nothing; the engine dispatches it through the existing route.

The chief may preflight `bash bundles/self/scripts/design-check.sh check .`
after committing these three artifacts. The sequence's dispatched pinned
check owns the design verdict. Implementation, mutation proof, full gates,
outer verify and independent review remain subsequent delivery work.
CI-only checks in the charter remain root/CI obligations, not waived or
claimed here. Use DashScope Qwen Max correctness, Claude Opus security,
official DeepSeek Flash guarantees and DashScope GLM 5.3 tests; launch no alternate
route or nested delivery run. Root owns integration and issue closure.

## Risks

| Risk | Mitigation and accepted cost |
| --- | --- |
| Status observation repairs missing reap | Capture non-mutating id-none first; missing-reap mutant must fail before any test-side reap. |
| Reused PID names another child | Delete all saved-PID probing; use only the original owning handle. Accept dependence on Tokio's documented completion state. |
| Failure panics before cleanup | Retain Result, capture state, bounded same-handle cleanup, then named failure; keep existing kill-on-drop fallback. Cleanup errors remain visible failures. |
| Mutant red proves only a string mismatch | Lifecycle guard precedes diagnostic asserts; require named assertion and cleanup receipt for each mode and mutant. |
| Early panic hides the closed-stdout mode | Separate mode-specific negative executions; restore the ordinary two-mode test. |
| Temporary mutation leaks into final candidate | Save exact bytes/hash, restore after each control, compare hashes and diff before final green tests/gates. |
| Tokio upgrade changes the proof assumptions | Keep source/contract rationale in test note; recheck id/wait/try_wait semantics with any future Tokio change. No new dependency pin is needed here. |
| Scope or evidence expands beyond this residual | For the residual, diff against c8364ed and preserve both test files byte-for-byte; label historical/design/implementation/verify evidence distinctly. |
| Historical tooling value becomes an operative instruction | Label the old inspection explicitly; hash the candidate-local binary against current tooling.json before execution, without rewriting past provenance. |
| Copied or inaccessible logs falsely certify AC5 | Record actual origin/search boundaries, validate every row, and require read/hash checks in each dispatched reviewer workspace. Accept manual handoff, not a new archive service. |
| Valid historical red is confused with current green | Label provenance separately; regenerate only unproved reds but require fresh complete verification on the final candidate. |
| Crash or parallel gate observes mutated source | Run mutations sequentially; save exact bytes outside the target, restore on exits and independently check bytes after interruption before any green work. |
| Read-only artifact destination repeats the stopped attempts | Use only the target receipt root for raw evidence; mandatory engine results are control records, not artifact-storage precedent. |
| Ignored evidence is erased or new reds appear to repair history | Retain through review and prove each reviewer read/hash; report original provenance gaps separately from successful regeneration. |
| External termination interrupts owned-child cleanup | Restore bytes before further work, retain incomplete receipt and report missing cleanup; source restoration and kill-on-drop do not prove reap. |
