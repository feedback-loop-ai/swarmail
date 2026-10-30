# Intake — issue #152 operator-directed conformance framing

Date: 2026-10-26. Run: `epic-164-issue-152-reframe-the-m-a4a37e5f`.
Disposition: resolved — recorded actionable framing, with implementation reserved
for subsequent seats. Inspected base: `9c0c9d4`.

## Goal

Reframe #152 as three-revision MCP version negotiation with C1–C6 retained as
operator-chosen documented deviations, correcting the conformance attribution
in the specification, reconciliation, proposed 0080 and public descriptions
without changing runtime behaviour.

## Authority and bounded evidence

Read the supplied issue README and operator rulings first, then verified the
entire packet's SHA256SUMS from its own directory: all entries pass. README
SHA-256 is `ca63fae88ee143f0f1b11b828130cb54da58c887ac8ba9a5eaea1df09a8cc952`;
operator-rulings SHA-256 is
`c8fbb03e09d73ec68e215edebfb3aa6b1e5062b16869d4a36c614aefe2c6cbf0`.
The README's prior-runs heading has no outcomes underneath it. Supplied
review-998279d4.md records REVIEW-RESIDUAL-UNSAFE, spec_defect true, no security
residual; its independent member identifies C1's over-attribution. The supplied
2026-10-26 '#152 conformance' ruling resolves the framing: keep behaviour and
record the choice, without accepting 0079 or 0080. No further ruling is needed
or inferred. Packet paths are evidence locators, not tracked hyperlinks.

Read the house charter, decision index, frozen 0001–0003 and relevant portions
of 0029, 0039, 0044, 0047, 0079 and 0080. Spot-checked existing spec/ledger,
README, src/static/llms.txt, src/static/openapi.json, Revision selection,
handle_revision, stdio parse_response, revision_response_disposition and
revision_served_contract; inspected local feature history through 9c0c9d4 and
the candidate's bundle routing/pinned verifier. This is bounded intake, not a
fresh conformance audit. The supplied source/review evidence is not re-derived.
No GitHub, CI, journal, sibling worktree or network access is required.

## Checkable acceptance criteria for subsequent seats

1. **Scope and authority agree.** spec.md, reconciliation.md and proposed 0080
   explicitly identify #152's deliverable as version negotiation for
   2025-03-26, 2025-06-18 and 2025-11-25, with March-only batching, and withhold
   full revision conformance. Each identifies retaining C1–C6 as the operator's
   2026-10-26 choice. Remove current/future instructions to repair those six
   deviations in this run or to await acceptance before retaining them.
   Historical evidence stays recognizably historical. In particular, do not
   claim 0029 §2, 0044 §1 or 0047 §3 mandates C1's HTTP 200 status. Distinguish
   the existing RPC envelope rules from the operator's decision to retain the
   status; retain the source obligation and observable mismatch.
2. **A finite deviation register is inspectable.** For every C1–C6, the spec,
   ledger and 0080 agree on each revision's observable behaviour, source
   obligation/strength and remaining compatibility cost, either directly or
   through an explicit shared table reference. Enumerate March, June and
   November individually; identical rows may say so explicitly. Keep the
   existing clause/source hashes and named witnesses. The register must state:

   | ID | Retained observation to record per revision |
   | --- | --- |
   | C1 | Unsolicited success/error response objects get HTTP 200 and -32600 missing-method RPC errors with carried IDs. March response-only/mixed batches keep per-member replies; June/November arrays instead get the existing pre-dispatch modern-array refusal. No HTTP 400 repair is commissioned. |
   | C2 | String/number/null IDs, including fractional and duplicate slots, retain their accepted values; owned null/fractional-ID outputs and null-ID errors retain their documented schema differences. Distinguish November's optional absent error ID from its disallowed null ID and the March/June required error-ID shapes. No stricter ID admission or history cache. |
   | C3 | Bridge advertises JSON/SSE Accept tokens but parses identity JSON/empty responses only; external SSE and conditional session-ID/404 duties remain unsupported. Paired server remains stateless JSON with GET 405 for valid headers; do not treat that pairing as general client conformance. |
   | C4 | Legacy sender omissions and tolerated invalid envelopes remain compatibility, not conforming sender fixtures. Distinguish missing jsonrpc/client fields or invalid params from receiver obligations, and March's tolerated batched initialize from June/November array refusal. |
   | C5 | Unknown tools retain -32601 rather than the examples' -32602; retain staging -32602 and the existing wait isError split. Preserve the example/SHOULD distinction, including November's clarification; do not promote an example difference into a mandatory violation. |
   | C6 | Complete malformed upstream JSON values remain forwarded on stdio (e.g. missing required envelope fields or result plus error); initialize metadata validation controls state installation only. State the stdout message-conformance cost for all three revisions. |

3. **Public claims are consistent.** README.md, src/static/llms.txt and served
   src/static/openapi.json descriptions say negotiation is supported while full
   revision conformance is not claimed, point to the documented deviations,
   and do not attribute C1's status to a compulsory accepted ruling. Check
   existing feature documentation/pointers for the same current-tense claim;
   correct contradictions without rewriting historical accepted decision bodies.
   Served OpenAPI remains valid JSON and preserves documented statuses, codes,
   headers, batch limits and tool semantics. A documentation assertion may be
   extended to verify the new wording; no behavioural expectation is changed.
4. **Runtime and governance are preserved.** Compare the implementation diff
   with base 9c0c9d4: no executable Rust, request/response schema, dependency,
   lockfile, routing, gate or coverage-policy change. Static description edits
   and documentation assertions/comments may change, with observable behaviour
   and existing behavioural assertions preserved. Both 0079 and 0080, and
   their index statuses, remain proposed. Do not silently accept or amend an
   accepted decision. The operator's direction is recorded within proposed
   0080; it is not acceptance of that document.
5. **Ownership stays explicit.** Original issue AC3 remains operator-owned and
   owed for Claude Code, Codex, MCP Inspector and Claude Desktop: no fixture
   or local gate fills a live-client receipt. Preserve candidate/client version,
   transport, revision and initialize/list/call evidence requirements. #151's
   wrappers and the epic exit matrix determine compatibility impact. Keep the
   baseline merge hold; local documentation completion does not close #152 or
   discharge AC3. CI retains platform/network proofs, and the operator retains
   live settings, baseline, decision acceptance, sealing and merge.
6. **Proof binds to committed work.** Commit tracked changes unsigned, then
   execute all local gates below using the pinned candidate engine and existing
   routing. Record actual exits against commit/tree; exact coverage requires
   zero missed lines, branches and functions. No push, merge, tag, publication,
   issue closure, paid trial or decision acceptance.

## Frozen guarantees and scope

No semantic change is requested, so there is no frozen-guarantee blocker.
0001 is adjacent through MCP reads after SMTP 250, 0002 through wait tools and
pre-dispatch subscription refusal, and 0003 through seed/pruning. Preserve all
three and extraction-at-ingest 0005. No Store/SMTP/pruning/extraction work,
protocol redesign, new dependency, schema feature (#153) or new source audit
belongs to this documentation task. Existing runtime witnesses prove retained
observations, not full revision conformance or live client compatibility.

## Verify gate commands

Run offline, install nothing. Check shim integrity and run pinned bundle copies
under 0050; the public scripts are dispatch shims, not the proof owners.

```bash
export CARGO_NET_OFFLINE=true
printf '%s  %s\n' \
  ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f \
  .forge/bin/brokkr | sha256sum --check --strict
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

Compile must emit no policy-sweep warning. Prove native-tls absence fail-closed:

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

The full test gate includes tests/mcp_revisions.rs and tests/stdio.rs. For focused
review, use `cargo test --locked --test mcp_revisions` (especially
revision_response_disposition and revision_served_contract) and
`cargo test --locked --test stdio`. No cargo bench is required: runtime performance
is unchanged. Check the final diff for AC4 and review the six finite rows against
the verified source/review packet; textual matches alone do not prove attribution.

CI owns macOS, Helm/chart/history, Docker smoke, network-backed RustSec and the
privileged port-80 proof. These are not added as local acceptance criteria.
No local namespace/network proof or live GitHub setting read is requested.
This intake's executed gate receipts are stored locally under
`.forge/issue152-reframe-intake-gates/`; they are not feature implementation or
live-client receipts. Intake resolves only the recorded framing.
