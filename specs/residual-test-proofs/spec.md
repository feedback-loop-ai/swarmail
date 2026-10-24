# Issue #191 residual test proofs

Status: Drafted design; implementation and verification remain pending.
Date: 2026-10-24.
Scope: Epic #192, issue #191 findings 23.2–23.5 only.
Baseline: integrated #179 rollup `34eb99f20d102914a7c9d53ed6b701ba90a1af7d`.

## Feature and rationale

Make the existing test helpers accurately documented and make two existing
regressions reject the defects they name. The refused-target fixture releases
its port before delivery, allowing another test to take it. The cumulative
scrape test checks an idle map's output but would accept separate locks for
the total and rows. Neither finding demonstrates broken production behavior;
both leave a hole in the evidence protecting existing behavior.

The retained implementation touches only helper documentation and the unused
wrapper in `tests/common/mod.rs`, the refusal fixture in `tests/webhook.rs`,
and the existing inline snapshot test in `src/api.rs`. Finding 23.1 is already
landed. Preserve its implementation and the Dockerfile AC52.2 correction
verbatim. No production API/runtime change, dependency, allowlist, gate,
policy, accepted-decision edit or broader cleanup belongs to this feature.

## Authority and classification

The intake classification is `engine`: socket lifetime and snapshot locking
are concurrency concerns under [0057 §7](../../docs/decisions/0057-codex-delivery-recipes.md).
Retain the policy-selected full lane and four independent review lenses;
the conditional chore-only single official DeepSeek Flash route does not
apply. Do not expand review beyond policy, substitute DeepSeek Pro, or use a
fallback. This design does not select the engine's next phase.

Preserve synchronous accept/store (0001), subscribe-before-check (0002),
oldest-only pruning (0003), isolated real-protocol tests (0004), and extraction
at ingest (0005). The snapshot authority is accepted
[0047 §2](../../docs/decisions/0047-storage-rpc.md); webhook delivery remains
independent of SMTP acceptance under [0041](../../docs/decisions/0041-webhook-boundary.md).
Follow [0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md)
for test placement and [0050](../../docs/decisions/0050-delivery-gates.md) for
proof ownership. No semantic amendment is proposed: this work changes the
proofs of existing contracts, not a frozen guarantee, public contract or
ruling. Consequently there is no new decision record.

## Panel synthesis and rulings

Read both run-local positions, `simplicity.md` and `robustness.md`, alongside
the intake under `.brokkr/state/tasks/`. These are evidence, not deliverables.

1. **Adopt both positions' narrow helper repair.** Move each description to
   its actual declaration and delete only the confirmed unused wrapper.
   Adopt robustness's explicit preservation of attacker-spelled Host/Origin
   bytes and baseline-before-trigger observation. Reject client consolidation,
   input normalization and generic polling redesign; their added scope would
   obscure the security and timing observations these helpers serve.
2. **Adopt both positions' owned non-listening socket.** Use existing Tokio
   support, explicitly disable address reuse, and retain the socket through
   the final mail assertion. Adopt robustness's mandatory bounded real refusal
   probe. Reject retaining a listener, bind-and-drop, fixed ports and timing
   retries: those either change the failure to TLS/read timeout or leave the
   ownership race. Cost: one held descriptor and one local connection probe.
   Simplicity's optional second-bind probe is not required; explicit ownership,
   reuse policy and the refusal probe are the scoped proof, not a guarantee
   against privileged socket interference.
3. **Adopt the complete-function source guard plus behavioral assertion.**
   Both positions reject a probabilistic writer and a bare `.lock()` count.
   Rule for robustness's stricter comparison: permit only line-ending and
   outer-whitespace normalization, not arbitrary internal whitespace removal.
   Simplicity's more tolerant normalization loses; the cost is intentional
   failure on benign internal formatting changes, requiring fixture review.
   Extract only the actual production definition, before the inline test
   module, so the embedded expected string cannot satisfy its own test.
4. **Require current-pass / split-lock-fail / restored-pass evidence.** Adopt
   robustness's requirement that the temporary mutant compile and fail the
   new assertion while its quiescent output remains identical. Reject runtime
   mutex wrappers, callbacks, traits, production hooks and a generalized Rust
   scanner. Their extra owners and coverage obligations exceed this bounded
   repair. The accepted loss is that a source guard pins this tiny function;
   it does not measure runtime acquisitions or prove arbitrary helper internals.
   Complete-body comparison rejects replacing the direct operations with
   delegated reads, an early drop, or an additional lock.
5. **Preserve the robustness boundary analysis without expanding the work.**
   Storage durability, crashes during retries, pruning, DATA chaos, hostile
   bytes and certificate lifetime retain their existing contracts. They do not
   justify a new outbox, exactly-once claim, runtime lock change or TLS bypass.
   Sequential mail checks establish queryability before/after the exhaustion
   observation; they do not establish that SMTP beat retry exhaustion in time.

## Acceptance Criteria

- **AC1 — 23.2, truthful documentation.** `http_send_text` owns the raw-text
  paragraph formerly attached to `HELPER_DEADLINE`; the constant retains its
  deadline contract. `http_send_text_with_headers` owns the extra-header/Origin
  paragraph. `send_with_host` has one description of its mandatory supplied
  Host spelling, sent verbatim, with no optional/default claim or duplicate.
  Inspection and existing Host/Origin/deadline tests show identical helper
  signatures, request bytes and deadline behavior.
- **AC2 — 23.3, conditional removal.** Immediately before removal, an exact
  identifier search across repository source, imports/re-exports and non-Rust
  references finds no caller of `wait_for_metric`. Record the search and
  distinguish prose references (including this design) from executable uses.
  Only that wrapper and its attached prose are removed. `metric_now`,
  `wait_for_metric_from`, `wait_for_metric_row` and their callers remain.
  A discovered caller prevents deletion and requires scope reconciliation;
  do not silently migrate its baseline semantics. Full tests compile and pass.
- **AC3 — 23.4, real refusal with continuous ownership.** The test binds a
  non-listening IPv4 socket to `127.0.0.1:0`, with reuse disabled, and holds it
  through the final stored-mail assertion. A deadline-bounded real connect
  attempt while held returns `ErrorKind::ConnectionRefused`; success, timeout
  and other errors fail. No `listen`, connect on the reservation socket,
  early drop, sleep synchronization or unreserved port fallback is allowed.
  Execute the regression on supported Linux and macOS; record each platform's
  actual result, and leave unavailable platform proof explicitly outstanding.
- **AC4 — preserve the webhook observation.** Keep target PUT, pre-trigger
  exhaustion baseline, SMTP send, immediate stored-mail count, eventual one-new
  exhaustion and final stored-mail count in their existing order. Both counts
  remain exactly one. Release the reservation explicitly after the final check;
  unwind cleanup uses RAII. The probe does not increment the webhook ledger.
- **AC5 — 23.5, discriminating snapshot proof.** The existing inline test keeps
  its local registry and exact output: total six, LF overflow two, `a` two,
  `b` one and `c` one. It additionally pins the complete current
  `cumulative_rows` definition to one held guard used for summing and rendering.
  Missing extraction anchors fail visibly. Current code passes; a compiling
  mutation that sums, releases the guard, then reacquires to render fails this
  specific source assertion even though the output assertion passes. Restore
  the exact production definition and demonstrate the test passing again.
- **AC6 — unchanged runtime and preservation.** The final implementation diff
  is confined to the three named files and their stated test/docs regions.
  The production `cumulative_rows` and all other runtime code match baseline;
  Dockerfile is byte-identical to baseline, including AC52.2. Finding 23.1,
  Cargo files, accepted decisions, gates and policy are unchanged. Keep the
  branch-stable snapshot assertions inline and measured; no coverage exclusion.
- **AC7 — complete verification and delivery discipline.** Before implementation
  completion, execute every gate in [the sole house inventory](../../docs/house-rules.md#gates--part-of-every-change),
  including exact 100% lines/branches/functions, coverage selftest, publish and
  seal proofs, pinned-engine bundle compile and fail-closed native-tls absence.
  Follow pinned proof ownership, retain required locked builds, and distinguish
  CI-only proofs from local results. Preserve protected review/security hold.
  Author commits are unsigned; never push, merge, tag or publish.

## Design handoff

The chief commit contains exactly this spec, [plan](plan.md) and
[tasks](tasks.md), with one spec directory touched at HEAD. Position files stay
ignored. The deterministic design check rules on that commit. A drafted design
or passing structural check is not evidence that AC1–AC7 are implemented.
