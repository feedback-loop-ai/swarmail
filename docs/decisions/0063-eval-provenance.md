# 0063 — Reproduce public fixture choices and bind eval evidence

- **Status:** proposed — acceptance is the operator's.
- **Date:** 2026-10-24; current-main synthesis 2026-10-25.
- **Issue:** Epic #192 / #200.
- **Specification:** [eval provenance](../../specs/eval-provenance/spec.md).
- **Plan:** [implementation and proofs](../../specs/eval-provenance/plan.md).
- **Supplements:** [0055](0055-agent-eval-harness.md) §§5, 8–10 within the scoped operator direction and accepted [0071](0071-eval-trap-scoring.md).
- **Preserves:** 0001–0005, 0020, 0045 §8, 0050, 0056, current routing and all accepted 0071 semantics.
- **Number:** Restore #200's operator-allocated 0063. Accepted 0071 explicitly preserves this reservation; no next-free number is consumed and no unrelated reservation is reused.

## Context and authority

The local issue-200 packet contains six shipped commits for provenance,
repeatable public fixtures, retained matrix evidence and offline replay,
including final privacy/boundary and cleanup repairs. Current main aec4b46
has accepted #198 trap scoring on top of #194 cancellation and #196 privacy.
The port must integrate into those current owners rather than replacing them
with historical files. All 53 packet checksums and candidate engine bytes
match. Historical ship/gate receipts are not proof of this candidate.

The chief read both current panel positions and intake from .forge/tasks;
.brokkr/state/tasks is absent. All necessary feature evidence is local. This
record restores the existing proposal at its allocated number, accepts nothing,
and introduces no root-runtime or delivery-routing change.

## Dated clarifications and supersession

2026-10-24, recovery: shipped 0063 records this scoped operator direction for
run epic-192-issue-200-recovery-from-89b3d04a:
“amend accepted decision 0055 to allow result schema 2 while preserving a
schema-1 reader for existing records.” It superseded the earlier schema-1-only
design; it was not acceptance of the entire proposal.

2026-10-24, privacy return: unknown identity headers must become canonical
provisioned identity or null at recording. Writer-only membership validation
must not change otherwise valid historical claim scoring. The port keeps this
boundary, including recognized decoys and foreign agents.

2026-10-25, earlier port: current #194/#196 owners govern signals, native argv,
closed environments, retained cleanup and private no-clobber publication.
Patches 04–06 add boundary controls, cleanup acknowledgement and source-helper
signal retention; they are part of shipped behavior, not optional refinements.

2026-10-25, this integration: accepted 0071 now requires catalogue, snapshot and
result business versions 2 and refuses v1 state/results and mixed reporting.
It supersedes the historical schema-1-reader, schema-1-snapshot/catalogue and
business-score-v1 assumptions above. Preserve any restored 0055 addendum only
as explicitly superseded history beside a dated integration explanation; never
reactivate that old reader. Accepted 0055/0071 bodies and statuses stay intact.
Existing proposed 0073/0075 retain status and lifecycle/privacy behavior; their
historical schema-1 wording grants no exception to accepted 0071.

## Proposed ruling

1. Every newly emitted schema-2 trial/matrix row carries required closed typed
   provenance, including handled setup, cancellation, suffix and retention
   errors. Preserve existing execution fields, outcomes, statuses and report
   arithmetic. Missing facts use finite typed unavailable reasons, not omitted
   keys, invented source/model facts or placeholder digests. Strict new readers
   refuse v1/mixed and pre-provenance v2 results. Historical data needs its
   historical binary; no converter, fallback decoder, pooled history or v3.
2. Keep current tasks::score as the sole scorer, catalogue/snapshot/result at
   2, and bind actual v2 definitions/catalogue to business-score-v2. Separate
   artifact/replay/projection envelopes remain version 1. Preserve seven traps,
   infrastructure-before-deadline-before-trap-before-goal precedence, exact
   attribution and earliest match. Known-Decoy/null-flow sibling evidence,
   expiry-only OTP recovery, reset replay and non-target recovery remain.
3. Resolve seed and UTC fixture time once; derive matrix seeds before admission
   from master seed and task occurrence/ID, agent index/label and repetition.
   Use the spec's exact framed SHA-256 semantic choices and independent vectors.
   Equal seed plus time reproduces public difficulty only. Tokens, OTPs,
   passwords, challenge/ticket secrets, admin secrets and invite UID remain
   independent CSPRNG values; IDs/inboxes remain fresh, deadlines/TLS time real.
4. Measure actual executable bytes, embed harness revision/dirty facts at build
   time, bind owned Cargo artifacts to before/after source observations and
   recheck before launch. Dirty-to-dirty edits invalidate association. Opaque
   overrides have measured bytes and explicit unknown source unless identical
   to an already known owned artifact. Unknown source can run; unreadability,
   replacement or contradicted identity fails the affected cell. No runtime
   HEAD attribution, remote attestation or hostile check-to-exec guarantee.
5. Model/config are explicit declarations. Hash original framed argv only after
   public opt-in; private argv is unavailable/private_argv, not automatically
   hashed. Keep literal native OsString execution and caller-context resolution.
   Public UTF-8 format refuses non-UTF-8 opt-in value-free. Keep closed worker,
   preparation and runtime maps; no environment forwarding, raw errors/logs,
   paths, panic payloads or private fingerprints in public evidence.
6. Canonicalize claims at shared recording, retaining every provisioned role,
   including Decoy and foreign Agent. Unknown values become null without
   changing HTTP status, action, flow, category or ordering. A small writer-only
   membership guard runs before admin or retained payload publication; it never
   enters reader/scorer validation. Compatible historical v2 unknown claims
   keep their meaning; successful replay certifies no historical privacy.
7. Retention is part of infrastructure success. Share exclusive 0700 leaf and
   descriptor-private 0600 temporary-before-payload, complete write/no-clobber
   publication and ownership-only cleanup. Finalize once after cleanup, metadata
   last. Parent owns plan/index; child owns trial leaf. Fallback is post-reap
   and only without child-owned output. Preserve unowned bytes/modes and leave
   partial/conflicting evidence incomplete, with exit 2 and no false marker.
8. Preserve sequential matrix product order/cardinality, unique fresh IDs,
   committed prefix, post-cleanup active row, zero-time refused suffix, raw
   statuses, no retries and 2 > 1 > 0 exits. Bound/drain child output at 64 KiB,
   require exactly one matching row/exit and bind retained equality after the
   existing cancellation arbitration. Divergent immutable child evidence stays
   untouched and prevents matrix.json. Index failure changes exit, never rows.
9. Keep one cleanup owner: subscription before any helper, latched admission,
   once-only forwarding, owned source/Cargo tasks, body abort/join, retained
   groups/reapers/drains, actual direct waits and observation. Preserve concurrent
   five-second TERM grace then five-second post-KILL observation. Matrix envelope
   expiry requests stop through that owner; it cannot reset/bypass cleanup.
   Signals during source-observation failure/cleanup prevent later Cargo launch.
10. Retained check validates original bytes, all versions/hashes/lengths/semantic
    associations/coordinates/seed derivations, fixed safe paths and all bounds
    before any replay row. Then call the current scorer. Never execute saved
    commands, require original binaries/services or mutate evidence. Original
    budget/infra result remains separate; missing/ineligible state is unscorable.
    Invalid later matrix cells produce no rows. Hashes establish consistency,
    not authenticity or durable multi-file transactions.

## Synthesis and explicit losses

Adopt simplicity's existing owners and narrow shipped modules. Reject new
supervisors, context files, startup/commit handshakes, generic replay frameworks,
resume/retry, root changes and namespace work. Cost: incomplete directories
remain manual evidence and no stronger confinement or durability is promised.

Adopt robustness's strict typed records, binding/bounds, repeated hashing and
causal negative controls over optional-field bags or best-effort retention.
Cost: more validation code and permanent boundary proofs. Both panels agree
on v2-only scoring and preservation of every #194/#196/#198 assertion.

Resolve cancellation against immutable retention by returning infrastructure
failure without rewriting child evidence or committed stdout. This takes the
robustness position while accepting simplicity's refusal of another handshake.
Cost: a handled race can leave complete stdout but incomplete retained evidence.

Reject automatic private-argv hashes and inferred source/model identity. Cost:
some rows have weaker attribution, stated explicitly. Reject global seeded
randomness. Cost: fresh secrets/wire bytes and nondeterministic agent outcomes.
Reject historical reader membership validation and broad refused-identity
redaction. Cost: old v2 files may remain sensitive; new unknown spelling is lost.
Reject active v1 compatibility under accepted 0071; historical replay costs a
historical binary. No panel preference can weaken an accepted guarantee.

## Proof and delivery

Spec AC1–AC13 and paired tasks require strict vectors/availability, full v2
trap composition through actual replay, live privacy canaries, an eight-cell
retained matrix, all at/over-limit and bad-later-cell controls, both-signal
cleanup acknowledgement/Pending/release/bypass and source-failure cancellation.
The plan preserves the named baseline inventory. No mock protocol, sleep-as-sync,
assertion deletion or weakened bound is authorized to make the port green.

Run every house gate with candidate-local pinned .forge/bin/brokkr, exact
coverage/selftest, publish/seal proofs, native-tls absence and standalone locked
eval fmt/clippy/tests. Current bundles govern protected review/security hold.
CI owns macOS eval proof through #201 and the charter's CI-only checks; the
operator owns live settings/sealing. Namespace work belongs to #221. These
are ownership records, not added local prerequisites or claims of execution.

The chief commits exactly this proposed record and spec/plan/tasks. The missing
single proposed 0063 index row and dated integration belong to implementation
T1; root inventory failure until then is explicit and not waived. A drafted
result and structural design check are not implementation/full-gate success.
Only the operator accepts decisions; the engine alone advances the sequence.
