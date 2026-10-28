# Combined report review

Date: 2026-10-28. Status: design draft for issue #259.

## Feature and rationale

Establish fresh, candidate-bound verification and independent protected review
of the complete HTML repair combined with the bulk checklist. Start from clean
`c793401fb71583714e452c60680fcbfdc542ffaf`, tree
`bd18e9ea7709f84f7007c7a6be0aa041ba2cdc3e`. The previous integrated candidate
`4dbbbab9d3c2c672da5cdec0fcb08b4e33a97414` stopped with unresolved findings,
eval coordination and hermetic proof. Neither its observations nor this design
clear the present candidate. Every later change requires evidence bound to the
actual descendant HEAD and tree.

A program must be able to trust both an accepted message and the report's
limits: SMTP acceptance promises queryability; it does not promise successful
analysis. A reviewer must see complete combined reports, explicit authentication
uncertainty and authentic proof gaps. This work verifies the existing behavior
and permits narrowly necessary regression-test strengthening or compatible
repairs of demonstrated defects; it does not introduce new product semantics.

The applicable contracts remain [0085](../../docs/decisions/0085-deliverability-report.md),
[0089](../../docs/decisions/0089-bulk-sender-checklist.md),
[0090](../../docs/decisions/0090-html-client-compatibility.md) and
[0091](../../docs/decisions/0091-html-review-repair.md), all still proposed.
The combined html/3 note already exists in 0091. No new ruling, public contract
or route is selected here, so this design declares no new decision. A necessary
semantic change must return for a proposed decision; contradiction of an accepted
ruling waits for operator acceptance, and frozen weakening is refused.

## Evidence and scope

Both run-local position papers were read in full. The requested intake task
directory is absent. The supplied task, recorded feature text and intake result
`6dde7bac-e460-4cd1-94d9-7acabebba0b9` supply the framing; no substitute journal
or missing task is invented. The four original issue-packet SHA256SUMS pass.
The stopped-run audit's observed SHA-256 is
`89c42bba556b8ba786cb8434cf71e60aac1f9ba38c0cd087cfd26db841d3abb5`,
matching intake. No independently supplied expected digest was located: this is
content identification and agreement between observations, not independent
authentication. Its experiments and verdict descriptions remain attributed to
the earlier implementer. Obtain authoritative provenance if needed downstream;
never silently upgrade the audit to a protected result.

Review the original `7f27fe6..a49c702` HTML feature, `a49c702..a306543` repair,
the full `7bf779f..candidate` integration, both integration parents and the
`4dbbbab..candidate` follow-up. Include bulk composition, shared decoder,
removed/retained eval assertions and mixed recipe/CI changes, not only report
tests. Chief inspection of inventories and selected owners is design grounding,
not exhaustive semantic review. Historical clean lenses must be reassessed too.

## Panel adjudication

| Question | Ruling and explicit cost |
|---|---|
| Owners and scope | Adopt simplicity's existing collector/finish, Service and Store wait owners; robustness agrees. Reject a second executor/cache/wait loop, persistent report state, generic parser/tracing/evidence framework and new authentication infrastructure. Accept recomputation, shared blocking-pool contention and some unavailable reports. |
| Strength of CSS wire evidence | Rule for robustness's independent whole-composition oracle over accepting filtered HTML plus adapter parity as sufficient. Simplicity's small extension of the existing suite is the implementation. Each of eight reports needs expected bulk and HTML fields independent of the response. Cost: maintained explicit expectations; parity alone loses the ability to detect a common wrong aggregate. |
| Failure matrix | Require direct zero-byte cold-zero and true-empty-inbox controls or an exact existing-test mapping proving every required cell and outcome. Reuse rather than duplicate any adequate assertion, as simplicity requests. If the map fails, extend the existing wire test; do not call a historical temporary probe durable. Cost: additional focused assertions and honest remaining gaps. |
| CSS and URLs | Adopt robustness's bounded Rules/Declarations/Component ownership and literal URL token handling. Reject blanket custom-property suppression and treating every brace as declarations. Accept bounded parser state, strict malformed failure and missed unmodeled browser behavior. No variable, cascade, selector or browser evaluation. |
| Work proof | Adopt actual helper reads and conservative per-invocation charges with a fixed bound; reject progress-only proof, fabricated increments and fitted constants. Reconcile simplicity's no-framework rule with robustness's census and real-read negative control. Cost: instrumentation maintenance and an isolated red/restored-green experiment; its sensitivity cannot prove census completeness. |
| Empty mail and uncertainty | Preserve HTML-only no-parts behavior locally and combined header admission failure. Reject an aggregate empty-success shortcut, SMTP rejection, synthetic headers, skipped predicate candidates or inferred auth. Cost: some accepted mail is unavailable for analysis and production Unknown excludes qualification at every threshold. |
| Verification and review | Both positions require authentic current evidence. Reject historical clearance, candidate-copy gates as hermetic verification, a trial lock as suite exclusivity, or compilation as dispatched review. Cost: waiting or stopping when coordination, pinned proofs or required reviewers are unavailable. |

## Preserved contracts and limits

Keep synchronous commit/visibility before SMTP 250, subscribe-before-scan and
Closed resubscription, synchronous oldest-only pruning, per-test inboxes and
ingest extraction under accepted 0001–0005 and their amendments. No analysis
runs on SMTP, seed, restore, persistence or webhook fan-out. Preserve sender
raw/body/link/code data, lazy immutable reports, selected snapshot semantics,
Weak mail identity plus full Inputs equality, four worker-owned permits,
128 cache slots, success-only caching and cancellation ownership. Selected
reads may finish after deletion; new lookups must not resurrect mail. No
stronger durability, retention, webhook replay, fairness or latency is promised.

Keep seven HTML rules, seven bulk checks and demo composition; schema, severity,
hints, coordinates, canonical ordering and deduplication remain unchanged.
Production authentication is Unknown, with distinct error-level not_evaluated
findings. SMTP AUTH routes inboxes; sender Authentication-Results/DKIM headers,
metadata syntax and an empty HTML contribution cannot establish authentication.
Trusted three-state facts remain internal immutable inputs with complete cache
identity and the existing truth tables. No DNS, fetch, public auth override,
new dependency or license widening is required.

Preserve inclusive raw 8,388,608-byte, individual-header 65,536-byte,
aggregate-header 262,144-byte, 256-entity, MIME-depth 32, aggregate decoded-HTML
4,194,304-byte, lexical-depth 32, pre-dedup 256-contribution and compact-report
65,536-byte bounds. Later-part or malformed/decode/limit failures discard all
findings and publish no success cache entry or partial exact count. Keep
102-KiB clipping's strict greater-than threshold and authored provenance limits.

## Historical findings and R1–R6

These are design observations and obligations, not protected dispositions.

| Item | Current observation and required independent disposition |
|---|---|
| R1: history/index | Chief byte comparison confirms original repair spec/plan/tasks equal a306543, and 0091 retains its historical bytes as a prefix with the combined note appended. Index has one contiguous table, 78 unique numbered rows and complete targets. Preserve originals and proposed statuses; recheck links and review the note. |
| R2: served empty semantics | Current README/llms/OpenAPI describe accepted empty mail and combined failure. Verify served bytes, exact-zero's failed-analysis exception and UI/raw retention; correct only a demonstrated remaining contradiction. |
| R3: identity | Combined constant, catalogue and tests name swarmail-html/3; historical html/2 stays meaningful. Audit every cache/config/fixture/doc/wire owner; no global replacement in historical records. |
| R4: durable proof | Tracked complete 15-finding oracle and positive-deadline three-threshold/count matrix exist. CSS pairs independently fix only HTML; full-response parity is weaker. Apply the adjudication above and map cold-zero/empty controls and sanitized MCP/stdio payloads. |
| R5: eval/CI wording | evals/Cargo.toml now assigns standalone gating to #201, consistent with README. Verify workflow and pinned owners: mixed recipe compilation does not run evals, and root tests exclude that workspace. Review integration deletions independently. |
| R6: 0081 provenance | Current decision/index/0058 pointer say proposed; the checksummed operator packet records accepted on 2026-10-26. Report both with attribution, retain explicitly directed Spark routing, and change no status or historical record. |
| Original context/URL findings | Evaluate bounded components, outer/following declarations and literal slash-star URL behavior with positive/negative pairs; original non-security classifications remain attributed, not independently reissued here. |
| Original inspected-work finding | Independently assess census and actual mutation evidence; progress alone and historical numerical receipts are insufficient. |
| Original empty/index findings | Resolve through R1–R4 with current combined semantics, not historical demo/HTML success. |
| Original fixed provenance/date INFO | Retain authored_heuristic/date validation and negative tests. Future provenance changes cost Rust and version review; do not claim arbitrary data-only extensibility. |
| Original malformed-input INFO | Retain documented strict failures for recognized unfinished syntax, optional HTML end tags and stray/mismatched CSS closers; lexical analysis is not browser recovery. |
| Original displaced-header witness INFO | Keep root-header overflow and malformed HTML as distinct real-wire failures; an owner-only limit test cannot replace the restored wire assertion. |

## Acceptance Criteria

1. **AC1 — Bound scope and provenance.** Record clean starting and final HEAD/tree,
   every changed-path inventory and both merge sides; verify packet checksums,
   engine digest and shim integrity. Each original finding and R1–R6 has a
   current source/test/evidence mapping and independent disposition or explicit
   unresolved reason. Preserve audit authentication limits and genuine refusals.
2. **AC2 — History, identity and wording.** Prove R1's byte comparisons and one
   row per numbered decision with valid links. Prove historical html/2 versus
   combined html/3 across catalogue validation, immutable Inputs/cache identity,
   tracked fixtures, live/served docs and all report adapters. Report R6 without
   accepting a decision; keep historical specifications intact.
3. **AC3 — Complete durable reports.** Execute a tracked exact canonical
   15-finding report (demo + seven bulk + seven HTML) and eight inline/stylesheet
   CSS/URL reports with independently specified full expected aggregates.
   Assert repeat REST, MCP text-block, UI and applicable stdio parity and unchanged
   raw/message data. Catalogue-derived HTML expectations also require independent
   catalogue snapshot/validation checks; no response-derived aggregate oracle.
4. **AC4 — Empty failure matrix.** Real immediate SMTP DATA terminator receives
   250, then lookup is 200 with exactly zero raw bytes before analysis. Repeated
   REST reports return exact sanitized 503 failure; MCP/stdio have isError and
   sanitized failure text; UI retains the message. For each info/warning/error
   threshold and count 0/1, REST exact asserts and positive-deadline REST/MCP waits
   fail analysis, never return clean/partial counts. Zero-time cold waits submit
   no analysis and return unavailable; genuinely empty inbox controls have exact
   zero success, exact one mismatch and minimum-one wait timeout, including count
   zero. Prove these distinctions with durable cells or an exact assertion map.
5. **AC5 — CSS/URL semantics.** Paired inline and stylesheet fixtures prove unused
   components cannot manufacture position declarations and genuine outer/following
   declarations remain visible. Cover nested MIME locations, opaque components,
   matching/mismatched delimiters, depth 32/33, URL token boundaries/case, literal
   slash-star, quotes, scalar/hex escapes, ordinary comments, controls and EOF.
   Preserve atomic malformed failure. Trusted-input/evaluator controls prove HTML
   threshold discrimination separately from production Unknown bulk errors.
6. **AC6 — Inspected work.** Independently review every helper/decoder operation
   and failure path against the census, including repeated library calls,
   40-scalar windows plus lookahead and 6/5 expansion. Keep progress separate.
   Prove fixed `W <= 8192*n + 4096` per eligible pre-entity HTML part and
   `W <= 8192*N + 4096*p` in composition. On an isolated candidate-bound copy,
   an actual counted repeated-read mutation must fail the unchanged bound after
   progress passes; retain patch/source/log hashes, byte-exact restoration and
   restored green. No committed fault hook. Explicitly exclude MIME-parser
   internals, allocation, hash-collision behavior, RSS, latency and future
   uninstrumented operations; sensitivity is not completeness of the model.
7. **AC7 — Bulk, limits and failure.** Execute three-state truth tables,
   immutable trusted-input/cache tests and spoofed-header Unknown controls.
   Verify inclusive boundaries listed above, per-part composition, later-part
   failure, failure retries and separate header-overflow/malformed wire witnesses.
   No omitted bulk findings, lower severity, changed limits or success caching.
8. **AC8 — Guarantees and ownership.** Source review and full protocol,
   persistence, extraction, burst and deterministic concurrency proofs preserve
   0001–0005, cancellation permits/pins, exact asserts, minimum-one waits,
   lag/deadline/shutdown and subscribe-before-scan including resubscription.
   Use 0020 placement, real servers, fresh inboxes and barriers, no sleep sync.
9. **AC9 — Fresh gates.** Execute every local command in the sole
   [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
   offline with locked dependency resolution, candidate-local digest-checked
   brokkr and engine-mounted pinned proof scripts. Include exact zero missed
   lines/branches/functions, coverage selftest, publish/seal proofs, licenses
   and fail-closed native-tls absence. Preserve command/exit/log digest and
   candidate identity. Missing pins remain unresolved; candidate-copy execution
   cannot emit a hermetic verified_candidate.
10. **AC10 — Separate eval proof.** Independently review integration retention
    and deletions, verify CI/eval comments, then run standalone fmt, locked
    all-target clippy and full tests against a captured candidate-bound root
    executable only in an authorized suite-wide same-user nonoverlap/cleanup
    window. Record binary/source identity and coordination evidence. A per-trial
    lock or invisible host processes is insufficient; unavailable coordination
    leaves proof pending. No paid trial or admission bypass.
11. **AC11 — Protected review.** After fresh hermetic verification, obtain five
    distinct candidate-bound gate-class/read-only results: Astra correctness,
    official DeepSeek Flash security, local Spark GLM-5.3-Flash tests (effort not
    applicable), Astra guarantees and independent Claude Fable. Prove effective
    mixed-opus-terra identities, classes and inherited policy. Preserve scope,
    fixes, worst verdict, unknown-fact refusal and security-owned hard hold.
    Compilation, author judgment and earlier verdicts are no substitute; no new
    fallback or reviewer impersonation is allowed.
12. **AC12 — Honest handoff.** Keep CI macOS, network RustSec, Helm/chart/history,
    Docker, privileged port-80 and tag-only proofs with CI; operator acceptance,
    sealing and final advanced-main/#260 integration remain separate. New
    integration requires fresh bound proof. Preserve all residuals and actual
    result-file/ledger refusals. No push, merge, publication, tag, issue closure,
    decision acceptance or paid trial. This chief's explicitly required unsigned
    design commit is the sole pre-ship commit exception in this phase; its
    `drafted` result claims only committed design artifacts, never delivery.
