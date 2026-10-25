# Sequential eval trials on current main

Date: 2026-10-25. Status: draft. Epic #192 / issue #199.
Base: c13a507; current intake commit: 428a8ec.

Decision: 0076 — [Sequential eval trials](../../docs/decisions/0076-sequential-eval-trials.md).

## Feature and rationale

Port the reviewed sequential-trial behavior from the supplied shipped patches
onto current main. One cooperating trial at a time, a fresh HOME, and explicit
row concurrency replace unsupported claims that concurrent trials are isolated.
This is the operator-selected otherwise branch of #199, not a namespace project.
An operator can run a sequential baseline, see an immediate named refusal when
another cooperating runner owns admission, and retain evidence that honestly
states the supported limit and unconfined execution.

The exact limitation remains: an arbitrary same-uid agent can read another
concurrent trial's shared-TMPDIR prompt, use its MCP/app/identity/inbox context,
complete that task and make its do-nothing victim pass. Private directories and
fresh HOME prevent accidental carryover; they do not confine that uid. Admission
removes the supported overlapping-runner path, not arbitrary local interference.

## Authority and dated integration

The issue packet README, operator-rulings.md, port.md and all three patches under
`.forge/epic192/issue199/` passed their supplied SHA256SUMS. The 2026-10-24 #199
ruling authorizes refusal, fresh HOME and sequential documentation/0055 text.
It accepts neither 0076 nor namespace isolation, which belongs to #221.
The packet's old next-free-number note is superseded by this port's explicit
0076 reservation and current 0077 occupancy for #195.

Both `.brokkr/state/design/positions/simplicity.md` and `robustness.md` were
read. Intake is `.forge/tasks/epic-192-issue-199-port-the-ship-82604c8b.md`;
`.brokkr/state/tasks/` is absent. These are run-local inputs, not committed
artifacts or new verification receipts. Historical SHIP-COMPLETE and
REVIEW-RESIDUAL-OK prove no property of this candidate.

2026-10-25 integration: main already implements required provenance under
proposed [0063](../../docs/decisions/0063-eval-provenance.md), accepted
[0071](../../docs/decisions/0071-eval-trap-scoring.md) trap scoring and business
versions 2, and #225's actual cleanup-entry/Pending probe. The shipped #199
schema-1-to-2 transition is historical. Add concurrency to current v2 rows;
keep all three business versions at 2 and existing artifact envelope versions.
Keep 0063 proposed, 0071 accepted and restored 0076 proposed.

## Panel synthesis and rulings

Both current positions converge on the same boundary. Adopt simplicity's small
private admission module, current RuntimeDirs and cleanup/finalization owners,
bounded worker categories and inert competing-launch proof. Reject a new
scheduler, supervisor, environment framework, generic worker protocol, namespace
or bearer project. Cost: no scheduling fairness, exclusive batch lease, hostile
same-uid confinement or automatic crash recovery.

Adopt robustness's reservation lifetime through actual cessation, drains,
finalization and private-state disposal, including a dropped library waiter on
a surviving runtime. Reject a caller-stack or execute_trial-only reservation,
even though either would reduce changes. Cost: shared ownership, longer refusal
windows and retained resources/admission when cleanup cannot be proved. Neither
current panel actually advocates those shorter lifetimes; this rules out the
shortcut explicitly rather than manufacturing a disagreement between them.

Resolve historical patch versus current main in favor of main: preserve typed
provenance and assigned IDs, v2-only trap semantics and the load-independent
cleanup probe. Reject the old schema bump, old row parsers and startup/polling
substitutes. Cost: deliberate integration and fixture updates instead of a
mechanical patch application. Older v2 rows missing concurrency require their
historical reader; no guessed defaults or v3 migration is introduced.

Choose the bounded inert refusal control over an exploit framework. It proves
the operator's selected boundary with a victim that fails; the exact same-uid
false-pass case remains disclosed. Cost: no claim that hostile local agents
are contained. Keep real protocol and causal controls; serializing production
or using sleeps to hide test races is not an acceptable simplification.

## Requirements

R1. Library and CLI trials share one nonblocking admission boundary per effective
uid and shared host `/tmp`, independent of HOME, TMPDIR, cwd, checkout and
artifact root. Admission precedes trial preparation, prompts and trial children.
Release closes the descriptor without unlinking the stable rendezvous inode.
Unsafe/unavailable admission fails closed. This is advisory runner cooperation.

R2. Default and explicit concurrency 1 are supported. Every other parsed integer
refuses before preparation; 0 and 2 are required controls. Malformed syntax stays
value-free `arguments_invalid`. Parsed trial refusals yield one harness-error
row and exit 2; invalid matrix options yield no rows and exit 2. Matrix remains
a sequential product walk whose worker, not parent, acquires trial admission.

R3. Reservation lifetime encloses current run_trial, target recheck and finalize,
and all existing owned groups, helpers, reapers, drains and private runtime
state. Cancellation, TERM/INT and dropped callers cannot release it early on a
surviving runtime. Unproved cleanup retains live/unknown ownership, reports
harness failure and stops later matrix admission. No duplicate cleanup owner,
renewed budget, retry, lock override or stale-lock reclamation is introduced.

R4. Preserve current fresh private HOME/TMPDIR/XDG role maps, env_clear, native
argv and canary privacy. Multiple agents inside one inbox-isolation trial stay
supported. Fresh HOME must be writable and exclude earlier trial sentinels.

R5. Every emitted or retained TrialResult requires integer `concurrency: 1`.
It records the supported trial limit, including unsupported requests and
never-started rows, not observed overlap or agent count. Shared validation
rejects missing, noninteger and non-1 values with no inferred default. Preserve
all current provenance fields, assigned IDs, seed/time/coordinates, source and
binary identities, availability reasons, retained equality, bounded readers,
closed failure taxonomy, raw statuses and report arithmetic.

R6. Preserve current matrix cancellation/accounting and current v2 scorer.
Admission/cleanup diagnostics use only finite bounded categories, with unknown
worker stderr discarded. A valid row is not proof of producer cleanup. Preserve
committed prefix, active row, zero-time harness-error suffix and cardinality,
no retries, and exit precedence 2 > 1 > 0. Keep infrastructure before deadline
before earliest named trap before goal, all seven traps, exact attribution,
known-Decoy/null-flow behavior, recovered goals and replay/check equality.

R7. README, local issue-#148 AC4 replacement, dated 0055 amendment and linked
local #151 run plan agree on sequential baseline, fresh HOME, row concurrency,
and the exact false-pass limitation above. Distinguish issue-#148's AC4 from
the original harness spec's positive-control AC4. Preserve accepted text as
history and main's #198/#200 additions; explicitly supersede active overlap
requirements. SIGKILL/abort, runtime teardown and host failure can release the
lock while descendants survive: those runs are invalid baseline evidence and
operator-confirmed cleanup is required before resumption. An unlocked file is
not a safe-resume proof.

## Acceptance Criteria

- **AC1 — Admission before work.** Acknowledged active library and independent
  CLI owners cause contenders to emit `concurrent_trials_refused`, a normal
  harness-error row and applicable exit 2, with no preparation/agent marker.
  Changed HOME/TMPDIR/cwd/artifact roots and independent executable locations
  cannot bypass admission. Unsafe owner, mode, symlink, nonregular or hard-link
  rendezvous fixtures yield `trial_admission_unavailable`; stable descriptor/
  pathname identity checks remain. After proved cleanup, a later oracle passes.
- **AC2 — Requests and matrix.** Default/1 work and parsed 0/2 refuse as
  `concurrency_unsupported` through library/trial/matrix paths. Syntax errors
  remain `arguments_invalid`; trial and matrix preserve their respective
  one-row/no-row boundaries. Matrix/trial and matrix/matrix contention refuses
  before competing agent execution. Valid-row-plus-unproved-cleanup stops later
  cells while preserving prefix, active result, full zero-time suffix, raw
  statuses, cardinality and exit precedence. Context/identity mismatches still
  refuse; no parent lease or retry is added.
- **AC3 — Lifecycle.** Dropped library waiter, normal/error completion, TERM and
  INT controls prove contenders refused while acknowledged cleanup is pending,
  through reaping/drains, target recheck, finalization and private-state disposal.
  Release then permits admission. An early-release negative control fails the
  held-reservation assertion. Preserve #225 actual entry/Pending/release events,
  source/build-helper cancellation, fixed budgets and one cleanup owner; no
  sleeps as synchronization or reduced assertions. Unproved cleanup retains
  ownership on a surviving runtime and exercises AC2 suffix refusal.
- **AC4 — Fresh HOME.** Two sequential real helpers record distinct generated,
  writable HOME paths, neither caller HOME; a first-trial sentinel is absent in
  the second. Existing closed-map, native-argv, canary and retained-publication
  controls remain green. The three-agent within-trial task remains valid.
- **AC5 — Composed records.** Pass/fail, setup, retention failure, unsupported,
  overlap, interrupted and never-started rows all carry concurrency 1 and full
  applicable current provenance. Report, worker and retained check readers
  reject absent/noninteger/0/>1 concurrency, v1/mixed and pre-provenance v2 rows.
  Actual retained replay/check preserves equality and trap outcomes; no decoder
  default, business version bump, scorer fork or report formula change occurs.
- **AC6 — Discriminating controls.** Restore
  `sequential_poacher_control_refuses_competing_trial`: acknowledge idle signup
  victim, refuse an inert competing marker agent before execution, causally
  release victim, observe gave_up and no accepted completion, then later oracle
  pass. A temporary test-only admission bypass fails the refusal assertion;
  restore it before final gates. Replace cooperative cross-trial overlap proof;
  all sixteen oracle/null controls, within-trial inbox/foreign-token controls,
  completed external naive-command traps and recovered-goal checks still pass.
  All trial-producing parent scopes coordinate with default Cargo threading.
- **AC7 — Contracts and authority.** A document/link test checks R7, the linked
  run plan and exactly one proposed 0076 index entry. It rejects revived active
  overlap requirements, broken links or status changes. Preserve harness AC4
  positive controls and main's provenance/trap addenda. Record current-base
  schema integration and namespace/hard-death limits without claiming isolation.
- **AC8 — Candidate proof.** Authorized implementation is committed unsigned;
  every local house gate, exact zero-missed root coverage and coverage selftest,
  pinned engine digest/compile, publish/seal proofs, native-tls absence and
  standalone eval fmt/clippy/full default-thread tests pass on that candidate.
  No #194/#196/#198/#200/#225 assertion is weakened or deleted. No root runtime,
  dependency/allowlist, gate or routing change is included. Design-only drafting
  and historical receipts are not implementation acceptance.

## Scope and proof ownership

Preserve frozen 0001 accept/store, 0002 subscribe-before-check, 0003 oldest-only
pruning, 0004 per-test inboxes and 0005 ingest extraction. Sequential eval
admission does not serialize the mail server or weaken its burst/rate proofs.
No network, namespace, new model measurement or external service action belongs
here. CI owns macOS eval evidence through #201, privileged port-80, chart/Docker,
network RustSec and tag-only rate proofs. The operator owns live settings,
external issue updates, sealing and decision acceptance. Current bundles own
routing and protected review; no seat invents those receipts.

The chief commits exactly spec/plan/tasks and proposed 0076. The index addition
and live documentation amendments are implementation tasks; until then the
root decision-inventory test is expected to fail for the missing index row.
This is a disclosed outstanding gate, never a waiver or feature-completion claim.
