# Intake — issue #152 normative source reconciliation

Historical record of the earlier negotiation/source-reconciliation runs.
Current commission and C1–C6 disposition follow [spec.md](spec.md) and the
[operator-directed register](reconciliation.md#exact-conflict-register).
Earlier prospective repair/defer wording and gate receipts do not authorize
runtime changes or prove this reframing candidate.

Date: 2026-10-26. Run: `epic-164-issue-152-verify-mcp-ve-998279d4`.
Disposition: resolved — actionable framing; feature implementation and its proof
belong to subsequent seats. Inspected candidate: `920ad67`.

## Goal

Reconcile swarmail's MCP 2025-03-26, 2025-06-18 and 2025-11-25 implementation
against the pinned official lifecycle, transports, message schemas and tools
texts, fixing every in-scope deviation with a behavioral failing test first
while retaining the operator's evidence obligations and decision boundaries.

## Authority and bounded inspection

Read the issue packet README and operator rulings first. All 118 entries in
its SHA256SUMS pass when checked from the packet directory, including those
two documents, the prior review, SOURCE.txt, and all three revision snapshots.
The README ends at its prior-runs heading without outcome entries; the supplied
review of `epic-164-issue-152-negotiate-mcp-95b27901` supplies the prior run's
REVIEW-RESIDUAL-OK outcome. The source is
`modelcontextprotocol/modelcontextprotocol` commit
`dae694f61a31f7a3ec2af3aff709e11f70f61e48`, as recorded in SOURCE.txt.
The packet remains local evidence; no tracked hyperlink targets run-local state.

Read the house charter and decision index, frozen 0001–0003, proposed 0079
and 0062, and the applicable RPC/transport material of 0029, 0039, 0043,
0044 and 0047. Inspected prior intake, local history (`f15183b`, `3ec23ef`,
`0464d1e`, `920ad67`), the pinned verifier and recipe inventory. Spot-checked
`Revision`, `header_revision`, `negotiated_revision`, `classify_revision`,
`dispatch_with_headers`, `handle_revision`, `tool_descriptors`, the API route,
stdio `request_text`, `read_line`, `Lifecycle::admit/complete`, and the named
revision integration tests. No new implementation audit is claimed here.

The review's missing normative-source residual now has local inputs. Its AC3
client receipts remain missing and operator-owned. Existing code selects the
revision per HTTP request, uses a bridge lifecycle fence, refuses modern arrays,
and projects modern annotations. The two modern header clauses both require
subsequent client headers, recommend March when neither a header nor another
version identifier exists, and require HTTP 400 for invalid/unsupported values.
These spot-checks establish the framing surfaces, not full conformance.

Keep 0079 proposed. The explicit task authorizes revision reconciliation and
compatible repairs; it does not accept any decision or authorize unrelated
changes to accepted contracts. Where the sources expose a contradiction with an
accepted ruling, document the exact clause and ruling in the proposed decision;
do not silently override it or claim that unresolved contradiction conforms.
Keep the prior operator-owned acceptance/merge prerequisites recorded. The
operator's #151 baseline hold remains; it is not an intake/build prerequisite.

## Checkable acceptance criteria for design and implementation

1. **Complete reconciliation ledger.** Check in a ledger for each of the three
   revisions covering Basic Lifecycle, Basic Transports (including Streamable
   HTTP and Protocol Version Header), Basic JSON-RPC messages, Server Tools,
   and their relevant schema.ts/schema.json definitions. Each normative clause
   has revision, source-relative path plus heading/line or schema symbol,
   obligation level and actor, applicability, implementation owner, outcome
   (conforms, repaired, inapplicable with reason, or an explicitly unresolved
   accepted-contract conflict), and named proof. No applicable requirement is
   left without a disposition; June is not used as a proxy for November.
   Record source commit and snapshot hashes so the ledger's provenance can be
   rechecked locally. Remove stale claims that these snapshots are missing.
2. **Failing test before each repair.** Every identified in-scope implementation
   deviation has a named regression test and a retained command/result showing
   a behavioral failure on the pre-fix candidate, then success after repair.
   Compile errors or rewritten expected values alone do not qualify. Map each
   repaired ledger row to its test and evidence. Any accepted-contract conflict
   requiring further operator acceptance stays explicitly unresolved, with the
   relevant proposed amendment; no new acceptance is inferred. No claim of full
   conformance is permitted while an applicable unresolved deviation remains.
3. **Lifecycle and revision pinning.** Real HTTP and real binary stdio tests
   cover the ledger's initialize request/result shapes, supported-version echo,
   unsupported-version fallback, omitted/wrong-typed version handling,
   capability declarations, initialized notification and operation ordering,
   reinitialization/failure behavior, and shutdown obligations that apply to
   this implementation. Cover all three negotiated revisions and interleaved
   clients; later exchanges cannot inherit another client's revision. Derive
   exact expected status, headers and RPC fields from the cited clauses and
   existing accepted compatibility rulings, distinguishing client obligations
   from required server refusals. The omitted-version binary leg must be
   exercised as well as HTTP. Negotiate only the three 2025 revisions;
   `2026-07-28` is outside the supported set (test that it is not echoed as a
   supported selection or accepted as a supported version header).
4. **Streamable HTTP and JSON-RPC shapes.** Assert the applicable clauses for
   POST request/notification/response handling, response media types, GET's
   permitted SSE-or-405 behavior, Origin guards, and HTTP version headers.
   Exercise missing, valid, malformed, duplicate, unsupported and conflicting
   header/body versions, initialization versus subsequent exchanges, refusal
   precedence, and the bridge's actual subsequent request bytes. Retain
   stateless HTTP unless a cited mandatory obligation requires a change;
   optional sessions, SSE streaming and resumption are not made mandatory by
   intake. Reconcile request/response/notification fields and schemas,
   correlation and silence explicitly. Preserve accepted ID-domain and RPC
   error contracts unless a scoped, authorized amendment applies; sender-side
   MUSTs do not automatically require blanket server-side validation.
5. **Batches and effects.** Both modern revisions refuse every top-level array
   before executing any member or registering a waiter, including empty,
   singleton, nested, mixed, all-notification, cap-sized and over-cap inputs.
   HTTP and binary stdio assert the precise refusal envelope; bridge local
   refusals must cause no upstream exchange. Use populated mail and chaos
   state, destructive calls and live mutation/subscription witnesses to prove
   zero transient effects. March retains cap 16, concurrent members, ordered
   reply positions, duplicate/null ID handling, effectful notification silence,
   and empty/over-cap refusal. Existing real-protocol positive controls remain.
6. **Tools and public contracts.** Reconcile tools capability, tools/list and
   tools/call schemas, input schemas, text result shapes, isError/protocol error
   split and annotations against each revision. Assert the complete expected
   per-tool readOnlyHint/destructiveHint mapping, including seed and chaos
   mutations, preserving legacy compatibility. Do not add structuredContent or
   outputSchema: those optional facilities belong to #153. Correct the review's
   stale modern wait description and served OpenAPI revision/batch/header/error
   claims. Add scoped dated provenance pointers for 0029, 0039, 0044 and 0047
   where 0079 qualifies their historical text; preserve accepted bodies and
   statuses, and label proposed changes accurately.
7. **Evidence ownership and delivery discipline.** The four-client matrix
   continues to mark actual Claude Code, Codex, MCP Inspector and Claude
   Desktop receipts as owed, with client/version, candidate, transport,
   revision and initialize/list/call result fields. Wire fixtures cannot fill
   those receipt cells. Preserve routing under bundles/, the engine pin,
   dependency policy, exact coverage denominator and protected review/security
   hold. All local gates below must pass on committed implementation before
   claiming delivery proof. Intake records only framing and its own gate
   outcomes, never completed reconciliation or a green client matrix.

## Frozen guarantees and scope

0001 is adjacent: MCP reads after SMTP 250 must see the stored message.
0002 is adjacent: the wait tool must still subscribe before scanning, and
modern refusal must precede subscription. 0003 remains protected: seed can
trigger oldest-only pruning, but negotiation cannot change pruning or delivery
visibility. Preserve extraction-at-ingest (0005), ordered mutation/refusal
boundaries, body/deadline limits, Host/Origin admission and legacy concurrency.
No storage redesign, dependencies, gate changes, routing changes, paid baseline
trials or upstream retrieval are authorized by this framing.

Tests use real servers and the real bridge binary, deterministic synchronization,
fresh inboxes and decision 0020 placement. No mocks in the protocol loop,
sleep-as-synchronization, coverage exclusions or weakened assertions.

## Verify commands and proof ownership

Focused commands (the ledger must name individual regressions within these):

```bash
cargo test --locked --test mcp_revisions
cargo test --locked --test mcp
cargo test --locked --test stdio
cargo test --locked --test p2 mcp_
cargo test --locked --lib mcp
cargo test --locked --lib stdio
```

The metric remains client compatibility. No cargo bench is required for this
protocol conformance task. Run every house gate offline without installing
anything. Under 0050 check the public shims and execute their pinned bundle
implementations, including the exact coverage gate and selftest:

```bash
printf '%s  %s\n' \
  ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f \
  .forge/bin/brokkr | sha256sum --check --strict
sha256sum --check --strict bundles/self/scripts/shims.sha256
export CARGO_NET_OFFLINE=true
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

Require exact zero missed lines, branches and functions, not rounded percentages.
Compile must not emit a policy-sweep warning. Also prove native-tls absent,
failing closed on unrelated Cargo errors:

```bash
out=$(cargo tree --locked -i native-tls 2>&1); code=$?
if [ "$code" -eq 0 ]; then
    exit 1
elif printf '%s' "$out" | rg -q 'did not match any packages'; then
    :
else
    printf '%s\n' "$out"
    exit 1
fi
```

CI owns its platform jobs (including macOS), Helm/chart and history proofs,
Docker smoke, network-backed RustSec and the real port-80 exchange. The
operator owns actual client receipts, live branch settings, #151's baseline,
decision acceptance, sealing and merge. Namespaces/network access are not
added as local proof obligations. Preserve these ownership records and keep
going with workspace evidence; name an exact missing artifact if one is needed.

Commit every phase's tracked work unsigned; no push, merge, tag, publication,
issue closure or decision acceptance. The result file is written through the
workspace tool after this framing is committed and its gate outcomes are known.
