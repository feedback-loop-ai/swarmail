# 0076 — Sequential eval trials with explicit concurrency evidence

- **Status:** accepted (operator, 2026-10-26)
- **Date:** 2026-10-25; current-main port synthesis.
- **Issue:** Epic #192 / #199; namespace isolation belongs to #221.
- **Specification:** [Sequential eval trials](../../specs/eval-sequential-trials/spec.md).
- **Amends:** [0055](0055-agent-eval-harness.md) §6, §9 and overlapping-trial proof, within the supplied scoped operator ruling.
- **Preserves:** 0001–0005, accepted [0071](0071-eval-trap-scoring.md), shipped provenance described by proposed [0063](0063-eval-provenance.md), lifecycle/privacy under proposed [0073](0073-matrix-cancellation.md)/[0075](0075-eval-privacy.md), #225 cleanup proof, report formulas and gate/dependency policy.
- **Number:** Restore #199's allocated 0076; 0077 is occupied by #195. Accept nothing here.

## Authority and problem

The verified 2026-10-24 operator ruling says: “Close #199 by refusing concurrent
trials (one trial at a time), a fresh HOME per trial, and documentation and 0055
text saying trials are sequential. Namespace isolation is split into #221; its
decision stays proposed.” The local issue199 packet's README, operator rulings,
port.md and three shipped patches all passed SHA256SUMS. That scoped direction
authorizes this behavior; it does not accept this proposal or any new exception.

Accepted 0055 describes overlapping trials and cooperative success as proof.
Concurrent trials are not isolated: an arbitrary same-uid agent can read a
concurrent trial's shared-TMPDIR prompt, use its MCP/app/identity/inbox context,
complete its task and make its do-nothing victim pass. Fresh private HOME and
TMPDIR prevent inherited state, not access by the same uid. The operator chose
refusal and honest documentation, leaving confinement to #221.

## Dated integration with current main

The shipped #199 decision proposed a result transition from 1 to 2 while #200
was separate. On c13a507, accepted 0071 already requires catalogue/snapshot/
result business versions 2, and #200 already requires typed provenance and
retained replay. That historical schema transition and exclusion of #200 are
superseded here: retain all current versions, add required concurrency to current
v2 rows, refuse historical rows missing it, and introduce neither v3 nor a legacy
reader. Keep artifact envelope versions. Proposed 0063 stays proposed.

Main's #225 actual cleanup-entry/Pending handshake supersedes old startup/polling
probe adjustments. Main's current run_trial target recheck/finalize and current
matrix context/identity validation remain owners. Historical ship/review/gate
receipts do not verify this port. Preserve accepted 0055/0071 bodies as history;
implementation adds dated active integration rather than rewriting acceptance.

## Proposed ruling

1. Admit one cooperating trial per effective uid sharing host `/tmp`, across
   library and independent CLI processes/checkouts. Use stable nonblocking
   admission independent of HOME/TMPDIR/cwd/artifacts, before preparation,
   prompts or trial children. Contention is concurrent_trials_refused; unsafe/
   unavailable rendezvous is trial_admission_unavailable. Keep owner, exact
   permission, type, no-follow, single-link and descriptor/path identity checks;
   close to release and never unlink the stable inode. No waits or retries.
2. Trial/matrix --concurrency and library requests support default/1. Other
   parsed integers refuse as concurrency_unsupported; syntax stays value-free
   arguments_invalid. Parsed trial refusal emits one harness-error row and exit
   2; invalid matrix options retain row-less exit 2. Matrix stays sequential;
   worker owns admission and parent takes no competing reservation.
3. Retain admission through current body completion/join, actual owned cleanup/
   reaping, drains, target recheck, artifact finalization and private-state
   disposal. A dropped library waiter on a surviving runtime cannot release it.
   Keep one cleanup owner, signal subscription before helpers, latched stop,
   fixed budgets and actual observation. Unproved cleanup reports harness error
   and trial_cleanup_unproved, retains live/unknown ownership and stops later
   matrix admission. Preserve prefix/active row/zero-time suffix, raw statuses,
   cardinality, no retries and 2 > 1 > 0 exit precedence. A valid row alone does
   not prove cleanup; bounded finite diagnostics must reach stop_admission.
4. Reuse closed RuntimeDirs HOME/TMPDIR/XDG maps and prove writable fresh HOME
   across sequential real agents with a sentinel absent next time. Keep native
   argv/privacy and multiple agents inside the inbox-isolation trial.
5. Every emitted/retained row requires integer concurrency 1: supported trial
   limit, not observed overlap, requested invalid value or agent count. Include
   all refusals, setup/retention errors, interrupts and never-started rows.
   Preserve full current provenance and shared strict validation through report,
   worker/context and retained readers. Reject missing/noninteger/non-1 values,
   v1/mixed and pre-provenance rows; no defaults, migration, scorer or formula
   changes. Keep current trap precedence, seven traps and replay equality.
6. Replace overlapping cooperative success proof with acknowledged inert
   competing-launch refusal, no contender execution, do-nothing victim gave_up
   without accepted completion and later oracle success. Preserve all oracle/
   null and within-trial controls, newer trap/provenance/cleanup tests and default
   Cargo threading with parent-test-scope coordination. Causal bypass controls
   must fail the admission/lifetime assertion and be removed before final gates.
7. README, local issue-#148 AC4 replacement, linked local #151 run plan and dated
   0055 addendum state sequential baseline, fresh HOME, row concurrency and exact
   false-pass limit. Keep harness positive-control AC4 distinct from issue #148.
   Correct active overlap proof requirements without erasing accepted history.

## Synthesis, rejected alternatives and costs

Both current panels agree on shared advisory admission and current owners.
Adopt simplicity's narrow module/finite diagnostics/inert control, rejecting
new scheduler, supervisor, environment framework and exploit client. Cost:
no fairness or exclusive batch ownership; matrices can interleave between cells.

Adopt robustness's lifecycle ownership and strict composed rows over minimal
caller-stack guards, execute_trial-only locking or optional concurrency. Cost:
more ownership/negative proofs, longer contention and historical readers for
older evidence. These are explicit implementation choices, not claimed panel
disagreements. Reject historical patch behavior that drops current provenance,
changes business versions or substitutes timing for #225 causal evidence.

Advisory admission is not confinement of hostile same-uid processes, old binaries,
other hosts or separate mount namespaces. Such processes can ignore the lock.
SIGKILL/abort, runtime teardown or host failure may release it while descendants
survive. Those runs are invalid baseline evidence; an operator must confirm
cleanup before resuming. No stale-lock recovery or unlocked-file safe-resume
claim is justified. Unproved cleanup can retain admission/resources indefinitely
on a surviving runtime. Accept slower baseline/test throughput and explicit
refusals across cooperating checkouts in exchange for the selected guarantee.

## Delivery and acceptance ownership

Commit exactly this decision and the linked spec/plan/tasks at the chief step.
Implementation T1 adds the unique proposed 0076 index row and dated 0055
integration; T6 aligns all remaining live documents. The temporary missing-index
inventory failure is explicit, not a waived gate. Drafting and structural design
validation are separate from implementation success and full gates.

Run every local house gate, exact coverage/selftest and standalone locked eval
suite using verified candidate-local .forge/bin/brokkr and unchanged routing.
CI owns macOS eval proof via #201 and the charter's network/platform checks;
operator owns live settings, sealing and decisions. Only the operator accepts
0076; the engine alone rules advancement. Commit unsigned; never push or merge.
