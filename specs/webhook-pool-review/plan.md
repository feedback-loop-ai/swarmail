> Historical #195 artifact restored on 2026-10-25. The current port and its
> proof obligations are governed by [webhook-ca-port-lows](../webhook-ca-port-lows/spec.md).
> Earlier stops and gate receipts below are dated history. 0067/0068 are
> accepted; 0069 was accepted for #220 on 2026-10-25 and is not implemented here.
> The low shared-blocking-pool availability residual is tracked by #220.

# Plan — webhook pool review

Implements the proposal-stage scope in [spec.md](spec.md) and proposed
[0069](../../docs/decisions/0069-bounded-ca-execution.md).

## Technical context

One Rust 2024 crate using existing Tokio, axum, reqwest/rustls and rcgen test
fixtures. OpenAPI is served from `src/static/openapi.json`. The existing
`src/webhook/ca_construction_tests.rs` harness owns cold watchdog/cancellation
paths and crate-private seams under 0020. No production change, dependency,
license widening, configuration, database entity or persistent cache is selected.
`.specify/templates/plan-template.md` is absent; the required sections are inline.

## Constitution check

0001–0005, 0020, 0023/0030, accepted 0041/0048, 0045 §8 and 0050/0058 remain
binding. A bounded executor with admission introduces new semantics under the
user's explicit stopping rule. Compatible documentation/evidence work is
allowed; the future runtime change is not. No acceptance of 0067/0068/0069,
accepted-record edits, weakening of guarantees, gate changes or review bypass.
Use real servers, generated certs, fresh stores/inboxes and deterministic
channels; retain exact 100% lines/branches/functions and coverage selftest.

## Research and resolved choices

Both current positions and intake agree GET 500 and capacity/cancellation proofs
already exist. Keep their owning functions/tests; no duplicate implementation.
The generated-key projection must validate the whole reconstructed bundle,
including direct `Webhooks::set` input. Removing this work is not a safe remedy.
A PUT-only cached projection fails to cover unchecked install, config churn and
cold delivery; any raw/view cache would create publication/invalidation duties.

Single shared-pool admission cannot reserve SMTP capacity on one-slot runtimes.
Waiter-only permits are unsound when blocking closures survive cancellation.
API-only limits omit delivery and queues of snapshots; a dedicated unbounded
queue simply moves resource retention. Thread count alone proves neither bytes
nor CPU-latency isolation. The selected proposal requires one separate owner
with finite W/J/B, J = W and no pending queue, without implementation authority.
This accepts earlier burst refusal instead of queue retention. Numeric defaults,
byte-accounting formula and stop/reclamation implementation are explicit
operator-review blockers; complete them before dependent code.

## Project structure and files touched

| Path | Work in this design |
| --- | --- |
| `specs/webhook-pool-review/spec.md` | Replace obsolete implementation requests with current facts, synthesis and acceptance criteria. |
| `specs/webhook-pool-review/plan.md` | Proposal-stage method, authority stop and risk/proof mapping. |
| `specs/webhook-pool-review/tasks.md` | Ordered proof tasks; preserve dated evidence and keep later authorization/proof unchecked. |
| `docs/decisions/0069-bounded-ca-execution.md` | Refine existing proposed direction: no pending queue, explicit unresolved ownership; never acceptance. |
| `.brokkr/state/design/positions/`, `.forge/results/` | Ignored evidence/command receipts and required chief result; never stage. |

Preserved source owners: `src/api.rs` GET/PUT handlers, `src/webhook.rs`
projection/validation/builder/cache/dispatcher, `src/smtp.rs` parse/acceptance,
`src/store.rs` synchronous mutation/offload. Preserve `src/static/openapi.json`,
`tests/api.rs`, `tests/webhook.rs`, the construction harness and
`tests/delivery_recipes.rs`. Accepted decisions, earlier proposals, older spec
families, index, bundle/pins/policy/shims remain untouched in this chief commit.
The proposed 0069 row is already present exactly once; verify its inventory and
status rather than editing the index. The earlier chief preflight was superseded
by the numbering commit. This chief must leave its own four-file commit at HEAD.

## Data model

Installed `WebhookTarget.ca_pem` remains optional raw input; public output is
null for absent input and complete canonical certificates or empty for failed
projection. Async owners alone swap PUT configuration or publish delivery cache
winners. Cache map/LRU share a mutex; preparation remains cache-neutral.

Proposed future execution ownership is described in 0069, not added now: one
facility accounts workers W, outstanding jobs J and charged job-owned bytes B
with J = W and no pending queue, through actual completion and retained-result
disposal, including cancelled running jobs. A job has a single
reservation owner; caller lifetime and running-work lifetime differ. Do not
claim a global heap bound from input bytes. HTTP extractor allocations,
installed configuration and synchronous embedder calls have separate exposure.

Evidence records carry candidate/tree/blob identities, command, exit, log hash
and tool provenance. Keep recovered and fresh command records separate. Retain
count/size/milestone outcomes; omit key-bearing response bodies, private keys
and credentials. Never convert test observations into safety inputs.

## Contracts

Current GET 200/403/500 and PUT error semantics remain unchanged. Fixed GET error:
`webhook response preparation failed`; PUT: `webhook configuration preparation
failed`. No partial replacement, late publication/send or changed terminal totals.
0069's proposed 503 is a future admission contract, not a new meaning for today's
500. Preserve complete-root/mixed-PEM and verified-delivery policies. Never
silently reject input as invalid because capacity is unavailable.

## Implementation approach and stop boundary

1. Verify candidate, recovered hashes, historical residual and current served
   contract; compare source/test blobs with f2fc20b and preserve selector fix.
2. Reuse positions' fresh causal evidence and execute appropriate checks; no
   new stress framework or privacy-leaking mutant. Keep actual builder entry,
   parse-submission, Pending DATA, counts and explicit-release milestones.
3. Correct this spec family and refine proposed 0069, explicitly ruling between positions.
   Review its amendment map, proposed overload semantics, ownership, unresolved
   operator choices and proof obligations. Stop before runtime work.
4. Attempt local house gates and record outcomes without altering index, gates
   or authorities to manufacture green. The prior decision-index failure is
   historical; rerun its proof on the corrected tree. Local attempts are not hermetic verify.
5. Stage exactly four artifacts, unsigned commit with why in its body, then run
   the design preflight against HEAD. Confirm exact paths, clean artifacts,
   unchanged source and declared proposed decision. Write the required ignored
   result through the workspace tool. No later commit.
6. Future work is gated: reconcile live framing, complete numeric/lifecycle
   choices, obtain authorization/acceptance of the exact semantic amendment,
   and only then implement/prove the separate execution owner. Absent that
   authority, stop; this list does not grant implementation permission.

## Quickstart and validation

```bash
cargo test --offline --locked --test api openapi_webhook_get_documents_awaited_preparation_failure
cargo test --offline --locked --lib webhook::ca_construction_tests
cargo test --offline --locked --test webhook
cargo test --offline --locked --test delivery_recipes
```

The construction harness includes live value-free API failures, off-worker and
unlocked construction, winner/bijection, inert cancelled PUT, cancelled delivery
and shutdown accounting. Wire tests cover generated keys, every-root rejection,
later-root TLS and >32-root fan-out. Full workspace tests preserve Store/SMTP/
extraction/wait/prune guarantees and documentation contracts.

Run all commands in the authoritative
[house gate inventory](../../docs/house-rules.md#gates--part-of-every-change),
including exact coverage/selftest, publish/seal proof and licenses; record
fail-closed native-tls absence and shim identities. Validate the brokkr binary
against `bundles/self/tooling.json` before compiling. Install nothing.
After commit, `bash bundles/self/scripts/design-check.sh check .` is a local
preflight only; the sequence's pinned exec owns its final verdict. Full pinned
verify/design/delivery execution and five-office review are separate obligations.
CI-only chart/history/selftest, Docker, RustSec, tag-rate and port-80 bridge
proofs remain unclaimed locally.

## Risks

| Risk | Mitigation and remaining limit |
| --- | --- |
| Proposal mistaken for acceptance | Explicit stop and blocked future tasks; accepted records unchanged; operator alone accepts. |
| Preserved fixes mistaken for security clearance | Retain exact true/low and saturation evidence; only authorized security resolves its finding. |
| Guessed numbers become production policy | W/B defaults, byte formula and lifecycle require explicit choices and measurement; J = W is only proposed. |
| Byte cap silently narrows CA admission | Distinguish resource 503 from invalid-input 400; permanently over-budget embedder snapshots are an unresolved compatibility choice. |
| Unbounded queues or uncharged cancelled jobs | Select J = W, no pending queue; reserve before copies/submission, charge handoff/running/results until disposal, and test cancellation ownership. |
| Starvation or shutdown wedge | Resolve fairness, panic/replacement and stop ownership before implementation; no hard thread-cancellation claim. |
| Synthetic tests overclaim isolation | State OS CPU/memory, persistent-commit and embedder limits; later proof must include real parse and persistent commit. |
| Evidence leaks private material | Preserve sanitized red receipts; never regenerate key-bearing mutant output. |
| Stale index failure or pass misreported | Verify the existing proposed row and record current exits; retain historical failures with their candidate. |
| Historical/donor proof reused as current | Bind command/candidate/tool identities; require final-candidate verifier and exact 0058 offices. |

## Dated clarification — 2026-10-24 run 6086030b

The no-queue choice narrows the proposal, not today's behavior. Reject queue
ordering, queued cancellation and dequeue-race machinery from the selected
future design. A bounded handoff to a reserved worker still needs ownership
proof, and retained results must keep their reservation until disposal; W cannot
be reused early merely because CPU execution finished. Numeric W/B, exact
accounting, oversized inputs, fairness, worker recovery and shutdown ownership
remain unresolved. A safe future implementation requires those choices and the
necessary operator acceptance first. No defaults are delegated to implementation.

## Accepted failure boundaries retained

Adopt robustness's failure analysis as preservation constraints. Persistent
commit and visibility still precede DATA 250; a crash after commit but before
the sender receives 250 can cause retry duplicates. Memory-only mail and
best-effort webhook notifications have no new crash-durability promise. Hangs
at webhook targets keep the existing retries and terminal accounting without
entering SMTP acceptance. Burst pruning remains one ordered Store mutation;
already-admitted payloads retain their own bytes. DATA chaos still refuses
before parse/insert, and cancellation never proves an entered mutation rolled
back. Hostile-body caps, raw-byte fidelity and ingest extraction stay intact.
Root loadability is not future certificate validity; real TLS verification and
per-run generated fixtures remain. Parallel sessions use fresh Stores/inboxes
and the existing shared-cache isolation seam. The full workspace suite is the
preservation proof; the proposed CA facility does not promise universal server
availability or replace any of these boundaries.
