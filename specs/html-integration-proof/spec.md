# HTML integration proof

Date: 2026-10-28. Phase: chief design for local PR #268 / issue #259.

## Feature and rationale

Obtain fresh, candidate-bound verification and independent protected review of
HTML compatibility repair composed with current main's #257 bulk checklist.
Review the full integration and every historical finding. A clean HTML-only
report, a passing earlier candidate or a successful journal inspection cannot
establish this combined candidate's correctness or delivery status.

The starting candidate is `728b99cfb4e4423757b6bdbfbbfe7d0d9b0dda4a`, tree
`afa6cd15929178a824fbba2ed622855cf392946c`. The required chief commit adds only
this directory's spec.md, plan.md and tasks.md. It advances HEAD, so subsequent
receipts must identify the actual clean descendant commit/tree and prove the
only difference from the starting candidate is these three design artifacts.
Never label a test of that descendant as execution at 728b99c. Any later repair
requires newly bound verification and review; this design authorizes no such
product repair. The specific instruction to commit this design is the sole
exception here to the task framing's restriction on commits before ship.

The intake directory `.brokkr/state/tasks/` is absent. Run-local
`.forge/issue259-review/integration-intake.md` and `integrated-review-feature.txt`
provide the framing. Both `.brokkr/state/design/positions/` papers were read.
The original issue.md, review.md, operator-rulings.md and feature.txt all passed
`sha256sum -c SHA256SUMS` in the packet directory at design time. These inputs
and `repair-inspect.json` remain run-local evidence; the last is a journal
snapshot. The a306543 repair candidate's earlier verify/review and read-only
ledger ship park are historical, not fresh clearance or a shipped integration.

## Authority and scope

Preserve the [house charter](../../docs/house-rules.md), accepted
[0001](../../docs/decisions/0001-synchronous-insert-lossless.md),
[0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md),
[0003](../../docs/decisions/0003-oldest-only-pruning.md),
[0005](../../docs/decisions/0005-extraction-at-ingest.md),
[0050](../../docs/decisions/0050-delivery-gates.md),
[0058](../../docs/decisions/0058-codex-review-routing.md) and
[0076](../../docs/decisions/0076-sequential-eval-trials.md).
The proposed [0085](../../docs/decisions/0085-deliverability-report.md),
[0089](../../docs/decisions/0089-bulk-sender-checklist.md) and
[0090](../../docs/decisions/0090-html-client-compatibility.md), and historical
[repair ledger](../html-review-repair/implementation.md), describe contracts
and evidence to assess, not authority to accept proposals. Follow the supplied
revised tests-seat direction recorded in
[0081](../../docs/decisions/0081-glm-flash-seats.md); leave recorded statuses
unchanged and report provenance discrepancies rather than resolving them by fiat.

This is a verification/review design, not a change to a frozen guarantee,
public contract, rule set or delivery route. No new decision is declared.
The missing 0091 repair record must not be reconstructed under a new proposal
that pretends to be historical evidence. A semantic repair, if separately
routed, needs its own proposed decision discipline and any necessary operator
acceptance. Historical specs and accepted decision bodies remain untouched.

Review includes both merge sides, the complete #259 feature, inherited #257,
eval conflict resolutions and version/stdio follow-ups. Exclude new parsers,
renderers, authentication implementations, DNS, dependencies, allowlist changes,
public knobs, caches, executors, durable analysis, receipt formats and paid evals.
No push, merge, tag, publication, issue closure, signing or decision acceptance.

## Panel adjudication

The papers substantially agree; do not invent a disagreement between them.
Resolve their different emphases and the alternatives they discuss as follows.

| Question | Ruling and adopted position | Rejected alternative and cost of the ruling |
|---|---|---|
| Product scope versus proof depth | Adopt simplicity's verification-only scope and existing owners; adopt robustness's full-candidate failure/composition evidence. | Reject another implementation/framework pass and treating a small diff as a small proof. Known gaps may prevent clearance and require a separate repair route. |
| Collector success versus report success | Adopt both papers: raw/header admission, demo, bulk, HTML, then the single finish owner. | Reject empty-raw report bypass and dropping bulk findings. Some SMTP-accepted mail has unavailable analysis, including exact-empty raw. |
| CSS implementation complexity | Keep existing Rules/Declarations/Component frames and literal URL consumer, as robustness requires and simplicity supports. | Reject recursive declaration interpretation or ignoring all components. Pay explicit delimiter/context/depth proofs; do not resolve CSS variables or promise browser equivalence. |
| Work evidence | Adopt robustness's counted-helper census and simplicity's local instrumentation, fixed bound and actual repeated-read negative control. | Reject cursor-only proof and counter-only sabotage. Pay independent accounting review and isolated experiment cost; no runtime work-quota API or RSS/latency guarantee. |
| Missing witnesses/documents | Adopt simplicity's refusal to edit this candidate and robustness's demand to expose missing proof and live ambiguity. | Reject silently fixing or waiving them to get clearance. Retain incomplete criteria and precise disposition, even if delivery parks. |
| Aggregate version | Preserve this candidate's swarmail-html/2 during review; require explicit assessment of historical HTML-only v2 versus combined v2 under 0085. | Reject assuming equal labels prove equivalent semantics, and an unreviewed bump in this design. Unestablished compatibility remains a residual for a separately authorized semantic repair. |
| Resource/failure ownership | Preserve one Store mutation boundary, one report Service, one wait owner and downstream webhook dispatch. | Reject new executors, polling loops, outboxes and liveness checks. Accept shared-pool contention, unavailable responses, selected reads finishing after prune, and best-effort webhook loss. |
| Verification and independence | Require fresh pinned gates and separate protected offices after verification. | Reject historical receipt reuse and chief/panel positions as review verdicts. Pay execution/provider/coordination costs and honest parks when evidence is unavailable. |

Finite authored heuristics, part-level diagnostics, strict supported-malformation
failures, no cascade/visibility/variable resolution and catalogue provenance
requiring reviewed Rust/version updates remain explicit costs, not defects
waived by this design. Frozen weakening, partial success and false clean
qualification are never acceptable costs.

## Acceptance Criteria

These are downstream proof obligations, not statements that this design has
executed them. An unmet criterion must appear as an evidence gap or finding;
policy alone decides disposition from valid review facts. A residual permission
never turns a failed criterion into a passed one.

1. **AC1 — Identity and provenance.** Record exact clean HEAD/tree before and
   after execution, the design-only diff from 728b99c, four packet checksum
   results, pinned bundle identity and candidate-local engine digest. Every
   fresh gate/review record binds commands, exits, log hashes and scope to that
   candidate. A change, skipped gate or sandbox refusal cannot inherit a pass.
2. **AC2 — Complete integration and findings.** Compare the combined tree with
   current-main side 7bf779f and historical repair a306543 and inspect merge
   resolutions and b49609d/728b99c follow-ups. Review all changed paths, including
   eval schema/provenance/privacy/tool-surface/admission code and removed tests.
   For every original correctness MEDIUM/LOW and independent items 1–8, preserve
   attribution, evidence and a resolved/unresolved disposition. Reassess earlier
   clean security/tests/guarantees conclusions on this full candidate.
3. **AC3 — CSS contexts and URLs.** Paired inline/style-block fixtures prove an
   unused custom-property component emits no position warning while genuine
   outer/following declarations do. Include media, strings, entities, opaque
   sentinels, matching delimiters, depth 32/33 and nested MIME locations.
   Literal unquoted `url(https://x.test/*/a.png)` succeeds, ordinary comments
   remain comments and following real warnings survive. Malformed URL/component
   tails, including after an earlier finding, deterministically fail the whole
   report. Real SMTP fixtures compare complete reports including bulk findings.
4. **AC4 — Actual inspected work.** Independently audit all charged helper reads,
   failed probes and entity windows against the ledger's census. Retain separate
   cursor progress. Existing adversarial sizes 64/512/4096/32768 cover successful
   and failing tag/raw-end/terminator/CSS/decoder paths. The fixed bound remains
   `W <= 8192*n + 4096` per eligible part, aggregate `8192*N + 4096*p`.
   Candidate-applicable independent red/green evidence must show real counted
   repeated-suffix reads break that same bound while progress passes, followed
   by byte-exact restoration and a passing test. A historical narrative or
   fabricated counter increment is insufficient. State excluded MIME internals,
   allocation and hash-model limitations; coverage cannot prove census completeness.
5. **AC5 — Exact-empty composition across surfaces.** Immediate SMTP DATA
   terminator receives 250; lookup succeeds and raw is exactly zero bytes before
   analysis. HTML alone contributes nothing; combined metadata admission refuses.
   Repeated REST reports yield sanitized 503; MCP and stdio yield isError without
   success/partial report; UI retains mail and displays failure. For each info,
   warning and error threshold, exact-zero and exact-one REST assertions,
   minimum-one REST awaits and MCP waits must propagate failure, never convert
   it to zero matches, a partial count or clean success. Preserve truly empty
   inbox timeout controls. Positive deadlines exercise completed analysis;
   zero-time cold requests remain ready-only unavailable and submit no work.
   Map actual assertions: the current empty-SMTP witness proves only info/exact-zero
   among these predicates; its name and the historical ledger prove no more.
6. **AC6 — Complete bulk/report semantics.** Complete canonical reports preserve
   demo, all seven HTML and seven bulk checks, IDs, hints, locations, severities,
   typed ordering and exact deduplication. Production authentication stays Unknown
   with error-level not_evaluated findings despite spoofed headers; no current
   threshold qualifies it. Trusted immutable owner-local fixtures prove three-state
   truth tables and full effective-input cache equality. Do not filter HTML-only
   subsets to establish combined report correctness.
7. **AC7 — Boundaries and atomic failure.** Prove inclusive raw 8 MiB, per-header
   65,536, aggregate headers 262,144, outer entities 256, root-relative MIME depth
   32, aggregate eligible decoded HTML 4 MiB, lexical depth 32, pre-dedup
   contributions 256 and compact JSON 65,536. At-limit/one-over fixtures isolate
   each bound; clipping starts strictly above 104,448 decoded UTF-8 bytes per
   part. Preserve attachment/embedded-message exclusion, charset and MIME paths.
   Malformed/decoding/header/later-part/budget failures repeatedly produce no
   partial report, success cache entry or partial exact count. Both header-overflow
   and malformed-HTML real-wire witnesses must remain.
8. **AC8 — Frozen guarantees and indirect effects.** Prove stored visibility
   before SMTP 250, persistence refusal before publication, subscribe-before-scan
   and resubscribe-before-scan after Closed, lag/deadline/shutdown behavior and
   synchronous oldest-only pruning. Retain ingest decoder/extraction parity,
   raw bytes, restart/deletion and burst/p2/rate evidence. Analysis stays lazy,
   outside SMTP and mutation guards, with 128 Weak identity/config/version slots
   and four worker-owned permits retained through cancellation/unwind. Preserve
   DATA boundary refusal and downstream nonblocking webhook ownership, generated
   certificate fixtures and observable drops. Neither memory-only durability,
   exactly-once delivery, indefinite retention nor a latency SLA is implied.
9. **AC9 — Version and documentation honesty.** Check aggregate version,
   catalogue, config/cache, complete fixtures and served README/llms/OpenAPI
   agreement, including cross-build v2 ambiguity. Map authored provenance/date,
   strict-malformation and header-wire residuals explicitly. Independently report
   the index's blank separator before 0090, absent 0091 target in 0085/0090/ledger
   pointers, and living text describing bulk as future while already present.
   Current guidance must plainly distinguish no HTML contribution from combined
   empty-input failure and authentication Unknown. Until corrected by an authorized
   route, these remain discrepancies, not completed historical repairs.
10. **AC10 — Fresh local gates.** Execute the entire sole house gate inventory
    offline with locked dependencies and the digest-checked `.forge/bin/brokkr`,
    pinned proof owners and verified public shims. Require zero missed lines,
    branches and functions, coverage selftest, publish/seal proofs, permissive
    licenses and fail-closed native-tls absence. Retain design/delivery selftests,
    package inclusion and targeted report/bulk/stdio/burst/p2/rate evidence.
    Compilation policy-sweep warnings refuse even with exit zero. Do not install,
    widen licenses, reduce the coverage denominator or treat missing tools as pass.
11. **AC11 — Standalone eval coordination and proof.** Before eval execution,
    establish an authorized suite-wide nonoverlap window with live #157 and other
    same-user eval suites; record observation/coordination and candidate root
    binary identity. A free per-trial lock is insufficient. If busy, wait outside
    admission; if visibility is unavailable, defer with the exact gap. Never
    unlink/bypass or hold the trial lock around the suite. Run locked standalone
    fmt/clippy/full tests, retaining oracle/null, sequential refusal, strict v2
    rows, provenance/privacy and reached failure-injection assertions. Preserve
    contention logs and rerun only after exclusivity; no paid trials.
12. **AC12 — Protected review and honest handoff.** Fresh hermetic verify precedes
    separate gate-class read-only correctness, security, tests, guarantees and
    independent results under 0058 and the revised tests-seat direction. Retain
    actual resolved identities, separate invocations, full scope and fixes facts,
    exact candidate binding, worst-verdict aggregation and security-owned hard hold;
    unknown/malformed facts refuse. This chief supplies none of those verdicts.
    CI retains macOS, RustSec, Helm/chart/history, Docker, privileged port-80 and
    tag-only proofs. Operator retains acceptance, sealing and publication. If
    policy reaches ship but writes or forbidden pushing prevent it, record the
    exact attempted action/error and park without synthetic ledger events or a
    shipped/merged claim.
