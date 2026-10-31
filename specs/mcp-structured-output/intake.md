# Intake — issue #153 structured MCP tool outputs

Date: 2026-10-26. Run: `epic-164-issue-153-return-struct-cae658a0`.
Disposition: resolved — actionable framing; no feature implementation.
Inspected candidate: `dc67f4a` (#152's shipped, unmerged candidate).

## Goal

For MCP 2025-06-18 and 2025-11-25, publish an outputSchema for every tool and
return schema-valid structuredContent alongside the existing JSON text on every
successful tool call, preserving 2025-03-26 results byte-for-byte.

## Authority and evidence

Read the supplied issue153 README and operator-rulings first and compared both
with that directory's SHA256SUMS. README SHA-256:
`b5cf69a9bfe6e704a57dfed5448b19dc92e97a75f9a80b475e8f6da11d6b958f`.
Operator-rulings SHA-256:
`b68560f6782247377e75e90698f95f33feef390cdbdb71ddd8c9a3cc0fbdfd9e`.
Both match. The prior-runs heading contains no outcomes. These are evidence
locators, not tracked hyperlinks into run-local state.

The operator's 2026-10-26 ruling authorizes #152 then #153 before #151's
baseline, but their PRs stay unmerged until that baseline exists. It explicitly
retains #152's C1–C6 deviations. Neither that ruling nor this intake accepts
0079, 0080 or any further decision. Proposed [0082](../../docs/decisions/0082-mcp-structured-output.md)
records this story's additive output contract; design refines it without
changing accepted semantics. 0081 is not used by this story.

Read the house charter, decision index, 0001–0003, relevant 0029/0039/0044/0047,
0055, 0079 and 0080. Spot-checked src/mcp.rs Revision, handle_revision,
tool_descriptors, text_result, error_result and all tool_call success shapes;
served OpenAPI's Email/EmailList/AwaitResponse/ChaosConfig schemas;
tests/mcp.rs, tests/mcp_revisions.rs and tests/p2.rs; recent local MCP history;
and the bundle verifier's exact gate inventory. No GitHub, CI, journal, sibling
worktree or network read was used. Recipe routing stays as checked in.

The existing tracked #152 reconciliation ledger records modern structured-output
MUST/SHOULD clauses and schema symbols, with source hashes. The actual upstream
snapshots are absent from this supplied workspace packet: specifically the
2025-06-18 and 2025-11-25 `spec/server/tools.mdx`, `schema/schema.ts` and
`schema/schema.json` files under the earlier packet's `mcp-spec/`. Intake can
frame from the supplied issue and tracked ledger; it does not claim fresh
verification of those upstream bytes. If design needs those bytes beyond the
recorded clauses, it must name these missing files, not fetch or fabricate them.

## Checkable acceptance criteria for design and implementation

1. **Complete revision-qualified catalogue.** Real tools/list exchanges for
   both modern revisions return all twelve existing tools, each with a valid,
   object-root outputSchema. Tests compare the tool-name set with the existing
   catalogue, so dropping a tool cannot make the test pass. March, including
   the absent-header default, retains its existing descriptor serialization
   with no outputSchema; existing inputs and modern annotations are preserved.
2. **Every success validates.** For each modern revision, execute every tool
   through real server calls and validate result.structuredContent against the
   outputSchema actually returned for that tool. Cover search (empty/nonempty),
   get, latest, delete, clear inbox, clear all, wait found and successful timeout,
   extract links/codes (empty/nonempty), seed, set chaos and clear chaos. Include
   nullable/omitted Email fields and configured/unset chaos events. Preserve
   today's payload shapes: Email directly for get; existing object wrappers for
   the others. A successful timeout remains a success variant with timed_out,
   rather than being converted into an error or a found-email response.
3. **Validation can fail.** Tests reject representative corrupted successes
   (missing required fields and wrong scalar/array/nested field types), including
   both wait variants. Unconstrained schemas or a validator that accepts every
   value cannot satisfy AC2. Resolve nested schema references locally; no network
   or general remote-schema consumer is commissioned. Design specifies the
   bounded validation approach and any defensible permissively licensed dependency.
4. **REST/MCP schema drift is detectable.** Tests structurally compare shared
   Email, address and chaos definitions and applicable response fields with the
   served OpenAPI document. Compare properties, types, requiredness, nullability,
   items and resolved references as applicable, rather than substring checks.
   List/seed/wait differences get explicit projections; document and model the
   existing MCP timeout shape instead of changing REST or MCP runtime semantics
   to force equality. Tool-specific wrappers must also be pinned. Any necessary
   OpenAPI accuracy correction documents existing behavior and has assertions.
5. **Text-only compatibility is proved locally.** Every modern success retains
   the existing content text block, containing the serialized JSON equal to
   structuredContent. A local text-only harness reads only content and completes
   the existing tool workflows for all twelve tools, including wait outcomes.
   For March and the omitted-version default, exact serialized descriptor/result
   fixtures against dc67f4a cover successes, isError results and RPC errors;
   substitute captured dynamic IDs/timestamps explicitly without changing field
   order, whitespace, escaping, missing/null fields or envelope shape. Modern
   and legacy sessions interleaved on one server must retain their own output
   shape. Exercise real HTTP and the binary stdio bridge; no mocked servers
   replace Swarmail's wire-level success tests.
6. **Errors and accepted semantics stay fixed.** Existing error text, isError
   versus JSON-RPC staging taxonomy, notification silence, mutation-before-reply
   ordering, wait subscription order, oldest-only pruning, extraction at ingest,
   rate limit, transport limits and C1–C6 witnesses remain green. The outputSchema
   contract covers successful tool results, not JSON-RPC/transport failures;
   do not invent success-shaped structured data for failures. Preserve #152's
   negotiation/lifecycle and public bounded-conformance claim.
7. **Documentation, metric and governance agree.** The design specification
   names the epic's **tokens-per-success** and **invalid-argument/tool-misuse
   rates**, retaining the issue's turns-per-task context. It identifies the
   operator-owned baseline/client matrix as the measurement source and does
   not claim a measured improvement from local protocol tests. Update served
   descriptions that currently say no outputSchema/structuredContent, and the
   structured-output applicability rows in the #152 ledger, while retaining
   its exact C1–C6 register. 0079, 0080 and 0082 remain proposed. The #151 merge
   hold and actual-client receipts stay explicit and owed; no paid trials,
   routing edits, decision acceptance, push, merge or issue closure.
8. **Proof binds to commits.** Commit all tracked work unsigned and run every
   local house gate and pinned-verifier selftest below. Exact coverage means
   zero missed lines, branches and functions, without exclusions or a lower
   floor. Record failures honestly; a framing resolution is not feature completion.

## Frozen guarantees and boundaries

0001 is adjacent through read visibility after SMTP 250 and seed replies;
0002 directly through swarmail_wait_for_email; 0003 through seeding under a cap.
This story changes representation only, preserving all three and 0005's
ingest extraction. No Store, persistence, SMTP or pruning redesign is needed.
No contradiction with a frozen guarantee blocks intake. The proposed output
contract does not accept or weaken an existing ruling.

The twelve tools are swarmail_search_emails, swarmail_get_email,
swarmail_get_latest_email, swarmail_delete_email, swarmail_clear_inbox,
swarmail_clear_all, swarmail_wait_for_email, swarmail_extract_links,
swarmail_extract_codes, swarmail_seed_email, swarmail_set_chaos and
swarmail_clear_chaos. Existing owners remain the starting point: src/mcp.rs
catalogue/projection/result helpers, src/static/openapi.json and llms.txt,
tests/mcp.rs, tests/mcp_revisions.rs, tests/p2.rs and tests/stdio.rs.
The design council owns detailed schema construction and test placement under
0020; this intake does not implement those choices.

## Verify commands and ownership

Run offline and install nothing. Verify the public dispatch shims, then execute
pinned bundle script owners under 0050. Candidate-local brokkr must match the
operator's digest; compilation must emit no policy-sweep warning. Preserve the
candidate's recipe routing.

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

Fail closed on native-tls absence:

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

Focused reproduction (also included in the full workspace suite):

```bash
cargo test --locked --test mcp
cargo test --locked --test mcp_revisions
cargo test --locked --test p2
cargo test --locked --test stdio
```

Extend these suites with the output-schema/legacy/text-only assertions above;
existing green tests alone do not prove #153. No cargo bench is required for
this representation change; paid/model metric comparisons belong to the
operator's later matrix. CI owns macOS, privileged port-80, chart/history,
Docker and network RustSec proofs. The operator owns actual clients, live
settings, #151 baseline, sealing and merge. Namespace or network evidence is
not a new local criterion. Intake gate logs are run-local evidence under
`.forge/issue153-intake-gates/`, not feature or client-compatibility receipts.
