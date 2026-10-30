# Fixture HTTP request metadata

Date: 2026-10-26. Status: draft. Epic #164, issue #151 part 3.

Decision: 0087 — [Fixture HTTP metadata](../../docs/decisions/0087-fixture-http-meta.md).

## Feature and rationale

The fixture-http MCP helper must accept and discard standard `params._meta`
so a Claude Code tool call reaches the public fixture application. The supplied
Claude smoke authenticated and exposed the intended servers, but every
`eval_http` call failed with `-32602 tool_arguments_invalid` because the closed
Call shape omitted this standard field. Preserve the closed shape by adding
this one reserved field. A GET with a supplied body, including `""`, receives a
fixed tool error naming the problem; callers must omit the body.

Implementation changes belong only in `evals/`. This chief's expressly required
artifacts are the three files in this directory and decision 0087. No root
server, dependencies, manifests, locks, routing, scoring or decision acceptance
changes are authorized. This draft is not implementation or baseline success.

## Evidence and authority

The issue151 packet README and operator rulings were read first after the
charter. The whole packet passed `sha256sum --check --quiet SHA256SUMS` from
its own directory. A preceding root-relative invocation used the wrong base;
its path failures are superseded by the successful directory-local check.
README SHA256: `bad2a4ba38b3c879d371b2657e48de29d452502f56ad9c5577d212f4359f824e`.
Rulings SHA256: `5b8da87b897306a313f673b134edd1a56e0f477117a07618e3ec345449a6c6f9`.
The README's prior-runs section is empty. Source evidence is the supplied
`.forge/epic192/issue151/` packet, including smoke-claude.md and mcp-spec/;
these are provenance paths, not tracked links into run-local state.

The requested `.brokkr/state/tasks/` directory is absent. The matching
[metadata intake](../../evals/issue151-meta-intake.md) names this exact run,
`epic-164-issue-151-make-the-fixt-547fc316`, and supplies sufficient framing.
Both run-local positions were read in full. The tracked
[operator rulings](../../docs/rulings/epic-192.md) remain authority; no new
operator acceptance is inferred. The supplied 2026-10-26 ledger reserves 0079–0083 and names 0084 next.
Integration commit `76678c3` records the subsequent allocation of 0084 to
the dsh fresh-home decision and renumbers this proposal to 0087; it does not
change the earlier ledger or accept this proposal.

Accepted [0055](../../docs/decisions/0055-agent-eval-harness.md) requires the
independent eval workspace's real-process proof. Accepted
[0075](../../docs/decisions/0075-eval-privacy.md) governs finite diagnostics.
Closed arguments under [0044](../../docs/decisions/0044-compat-validation.md)
remain closed. Proposed [0083](../../docs/decisions/0083-cli-wrapper-launch.md)
is boundary context, not an accepted ruling. Metadata is protocol context,
not a tool argument or a value permitted onto the HTTP wire.

## Synthesis and explicit rulings

| Panel position | Ruling and accepted cost |
|---|---|
| Simplicity: one local discarded Call field, existing dispatch owners | Adopt. Keep both deny_unknown_fields declarations. Reject a shared params hierarchy, MCP SDK, revision-specific handlers and metadata validation framework. Opaque non-object values may also be discarded; this is not strict metadata conformance. |
| Robustness: complete real-process leakage and no-effect proof | Adopt alongside the small production patch. Capture all HTTP headers, drain stderr and stdout through EOF, and count actual HTTP requests. A happy-path-only patch loses: it would miss misplaced metadata and extra output. Cost: a larger test matrix and disciplined child cleanup. |
| Both: every present GET body gets a named tool refusal | Adopt the permitted refusal branch. Reject empty-string normalization. Cost: Claude's empty-body retry still needs correction to omit body, but gets actionable feedback; no implicit body rewriting or policy switch. |
| Simplicity: preserve already permissive initialize/ping/list handling | Adopt, consistent with robustness's discard-only rule. Do not tighten those methods or claim full MCP revision conformance. Cost: this repair leaves their existing permissiveness intact. |
| Robustness: storage, webhook, timeout and crash failure analysis | Retain as limits and preservation obligations, not new features or tests of unrelated root behavior. No progress registry, retries, persistence or exactly-once HTTP claim. |

The positions do not genuinely conflict on the selected production behavior.
Their emphasis is reconciled by keeping the implementation local while adopting
the full negative proof. Rich diagnostics, token validation and progress output
lose to bounded discard and a finite error vocabulary; their diagnostic and
progress convenience is deliberately forgone.

## Contract

For negotiated revisions 2025-03-26, 2025-06-18 and 2025-11-25, accept metadata
on initialize, ping, tools/list, tools/call and notifications/initialized.
Omitted metadata, empty objects, integer/string progress tokens and extension
values are inert. Supported notifications remain silent. Unsupported methods
remain unsupported. Only `params._meta` is the new Call slot: never recursively
erase keys or admit `arguments._meta`. Metadata is neither echoed nor forwarded,
and changes no ID, tool selection, body, identity, deadline or retry behavior.
No new tools, resources, progress notifications or capabilities are advertised.

For otherwise valid `eval_http` arguments, GET omission succeeds normally;
any present body yields `result.isError=true` and the text
`http_get_body_not_supported`, no JSON-RPC error and no HTTP request. Preserve
POST JSON serialization and its omission/null distinction. Other helper failures
retain `fixture_request_refused`. Existing malformed call refusals retain
`-32602/tool_arguments_invalid`; unknown methods retain `-32601/method_not_found`.

## Acceptance Criteria

1. **AC1 — Failing first:** before helper edits, add and run focused tests in
   evals/tests/wrappers.rs against the actual fixture-http child and real
   loopback HTTP. Retain command, nonzero exit and failed assertions for metadata
   admission and named GET-body errors. The identical tests pass after repair.
2. **AC2 — Revision/location matrix:** for each of the three revisions, fresh
   helper sessions accept metadata on every supported location above. Cover
   omitted/empty metadata, integer/string progress tokens and nested extension
   objects. Assert negotiated version, response IDs, successful HTTP receipt
   and ordinary tool results. tools/list still advertises exactly eval_http
   with its closed method/path/body schema and no metadata argument.
3. **AC3 — Discard proof:** distinctive metadata sentinels occur in neither
   complete stdout, stderr nor any captured HTTP path/header/body. Finish stdin,
   read through EOF and reap under a bound; assert the complete expected reply
   sequence and zero progress/initialized replies. Include a legitimate POST
   body control whose bytes survive unchanged by metadata handling.
4. **AC4 — Closed shapes:** unknown call params beside _meta, arguments._meta,
   other unknown arguments, missing name/arguments/method/path, wrong-shaped
   arguments and unknown tools retain existing refusal codes and have no HTTP
   effect. Unsupported methods retain their error. Count fixture requests after
   causal child completion; no sleep-based absence assertion.
5. **AC5 — GET and POST:** absent GET body reaches HTTP; empty/nonempty strings,
   null, objects, arrays, booleans and numbers supplied as GET body produce the
   named tool error, never a protocol error or HTTP request. Valid POST bodies
   retain their JSON bytes, including empty string and explicit null; unrelated
   failures stay generic and value-free.
6. **AC6 — Bounds and regressions:** oversized metadata in a real stdin record
   trips the existing 64 KiB whole-line cap (newline included) with no HTTP
   effect or echo. Preserve argument and 1 MiB response caps, identity injection,
   app-relative URL/path and caller-header refusals, no proxy/redirects,
   remaining-deadline behavior and exact UTF-8/base64 response fidelity. Keep
   current assertions and prove no dependencies or root implementation changes.
7. **AC7 — Delivery proof:** independent eval fmt, locked all-target clippy,
   focused wrappers and full eval tests pass. Reprove every local house gate
   using the pinned candidate engine, exact coverage/selftest and guard proofs;
   record actual exits. Root coverage does not measure evals. No green delivery
   claim while a gate fails, including the decision-index conflict below.
8. **AC8 — Design artifact proof:** this chief commits exactly spec.md, plan.md,
   tasks.md and proposed decision 0087 unsigned, with one touched spec directory,
   clean committed artifacts and a passing pinned design-check. Positions stay
   ignored. No push, merge, tag, publication or decision acceptance.

## Delivery limitations and ownership

Adding a decision requires an index row under the existing
`every_decision_file_has_exactly_one_resolving_index_row` root test. The exact
chief artifact whitelist excludes docs/decisions/README.md; the subsequent
implementation scope is evals-only. Therefore this design cannot honestly claim
all root gates green. Record the resulting inventory failure and its effect on
coverage without weakening either check. An operator/documentation owner must
authorize and supply index integration before AC7 can be complete; this draft
does not grant that scope or acceptance. Structural design success is distinct.

CI owns charter macOS/chart/Docker/network-audit/privileged-port proofs; #201
owns eval macOS gating. The operator owns authenticated client interoperability,
paid trials and the baseline. Namespaces remain deferred under #221. No such
external proof becomes a local acceptance criterion. HTTP side effects and
stdio replies are not atomic; interruption can leave an uncertain POST outcome.
The frozen accept==stored, subscribe-before-check, oldest-only pruning and
ingest-extraction owners remain untouched.
