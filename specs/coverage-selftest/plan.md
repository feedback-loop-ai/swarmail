# Implementation plan — coverage selftest recovery

Updated: 2026-10-24 — attributed scope return from `f3cee46`, descending from
recovery HEAD `310b969`. Implements [spec.md](spec.md) AC1–AC16 and
[proposed 0064](../../docs/decisions/0064-coverage-reservation-tooling.md).
This replaces the unresolved-causality plan; dated spec history remains.
No implementation, gate or review completion is certified here.

## Technical context

The canonical gate is Bash with `set -euo pipefail`. Explicit resolution uses
checked parent creation, plain leaf mkdir and caller ownership 0; defaults
use owned disk-backed mktemp trees. Retained full-context and same-resolver
traces attribute two admissions to uutils 0.8.0 turning kernel EEXIST into exit
0. The fixture also has unchecked FIFO operations, fixed descriptors 3/4,
status-only records and combined-log refusal matching. Both need repair.

Choose inline Python 3 stdlib under `-I -S` for direct leaf creation. This box
has `/usr/bin/python3` 3.14.4 with isolated stdlib imports; that does not certify
every verifier/platform. Publish-guard's Python use does not establish a
standalone coverage prerequisite. AC16/0064 explicitly add that prerequisite
for explicit reservation and selftest. Default resolution stays independent.
No Cargo dependency, Rust line, scanner command or compiled bootstrap helper.

Compiler fixtures retain jq, Cargo, installed coverage nightly, cargo-llvm-cov
and the standalone scanner. No plan template exists in `.specify/templates/`;
requested research, data model, contracts and quickstart are included below.

## Constitution check

Preserve accepted 0007/0020/0035/0045/0050, exact positive 100/100/100, every
frozen guarantee and ingest extraction. AC8 permits only the evidenced
resolver exception and selftest work. The Python prerequisite is a proposed
tooling contract. Under 0045 §8 compatible work may proceed while proposed;
an actual accepted-policy or supported-environment conflict requires explicit
return, not installation, fallback or silent platform removal.

Keep the engine lane, four protected reviews and security hold under 0057:
Qwen Max correctness, Claude Opus security, official DeepSeek Flash guarantees
and DashScope GLM 5.3 tests. Engine owns dispatch; never Pro or fallback.
Unsigned commits under 0056; no push, merge, tag or publication.
Recheck candidate-local `.forge/bin/brokkr` against `bundles/self/tooling.json`
before compile/dispatch; version equality or a sibling binary is insufficient.
Verify runs the pinned transitive proof chain and checks public shims.

## Project structure and files touched

| Path | Role and permitted change |
|---|---|
| `specs/coverage-selftest/{spec,plan,tasks}.md` | Reconciled requirements, design and paired tasks |
| `docs/decisions/0064-coverage-reservation-tooling.md` | Proposed checked Python prerequisite |
| `bundles/self/scripts/coverage-gate.sh` | Downstream explicit-leaf operation, directly supporting tool/error handling and comments; local selftest/controller |
| `.forge/epic192/issue174-recovery-t1/attributed/` | Retained ignored evidence; never overwrite |
| `.forge/`, `.brokkr/state/` | Ignored receipts, framing and positions; never stage |
| `target/coverage/selftest.XXXXXX` | Existing unique disk-backed root for generated workers, channels and records |

Chief commits exactly the three spec files and proposed decision. Later
implementation may index 0064 in `docs/decisions/README.md` as proposed for
index integrity; this metadata-only exception permits no accepted-record edit.
No separate permanent helper, shell executable, fixture crate or research file.
Inline Python remains pinned. Parent/default allocation, cleanup, preflight,
scanner, application, dependencies, shims/checksums, policies and recipes remain
preserved. #175 is excluded.

## Research and implementation approach

1. **Audit T1 before patching.** Bind retained evidence to f3cee46, recovery
   310b969, initial #174 16841aa and reviewed #173 ff9bf8a. Audit summary,
   context/command/status/output, full `.212/.213`, direct `.25/.26`, and cited
   worker/cleanup traces. Check exact path, command selection, shell options,
   descriptors/traps, outcomes and no removal. Separate kernel, utility and
   resolver results. Keep the ordinary direct pass as comparison. Trace delay
   is investigative only. Fill missing audit fields with bounded checks; do
   not discard attributed failure in favor of a lucky fresh pass.
2. **Direct explicit-leaf operation.** Keep parent derivation/creation and
   requested spelling. Before leaf creation check installed Python and isolated
   stdlib startup. Pass path as one argv element to pinned inline code, never
   interpolate it into code. Call `os.mkdir(path, 0o777)` once, inheriting umask.
   No exist_ok, precheck, retry, marker, canonicalization or alternative path.
   A small script-local helper may own the operation and tool handling; no
   production injection setting or scanner change.

   Check an explicit completion protocol: e.g. `created` with exit 0,
   `oserror:<numeric errno>` with reserved exit 10 for caught OSError; every
   other output/status combination means tool failure. Emit completion only
   after the operation; absent/malformed/extra output refuses. Bash emits
   stable fresh-leaf category/path for OSError and `reservation tool unavailable`
   for missing/broken startup or protocol. Include truthful operation detail,
   not obsolete mkdir status wording. Never classify localized stderr.
   Explicitly check status even under conditional errexit. Assign path and
   ownership 0 only after validated success. Reporting failure after creation
   may retain an unused leaf; refuse without deleting evidence.

   Prove installed capabilities in the candidate and hermetic verifier; keep
   Linux/macOS stdlib compatibility. No auto-install or silent fallback. Test
   representative umasks, relative/space/trailing paths, files/symlinks and
   parent/leaf failures. Adapt old mkdir fault injection to the live boundary
   and witness it. Index 0064 as proposed, never accepted.
3. **One finite two-worker controller.** Keep it inside selftest, shared by
   the race and controls. A local inline stdlib supervisor may launch generated
   Bash workers containing actual canonical function definitions. Private
   pipes/socketpairs avoid blocking FIFO open handshakes; use owned/dynamic
   descriptors. Acknowledge usable receive endpoints, then release checked
   identity-bound tokens only after both live acknowledgements. Both request
   one initially absent exact leaf.

   Use monotonic deadlines and readiness-driven I/O (selectors or equivalent),
   never sleep to establish readiness. The 30-second bound starts before
   controller setup and covers spawn, I/O, resolver descendants, delivery and
   joins; teardown has at most 5 seconds grace. Isolate owned worker sessions/
   process groups. Retain identity until shutdown; never reap/recycle a leader
   then signal its numeric ID. Stop groups before joining leaders and escalate
   only owned groups within grace. Finish watchdogs before root cleanup. Check
   partial setup and writes. Uncontained/unreapable children mean failed proof
   and retained root, never successful timeout. No caller-group signals,
   process-name killing, generic framework or unbounded final wait. Validate
   lifecycle facilities on supported environments; Linux-only support cannot
   silently become the solution.
4. **Attribute outcomes.** Run refuse-capable resolution in a subshell; no
   trailing printf may mask status. Fresh fixed-field NUL-framed records bind
   worker identity, resolver status and actual successful path/ownership.
   Reject truncation, extra fields and stale/mismatched records; never source
   data. Observe delivery and worker completion independently. Require {0,1},
   actual winner exact path/ownership 0 and that status-1 child's own literal
   category/path. Either worker can win; its record determines log attribution.
5. **Prove lifetime.** Neither child deletes the leaf. After results and joins,
   write sentinel bytes, call actual cleanup with observed ownership 0 and
   retry the retained exact path. Require category/path and unchanged sentinel/
   leaf. Only the outer owner removes its root after assertions and all children
   and watchdogs finish. Keep default-cleanup and explicit-failure controls;
   no crash reclamation.
6. **Falsifiable repair.** Through the same controller/assertions, restore only
   the old mkdir-only operation in an isolated canonical resolver definition.
   A selftest-local utility override delegates real creation, accepts only
   actual creation or EEXIST and reports success for each contender. Witness
   both entries/outcomes; unrelated infrastructure failure is not the expected
   red. This models the traced translation without scheduling luck. Require
   0/0 to fail the real proof. Repaired canonical code under that same override
   must reach direct creation and yield one attributable loser, bypassing only
   the leaf override while parent creation stays genuine. An isolated operation
   mutation needs no historical Git objects at runtime; never handwrite a
   substitute resolver or mutate the parent's definition. Separately test
   direct EEXIST, a normal real race and the accumulated canonical selftest.

   Run death-before-readiness, failure-after-release, missing completion,
   missing/malformed records, wrong tokens, unrelated status and wrong-log
   refusal through the actual local code. Include conditional Bash invocation,
   inherited descriptor sentinels, failed result writes and descendant teardown.
   Controls must fail finitely without survivors; child-local counters never
   reset real parent failures.
7. **Preservation/delivery.** Audit diff against 16841aa; byte-compare scanner
   and unaffected non-selftest regions to ff9bf8a, explicitly excluding and
   reviewing only AC8's operation/tooling exception. Revalidate AC1–AC9,
   separate application coverage, every gate and candidate-bound review.

## Data model and contracts

No application schema/API/persistence change. Production state remains exact
resolved path and cleanup ownership. Directory creation is the sole arbitration
event. I/O/reporting failure can leave caller-owned evidence; resolver never
reclaims it. Cooperating callers retain the namespace. Mixed old/new resolvers,
hostile deletion, distributed locking and crash recovery are outside scope.

Fixture identities bind owned process handles, readiness, token, output,
resolver result, delivery and completion. Lifecycle: allocate -> both ready ->
release -> results -> stop/join -> validate -> sentinel/cleanup/retry -> outer
cleanup. Failure enters bounded owned teardown and failed proof. Status 1 is
an expected loser only with completed worker and its own category/path evidence.

## Quickstart and validation

Chief: `git diff --check`, then after commit
`bash bundles/self/scripts/design-check.sh check .` as structural preflight.
Leave this design commit at HEAD for the final pinned sequence check. Neither
check certifies implementation. Downstream run shell syntax, focused controls,
`bash scripts/coverage-gate.sh selftest`, and separately actual coverage with
an unset explicit target or never-created leaf. No reused target or preflight
bypass.

Run the complete sole [house-rule inventory](../../docs/house-rules.md#gates--part-of-every-change)
on the final candidate: fmt, locked clippy/tests, digest-matched bundle compile,
exact coverage, separate selftest, publish-guard proof, keyless seal and
licenses. Require positive integer counts and covered == count in all three
dimensions. Also run pinned design-check selftest, delivery-proof-test.sh,
strict shim digests and fail-closed native-tls absence: nonzero
`cargo tree --locked -i native-tls` proves absence only with the specific
`did not match any packages` diagnostic. Retain command/status, candidate/pin
and output receipts. CI-only obligations keep their charter owner. Verify is
offline and installs nothing. Four actual engine review receipts remain due;
failed/blocked/not-run checks stay outstanding. Chief reports only drafted.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Scope drifts from causal defect | Explicit-leaf operation/support plus selftest only; audit #173 regions and initial #174 diff. |
| Python breaks standalone coverage | Proposed 0064, capability/completion checks, actual verifier proof and named failure controls; accept explicit prerequisite and startup cost. |
| Probe masks operation/report failure | Validate exit plus complete protocol; no success metadata on partial output, retain created leaf on failure. |
| Changed umask/path behavior | argv-only exact spelling, mode 0777 with inherited umask; path/mode and parent/leaf tests. |
| New tool surface | Inline stdlib only, pinned source, no third-party packages/network/compiler bootstrap/fallback. |
| Vacuous causal control | Witness red override and green direct operation, same actual controller/assertions, separate real race/full selftest. |
| Wrong log or error impersonates loser | Separate delivery/completion/outcome, fixed records and loser-local literals; negative controls. |
| Hung I/O or orphaned descendants | 30-second deadline plus at most 5-second grace, private groups, retained identity, checked joins; hard host failure cannot earn green. |
| Descriptor/conditional-errexit damage | Private descriptors, explicit checks, inherited sentinels and conditional tests. |
| Lost caller evidence | Observed ownership, actual cleanup/sentinel/retry, outer cleanup only after joins; no stale recovery. |
| Controls pollute later arms | Fresh records/roots and child-local overrides/counters; never reset parent failures. |
| Overloaded host times out | Accept finite diagnosable failure; no retries buying green. |
| Linux evidence mistaken for all-platform proof | Record limits, preserve Linux/macOS, validate stdlib compatibility; return actual incompatibility. |
| Draft mistaken for delivery | Unchecked tasks until complete evidence; structural check, gates and reviews retain separate owners. |
