# Fixture follow-up verification and protected review

Date: 2026-10-28. Run: verify-and-independently-protect-4f13ffbd.
Design status: drafted; no verification, protected-review or ship verdict.

## Feature and rationale

Obtain fresh, independently assessed evidence for the entire issue #151 fixture
HTTP metadata candidate. The starting clean commit is
`09852d8c9c6b0485bbeb4948068db86f69681ae3`, tree
`6815a8cfc05ef0e5de8edb6545a629663657b5e7`. The prior run
`verify-and-protected-review-the--04cce2fe` parked at event 701 with a
schema-invalid member result. It neither shipped nor established the paid
#151 baseline. Local green receipts cannot replace this continuation's proofs.

The expected runtime delta is zero. Preserve the existing helper and integrated
repairs; investigate demonstrated failures through their current owners. Review
original behavior, metadata/positional fixes, merge resolutions, provenance,
reproducibility and the final CI-comment/MCP/UI corrections. A review confined
to the last three-file diff is insufficient.

The design commit necessarily changes HEAD. Later receipts must identify the
actual descendant commit/tree and record its relation to the starting candidate;
they must never label that execution as untouched 09852d8. A further repair
invalidates affected evidence and review and requires renewed candidate binding.

## Evidence and authority

Both `.brokkr/state/design/positions/simplicity.md` and `robustness.md` were
read. The historical `.brokkr/state/tasks/` directory is absent; current intake
is `.forge/tasks/verify-and-independently-protect-4f13ffbd/intake.md`.
The supplied task and retained feature framing agree with that intake.
The local `issue151.json`, `prior-review-inspect.json`, `followup-index.json`
and `followup-evals-test.log` under `.forge/evidence/` are historical inputs,
not current verification receipts. Missing historical logs or authority packets
must be recorded as missing, never reconstructed.

The [house rules](../../docs/house-rules.md) and accepted decisions
[0050](../../docs/decisions/0050-delivery-gates.md),
[0055](../../docs/decisions/0055-agent-eval-harness.md),
[0058](../../docs/decisions/0058-codex-review-routing.md),
[0063](../../docs/decisions/0063-eval-provenance.md),
[0075](../../docs/decisions/0075-eval-privacy.md) and
[0076](../../docs/decisions/0076-sequential-eval-trials.md) govern proof,
review, provenance, privacy and admission. Preserve frozen 0001–0003 and
extraction-at-ingest under 0005, including their accepted amendments.

This follow-up introduces no semantic change or new ruling, so it declares
no new decision. Existing proposed
[0087](../../docs/decisions/0087-fixture-http-meta.md) records the helper
contract, and proposed [0083](../../docs/decisions/0083-cli-wrapper-launch.md)
records wrapper scope. The revised tests-only operator direction recorded in
proposed [0081](../../docs/decisions/0081-glm-flash-seats.md) supplies the
current GLM tests route. Those references do not accept the proposals. Any
new semantic repair needs its own proposed record and, if it contradicts an
accepted ruling, operator acceptance before implementation.

## Panel synthesis and explicit rulings

The positions agree on runtime ownership; inventing disagreement would obscure
their useful difference in emphasis. These choices rule competing costs explicitly.

| Question | Ruling | Rejected alternative and cost of the ruling |
| --- | --- | --- |
| Runtime scope | Adopt simplicity: retain the named discarded metadata field and current owners. Adopt robustness: preserve positional arity and all closed shapes. | Reject an MCP framework, metadata schema/progress state, retries and a second scheduler. Opaque metadata remains tolerated; no general MCP-conformance or exactly-once promise. |
| Evidence breadth | Adopt robustness: review the full integration and run complete proofs. Reconcile simplicity by reusing existing tests and receipt mechanisms. | Reject last-diff/happy-path-only verification and a duplicate harness. Pay sequential suite time, immutable executable storage and complete logs despite the small helper diff. |
| GET bodies | Retain presence-based refusal, including null and empty string, as both positions recommend. | Reject normalization and recursive removal of application `_meta` keys. Clients must omit a GET body; POST JSON stays unchanged. |
| Runtime failure analysis | Adopt robustness's preservation checks for storage, waits, pruning, webhook ownership and reproducible fixtures. Apply simplicity's limit against speculative redesign. | No new queue, outbox, namespace boundary or chaos subsystem. Crash uncertainty, best-effort webhooks and existing resource limits remain explicit. |
| Governance | Require five actual protected results plus concrete candidate/engine-bound disposition of the inherited high finding. | Reject compilation, a later upstream patch, route arrays or five unsupported opinions as closure. Delivery may remain blocked after every local test passes. |
| Missing evidence | Preserve failed attempts and distinguish missing facts from passes. | Reject fabricated red logs, synthetic reviewer results, gate waivers and pin/fallback changes. Recovery can require another authorized engine attempt or unavailable external evidence. |

## Acceptance Criteria

1. **AC1 — Candidate identity.** Every fresh receipt records commit/tree,
   tracked-source cleanliness before/after, command, actual exit and retained
   complete log. Candidate-local `.forge/bin/brokkr` matches the tooling pin
   (currently SHA-256 `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`);
   tooling patch identities and public shims match their pins. No PATH engine,
   re-pin, dependency allowance, assertion weakening or coverage exclusion is
   used. Evidence names the design descendant and any later repairs explicitly.
2. **AC2 — Metadata compatibility.** Real stdio-child and loopback HTTP tests
   cover 2025-03-26, 2025-06-18 and 2025-11-25. Supported request metadata is
   discarded without echo, forwarding, retention or progress output; initialized
   notifications remain silent. Full stdout/stderr and complete HTTP capture
   prove exact replies and request totals after owned cleanup. Closed Call and
   Arguments reject unknown fields, arguments._meta, missing/type-invalid
   required fields and a third positional metadata slot; the legacy two-slot
   call remains valid. Preserve existing permissive initialize/ping/list paths,
   one eval_http descriptor, 64 KiB input/argument and 1 MiB response limits.
3. **AC3 — HTTP behavior.** Every present GET body, including null and empty
   string, yields `result.isError=true`, fixed `http_get_body_not_supported`,
   no JSON-RPC error and no HTTP request. Omitted-body GET succeeds. POST retains
   exact JSON serialization, including legitimate application `_meta` keys.
   Unrelated failures retain `fixture_request_refused`. Identity injection,
   app-relative origin/path, header controls, no proxies/redirects, deadlines
   and response-byte behavior remain proven by existing real-protocol tests.
4. **AC4 — Honest descriptions.** Inspect actual workflow/pinned verify source
   alongside evals/Cargo.toml: standalone fmt/lint/full tests remain separately
   required until #201 wires them. MCP/UI descriptions name demo and bulk-sender
   checks and retain the empty-findings limitation; root MCP/UI tests and actual
   tools/list parity pass. Comments alone establish no CI or report behavior.
5. **AC5 — Integrated guarantees.** Full-source review and root suites preserve
   final DATA 250 only after synchronous ordered storage, commit-before-memory
   visibility, subscribe-before-scan (also report waits/Closed recovery),
   oldest-only eviction and ingest extraction. Keep webhook work outside the
   mutation boundary and existing bounded delivery/cleanup ownership. Tests
   retain fresh inboxes, generated TLS fixtures, causal synchronization and
   chaos at probability 0 or 100. Do not add durability, indefinite retention,
   total-memory, total-session-lifetime or exactly-once claims.
6. **AC6 — Executable and standalone proof.** Fresh locked Cargo build evidence
   associates the successful root swarmail executable with this exact candidate.
   Preserve its artifact record and source observations, copy the bytes to a
   unique non-overwritten read-only run-owned path, and set absolute SWARMAIL_BIN.
   Its SHA-256 must match immediately before and after the complete standalone
   suite. Standalone fmt, locked all-target clippy and full tests pass, including
   provenance, v2-only readers/scorer, identity, retention/replay, wrapper launch,
   cleanup and admission tests. Preserve intentional override/missing/build-path
   tests; a global binary setting must not erase their controls. Mutable target
   paths, missing attribution, changed bytes, timeout or partial runs do not pass.
7. **AC7 — Suite coordination.** Before any complete or focused eval suite,
   establish accessible same-user suite/admission activity and wait for another
   suite to finish. Preserve parent-test coordination and production nonblocking
   trial admission; never hold the trial reservation around children needing it.
   Do not delete/recreate locks, kill other suites or bypass admission. Retain
   contention as scheduling evidence and obtain an uncontended successful run.
   Inaccessible activity or unproved descendant cleanup remains an evidence gap;
   an unlocked inode after a crash alone does not prove safe resumption.
8. **AC8 — Local gates.** The full local inventory in house rules passes offline
   with locked dependencies and no installation, through the pinned proof owners:
   root fmt/clippy/tests, candidate-local bundle compile, exact zero-missed
   line/branch/function coverage and selftest, publish guard, keyless seal proof,
   licenses and fail-closed native-tls absence in both locked trees. Include the
   intake's design selftest and delivery-proof test. Require no unruled valuation
   or policy-sweep warning. Keep commands, exits and complete reports. Missing
   exits, dependency errors, partial logs or timeouts are not successes.
9. **AC9 — Protected panel.** The engine obtains and retains five separate
   gate/read-only, schema-valid results on the verified candidate: Astra
   correctness, official DeepSeek Flash security, local Spark GLM Flash tests
   (effort not applicable), Astra guarantees and independent Fable. Record
   resolved provider/model/effort, role, run/attempt, candidate, launched engine
   and actual member/aggregate evidence. The two Astra offices have distinct
   invocations and charters. Preserve worst-verdict policy, security-hold
   authority, bounded returns and unknown-fact refusal. Malformed/missing results
   and dispatch failures cannot be rewritten as clean; recovery uses only the
   authorized engine path, without author substitution or new fallback.
10. **AC10 — Governance finding.** The inherited high routing finding receives
    an explicit independent protected disposition backed by concrete evidence
    for the actual candidate and launched engine. Retained source evidence in
    specs/glm-flash-seats/intake.md, its implementation record and proposed 0081
    reports a dispatcher correlation failure; this is historical evidence,
    not a new reproduction. Reviewing/compiling a later upstream patch or
    matching a binary digest alone cannot resolve it. Missing evidence leaves
    it outstanding and shipping blocked. No attack reproduction, boundary
    manipulation, fabricated artifacts or decision acceptance is authorized.
11. **AC11 — Ownership and reporting.** Receipts separate local success from
    CI-owned platform, chart/history, Docker, network audit, real port-80 and
    tag-only proofs. Paid multi-model baseline, taxonomy publication, live
    interoperability and external artifacts require their own actual evidence.
    This follow-up does not satisfy #151's paid-baseline acceptance criteria.
    No push, merge, tag, publication, issue closure, sealing or decision acceptance
    occurs. Authorized commits are unsigned; only engine policy can reach ship.

## Accepted limits

A POST can take effect before a crash loses its reply; no retry is added.
Persistent commit before a lost SMTP reply can leave sender uncertainty;
in-memory mode makes no crash-durability promise. Webhooks remain best-effort,
not an outbox. The bounded wake ring is not durable history and retained mail
can age out under the cap. Fresh HOME and cooperative same-user admission do
not confine hostile peers. Hash checks establish measured run consistency,
not hostile-writer resistance or independently reproducible engine provenance.
These limits do not waive any acceptance criterion or resolve the high finding.
