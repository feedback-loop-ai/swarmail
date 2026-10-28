# Intake — issue #152 MCP revision negotiation

Historical record of the earlier negotiation/source-reconciliation runs.
Current commission and C1–C6 disposition follow [spec.md](spec.md) and the
[operator-directed register](reconciliation.md#exact-conflict-register).
Earlier prospective repair/defer wording and gate receipts do not authorize
runtime changes or prove this reframing candidate.

Date: 2026-10-26. Run: `epic-164-issue-152-negotiate-mcp-95b27901`.
Disposition: resolved — framing only; implementation and feature proof remain.
Inspected base: `f49d122`.

## Goal

Negotiate MCP 2025-03-26, 2025-06-18 and 2025-11-25 per client, preserving today's 2025-03-26 behavior including batches while applying each later revision's transport/lifecycle rules and refusing its batches before effects.

## Authority and bounded evidence

The issue packet's README and operator rulings were read first; both SHA-256
values match its SHA256SUMS. The README contains no prior-run outcomes.
The candidate-local engine matches the required digest
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
Read the house charter, decision index, frozen 0001–0003, 0029, 0044,
0047's RPC material and 0062. Spot-checked mcp::dispatch, classify, reply_id,
handle, McpContext, api::mcp_endpoint, stdio's HTTP bridge, tests/mcp.rs,
tests/p2.rs, the pinned verifier and local owner history. Relevant history:
`14dee3c` (RPC residual repairs), `3ed2fbb` (object-only REST admission).
Current handle always returns PROTOCOL_VERSION = 2025-03-26; dispatch admits
bounded arrays without revision context, the endpoint constructs a fresh
McpContext per HTTP request, and the bridge currently sends no revision
header. These are design surfaces, not a prescribed session architecture.

The operator's 2026-10-26 direction authorizes delivery now; #151's baseline
is an operator-owned merge prerequisite, not a build, design or test input.
Do not run baseline trials. Deliver #152 before #153; structuredContent and
outputSchema belong to #153. The story metric, to carry into spec.md, is
**compatibility: the client compatibility matrix**, not throughput or an
agent-score improvement. The matrix names Claude Code, Codex, MCP Inspector
and one explicitly named additional client.

The packet's older next-free ledger ends at 0077; the current decision index
already contains 0077 and 0078. The task explicitly reserves **0079** for
this work. Design must record the revision-qualified extension to accepted
0029/0044/0047 in a new proposed decision, without accepting it or rewriting
accepted historical bodies. The explicit operator instruction supplies the
narrow authority for revision negotiation and later-revision batch removal;
it does not authorize weakening RPC edge contracts or unrelated semantics.
No additional operator acceptance is inferred from other stories' rulings.

## Acceptance criteria for design and implementation

1. **Pinned negotiation.** A checked-in table in the specification names all
   three revisions and gives exact initialize request/response expectations,
   unsupported-version fallback, omitted/wrong-typed version handling,
   post-initialize version selection and per-transport lifecycle/header
   requirements. Real HTTP and binary stdio tests assert complete relevant
   status/header/RPC/version fields for each row. Supported versions negotiate
   themselves; an unsupported version receives a supported version according
   to MCP negotiation, rather than an unsupported echo or an invented RPC
   error. The chosen supported fallback is explicit and tested. Preserve
   legacy omitted-version behavior where the existing contract permits it.
2. **Revision isolation and lifecycle.** A subsequent request uses the
   negotiated revision according to the transport's rules; it cannot inherit
   another client's negotiation. Interleave clients using different revisions
   and prove no cross-client change. For 2025-06-18 and 2025-11-25, pin the
   exact required HTTP headers, version-header default/refusal behavior and
   initialized lifecycle in the table and real-wire tests, including missing,
   malformed, unsupported and conflicting version inputs and requests before
   initialization. Do not impose new lifecycle restrictions on existing
   2025-03-26 clients. The bridge propagates the applicable negotiated metadata
   on later POSTs and preserves protocol-only stdout. Any optional session
   mechanism must be documented with its lifecycle; optional spec features
   are not made mandatory by intake.
3. **Batches only for 2025-03-26.** Existing legacy tests stay green: cap 16,
   empty/over-cap refusal before effects, concurrent waits, input reply order,
   duplicate/null reply positions, notification omission and effectful
   all-notification silence. Under each later revision, nonempty, empty,
   mixed and all-notification arrays are refused as a whole before any member
   executes or registers a waiter. Pin the refusal status/error envelope in
   the specification and assert it on HTTP and through the bridge. Include
   a batch containing destructive/seed calls against populated state and
   prove exact state preservation. A modern refusal must not be expanded
   into misleading per-member transport errors by legacy classification.
4. **Annotations.** tools/list for the later revisions includes readOnlyHint
   on read tools and destructiveHint on clears/deletes, with an explicit
   per-tool expected mapping and regression assertions. Existing tool names,
   argument schemas, result envelopes and 2025-03-26 behavior remain
   compatible. This change adds neither outputSchema nor structuredContent.
5. **RPC edge preservation.** Preserve 0029/0044 and 0047's dated supplied-ID
   amendment (recorded in proposed 0062): malformed JSON/UTF-8 is HTTP
   400/-32700/null; body overflow retains HTTP 413/-32600/null; invalid
   objects/nonobjects and Boolean/object/array IDs refuse -32600/null before
   effects; valid string/number/null IDs retain correlation. Missing ID plus
   string method remains a silent notification; unknown methods/tools retain
   -32601; argument staging retains -32602 and the established isError split.
   No new blanket jsonrpc-member/params validation, integer-only ID policy,
   numeric canonicalization, bridge framing/deadline/size relaxation or
   unrelated admission limit follows from version support. Existing edge
   suites remain green; new checks exercise these contracts in each revision
   where applicable, with explicit precedence where header refusal applies.
6. **Compatibility evidence ownership.** The spec names the compatibility
   metric and a four-client matrix with client/version, negotiated revision,
   transport, initialize/tools-list/tool-call outcome, evidence location and
   pass/fail. CI or recorded evidence must show all four clients green before
   issue-level compatibility is claimed. Real client execution/network access
   is CI/operator-owned; local wire fixtures prove server conformance and
   cannot be relabeled as those clients. Missing client artifacts remain
   explicitly unproven, not a reason to fabricate a local green matrix or
   block intake. The packet provides the required client names but no green
   matrix receipt or pinned normative revision snapshots; design must state
   the exact revision clauses behind its table and name any missing local
   source it actually needs. No external retrieval is authorized for this run.
7. **Decision, scope and proof.** Design supplies spec.md, plan.md, tasks.md
   and proposed decision 0079 with exact revision-qualified amendments and
   tests mapped to criteria 1–5. Keep 0079 proposed and index it accordingly.
   Preserve the current bundles/routing, tooling pin, dependency policy and
   exact coverage denominator. Commit work unsigned; do not push, merge,
   tag, publish, close issues or accept decisions. The operator keeps this
   PR unmerged until #151's baseline exists. Every local house gate below
   must pass on the committed implementation before delivery success; an
   intake gate run proves only this framing's unchanged runtime baseline.

## Frozen guarantees and test placement

Nearest are 0001 (MCP must see mail immediately after SMTP 250) and 0002
(the existing wait tool subscribes before scanning); negotiation/refusal
must occur before tool effects and may not introduce a store visibility gap
or a different wait algorithm. 0003 oldest-only pruning remains protected;
no storage, persistence or pruning redesign is in scope. Extraction remains
at ingest under 0005. This feature changes the MCP surface, not these
semantics. Retain real protocol/server tests (0004), fresh test inboxes,
deterministic synchronization and 0020 placement; no mock loop, sleeps as
synchronization, coverage exclusions or weakened assertions.

## Verify commands

Run focused real-protocol suites on the committed implementation:

```bash
cargo test --locked --test mcp
cargo test --locked --test p2 mcp_
cargo test --locked --test stdio
cargo test --locked --lib mcp
```

Design assigns stable names to new revision/lifecycle/isolation tests; each
must be included above, with owner-local seams only when needed. No cargo
bench is needed: compatibility, rather than speed, is this story's metric.

Every house gate is required. Following 0050, verify first checks the public
shims and executes the pinned bundle copies against the candidate. The
coverage commands below are the underlying proofs of the public
scripts/coverage-gate.sh entry points, including its selftest:

```bash
sha256sum .forge/bin/brokkr
# Require ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f.
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
```

Also prove native-tls absent, failing closed unless Cargo explicitly reports
that the package does not match (a generic Cargo failure is not absence):

```bash
out=$(cargo tree --locked -i native-tls 2>&1); code=$?
if [ "$code" -eq 0 ]; then
    exit 1
elif printf '%s' "$out" | grep -q 'did not match any packages'; then
    :
else
    exit 1
fi
```

Verification is offline and installs nothing. Exact coverage requires zero
missed lines, branches and functions, plus a green selftest. Preserve the
recipe's protected review and security hold; intake dispatches no reviewers.
CI owns platform runs including macOS, Helm/chart bump/history, Docker smoke,
network-backed RustSec and the real port-80 bridge exchange; the operator
owns live branch protection, baseline/client provisioning and sealing/merge.
These are not added as local acceptance claims.

## Intake deliverable

This phase records only this framing and its gate outcomes; it does not
implement version support, author the design decision, claim a green client
matrix or select the next phase. Gate logs are retained under the run's
local intake-gates evidence directory; failures remain failures and do not
turn into a feature-completion claim.
