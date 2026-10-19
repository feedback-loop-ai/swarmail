# 0055 — Prove the agent eval harness before measuring agents

- **Status:** accepted (operator, 2026-10-19)
  Original draft status: proposed — acceptance is the operator's.
- **Date:** 2026-10-19.
- **Issues:** epic #164; #148, #149, #150, Phase 0 run A.
- **Specification:** [agent eval harness](../../specs/agent-eval-harness/spec.md).
- **Preserves:** 0001 synchronous accept/store, 0002 subscribe-before-check,
  0003 oldest-only pruning, 0004 AUTH inbox routing, 0005 ingest extraction,
  and the root gate/dependency policies.
- **Number reservation:** 0049–0054 belong to epic #120's open batches.

## Context and operator rulings

The epic's rule is that the harness must be proven able to fail before it
measures today's main. Oracle and null controls must traverse the same
agent-command path later used by models; app state determines success.
The two panel positions are run-local evidence; their adopted and rejected
arguments are preserved in this decision and the linked spec.

The operator ruled on 2026-10-18 that the harness is built on **brokkr**:
eventually each trial is an eval recipe with an agent seat and exec scoring
gate, with journal-owned tokens, turns and cost. Until brokkr's MCP broker
can grant the seat Swarmail's MCP server (the broker/URL-connection gaps
named in #148), an interim opaque driver owns the servers and invokes the
real brokkr driver as its child. That execution is **unconfined**, and every
such result must say so. This run builds the agnostic runner the next run's
recipe/driver wraps; it does not build either, generate MCP configuration,
file fictional broker issue IDs, or run model measurements.

The operator also ruled on 2026-10-18 that `evals/` is a **standalone crate**
with its own `[workspace]` and Cargo.lock, matching tools/coverage-fn-scan.
The earlier requirement that root workspace tests execute evals contradicted
the root coverage boundary and is superseded. Root Cargo.toml/Cargo.lock
gain nothing, the publication allowlist stays unchanged, and every evals
dependency at every depth reuses a root-locked version, resolved offline.

CI and verify wiring is deliberately deferred until epic #120 batch 5's
gate changes land. **Until then, evals is proven by its own suite run by
the implement seat and reproduced by reviewers.** Its fmt, clippy and
oracle/null/isolation tests are mandatory independent evidence. Root gates
remain unchanged and green; root green alone says nothing about evals.

## Proposed ruling

1. One unpublished crate/executable owns fixture, typed tasks, oracle/null,
   trial lifecycle, snapshot check, matrix and report. The fixed schema-v1
   catalogue has the spec's 15 tasks in three tiers plus one isolation task.
   Each declares id, tier, target `failure_class`, `idle_class`, templated
   goal, scenario, deterministic app-state predicate, agent count and bound.
   Templates include app/MCP URLs and identity/inbox; unknown values refuse.
2. Keep the closed failure enum exactly `wrong_message`, `stale_code`,
   `wrong_link`, `wait_timeout`, `gave_up`, `tool_misuse`, `budget`,
   `harness_error`. Targeted class and idle class have distinct meanings.
   Categorized app rejection events identify observable wrong actions;
   do-nothing failures use declared gave_up/wait_timeout, not a fictional
   stale-code submission. Infrastructure overrides deadline, deadline
   overrides business state, otherwise a satisfied predicate passes even
   after nonzero agent exit. Failed state takes the first relevant rejected
   action, then idle fallback. Full precedence is normative in the spec.
3. App state is the sole business-scoring authority. The fixture sends real
   SMTP, records final DATA acceptance, binds tokens to identity/flow, and
   never permits an agent to set a success flag. Separate bearer-admin
   routes expose redacted, read-only per-flow state. A frozen, versioned,
   secret-free snapshot drives both the runner's pure check and the future
   exec scoring gate. No mailbox, transcript or agent account is consulted.
4. Exercise all eight requested flows. For invites, send a real ICS
   attachment and offer identical bytes through the emailed invitation's
   token-authorized app page; confirm UID/time on acceptance. Support uses
   an emailed challenge with a public fixture HTTP reply. These are
   explicitly **not** MCP attachment retrieval or Swarmail inbound reply.
   Leaving support unanswerable cannot coexist with all-oracle-pass proof;
   this design chooses an answerable, honestly scoped task.
5. Determinism uses OTP supersession, generated dates, a slow-mail release
   after an instructed initial wait, and observed 100% DATA chaos refusal
   followed by fixture retry after clear, before agent launch. App-only
   state does not prove an external agent actually performed that first
   wait. No timer-only scheduling claim, mid-band chaos, sleep-based proof,
   or shared-server chaos. No TLS, persistence or pruning coverage claim.
6. Each trial owns fresh temporary state, in-memory Swarmail and fixture,
   all bound to loopback port zero with announced addresses and protocol
   readiness. Resolve SWARMAIL_BIN or build the root binary once offline/
   locked, never once per proof case. Swarmail receives sanitized relevant
   environment. SMTP AUTH selects distinct inboxes; isolation's three
   simultaneous agents share one server **inside** a trial, while two
   concurrent trials own two servers and six agents.
7. External agents are a label and literal program/argv after `--`. The
   environment contract is EVAL_TASK_ID, EVAL_APP_URL, EVAL_MCP_URL (ending
   `/mcp`), EVAL_IDENTITY, EVAL_INBOX, EVAL_GOAL, EVAL_PROMPT_FILE and
   EVAL_DEADLINE_MS. The prompt file and EVAL_GOAL are identical; no admin
   inputs are exported. Every app request names its identity in the
   `X-Eval-Identity` header, so a presentation of another identity's token
   is observable as `wrong_message` rather than silently completing it.
   Oracle and null use this exact subprocess path.
   Oracle uses public app HTTP and current MCP 2025-03-26 text-block data,
   never REST mailbox reads, seeds or private state.
8. Runner owns Unix process groups, bounded readiness/I/O/agent/shutdown
   deadlines, log draining, server-death detection and terminate/kill/reap
   cleanup. Snapshot before server shutdown; score after cleanup. Handled
   errors and interruption emit one record; SIGKILL/abort or unavailable
   stdout cannot. Daemonizing commands are unsupported. This is lifecycle
   ownership, not confinement against hostile local processes.
9. Result schema 1 emits task, agent, outcome pass/fail/harness_error,
   nullable failure_class, wall_time_ms, real agent_exit_status, per-child
   agent_exits, trial_id and `confinement:"unconfined"`, including built-ins.
   Isolation has no invented aggregate command status: the singular field
   is null and the three raw statuses remain in agent_exits. Trial exits
   0/1/2 respectively. Optional artifacts retain state/result/metadata for
   later scoring. Matrix emits one row per requested product element,
   including failures, without automatic retries.
10. Report counts attempts and infrastructure errors explicitly, computes
    success over pass+fail with a 95% Wilson interval and sample size,
    also prints passes/attempts, nearest-rank median/p95 non-error wall
    times and closed failure counts. Empty estimates are null. No arbitrary
    error threshold, token/turn inference, or published model claim.

## Alternatives, explicit losses and proof

Adopt simplicity's single crate/executable, fixed catalogue, minimal wire
templates and sequential matrix over an extensible harness platform. Cost:
new tasks require code review and v1 process support is Unix. Reject a
third public stall agent; test helper commands prove deadline and traps.

Adopt robustness's admin separation, snapshots, real child seam, process
groups, protocol readiness and event-grounded classification. Reject the
simpler rule that every unsatisfied task gets its targeted trap class.
Cost: more fixture state and tests and a second listener; null failure
counts now describe inactivity honestly. Admin separation is only defense
against accidental access on an unconfined host; it is not authentication
of an adversarial model's filesystem/process access.

Adopt simplicity's reachable ICS and support flows over robustness's
unanswerable support stub. Cost: these tasks do not establish the need or
performance of #159's attachment API or #160's inbound target by themselves.
Future attachment-only/reply-transport tasks must have distinct IDs and
baseline labels. Reject timer-only slow delivery, builtin-as-confinement
labels and unconditional output-after-panic/kill claims for the reasons
stated in the spec; retain their limits in README and result interpretation.

Proof requires every task with oracle passing, every task with null failing
with its declared idle class, deliberate wrong actions failing in their
observable classes, deadline/infrastructure controls, and two overlapping
isolation trials both passing. Tests drive real SMTP/HTTP/MCP, with no
mock substitute for the acceptance path. Matrix/schema/report/lifecycle
tests complete the evidence. A source defect discovered in Swarmail is
recorded with a reproduction and leaves the affected proof outstanding;
no source change or transport workaround is authorized here.

The proof target is `evals/tests/proof.rs` (`cargo test --locked --test
proof` from `evals/`); the fixture's wire controls, the lifecycle and
external-command controls, the scorer and the matrix/report live beside it
in `evals/tests/`. Until the CI and verify wiring follows #120 batch 5,
**evals is proven by its own suite run by the implement seat and
reproduced by reviewers**; nothing in the root gate inventory runs it. The
implementing run found no Swarmail defect: every flow's mail was accepted
with a final DATA `250` and read back over MCP as sent.

This proposed decision records the new eval task/result contracts and the
operator's scoped proof exception. It changes no accepted frozen guarantee
and does not self-accept a decision, implement a feature or claim a gate ran.
