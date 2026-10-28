# Independent rate and subprocess proof

Created: 2026-10-28. Status: draft design. Epic #164, issue #152.
Baseline: `7f2e2129e36577f4c65703c70e20d35a484e6baf`, tree
`46fbd52c08d419e4c0fd8bbfa92bdc78285759f6`.
Run: `follow-up-epic-164-issue-152-on--ed4c6918`.

## Feature and rationale

Repair two independently observed gaps in the candidate's proof. The deterministic
rate test derives its expected count and interval from production constants, so a
changed numerical contract could remain green. The C6 malformed-reply tests use
real TCP but run the bridge in-process, leaving the promised executable witness
unproved. Add independent literal expectations and actual subprocess evidence in
the existing test owners. Production behavior remains unchanged.

Both current simplicity and robustness positions were read. The requested
`.brokkr/state/tasks/` directory is absent; framing comes from the supplied task
and intake `.forge/results/7f2087d1-c77c-47fd-ab8d-f8cb2e959ab0.json`.
The chief checked both supplied manifests with `sha256sum --check --strict`:

| Evidence | SHA-256 | Attribution |
| --- | --- | --- |
| `.forge/evidence/issue152-options-review-stop/review-event.json` | `f0e5f8b484967490f7e90dbbea7e37a3102b0cf70f0f1b1a92e41420bba27b44` | Run 88f22317, this baseline; HIGH security-relevant integrity concern and MEDIUM rate-proof weakness. |
| `.forge/evidence/issue152-mixed-review-park/panel-event.json` | `a5639682f6f1e34cc458ed8d276def28d55339e867b68e3815df24ef9467447c` | Different candidate aece916; indeterminate panel, tests completion unestablished; a member identifies missing C6 subprocess evidence. |

Checksums establish agreement with supplied bytes, not review authenticity. Current
source independently confirms `ToolRate::CAP/WINDOW` in the numerical oracle and
`tests/stdio.rs::Running::spawn` calling `swarmail::stdio::bridge` in a task. No
cross-tree verdict or prior gate pass is transferred. The inherited HIGH remains
OPEN, security-relevant and blocking until independently resolved. A completed
panel, this design, checksums and green application gates cannot resolve it.

This is a proof-only successor to [mcp-engine-followup](../mcp-engine-followup/spec.md).
All its AC1–AC11 remain required, including the original issue's client matrix.
No public contract, frozen guarantee or ruling is changed; no new ADR is needed.
Existing proposed [0079](../../docs/decisions/0079-mcp-version-negotiation.md),
[0080](../../docs/decisions/0080-mcp-source-reconciliation.md),
[0090](../../docs/decisions/0090-mcp-options-admission.md) and
[0092](../../docs/decisions/0092-mcp-candidate-proof.md) retain their meaning and
status. The historical specs and reconciliation ledger remain unmodified. An
unexpected semantic need requires scope reconciliation and, only if necessary,
the then-next free proposed decision; only the operator accepts it, and accepted
ruling conflicts wait under [0045 §8](../../docs/decisions/0045-delivery-repairs.md).

## Panel adjudication

The positions agree on scope and evidence honesty. Do not invent a conflict over
full gates, real protocols or the integrity hold. Their differing minimum proof
and helper preferences are resolved explicitly:

| Question | Ruling | Losing alternative and cost |
| --- | --- | --- |
| Numerical oracle | Adopt both positions' literal 1024 and one-second boundaries; require robustness's complete second-window refill and next refusal. | Simplicity's two post-boundary admissions alone lose: slightly more assertions buy proof against partial refill. |
| Refusal/router setup | Adopt simplicity's literal exhausted count and elapsed-second setup in both existing witnesses. | Robustness permits constants there once the owner proof is independent; reject that permission for these contract-bearing checks. Intentional duplication costs maintenance but prevents adaptive expectations. |
| Subprocess owner | Adopt simplicity's `tests/binary.rs` and its existing `McpChild` helpers. Apply robustness's bounded cleanup and full peer accounting. | Do not extend the `tests/mcp_revisions.rs` Binary helper with its unbounded pre-wait stdout drain or add a second supervisor. The binary suite gains a small local peer and several launches. |
| Initialization controls | Require robustness's valid recovery and admitted operation after each malformed initialization, in addition to simplicity's unversioned ping. | A ping-only positive control is insufficient for operation recovery; extra protocol exchanges cost test time. |
| General hardening | Adopt both positions' existing-owner, test-only approach; retain robustness's failure catalogue as regression-selection guidance. | No new runtime guard, clock framework, fairness map, envelope validator, fixture framework, dependency or review-engine repair. Existing runtime limitations remain. |
| Evidence disposition | Adopt both positions' separate final-candidate proofs and external ownership. | Historical receipts, a compiled roster, panel completion or a private eval lock cannot substitute; correct repairs may still leave delivery blocked. |

## Acceptance Criteria

These are downstream obligations. `drafted` certifies only the committed design;
it does not certify implementation, gates, protected review or delivery completion.

1. **AC1 — Independent numeric boundary.** On the real `ToolRate`, a fresh generated
   monotonic instant t and an empty window admit exactly 1024 calls at t. Attempt
   1025 at t refuses immediately. An attempt at t + 999,999,999 ns refuses; at
   t + exactly one second it succeeds. At that same boundary, the remaining 1023
   calls succeed and the next refuses. Expected counts and instants use test-owned
   literals, never production CAP/WINDOW or indirect aliases. Refusals neither
   shift the window nor create capacity. A smaller/larger capacity, shorter/longer
   window, strict-greater boundary or partial refill must fail behavioral assertions.
   No sleep, timed HTTP exhaustion loop or constant/string-only assertion suffices.
2. **AC2 — Existing ownership/effect proofs.** Retain and execute the deterministic
   refusal-before-effects and production-router real-connection witnesses. Use
   literal 1024 exhaustion/count expectations and a literal one-second elapsed
   setup, preserving the future-start seam. Preserve shared Arc ownership across
   fresh connections, all three revisions, invalid arguments, notifications and
   March batch members; charge before staging. Mutation/subscription observers
   must stay zero under refusal and fire under admitted positive controls; chaos
   stays unchanged. Control/discovery remain available. Preserve existing batch
   and binary wait/seed/ping concurrency tests; sequential sharing is insufficient.
3. **AC3 — Binary malformed initialization.** For each complete HTTP-200 envelope
   defect (missing `jsonrpc`; simultaneous `result` and `error`), launch the actual
   Cargo-built `swarmail mcp` with piped stdin/stdout against a real loopback TCP
   peer. Use matching IDs and otherwise valid supported revision, capabilities
   and serverInfo. Hold initialization while queuing meaningful tool/discovery
   requests and an effectful notification, then release it. Assert the entire
   response JSON value reaches stdout unchanged. Requests refuse locally, effectful
   notifications remain silent, and none reaches the peer. A subsequent ping
   reaches the peer without an installed revision header. A fresh valid modern
   initialize/initialized sequence then admits an operation with the negotiated
   header, proving recovery and live input/observation.
4. **AC4 — Binary ordinary exchanges and lifecycle.** For both envelope defects,
   prove unchanged JSON-value forwarding in implicit March and negotiated modern
   operation, checking actual version headers. Retain existing June/November,
   legacy and in-process coverage. Assert one line per owed response, no extra
   notification line, EOF drain and successful child exit. Negative peer evidence
   is a complete trace through child drain/exit and joined peer work, not a short
   timeout, immediate empty-channel poll or first ping alone. Respond to unexpected
   traffic so a regression reaches an assertion. Every wait and cleanup is bounded;
   failure kills/reaps the child and joins or aborts owned peer tasks. No whitespace
   or object-key byte-order preservation contract is introduced.
5. **AC5 — Preserve issue #152 and guarantees.** All predecessor AC1–AC11 apply:
   guard-first OPTIONS 405/Allow/no-CORS, other method/header and POST precedence,
   three revisions, immutable exchange revisions and lifecycle fences, modern-array
   pre-effect refusal, March concurrent members/reply order, exact independent
   thirteen-tool catalogue (seven reads/six mutations), served descriptions and
   source attribution. Keep C1–C6 and the separate OPTIONS MUST mismatch explicit;
   no full-conformance claim or C6 rejection repair. Preserve synchronous storage
   before SMTP 250, subscribe-before-scan/re-entry, oldest-only pruning, isolated
   inboxes and ingest extraction. Lazy report analysis, persistence/shutdown and
   best-effort webhooks retain existing ownership. No accepted decision or historical
   spec is rewritten; proposed 0079/0080/0081/0085/0090–0092 stay proposed.
6. **AC6 — Fresh final-candidate verification.** Record commit/tree, clean state,
   executed commands, outcomes and executable identities. Run the entire current
   [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change),
   including locked builds/tests, exact zero missed lines/branches/functions,
   coverage selftest, publish/seal proofs and licenses. Keep pinned transitive
   proof execution, checked shims, fail-closed native-tls absence, focused suites,
   design-check/selftest and authorized delivery proof. Filters must execute a
   nonzero intended count. Verify the exact absolute engine against tooling.json
   and digest `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`
   before self/mixed compiles. No install, pin substitution or historical-pass reuse.
   Any later tracked edit requires fresh successor-bound verification and review.
7. **AC7 — Protected review and unresolved HIGH.** Retain Terra 6.1 intake, Opus
   5.5 implementation and five separately authenticated, final-candidate protected
   results: Astra correctness, official DSH Flash security, Spark GLM Flash tests,
   Astra guarantees and independent Fable. Keep distinct invocation identities,
   read-only boundaries, exact providers, Spark effort not applicable, singleton
   chains/no unauthorized fallback, worst verdict, unknown-fact refusal and hard
   security holds. Two Astra roles are not separate model families. Keep the
   inherited HIGH security-relevant protected-review-integrity finding open until
   independently and legitimately resolved; do not reclassify it, reproduce attacks,
   bypass boundaries or fabricate verdicts. Panel completion is not resolution.
   Unresolved HIGH/security or untrustworthy review blocks delivery completion.
8. **AC8 — External obligations remain separate.** Full standalone eval requires
   actual host-wide nonoverlap through cleanup and exact executable binding under
   accepted 0063/0076; otherwise record unavailable/not run. No private lock or
   isolated worktree supplies this proof. Preserve all four owed real-client receipts
   (Claude Code, Codex, MCP Inspector, Claude Desktop), #151 baseline/merge hold,
   #152-before-#153 sequencing, CI platform/network/Helm/Docker/RustSec/port-80 and
   release proofs, operator decision numbering/acceptance and seal. No paid trials,
   push, merge, publication, tag, issue closure or decision acceptance is authorized.

## Retained costs and limits

The shared per-router fixed window retains contention, adjacent-window bursts,
restart allowance and independent routers' allowances; no fairness or rolling limit
is promised. C6 forwarding remains a compatibility deviation. A committed mutation
can outlive a lost reply; retries may duplicate. Webhooks remain best effort, not a
durable outbox. Memory-only mail disappears on process death; persistent restart
reloads rather than reconciles histories. Caps do not bound arbitrary inbox counts
or held Arcs. TLS validation and per-run certificate fixtures remain intact. This
proof adds no global resource, exactly-once or new failure-recovery guarantee.
