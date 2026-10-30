# Plan — MCP candidate proof

## Technical context

Single Rust crate; application owners are `src/api.rs`, `src/mcp.rs` and
`src/stdio.rs`; real protocol suites live under `tests/`. Baseline is
`ab807436aa9bab8ebe21c49f6dea3481bb74eebe`, clean at inspection. The design/doc
commit is a successor, never silently called ab80743. No new dependency, SDK,
service, storage field or limiter is planned.

Only `.specify/templates/spec-template.md` exists in the available template
catalogue; no plan template was found. This plan supplies the requested technical
context, constitution check, structure, research, data model, contracts,
quickstart and risks directly. Research here means local source inspection;
there is no new remote-source validation or provider dispatch in this seat.

## Constitution check

Preserve 0001–0005 and accepted 0043 guard/method precedence. A proposal cannot
overrule accepted policy (0045 §8). The report is derived analysis under 0085,
not query-time link/code extraction. Preserve 0020 test placement, mutation/wait
owners, per-test inboxes, exact coverage and license allowlist. Retain 0057/0058
protected review, bounded hands and the explicit mixed route. Keep
0079/0080/0081/0085/0090/0103 and this correction proposed. No engine/adapter
capability change, protection bypass or full-eval workaround is commissioned.
Commits are unsigned; only the operator seals and merges.

## Research and implementation approach

1. Capture baseline and successor commit/tree identities and clean status before
   each proof group. Preserve historical specs and ignored position/intake
   evidence. Use existing run-local logs/receipts rather than inventing a journal
   or another permanent evidence schema.
2. Use decision 0104 for three observed ledger gaps: thirteen versus twelve
   tools, the OPTIONS header exception and the new real-connection limiter
   witness. It records presence, not a passing result. Add discoverable links in
   current served descriptions and the decision index during implementation.
   Do not replace the historical C1–C6 register.
3. Run existing HTTP/stdio and deterministic limiter tests against the identified
   tree. Compare wire outcomes with served descriptions, not just strings with
   strings. Extend only existing served-contract assertions for the correction
   link; preserve structural OpenAPI checks.
4. If a concrete failure requires repair, localize it to the existing owner and
   add the discriminating regression there. Recheck scope and accepted authority.
   No general middleware, second parser, another wait loop or unrelated hardening.
   Record a new candidate and rerun verification/protected review; do not import
   previous passes.
5. Freeze the implementation successor for hermetic verification. Execute every
   house gate through digest-matched engine and transitive pinned resources;
   verify public shims without executing them. Include intake's focused suites,
   design-check selftest and delivery-proof obligation through the authorized
   proof path. If a required proof cannot run within the no-attack-reproduction
   boundary, record that precise unmet obligation and stop; never replace it
   with a weaker assertion or reproduce the inherited review-integrity attack.
   This design does not repair that boundary.
6. Resolve the effective mixed route and five protected roles, then leave actual
   invocation to protected review. Preserve high concern/hold state. Review every
   final change including docs, recipe and tests; route compilation cannot
   substitute for a member result. Any later candidate change requires fresh
   binding and applicable gates, not an amended old receipt.
7. Record command exits, receipts and remaining CI/operator obligations. Do not
   run paid trials or full standalone eval under #201's unproved coordination
   scope. Retain #151's merge hold.

## Project structure and files touched

| Files | Planned treatment |
| --- | --- |
| `specs/mcp-candidate-proof/{spec,plan,tasks}.md` | Chief commits only this new design directory. |
| `docs/decisions/0104-mcp-candidate-proof.md` | Chief's new provisional dated correction; no accepted text rewrite. |
| `src/static/openapi.json`, `src/static/llms.txt` | Implementation adds current correction pointers and retains accurate bounded claims. |
| `docs/decisions/README.md` | Implementation indexes the new proposal without accepting or reallocating decisions. |
| `tests/mcp_revisions.rs` | Extend existing served-contract assertion for correction pointer; run method, catalogue, lifecycle and SMTP witnesses. |
| `tests/stdio.rs`, `src/api.rs`, `src/mcp.rs` | Existing protocol/limiter witnesses; no edits unless an evidenced defect needs a minimal regression/repair. |
| `tests/delivery_recipes.rs`, `tests/delivery_constitution.rs`, `bundles/mixed-opus-terra/`, mixed work offices | Inspect and prove route; no planned routing rewrite. |
| `src/store.rs`, `src/smtp.rs`, `src/extract.rs`, persistence/burst/wait suites | Preserve owners; run guarantees proofs through full workspace tests. |
| Historical `specs/mcp-version-negotiation/`, accepted decisions, gate scripts, tooling pin, dependency files | Preserve; a failure is evidence to report, not permission to weaken them. |

Research/data-model/contracts/quickstart sections are embedded here; no extra
tracked design artifacts are needed. Execution receipts use existing ignored
run storage. Any later tracked evidence summary must be true and included in
the final verified/reviewed candidate.

## Data model and contracts

Runtime models stay unchanged. Each HTTP exchange owns its revision; the stdio
owner owns lifecycle state; the router owns shared ToolRate. The catalogue is
the sole descriptor source. Store and report cache remain separate owners.
Evidence needs candidate commit/tree, command/invocation, exit/verdict, artifact
identity and limitation, using existing formats. Missing safety/result facts
remain unknown, never synthesized as success.

Spec AC2–AC5 own observable method, version, hint and budget contracts. Decision
0104 owns dated qualifications of old claims. Keep C1–C6's full actor/strength
ledger and record OPTIONS separately. No new API, transport promise, retry
semantics, full source conformance or model safety claim follows.

## Quickstart and validation

Inspect `git rev-parse HEAD`, `git rev-parse HEAD^{tree}` and `git status --short`.
Check candidate-local `.forge/bin/brokkr` against `bundles/self/tooling.json`
before use. The chief's workspace inspection found neither `.forge/bin/` nor
`/runtime/bundle/scripts/design-check.sh`; the latter belongs to the separately
dispatched exec check. These are observed availability limits, not claims about
the host or final-check environment. Do not fetch/install/re-pin to conceal a
verifier setup block.

Focused application proof commands, through workspace hands:

```bash
cargo test --locked --test mcp_revisions
cargo test --locked --test stdio
cargo test --locked --test delivery_recipes
cargo test --locked --lib tool_rate_uses_exact_monotonic_windows
cargo test --locked --lib exhausted_tool_rate_refuses_before_effects_and_keeps_control_available
cargo test --locked --lib the_router_owns_one_tool_rate_across_connections_revisions_and_members
```

These diagnostics do not replace the sole gate inventory in
[house rules](../../docs/house-rules.md#gates--part-of-every-change). Verify covers
format, clippy, full workspace tests, bundle compile, coverage and selftest,
publish/seal proofs and license check, with required locked commands and exact
zero misses. Also require shim integrity and fail-closed native-tls proof from
the pinned verifier. Preserve intake's additional design selftest and delivery
proof. A filter matching zero tests supplies no evidence. The digest-checked
engine must compile self and mixed-opus-terra without policy-sweep warnings;
inspect resolved classes/providers/model chains/efforts. Compilation establishes
no provider availability or protected verdict.

The chief may run the repository design checker in `check` mode for an advisory
structural result; the separately dispatched pinned final check owns the sequence
verdict. Leave the design commit last with exactly one spec directory and one
linked, clean proposed decision. Do not write a `designed` result as chief.

## Risks

| Risk | Mitigation and remaining cost |
| --- | --- |
| Proof for ab80743 attributed to changed HEAD | Record both identities; freeze final successor and rerun gates/reviews. Docs also move the candidate. |
| Historical ledger read as current certification | Dated decision plus current served/index pointers; preserve history. Extra document and numbering reconciliation remain. |
| OPTIONS repair marketed as conformance | Explicit additional modern MUST mismatch under accepted 0043; retain C1–C6 and four owed client receipts. |
| Limiter test proves only a helper | Keep monotonic unit proof plus production-router real-connection witness with live transient observers. Shared contention/bursts remain. |
| New tests introduce timing/coverage flakiness | Reuse causal seams and suites; 0020 placement; no sleeps, mid-band chaos, global resets or static calendar-dependent TLS success fixtures. |
| Integrity remains HIGH or provider unavailable | Independent evidence and hard stops; no inference from source merge, alias or compile. Delivery may remain blocked. |
| Missing pinned engine or unavailable proof | Record exact path, expected digest/command and outcome; no installs, substitutions or weakened assertions by verify. |
| Provisional numbers collide | 0104 is local only; do not renumber 0090/0103 or reuse reserved holes. Operator reconciles final numbers. |
| Eval is locally serial but host scope unknown | Do not launch/claim full eval. Require authorized host-wide participation and cleanup-scope proof separately; #201 remains unresolved. |
| Scope expands during repair | Minimal existing owner and complete gates; return blocker for accepted-contract contradictions instead of enlarging authority. |
