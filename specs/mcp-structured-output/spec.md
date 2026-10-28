# Structured MCP tool outputs

Date: 2026-10-26. Status: implemented locally; delivery review pending. Epic #164, issue #153.
Scope: this Swarmail crate, building on #152's shipped, unmerged candidate.

Decision: 0082 — [Revision-qualified structured outputs](../../docs/decisions/0082-mcp-structured-output.md), proposed.

## Feature and rationale

For MCP 2025-06-18 and 2025-11-25, every tool advertises an object-root
`outputSchema` and every successful call returns conforming `structuredContent`
alongside its existing JSON text block. Clients can consume typed data without
recovering a contract from prose. Explicit 2025-03-26 and omitted-version clients
retain today's descriptor and result bytes. Future revisions must deliberately
select an output contract when added; no date-string comparison enables them.

The epic measures **tokens-per-success** and **invalid-argument/tool-misuse
rates**, with the issue's invalid-argument errors and turns-per-task context.
Schemas and duplicated results can increase tokens. Local protocol tests prove
representation and compatibility, not metric improvement. The operator's #151
baseline and subsequent client matrix own measurement and actual-client receipts.
#152/#153 PRs stay unmerged until #151's baseline exists. No paid trials or
proposal acceptance belong to delivery.

## Authority and evidence

The supplied issue153 README and operator-rulings were read and verified from
their directory with SHA256SUMS. Their respective SHA-256 values are
`b5cf69a9bfe6e704a57dfed5448b19dc92e97a75f9a80b475e8f6da11d6b958f` and
`b68560f6782247377e75e90698f95f33feef390cdbdb71ddd8c9a3cc0fbdfd9e`.
The packet's prior-runs section is empty. The run-local tasks directory is absent;
[tracked intake](intake.md) and the verified packet supply framing. Both run-local
panel positions were read; their explicit synthesis is below.

Preserve the operator's 2026-10-26 retention of C1–C6 in proposed
[0080](../../docs/decisions/0080-mcp-source-reconciliation.md) and its
[exact register](../mcp-version-negotiation/reconciliation.md#exact-conflict-register).
[0079](../../docs/decisions/0079-mcp-version-negotiation.md), 0080 and 0082 remain
proposed. This additive contract supersedes #152's temporary statements that
structured outputs are absent when implemented; it changes none of its deviations.

The tracked source ledger is available. Upstream 2025-06-18 and 2025-11-25
`spec/server/tools.mdx`, `schema/schema.ts` and `schema/schema.json` snapshots from
the earlier packet's `mcp-spec/` are absent. Use the supplied contract and recorded
clauses without claiming a fresh upstream-byte audit. No GitHub, CI, journal,
sibling-worktree or network evidence is required here.

## User scenarios

1. A modern client lists tools, calls any tool and validates structured output
   against its returned schema, including a successful wait timeout.
2. A text-only client completes the same workflow using only `content`; adopting
   a structured reader is optional and existing text serialization remains intact.
3. March and modern clients share a server without changing each other's outputs;
   the binary stdio bridge forwards the selected representation.

## Functional contract

Use the existing immutable exchange `Revision`. HTTP selects it per request;
a missing header means March even after a different request negotiates modern.
The bridge keeps its negotiated revision. Preserve names, inputs and annotations;
description changes are restricted to aligning the documented output contract.
The March catalogue literal and input validation ownership remain intact.

Every successful path uses one formatter with the original owned JSON value.
Serialize it once into the existing single text block and add that same value as
`structuredContent` only for June/November. Keep `isError:false`. No parse-back,
new wrapper, re-fetch, retry or runtime schema validation follows a mutation.
Notifications still execute admitted effects and answer nothing. `isError:true`,
JSON-RPC errors and transport refusals retain shapes, text and taxonomy and gain
no success-shaped structured payload.

### Output inventory

Every name has the unchanged `swarmail_` prefix. All listed outer fields are
required. All owned output objects, including nested Email/address/chaos objects,
are closed with `additionalProperties:false`. Schemas are self-contained; local
references must carry every transitive definition, with no remote resolution.

| Tool suffix | Success schema |
| --- | --- |
| search_emails | inbox:string, total:nonnegative integer, emails:Email[]; empty pages allowed |
| get_email | Email directly, without another wrapper |
| get_latest_email | emails:Email[], minItems 1; empty inbox remains an error |
| delete_email | deleted:boolean const true |
| clear_inbox | removed:nonnegative integer, including zero |
| clear_all | removed:nonnegative integer, including zero |
| wait_for_email | Object-root oneOf: found has matched/total nonnegative integers and emails:Email[] with minItems 1; timeout has matched/total nonnegative integers and timed_out:boolean const true |
| extract_links | links:string[], empty allowed |
| extract_codes | codes:string[], empty allowed |
| seed_email | id:string, links:string[], codes:string[]; arrays always present |
| set_chaos | connect, mail_from, rcpt, data: each null or output ChaosRule |
| clear_chaos | chaos:string const "disabled" |

Wait alternatives are individually closed: hybrids, absent discriminators and
`timed_out:false` fail validation. Do not assert total equals matched or array
length: total is read separately after waiting and concurrent changes are legal.
Set_chaos likewise reports current configuration after its update, not an atomic
transaction spanning that update and the subsequent snapshot.

Email requires id:string, inbox:string, envelope_from:string|null,
to:EmailAddress[], received_at:string, received_ms:integer and size:nonnegative
integer. received_ms may be negative. Optional from is EmailAddress; optional
subject, text, html, message_id and in_reply_to are strings when present. Optional
cc:EmailAddress[] and recipients/links/codes/references:string[] follow serializer
omission of empty arrays. EmailAddress requires address:string and permits
name:string when present. Omission differs from null. envelope_from null retains
unknown historical provenance; empty string retains the known null reverse path.
Raw is excluded. Do not add UUID/date/URL formats or a 64-code cap to stored output.

Output ChaosRule requires probability:integer 0..100 and delay_ms:integer
0..300000; error is optional string, omitted when absent. Existing SMTP reply-line
validation stays; do not invent character length constraints for an octet limit.
All four ChaosConfig keys are emitted, null for unset events. Input omission and
default rules remain unchanged.

### REST/MCP relationship

Keep production schema literals beside the MCP catalogue. Tests fetch served
`/openapi.json` and independently compare resolved structural constraints using
these named projections, checking source shapes before applying differences:

| Source | Explicit output projection |
| --- | --- |
| EmailAddress | Require address; remove null from optional name; close object |
| Email | Require the always-emitted set above; remove null from optional subject/text/html; retain envelope_from null; size minimum 0; close object; recursively project addresses |
| EmailList | Require total/emails, total minimum 0, project Email and close wrappers; search adds required inbox; latest selects emails with minItems 1 |
| AwaitResponse | Require matched/total/emails, counts minimum 0, project Email with minItems 1 for found; MCP timeout replaces emails with required timed_out const true; close each arm |
| Email id/links/codes properties | Seed selects these and requires arrays; extraction selects/requires its array; close wrappers |
| ChaosConfig/ChaosRule | Require all event keys and rule probability/delay_ms; retain bounds, null event alternatives and closedness |
| No shared response schema | Independently pin deleted:true, removed:nonnegative integer and chaos:"disabled" to this inventory and real replies |

Compare properties, types, requiredness, nullability, items, bounds, closedness
and resolved references, ignoring only enumerated annotations. Changes outside
these projections fail. No general translator, generator or public MCP-only REST
components are added. OpenAPI edits are descriptions only: shared response/request
schemas are not tightened merely to make equality possible.

## Panel synthesis and explicit rulings

| Question | Ruling and losing alternative's cost |
| --- | --- |
| Result ownership | Adopt both: existing tool_call/formatter, immutable Revision, one owned value. Reject reply decoration and text parse-back; those add fallible interpretation after effects and can misclassify errors. |
| Schema architecture | Adopt simplicity's existing-file mapping/shared fragments and no dependency. Reject frameworks, generators and runtime validation, avoiding extra owners/coverage/failure paths. Accept finite handwritten duplication. |
| Schema strength | Adopt robustness's closed objects, mandatory serializer fields and discriminating wait alternatives. Reject permissive schemas and optional-null MCP allowances copied merely from REST; they lose malformed-output detection. Accept explicit projections and deliberate future schema evolution. |
| OpenAPI accuracy | Choose simplicity's projections over robustness's preferred response-schema correction. Tightening shared definitions risks chaos request defaults. The losing correction leaves REST descriptions looser than actual serialization; maintained projection proofs are the chosen cost. |
| Validator | Adopt the bounded test-only evaluator with negative controls and unsupported-key/reference refusals. Reject a new validator dependency for this finite vocabulary; accept evaluator maintenance and a limited validity claim. |
| Proof breadth | Adopt robustness's real-bridge size/error proofs in addition to simplicity's all-tool workflows. Happy paths alone lose the new cap-crossing failure. Reuse existing storage/SMTP/TLS witnesses; infrastructure redesign adds cost outside this representation change. |
| Governance/metrics | Adopt both: retain text, limits and C1–C6; leave measurements/client receipts with the operator. Removing text, raising caps or starting paid trials trades away commissioned compatibility and governance. |

## Acceptance Criteria

- **AC1 — Catalogue:** each modern tools/list returns exactly these twelve names,
  each with a non-vacuous object-root schema. Schema inventory equals the legacy
  name inventory; inputs/annotations stay unchanged. Every reference resolves
  locally without network access.
- **AC2 — Successes:** for each modern revision, real HTTP and binary stdio
  workflows execute all twelve tools and validate every success against that
  transport's returned schema. Cover empty/nonempty search/extraction, get/latest,
  seed/delete, zero/nonzero removals, both waits, optional Email members, historical
  null envelope_from and configured/unset chaos. Fixed stored Email fixtures also
  prove signed timestamps and historical code arrays.
- **AC3 — Negative proof:** independent controls reject missing required fields,
  wrong scalar/array/nested types, extra raw/properties, bad literals/bounds and
  wait hybrids/missing discriminators. Unknown assertion keywords, malformed
  schemas and unresolved/nonlocal/cyclic references fail closed. Valid optional
  omissions and null envelope_from pass.
- **AC4 — OpenAPI pin:** fetch served OpenAPI, resolve references and compare all
  constraints with only the explicit projections above. Mutated nested type,
  requiredness, nullability and bound controls fail the pin. Tool-specific wrappers
  also have independent literal assertions.
- **AC5 — Compatibility:** a consumer reading only content completes every tool
  workflow on March, June and November. Modern parsed text equals structuredContent,
  with literal text serialization assertions. Explicit March and omitted-version
  catalogue/results match pre-change dc67f4a bytes for all tools, both waits,
  isError and RPC failures. Substitute only identified dynamic IDs/timestamps;
  never reserialize expected JSON. Quoted/control/Unicode fixtures expose escaping,
  ordering, omission and null differences.
- **AC6 — Isolation/refusals:** interleave legacy/modern requests on one server
  and use independently negotiated real bridge processes. Existing malformed-input,
  not-found, storage-failure, shutdown, rate, notification, negotiation and C1–C6
  witnesses keep taxonomy/text; modern errors have no structuredContent. Refusals
  gain no new effects/subscriptions; admitted notifications stay effectful/silent.
- **AC7 — Guarantees:** real SMTP acceptance/readback and existing deterministic
  wait-order, cap/pruning, extraction and mutation-boundary regressions stay green.
  Both representations come from one captured payload; no new Store read, wait,
  extraction or webhook delivery dependency is introduced.
- **AC8 — Size cost:** real bridge tests show valid dual output below the existing
  10 MiB response cap and the existing -32000 refusal above it. Include a query
  that fits as March text but exceeds the cap when duplicated in modern mode.
  No partial success, truncation, replay or cap increase is permitted.
- **AC9 — Documentation:** served descriptions and #152's structured applicability
  rows explain the revision-qualified contract and size cost. Preserve the exact
  C1–C6 register, bounded conformance claim, proposal statuses, #151 merge hold and
  external client/metric ownership. No new tracked link into run-local state;
  routing and accepted decisions stay unchanged.
- **AC10 — Proof:** all local house gates, exact coverage/selftest, design/delivery
  proof selftests, shim/engine checks and fail-closed native-tls absence pass on the
  committed candidate. The design check identifies these three artifacts and
  proposed 0082 from HEAD. Design-time green gates do not complete implementation.

CI owns macOS, privileged port-80, chart/history, Docker and network RustSec proofs.
The operator owns live settings, external clients, measurements, baseline, sealing
and merge. Those are external obligations, not local criteria requiring unavailable
services, namespaces or paid calls.
