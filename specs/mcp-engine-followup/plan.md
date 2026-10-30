# Plan — MCP engine follow-up

## Technical context

One Rust crate, real HTTP/SMTP/binary stdio suites, existing MCP/router/store
owners and an inherited mixed delivery recipe. Baseline is 9317cd1 / tree
8080a66 as fully identified in [spec.md](spec.md). This design is a successor.
No runtime, dependency, lockfile, license, engine, adapter or recipe change is
planned. A successful no-repair implementation phase may leave tracked files
unchanged and record execution evidence in ignored run storage.

The repository supplies only `.specify/templates/spec-template.md`, no plan
template. This plan supplies technical context, constitution check, project
structure, research, data model, contracts, quickstart and risks directly.
There is no remote research or live provider probe in this design phase.

## Constitution check

Preserve accepted 0001–0005, 0043 guard/method precedence, 0045 §8 proposal
boundaries, 0057/0058 protected delivery and 0056 unsigned authorship. Keep 0020
placement and deterministic test disciplines: real protocols, isolated inboxes,
no mocks, no sleep-as-sync or mid-band chaos, generated TLS success fixtures.
Retain every house gate, exact coverage and permissive license policy. Proposed
0079/0080/0081/0085/0101, 0103 and 0104 stay proposed with operator-owned numbering.
This follow-up defines proof work without a new semantic ruling or ADR.

## Research and implementation approach

1. Compare the two actual previous results with current intake and both positions.
   The prior missing-engine observations were true then. The current local digest
   and fresh compiles establish the changed setup premise; old tests and host
   compile logs are historical only. Never fabricate a journal or review receipt.
2. Reuse existing tests for AC2–AC7. Verify named tests actually execute and that
   expected catalogues are independent. Require mutation/subscription observers
   with positive controls, the exact-time limiter proof and its router wiring
   witness. Read a failure in its owning path before considering any change.
3. Localize a demonstrated compatible defect and its regression; otherwise leave
   source unchanged. Existing 0104 and served pointers already reconcile this
   candidate. Do not repeat that edit, copy the historical ledger or allocate
   another decision to narrate successful engine provisioning. New semantic work
   needs scope/authority reconciliation, not an incidental repair.
4. Freeze the final commit/tree and run the complete current house inventory,
   focused obligations and safe authorized delivery proof. Use the separately
   dispatched verifier's pinned resources; public shims are checked, not trusted
   to select executable proof code. Record real commands, exits and limitations.
   Any later tracked edit invalidates the prior candidate binding and requires
   fresh final-candidate verification/review. Never relax a proof to conceal an
   unavailable protected boundary or reproduce the inherited integrity attack.
5. Inspect effective route membership/model identity and preserve the inherited
   five roles. Actual protected invocation belongs to the engine's review phase,
   not chief-launched agents or ad hoc provider calls. Carry HIGH integrity and
   unknown facts honestly; required unavailable/failed evidence blocks delivery.
6. Record final disposition separately from CI/operator obligations. Full standalone
   eval is excluded while #201 coordination remains unproved; do not install a
   private lock and call it host participation. Preserve #151 and four client
   receipts. No release or decision-acceptance action follows.

## Project structure and files touched

| Path | Treatment |
| --- | --- |
| `specs/mcp-engine-followup/{spec,plan,tasks}.md` | Exactly these three tracked files in chief's unsigned design commit. |
| `.forge/evidence/chief-engine-followup/`, phase result paths | Ignored command receipts/results through workspace hands; never staged. |
| `tests/mcp_revisions.rs`, `tests/stdio.rs` | Execute current method/version, catalogue, lifecycle and served-description witnesses; edit only for an evidenced regression gap. |
| `src/api.rs`, `src/mcp.rs`, `src/stdio.rs` | Preserve router/dispatch/lifecycle owners and inline/harness tests; no speculative production edits. |
| `src/store.rs`, `src/smtp.rs`, `src/extract.rs`, `src/webhook.rs`, storage/burst/wait suites | Prove guarantees via existing workspace suites; retain commit, extraction and nonblocking fan-out ownership. |
| `tests/delivery_recipes.rs`, `tests/delivery_constitution.rs`, mixed recipe and inherited offices | Inspect/test routing; no provider, permission or fallback changes. |
| Served OpenAPI/llms.txt, historical specs, decision index and 0104 | Current correction already integrated; preserve, check existing served-contract assertions. |
| Canonical gate scripts, public shims, tooling record and dependency files | Verify with existing policy; no planned modifications or allowlist widening. |

## Data model and contracts

No API, persisted field or runtime model is added. Revision belongs to each
exchange, lifecycle fences to the bridge owner, descriptors to `TOOLS`, the
fixed rate window to one router and derived analysis to its existing cache.
Mail remains authoritative after successful commit; a lost reply does not undo
a mutation, and best-effort webhooks do not create an outbox or exactly-once
promise. Subscribe-before-scan applies on every wait entry/re-entry; pruning
retains insertion ordering and does not select an in-flight parse.

Evidence uses existing run-local formats: command/invocation, actual commit/tree,
exit/verdict, relevant artifact hashes and explicit limits. Missing facts remain
unknown. Historical source corrections stay in 0104; AC2–AC6 restate required
observations without changing the negotiated protocol or operator-retained C1–C6.

## Quickstart and validation

Through the bounded workspace tool, inspect clean status and commit/tree, then
compare `.forge/bin/brokkr` SHA-256 to `bundles/self/tooling.json` and the literal
in AC1. Run the absolute candidate-local binary for both commands below. The
now-provisioned executable supersedes the older plan's setup observation; no
fetch, install, substitute or engine pin change is needed or authorized.

```bash
git status --short
git rev-parse HEAD 'HEAD^{tree}'
sha256sum .forge/bin/brokkr
sha256sum --check --strict .forge/evidence/previous-missing-engine/SHA256SUMS
sha256sum --check --strict bundles/self/scripts/shims.sha256
"$PWD/.forge/bin/brokkr" compile --bundle bundles/self
"$PWD/.forge/bin/brokkr" compile --bundle bundles/mixed-opus-terra
cargo test --locked --test mcp_revisions
cargo test --locked --test stdio
cargo test --locked --test delivery_recipes
cargo test --locked --test delivery_constitution
cargo test --locked --lib revision_admission_has_no_transient_mutations_or_waiters
cargo test --locked --lib tool_rate_uses_exact_monotonic_windows
cargo test --locked --lib exhausted_tool_rate_refuses_before_effects_and_keeps_control_available
cargo test --locked --lib the_router_owns_one_tool_rate_across_connections_revisions_and_members
```

Filtered commands must show the intended nonzero test count. These diagnostics
are not a second gate inventory: the complete authoritative inventory is
[house rules](../../docs/house-rules.md#gates--part-of-every-change). The final
hermetic verifier runs its pinned chain with required locked commands, exact
coverage and selftest, publish/seal proofs, licenses, shim integrity and
fail-closed native-tls absence. Retain intake's design-check selftest and delivery
proof obligations; any unsafe-to-run obligation remains explicitly blocking.
No full application gate result is claimed by this design-only phase.

Chief leaves its commit last, touching exactly this spec directory. Repository
`bundles/self/scripts/design-check.sh check .` and its selftest provide local
structural diagnostics. The separately dispatched pinned final check owns the
sequence verdict; chief returns only `drafted` through its designated JSON file.
No extra tracked evidence commit follows the design commit.

## Risks

| Risk | Mitigation and retained cost |
| --- | --- |
| Engine availability mistaken for complete proof | Distinguish fresh compile, application gates and protected review; rerun on final successor. Extra full-proof cost remains. |
| Old results relabeled as current | Record identities/commands/exits and check historical bytes separately; do not copy verdicts. |
| HIGH review-integrity concern survives | Carry unchanged; no attacks/access widening; trustworthy review must be established through authorized path or delivery stops. |
| Required delivery proof crosses prohibited scope | Inspect authorized proof boundary, report unmet obligation without weakening it; no false completion. |
| Blanket middleware regresses OPTIONS | Keep accepted 0043 ordering and matrix tests; document separate modern MUST mismatch. |
| Limiter tests miss transient effects or refill nondeterministically | Reuse causal observers with positive controls and test-owned time plus real router connections; contention/boundary bursts remain. |
| Catalogue silently drops report hints | Independent exact thirteen-name oracle over HTTP and binary stdio; cache work is bounded derived analysis, not mail mutation. |
| Robustness review expands into unrelated hardening | Repair only a demonstrated compatible defect in its current owner; no new services, retries, dependencies or guarantees. |
| Provider outage or correlated review | Exact singleton routes, separate invocation IDs/charters and no unauthorized fallback; five roles still share model families. |
| Proposal collision or implied acceptance | Keep 0101, 0103 and 0104 provisional and existing proposals unchanged; operator reconciliation remains necessary. |
| Private serialization mistaken for #201 host scope | No full eval claim or launch; require participation through cleanup outside this design. |
| Documentation successor inherits baseline evidence | Record successor identity and rerun final proofs/reviews even when only documentation changed. |
